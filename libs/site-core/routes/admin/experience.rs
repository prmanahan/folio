use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use crate::error::AppError;
use crate::models::experience::{self, ExperienceInput};
use crate::state::DbState;

async fn list_experience(
    State(state): State<DbState>,
) -> Result<Json<Vec<experience::ExperienceFull>>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = experience::list_all(&conn)?;
    Ok(Json(items))
}

async fn get_experience(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<experience::ExperienceFull>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = experience::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Experience {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_experience(
    State(state): State<DbState>,
    Json(input): Json<ExperienceInput>,
) -> Result<(StatusCode, Json<experience::ExperienceFull>), AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = experience::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_experience(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<ExperienceInput>,
) -> Result<Json<experience::ExperienceFull>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = experience::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Experience {} not found", id)),
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_experience(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    experience::delete(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/admin/experience", get(list_experience).post(create_experience))
        .route(
            "/api/admin/experience/{id}",
            get(get_experience).put(update_experience).delete(delete_experience),
        )
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::experience;

    fn make_input(company: &str, order: i64) -> experience::ExperienceInput {
        experience::ExperienceInput {
            company_name: company.to_string(),
            title: "Engineer".to_string(),
            location: "Remote".to_string(),
            start_date: "2020-01".to_string(),
            end_date: None,
            is_current: true,
            summary: "Built things.".to_string(),
            bullet_points: serde_json::json!(["Built X", "Scaled Y"]),
            display_order: order,
            title_progression: "IC2 → IC3".to_string(),
            quantified_impact: serde_json::json!({"revenue": "$1M"}),
            why_joined: "Great mission.".to_string(),
            why_left: "".to_string(),
            actual_contributions: "Led architecture.".to_string(),
            proudest_achievement: "Zero-downtime migration.".to_string(),
            would_do_differently: "More docs.".to_string(),
            challenges_faced: "Legacy debt.".to_string(),
            lessons_learned: "Incremental wins.".to_string(),
            manager_would_say: "Reliable and sharp.".to_string(),
            reports_would_say: "Clear and supportive.".to_string(),
        }
    }

    #[test]
    fn test_crud_cycle() {
        let conn = db::connect(":memory:").expect("in-memory db");

        // Create two entries
        let a = experience::create(&conn, &make_input("Acme Corp", 1)).unwrap();
        let b = experience::create(&conn, &make_input("Beta Inc", 2)).unwrap();

        assert_eq!(a.company_name, "Acme Corp");
        assert_eq!(b.company_name, "Beta Inc");

        // List all — should have 2 (plus any seed data; use distinct IDs)
        let all = experience::list_all(&conn).unwrap();
        // Both created records must be in the list
        assert!(all.iter().any(|e| e.id == a.id && e.company_name == "Acme Corp"));
        assert!(all.iter().any(|e| e.id == b.id && e.company_name == "Beta Inc"));

        // Get by id
        let fetched = experience::get_by_id(&conn, a.id).unwrap();
        assert_eq!(fetched.company_name, "Acme Corp");
        assert_eq!(fetched.bullet_points, serde_json::json!(["Built X", "Scaled Y"]));
        assert_eq!(fetched.quantified_impact, serde_json::json!({"revenue": "$1M"}));

        // Update
        let mut updated_input = make_input("Acme Corp Updated", 1);
        updated_input.summary = "Led a team of 10.".to_string();
        let updated = experience::update(&conn, a.id, &updated_input).unwrap();
        assert_eq!(updated.company_name, "Acme Corp Updated");
        assert_eq!(updated.summary, "Led a team of 10.");

        // Delete
        experience::delete(&conn, b.id).unwrap();

        // b is gone; get_by_id should return error
        let result = experience::get_by_id(&conn, b.id);
        assert!(result.is_err());

        // list_all should no longer contain b
        let remaining = experience::list_all(&conn).unwrap();
        assert!(!remaining.iter().any(|e| e.id == b.id));
    }

    #[test]
    fn test_public_list_lacks_admin_fields() {
        let conn = db::connect(":memory:").expect("in-memory db");
        experience::create(&conn, &make_input("Test Co", 1)).unwrap();

        let public = experience::list_public(&conn).unwrap();
        assert!(!public.is_empty());
        let json = serde_json::to_value(&public[0]).unwrap();

        // Admin-only fields must not appear in public struct
        assert!(json.get("why_joined").is_none());
        assert!(json.get("why_left").is_none());
        assert!(json.get("career_narrative").is_none());
        assert!(json.get("manager_would_say").is_none());
        // Public fields must be present
        assert!(json.get("company_name").is_some());
        assert!(json.get("bullet_points").is_some());
    }
}
