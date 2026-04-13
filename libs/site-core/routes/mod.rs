pub mod admin;
pub mod agents;
pub mod ai;
pub mod articles;
pub mod education;
pub mod experience;
pub mod faq;
pub mod links;
pub mod profile;
pub mod projects;
pub mod skills;

use crate::state::DbState;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;

/// Health check handler — executes SELECT 1 to verify the DB connection is live.
/// Returns 200 "ok" on success, 503 "db unavailable" on failure.
pub async fn health_check(State(state): State<DbState>) -> (StatusCode, &'static str) {
    let db = state.db.lock().unwrap();
    match db.query_row("SELECT 1", [], |_| Ok(())) {
        Ok(_) => (StatusCode::OK, "ok"),
        Err(e) => {
            tracing::error!(error = %e, "health check: db query failed");
            (StatusCode::SERVICE_UNAVAILABLE, "db unavailable")
        }
    }
}

pub fn public_router() -> Router<DbState> {
    Router::new()
        .merge(profile::routes())
        .merge(experience::routes())
        .merge(skills::routes())
        .merge(education::routes())
        .merge(projects::routes())
        .merge(articles::routes())
        .merge(links::routes())
        .merge(faq::routes())
        .merge(agents::routes())
}
