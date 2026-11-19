//! R-27 (#4853, docs/specs/2026-11-18-live-lists-and-blog-move.md): the article
//! APIs are removed, so each removed path answers like an unknown one. GET,
//! POST, PUT and DELETE on `/api/articles`, `/api/articles/x`,
//! `/api/admin/articles` and `/api/admin/articles/1` each return status 404,
//! `Content-Type: application/json` and a body equal to
//! `{"error":"Not found"}`, both with and without an authenticated admin
//! session (4 paths x 4 methods x 2 auth states).
//!
//! Driven through the production router (`common::build_app_server`), the only
//! wiring that carries the `/api` API-miss 404. Every request carries the edge
//! header, the login included. The admin password is the per-process generated
//! one behind `common::admin_login`; no credential literal lives here.
//!
//! Controls: the seeded database still holds an article row (this pins intake
//! NG-2: the table and its rows stay); a removed path answers exactly what an
//! unknown `/api` path answers, under both auth states; and the same admin
//! token gets 200 on a live admin path, so a 404 is not an auth failure.

mod common;

use axum::http::{HeaderValue, Method, StatusCode, header::AUTHORIZATION, header::CONTENT_TYPE};
use common::{EDGE_AUTH_HEADER, TEST_EDGE_SECRET};

fn bearer(token: &str) -> HeaderValue {
    format!("Bearer {token}")
        .parse()
        .expect("token must produce a valid header value")
}

fn edge_server(tag: &str) -> (axum_test::TestServer, std::path::PathBuf) {
    let dir = common::temp_static_site(tag, None);
    let mut server = common::build_app_server(&dir);
    server.add_header(EDGE_AUTH_HEADER, TEST_EDGE_SECRET);
    (server, dir)
}

