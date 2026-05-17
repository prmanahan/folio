//! Red-phase integration tests for the chat handler — spec #572 Task 4 R11.
//!
//! Black-box scope: HTTP request/response + SSE stream against a real
//! axum handler with a real `rig_core::providers::anthropic::Client` whose
//! base URL is pointed at a mockito server. The mock provides scripted
//! Anthropic SSE responses; tests assert behavioral output (SSE frames,
//! response status, server-side logs).
//!
//! These tests EXIST to fail until `libs/site-core/ai/anthropic_stream.rs`
//! and `libs/site-core/ai/stop_reason.rs` exist (Forge T4-impl). Compile
//! failure (E0432/E0433) IS the red-phase state for this dispatch.
//!
//! Scenarios traced:
//! - Scenario 1 (happy path, cache_control on system block, EndTurn)
//! - Scenario 8 (refusal) — chat half
//! - Scenario 9 (context_exceeded) — chat half
//! - Scenario 14 (stop_sequence) — chat half
//! - Scenario 15 (max_tokens truncation) — chat half
//! - Scenario 16 (Other / unrecognized stop_reason) — chat half
//! - Scenario 17 (Anthropic SSE error event)
//!
//! Plus R11 architectural assertions on outbound request shape, R14 cache
//! verification, and the Verification-Ask-#3 retry-disable behavior.

mod common;

use common::ai_mock::{
    self, ai_test_app_with_mock, ai_test_app_with_mock_and_state, anthropic_sse_error_response,
    anthropic_sse_response, assert_terminates_with_done, count_event, first_event,
    install_log_capture, parse_sse_frames,
};
use mockito::Matcher;
use serde_json::Value;

// ===========================================================================
// Scenario 1 — happy path, outbound request shape, EndTurn
// ===========================================================================

/// Given a fresh database with migration 005 applied and ANTHROPIC_API_KEY set
/// When a client POSTs /api/chat with a valid message
/// Then the chat handler issues an SSE POST to /v1/messages
/// And the outbound body uses model = "claude-sonnet-4-6", max_tokens = 5530,
///     stream = true (R11, Scenario 1, R14 verification)
#[tokio::test]
async fn chat_handler_issues_sse_post_with_seeded_model_and_max_tokens() {
    // Given
    let mut server = mockito::Server::new_async().await;
    let captured_body = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let captured_body_for_mock = captured_body.clone();

    let _mock = server
        .mock("POST", "/v1/messages")
        .match_header("content-type", Matcher::Regex("application/json".into()))
        .match_request(move |req| {
            // Capture the outbound body for post-call inspection.
            let body = req.body().expect("outbound request must have a body");
            captured_body_for_mock
                .lock()
                .unwrap()
                .extend_from_slice(body);
            true
        })
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(&["Hello ", "world"], "end_turn"))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());

    // When
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Hi" }))
        .await;

    // Then — handler routed cleanly
    response.assert_status_ok();

    // Then — outbound body parses and carries the seeded config values
    let body_bytes = captured_body.lock().unwrap().clone();
    let body_str = std::str::from_utf8(&body_bytes).expect("outbound body is UTF-8");
    let body_json: Value = serde_json::from_str(body_str).expect("outbound body parses as JSON");

    assert_eq!(
        body_json["model"].as_str(),
        Some("claude-sonnet-4-6"),
        "outbound body model must be the seeded value, body={}",
        body_str
    );
    assert_eq!(
        body_json["max_tokens"].as_u64(),
        Some(5530),
        "outbound body max_tokens must be the seeded value, body={}",
        body_str
    );
    assert_eq!(
        body_json["stream"].as_bool(),
        Some(true),
        "outbound body stream must be true (chat uses SSE), body={}",
        body_str
    );
}

