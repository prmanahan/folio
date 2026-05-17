//! Spec #572 round-2 — M2 red-phase + R11a buffered-contract regression pins.
//!
//! Two mechanically-distinct kinds of test live here; do NOT homogenize them:
//!
//!   A. `m2_lenient_fallback_*` — RED-phase. Designed to FAIL on the code at
//!      HEAD (Warden #562 Medium, conf 76: lenient-fallback path emits
//!      still-JSON-escaped text verbatim to the client). Forge's #1012 fix
//!      turns it green. It asserts ONLY client-observable bytes.
//!
//!   B. `r11a_pin_*` — STAY-GREEN. R11a (spec line 76) formalizes server-side
//!      buffering as the chat contract; the shipped impl already conforms.
//!      These pass now and guard against the M2 fix silently undoing the
//!      buffer/discard contract. They are regression guards, not red tests.
//!
//! Observation mechanism (all tests): black-box. The mock Anthropic server
//! (`mockito`) consumes the wire bytes we script as the SSE response; we POST
//! `/api/chat` and assert on the SSE `data:`/`event:` bytes the axum handler
//! streams back to the client. No parser internals, no source-text greps, no
//! internal-module imports for the integration tests.
//!
//! Spec refs: docs/specs/2026-05-15-sonnet-migration-config-table.md
//! (v4, SHA 8bd1a55) — R11a (line 76), R13 (line 85), R29 (line 174),
//! Scenario 1 (line 192), Scenario 8 (line 250). Warden #562 report:
//! scratch/dispatch-562-report.md (Medium finding, lines 31).

mod common;

use common::ai_mock::{
    self, ai_test_app_with_mock, anthropic_sse_response, assert_terminates_with_done, count_event,
    first_event, parse_sse_frames,
};

// ===========================================================================
// Hand-crafted M2 fixture
// ===========================================================================

/// A scripted Anthropic SSE response whose single `content_block_delta`
/// frame is **structurally truncated JSON** (the `delta` object and the
/// outer frame object are never closed) but still carries
/// `"text":"<JSON-escaped fragment>"`. Terminated by a well-formed
/// `EndTurn` `message_delta`.
///
/// Why this triggers the M2 bug (durable trail for Forge #1012):
///
///   1. The truncated `content_block_delta` JSON has no raw control bytes
///      inside the string literal — only the literal two-byte sequences
///      backslash-`"` and backslash-backslash. So the handler's
///      `escape_control_chars_in_json` pre-processor is a no-op on it.
///   2. `serde_json::from_str::<ChatStreamFrame>` then FAILS because the
///      JSON object is unterminated (missing `}` / `}`), exercising the
///      lenient fallback branch.
///   3. `extract_text_delta_lossy` anchors on `"text":"` and reads to the
///      next un-escaped `"`, returning the fragment **verbatim** — still
///      JSON-escaped: it contains literal backslash-`"` and
///      backslash-backslash.
///   4. That verbatim fragment is buffered and, on the `EndTurn` terminal,
///      emitted to the client as a `data:` frame WITHOUT JSON-unescaping.
///
/// The wire JSON stays on a single `data:` line (the escapes are literal
/// backslash sequences, not raw newlines) so SSE line-splitting does not
/// pre-fragment it and the lossy extractor sees the whole `"text":"..."`.
///
/// Deliberately NO `\n` escape in the marker: axum 0.8's
/// `SseEvent::data(v)` truncates a value at the first embedded real
/// newline (it is the SSE field-line terminator — verified empirically:
/// `data("a\nb")` wires as `data: a\n\n`, client sees only `a`). So a
/// correct decode of a `\n`-bearing fragment could never round-trip to
/// the client and the positive assertion would be unsatisfiable for a
/// reason unrelated to the bug. The discriminators are the escaped quote
/// (`\"`) and escaped backslash (`\\`) — both decode to single-line text
/// that round-trips cleanly.
///
/// Wire fragment (still JSON-escaped):   say \"hi\" and a backslash \\ done
/// Decoded intent (what the user sees):  say "hi" and a backslash \ done
const M2_DECODED_SINGLE_LINE: &str = "say \"hi\" and a backslash \\ done";

