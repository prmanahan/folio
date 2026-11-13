//! Production-path tests through `site_core::app::build_app`, the same router
//! `cmd/server` serves (task #3558 M2 — a test-local router can drift):
//!
//!  - #4710 F2: security headers on every response, the SPA fallback included;
//!    exactly one `Content-Security-Policy` header per response: on responses
//!    from the static service, the build-validated `csp-header.txt` verbatim
//!    (or the minimal `STATIC_CSP` fallback when that file is unusable), the
//!    full policy elsewhere. HTML gets `charset=utf-8`.
//!  - #4711 F7: the AI per-IP limiter keyed on the configured trusted header,
//!    never on `x-forwarded-for`.
//!  - #4711 / LLM01: invisible Unicode is stripped before the length checks.
//!
//! `build_app` is wrapped in `into_make_service_with_connect_info` and served
//! over a real socket (axum-test's HTTP transport), as `main.rs` does, because
//! the production AI handlers extract `ConnectInfo`.

mod common;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::http::{HeaderMap, StatusCode};
use serde_json::{Value, json};
use site_core::app::build_app;
use site_core::middleware::global_rate_limit::GlobalRateLimitState;
use site_core::middleware::origin_lock::OriginLockState;
use site_core::state::{AppState, DbState};

const EDGE_AUTH_HEADER: &str = "x-folio-edge-auth";
// Throwaway test value only; never a real secret.
const TEST_SECRET: &str = "folio_edge_test_secret_for_ci_only";
const STATIC_FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/static_site");
const TEST_CORS_ORIGIN: &str = "http://localhost:3000";
const TRUSTED_HEADER: &str = "cf-connecting-ip";

fn server_with(trusted_ip_header: Option<&str>) -> axum_test::TestServer {
    server_in(STATIC_FIXTURE_DIR, trusted_ip_header)
}

fn server_in(static_dir: &str, trusted_ip_header: Option<&str>) -> axum_test::TestServer {
    let state: DbState = Arc::new(AppState {
        db: Arc::new(Mutex::new(common::seeded_db())),
        admin_password_hash: common::test_password::password_hash(),
        rig_client: None,
        trusted_ip_header: trusted_ip_header.map(str::to_string),
        page_hit_salt: "test-salt".to_string(),
        projects: common::test_projects::projects(),
    });
    let app = build_app(
        state,
        OriginLockState::new(TEST_SECRET),
        GlobalRateLimitState::new(),
        static_dir,
        TEST_CORS_ORIGIN,
    );
    axum_test::TestServer::new(app.into_make_service_with_connect_info::<SocketAddr>())
}

/// The single value of header `name`, `None` if absent. Panics on a duplicated
/// header, naming `what` (the request under test) in the message:
/// `HeaderMap::get` returns only the first value, so a response carrying two
/// `Content-Security-Policy` headers (the browser enforces both) would pass.
fn header<'a>(headers: &'a HeaderMap, name: &str, what: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let first = values.next()?;
    if values.next().is_some() {
        let all: Vec<_> = headers.get_all(name).iter().collect();
        panic!(
            "{what}: {name}: expected exactly one header, got {}: {all:?}",
            all.len()
        );
    }
    Some(first.to_str().unwrap())
}

#[test]
fn header_helper_accepts_a_single_value() {
    let mut headers = HeaderMap::new();
    headers.append(
        "content-security-policy",
        "default-src 'self'".parse().unwrap(),
    );
    assert_eq!(
        header(&headers, "content-security-policy", "unit"),
        Some("default-src 'self'")
    );
    assert_eq!(header(&headers, "x-absent", "unit"), None);
}

#[test]
#[should_panic(
    expected = "unit: content-security-policy: expected exactly one header, got 2: [\"frame-ancestors 'none'\", \"object-src 'none'\"]"
)]
fn header_helper_rejects_a_duplicated_header() {
    let mut headers = HeaderMap::new();
    headers.append(
        "content-security-policy",
        "frame-ancestors 'none'".parse().unwrap(),
    );
    headers.append(
        "content-security-policy",
        "object-src 'none'".parse().unwrap(),
    );
    header(&headers, "content-security-policy", "unit");
}

/// The headers every response carries, CSP excluded.
fn assert_common_security_headers(headers: &HeaderMap, what: &str) {
    assert_eq!(
        header(headers, "x-frame-options", what),
        Some("DENY"),
        "{what}"
    );
    assert_eq!(
        header(headers, "x-content-type-options", what),
        Some("nosniff"),
        "{what}"
    );
    assert_eq!(
        header(headers, "referrer-policy", what),
        Some("strict-origin-when-cross-origin"),
        "{what}"
    );
    assert_eq!(
        header(headers, "permissions-policy", what),
        Some("camera=(), microphone=(), geolocation=()"),
        "{what}"
    );
    assert_eq!(
        header(headers, "strict-transport-security", what),
        Some("max-age=31536000; includeSubDomains"),
        "{what}"
    );
}

