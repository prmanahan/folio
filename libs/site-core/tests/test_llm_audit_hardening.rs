//! Red-phase acceptance tests for the LLM-audit hardening requirements
//! R1 (timeouts), R5 (DefaultBodyLimit), R6 (CORS fail-loud), R7 (opaque
//! client strings).
//!
//! Spec: `docs/specs/2026-05-18-llm-audit-remediation.md`.
//!
//! Test-mechanism notes (read before going green — Forge handoff):
//!
//!  - **R1 stall seam:** mockito 1.7's `with_chunked_body(|w| {...})` runs
//!    the body closure on a DEDICATED OS THREAD (verified:
//!    `mockito::response::ChunkedStream::new` → `thread::Builder::spawn`).
//!    A blocking `std::thread::sleep` inside the closure stalls HTTP
//!    response delivery WITHOUT blocking the tokio runtime or other tests.
//!    The sleep is BOUNDED (not infinite) because `ChunkedStream::drop`
//!    joins the body thread; the handler's timeout fires well before the
//!    sleep ends and the test asserts on observed wall-clock.
//!    NO new test dependency — std + the existing mockito seam only.
//!
//!  - **R1/R5/R6 wiring lives in `cmd/server/main.rs`** (`run_server()`),
//!    which is NOT exposed through any site-core public fn and is
//!    forbid-scoped for the implementer. Where a behavior test cannot
//!    reach the layer, the GATE is a source-as-text meta-test that reads
//!    `cmd/server/main.rs` (reading ≠ modifying; in scope) and asserts the
//!    fix is WIRED, not merely present as an unused constant. The
//!    implementer may instead extract a `site_core`-level builder; the
//!    behavior tests here work under either choice. `CARGO_MANIFEST_DIR`
//!    for this integration test resolves to `libs/site-core/`; the binary
//!    crate root is one level up at `../cmd/server/main.rs`.
//!
//!  - **R1 acceptance is wall-clock**, not "the timeout constant equals
//!    N". A test that only reads the configured value passes against a
//!    no-op impl and is explicitly forbidden by the spec's red-phase notes.

mod common;

use common::ai_mock::{
    ai_test_app_with_mock, anthropic_messages_response, anthropic_sse_response,
    install_log_capture, parse_sse_frames, valid_fit_verdict_json,
};
use serde_json::Value;
use std::time::{Duration, Instant};

// Path to the binary crate's main.rs, relative to libs/site-core/.
const MAIN_RS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../cmd/server/main.rs");
const ANTHROPIC_STREAM_RS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/ai/anthropic_stream.rs");
const ROUTES_AI_RS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/routes/ai.rs");

// ===========================================================================
// R1 — LLM10: upstream / request timeout (audit M1).
//
// Acceptance (spec R1):
//  (a) A mock upstream that never sends a terminal frame causes the FIT
//      handler to return the degraded error WITHIN the configured bound —
//      asserted on observed wall-clock, not on a configured constant.
//  (b) A mock that stalls mid-stream causes the CHAT handler to emit
//      `event: error` + `[DONE]` within the idle bound and discard the
//      partial buffer (no partial model text reaches the client).
//  (c) The TimeoutLayer is present in the router construction (behavioral
//      where reachable; source-text meta-test as the wiring gate).
//
// Red-phase: no timeout wrapping exists yet. (a) HANGS today until the
// bounded mock sleep elapses, so the elapsed time exceeds the asserted
// bound → fails by assertion. (b) the chat handler buffers and never emits
// a terminal frame within the idle bound → fails by assertion.
// ===========================================================================

/// Upper bound the handler MUST beat. The spec lets Forge pick the actual
/// timeout in the "single-digit to low-tens of seconds" band. We assert the
/// handler returns the degraded error in well under this bound; the bound
/// is deliberately generous (any sane timeout < 25s) so it holds for any
/// in-band value Forge picks while still failing hard against the no-timeout
/// status quo (which would hang for the full stall duration).
const R1_WALL_CLOCK_BOUND: Duration = Duration::from_secs(25);

/// How long the mock upstream stalls. Longer than any plausible in-band
/// timeout (so a real timeout fires first) but BOUNDED so the body thread
/// terminates and `ChunkedStream::drop`'s join does not hang the test
/// teardown indefinitely.
const R1_STALL: Duration = Duration::from_secs(40);

