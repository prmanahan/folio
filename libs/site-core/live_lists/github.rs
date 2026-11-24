//! The projects source (R-6): fetches the public repos of the two GitHub
//! owners through the outbound client, applies the adopted projects spec's
//! R-4 (every element checked), R-3 (exclusions) and R-8 (order), and returns
//! the document the projects validator accepts.
//!
//! The adopted clauses are read from blob 32f66b7 of
//! `docs/specs/2026-11-11-projects-from-github.md` (plan D21).
//!
//! Every failure is a [`ReasonCode`]; no response byte reaches a log line
//! (R-30).

use futures::future::BoxFuture;
use serde_json::{Value, json};
use time::OffsetDateTime;

use super::ReasonCode;
use super::clock::format_fetched_at;
use super::outbound::{FetchFailure, OutboundClient, Page, Target};
use super::refresher::{FetchReport, ListSource, RequestNote};
use crate::projects_list::{MAX_DESCRIPTION_CODE_POINTS, is_rejected, is_valid_name};

/// One owner: its name in the document and the target of its page N.
struct Owner {
    name: &'static str,
    target: fn(Page) -> Target,
}

/// R-8 of the adopted spec: `mnemra`, then `prmanahan`.
const OWNERS: [Owner; 2] = [
    Owner {
        name: "mnemra",
        target: Target::GithubOrg,
    },
    Owner {
        name: "prmanahan",
        target: Target::GithubUser,
    },
];

/// Fetches the projects document from GitHub.
pub struct GithubProjectsSource {
    client: OutboundClient,
}

impl GithubProjectsSource {
    pub fn new(client: OutboundClient) -> Self {
        Self { client }
    }

    async fn run(&self, now: OffsetDateTime) -> FetchReport {
        let mut requests = Vec::new();
        let document = self.document(now, &mut requests).await;
        FetchReport {
            document,
            skipped: Vec::new(),
            requests,
        }
    }

    /// Each owner in turn: its pages, then R-4's checks, R-3's exclusions and
    /// R-8's order. The first failure ends the fetch, so no later owner is
    /// requested (one request per URL attempted, R-4).
    async fn document(
        &self,
        now: OffsetDateTime,
        requests: &mut Vec<RequestNote>,
    ) -> Result<String, ReasonCode> {
        let mut groups = Vec::with_capacity(OWNERS.len());
        for owner in &OWNERS {
            let elements = self.owner_elements(owner, requests).await?;
            let repos = repos_of(owner.name, &elements)?;
            groups.push(json!({ "owner": owner.name, "repos": repos }));
        }
        // The year cannot be out of range for a real clock; the backstop code
        // is the one for a document that cannot be built.
        let fetched_at =
            format_fetched_at(now).map_err(|_cause| ReasonCode::ProjectsDocumentRejected)?;
        Ok(json!({ "fetched_at": fetched_at, "groups": groups }).to_string())
    }

    /// Every element of every page of one owner (R-1, R-4).
    async fn owner_elements(
        &self,
        owner: &Owner,
        requests: &mut Vec<RequestNote>,
    ) -> Result<Vec<Value>, ReasonCode> {
        let mut elements = Vec::new();
        let mut page = Page::FIRST;
        loop {
            let fetched = match self.client.fetch((owner.target)(page)).await {
                Ok(fetched) => fetched,
                Err(FetchFailure { reason, note }) => {
                    requests.extend(note);
                    return Err(reason);
                }
            };
            requests.push(fetched.note);
            // The parse error is dropped: its text may quote the body (R-30).
            match serde_json::from_slice::<Value>(&fetched.body) {
                Ok(Value::Array(items)) => elements.extend(items),
                Ok(_) | Err(_) => return Err(ReasonCode::ProjectsBodyNotArray),
            }
            let next = fetched
                .link
                .as_deref()
                .map_or(LinkNext::NoNext, has_rel_next);
            if next != LinkNext::Next {
                return Ok(elements);
            }
            // The next page's URL is the fixed one plus `&page=N`, never the
            // URL in the header. At the cap, a further `next` is a failure.
            page = page.next().ok_or(ReasonCode::PageCapReached)?;
        }
    }
}

impl ListSource for GithubProjectsSource {
    fn fetch(&self, now: OffsetDateTime) -> BoxFuture<'_, FetchReport> {
        Box::pin(self.run(now))
    }
}

/// An element that passed R-4.
struct Checked {
    name: String,
    description: String,
    fork: bool,
    archived: bool,
}

