//! R-29 (#4853, plan T15): the static service gives `/projects.json` no special handling.
//! The list is served by `/api/lists/projects` now; a stray `projects.json` in the static
//! directory is an ordinary static file. Asserted through `site_core::app::build_app`,
//! the router `cmd/server` serves: no response for the path, in any spelling `ServeDir`
//! resolves to it, carries a `Cache-Control` the server added.

mod common;

use axum::http::StatusCode;
use common::{EDGE_AUTH_HEADER, TEST_EDGE_SECRET};

const FAR_FUTURE: &str = "Fri, 01 Jan 2100 00:00:00 GMT";
const PROJECTS_JSON: &str = r#"{"fetched_at":"2026-11-13T00:00:00.000Z","groups":[]}"#;

/// Every Cache-Control value on the response, in order.
fn cache_control(headers: &axum::http::HeaderMap) -> Vec<&str> {
    headers
        .get_all("cache-control")
        .iter()
        .map(|v| v.to_str().unwrap())
        .collect()
}

#[tokio::test]
async fn projects_json_200_gets_no_server_cache_header() {
    let dir = common::temp_static_site("nsc-200", Some(PROJECTS_JSON));
    let response = common::build_app_server(&dir)
        .get("/projects.json")
        .add_header(EDGE_AUTH_HEADER, TEST_EDGE_SECRET)
        .await;
    response.assert_status(StatusCode::OK);
    assert_eq!(
        response.text(),
        PROJECTS_JSON,
        "precondition: the file itself"
    );
    assert!(
        cache_control(response.headers()).is_empty(),
        "unexpected Cache-Control {:?}",
        response.headers().get_all("cache-control")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn projects_json_304_and_head_get_no_server_cache_header() {
    let dir = common::temp_static_site("nsc-304", Some(PROJECTS_JSON));
    let server = common::build_app_server(&dir);
    let not_modified = server
        .get("/projects.json")
        .add_header(EDGE_AUTH_HEADER, TEST_EDGE_SECRET)
        .add_header("if-modified-since", FAR_FUTURE)
        .await;
    not_modified.assert_status(StatusCode::NOT_MODIFIED);
    assert!(cache_control(not_modified.headers()).is_empty());

    let head = server
        .method(axum::http::Method::HEAD, "/projects.json")
        .add_header(EDGE_AUTH_HEADER, TEST_EDGE_SECRET)
        .await;
    head.assert_status(StatusCode::OK);
    assert_eq!(head.header("content-type"), "application/json");
    assert!(cache_control(head.headers()).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Spellings `ServeDir` resolves to the file. The body is the control: it proves the
/// alias reached the file, so an empty header list is not a miss.
#[tokio::test]
async fn every_servedir_alias_of_projects_json_gets_no_server_cache_header() {
    let dir = common::temp_static_site("nsc-alias", Some(PROJECTS_JSON));
    let server = common::build_app_server(&dir);
    for path in [
        "//projects.json",
        "/projects%2Ejson",
        "/%70rojects.json",
        "/projects.json/",
        "/projects.json%2F",
    ] {
        let response = server
            .get(path)
            .add_header(EDGE_AUTH_HEADER, TEST_EDGE_SECRET)
            .await;
        response.assert_status(StatusCode::OK);
        assert_eq!(
            response.text(),
            PROJECTS_JSON,
            "{path}: must reach the file"
        );
        assert!(
            cache_control(response.headers()).is_empty(),
            "{path}: unexpected Cache-Control {:?}",
            response.headers().get_all("cache-control")
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
