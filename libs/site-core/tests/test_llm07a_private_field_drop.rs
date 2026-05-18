//! Red-phase acceptance tests for R3 / LLM07(a) — drop the three most
//! sensitive private fields from the constructed prompt.
//!
//! Spec: `docs/specs/2026-05-18-llm-audit-remediation.md` R3, decision
//! "LLM07 disposition: option (a) — narrow classify + drop."
//!
//! The DB still stores `would_do_differently`, `manager_would_say`,
//! `honest_notes` (admin-managed data). Only the prompt-construction path
//! (`ai/context.rs::build_system_prompt`) must stop reading them. These
//! tests assert the *constructed prompt string* and the *construction code
//! path*, never the storage layer — schema columns / struct fields / admin
//! CRUD are explicitly out of scope and stay.
//!
//! Four acceptance bullets, each with explicit coverage:
//!  - Bullet 1 (data-absence): unique sentinels in the three fields →
//!    none appear in the constructed prompt.
//!  - Bullet 2 (structural): a source-as-text meta-test asserting the
//!    field-access expressions are gone from the construction path.
//!  - Bullet 3 (retained-field regression): a sentinel in a RETAINED
//!    field still appears — the drop is surgical, not a blanket gut.
//!  - Bullet 4 (DATA_CLASSIFICATION example-list polish): optional, NOT
//!    gated here.
//!
//! Red-phase failure mode: bullets 1 and 3 fail by assertion (current
//! `build_system_prompt` interpolates all three fields, so the sentinels
//! ARE present); bullet 2 fails by assertion (the accessor expressions
//! `exp.would_do_differently`, `exp.manager_would_say`, `s.honest_notes`
//! are present in `ai/context.rs` today). All compile cleanly against the
//! existing public API.

mod common;

use rusqlite::Connection;
use site_core::ai::context::build_system_prompt;

// ---------------------------------------------------------------------------
// Fixture: the seed (`db/seed.rs`) inserts experience/skill rows WITHOUT the
// private fields, so we cannot rely on it for sentinel injection. Instead we
// migrate a fresh DB and INSERT one experience row + one skill row carrying
// unique sentinels in every field of interest (dropped AND retained). Mirrors
// the pattern in `ai/context.rs`'s own unit-test `seed_experience`/`seed_skills`.
// ---------------------------------------------------------------------------

/// Unique, high-entropy sentinels — one per dropped field. Chosen so a
/// substring match is unambiguous and a paraphrase/high-similarity span
/// would still trip a contains() check on the literal token.
const SENTINEL_WOULD_DO_DIFFERENTLY: &str = "ZZSENTINEL-would-do-differently-7f3a9b2c-DROP";
const SENTINEL_MANAGER_WOULD_SAY: &str = "ZZSENTINEL-manager-would-say-1d8e4c0a-DROP";
const SENTINEL_HONEST_NOTES: &str = "ZZSENTINEL-honest-notes-9b6f2e5d-DROP";

/// Sentinels in RETAINED fields — these MUST survive (bullet 3).
const SENTINEL_WHY_JOINED: &str = "ZZSENTINEL-why-joined-3c7a1f8b-KEEP";
const SENTINEL_ACTUAL_CONTRIBUTIONS: &str = "ZZSENTINEL-actual-contributions-5e2d9a4c-KEEP";
const SENTINEL_LESSONS_LEARNED: &str = "ZZSENTINEL-lessons-learned-8a0b6d3f-KEEP";
const SENTINEL_REPORTS_WOULD_SAY: &str = "ZZSENTINEL-reports-would-say-2f4c8e1a-KEEP";
const SENTINEL_SKILL_EVIDENCE: &str = "ZZSENTINEL-skill-evidence-6d1b9c7e-KEEP";

