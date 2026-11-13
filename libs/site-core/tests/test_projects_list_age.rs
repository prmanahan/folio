//! Unit tests for the R-18 age function (plan T3). Red phase: written before
//! the function exists.
//!
//! The spec mandates a unit test on a `now`-parameterised age function, so
//! this is the one white-box file in the T3 suite: it calls the function
//! with LITERAL strings and no formatter. No implementation code was read.
//!
//! Unpinned by spec and picked here (dogfooder-default, revisit if T4 finds
//! a better shape; the tests are the only caller):
//!   `site_core::projects_list::check_fetched_at_age(fetched_at: &str,
//!    now: time::OffsetDateTime) -> Result<T, E>`
//! `Ok` means the age is within the bound; `Err` means it is outside it.
//! `now` is built from a literal RFC 3339 string with `time`'s `parsing`
//! feature (dev-dependency here); `fetched_at` is passed as the literal.
//!
//! Both bounds are inclusive: exactly 30 minutes old and exactly 5 minutes
//! ahead pass; one second beyond either fails (R-18: "more than 30 minutes
//! before ... more than 5 minutes after ... exactly 30 and exactly 5 pass").

use site_core::projects_list::check_fetched_at_age;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

fn now(literal: &str) -> OffsetDateTime {
    OffsetDateTime::parse(literal, &Rfc3339).expect("test literal is valid RFC 3339")
}

#[test]
fn r18_month_boundary_exactly_30_minutes_old_passes() {
    assert!(check_fetched_at_age("2026-11-30T23:59:00.000Z", now("2026-12-01T00:29:00Z")).is_ok());
}

#[test]
fn r18_month_boundary_30_minutes_and_one_second_old_fails() {
    assert!(check_fetched_at_age("2026-11-30T23:59:00.000Z", now("2026-12-01T00:29:01Z")).is_err());
}

#[test]
fn r18_leap_day_29_minutes_old_passes() {
    assert!(check_fetched_at_age("2028-02-29T23:50:00.000Z", now("2028-03-01T00:19:00Z")).is_ok());
}

#[test]
fn r18_leap_day_31_minutes_old_fails() {
    assert!(check_fetched_at_age("2028-02-29T23:50:00.000Z", now("2028-03-01T00:21:00Z")).is_err());
}

#[test]
fn r18_exactly_5_minutes_ahead_passes() {
    assert!(check_fetched_at_age("2026-12-01T00:04:00.000Z", now("2026-11-30T23:59:00Z")).is_ok());
}

#[test]
fn r18_5_minutes_and_one_second_ahead_fails() {
    assert!(check_fetched_at_age("2026-12-01T00:04:00.000Z", now("2026-11-30T23:58:59Z")).is_err());
}

#[test]
fn r18_age_zero_passes() {
    assert!(check_fetched_at_age("2026-11-13T12:00:00.000Z", now("2026-11-13T12:00:00Z")).is_ok());
}

#[test]
fn r18_30_minutes_and_one_millisecond_old_fails() {
    // "more than 30 minutes before": strict, at sub-second precision too
    assert!(
        check_fetched_at_age("2026-11-13T12:00:00.000Z", now("2026-11-13T12:30:00.001Z")).is_err()
    );
}

#[test]
fn r18_exactly_30_minutes_old_with_fractional_seconds_passes() {
    assert!(
        check_fetched_at_age("2026-11-13T12:00:00.500Z", now("2026-11-13T12:30:00.500Z")).is_ok()
    );
}

#[test]
fn r18_5_minutes_and_one_millisecond_ahead_fails() {
    assert!(
        check_fetched_at_age("2026-11-13T12:05:00.001Z", now("2026-11-13T12:00:00.000Z")).is_err()
    );
}

#[test]
fn r18_now_with_a_numeric_offset_is_the_same_instant() {
    // 02:29:00+02:00 is 00:29:00Z: exactly 30 minutes after the fetch
    assert!(
        check_fetched_at_age("2026-11-30T23:59:00.000Z", now("2026-12-01T02:29:00+02:00")).is_ok()
    );
    assert!(
        check_fetched_at_age("2026-11-30T23:59:00.000Z", now("2026-12-01T02:29:01+02:00")).is_err()
    );
}

#[test]
fn r18_verdict_is_exactly_the_window_minus_300s_to_plus_1800s_across_a_year_end() {
    // Property: for every whole-second offset, Ok iff -300 <= age <= 1800
    // seconds. Base crosses year end and uses literal strings throughout.
    let fetched = "2026-12-31T23:40:00.000Z";
    let base = now("2026-12-31T23:40:00Z");
    for age_secs in -400i64..=1900 {
        let n = base + time::Duration::seconds(age_secs);
        let ok = check_fetched_at_age(fetched, n).is_ok();
        let expected = (-300..=1800).contains(&age_secs);
        assert_eq!(
            ok, expected,
            "age {age_secs}s: expected ok={expected}, got ok={ok}"
        );
    }
}

// ---- added in the T4 fix pass (#4726): the variant, not just is_err ----

#[test]
fn r18_stale_is_too_old_not_too_far_ahead() {
    use site_core::projects_list::AgeError;
    assert!(matches!(
        check_fetched_at_age("2026-11-30T23:59:00.000Z", now("2026-12-01T00:29:01Z")),
        Err(AgeError::TooOld { .. })
    ));
}

#[test]
fn r18_future_is_too_far_ahead_not_too_old() {
    use site_core::projects_list::AgeError;
    assert!(matches!(
        check_fetched_at_age("2026-12-01T00:04:00.000Z", now("2026-11-30T23:58:59Z")),
        Err(AgeError::TooFarAhead { .. })
    ));
}