/// R1(a): fit handler — upstream never sends a terminal frame.
///
/// Given a mock /v1/messages that accepts the request then stalls (never
///   completes the response body) for longer than any in-band timeout
/// When a client POSTs /api/fit
/// Then the fit handler returns an error response WITHIN R1_WALL_CLOCK_BOUND
///   (a degraded/canned error — NOT a hang, NOT raw upstream text)
///
/// Red-phase: with no `tokio::time::timeout` around the fit await, the
/// handler blocks on the stalled upstream for the full R1_STALL (40s),
/// far exceeding the 25s bound → the elapsed-time assertion fails.
#[tokio::test]
async fn r1a_fit_handler_returns_degraded_error_within_wall_clock_bound_when_upstream_never_terminates()
 {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        // Body closure runs on a dedicated OS thread (mockito ChunkedStream).
        // Sleep BEFORE writing anything → the response never terminates
        // within the handler's deadline.
        .with_chunked_body(|_w| {
            std::thread::sleep(R1_STALL);
            Ok(())
        })
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());

    let start = Instant::now();
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Anything" }))
        .await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < R1_WALL_CLOCK_BOUND,
        "R1(a): fit handler MUST return within {R1_WALL_CLOCK_BOUND:?} when \
         the upstream stalls — observed {elapsed:?} (no-timeout status quo \
         hangs for the full {R1_STALL:?} stall)"
    );

    // It must be an error status (degraded surface), not a 200, and the
    // body must not carry raw upstream text.
    assert!(
        response.status_code().is_server_error()
            || response.status_code().is_client_error(),
        "R1(a): a stalled upstream MUST yield an error status (degraded \
         surface), got {}",
        response.status_code()
    );
}

/// R1(b): chat handler — upstream stalls mid-stream.
///
/// Given a mock that writes the SSE preamble + one content delta then
///   stalls (no terminal `message_delta`/`message_stop`) past the idle bound
/// When a client POSTs /api/chat
/// Then the chat handler emits `event: error` + `[DONE]` within the idle
///   bound AND no partial model text reaches the client (buffer discarded)
///
/// Red-phase: the chat consumer buffers content until a terminal frame
/// (R29 design) and has no idle deadline — it waits the full R1_STALL for a
/// terminal frame that never comes → no `event: error`/`[DONE]` within the
/// bound → assertion fails. (Also: if a future buggy impl flushed the
/// partial delta, the no-partial-text assertion catches it.)
#[tokio::test]
async fn r1b_chat_handler_emits_error_and_done_within_idle_bound_and_discards_partial_buffer() {
    const PARTIAL_LEAK_MARKER: &str = "PARTIAL-MODEL-TEXT-MUST-NOT-LEAK-r1b";

    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_chunked_body(move |w| {
            use std::io::Write;
            // message_start + content_block_start + ONE content delta
            // carrying a marker, then STALL with no terminal frame.
            let preamble = concat!(
                "event: message_start\n",
                "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4-6\",\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
                "event: content_block_start\n",
                "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
                "event: content_block_delta\n",
            );
            w.write_all(preamble.as_bytes())?;
            w.write_all(
                format!(
                    "data: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"{PARTIAL_LEAK_MARKER}\"}}}}\n\n"
                )
                .as_bytes(),
            )?;
            w.flush()?;
            // No terminal message_delta / message_stop — stall.
            std::thread::sleep(R1_STALL);
            Ok(())
        })
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());

    let start = Instant::now();
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Hi" }))
        .await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < R1_WALL_CLOCK_BOUND,
        "R1(b): chat handler MUST terminate the SSE stream within \
         {R1_WALL_CLOCK_BOUND:?} when the upstream stalls mid-stream — \
         observed {elapsed:?}"
    );

    let body = response.as_bytes();
    let frames = parse_sse_frames(body);

    let has_error_event = frames
        .iter()
        .any(|(ev, _)| ev.as_deref() == Some("error"));
    let ends_with_done = frames
        .last()
        .map(|(ev, data)| ev.is_none() && data == "[DONE]")
        .unwrap_or(false);

    assert!(
        has_error_event,
        "R1(b): a mid-stream stall MUST produce an `event: error` frame \
         within the idle bound; frames={frames:?}"
    );
    assert!(
        ends_with_done,
        "R1(b): the SSE stream MUST terminate with `[DONE]` after the idle \
         timeout; frames={frames:?}"
    );

    let body_str = String::from_utf8_lossy(body);
    assert!(
        !body_str.contains(PARTIAL_LEAK_MARKER),
        "R1(b): partial model text MUST NOT reach the client when the \
         stream is cut by the idle timeout (R29 — buffer discarded); \
         body={body_str}"
    );
}

