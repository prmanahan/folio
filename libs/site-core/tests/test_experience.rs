mod common;

#[tokio::test]
async fn test_list_experience_returns_public_fields() {
    let server = common::test_app();
    let response = server.get("/api/experience").await;
    response.assert_status_ok();

    let body: Vec<serde_json::Value> = response.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["company_name"], "Meridian Systems");
    assert_eq!(body[0]["title"], "Software Architect");

    // Verify Private-tier fields are NOT present
    assert!(body[0].get("why_joined").is_none());
    assert!(body[0].get("why_left").is_none());
    assert!(body[0].get("manager_would_say").is_none());
}

// ===========================================================================
// R-0002 / S1 — public exclusion of hidden rows
// (spec #2715, docs/specs/2026-07-24-experiences-visible-flag.md)
// ===========================================================================

/// Given experiences "Alpha Corp" (`visible = 1`, from the shared seed
///   fixture — represented here by "Meridian Systems") and "Hidden Corp"
///   (`visible = 0`, seeded in-test)
/// When an unauthenticated client requests GET /api/experience
/// Then the response contains the visible row, does NOT contain the hidden
///   row, and no object carries a `visible` key (S1, R-0002)
///
/// Seeded via `test_app_with_state()` — a server/DB independent of the
/// shared `test_app()` fixture — so `test_list_experience_returns_public_fields`
/// above (which asserts `body.len() == 1` against that shared fixture) is
/// unaffected.
///
/// Red-phase failure: seeding the hidden row fails with "no such column:
/// visible" — migration 006 does not exist yet. Accepted schema-red (mirrors
/// R-0003's red-phase form; the column does not exist until the implementer
/// creates migration 006).
#[tokio::test]
async fn public_list_excludes_hidden_rows_and_serializes_no_visible_key() {
    let (server, state) = common::test_app_with_state();
    {
        let conn = state.db.lock().expect("db lock");
        conn.execute(
            "INSERT INTO experiences (
                company_name, title, location, start_date, end_date, is_current,
                summary, bullet_points, display_order, visible
             ) VALUES (
                'Hidden Corp', 'Ghost Engineer', 'Nowhere', '2010-01', '2012-01', 0,
                'Hidden summary.', '[]', 99, 0
             )",
            [],
        )
        .expect(
            "seeding a visible = 0 row must succeed — migration 006 must \
             exist (visible column); accepted schema-red pre-006",
        );
    }

    let response = server.get("/api/experience").await;
    response.assert_status_ok();

    let body: Vec<serde_json::Value> = response.json();
    let companies: Vec<&str> = body
        .iter()
        .filter_map(|row| row["company_name"].as_str())
        .collect();

    assert!(
        companies.contains(&"Meridian Systems"),
        "the visible seed row must still be present, got: {:?}",
        companies
    );
    assert!(
        !companies.contains(&"Hidden Corp"),
        "the hidden row must be excluded from the public list, got: {:?}",
        companies
    );
    for row in &body {
        assert!(
            row.get("visible").is_none(),
            "public response objects must not serialize a `visible` key, \
             got: {:?}",
            row
        );
    }
}
