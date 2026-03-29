mod common;

#[tokio::test]
async fn test_list_projects() {
    let server = common::test_app();
    let response = server.get("/api/projects").await;
    response.assert_status_ok();

    let body: Vec<serde_json::Value> = response.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["title"], "EventFlow");
}

#[tokio::test]
async fn test_get_project_by_slug() {
    let server = common::test_app();
    let response = server.get("/api/projects/eventflow").await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert_eq!(body["slug"], "eventflow");
}

#[tokio::test]
async fn test_get_project_not_found() {
    let server = common::test_app();
    let response = server.get("/api/projects/nonexistent").await;
    response.assert_status_not_found();
}
