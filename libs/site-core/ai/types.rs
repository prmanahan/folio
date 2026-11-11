use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use super::invisible_table::INVISIBLE_RANGES;

/// Remove every code point in the "invisible-unicode strip set": Unicode
/// Default_Ignorable_Code_Point plus U+FFF9..=U+FFFB. They are invisible yet
/// readable by a model, so they can carry hidden instructions (OWASP LLM Top 10
/// 2026, LLM01; #4711). One rule, no exceptions: variation selectors (so emoji
/// lose U+FE0F), ZWJ, the tag block, bidi controls and the rest all go.
///
/// The table is generated, with its frontend twin, by
/// `scripts/gen-invisible.mjs`; CI fails when either is stale.
pub fn strip_invisible_unicode(s: &str) -> String {
    s.chars().filter(|c| !is_invisible(*c)).collect()
}

fn is_invisible(c: char) -> bool {
    let cp = c as u32;
    INVISIBLE_RANGES
        .binary_search_by(|&(lo, hi)| {
            if hi < cp {
                std::cmp::Ordering::Less
            } else if lo > cp {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// Serde hook: strip at the request boundary, so the length checks and prompt
/// construction downstream only ever see the stripped text.
fn deserialize_stripped<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    String::deserialize(d).map(|s| strip_invisible_unicode(&s))
}

#[derive(Deserialize)]
pub struct ChatRequest {
    #[serde(deserialize_with = "deserialize_stripped")]
    pub message: String,
}

#[derive(Deserialize)]
pub struct FitRequest {
    #[serde(deserialize_with = "deserialize_stripped")]
    pub job_description: String,
}

/// The three verdicts the fit prompt (`ai/prompts.rs`) offers and the frontend
/// (`FitVerdict.svelte`) renders. A model value outside this set fails
/// deserialization, which the fit handler maps to the opaque 500 (#4711 FO1).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    StrongFit,
    WorthConversation,
    ProbablyNot,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct FitVerdict {
    pub verdict: Verdict,
    pub headline: String,
    pub opening: String,
    pub gaps: Vec<FitGap>,
    pub transfers: Vec<FitTransfer>,
    pub recommendation: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct FitGap {
    pub requirement: String,
    pub gap_title: String,
    pub explanation: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct FitTransfer {
    pub skill: String,
    pub relevance: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Membership by linear scan: deliberately not the binary search the
    /// implementation uses, so the exhaustive test cross-checks one against
    /// the other.
    fn in_table(cp: u32) -> bool {
        INVISIBLE_RANGES
            .iter()
            .any(|&(lo, hi)| (lo..=hi).contains(&cp))
    }

    #[test]
    fn table_is_sorted_non_overlapping_and_non_empty() {
        assert!(!INVISIBLE_RANGES.is_empty());
        for &(lo, hi) in INVISIBLE_RANGES {
            assert!(lo <= hi, "{lo:X}..{hi:X}");
        }
        for pair in INVISIBLE_RANGES.windows(2) {
            assert!(
                pair[0].1 < pair[1].0,
                "{:X?} overlaps or is out of order",
                pair
            );
        }
    }

    /// Every Unicode scalar value: stripped exactly when it is in the table.
    #[test]
    fn strips_exactly_the_table_over_every_scalar_value() {
        let mut stripped = 0u32;
        for c in (0..=0x10FFFFu32).filter_map(char::from_u32) {
            let got = strip_invisible_unicode(&format!("a{c}b"));
            let want_stripped = in_table(c as u32);
            if want_stripped {
                stripped += 1;
            }
            assert_eq!(
                got == "ab",
                want_stripped,
                "U+{:04X}: stripped={}, in table={}",
                c as u32,
                got == "ab",
                want_stripped
            );
            if !want_stripped {
                assert_eq!(
                    got,
                    format!("a{c}b"),
                    "U+{:04X} must pass through",
                    c as u32
                );
            }
        }
        assert!(
            stripped > 4000,
            "table looks empty or truncated: {stripped}"
        );
    }

    /// Positive control: the loop above would pass vacuously on an empty
    /// table, so pin members the rule must cover (variation selectors, ZWJ,
    /// tag block, bidi, soft hyphen, Hangul fillers, FFF9..FFFB) and
    /// characters it must leave alone.
    #[test]
    fn known_members_are_stripped_and_ordinary_text_is_not() {
        for cp in [
            0x00ADu32, 0x200B, 0x200D, 0x202E, 0x2066, 0x3164, 0xFE0E, 0xFE0F, 0xFEFF, 0xFFA0,
            0xFFF9, 0xFFFB, 0xE0041, 0xE0100,
        ] {
            let c = char::from_u32(cp).unwrap();
            assert_eq!(strip_invisible_unicode(&format!("a{c}b")), "ab", "{cp:X}");
        }
        let text = "caf\u{e9} \u{65E5}\u{672C} \u{1F600} \u{2010}\u{FFFC}";
        assert_eq!(strip_invisible_unicode(text), text);
        // Emoji lose the presentation selector by design (no keep rule).
        assert_eq!(strip_invisible_unicode("\u{2764}\u{FE0F}"), "\u{2764}");
    }

    #[test]
    fn requests_are_stripped_when_deserialized() {
        let body = serde_json::json!({ "message": "hi\u{200B}\u{E0041} there" }).to_string();
        let chat: ChatRequest = serde_json::from_str(&body).unwrap();
        assert_eq!(chat.message, "hi there");
        let body =
            serde_json::json!({ "job_description": "\u{FEFF}Senior\u{2060} Rust" }).to_string();
        let fit: FitRequest = serde_json::from_str(&body).unwrap();
        assert_eq!(fit.job_description, "Senior Rust");
    }

    #[test]
    fn verdict_accepts_exactly_the_three_prompted_values() {
        for (wire, expected) in [
            ("strong_fit", Verdict::StrongFit),
            ("worth_conversation", Verdict::WorthConversation),
            ("probably_not", Verdict::ProbablyNot),
        ] {
            let got: Verdict = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
            assert_eq!(got, expected);
            assert_eq!(serde_json::to_string(&got).unwrap(), format!("\"{wire}\""));
        }
        for bad in ["strong-fit", "Strong_Fit", "", "great_fit", "strong_fit "] {
            assert!(
                serde_json::from_str::<Verdict>(&format!("\"{bad}\"")).is_err(),
                "{bad:?} must be rejected"
            );
        }
    }

    /// Nothing in the crate feeds this schema to the model today (the
    /// `JsonSchema` derive is unused outside this test); the prompt text in
    /// `ai/prompts.rs` is what constrains the model. This pins that the derive
    /// would carry the constraint if a schema consumer is added.
    #[test]
    fn json_schema_lists_the_three_verdict_values() {
        let schema = serde_json::to_value(schemars::schema_for!(FitVerdict)).unwrap();
        let verdict = &schema["definitions"]["Verdict"];
        let mut values: Vec<&str> = verdict["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("Verdict schema has no enum: {verdict}"))
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        values.sort_unstable();
        assert_eq!(values, ["probably_not", "strong_fit", "worth_conversation"]);
    }
}
