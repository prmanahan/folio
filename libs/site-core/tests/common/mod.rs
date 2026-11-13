#![allow(dead_code)]

pub mod ai_mock;
pub mod test_password;
pub mod test_projects;

use rusqlite::Connection;

pub fn test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size=-64000;
         PRAGMA temp_store=memory;",
    )
    .unwrap();
    site_core::db::migrate(&conn).unwrap();
    conn
}

pub fn seeded_db() -> Connection {
    let conn = test_db();
    site_core::db::seed::seed_test_data(&conn).unwrap();
    conn
}

pub const EDGE_AUTH_HEADER: &str = "x-folio-edge-auth";
/// Throwaway test value only; never a real secret.
pub const TEST_EDGE_SECRET: &str = "folio_edge_test_secret_for_ci_only";
pub const TEST_CORS_ORIGIN: &str = "http://localhost:3000";
pub const STATIC_FIXTURE_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/static_site");

/// The production router (`site_core::app::build_app`, as `cmd/server` serves
/// it) over `static_dir`, on a real socket with connect info.
pub fn build_app_server(static_dir: &std::path::Path) -> axum_test::TestServer {
    use site_core::middleware::global_rate_limit::GlobalRateLimitState;
    use site_core::middleware::origin_lock::OriginLockState;
    use site_core::state::{AppState, DbState};
    use std::sync::{Arc, Mutex};

    let state: DbState = Arc::new(AppState {
        db: Arc::new(Mutex::new(seeded_db())),
        admin_password_hash: test_password::password_hash(),
        rig_client: None,
        trusted_ip_header: None,
        page_hit_salt: "test-salt".to_string(),
        projects: test_projects::projects(),
    });
    let app = site_core::app::build_app(
        state,
        OriginLockState::new(TEST_EDGE_SECRET),
        GlobalRateLimitState::new(),
        static_dir.to_str().unwrap(),
        TEST_CORS_ORIGIN,
    );
    axum_test::TestServer::new(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
}

/// A fresh temp copy of the fixture static site (index, asset, CSP file),
/// plus `projects.json` with `projects_json` as its body when given. `tag`
/// keeps parallel tests in separate directories.
pub fn temp_static_site(tag: &str, projects_json: Option<&str>) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("folio-static-site-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in ["index.html", "asset.txt", "csp-header.txt"] {
        std::fs::copy(format!("{STATIC_FIXTURE_DIR}/{name}"), dir.join(name)).unwrap();
    }
    if let Some(body) = projects_json {
        std::fs::write(dir.join("projects.json"), body).unwrap();
    }
    dir
}

pub fn test_app() -> axum_test::TestServer {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size=-64000;
         PRAGMA temp_store=memory;",
    )
    .unwrap();
    site_core::db::migrate(&conn).unwrap();
    site_core::db::seed::seed_test_data(&conn).unwrap();

    let password_hash = test_password::password_hash();
    let state: site_core::state::DbState = std::sync::Arc::new(site_core::state::AppState {
        db: std::sync::Arc::new(std::sync::Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client: None,
        trusted_ip_header: None,
        page_hit_salt: "test-salt".to_string(),
        projects: test_projects::projects(),
    });

    let app = axum::Router::new()
        .route(
            "/api/health",
            axum::routing::get(site_core::routes::health_check),
        )
        .merge(site_core::routes::public_router())
        .merge(site_core::routes::admin::admin_router(state.clone()))
        .with_state(state);

    axum_test::TestServer::new(app)
}

/// Variant of [`test_app`] that also returns the [`DbState`], so a test can
/// seed rows **in-test** through the exposed connection — e.g. a hidden
/// (`visible = 0`) experience row for spec #2715 (R-0002/R-0004/R-0005/S7) —
/// without mutating the shared `seed_test_data()` fixture that every plain
/// `test_app()`-based HTTP test (and its row-count assertions, e.g.
/// `tests/test_experience.rs`) depends on staying stable.
///
/// Mirrors `common::ai_mock::ai_test_app_with_mock_and_state` (same
/// `(TestServer, DbState)` shape), but wires the public + admin routers
/// (like `test_app`) instead of the AI routes.
pub fn test_app_with_state() -> (axum_test::TestServer, site_core::state::DbState) {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size=-64000;
         PRAGMA temp_store=memory;",
    )
    .unwrap();
    site_core::db::migrate(&conn).unwrap();
    site_core::db::seed::seed_test_data(&conn).unwrap();

    let password_hash = test_password::password_hash();
    let state: site_core::state::DbState = std::sync::Arc::new(site_core::state::AppState {
        db: std::sync::Arc::new(std::sync::Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client: None,
        trusted_ip_header: None,
        page_hit_salt: "test-salt".to_string(),
        projects: test_projects::projects(),
    });

    let app = axum::Router::new()
        .route(
            "/api/health",
            axum::routing::get(site_core::routes::health_check),
        )
        .merge(site_core::routes::public_router())
        .merge(site_core::routes::admin::admin_router(state.clone()))
        .with_state(state.clone());

    (axum_test::TestServer::new(app), state)
}

/// Log in against `server`'s `/api/admin/login` with the shared per-process
/// test password (see `test_password`) and return the bearer token.
///
/// Written once here per spec #2715's Notes ("a shared admin-auth helper —
/// it does not exist yet; existing admin tests hand-roll the login dance
/// locally in `auth_tests.rs`"). `admin_router` mounts `/api/admin/login`
/// unprotected, so any `test_app()` / `test_app_with_state()` server can
/// authenticate through it.
pub async fn admin_login(server: &axum_test::TestServer) -> String {
    let response = server
        .post("/api/admin/login")
        .json(&serde_json::json!({ "password": test_password::password() }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    body["token"]
        .as_str()
        .expect("login response must include a string token")
        .to_string()
}
