//! Admin-surface black-box tests for spec #2715 (experiences.visible flag).
//!
//! docs/specs/2026-07-24-experiences-visible-flag.md — covers S3 (R-0005
//! round-trip preservation), S4 (R-0004 stale-client fail-loud), and the
//! HTTP/persistence half of S7 (R-0006 — hiding an existing public row).
//!
//! All requests are raw JSON (`serde_json::Value`), never `ExperienceInput`
//! struct literals — this crate's `ExperienceInput` does not carry `visible`
//! until the implementer's green phase (R-0004), and S4 specifically MUST be
//! expressed as raw JSON: a Rust struct literal cannot omit a required field
//! and compile (spec Notes, "ownership split").
//!
//! Auth via `common::admin_login`, a shared helper written for this dispatch
//! (spec Notes — "a shared admin-auth helper... it does not exist yet").

mod common;

use axum::http::{HeaderValue, StatusCode, header::AUTHORIZATION};

fn bearer(token: &str) -> HeaderValue {
    format!("Bearer {token}")
        .parse()
        .expect("token must produce a valid header value")
}

/// The full admin `ExperienceInput`-shaped JSON body this repo's admin API
/// accepts, including the forward-looking `visible` key (R-0004's future
/// required field). Pre-006, `ExperienceInput` does not have a `visible`
/// field yet, so this extra key is silently ignored by serde (no
/// `deny_unknown_fields`) — the fixture is written once, correct both before
/// and after the implementer's green phase.
fn full_valid_experience_body() -> serde_json::Value {
    serde_json::json!({
        "company_name": "Full Co",
        "title": "Engineer",
        "location": "Remote",
        "start_date": "2020-01",
        "end_date": null,
        "is_current": true,
        "summary": "Built things.",
        "bullet_points": [],
        "display_order": 1,
        "title_progression": "IC2",
        "quantified_impact": {},
        "why_joined": "",
        "why_left": "",
        "actual_contributions": "",
        "proudest_achievement": "",
        "would_do_differently": "",
        "challenges_faced": "",
        "lessons_learned": "",
        "manager_would_say": "",
        "reports_would_say": "",
        "visible": true
    })
}

/// Return a copy of `body` with `key` removed. Used to derive the "missing
/// company_name" oracle body and the "missing visible" (pre-006 stale
/// client) body from a single source fixture.
fn without_key(mut body: serde_json::Value, key: &str) -> serde_json::Value {
    body.as_object_mut()
        .expect("fixture body must be a JSON object")
        .remove(key);
    body
}

// ===========================================================================
// S4 / R-0004 — stale client fails loud (no visible key -> rejected)
// ===========================================================================

/// Given an authenticated admin client
/// When it POSTs the pre-006 object shape (every field this API accepts
///   today, but no `visible` key)
/// Then the request is rejected with the SAME status this API already
///   returns today for a body missing any other required field (the oracle
///   is `company_name` — its value is not asserted, only used as the
///   comparison target, per the spec's black-box discipline: "do not assert
///   SQL text" generalizes to "do not assert implementation-derived status
///   codes")
///
/// Red-phase failure (behavioral, not structural): pre-006, `ExperienceInput`
/// has no `visible` field at all, so the "body omitting visible" IS today's
/// full valid shape and this POST currently SUCCEEDS (201) instead of
/// matching the oracle's rejection status.
#[tokio::test]
async fn admin_post_omitting_visible_is_rejected_same_as_a_body_missing_company_name() {
    let (server, _state) = common::test_app_with_state();
    let token = common::admin_login(&server).await;

    // Oracle: this API's existing behavior for ANY missing required field.
    let oracle_body = without_key(full_valid_experience_body(), "company_name");
    let oracle_response = server
        .post("/api/admin/experience")
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&oracle_body)
        .await;
    let oracle_status = oracle_response.status_code();

    // When: a stale (pre-006) client POSTs the pre-006 object shape.
    let stale_body = without_key(full_valid_experience_body(), "visible");
    let response = server
        .post("/api/admin/experience")
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&stale_body)
        .await;

    // Then
    assert_eq!(
        response.status_code(),
        oracle_status,
        "S4/R-0004: a body omitting `visible` must be rejected with the \
         same status this API returns for a body missing company_name \
         (oracle status: {oracle_status}); got {} — pre-006 this create \
         likely SUCCEEDS today (a stale-client body IS today's full valid \
         shape), which is the expected behavioral red",
        response.status_code()
    );
    assert_ne!(
        response.status_code(),
        StatusCode::CREATED,
        "S4/R-0004: a body omitting `visible` MUST NOT succeed with an \
         implied value"
    );
}

