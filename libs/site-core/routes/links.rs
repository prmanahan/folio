use crate::error::AppError;
use crate::models::link::{self, Link};
use crate::state::DbState;
use axum::{Json, Router, extract::State, routing::get};

async fn list_links(State(state): State<DbState>) -> Result<Json<Vec<Link>>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let links = link::list(&conn)?;
    Ok(Json(links))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/links", get(list_links))
}
