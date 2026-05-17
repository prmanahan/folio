//! Spec #572, Task 3 — Red-phase tests for typed config accessors.
//!
//! Contract under test (per R3, R4, R5, R7):
//!   - `site_core::db::config::get_model_id(&Connection) -> String`
//!   - `site_core::db::config::get_max_tokens(&Connection) -> u32`
//!
//! Module path is load-bearing: `libs/site-core/db/config.rs` (NOT
//! `libs/site-core/config.rs` — that file already exists for env-var Config).
//!
//! Logging-assertion scope (R3/R4/R13/R17 calls for warn/error tracing
//! emissions). Exercising those would require a `tracing-subscriber` dev-dep
//! on `site-core` (it lives in workspace deps, but is not a site-core
//! dev-dep today). Spec R25 prohibits new dependencies beyond the rig-core
//! bump in R9. Per the dispatch envelope's escape hatch ("don't block on
//! logging-test infra if it would require Puck approval to add a dep"),
//! these tests assert the return-value contract for every scenario and
//! annotate the expected log emission inline with `// LOG: ...` comments.
//! A follow-up task (Forge T3-impl review or a separate dispatch) should
//! decide whether to thread `tracing-subscriber` into site-core's
//! dev-dependencies for log capture.
//!
//! Red-phase failure mode: this entire file fails to COMPILE because the
//! `site_core::db::config` module does not yet exist. That compile error
//! IS the red signal Forge T3-impl chases — it points directly at the
//! contract: create the module, expose the two named accessors, satisfy the
//! per-test scenarios below.

mod common;

use site_core::db::config::{get_max_tokens, get_model_id};

// ---------------------------------------------------------------------------
// get_model_id — happy path + fallbacks (R3, Scenarios 3, 5, 6)
// ---------------------------------------------------------------------------

/// Given: a fresh database with migration 005 seed rows in place
/// When:  `get_model_id` is called with the connection
/// Then:  the seeded value `"claude-sonnet-4-6"` is returned verbatim
///
/// Red-phase failure: `site_core::db::config` module does not exist yet.
#[test]
fn get_model_id_returns_seeded_value_on_fresh_db() {
    // Given: the standard test DB with all migrations (including 005) applied.
    let conn = common::test_db();

    // When: the typed accessor reads ai.model_id.
    let model_id = get_model_id(&conn);

    // Then: the seeded value is returned verbatim.
    assert_eq!(
        model_id, "claude-sonnet-4-6",
        "fresh DB seed value for ai.model_id must be 'claude-sonnet-4-6'"
    );
}

/// Given: a database where the `ai.model_id` row has been deleted
/// When:  `get_model_id` is called
/// Then:  the compiled-in default `"claude-sonnet-4-6"` is returned
/// And:   a `tracing::warn!` is emitted naming the missing key (R3)
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_model_id_returns_default_on_missing_row() {
    // Given: a fresh DB with the ai.model_id seed row removed.
    let conn = common::test_db();
    let deleted = conn
        .execute("DELETE FROM site_config WHERE key = 'ai.model_id'", [])
        .expect("DELETE on site_config must succeed");
    assert_eq!(deleted, 1, "DELETE must remove exactly the seeded row");

    // When: the accessor reads the missing key.
    let model_id = get_model_id(&conn);

    // Then: the compiled-in default fires.
    // LOG: tracing::warn! expected naming "ai.model_id" and the fallback path (R3).
    assert_eq!(
        model_id, "claude-sonnet-4-6",
        "missing ai.model_id row must yield compiled-in default 'claude-sonnet-4-6'"
    );
}

/// Given: a database where the `ai.model_id` row has `value = ""`
/// When:  `get_model_id` is called
/// Then:  the compiled-in default `"claude-sonnet-4-6"` is returned
/// And:   a `tracing::warn!` is emitted (R3, Scenario 5)
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_model_id_returns_default_on_empty_value() {
    // Given: a fresh DB with ai.model_id set to the empty string.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '' WHERE key = 'ai.model_id'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the empty value.
    let model_id = get_model_id(&conn);

    // Then: the compiled-in default fires.
    // LOG: tracing::warn! expected naming "ai.model_id" and the empty-after-trim fallback (R3).
    assert_eq!(
        model_id, "claude-sonnet-4-6",
        "empty ai.model_id value must yield compiled-in default 'claude-sonnet-4-6'"
    );
}

