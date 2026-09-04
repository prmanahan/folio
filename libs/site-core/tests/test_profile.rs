mod common;

#[tokio::test]
async fn test_get_profile_returns_public_fields() {
    let server = common::test_app();
    let response = server.get("/api/profile").await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "Alex Rivera");
    assert_eq!(body["email"], "alex@example.com");
    assert_eq!(body["availability_status"], "open");

    // Pitch split is exposed publicly
    assert!(
        body.get("pitch_short").is_some(),
        "pitch_short must be in public profile"
    );
    assert!(
        body.get("pitch_long").is_some(),
        "pitch_long must be in public profile"
    );
    assert!(
        !body["pitch_short"].as_str().unwrap_or("").is_empty(),
        "seed pitch_short must be non-empty"
    );

    // Old elevator_pitch is gone from the API surface
    assert!(
        body.get("elevator_pitch").is_none(),
        "elevator_pitch must not appear post-migration"
    );

    // Verify AI-tier and Private-tier fields are NOT present
    assert!(body.get("salary_min").is_none()); // Private
    assert!(body.get("salary_max").is_none()); // Private
    assert!(body.get("career_narrative").is_none()); // AI
    assert!(body.get("looking_for").is_none()); // AI
}

// --- PUT /api/admin/profile validation wire contract (task #3473) -----------
//
// `update_profile` used to build its 400 body inline with `Response` as the
// error type; it now returns `AppError::ProfileValidation`. The body must be
// byte-identical across that change. The admin UI validates client-side and
// its catch block ignores `field` / `limit`, so nothing downstream — e2e
// included — would notice a regression here. These tests are the only thing
// holding the shape.

use axum::http::StatusCode;

async fn admin_token(server: &axum_test::TestServer) -> String {
    let response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": common::test_password::password() }))
        .await;
    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    body["token"].as_str().unwrap().to_string()
}

/// Every `ProfileInput` field, all valid. A test overrides one field so the
/// rejection under test is the only reason the request can fail.
fn valid_profile_body() -> serde_json::Value {
    serde_json::json!({
        "name": "Alex Rivera",
        "email": "alex@example.com",
        "title": "Senior Architect",
        "location": "Remote",
        "phone": "555-1234",
        "linkedin_url": "https://linkedin.com/in/alex-rivera-example",
        "github_url": "https://github.com/alex-rivera-example",
        "twitter_url": "",
        "pitch_short": "Short and tight pitch.",
        "pitch_long": "Longer narrative pitch with more detail.",
        "availability_status": "available",
        "availability_date": "2026-04-01",
        "remote_preference": "remote_only",
        "target_titles": ["Staff Engineer", "Principal Engineer"],
        "target_company_stages": ["Series B", "Series C"],
        "career_narrative": "20 years building distributed systems.",
        "looking_for": "High-impact technical leadership.",
        "not_looking_for": "Pure management roles.",
        "management_style": "Servant leadership.",
        "work_style": "Async-first.",
        "salary_min": 200000,
        "salary_max": 280000,
    })
}

async fn put_profile(body: serde_json::Value) -> axum_test::TestResponse {
    let server = common::test_app();
    let token = admin_token(&server).await;
    server
        .put("/api/admin/profile")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {token}")
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .json(&body)
        .await
}

#[tokio::test]
async fn test_update_profile_over_limit_returns_400_with_field_and_limit() {
    let mut body = valid_profile_body();
    // 281 chars — one past the 280-char pitch_short cap (spec R6).
    body["pitch_short"] = serde_json::json!("x".repeat(281));

    let response = put_profile(body).await;
    response.assert_status(StatusCode::BAD_REQUEST);

    // Whole-object equality: pins the key set (no extras, none missing), the
    // exact validator text, and `limit` as a number rather than a string.
    let actual: serde_json::Value = response.json();
    assert_eq!(
        actual,
        serde_json::json!({
            "error": "field `pitch_short` exceeds maximum length (limit 280 characters)",
            "field": "pitch_short",
            "limit": 280,
        })
    );
}

#[tokio::test]
async fn test_update_profile_empty_required_field_returns_400_with_null_limit() {
    let mut body = valid_profile_body();
    body["pitch_short"] = serde_json::json!("   ");

    let response = put_profile(body).await;
    response.assert_status(StatusCode::BAD_REQUEST);

    // The no-numeric-cap branch. `limit` must be present and null, NOT absent.
    // The body is hand-built with `json!` in `AppError::into_response` and
    // there is no `Serialize` impl on this path, so the regression to guard
    // against is someone making the key conditional on `Some(_)` rather than a
    // serde attribute. Whole-object equality is what tells present-and-null
    // apart from absent.
    let actual: serde_json::Value = response.json();
    assert_eq!(
        actual,
        serde_json::json!({
            "error": "field `pitch_short` must not be empty",
            "field": "pitch_short",
            "limit": null,
        })
    );
    assert!(
        actual.as_object().unwrap().contains_key("limit"),
        "`limit` must be present-and-null, never omitted"
    );
}

#[tokio::test]
async fn test_update_profile_valid_input_returns_200() {
    let response = put_profile(valid_profile_body()).await;
    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "Alex Rivera");
    assert_eq!(body["pitch_short"], "Short and tight pitch.");
}
