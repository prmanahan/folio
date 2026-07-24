use crate::error::AppError;
use crate::models::experience::{self, ExperienceInput};
use crate::state::DbState;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};

async fn list_experience(
    State(state): State<DbState>,
) -> Result<Json<Vec<experience::ExperienceFull>>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let items = experience::list_all(&conn)?;
    Ok(Json(items))
}

async fn get_experience(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<Json<experience::ExperienceFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = experience::get_by_id(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Experience {} not found", id))
        }
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn create_experience(
    State(state): State<DbState>,
    Json(input): Json<ExperienceInput>,
) -> Result<(StatusCode, Json<experience::ExperienceFull>), AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = experience::create(&conn, &input)?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn update_experience(
    State(state): State<DbState>,
    Path(id): Path<i64>,
    Json(input): Json<ExperienceInput>,
) -> Result<Json<experience::ExperienceFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = experience::update(&conn, id, &input).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Experience {} not found", id))
        }
        other => AppError::Internal(other.to_string()),
    })?;
    Ok(Json(item))
}

async fn delete_experience(
    State(state): State<DbState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    experience::delete(&conn, id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> Router<DbState> {
    Router::new()
        .route(
            "/api/admin/experience",
            get(list_experience).post(create_experience),
        )
        .route(
            "/api/admin/experience/{id}",
            get(get_experience)
                .put(update_experience)
                .delete(delete_experience),
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
            // Spec #2715 / R-0004: `visible` becomes a required field on
            // `ExperienceInput` (no serde default, D-1). Mechanical
            // red-phase compile-fix (spec Notes, "Ownership split") — this
            // literal does not compile until the implementer adds the field
            // to `ExperienceInput` in models/experience.rs; that IS this
            // dispatch's accepted compile-red state (mirrors #572's
            // precedent). Once the field lands, this fixture needs no
            // further edit.
            visible: true,
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
        // Spec #2715 / R-0004: create() persists `visible` — make_input()
        // sets `visible: true`, so both rows must round-trip as visible.
        assert!(a.visible, "created row must carry the input's visible=true");
        assert!(b.visible, "created row must carry the input's visible=true");

        // List all — should have 2 (plus any seed data; use distinct IDs)
        let all = experience::list_all(&conn).unwrap();
        // Both created records must be in the list
        assert!(
            all.iter()
                .any(|e| e.id == a.id && e.company_name == "Acme Corp")
        );
        assert!(
            all.iter()
                .any(|e| e.id == b.id && e.company_name == "Beta Inc")
        );

        // Get by id
        let fetched = experience::get_by_id(&conn, a.id).unwrap();
        assert_eq!(fetched.company_name, "Acme Corp");
        assert_eq!(
            fetched.bullet_points,
            serde_json::json!(["Built X", "Scaled Y"])
        );
        assert_eq!(
            fetched.quantified_impact,
            serde_json::json!({"revenue": "$1M"})
        );
        assert!(
            fetched.visible,
            "get_by_id must return the persisted visible value"
        );

        // Update — round-trip visible: false to confirm update() writes it
        // (R-0004/R-0005: the write path must not drop or default the flag).
        let mut updated_input = make_input("Acme Corp Updated", 1);
        updated_input.summary = "Led a team of 10.".to_string();
        updated_input.visible = false;
        let updated = experience::update(&conn, a.id, &updated_input).unwrap();
        assert_eq!(updated.company_name, "Acme Corp Updated");
        assert_eq!(updated.summary, "Led a team of 10.");
        assert!(
            !updated.visible,
            "update() must persist visible=false, not drop or default it"
        );

        // Re-fetch to confirm the write actually landed in storage, not just
        // in the in-memory return value.
        let refetched = experience::get_by_id(&conn, a.id).unwrap();
        assert!(
            !refetched.visible,
            "visible=false must survive a re-read from storage after update()"
        );

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
        // Spec #2715 / D-3: `ExperiencePublic` must never serialize `visible`
        // — the public query filters on it, so the value would be constant
        // `true` on every row (zero information, one more frozen surface).
        assert!(
            json.get("visible").is_none(),
            "ExperiencePublic must not carry a `visible` key (D-3)"
        );
        // Public fields must be present
        assert!(json.get("company_name").is_some());
        assert!(json.get("bullet_points").is_some());
    }
}
