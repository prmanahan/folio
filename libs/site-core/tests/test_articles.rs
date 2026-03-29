mod common;

#[tokio::test]
async fn test_list_articles() {
    let server = common::test_app();
    let response = server.get("/api/articles").await;
    response.assert_status_ok();

    let body: Vec<serde_json::Value> = response.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["title"], "Event Sourcing");
}

#[tokio::test]
async fn test_get_article_by_slug() {
    let server = common::test_app();
    let response = server.get("/api/articles/event-sourcing").await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["slug"], "event-sourcing");
    assert_eq!(body["content"], "# Event Sourcing");
}

#[tokio::test]
async fn test_get_article_not_found() {
    let server = common::test_app();
    let response = server.get("/api/articles/nonexistent").await;
    response.assert_status_not_found();
}
