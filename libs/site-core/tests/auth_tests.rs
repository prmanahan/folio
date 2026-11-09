mod common;

use axum::{
    Router, middleware,
    routing::{get, post},
};
use site_core::auth;
use site_core::state::DbState;

fn auth_test_app() -> axum_test::TestServer {
    let conn = common::test_db();
    site_core::db::seed::seed_test_data(&conn).unwrap();

    let password_hash = common::test_password::password_hash();
    let state: DbState = std::sync::Arc::new(site_core::state::AppState {
        db: std::sync::Arc::new(std::sync::Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client: None,
        trusted_ip_header: None,
        page_hit_salt: "test-salt".to_string(),
    });

    let protected = Router::new()
        .route("/api/admin/test", get(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));

    let app = Router::new()
        .route("/api/admin/login", post(auth::login))
        .route("/api/admin/logout", post(auth::logout))
        .merge(protected)
        .with_state(state);

    axum_test::TestServer::new(app)
}

#[tokio::test]
async fn test_login_success() {
    let server = auth_test_app();

    let response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": common::test_password::password() }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["token"].is_string(), "response should have a token");
    assert!(
        body["expires_at"].is_string(),
        "response should have expires_at"
    );
    let token = body["token"].as_str().unwrap();
    assert!(!token.is_empty(), "token should not be empty");
}

#[tokio::test]
async fn test_login_wrong_password() {
    let server = auth_test_app();

    let response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": "wrongpass" }))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_with_valid_token() {
    let server = auth_test_app();

    // First, log in to get a token
    let login_response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": common::test_password::password() }))
        .await;
    login_response.assert_status_ok();
    let body: serde_json::Value = login_response.json();
    let token = body["token"].as_str().unwrap().to_string();

    // Use the token to access protected route
    let response = server
        .get("/api/admin/test")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token)
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn test_protected_without_token() {
    let server = auth_test_app();

    let response = server.get("/api/admin/test").await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_with_invalid_token() {
    let server = auth_test_app();

    let response = server
        .get("/api/admin/test")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer not-a-real-token"
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_logout_invalidates_token() {
    let server = auth_test_app();

    // Log in
    let login_response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": common::test_password::password() }))
        .await;
    login_response.assert_status_ok();
    let body: serde_json::Value = login_response.json();
    let token = body["token"].as_str().unwrap().to_string();

    // Log out
    let logout_response = server
        .post("/api/admin/logout")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token)
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .await;
    logout_response.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Try to use the same token after logout - should be 401
    let response = server
        .get("/api/admin/test")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token)
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// --- Login brute-force limiter keying (task #3482) -------------------------
//
// The limiter must key on the same value as every other rate limiter:
// `extract_ip_for_rate_limit`. With a trusted header configured but absent
// from the request, that is the peer address — never the client-controlled
// `x-forwarded-for`.

const LOGIN_FAIL_LIMIT: usize = 5;

type SharedDb = std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>;

/// Login app, with `TRUSTED_IP_HEADER` set to `trusted` (or unset). The peer
/// address is injected from the `x-test-peer` request header (an IP literal)
/// so one test server can present several distinct `ConnectInfo` peers. The
/// DB handle is returned so tests can assert on `rate_limits` rows.
fn login_app(trusted: Option<&str>) -> (axum_test::TestServer, SharedDb) {
    use axum::extract::{ConnectInfo, Request};
    use std::net::SocketAddr;

    let db: SharedDb = std::sync::Arc::new(std::sync::Mutex::new(common::test_db()));
    let state: DbState = std::sync::Arc::new(site_core::state::AppState {
        db: std::sync::Arc::clone(&db),
        admin_password_hash: common::test_password::password_hash(),
        rig_client: None,
        trusted_ip_header: trusted.map(str::to_string),
        page_hit_salt: "test-salt".to_string(),
    });

    let app = Router::new()
        .route("/api/admin/login", post(auth::login))
        .with_state(state)
        .layer(middleware::from_fn(
            |mut req: Request, next: middleware::Next| async move {
                if let Some(peer) = req
                    .headers()
                    .get("x-test-peer")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<std::net::IpAddr>().ok())
                {
                    req.extensions_mut()
                        .insert(ConnectInfo(SocketAddr::new(peer, 40000)));
                }
                next.run(req).await
            },
        ));

    (axum_test::TestServer::new(app), db)
}

/// One failed login from `peer`, carrying each `(name, value)` in `extra`.
async fn failed_login(
    server: &axum_test::TestServer,
    peer: &str,
    extra: &[(&str, &str)],
) -> axum::http::StatusCode {
    let mut req = server
        .post("/api/admin/login")
        .add_header("x-test-peer", peer)
        .json(&serde_json::json!({ "password": "wrongpass" }));
    for (name, value) in extra {
        req = req.add_header(*name, *value);
    }
    req.await.status_code()
}

/// The limiter's buckets are clock-hour aligned, so a run that straddles the
/// hour resets counts mid-test. Keys (the table's primary key) are stable
/// across a rollover; counts and 429s are not.
fn current_hour(db: &SharedDb) -> String {
    db.lock()
        .unwrap()
        .query_row("SELECT strftime('%Y-%m-%d %H', 'now')", [], |r| r.get(0))
        .unwrap()
}

/// `(ip, request_count)` of every `login_fail` row, ordered by ip.
fn login_fail_rows(db: &SharedDb) -> Vec<(String, i64)> {
    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT ip, request_count FROM rate_limits
             WHERE endpoint = 'login_fail' ORDER BY ip",
        )
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// Drive `LOGIN_FAIL_LIMIT + 1` failed logins from one peer, rotating
/// `x-forwarded-for` each time, and assert the whole run landed in the one
/// bucket `expected_key`. Row assertions hold across an hour rollover; the
/// count and 429 assertions are skipped if the hour changed mid-run.
async fn assert_rotating_xff_stays_in_one_bucket(trusted: Option<&str>, expected_key: &str) {
    use axum::http::StatusCode;

    let (server, db) = login_app(trusted);
    let hour_before = current_hour(&db);

    let mut statuses = Vec::new();
    for i in 0..=LOGIN_FAIL_LIMIT {
        let xff = format!("203.0.113.{i}");
        statuses.push(failed_login(&server, "198.51.100.7", &[("x-forwarded-for", &xff)]).await);
    }

    let rows = login_fail_rows(&db);
    assert_eq!(
        rows.iter().map(|(ip, _)| ip.as_str()).collect::<Vec<_>>(),
        vec![expected_key],
        "a rotated x-forwarded-for must not buy a fresh bucket: {rows:?}"
    );

    if current_hour(&db) == hour_before {
        assert_eq!(rows[0].1, LOGIN_FAIL_LIMIT as i64, "five failures counted");
        assert!(
            statuses[..LOGIN_FAIL_LIMIT]
                .iter()
                .all(|s| *s == StatusCode::UNAUTHORIZED)
        );
        assert_eq!(statuses[LOGIN_FAIL_LIMIT], StatusCode::TOO_MANY_REQUESTS);
    }
}

/// Trusted header configured but absent; one peer rotating a spoofed
/// `x-forwarded-for`. All attempts share the peer's bucket.
#[tokio::test]
async fn login_limiter_ignores_rotating_xff_when_trusted_header_absent() {
    assert_rotating_xff_stays_in_one_bucket(Some("x-test-ip"), "198.51.100.7").await;
}

/// `TRUSTED_IP_HEADER` unset (the dropped-setting case): the same guarantee.
#[tokio::test]
async fn login_limiter_ignores_rotating_xff_when_no_trusted_header_configured() {
    assert_rotating_xff_stays_in_one_bucket(None, "198.51.100.7").await;
}

/// Two distinct peers do not share a bucket: two rows, one per peer.
#[tokio::test]
async fn login_limiter_does_not_share_a_bucket_across_peers() {
    use axum::http::StatusCode;

    let (server, db) = login_app(Some("x-test-ip"));
    let hour_before = current_hour(&db);

    let xff = [("x-forwarded-for", "203.0.113.1")];
    let mut first = Vec::new();
    for _ in 0..=LOGIN_FAIL_LIMIT {
        first.push(failed_login(&server, "198.51.100.7", &xff).await);
    }
    let second = failed_login(&server, "198.51.100.8", &xff).await;

    let rows = login_fail_rows(&db);
    assert_eq!(
        rows.iter().map(|(ip, _)| ip.as_str()).collect::<Vec<_>>(),
        vec!["198.51.100.7", "198.51.100.8"],
        "one bucket per peer: {rows:?}"
    );

    if current_hour(&db) == hour_before {
        assert_eq!(first[LOGIN_FAIL_LIMIT], StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            second,
            StatusCode::UNAUTHORIZED,
            "second peer not locked out"
        );
        assert_eq!(rows[0].1, LOGIN_FAIL_LIMIT as i64);
        assert_eq!(rows[1].1, 1);
    }
}

/// Production branch: one peer, trusted header present with two different
/// values. Each value is its own bucket, keyed on the header, not the peer.
#[tokio::test]
async fn login_limiter_keys_on_trusted_header_value_not_peer() {
    use axum::http::StatusCode;

    let (server, db) = login_app(Some("x-test-ip"));
    let hour_before = current_hour(&db);

    let mut first = Vec::new();
    for _ in 0..=LOGIN_FAIL_LIMIT {
        first.push(
            failed_login(
                &server,
                "198.51.100.7",
                &[
                    ("x-test-ip", "203.0.113.50"),
                    ("x-forwarded-for", "192.0.2.1"),
                ],
            )
            .await,
        );
    }
    let second = failed_login(&server, "198.51.100.7", &[("x-test-ip", "203.0.113.51")]).await;

    let rows = login_fail_rows(&db);
    assert_eq!(
        rows.iter().map(|(ip, _)| ip.as_str()).collect::<Vec<_>>(),
        vec!["203.0.113.50", "203.0.113.51"],
        "buckets keyed on the header value, not the shared peer: {rows:?}"
    );

    if current_hour(&db) == hour_before {
        assert_eq!(first[LOGIN_FAIL_LIMIT], StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(second, StatusCode::UNAUTHORIZED);
        assert_eq!(rows[0].1, LOGIN_FAIL_LIMIT as i64);
        assert_eq!(rows[1].1, 1);
    }
}
