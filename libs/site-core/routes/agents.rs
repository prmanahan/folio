use axum::{extract::State, routing::get, Json, Router};
use crate::error::AppError;
use crate::models::agent::{self, AgentPublic};
use crate::state::DbState;

async fn list_agents(State(state): State<DbState>) -> Result<Json<Vec<AgentPublic>>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let agents = agent::list_published(&conn)?;
    Ok(Json(agents))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/agents", get(list_agents))
}

#[cfg(test)]
mod tests {
    use axum_test::TestServer;
    use crate::db;
    use crate::models::agent::{self, AgentInput};
    use crate::state::{AppState, DbState};
    use std::sync::{Arc, Mutex};

    fn make_db_state() -> DbState {
        let conn = db::connect(":memory:").expect("in-memory db");
        Arc::new(AppState {
            db: Arc::new(Mutex::new(conn)),
            admin_password_hash: String::new(),
            rig_client: None,
            trusted_ip_header: None,
            page_hit_salt: "test-salt".to_string(),
        })
    }

    fn make_input(name: &str) -> AgentInput {
        AgentInput {
            name: name.to_string(),
            role: "Tester".to_string(),
            short_role: "Test".to_string(),
            model: "sonnet".to_string(),
            personality_blurb: "A test agent.".to_string(),
            responsibilities: vec![],
            avatar_filename: "test.png".to_string(),
            display_order: 1,
            is_featured: false,
            is_review_gate: false,
            published: true,
        }
    }

    #[tokio::test]
    async fn test_list_agents_empty() {
        let state = make_db_state();
        let app = super::routes().with_state(state);
        let server = TestServer::new(app);

        let res = server.get("/api/agents").await;
        res.assert_status_ok();
        let body: serde_json::Value = res.json();
        assert_eq!(body, serde_json::json!([]));
    }

    #[tokio::test]
    async fn test_list_agents_returns_published_only() {
        let state = make_db_state();
        {
            let conn = state.db.lock().unwrap();
            agent::create(&conn, &make_input("Puck")).unwrap();
            let mut unpublished = make_input("Ghost");
            unpublished.published = false;
            agent::create(&conn, &unpublished).unwrap();
        }

        let app = super::routes().with_state(state);
        let server = TestServer::new(app);

        let res = server.get("/api/agents").await;
        res.assert_status_ok();
        let body: Vec<serde_json::Value> = res.json();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0]["name"], "Puck");
    }
}
