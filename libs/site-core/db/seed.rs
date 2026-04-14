use rusqlite::Connection;
use tracing::warn;

/// Seed the database with fictional demo data for testing.
/// Uses the same "Alex Rivera" persona as data/seed.sql.
///
/// Both pitch fields are populated inline so the seed is valid against the
/// pitch_short non-empty validation rule introduced in migration 004
/// (see models::profile::ProfileInput::validate).
pub fn seed_test_data(conn: &Connection) -> Result<(), rusqlite::Error> {
    // pitch_short: ≤280 chars, the hub-visible tweet-length pitch.
    // pitch_long: the original Alex Rivera bio (≤1500 chars, surfaced on /resume).
    conn.execute_batch(
        "
        UPDATE candidate_profile SET
            name = 'Alex Rivera',
            email = 'alex@example.com',
            title = 'Software Architect / Engineering Manager',
            location = 'Vancouver, BC',
            phone = '604-555-0199',
            pitch_short = 'Software architect with 12 years building distributed systems. Backend Java to architecture and team leadership.',
            pitch_long = 'Software architect with 12 years building distributed systems. Started in backend Java, evolved into architecture and team leadership. Track record of shipping reliable systems at scale while growing engineering teams.',
            availability_status = 'open',
            remote_preference = 'remote'
        WHERE id = 1;

        INSERT INTO experiences (company_name, title, location, start_date, end_date, is_current, summary, bullet_points, display_order)
        VALUES ('Meridian Systems', 'Software Architect', 'Vancouver, BC', '2022-01', NULL, 1, 'Event processing platform.', '[\"Designed event-driven architecture\"]', 1);

        INSERT INTO skills (skill_name, category, years_experience, last_used)
        VALUES ('Java', 'strong', 12, '2024'),
               ('Rust', 'moderate', 2, '2026');

        INSERT INTO education (degree, institution, location, start_year, end_year)
        VALUES ('BSc Computer Science', 'University of British Columbia', 'Vancouver, BC', '2010', '2014');

        INSERT INTO projects (title, slug, summary, description, tech_stack, url, sort_order, published)
        VALUES ('EventFlow', 'eventflow', 'Event processing framework.', 'Built at Meridian.', '[\"Go\",\"Kafka\"]', '', 1, 1);

        INSERT INTO articles (title, slug, summary, content, tags, published_at, published)
        VALUES ('Event Sourcing', 'event-sourcing', 'Lessons learned.', '# Event Sourcing', '[\"architecture\"]', '2025-11-15', 1);

        INSERT INTO links (label, url, icon, sort_order)
        VALUES ('LinkedIn', 'https://linkedin.com/in/alex-rivera-example', 'linkedin', 1),
               ('GitHub', 'https://github.com/alex-rivera-example', 'github', 2),
               ('Email', 'mailto:alex@example.com', 'mail', 3);

        INSERT INTO faq_responses (question, answer, is_common_question)
        VALUES ('What are you looking for?', 'Architecture roles with technical ownership.', 1),
               ('Why leaving?', 'Looking for a new challenge.', 0);
        "
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Live-profile fixture (R13/R14)
//
// A static, one-time capture of `https://peter.manahan.io/api/profile`
// transformed to the post-migration schema. Used by bounds tests to verify the
// hero hub renders without overflow against the real long-form pitch that
// triggered the original bug.
//
// NOT a runtime fetch — the JSON is committed alongside this file under
// `fixtures/live_profile.json` and embedded at compile time via include_str!.
// ---------------------------------------------------------------------------

const LIVE_PROFILE_JSON: &str = include_str!("fixtures/live_profile.json");

#[derive(serde::Deserialize)]
struct LiveProfile {
    name: String,
    email: String,
    title: String,
    location: String,
    phone: String,
    linkedin_url: String,
    github_url: String,
    twitter_url: String,
    pitch_short: String,
    pitch_long: String,
    availability_status: String,
    availability_date: String,
    remote_preference: String,
}

/// Parse the live-profile fixture JSON into a typed value.
/// Exposed for tests so the bounds suite can assert against the same data
/// that ships into the seed.
pub fn live_profile_fixture() -> serde_json::Value {
    serde_json::from_str(LIVE_PROFILE_JSON).expect("live_profile.json must parse")
}

/// Seed the candidate_profile row from the static live-profile fixture.
///
/// Selection: callers should invoke this only when the operator wants the
/// live-profile bounds case. The convenience entry point is
/// `seed_if_requested` which honors the `FOLIO_SEED` env var (`=live`).
///
/// The fixture's `pitch_short` is intentionally empty (Peter supplies it as
/// content work post-migration). `seed_live_profile` therefore writes the row
/// with empty `pitch_short`; the validation layer only fires on admin update,
/// not on seed inserts, so this is safe and explicit.
pub fn seed_live_profile(conn: &Connection) -> Result<(), rusqlite::Error> {
    warn!("seed_live_profile: fixture ships with empty pitch_short; admin UI will reject round-trip save until content work lands");
    let p: LiveProfile = serde_json::from_str(LIVE_PROFILE_JSON)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    conn.execute(
        "UPDATE candidate_profile SET
            name = ?1, email = ?2, title = ?3, location = ?4, phone = ?5,
            linkedin_url = ?6, github_url = ?7, twitter_url = ?8,
            pitch_short = ?9, pitch_long = ?10,
            availability_status = ?11, availability_date = ?12,
            remote_preference = ?13
         WHERE id = 1",
        rusqlite::params![
            p.name,
            p.email,
            p.title,
            p.location,
            p.phone,
            p.linkedin_url,
            p.github_url,
            p.twitter_url,
            p.pitch_short,
            p.pitch_long,
            p.availability_status,
            p.availability_date,
            p.remote_preference,
        ],
    )?;
    Ok(())
}

/// Convenience selector for operators: when `FOLIO_SEED=live` is set in the
/// environment, seed from the live-profile fixture; otherwise seed the default
/// Alex Rivera fixture. Used by the dev/server entry point.
pub fn seed_if_requested(conn: &Connection) -> Result<(), rusqlite::Error> {
    match std::env::var("FOLIO_SEED").as_deref() {
        Ok("live") => seed_live_profile(conn),
        _ => seed_test_data(conn),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    fn fresh_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn seed_test_data_populates_both_pitch_fields() {
        let conn = fresh_db();
        seed_test_data(&conn).unwrap();
        let (short, long): (String, String) = conn
            .query_row(
                "SELECT pitch_short, pitch_long FROM candidate_profile WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(!short.is_empty(), "pitch_short must be non-empty");
        assert!(
            short.chars().count() <= 280,
            "pitch_short must respect 280-char cap"
        );
        assert!(!long.is_empty(), "pitch_long must be non-empty");
        assert!(
            long.chars().count() <= 1500,
            "pitch_long must respect 1500-char cap"
        );
    }

    #[test]
    fn live_profile_fixture_parses_and_has_long_pitch() {
        let v = live_profile_fixture();
        let pitch_long = v.get("pitch_long").and_then(|x| x.as_str()).unwrap_or("");
        // The bug repro requires a pitch ≥500 chars; the live capture is ~700.
        assert!(
            pitch_long.chars().count() >= 500,
            "live fixture pitch_long must be ≥500 chars to repro the original bug"
        );
        assert!(
            pitch_long.chars().count() <= 1500,
            "live fixture pitch_long must respect 1500-char cap"
        );
    }

    #[test]
    fn seed_live_profile_writes_long_pitch() {
        let conn = fresh_db();
        seed_live_profile(&conn).unwrap();
        let pitch_long: String = conn
            .query_row(
                "SELECT pitch_long FROM candidate_profile WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(pitch_long.contains("15+ years"));
    }

    #[test]
    fn seed_if_requested_default_is_alex_rivera() {
        let conn = fresh_db();
        // Make sure the env var is unset for this assertion.
        // SAFETY: tests touching env vars must be serialized; we use a unique key to avoid races.
        unsafe {
            std::env::remove_var("FOLIO_SEED");
        }
        seed_if_requested(&conn).unwrap();
        let name: String = conn
            .query_row("SELECT name FROM candidate_profile WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "Alex Rivera");
    }
}
