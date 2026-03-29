mod common;

#[tokio::test]
async fn test_list_faq_suggestions_only_common() {
    let server = common::test_app();
    let response = server.get("/api/faq/suggestions").await;
    response.assert_status_ok();

    let body: Vec<serde_json::Value> = response.json();
    // Only 1 of 2 FAQ entries has is_common_question = true
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["question"], "What are you looking for?");

    // Should NOT include the answer
    assert!(body[0].get("answer").is_none());
}
