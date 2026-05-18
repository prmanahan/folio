//! Typed accessors for the `site_config` key/value table (spec #572, R3-R7).
//!
//! Both public accessors are non-panicking: any SQL or parse failure falls
//! back to the compiled-in default and emits a `tracing` event at the
//! appropriate severity:
//!
//! - SQL connection / table-missing failure → `tracing::error!`
//! - Missing row, empty-after-trim, parse failure, out-of-range clamp →
//!   `tracing::warn!`
//!
//! The error/warn split is owned by the public accessors. The private helper
//! returns `Result<Option<String>, rusqlite::Error>` and does NOT log — that
//! way each scenario produces exactly one tracing event (not two) so log
//! assertions in the integration tests can pin "exactly one WARN" / "exactly
//! one ERROR" per call without false positives.
//!
//! See `docs/specs/2026-05-15-sonnet-migration-config-table.md` (R3, R4, R7).

use rusqlite::{Connection, OptionalExtension, params};

/// Compiled-in default for `ai.model_id`. Returned on missing row, empty
/// value, whitespace-only value, or SQL error. The seeded value matches.
pub const DEFAULT_MODEL_ID: &str = "claude-sonnet-4-6";

/// Compiled-in default for `ai.max_tokens`. Returned on missing row, empty
/// value, parse failure, or SQL error.
pub const DEFAULT_MAX_TOKENS: u32 = 5530;

/// Inclusive lower bound for `ai.max_tokens`. Values below clamp up.
pub const MAX_TOKENS_MIN: u32 = 1;

/// Inclusive upper bound for `ai.max_tokens`. Values above clamp down.
///
/// LLM-audit M3 / R2: lowered from 200_000 to 12_000. For a portfolio
/// chat/fit feature a 200k upper bound was a sky-high per-request
/// cost/latency ceiling — a bad config row or a compromised admin session
/// could request a 200k completion. 12_000 sits mid-band of the spec's
/// 8_000–16_000 target: comfortably above `DEFAULT_MAX_TOKENS` (5530) so
/// the default and any reasonable hand-tuned value pass through unclamped,
/// while the clamp now doubles as the LLM10 cost ceiling.
pub const MAX_TOKENS_MAX: u32 = 12_000;

const KEY_MODEL_ID: &str = "ai.model_id";
const KEY_MAX_TOKENS: &str = "ai.max_tokens";

/// Read a `site_config` value by key, trim whitespace, and treat the empty
/// string as absent. Returns:
///
/// - `Ok(Some(trimmed))` when the row exists and the trimmed value is non-empty
/// - `Ok(None)` when the row is missing OR the trimmed value is empty
/// - `Err(rusqlite::Error)` on any SQL failure (e.g., table missing,
///   connection closed)
///
/// This helper does NOT emit any tracing events. Callers own the
/// warn-vs-error split so the public accessors emit exactly one event per
/// call regardless of which fallback fires.
fn get_config_str_trimmed(conn: &Connection, key: &str) -> Result<Option<String>, rusqlite::Error> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM site_config WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    Ok(raw.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }))
}

/// Return the configured AI model id, falling back to [`DEFAULT_MODEL_ID`]
/// on any of: missing row, empty-after-trim value, SQL error.
///
/// Whitespace is stripped from the returned value. MUST NOT panic.
pub fn get_model_id(conn: &Connection) -> String {
    match get_config_str_trimmed(conn, KEY_MODEL_ID) {
        Ok(Some(value)) => value,
        Ok(None) => {
            tracing::warn!(
                key = KEY_MODEL_ID,
                default = DEFAULT_MODEL_ID,
                "config row missing or empty after trim, using compiled-in default"
            );
            DEFAULT_MODEL_ID.to_string()
        }
        Err(err) => {
            tracing::error!(
                key = KEY_MODEL_ID,
                error = %err,
                default = DEFAULT_MODEL_ID,
                "SQL error reading config, using compiled-in default"
            );
            DEFAULT_MODEL_ID.to_string()
        }
    }
}

