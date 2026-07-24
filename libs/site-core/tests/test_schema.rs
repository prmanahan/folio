mod common;

#[test]
fn test_migration_creates_all_tables() {
    let conn = common::test_db();

    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let expected = vec![
        "admin_sessions",
        "ai_instructions",
        "articles",
        "candidate_profile",
        "education",
        "experiences",
        "faq_responses",
        "gaps_weaknesses",
        "links",
        "projects",
        "rate_limits",
        "skills",
        "values_culture",
    ];

    for table in &expected {
        assert!(
            tables.contains(&table.to_string()),
            "Missing table: {}",
            table
        );
    }
}

#[test]
fn test_migration_is_idempotent() {
    let conn = common::test_db();
    // Run migration again — should not error
    site_core::db::migrate(&conn).unwrap();
}

// ---------------------------------------------------------------------------
// Spec #572, Task 2 — Red-phase tests for migration 005 (site_config)
// ---------------------------------------------------------------------------

/// Test: migration 005 creates `site_config` with the correct column shape.
///
/// Given: a fresh in-memory database
/// When:  the migration runner runs (via `common::test_db()`)
/// Then:  the `site_config` table exists with columns `key`, `value`, and
///        `updated_at`, where `key` is NOT NULL PRIMARY KEY (TEXT), `value`
///        is NOT NULL (TEXT), and `updated_at` is NOT NULL with a DEFAULT
///        expression.
///
/// Red-phase failure: `site_config` table does not exist — migration 005 is
/// not yet implemented. Cargo-test output should contain a SQL error such as
/// "no such table: site_config".
#[test]
fn migration_005_creates_site_config_table_with_required_columns_and_constraints() {
    // Given: a fresh database with all migrations applied.
    let conn = common::test_db();

    // When: we query the column metadata for `site_config`.
    // This uses SQLite's pragma to retrieve per-column info.
    let mut stmt = conn
        .prepare("PRAGMA table_info(site_config)")
        .expect("PRAGMA table_info must not fail");

    #[derive(Debug)]
    struct ColInfo {
        name: String,
        col_type: String,
        not_null: bool,
        pk: i32,
        default_value: Option<String>,
    }

    let cols: Vec<ColInfo> = stmt
        .query_map([], |row| {
            Ok(ColInfo {
                name: row.get(1)?,
                col_type: row.get(2)?,
                not_null: row.get::<_, i32>(3)? != 0,
                pk: row.get(5)?,
                default_value: row.get(4)?,
            })
        })
        .expect("query_map must succeed")
        .filter_map(|r| r.ok())
        .collect();

    // Then: the table must exist (non-empty column list) and have the right shape.
    assert!(
        !cols.is_empty(),
        "site_config table must exist after migration 005; got no columns — migration 005 is not registered or the SQL file does not exist"
    );

    // key: TEXT PRIMARY KEY NOT NULL
    let key_col = cols
        .iter()
        .find(|c| c.name == "key")
        .expect("column `key` must exist in site_config");
    assert_eq!(
        key_col.col_type.to_uppercase(),
        "TEXT",
        "`key` column must be TEXT"
    );
    assert!(key_col.not_null, "`key` column must be NOT NULL");
    assert_eq!(key_col.pk, 1, "`key` column must be PRIMARY KEY");

    // value: TEXT NOT NULL (no DEFAULT clause required)
    let value_col = cols
        .iter()
        .find(|c| c.name == "value")
        .expect("column `value` must exist in site_config");
    assert_eq!(
        value_col.col_type.to_uppercase(),
        "TEXT",
        "`value` column must be TEXT"
    );
    assert!(value_col.not_null, "`value` column must be NOT NULL");

    // updated_at: TEXT NOT NULL DEFAULT (datetime('now'))
    let updated_col = cols
        .iter()
        .find(|c| c.name == "updated_at")
        .expect("column `updated_at` must exist in site_config");
    assert_eq!(
        updated_col.col_type.to_uppercase(),
        "TEXT",
        "`updated_at` column must be TEXT"
    );
    assert!(updated_col.not_null, "`updated_at` column must be NOT NULL");
    assert!(
        updated_col.default_value.is_some(),
        "`updated_at` column must have a DEFAULT expression"
    );
}

