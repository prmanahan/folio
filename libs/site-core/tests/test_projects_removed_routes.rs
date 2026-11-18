//! R-15 (#4726, docs/specs/2026-11-11-projects-from-github.md): a removed API
//! path answers like an unknown one. GET, POST, PUT and DELETE on
//! `/api/projects`, `/api/projects/{slug}`, `/api/admin/projects` and
//! `/api/admin/projects/{id}` each return status 404, `Content-Type:
//! application/json`, and a body that parses as JSON equal to
//! `{"error":"Not found"}`, which holds none of the seeded row's fields. The
//! admin paths are requested under an authenticated session.
//!
//! Driven through the production router, `common::build_app_server` over
//! `common::temp_static_site` (`build_app` is the only wiring that carries the
//! `/api` API-miss 404). Every request carries the edge header
//! (`x-folio-edge-auth`), the login included; without it the origin lock
//! answers a bodyless 404, which the controls below use to tell the two 404s
//! apart.
//!
//! Sanctioned reads, declared: `db/seed.rs` for the published project row the
//! seeded database holds, slug `eventflow`, id 1 (the first and only
//! `projects` insert); `routes/admin/dashboard.rs` for a live admin path
//! (`GET /api/admin/dashboard`) to use as the 200 control; `common/mod.rs` for
//! the harness helpers. The 404 body and content type are asserted from the
//! spec's text, not read from the handler.
//!
//! Controls (each its own scenario):
//! - an unknown `/api` path gets the same 404 and body;
//! - a request without the edge header gets a 404 with no body, so the 404s
//!   are distinguishable;
//! - the same token gets 200 on a live admin path, so a 404 on a removed
//!   admin path is not an auth failure in disguise.
//!
//! Scenario names are `r15_<verb>_<path>_is_the_api_miss_404`.

mod common;

use axum::http::{HeaderValue, Method, StatusCode, header::AUTHORIZATION, header::CONTENT_TYPE};
use common::{EDGE_AUTH_HEADER, TEST_EDGE_SECRET};

/// The published row `db/seed.rs` inserts into `projects`.
const SEEDED_SLUG: &str = "eventflow";
const SEEDED_ID: u32 = 1;
/// Distinctive fields of that row: none may reach a removed path's response.
const SEEDED_ROW_FIELDS: [&str; 5] = [
    "EventFlow",
    "eventflow",
    "Event processing framework.",
    "Built at Meridian.",
    "Kafka",
];

fn bearer(token: &str) -> HeaderValue {
    format!("Bearer {token}")
        .parse()
        .expect("token must produce a valid header value")
}

/// A production-router server whose every request carries the edge header.
fn edge_server(tag: &str) -> (axum_test::TestServer, std::path::PathBuf) {
    let dir = common::temp_static_site(tag, None);
    let mut server = common::build_app_server(&dir);
    server.add_header(EDGE_AUTH_HEADER, TEST_EDGE_SECRET);
    (server, dir)
}

