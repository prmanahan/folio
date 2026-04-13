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

    let password_hash =
        site_core::auth::hash_password("testpass").expect("Failed to hash test password");
    let state: site_core::state::DbState = std::sync::Arc::new(site_core::state::AppState {
        db: std::sync::Arc::new(std::sync::Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client: None,
        trusted_ip_header: None,
        page_hit_salt: "test-salt".to_string(),
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
