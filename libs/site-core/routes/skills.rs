use axum::{extract::State, routing::get, Json, Router};
use crate::error::AppError;
use crate::models::skill::{self, SkillPublic};
use crate::state::DbState;

async fn list_skills(State(state): State<DbState>) -> Result<Json<Vec<SkillPublic>>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let skills = skill::list_public(&conn)?;
    Ok(Json(skills))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/skills", get(list_skills))
}
