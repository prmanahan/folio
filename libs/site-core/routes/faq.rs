use crate::error::AppError;
use crate::models::faq::{self, FaqSuggestion};
use crate::state::DbState;
use axum::{Json, Router, extract::State, routing::get};

async fn list_suggestions(
    State(state): State<DbState>,
) -> Result<Json<Vec<FaqSuggestion>>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let suggestions = faq::list_suggestions(&conn)?;
    Ok(Json(suggestions))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/faq/suggestions", get(list_suggestions))
}
