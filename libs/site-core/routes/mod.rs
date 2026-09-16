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

/// Runs `SELECT 1` against the shared DB connection and renders the
/// health-check status. Shared by the normal `/api/health` route and
/// `middleware::origin_lock`'s exempt fast path (task #3558 C3), so both
/// retain exactly one implementation of the DB-liveness check.
///
/// Returns 200 "ok" on success, 503 "db unavailable" on a query failure
/// or a poisoned lock — never panics on either.
pub fn health_check_body(state: &DbState) -> (StatusCode, &'static str) {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => {
            tracing::error!("health check: db lock poisoned");
            return (StatusCode::SERVICE_UNAVAILABLE, "db unavailable");
        }
    };
    match db.query_row("SELECT 1", [], |_| Ok(())) {
        Ok(_) => (StatusCode::OK, "ok"),
        Err(e) => {
            tracing::error!(error = %e, "health check: db query failed");
            (StatusCode::SERVICE_UNAVAILABLE, "db unavailable")
        }
    }
}

/// Health check handler — the normal routed path. See [`health_check_body`].
pub async fn health_check(State(state): State<DbState>) -> (StatusCode, &'static str) {
    health_check_body(&state)
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