/// R-4 of the adopted spec. `None` is a failed element. A key that is
/// missing fails like one of the wrong type, so a response that dropped a
/// field is refused, not read as a default.
fn check_element(owner: &str, element: &Value) -> Option<Checked> {
    let object = element.as_object()?;
    if object.get("private") != Some(&Value::Bool(false)) {
        return None;
    }
    let fork = object.get("fork")?.as_bool()?;
    let archived = object.get("archived")?.as_bool()?;
    let login = object.get("owner")?.as_object()?.get("login")?.as_str()?;
    if !login.eq_ignore_ascii_case(owner) {
        return None;
    }
    let name = object.get("name")?.as_str()?;
    if !is_valid_name(name) {
        return None;
    }
    let description = match object.get("description")? {
        Value::Null => "",
        Value::String(text) => text.as_str(),
        _ => return None,
    };
    // A lone surrogate cannot reach here: serde_json refuses the body.
    if description.chars().count() > MAX_DESCRIPTION_CODE_POINTS
        || description.chars().any(is_rejected)
    {
        return None;
    }
    Some(Checked {
        name: name.to_string(),
        description: description.to_string(),
        fork,
        archived,
    })
}

/// R-4 over every element (a failure fails the run, never skips the
/// element), then R-3's exclusions, then R-8's order.
fn repos_of(owner: &str, elements: &[Value]) -> Result<Vec<Value>, ReasonCode> {
    let checked: Option<Vec<Checked>> = elements
        .iter()
        .map(|element| check_element(owner, element))
        .collect();
    let mut included: Vec<Checked> = checked
        .ok_or(ReasonCode::ProjectsElementRejected)?
        .into_iter()
        .filter(|c| !c.fork && !c.archived && !c.name.eq_ignore_ascii_case(".github"))
        .collect();
    if included.is_empty() {
        return Err(ReasonCode::ProjectsOwnerEmpty);
    }
    // Names are ASCII by now (R-4), so lowercasing and comparing bytes is
    // R-8's "ASCII-lowercased name, compared by code unit".
    included.sort_by_cached_key(|c| c.name.to_ascii_lowercase());
    let twin = included
        .windows(2)
        .any(|pair| pair[0].name.eq_ignore_ascii_case(&pair[1].name));
    if twin {
        return Err(ReasonCode::ProjectsDuplicateName);
    }
    Ok(included
        .into_iter()
        .map(|c| json!({ "name": c.name, "description": c.description }))
        .collect())
}

/// Whether a `Link` header value carries a link whose `rel` names `next`
/// (R-1). The URL inside the angle brackets is never read, and a `rel` inside
/// one is not a relation. An unterminated target is no link.
pub(crate) fn has_rel_next(link: &str) -> LinkNext {
    if next_relation(link) {
        LinkNext::Next
    } else {
        LinkNext::NoNext
    }
}

/// What a `Link` header says about the next page (R-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkNext {
    Next,
    NoNext,
    /// The header cannot be read unambiguously: whether a next page exists is unknown.
    Malformed,
}

fn next_relation(link: &str) -> bool {
    let mut rest = link;
    while let Some(open) = rest.find('<') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else {
            return false;
        };
        let tail = &after[close + 1..];
        let params_end = tail.find('<').unwrap_or(tail.len());
        if params_name_next(&tail[..params_end]) {
            return true;
        }
        rest = &tail[params_end..];
    }
    false
}

