//! R-18: the `LIVE_LISTS_DISABLED` break-glass switch.
//!
//! The parser takes the raw OS value, so a value that is not valid Unicode is
//! never read as absent. It never fails: every input yields a set, and a
//! malformed one yields both lists disabled plus a [`SwitchProblem`] for the
//! caller to log at `error`. A control that errors denies rather than passes
//! through, so an unparseable switch can only disable more, never less.

use std::ffi::OsStr;

use super::ListName;

/// The set of lists the switch disables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DisabledLists {
    projects: bool,
    articles: bool,
}

impl DisabledLists {
    pub const NONE: DisabledLists = DisabledLists {
        projects: false,
        articles: false,
    };
    pub const BOTH: DisabledLists = DisabledLists {
        projects: true,
        articles: true,
    };

    pub const fn contains(self, list: ListName) -> bool {
        match list {
            ListName::Projects => self.projects,
            ListName::Articles => self.articles,
        }
    }

    fn insert(&mut self, list: ListName) {
        match list {
            ListName::Projects => self.projects = true,
            ListName::Articles => self.articles = true,
        }
    }

    /// Set notation for the start line: `{}`, `{projects}`, `{projects,articles}`.
    pub fn describe(self) -> String {
        let names: Vec<&str> = ListName::ALL
            .into_iter()
            .filter(|l| self.contains(*l))
            .map(ListName::as_str)
            .collect();
        format!("{{{}}}", names.join(","))
    }
}

/// Why the value was not a clean allowlist. Both disable both lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchProblem {
    UnknownToken,
    NotUnicode,
}

