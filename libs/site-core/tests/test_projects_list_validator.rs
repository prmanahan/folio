//! Rule-level pins for the list validator (spec Data Model, R-8, R-10, R-18),
//! added in the T4 fix pass (#4726). They call the shared validator
//! directly so each rule is pinned to its own error variant: a mutant that
//! swaps one rule for another, or rejects for a different reason, fails here.
//! Code points of the rejected class are built from numbers, never typed.

use site_core::projects_list::{
    AgeError, FetchedAtError, ListError, check_fetched_at_age, parse, parse_fetched_at,
};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const TS: &str = "2026-11-13T12:00:00.000Z";

fn doc(mnemra: &str, prmanahan: &str) -> String {
    format!(
        r#"{{"fetched_at":"{TS}","groups":[{{"owner":"mnemra","repos":[{mnemra}]}},{{"owner":"prmanahan","repos":[{prmanahan}]}}]}}"#
    )
}

fn repo(name: &str, description: &str) -> String {
    format!(r#"{{"name":"{name}","description":"{description}"}}"#)
}

/// A doc with the given mnemra repos and one valid prmanahan repo.
fn mnemra_of(repos: &[String]) -> String {
    doc(&repos.join(","), &repo("fixture-c", ""))
}

fn now(s: &str) -> OffsetDateTime {
    OffsetDateTime::parse(s, &Rfc3339).unwrap()
}

#[test]
fn valid_doc_parses_with_counts_and_literal() {
    let list = parse(&mnemra_of(&[repo("fixture-a", "x"), repo("fixture-b", "")])).unwrap();
    assert_eq!(list.fetched_at(), TS);
    assert_eq!(
        list.counts().collect::<Vec<_>>(),
        vec![("mnemra", 2), ("prmanahan", 1)]
    );
}

#[test]
fn name_rules_are_ascii_and_bounded() {
    let bad = |name: &str| {
        matches!(
            parse(&mnemra_of(&[repo(name, "")])),
            Err(ListError::RepoName { .. })
        )
    };
    assert!(bad(""));
    assert!(bad("a b"));
    assert!(bad("a/b"));
    assert!(bad(&format!("fixtur{}-a", char::from_u32(0xe9).unwrap())));
    assert!(bad(&"a".repeat(101)));
    assert!(!bad(&"a".repeat(100)));
    assert!(!bad("A.b_c-9"));
    for ok in ["0", "9", "a", "z", "A", "Z", ".", "_", "-"] {
        assert!(!bad(ok), "{ok} must be accepted");
    }
    // the characters just outside each accepted range
    for no in ["/", ":", "@", "[", "`", "{", "\\u0001"] {
        assert!(bad(no), "{no} must be rejected");
    }
}

#[test]
fn order_is_strict_by_ascii_lowercased_name() {
    let order_err = |names: &[&str]| {
        let repos: Vec<String> = names.iter().map(|n| repo(n, "")).collect();
        matches!(parse(&mnemra_of(&repos)), Err(ListError::RepoOrder { .. }))
    };
    assert!(order_err(&["fixture-a", "fixture-a"]));
    assert!(order_err(&["fixture-A", "fixture-a"]));
    assert!(order_err(&["fixture-a", "fixture-A"]));
    assert!(order_err(&["fixture-b", "fixture-a"]));
    assert!(order_err(&["fixture-Zed", "fixture-yak"]));
    assert!(!order_err(&["fixture-a", "fixture-b"]));
    assert!(!order_err(&["fixture-yak", "fixture-Zed"]));
    assert!(!order_err(&["fixture-a-b", "fixture-a_b"]));
    assert!(!order_err(&["fixture-a", "fixture-a1"]));
}

#[test]
fn description_limit_is_350_code_points() {
    let long = |d: &str| {
        matches!(
            parse(&mnemra_of(&[repo("fixture-a", d)])),
            Err(ListError::DescriptionTooLong { .. })
        )
    };
    assert!(!long(&"a".repeat(350)));
    assert!(long(&"a".repeat(351)));
    let e = char::from_u32(0xe9).unwrap().to_string();
    assert!(!long(&e.repeat(350)));
    assert!(long(&e.repeat(351)));
}

/// `Some(code_point)` when the description is refused for a rejected code
/// point, `None` when it parses. The character is JSON-escaped by serde.
fn rejected_cp(cp: u32) -> Option<u32> {
    let c = char::from_u32(cp).unwrap();
    let quoted = serde_json::to_string(&format!("x{c}y")).unwrap();
    let desc = &quoted[1..quoted.len() - 1];
    match parse(&mnemra_of(&[repo("fixture-a", desc)])) {
        Err(ListError::DescriptionRejectedChar { code_point, .. }) => Some(code_point),
        Ok(_) => None,
        Err(other) => panic!("U+{cp:04X}: unexpected error {other}"),
    }
}

#[test]
fn rejected_class_covers_cc_separators_and_the_generated_table() {
    // \p{Cc}: C0, DEL, C1
    for cp in (0x00..=0x1f).chain(0x7f..=0x9f) {
        assert_eq!(rejected_cp(cp), Some(cp), "U+{cp:04X} (Cc)");
    }
    for cp in [0x2028, 0x2029] {
        assert_eq!(rejected_cp(cp), Some(cp), "U+{cp:04X}");
    }
    // each range of the generated table, both ends
    for cp in [
        0xad, 0x34f, 0x61c, 0x115f, 0x1160, 0x17b4, 0x17b5, 0x180b, 0x180f, 0x200b, 0x200f, 0x202a,
        0x202e, 0x2060, 0x206f, 0x3164, 0xfe00, 0xfe0f, 0xfeff, 0xffa0, 0xfff0, 0xfffb, 0x1bca0,
        0x1bca3, 0x1d173, 0x1d17a, 0xe0000, 0xe0fff,
    ] {
        assert_eq!(rejected_cp(cp), Some(cp), "U+{cp:04X} (table)");
    }
}

#[test]
fn rejected_class_leaves_its_neighbours_alone() {
    for cp in [
        0x20, 0x7e, 0xa0, 0xa1, 0xac, 0xae, 0x2027, 0x202f, 0x65e5, 0x1f600, 0xe1000,
    ] {
        assert_eq!(rejected_cp(cp), None, "U+{cp:04X} must be accepted");
    }
}

#[test]
fn positional_arrays_are_not_objects() {
    let ts = TS;
    let root =
        format!(r#"["{ts}",[["mnemra",[["fixture-a","x"]]],["prmanahan",[["fixture-c","c"]]]]]"#);
    assert!(matches!(
        parse(&root),
        Err(ListError::NotAnObject { at: "root" })
    ));
    let groups = format!(
        r#"{{"fetched_at":"{ts}","groups":[["mnemra",[{{"name":"fixture-a","description":"x"}}]],["prmanahan",[]]]}}"#
    );
    assert!(matches!(
        parse(&groups),
        Err(ListError::NotAnObject { at: "group" })
    ));
    let repos = format!(
        r#"{{"fetched_at":"{ts}","groups":[{{"owner":"mnemra","repos":[["fixture-a","x"]]}},{{"owner":"prmanahan","repos":[]}}]}}"#
    );
    assert!(matches!(
        parse(&repos),
        Err(ListError::NotAnObject { at: "repo" })
    ));
}

#[test]
fn duplicate_keys_are_rejected() {
    let dup = format!(r#"{{"fetched_at":"{TS}","fetched_at":"{TS}","groups":[]}}"#);
    assert!(matches!(parse(&dup), Err(ListError::Json(_))));
}

#[test]
fn group_rules_have_their_own_variants() {
    let one = format!(
        r#"{{"fetched_at":"{TS}","groups":[{{"owner":"mnemra","repos":[{}]}}]}}"#,
        repo("fixture-a", "")
    );
    assert!(matches!(
        parse(&one),
        Err(ListError::GroupCount { found: 1 })
    ));
    assert!(matches!(
        parse(&doc("", &repo("fixture-c", ""))),
        Err(ListError::EmptyOwner { owner: "mnemra" })
    ));
    assert!(matches!(
        parse(&doc(&repo("fixture-a", ""), "")),
        Err(ListError::EmptyOwner { owner: "prmanahan" })
    ));
    let swapped =
        doc(&repo("fixture-a", ""), &repo("fixture-c", "")).replacen("mnemra", "prmanahan", 1);
    assert!(matches!(
        parse(&swapped),
        Err(ListError::GroupOwner { index: 0, .. })
    ));
}

#[test]
fn display_escapes_untrusted_text() {
    // an owner found in the file, carrying a JSON-escaped newline
    let bad_owner =
        doc(&repo("fixture-a", ""), &repo("fixture-c", "")).replacen("mnemra", "m\\nx", 1);
    let msg = parse(&bad_owner).unwrap_err().to_string();
    assert!(!msg.contains('\n'), "{msg}");
    assert!(msg.contains("\\n"), "{msg}");
    // the description arms quote the repo name
    let long = parse(&mnemra_of(&[repo("fixture-a", &"a".repeat(351))])).unwrap_err();
    assert!(long.to_string().contains(r#"mnemra/"fixture-a""#), "{long}");
    let ctl = parse(&mnemra_of(&[repo("fixture-a", "x\\u0001y")])).unwrap_err();
    assert!(ctl.to_string().contains(r#"mnemra/"fixture-a""#), "{ctl}");
    assert!(ctl.to_string().contains("U+0001"), "{ctl}");
}

#[test]
fn fetched_at_grammar_is_exact_and_seconds_stop_at_59() {
    assert!(parse_fetched_at(TS).is_ok());
    assert!(parse_fetched_at("2026-11-13T12:00:59.999Z").is_ok());
    for bad in [
        "2026-11-30T23:59:60.000Z",
        "2026-11-13T12:00:60.000Z",
        "2026-11-13T12:00:99.000Z",
        "2026-11-13T12:00:00.000+00:00",
        "2026-11-13t12:00:00.000Z",
        "2026-11-13T12:00:00.000z",
        "2026-11-13T12:00:00Z",
        "2026-11-13T12:00:00.0Z",
        "2026-11-13T12:00:00.0000Z",
        "2026-11-13 12:00:00.000Z",
        "2026/11/13T12:00:00.000Z",
        "2026-11/13T12:00:00.000Z",
        "2026-11-13T12-00:00.000Z",
        "2026-11-13T12:00-00.000Z",
        "2026-11-13T12:00:00,000Z",
        "2026-11-13T12:00:00.00aZ",
        "202a-11-13T12:00:00.000Z",
        "2026-1a-13T12:00:00.000Z",
        "2026-11-1aT12:00:00.000Z",
        "2026-11-13T1a:00:00.000Z",
        "2026-11-13T12:0a:00.000Z",
        "2026-11-13T12:00:a0.000Z",
        "2026-11-13T12:00:0a.000Z",
        "",
    ] {
        assert_eq!(
            parse_fetched_at(bad),
            Err(FetchedAtError::Grammar),
            "{bad:?}"
        );
    }
    for bad in [
        "2026-02-30T12:00:00.000Z",
        "2026-13-01T12:00:00.000Z",
        "2026-11-13T24:00:00.000Z",
    ] {
        assert_eq!(
            parse_fetched_at(bad),
            Err(FetchedAtError::NotAnInstant),
            "{bad:?}"
        );
    }
}

#[test]
fn age_errors_are_directional() {
    let f = "2026-11-13T12:00:00.000Z";
    assert!(matches!(
        check_fetched_at_age(f, now("2026-11-13T12:30:00.001Z")),
        Err(AgeError::TooOld { .. })
    ));
    assert!(matches!(
        check_fetched_at_age(f, now("2026-11-13T11:54:59.999Z")),
        Err(AgeError::TooFarAhead { .. })
    ));
    assert!(matches!(
        check_fetched_at_age("2026-11-13T12:00:00Z", now("2026-11-13T12:00:00Z")),
        Err(AgeError::Invalid(FetchedAtError::Grammar))
    ));
    let age = check_fetched_at_age(f, now("2026-11-13T12:30:00Z")).unwrap();
    assert_eq!(age.0.whole_seconds(), 1800);
    let ahead = check_fetched_at_age(f, now("2026-11-13T11:55:00Z")).unwrap();
    assert_eq!(ahead.0.whole_seconds(), -300);
}

// ---- accessors, error sources and messages (surfaced by the mutation run) ----

#[test]
fn accessors_expose_the_validated_content_in_order() {
    let list = parse(&doc(
        &[repo("fixture-a", "first"), repo("fixture-b", "")].join(","),
        &repo("fixture-c", "c"),
    ))
    .unwrap();
    let groups = list.groups();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].owner(), "mnemra");
    assert_eq!(groups[1].owner(), "prmanahan");
    let repos = groups[0].repos();
    assert_eq!(repos.len(), 2);
    assert_eq!(repos[0].name(), "fixture-a");
    assert_eq!(repos[0].description(), "first");
    assert_eq!(repos[1].name(), "fixture-b");
    assert_eq!(repos[1].description(), "");
    assert_eq!(groups[1].repos()[0].name(), "fixture-c");
    assert_eq!(groups[1].repos()[0].description(), "c");
    assert_eq!(list.fetched_at_instant(), now("2026-11-13T12:00:00Z"));
}

#[test]
fn error_sources_and_messages_carry_the_cause() {
    use std::error::Error;
    let read = site_core::projects_list::load(std::path::Path::new("/nonexistent/projects.json"))
        .unwrap_err();
    assert!(matches!(read, ListError::Read(_)));
    assert!(read.source().is_some());
    assert!(read.to_string().contains("cannot read"), "{read}");

    let json = parse("{not json").unwrap_err();
    assert!(json.source().is_some());
    assert!(json.to_string().contains("Data Model"), "{json}");

    let fetched = parse(
        &doc(&repo("fixture-a", ""), &repo("fixture-c", "")).replacen(
            TS,
            "2026-02-30T12:00:00.000Z",
            1,
        ),
    )
    .unwrap_err();
    assert!(matches!(fetched, ListError::FetchedAt(_)));
    assert!(fetched.source().is_some());
    assert!(fetched.to_string().contains("fetched_at"), "{fetched}");
    assert!(fetched.to_string().contains("calendar"), "{fetched}");

    let grammar = parse_fetched_at("nope").unwrap_err();
    assert!(grammar.to_string().contains("YYYY-MM-DD"), "{grammar}");

    let f = "2026-11-13T12:00:00.000Z";
    let old = check_fetched_at_age(f, now("2026-11-13T12:31:00Z")).unwrap_err();
    assert!(old.to_string().contains("stale"), "{old}");
    assert!(old.to_string().contains(f), "{old}");
    assert!(old.to_string().contains("1860"), "{old}");
    let ahead = check_fetched_at_age(f, now("2026-11-13T11:54:00Z")).unwrap_err();
    assert!(ahead.to_string().contains("future"), "{ahead}");
    assert!(ahead.to_string().contains(f), "{ahead}");
    assert!(
        ahead
            .to_string()
            .contains("in the future: 360 seconds ahead"),
        "{ahead}"
    );
    let invalid = check_fetched_at_age("nope", now("2026-11-13T12:00:00Z")).unwrap_err();
    assert!(invalid.to_string().contains("fetched_at"), "{invalid}");
}

// ---- log-forging pins: main.rs logs `%e`, so each arm escapes for itself ----

#[test]
fn json_arm_escapes_an_unknown_key_with_a_newline() {
    // the JSON escape backslash-n in the key; the serde message holds a real newline
    let text = format!(r#"{{"fetched_at":"{TS}","a\nb":1,"groups":[]}}"#);
    let e = parse(&text).unwrap_err();
    assert!(matches!(e, ListError::Json(_)), "{e}");
    assert!(!e.to_string().contains('\n'), "{e}");
    assert!(e.to_string().contains("a\\nb"), "{e}");
}

#[test]
fn repo_name_arm_escapes_a_newline_in_the_name() {
    let e = parse(&mnemra_of(&[repo("a\\nb", "")])).unwrap_err();
    assert!(matches!(e, ListError::RepoName { .. }), "{e}");
    assert!(!e.to_string().contains('\n'), "{e}");
}

#[test]
fn nested_duplicate_repos_key_is_rejected() {
    let text = format!(
        r#"{{"fetched_at":"{TS}","groups":[{{"owner":"mnemra","repos":[["fixture-a","x"]],"repos":[{{"name":"fixture-a","description":"x"}}]}},{{"owner":"prmanahan","repos":[{{"name":"fixture-c","description":"c"}}]}}]}}"#
    );
    assert!(matches!(parse(&text), Err(ListError::Json(_))));
}
