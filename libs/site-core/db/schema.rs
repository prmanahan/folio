use rusqlite::Connection;

/// All migrations in order. Each tuple is (version, name, sql).
/// SQL is loaded at compile time from the migrations/ directory.
const MIGRATIONS: &[(i32, &str, &str)] = &[
    (1, "initial_schema", include_str!("../../../migrations/001_initial_schema.sql")),
    (2, "agents", include_str!("../../../migrations/002_agents.sql")),
];

/// Ensure the migrations tracking table exists.
fn ensure_migrations_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    )
}

/// Run all pending migrations in order.
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    ensure_migrations_table(conn)?;

    for (version, name, sql) in MIGRATIONS {
        let applied: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM _migrations WHERE version = ?1",
            [version],
            |row| row.get(0),
        )?;

        if !applied {
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
                rusqlite::params![version, name],
            )?;
        }
    }

    Ok(())
}