/// Test: migration 005 seeds exactly two rows with the expected values.
///
/// Given: a fresh in-memory database
/// When:  the migration runner runs
/// Then:  `SELECT key, value FROM site_config ORDER BY key` returns exactly
///        two rows: `(ai.max_tokens, 5530)` and `(ai.model_id, claude-sonnet-4-6)`.
///
/// Red-phase failure: the `site_config` table does not exist — SQL will error
/// with "no such table: site_config".
#[test]
fn migration_005_seeds_two_rows_on_fresh_db() {
    // Given: a fresh database with all migrations applied.
    let conn = common::test_db();

    // When: we query all rows from site_config ordered by key.
    let mut stmt = conn
        .prepare("SELECT key, value FROM site_config ORDER BY key")
        .expect("SELECT from site_config must not fail — table must exist after migration 005");

    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query_map must succeed")
        .filter_map(|r| r.ok())
        .collect();

    // Then: exactly two seed rows must be present.
    assert_eq!(
        rows.len(),
        2,
        "site_config must contain exactly 2 seed rows after migration 005; got: {:?}",
        rows
    );

    // Row 0 (alphabetically first): ai.max_tokens = 5530
    assert_eq!(
        rows[0].0, "ai.max_tokens",
        "first row key must be ai.max_tokens"
    );
    assert_eq!(rows[0].1, "5530", "ai.max_tokens seed value must be '5530'");

    // Row 1: ai.model_id = claude-sonnet-4-6
    assert_eq!(
        rows[1].0, "ai.model_id",
        "second row key must be ai.model_id"
    );
    assert_eq!(
        rows[1].1, "claude-sonnet-4-6",
        "ai.model_id seed value must be 'claude-sonnet-4-6'"
    );
}