/// Migrate a fresh in-memory DB and insert a profile + one experience + one
/// skill, every field of interest stuffed with its unique sentinel.
fn db_with_sentinel_rows() -> Connection {
    let conn = common::test_db();

    // Profile must exist & be non-empty for build_system_prompt to proceed.
    conn.execute(
        "UPDATE candidate_profile SET
            name = 'Sentinel Subject',
            title = 'Test Architect',
            pitch_short = 'Short pitch.',
            pitch_long = 'Long pitch for AI context.',
            career_narrative = 'Career narrative body.'
         WHERE id = 1",
        [],
    )
    .expect("profile UPDATE must succeed");

    // One experience row: dropped fields + retained fields both carry
    // distinct sentinels.
    conn.execute(
        "INSERT INTO experiences (
            company_name, title, location, start_date, end_date, is_current,
            summary, bullet_points, display_order,
            why_joined, why_left, actual_contributions, proudest_achievement,
            would_do_differently, challenges_faced, lessons_learned,
            manager_would_say, reports_would_say
         ) VALUES (
            'Sentinel Corp', 'Lead', 'Remote', '2020-01', '2023-01', 0,
            'Summary text.', '[\"Public bullet\"]', 1,
            ?1, 'Why left text', ?2, 'Proudest text',
            ?3, 'Challenges text', ?4,
            ?5, ?6
         )",
        rusqlite::params![
            SENTINEL_WHY_JOINED,           // why_joined (RETAINED)
            SENTINEL_ACTUAL_CONTRIBUTIONS, // actual_contributions (RETAINED)
            SENTINEL_WOULD_DO_DIFFERENTLY, // would_do_differently (DROPPED)
            SENTINEL_LESSONS_LEARNED,      // lessons_learned (RETAINED)
            SENTINEL_MANAGER_WOULD_SAY,    // manager_would_say (DROPPED)
            SENTINEL_REPORTS_WOULD_SAY,    // reports_would_say (RETAINED)
        ],
    )
    .expect("experience INSERT must succeed");

    // One skill row: honest_notes (DROPPED) + evidence (RETAINED) sentinels.
    conn.execute(
        "INSERT INTO skills (skill_name, category, years_experience, last_used,
                             self_rating, evidence, honest_notes)
         VALUES ('Rust', 'strong', 5, '2026', 4, ?1, ?2)",
        rusqlite::params![SENTINEL_SKILL_EVIDENCE, SENTINEL_HONEST_NOTES],
    )
    .expect("skill INSERT must succeed");

    conn
}

// ===========================================================================
// Bullet 1 — data-absence. Sentinel data (NOT the field-name label) must
// be absent from the constructed prompt. This is the assertion the spec
// names as the gate; `!prompt.contains("manager_would_say")` (the label)
// is explicitly forbidden as the weak test.
// ===========================================================================

/// Given an experience row whose `would_do_differently`, `manager_would_say`
///   and a skill row whose `honest_notes` each hold a unique sentinel
/// When the system prompt is constructed via build_system_prompt
/// Then NONE of the three dropped-field sentinels appear in the prompt
///
/// Red-phase: current `ai/context.rs` interpolates all three (lines 169,
/// 181, 210), so every sentinel IS present → this fails by assertion.
#[test]
fn constructed_prompt_contains_none_of_the_three_dropped_field_sentinels() {
    let conn = db_with_sentinel_rows();
    let prompt = build_system_prompt(&conn).expect("prompt build must succeed");

    assert!(
        !prompt.contains(SENTINEL_WOULD_DO_DIFFERENTLY),
        "R3: experience.would_do_differently data MUST NOT appear in the \
         constructed prompt (LLM07a drop). Found sentinel {SENTINEL_WOULD_DO_DIFFERENTLY:?}"
    );
    assert!(
        !prompt.contains(SENTINEL_MANAGER_WOULD_SAY),
        "R3: experience.manager_would_say data MUST NOT appear in the \
         constructed prompt (LLM07a drop). Found sentinel {SENTINEL_MANAGER_WOULD_SAY:?}"
    );
    assert!(
        !prompt.contains(SENTINEL_HONEST_NOTES),
        "R3: skill.honest_notes data MUST NOT appear in the constructed \
         prompt (LLM07a drop). Found sentinel {SENTINEL_HONEST_NOTES:?}"
    );
}

