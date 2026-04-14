use crate::error::AppError;
use crate::models::profile::{self, ProfileInput};
use crate::state::DbState;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::json;

async fn get_profile(State(state): State<DbState>) -> Result<Json<profile::ProfileFull>, AppError> {
    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let p = profile::get_full(&conn)?;
    Ok(Json(p))
}

/// Update the profile.
///
/// Validates length limits before touching the DB. On validation failure,
/// returns a 400 with a structured body containing the offending `field`,
/// the `limit` (when applicable), and a human-readable `error` message.
async fn update_profile(
    State(state): State<DbState>,
    Json(input): Json<ProfileInput>,
) -> Result<Json<profile::ProfileFull>, Response> {
    if let Err(verr) = input.validate() {
        let body = json!({
            "error": verr.to_string(),
            "field": verr.field,
            "limit": verr.limit,
        });
        return Err((StatusCode::BAD_REQUEST, Json(body)).into_response());
    }

    let conn = state
        .db
        .lock()
        .map_err(|_| AppError::Internal("DB lock poisoned".into()).into_response())?;
    let p = profile::update(&conn, &input)
        .map_err(|e| AppError::Internal(format!("DB update failed: {e}")).into_response())?;
    Ok(Json(p))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/admin/profile", get(get_profile).put(update_profile))
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::profile;

    fn ok_input() -> profile::ProfileInput {
        profile::ProfileInput {
            name: "Alex Rivera".to_string(),
            email: "alex@example.com".to_string(),
            title: "Senior Architect".to_string(),
            location: "Remote".to_string(),
            phone: "555-1234".to_string(),
            linkedin_url: "https://linkedin.com/in/alex-rivera-example".to_string(),
            github_url: "https://github.com/alex-rivera-example".to_string(),
            twitter_url: "".to_string(),
            pitch_short: "Short and tight pitch.".to_string(),
            pitch_long: "Longer narrative pitch with more detail.".to_string(),
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
        }
    }

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
        // Pitch split is exposed
        assert!(json.get("pitch_short").is_some());
        assert!(json.get("pitch_long").is_some());
        // Old elevator_pitch must be gone
        assert!(json.get("elevator_pitch").is_none());
    }

    #[test]
    fn test_update_profile() {
        let conn = db::connect(":memory:").expect("in-memory db");
        let input = ok_input();

        let updated = profile::update(&conn, &input).unwrap();
        assert_eq!(updated.name, "Alex Rivera");
        assert_eq!(updated.title, "Senior Architect");
        assert_eq!(updated.salary_min, Some(200_000));
        assert_eq!(updated.salary_max, Some(280_000));
        assert_eq!(updated.pitch_short, "Short and tight pitch.");
        assert_eq!(
            updated.pitch_long,
            "Longer narrative pitch with more detail."
        );
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
        assert!(json.get("pitch_short").is_some());
        assert!(json.get("pitch_long").is_some());
        // No legacy field
        assert!(json.get("elevator_pitch").is_none());
    }
}
