pub mod admin;
pub mod agents;
pub mod ai;
pub mod articles;
pub mod education;
pub mod experience;
pub mod faq;
pub mod links;
pub mod profile;
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
        .merge(articles::routes())
        .merge(links::routes())
        .merge(faq::routes())
        .merge(agents::routes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn state() -> DbState {
        Arc::new(AppState {
            db: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            admin_password_hash: crate::test_password::password_hash(),
            rig_client: None,
            trusted_ip_header: None,
            page_hit_salt: "test-salt".to_string(),
            projects: crate::test_projects::projects(),
        })
    }

    #[test]
    fn health_check_body_ok_when_db_responds() {
        assert_eq!(health_check_body(&state()), (StatusCode::OK, "ok"));
    }

    /// #3486: a panic while holding the db lock poisons it permanently. The
    /// health endpoint must report 503 rather than panic, because Fly's
    /// health check is what decides the machine is alive.
    #[test]
    fn health_check_body_returns_503_when_db_lock_poisoned() {
        let state = state();
        let db = Arc::clone(&state.db);
        let joined = std::thread::spawn(move || {
            let _guard = db.lock().unwrap();
            panic!("poison the db lock");
        })
        .join();
        assert!(joined.is_err(), "setup: thread must have panicked");
        assert!(state.db.is_poisoned(), "setup: lock must be poisoned");

        assert_eq!(
            health_check_body(&state),
            (StatusCode::SERVICE_UNAVAILABLE, "db unavailable")
        );
    }
}
