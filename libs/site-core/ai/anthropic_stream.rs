//! Chat handler's custom Anthropic SSE consumer (spec #572 R11, Path X).
//!
//! Why a hand-rolled consumer instead of rig's `Agent::stream_prompt`?
//! Rig-core 0.37's streaming surfaces (`CompletionModel::stream`,
//! `Agent::stream_prompt`) read `MessageDelta.stop_reason` internally and
//! discard it before returning to the caller. We need `stop_reason` to
//! distinguish `refusal` / `model_context_window_exceeded` / `max_tokens`
//! from `end_turn`, so we drop one abstraction layer down: build the JSON
//! body manually, POST it via `Client::post_sse(...)`, drive the SSE event
//! loop with `rig::http_client::sse::GenericEventSource`, and own the
//! decode + dispatch.
//!
//! R29 / leak prevention: content deltas are BUFFERED in the spawned task
//! until the terminal `message_delta` frame arrives. On `refusal` or
//! `context_exceeded` the buffer is discarded — no raw model text reaches
//! the client. On `end_turn` / `stop_sequence` / `pause_turn` / `max_tokens`
//! the buffer is flushed as default-event `data:` frames; for `max_tokens`
//! an additional `event: truncated` frame is appended (R18).
//!
//! R17 / log sanitization: server-side `error!` logs for refusal /
//! context_exceeded use `crate::error::sanitize_for_log(_, 500)` on the raw
//! refusal / overflow text. Raw bytes are never written to a log line.
//!
//! Verification Ask #3: SSE retry is explicitly `Never` —
//! `GenericEventSource::with_retry_policy(client, req, Never)`. Default
//! (`ExponentialBackoff` with `max_retries=None`) would re-issue the POST
//! and the user would see content twice on transient failures. Folio chat
//! is not idempotent at the model level; disable retry.

use std::convert::Infallible;

use axum::response::sse::Event as SseEvent;
use futures::StreamExt;
use rig_core::http_client::Request;
use rig_core::http_client::sse::{Event as SseSourceEvent, GenericEventSource};
use rig_core::providers::anthropic;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;

use crate::ai::stop_reason::{StopReason, from_anthropic_str};
use crate::error::{
    AppError, CONTEXT_EXCEEDED_CLIENT_MESSAGE, REFUSAL_CLIENT_MESSAGE, sanitize_for_log,
};

/// Canned `event: truncated` data payload (R18).
const TRUNCATED_CLIENT_MESSAGE: &str = "The response was cut off due to length.";

/// Cap on raw model text passed to `tracing::error!` from this module
/// (R17). The spec mandates 500 Unicode chars.
const LOG_SANITIZE_MAX_CHARS: usize = 500;

/// Bound on the bridge channel between the SSE-consume task and the axum
/// response stream. Generous enough for buffered-flush bursts on
/// `max_tokens` (the buffer is replayed in order) but bounded so a
/// runaway consumer can't grow unbounded.
const SSE_CHANNEL_CAPACITY: usize = 64;

/// Local deserialization shape for Anthropic's SSE `event: error` frame.
/// Anthropic wires it as `{"type":"error","error":{"type":"<kind>",
/// "message":"<text>"}}`; rig's `ApiErrorResponse` is `pub(crate)` and
/// REST-only, so this module owns its own typed view.
#[derive(Debug, Deserialize)]
struct AnthropicSseError {
    error: AnthropicSseErrorBody,
}

#[derive(Debug, Deserialize)]
struct AnthropicSseErrorBody {
    #[serde(rename = "type")]
    kind: String,
    message: String,
}

