mod common;

#[tokio::test]
async fn test_list_experience_returns_public_fields() {
    let server = common::test_app();
    let response = server.get("/api/experience").await;
    response.assert_status_ok();

    let body: Vec<serde_json::Value> = response.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["company_name"], "Meridian Systems");
    assert_eq!(body[0]["title"], "Software Architect");

    // Verify Private-tier fields are NOT present
    assert!(body[0].get("why_joined").is_none());
    assert!(body[0].get("why_left").is_none());
    assert!(body[0].get("manager_would_say").is_none());
}