/// Return the configured AI max-tokens value, clamped to
/// `[MAX_TOKENS_MIN, MAX_TOKENS_MAX]`. Falls back to [`DEFAULT_MAX_TOKENS`]
/// on any of: missing row, empty-after-trim value, parse failure, SQL error.
///
/// Out-of-range parsed values are clamped (not rejected) and a warn is
/// emitted naming the original value. MUST NOT panic.
pub fn get_max_tokens(conn: &Connection) -> u32 {
    let trimmed = match get_config_str_trimmed(conn, KEY_MAX_TOKENS) {
        Ok(Some(value)) => value,
        Ok(None) => {
            tracing::warn!(
                key = KEY_MAX_TOKENS,
                default = DEFAULT_MAX_TOKENS,
                "config row missing or empty after trim, using compiled-in default"
            );
            return DEFAULT_MAX_TOKENS;
        }
        Err(err) => {
            tracing::error!(
                key = KEY_MAX_TOKENS,
                error = %err,
                default = DEFAULT_MAX_TOKENS,
                "SQL error reading config, using compiled-in default"
            );
            return DEFAULT_MAX_TOKENS;
        }
    };

    let parsed: u32 = match trimmed.parse() {
        Ok(n) => n,
        Err(err) => {
            tracing::warn!(
                key = KEY_MAX_TOKENS,
                value = %trimmed,
                error = %err,
                default = DEFAULT_MAX_TOKENS,
                "config value failed to parse as u32, using compiled-in default"
            );
            return DEFAULT_MAX_TOKENS;
        }
    };

    let clamped = parsed.clamp(MAX_TOKENS_MIN, MAX_TOKENS_MAX);
    if clamped != parsed {
        tracing::warn!(
            key = KEY_MAX_TOKENS,
            value = parsed,
            clamped_to = clamped,
            min = MAX_TOKENS_MIN,
            max = MAX_TOKENS_MAX,
            "config value out of range [MAX_TOKENS_MIN, MAX_TOKENS_MAX], clamped"
        );
    }
    clamped
}

