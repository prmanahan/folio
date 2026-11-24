//! The articles source (R-7 to R-12 wired to R-1 to R-5): fetches
//! `https://mnemra.dev/rss.xml` through the outbound client, runs the body
//! through [`parse_feed`], and reports the document and the skipped posts.
//!
//! Every failure is a [`ReasonCode`]; no response byte reaches a log line
//! (R-30). The mappings below are wildcard-free matches, so a rule added to
//! the parser fails to compile here until it has a code. The refresher
//! validates the document again with the articles validator before it
//! persists or swaps it (R-15).

use futures::future::BoxFuture;
use time::OffsetDateTime;

use super::ReasonCode;
use super::articles::{LinkRule, TextRule};
use super::feed::{FeedRule, Field, PostRule, SkippedItem, parse_feed};
use super::outbound::{FetchFailure, Fetched, OutboundClient, Target};
use super::refresher::{FetchReport, ListSource, SkippedPost};

/// Fetches the articles document from the feed.
pub struct FeedArticlesSource {
    client: OutboundClient,
}

impl FeedArticlesSource {
    pub fn new(client: OutboundClient) -> Self {
        Self { client }
    }
}

impl ListSource for FeedArticlesSource {
    fn fetch(&self, now: OffsetDateTime) -> BoxFuture<'_, FetchReport> {
        Box::pin(async move { feed_report(self.client.fetch(Target::Feed).await, now) })
    }
}

/// What a fetch of the feed produced, turned into the refresher's report.
/// Everything after the transport is here, so a test drives it without one.
/// A skipped post's `index` is zero-based (the parser's own base).
pub(crate) fn feed_report(
    fetched: Result<Fetched, FetchFailure>,
    now: OffsetDateTime,
) -> FetchReport {
    let fetched = match fetched {
        Ok(fetched) => fetched,
        Err(FetchFailure { reason, note }) => {
            return FetchReport {
                document: Err(reason),
                skipped: Vec::new(),
                requests: note.into_iter().collect(),
            };
        }
    };
    let requests = vec![fetched.note];
    match parse_feed(&fetched.body, now) {
        Err(failure) => FetchReport {
            document: Err(reason_of_feed(failure.rule)),
            skipped: skipped_posts(failure.skipped),
            requests,
        },
        Ok(outcome) => FetchReport {
            document: Ok(outcome.document),
            skipped: skipped_posts(outcome.skipped),
            requests,
        },
    }
}

fn skipped_posts(skipped: Vec<SkippedItem>) -> Vec<SkippedPost> {
    skipped
        .into_iter()
        .map(|s| SkippedPost {
            index: s.index,
            reason: reason_of_post(s.rule),
        })
        .collect()
}

/// R-30: the code a failed feed logs. A body over the cap is R-4's code.
pub(crate) fn reason_of_feed(rule: FeedRule) -> ReasonCode {
    match rule {
        FeedRule::TooLarge => ReasonCode::BodyOverCap,
        FeedRule::NotUtf8 => ReasonCode::FeedNotUtf8,
        FeedRule::Xml => ReasonCode::FeedXmlParse,
        FeedRule::Prolog => ReasonCode::FeedProlog,
        FeedRule::Doctype => ReasonCode::FeedDoctype,
        FeedRule::ProcessingInstruction => ReasonCode::FeedProcessingInstruction,
        FeedRule::XmlDeclaration => ReasonCode::FeedXmlDeclaration,
        FeedRule::Root => ReasonCode::FeedRoot,
        FeedRule::Channel => ReasonCode::FeedChannel,
        FeedRule::Close => ReasonCode::FeedClose,
        FeedRule::StrayContent => ReasonCode::FeedStrayContent,
        FeedRule::TooDeep => ReasonCode::FeedTooDeep,
        FeedRule::NoItems => ReasonCode::FeedNoItems,
        FeedRule::NoValidPost => ReasonCode::FeedNoValidPost,
        FeedRule::DocumentInvalid => ReasonCode::FeedDocumentInvalid,
    }
}

