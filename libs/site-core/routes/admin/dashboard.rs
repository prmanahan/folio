use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use crate::error::AppError;
use crate::state::DbState;

#[derive(Serialize)]
pub struct DashboardCounts {
    pub experiences: i64,
    pub skills: i64,
    pub education: i64,
    pub projects: i64,
    pub articles: i64,
    pub links: i64,
    pub faq_responses: i64,
    pub gaps_weaknesses: i64,
    pub ai_instructions: i64,
}

async fn get_dashboard(State(state): State<DbState>) -> Result<Json<DashboardCounts>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;

    // Use individual parameterized queries per table rather than string-formatting
    // the table name into SQL. Table names cannot be bound as query parameters,
    // so each table gets its own static query string.
    let experiences: i64 = conn.query_row("SELECT COUNT(*) FROM experiences", [], |row| row.get(0))?;
    let skills: i64 = conn.query_row("SELECT COUNT(*) FROM skills", [], |row| row.get(0))?;
    let education: i64 = conn.query_row("SELECT COUNT(*) FROM education", [], |row| row.get(0))?;
    let projects: i64 = conn.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?;
    let articles: i64 = conn.query_row("SELECT COUNT(*) FROM articles", [], |row| row.get(0))?;
    let links: i64 = conn.query_row("SELECT COUNT(*) FROM links", [], |row| row.get(0))?;
    let faq_responses: i64 = conn.query_row("SELECT COUNT(*) FROM faq_responses", [], |row| row.get(0))?;
    let gaps_weaknesses: i64 = conn.query_row("SELECT COUNT(*) FROM gaps_weaknesses", [], |row| row.get(0))?;
    let ai_instructions: i64 = conn.query_row("SELECT COUNT(*) FROM ai_instructions", [], |row| row.get(0))?;

    Ok(Json(DashboardCounts {
        experiences,
        skills,
        education,
        projects,
        articles,
        links,
        faq_responses,
        gaps_weaknesses,
        ai_instructions,
    }))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/admin/dashboard", get(get_dashboard))
}