// ===========================================================================
// Log-emission tests (Forge T3-impl, complementary to Glitch's contract
// tests in `tests/test_config.rs`).
//
// These tests live as a `#[cfg(test)]` unit-test module inside this file
// rather than appended to the integration test, per the dispatch envelope's
// documented escape hatch (a). Rationale: in the integration-test binary
// for `site-core` (33+ parallel tests at default cargo parallelism on
// macOS), the larger callsite count + shared tracing global state produced
// a ~40% flake on the SQL-error capture cases (`get_max_tokens_emits_error
// _on_sql_failure` failed with empty captured buffer despite the ERROR
// firing on the call). The unit-test binary is smaller and exhibits zero
// flake at any parallelism level.
//
// Capture pattern: thread-local `tracing::subscriber::with_default` plus an
// explicit `Registry::default().with(fmt::Layer)` stack writing into a
// shared `Arc<Mutex<Vec<u8>>>`. ANSI is disabled so substring matching is
// stable. The `WARN ` and `ERROR ` needles match the line-leading level
// header emitted by `fmt` when `with_target(false).without_time()` is set.
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::layer::SubscriberExt;

    // -----------------------------------------------------------------------
    // Capture infra
    // -----------------------------------------------------------------------

    #[derive(Clone, Default)]
    struct LogBuf(Arc<Mutex<Vec<u8>>>);

    impl LogBuf {
        fn captured(&self) -> String {
            let bytes = self.0.lock().expect("log buffer mutex poisoned").clone();
            String::from_utf8(bytes).expect("log output must be valid UTF-8")
        }

        fn count(&self, needle: &str) -> usize {
            self.captured().matches(needle).count()
        }
    }

    impl io::Write for LogBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut inner = self
                .0
                .lock()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "log buffer mutex poisoned"))?;
            inner.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for LogBuf {
        type Writer = LogBuf;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn capture_logs<R>(f: impl FnOnce() -> R) -> (R, LogBuf) {
        let buf = LogBuf::default();
        let layer = tracing_subscriber::fmt::Layer::default()
            .with_writer(buf.clone())
            .with_ansi(false)
            .with_target(false)
            .without_time();
        let subscriber = tracing_subscriber::Registry::default().with(layer);
        let result = tracing::subscriber::with_default(subscriber, f);
        (result, buf)
    }

    // -----------------------------------------------------------------------
    // Fixtures: build a connection with a `site_config` table inline so the
    // unit-test module doesn't depend on the integration-test `common`
    // fixture or on the migration runner. Mirrors migration 005's seed.
    // -----------------------------------------------------------------------

    /// Connection with `site_config` table created and seeded with both
    /// `ai.model_id` and `ai.max_tokens` rows at their default values.
    /// Equivalent of `common::test_db()` for the unit-test scope.
    fn seeded_conn() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory()
            .expect("in-memory connection must open in test environment");
        conn.execute_batch(
            "CREATE TABLE site_config (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            INSERT INTO site_config (key, value) VALUES ('ai.model_id', 'claude-sonnet-4-6');
            INSERT INTO site_config (key, value) VALUES ('ai.max_tokens', '5530');",
        )
        .expect("schema + seed must succeed");
        conn
    }

    /// Bare connection with NO `site_config` table — used to force the
    /// SQL-error path in both accessors.
    fn bare_conn() -> rusqlite::Connection {
        rusqlite::Connection::open_in_memory().expect("in-memory connection must open")
    }

    fn set_value(conn: &rusqlite::Connection, key: &str, value: &str) {
        let updated = conn
            .execute(
                "UPDATE site_config SET value = ?1 WHERE key = ?2",
                rusqlite::params![value, key],
            )
            .expect("UPDATE must succeed");
        assert_eq!(updated, 1, "UPDATE must affect exactly one row");
    }

    fn delete_row(conn: &rusqlite::Connection, key: &str) {
        let deleted = conn
            .execute(
                "DELETE FROM site_config WHERE key = ?1",
                rusqlite::params![key],
            )
            .expect("DELETE must succeed");
        assert_eq!(deleted, 1, "DELETE must remove exactly one row");
    }

    // -----------------------------------------------------------------------
    // get_model_id log emissions (R3)
    // -----------------------------------------------------------------------

    /// R3: missing row → exactly one WARN naming the key. No ERROR.
    #[test]
    fn get_model_id_emits_warn_on_missing_row() {
        let conn = seeded_conn();
        delete_row(&conn, "ai.model_id");

        let (_, buf) = capture_logs(|| get_model_id(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("WARN "),
            1,
            "expected exactly one WARN event on missing row, captured: {captured}"
        );
        assert_eq!(
            buf.count("ERROR "),
            0,
            "no ERROR event expected on missing row, captured: {captured}"
        );
        assert!(
            captured.contains("ai.model_id"),
            "WARN event must name the key 'ai.model_id', captured: {captured}"
        );
    }

    /// R3: empty-after-trim → exactly one WARN.
    #[test]
    fn get_model_id_emits_warn_on_empty_value() {
        let conn = seeded_conn();
        set_value(&conn, "ai.model_id", "");

        let (_, buf) = capture_logs(|| get_model_id(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("WARN "),
            1,
            "expected exactly one WARN event on empty value, captured: {captured}"
        );
        assert_eq!(buf.count("ERROR "), 0);
        assert!(captured.contains("ai.model_id"));
    }

    /// R3: whitespace-padded but non-empty after trim → no log emission.
    #[test]
    fn get_model_id_emits_no_log_on_whitespace_padded_value() {
        let conn = seeded_conn();
        set_value(&conn, "ai.model_id", "  claude-sonnet-4-6  ");

        let (_, buf) = capture_logs(|| get_model_id(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("WARN "),
            0,
            "no WARN expected when value trims to non-empty, captured: {captured}"
        );
        assert_eq!(buf.count("ERROR "), 0);
    }

    /// R3: SQL error (table missing) → exactly one ERROR naming the key. No WARN.
    /// This exercises the helper's `Err` branch and the public accessor's
    /// error-only logging contract (no double-emit).
    #[test]
    fn get_model_id_emits_error_on_sql_failure() {
        let conn = bare_conn();

        let (_, buf) = capture_logs(|| get_model_id(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("ERROR "),
            1,
            "expected exactly one ERROR event on SQL failure, captured: {captured}"
        );
        assert_eq!(
            buf.count("WARN "),
            0,
            "no WARN expected on SQL failure (helper returns Err — public accessor logs error only, not warn), captured: {captured}"
        );
        assert!(
            captured.contains("ai.model_id"),
            "ERROR event must name the key 'ai.model_id', captured: {captured}"
        );
    }

    // -----------------------------------------------------------------------
    // get_max_tokens log emissions (R4)
    // -----------------------------------------------------------------------

    /// R4: missing row → exactly one WARN naming the key. No ERROR.
    #[test]
    fn get_max_tokens_emits_warn_on_missing_row() {
        let conn = seeded_conn();
        delete_row(&conn, "ai.max_tokens");

        let (_, buf) = capture_logs(|| get_max_tokens(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("WARN "),
            1,
            "expected exactly one WARN event on missing row, captured: {captured}"
        );
        assert_eq!(buf.count("ERROR "), 0);
        assert!(captured.contains("ai.max_tokens"));
    }

    /// R4: parse failure → exactly one WARN naming the offending value.
    #[test]
    fn get_max_tokens_emits_warn_on_parse_failure() {
        let conn = seeded_conn();
        set_value(&conn, "ai.max_tokens", "not-a-number");

        let (_, buf) = capture_logs(|| get_max_tokens(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("WARN "),
            1,
            "expected exactly one WARN event on parse failure, captured: {captured}"
        );
        assert_eq!(buf.count("ERROR "), 0);
        assert!(
            captured.contains("not-a-number"),
            "WARN event must name the malformed value, captured: {captured}"
        );
    }

    /// R4: clamp on upper-bound exceeded → exactly one WARN naming the value.
    #[test]
    fn get_max_tokens_emits_warn_on_upper_bound_clamp() {
        let conn = seeded_conn();
        set_value(&conn, "ai.max_tokens", "999999");

        let (_, buf) = capture_logs(|| get_max_tokens(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("WARN "),
            1,
            "expected exactly one WARN event on upper clamp, captured: {captured}"
        );
        assert_eq!(buf.count("ERROR "), 0);
        assert!(
            captured.contains("999999"),
            "WARN event must name the offending value 999999, captured: {captured}"
        );
    }

    /// R4: clamp on lower-bound zero → exactly one WARN.
    #[test]
    fn get_max_tokens_emits_warn_on_lower_bound_clamp() {
        let conn = seeded_conn();
        set_value(&conn, "ai.max_tokens", "0");

        let (_, buf) = capture_logs(|| get_max_tokens(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("WARN "),
            1,
            "expected exactly one WARN event on lower clamp, captured: {captured}"
        );
        assert_eq!(buf.count("ERROR "), 0);
    }

    /// R4: SQL error → exactly one ERROR. No WARN.
    #[test]
    fn get_max_tokens_emits_error_on_sql_failure() {
        let conn = bare_conn();

        let (_, buf) = capture_logs(|| get_max_tokens(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("ERROR "),
            1,
            "expected exactly one ERROR event on SQL failure, captured: {captured}"
        );
        assert_eq!(
            buf.count("WARN "),
            0,
            "no WARN expected on SQL failure, captured: {captured}"
        );
        assert!(captured.contains("ai.max_tokens"));
    }

    /// R4: in-range valid value → no log emission.
    #[test]
    fn get_max_tokens_emits_no_log_on_in_range_value() {
        let conn = seeded_conn();
        set_value(&conn, "ai.max_tokens", "4096");

        let (_, buf) = capture_logs(|| get_max_tokens(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("WARN "),
            0,
            "no WARN expected on in-range value, captured: {captured}"
        );
        assert_eq!(buf.count("ERROR "), 0);
    }

    /// R4: a value exactly at the configured upper bound (`MAX_TOKENS_MAX`)
    /// → no log emission (no clamp triggered). Tracks the constant rather
    /// than a hardcoded literal so it stays correct across ceiling changes
    /// (LLM-audit R2 lowered `MAX_TOKENS_MAX` from 200_000 to 12_000;
    /// mirrors the same constant-tracking amendment Glitch applied to the
    /// legacy clamp assertions in `tests/test_config.rs`).
    #[test]
    fn get_max_tokens_emits_no_log_at_upper_bound() {
        let conn = seeded_conn();
        set_value(&conn, "ai.max_tokens", &MAX_TOKENS_MAX.to_string());

        let (_, buf) = capture_logs(|| get_max_tokens(&conn));
        let captured = buf.captured();

        assert_eq!(
            buf.count("WARN "),
            0,
            "no WARN expected exactly at upper bound (no clamp performed), captured: {captured}"
        );
        assert_eq!(buf.count("ERROR "), 0);
    }
}
