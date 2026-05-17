//! Red-phase integration tests for the fit handler — spec #572 Task 4 R12.
//!
//! Black-box scope: HTTP request/response against a real axum handler
//! whose `rig_client` is pointed at a mockito server that returns
//! non-streaming Anthropic Messages API responses.
//!
//! Compile failure (E0432/E0433 missing modules / variants) IS the
//! red-phase state — `libs/site-core/ai/stop_reason.rs` (StopReasonCapture
//! hook), `AppError::Refusal`, `AppError::ContextExceeded`, and the
//! handler rewrite per R12 don't exist yet.
//!
//! Scenarios traced:
//! - Scenario 2 (happy path, cache_control via with_prompt_caching, EndTurn)
//! - Scenario 8 (refusal) — fit half: 422 + canned body
//! - Scenario 9 (context_exceeded) — fit half: 413 + canned body
//! - Scenario 14 (stop_sequence) — fit half: 200 + parsed verdict
//! - Scenario 15 (max_tokens) — fit half: warn + JSON parse
//! - Scenario 16 (Other / unrecognized) — fit half: 500

mod common;

use common::ai_mock::{
    self, ai_test_app_with_mock, anthropic_messages_response, install_log_capture,
    valid_fit_verdict_json,
};
use mockito::Matcher;
use serde_json::Value;

// ===========================================================================
// Scenario 2 — happy path, hook attached, outbound shape, EndTurn
// ===========================================================================

/// Given a fresh DB with migration 005 applied
/// When a client POSTs /api/fit with a valid job description
/// Then the fit handler calls agent.prompt(...).with_hook(StopReasonCapture)
///     (Verification Ask #1: with_hook is the rig-core 0.37 method)
/// And the response is 200 OK with the parsed FitVerdict
///     (Scenario 2, R12)
///
/// Black-box observation: a successful round-trip + an outbound POST to
/// /v1/messages. The hook attachment is a code-shape concern Warden's
/// static review owns; behaviorally, the only consequence is that the
/// handler's mapping logic runs against the captured stop_reason —
/// verified by the refusal/context-exceeded/max-tokens tests below
/// which all REQUIRE the hook to have captured stop_reason for the
/// branch to fire.
#[tokio::test]
async fn fit_handler_uses_agent_prompt_with_stop_reason_capture_hook() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(anthropic_messages_response(
            valid_fit_verdict_json(),
            "end_turn",
        ))
        .expect(1)
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());

    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({
            "job_description": "Senior systems engineer wanted, must love Rust."
        }))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["verdict"].as_str(), Some("strong-fit"));
    assert_eq!(body["headline"].as_str(), Some("Strong match"));
}

