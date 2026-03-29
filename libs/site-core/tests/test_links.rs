mod common;

#[tokio::test]
async fn test_list_links() {
    let server = common::test_app();
    let response = server.get("/api/links").await;
    response.assert_status_ok();

    let body: Vec<serde_json::Value> = response.json();
    assert_eq!(body.len(), 3);
    assert_eq!(body[0]["label"], "LinkedIn");
}
