use crate::error::AppError;
use crate::models::link::{self, Link, LinkInput};
use crate::state::DbState;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

async fn list_links(State(state): State<DbState>) -> Result<Json<Vec<Link>>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = link::list(&conn)?;
    Ok(Json(items))
}

async fn get_link(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<Link>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = link::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Link {} not found", id))
        }
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_link(
    State(state): State<DbState>,
    Json(input): Json<LinkInput>,
) -> Result<(StatusCode, Json<Link>), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = link::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_link(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<LinkInput>,
) -> Result<Json<Link>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = link::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Link {} not found", id))
        }
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_link(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    link::delete(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/admin/links", get(list_links).post(create_link))
        .route(
            "/api/admin/links/{id}",
            get(get_link).put(update_link).delete(delete_link),
        )
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::link;

    fn make_input(label: &str, order: i64) -> link::LinkInput {
        link::LinkInput {
            label: label.to_string(),
            url: format!("https://example.com/{}", label.to_lowercase()),
            icon: "link".to_string(),
            sort_order: order,
        }
    }

    #[test]
    fn test_crud_cycle() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let a = link::create(&conn, &make_input("GitHub", 1)).unwrap();
        let b = link::create(&conn, &make_input("LinkedIn", 2)).unwrap();

        assert_eq!(a.label, "GitHub");
        assert_eq!(b.sort_order, 2);

        let all = link::list(&conn).unwrap();
        assert!(all.iter().any(|l| l.id == a.id));
        assert!(all.iter().any(|l| l.id == b.id));

        let fetched = link::get_by_id(&conn, a.id).unwrap();
        assert_eq!(fetched.label, "GitHub");

        let updated = link::update(&conn, a.id, &make_input("GitHub Profile", 1)).unwrap();
        assert_eq!(updated.label, "GitHub Profile");

        link::delete(&conn, b.id).unwrap();
        let result = link::get_by_id(&conn, b.id);
        assert!(result.is_err());

        let remaining = link::list(&conn).unwrap();
        assert!(!remaining.iter().any(|l| l.id == b.id));
    }
}
