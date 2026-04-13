use crate::error::AppError;
use crate::models::faq::{self, FaqFull, FaqInput};
use crate::state::DbState;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

async fn list_faq(State(state): State<DbState>) -> Result<Json<Vec<FaqFull>>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = faq::list_all(&conn)?;
    Ok(Json(items))
}

async fn get_faq(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<FaqFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = faq::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("FAQ {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_faq(
    State(state): State<DbState>,
    Json(input): Json<FaqInput>,
) -> Result<(StatusCode, Json<FaqFull>), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = faq::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_faq(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<FaqInput>,
) -> Result<Json<FaqFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = faq::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("FAQ {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_faq(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    faq::delete(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/admin/faq", get(list_faq).post(create_faq))
        .route(
            "/api/admin/faq/{id}",
            get(get_faq).put(update_faq).delete(delete_faq),
        )
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::faq;

    fn make_input(question: &str, common: bool) -> faq::FaqInput {
        faq::FaqInput {
            question: question.to_string(),
            answer: format!("Answer to: {}", question),
            is_common_question: common,
        }
    }

    #[test]
    fn test_crud_cycle() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let a = faq::create(&conn, &make_input("What's your stack?", true)).unwrap();
        let b = faq::create(&conn, &make_input("Remote only?", false)).unwrap();

        assert_eq!(a.question, "What's your stack?");
        assert_eq!(a.is_common_question, true);
        assert_eq!(b.is_common_question, false);

        let all = faq::list_all(&conn).unwrap();
        assert!(all.iter().any(|f| f.id == a.id));
        assert!(all.iter().any(|f| f.id == b.id));

        let fetched = faq::get_by_id(&conn, a.id).unwrap();
        assert_eq!(fetched.answer, "Answer to: What's your stack?");

        let updated =
            faq::update(&conn, a.id, &make_input("What's your primary stack?", true)).unwrap();
        assert_eq!(updated.question, "What's your primary stack?");

        faq::delete(&conn, b.id).unwrap();
        let result = faq::get_by_id(&conn, b.id);
        assert!(result.is_err());

        let remaining = faq::list_all(&conn).unwrap();
        assert!(!remaining.iter().any(|f| f.id == b.id));
    }

    #[test]
    fn test_suggestions_still_work() {
        let conn = db::connect(":memory:").expect("in-memory db");

        faq::create(&conn, &make_input("Common question?", true)).unwrap();
        faq::create(&conn, &make_input("Uncommon question?", false)).unwrap();

        let suggestions = faq::list_suggestions(&conn).unwrap();
        assert!(suggestions.iter().any(|s| s.question == "Common question?"));
        assert!(
            !suggestions
                .iter()
                .any(|s| s.question == "Uncommon question?")
        );
    }
}