/// Anti-paraphrase guard: assert no high-similarity span survives. The
/// sentinels are opaque tokens with no natural-language meaning, so any
/// occurrence of a substantial contiguous slice (>= 20 chars) of any
/// dropped sentinel indicates the data leaked even if not byte-identical.
#[test]
fn constructed_prompt_has_no_high_similarity_span_of_dropped_sentinels() {
    let conn = db_with_sentinel_rows();
    let prompt = build_system_prompt(&conn).expect("prompt build must succeed");

    for sentinel in [
        SENTINEL_WOULD_DO_DIFFERENTLY,
        SENTINEL_MANAGER_WOULD_SAY,
        SENTINEL_HONEST_NOTES,
    ] {
        // Slide a 20-char window over the sentinel; assert no window is a
        // substring of the prompt.
        let chars: Vec<char> = sentinel.chars().collect();
        let window = 20.min(chars.len());
        for start in 0..=chars.len().saturating_sub(window) {
            let span: String = chars[start..start + window].iter().collect();
            assert!(
                !prompt.contains(&span),
                "R3: a >= {window}-char span of dropped sentinel {sentinel:?} \
                 leaked into the prompt: span={span:?}"
            );
        }
    }
}

// ===========================================================================
// Bullet 2 — STRUCTURAL meta-test. The field-access expressions must not
// appear on the context-construction path. This is the defense against a
// future regression that re-introduces the read via paraphrase while still
// passing bullet 1 (e.g. a different label string but same field read).
//
// Source-as-text only — reads the file at runtime, does NOT modify it.
// CARGO_MANIFEST_DIR for this integration test resolves to libs/site-core/.
// ===========================================================================

/// Given the context-construction source files
/// When their text is scanned for the dropped-field accessor expressions
/// Then `exp.would_do_differently`, `exp.manager_would_say`, and
///   `s.honest_notes` do NOT appear in `ai/context.rs`, and the dropped
///   field labels are not interpolated from `ai/prompt_templates.rs` into
///   the construction path
///
/// Exact accessor forms verified against the parent commit:
///  - `exp.would_do_differently` — ai/context.rs:169
///  - `exp.manager_would_say`    — ai/context.rs:181
///  - `s.honest_notes`           — ai/context.rs:210 (format_skill closure)
///
/// Red-phase: all three expressions are present in ai/context.rs today →
/// this fails by assertion. The implementer removes the interpolation
/// blocks; this then passes.
#[test]
fn context_construction_path_does_not_read_the_three_dropped_field_accessors() {
    let context_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ai/context.rs"
    ))
    .expect("ai/context.rs must be readable");

    // Strip the `#[cfg(test)] mod tests` block: the unit-test seed helpers
    // legitimately reference these field names in SQL string literals and
    // we only assert on the production construction path. The tests module
    // begins at `mod tests {` after a `#[cfg(test)]` attribute; scanning
    // only the pre-test region keeps this a construction-path assertion.
    let construction_region = match context_src.find("#[cfg(test)]") {
        Some(idx) => &context_src[..idx],
        None => context_src.as_str(),
    };

    // Anti-silent-truncation guard: if a future refactor adds a second,
    // higher `#[cfg(test)]` (e.g. a cfg(test)-only helper above the
    // construction code), the strip above would truncate the construction
    // region and this meta-test would pass on incomplete coverage. Assert
    // the construction region still contains the function we are scanning.
    assert!(
        construction_region.contains("pub fn build_system_prompt"),
        "R3 bullet 2 meta-test integrity: the scanned construction region \
         MUST still contain `pub fn build_system_prompt` — if it does not, \
         the #[cfg(test)] strip cut too much and this assertion would be \
         silently vacuous"
    );

    assert!(
        !construction_region.contains("exp.would_do_differently"),
        "R3 bullet 2 (structural): `exp.would_do_differently` MUST NOT \
         appear on the context-construction path in ai/context.rs — the \
         field read must be removed, not just relabeled"
    );
    assert!(
        !construction_region.contains("exp.manager_would_say"),
        "R3 bullet 2 (structural): `exp.manager_would_say` MUST NOT appear \
         on the context-construction path in ai/context.rs"
    );
    assert!(
        !construction_region.contains("s.honest_notes"),
        "R3 bullet 2 (structural): `s.honest_notes` MUST NOT appear on the \
         context-construction path in ai/context.rs (format_skill closure)"
    );
}

