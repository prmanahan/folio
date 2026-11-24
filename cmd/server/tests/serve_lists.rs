//! Spawned-binary cases for the live-lists core (spec R-5, plan T8, D12).
//!
//! Black-box: the built `folio` binary runs as `folio serve` (see
//! `common::server`) and every assertion is on what it answers over HTTP.
//!
//! The binary has no source override (R-5), so its refresher may reach the
//! real GitHub and mnemra.dev, or fail to, and these cases cannot choose
//! which. Each asserts only a property that holds either way: no case reads
//! a list's content, and the freshness check reads keys, never times.
//!
//! One scenario per test; each spawns its own server.

mod common;

use common::Sandbox;
use common::server::{HEALTH_DEADLINE, spawn_serve};

#[test]
fn r5_health_answers_200_within_the_fifteen_second_grace_period() {
    // Given a fresh sandbox and the variable unset
    let sb = Sandbox::new();
    // When the binary is spawned
    let server = spawn_serve(&sb);
    // Then /api/health answered 200 within 15 s of spawn (spawn_serve fails
    // otherwise); and it still answers, so the process did not just flap up
    assert!(server.time_to_healthy <= HEALTH_DEADLINE);
    let health = server.get("/api/health");
    assert_eq!((health.status, health.body.as_str()), (200, "ok"));
}

#[test]
fn r22_freshness_has_exactly_its_three_keys() {
    let sb = Sandbox::new();
    let server = spawn_serve(&sb);
    // When /api/lists/freshness is read with the edge secret
    let response = server.get("/api/lists/freshness");
    // Then it answers 200 and no-store, with exactly projects, articles and started_at
    assert_eq!(response.status, 200, "{}", response.body);
    assert_eq!(response.header("cache-control"), Some("no-store"));
    let body: serde_json::Value = serde_json::from_str(&response.body).expect("a JSON object");
    let mut keys: Vec<&str> = body
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["articles", "projects", "started_at"]);
    // and started_at is always a string, whatever the sources did
    assert!(body["started_at"].is_string(), "{body}");
}

#[test]
fn r25_articles_answers_the_301_to_the_blog() {
    let sb = Sandbox::new();
    let server = spawn_serve(&sb);
    let response = server.get("/articles");
    assert_eq!(response.status, 301);
    assert_eq!(
        response.header("location"),
        Some("https://mnemra.dev/blog/")
    );
}

#[test]
fn removed_api_articles_answers_the_json_404() {
    let sb = Sandbox::new();
    let server = spawn_serve(&sb);
    let response = server.get("/api/articles");
    assert_eq!(response.status, 404);
    assert_eq!(response.body, r#"{"error":"Not found"}"#);
    assert!(
        response
            .header("content-type")
            .is_some_and(|v| v.starts_with("application/json")),
        "{:?}",
        response.headers
    );
}