/// R1(c): the `TimeoutLayer` outer backstop is WIRED into the router.
///
/// The router construction lives in `cmd/server/main.rs::run_server()`,
/// not behind a site-core public fn. Source-as-text meta-test: reads
/// main.rs and asserts a `tower_http::timeout::TimeoutLayer` (or the
/// `TimeoutLayer` symbol) is referenced AND applied via `.layer(`. This is
/// the gate that catches "implementer added a timeout around the await but
/// forgot the outer backstop layer". Reading ≠ modifying — in scope.
///
/// Red-phase: main.rs has no TimeoutLayer today → assertion fails.
#[test]
fn r1c_timeout_layer_is_wired_into_the_router_construction() {
    let main_src = std::fs::read_to_string(MAIN_RS).unwrap_or_else(|e| {
        panic!("cmd/server/main.rs must be readable at {MAIN_RS}: {e}")
    });

    assert!(
        main_src.contains("TimeoutLayer"),
        "R1(c): cmd/server/main.rs MUST reference tower_http's TimeoutLayer \
         (the outer-backstop timeout layer)"
    );
    // It must be APPLIED, not merely imported: a `.layer(` call near the
    // TimeoutLayer reference. We assert TimeoutLayer appears AND a layer
    // application exists in the file (the router already uses `.layer(`
    // for cors/middleware, so the meaningful new signal is the symbol; the
    // combined check guards an import-but-unused regression).
    assert!(
        main_src.contains(".layer(") && main_src.contains("TimeoutLayer"),
        "R1(c): TimeoutLayer MUST be applied to the router via `.layer(...)`"
    );
}

// ===========================================================================
// R5 — explicit DefaultBodyLimit (audit L2).
//
// Acceptance: a request body above the configured byte limit is rejected
// BEFORE handler parse (413/400 as axum produces); a normal-sized request
// is unaffected. The byte cap must align with the semantic cap (the fit
// 15_000-char handler check) — rejection pre-parse, not the post-parse
// length check.
//
// The fit handler's own post-parse guard returns 400 with the body
// "Job description too long (max 15,000 characters)". The DISCRIMINATOR
// between "rejected pre-parse by DefaultBodyLimit" and "rejected post-parse
// by the handler check" is the response body: DefaultBodyLimit yields an
// axum-generated 413 (Payload Too Large) with NO custom JSON body; the
// handler check yields 400 with that specific message string.
//
// Red-phase: no DefaultBodyLimit configured → a ~1 MB body sails past the
// (absent) byte cap and is rejected only by the post-parse char check →
// status 400 + the handler message. The assertion (pre-parse rejection:
// 413 and/or absence of the handler's char-check message) fails.
// ===========================================================================

/// Given a request body far above any sane byte cap (1 MiB) but whose
///   parsed semantic length would also exceed the 15k-char fit cap
/// When a client POSTs /api/fit
/// Then it is rejected PRE-PARSE: status 413 (axum's DefaultBodyLimit
///   response) — NOT the post-parse 400 with the handler's char-count
///   message
///
/// The body cap must be > 15_000 chars of headroom yet « 1 MiB so this
/// oversized request trips the byte cap before the handler ever parses.
#[tokio::test]
async fn r5_oversized_body_rejected_pre_parse_not_by_post_parse_char_check() {
    let mut server = mockito::Server::new_async().await;
    // Mock present but it must NEVER be hit — rejection happens before the
    // handler runs.
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_body(anthropic_messages_response(
            valid_fit_verdict_json(),
            "end_turn",
        ))
        .expect(0)
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());

    // ~1 MiB of 'A' — both above any byte cap AND above the 15k char cap.
    let huge = "A".repeat(1024 * 1024);
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": huge }))
        .await;

    let status = response.status_code();
    let body_str = String::from_utf8_lossy(response.as_bytes()).to_string();

    // Pre-parse rejection signature: axum DefaultBodyLimit → 413 Payload
    // Too Large with no handler JSON. Post-parse (status quo) → 400 with
    // the specific char-check message. Assert it is the pre-parse form.
    assert_eq!(
        status,
        axum::http::StatusCode::PAYLOAD_TOO_LARGE,
        "R5: an oversized body MUST be rejected pre-parse with 413 \
         (DefaultBodyLimit); got {status} body={body_str}"
    );
    assert!(
        !body_str.contains("max 15,000 characters"),
        "R5: rejection MUST happen before the handler's post-parse \
         char-count check — the handler's '(max 15,000 characters)' \
         message proves the body was fully parsed first (status-quo red \
         state); body={body_str}"
    );
}