/// Belt-and-suspenders for bullet 2: the raw column-bound names must also
/// not be interpolated via any aliasing rebind on the construction path.
/// We additionally scan for the field-label strings the construction code
/// emits today ("Would do differently:", "Manager would say:",
/// ". Notes:" for honest_notes) to catch a rename-the-binding-keep-the-read
/// regression. The DATA_CLASSIFICATION prose block in prompt_templates.rs
/// may still *mention* these labels harmlessly (bullet 4) — so we scan
/// ai/context.rs construction region only, NOT prompt_templates.rs.
#[test]
fn context_construction_path_does_not_emit_the_three_dropped_field_labels() {
    let context_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ai/context.rs"
    ))
    .expect("ai/context.rs must be readable");
    let construction_region = match context_src.find("#[cfg(test)]") {
        Some(idx) => &context_src[..idx],
        None => context_src.as_str(),
    };
    assert!(
        construction_region.contains("pub fn build_system_prompt"),
        "R3 bullet 2 meta-test integrity: scanned region MUST still contain \
         `pub fn build_system_prompt` (guards against an over-eager \
         #[cfg(test)] strip making this assertion vacuous)"
    );

    // The literal emission strings used at ai/context.rs:170, 182, 211.
    assert!(
        !construction_region.contains("Would do differently:"),
        "R3 bullet 2: the 'Would do differently:' emission MUST be removed \
         from the construction path"
    );
    assert!(
        !construction_region.contains("Manager would say:"),
        "R3 bullet 2: the 'Manager would say:' emission MUST be removed \
         from the construction path"
    );
    // honest_notes is emitted as `. Notes: {}` inside format_skill.
    assert!(
        !construction_region.contains(". Notes:"),
        "R3 bullet 2: the honest_notes '. Notes:' emission MUST be removed \
         from the format_skill closure"
    );
}

// ===========================================================================
// Bullet 3 — retained-field regression. The drop must be SURGICAL: a
// sentinel in a retained field still appears. Without this, an
// implementer could pass bullets 1+2 by gutting the whole experience /
// skill section.
// ===========================================================================

/// Given retained-field sentinels (why_joined, actual_contributions,
///   lessons_learned, reports_would_say on experience; evidence on skill)
/// When the system prompt is constructed
/// Then each retained sentinel STILL appears — proving the drop is
///   surgical, not a blanket removal of the private-context section
///
/// Red-phase note: this currently PASSES (those fields are interpolated
/// today). It is the anti-over-removal guard that must STILL pass after
/// Forge's surgical drop — it fails ONLY if the implementer over-removes.
/// It is included now so the suite is complete and Forge sees the full
/// contract; the gate for the drop itself is bullets 1 and 2.
#[test]
fn constructed_prompt_retains_non_dropped_private_field_sentinels() {
    let conn = db_with_sentinel_rows();
    let prompt = build_system_prompt(&conn).expect("prompt build must succeed");

    assert!(
        prompt.contains(SENTINEL_WHY_JOINED),
        "R3 bullet 3: experience.why_joined is RETAINED — its data MUST \
         still appear (drop must be surgical)"
    );
    assert!(
        prompt.contains(SENTINEL_ACTUAL_CONTRIBUTIONS),
        "R3 bullet 3: experience.actual_contributions is RETAINED — its \
         data MUST still appear"
    );
    assert!(
        prompt.contains(SENTINEL_LESSONS_LEARNED),
        "R3 bullet 3: experience.lessons_learned is RETAINED — its data \
         MUST still appear"
    );
    assert!(
        prompt.contains(SENTINEL_REPORTS_WOULD_SAY),
        "R3 bullet 3: experience.reports_would_say is RETAINED — its data \
         MUST still appear (note: reports_would_say STAYS; only \
         manager_would_say is dropped)"
    );
    assert!(
        prompt.contains(SENTINEL_SKILL_EVIDENCE),
        "R3 bullet 3: skill.evidence is RETAINED — its data MUST still \
         appear (only skill.honest_notes is dropped)"
    );
}

#[allow(dead_code)]
fn _silence_unused() {
    let _ = common::seeded_db();
}
