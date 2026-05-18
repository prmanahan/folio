mod common;

use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::{Router, routing::get};
use rusqlite::Connection;
use site_core::middleware::global_rate_limit::{
    GlobalRateLimitState, global_rate_limit_middleware,
};
use site_core::state::{AppState, DbState};
use std::sync::{Arc, Mutex};

/// Build a minimal test app that applies the global rate limit middleware to a
/// single GET /ping route.
fn rate_limit_test_app() -> (axum_test::TestServer, GlobalRateLimitState) {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
        .unwrap();
    site_core::db::migrate(&conn).unwrap();

    let password_hash = common::test_password::password_hash();
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

    let password_hash = common::test_password::password_hash();
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
    use axum::http::HeaderMap;
    use site_core::middleware::global_rate_limit::extract_ip_for_rate_limit;

    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", "1.1.1.1".parse().unwrap());
    headers.insert("x-test-ip", "9.9.9.9".parse().unwrap());

    // With trusted header configured
    assert_eq!(
        extract_ip_for_rate_limit(&headers, Some("x-test-ip")),
        "9.9.9.9"
    );

    // Without trusted header configured — falls back to XFF
    assert_eq!(extract_ip_for_rate_limit(&headers, None), "1.1.1.1");
}

// ===========================================================================
// R4 (LLM-audit L1) — `"unknown"` IP fallback collapses no-header callers
// into one bucket. Spec: docs/specs/2026-05-18-llm-audit-remediation.md R4.
//
// Fix: when neither the trusted header nor x-forwarded-for resolves, prefer
// the ConnectInfo peer address; fall back to "unknown" ONLY when no peer
// addr is available. Apply consistently to the mirrored site
// (routes/ai.rs::extract_ip).
//
// Red-phase mechanism: `extract_ip_for_rate_limit` is `pub`. R4 changes its
// signature to accept the peer addr. The three signature-dependent
// behavior tests below call the POST-FIX 3-arg signature and are gated
// behind `#[cfg(any())]` (compiled out today) so the existing 2-arg green
// tests in this file STILL COMPILE AND PASS (definition-of-done item 3 —
// "existing tests still compile and pass"). The gate is the red signal:
// Forge REMOVES `#[cfg(any())]` as the first R4 step; the module then
// participates and the assertions become the gate.
//
// The `r4_mirrored_...` source-text meta-test at the bottom is NOT gated —
// it reads routes/ai.rs as text and is RED today (the hardcoded "unknown"
// fallback literal is present). That keeps R4 with a live failing test in
// the suite now, independent of the gated signature tests.
//
// Forge handoff (R4, do these together in one pass):
//   1. Remove the `#[cfg(any())]` gate on `mod r4_peer_addr_fallback`.
//   2. Change the signature to:
//        pub fn extract_ip_for_rate_limit(
//            headers: &HeaderMap,
//            trusted_header: Option<&str>,
//            peer_addr: Option<SocketAddr>,
//        ) -> String
//      preferring peer_addr.ip() over "unknown" when both header paths miss.
//   3. Update ALL callers to the 3-arg form: the existing
//      `trusted_header_takes_priority_over_xff` test above, the middleware
//      caller (`global_rate_limit.rs` ~line 113, threading the peer addr
//      via `request.extensions().get::<ConnectInfo<SocketAddr>>()`), and
//      the analogous mirrored `routes/ai.rs::extract_ip` (its `chat()` /
//      `fit_analysis()` no-ConnectInfo arms pass `None`; the `_with_addr`
//      arms pass `Some(addr)`).
// ===========================================================================

