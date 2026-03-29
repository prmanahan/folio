mod common;

#[tokio::test]
async fn test_list_education() {
    let server = common::test_app();
    let response = server.get("/api/education").await;
    response.assert_status_ok();

    let body: Vec<serde_json::Value> = response.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["degree"], "BSc Computer Science");
    assert_eq!(body[0]["institution"], "University of British Columbia");
}
