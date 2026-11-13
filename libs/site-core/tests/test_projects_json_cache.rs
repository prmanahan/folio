//! R-11 (#4726): responses for `/projects.json` carry `Cache-Control: no-cache`,
//! asserted through `site_core::app::build_app`, the router `cmd/server` serves,
//! on a 200, a 304 and a HEAD, and on every spelling of the path that
//! `ServeDir` resolves to the same file. Every other static path is left as
//! `ServeDir` set it.

mod common;

use axum::http::StatusCode;
use common::{EDGE_AUTH_HEADER, TEST_EDGE_SECRET};

const FAR_FUTURE: &str = "Fri, 01 Jan 2100 00:00:00 GMT";
/// Spec-valid list file (Data Model): two groups in order, `fixture-` repos,
/// `name` and `description` only.
const PROJECTS_JSON: &str = r#"{"fetched_at":"2026-11-13T00:00:00.000Z","groups":[{"owner":"mnemra","repos":[{"name":"fixture-alpha","description":"A fixture repository"}]},{"owner":"prmanahan","repos":[{"name":"fixture-beta","description":""}]}]}"#;
const INDEX_BODY: &str = "origin-lock test fixture index";

/// Every Cache-Control value on the response, in order.
fn cache_control(headers: &axum::http::HeaderMap) -> Vec<&str> {
    headers
        .get_all("cache-control")
        .iter()
        .map(|v| v.to_str().unwrap())
        .collect()
}

#[tokio::test]
async fn projects_json_200_carries_no_cache() {
    let dir = common::temp_static_site("200", Some(PROJECTS_JSON));
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
    assert_eq!(cache_control(response.headers()), ["no-cache"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn projects_json_304_carries_no_cache() {
    let dir = common::temp_static_site("304", Some(PROJECTS_JSON));
    let response = common::build_app_server(&dir)
        .get("/projects.json")
        .add_header(EDGE_AUTH_HEADER, TEST_EDGE_SECRET)
        .add_header("if-modified-since", FAR_FUTURE)
        .await;
    response.assert_status(StatusCode::NOT_MODIFIED);
    assert_eq!(cache_control(response.headers()), ["no-cache"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn projects_json_head_carries_no_cache() {
    let dir = common::temp_static_site("head", Some(PROJECTS_JSON));
    let response = common::build_app_server(&dir)
        .method(axum::http::Method::HEAD, "/projects.json")
        .add_header(EDGE_AUTH_HEADER, TEST_EDGE_SECRET)
        .await;
    response.assert_status(StatusCode::OK);
    assert_eq!(
        response.header("content-type"),
        "application/json",
        "precondition: the file was served, not the SPA page"
    );
    assert_eq!(cache_control(response.headers()), ["no-cache"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Production's response until go-live: no list file, so the SPA fallback
/// answers the path.
#[tokio::test]
async fn absent_projects_json_falls_back_to_the_index_with_no_cache() {
    let dir = common::temp_static_site("absent", None);
    let response = common::build_app_server(&dir)
        .get("/projects.json")
        .add_header(EDGE_AUTH_HEADER, TEST_EDGE_SECRET)
        .await;
    response.assert_status(StatusCode::OK);
    assert!(response.text().contains(INDEX_BODY), "SPA index expected");
    assert_eq!(cache_control(response.headers()), ["no-cache"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Spellings `ServeDir` resolves to the file (trims leading `/`,
/// percent-decodes, skips `.`, drops a trailing `/`). The body is the control:
/// it proves the alias reached the file and did not fall through to the SPA.
///
/// `/./projects.json` and `/projects.json/.` are not here: axum_test's URL
/// parser normalises them to `/projects.json` and `/projects.json/` before the
/// app sees them (measured: with a raw-path matcher that knew only
/// `/projects.json`, `//projects.json` and `/projects.json/`, both still
/// passed). The `.` forms are covered by the unit test on
/// `resolves_to_no_cache_file` and by the curl `--path-as-is` real run.
#[tokio::test]
async fn every_servedir_alias_of_projects_json_carries_no_cache() {
    let dir = common::temp_static_site("alias", Some(PROJECTS_JSON));
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
            "{path}: must reach the file, not the SPA page"
        );
        assert_eq!(cache_control(response.headers()), ["no-cache"], "{path}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn other_static_paths_are_unchanged() {
    let dir = common::temp_static_site("other", Some(PROJECTS_JSON));
    let server = common::build_app_server(&dir);
    for path in ["/asset.txt", "/projects.json.bak", "/x/projects.json"] {
        let response = server
            .get(path)
            .add_header(EDGE_AUTH_HEADER, TEST_EDGE_SECRET)
            .await;
        assert!(
            cache_control(response.headers()).is_empty(),
            "{path}: unexpected Cache-Control {:?}",
            response.headers().get_all("cache-control")
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
