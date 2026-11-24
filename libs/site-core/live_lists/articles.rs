//! The articles document (spec Data Model) and its validator.
//!
//! One validator serves the refresher (a fetched document, before it is
//! persisted) and the start-up load (a stored row, R-16), so a document the
//! store holds is one the validator accepts. Fields are private: the only
//! constructor is [`parse`].
//!
//! Errors name the rule and the item index, never the offending text: a post
//! rejected here came from a response body, and R-30 keeps response bytes out
//! of logs.

use serde::Deserialize;
use time::OffsetDateTime;
use url::Url;

use crate::projects_list::{FetchedAtError, is_rejected, parse_fetched_at};

pub const MAX_POSTS: usize = 20;
pub const MAX_TITLE_CODE_POINTS: usize = 200;
pub const MAX_SUMMARY_CODE_POINTS: usize = 600;
pub const MAX_LINK_BYTES: usize = 300;

/// An articles document that passed every Data Model rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticlesList {
    fetched_at: String,
    fetched_at_instant: OffsetDateTime,
    posts: Vec<Post>,
}

/// One post, in feed document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    title: String,
    summary: String,
    link: String,
    published: String,
}

impl ArticlesList {
    /// The `fetched_at` literal, exactly as stored.
    pub fn fetched_at(&self) -> &str {
        &self.fetched_at
    }

    pub fn fetched_at_instant(&self) -> OffsetDateTime {
        self.fetched_at_instant
    }

    /// 1 to [`MAX_POSTS`] posts, in document order.
    pub fn posts(&self) -> &[Post] {
        &self.posts
    }
}

impl Post {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// The parsed link's serialized form, which equals the raw link.
    pub fn link(&self) -> &str {
        &self.link
    }

    /// The `fetched_at` grammar, UTC.
    pub fn published(&self) -> &str {
        &self.published
    }
}

/// Which text rule a title or summary broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRule {
    Empty,
    TooLong,
    RejectedChar,
    /// Decoded text holds `<` (R-10).
    AngleBracket,
}

/// Which link rule a post broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkRule {
    TooLong,
    RejectedChar,
    NotAUrl,
    Scheme,
    Host,
    Port,
    Userinfo,
    Query,
    Fragment,
    Path,
    /// The parsed link's serialized form differs from the raw link.
    NotCanonical,
}

/// Why a document was refused. No variant carries document text.
#[derive(Debug, PartialEq, Eq)]
pub enum ArticlesError {
    /// Not JSON, a wrong type, a missing or unknown field, or a duplicate key.
    Json,
    NotAnObject {
        at: &'static str,
    },
    FetchedAt(FetchedAtError),
    PostCount {
        found: usize,
    },
    Title {
        index: usize,
        rule: TextRule,
    },
    Summary {
        index: usize,
        rule: TextRule,
    },
    Link {
        index: usize,
        rule: LinkRule,
    },
    Published {
        index: usize,
        error: FetchedAtError,
    },
}

impl std::fmt::Display for ArticlesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "articles document refused: {self:?}")
    }
}

impl std::error::Error for ArticlesError {}