fn m2_truncated_content_frame_sse() -> String {
    let mut out = String::new();

    out.push_str("event: message_start\n");
    out.push_str(
        r#"data: {"type":"message_start","message":{"id":"msg_m2","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-6","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":1,"output_tokens":1}}}"#,
    );
    out.push_str("\n\n");

    out.push_str("event: content_block_start\n");
    out.push_str(
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
    );
    out.push_str("\n\n");

    // The malformed frame. NOTE the deliberately missing closing braces:
    // `delta` object and the outer object are never closed. The `text`
    // value embeds the JSON escape sequences \" and \\ — literal
    // backslash-quote / backslash-backslash, NOT raw control bytes (and
    // deliberately NO \n, per M2_DECODED_SINGLE_LINE rationale).
    out.push_str("event: content_block_delta\n");
    out.push_str(
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"say \"hi\" and a backslash \\ done""#,
    );
    out.push_str("\n\n");

    // Well-formed terminal: EndTurn. This is the path that flushes the
    // buffered (escaped) fragment verbatim — the bug surface.
    out.push_str("event: message_delta\n");
    out.push_str(
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":2}}"#,
    );
    out.push_str("\n\n");

    out.push_str("event: message_stop\n");
    out.push_str(r#"data: {"type":"message_stop"}"#);
    out.push_str("\n\n");

    out
}

// ===========================================================================
// Fixture sanity guard (loud-fail if the fixture stops being a valid trigger)
// ===========================================================================

/// Guards the M2 fixture's load-bearing precondition: the truncated
/// `content_block_delta` `data:` payload MUST fail a permissive
/// `serde_json::Value` parse. If a future edit accidentally makes it
/// well-formed JSON, the M2 test would silently stop exercising the
/// lenient-fallback branch and pass for the wrong reason. This catches that.
///
/// Mechanism: parses the raw fixture string itself (not a handler) — a
/// pure-data assertion, no internal modules touched.
#[test]
fn m2_fixture_data_payload_fails_strict_json_parse_precondition() {
    // Pull the truncated content_block_delta data line out of the fixture.
    let sse = m2_truncated_content_frame_sse();
    let data_line = sse
        .lines()
        .find(|l| l.starts_with("data:") && l.contains("content_block_delta"))
        .expect("fixture must contain a content_block_delta data line");
    let payload = data_line.strip_prefix("data:").unwrap().trim_start();

    let parsed: Result<serde_json::Value, _> = serde_json::from_str(payload);
    assert!(
        parsed.is_err(),
        "M2 fixture precondition violated: the truncated content_block_delta \
         payload parsed as valid JSON, so the M2 test would NOT exercise the \
         lenient-fallback branch. payload={payload:?}"
    );

    // And the verbatim (still-escaped) fragment the lossy extractor would
    // return must contain the literal escape sequences — otherwise the M2
    // assertion has nothing to discriminate buggy-vs-fixed on.
    assert!(
        payload.contains(r#"\""#) && payload.contains(r#"\\"#),
        "M2 fixture must embed literal \\\" and \\\\ escape sequences in the \
         text value; payload={payload:?}"
    );
    // And explicitly NO `\n` — a `\n`-bearing decoded marker cannot
    // round-trip through axum's SSE framing (newline truncates the
    // data: field), which would make the M2 positive assertion
    // unsatisfiable for a reason unrelated to the bug.
    assert!(
        !payload.contains(r#"\n"#),
        "M2 fixture must NOT embed a \\n escape — see M2_DECODED_SINGLE_LINE \
         rationale; payload={payload:?}"
    );
}

// ===========================================================================
// A. M2 — RED-phase: lenient fallback must deliver DECODED text to the client
// ===========================================================================

/// Given the model streams a content frame whose JSON fails strict
///     `ChatStreamFrame` parse (structurally truncated) but carries text
///     with JSON escape sequences (`\"`, `\\`), terminated by an `EndTurn`
///     `message_delta`
/// When a client POSTs `/api/chat`
/// Then the SSE `data:` bytes the client receives carry the **decoded**
///     text — a real `"` and a real `\` — and NEVER the literal
///     two-character sequences backslash-`"` / backslash-backslash.
///
/// Why decoded-client-output is the right black-box observable: the spec's
/// chat contract is "text the user sees". A fragment routed through the
/// lenient fallback is still the model's content; the user-facing contract
/// is that it renders as text, not as literal escape glyphs. The only
/// surface that contract is observable on is the SSE `data:` bytes the
/// browser EventSource receives — which is exactly what this test reads.
/// The test says NOTHING about which parser ran or how decoding happens;
/// Forge is free to fix by unescaping in the fallback OR by widening strict
/// parse — both satisfy this assertion. (Pre-signed boundary: lenient
/// parsing itself stays; only the client-visible bytes are pinned.)
///
/// RED-phase expectation: this FAILS on the code at HEAD (Warden #562
/// Medium) — the fallback fragment is buffered and flushed verbatim, so
/// the client sees `say \"hi\" and a backslash \\ done` with literal
/// backslashes. Forge #1012 turns it green.
#[tokio::test]
async fn m2_lenient_fallback_path_delivers_decoded_text_not_json_escaped_bytes_to_client() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(m2_truncated_content_frame_sse())
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Hi" }))
        .await;
    response.assert_status_ok();

    let body_bytes = response.as_bytes();
    let body_str = std::str::from_utf8(body_bytes).expect("SSE body must be UTF-8");

    // Frames the client actually receives. The decoded marker is
    // single-line by construction (no `\n`), so it round-trips through
    // axum's `SseEvent::data()` and `parse_sse_frames` without any
    // newline-framing concern.
    let frames = parse_sse_frames(body_bytes);
    let content: String = frames
        .iter()
        .filter(|(name, data)| name.is_none() && data != "[DONE]")
        .map(|(_, d)| d.clone())
        .collect::<Vec<_>>()
        .join("");

    // Negative side (R29-adjacent display contract): the client MUST NOT
    // receive the literal escape sequences.
    assert!(
        !content.contains(r#"\""#),
        "M2: client content MUST NOT contain the literal backslash-quote \
         sequence (lenient fallback emitted JSON-escaped bytes verbatim — \
         Warden #562 Medium). content={content:?} body={body_str:?}"
    );
    assert!(
        !content.contains(r#"\\"#),
        "M2: client content MUST NOT contain the literal double-backslash \
         sequence. content={content:?} body={body_str:?}"
    );

    // Positive side: the DECODED text actually reaches the client. This
    // trips if a "fix" silently drops the fallback frame instead of
    // decoding it — no content is not an acceptable resolution.
    assert!(
        content.contains(M2_DECODED_SINGLE_LINE),
        "M2: client content MUST contain the DECODED fragment {:?} (real \
         quote + real backslash); a fix that drops the frame entirely is \
         not acceptable. content={content:?} body={body_str:?}",
        M2_DECODED_SINGLE_LINE
    );

    // The stream still terminates cleanly regardless of the fallback.
    assert_terminates_with_done(&frames);
}

// ===========================================================================
// B. R11a — STAY-GREEN regression pins (server-side buffering is the
//    chat contract; the shipped impl already conforms)
// ===========================================================================

/// R11a / Scenario 1 (spec lines 76, 192) — buffered in-order flush on
/// EndTurn.
///
/// Given a mock stream of multiple content deltas then an `EndTurn`
///     `message_delta`
/// When a client POSTs `/api/chat`
/// Then the buffered content is flushed **in order** as default-event
///     `data:` frames followed by `[DONE]`, with NO terminal error event
///     (`refusal` / `context_exceeded` / `truncated` / `error`).
///
/// Mechanism: black-box — mock SSE wire bytes in, SSE `data:`/`event:`
/// bytes out. Regression guard: it passes today and MUST keep passing so
/// the M2 fix cannot silently undo the buffer-then-flush-in-order contract.
#[tokio::test]
async fn r11a_pin_endturn_flushes_buffered_content_in_order_then_done_no_terminal_error() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(
            &["alpha ", "beta ", "gamma"],
            "end_turn",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Hi" }))
        .await;
    response.assert_status_ok();

    let frames = parse_sse_frames(response.as_bytes());

    // No terminal error/notice events on a clean EndTurn.
    assert_eq!(
        count_event(&frames, "refusal"),
        0,
        "R11a EndTurn must not emit refusal, frames={frames:?}"
    );
    assert_eq!(
        count_event(&frames, "context_exceeded"),
        0,
        "R11a EndTurn must not emit context_exceeded, frames={frames:?}"
    );
    assert_eq!(
        count_event(&frames, "truncated"),
        0,
        "R11a EndTurn must not emit truncated, frames={frames:?}"
    );
    assert_eq!(
        count_event(&frames, "error"),
        0,
        "R11a EndTurn must not emit error, frames={frames:?}"
    );

    // Buffered content flushed IN ORDER. Concatenated default-event data
    // frames (excluding the [DONE] sentinel) must equal the deltas joined
    // in stream order.
    let content: String = frames
        .iter()
        .filter(|(name, data)| name.is_none() && data != "[DONE]")
        .map(|(_, d)| d.clone())
        .collect::<Vec<_>>()
        .join("");
    assert_eq!(
        content, "alpha beta gamma",
        "R11a: buffered content MUST be flushed in stream order, frames={frames:?}"
    );

    assert_terminates_with_done(&frames);
}

/// R11a / R29 / Scenario 8 (spec lines 76, 174, 250) — refusal discards
/// the buffer.
///
/// Given a mock stream with content deltas (carrying a DO-NOT-LEAK marker)
///     then a `refusal` stop_reason
/// When a client POSTs `/api/chat`
/// Then the client receives ONLY `event: refusal` with the canned data
///     `The assistant declined to respond to that request.` then `[DONE]`
///     — the buffered model text is discarded; the raw model text NEVER
///     reaches the client (R29).
///
/// Mechanism: black-box — mock SSE wire bytes in, SSE bytes out. Regression
/// guard: passes today; MUST keep passing so the M2 decode fix cannot
/// resurrect the discarded buffer onto the refusal path (which would be an
/// R29 violation).
#[tokio::test]
async fn r11a_pin_refusal_discards_buffer_client_receives_only_canned_refusal_then_done() {
    const DO_NOT_LEAK: &str = "DO-NOT-LEAK-r11a-pin-refusal-raw-model-text";

    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(
            &["some buffered ", DO_NOT_LEAK, " more"],
            "refusal",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Anything" }))
        .await;
    response.assert_status_ok();

    let body_bytes = response.as_bytes();
    let body_str = std::str::from_utf8(body_bytes).expect("UTF-8");
    let frames = parse_sse_frames(body_bytes);

    // Exactly one refusal frame with the canned message.
    assert_eq!(
        count_event(&frames, "refusal"),
        1,
        "R11a/Scenario 8: exactly one event:refusal frame expected, frames={frames:?}"
    );
    let refusal = first_event(&frames, "refusal").expect("refusal frame present");
    assert_eq!(
        refusal.1, "The assistant declined to respond to that request.",
        "R11a/Scenario 8: event:refusal data MUST be the canned message"
    );

    // The buffered model text (DO-NOT-LEAK marker) MUST be discarded —
    // R29: never reaches the client body under any path.
    assert!(
        !body_str.contains(DO_NOT_LEAK),
        "R11a/R29: refusal MUST discard the buffer; raw model text leaked. body={body_str:?}"
    );

    // No content default-event frames before the refusal (buffer discarded,
    // not flushed-then-refused).
    let content_frames: Vec<&String> = frames
        .iter()
        .filter(|(name, data)| name.is_none() && data != "[DONE]")
        .map(|(_, d)| d)
        .collect();
    assert!(
        content_frames.is_empty(),
        "R11a: refusal path MUST NOT flush any buffered content frame; \
         got content frames {content_frames:?}, all frames={frames:?}"
    );

    assert_terminates_with_done(&frames);
}

// ===========================================================================
// Suppress unused-helper warnings (helpers shared across the AI test files)
// ===========================================================================

#[allow(dead_code)]
fn _silence_unused() {
    let _ = ai_mock::valid_fit_verdict_json();
}
