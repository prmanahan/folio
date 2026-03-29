use axum::{extract::State, routing::get, Json, Router};
use crate::error::AppError;
use crate::models::education::{self, Education};
use crate::state::DbState;

async fn list_education(State(state): State<DbState>) -> Result<Json<Vec<Education>>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let edu = education::list(&conn)?;
    Ok(Json(edu))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/education", get(list_education))
}