/// Validate articles document text against every Data Model rule. Does not
/// apply the ahead-of-now bound; the start-up load does.
///
/// # Errors
/// An [`ArticlesError`] naming the first rule the text breaks.
pub fn parse(text: &str) -> Result<ArticlesList, ArticlesError> {
    // serde's messages can quote the input, so the cause is dropped on
    // purpose: the error names a rule, never text (R-30).
    //
    // Positional arrays satisfy a derived struct `Deserialize`; require
    // objects first (the projects validator does the same).
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| ArticlesError::Json)?;
    require_objects(&value)?;
    // The strict typed parse still runs: it rejects unknown fields and
    // duplicate keys, which `Value` would let through.
    let raw: RawList = serde_json::from_str(text).map_err(|_| ArticlesError::Json)?;
    let fetched_at_instant = parse_fetched_at(&raw.fetched_at).map_err(ArticlesError::FetchedAt)?;
    if !(1..=MAX_POSTS).contains(&raw.posts.len()) {
        return Err(ArticlesError::PostCount {
            found: raw.posts.len(),
        });
    }
    let mut posts = Vec::with_capacity(raw.posts.len());
    for (index, p) in raw.posts.into_iter().enumerate() {
        validate_text(&p.title, MAX_TITLE_CODE_POINTS)
            .map_err(|rule| ArticlesError::Title { index, rule })?;
        validate_text(&p.summary, MAX_SUMMARY_CODE_POINTS)
            .map_err(|rule| ArticlesError::Summary { index, rule })?;
        validate_link(&p.link).map_err(|rule| ArticlesError::Link { index, rule })?;
        parse_fetched_at(&p.published)
            .map_err(|error| ArticlesError::Published { index, error })?;
        posts.push(Post {
            title: p.title,
            summary: p.summary,
            link: p.link,
            published: p.published,
        });
    }
    Ok(ArticlesList {
        fetched_at: raw.fetched_at,
        fetched_at_instant,
        posts,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawList {
    fetched_at: String,
    posts: Vec<RawPost>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPost {
    title: String,
    summary: String,
    link: String,
    published: String,
}

fn require_objects(root: &serde_json::Value) -> Result<(), ArticlesError> {
    let Some(root) = root.as_object() else {
        return Err(ArticlesError::NotAnObject { at: "root" });
    };
    for post in root
        .get("posts")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
    {
        if !post.is_object() {
            return Err(ArticlesError::NotAnObject { at: "post" });
        }
    }
    Ok(())
}

/// R-10 and R-11 on decoded text: length in code points, the rejected class
/// the projects validator uses, and no `<`.
fn validate_text(s: &str, max_code_points: usize) -> Result<(), TextRule> {
    let code_points = s.chars().count();
    if code_points == 0 {
        return Err(TextRule::Empty);
    }
    if code_points > max_code_points {
        return Err(TextRule::TooLong);
    }
    if s.chars().any(is_rejected) {
        return Err(TextRule::RejectedChar);
    }
    if s.contains('<') {
        return Err(TextRule::AngleBracket);
    }
    Ok(())
}

/// R-11's link rules. The parsed link's serialized form must equal the raw
/// link, so what is stored is exactly what was checked.
fn validate_link(raw: &str) -> Result<(), LinkRule> {
    if raw.len() > MAX_LINK_BYTES {
        return Err(LinkRule::TooLong);
    }
    if raw.chars().any(is_rejected) {
        return Err(LinkRule::RejectedChar);
    }
    let url = Url::parse(raw).map_err(|_| LinkRule::NotAUrl)?;
    if url.scheme() != "https" {
        return Err(LinkRule::Scheme);
    }
    if url.host_str() != Some("mnemra.dev") {
        return Err(LinkRule::Host);
    }
    if url.port().is_some() {
        return Err(LinkRule::Port);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(LinkRule::Userinfo);
    }
    if url.query().is_some() {
        return Err(LinkRule::Query);
    }
    if url.fragment().is_some() {
        return Err(LinkRule::Fragment);
    }
    match url.path().strip_prefix("/blog/") {
        Some(rest) if !rest.is_empty() => {}
        _ => return Err(LinkRule::Path),
    }
    if url.as_str() != raw {
        return Err(LinkRule::NotCanonical);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const FETCHED_AT: &str = "2026-11-19T12:00:00.000Z";
    const LINK: &str = "https://mnemra.dev/blog/post-one";

    fn post() -> Value {
        json!({
            "title": "Post one",
            "summary": "A summary of post one.",
            "link": LINK,
            "published": "2026-11-18T08:30:00.000Z",
        })
    }

    fn doc_of(posts: Vec<Value>) -> Value {
        json!({ "fetched_at": FETCHED_AT, "posts": posts })
    }

    fn parse_value(v: &Value) -> Result<ArticlesList, ArticlesError> {
        parse(&v.to_string())
    }

    /// A document with one post whose `field` is replaced by `value`.
    fn with_field(field: &str, value: Value) -> Value {
        let mut p = post();
        p[field] = value;
        doc_of(vec![p])
    }

    fn title_len(n: usize) -> Value {
        Value::String("t".repeat(n))
    }

    // ---- positive control ----

    #[test]
    fn a_valid_document_parses_and_keeps_every_value_in_order() {
        // Given three valid posts
        let posts: Vec<Value> = (0..3)
            .map(|i| {
                json!({
                    "title": format!("Title {i}"),
                    "summary": format!("Summary {i}"),
                    "link": format!("https://mnemra.dev/blog/p-{i}"),
                    "published": format!("2026-11-1{i}T08:30:00.000Z"),
                })
            })
            .collect();
        // When parsed
        let list = parse_value(&doc_of(posts)).expect("valid document");
        // Then fetched_at and each field survive, in document order
        assert_eq!(list.fetched_at(), FETCHED_AT);
        let got: Vec<(&str, &str, &str, &str)> = list
            .posts()
            .iter()
            .map(|p| (p.title(), p.summary(), p.link(), p.published()))
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    "Title 0",
                    "Summary 0",
                    "https://mnemra.dev/blog/p-0",
                    "2026-11-10T08:30:00.000Z"
                ),
                (
                    "Title 1",
                    "Summary 1",
                    "https://mnemra.dev/blog/p-1",
                    "2026-11-11T08:30:00.000Z"
                ),
                (
                    "Title 2",
                    "Summary 2",
                    "https://mnemra.dev/blog/p-2",
                    "2026-11-12T08:30:00.000Z"
                ),
            ]
        );
    }

    // ---- shape ----

    #[test]
    fn text_that_is_not_json_is_refused() {
        assert_eq!(parse("not json"), Err(ArticlesError::Json));
    }

    #[test]
    fn a_root_that_is_not_an_object_is_refused() {
        assert_eq!(parse("[]"), Err(ArticlesError::NotAnObject { at: "root" }));
    }

    #[test]
    fn a_post_that_is_not_an_object_is_refused() {
        // positional arrays satisfy a derived struct Deserialize; objects are required
        let text = format!(
            r#"{{"fetched_at":"{FETCHED_AT}","posts":[["Post one","s","{LINK}","2026-11-18T08:30:00.000Z"]]}}"#
        );
        assert_eq!(parse(&text), Err(ArticlesError::NotAnObject { at: "post" }));
    }

    #[test]
    fn an_unknown_field_at_either_level_is_refused() {
        let mut root = doc_of(vec![post()]);
        root["extra"] = json!(1);
        assert_eq!(parse_value(&root), Err(ArticlesError::Json));

        let mut p = post();
        p["guid"] = json!("x");
        assert_eq!(parse_value(&doc_of(vec![p])), Err(ArticlesError::Json));
    }

    #[test]
    fn a_missing_field_is_refused() {
        for field in ["title", "summary", "link", "published"] {
            let mut p = post();
            p.as_object_mut().unwrap().remove(field);
            assert_eq!(
                parse_value(&doc_of(vec![p])),
                Err(ArticlesError::Json),
                "{field}"
            );
        }
        let no_posts = json!({ "fetched_at": FETCHED_AT });
        assert_eq!(parse_value(&no_posts), Err(ArticlesError::Json));
    }

    #[test]
    fn a_duplicate_key_is_refused() {
        let text =
            format!(r#"{{"fetched_at":"{FETCHED_AT}","fetched_at":"{FETCHED_AT}","posts":[]}}"#);
        assert_eq!(parse(&text), Err(ArticlesError::Json));
    }

    #[test]
    fn a_wrong_type_is_refused() {
        assert_eq!(
            parse_value(&with_field("title", json!(7))),
            Err(ArticlesError::Json)
        );
    }

    // ---- fetched_at ----

    #[test]
    fn fetched_at_must_be_the_exact_grammar() {
        for bad in [
            "2026-11-19T12:00:00Z",
            "2026-11-19T12:00:00.000+00:00",
            "2026-11-19t12:00:00.000Z",
            "2026-13-19T12:00:00.000Z",
            "",
        ] {
            let mut d = doc_of(vec![post()]);
            d["fetched_at"] = json!(bad);
            assert!(
                matches!(parse_value(&d), Err(ArticlesError::FetchedAt(_))),
                "{bad:?}"
            );
        }
    }

    // ---- post count: 1 to 20 ----

    #[test]
    fn zero_posts_is_refused() {
        assert_eq!(
            parse_value(&doc_of(vec![])),
            Err(ArticlesError::PostCount { found: 0 })
        );
    }

    #[test]
    fn twenty_posts_are_accepted_and_twenty_one_refused() {
        let twenty = doc_of((0..20).map(|_| post()).collect());
        assert_eq!(parse_value(&twenty).unwrap().posts().len(), 20);
        let twenty_one = doc_of((0..21).map(|_| post()).collect());
        assert_eq!(
            parse_value(&twenty_one),
            Err(ArticlesError::PostCount { found: 21 })
        );
    }

    // ---- title and summary (R-10, R-11) ----

    #[test]
    fn title_is_one_to_two_hundred_code_points() {
        assert_eq!(
            parse_value(&with_field("title", title_len(0))),
            Err(ArticlesError::Title {
                index: 0,
                rule: TextRule::Empty
            })
        );
        assert!(parse_value(&with_field("title", title_len(1))).is_ok());
        assert!(parse_value(&with_field("title", title_len(200))).is_ok());
        assert_eq!(
            parse_value(&with_field("title", title_len(201))),
            Err(ArticlesError::Title {
                index: 0,
                rule: TextRule::TooLong
            })
        );
    }

    #[test]
    fn title_length_counts_code_points_not_bytes() {
        // 200 two-byte code points are 400 bytes and still valid
        let t = Value::String("é".repeat(200));
        assert!(parse_value(&with_field("title", t)).is_ok());
    }

    #[test]
    fn summary_is_one_to_six_hundred_code_points() {
        assert_eq!(
            parse_value(&with_field("summary", title_len(0))),
            Err(ArticlesError::Summary {
                index: 0,
                rule: TextRule::Empty
            })
        );
        assert!(parse_value(&with_field("summary", title_len(600))).is_ok());
        assert_eq!(
            parse_value(&with_field("summary", title_len(601))),
            Err(ArticlesError::Summary {
                index: 0,
                rule: TextRule::TooLong
            })
        );
    }

    #[test]
    fn a_rejected_class_code_point_in_title_or_summary_is_refused() {
        // C0, DEL, C1, U+2028, U+2029 and bidi/invisible (U+202E, U+200B)
        for cp in [
            0x00u32, 0x0a, 0x1f, 0x7f, 0x85, 0x2028, 0x2029, 0x202e, 0x200b,
        ] {
            let c = char::from_u32(cp).unwrap();
            let text = Value::String(format!("a{c}b"));
            assert_eq!(
                parse_value(&with_field("title", text.clone())),
                Err(ArticlesError::Title {
                    index: 0,
                    rule: TextRule::RejectedChar
                }),
                "title U+{cp:04X}"
            );
            assert_eq!(
                parse_value(&with_field("summary", text)),
                Err(ArticlesError::Summary {
                    index: 0,
                    rule: TextRule::RejectedChar
                }),
                "summary U+{cp:04X}"
            );
        }
    }

    #[test]
    fn decoded_text_holding_an_angle_bracket_is_refused_but_gt_and_amp_are_allowed() {
        assert_eq!(
            parse_value(&with_field("title", json!("a <b> c"))),
            Err(ArticlesError::Title {
                index: 0,
                rule: TextRule::AngleBracket
            })
        );
        assert_eq!(
            parse_value(&with_field("summary", json!("x < y"))),
            Err(ArticlesError::Summary {
                index: 0,
                rule: TextRule::AngleBracket
            })
        );
        // narrowness: `>` and `&` alone are allowed
        assert!(parse_value(&with_field("title", json!("Q&A > FAQ"))).is_ok());
    }

    #[test]
    fn the_error_names_the_index_of_the_failing_post() {
        let mut bad = post();
        bad["title"] = json!("");
        let d = doc_of(vec![post(), post(), bad]);
        assert_eq!(
            parse_value(&d),
            Err(ArticlesError::Title {
                index: 2,
                rule: TextRule::Empty
            })
        );
    }

    // ---- link (R-11) ----

    fn link_error(link: &str) -> Option<LinkRule> {
        match parse_value(&with_field("link", json!(link))) {
            Err(ArticlesError::Link { index: 0, rule }) => Some(rule),
            Ok(_) => None,
            Err(other) => panic!("{link:?}: unexpected {other:?}"),
        }
    }

    #[test]
    fn a_blog_link_is_accepted() {
        assert_eq!(link_error("https://mnemra.dev/blog/post-one"), None);
        assert_eq!(link_error("https://mnemra.dev/blog/a/b-c_d.e"), None);
    }

    #[test]
    fn link_is_at_most_three_hundred_bytes() {
        let prefix = "https://mnemra.dev/blog/";
        let at_cap = format!("{prefix}{}", "a".repeat(MAX_LINK_BYTES - prefix.len()));
        assert_eq!(at_cap.len(), 300);
        assert_eq!(link_error(&at_cap), None);
        assert_eq!(link_error(&format!("{at_cap}a")), Some(LinkRule::TooLong));
    }

    #[test]
    fn a_rejected_class_code_point_in_the_raw_link_is_refused() {
        for cp in [0x00u32, 0x0a, 0x7f, 0x2028, 0x202e] {
            let c = char::from_u32(cp).unwrap();
            assert_eq!(
                link_error(&format!("https://mnemra.dev/blog/a{c}b")),
                Some(LinkRule::RejectedChar),
                "U+{cp:04X}"
            );
        }
    }

    #[test]
    fn a_link_that_is_not_an_absolute_url_is_refused() {
        for bad in ["", "not a url", "/blog/x", "//mnemra.dev/blog/x"] {
            assert_eq!(link_error(bad), Some(LinkRule::NotAUrl), "{bad:?}");
        }
    }

    #[test]
    fn each_link_rule_has_its_own_refusal() {
        let cases: [(&str, LinkRule); 11] = [
            ("http://mnemra.dev/blog/x", LinkRule::Scheme),
            ("ftp://mnemra.dev/blog/x", LinkRule::Scheme),
            ("https://www.mnemra.dev/blog/x", LinkRule::Host),
            ("https://mnemra.dev.evil.example/blog/x", LinkRule::Host),
            ("https://127.0.0.1/blog/x", LinkRule::Host),
            ("https://mnemra.dev:8443/blog/x", LinkRule::Port),
            ("https://user@mnemra.dev/blog/x", LinkRule::Userinfo),
            ("https://mnemra.dev/blog/x?a=1", LinkRule::Query),
            ("https://mnemra.dev/blog/x#frag", LinkRule::Fragment),
            ("https://mnemra.dev/blog/", LinkRule::Path),
            ("https://mnemra.dev/blogx/y", LinkRule::Path),
        ];
        for (link, rule) in cases {
            assert_eq!(link_error(link), Some(rule), "{link}");
        }
    }

    #[test]
    fn a_path_that_leaves_blog_after_normalization_is_refused() {
        assert_eq!(
            link_error("https://mnemra.dev/blog/../secret"),
            Some(LinkRule::Path)
        );
        assert_eq!(link_error("https://mnemra.dev/blog"), Some(LinkRule::Path));
    }

    #[test]
    fn a_link_whose_serialized_form_differs_from_the_raw_link_is_refused() {
        for raw in [
            "https://MNEMRA.DEV/blog/x",     // host lowercased
            "https://mnemra.dev:443/blog/x", // default port dropped
            "https://mnemra.dev/blog/a b",   // space percent-encoded
            "https://mnemra.dev/blog/./x",   // dot segment removed
        ] {
            assert_eq!(link_error(raw), Some(LinkRule::NotCanonical), "{raw}");
        }
    }

    // ---- published ----

    #[test]
    fn published_must_be_the_exact_grammar() {
        for bad in [
            "Wed, 18 Nov 2026 08:30:00 GMT",
            "2026-11-18T08:30:00Z",
            "2026-11-18T08:30:00.000+01:00",
            "2026-02-31T08:30:00.000Z",
            "",
        ] {
            assert!(
                matches!(
                    parse_value(&with_field("published", json!(bad))),
                    Err(ArticlesError::Published { index: 0, .. })
                ),
                "{bad:?}"
            );
        }
    }

    // ---- property: the validator and a builder of valid posts agree ----

    #[test]
    fn property_any_document_built_from_valid_parts_parses_to_those_parts() {
        let mut state = 0x4A57u64;
        let mut next = move || {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        const WORDS: [&str; 6] = ["alpha", "Beta", "g&mma", "d > e", "ζeta", "日本語"];
        for case in 0..300 {
            let n = 1 + (next() % 20) as usize;
            let mut expected = Vec::new();
            let mut posts = Vec::new();
            for i in 0..n {
                let title: String = (0..1 + next() % 10)
                    .map(|_| WORDS[(next() % 6) as usize])
                    .collect::<Vec<_>>()
                    .join(" ");
                let summary = format!("{title} / {}", WORDS[(next() % 6) as usize]);
                let link = format!("https://mnemra.dev/blog/p-{case}-{i}");
                let published = format!(
                    "2026-{:02}-{:02}T08:30:00.000Z",
                    1 + next() % 12,
                    1 + next() % 28
                );
                expected.push((
                    title.clone(),
                    summary.clone(),
                    link.clone(),
                    published.clone(),
                ));
                posts.push(json!({"title": title, "summary": summary, "link": link, "published": published}));
            }
            let list = parse_value(&doc_of(posts)).unwrap_or_else(|e| panic!("case {case}: {e}"));
            let got: Vec<_> = list
                .posts()
                .iter()
                .map(|p| {
                    (
                        p.title().to_string(),
                        p.summary().to_string(),
                        p.link().to_string(),
                        p.published().to_string(),
                    )
                })
                .collect();
            assert_eq!(got, expected, "case {case}");
        }
    }
}
