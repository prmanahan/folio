use rusqlite::Connection;

use crate::error::AppError;

/// Check and enforce rate limiting for a given IP and endpoint.
///
/// Uses a per-hour sliding window stored in SQLite. On each call:
/// 1. Cleans up rows older than 24 hours (maintenance, not enforcement).
/// 2. UPSERTs a row for (ip, endpoint): resets count to 1 if the current hour
///    window differs from the stored window, or increments count if same window.
/// 3. Returns `AppError::RateLimited` if the resulting count exceeds `max_requests`.
pub fn check_rate_limit(
    conn: &Connection,
    ip: &str,
    endpoint: &str,
    max_requests: i64,
) -> Result<(), AppError> {
    // Step 1: Clean stale rows older than 24 hours.
    // window_start is stored as TEXT in SQLite datetime format (e.g., '2024-01-15 14:00:00').
    conn.execute(
        "DELETE FROM rate_limits WHERE window_start < datetime('now', '-1 day')",
        [],
    )?;

    // Step 2: UPSERT for current hour window.
    // The current hour window key is e.g. '2024-01-15 14:00:00'.
    // ON CONFLICT logic:
    //   - If the stored window_start matches the current hour → increment count.
    //   - If the stored window_start is from a prior hour → reset count to 1, update window.
    conn.execute(
        "INSERT INTO rate_limits (ip, endpoint, request_count, window_start)
         VALUES (?1, ?2, 1, strftime('%Y-%m-%d %H:00:00', 'now'))
         ON CONFLICT(ip, endpoint) DO UPDATE SET
             request_count = CASE
                 WHEN window_start = strftime('%Y-%m-%d %H:00:00', 'now')
                 THEN request_count + 1
                 ELSE 1
             END,
             window_start = strftime('%Y-%m-%d %H:00:00', 'now')",
        rusqlite::params![ip, endpoint],
    )?;

    // Step 3: Read back the current count and enforce the limit.
    let count: i64 = conn.query_row(
        "SELECT request_count FROM rate_limits WHERE ip = ?1 AND endpoint = ?2",
        rusqlite::params![ip, endpoint],
        |row| row.get(0),
    )?;

    if count > max_requests {
        return Err(AppError::RateLimited("Rate limit exceeded".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    #[test]
    fn first_request_succeeds() {
        let conn = setup_db();
        let result = check_rate_limit(&conn, "127.0.0.1", "/api/chat", 10);
        assert!(result.is_ok(), "First request should succeed");
    }

    #[test]
    fn requests_up_to_limit_succeed() {
        let conn = setup_db();
        let limit = 5;
        for i in 1..=limit {
            let result = check_rate_limit(&conn, "10.0.0.1", "/api/fit", limit);
            assert!(
                result.is_ok(),
                "Request #{i} of {limit} should succeed, got: {result:?}"
            );
        }
    }

    #[test]
    fn request_over_limit_returns_error() {
        let conn = setup_db();
        let limit = 3;
        // Exhaust the limit
        for _ in 0..limit {
            check_rate_limit(&conn, "10.0.0.2", "/api/chat", limit)
                .expect("Requests within limit should succeed");
        }
        // The next request (limit + 1) should fail
        let result = check_rate_limit(&conn, "10.0.0.2", "/api/chat", limit);
        assert!(result.is_err(), "Request at limit+1 should return an error");
        match result {
            Err(AppError::RateLimited(msg)) => {
                assert_eq!(msg, "Rate limit exceeded");
            }
            other => panic!("Expected RateLimited error, got: {other:?}"),
        }
    }

    #[test]
    fn different_ips_have_independent_limits() {
        let conn = setup_db();
        let limit = 2;
        // Exhaust limit for ip_a
        for _ in 0..limit {
            check_rate_limit(&conn, "192.168.1.1", "/api/chat", limit)
                .expect("ip_a requests within limit should succeed");
        }
        // ip_b should still succeed
        let result = check_rate_limit(&conn, "192.168.1.2", "/api/chat", limit);
        assert!(result.is_ok(), "Different IP should have its own limit");
    }

    #[test]
    fn different_endpoints_have_independent_limits() {
        let conn = setup_db();
        let limit = 2;
        // Exhaust limit for /api/chat
        for _ in 0..limit {
            check_rate_limit(&conn, "10.0.0.3", "/api/chat", limit)
                .expect("chat requests within limit should succeed");
        }
        // /api/fit should still succeed for same IP
        let result = check_rate_limit(&conn, "10.0.0.3", "/api/fit", limit);
        assert!(
            result.is_ok(),
            "Different endpoint should have its own limit"
        );
    }

    #[test]
    fn stale_row_cleanup_works() {
        let conn = setup_db();

        // Insert a row with a window_start older than 24 hours
        conn.execute(
            "INSERT INTO rate_limits (ip, endpoint, request_count, window_start)
             VALUES ('1.2.3.4', '/api/chat', 999, datetime('now', '-2 days'))",
            [],
        )
        .expect("Failed to insert stale row");

        // Verify the stale row exists
        let count_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM rate_limits WHERE ip = '1.2.3.4'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query");
        assert_eq!(count_before, 1, "Stale row should exist before cleanup");

        // Calling check_rate_limit triggers cleanup
        check_rate_limit(&conn, "1.2.3.4", "/api/chat", 10)
            .expect("First real request should succeed");

        // The stale row should be gone; the new row has request_count = 1
        let request_count: i64 = conn
            .query_row(
                "SELECT request_count FROM rate_limits WHERE ip = '1.2.3.4' AND endpoint = '/api/chat'",
                [],
                |row| row.get(0),
            )
            .expect("Row should exist after upsert");
        assert_eq!(
            request_count, 1,
            "After cleanup and fresh insert, count should be 1"
        );
    }

    #[test]
    fn limit_of_one_allows_first_denies_second() {
        let conn = setup_db();
        let result1 = check_rate_limit(&conn, "5.5.5.5", "/api/chat", 1);
        assert!(result1.is_ok(), "First request with limit=1 should pass");

        let result2 = check_rate_limit(&conn, "5.5.5.5", "/api/chat", 1);
        assert!(result2.is_err(), "Second request with limit=1 should fail");
    }
}