/// Test: the MIGRATIONS const array registers migration 005.
///
/// This test is black-box: it asserts the *side effect* of MIGRATIONS
/// containing an entry for version 5. After `site_core::db::migrate()` runs,
/// the `_migrations` tracking table must record a row with version=5 and
/// name='site_config'. If migration 005 is not registered in the MIGRATIONS
/// array, the runner never records it and the assertion fails.
///
/// Given: a fresh in-memory database
/// When:  the migration runner runs
/// Then:  `SELECT version, name FROM _migrations WHERE version = 5` returns
///        one row with name = 'site_config'.
///
/// Red-phase failure: no row with version=5 in `_migrations` — migration 005
/// is not registered in the MIGRATIONS const array.
#[test]
fn migrations_const_array_registers_005_site_config() {
    // Given: a fresh database with all migrations applied.
    let conn = common::test_db();

    // When: we query the _migrations tracking table for version 5.
    let result: rusqlite::Result<(i32, String)> = conn.query_row(
        "SELECT version, name FROM _migrations WHERE version = 5",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    // Then: exactly one row must exist for version 5.
    let (version, name) = result.expect(
        "migration version 5 must appear in _migrations after running the migration runner — \
         add an entry for (5, 'site_config', include_str!(...)) to the MIGRATIONS const array \
         in libs/site-core/db/schema.rs",
    );

    assert_eq!(version, 5, "migration version must be 5");
    assert_eq!(
        name, "site_config",
        "migration name must be 'site_config' (matches the MIGRATIONS entry name field)"
    );
}

/// Test: the migration runner skips migration 005 when `_migrations` already
/// records it as applied, preserving any hand-edited values (Scenario 10 main path).
///
/// Given: a database where migration 005 has already been applied
/// And:   an operator has hand-edited `ai.model_id` to a future model name
/// When:  the migration runner is invoked again (simulating an app restart)
/// Then:  the `_migrations` table still records version 5 (no duplicate)
/// And:   the hand-edited value of `ai.model_id` is preserved
/// And:   the runner returns Ok (no error or panic)
///
/// Red-phase failure: the `site_config` table does not exist, so the UPDATE
/// that sets up the hand-edited value will fail — the assertion on the UPDATE
/// result will catch this before the runner re-run even occurs.
#[test]
fn runner_skips_005_when_migrations_table_records_it_applied() {
    // Given: a fresh database with migration 005 applied (all migrations run).
    let conn = common::test_db();

    // And: an operator has hand-edited ai.model_id to a future model name.
    let update_result = conn.execute(
        "UPDATE site_config SET value = 'claude-opus-5-20260101' WHERE key = 'ai.model_id'",
        [],
    );
    assert!(
        update_result.is_ok(),
        "UPDATE on site_config must succeed — site_config table must exist after migration 005 \
         (migration 005 not yet registered or SQL file missing)"
    );
    assert_eq!(
        update_result.unwrap(),
        1,
        "UPDATE must affect exactly 1 row — ai.model_id seed row must be present"
    );

    // When: the migration runner is invoked again (app restart simulation).
    let run_again_result = site_core::db::migrate(&conn);
    assert!(
        run_again_result.is_ok(),
        "re-running the migration runner must not error: {:?}",
        run_again_result
    );

    // Then: the hand-edited value is preserved (runner did not re-run migration 005).
    let model_id: String = conn
        .query_row(
            "SELECT value FROM site_config WHERE key = 'ai.model_id'",
            [],
            |row| row.get(0),
        )
        .expect("ai.model_id row must still exist");

    assert_eq!(
        model_id, "claude-opus-5-20260101",
        "hand-edited ai.model_id must be preserved — migration runner must skip 005 when \
         _migrations already records version 5 as applied"
    );

    // And: _migrations records exactly one row for version 5 (no duplicate insert).
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM _migrations WHERE version = 5",
            [],
            |row| row.get(0),
        )
        .expect("COUNT query on _migrations must succeed");
    assert_eq!(
        count, 1,
        "_migrations must record version 5 exactly once (no duplicate on re-run)"
    );
}

/// Test: belt-and-braces — if migration 005 SQL were re-executed directly,
/// `INSERT OR IGNORE` causes the seed rows to be no-ops, preserving any
/// hand-edited values (Scenario 10 belt-and-braces path).
///
/// Given: the migration 005 SQL has been applied (table + seed rows exist)
/// And:   an operator has hand-edited `ai.model_id` to a custom value
/// When:  the migration 005 SQL is executed again directly (forced re-run)
/// Then:  `INSERT OR IGNORE` causes seed rows to be no-ops
/// And:   the hand-edited value is still preserved
///
/// Red-phase failure: `migrations/005_site_config.sql` does not exist yet —
/// `std::fs::read_to_string` will return an error, causing this test to fail
/// before the belt-and-braces assertion is ever reached.
#[test]
fn forced_rerun_of_005_uses_insert_or_ignore_and_preserves_edits() {
    // Given: a fresh database (migration 005 applied if registered; otherwise
    // we manually create the table + seed to isolate the INSERT OR IGNORE behavior).
    //
    // The real red-phase trigger: reading the SQL file itself fails because the
    // file does not exist. This IS the assertion — the SQL file must ship with
    // the registration entry.
    let migration_path = format!(
        "{}/../../migrations/005_site_config.sql",
        env!("CARGO_MANIFEST_DIR")
    );
    let migration_sql = std::fs::read_to_string(&migration_path).unwrap_or_else(|e| {
        panic!(
            "migrations/005_site_config.sql must exist at {} — \
             Forge T2-impl creates this file; red-phase failure here is expected: {}",
            migration_path, e
        )
    });

    // Given: a connection with the table and seed rows already applied.
    let conn = common::test_db();

    // And: an operator has hand-edited ai.model_id.
    let update_result = conn.execute(
        "UPDATE site_config SET value = 'claude-future-model' WHERE key = 'ai.model_id'",
        [],
    );
    assert!(
        update_result.is_ok(),
        "UPDATE on site_config must succeed — site_config table must exist: {:?}",
        update_result
    );

    // When: the migration SQL is executed again (forced re-run simulation).
    let rerun_result = conn.execute_batch(&migration_sql);
    assert!(
        rerun_result.is_ok(),
        "forced re-run of migration 005 SQL must not error (INSERT OR IGNORE is idempotent): {:?}",
        rerun_result
    );

    // Then: the hand-edited value is preserved — INSERT OR IGNORE was a no-op.
    let model_id: String = conn
        .query_row(
            "SELECT value FROM site_config WHERE key = 'ai.model_id'",
            [],
            |row| row.get(0),
        )
        .expect("ai.model_id row must still exist after re-run");

    assert_eq!(
        model_id, "claude-future-model",
        "hand-edited ai.model_id must survive a forced re-run of migration 005 — \
         the migration must use INSERT OR IGNORE (not INSERT OR REPLACE)"
    );
}

