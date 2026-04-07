mod common;

use axum::{middleware, routing::{get, post}, Router};
use site_core::auth;
use site_core::state::DbState;

fn auth_test_app() -> axum_test::TestServer {
    let conn = common::test_db();
    site_core::db::seed::seed_test_data(&conn).unwrap();

    let password_hash = site_core::auth::hash_password("testpass")
        .expect("Failed to hash test password");
    let state: DbState = std::sync::Arc::new(site_core::state::AppState {
        db: std::sync::Arc::new(std::sync::Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client: None,
        trusted_ip_header: None,
        page_hit_salt: "test-salt".to_string(),
    });

    let protected = Router::new()
        .route("/api/admin/test", get(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(state.clone(), auth::require_auth));

    let app = Router::new()
        .route("/api/admin/login", post(auth::login))
        .route("/api/admin/logout", post(auth::logout))
        .merge(protected)
        .with_state(state);

    axum_test::TestServer::new(app)
}

#[tokio::test]
async fn test_login_success() {
    let server = auth_test_app();

    let response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": "testpass" }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["token"].is_string(), "response should have a token");
    assert!(body["expires_at"].is_string(), "response should have expires_at");
    let token = body["token"].as_str().unwrap();
    assert!(!token.is_empty(), "token should not be empty");
}

#[tokio::test]
async fn test_login_wrong_password() {
    let server = auth_test_app();

    let response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": "wrongpass" }))
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_with_valid_token() {
    let server = auth_test_app();

    // First, log in to get a token
    let login_response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": "testpass" }))
        .await;
    login_response.assert_status_ok();
    let body: serde_json::Value = login_response.json();
    let token = body["token"].as_str().unwrap().to_string();

    // Use the token to access protected route
    let response = server
        .get("/api/admin/test")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse::<axum::http::HeaderValue>().unwrap(),
        )
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn test_protected_without_token() {
    let server = auth_test_app();

    let response = server.get("/api/admin/test").await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_with_invalid_token() {
    let server = auth_test_app();

    let response = server
        .get("/api/admin/test")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer not-a-real-token".parse::<axum::http::HeaderValue>().unwrap(),
        )
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_logout_invalidates_token() {
    let server = auth_test_app();

    // Log in
    let login_response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": "testpass" }))
        .await;
    login_response.assert_status_ok();
    let body: serde_json::Value = login_response.json();
    let token = body["token"].as_str().unwrap().to_string();

    // Log out
    let logout_response = server
        .post("/api/admin/logout")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse::<axum::http::HeaderValue>().unwrap(),
        )
        .await;
    logout_response.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Try to use the same token after logout - should be 401
    let response = server
        .get("/api/admin/test")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse::<axum::http::HeaderValue>().unwrap(),
        )
        .await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}