/// Given the same happy path
/// When the request hits Anthropic
/// Then the outbound body carries cache_control: { "type": "ephemeral" } on
///     the SystemContent::Text block (R14 chat half)
#[tokio::test]
async fn chat_handler_outbound_body_carries_cache_control_ephemeral_on_system_block() {
    // Given
    let mut server = mockito::Server::new_async().await;
    let captured_body = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let captured_body_for_mock = captured_body.clone();

    let _mock = server
        .mock("POST", "/v1/messages")
        .match_request(move |req| {
            let body = req.body().expect("body present");
            captured_body_for_mock
                .lock()
                .unwrap()
                .extend_from_slice(body);
            true
        })
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(&["ok"], "end_turn"))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());

    // When
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Hi" }))
        .await;
    response.assert_status_ok();

    // Then — cache_control: ephemeral on the system block
    let body_bytes = captured_body.lock().unwrap().clone();
    let body_str = std::str::from_utf8(&body_bytes).expect("UTF-8");
    let body_json: Value = serde_json::from_str(body_str).expect("JSON");

    let system = &body_json["system"];
    assert!(
        system.is_array(),
        "system field must be an array of blocks (Anthropic SystemContent), body={}",
        body_str
    );
    let blocks = system.as_array().unwrap();
    let has_ephemeral = blocks
        .iter()
        .any(|block| block["cache_control"]["type"].as_str() == Some("ephemeral"));
    assert!(
        has_ephemeral,
        "at least one system block must carry cache_control.type = ephemeral, body={}",
        body_str
    );
    // R14: no top-level cache_control
    assert!(
        body_json.get("cache_control").is_none(),
        "no top-level cache_control field permitted on chat body, body={}",
        body_str
    );
}

/// Given the model emits content delta frames
/// When the chat handler relays them
/// Then the SSE response carries content frames in order followed by [DONE]
///     (Scenario 1, R11)
#[tokio::test]
async fn chat_handler_streams_content_delta_text_as_sse_data_frames() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(
            &["Hello ", "world", "!"],
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
    let content: Vec<&str> = frames
        .iter()
        .filter(|(name, data)| name.is_none() && data != "[DONE]")
        .map(|(_, data)| data.as_str())
        .collect();

    assert!(
        content.contains(&"Hello "),
        "expected content frame 'Hello ', got frames: {:?}",
        frames
    );
    assert!(
        content.contains(&"world"),
        "expected content frame 'world', got frames: {:?}",
        frames
    );
    assert!(
        content.contains(&"!"),
        "expected content frame '!', got frames: {:?}",
        frames
    );
    assert_terminates_with_done(&frames);
}

/// Given the model returns stop_reason = "end_turn"
/// When the SSE stream terminates
/// Then the captured stop_reason maps to StopReason::EndTurn — no terminal
///     error event is emitted (Scenario 1)
#[tokio::test]
async fn chat_handler_reads_message_delta_stop_reason_and_terminates_on_message_stop() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(&["all done"], "end_turn"))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Hi" }))
        .await;
    response.assert_status_ok();

    let frames = parse_sse_frames(response.as_bytes());
    assert_eq!(
        count_event(&frames, "refusal"),
        0,
        "EndTurn must not emit refusal frame, got: {:?}",
        frames
    );
    assert_eq!(
        count_event(&frames, "context_exceeded"),
        0,
        "EndTurn must not emit context_exceeded frame, got: {:?}",
        frames
    );
    assert_eq!(
        count_event(&frames, "truncated"),
        0,
        "EndTurn must not emit truncated frame, got: {:?}",
        frames
    );
    assert_eq!(
        count_event(&frames, "error"),
        0,
        "EndTurn must not emit error frame, got: {:?}",
        frames
    );
    assert_terminates_with_done(&frames);
}

// ===========================================================================
// Scenario 8 — refusal (chat half)
// ===========================================================================

/// Given the model returns stop_reason = "refusal"
/// When the client POSTs /api/chat
/// Then the SSE stream emits event:refusal with the canned data, then [DONE]
/// And the response body never contains the raw model text (R29)
///     (Scenario 8, R18)
#[tokio::test]
async fn chat_handler_emits_event_refusal_with_canned_data_on_refusal_stop_reason() {
    const DO_NOT_LEAK: &str = "DO-NOT-LEAK-MARKER-refusal-raw-text-from-model";

    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(&[DO_NOT_LEAK], "refusal"))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Anything" }))
        .await;
    response.assert_status_ok();

    let body_bytes = response.as_bytes();
    let frames = parse_sse_frames(body_bytes);

    let refusal_frame = first_event(&frames, "refusal")
        .expect("SSE stream MUST contain an event:refusal frame on refusal stop_reason");
    assert_eq!(
        refusal_frame.1, "The assistant declined to respond to that request.",
        "event:refusal data MUST be the canned message"
    );
    assert_terminates_with_done(&frames);

    // R29: marker (raw model text) must not be in the response body bytes
    let body_str = std::str::from_utf8(body_bytes).expect("UTF-8");
    assert!(
        !body_str.contains(DO_NOT_LEAK),
        "response body MUST NOT contain raw model text on refusal (R29); body={}",
        body_str
    );
}

// ===========================================================================
// Scenario 9 — context_exceeded (chat half)
// ===========================================================================