/// Given: a database where the `ai.model_id` row has `value = "   "` (whitespace-only)
/// When:  `get_model_id` is called
/// Then:  the compiled-in default `"claude-sonnet-4-6"` is returned (empty-after-trim)
/// And:   a `tracing::warn!` is emitted (R3)
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_model_id_returns_default_on_whitespace_only_value() {
    // Given: a fresh DB with ai.model_id set to whitespace-only.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '   ' WHERE key = 'ai.model_id'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the whitespace-only value.
    let model_id = get_model_id(&conn);

    // Then: the compiled-in default fires (whitespace trims to empty).
    // LOG: tracing::warn! expected naming "ai.model_id" and the empty-after-trim fallback (R3).
    assert_eq!(
        model_id, "claude-sonnet-4-6",
        "whitespace-only ai.model_id value must trim to empty and yield compiled-in default"
    );
}

/// Given: a database where `ai.model_id` has whitespace padding: `"  claude-sonnet-4-6  "`
/// When:  `get_model_id` is called
/// Then:  the trimmed value `"claude-sonnet-4-6"` is returned (R3, Scenario 6)
/// And:   NO warn is emitted (the trimmed value is non-empty)
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_model_id_strips_leading_and_trailing_whitespace() {
    // Given: a fresh DB with ai.model_id padded by whitespace on both sides.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '  claude-sonnet-4-6  ' WHERE key = 'ai.model_id'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the padded value.
    let model_id = get_model_id(&conn);

    // Then: leading and trailing whitespace are stripped; no fallback fires.
    // LOG: NO warn expected — trimmed value is non-empty (R3 / Scenario 6).
    assert_eq!(
        model_id, "claude-sonnet-4-6",
        "leading/trailing whitespace must be stripped from ai.model_id"
    );
}

/// Given: a database where `ai.model_id` has only leading whitespace: `"\t\t  claude-sonnet-4-6"`
/// When:  `get_model_id` is called
/// Then:  the trimmed value is returned (R3 — boundary on leading-only padding)
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_model_id_strips_leading_only_whitespace() {
    // Given: a fresh DB with ai.model_id padded by leading whitespace only (tabs + spaces).
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '		  claude-sonnet-4-6' WHERE key = 'ai.model_id'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the leading-padded value.
    let model_id = get_model_id(&conn);

    // Then: leading whitespace (including tabs) is stripped.
    // LOG: NO warn expected.
    assert_eq!(
        model_id, "claude-sonnet-4-6",
        "leading whitespace (spaces and tabs) must be stripped from ai.model_id"
    );
}

/// Given: a database where `ai.model_id` has only trailing whitespace: `"claude-sonnet-4-6  \t"`
/// When:  `get_model_id` is called
/// Then:  the trimmed value is returned (R3 — boundary on trailing-only padding)
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_model_id_strips_trailing_only_whitespace() {
    // Given: a fresh DB with ai.model_id padded by trailing whitespace only.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = 'claude-sonnet-4-6  	' WHERE key = 'ai.model_id'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the trailing-padded value.
    let model_id = get_model_id(&conn);

    // Then: trailing whitespace (including tabs) is stripped.
    // LOG: NO warn expected.
    assert_eq!(
        model_id, "claude-sonnet-4-6",
        "trailing whitespace (spaces and tabs) must be stripped from ai.model_id"
    );
}

/// Given: a database where `ai.model_id` has been hand-edited to a non-default value
/// When:  `get_model_id` is called
/// Then:  the hand-edited value is returned verbatim (no fallback) — R3 happy path
///        with operator-edited row.
///
/// This guards against a buggy impl that always returns the compiled-in
/// default. Without this assertion, a stub `fn get_model_id(_) -> String {
/// "claude-sonnet-4-6".to_string() }` would pass every other test in this file
/// (because the default IS the seed value).
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_model_id_returns_hand_edited_value_when_present() {
    // Given: a fresh DB with ai.model_id hand-edited to a future model name.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = 'claude-opus-5-20260101' WHERE key = 'ai.model_id'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the hand-edited value.
    let model_id = get_model_id(&conn);

    // Then: the hand-edited value comes back verbatim — no silent default override.
    // LOG: NO warn expected — value is non-empty after trim.
    assert_eq!(
        model_id, "claude-opus-5-20260101",
        "hand-edited ai.model_id value must be returned verbatim (impl must NOT always return the compiled-in default)"
    );
}

// ---------------------------------------------------------------------------
// get_max_tokens — happy path + fallbacks + clamp (R4, Scenarios 4, 7)
// ---------------------------------------------------------------------------