// ---------------------------------------------------------------------------
// #4710 F2: security headers; CSP keyed on where the response came from
// ---------------------------------------------------------------------------

/// Contents of `tests/fixtures/static_site/csp-header.txt`, byte for byte.
const FIXTURE_CSP: &str = "default-src 'self'; script-src 'self' 'sha256-FIXTURE+hash='; object-src 'none'; frame-ancestors 'none'";
/// The server's fallback when `csp-header.txt` is unusable (`STATIC_CSP`).
const MINIMAL_CSP: &str = "frame-ancestors 'none'; object-src 'none'; base-uri 'none'";
/// A date no file's mtime can exceed: forces a 304 on a conditional GET.
const FAR_FUTURE: &str = "Fri, 01 Jan 2100 00:00:00 GMT";

#[test]
fn fixture_csp_constant_matches_the_fixture_file_byte_for_byte() {
    let bytes = std::fs::read(format!("{STATIC_FIXTURE_DIR}/csp-header.txt")).unwrap();
    assert_eq!(bytes, FIXTURE_CSP.as_bytes());
}

fn assert_static_csp(headers: &HeaderMap, what: &str) {
    assert_eq!(
        header(headers, "content-security-policy", what),
        Some(FIXTURE_CSP),
        "{what}: static responses carry csp-header.txt verbatim, as the only CSP header"
    );
}

fn assert_fallback_csp(headers: &HeaderMap, what: &str) {
    assert_eq!(
        header(headers, "content-security-policy", what),
        Some(MINIMAL_CSP),
        "{what}: with csp-header.txt unusable, static responses carry the minimal fallback"
    );
}

fn assert_full_csp(headers: &HeaderMap, what: &str) {
    let csp = header(headers, "content-security-policy", what)
        .unwrap_or_else(|| panic!("{what}: expected the full header CSP"));
    assert!(csp.contains("default-src 'self'"), "{what}: {csp}");
    assert!(csp.contains("script-src 'self'"), "{what}: {csp}");
    assert!(csp.contains("frame-ancestors 'none'"), "{what}: {csp}");
}

#[tokio::test]
async fn root_html_gets_security_headers_and_the_file_csp() {
    let server = server_with(None);
    let response = server
        .get("/")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    response.assert_status(StatusCode::OK);
    assert!(
        header(response.headers(), "content-type", "/")
            .is_some_and(|ct| ct.starts_with("text/html")),
        "precondition: / must be an HTML document, got {:?}",
        response.headers().get("content-type")
    );
    assert_common_security_headers(response.headers(), "/");
    assert_static_csp(response.headers(), "/");
    assert_eq!(
        header(response.headers(), "content-type", "/"),
        Some("text/html; charset=utf-8")
    );
}

#[tokio::test]
async fn spa_route_gets_security_headers_and_the_file_csp() {
    let server = server_with(None);
    let response = server
        .get("/some/client/side/route")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    assert!(
        response.text().contains("origin-lock test fixture index"),
        "precondition: the SPA route must have reached the index fallback"
    );
    assert_common_security_headers(response.headers(), "SPA route");
    assert_static_csp(response.headers(), "SPA route");
}

#[tokio::test]
async fn static_asset_gets_the_file_csp() {
    let server = server_with(None);
    let response = server
        .get("/asset.txt")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    response.assert_status(StatusCode::OK);
    assert!(
        !header(response.headers(), "content-type", "/asset.txt")
            .unwrap_or_default()
            .contains("charset=utf-8"),
        "charset=utf-8 is added to text/html only: {:?}",
        response.headers().get("content-type")
    );
    assert_common_security_headers(response.headers(), "/asset.txt");
    assert_static_csp(response.headers(), "/asset.txt");
}

#[tokio::test]
async fn conditional_get_of_root_is_304_with_only_the_file_csp() {
    let server = server_with(None);
    let response = server
        .get("/")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .add_header("if-modified-since", FAR_FUTURE)
        .await;
    response.assert_status(StatusCode::NOT_MODIFIED);
    assert!(
        response.headers().get("content-type").is_none(),
        "precondition: a 304 carries no Content-Type (the reason CSP is not keyed on it)"
    );
    assert_common_security_headers(response.headers(), "304 /");
    assert_static_csp(response.headers(), "304 /");
}

#[tokio::test]
async fn conditional_get_of_a_static_asset_is_304_with_only_the_file_csp() {
    let server = server_with(None);
    let response = server
        .get("/asset.txt")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .add_header("if-modified-since", FAR_FUTURE)
        .await;
    response.assert_status(StatusCode::NOT_MODIFIED);
    assert_common_security_headers(response.headers(), "304 /asset.txt");
    assert_static_csp(response.headers(), "304 /asset.txt");
}