/// Send `method` to `path`, without and then with an admin session, and assert
/// the exact API-miss 404 each time.
async fn assert_api_miss_both_auth_states(method: Method, path: &str, tag: &str) {
    for authed in [false, true] {
        // Given the production app and, when `authed`, a real admin session
        let (server, dir) = edge_server(&format!("{tag}-{authed}"));
        let token = if authed {
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

        // Then it is the API-miss 404, exactly
        let ctx = format!("{method} {path} authed={authed}");
        response.assert_status(StatusCode::NOT_FOUND);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(|v| v.to_str().unwrap()),
            Some("application/json"),
            "{ctx}: Content-Type"
        );
        let text = response.text();
        let body: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{ctx}: body is not JSON ({e}): {text:?}"));
        assert_eq!(
            body,
            serde_json::json!({"error": "Not found"}),
            "{ctx}: body"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

macro_rules! r27_scenario {
    ($name:ident, $method:expr, $path:expr) => {
        #[tokio::test]
        async fn $name() {
            assert_api_miss_both_auth_states($method, $path, stringify!($name)).await;
        }
    };
}

r27_scenario!(
    r27_get_api_articles_is_the_api_miss_404,
    Method::GET,
    "/api/articles"
);
r27_scenario!(
    r27_post_api_articles_is_the_api_miss_404,
    Method::POST,
    "/api/articles"
);
r27_scenario!(
    r27_put_api_articles_is_the_api_miss_404,
    Method::PUT,
    "/api/articles"
);
r27_scenario!(
    r27_delete_api_articles_is_the_api_miss_404,
    Method::DELETE,
    "/api/articles"
);

r27_scenario!(
    r27_get_api_articles_x_is_the_api_miss_404,
    Method::GET,
    "/api/articles/x"
);
r27_scenario!(
    r27_post_api_articles_x_is_the_api_miss_404,
    Method::POST,
    "/api/articles/x"
);
r27_scenario!(
    r27_put_api_articles_x_is_the_api_miss_404,
    Method::PUT,
    "/api/articles/x"
);
r27_scenario!(
    r27_delete_api_articles_x_is_the_api_miss_404,
    Method::DELETE,
    "/api/articles/x"
);

r27_scenario!(
    r27_get_api_admin_articles_is_the_api_miss_404,
    Method::GET,
    "/api/admin/articles"
);
r27_scenario!(
    r27_post_api_admin_articles_is_the_api_miss_404,
    Method::POST,
    "/api/admin/articles"
);
r27_scenario!(
    r27_put_api_admin_articles_is_the_api_miss_404,
    Method::PUT,
    "/api/admin/articles"
);
r27_scenario!(
    r27_delete_api_admin_articles_is_the_api_miss_404,
    Method::DELETE,
    "/api/admin/articles"
);

r27_scenario!(
    r27_get_api_admin_articles_id_is_the_api_miss_404,
    Method::GET,
    "/api/admin/articles/1"
);
r27_scenario!(
    r27_post_api_admin_articles_id_is_the_api_miss_404,
    Method::POST,
    "/api/admin/articles/1"
);
r27_scenario!(
    r27_put_api_admin_articles_id_is_the_api_miss_404,
    Method::PUT,
    "/api/admin/articles/1"
);
r27_scenario!(
    r27_delete_api_admin_articles_id_is_the_api_miss_404,
    Method::DELETE,
    "/api/admin/articles/1"
);

/// Pins intake NG-2: the removal keeps the `articles` table and its rows.
#[test]
fn r27_control_the_articles_table_keeps_a_row() {
    // Given the seeded database
    let conn = common::seeded_db();
    // When the articles are counted
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM articles", [], |r| r.get(0))
        .expect("the articles table must still exist");
    // Then at least one row remains
    assert!(n >= 1, "seeded articles table is empty");
}

/// (status, content-type, body text) of one GET, with or without an admin session.
async fn get_triple(path: &str, authed: bool, tag: &str) -> (StatusCode, Option<String>, String) {
    let (server, dir) = edge_server(tag);
    let mut req = server.get(path);
    if authed {
        let token = common::admin_login(&server).await;
        req = req.add_header(AUTHORIZATION, bearer(&token));
    }
    let response = req.await;
    let triple = (
        response.status_code(),
        response
            .headers()
            .get(CONTENT_TYPE)
            .map(|v| v.to_str().unwrap().to_string()),
        response.text(),
    );
    std::fs::remove_dir_all(&dir).unwrap();
    triple
}

/// R-27: a removed path answers the same JSON 404 as any unknown `/api` path.
/// `event-sourcing` is the seeded article slug (`db/seed.rs`), so a
/// reintroduced `/api/articles/{slug}` with its own miss arm cannot pass.
#[tokio::test]
async fn r27_control_removed_paths_match_an_unknown_api_path_in_both_auth_states() {
    for authed in [false, true] {
        // Given the answer for an unknown `/api` path under this auth state
        let unknown = get_triple("/api/zz-unknown", authed, "r27-diff-unknown").await;
        // When each removed path is requested under the same state
        for path in [
            "/api/articles",
            "/api/articles/x",
            "/api/articles/event-sourcing",
            "/api/admin/articles",
            "/api/admin/articles/1",
        ] {
            let got = get_triple(path, authed, "r27-diff-removed").await;
            // Then status, content type and body are identical
            assert_eq!(got, unknown, "GET {path} authed={authed}");
        }
    }
}

#[tokio::test]
async fn r27_control_the_same_token_gets_200_on_a_live_admin_path() {
    // Given an authenticated session through the production app
    let (server, dir) = edge_server("r27-control-live-admin");
    let token = common::admin_login(&server).await;
    // When a live admin path is requested with that token
    let response = server
        .get("/api/admin/dashboard")
        .add_header(AUTHORIZATION, bearer(&token))
        .await;
    // Then it is 200, so a 404 on a removed admin path is not an auth failure
    response.assert_status(StatusCode::OK);
    std::fs::remove_dir_all(&dir).unwrap();
}
