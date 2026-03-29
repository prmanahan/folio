use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use crate::error::AppError;
use crate::models::agent::{self, AgentFull, AgentInput};
use crate::state::DbState;

async fn list_agents(
    State(state): State<DbState>,
) -> Result<Json<Vec<AgentFull>>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = agent::list_all(&conn)?;
    Ok(Json(items))
}

async fn get_agent(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<AgentFull>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = agent::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Agent {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_agent(
    State(state): State<DbState>,
    Json(input): Json<AgentInput>,
) -> Result<(StatusCode, Json<AgentFull>), AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = agent::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_agent(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<AgentInput>,
) -> Result<Json<AgentFull>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = agent::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Agent {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_agent(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    agent::delete(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Agent {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/admin/agents", get(list_agents).post(create_agent))
        .route(
            "/api/admin/agents/{id}",
            get(get_agent).put(update_agent).delete(delete_agent),
        )
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use crate::db;
    use crate::models::agent::AgentInput;
    use crate::state::{AppState, DbState};
    use std::sync::{Arc, Mutex};

    fn make_db_state() -> DbState {
        let conn = db::connect(":memory:").expect("in-memory db");
        Arc::new(AppState {
            db: Arc::new(Mutex::new(conn)),
            admin_password_hash: String::new(),
            rig_client: None,
            trusted_ip_header: None,
        })
    }

    fn make_input(name: &str) -> AgentInput {
        AgentInput {
            name: name.to_string(),
            role: "Tester".to_string(),
            short_role: "Test".to_string(),
            model: "sonnet".to_string(),
            personality_blurb: "A test agent.".to_string(),
            responsibilities: vec!["Task A".to_string()],
            avatar_filename: "avatar.png".to_string(),
            display_order: 1,
            is_featured: false,
            is_review_gate: false,
            published: true,
        }
    }

    fn make_app(state: DbState) -> axum::Router {
        super::routes().with_state(state)
    }

    #[tokio::test]
    async fn test_list_agents_empty() {
        let state = make_db_state();
        let server = TestServer::new(make_app(state));
        let res = server.get("/api/admin/agents").await;
        assert_eq!(res.status_code(), StatusCode::OK);
        let body: serde_json::Value = res.json();
        assert_eq!(body, serde_json::json!([]));
    }

    #[tokio::test]
    async fn test_create_and_get_agent() {
        let state = make_db_state();
        let server = TestServer::new(make_app(state));

        let res = server
            .post("/api/admin/agents")
            .json(&make_input("Forge"))
            .await;
        assert_eq!(res.status_code(), StatusCode::CREATED);
        let created: serde_json::Value = res.json();
        let id = created["id"].as_i64().unwrap();
        assert_eq!(created["name"], "Forge");

        let res = server.get(&format!("/api/admin/agents/{}", id)).await;
        assert_eq!(res.status_code(), StatusCode::OK);
        let fetched: serde_json::Value = res.json();
        assert_eq!(fetched["name"], "Forge");
    }

    #[tokio::test]
    async fn test_update_agent() {
        let state = make_db_state();
        let server = TestServer::new(make_app(state));

        let res = server
            .post("/api/admin/agents")
            .json(&make_input("Forge"))
            .await;
        let created: serde_json::Value = res.json();
        let id = created["id"].as_i64().unwrap();

        let mut updated = make_input("Forge Updated");
        updated.is_featured = true;
        let res = server
            .put(&format!("/api/admin/agents/{}", id))
            .json(&updated)
            .await;
        assert_eq!(res.status_code(), StatusCode::OK);
        let body: serde_json::Value = res.json();
        assert_eq!(body["name"], "Forge Updated");
        assert_eq!(body["is_featured"], true);
    }

    #[tokio::test]
    async fn test_delete_agent() {
        let state = make_db_state();
        let server = TestServer::new(make_app(state));

        let res = server
            .post("/api/admin/agents")
            .json(&make_input("Puck"))
            .await;
        let created: serde_json::Value = res.json();
        let id = created["id"].as_i64().unwrap();

        let res = server.delete(&format!("/api/admin/agents/{}", id)).await;
        assert_eq!(res.status_code(), StatusCode::NO_CONTENT);

        let res = server.get(&format!("/api/admin/agents/{}", id)).await;
        assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_nonexistent_agent() {
        let state = make_db_state();
        let server = TestServer::new(make_app(state));
        let res = server.get("/api/admin/agents/999").await;
        assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
    }
}
