use crate::error::AppError;
use crate::models::article::{self, ArticleFull, ArticleInput};
use crate::state::DbState;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

async fn list_articles(State(state): State<DbState>) -> Result<Json<Vec<ArticleFull>>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = article::list_all(&conn)?;
    Ok(Json(items))
}

async fn get_article(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<ArticleFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = article::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Article {} not found", id))
        }
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_article(
    State(state): State<DbState>,
    Json(input): Json<ArticleInput>,
) -> Result<(StatusCode, Json<ArticleFull>), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = article::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_article(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<ArticleInput>,
) -> Result<Json<ArticleFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = article::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Article {} not found", id))
        }
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_article(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    article::delete(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route(
            "/api/admin/articles",
            get(list_articles).post(create_article),
        )
        .route(
            "/api/admin/articles/{id}",
            get(get_article).put(update_article).delete(delete_article),
        )
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::article;

    fn make_input(title: &str, published: bool) -> article::ArticleInput {
        article::ArticleInput {
            title: title.to_string(),
            slug: None,
            summary: "A summary.".to_string(),
            content: "Full content here.".to_string(),
            tags: serde_json::json!(["rust", "backend"]),
            published_at: Some("2025-01-01".to_string()),
            published,
        }
    }

    #[test]
    fn test_crud_cycle() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let a = article::create(&conn, &make_input("Hello World", false)).unwrap();
        let b = article::create(&conn, &make_input("Rust Tips", true)).unwrap();

        assert_eq!(a.title, "Hello World");
        assert_eq!(a.slug, "hello-world");
        assert_eq!(b.published, true);

        let all = article::list_all(&conn).unwrap();
        assert!(all.iter().any(|art| art.id == a.id));
        assert!(all.iter().any(|art| art.id == b.id));

        let fetched = article::get_by_id(&conn, a.id).unwrap();
        assert_eq!(fetched.tags, serde_json::json!(["rust", "backend"]));

        let updated =
            article::update(&conn, a.id, &make_input("Hello World Updated", true)).unwrap();
        assert_eq!(updated.title, "Hello World Updated");
        assert_eq!(updated.slug, "hello-world-updated");
        assert_eq!(updated.published, true);

        article::delete(&conn, b.id).unwrap();
        let result = article::get_by_id(&conn, b.id);
        assert!(result.is_err());

        let remaining = article::list_all(&conn).unwrap();
        assert!(!remaining.iter().any(|art| art.id == b.id));
    }

    #[test]
    fn test_explicit_slug() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let input = article::ArticleInput {
            title: "Some Title".to_string(),
            slug: Some("my-custom-slug".to_string()),
            summary: "".to_string(),
            content: "".to_string(),
            tags: serde_json::json!([]),
            published_at: None,
            published: false,
        };
        let art = article::create(&conn, &input).unwrap();
        assert_eq!(art.slug, "my-custom-slug");
    }

    #[test]
    fn test_published_filter() {
        let conn = db::connect(":memory:").expect("in-memory db");

        article::create(&conn, &make_input("Draft Post", false)).unwrap();
        let pub_a = article::create(&conn, &make_input("Live Post", true)).unwrap();

        let published = article::list_published(&conn).unwrap();
        assert!(published.iter().any(|a| a.id == pub_a.id));

        let all = article::list_all(&conn).unwrap();
        assert!(all.iter().any(|a| !a.published));
    }
}
