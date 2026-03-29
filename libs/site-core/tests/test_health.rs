mod common;

/// Health check integration tests.
///
/// The failure path (503 response) is not tested here. The handler returns 503 when
/// the rusqlite `query_row` call fails. With in-memory SQLite there is no mechanism
/// to break a live `Connection` after construction — dropping the connection closes it,
/// but the `Arc<Mutex<Connection>>` in AppState keeps it alive for the full test lifetime.
/// Simulating a broken connection would require either a file-backed DB that we delete
/// mid-test (racy, platform-dependent) or a mock Connection type (adds a test-only
/// abstraction seam that doesn't exist in the codebase). The happy path below confirms
/// the handler correctly reaches the DB and returns the expected status.

#[tokio::test]
async fn health_check_returns_ok_with_live_db() {
    let server = common::test_app();
    let response = server.get("/api/health").await;
    response.assert_status_ok();
    response.assert_text("ok");
}
