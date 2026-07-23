use rusqlite::Connection;

/// All migrations in order. Each tuple is (version, name, sql).
/// SQL is loaded at compile time from the migrations/ directory.
const MIGRATIONS: &[(i32, &str, &str)] = &[
    (
        1,
        "initial_schema",
        include_str!("../../../migrations/001_initial_schema.sql"),
    ),
    (
        2,
        "agents",
        include_str!("../../../migrations/002_agents.sql"),
    ),
    (
        3,
        "page_hits",
        include_str!("../../../migrations/003_page_hits.sql"),
    ),
    (
        4,
        "profile_pitch_split",
        include_str!("../../../migrations/004_profile_pitch_split.sql"),
    ),
    (
        5,
        "site_config",
        include_str!("../../../migrations/005_site_config.sql"),
    ),
];

/// Ensure the migrations tracking table exists.
fn ensure_migrations_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
}

/// Run all pending migrations from `migrations`, in order.
///
/// Each migration's DDL and its `_migrations` bookkeeping row commit
/// atomically inside one `unchecked_transaction()` per migration — a crash
/// or error between the two can no longer leave a migration applied but
/// unrecorded (which would otherwise cause it to be re-run against an
/// already-migrated schema on the next boot). A failed migration rolls back
/// completely, including any DDL it already ran in the same batch; earlier
/// migrations that already committed stay committed.
///
/// `unchecked_transaction()` is used instead of `conn.transaction()` so this
/// helper (and the public `run_migrations` it backs) keeps taking
/// `&Connection` rather than `&mut Connection`, avoiding a signature change
/// that would ripple across every call site. The `&mut`-enforced guarantee
/// `transaction()` adds over `unchecked_transaction()` is compile-time
/// protection against *nested* transactions — not needed here because no
/// caller anywhere in this codebase ever holds an open transaction on the
/// connection passed in here (no other `.transaction()` /
/// `unchecked_transaction()` call exists anywhere else in the codebase), and
/// no migration SQL contains BEGIN/COMMIT/SAVEPOINT. This function *is*
/// called more than once per process on the same connection in some paths
/// (e.g. re-entrant `db::migrate` calls in tests) — that's fine precisely
/// because those calls are sequential, not nested. If a future change
/// introduces a caller that opens a transaction and holds it open across a
/// call into `run_migrations`/`run_migrations_with`, that invariant breaks
/// and `unchecked_transaction()`'s lack of nesting protection becomes a real
/// risk — switch to `conn.transaction()` (and thread `&mut Connection`
/// through) at that point.
fn run_migrations_with(
    conn: &Connection,
    migrations: &[(i32, &str, &str)],
) -> Result<(), rusqlite::Error> {
    ensure_migrations_table(conn)?;

    for (version, name, sql) in migrations {
        let applied: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM _migrations WHERE version = ?1",
            [version],
            |row| row.get(0),
        )?;

        if !applied {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
                rusqlite::params![version, name],
            )?;
            tx.commit()?;
        }
    }

    Ok(())
}