/// Given a fresh DB and the fit endpoint
/// When the request hits Anthropic
/// Then the outbound body carries cache_control: { "type": "ephemeral" } on
///     the system block per rig's apply_cache_control via with_prompt_caching()
///     (Scenario 2, R14 fit half)
#[tokio::test]
async fn fit_handler_outbound_body_carries_cache_control_ephemeral_on_system_block_via_with_prompt_caching()
 {
    let mut server = mockito::Server::new_async().await;
    let captured_body = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let captured_body_for_mock = captured_body.clone();

    let _mock = server
        .mock("POST", "/v1/messages")
        .match_header("content-type", Matcher::Regex("application/json".into()))
        .match_request(move |req| {
            let body = req.body().expect("body present");
            captured_body_for_mock
                .lock()
                .unwrap()
                .extend_from_slice(body);
            true
        })
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(anthropic_messages_response(
            valid_fit_verdict_json(),
            "end_turn",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Hi there" }))
        .await;
    response.assert_status_ok();

    let body_bytes = captured_body.lock().unwrap().clone();
    let body_str = std::str::from_utf8(&body_bytes).expect("UTF-8");
    let body_json: Value = serde_json::from_str(body_str).expect("outbound body is JSON");

    let system = &body_json["system"];
    assert!(
        system.is_array(),
        "system field must be an array of blocks, body={}",
        body_str
    );
    let has_ephemeral = system
        .as_array()
        .unwrap()
        .iter()
        .any(|block| block["cache_control"]["type"].as_str() == Some("ephemeral"));
    assert!(
        has_ephemeral,
        "at least one system block MUST carry cache_control.type=ephemeral (with_prompt_caching), body={}",
        body_str
    );
    // R14: no with_automatic_caching means no top-level cache_control
    assert!(
        body_json.get("cache_control").is_none(),
        "no top-level cache_control field permitted on fit body, body={}",
        body_str
    );
}

/// Given the model returns stop_reason = "end_turn"
/// When the fit handler completes
/// Then the StopReasonCapture hook has stored "end_turn" before the await
///     returns, and the captured value drives the handler's mapping
///     (Scenario 2)
///
/// Black-box assertion: the EndTurn branch returns 200 with the parsed
/// verdict. If the hook had NOT captured stop_reason, the handler would
/// have no signal to distinguish EndTurn from any other branch — and
/// the spec dictates that other branches diverge in status code. The
/// 200-with-parsed-verdict outcome is observable evidence the hook fired.
#[tokio::test]
async fn fit_handler_stop_reason_capture_stores_raw_response_stop_reason_after_await() {
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
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Hi" }))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["verdict"].as_str(), Some("strong-fit"));
}

// ===========================================================================
// Scenario 8 — refusal (fit half)
// ===========================================================================

/// Given the model returns stop_reason = "refusal"
/// When the client POSTs /api/fit
/// Then the response is HTTP 422
/// And the body is the canned {"error":"refusal","message":"<canned>"}
/// And the body never contains the raw model text (R29, R16)
#[tokio::test]
async fn fit_handler_returns_http_422_with_canned_refusal_body_on_refusal_stop_reason() {
    const DO_NOT_LEAK: &str = "DO-NOT-LEAK-MARKER-fit-refusal-raw-text";

    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(anthropic_messages_response(DO_NOT_LEAK, "refusal"))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Anything" }))
        .await;

    assert_eq!(
        response.status_code(),
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        "refusal MUST map to HTTP 422 (R16)"
    );

    let body: Value = response.json();
    assert_eq!(body["error"].as_str(), Some("refusal"));
    assert_eq!(
        body["message"].as_str(),
        Some("The assistant declined to respond to that request."),
        "fit refusal body MUST carry the canned message"
    );

    let body_str = std::str::from_utf8(response.as_bytes()).expect("UTF-8");
    assert!(
        !body_str.contains(DO_NOT_LEAK),
        "fit response body MUST NOT contain raw model text on refusal (R29); body={}",
        body_str
    );
}

// ===========================================================================
// Scenario 9 — context_exceeded (fit half)
// ===========================================================================

/// Given the model returns stop_reason = "model_context_window_exceeded"
/// When the client POSTs /api/fit
/// Then the response is HTTP 413
/// And the body is the canned {"error":"context_exceeded","message":"<canned>"}
/// And the body never contains the raw model text (R16, R29)
#[tokio::test]
async fn fit_handler_returns_http_413_with_canned_context_exceeded_body_on_model_context_window_exceeded()
 {
    const DO_NOT_LEAK: &str = "DO-NOT-LEAK-MARKER-fit-context-raw-text";

    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(anthropic_messages_response(
            DO_NOT_LEAK,
            "model_context_window_exceeded",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Long" }))
        .await;

    assert_eq!(
        response.status_code(),
        axum::http::StatusCode::PAYLOAD_TOO_LARGE,
        "context_exceeded MUST map to HTTP 413 (R16)"
    );

    let body: Value = response.json();
    assert_eq!(body["error"].as_str(), Some("context_exceeded"));
    assert_eq!(
        body["message"].as_str(),
        Some("The request exceeded the model's context window. Try a shorter input."),
        "fit context_exceeded body MUST carry the canned message"
    );

    let body_str = std::str::from_utf8(response.as_bytes()).expect("UTF-8");
    assert!(
        !body_str.contains(DO_NOT_LEAK),
        "fit response body MUST NOT contain raw model text (R29)"
    );
}

// ===========================================================================
// Scenario 15 — max_tokens (fit half)
// ===========================================================================

/// Given the model returns stop_reason = "max_tokens" and a buffered response
///     that happens to be valid FitVerdict JSON
/// When the client POSTs /api/fit
/// Then a warn log is written naming max_tokens
/// And the buffered response is passed to extract_json + serde_json::from_str
/// And the response is 200 with the parsed verdict (Scenario 15)
///
/// NOTE on parse-failure case (Scenario 15 secondary path): the spec says
/// existing parse-error code path returns — that's still AppError::Internal
/// with the existing parse error message. Tested implicitly via the
/// existing parse-error path; no new test needed here since no new variant
/// is added for this case.
#[tokio::test]
async fn fit_handler_logs_warn_on_max_tokens_stop_reason_and_attempts_json_parse_on_buffered_response()
 {
    let (buf, _guard) = install_log_capture();

    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(anthropic_messages_response(
            valid_fit_verdict_json(),
            "max_tokens",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Long" }))
        .await;

    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(
        body["verdict"].as_str(),
        Some("strong-fit"),
        "fit handler MUST attempt JSON parse on max_tokens and return 200 if parse succeeds"
    );

    let captured = buf.captured();
    assert!(
        captured.contains("WARN ") && captured.contains("max_tokens"),
        "fit handler MUST log warn naming max_tokens on max_tokens stop_reason, captured: {}",
        captured
    );
}

// ===========================================================================
// Scenario 14 — stop_sequence and pause_turn (fit half)
// ===========================================================================

/// Given the model returns stop_reason = "stop_sequence" and a valid JSON body
/// When the client POSTs /api/fit
/// Then the response is 200 OK with the parsed verdict (Scenario 14)
#[tokio::test]
async fn fit_handler_treats_stop_sequence_as_end_turn_equivalent() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(anthropic_messages_response(
            valid_fit_verdict_json(),
            "stop_sequence",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Hi" }))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["verdict"].as_str(), Some("strong-fit"));
}

/// Given the model returns stop_reason = "pause_turn" and a valid JSON body
/// When the client POSTs /api/fit
/// Then the response is 200 OK with the parsed verdict
#[tokio::test]
async fn fit_handler_treats_pause_turn_as_end_turn_equivalent() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(anthropic_messages_response(
            valid_fit_verdict_json(),
            "pause_turn",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Hi" }))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["verdict"].as_str(), Some("strong-fit"));
}

// ===========================================================================
// Scenario 16 — Other / unrecognized (fit half)
// ===========================================================================

/// Given the model returns an unrecognized stop_reason
/// When the client POSTs /api/fit
/// Then the response is HTTP 500 (existing AppError::Internal path)
/// And a warn log names the sanitized stop_reason value (Scenario 16, R13)
#[tokio::test]
async fn fit_handler_returns_500_on_unrecognized_stop_reason_after_warn_log() {
    let (buf, _guard) = install_log_capture();

    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/v1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(anthropic_messages_response(
            valid_fit_verdict_json(),
            "some_future_anthropic_string",
        ))
        .create_async()
        .await;

    let app = ai_test_app_with_mock(&server.url());
    let response = app
        .post("/api/fit")
        .json(&serde_json::json!({ "job_description": "Hi" }))
        .await;

    assert_eq!(
        response.status_code(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "Other stop_reason MUST map to HTTP 500"
    );

    let captured = buf.captured();
    assert!(
        captured.contains("WARN "),
        "Other stop_reason MUST emit a warn log, captured: {}",
        captured
    );
    assert!(
        captured.contains("some_future_anthropic_string"),
        "warn log MUST name the sanitized stop_reason value, captured: {}",
        captured
    );
}

#[allow(dead_code)]
fn _silence_unused() {
    let _ = ai_mock::valid_fit_verdict_json();
}