/// Given: a fresh database with migration 005 seed rows
/// When:  `get_max_tokens` is called
/// Then:  the seeded value `5530` is returned (R4 happy path)
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_returns_seeded_value_on_fresh_db() {
    // Given: the standard test DB with migration 005 applied.
    let conn = common::test_db();

    // When: the typed accessor reads ai.max_tokens.
    let max_tokens = get_max_tokens(&conn);

    // Then: the seeded value is returned.
    assert_eq!(
        max_tokens, 5530,
        "fresh DB seed value for ai.max_tokens must be 5530"
    );
}

/// Given: a database where the `ai.max_tokens` row has been deleted
/// When:  `get_max_tokens` is called
/// Then:  the compiled-in default `5530` is returned (R4)
/// And:   a `tracing::warn!` is emitted naming the missing key
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_returns_default_on_missing_row() {
    // Given: a fresh DB with the ai.max_tokens seed row removed.
    let conn = common::test_db();
    let deleted = conn
        .execute("DELETE FROM site_config WHERE key = 'ai.max_tokens'", [])
        .expect("DELETE on site_config must succeed");
    assert_eq!(deleted, 1, "DELETE must remove exactly the seeded row");

    // When: the accessor reads the missing key.
    let max_tokens = get_max_tokens(&conn);

    // Then: the compiled-in default fires.
    // LOG: tracing::warn! expected naming "ai.max_tokens" and the fallback path (R4).
    assert_eq!(
        max_tokens, 5530,
        "missing ai.max_tokens row must yield compiled-in default 5530"
    );
}

/// Given: a database where `ai.max_tokens` has `value = "not-a-number"` (Scenario 4)
/// When:  `get_max_tokens` is called
/// Then:  the compiled-in default `5530` is returned (parse failure → fallback)
/// And:   a `tracing::warn!` is emitted naming the parse failure (R4)
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_returns_default_on_malformed_value() {
    // Given: a fresh DB with ai.max_tokens set to garbage.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = 'not-a-number' WHERE key = 'ai.max_tokens'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the malformed value.
    let max_tokens = get_max_tokens(&conn);

    // Then: parse fails → fallback to the compiled-in default.
    // LOG: tracing::warn! expected naming "ai.max_tokens" and the parse-failure fallback (R4).
    assert_eq!(
        max_tokens, 5530,
        "malformed ai.max_tokens value must yield compiled-in default 5530"
    );
}

/// Given: a database where `ai.max_tokens` has `value = ""`
/// When:  `get_max_tokens` is called
/// Then:  the compiled-in default `5530` is returned (R4)
/// And:   a `tracing::warn!` is emitted
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_returns_default_on_empty_value() {
    // Given: a fresh DB with ai.max_tokens set to an empty string.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '' WHERE key = 'ai.max_tokens'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the empty value.
    let max_tokens = get_max_tokens(&conn);

    // Then: empty-after-trim → fallback.
    // LOG: tracing::warn! expected naming "ai.max_tokens" and the empty-after-trim fallback (R4).
    assert_eq!(
        max_tokens, 5530,
        "empty ai.max_tokens value must yield compiled-in default 5530"
    );
}

/// Given: a database where `ai.max_tokens` has `value = "   "` (whitespace-only)
/// When:  `get_max_tokens` is called
/// Then:  the compiled-in default `5530` is returned (empty-after-trim, R4)
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_returns_default_on_whitespace_only_value() {
    // Given: a fresh DB with ai.max_tokens set to whitespace only.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '   ' WHERE key = 'ai.max_tokens'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the whitespace-only value.
    let max_tokens = get_max_tokens(&conn);

    // Then: whitespace trims to empty → fallback.
    // LOG: tracing::warn! expected naming "ai.max_tokens" and the empty-after-trim fallback (R4).
    assert_eq!(
        max_tokens, 5530,
        "whitespace-only ai.max_tokens value must trim to empty and yield compiled-in default"
    );
}

/// Given: a database where `ai.max_tokens` has whitespace-padded numeric value: `"  5530  "`
/// When:  `get_max_tokens` is called
/// Then:  the trimmed-then-parsed value `5530` is returned (R4 — trim before parse)
///
/// This forces the impl to whitespace-trim before parsing. A naive
/// `value.parse::<u32>()` against `"  5530  "` returns Err — without the
/// trim, this would fall back to the default and the test would still pass
/// by coincidence. The assertion-by-coincidence is acceptable here only
/// because the whitespace-strip behavior is also covered by the model_id
/// counterpart test above; this test confirms the same trim-before-parse
/// contract holds for max_tokens.
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_trims_whitespace_before_parsing() {
    // Given: a fresh DB with ai.max_tokens padded with whitespace around a valid integer.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '  4096  ' WHERE key = 'ai.max_tokens'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the padded value.
    let max_tokens = get_max_tokens(&conn);

    // Then: trim-then-parse yields the integer (NOT the default — that would
    // mean the impl forgot to trim before parsing).
    // LOG: NO warn expected — value parses cleanly after trim and is in range.
    assert_eq!(
        max_tokens, 4096,
        "ai.max_tokens with surrounding whitespace must be trimmed before parsing — got {max_tokens} (default would indicate trim was skipped)"
    );
}

