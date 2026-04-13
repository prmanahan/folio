use crate::error::AppError;
use crate::models::profile::{self, ProfilePublic};
use crate::state::DbState;
use axum::{Json, Router, extract::State, routing::get};

async fn get_profile(State(state): State<DbState>) -> Result<Json<ProfilePublic>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let profile = profile::get_public(&conn)?;
    Ok(Json(profile))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/profile", get(get_profile))
}
