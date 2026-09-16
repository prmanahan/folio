//! Task #3558 C2: production-path router tests. These build the app
//! through the SAME `site_core::app::build_app` the running server uses
//! (Warden M2) — a test that constructs its own router could drift from
//! production wiring with no test ever noticing.
//!
//! `origin_lock` is the outermost layer (C1): every one of these
//! assertions exercises the real fallback/route/health path underneath
//! it, not a stand-in.

mod common;

use axum::http::StatusCode;
use site_core::app::build_app;
use site_core::middleware::origin_lock::OriginLockState;

const EDGE_AUTH_HEADER: &str = "x-folio-edge-auth";
// Throwaway test values only — never a real secret, never committed
// anywhere outside this test file.
const TEST_SECRET: &str = "folio_edge_test_secret_for_ci_only";
const WRONG_SECRET: &str = "folio_edge_a_different_secret_value";
const STATIC_FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/static_site");
const TEST_CORS_ORIGIN: &str = "http://localhost:3000";

fn locked_server() -> axum_test::TestServer {
    let (_unused_server, db_state) = common::test_app_with_state();
    let origin_lock_state = OriginLockState::new(TEST_SECRET);
    let app = build_app(
        db_state,
        origin_lock_state,
        STATIC_FIXTURE_DIR,
        TEST_CORS_ORIGIN,
    );
    axum_test::TestServer::new(app)
}

// ---------------------------------------------------------------------------
// Without the secret: every one of these must 404, health exempted.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn root_without_secret_is_404() {
    let server = locked_server();
    let response = server.get("/").await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn static_asset_without_secret_is_404() {
    let server = locked_server();
    let response = server.get("/asset.txt").await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn spa_path_without_secret_is_404() {
    let server = locked_server();
    let response = server.get("/some/client/side/route").await;
    response.assert_status(StatusCode::NOT_FOUND);
    // Stronger than the status code alone: `ServeDir`'s own SPA
    // not-found fallback ALSO answers with a 404 status (it wraps the
    // served index.html in `SetStatus::new(_, StatusCode::NOT_FOUND)`,
    // per tower_http), so a bare 404 assertion here can't tell "origin
    // lock rejected this" apart from "origin lock let it through and the
    // fallback happened to 404 it too". An empty body distinguishes
    // them: origin_lock's own rejection never touches the fallback, so
    // it can't carry the fixture's index content.
    let body = response.text();
    assert!(
        !body.contains("origin-lock test fixture index"),
        "a request rejected by origin_lock must never reach the SPA \
         fallback and carry its content, got body: {body:?}"
    );
}

#[tokio::test]
async fn api_route_without_secret_is_404() {
    let server = locked_server();
    let response = server.get("/api/profile").await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn health_with_trailing_slash_without_secret_is_404() {
    // Not the exempt path — the exemption is exact, not a prefix.
    let server = locked_server();
    let response = server.get("/api/health/").await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn head_health_without_secret_is_404() {
    let server = locked_server();
    let response = server.method(axum::http::Method::HEAD, "/api/health").await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn post_health_without_secret_is_404() {
    let server = locked_server();
    let response = server.post("/api/health").await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn wrong_secret_is_404_same_as_missing() {
    let server = locked_server();
    let response = server
        .get("/api/profile")
        .add_header(EDGE_AUTH_HEADER, WRONG_SECRET)
        .await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn repeated_secret_header_is_rejected_even_when_both_values_are_correct() {
    let server = locked_server();
    let response = server
        .get("/api/profile")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    response.assert_status(StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// The health exemption: exact GET /api/health, with or without a secret.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_health_without_secret_is_200() {
    let server = locked_server();
    let response = server.get("/api/health").await;
    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn get_health_with_query_string_without_secret_is_200() {
    let server = locked_server();
    let response = server.get("/api/health?x=1").await;
    response.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// With the correct secret: the real content underneath origin_lock serves.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn root_with_secret_is_200() {
    let server = locked_server();
    let response = server
        .get("/")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    response.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn static_asset_with_secret_is_200_with_real_content() {
    let server = locked_server();
    let response = server
        .get("/asset.txt")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    response.assert_status(StatusCode::OK);
    let body = response.text();
    assert!(
        body.contains("origin-lock test fixture static asset"),
        "expected the real fixture asset content, got: {body:?}"
    );
}

#[tokio::test]
async fn spa_path_with_secret_reaches_the_real_index_fallback() {
    let server = locked_server();
    let response = server
        .get("/some/client/side/route")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    // `ServeDir`'s own not-found fallback answers 404 (it wraps the
    // served index.html in `SetStatus::new(_, StatusCode::NOT_FOUND)`,
    // per tower_http) — that status is unrelated to origin_lock, which
    // is exactly the point: with the correct secret, this request
    // reaches the REAL fallback and gets the REAL content, rather than
    // origin_lock's own contentless rejection.
    response.assert_status(StatusCode::NOT_FOUND);
    let body = response.text();
    assert!(
        body.contains("origin-lock test fixture index"),
        "expected the SPA index fallback content, got: {body:?}"
    );
}

#[tokio::test]
async fn api_route_with_secret_is_200() {
    let server = locked_server();
    let response = server
        .get("/api/profile")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    response.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Rejected requests touch no DB and no limiter state (C2).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rejected_requests_never_reach_the_global_limiter() {
    let server = locked_server();

    // The global limit is 60/min. If a rejected request touched the
    // limiter, the 61st of these (all lacking the secret) would flip
    // from 404 (origin_lock's own rejection) to 429 (the limiter firing).
    // It must not: origin_lock never calls `next` on the reject path, so
    // the limiter is structurally unreachable from here.
    for i in 0..61 {
        let response = server.get("/api/profile").await;
        response.assert_status_not_found();
        assert_ne!(
            response.status_code(),
            StatusCode::TOO_MANY_REQUESTS,
            "request {i} must never be rate-limited — it should never reach the limiter at all"
        );
    }
}

#[tokio::test]
async fn rejected_requests_never_write_a_page_hit() {
    let (_unused_server, db_state) = common::test_app_with_state();
    let origin_lock_state = OriginLockState::new(TEST_SECRET);
    let app = build_app(
        db_state.clone(),
        origin_lock_state,
        STATIC_FIXTURE_DIR,
        TEST_CORS_ORIGIN,
    );
    let server = axum_test::TestServer::new(app);

    // "/" is a page_hits-tracked path. Without the secret, origin_lock
    // must reject before `page_hits_middleware` ever runs.
    for _ in 0..5 {
        server.get("/").await.assert_status_not_found();
    }

    let counts = {
        let conn = db_state.db.lock().expect("db lock");
        site_core::models::page_hits::get_hit_counts(&conn).expect("query hit counts")
    };
    assert!(
        counts.is_empty(),
        "no page-hit row should exist after only rejected requests, got: {counts:?}"
    );
}
