use crate::error::AppError;
use crate::models::skill::{self, SkillInput};
use crate::state::DbState;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

async fn list_skills(
    State(state): State<DbState>,
) -> Result<Json<Vec<skill::SkillFull>>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = skill::list_all(&conn)?;
    Ok(Json(items))
}

async fn get_skill(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<skill::SkillFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = skill::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Skill {} not found", id))
        }
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_skill(
    State(state): State<DbState>,
    Json(input): Json<SkillInput>,
) -> Result<(StatusCode, Json<skill::SkillFull>), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = skill::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_skill(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<SkillInput>,
) -> Result<Json<skill::SkillFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = skill::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Skill {} not found", id))
        }
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_skill(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    skill::delete(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/admin/skills", get(list_skills).post(create_skill))
        .route(
            "/api/admin/skills/{id}",
            get(get_skill).put(update_skill).delete(delete_skill),
        )
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::skill;

    fn make_input(name: &str, rating: i64) -> skill::SkillInput {
        skill::SkillInput {
            skill_name: name.to_string(),
            category: "backend".to_string(),
            years_experience: 5,
            last_used: "2025".to_string(),
            self_rating: rating,
            evidence: "Used in production.".to_string(),
            honest_notes: "Could go deeper.".to_string(),
        }
    }

    #[test]
    fn test_crud_cycle() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let a = skill::create(&conn, &make_input("Rust", 4)).unwrap();
        let b = skill::create(&conn, &make_input("Python", 5)).unwrap();

        assert_eq!(a.skill_name, "Rust");
        assert_eq!(b.self_rating, 5);

        let all = skill::list_all(&conn).unwrap();
        assert!(all.iter().any(|s| s.id == a.id && s.skill_name == "Rust"));
        assert!(all.iter().any(|s| s.id == b.id && s.skill_name == "Python"));

        let fetched = skill::get_by_id(&conn, a.id).unwrap();
        assert_eq!(fetched.skill_name, "Rust");
        assert_eq!(fetched.self_rating, 4);
        assert_eq!(fetched.honest_notes, "Could go deeper.");

        let updated = skill::update(&conn, a.id, &make_input("Rust (Advanced)", 5)).unwrap();
        assert_eq!(updated.skill_name, "Rust (Advanced)");
        assert_eq!(updated.self_rating, 5);

        skill::delete(&conn, b.id).unwrap();
        let result = skill::get_by_id(&conn, b.id);
        assert!(result.is_err());

        let remaining = skill::list_all(&conn).unwrap();
        assert!(!remaining.iter().any(|s| s.id == b.id));
    }

    #[test]
    fn test_public_list_lacks_admin_fields() {
        let conn = db::connect(":memory:").expect("in-memory db");
        skill::create(&conn, &make_input("Go", 3)).unwrap();

        let public = skill::list_public(&conn).unwrap();
        assert!(!public.is_empty());
        let json = serde_json::to_value(&public[0]).unwrap();

        assert!(json.get("honest_notes").is_none());
        assert!(json.get("evidence").is_none());
        assert!(json.get("self_rating").is_none());
        assert!(json.get("skill_name").is_some());
    }
}