/// Send `method` to `path` and assert the API-miss 404: status, content type,
/// and a body JSON-equal to `{"error":"Not found"}` holding no seeded field.
async fn assert_api_miss(method: Method, path: &str, admin: bool, tag: &str) {
    // Given the production app and, for admin paths, an authenticated session
    let (server, dir) = edge_server(tag);
    let token = if admin {
        Some(common::admin_login(&server).await)
    } else {
        None
    };

    // When the verb is sent to the removed path
    let mut req = server.method(method.clone(), path);
    if let Some(t) = &token {
        req = req.add_header(AUTHORIZATION, bearer(t));
    }
    let response = req.json(&serde_json::json!({"title": "x"})).await;

    // Then it is the API-miss 404
    response.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .map(|v| v.to_str().unwrap()),
        Some("application/json"),
        "{method} {path}: Content-Type"
    );
    let text = response.text();
    let body: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{method} {path}: body is not JSON ({e}): {text:?}"));
    assert_eq!(
        body,
        serde_json::json!({"error": "Not found"}),
        "{method} {path}: body"
    );
    for field in SEEDED_ROW_FIELDS {
        assert!(
            !text.contains(field),
            "{method} {path}: body holds seeded field {field:?}: {text}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

macro_rules! r15_scenario {
    ($name:ident, $method:expr, $path:expr, $admin:expr) => {
        #[tokio::test]
        async fn $name() {
            assert_api_miss($method, &$path, $admin, stringify!($name)).await;
        }
    };
}

// Public list
r15_scenario!(
    r15_get_api_projects_is_the_api_miss_404,
    Method::GET,
    "/api/projects",
    false
);
r15_scenario!(
    r15_post_api_projects_is_the_api_miss_404,
    Method::POST,
    "/api/projects",
    false
);
r15_scenario!(
    r15_put_api_projects_is_the_api_miss_404,
    Method::PUT,
    "/api/projects",
    false
);
r15_scenario!(
    r15_delete_api_projects_is_the_api_miss_404,
    Method::DELETE,
    "/api/projects",
    false
);

// Public by slug
r15_scenario!(
    r15_get_api_projects_slug_is_the_api_miss_404,
    Method::GET,
    format!("/api/projects/{SEEDED_SLUG}"),
    false
);
r15_scenario!(
    r15_post_api_projects_slug_is_the_api_miss_404,
    Method::POST,
    format!("/api/projects/{SEEDED_SLUG}"),
    false
);
r15_scenario!(
    r15_put_api_projects_slug_is_the_api_miss_404,
    Method::PUT,
    format!("/api/projects/{SEEDED_SLUG}"),
    false
);
r15_scenario!(
    r15_delete_api_projects_slug_is_the_api_miss_404,
    Method::DELETE,
    format!("/api/projects/{SEEDED_SLUG}"),
    false
);

// Admin collection, authenticated
r15_scenario!(
    r15_get_api_admin_projects_is_the_api_miss_404,
    Method::GET,
    "/api/admin/projects",
    true
);
r15_scenario!(
    r15_post_api_admin_projects_is_the_api_miss_404,
    Method::POST,
    "/api/admin/projects",
    true
);
r15_scenario!(
    r15_put_api_admin_projects_is_the_api_miss_404,
    Method::PUT,
    "/api/admin/projects",
    true
);
r15_scenario!(
    r15_delete_api_admin_projects_is_the_api_miss_404,
    Method::DELETE,
    "/api/admin/projects",
    true
);

// Admin by id, authenticated
r15_scenario!(
    r15_get_api_admin_projects_id_is_the_api_miss_404,
    Method::GET,
    format!("/api/admin/projects/{SEEDED_ID}"),
    true
);
r15_scenario!(
    r15_post_api_admin_projects_id_is_the_api_miss_404,
    Method::POST,
    format!("/api/admin/projects/{SEEDED_ID}"),
    true
);
r15_scenario!(
    r15_put_api_admin_projects_id_is_the_api_miss_404,
    Method::PUT,
    format!("/api/admin/projects/{SEEDED_ID}"),
    true
);
r15_scenario!(
    r15_delete_api_admin_projects_id_is_the_api_miss_404,
    Method::DELETE,
    format!("/api/admin/projects/{SEEDED_ID}"),
    true
);

#[tokio::test]
async fn r15_control_unknown_api_path_gets_the_same_404_and_body() {
    // Given the production app
    let (server, dir) = edge_server("r15-control-unknown");
    // When an unknown `/api` path is requested
    let response = server.get("/api/zz-unknown").await;
    // Then it answers exactly what the removed paths answer
    response.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .map(|v| v.to_str().unwrap()),
        Some("application/json")
    );
    assert_eq!(
        response.json::<serde_json::Value>(),
        serde_json::json!({"error": "Not found"})
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn r15_control_request_without_the_edge_header_gets_a_bodyless_404() {
    // Given the production app and a request with no edge header
    let dir = common::temp_static_site("r15-control-no-edge", None);
    let server = common::build_app_server(&dir);
    // When a removed path is requested
    let response = server.get("/api/projects").await;
    // Then the origin lock answers 404 with no body: not the API-miss JSON
    response.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(
        response.text(),
        "",
        "the origin-lock 404 must be bodyless, so it differs from the API-miss 404"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn r15_control_the_same_token_gets_200_on_a_live_admin_path() {
    // Given an authenticated session through the production app
    let (server, dir) = edge_server("r15-control-live-admin");
    let token = common::admin_login(&server).await;
    // When a live admin path is requested with the same token and edge header
    let response = server
        .get("/api/admin/dashboard")
        .add_header(AUTHORIZATION, bearer(&token))
        .await;
    // Then it is 200, so a 404 on a removed admin path is not an auth failure
    response.assert_status(StatusCode::OK);
    std::fs::remove_dir_all(&dir).unwrap();
}