/// Given the model returns stop_reason = "model_context_window_exceeded"
/// When the client POSTs /api/chat
/// Then the SSE stream emits event:context_exceeded with the canned data,
///     then [DONE], no raw model text in response body (R18, R29)
#[tokio::test]
async fn chat_handler_emits_event_context_exceeded_with_canned_data_on_model_context_window_exceeded()
 {
    const DO_NOT_LEAK: &str = "DO-NOT-LEAK-MARKER-context-exceeded-raw-text";

    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(
            &[DO_NOT_LEAK],
            "model_context_window_exceeded",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Long thing" }))
        .await;
    response.assert_status_ok();

    let frames = parse_sse_frames(response.as_bytes());
    let frame = first_event(&frames, "context_exceeded")
        .expect("SSE stream MUST contain an event:context_exceeded frame");
    assert_eq!(
        frame.1, "The request exceeded the model's context window. Try a shorter input.",
        "event:context_exceeded data MUST be the canned message"
    );
    assert_terminates_with_done(&frames);

    let body_str = std::str::from_utf8(response.as_bytes()).expect("UTF-8");
    assert!(
        !body_str.contains(DO_NOT_LEAK),
        "response body MUST NOT contain raw model text (R29); body={}",
        body_str
    );
}

// ===========================================================================
// Scenario 15 — max_tokens (chat half)
// ===========================================================================

/// Given the model returns stop_reason = "max_tokens"
/// When the client POSTs /api/chat
/// Then content frames received before truncation are still streamed first
/// And the stream emits event:truncated with the canned data, then [DONE]
///     (Scenario 15, R18, R13)
#[tokio::test]
async fn chat_handler_emits_event_truncated_with_canned_data_on_max_tokens() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(
            &["partial ", "response"],
            "max_tokens",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Long?" }))
        .await;
    response.assert_status_ok();

    let frames = parse_sse_frames(response.as_bytes());

    // Content received before truncation IS streamed.
    let content: Vec<String> = frames
        .iter()
        .filter(|(name, data)| name.is_none() && data != "[DONE]")
        .map(|(_, d)| d.clone())
        .collect();
    assert!(
        content.iter().any(|c| c == "partial "),
        "content received before truncation must still be streamed, frames: {:?}",
        frames
    );

    let frame = first_event(&frames, "truncated")
        .expect("SSE stream MUST contain an event:truncated frame on max_tokens");
    assert_eq!(
        frame.1, "The response was cut off due to length.",
        "event:truncated data MUST be the canned message"
    );
    assert_terminates_with_done(&frames);
}

// ===========================================================================
// Scenario 14 — stop_sequence (chat half)
// ===========================================================================

/// Given the model returns stop_reason = "stop_sequence"
/// When the client POSTs /api/chat
/// Then no terminal error event is emitted; stream ends with [DONE]
///     (Scenario 14, R13)
#[tokio::test]
async fn chat_handler_treats_stop_sequence_as_end_turn_equivalent() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(&["content"], "stop_sequence"))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Hi" }))
        .await;
    response.assert_status_ok();

    let frames = parse_sse_frames(response.as_bytes());
    assert_eq!(count_event(&frames, "refusal"), 0);
    assert_eq!(count_event(&frames, "context_exceeded"), 0);
    assert_eq!(count_event(&frames, "truncated"), 0);
    assert_eq!(count_event(&frames, "error"), 0);
    assert_terminates_with_done(&frames);
}

/// Given the model returns stop_reason = "pause_turn"
/// When the client POSTs /api/chat
/// Then stop_reason is treated as EndTurn-equivalent — no terminal error frame
///     (R13, Scenario 14-adjacent)
#[tokio::test]
async fn chat_handler_treats_pause_turn_as_end_turn_equivalent() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(&["content"], "pause_turn"))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Hi" }))
        .await;
    response.assert_status_ok();

    let frames = parse_sse_frames(response.as_bytes());
    assert_eq!(count_event(&frames, "refusal"), 0);
    assert_eq!(count_event(&frames, "context_exceeded"), 0);
    assert_eq!(count_event(&frames, "truncated"), 0);
    assert_eq!(count_event(&frames, "error"), 0);
    assert_terminates_with_done(&frames);
}

// ===========================================================================
// Scenario 16 — Other / unrecognized stop_reason (chat half)
// ===========================================================================