/// R-30: the code a skipped post logs. Two parser routes to one rule share a
/// code: an empty title or summary is `EmptyField`, a decoded `<` is
/// `AngleBracket`.
pub(crate) fn reason_of_post(rule: PostRule) -> ReasonCode {
    match rule {
        PostRule::UnknownChild => ReasonCode::PostUnknownChild,
        PostRule::SelfClosingChild => ReasonCode::PostSelfClosingChild,
        PostRule::DuplicateField => ReasonCode::PostDuplicateField,
        PostRule::EmptyField => ReasonCode::PostEmptyField,
        PostRule::MissingField(
            Field::Title | Field::Link | Field::Description | Field::PubDate,
        ) => ReasonCode::PostMissingField,
        PostRule::StartTagNotBare => ReasonCode::PostStartTagNotBare,
        PostRule::StrayContent => ReasonCode::PostStrayContent,
        PostRule::ElementInField => ReasonCode::PostElementInField,
        PostRule::CDataInField => ReasonCode::PostCDataInField,
        PostRule::CommentInField => ReasonCode::PostCommentInField,
        PostRule::AngleBracketInField => ReasonCode::PostAngleBracket,
        PostRule::NumericReference => ReasonCode::PostNumericReference,
        PostRule::UnknownEntity => ReasonCode::PostUnknownEntity,
        PostRule::Title(TextRule::Empty) | PostRule::Summary(TextRule::Empty) => {
            ReasonCode::PostEmptyField
        }
        PostRule::Title(TextRule::AngleBracket) | PostRule::Summary(TextRule::AngleBracket) => {
            ReasonCode::PostAngleBracket
        }
        PostRule::Title(TextRule::TooLong) => ReasonCode::PostTitleTooLong,
        PostRule::Title(TextRule::RejectedChar) => ReasonCode::PostTitleRejectedChar,
        PostRule::Summary(TextRule::TooLong) => ReasonCode::PostSummaryTooLong,
        PostRule::Summary(TextRule::RejectedChar) => ReasonCode::PostSummaryRejectedChar,
        PostRule::Link(LinkRule::TooLong) => ReasonCode::PostLinkTooLong,
        PostRule::Link(LinkRule::RejectedChar) => ReasonCode::PostLinkRejectedChar,
        PostRule::Link(LinkRule::NotAUrl) => ReasonCode::PostLinkNotUrl,
        PostRule::Link(LinkRule::Scheme) => ReasonCode::PostLinkScheme,
        PostRule::Link(LinkRule::Host) => ReasonCode::PostLinkHost,
        PostRule::Link(LinkRule::Port) => ReasonCode::PostLinkPort,
        PostRule::Link(LinkRule::Userinfo) => ReasonCode::PostLinkUserinfo,
        PostRule::Link(LinkRule::Query) => ReasonCode::PostLinkQuery,
        PostRule::Link(LinkRule::Fragment) => ReasonCode::PostLinkFragment,
        PostRule::Link(LinkRule::Path) => ReasonCode::PostLinkPath,
        PostRule::Link(LinkRule::NotCanonical) => ReasonCode::PostLinkNotCanonical,
        PostRule::PubDate => ReasonCode::PostPubDate,
        PostRule::PubDateTooLong => ReasonCode::PostPubDateTooLong,
        PostRule::Validator => ReasonCode::PostValidatorRefused,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    use serde_json::json;
    use time::macros::datetime;

    use super::*;
    use crate::live_lists::ListName;
    use crate::live_lists::articles::{LinkRule, TextRule};
    use crate::live_lists::feed::Field;
    use crate::live_lists::feed::tests::{Verdict, negative_fixtures, unreachable_post_rule};
    use crate::live_lists::harness::{Env, LogCapture, Running, launch, settle};
    use crate::live_lists::refresher::{REFRESH_INTERVAL, RequestNote, Sources};
    use crate::live_lists::test_support::{T0, articles_doc};
    use crate::live_lists::tls_harness::{Pki, Reply, TestNet, TlsServer};
    use axum::http::StatusCode;

    const CAPTURE: &[u8] = include_bytes!("fixtures/mnemra-rss-2026-11-24.xml");
    const MNEMRA: &str = "mnemra.dev";

    // ---- R-30: one code per rule ----

    fn all_feed_rules() -> Vec<FeedRule> {
        vec![
            FeedRule::TooLarge,
            FeedRule::NotUtf8,
            FeedRule::Xml,
            FeedRule::Prolog,
            FeedRule::Doctype,
            FeedRule::ProcessingInstruction,
            FeedRule::XmlDeclaration,
            FeedRule::Root,
            FeedRule::Channel,
            FeedRule::Close,
            FeedRule::StrayContent,
            FeedRule::TooDeep,
            FeedRule::NoItems,
            FeedRule::NoValidPost,
            FeedRule::DocumentInvalid,
        ]
    }

    fn all_post_rules() -> Vec<PostRule> {
        let mut rules = vec![
            PostRule::UnknownChild,
            PostRule::SelfClosingChild,
            PostRule::DuplicateField,
            PostRule::EmptyField,
            PostRule::MissingField(Field::Title),
            PostRule::StartTagNotBare,
            PostRule::StrayContent,
            PostRule::ElementInField,
            PostRule::CDataInField,
            PostRule::CommentInField,
            PostRule::AngleBracketInField,
            PostRule::NumericReference,
            PostRule::UnknownEntity,
            PostRule::PubDate,
            PostRule::PubDateTooLong,
            PostRule::Validator,
        ];
        for t in [
            TextRule::Empty,
            TextRule::TooLong,
            TextRule::RejectedChar,
            TextRule::AngleBracket,
        ] {
            rules.push(PostRule::Title(t));
            rules.push(PostRule::Summary(t));
        }
        for l in [
            LinkRule::TooLong,
            LinkRule::RejectedChar,
            LinkRule::NotAUrl,
            LinkRule::Scheme,
            LinkRule::Host,
            LinkRule::Port,
            LinkRule::Userinfo,
            LinkRule::Query,
            LinkRule::Fragment,
            LinkRule::Path,
            LinkRule::NotCanonical,
        ] {
            rules.push(PostRule::Link(l));
        }
        rules
    }

    /// An exhaustive witness: a rule added to the parser fails to compile
    /// here until it has an index, and `the_rule_lists_cover_every_rule`
    /// then fails until the list holds it.
    /// Every feed rule has its own index.
    fn index_of_feed(rule: FeedRule) -> usize {
        match rule {
            FeedRule::TooLarge => 0,
            FeedRule::NotUtf8 => 1,
            FeedRule::Xml => 2,
            FeedRule::Prolog => 3,
            FeedRule::Doctype => 4,
            FeedRule::ProcessingInstruction => 5,
            FeedRule::XmlDeclaration => 6,
            FeedRule::Root => 7,
            FeedRule::Channel => 8,
            FeedRule::Close => 9,
            FeedRule::StrayContent => 10,
            FeedRule::TooDeep => 11,
            FeedRule::NoItems => 12,
            FeedRule::NoValidPost => 13,
            FeedRule::DocumentInvalid => 14,
        }
    }
    const FEED_RULES: usize = 15;

    /// Every post rule has its own index, except that the four `MissingField`
    /// alternatives share one: the cover test needs one index per rule, not per field.
    fn index_of_post(rule: PostRule) -> usize {
        match rule {
            PostRule::UnknownChild => 0,
            PostRule::SelfClosingChild => 1,
            PostRule::DuplicateField => 2,
            PostRule::EmptyField => 3,
            PostRule::MissingField(
                Field::Title | Field::Link | Field::Description | Field::PubDate,
            ) => 4,
            PostRule::StartTagNotBare => 5,
            PostRule::StrayContent => 6,
            PostRule::ElementInField => 7,
            PostRule::CDataInField => 8,
            PostRule::CommentInField => 9,
            PostRule::AngleBracketInField => 10,
            PostRule::NumericReference => 11,
            PostRule::UnknownEntity => 12,
            PostRule::Title(TextRule::Empty) => 13,
            PostRule::Title(TextRule::TooLong) => 14,
            PostRule::Title(TextRule::RejectedChar) => 15,
            PostRule::Title(TextRule::AngleBracket) => 16,
            PostRule::Summary(TextRule::Empty) => 17,
            PostRule::Summary(TextRule::TooLong) => 18,
            PostRule::Summary(TextRule::RejectedChar) => 19,
            PostRule::Summary(TextRule::AngleBracket) => 20,
            PostRule::Link(LinkRule::TooLong) => 21,
            PostRule::Link(LinkRule::RejectedChar) => 22,
            PostRule::Link(LinkRule::NotAUrl) => 23,
            PostRule::Link(LinkRule::Scheme) => 24,
            PostRule::Link(LinkRule::Host) => 25,
            PostRule::Link(LinkRule::Port) => 26,
            PostRule::Link(LinkRule::Userinfo) => 27,
            PostRule::Link(LinkRule::Query) => 28,
            PostRule::Link(LinkRule::Fragment) => 29,
            PostRule::Link(LinkRule::Path) => 30,
            PostRule::Link(LinkRule::NotCanonical) => 31,
            PostRule::PubDate => 32,
            PostRule::PubDateTooLong => 33,
            PostRule::Validator => 34,
        }
    }
    const POST_RULES: usize = 35;

    #[test]
    fn the_rule_lists_cover_every_rule() {
        let feed: Vec<usize> = all_feed_rules().into_iter().map(index_of_feed).collect();
        assert_eq!(
            feed.iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>(),
            (0..FEED_RULES).collect(),
            "all_feed_rules is missing a rule"
        );
        assert_eq!(feed.len(), FEED_RULES, "all_feed_rules repeats a rule");
        let post: Vec<usize> = all_post_rules().into_iter().map(index_of_post).collect();
        assert_eq!(
            post.iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>(),
            (0..POST_RULES).collect(),
            "all_post_rules is missing a rule"
        );
        assert_eq!(post.len(), POST_RULES, "all_post_rules repeats a rule");
    }

    #[test]
    fn the_negative_table_produces_exactly_the_reason_codes_the_mappings_can() {
        // The codes the table's fixtures produce, through the same mappings
        let produced: std::collections::BTreeSet<&str> = negative_fixtures()
            .into_iter()
            .map(|(_, _, verdict)| match verdict {
                Verdict::Feed(rule) => reason_of_feed(rule).as_str(),
                Verdict::Post(rule) => reason_of_post(rule).as_str(),
            })
            .collect();
        // The image of the mappings over every reachable rule
        let mut image: std::collections::BTreeSet<&str> = all_feed_rules()
            .into_iter()
            .filter(|r| *r != FeedRule::DocumentInvalid)
            .map(|r| reason_of_feed(r).as_str())
            .collect();
        image.extend(
            all_post_rules()
                .into_iter()
                .filter(|r| !unreachable_post_rule(*r))
                .map(|r| reason_of_post(r).as_str()),
        );
        let missing: Vec<_> = image.difference(&produced).collect();
        let extra: Vec<_> = produced.difference(&image).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "table lacks codes {missing:?}; table has unreachable codes {extra:?}"
        );
    }

    #[test]
    fn r30_every_feed_rule_has_its_own_reason_code_and_a_body_over_the_cap_reuses_r4s() {
        let codes: Vec<ReasonCode> = all_feed_rules().into_iter().map(reason_of_feed).collect();
        let distinct: HashSet<&str> = codes.iter().map(|c| c.as_str()).collect();
        assert_eq!(distinct.len(), codes.len(), "{codes:?}");
        assert_eq!(reason_of_feed(FeedRule::TooLarge), ReasonCode::BodyOverCap);
        assert_eq!(reason_of_feed(FeedRule::Doctype), ReasonCode::FeedDoctype);
        assert_eq!(reason_of_feed(FeedRule::Xml), ReasonCode::FeedXmlParse);
    }

    #[test]
    fn r30_every_post_rule_has_its_own_reason_code() {
        // the four routes that share another rule's code are asserted below
        let codes: Vec<ReasonCode> = all_post_rules()
            .into_iter()
            .filter(|r| {
                !matches!(
                    r,
                    PostRule::Title(TextRule::Empty | TextRule::AngleBracket)
                        | PostRule::Summary(TextRule::Empty | TextRule::AngleBracket)
                )
            })
            .map(reason_of_post)
            .collect();
        let distinct: HashSet<&str> = codes.iter().map(|c| c.as_str()).collect();
        assert_eq!(distinct.len(), codes.len(), "{codes:?}");
        // the same rule through two parser routes is one code
        assert_eq!(
            reason_of_post(PostRule::Title(TextRule::AngleBracket)),
            reason_of_post(PostRule::AngleBracketInField)
        );
        assert_eq!(
            reason_of_post(PostRule::Summary(TextRule::AngleBracket)),
            ReasonCode::PostAngleBracket
        );
        assert_eq!(
            reason_of_post(PostRule::Title(TextRule::Empty)),
            ReasonCode::PostEmptyField
        );
        for f in [Field::Link, Field::Description, Field::PubDate] {
            assert_eq!(
                reason_of_post(PostRule::MissingField(f)),
                ReasonCode::PostMissingField
            );
        }
    }

    // ---- the report, without a transport ----

    fn ok_fetched(body: &[u8]) -> Result<Fetched, FetchFailure> {
        Ok(Fetched {
            note: RequestNote {
                status: 200,
                ratelimit_remaining: None,
            },
            link: None,
            body: body.to_vec(),
        })
    }

    #[test]
    fn the_capture_reports_its_document_and_the_request_note() {
        let report = feed_report(ok_fetched(CAPTURE), datetime!(2026-11-24 12:00 UTC));
        let doc = report.document.expect("a document");
        let v: serde_json::Value = serde_json::from_str(&doc).expect("json");
        assert_eq!(v["fetched_at"], "2026-11-24T12:00:00.000Z");
        assert_eq!(v["posts"][0]["title"], "Hello, Mnemra");
        assert!(report.skipped.is_empty());
        assert_eq!(report.requests.len(), 1);
        assert_eq!(report.requests[0].status, 200);
    }

    #[test]
    fn a_transport_failure_reports_its_code_and_note_and_no_document() {
        let note = RequestNote {
            status: 503,
            ratelimit_remaining: None,
        };
        let report = feed_report(
            Err(FetchFailure {
                reason: ReasonCode::NonSuccessStatus,
                note: Some(note),
            }),
            datetime!(2026-11-24 12:00 UTC),
        );
        assert_eq!(report.document, Err(ReasonCode::NonSuccessStatus));
        assert_eq!(report.requests, vec![note]);
    }

    #[test]
    fn an_all_bad_feed_reports_the_failure_code_and_one_skip_per_item() {
        let body = "<rss><channel><item></item><item/></channel></rss>";
        let report = feed_report(ok_fetched(body.as_bytes()), datetime!(2026-11-24 12:00 UTC));
        assert_eq!(report.document, Err(ReasonCode::FeedNoValidPost));
        let skips: Vec<(usize, ReasonCode)> =
            report.skipped.iter().map(|s| (s.index, s.reason)).collect();
        assert_eq!(
            skips,
            vec![
                (0, ReasonCode::PostMissingField),
                (1, ReasonCode::PostMissingField)
            ]
        );
    }

    #[test]
    fn a_skipped_post_is_reported_by_zero_based_index_and_reason_code_only() {
        let body = format!(
            "<rss><channel><item></item>{}</channel></rss>",
            "<item><title>T</title><link>https://mnemra.dev/blog/x/</link><description>D</description><pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate></item>"
        );
        let report = feed_report(ok_fetched(body.as_bytes()), datetime!(2026-11-24 12:00 UTC));
        assert!(report.document.is_ok());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].index, 0);
        assert_eq!(report.skipped[0].reason, ReasonCode::PostMissingField);
    }

    // ---- through the real client, on the feed's own host ----

    async fn feed_server(reply: Reply) -> (TlsServer, FeedArticlesSource) {
        let pki = Pki::new();
        let server = TlsServer::start(pki.issue(&[MNEMRA]), move |_, _| reply.clone()).await;
        let client = OutboundClient::for_test(TestNet::new(&pki, &[(MNEMRA, server.addr())]));
        (server, FeedArticlesSource::new(client))
    }

    #[tokio::test]
    async fn the_source_fetches_the_feed_path_and_parses_the_capture() {
        let (server, source) = feed_server(Reply::ok(CAPTURE)).await;
        let report = source.fetch(datetime!(2026-11-24 12:00 UTC)).await;
        assert!(
            report.document.is_ok(),
            "{:?}",
            report.document.as_ref().err()
        );
        assert_eq!(server.requests().len(), 1);
        assert_eq!(server.requests()[0].target, "/rss.xml");
        assert_eq!(report.requests[0].status, 200);
    }

    #[tokio::test]
    async fn a_non_2xx_feed_is_a_failed_fetch_with_the_clients_code() {
        let (_server, source) = feed_server(Reply::status(404)).await;
        let report = source.fetch(datetime!(2026-11-24 12:00 UTC)).await;
        assert_eq!(report.document, Err(ReasonCode::NonSuccessStatus));
        assert_eq!(report.requests.len(), 1);
        assert_eq!(report.requests[0].status, 404);
    }

    // ---- R-13: every negative fixture, through a live refresher ----

    /// An articles source whose body the test swaps between refreshes. It is
    /// the production path minus the socket: `feed_report` is what
    /// `FeedArticlesSource` calls after the client returns, and a paused
    /// tokio clock is not deterministic against the TLS server (T9).
    struct ScriptedFeed {
        body: Mutex<Vec<u8>>,
    }

    impl ListSource for ScriptedFeed {
        fn fetch(&self, now: OffsetDateTime) -> BoxFuture<'_, FetchReport> {
            let body = self.body.lock().expect("body lock").clone();
            Box::pin(async move { feed_report(ok_fetched(&body), now) })
        }
    }

    async fn freshness_articles(running: &Running, env: &Env) -> serde_json::Value {
        running
            .get_json(env, "/api/lists/freshness", StatusCode::OK)
            .await["articles"]
            .clone()
    }

    #[tokio::test(start_paused = true)]
    async fn r13_every_negative_fixture_through_a_live_refresher_then_a_valid_refresh() {
        let fixtures = negative_fixtures();
        for (name, body, verdict) in fixtures {
            // Given a held articles copy and a refresher on a feed serving the
            // negative fixture
            let env = Env::new();
            env.seed(ListName::Articles, &articles_doc(T0));
            env.clock.advance(time::Duration::minutes(10));
            let feed = Arc::new(ScriptedFeed {
                body: Mutex::new(body),
            });
            let (log, _guard) = LogCapture::install();
            let sources = Sources {
                projects: None,
                articles: Some(feed.clone() as Arc<dyn ListSource>),
            };
            let running = launch(env.inputs(None, sources)).await;
            settle().await;
            // Then the server still serves
            assert!(running.is_serving(), "{name}");
            match verdict {
                Verdict::Feed(rule) => {
                    // the held copy and its time are untouched, and the warn
                    // line names the rule's code
                    assert_eq!(
                        freshness_articles(&running, &env).await,
                        json!(T0),
                        "{name}"
                    );
                    assert_eq!(
                        env.stored(ListName::Articles),
                        Some(articles_doc(T0)),
                        "{name}"
                    );
                    if rule == FeedRule::NoValidPost {
                        // R-12, R-30: one `post skipped` line per item, though
                        // the feed failed
                        assert_eq!(log.lines_containing("post skipped").len(), 2, "{name}");
                    }
                    let line = log.one_line("live list refresh failed");
                    assert!(
                        line.contains(&format!(r#"reason="{}""#, reason_of_feed(rule).as_str())),
                        "{name}: {line}"
                    );
                }
                Verdict::Post(rule) => {
                    // the other post advanced the list, and the skip is
                    // logged by index and code only
                    assert_ne!(
                        freshness_articles(&running, &env).await,
                        json!(T0),
                        "{name}"
                    );
                    let line = log.one_line("post skipped");
                    assert!(line.contains("index=0"), "{name}: {line}");
                    assert!(
                        line.contains(&format!(r#"reason="{}""#, reason_of_post(rule).as_str())),
                        "{name}: {line}"
                    );
                }
            }
            // When the feed turns valid and the interval passes
            *feed.body.lock().expect("body lock") = CAPTURE.to_vec();
            env.clock.advance(time::Duration::hours(6));
            tokio::time::advance(REFRESH_INTERVAL).await;
            settle().await;
            // Then the refresher completes a valid refresh: the copy now
            // holds the capture's post, at the new time
            let stored: serde_json::Value =
                serde_json::from_str(&env.stored(ListName::Articles).expect("a stored row"))
                    .expect("json");
            assert_eq!(stored["posts"][0]["title"], "Hello, Mnemra", "{name}");
            assert_eq!(stored["posts"].as_array().map(Vec::len), Some(1), "{name}");
            assert_eq!(
                freshness_articles(&running, &env).await,
                json!(stored["fetched_at"]),
                "{name}"
            );
            assert_ne!(stored["fetched_at"], json!(T0), "{name}");
        }
    }
}
