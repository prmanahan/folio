mod common;

#[tokio::test]
async fn test_list_skills_returns_public_fields() {
    let server = common::test_app();
    let response = server.get("/api/skills").await;
    response.assert_status_ok();

    let body: Vec<serde_json::Value> = response.json();
    assert_eq!(body.len(), 2);

    // Verify AI-tier and Private-tier fields are NOT present
    assert!(body[0].get("self_rating").is_none()); // AI
    assert!(body[0].get("honest_notes").is_none()); // Private
}
