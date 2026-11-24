//! R-30's sentinel suite: no byte of a response body reaches a log line.
//!
//! Each case serves one sentinel inside a response, through the production
//! sources and the outbound client over a local TLS server, started through
//! the start function with the log captured. The sentinels are per-run
//! generated tokens, so a match is the response text and never a coincidence.
//!
//! The spawned binary's stderr is not read here: the binary has no source
//! override (R-2, R-5), so a spawned `folio serve` can only reach the real
//! hosts and no sentinel can be planted in what it fetches. The capture below
//! installs the same `tracing` events the binary writes, minus main.rs's
//! subscriber and, under `cfg(test)`, the payload-free panic hook.
//!
//! Test-only; the module is declared under `#[cfg(test)]`.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use super::feed_source::FeedArticlesSource;
use super::github::GithubProjectsSource;
use super::harness::{Env, LogCapture, launch};
use super::outbound::OutboundClient;
use super::refresher::Sources;
use super::tls_harness::{Pki, Reply, TestNet, TlsServer};

const GITHUB: &str = "api.github.com";
const MNEMRA: &str = "mnemra.dev";
const ORG: &str = "/orgs/mnemra/repos?type=public&per_page=100";
const USER: &str = "/users/prmanahan/repos?type=owner&per_page=100";
const FEED: &str = "/rss.xml";
const CAPTURE: &str = include_str!("fixtures/mnemra-rss-2026-11-24.xml");

/// The seven sentinels R-30 lists, in its order. Generated per run.
struct Sentinels([String; 7]);

impl Sentinels {
    fn new() -> Self {
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let roles = [
            "endtag",
            "postdesc",
            "jsontype",
            "ghdesc",
            "channel",
            "attr",
            "ratelimit",
        ];
        Self(roles.map(|role| format!("SENTINEL{role}x{nonce}")))
    }

    fn endtag(&self) -> &str {
        &self.0[0]
    }
    fn post_description(&self) -> &str {
        &self.0[1]
    }
    fn json_type(&self) -> &str {
        &self.0[2]
    }
    fn github_description(&self) -> &str {
        &self.0[3]
    }
    fn channel_child(&self) -> &str {
        &self.0[4]
    }
    fn attribute(&self) -> &str {
        &self.0[5]
    }
    fn ratelimit(&self) -> &str {
        &self.0[6]
    }

    /// The sentinels found anywhere in the captured output.
    fn found_in(&self, log: &LogCapture) -> Vec<&str> {
        let text = log.text();
        self.0
            .iter()
            .filter(|s| text.contains(s.as_str()))
            .map(String::as_str)
            .collect()
    }

    /// Fails when any of the seven is anywhere in the captured output.
    fn assert_absent_from(&self, log: &LogCapture) {
        assert_eq!(
            self.found_in(log),
            Vec::<&str>::new(),
            "a sentinel reached the log:\n{}",
            log.text()
        );
    }
}

fn repo(owner: &str, name: &str, description: Value) -> Value {
    json!({
        "name": name, "private": false, "fork": false, "archived": false,
        "description": description, "owner": {"login": owner},
    })
}

fn org_reply(repos: &Value) -> Reply {
    Reply::ok(serde_json::to_vec(repos).expect("json"))
}

fn user_reply() -> Reply {
    Reply::ok(serde_json::to_vec(&json!([repo("prmanahan", "beta", json!(null))])).expect("json"))
}

/// What the two hosts answer. The defaults are valid, so a case changes one
/// thing and the other list still succeeds.
struct World {
    org: Reply,
    user: Reply,
    feed: Vec<u8>,
}

impl World {
    fn valid() -> Self {
        Self {
            org: org_reply(&json!([repo("mnemra", "alpha", json!("a"))])),
            user: user_reply(),
            feed: CAPTURE.as_bytes().to_vec(),
        }
    }
}

/// One server presents a certificate for both hosts and answers by target,
/// so the production sources run unchanged against it.
struct Served {
    _env: Env,
    log: LogCapture,
    server: TlsServer,
    _guard: tracing::subscriber::DefaultGuard,
    _running: super::harness::Running,
}

