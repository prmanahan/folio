use rusqlite::{Connection, params};

/// Record a unique page hit. Duplicate (path, ip_hash) pairs are silently ignored.
pub fn record_hit(conn: &Connection, path: &str, ip_hash: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT OR IGNORE INTO page_hits (path, ip_hash) VALUES (?1, ?2)",
        params![path, ip_hash],
    )?;
    Ok(())
}

/// Get unique hit counts per path, ordered by count descending.
pub fn get_hit_counts(conn: &Connection) -> Result<Vec<(String, i64)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT path, COUNT(DISTINCT ip_hash) as cnt FROM page_hits GROUP BY path ORDER BY cnt DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;"
        ).expect("pragmas");
        db::schema::run_migrations(&conn).expect("migrations");
        conn
    }

    #[test]
    fn record_and_count() {
        let conn = test_db();
        record_hit(&conn, "/", "hash_a").unwrap();
        record_hit(&conn, "/", "hash_b").unwrap();
        record_hit(&conn, "/projects", "hash_a").unwrap();

        let counts = get_hit_counts(&conn).unwrap();
        assert_eq!(counts.len(), 2);
        // "/" has 2 unique visitors
        assert_eq!(counts[0], ("/".to_string(), 2));
        // "/projects" has 1
        assert_eq!(counts[1], ("/projects".to_string(), 1));
    }

    #[test]
    fn duplicate_ignored() {
        let conn = test_db();
        record_hit(&conn, "/", "hash_a").unwrap();
        record_hit(&conn, "/", "hash_a").unwrap();

        let counts = get_hit_counts(&conn).unwrap();
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].1, 1);
    }
}
