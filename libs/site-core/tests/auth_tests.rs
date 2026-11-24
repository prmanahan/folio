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
        lists: common::test_projects::lists(),
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
        lists: common::test_projects::lists(),
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

// --- Admin session expiry (task #4851, finding M1) -------------------------
//
// Spec (task #4851 brief): login writes an `admin_sessions` row whose
// `created_at` / `expires_at` are `YYYY-MM-DDTHH:MM:SSZ` strings 24 hours apart;
// a token is accepted only while `expires_at > now` (TEXT comparison);
// `cleanup_expired` deletes rows with `expires_at <= now`.
//
// Sanctioned reads: `libs/site-core/auth.rs` for the route/handler names, the
// `admin_sessions` column names and the `Bearer` header shape only. Every
// verdict is observed as an HTTP status or operator SQL on the table; "now"
// and the offsets come from SQLite's own `strftime`, not from the code under
// test. The one exception, `auth::cleanup_expired`, has no caller (#4859) and
// is driven as a function; its effect is still read back with SQL.

const SESSION_TS_SQL: &str = "'%Y-%m-%dT%H:%M:%SZ'";

/// Same app as `auth_test_app`, with the DB handle so a test can seed and read
/// `admin_sessions` rows with operator SQL.
fn auth_test_app_with_db() -> (axum_test::TestServer, SharedDb) {
    let db: SharedDb = std::sync::Arc::new(std::sync::Mutex::new(common::test_db()));
    let state: DbState = std::sync::Arc::new(site_core::state::AppState {
        db: std::sync::Arc::clone(&db),
        admin_password_hash: common::test_password::password_hash(),
        rig_client: None,
        trusted_ip_header: None,
        page_hit_salt: "test-salt".to_string(),
        lists: common::test_projects::lists(),
    });
    let protected = Router::new()
        .route("/api/admin/test", get(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));
    let app = Router::new()
        .route("/api/admin/login", post(auth::login))
        .merge(protected)
        .with_state(state);
    (axum_test::TestServer::new(app), db)
}

/// Insert a session whose `expires_at` is SQLite's `now` shifted by `modifier`
/// (e.g. `"-1 hour"`, `"+1 hour"`, or `""` for the current second).
fn seed_session(db: &SharedDb, token: &str, modifier: &str) {
    let modifier_sql = if modifier.is_empty() {
        String::new()
    } else {
        format!(", '{modifier}'")
    };
    db.lock()
        .unwrap()
        .execute(
            &format!(
                "INSERT INTO admin_sessions (token, created_at, expires_at)
                 VALUES (?1, strftime({SESSION_TS_SQL}, 'now', '-1 day'),
                         strftime({SESSION_TS_SQL}, 'now'{modifier_sql}))"
            ),
            [token],
        )
        .unwrap();
}

async fn admin_status(server: &axum_test::TestServer, token: &str) -> axum::http::StatusCode {
    server
        .get("/api/admin/test")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {token}")
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .await
        .status_code()
}

fn session_count(db: &SharedDb, token: &str) -> i64 {
    db.lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM admin_sessions WHERE token = ?1",
            [token],
            |r| r.get(0),
        )
        .unwrap()
}

/// Given a session expired an hour ago, when its token is used on an admin
/// route, then the request is rejected with 401.
#[tokio::test]
async fn session_expired_in_the_past_is_rejected() {
    let (server, db) = auth_test_app_with_db();
    seed_session(&db, "expired-token", "-1 hour");

    assert_eq!(
        admin_status(&server, "expired-token").await,
        axum::http::StatusCode::UNAUTHORIZED
    );
}

/// Given a session whose `expires_at` is the current second, when its token is
/// used, then it is rejected: the comparison is strict (`expires_at > now`).
///
/// "Now" cannot be pinned, so the test starts near the top of a wall-clock
/// second (polling the clock, not sleeping a fixed time) and seeds the current
/// second, so the request lands in that same second and `now == expires_at`.
/// If the second ticks anyway the row is already in the past and the assertion
/// still holds: the test can miss a `>=` mutation on a rare
/// boundary-straddling run, but never fails on correct code.
#[tokio::test]
async fn session_expiring_this_second_is_rejected() {
    let (server, db) = auth_test_app_with_db();

    let sub_second_ms = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_millis()
    };
    while sub_second_ms() > 150 {
        tokio::task::yield_now().await;
    }
    seed_session(&db, "boundary-token", "");

    assert_eq!(
        admin_status(&server, "boundary-token").await,
        axum::http::StatusCode::UNAUTHORIZED,
        "expires_at == now must be rejected (strict >)"
    );
}

/// Positive control: given a session expiring in an hour, when its token is
/// used, then the admin route answers 200. Without this the two rejections
/// above could pass on a route that rejects everything.
#[tokio::test]
async fn session_expiring_in_the_future_is_accepted() {
    let (server, db) = auth_test_app_with_db();
    seed_session(&db, "live-token", "+1 hour");

    assert_eq!(
        admin_status(&server, "live-token").await,
        axum::http::StatusCode::OK
    );
}

/// Given a real login, then the stored `expires_at` minus `created_at` is
/// exactly 86400 seconds, both are `YYYY-MM-DDTHH:MM:SSZ`, and the login
/// response reports the stored `expires_at`.
#[tokio::test]
async fn login_session_lasts_exactly_24_hours() {
    let (server, db) = auth_test_app_with_db();

    let response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": common::test_password::password() }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let token = body["token"].as_str().unwrap().to_string();

    let (created_at, expires_at, seconds): (String, String, Option<i64>) = db
        .lock()
        .unwrap()
        .query_row(
            "SELECT created_at, expires_at,
                    CAST(strftime('%s', expires_at) AS INTEGER)
                      - CAST(strftime('%s', created_at) AS INTEGER)
             FROM admin_sessions WHERE token = ?1",
            [&token],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();

    for ts in [&created_at, &expires_at] {
        let b = ts.as_bytes();
        assert_eq!(b.len(), 20, "fixed-width timestamp: {ts}");
        assert!(
            b[4] == b'-'
                && b[7] == b'-'
                && b[10] == b'T'
                && b[13] == b':'
                && b[16] == b':'
                && b[19] == b'Z',
            "format YYYY-MM-DDTHH:MM:SSZ: {ts}"
        );
    }
    assert_eq!(
        seconds,
        Some(86_400),
        "{created_at} -> {expires_at} must be exactly 24h"
    );
    assert_eq!(body["expires_at"].as_str().unwrap(), expires_at);
}

/// Given one expired and one live session, when `cleanup_expired` runs, then
/// the expired row is gone and the live row stays (and still authenticates).
/// `cleanup_expired` has no caller (#4859), so it is invoked directly; the
/// outcome is read with SQL and over HTTP.
#[tokio::test]
async fn cleanup_expired_removes_only_expired_sessions() {
    let (server, db) = auth_test_app_with_db();
    seed_session(&db, "expired-token", "-1 hour");
    seed_session(&db, "live-token", "+1 hour");

    auth::cleanup_expired(&db.lock().unwrap()).unwrap();

    assert_eq!(session_count(&db, "expired-token"), 0, "expired row kept");
    assert_eq!(session_count(&db, "live-token"), 1, "live row removed");
    assert_eq!(
        admin_status(&server, "live-token").await,
        axum::http::StatusCode::OK
    );
}