/// Run all pending migrations in order.
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    run_migrations_with(conn, MIGRATIONS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A migration whose batch fails partway (valid DDL, then an invalid
    /// statement) must roll back completely: no DDL effect AND no
    /// `_migrations` row. This is criterion 2 — the applied-but-unrecorded
    /// window the transaction wrap exists to close.
    #[test]
    fn failed_migration_rolls_back_ddl_and_migrations_row() {
        let conn = Connection::open_in_memory().unwrap();

        let migrations: &[(i32, &str, &str)] = &[(
            1,
            "bad_migration",
            "CREATE TABLE rollback_test (id INTEGER PRIMARY KEY); \
             INSERT INTO nonexistent_table (id) VALUES (1);",
        )];

        let result = run_migrations_with(&conn, migrations);
        assert!(
            result.is_err(),
            "a migration with an invalid statement must return Err"
        );

        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='rollback_test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            !table_exists,
            "DDL from a failed migration must roll back, even DDL that ran \
             before the failing statement in the same batch"
        );

        let recorded: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM _migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            !recorded,
            "a failed migration must not be recorded as applied in _migrations"
        );
    }

    /// Migrations that already committed before a later one fails must stay
    /// committed — per-migration transactions, not one transaction for the
    /// whole run.
    #[test]
    fn earlier_committed_migrations_survive_a_later_failure() {
        let conn = Connection::open_in_memory().unwrap();

        let migrations: &[(i32, &str, &str)] = &[
            (
                1,
                "good_migration",
                "CREATE TABLE good_table (id INTEGER PRIMARY KEY);",
            ),
            (
                2,
                "bad_migration",
                "CREATE TABLE bad_table (id INTEGER PRIMARY KEY); \
                 INSERT INTO nonexistent_table (id) VALUES (1);",
            ),
        ];

        let result = run_migrations_with(&conn, migrations);
        assert!(result.is_err(), "the second migration must fail");

        let good_table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='good_table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            good_table_exists,
            "an earlier successful migration's DDL must remain committed"
        );

        let good_recorded: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM _migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            good_recorded,
            "an earlier successful migration must remain recorded"
        );

        let bad_table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='bad_table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            !bad_table_exists,
            "the failed migration's DDL must not persist"
        );

        let bad_recorded: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM _migrations WHERE version = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            !bad_recorded,
            "the failed migration must not be recorded as applied"
        );
    }

    /// Happy-path sanity: a normal migration set applies via
    /// `run_migrations_with` and is recorded as expected, against a virgin
    /// DB where `run_migrations_with` creates `_migrations` itself. The
    /// `run_migrations -> run_migrations_with(conn, MIGRATIONS)` delegation
    /// isn't exercised here — that would require hardcoding the real
    /// `MIGRATIONS` const — but it's covered by the integration suite via
    /// `db::migrate -> run_migrations` (see `tests/common/mod.rs:18`).
    #[test]
    fn run_migrations_applies_and_records_a_successful_migration() {
        let conn = Connection::open_in_memory().unwrap();

        let migrations: &[(i32, &str, &str)] = &[(
            1,
            "ok_migration",
            "CREATE TABLE ok_table (id INTEGER PRIMARY KEY);",
        )];

        run_migrations_with(&conn, migrations).unwrap();

        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='ok_table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(table_exists, "a successful migration's DDL must commit");

        let recorded: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM _migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(recorded, "a successful migration must be recorded");
    }

    /// The exact shape of the production bug this fix closes: the DDL
    /// succeeds, but the bookkeeping INSERT that records it fails
    /// afterward. Unlike `failed_migration_rolls_back_ddl_and_migrations_row`
    /// above (which fails mid-batch, *before* the INSERT is ever reached,
    /// so its `!recorded` assertion passes even pre-fix and isn't doing
    /// discriminating work on its own), this migration's batch
    /// completes cleanly and only the INSERT fails, so a correct fix must
    /// roll back DDL that had already fully succeeded, not just abort DDL
    /// that was mid-flight.
    ///
    /// The migration drops `_migrations` itself as its DDL, which makes the
    /// following bookkeeping INSERT fail with "no such table". Pre-fix (no
    /// transaction wrap): the DROP commits immediately, so the probe table
    /// exists but `_migrations` is gone — applied-but-unrecorded, and worse,
    /// bookkeeping itself is destroyed. Post-fix: the whole batch rolls
    /// back — probe is absent and `_migrations` is intact.
    #[test]
    fn ddl_succeeds_but_bookkeeping_insert_fails_still_rolls_back() {
        let conn = Connection::open_in_memory().unwrap();

        let migrations: &[(i32, &str, &str)] = &[(
            1,
            "drops_its_own_bookkeeping_table",
            "CREATE TABLE probe (id INTEGER); DROP TABLE _migrations;",
        )];

        let result = run_migrations_with(&conn, migrations);
        assert!(
            result.is_err(),
            "the bookkeeping INSERT must fail once _migrations no longer exists"
        );

        let probe_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='probe'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            !probe_exists,
            "DDL that already succeeded must roll back when the bookkeeping \
             INSERT fails afterward"
        );

        let migrations_table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            migrations_table_exists,
            "the DROP TABLE _migrations must roll back too — bookkeeping stays intact"
        );
    }
}
