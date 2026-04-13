use crate::error::AppError;
use crate::models::profile::{self, ProfileInput};
use crate::state::DbState;
use axum::{Json, Router, extract::State, routing::get};

async fn get_profile(State(state): State<DbState>) -> Result<Json<profile::ProfileFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let p = profile::get_full(&conn)?;
    Ok(Json(p))
}

async fn update_profile(
    State(state): State<DbState>,
    Json(input): Json<ProfileInput>,
) -> Result<Json<profile::ProfileFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let p = profile::update(&conn, &input)?;
    Ok(Json(p))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/admin/profile", get(get_profile).put(update_profile))
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::profile;

    #[test]
    fn test_get_full_profile_returns_admin_fields() {
        let conn = db::connect(":memory:").expect("in-memory db");
        let full = profile::get_full(&conn).unwrap();
        let json = serde_json::to_value(&full).unwrap();

        // Full profile exposes all admin-only fields
        assert!(json.get("career_narrative").is_some());
        assert!(json.get("looking_for").is_some());
        assert!(json.get("salary_min").is_some());
        assert!(json.get("target_titles").is_some());
        assert!(json.get("target_company_stages").is_some());
        assert!(json.get("not_looking_for").is_some());
        assert!(json.get("management_style").is_some());
        assert!(json.get("work_style").is_some());
    }

    #[test]
    fn test_update_profile() {
        let conn = db::connect(":memory:").expect("in-memory db");

        let input = profile::ProfileInput {
            name: "Alex Rivera".to_string(),
            email: "alex@example.com".to_string(),
            title: "Senior Architect".to_string(),
            location: "Remote".to_string(),
            phone: "555-1234".to_string(),
            linkedin_url: "https://linkedin.com/in/alex-rivera-example".to_string(),
            github_url: "https://github.com/alex-rivera-example".to_string(),
            twitter_url: "".to_string(),
            elevator_pitch: "I build reliable systems.".to_string(),
            availability_status: "available".to_string(),
            availability_date: "2026-04-01".to_string(),
            remote_preference: "remote_only".to_string(),
            target_titles: serde_json::json!(["Staff Engineer", "Principal Engineer"]),
            target_company_stages: serde_json::json!(["Series B", "Series C"]),
            career_narrative: "20 years building distributed systems.".to_string(),
            looking_for: "High-impact technical leadership.".to_string(),
            not_looking_for: "Pure management roles.".to_string(),
            management_style: "Servant leadership.".to_string(),
            work_style: "Async-first.".to_string(),
            salary_min: Some(200_000),
            salary_max: Some(280_000),
        };

        let updated = profile::update(&conn, &input).unwrap();
        assert_eq!(updated.name, "Alex Rivera");
        assert_eq!(updated.title, "Senior Architect");
        assert_eq!(updated.salary_min, Some(200_000));
        assert_eq!(updated.salary_max, Some(280_000));
        assert_eq!(
            updated.career_narrative,
            "20 years building distributed systems."
        );
        // JSON arrays round-trip correctly
        assert_eq!(
            updated.target_titles,
            serde_json::json!(["Staff Engineer", "Principal Engineer"])
        );
    }

    #[test]
    fn test_public_profile_lacks_admin_fields() {
        let conn = db::connect(":memory:").expect("in-memory db");
        let public = profile::get_public(&conn).unwrap();
        let json = serde_json::to_value(&public).unwrap();

        // Admin-only fields must not appear in public struct
        assert!(json.get("career_narrative").is_none());
        assert!(json.get("salary_min").is_none());
        assert!(json.get("looking_for").is_none());
        assert!(json.get("target_titles").is_none());
        // Public fields must be present
        assert!(json.get("name").is_some());
        assert!(json.get("elevator_pitch").is_some());
    }
}
