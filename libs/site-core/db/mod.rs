pub mod schema;
pub mod seed;

use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> Result<(), rusqlite::Error> {
    schema::run_migrations(conn)
}

pub fn connect(database_url: &str) -> Result<Connection, rusqlite::Error> {
    tracing::info!(database_url, "connecting to database");
    let conn = Connection::open(database_url)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size=-64000;
         PRAGMA temp_store=memory;",
    )?;
    migrate(&conn)?;
    tracing::info!("migrations complete");
    Ok(conn)
}
