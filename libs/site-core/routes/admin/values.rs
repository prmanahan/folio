use axum::{extract::State, routing::get, Json, Router};
use crate::error::AppError;
use crate::models::values::{self, ValuesCulture, ValuesCultureInput};
use crate::state::DbState;

async fn get_values(
    State(state): State<DbState>,
) -> Result<Json<ValuesCulture>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = values::get(&conn)?;
    Ok(Json(item))
}

async fn update_values(
    State(state): State<DbState>,
    Json(input): Json<ValuesCultureInput>,
) -> Result<Json<ValuesCulture>, AppError> {
    let conn = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let item = values::update(&conn, &input)?;
    Ok(Json(item))
}

pub fn routes() -> Router<DbState> {
    Router::new().route("/api/admin/values", get(get_values).put(update_values))
}

#[cfg(test)]
mod tests {
    use crate::db;
    use crate::models::values;

    fn make_input() -> values::ValuesCultureInput {
        values::ValuesCultureInput {
            must_haves: "Psychological safety".to_string(),
            dealbreakers: "Micromanagement".to_string(),
            management_style_preferences: "Servant leadership".to_string(),
            team_size_preferences: "Small teams (5-10)".to_string(),
            how_handle_conflict: "Direct but kind conversation".to_string(),
            how_handle_ambiguity: "Break it into smaller knowns".to_string(),
            how_handle_failure: "Post-mortems, not blame".to_string(),
        }
    }

    #[test]
    fn test_get_and_update() {
        let conn = db::connect(":memory:").expect("in-memory db");

        // Default row should exist from seed
        let initial = values::get(&conn).unwrap();
        assert_eq!(initial.id, 1);

        let updated = values::update(&conn, &make_input()).unwrap();
        assert_eq!(updated.must_haves, "Psychological safety");
        assert_eq!(updated.dealbreakers, "Micromanagement");
        assert_eq!(updated.how_handle_failure, "Post-mortems, not blame");
    }
}
