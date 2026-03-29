use axum::{extract::{Path, State}, routing::get, Json, Router};
use crate::error::AppError;
use crate::models::project::{self, Project};
use crate::state::DbState;

async fn list_projects(State(state): State<DbState>) -> Result<Json<Vec<Project>>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let projects = project::list_published(&conn)?;
    Ok(Json(projects))
}

async fn get_project(
    State(state): State<DbState>,
    Path(slug): Path<String>,
) -> Result<Json<Project>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    match project::get_by_slug(&conn, &slug)? {
        Some(p) => Ok(Json(p)),
        None => Err(AppError::NotFound(format!("Project '{}' not found", slug))),
    }
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/projects", get(list_projects))
        .route("/api/projects/{slug}", get(get_project))
}
