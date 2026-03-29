use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use crate::error::AppError;
use crate::models::project::{self, ProjectFull, ProjectInput};
use crate::state::DbState;

async fn list_projects(
    State(state): State<DbState>,
) -> Result<Json<Vec<ProjectFull>>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = project::list_all(&conn)?;
    Ok(Json(items))
}

async fn get_project(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<ProjectFull>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = project::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Project {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_project(
    State(state): State<DbState>,
    Json(input): Json<ProjectInput>,
) -> Result<(StatusCode, Json<ProjectFull>), AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = project::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_project(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<ProjectInput>,
) -> Result<Json<ProjectFull>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = project::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Project {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_project(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    project::delete(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/admin/projects", get(list_projects).post(create_project))
        .route(
            "/api/admin/projects/{id}",
            get(get_project).put(update_project).delete(delete_project),
        )
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::project;

    fn make_input(title: &str, order: i64, published: bool) -> project::ProjectInput {
        project::ProjectInput {
            title: title.to_string(),
            slug: None,
            summary: "A cool project.".to_string(),
            description: "Detailed description.".to_string(),
            tech_stack: serde_json::json!(["Rust", "Axum"]),
            url: "https://github.com/example/project".to_string(),
            sort_order: order,
            published,
        }
    }

    #[test]
    fn test_crud_cycle() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let a = project::create(&conn, &make_input("My Site", 1, false)).unwrap();
        let b = project::create(&conn, &make_input("CLI Tool", 2, true)).unwrap();

        assert_eq!(a.title, "My Site");
        assert_eq!(a.slug, "my-site"); // auto-generated
        assert_eq!(b.published, true);

        let all = project::list_all(&conn).unwrap();
        assert!(all.iter().any(|p| p.id == a.id));
        assert!(all.iter().any(|p| p.id == b.id));

        let fetched = project::get_by_id(&conn, a.id).unwrap();
        assert_eq!(fetched.tech_stack, serde_json::json!(["Rust", "Axum"]));

        let updated = project::update(&conn, a.id, &make_input("My Site v2", 1, true)).unwrap();
        assert_eq!(updated.title, "My Site v2");
        assert_eq!(updated.slug, "my-site-v2");
        assert_eq!(updated.published, true);

        project::delete(&conn, b.id).unwrap();
        let result = project::get_by_id(&conn, b.id);
        assert!(result.is_err());

        let remaining = project::list_all(&conn).unwrap();
        assert!(!remaining.iter().any(|p| p.id == b.id));
    }

    #[test]
    fn test_explicit_slug() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let input = project::ProjectInput {
            title: "Some Title".to_string(),
            slug: Some("custom-slug".to_string()),
            summary: "".to_string(),
            description: "".to_string(),
            tech_stack: serde_json::json!([]),
            url: "".to_string(),
            sort_order: 1,
            published: false,
        };
        let p = project::create(&conn, &input).unwrap();
        assert_eq!(p.slug, "custom-slug");
    }

    #[test]
    fn test_published_filter() {
        let conn = db::connect(":memory:").expect("in-memory db");

        project::create(&conn, &make_input("Draft", 1, false)).unwrap();
        let pub_p = project::create(&conn, &make_input("Published", 2, true)).unwrap();

        let published = project::list_published(&conn).unwrap();
        assert!(published.iter().any(|p| p.id == pub_p.id));
        // list_all includes drafts
        let all = project::list_all(&conn).unwrap();
        assert!(all.iter().any(|p| !p.published));
    }
}