/// Given: a database where `ai.max_tokens` has `value = "999999"` (above 200_000) — Scenario 7
/// When:  `get_max_tokens` is called
/// Then:  the value is clamped to `200_000` (R4 upper bound)
/// And:   a `tracing::warn!` is emitted naming the offending value
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_clamps_to_upper_bound_when_exceeded() {
    // Given: a fresh DB with ai.max_tokens above the upper bound.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '999999' WHERE key = 'ai.max_tokens'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the over-range value.
    let max_tokens = get_max_tokens(&conn);

    // Then: the value is clamped to the upper bound (200_000).
    // LOG: tracing::warn! expected naming "999999" and the clamp behavior (R4).
    assert_eq!(
        max_tokens, 200_000,
        "ai.max_tokens=999999 must clamp to upper bound 200_000"
    );
}

/// Given: a database where `ai.max_tokens` has `value = "0"` (below the lower bound)
/// When:  `get_max_tokens` is called
/// Then:  the value is clamped to `1` (R4 lower bound)
/// And:   a `tracing::warn!` is emitted
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_clamps_to_lower_bound_when_zero() {
    // Given: a fresh DB with ai.max_tokens at zero (below the [1, 200_000] range).
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '0' WHERE key = 'ai.max_tokens'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the zero value.
    let max_tokens = get_max_tokens(&conn);

    // Then: the value is clamped to the lower bound (1).
    // LOG: tracing::warn! expected naming "0" and the clamp behavior (R4).
    assert_eq!(
        max_tokens, 1,
        "ai.max_tokens=0 must clamp to lower bound 1 (R4 inclusive range [1, 200_000])"
    );
}

/// Given: a database where `ai.max_tokens` has `value = "200000"` (exactly the upper bound)
/// When:  `get_max_tokens` is called
/// Then:  the value `200_000` is returned (passthrough, R4 boundary)
/// And:   NO warn is emitted (in-range)
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_passes_through_value_at_upper_bound() {
    // Given: a fresh DB with ai.max_tokens exactly at the upper bound.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '200000' WHERE key = 'ai.max_tokens'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the boundary value.
    let max_tokens = get_max_tokens(&conn);

    // Then: passthrough — the inclusive upper bound is honored.
    // LOG: NO warn expected — exactly at upper bound is in-range.
    assert_eq!(
        max_tokens, 200_000,
        "ai.max_tokens=200000 must pass through unchanged (inclusive upper bound)"
    );
}

/// Given: a database where `ai.max_tokens` has `value = "1"` (exactly the lower bound)
/// When:  `get_max_tokens` is called
/// Then:  the value `1` is returned (passthrough, R4 boundary)
/// And:   NO warn is emitted
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_passes_through_value_at_lower_bound() {
    // Given: a fresh DB with ai.max_tokens exactly at the lower bound.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '1' WHERE key = 'ai.max_tokens'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the boundary value.
    let max_tokens = get_max_tokens(&conn);

    // Then: passthrough — the inclusive lower bound is honored.
    // LOG: NO warn expected — exactly at lower bound is in-range.
    assert_eq!(
        max_tokens, 1,
        "ai.max_tokens=1 must pass through unchanged (inclusive lower bound)"
    );
}

/// Given: a database where `ai.max_tokens` has `value = "4294967295"` (u32::MAX)
/// When:  `get_max_tokens` is called
/// Then:  the value is clamped to `200_000` (R4 — far above upper bound)
/// And:   a `tracing::warn!` is emitted
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_clamps_u32_max_to_upper_bound() {
    // Given: a fresh DB with ai.max_tokens at the maximum u32 value.
    let conn = common::test_db();
    let u32_max_str = u32::MAX.to_string();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = ?1 WHERE key = 'ai.max_tokens'",
            [&u32_max_str],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads u32::MAX.
    let max_tokens = get_max_tokens(&conn);

    // Then: clamped to upper bound — must NOT panic on the parse, must NOT
    // overflow on the clamp. (Parsing u32::MAX as u32 succeeds; clamp pulls it down.)
    // LOG: tracing::warn! expected naming u32::MAX and the clamp behavior (R4).
    assert_eq!(
        max_tokens, 200_000,
        "ai.max_tokens=u32::MAX must clamp to upper bound 200_000 (no panic, no overflow)"
    );
}

