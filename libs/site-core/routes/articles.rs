use crate::error::AppError;
use crate::models::article::{self, Article};
use crate::state::DbState;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

async fn list_articles(State(state): State<DbState>) -> Result<Json<Vec<Article>>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let articles = article::list_published(&conn)?;
    Ok(Json(articles))
}

async fn get_article(
    State(state): State<DbState>,
    Path(slug): Path<String>,
) -> Result<Json<Article>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    match article::get_by_slug(&conn, &slug)? {
        Some(a) => Ok(Json(a)),
        None => Err(AppError::NotFound(format!("Article '{}' not found", slug))),
    }
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/articles", get(list_articles))
        .route("/api/articles/{slug}", get(get_article))
}