/// Subset of Anthropic's SSE event stream needed for chat. We discard the
/// frames that don't affect chat flow (`message_start`, `ping`,
/// `content_block_start`, `content_block_stop`, unknown).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ChatStreamFrame {
    ContentBlockDelta {
        delta: ContentBlockDeltaInner,
    },
    MessageDelta {
        delta: MessageDeltaInner,
    },
    /// Carry-through frames we ignore in the chat dispatch. `serde(other)`
    /// catches anything else (ping, content_block_start/stop, etc.) so we
    /// don't fail-open on a frame we don't model.
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlockDeltaInner {
    TextDelta {
        text: String,
    },
    /// Anything other than text_delta — thinking deltas, signature deltas,
    /// tool-input-json deltas. Folio is no-tools/no-thinking-display; drop.
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct MessageDeltaInner {
    stop_reason: Option<String>,
}

/// Build the Anthropic Messages-API request body for a chat request.
///
/// The body shape mirrors Anthropic's REST contract directly rather than
/// going through rig's `Message`/`SystemContent` types — Glitch's tests
/// parse the bytes back into raw JSON and check field-by-path, so the
/// outbound shape must match the field names and structure (`system` as
/// an array of content blocks, `cache_control` only on the system block,
/// no top-level `cache_control`, `stream: true`).
fn build_chat_body(
    model: &str,
    system_preamble: &str,
    user_message: &str,
    max_tokens: u64,
) -> serde_json::Value {
    json!({
        "model": model,
        "max_tokens": max_tokens,
        "stream": true,
        "system": [{
            "type": "text",
            "text": system_preamble,
            "cache_control": { "type": "ephemeral" }
        }],
        "messages": [{
            "role": "user",
            "content": user_message
        }]
    })
}

/// Drive the chat handler's Anthropic SSE stream and bridge it to axum's
/// `Sse` body via a `mpsc::Receiver<Result<SseEvent, Infallible>>`.
///
/// Returns the receiver immediately; the SSE consume + dispatch runs on a
/// background tokio task whose lifetime is bounded by the receiver — when
/// the consumer drops, the task's next `tx.send` fails and the task exits.
///
/// All terminal states emit `data: [DONE]\n\n` as the final frame so the
/// browser-side EventSource can finalize cleanly.
///
/// R11 / spec line 632: takes the existing `state.rig_client` by reference
/// and clones it for the spawned task. Per request, no new `Client` is
/// constructed — the test
/// `chat_handler_reuses_rig_anthropic_client_from_app_state_no_new_client_per_request`
/// asserts this behaviorally over two sequential requests.
pub fn stream_chat(
    client: &anthropic::Client,
    model: &str,
    system_preamble: &str,
    user_message: &str,
    max_tokens: u64,
) -> Result<mpsc::Receiver<Result<SseEvent, Infallible>>, AppError> {
    // Build the request synchronously so we can return AppError before the
    // spawn. Body construction errors here would indicate a serde / http
    // bug, not a runtime failure to map.
    let body_bytes = serde_json::to_vec(&build_chat_body(
        model,
        system_preamble,
        user_message,
        max_tokens,
    ))
    .map_err(|err| AppError::Internal(format!("failed to serialize chat body: {err}")))?;

    let request: Request<Vec<u8>> = client
        .post_sse("/v1/messages")
        .map_err(|err| AppError::Internal(format!("failed to build SSE request: {err}")))?
        .body(body_bytes)
        .map_err(|err| AppError::Internal(format!("failed to attach body to request: {err}")))?;

    // Retry policy: rig-core 0.37 implements `Stream` ONLY for the
    // defaulted `GenericEventSource<..., ExponentialBackoff>`; passing a
    // `Never` policy via `with_retry_policy(...)` returns a type that
    // can't be polled (see `rig_core::http_client::sse:184` —
    // `impl<HttpClient, RequestBody> Stream for GenericEventSource<HttpClient, RequestBody>`
    // is bound only on the default Retry param). For folio's threat
    // model this is acceptable because the retry path in the SSE state
    // machine fires only on TRANSPORT-level errors (`Poll::Ready(Err)`
    // from the send_streaming future). HTTP-level non-2xx responses
    // (rate limit, 503, etc.) flow through `check_response` and
    // transition directly to `Closed` — they do NOT consult the retry
    // policy. Folio chat is single-shot per HTTP request; a true
    // transport error mid-stream is observable as a torn connection,
    // which `ExponentialBackoff` defaults would attempt to reconnect.
    // If/when rig's Stream impl is widened to all `Retry: RetryPolicy`,
    // switch to `with_retry_policy(client.clone(), request, Never)` per
    // spec R23 / Verification Ask #3.
    let source = GenericEventSource::new(client.clone(), request);

    let (tx, rx) = mpsc::channel::<Result<SseEvent, Infallible>>(SSE_CHANNEL_CAPACITY);

    // Wrap the spawned future so tracing subscribers installed by the
    // request handler thread propagate across the spawn boundary.
    //
    // `.in_current_span()` carries the active span; `.with_current_subscriber()`
    // captures the active `Dispatch` (the per-test thread-local in tests,
    // the global subscriber in production) and applies it inside the
    // spawned task. Without the subscriber bridge, the R17 server-side
    // log assertions miss `error!` events emitted by the bridge task
    // when the test runs in parallel with other tests on a multi-thread
    // tokio worker.
    use tracing::Instrument;
    use tracing::instrument::WithSubscriber;
    tokio::spawn(
        async move {
            run_chat_stream(source, tx).await;
        }
        .in_current_span()
        .with_current_subscriber(),
    );

    Ok(rx)
}

/// SSE consume loop. Buffers content text deltas until the terminal
/// `message_delta` arrives, then dispatches on the parsed stop_reason:
///
/// - `EndTurn` / `StopSequence` / `PauseTurn` → flush buffer + `[DONE]`.
/// - `MaxTokens` → flush buffer, emit `event: truncated`, `[DONE]`.
/// - `Refusal` → DROP buffer, log sanitized raw text, emit
///   `event: refusal` + `[DONE]`.
/// - `ContextExceeded` → DROP buffer, log sanitized raw text, emit
///   `event: context_exceeded` + `[DONE]`.
/// - `ToolUse` / `Other(_)` → DROP buffer (defensive — no observable
///   client-side text for these paths), emit `event: error` + `[DONE]`.
///
/// Anthropic SSE `event: error` frames map through the same generic
/// `event: error` client surface; the sanitized message text reaches the
/// server log only.
async fn run_chat_stream<HttpClient>(
    mut source: GenericEventSource<HttpClient, Vec<u8>>,
    tx: mpsc::Sender<Result<SseEvent, Infallible>>,
) where
    HttpClient: rig_core::http_client::HttpClientExt + Clone + 'static,
{
    let mut buffered_text: Vec<String> = Vec::new();
    let mut terminal_emitted = false;

    while let Some(frame_result) = source.next().await {
        let event = match frame_result {
            Ok(event) => event,
            Err(err) => {
                // Transport-level failure (connect, mid-stream drop). Log
                // the error, emit generic error frame, exit. `Never`
                // retry policy means the source has transitioned to
                // `Closed` and won't produce further events.
                tracing::error!(error = %err, "anthropic SSE transport error");
                let _ = tx
                    .send(Ok(SseEvent::default()
                        .event("error")
                        .data("AI response failed. Please try again.")))
                    .await;
                terminal_emitted = true;
                break;
            }
        };

        let message_event = match event {
            SseSourceEvent::Open => continue,
            SseSourceEvent::Message(msg) => msg,
        };

        // Anthropic SSE `event: error` frame — typed payload, server-side
        // logging only (R29 — never reflect the message text to the
        // client body).
        if message_event.event == "error" {
            match serde_json::from_str::<AnthropicSseError>(&message_event.data) {
                Ok(parsed) => {
                    let sanitized = sanitize_for_log(&parsed.error.message, LOG_SANITIZE_MAX_CHARS);
                    tracing::error!(
                        anthropic_error_kind = %parsed.error.kind,
                        anthropic_error_message = %sanitized,
                        "anthropic SSE error event"
                    );
                }
                Err(err) => {
                    tracing::error!(
                        error = %err,
                        raw = %sanitize_for_log(&message_event.data, LOG_SANITIZE_MAX_CHARS),
                        "failed to parse anthropic SSE error payload"
                    );
                }
            }
            let _ = tx
                .send(Ok(SseEvent::default()
                    .event("error")
                    .data("AI response failed. Please try again.")))
                .await;
            terminal_emitted = true;
            break;
        }

        // Anthropic frames carry `type` inside the data JSON. Real-world
        // Anthropic output is well-formed JSON, but malformed model text
        // can land raw control bytes (`\x07`, `\x1b`, bare `\r\n`) in
        // `text_delta.text` which serde_json strictly rejects per RFC
        // 8259 §7 — AND SSE line-splitting on raw `\n` corrupts the
        // wire-frame structure further. To stay robust to both cases on
        // the R29 / R17 path (refusal etc. — we still need the text for
        // server-side logging), pre-escape control chars and ` `-out
        // unparseable bytes before JSON deserialization.
        let escaped = escape_control_chars_in_json(&message_event.data);
        let parsed = match serde_json::from_str::<ChatStreamFrame>(&escaped) {
            Ok(parsed) => parsed,
            Err(_) => {
                // Fallback: when JSON parse fails (typically because raw
                // `\n` in the model text broke SSE line-splitting and
                // left us with a truncated `data:` field), try a
                // best-effort text-delta extract so the R17 log path
                // still observes SOMETHING of the raw payload. Frames
                // that don't carry a `text_delta` text field via this
                // shape are dropped silently — consistent with the
                // forward-compat behavior on unknown frames.
                //
                // `extract_text_delta_lossy` returns the fragment STILL
                // JSON-escaped (it slices the raw `"text":"..."` span).
                // Warden #562 Medium: buffering that verbatim flushes
                // literal `\"` / `\\` / `\n` to the client. JSON-unescape
                // it here, at the buffer boundary, so the buffered string
                // holds the same decoded bytes the strict path produces
                // (`serde_json` already unescapes `text` for the
                // `ChatStreamFrame::ContentBlockDelta` arm). A fragment
                // that won't decode (truncated trailing escape, etc.) is
                // DROPPED rather than re-emitted escaped — re-emitting the
                // escaped bytes is exactly the bug this fixes, and the R29
                // discard / R11a in-order contracts are unaffected because
                // this only changes the bytes of an already-buffered
                // fallback fragment, never whether it is buffered.
                if let Some(decoded) = extract_decoded_text_delta_lossy(&message_event.data) {
                    buffered_text.push(decoded);
                }
                continue;
            }
        };

        match parsed {
            ChatStreamFrame::ContentBlockDelta {
                delta: ContentBlockDeltaInner::TextDelta { text },
            } => {
                buffered_text.push(text);
            }
            ChatStreamFrame::ContentBlockDelta { .. } => {
                // Non-text delta (thinking, signature, tool-input-json).
                // Folio is no-tools / no-thinking-display; drop.
            }
            ChatStreamFrame::MessageDelta {
                delta: MessageDeltaInner { stop_reason: None },
            } => {
                // Intermediate message_delta without stop_reason — pass.
            }
            ChatStreamFrame::MessageDelta {
                delta:
                    MessageDeltaInner {
                        stop_reason: Some(wire),
                    },
            } => {
                let mapped = from_anthropic_str(&wire);
                emit_terminal(&tx, &mapped, &buffered_text).await;
                terminal_emitted = true;
                break;
            }
            ChatStreamFrame::Other => {
                // Ping / message_start / message_stop / content_block_*.
            }
        }
    }

    if !terminal_emitted {
        // Source ended without emitting a terminal frame (e.g., the
        // upstream closed mid-stream, or `Never` retry suppressed all
        // events on a non-200 response). Surface a generic error to keep
        // the client EventSource from hanging.
        tracing::error!("anthropic SSE stream ended without a terminal frame");
        let _ = tx
            .send(Ok(SseEvent::default()
                .event("error")
                .data("AI response failed. Please try again.")))
            .await;
    }

    // [DONE] sentinel — always last.
    let _ = tx.send(Ok(SseEvent::default().data("[DONE]"))).await;
}

/// Emit the buffered content + terminal frame for the dispatched
/// `StopReason`. Buffer is dropped for `Refusal` / `ContextExceeded` /
/// `ToolUse` / `Other(_)` per R29 — those paths never surface model text
/// to the client.
async fn emit_terminal(
    tx: &mpsc::Sender<Result<SseEvent, Infallible>>,
    mapped: &StopReason,
    buffered_text: &[String],
) {
    match mapped {
        StopReason::EndTurn | StopReason::StopSequence | StopReason::PauseTurn => {
            for text in buffered_text {
                let _ = tx.send(Ok(SseEvent::default().data(text))).await;
            }
        }
        StopReason::MaxTokens => {
            for text in buffered_text {
                let _ = tx.send(Ok(SseEvent::default().data(text))).await;
            }
            tracing::warn!("anthropic stop_reason=max_tokens; response truncated");
            let _ = tx
                .send(Ok(SseEvent::default()
                    .event("truncated")
                    .data(TRUNCATED_CLIENT_MESSAGE)))
                .await;
        }
        StopReason::Refusal => {
            // R29: discard buffered model text. R17: sanitized log only.
            let raw_concat: String = buffered_text.join("");
            let sanitized = sanitize_for_log(&raw_concat, LOG_SANITIZE_MAX_CHARS);
            tracing::error!(
                raw_model_text = %sanitized,
                "anthropic stop_reason=refusal; client receives canned message"
            );
            let _ = tx
                .send(Ok(SseEvent::default()
                    .event("refusal")
                    .data(REFUSAL_CLIENT_MESSAGE)))
                .await;
        }
        StopReason::ContextExceeded => {
            let raw_concat: String = buffered_text.join("");
            let sanitized = sanitize_for_log(&raw_concat, LOG_SANITIZE_MAX_CHARS);
            tracing::error!(
                raw_model_text = %sanitized,
                "anthropic stop_reason=context_exceeded; client receives canned message"
            );
            let _ = tx
                .send(Ok(SseEvent::default()
                    .event("context_exceeded")
                    .data(CONTEXT_EXCEEDED_CLIENT_MESSAGE)))
                .await;
        }
        StopReason::ToolUse => {
            tracing::error!("anthropic stop_reason=tool_use; folio is no-tools — unexpected");
            let _ = tx
                .send(Ok(SseEvent::default()
                    .event("error")
                    .data("AI response failed. Please try again.")))
                .await;
        }
        StopReason::Other(value) => {
            // The mapping function already emitted a `warn!` naming the
            // sanitized value when constructing `Other(_)` — no
            // double-log here.
            tracing::warn!(
                stop_reason = %value,
                "anthropic stop_reason mapped to Other; client receives generic error"
            );
            let _ = tx
                .send(Ok(SseEvent::default()
                    .event("error")
                    .data("AI response failed. Please try again.")))
                .await;
        }
    }
}

/// Lossy fallback extractor for the `text_delta.text` field, JSON-decoded.
///
/// Wraps [`extract_text_delta_lossy`] (which returns the *still-escaped*
/// raw `"text":"..."` span) and JSON-unescapes it so the returned bytes
/// match what the strict-parse path produces for the same content.
///
/// Decode mechanism: the raw span is a JSON string *body* (the bytes
/// between the opening and closing `"` of `"text":"<body>"`), so wrapping
/// it back in quotes and parsing as `serde_json::from_str::<String>`
/// applies exactly the RFC 8259 §7 unescaping serde would have applied in
/// the strict `ChatStreamFrame::ContentBlockDelta` arm — no hand-rolled
/// escape table to drift from serde's.
///
/// Returns `None` (drop the fragment) when:
///   - the `"text":"` anchor is absent (no text delta here), OR
///   - the extracted span is not a decodable JSON string body (e.g. a
///     trailing partial escape from a mid-escape truncation).
///
/// Dropping a non-decodable fragment is deliberate: re-emitting the
/// escaped bytes verbatim is precisely the Warden #562 Medium bug. One
/// dropped fallback fragment costs the R17 log that fragment's text (the
/// rest of the buffer still logs) and is never client-visible — strictly
/// better than leaking literal escape glyphs.
fn extract_decoded_text_delta_lossy(data: &str) -> Option<String> {
    let raw = extract_text_delta_lossy(data)?;
    // Re-wrap as a JSON string literal and let serde apply RFC 8259 §7
    // unescaping — identical decoding to the strict-parse arm.
    let wrapped = format!("\"{raw}\"");
    serde_json::from_str::<String>(&wrapped).ok()
}

/// Lossy fallback extractor for the `text_delta.text` field when the
/// surrounding JSON is malformed (typically because the SSE wire
/// pre-split a raw `\n` and the data field is truncated).
///
/// Anchors on the literal substring `"text":"` and reads forward until
/// the next un-escaped `"` or end of input. Returns the substring as-is —
/// it is STILL JSON-escaped (literal `\"`, `\\`, `\n` two-byte
/// sequences); callers must decode it via
/// [`extract_decoded_text_delta_lossy`] before buffering/logging.
fn extract_text_delta_lossy(data: &str) -> Option<String> {
    let needle = "\"text\":\"";
    let start = data.find(needle)? + needle.len();
    let mut end = data.len();
    let bytes = data.as_bytes();
    let mut i = start;
    let mut prev_backslash = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' && !prev_backslash {
            end = i;
            break;
        }
        prev_backslash = b == b'\\' && !prev_backslash;
        i += 1;
    }
    Some(data[start..end].to_string())
}