/// Given an authenticated admin client and an existing row
/// When it PUTs the pre-006 object shape (no `visible` key) over that row
/// Then the request is rejected (same-status oracle as the POST test above)
///   AND the stored row is byte-unchanged — the rejection must not silently
///   mutate anything
///
/// Red-phase failure (behavioral): pre-006, this PUT currently SUCCEEDS
/// (200), so both the "rejected" and "unchanged" assertions are moot in the
/// sense that nothing was rejected — the status-equality assertion is what
/// fails.
#[tokio::test]
async fn admin_put_omitting_visible_is_rejected_and_leaves_the_stored_row_unchanged() {
    let (server, _state) = common::test_app_with_state();
    let token = common::admin_login(&server).await;

    // Given: an existing row.
    let create_response = server
        .post("/api/admin/experience")
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&full_valid_experience_body())
        .await;
    create_response.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_response.json();
    let id = created["id"].as_i64().expect("created row must have an id");

    let before_get = server
        .get(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .await;
    before_get.assert_status_ok();
    let before: serde_json::Value = before_get.json();

    // Oracle: PUT missing company_name over the same row.
    let oracle_body = without_key(full_valid_experience_body(), "company_name");
    let oracle_response = server
        .put(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&oracle_body)
        .await;
    let oracle_status = oracle_response.status_code();

    // When: a stale client PUTs the pre-006 shape (no `visible`).
    let stale_body = without_key(full_valid_experience_body(), "visible");
    let put_response = server
        .put(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&stale_body)
        .await;

    // Then: rejected with the same status as the oracle...
    assert_eq!(
        put_response.status_code(),
        oracle_status,
        "S4/R-0004: a PUT omitting `visible` must be rejected with the same \
         status as a PUT missing company_name (oracle: {oracle_status}); \
         got {}",
        put_response.status_code()
    );

    // ...and the stored row is unchanged.
    let after_get = server
        .get(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .await;
    after_get.assert_status_ok();
    let after: serde_json::Value = after_get.json();
    assert_eq!(
        before, after,
        "S4/R-0004: the stored row MUST be unchanged after a rejected PUT \
         omitting `visible`"
    );
}

// ===========================================================================
// S3 / R-0005 — round-trip preserves visible = 0
// ===========================================================================

/// Given a row with `visible = 0`
/// When an authenticated admin GETs the full object and PUTs it back
///   completely unmodified
/// Then the re-fetched object still has `visible = false`, and the full
///   object is byte-equal across the round-trip — no field exclusions
///   (R-0005's acceptance is explicit that none are needed: `update`'s SET
///   list does not write `created_at`/`id`, and the JSON columns
///   re-serialize canonically on both GETs)
///
/// Red-phase failure (behavioral): pre-006, `ExperienceFull` does not
/// serialize a `visible` key at all, so `second_get_body["visible"]` reads
/// as JSON null / `.as_bool()` is `None`, not `Some(false)`.
#[tokio::test]
async fn admin_round_trip_preserves_visible_false_on_an_unmodified_put() {
    let (server, _state) = common::test_app_with_state();
    let token = common::admin_login(&server).await;

    let mut create_body = full_valid_experience_body();
    create_body["company_name"] = serde_json::json!("Hidden Round Trip Co");
    create_body["visible"] = serde_json::json!(false);
    let create_response = server
        .post("/api/admin/experience")
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&create_body)
        .await;
    create_response.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_response.json();
    let id = created["id"].as_i64().expect("created row must have an id");

    let first_get = server
        .get(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .await;
    first_get.assert_status_ok();
    let first_get_body: serde_json::Value = first_get.json();

    // PUT it back completely unmodified. `id`/`created_at` are read-only
    // fields the GET response carries but ExperienceInput's write shape does
    // not accept; strip them before round-tripping.
    let mut put_body = first_get_body.clone();
    let put_body_obj = put_body
        .as_object_mut()
        .expect("GET response body must be a JSON object");
    put_body_obj.remove("id");
    put_body_obj.remove("created_at");

    let put_response = server
        .put(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&put_body)
        .await;
    put_response.assert_status_ok();

    let second_get = server
        .get(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .await;
    second_get.assert_status_ok();
    let second_get_body: serde_json::Value = second_get.json();

    assert_eq!(
        second_get_body["visible"].as_bool(),
        Some(false),
        "R-0005/S3: visible = false must survive an unmodified round-trip \
         PUT, got: {:?}",
        second_get_body.get("visible")
    );
    assert_eq!(
        first_get_body, second_get_body,
        "R-0005/S3: the full object must be byte-equal across an \
         unmodified round-trip — no field exclusions"
    );
}

/// Given a row with `visible = 0`
/// When the SAME cycle mutates one unrelated field (`summary`)
/// Then `visible = false` is still preserved (R-0005's second acceptance
///   bullet — the full-object-overwrite update path does not require every
///   field to be re-sent unchanged to keep `visible` from being clobbered)
#[tokio::test]
async fn admin_round_trip_mutating_unrelated_field_still_preserves_visible_false() {
    let (server, _state) = common::test_app_with_state();
    let token = common::admin_login(&server).await;

    let mut create_body = full_valid_experience_body();
    create_body["company_name"] = serde_json::json!("Hidden Mutate Co");
    create_body["visible"] = serde_json::json!(false);
    let create_response = server
        .post("/api/admin/experience")
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&create_body)
        .await;
    create_response.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_response.json();
    let id = created["id"].as_i64().expect("created row must have an id");

    let first_get = server
        .get(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .await;
    first_get.assert_status_ok();
    let mut put_body: serde_json::Value = first_get.json();
    let put_body_obj = put_body
        .as_object_mut()
        .expect("GET response body must be a JSON object");
    put_body_obj.remove("id");
    put_body_obj.remove("created_at");
    put_body_obj.insert(
        "summary".to_string(),
        serde_json::json!("Updated summary text."),
    );

    let put_response = server
        .put(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&put_body)
        .await;
    put_response.assert_status_ok();

    let second_get = server
        .get(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .await;
    second_get.assert_status_ok();
    let second_get_body: serde_json::Value = second_get.json();

    assert_eq!(
        second_get_body["summary"].as_str(),
        Some("Updated summary text."),
        "sanity: the unrelated field mutation must have applied"
    );
    assert_eq!(
        second_get_body["visible"].as_bool(),
        Some(false),
        "R-0005: visible = false must survive a round-trip that mutates an \
         unrelated field, got: {:?}",
        second_get_body.get("visible")
    );
}

// ===========================================================================
// R-0004 acceptance bullet 1 — the admin LIST endpoint (not just get_by_id)
// returns `visible` for every row, including hidden ones (unfiltered).
// ===========================================================================

/// Given a visible row (from the shared seed fixture) and a hidden row
///   (created via the admin API)
/// When an authenticated admin lists GET /api/admin/experience
/// Then BOTH rows are present, and both carry a `visible` field — visible
///   for the seed row, false for the hidden row (R-0004: "the admin list
///   return visible for every row, including hidden rows")
///
/// Distinct from the `/{id}` GET covered by the S3/S7 tests above — this is
/// the plural list endpoint (`list_all`, admin dashboard + AI prompt
/// assembly's shared read per D-4), which must stay unfiltered too.
///
/// Red-phase failure (behavioral): pre-006, `ExperienceFull` does not
/// serialize `visible` at all, so every row's `visible` field reads as
/// JSON-absent (`None`), not `Some(true)`/`Some(false)`.
#[tokio::test]
async fn admin_list_endpoint_returns_visible_for_every_row_including_hidden() {
    let (server, _state) = common::test_app_with_state();
    let token = common::admin_login(&server).await;

    let mut hidden_body = full_valid_experience_body();
    hidden_body["company_name"] = serde_json::json!("List Hidden Co");
    hidden_body["visible"] = serde_json::json!(false);
    let create_response = server
        .post("/api/admin/experience")
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&hidden_body)
        .await;
    create_response.assert_status(StatusCode::CREATED);

    let list_response = server
        .get("/api/admin/experience")
        .add_header(AUTHORIZATION, bearer(&token))
        .await;
    list_response.assert_status_ok();
    let list: Vec<serde_json::Value> = list_response.json();

    let seed_row = list
        .iter()
        .find(|r| r["company_name"] == "Meridian Systems")
        .expect("the shared seed row must appear in the admin list");
    assert_eq!(
        seed_row["visible"].as_bool(),
        Some(true),
        "R-0004: the admin list must return visible: true for a visible \
         row, got: {:?}",
        seed_row.get("visible")
    );

    let hidden_row = list
        .iter()
        .find(|r| r["company_name"] == "List Hidden Co")
        .expect("the hidden row must still appear in the (unfiltered) admin list");
    assert_eq!(
        hidden_row["visible"].as_bool(),
        Some(false),
        "R-0004: the admin list must return visible: false for a hidden \
         row (the admin list stays unfiltered), got: {:?}",
        hidden_row.get("visible")
    );
}

// ===========================================================================
// S7 / R-0006 — hiding an existing public row (HTTP/persistence half)
//
// The checkbox binding half (both FormSection render sites) is frontend and
// out of this dispatch's scope (Out of scope: "New e2e (browser) coverage
// for the admin toggle"). This test asserts the operation's effect at the
// HTTP layer: exactly what the spec names as covered by R-0004/R-0005 for
// this scenario. The "remains in the AI context" clause of S7 is covered by
// the R-0003/S2 test in test_ai_chat_endpoint.rs, not duplicated here.
// ===========================================================================

/// Given an existing row currently on the public page (`visible = true`)
/// When the maintainer (via the admin API — the inline-edit operation's HTTP
///   effect) unchecks the visibility toggle and saves
/// Then the row disappears from GET /api/experience, shows `visible: false`
///   on admin GET; re-checking and saving brings it back
///
/// Red-phase failure (behavioral, not schema): pre-006, `visible` is an
/// ignored extra JSON key on both create and update, so the row never
/// actually leaves the public list — the "must disappear" assertion fails.
#[tokio::test]
async fn admin_toggling_visible_off_then_on_removes_then_restores_public_visibility() {
    let (server, _state) = common::test_app_with_state();
    let token = common::admin_login(&server).await;

    let mut create_body = full_valid_experience_body();
    create_body["company_name"] = serde_json::json!("Toggle Co");
    create_body["visible"] = serde_json::json!(true);
    let create_response = server
        .post("/api/admin/experience")
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&create_body)
        .await;
    create_response.assert_status(StatusCode::CREATED);
    let created: serde_json::Value = create_response.json();
    let id = created["id"].as_i64().expect("created row must have an id");

    let public_before = server.get("/api/experience").await;
    public_before.assert_status_ok();
    let before_list: Vec<serde_json::Value> = public_before.json();
    assert!(
        before_list.iter().any(|r| r["company_name"] == "Toggle Co"),
        "sanity: a newly-created visible row must appear on the public page \
         before the toggle"
    );

    // When: hide it.
    let mut hide_body = full_valid_experience_body();
    hide_body["company_name"] = serde_json::json!("Toggle Co");
    hide_body["visible"] = serde_json::json!(false);
    let hide_response = server
        .put(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&hide_body)
        .await;
    hide_response.assert_status_ok();

    // Then: gone from the public list...
    let public_after_hide = server.get("/api/experience").await;
    public_after_hide.assert_status_ok();
    let after_hide_list: Vec<serde_json::Value> = public_after_hide.json();
    assert!(
        !after_hide_list
            .iter()
            .any(|r| r["company_name"] == "Toggle Co"),
        "S7: the row must disappear from GET /api/experience after hiding, \
         got: {:?}",
        after_hide_list
    );

    // ...still present in the admin list, visible: false.
    let admin_get = server
        .get(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .await;
    admin_get.assert_status_ok();
    let admin_body: serde_json::Value = admin_get.json();
    assert_eq!(
        admin_body["visible"].as_bool(),
        Some(false),
        "S7: admin GET must show visible: false after hiding, got: {:?}",
        admin_body.get("visible")
    );

    // And: re-checking + saving brings it back.
    let mut restore_body = full_valid_experience_body();
    restore_body["company_name"] = serde_json::json!("Toggle Co");
    restore_body["visible"] = serde_json::json!(true);
    let restore_response = server
        .put(&format!("/api/admin/experience/{id}"))
        .add_header(AUTHORIZATION, bearer(&token))
        .json(&restore_body)
        .await;
    restore_response.assert_status_ok();

    let public_after_restore = server.get("/api/experience").await;
    public_after_restore.assert_status_ok();
    let after_restore_list: Vec<serde_json::Value> = public_after_restore.json();
    assert!(
        after_restore_list
            .iter()
            .any(|r| r["company_name"] == "Toggle Co"),
        "S7: the row must reappear on the public page after re-checking \
         visible and saving, got: {:?}",
        after_restore_list
    );
}
