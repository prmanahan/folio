use crate::error::AppError;
use crate::models::gaps::{self, GapWeakness, GapWeaknessInput};
use crate::state::DbState;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

async fn list_gaps(State(state): State<DbState>) -> Result<Json<Vec<GapWeakness>>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = gaps::list_all(&conn)?;
    Ok(Json(items))
}

async fn get_gap(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<GapWeakness>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = gaps::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Gap {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_gap(
    State(state): State<DbState>,
    Json(input): Json<GapWeaknessInput>,
) -> Result<(StatusCode, Json<GapWeakness>), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = gaps::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_gap(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<GapWeaknessInput>,
) -> Result<Json<GapWeakness>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = gaps::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Gap {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_gap(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    gaps::delete(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/admin/gaps", get(list_gaps).post(create_gap))
        .route(
            "/api/admin/gaps/{id}",
            get(get_gap).put(update_gap).delete(delete_gap),
        )
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::gaps;

    fn make_input(desc: &str, interest: bool) -> gaps::GapWeaknessInput {
        gaps::GapWeaknessInput {
            gap_type: "skill".to_string(),
            description: desc.to_string(),
            why_its_a_gap: "Haven't worked on it professionally".to_string(),
            interest_in_learning: interest,
        }
    }

    #[test]
    fn test_crud_cycle() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let a = gaps::create(&conn, &make_input("ML/AI", true)).unwrap();
        let b = gaps::create(&conn, &make_input("Mobile dev", false)).unwrap();

        assert_eq!(a.description, "ML/AI");
        assert_eq!(a.interest_in_learning, true);
        assert_eq!(b.interest_in_learning, false);

        let all = gaps::list_all(&conn).unwrap();
        assert!(all.iter().any(|g| g.id == a.id));
        assert!(all.iter().any(|g| g.id == b.id));

        let fetched = gaps::get_by_id(&conn, a.id).unwrap();
        assert_eq!(fetched.gap_type, "skill");

        let updated =
            gaps::update(&conn, a.id, &make_input("ML/AI (getting better)", false)).unwrap();
        assert_eq!(updated.description, "ML/AI (getting better)");
        assert_eq!(updated.interest_in_learning, false);

        gaps::delete(&conn, b.id).unwrap();
        let result = gaps::get_by_id(&conn, b.id);
        assert!(result.is_err());

        let remaining = gaps::list_all(&conn).unwrap();
        assert!(!remaining.iter().any(|g| g.id == b.id));
    }
}
