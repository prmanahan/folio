use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use crate::error::AppError;
use crate::models::education::{self, Education, EducationInput};
use crate::state::DbState;

async fn list_education(
    State(state): State<DbState>,
) -> Result<Json<Vec<Education>>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = education::list(&conn)?;
    Ok(Json(items))
}

async fn get_education(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<Education>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = education::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Education {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_education(
    State(state): State<DbState>,
    Json(input): Json<EducationInput>,
) -> Result<(StatusCode, Json<Education>), AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = education::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_education(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<EducationInput>,
) -> Result<Json<Education>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = education::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Education {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_education(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    education::delete(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/admin/education", get(list_education).post(create_education))
        .route(
            "/api/admin/education/{id}",
            get(get_education).put(update_education).delete(delete_education),
        )
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::education;

    fn make_input(degree: &str, year: &str) -> education::EducationInput {
        education::EducationInput {
            degree: degree.to_string(),
            institution: "State University".to_string(),
            location: "Springfield".to_string(),
            start_year: year.to_string(),
            end_year: (year.parse::<i32>().unwrap() + 4).to_string(),
        }
    }

    #[test]
    fn test_crud_cycle() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let a = education::create(&conn, &make_input("BS Computer Science", "2000")).unwrap();
        let b = education::create(&conn, &make_input("MS Software Engineering", "2005")).unwrap();

        assert_eq!(a.degree, "BS Computer Science");
        assert_eq!(b.degree, "MS Software Engineering");

        let all = education::list(&conn).unwrap();
        assert!(all.iter().any(|e| e.id == a.id));
        assert!(all.iter().any(|e| e.id == b.id));

        let fetched = education::get_by_id(&conn, a.id).unwrap();
        assert_eq!(fetched.institution, "State University");

        let updated = education::update(&conn, a.id, &make_input("BS CS (Honors)", "2000")).unwrap();
        assert_eq!(updated.degree, "BS CS (Honors)");

        education::delete(&conn, b.id).unwrap();
        let result = education::get_by_id(&conn, b.id);
        assert!(result.is_err());

        let remaining = education::list(&conn).unwrap();
        assert!(!remaining.iter().any(|e| e.id == b.id));
    }
}
