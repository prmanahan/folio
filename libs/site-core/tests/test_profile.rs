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

    // Verify AI-tier and Private-tier fields are NOT present
    assert!(body.get("salary_min").is_none()); // Private
    assert!(body.get("salary_max").is_none()); // Private
    assert!(body.get("career_narrative").is_none()); // AI
    assert!(body.get("looking_for").is_none()); // AI
}
