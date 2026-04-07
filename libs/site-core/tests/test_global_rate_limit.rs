mod common;

use axum::{routing::get, Router};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use rusqlite::Connection;
use site_core::middleware::global_rate_limit::{GlobalRateLimitState, global_rate_limit_middleware};
use site_core::state::{AppState, DbState};
use std::sync::{Arc, Mutex};

/// Build a minimal test app that applies the global rate limit middleware to a
/// single GET /ping route.
fn rate_limit_test_app() -> (axum_test::TestServer, GlobalRateLimitState) {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
        .unwrap();
    site_core::db::migrate(&conn).unwrap();

    let password_hash = site_core::auth::hash_password("testpass").unwrap();
    let state: DbState = Arc::new(AppState {
        db: Arc::new(Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client: None,
        trusted_ip_header: Some("x-test-ip".to_string()),
        page_hit_salt: "test-salt".to_string(),
    });

    let global_rl = GlobalRateLimitState::new();

    let app = Router::new()
        .route("/ping", get(|| async { (StatusCode::OK, "pong") }))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            global_rate_limit_middleware,
        ))
        .layer(axum::Extension(global_rl.clone()));

    (axum_test::TestServer::new(app), global_rl)
}

/// A simpler test app that directly checks GlobalRateLimitState without HTTP overhead.
/// This validates the core sliding-window logic end-to-end.
#[tokio::test]
async fn global_rate_limit_within_limit_succeeds() {
    let state = GlobalRateLimitState::new();
    let window = std::time::Duration::from_secs(60);

    // 60 requests should all pass
    for i in 0..60 {
        assert!(
            state.check("1.2.3.4", 60, window),
            "request {i} of 60 should be allowed"
        );
    }
}

#[tokio::test]
async fn global_rate_limit_61st_request_denied() {
    let state = GlobalRateLimitState::new();
    let window = std::time::Duration::from_secs(60);

    for _ in 0..60 {
        state.check("10.0.0.1", 60, window);
    }
    assert!(
        !state.check("10.0.0.1", 60, window),
        "61st request should be denied"
    );
}

/// Integration test: verify that within-limit requests get 200 via the real HTTP stack.
#[tokio::test]
async fn http_request_within_limit_returns_200() {
    let (server, _rl) = rate_limit_test_app();

    let response = server
        .get("/ping")
        .add_header(
            HeaderName::from_static("x-test-ip"),
            HeaderValue::from_static("99.0.0.1"),
        )
        .await;

    response.assert_status(StatusCode::OK);
}

/// Integration test: verify that the 61st request returns 429 with Retry-After.
/// Uses the unit-level GlobalRateLimitState directly to exhaust the limit,
/// then fires one HTTP request to verify the middleware responds correctly.
#[tokio::test]
async fn http_request_over_limit_returns_429() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    site_core::db::migrate(&conn).unwrap();

    let password_hash = site_core::auth::hash_password("testpass").unwrap();
    let state: DbState = Arc::new(AppState {
        db: Arc::new(Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client: None,
        trusted_ip_header: Some("x-test-ip".to_string()),
        page_hit_salt: "test-salt".to_string(),
    });

    let global_rl = GlobalRateLimitState::new();
    let window = std::time::Duration::from_secs(60);

    // Exhaust 60 slots directly so we don't need 60 HTTP round-trips.
    for _ in 0..60 {
        global_rl.check("55.55.55.55", 60, window);
    }

    let app = Router::new()
        .route("/ping", get(|| async { (StatusCode::OK, "pong") }))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            global_rate_limit_middleware,
        ))
        .layer(axum::Extension(global_rl));

    let server = axum_test::TestServer::new(app);

    let response = server
        .get("/ping")
        .add_header(
            HeaderName::from_static("x-test-ip"),
            HeaderValue::from_static("55.55.55.55"),
        )
        .await;

    response.assert_status(StatusCode::TOO_MANY_REQUESTS);
    // Verify Retry-After header is present
    assert!(
        response.headers().contains_key("retry-after"),
        "429 response should include Retry-After header"
    );
}

/// Integration test: verify that IP extraction uses trusted header over XFF.
#[tokio::test]
async fn trusted_header_takes_priority_over_xff() {
    use site_core::middleware::global_rate_limit::extract_ip_for_rate_limit;
    use axum::http::HeaderMap;

    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", "1.1.1.1".parse().unwrap());
    headers.insert("x-test-ip", "9.9.9.9".parse().unwrap());

    // With trusted header configured
    assert_eq!(
        extract_ip_for_rate_limit(&headers, Some("x-test-ip")),
        "9.9.9.9"
    );

    // Without trusted header configured — falls back to XFF
    assert_eq!(
        extract_ip_for_rate_limit(&headers, None),
        "1.1.1.1"
    );
}