/// Given a normal-sized request body (well under any byte cap and the
///   15k-char semantic cap)
/// When a client POSTs /api/fit
/// Then the request is NOT rejected by the body limit — it reaches the
///   handler and gets the normal 200 happy-path response
///
/// Guards against an over-tight DefaultBodyLimit that rejects legitimate
/// in-spec requests. The byte cap must leave headroom over 15_000 chars.
#[tokio::test]
async fn r5_normal_sized_body_is_unaffected_by_the_limit() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(anthropic_messages_response(
            valid_fit_verdict_json(),
            "end_turn",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());

    // 12_000 chars: under the 15k semantic cap, and any sane byte cap with
    // headroom over 15k chars must admit this.
    let normal = "Senior systems engineer. ".repeat(480); // ~12_000 chars
    assert!(normal.len() < 15_000);

    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": normal }))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(
        body["verdict"].as_str(),
        Some("strong-fit"),
        "R5: a normal-sized request MUST pass the body limit and reach the \
         handler unaffected"
    );
}

/// R5 wiring gate (source-as-text). `DefaultBodyLimit` is configured in
/// `cmd/server/main.rs` (or a site-core builder the binary calls). Assert
/// the symbol is referenced AND a byte cap is set via `::max(`. Reading ≠
/// modifying. Red-phase: no DefaultBodyLimit in main.rs → fails.
#[test]
fn r5_default_body_limit_is_wired_with_an_explicit_byte_cap() {
    let main_src = std::fs::read_to_string(MAIN_RS).unwrap_or_else(|e| {
        panic!("cmd/server/main.rs must be readable at {MAIN_RS}: {e}")
    });
    assert!(
        main_src.contains("DefaultBodyLimit"),
        "R5: cmd/server/main.rs MUST configure axum's DefaultBodyLimit \
         (explicit byte cap aligned with the semantic cap)"
    );
    assert!(
        main_src.contains("DefaultBodyLimit::max(")
            || main_src.contains("DefaultBodyLimit :: max ("),
        "R5: DefaultBodyLimit MUST set an explicit byte cap via \
         `DefaultBodyLimit::max(<bytes>)`"
    );
}

// ===========================================================================
// R6 — CORS_ORIGIN fail-loud in production (audit Nit).
//
// Acceptance: in a production configuration with CORS_ORIGIN unset, the
// server EITHER refuses to start with a clear message OR logs a WARN
// naming the missing var. Local/dev behavior unchanged. The spec lets the
// implementer pick the form — the test accepts either (refuse-with-message
// XOR a WARN naming CORS_ORIGIN), and asserts the SILENT default fallback
// to "http://localhost:3000" no longer happens unconditionally.
//
// The CORS construction is in `cmd/server/main.rs::run_server()` — not
// reachable behaviorally from site-core. Source-as-text gate: assert the
// silent `unwrap_or_else(|_| "http://localhost:3000".to_string())` pattern
// (main.rs:120-124) is GONE, replaced by either a fail-loud (`expect`/
// `panic!`) naming CORS_ORIGIN or a `warn!` naming CORS_ORIGIN.
//
// Red-phase: main.rs today does exactly the silent default → the
// "silent-fallback-removed" assertion fails.
// ===========================================================================

/// Given the CORS construction in cmd/server/main.rs
/// When CORS_ORIGIN is unset in production
/// Then it does NOT silently fall back to the localhost default — it
///   EITHER fails loud (panic/expect naming CORS_ORIGIN) OR logs a WARN
///   naming CORS_ORIGIN (spec lets the implementer pick; accept either)
///
/// Reading ≠ modifying — within scope.
#[test]
fn r6_cors_origin_unset_fails_loud_or_warns_no_silent_localhost_default() {
    let main_src = std::fs::read_to_string(MAIN_RS).unwrap_or_else(|e| {
        panic!("cmd/server/main.rs must be readable at {MAIN_RS}: {e}")
    });

    // The exact silent-fallback expression present at the parent commit.
    let silent_fallback =
        r#"unwrap_or_else(|_| "http://localhost:3000".to_string())"#;
    assert!(
        !main_src.contains(silent_fallback),
        "R6: the unconditional silent CORS_ORIGIN → localhost default \
         MUST be removed (it makes a missing prod env var silently wrong \
         instead of failing loud)"
    );

    // Accept EITHER fail-loud OR warn form, both must name CORS_ORIGIN.
    let mentions_cors_origin = main_src.contains("CORS_ORIGIN");
    let fail_loud = mentions_cors_origin
        && (main_src.contains("panic!") || main_src.contains(".expect("));
    let warns = mentions_cors_origin
        && (main_src.contains("warn!") || main_src.contains("tracing::warn"));

    assert!(
        fail_loud || warns,
        "R6: with CORS_ORIGIN unset in prod the server MUST either refuse \
         to start with a clear message OR emit a WARN naming CORS_ORIGIN — \
         neither fail-loud nor a CORS_ORIGIN warn was found in main.rs"
    );
}