/// Given: a database where `ai.max_tokens` has `value = "5000000000"` (above u32::MAX)
/// When:  `get_max_tokens` is called
/// Then:  the compiled-in default `5530` is returned — value is unparseable as `u32`,
///        so this is a parse-failure path NOT a clamp path (R4 spec ordering: parse
///        failure → fallback to default; clamp only applies to *parsed* values).
/// And:   a `tracing::warn!` is emitted (R4)
///
/// This test pins down the contract for above-u32-MAX inputs: they fall into
/// the parse-failure bucket, not the clamp bucket. An impl that special-cases
/// "looks-numeric, exceeds u32" to clamp instead of fallback would diverge
/// from the spec wording. R4: "the value cannot be parsed as `u32`" is one of
/// the fallback triggers; clamp applies only after a successful parse.
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_returns_default_when_value_exceeds_u32_max() {
    // Given: a fresh DB with ai.max_tokens above u32::MAX (cannot parse as u32).
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '5000000000' WHERE key = 'ai.max_tokens'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the unparseable value.
    let max_tokens = get_max_tokens(&conn);

    // Then: fallback fires — parse-failure path (NOT clamp).
    // LOG: tracing::warn! expected naming "ai.max_tokens" and the parse-failure fallback (R4).
    assert_eq!(
        max_tokens, 5530,
        "ai.max_tokens above u32::MAX must take the parse-failure fallback (5530), NOT the clamp path"
    );
}

/// Given: a database where `ai.max_tokens` has `value = "-1"` (negative)
/// When:  `get_max_tokens` is called
/// Then:  the compiled-in default `5530` is returned (negative is unparseable as `u32`)
/// And:   a `tracing::warn!` is emitted
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn get_max_tokens_returns_default_on_negative_value() {
    // Given: a fresh DB with ai.max_tokens set to a negative integer.
    let conn = common::test_db();
    let updated = conn
        .execute(
            "UPDATE site_config SET value = '-1' WHERE key = 'ai.max_tokens'",
            [],
        )
        .expect("UPDATE on site_config must succeed");
    assert_eq!(updated, 1, "UPDATE must affect exactly the seed row");

    // When: the accessor reads the negative value.
    let max_tokens = get_max_tokens(&conn);

    // Then: parse failure → fallback.
    // LOG: tracing::warn! expected naming "ai.max_tokens" and the parse-failure fallback (R4).
    assert_eq!(
        max_tokens, 5530,
        "ai.max_tokens=-1 must take the parse-failure fallback (negatives are not valid u32)"
    );
}

// ---------------------------------------------------------------------------
// Non-panic guarantees (R3, R4 — "MUST NOT panic")
// ---------------------------------------------------------------------------

/// Given: a database missing the `site_config` table entirely (simulated SQL
///        connection error / schema-missing scenario).
/// When:  `get_model_id` and `get_max_tokens` are called
/// Then:  both return their compiled-in defaults
/// And:   neither panics (R3, R4 explicit MUST NOT panic clause)
/// And:   a `tracing::error!` is emitted naming the underlying SQL error
///
/// This exercises the "SQL connection error" branch from R3/R4. A pristine
/// in-memory connection without any migrations applied has no `site_config`
/// table; the impl's SELECT will return a SQL error. The accessors must
/// swallow this and fall back, NOT propagate or panic.
///
/// Red-phase failure: `site_core::db::config` does not exist yet.
#[test]
fn accessors_do_not_panic_when_site_config_table_is_missing() {
    // Given: a bare in-memory connection with NO migrations applied — site_config absent.
    let conn = rusqlite::Connection::open_in_memory()
        .expect("open_in_memory must succeed in a test environment");

    // When: both accessors are invoked. Either may attempt SELECT against a
    // non-existent table; both must catch the error and fall back.
    let model_id = get_model_id(&conn);
    let max_tokens = get_max_tokens(&conn);

    // Then: both return their compiled-in defaults; neither panicked.
    // LOG: tracing::error! expected from each accessor naming the SQL error (R3/R4 SQL-error branch).
    assert_eq!(
        model_id, "claude-sonnet-4-6",
        "get_model_id must fall back to compiled-in default on missing-table SQL error (no panic, no propagate)"
    );
    assert_eq!(
        max_tokens, 5530,
        "get_max_tokens must fall back to compiled-in default on missing-table SQL error (no panic, no propagate)"
    );
}