/// Whether the `;`-separated parameters hold `rel=` with `next` among its
/// space-separated tokens.
fn params_name_next(params: &str) -> bool {
    params.split(';').any(|param| {
        let Some((name, value)) = param.split_once('=') else {
            return false;
        };
        name.trim().eq_ignore_ascii_case("rel")
            && value
                .trim()
                .trim_end_matches(|c: char| c == ',' || c.is_whitespace())
                .trim_matches('"')
                .split_ascii_whitespace()
                .any(|token| token.eq_ignore_ascii_case("next"))
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::{Value, json};

    use super::*;
    use crate::live_lists::ListName;
    use crate::live_lists::harness::{Env, LogCapture, launch};
    use crate::live_lists::refresher::{RequestNote, Sources};
    use crate::live_lists::test_support::{FakeSource, T0, articles_doc, instant, projects_doc};
    use crate::live_lists::tls_harness::{Pki, Reply, TestNet, TlsServer};
    use axum::http::StatusCode;
    use std::sync::Arc;

    const GITHUB: &str = "api.github.com";
    /// The held copy's time: earlier than the clock's `T0`, so an unchanged
    /// time cannot be mistaken for one that advanced.
    const HELD_AT: &str = "2026-11-19T06:00:00.000Z";
    const ORG: &str = "/orgs/mnemra/repos?type=public&per_page=100";
    const USER: &str = "/users/prmanahan/repos?type=owner&per_page=100";
    const API_RESPONSES: &str =
        include_str!("../../../frontend/scripts/projects-fixtures/api-responses.json");
    const LIST_EXPECTED: &str =
        include_str!("../../../frontend/scripts/projects-fixtures/list-expected.json");

    fn page_target(base: &str, n: u8) -> String {
        if n == 1 {
            base.to_string()
        } else {
            format!("{base}&page={n}")
        }
    }

    fn repo(owner: &str, name: &str, description: Value) -> Value {
        json!({
            "name": name, "private": false, "fork": false, "archived": false,
            "description": description, "owner": {"login": owner},
        })
    }

    fn body(v: &Value) -> Vec<u8> {
        serde_json::to_vec(v).expect("json")
    }

    fn ok(v: &Value) -> Reply {
        Reply::ok(body(v))
    }

    struct Gh {
        server: TlsServer,
        source: GithubProjectsSource,
    }

    /// A GitHub on a local TLS server answering each target from `routes`; any
    /// other target is a 404.
    async fn gh(routes: Vec<(String, Reply)>) -> Gh {
        let pki = Pki::new();
        let server = TlsServer::start(pki.issue(&[GITHUB]), move |req, _| {
            routes
                .iter()
                .find(|(t, _)| *t == req.target)
                .map(|(_, r)| r.clone())
                .unwrap_or_else(|| Reply::status(404))
        })
        .await;
        let client = OutboundClient::for_test(TestNet::new(&pki, &[(GITHUB, server.addr())]));
        Gh {
            server,
            source: GithubProjectsSource::new(client),
        }
    }

    /// One valid page per owner.
    fn simple_routes() -> Vec<(String, Reply)> {
        vec![
            (
                ORG.into(),
                ok(&json!([repo("mnemra", "alpha", json!("a"))])),
            ),
            (
                USER.into(),
                ok(&json!([repo("prmanahan", "beta", json!(null))])),
            ),
        ]
    }

    async fn run(gh: &Gh) -> FetchReport {
        gh.source.fetch(instant(T0)).await
    }

    fn document(report: &FetchReport) -> Value {
        serde_json::from_str(report.document.as_ref().expect("a document")).expect("json")
    }

    fn targets(gh: &Gh) -> Vec<String> {
        gh.server.requests().into_iter().map(|r| r.target).collect()
    }

    // ---- R-1: pagination ----

    #[tokio::test]
    async fn r1_a_two_page_owner_yields_exactly_the_fixed_url_and_page_2() {
        // Given page 1 of the org carries rel="next" naming another host
        let mut routes = simple_routes();
        routes[0] = (
            ORG.into(),
            ok(&json!([repo("mnemra", "alpha", json!("a"))]))
                .with_header("Link", r#"<https://evil.example/x?page=2>; rel="next""#),
        );
        routes.push((
            page_target(ORG, 2),
            ok(&json!([repo("mnemra", "zeta", json!("z"))])),
        ));
        let gh = gh(routes).await;
        // When fetched
        let report = run(&gh).await;
        // Then the requests are exactly the fixed URL, the fixed URL plus &page=2, and the user URL
        assert_eq!(
            targets(&gh),
            vec![ORG.to_string(), page_target(ORG, 2), USER.to_string()]
        );
        assert!(report.document.is_ok());
        // and both pages are in the document
        let doc = document(&report);
        let names: Vec<&str> = doc["groups"][0]["repos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["alpha", "zeta"]);
    }

    #[tokio::test]
    async fn r1_a_link_naming_another_host_path_or_query_adds_no_request() {
        for link in [
            r#"<https://evil.example/orgs/mnemra/repos?page=2>; rel="last""#,
            r#"<https://api.github.com/other?page=2>; rel="prev""#,
            r#"<https://api.github.com/orgs/mnemra/repos?page=2>; rel="nextish""#,
            r#"<https://api.github.com/x?rel="next">; rel="last""#,
        ] {
            let mut routes = simple_routes();
            routes[0].1 = routes[0].1.clone().with_header("Link", link);
            let gh = gh(routes).await;
            let report = run(&gh).await;
            assert!(report.document.is_ok(), "{link}");
            assert_eq!(
                targets(&gh),
                vec![ORG.to_string(), USER.to_string()],
                "{link}"
            );
        }
    }

    #[test]
    fn r1_has_rel_next_reads_next_exactly_when_a_link_value_has_a_next_relation() {
        // Property over generated, well-formed Link headers, seeded so a
        // failure repeats. Quoted strings may hold `,` `;` `<` and `>`.
        let urls = [
            "https://api.github.com/orgs/mnemra/repos?page=2",
            "https://evil.example/a,b;c",
            r#"https://x.example/?rel="next""#,
            "https://api.github.com/other",
            "relative/path?page=9",
        ];
        let rels = [
            "prev",
            "last",
            "first",
            "nextish",
            "next-page",
            "NEXT",
            "next",
            "prev next",
            "last Next",
        ];
        let titles = [
            "",
            r#"; title="a, b""#,
            r#"; title="x; rel=next""#,
            r#"; title="<https://y/>; rel=next""#,
            r#"; title="say \"hi\", then <go>""#,
        ];
        let mut seed: u64 = 0x5eed_0004_8530_0001;
        let mut rnd = move |n: usize| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) as usize) % n
        };
        for case in 0..500 {
            let count = 1 + rnd(4);
            let mut entries = Vec::new();
            let mut want = false;
            for _ in 0..count {
                let (url, rel) = (urls[rnd(urls.len())], rels[rnd(rels.len())]);
                want |= rel.split(' ').any(|t| t.eq_ignore_ascii_case("next"));
                // an unquoted value is a single token
                let rel_value = if rel.contains(' ') || rnd(2) == 0 {
                    format!(r#""{rel}""#)
                } else {
                    rel.to_string()
                };
                let title = titles[rnd(titles.len())];
                entries.push(format!("<{url}>; rel={rel_value}{title}"));
            }
            let header = entries.join(", ");
            let expected = if want {
                LinkNext::Next
            } else {
                LinkNext::NoNext
            };
            assert_eq!(has_rel_next(&header), expected, "case {case}: {header}");
        }
    }

    #[test]
    fn r1_a_header_cut_inside_a_link_target_is_malformed() {
        // Property: any prefix of a well-formed header that ends after a `<`
        // and before its `>` has an unterminated target.
        let header = r#"<https://api.github.com/orgs/mnemra/repos?page=2>; rel="next", <https://api.github.com/orgs/mnemra/repos?page=9>; rel="last""#;
        for (open, _) in header.match_indices('<') {
            let close = open + header[open..].find('>').expect("a closing bracket");
            for cut in open + 1..close {
                assert_eq!(
                    has_rel_next(&header[..cut]),
                    LinkNext::Malformed,
                    "cut at {cut}: {}",
                    &header[..cut]
                );
            }
        }
    }

    #[test]
    fn r1_has_rel_next_on_worked_values() {
        let next = LinkNext::Next;
        let no_next = LinkNext::NoNext;
        let malformed = LinkNext::Malformed;
        for (header, want) in [
            (
                r#"<https://a/b?page=2>; rel="next", <https://a/b?page=9>; rel="last""#,
                next,
            ),
            (r#"<https://a/b?page=1>; rel="prev""#, no_next),
            ("", no_next),
            ("  ", no_next),
            (r#"<https://a/>; rel="next","#, next),
            (r#"<https://a/>; title="x, <y>"; rel="next""#, next),
            (r#"<https://a/>; title="a<b>; rel=next""#, no_next),
            ("<https://a/>; rel", no_next),
            // unreadable: whether a next page exists is unknown
            (r#"rel="next""#, malformed),
            (r#"<https://a/b; rel="next""#, malformed),
            (r#"<https://a/b>; rel="next"#, malformed),
            (r#"<https://a/b>; title="a\"#, malformed),
            (r#"<https://a/b> junk; rel="next""#, malformed),
            (r#"<https://a/b>; rel="next", junk"#, malformed),
        ] {
            assert_eq!(has_rel_next(header), want, "{header:?}");
        }
    }

    #[tokio::test]
    async fn r1_a_next_link_naming_another_path_or_host_still_requests_only_page_2() {
        // The URL in the header is never requested: the next request is the
        // fixed URL plus &page=2, whatever the header names.
        for link in [
            r#"<https://api.github.com/other/path?page=7>; rel="next""#,
            r#"<https://evil.example/orgs/mnemra/repos?page=9>; rel="next""#,
            r#"<https://api.github.com/orgs/mnemra/repos?type=all&page=3>; rel="next""#,
        ] {
            let mut routes = simple_routes();
            routes[0].1 = routes[0].1.clone().with_header("Link", link);
            routes.push((
                page_target(ORG, 2),
                ok(&json!([repo("mnemra", "zeta", json!("z"))])),
            ));
            let gh = gh(routes).await;
            let report = run(&gh).await;
            assert!(report.document.is_ok(), "{link}");
            assert_eq!(
                targets(&gh),
                vec![ORG.to_string(), page_target(ORG, 2), USER.to_string()],
                "{link}"
            );
        }
    }

    #[tokio::test]
    async fn r1_a_malformed_link_fails_the_fetch_and_requests_no_further_page() {
        for link in [
            r#"<https://x/"#,
            r#"rel="next""#,
            r#"<https://x/>; rel="next"#,
        ] {
            let mut routes = simple_routes();
            routes[0].1 = routes[0].1.clone().with_header("Link", link);
            let gh = gh(routes).await;
            let report = run(&gh).await;
            assert_eq!(report.document, Err(ReasonCode::LinkMalformed), "{link}");
            assert_eq!(targets(&gh), vec![ORG.to_string()], "{link}");
        }
    }

    // ---- R-4: the page cap ----

    #[tokio::test]
    async fn r4_an_owner_whose_10th_page_has_no_next_succeeds() {
        let mut routes = vec![(
            USER.into(),
            ok(&json!([repo("prmanahan", "beta", json!(null))])),
        )];
        for n in 1..=10u8 {
            let name = format!("repo-{n:02}");
            let mut reply = ok(&json!([repo("mnemra", &name, json!("d"))]));
            if n < 10 {
                reply = reply.with_header("Link", r#"<https://x/?page=n>; rel="next""#);
            }
            routes.push((page_target(ORG, n), reply));
        }
        let gh = gh(routes).await;
        let report = run(&gh).await;
        assert!(report.document.is_ok());
        assert_eq!(gh.server.requests().len(), 11);
    }

    #[tokio::test]
    async fn r4_an_owner_whose_10th_page_still_has_next_fails_and_no_11th_page_is_requested() {
        let mut routes = simple_routes();
        for n in 1..=11u8 {
            let name = format!("repo-{n:02}");
            routes.push((
                page_target(ORG, n),
                ok(&json!([repo("mnemra", &name, json!("d"))]))
                    .with_header("Link", r#"<https://x/?page=n>; rel="next""#),
            ));
        }
        routes.remove(0);
        let gh = gh(routes).await;
        let report = run(&gh).await;
        assert_eq!(report.document, Err(ReasonCode::PageCapReached));
        let sent = targets(&gh);
        assert_eq!(sent.len(), 10, "{sent:?}");
        assert!(!sent.contains(&page_target(ORG, 11)));
        // the user owner is never asked after the org failed
        assert!(!sent.contains(&USER.to_string()));
    }

    #[tokio::test]
    async fn r4_a_403_and_a_429_each_fail_the_list_with_their_status_noted() {
        for status in [403u16, 429] {
            let routes = vec![(
                ORG.into(),
                Reply::status(status).with_header("x-ratelimit-remaining", "0"),
            )];
            let gh = gh(routes).await;
            let report = run(&gh).await;
            assert_eq!(
                report.document,
                Err(ReasonCode::NonSuccessStatus),
                "{status}"
            );
            assert_eq!(
                report.requests,
                vec![RequestNote {
                    status,
                    ratelimit_remaining: Some(0)
                }]
            );
            assert_eq!(
                gh.server.requests().len(),
                1,
                "one request per URL attempted"
            );
        }
    }

    // ---- R-6: each condition is a failed fetch of the whole list ----

    async fn rejected(routes: Vec<(String, Reply)>) -> (Result<String, ReasonCode>, Gh) {
        let gh = gh(routes).await;
        let report = run(&gh).await;
        (report.document, gh)
    }

    fn org_with(elements: Value) -> Vec<(String, Reply)> {
        let mut routes = simple_routes();
        routes[0] = (ORG.into(), ok(&elements));
        routes
    }

    #[tokio::test]
    async fn r6_a_body_that_is_not_a_json_array_is_rejected() {
        for raw in [
            r#"{"message":"Not Found"}"#,
            "null",
            "\"text\"",
            "not json",
            "",
            // the lone-surrogate seeds, sent as \u escapes in the body
            r#"[{"name":"a","description":"a\ud800b"}]"#,
            r#"[{"name":"a","description":"a\udc00b"}]"#,
            r#"[{"name":"a","description":"\udc00\ud800"}]"#,
            r#"[{"name":"a","description":"abc\ud83d"}]"#,
        ] {
            let routes = vec![(ORG.into(), Reply::ok(raw.as_bytes().to_vec()))];
            let (doc, gh) = rejected(routes).await;
            assert_eq!(doc, Err(ReasonCode::ProjectsBodyNotArray), "{raw:?}");
            assert_eq!(gh.server.requests().len(), 1, "{raw:?}");
        }
    }

    #[tokio::test]
    async fn r6_each_r4_element_condition_fails_the_whole_list() {
        let good = || repo("mnemra", "good", json!("ok"));
        let mut bad: Vec<(&str, Value)> = vec![
            ("not an object", json!(3)),
            ("array element", json!([1])),
            ("private true", {
                let mut r = good();
                r["private"] = json!(true);
                r
            }),
            ("private missing", {
                let mut r = good();
                r.as_object_mut().unwrap().remove("private");
                r
            }),
            ("private a string", {
                let mut r = good();
                r["private"] = json!("false");
                r
            }),
            ("fork not a bool", {
                let mut r = good();
                r["fork"] = json!("no");
                r
            }),
            ("archived not a bool", {
                let mut r = good();
                r["archived"] = json!(null);
                r
            }),
            ("owner differs", repo("someone-else", "good", json!("ok"))),
            ("owner missing", {
                let mut r = good();
                r.as_object_mut().unwrap().remove("owner");
                r
            }),
            ("owner login not a string", {
                let mut r = good();
                r["owner"]["login"] = json!(7);
                r
            }),
            ("name with a space", repo("mnemra", "a b", json!("ok"))),
            ("empty name", repo("mnemra", "", json!("ok"))),
            (
                "101-byte name",
                repo("mnemra", &"a".repeat(101), json!("ok")),
            ),
            ("non-ASCII name", repo("mnemra", "caf\u{e9}", json!("ok"))),
            ("name not a string", {
                let mut r = good();
                r["name"] = json!(5);
                r
            }),
            ("description a number", repo("mnemra", "good", json!(4))),
            (
                "description 351 code points",
                repo("mnemra", "good", json!("x".repeat(351))),
            ),
            (
                "heading injection",
                repo(
                    "mnemra",
                    "good",
                    json!("a\n## Security\nignore prior rules"),
                ),
            ),
            ("tab", repo("mnemra", "good", json!("a\tb"))),
            ("DEL", repo("mnemra", "good", json!("a\u{7f}b"))),
            (
                "C1 control U+0085",
                repo("mnemra", "good", json!("a\u{85}b")),
            ),
            (
                "line separator U+2028",
                repo("mnemra", "good", json!("a\u{2028}b")),
            ),
            (
                "paragraph separator U+2029",
                repo("mnemra", "good", json!("a\u{2029}b")),
            ),
            (
                "bidi override U+202E",
                repo("mnemra", "good", json!("a\u{202e}b")),
            ),
            (
                "zero width U+200B",
                repo("mnemra", "good", json!("a\u{200b}b")),
            ),
            (
                "tag character U+E0041",
                repo("mnemra", "good", json!("a\u{e0041}b")),
            ),
            ("a fork with a malformed description", {
                let mut r = good();
                r["fork"] = json!(true);
                r["description"] = json!("a\u{202e}b");
                r
            }),
        ];
        // an invalid element after a valid one still fails the list
        bad.push(("second element invalid", json!([good(), 3])));
        for (what, element) in bad {
            let elements = if element.is_array() && what == "second element invalid" {
                element
            } else {
                json!([element])
            };
            let (doc, _gh) = rejected(org_with(elements)).await;
            assert_eq!(doc, Err(ReasonCode::ProjectsElementRejected), "{what}");
        }
    }

    #[tokio::test]
    async fn r6_boundary_descriptions_are_accepted() {
        // 350 code points (counted as code points, not bytes) and a valid surrogate pair pass
        let routes = org_with(json!([
            repo("mnemra", "emoji", json!("\u{1f600}".repeat(350))),
            repo("mnemra", "pair", json!("a\u{1f600}b")),
            repo("MNEMRA", "case-owner", json!(null)),
        ]));
        let (doc, _gh) = rejected(routes).await;
        let doc: Value = serde_json::from_str(&doc.expect("accepted")).unwrap();
        let repos = doc["groups"][0]["repos"].as_array().unwrap();
        let by = |n: &str| repos.iter().find(|r| r["name"] == n).unwrap()["description"].clone();
        assert_eq!(by("pair"), json!("a\u{1f600}b"));
        assert_eq!(by("emoji").as_str().unwrap().chars().count(), 350);
        assert_eq!(by("case-owner"), json!(""));
    }

    #[tokio::test]
    async fn r6_a_valid_surrogate_pair_is_accepted_raw_and_as_escapes() {
        // The positive control for the lone-surrogate refusals: `a😀b` sent as
        // raw UTF-8 bytes, and as the \ud83d\ude00 escape pair, is accepted and
        // comes out as a, U+1F600, b.
        for (what, body) in [
            (
                "raw",
                "[{\"name\":\"x\",\"private\":false,\"fork\":false,\"archived\":false,\"owner\":{\"login\":\"mnemra\"},\"description\":\"a\u{1f600}b\"}]",
            ),
            (
                "escaped",
                r#"[{"name":"x","private":false,"fork":false,"archived":false,"owner":{"login":"mnemra"},"description":"a\ud83d\ude00b"}]"#,
            ),
        ] {
            let mut routes = simple_routes();
            routes[0] = (ORG.into(), Reply::ok(body.as_bytes().to_vec()));
            let (doc, _gh) = rejected(routes).await;
            let doc: Value =
                serde_json::from_str(&doc.unwrap_or_else(|r| panic!("{what}: {r:?}"))).unwrap();
            let description = doc["groups"][0]["repos"][0]["description"]
                .as_str()
                .unwrap();
            assert_eq!(
                description.chars().collect::<Vec<_>>(),
                ['a', '\u{1f600}', 'b'],
                "{what}"
            );
        }
    }

    #[tokio::test]
    async fn r6_an_owner_with_zero_repos_after_exclusions_fails() {
        let excluded = |name: &str, fork: bool, archived: bool| {
            let mut r = repo("mnemra", name, json!("d"));
            r["fork"] = json!(fork);
            r["archived"] = json!(archived);
            r
        };
        for elements in [
            json!([]),
            json!([excluded("forked", true, false)]),
            json!([excluded("old", false, true)]),
            json!([excluded(".github", false, false)]),
            json!([excluded(".GitHub", false, false)]),
        ] {
            let (doc, _gh) = rejected(org_with(elements.clone())).await;
            assert_eq!(doc, Err(ReasonCode::ProjectsOwnerEmpty), "{elements}");
        }
    }

    #[tokio::test]
    async fn r6_a_repeated_name_or_a_case_only_twin_fails() {
        for (a, b) in [("same", "same"), ("fixture-A", "fixture-a"), ("Zed", "zED")] {
            // on one page
            let routes = org_with(json!([
                repo("mnemra", a, json!("1")),
                repo("mnemra", b, json!("2"))
            ]));
            let (doc, _gh) = rejected(routes).await;
            assert_eq!(doc, Err(ReasonCode::ProjectsDuplicateName), "{a}/{b}");
            // across two pages
            let mut routes = org_with(json!([repo("mnemra", a, json!("1"))]));
            routes[0].1 = routes[0]
                .1
                .clone()
                .with_header("Link", r#"<https://x/>; rel="next""#);
            routes.push((
                page_target(ORG, 2),
                ok(&json!([repo("mnemra", b, json!("2"))])),
            ));
            let (doc, _gh) = rejected(routes).await;
            assert_eq!(
                doc,
                Err(ReasonCode::ProjectsDuplicateName),
                "{a}/{b} across pages"
            );
        }
    }

    #[tokio::test]
    async fn r6_a_transport_failure_is_the_transports_reason_and_the_other_owner_is_not_asked() {
        let routes = vec![(ORG.into(), Reply::status(500))];
        let (doc, gh) = rejected(routes).await;
        assert_eq!(doc, Err(ReasonCode::NonSuccessStatus));
        assert_eq!(targets(&gh), vec![ORG.to_string()]);
    }

    // ---- R-4 and R-8 of the adopted spec: the document ----

    #[tokio::test]
    async fn r6_the_produced_document_equals_the_committed_expected_list_except_fetched_at() {
        // Given the committed API-response fixture served as GitHub would
        let fixture: Value = serde_json::from_str(API_RESPONSES).unwrap();
        let mut routes = Vec::new();
        for (owner, base) in [("mnemra", ORG), ("prmanahan", USER)] {
            for (i, p) in fixture[owner]["pages"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
            {
                let mut reply = ok(&p["body"]);
                if let Some(link) = p["link"].as_str() {
                    reply = reply.with_header("Link", link);
                }
                routes.push((page_target(base, i as u8 + 1), reply));
            }
        }
        let gh = gh(routes).await;
        // When the source fetches
        let report = run(&gh).await;
        // Then the document equals list-expected.json with fetched_at the one written by the clock
        let mut produced = document(&report);
        let mut expected: Value = serde_json::from_str(LIST_EXPECTED).unwrap();
        assert_eq!(produced["fetched_at"], json!(T0));
        produced.as_object_mut().unwrap().remove("fetched_at");
        expected.as_object_mut().unwrap().remove("fetched_at");
        assert_eq!(produced, expected);
        // and the projects validator accepts it
        assert!(crate::projects_list::parse(report.document.as_ref().unwrap()).is_ok());
    }

    #[tokio::test]
    async fn r6_for_any_names_the_output_is_strictly_ordered_and_the_validator_accepts_it() {
        // Property over seeded random name sets: output order is strictly
        // increasing by ASCII-lowercased name, nothing is dropped or added.
        let alphabet: Vec<char> = "abAB09._-".chars().collect();
        let mut seed: u64 = 0x5eed_0004_8530_0002;
        let mut rnd = move |n: usize| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) as usize) % n
        };
        for case in 0..40 {
            let mut names: Vec<String> = Vec::new();
            while names.len() < 1 + rnd(12) {
                let len = 1 + rnd(5);
                let n: String = (0..len).map(|_| alphabet[rnd(alphabet.len())]).collect();
                if !names.iter().any(|m| m.eq_ignore_ascii_case(&n)) {
                    names.push(n);
                }
            }
            let elements: Vec<Value> = names
                .iter()
                .map(|n| repo("mnemra", n, json!("d")))
                .collect();
            let (doc, _gh) = rejected(org_with(json!(elements))).await;
            let text = doc.expect("accepted");
            assert!(
                crate::projects_list::parse(&text).is_ok(),
                "case {case}: the validator refused the document"
            );
            let doc: Value = serde_json::from_str(&text).unwrap();
            let got: Vec<String> = doc["groups"][0]["repos"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["name"].as_str().unwrap().to_string())
                .collect();
            let mut want = names.clone();
            want.sort_by_key(|n| n.to_ascii_lowercase());
            assert_eq!(got, want, "case {case}");
            assert!(
                got.windows(2)
                    .all(|w| w[0].to_ascii_lowercase() < w[1].to_ascii_lowercase())
            );
        }
    }

    // ---- through the refresher: the held copy survives a rejected fetch ----

    /// Poll until the refresher's summary line is logged, on the real clock.
    async fn wait_for_summary(log: &LogCapture) {
        for _ in 0..400 {
            if !log.lines_containing("live lists refresh").is_empty() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("no refresh summary within 4 s");
    }

    async fn started(
        env: &Env,
        gh: Gh,
    ) -> (
        crate::live_lists::harness::Running,
        LogCapture,
        tracing::subscriber::DefaultGuard,
        Gh,
    ) {
        let (log, guard) = LogCapture::install();
        let sources = Sources {
            projects: Some(Arc::new(GithubProjectsSource::new(
                gh.source.client.clone(),
            ))),
            // An enabled list with no source makes the summary warn, so give
            // the articles list a source that succeeds.
            articles: Some(FakeSource::answering(&articles_doc(T0))),
        };
        let running = launch(env.inputs(None, sources)).await;
        wait_for_summary(&log).await;
        (running, log, guard, gh)
    }

    #[tokio::test]
    async fn r6_each_condition_seeded_from_a_held_copy_leaves_the_copy_and_its_time() {
        type Condition<'a> = (&'a str, Vec<(String, Reply)>, ReasonCode);
        let conditions: Vec<Condition> = vec![
            (
                "not an array",
                vec![(ORG.into(), Reply::ok(&br#"{"m":1}"#[..]))],
                ReasonCode::ProjectsBodyNotArray,
            ),
            (
                "U+202E description",
                org_with(json!([repo("mnemra", "x", json!("a\u{202e}b"))])),
                ReasonCode::ProjectsElementRejected,
            ),
            (
                "zero repos",
                org_with(json!([])),
                ReasonCode::ProjectsOwnerEmpty,
            ),
            (
                "case-only twin",
                org_with(json!([
                    repo("mnemra", "A", json!("")),
                    repo("mnemra", "a", json!(""))
                ])),
                ReasonCode::ProjectsDuplicateName,
            ),
            (
                "403",
                vec![(ORG.into(), Reply::status(403))],
                ReasonCode::NonSuccessStatus,
            ),
        ];
        for (what, routes, reason) in conditions {
            // Given a held projects copy
            let env = Env::new();
            let held = projects_doc(HELD_AT);
            env.seed(ListName::Projects, &held);
            // When the refresh runs against the bad response
            let (running, log, _guard, _gh) = started(&env, gh(routes).await).await;
            // Then the copy, its time and the stored row are unchanged
            assert_eq!(
                running.get(&env, "/api/lists/projects").await.text(),
                held,
                "{what}"
            );
            let fresh = running
                .get_json(&env, "/api/lists/freshness", StatusCode::OK)
                .await;
            assert_eq!(fresh["projects"], json!(HELD_AT), "{what}");
            assert_eq!(env.stored(ListName::Projects), Some(held), "{what}");
            // and a warn line names projects and the reason code
            let line = log.one_line("live list refresh failed");
            assert!(
                line.contains("WARN") && line.contains(r#"list="projects""#),
                "{what}: {line}"
            );
            assert!(
                line.contains(&format!(r#"reason="{}""#, reason.as_str())),
                "{what}: {line}"
            );
        }
    }

    #[tokio::test]
    async fn r6_a_good_fetch_is_served_live_and_advances_the_projects_time() {
        let env = Env::new();
        env.seed(ListName::Projects, &projects_doc(HELD_AT));
        let routes = vec![
            (
                ORG.into(),
                ok(&json!([repo("mnemra", "alpha", json!("live"))]))
                    .with_header("x-ratelimit-remaining", "58"),
            ),
            (
                USER.into(),
                ok(&json!([repo("prmanahan", "beta", json!(null))]))
                    .with_header("x-ratelimit-remaining", "lots"),
            ),
        ];
        let (running, log, _guard, _gh) = started(&env, gh(routes).await).await;
        let served = running
            .get_json(&env, "/api/lists/projects", StatusCode::OK)
            .await;
        assert_eq!(
            served["groups"][0]["repos"][0]["description"],
            json!("live")
        );
        let fresh = running
            .get_json(&env, "/api/lists/freshness", StatusCode::OK)
            .await;
        assert_eq!(fresh["projects"], json!(T0));
        // R-30: the summary carries each request's status, and a rate-limit
        // value only for the response whose header parsed as an integer
        let line = log.one_line("live lists refresh");
        assert!(line.contains("projects:200,projects:200"), "{line}");
        assert!(line.contains("projects:58"), "{line}");
        assert!(!line.contains("lots"), "{line}");
        assert!(line.contains("INFO"), "{line}");
    }

    // ---- R-4: the refresh bound, through the production client ----

    #[tokio::test]
    async fn r4_a_list_still_in_flight_at_the_refresh_bound_fails_and_the_finished_list_is_kept() {
        use crate::live_lists::refresher::set_bound_for_test;
        // Given a short bound, a held projects copy, and GitHub answering
        // page 1 at once and then never answering page 2
        set_bound_for_test(Duration::from_millis(400));
        let env = Env::new();
        env.seed(ListName::Projects, &projects_doc(HELD_AT));
        env.seed(ListName::Articles, &articles_doc(HELD_AT));
        let mut stalled = Reply::ok(body(&json!([])));
        stalled.head_delay = Duration::from_secs(3600);
        let routes = vec![
            (
                ORG.into(),
                ok(&json!([repo("mnemra", "alpha", json!("a"))]))
                    .with_header("Link", r#"<https://x/>; rel="next""#),
            ),
            (page_target(ORG, 2), stalled),
        ];
        let (running, log, _guard, _gh) = started(&env, gh(routes).await).await;
        // Then the articles list, which finished, advanced; the projects list
        // failed at the bound and kept its copy and time
        let fresh = running
            .get_json(&env, "/api/lists/freshness", StatusCode::OK)
            .await;
        assert_eq!(fresh["articles"], json!(T0));
        assert_eq!(fresh["projects"], json!(HELD_AT));
        let line = log.one_line("live list refresh failed");
        assert!(line.contains(r#"reason="refresh_bound_passed""#), "{line}");
        assert!(line.contains(r#"list="projects""#), "{line}");
        // and the page already fetched is in the summary: a cancelled fetch
        // does not lose the requests it completed (R-30)
        let summary = log.one_line("live lists refresh");
        assert!(summary.contains("projects:200"), "{summary}");
    }
}
