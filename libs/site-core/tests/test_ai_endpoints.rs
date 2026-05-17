//! Preserved tests from the pre-spec-#572 test suite.
//!
//! The spec #572 dispatch envelope (Task 4) directed Glitch to "drop
//! existing tests" — that direction targets the `rig_client: None`
//! early-exit assertions which test_ai_cross_cutting.rs supersedes.
//! The rate-limit tests below are ORTHOGONAL to spec #572's scope (they
//! exercise the pre-existing rate-limit logic in `ai/rate_limit.rs`,
//! unchanged by this spec) and remain as before — preserved coverage.

mod common;

use rusqlite::Connection;
use site_core::state::{AppState, DbState};
use std::sync::{Arc, Mutex};

/// Build a test app that includes the AI routes (without ConnectInfo).
fn ai_test_app() -> axum_test::TestServer {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size=-64000;
         PRAGMA temp_store=memory;",
    )
    .unwrap();
    site_core::db::migrate(&conn).unwrap();
    site_core::db::seed::seed_test_data(&conn).unwrap();

    let password_hash = common::test_password::password_hash();
    let state: DbState = Arc::new(AppState {
        db: Arc::new(Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client: None,
        trusted_ip_header: None,
        page_hit_salt: "test-salt".to_string(),
    });

    let app = axum::Router::new()
        .merge(site_core::routes::ai::routes())
        .with_state(state);

    axum_test::TestServer::new(app)
}

#[tokio::test]
async fn test_chat_rate_limit_exceeded_returns_429() {
    let server = ai_test_app();

    // The rate limit for chat is 10 per hour.
    // Send 11 requests — the 11th should be rate-limited.
    // Note: rate limit check happens BEFORE the rig client check,
    // so the first 10 will fail with 500 (no rig client), and
    // the 11th will fail with 429.
    for i in 1..=10 {
        let response = server
            .post("/api/chat")
            .json(&serde_json::json!({ "message": "Hello" }))
            .await;
        assert_ne!(
            response.status_code(),
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "Request {} should not be rate limited",
            i
        );
    }

    let response = server
        .post("/api/chat")
        .json(&serde_json::json!({ "message": "Hello" }))
        .await;

    response.assert_status(axum::http::StatusCode::TOO_MANY_REQUESTS);
    let body: serde_json::Value = response.json();
    assert!(
        body["error"].as_str().unwrap_or("").contains("Rate limit"),
        "Error should mention rate limit, got: {:?}",
        body
    );
}

#[tokio::test]
async fn test_fit_rate_limit_exceeded_returns_429() {
    let server = ai_test_app();

    for i in 1..=5 {
        let response = server
            .post("/api/fit")
            .json(&serde_json::json!({
                "job_description": "Looking for a senior engineer"
            }))
            .await;
        assert_ne!(
            response.status_code(),
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "Request {} should not be rate limited",
            i
        );
    }

    let response = server
        .post("/api/fit")
        .json(&serde_json::json!({
            "job_description": "Looking for a senior engineer"
        }))
        .await;

    response.assert_status(axum::http::StatusCode::TOO_MANY_REQUESTS);
    let body: serde_json::Value = response.json();
    assert!(
        body["error"].as_str().unwrap_or("").contains("Rate limit"),
        "Error should mention rate limit, got: {:?}",
        body
    );
}