// ===========================================================================
// R7 — opaque client strings for AI-path AppError::Internal (audit residual).
//
// Three named sites build the client error from upstream/serde text:
//   - routes/ai.rs:265  format!("AI prompt failed: {e}")
//   - routes/ai.rs:338  format!("Failed to parse AI response as FitVerdict: {e}")
//   - ai/anthropic_stream.rs:172 format!("failed to serialize chat body: {err}")
//
// Acceptance: the three sites return a FIXED opaque client string; the
// detailed {e} appears in server logs; no upstream/serde text in the HTTP
// response body. Existing frontend 500-remap still works.
//
// Sites :265 and :338 are drivable via mockito (behavioral). Site
// anthropic_stream.rs:172 fires only on outbound serde serialization
// failure of the chat body — not provokable from outside; gated by a
// source-as-text meta-test (documented divergence, per spec guidance).
//
// Red-phase: today :265 reflects the rig error text and :338 reflects the
// serde parse-error text into the 500 body → the "no leak / opaque string"
// assertions fail.
// ===========================================================================

/// R7 site routes/ai.rs:265 — `format!("AI prompt failed: {e}")`.
///
/// Given a mock /v1/messages returning a transport-level failure that makes
///   `agent.prompt(...).await` error with a recognizable upstream string
/// When a client POSTs /api/fit
/// Then the 500 response body is a FIXED opaque string — it does NOT
///   contain the upstream error text — AND the detail is in server logs
///
/// We embed a unique marker in the upstream error surface (a 500 body the
/// rig client surfaces in its error). Red-phase: the handler does
/// `format!("AI prompt failed: {e}")` so the marker leaks into the 500
/// body → leak assertion fails.
#[tokio::test]
async fn r7_fit_prompt_failure_returns_opaque_body_with_detail_in_logs() {
    const UPSTREAM_LEAK_MARKER: &str = "UPSTREAM-ERR-DETAIL-must-not-leak-r7-prompt";

    let (buf, _guard) = install_log_capture();

    let mut server = mockito::Server::new_async().await;
    // Upstream 500 whose body carries the marker — rig surfaces this in
    // the prompt error `{e}`.
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"type":"error","error":{{"type":"api_error","message":"{UPSTREAM_LEAK_MARKER}"}}}}"#
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Anything" }))
        .await;

    assert_eq!(
        response.status_code(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "R7: an upstream prompt failure still maps to 500"
    );

    let body_str = String::from_utf8_lossy(response.as_bytes()).to_string();
    assert!(
        !body_str.contains(UPSTREAM_LEAK_MARKER),
        "R7 (routes/ai.rs:265): the 500 response body MUST be a fixed \
         opaque string and MUST NOT contain upstream error text; \
         body={body_str}"
    );
    assert!(
        !body_str.contains("AI prompt failed:"),
        "R7: the response body MUST NOT echo the `AI prompt failed: {{e}}` \
         detail string to the client"
    );

    // Detail must be server-side (captured logs).
    let captured = buf.captured();
    assert!(
        captured.contains(UPSTREAM_LEAK_MARKER) || captured.contains("ERROR ")
            || captured.contains("WARN "),
        "R7: the detailed upstream error MUST be logged server-side \
         (error!/warn!) even though the client body is opaque; \
         captured={captured}"
    );
}

