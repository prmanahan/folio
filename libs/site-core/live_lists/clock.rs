//! The injected wall clock (R-5) and the `fetched_at` grammar's writer.
//!
//! Every timestamp the live lists write, and the skew comparison at load,
//! reads a [`WallClock`], so a test controls them. Timers are a separate
//! matter: they run on tokio time, which tests pause and advance.

use time::OffsetDateTime;

/// A source of the current instant.
pub trait WallClock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}

/// The production clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl WallClock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// Write `instant` in the `fetched_at` grammar, `YYYY-MM-DDTHH:MM:SS.sssZ`:
/// UTC, millisecond precision, the sub-millisecond part truncated.
///
/// # Errors
/// [`FormatError::YearOutOfRange`] when the UTC year is not in `0..=9999` or the
/// conversion leaves `time`'s range, which
/// the grammar's four-digit year cannot hold; [`FormatError::Format`] when
/// `time` fails to write.
pub fn format_fetched_at(instant: OffsetDateTime) -> Result<String, FormatError> {
    use time::macros::format_description;
    const FETCHED_AT: &[time::format_description::BorrowedFormatItem<'static>] =
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");
    // `to_offset` panics when the UTC date leaves `time`'s range (9999-12-31
    // 23:00 at -01:00 is year 10000), and an instant can come from a feed.
    let utc = instant
        .checked_to_offset(time::UtcOffset::UTC)
        .ok_or(FormatError::YearOutOfRange)?;
    // A negative year would write a leading `-`, and a year past 9999 five
    // digits: neither is the grammar.
    if !(0..=9999).contains(&utc.year()) {
        return Err(FormatError::YearOutOfRange);
    }
    utc.format(FETCHED_AT).map_err(FormatError::Format)
}

#[derive(Debug)]
pub enum FormatError {
    YearOutOfRange,
    Format(time::error::Format),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::YearOutOfRange => write!(f, "year outside 0..=9999"),
            FormatError::Format(e) => write!(f, "cannot write fetched_at: {e}"),
        }
    }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects_list::parse_fetched_at;
    use time::macros::datetime;

    #[test]
    fn format_writes_the_exact_grammar() {
        // Given an instant with a sub-millisecond part
        let t = datetime!(2026-11-19 12:34:56.789_654_321 UTC);
        // Then the string is the grammar, truncated (not rounded) to ms
        assert_eq!(format_fetched_at(t).unwrap(), "2026-11-19T12:34:56.789Z");
    }

    #[test]
    fn format_pads_every_field() {
        let t = datetime!(2026-01-02 03:04:05 UTC);
        assert_eq!(format_fetched_at(t).unwrap(), "2026-01-02T03:04:05.000Z");
    }

    #[test]
    fn format_converts_a_non_utc_offset_to_utc() {
        let t = datetime!(2026-11-19 12:34:56.5 +05:30);
        assert_eq!(format_fetched_at(t).unwrap(), "2026-11-19T07:04:56.500Z");
    }

    #[test]
    fn format_refuses_a_year_the_grammar_cannot_hold() {
        // A negative year would write a leading `-`, which is not the grammar
        let t = datetime!(-0001-01-01 00:00 UTC);
        assert!(matches!(
            format_fetched_at(t),
            Err(FormatError::YearOutOfRange)
        ));
    }

    #[test]
    fn format_refuses_an_instant_whose_utc_year_is_out_of_the_representable_range() {
        // 23:00 at -01:00 on 31 Dec 9999 is year 10000 in UTC, which `time`
        // cannot hold: the conversion must refuse, not panic (R-13, Frame C7)
        let t = datetime!(9999-12-31 23:00 -1);
        assert!(matches!(
            format_fetched_at(t),
            Err(FormatError::YearOutOfRange)
        ));
        // The lower edge is not tested: `time` 0.3.47 debug-asserts inside
        // `checked_to_offset` there. A release build returns None, and RFC 2822
        // years start at 1900, so a feed cannot reach it.
    }

    #[test]
    fn property_what_format_writes_the_validator_reads_back_to_the_millisecond() {
        // SplitMix64 over instants 2000..2100 with nanosecond detail
        let mut state = 0x42u64;
        let mut next = move || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        for case in 0..2000 {
            let secs = 946_684_800 + (next() % 3_155_760_000) as i64;
            let nanos = (next() % 1_000_000_000) as i128;
            let t = OffsetDateTime::from_unix_timestamp_nanos(secs as i128 * 1_000_000_000 + nanos)
                .unwrap();
            let written = format_fetched_at(t).unwrap();
            let read = parse_fetched_at(&written)
                .unwrap_or_else(|e| panic!("case {case}: {written} is not the grammar: {e}"));
            let truncated = t.unix_timestamp_nanos() / 1_000_000 * 1_000_000;
            assert_eq!(read.unix_timestamp_nanos(), truncated, "case {case}");
        }
    }
}
