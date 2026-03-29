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