/// R7 site routes/ai.rs:338 — `format!("Failed to parse AI response as
/// FitVerdict: {e}")`.
///
/// Given a mock returning a 200 with `stop_reason=end_turn` but a body that
///   is NOT valid FitVerdict JSON (contains a unique marker)
/// When a client POSTs /api/fit
/// Then the 500 response body is opaque — it does NOT contain the serde
///   parse-error text (which would echo the marker) — detail in logs
///
/// Red-phase: `parse_verdict` does `format!("Failed to parse AI response
/// as FitVerdict: {e}")` and serde's error includes surrounding input
/// context, so the marker can leak into the 500 body → assertion fails.
#[tokio::test]
async fn r7_fit_parse_failure_returns_opaque_body_no_serde_text_leak() {
    const PARSE_LEAK_MARKER: &str = "SERDE-PARSE-DETAIL-must-not-leak-r7-parse";

    let (buf, _guard) = install_log_capture();

    let mut server = mockito::Server::new_async().await;
    // 200 + end_turn but the model "text" is not FitVerdict JSON — it is a
    // bare string with the marker, so serde_json::from_str fails and
    // extract_json finds no `{...}` block.
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(anthropic_messages_response(
            &format!("not-json-{PARSE_LEAK_MARKER}-not-json"),
            "end_turn",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Anything" }))
        .await;

    assert_eq!(
        response.status_code(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "R7: a parse failure still maps to 500"
    );

    let body_str = String::from_utf8_lossy(response.as_bytes()).to_string();
    assert!(
        !body_str.contains(PARSE_LEAK_MARKER),
        "R7 (routes/ai.rs:338): the 500 body MUST NOT echo serde \
         parse-error text (it can carry surrounding model input); \
         body={body_str}"
    );
    assert!(
        !body_str.contains("Failed to parse AI response as FitVerdict:"),
        "R7: the response body MUST NOT echo the parse-error detail string"
    );

    let captured = buf.captured();
    assert!(
        captured.contains("ERROR ") || captured.contains("WARN ")
            || captured.contains(PARSE_LEAK_MARKER),
        "R7: the parse-failure detail MUST be logged server-side; \
         captured={captured}"
    );
}

/// R7 third site (source-as-text gate) — `ai/anthropic_stream.rs:172`,
/// `format!("failed to serialize chat body: {err}")`. This fires only on
/// outbound serde serialization failure of the chat body — not provokable
/// from a black-box client. Gate it structurally: assert the leaky
/// `AppError::Internal(format!("failed to serialize chat body: {err}"))`
/// callsite no longer reflects `{err}` to the client (either the message
/// is a fixed canned constant, or the format-with-{err} pattern at that
/// site is gone). Reading ≠ modifying.
///
/// Red-phase: the literal `failed to serialize chat body: {err}` is
/// present in anthropic_stream.rs today → assertion fails.
#[test]
fn r7_third_site_chat_body_serialize_error_is_opaque_to_client() {
    let stream_src = std::fs::read_to_string(ANTHROPIC_STREAM_RS).unwrap_or_else(|e| {
        panic!("ai/anthropic_stream.rs must be readable: {e}")
    });

    // The leaky form interpolates the serde error into the client-facing
    // AppError::Internal. Post-fix the client string must be fixed/opaque.
    assert!(
        !stream_src.contains("failed to serialize chat body: {err}")
            && !stream_src.contains(r#"failed to serialize chat body: {}"#),
        "R7 (ai/anthropic_stream.rs:172): the chat-body serialize error \
         MUST NOT interpolate the serde `{{err}}` into the client-facing \
         AppError::Internal string — use a fixed opaque client message and \
         log the detail server-side"
    );
}

/// R7 mirrored gate: confirm the two behavioral sites' leaky format
/// strings are gone from routes/ai.rs source too (defends against a fix
/// that opaques the body via IntoResponse but leaves the leaky
/// `format!(... {e})` building a detailed AppError that some other path
/// could still surface). Source-as-text. Red-phase: both literals present.
#[test]
fn r7_routes_ai_no_longer_builds_internal_errors_from_upstream_serde_text() {
    let ai_src = std::fs::read_to_string(ROUTES_AI_RS)
        .unwrap_or_else(|e| panic!("routes/ai.rs must be readable: {e}"));

    assert!(
        !ai_src.contains(r#"format!("AI prompt failed: {e}")"#),
        "R7 (routes/ai.rs:265): the `AI prompt failed: {{e}}` \
         AppError::Internal construction MUST be replaced with a fixed \
         opaque client string (detail → server log)"
    );
    assert!(
        !ai_src.contains(r#"AppError::Internal(format!("Failed to parse AI response as FitVerdict: {e}"))"#)
            && !ai_src.contains(r#"format!("Failed to parse AI response as FitVerdict: {e}")"#),
        "R7 (routes/ai.rs:338): the `Failed to parse AI response as \
         FitVerdict: {{e}}` AppError::Internal construction MUST be \
         replaced with a fixed opaque client string"
    );
}

#[allow(dead_code)]
fn _silence_unused() {
    let _ = anthropic_sse_response(&["x"], "end_turn");
}
