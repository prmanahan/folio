use axum::{extract::State, routing::get, Json, Router};
use crate::error::AppError;
use crate::models::experience::{self, ExperiencePublic};
use crate::state::DbState;

async fn list_experience(State(state): State<DbState>) -> Result<Json<Vec<ExperiencePublic>>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let experiences = experience::list_public(&conn)?;
    Ok(Json(experiences))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/experience", get(list_experience))
}
