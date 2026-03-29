mod common;

use axum::http::StatusCode;

async fn login_and_get_token(server: &axum_test::TestServer) -> String {
    let response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": "testpass" }))
        .await;
    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();
    body["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_dashboard_authenticated() {
    let server = common::test_app();
    let token = login_and_get_token(&server).await;

    let response = server
        .get("/api/admin/dashboard")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse::<axum::http::HeaderValue>().unwrap(),
        )
        .await;

    response.assert_status(StatusCode::OK);
    let body: serde_json::Value = response.json();

    // All fields should be present and be non-negative integers
    for field in &[
        "experiences",
        "skills",
        "education",
        "projects",
        "articles",
        "links",
        "faq_responses",
        "gaps_weaknesses",
        "ai_instructions",
    ] {
        assert!(
            body[field].is_i64() || body[field].is_u64(),
            "field '{}' missing or not a number: {:?}",
            field,
            body[field]
        );
    }
}

#[tokio::test]
async fn test_dashboard_unauthenticated() {
    let server = common::test_app();

    let response = server.get("/api/admin/dashboard").await;
    response.assert_status(StatusCode::UNAUTHORIZED);
}