async fn start_against(world: World) -> Served {
    let pki = Pki::new();
    let World { org, user, feed } = world;
    let server = TlsServer::start(pki.issue(&[GITHUB, MNEMRA]), move |req, _| {
        match req.target.as_str() {
            ORG => org.clone(),
            USER => user.clone(),
            FEED => Reply::ok(feed.clone()),
            _ => Reply::status(404),
        }
    })
    .await;
    let client = OutboundClient::for_test(TestNet::new(
        &pki,
        &[(GITHUB, server.addr()), (MNEMRA, server.addr())],
    ));
    let sources = Sources {
        projects: Some(Arc::new(GithubProjectsSource::new(client.clone()))),
        articles: Some(Arc::new(FeedArticlesSource::new(client))),
    };
    let env = Env::new();
    let (log, guard) = LogCapture::install();
    let running = launch(env.inputs(None, sources)).await;
    for _ in 0..400 {
        if !log.lines_containing("live lists refresh").is_empty() {
            return Served {
                _env: env,
                log,
                server,
                _guard: guard,
                _running: running,
            };
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no refresh summary within 4 s:\n{}", log.text());
}

impl Served {
    /// The sentinel's carrier was fetched: the case reached the code that
    /// holds it, so its absence from the log is not an absence of the input.
    fn assert_fetched(&self, target: &str) {
        assert!(
            self.server.requests().iter().any(|r| r.target == target),
            "{target} was never requested; saw {:?}",
            self.server
                .requests()
                .iter()
                .map(|r| r.target.clone())
                .collect::<Vec<_>>()
        );
    }

    fn summary(&self) -> String {
        self.log.one_line("live lists refresh")
    }
}

/// The instrument sees a sentinel when one is logged: without this, an empty
/// capture and an empty population are the same result.
#[tokio::test]
async fn r30_control_the_capture_finds_a_sentinel_that_is_logged() {
    let sentinels = Sentinels::new();
    let (log, _guard) = LogCapture::install();
    tracing::warn!(reason = sentinels.endtag(), "a deliberate leak");
    assert_eq!(sentinels.found_in(&log), vec![sentinels.endtag()]);
}

#[tokio::test]
async fn r30_a_sentinel_in_a_mismatched_end_tag_name_stays_out_of_the_log() {
    let s = Sentinels::new();
    let mut world = World::valid();
    world.feed = CAPTURE
        .replacen(
            "Hello, Mnemra</title>",
            &format!("Hello, Mnemra</{}>", s.endtag()),
            1,
        )
        .into_bytes();
    let served = start_against(world).await;
    served.assert_fetched(FEED);
    served.log.one_line("live list refresh failed");
    s.assert_absent_from(&served.log);
}

#[tokio::test]
async fn r30_a_sentinel_in_a_rejected_posts_description_stays_out_of_the_log() {
    let s = Sentinels::new();
    let mut world = World::valid();
    // U+202E is in the rejected class, so the post is skipped; the capture's
    // post is valid, so the list advances and the skip is logged by index.
    let bad = format!(
        "<item><title>Bad</title><link>https://mnemra.dev/blog/bad/</link><description>{}\u{202e}</description><pubDate>Tue, 28 Apr 2026 00:00:00 GMT</pubDate></item>",
        s.post_description()
    );
    world.feed = CAPTURE
        .replacen("<item>", &format!("{bad}<item>"), 1)
        .into_bytes();
    let served = start_against(world).await;
    served.assert_fetched(FEED);
    let skip = served.log.one_line("post skipped");
    assert!(skip.contains("index=0"), "{skip}");
    s.assert_absent_from(&served.log);
    // the skipped post is in the summary by index and code, and the other
    // post was kept
    assert!(
        served
            .summary()
            .contains("outcome=advanced count=1 skipped=0:"),
        "{}",
        served.summary()
    );
}

#[tokio::test]
async fn r30_a_sentinel_in_a_json_string_where_the_type_is_not_a_string_stays_out_of_the_log() {
    let s = Sentinels::new();
    let mut world = World::valid();
    let mut element = repo("mnemra", "alpha", json!("a"));
    element["archived"] = json!(s.json_type());
    world.org = org_reply(&json!([element]));
    let served = start_against(world).await;
    served.assert_fetched(ORG);
    let line = served.log.one_line("live list refresh failed");
    assert!(line.contains(r#"list="projects""#), "{line}");
    s.assert_absent_from(&served.log);
}

#[tokio::test]
async fn r30_a_sentinel_in_a_github_description_that_fails_r4_stays_out_of_the_log() {
    let s = Sentinels::new();
    let mut world = World::valid();
    let description = format!("{}\u{202e}", s.github_description());
    world.org = org_reply(&json!([repo("mnemra", "alpha", json!(description))]));
    let served = start_against(world).await;
    served.assert_fetched(ORG);
    let line = served.log.one_line("live list refresh failed");
    assert!(line.contains(r#"list="projects""#), "{line}");
    s.assert_absent_from(&served.log);
}

#[tokio::test]
async fn r30_a_sentinel_in_a_discarded_channel_child_stays_out_of_the_log() {
    let s = Sentinels::new();
    let mut world = World::valid();
    world.feed = CAPTURE
        .replacen(
            "<item>",
            &format!("<generator>{}</generator><item>", s.channel_child()),
            1,
        )
        .into_bytes();
    let served = start_against(world).await;
    served.assert_fetched(FEED);
    s.assert_absent_from(&served.log);
}

#[tokio::test]
async fn r30_a_sentinel_in_an_attribute_value_stays_out_of_the_log() {
    let s = Sentinels::new();
    let mut world = World::valid();
    world.feed = CAPTURE
        .replacen(
            r#"isPermaLink="true""#,
            &format!(r#"isPermaLink="{}""#, s.attribute()),
            1,
        )
        .into_bytes();
    let served = start_against(world).await;
    served.assert_fetched(FEED);
    s.assert_absent_from(&served.log);
}

#[tokio::test]
async fn r30_a_sentinel_as_a_non_integer_ratelimit_remaining_stays_out_of_the_log() {
    let s = Sentinels::new();
    let mut world = World::valid();
    world.org = world
        .org
        .with_header("x-ratelimit-remaining", s.ratelimit());
    let served = start_against(world).await;
    served.assert_fetched(ORG);
    s.assert_absent_from(&served.log);
    // and the field is absent from the summary, not rendered as text
    assert!(
        !served.summary().contains("ratelimit_remaining"),
        "{}",
        served.summary()
    );
}

#[tokio::test]
async fn r30_the_summary_line_carries_each_listed_field() {
    // Given both lists succeeding and one GitHub response with an integer
    // rate-limit value
    let mut world = World::valid();
    world.org = world.org.with_header("x-ratelimit-remaining", "58");
    let served = start_against(world).await;
    let line = served.summary();
    // Then the line is info (every enabled list succeeded)
    assert!(line.contains("INFO"), "{line}");
    // and carries the trigger,
    assert!(line.contains("trigger=start"), "{line}");
    // each list's outcome, count, skipped posts and elapsed time,
    assert!(
        line.contains("outcome=advanced count=2 skipped=none elapsed_ms="),
        "projects: {line}"
    );
    assert!(
        line.contains("outcome=advanced count=1 skipped=none elapsed_ms="),
        "articles: {line}"
    );
    // each request's HTTP status,
    assert!(
        line.contains("projects:200,projects:200,articles:200"),
        "{line}"
    );
    // and a GitHub response's integer rate-limit value
    assert!(line.contains("projects:58"), "{line}");
    // the line is not rate-limited: a second refresh would log a second line
    // (R-30), which the refresher's own tests hold; here, the started server
    // logged exactly one for its start refresh
    assert_eq!(served.log.lines_containing("live lists refresh").len(), 1);
}