impl SwitchProblem {
    pub const fn as_str(self) -> &'static str {
        match self {
            SwitchProblem::UnknownToken => "unknown_token",
            SwitchProblem::NotUnicode => "not_unicode",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedSwitch {
    pub disabled: DisabledLists,
    pub problem: Option<SwitchProblem>,
}

/// Parse the raw value of `LIVE_LISTS_DISABLED` (`None` when the variable is
/// unset).
pub fn parse_switch(raw: Option<&OsStr>) -> ParsedSwitch {
    ParsedSwitch {
        disabled: DisabledLists::NONE,
        problem: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(s: &str) -> ParsedSwitch {
        parse_switch(Some(OsStr::new(s)))
    }

    fn only(list: ListName) -> DisabledLists {
        let mut d = DisabledLists::NONE;
        d.insert(list);
        d
    }

    fn clean(disabled: DisabledLists) -> ParsedSwitch {
        ParsedSwitch {
            disabled,
            problem: None,
        }
    }

    fn both_with(problem: SwitchProblem) -> ParsedSwitch {
        ParsedSwitch {
            disabled: DisabledLists::BOTH,
            problem: Some(problem),
        }
    }

    // ---- R-18 worked scenarios (Scenarios, Break-glass switch) ----

    #[test]
    fn r18_absent_disables_nothing() {
        // Given the variable unset, Then nothing is disabled and nothing is wrong
        assert_eq!(parse_switch(None), clean(DisabledLists::NONE));
    }

    #[test]
    fn r18_empty_and_whitespace_only_disable_nothing() {
        for value in ["", " ", "\t", "  \t ", ",", " , ,", ",,,"] {
            assert_eq!(parse_str(value), clean(DisabledLists::NONE), "{value:?}");
        }
    }

    #[test]
    fn r18_single_tokens_disable_exactly_that_list() {
        assert_eq!(
            parse_str("projects"),
            clean(only(ListName::Projects)),
            "projects"
        );
        assert_eq!(
            parse_str("articles"),
            clean(only(ListName::Articles)),
            "articles"
        );
    }

    #[test]
    fn r18_both_tokens_disable_both_without_a_problem() {
        assert_eq!(parse_str("projects,articles"), clean(DisabledLists::BOTH));
        assert_eq!(parse_str("articles,projects"), clean(DisabledLists::BOTH));
    }

    #[test]
    fn r18_tokens_are_trimmed_and_empty_segments_ignored() {
        // Scenario: LIVE_LISTS_DISABLED=" articles ," disables only articles
        assert_eq!(parse_str(" articles ,"), clean(only(ListName::Articles)));
    }

    #[test]
    fn r18_unknown_or_miscased_tokens_disable_both_and_flag_a_problem() {
        // Scenario: =articles,projcts  =Articles  =projcts
        for value in ["articles,projcts", "Articles", "projcts", "PROJECTS", "all"] {
            assert_eq!(
                parse_str(value),
                both_with(SwitchProblem::UnknownToken),
                "{value:?}"
            );
        }
    }

    #[test]
    fn r18_inner_whitespace_is_not_trimmed_away() {
        // "proj ects" is one unknown token, not "projects"
        assert_eq!(
            parse_str("proj ects"),
            both_with(SwitchProblem::UnknownToken)
        );
    }

    #[cfg(unix)]
    #[test]
    fn r18_a_value_that_is_not_unicode_disables_both_and_is_never_read_as_absent() {
        use std::os::unix::ffi::OsStrExt;
        // Scenario: a value that is not valid Unicode
        let bytes = b"articles\xff";
        assert_eq!(
            parse_switch(Some(OsStr::from_bytes(bytes))),
            both_with(SwitchProblem::NotUnicode)
        );
    }

    #[test]
    fn r18_describe_renders_the_set() {
        assert_eq!(DisabledLists::NONE.describe(), "{}");
        assert_eq!(only(ListName::Articles).describe(), "{articles}");
        assert_eq!(DisabledLists::BOTH.describe(), "{projects,articles}");
    }

    // ---- R-18 properties, over a seeded generator (no proptest in the tree) ----

    /// SplitMix64: deterministic, so a failure reproduces from its seed.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    fn padding(rng: &mut Rng) -> String {
        const WS: [&str; 4] = [" ", "\t", "\u{a0}", "\n"];
        (0..rng.below(4)).map(|_| WS[rng.below(WS.len())]).collect()
    }

    #[test]
    fn r18_property_an_allowlisted_token_set_parses_to_exactly_that_set() {
        let mut rng = Rng(0x18);
        for case in 0..2000 {
            // Given a random sequence of allowlisted tokens, padding and empty
            // segments
            let mut expected = DisabledLists::NONE;
            let mut segments: Vec<String> = Vec::new();
            for _ in 0..rng.below(6) {
                match rng.below(3) {
                    0 => {
                        expected.insert(ListName::Projects);
                        segments.push(format!(
                            "{}projects{}",
                            padding(&mut rng),
                            padding(&mut rng)
                        ));
                    }
                    1 => {
                        expected.insert(ListName::Articles);
                        segments.push(format!(
                            "{}articles{}",
                            padding(&mut rng),
                            padding(&mut rng)
                        ));
                    }
                    _ => segments.push(padding(&mut rng)),
                }
            }
            let value = segments.join(",");
            // Then it parses to exactly that set, with no problem
            assert_eq!(parse_str(&value), clean(expected), "case {case}: {value:?}");
        }
    }

    #[test]
    fn r18_property_any_unknown_token_disables_both_whatever_surrounds_it() {
        let mut rng = Rng(0x1819);
        const ALPHABET: &[u8] = b"abcdeijlmnoprstPAXZ0-_.;:/";
        for case in 0..2000 {
            // Given one token that, trimmed, is neither allowlisted token
            let token = loop {
                let len = 1 + rng.below(10);
                let t: String = (0..len)
                    .map(|_| ALPHABET[rng.below(ALPHABET.len())] as char)
                    .collect();
                if t != "projects" && t != "articles" {
                    break t;
                }
            };
            // placed among valid tokens, padding and empty segments
            let mut segments = vec![token.clone()];
            for _ in 0..rng.below(4) {
                segments.push(["projects", "articles", "", " "][rng.below(4)].to_string());
            }
            // Fisher-Yates shuffle
            for i in (1..segments.len()).rev() {
                segments.swap(i, rng.below(i + 1));
            }
            let value = segments
                .iter()
                .map(|s| format!("{}{s}{}", padding(&mut rng), padding(&mut rng)))
                .collect::<Vec<_>>()
                .join(",");
            // Then both lists are disabled and the problem is flagged
            assert_eq!(
                parse_str(&value),
                both_with(SwitchProblem::UnknownToken),
                "case {case}: {value:?}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn r18_property_any_value_that_is_not_valid_unicode_disables_both() {
        use std::os::unix::ffi::OsStrExt;
        let mut rng = Rng(0x181A);
        for case in 0..2000 {
            // Given random bytes with at least one byte that cannot be UTF-8
            let mut bytes: Vec<u8> = (0..rng.below(12))
                .map(|_| b"projectsarticles, "[rng.below(18)])
                .collect();
            let at = rng.below(bytes.len() + 1);
            bytes.insert(at, 0xff);
            // Then both are disabled with the not-unicode problem, never absent
            assert_eq!(
                parse_switch(Some(OsStr::from_bytes(&bytes))),
                both_with(SwitchProblem::NotUnicode),
                "case {case}: {bytes:?}"
            );
        }
    }
}
