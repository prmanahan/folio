//! The articles source (R-7 to R-12 wired to R-1 to R-5): fetches
//! `https://mnemra.dev/rss.xml` through the outbound client, runs the body
//! through [`parse_feed`], and reports the document and the skipped posts.
//!
//! Every failure is a [`ReasonCode`]; no response byte reaches a log line
//! (R-30). The mappings below are wildcard-free matches, so a rule added to
//! the parser fails to compile here until it has a code.

use futures::future::BoxFuture;
use time::OffsetDateTime;

use super::ReasonCode;
use super::feed::{FeedRule, PostRule};
use super::outbound::{FetchFailure, Fetched, OutboundClient};
use super::refresher::{FetchReport, ListSource};

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
        let _ = (&self.client, now);
        Box::pin(async { FetchReport::failed(ReasonCode::TransportFailed) })
    }
}

/// What a fetch of the feed produced, turned into the refresher's report.
/// Everything after the transport is here, so a test drives it without one.
pub(crate) fn feed_report(
    _fetched: Result<Fetched, FetchFailure>,
    _now: OffsetDateTime,
) -> FetchReport {
    FetchReport::failed(ReasonCode::TransportFailed)
}

/// R-30: the code a failed feed logs.
pub(crate) fn reason_of_feed(_rule: FeedRule) -> ReasonCode {
    ReasonCode::TransportFailed
}

/// R-30: the code a skipped post logs.
pub(crate) fn reason_of_post(_rule: PostRule) -> ReasonCode {
    ReasonCode::TransportFailed
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
    use crate::live_lists::feed::tests::{Verdict, negative_fixtures};
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
            PostRule::Validator,
        ];
        for t in [TextRule::TooLong, TextRule::RejectedChar] {
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
        let codes: Vec<ReasonCode> = all_post_rules().into_iter().map(reason_of_post).collect();
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