// Forge: delete this `#[cfg(any())]` line to enable the R4 signature
// contract tests (step 1 of the R4 handoff above).
#[cfg(any())]
mod r4_peer_addr_fallback {

/// Given: neither trusted header nor x-forwarded-for present, but two
///   distinct ConnectInfo peer addrs
/// When:  extract_ip_for_rate_limit is called for each
/// Then:  the two peer addrs yield DISTINCT bucket keys (R4 acceptance)
///
/// Red-phase: fails to compile until the peer-addr arg is added; once
/// added, pre-fix logic would return "unknown" for both (shared bucket) —
/// the assertion then drives the behavior.
#[tokio::test]
async fn r4_distinct_peer_addrs_yield_distinct_buckets_when_headers_absent() {
    use axum::http::HeaderMap;
    use site_core::middleware::global_rate_limit::extract_ip_for_rate_limit;
    use std::net::SocketAddr;

    let headers = HeaderMap::new(); // no trusted header, no XFF

    let peer_a: SocketAddr = "1.1.1.1:51000".parse().unwrap();
    let peer_b: SocketAddr = "2.2.2.2:51000".parse().unwrap();

    let bucket_a = extract_ip_for_rate_limit(&headers, None, Some(peer_a));
    let bucket_b = extract_ip_for_rate_limit(&headers, None, Some(peer_b));

    assert_ne!(
        bucket_a, bucket_b,
        "R4: two distinct ConnectInfo peer addrs MUST resolve to distinct \
         rate-limit bucket keys when both header sources are absent \
         (pre-fix both collapse to \"unknown\")"
    );
    assert!(
        bucket_a.contains("1.1.1.1"),
        "R4: peer_a bucket key must derive from its peer addr; got {bucket_a:?}"
    );
    assert!(
        bucket_b.contains("2.2.2.2"),
        "R4: peer_b bucket key must derive from its peer addr; got {bucket_b:?}"
    );
}

/// Given: neither header present AND no ConnectInfo peer addr available
/// When:  extract_ip_for_rate_limit is called
/// Then:  the literal "unknown" is returned — `"unknown"` is the fallback
///   ONLY when no peer addr is available (R4 acceptance, second clause)
#[tokio::test]
async fn r4_unknown_only_when_no_peer_addr_available() {
    use axum::http::HeaderMap;
    use site_core::middleware::global_rate_limit::extract_ip_for_rate_limit;

    let headers = HeaderMap::new();
    assert_eq!(
        extract_ip_for_rate_limit(&headers, None, None),
        "unknown",
        "R4: with no headers AND no peer addr, the fallback is still \
         \"unknown\" (the only path that may use it)"
    );
}

/// Given: trusted header present alongside a peer addr
/// When:  extract_ip_for_rate_limit is called
/// Then:  the trusted header still wins — peer addr is a fallback, NOT an
///   override of the existing precedence (R4 must not regress header
///   priority)
#[tokio::test]
async fn r4_peer_addr_does_not_override_trusted_header_or_xff() {
    use axum::http::HeaderMap;
    use site_core::middleware::global_rate_limit::extract_ip_for_rate_limit;
    use std::net::SocketAddr;

    let peer: SocketAddr = "9.9.9.9:443".parse().unwrap();

    // Trusted header present → wins over peer addr.
    let mut h_trusted = HeaderMap::new();
    h_trusted.insert("x-real-ip", "5.5.5.5".parse().unwrap());
    assert_eq!(
        extract_ip_for_rate_limit(&h_trusted, Some("x-real-ip"), Some(peer)),
        "5.5.5.5",
        "R4: trusted header MUST still take priority over the peer addr"
    );

    // XFF present, no trusted header → wins over peer addr.
    let mut h_xff = HeaderMap::new();
    h_xff.insert("x-forwarded-for", "6.6.6.6, 7.7.7.7".parse().unwrap());
    assert_eq!(
        extract_ip_for_rate_limit(&h_xff, None, Some(peer)),
        "6.6.6.6",
        "R4: x-forwarded-for MUST still take priority over the peer addr"
    );
    }
} // end #[cfg(any())] mod r4_peer_addr_fallback — Forge removes the gate

/// R4 mirrored-site consistency (source-text meta-test). The spec requires
/// the SAME fix at `routes/ai.rs::extract_ip`. Today its no-ConnectInfo
/// arms call `extract_ip(&headers, "unknown", ...)` (routes/ai.rs:87, 180),
/// hardcoding the "unknown" fallback at the call site. Post-fix the
/// extractor must take a peer-addr arg and the literal
/// `extract_ip(&headers, "unknown",` call form must be gone (replaced by a
/// peer-addr-threading form). Source-as-text: reads routes/ai.rs, does NOT
/// modify it. CARGO_MANIFEST_DIR resolves to libs/site-core/.
#[test]
fn r4_mirrored_extract_ip_in_routes_ai_no_longer_hardcodes_unknown_fallback() {
    let ai_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/routes/ai.rs"
    ))
    .expect("routes/ai.rs must be readable");

    assert!(
        !ai_src.contains(r#"extract_ip(&headers, "unknown","#),
        "R4 (mirrored site): routes/ai.rs MUST NOT hardcode the literal \
         \"unknown\" fallback at the extract_ip call site — the \
         no-ConnectInfo handlers must thread a peer addr (or None) through \
         the same fix applied to extract_ip_for_rate_limit"
    );
}