// ---------------------------------------------------------------------------
// Spec #2715, R-0001 — Red-phase tests for migration 006 (experiences.visible)
// docs/specs/2026-07-24-experiences-visible-flag.md
// ---------------------------------------------------------------------------

/// Test: migration 006 adds `visible` to `experiences` with the correct
/// shape — INTEGER, NOT NULL, default the STRING "1" (PRAGMA returns
/// defaults as text; established by the 005 precedent above).
///
/// Given: a fresh in-memory database
/// When:  the migration runner runs (via `common::test_db()`)
/// Then:  `experiences.visible` exists, type INTEGER, NOT NULL, dflt_value
///        == Some("1")
///
/// Red-phase failure: `visible` is absent from `PRAGMA table_info(experiences)`
/// — migration 006 does not exist yet / is not registered.
#[test]
fn migration_006_adds_visible_column_integer_not_null_default_1() {
    // Given: a fresh database with all migrations applied.
    let conn = common::test_db();

    // When: we query the column metadata for `experiences`.
    let mut stmt = conn
        .prepare("PRAGMA table_info(experiences)")
        .expect("PRAGMA table_info must not fail");

    #[derive(Debug)]
    struct ColInfo {
        name: String,
        col_type: String,
        not_null: bool,
        default_value: Option<String>,
    }

    let cols: Vec<ColInfo> = stmt
        .query_map([], |row| {
            Ok(ColInfo {
                name: row.get(1)?,
                col_type: row.get(2)?,
                not_null: row.get::<_, i32>(3)? != 0,
                default_value: row.get(4)?,
            })
        })
        .expect("query_map must succeed")
        .filter_map(|r| r.ok())
        .collect();

    // Then: `visible` must exist with the right shape.
    let visible_col = cols.iter().find(|c| c.name == "visible").expect(
        "column `visible` must exist on `experiences` after migration 006 — \
         migration 006 is not registered in the MIGRATIONS const array \
         (libs/site-core/db/schema.rs) or migrations/006_experience_visibility.sql \
         does not exist",
    );
    assert_eq!(
        visible_col.col_type.to_uppercase(),
        "INTEGER",
        "`visible` column must be INTEGER"
    );
    assert!(visible_col.not_null, "`visible` column must be NOT NULL");
    assert_eq!(
        visible_col.default_value.as_deref(),
        Some("1"),
        "`visible` column default must be the string \"1\" (PRAGMA returns \
         defaults as text) — got: {:?}",
        visible_col.default_value
    );
}