/// A copy of the fixture site in a fresh temp dir, without `csp-header.txt`.
fn site_without_csp_file(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("folio-csp-file-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["index.html", "asset.txt"] {
        std::fs::copy(format!("{STATIC_FIXTURE_DIR}/{name}"), dir.join(name)).unwrap();
    }
    dir
}

/// Builds the app over `dir` while capturing startup logs; returns the server
/// and the captured log text.
fn server_capturing_startup_log(dir: &std::path::Path) -> (axum_test::TestServer, String) {
    let (server, buf) =
        common::ai_mock::capture_logs_sync(|| server_in(dir.to_str().unwrap(), None));
    (server, buf.captured())
}

#[tokio::test]
async fn unusable_csp_file_logs_one_error_and_falls_back_to_the_minimal_csp() {
    // (case, csp-header.txt contents; None = file absent)
    let cases: [(&str, Option<&[u8]>, &str); 4] = [
        ("missing", None, "unreadable"),
        ("empty", Some(b""), "empty"),
        (
            "invalid",
            Some(b"frame-ancestors 'none'\x01"),
            "not a valid header value",
        ),
        // Verbatim means no trim: a trailing newline is not a valid value.
        (
            "newline",
            Some(b"frame-ancestors 'none'\n"),
            "not a valid header value",
        ),
    ];
    for (tag, contents, reason) in cases {
        let dir = site_without_csp_file(tag);
        if let Some(bytes) = contents {
            std::fs::write(dir.join("csp-header.txt"), bytes).unwrap();
        }
        let (server, log) = server_capturing_startup_log(&dir);

        assert_eq!(log.matches("ERROR").count(), 1, "{tag}: {log}");
        assert!(
            log.contains(dir.join("csp-header.txt").to_str().unwrap()),
            "{tag}: log names the path: {log}"
        );
        assert!(log.contains(reason), "{tag}: log names the reason: {log}");

        for path in ["/", "/some/spa/route", "/asset.txt"] {
            let response = server
                .get(path)
                .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
                .await;
            assert_fallback_csp(response.headers(), &format!("{tag} {path}"));
        }
        let api = server
            .get("/api/profile")
            .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
            .await;
        assert_full_csp(api.headers(), &format!("{tag} /api/profile"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[tokio::test]
async fn usable_csp_file_logs_no_error() {
    let (_server, log) = server_capturing_startup_log(std::path::Path::new(STATIC_FIXTURE_DIR));
    assert_eq!(log.matches("ERROR").count(), 0, "{log}");
}

#[tokio::test]
async fn health_gets_security_headers_and_the_full_csp() {
    let server = server_with(None);
    let response = server.get("/api/health").await;
    response.assert_status(StatusCode::OK);
    assert_common_security_headers(response.headers(), "/api/health");
    assert_full_csp(response.headers(), "/api/health");
}

#[tokio::test]
async fn routed_json_response_gets_the_full_csp() {
    let server = server_with(None);
    let response = server
        .get("/api/profile")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    response.assert_status(StatusCode::OK);
    assert_common_security_headers(response.headers(), "/api/profile");
    assert_full_csp(response.headers(), "/api/profile");
}

#[tokio::test]
async fn avatar_route_response_gets_the_full_csp() {
    let server = server_with(None);
    let response = server
        .get("/api/avatars/no-such-file.html")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    response.assert_status(StatusCode::NOT_FOUND);
    assert_full_csp(response.headers(), "avatar route");
}

#[tokio::test]
async fn origin_lock_rejection_gets_security_headers_and_the_full_csp() {
    let server = server_with(None);
    let response = server.get("/").await; // no edge secret
    response.assert_status(StatusCode::NOT_FOUND);
    assert!(
        !response.text().contains("origin-lock test fixture index"),
        "precondition: this is origin_lock's own rejection, not the fallback"
    );
    assert_common_security_headers(response.headers(), "origin_lock 404");
    assert_full_csp(response.headers(), "origin_lock 404");
}

// ---------------------------------------------------------------------------
// #4718: SPA routes are 200; unknown /api paths stay a JSON 404
// ---------------------------------------------------------------------------

#[tokio::test]
async fn spa_route_returns_200_with_the_index_page() {
    let server = server_with(None);
    let response = server
        .get("/projects")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await;
    response.assert_status(StatusCode::OK);
    assert!(response.text().contains("origin-lock test fixture index"));
    assert_static_csp(response.headers(), "/projects");
}

#[tokio::test]
async fn unknown_api_paths_return_a_json_404_not_the_index_page() {
    let server = server_with(None);
    for path in [
        "/api/no-such-endpoint",
        "/api/deep/er/no-such",
        "/api",
        "/api/",
    ] {
        let response = server
            .get(path)
            .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
            .await;
        response.assert_status(StatusCode::NOT_FOUND);
        assert!(
            !response.text().contains("origin-lock test fixture index"),
            "{path}: must not serve the index page"
        );
        assert_eq!(response.json::<Value>()["error"], "Not found", "{path}");
        assert_common_security_headers(response.headers(), path);
        assert_full_csp(response.headers(), path);
    }
}

#[tokio::test]
async fn known_api_routes_still_win_over_the_api_catch_all() {
    let server = server_with(None);
    server
        .get("/api/profile")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await
        .assert_status(StatusCode::OK);
    // A known path with a wrong method is the route's 405, not the catch-all 404.
    server
        .delete("/api/profile")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .await
        .assert_status(StatusCode::METHOD_NOT_ALLOWED);
}

// ---------------------------------------------------------------------------
// #4711 F7: production limiter keying
// ---------------------------------------------------------------------------

async fn chat(
    server: &axum_test::TestServer,
    message: &str,
    ip_header: Option<&str>,
    xff: Option<&str>,
) -> axum_test::TestResponse {
    let mut request = server
        .post("/api/chat")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .json(&json!({ "message": message }));
    if let Some(v) = ip_header {
        request = request.add_header(TRUSTED_HEADER, v);
    }
    if let Some(v) = xff {
        request = request.add_header("x-forwarded-for", v);
    }
    request.await
}

#[tokio::test]
async fn chat_limiter_keys_on_the_trusted_header_value() {
    let server = server_with(Some(TRUSTED_HEADER));

    // `rig_client` is None, so an admitted request answers 500 "AI features
    // not configured" AFTER the rate-limit check; a limited one answers 429.
    for i in 1..=10 {
        let r = chat(&server, "hello", Some("203.0.113.7"), None).await;
        r.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            r.json::<Value>()["error"],
            "AI features not configured",
            "request {i} must reach the AI-disabled guard"
        );
    }
    chat(&server, "hello", Some("203.0.113.7"), None)
        .await
        .assert_status(StatusCode::TOO_MANY_REQUESTS);

    // A different header value is a different visitor.
    let other = chat(&server, "hello", Some("203.0.113.8"), None).await;
    other.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn rotating_x_forwarded_for_does_not_escape_the_limit() {
    let server = server_with(Some(TRUSTED_HEADER));

    for i in 1..=10 {
        let xff = format!("198.51.100.{i}");
        chat(&server, "hello", Some("203.0.113.9"), Some(&xff))
            .await
            .assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    }
    // Eleventh request, yet another x-forwarded-for, same trusted header.
    chat(
        &server,
        "hello",
        Some("203.0.113.9"),
        Some("198.51.100.200"),
    )
    .await
    .assert_status(StatusCode::TOO_MANY_REQUESTS);
}

// ---------------------------------------------------------------------------
// #4711 / LLM01: invisible Unicode stripped before the length check
// ---------------------------------------------------------------------------

#[tokio::test]
async fn chat_length_check_counts_text_after_invisible_characters_are_stripped() {
    let server = server_with(Some(TRUSTED_HEADER));
    // 10_000 visible bytes (the cap) plus 300 tag-block and zero-width
    // characters: over the cap raw, exactly at it once stripped.
    let hidden = "\u{E0041}\u{200B}\u{FEFF}".repeat(100);
    let message = format!("{}{hidden}", "a".repeat(10_000));
    assert!(
        message.len() > 10_000,
        "precondition: raw length is over the cap"
    );

    let r = chat(&server, &message, Some("203.0.113.20"), None).await;
    r.assert_status(StatusCode::INTERNAL_SERVER_ERROR); // reached the AI-disabled guard

    // Control: one visible byte more is rejected by the same check.
    let over = "a".repeat(10_001);
    chat(&server, &over, Some("203.0.113.21"), None)
        .await
        .assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn fit_length_check_counts_text_after_invisible_characters_are_stripped() {
    let server = server_with(Some(TRUSTED_HEADER));
    let hidden = "\u{E0041}\u{2060}\u{200D}".repeat(100);
    let jd = format!("{}{hidden}", "a".repeat(15_000));
    // 15_300 bytes of JSON string body is well under the 64 KiB body cap.
    let r = server
        .post("/api/fit")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .add_header(TRUSTED_HEADER, "203.0.113.30")
        .json(&json!({ "job_description": jd }))
        .await;
    r.assert_status(StatusCode::INTERNAL_SERVER_ERROR); // AI-disabled guard, not 400

    let over = "a".repeat(15_001);
    server
        .post("/api/fit")
        .add_header(EDGE_AUTH_HEADER, TEST_SECRET)
        .add_header(TRUSTED_HEADER, "203.0.113.31")
        .json(&json!({ "job_description": over }))
        .await
        .assert_status(StatusCode::BAD_REQUEST);
}