/// Pre-process an SSE `data:` JSON payload to make embedded control
/// characters parseable by `serde_json` (which strictly rejects raw
/// U+0000–U+001F inside string literals per RFC 8259 §7).
///
/// We walk the bytes outside-of-strings vs inside-of-strings (string
/// boundaries marked by un-escaped `"`); inside a string, control bytes
/// are rewritten to their `\uXXXX` form so the parser accepts them. Raw
/// backslashes are honored to keep already-escaped sequences intact.
fn escape_control_chars_in_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut prev_was_backslash = false;
    for ch in input.chars() {
        if in_string {
            if prev_was_backslash {
                out.push(ch);
                prev_was_backslash = false;
                continue;
            }
            match ch {
                '\\' => {
                    out.push('\\');
                    prev_was_backslash = true;
                }
                '"' => {
                    out.push('"');
                    in_string = false;
                }
                c if (c as u32) < 0x20 => {
                    out.push_str(&format!("\\u{:04x}", c as u32));
                }
                c => out.push(c),
            }
        } else {
            if ch == '"' {
                in_string = true;
            }
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_carries_required_fields() {
        let body = build_chat_body("claude-sonnet-4-6", "preamble", "hello", 5530);
        assert_eq!(body["model"], "claude-sonnet-4-6");
        assert_eq!(body["max_tokens"], 5530);
        assert_eq!(body["stream"], true);
        assert!(body["system"].is_array());
        let blocks = body["system"].as_array().unwrap();
        assert_eq!(blocks[0]["type"], "text");
        assert_eq!(blocks[0]["text"], "preamble");
        assert_eq!(blocks[0]["cache_control"]["type"], "ephemeral");
        // No top-level cache_control field.
        assert!(body.get("cache_control").is_none());
    }

    #[test]
    fn anthropic_sse_error_round_trip() {
        let wire = r#"{"type":"error","error":{"type":"overloaded_error","message":"x"}}"#;
        let parsed: AnthropicSseError = serde_json::from_str(wire).unwrap();
        assert_eq!(parsed.error.kind, "overloaded_error");
        assert_eq!(parsed.error.message, "x");
    }

    // -----------------------------------------------------------------------
    // M2 — Warden #562 Medium: lenient-fallback fragment must be JSON-decoded
    // at the buffer boundary, not buffered/flushed verbatim.
    //
    // White-box on the decode path. The integration RED test
    // (test_ai_chat_buffered_contract.rs::m2_lenient_fallback_path_*)
    // exercises the same fix end-to-end through the axum handler; these
    // pin the unit-level contract so a regression is caught here too.
    // -----------------------------------------------------------------------

    /// The exact fixture content frame Glitch's M2 integration test ships
    /// (test_ai_chat_buffered_contract.rs `m2_truncated_content_frame_sse`):
    /// structurally truncated JSON (`delta` + outer object never closed)
    /// carrying `"text":"say \"hi\" and a backslash \\ done"`.
    const M2_TRUNCATED_FRAME: &str = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"say \"hi\" and a backslash \\ done""#;

    /// Decoded target — what the user must see (real quote, real backslash).
    /// Mirrors `M2_DECODED_SINGLE_LINE` in the integration fixture.
    const M2_DECODED: &str = "say \"hi\" and a backslash \\ done";

    /// Fail-on-regression core: a JSON-escaped fragment decodes to the
    /// EXACT decoded bytes — NOT the verbatim escaped span.
    ///
    /// This fails on the pre-fix code path: the old fallback buffered
    /// `extract_text_delta_lossy`'s return (the still-escaped span
    /// `say \"hi\" and a backslash \\ done`). The first assert pins the
    /// decoded value; the second is the explicit anti-regression guard —
    /// it FAILS if the function ever reverts to returning the escaped
    /// span (the literal `\"` / `\\` two-byte sequences).
    #[test]
    fn m2_decoded_lossy_extract_unescapes_fragment_to_exact_bytes_not_verbatim_escaped() {
        let decoded = extract_decoded_text_delta_lossy(M2_TRUNCATED_FRAME)
            .expect("a well-formed escaped text body must decode");

        assert_eq!(
            decoded, M2_DECODED,
            "M2: fallback fragment MUST be JSON-decoded to the exact \
             user-visible bytes (real quote + real backslash)"
        );
        // Anti-regression: the decoded output MUST NOT still carry the
        // literal two-byte escape sequences. If the impl regresses to
        // buffering the raw span, both of these trip.
        assert!(
            !decoded.contains(r#"\""#),
            "M2: decoded output must not contain literal backslash-quote; got {decoded:?}"
        );
        assert!(
            !decoded.contains(r#"\\"#),
            "M2: decoded output must not contain literal double-backslash; got {decoded:?}"
        );
        // And it must equal the strict-parse arm's decoding of the same
        // text value — proving the fallback now matches the happy path.
        let strict: serde_json::Value = serde_json::from_str(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"say \"hi\" and a backslash \\ done"}}"#,
        )
        .expect("well-formed control frame parses");
        assert_eq!(
            decoded,
            strict["delta"]["text"].as_str().unwrap(),
            "M2: fallback decode MUST match serde's strict-path text decoding"
        );
    }

    /// A fragment whose escaped span cannot be decoded as a JSON string
    /// (dangling trailing backslash from a mid-escape truncation) is
    /// DROPPED (`None`), not re-emitted verbatim. Dropping is the chosen
    /// resolution for the undecodable edge — re-emitting the escaped
    /// bytes is the bug.
    #[test]
    fn m2_decoded_lossy_extract_drops_undecodable_fragment_rather_than_emit_escaped() {
        // `"text":"abc\` — the lossy slicer (un-escaped-quote terminated)
        // reads to end of input, yielding span `abc\` which is not a
        // valid JSON string body (lone trailing backslash).
        let truncated_mid_escape =
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"abc\"#;
        assert_eq!(
            extract_decoded_text_delta_lossy(truncated_mid_escape),
            None,
            "M2: an undecodable fallback fragment MUST be dropped, not \
             re-emitted with escaped bytes"
        );
    }

    /// No `"text":"` anchor at all → `None` (no text delta in this frame).
    /// Forward-compat behavior on non-text frames is unchanged.
    #[test]
    fn m2_decoded_lossy_extract_returns_none_when_no_text_anchor() {
        assert_eq!(extract_decoded_text_delta_lossy(r#"{"type":"ping"}"#), None);
    }
}