/// Test: the MIGRATIONS const array registers migration 006.
///
/// Given: a fresh in-memory database
/// When:  the migration runner runs
/// Then:  `SELECT version, name FROM _migrations WHERE version = 6` returns
///        one row with name = 'experience_visibility'.
///
/// Red-phase failure: no row with version=6 in `_migrations` — migration 006
/// is not registered in the MIGRATIONS const array. Follows the same
/// dedicated-registration-test pattern as
/// `migrations_const_array_registers_005_site_config` above (repo
/// precedent, per R-0001's acceptance).
#[test]
fn migrations_const_array_registers_006_experience_visibility() {
    // Given: a fresh database with all migrations applied.
    let conn = common::test_db();

    // When: we query the _migrations tracking table for version 6.
    let result: rusqlite::Result<(i32, String)> = conn.query_row(
        "SELECT version, name FROM _migrations WHERE version = 6",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    // Then: exactly one row must exist for version 6.
    let (version, name) = result.expect(
        "migration version 6 must appear in _migrations after running the \
         migration runner — add an entry for (6, \"experience_visibility\", \
         include_str!(...)) to the MIGRATIONS const array in \
         libs/site-core/db/schema.rs",
    );

    assert_eq!(version, 6, "migration version must be 6");
    assert_eq!(
        name, "experience_visibility",
        "migration name must be 'experience_visibility' (matches the \
         MIGRATIONS entry name field)"
    );
}

/// Test: migration 006's SQL contains exactly one statement (the single
/// `ALTER TABLE`) — no UPDATE/INSERT/DELETE. R-0007's behaviour-preservation
/// claim rests on this: production DDL only, never a silent data mutation. A
/// conventional `--`-prefixed header comment block (the 005 precedent) is
/// expected and is NOT counted as a statement (R-0001's acceptance,
/// A-N1 disposition).
///
/// Red-phase failure: the file does not exist yet (accepted — mirrors
/// `forced_rerun_of_005_uses_insert_or_ignore_and_preserves_edits` above,
/// one migration further along).
#[test]
fn migration_006_sql_contains_only_the_single_alter_table_statement() {
    let path = format!(
        "{}/../../migrations/006_experience_visibility.sql",
        env!("CARGO_MANIFEST_DIR")
    );
    let sql = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "migrations/006_experience_visibility.sql must exist at {} — \
             the implementer creates this file; red-phase failure here is \
             expected: {}",
            path, e
        )
    });

    let code_only: String = sql
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("--")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let statements: Vec<String> = code_only
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    assert_eq!(
        statements.len(),
        1,
        "migration 006 must contain exactly one SQL statement (a header \
         comment block is expected and not counted); got {} non-comment \
         statement(s): {:?}",
        statements.len(),
        statements
    );
    let stmt_upper = statements[0].to_uppercase();
    assert!(
        stmt_upper.starts_with("ALTER TABLE"),
        "migration 006's single statement must be an ALTER TABLE, got: {}",
        statements[0]
    );
    for forbidden in ["UPDATE ", "INSERT ", "DELETE "] {
        assert!(
            !stmt_upper.contains(forbidden),
            "R-0007: migration 006 MUST NOT contain {} — the deploy must be \
             behaviour-preserving (no data mutation), got: {}",
            forbidden.trim(),
            statements[0]
        );
    }
}