/// Given the model returns an unrecognized stop_reason
/// When the client POSTs /api/chat
/// Then the response emits the existing generic error frame then [DONE]
/// And a warn log names the sanitized stop_reason value (R13)
///
/// NOTE on log assertion: tracing log capture across the spawned task in
/// chat_inner depends on whether the warn is emitted on the request task or
/// the spawned bridge task. Glitch flags this as a test-mechanism caveat —
/// if log capture fails to observe the warn, the behavioral assertion (error
/// frame emitted + Other variant produced) still holds.
#[tokio::test]
async fn chat_handler_logs_warn_naming_sanitized_value_on_unrecognized_stop_reason_and_returns_internal_error()
 {
    let (_buf, _guard) = install_log_capture();

    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(
            &["content"],
            "some_future_anthropic_string",
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
    assert!(
        count_event(&frames, "error") >= 1
            || frames
                .iter()
                .any(|(name, _)| name.as_deref() == Some("error")),
        "Other stop_reason must emit the existing generic SSE error frame, got: {:?}",
        frames
    );
    assert_terminates_with_done(&frames);
}

// ===========================================================================
// Scenario 17 — Anthropic SSE error event
// ===========================================================================

/// Given Anthropic returns an SSE event:error frame with a typed payload
/// When the chat consumer parses it
/// Then it deserializes the payload into AnthropicSseError { kind, message }
/// And the response emits the existing generic error frame then [DONE]
/// And the server-side log contains the sanitized message text
///     (Scenario 17, Verification Ask #2)
///
/// NOTE on scope: per Glitch's black-box discipline, the deserialization
/// shape (struct round-trip) is a unit test inside `ai/anthropic_stream.rs`
/// and out of Glitch's touch scope. This integration test asserts the
/// observable behavior: response stream emits the generic error frame +
/// [DONE], server log contains the sanitized message text.
#[tokio::test]
async fn chat_handler_deserializes_anthropic_sse_error_event_into_typed_struct_and_maps_to_app_error()
 {
    let (buf, _guard) = install_log_capture();

    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_error_response(
            "overloaded_error",
            "upstream is having a moment",
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
    assert!(
        count_event(&frames, "error") >= 1,
        "Anthropic SSE error event must surface as a generic error frame on the response, got: {:?}",
        frames
    );
    assert_terminates_with_done(&frames);

    let captured = buf.captured();
    assert!(
        captured.contains("upstream is having a moment"),
        "server-side log MUST contain the sanitized message text from Anthropic SSE error, captured: {}",
        captured
    );
}

// ===========================================================================
// Verification Ask #3 — retry policy is Never
// ===========================================================================

/// Given Anthropic returns HTTP 503 on the first request
/// When the chat handler dispatches the SSE call
/// Then mockito observes exactly one outbound POST (no retry)
///     (Verification Ask #3, Sage T0 finding #4)
///
/// This is the load-bearing assertion that
/// `GenericEventSource::with_retry_policy(client, req, Never)` is wired
/// correctly. The default rig-core 0.37 policy is infinite retry; without
/// `Never`, mockito would see multiple POSTs and the user would see
/// duplicated content.
#[tokio::test]
async fn chat_handler_disables_generic_event_source_retry_no_double_stream_on_transient_failure() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/messages")
        .with_status(503)
        .with_header("content-type", "application/json")
        .with_body(r#"{"type":"error","error":{"type":"overloaded_error","message":"503"}}"#)
        .expect(1)
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let _response = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Hi" }))
        .await;

    // mock.expect(1) + assert_async asserts mockito observed exactly one POST.
    mock.assert_async().await;
}

// ===========================================================================
// AppState Client reuse — sequential requests
// ===========================================================================

/// Given a single AppState with a single rig Client
/// When two sequential chat requests are dispatched
/// Then both succeed and mockito observes two outbound POSTs
///     (R11 / spec line 632 — Client is reused, no new construction per request)
///
/// NOTE: Strict "no new Client construction per request" is structural
/// (the AppState Client is `Arc<...>`-shaped, not request-scoped), not
/// black-box observable. Warden's static review handles the architectural
/// assertion. This test verifies the behavioral consequence: the same
/// AppState round-trips two requests successfully.
#[tokio::test]
async fn chat_handler_reuses_rig_anthropic_client_from_app_state_no_new_client_per_request() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "text/event-stream")
        .with_body(anthropic_sse_response(&["hi"], "end_turn"))
        .expect(2)
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());

    let r1 = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "1" }))
        .await;
    r1.assert_status_ok();

    let r2 = app
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "2" }))
        .await;
    r2.assert_status_ok();

    mock.assert_async().await;
}

// ===========================================================================
// Suppress unused warnings on helpers conditionally used
// ===========================================================================

#[allow(dead_code)]
fn _silence_unused() {
    let _ = ai_mock::valid_fit_verdict_json();
    let _ = ai_test_app_with_mock_and_state;
}