/// Test: migration 006 is behaviour-preserving over PRE-EXISTING data — the
/// R-0001/R-0007 "migration-on-legacy-data" acceptance criterion.
///
/// Given: a database migrated only through 005 (built by applying
///   migrations/001..005 directly — `run_migrations_with`/`MIGRATIONS` are
///   crate-private, so this is the only black-box path to a genuinely
///   pre-006 schema state) with N pre-existing experience rows inserted
///   BEFORE 006 ever runs
/// When: migration 006's SQL is applied directly to that same connection
/// Then: every pre-existing row now has `visible = 1`, and the row set
///   returned by an identical query is the same identities in the same
///   order before and after
///
/// Distinct `display_order` values are used per the spec's red-phase note:
/// SQLite has no defined `ORDER BY` tie-break, and the live data is known to
/// carry `display_order` collisions — colliding seeds would make the
/// same-order assertion flaky.
///
/// Red-phase failure: `migrations/006_experience_visibility.sql` does not
/// exist yet — the file read fails. Accepted red state (mirrors
/// `forced_rerun_of_005_uses_insert_or_ignore_and_preserves_edits` above,
/// one migration further along); this test does not exercise the MIGRATIONS
/// runner/registration wiring (the dedicated registration test above already
/// covers that) — only the DDL's backfill behavior over legacy rows.
#[test]
fn migration_006_is_behaviour_preserving_over_pre_existing_experience_rows() {
    // Given: a DB migrated only through 005, built without touching the
    // crate-private MIGRATIONS array / run_migrations_with.
    let conn = rusqlite::Connection::open_in_memory().expect("in-memory db must open");
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size=-64000;
         PRAGMA temp_store=memory;",
    )
    .expect("PRAGMA setup must succeed");
    for filename in [
        "001_initial_schema",
        "002_agents",
        "003_page_hits",
        "004_profile_pitch_split",
        "005_site_config",
    ] {
        let path = format!(
            "{}/../../migrations/{}.sql",
            env!("CARGO_MANIFEST_DIR"),
            filename
        );
        let sql =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("must read {}: {}", path, e));
        conn.execute_batch(&sql)
            .unwrap_or_else(|e| panic!("must apply {}: {}", filename, e));
    }

    // And: N pre-existing rows, distinct display_order, inserted before 006
    // ever runs.
    conn.execute_batch(
        "INSERT INTO experiences (company_name, title, location, start_date, end_date, is_current, summary, bullet_points, display_order)
         VALUES ('Legacy Alpha', 'Engineer', 'Remote', '2018-01', '2020-01', 0, 'Alpha summary.', '[]', 10);
         INSERT INTO experiences (company_name, title, location, start_date, end_date, is_current, summary, bullet_points, display_order)
         VALUES ('Legacy Beta', 'Senior Engineer', 'Remote', '2020-02', '2022-06', 0, 'Beta summary.', '[]', 20);
         INSERT INTO experiences (company_name, title, location, start_date, end_date, is_current, summary, bullet_points, display_order)
         VALUES ('Legacy Gamma', 'Staff Engineer', 'Remote', '2022-07', NULL, 1, 'Gamma summary.', '[]', 30);",
    )
    .expect("legacy row inserts must succeed against the pre-006 schema");

    let pre_migration: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, company_name FROM experiences ORDER BY display_order ASC")
            .unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };
    assert_eq!(pre_migration.len(), 3, "sanity: 3 legacy rows inserted");

    // When: migration 006 runs.
    let path_006 = format!(
        "{}/../../migrations/006_experience_visibility.sql",
        env!("CARGO_MANIFEST_DIR")
    );
    let sql_006 = std::fs::read_to_string(&path_006).unwrap_or_else(|e| {
        panic!(
            "migrations/006_experience_visibility.sql must exist at {} — \
             the implementer creates this file; red-phase failure here is \
             expected: {}",
            path_006, e
        )
    });
    conn.execute_batch(&sql_006)
        .expect("migration 006 SQL must apply cleanly to a populated table");

    // Then: every pre-existing row now has visible = 1.
    let visibilities: Vec<(String, i64)> = {
        let mut stmt = conn
            .prepare("SELECT company_name, visible FROM experiences ORDER BY display_order ASC")
            .expect("`visible` column must exist after applying migration 006");
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };
    assert_eq!(visibilities.len(), 3);
    for (company, visible) in &visibilities {
        assert_eq!(
            *visible, 1,
            "legacy row {} must default to visible = 1 — R-0007: the \
             migration must not hide any pre-existing row",
            company
        );
    }

    // And: the row set/order is unchanged (R-0007 behaviour preservation) —
    // same identities, same order, before vs. after.
    let post_migration: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, company_name FROM experiences WHERE visible = 1 ORDER BY display_order ASC",
            )
            .unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };
    assert_eq!(
        post_migration, pre_migration,
        "R-0007: the row set/order must be unchanged by the migration"
    );
}
