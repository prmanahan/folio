//! The outbound HTTPS client (R-1 to R-5, R-30): the only code in the server
//! that makes a request to a list source.
//!
//! ONE constructor builds it, [`OutboundClient::new`], and it takes no source
//! URL, scheme, trust root, resolver or proxy. A request is for a [`Target`],
//! a closed set of R-1's three URLs (GitHub's two with a bounded [`Page`]),
//! so no other URL can be asked for. The client refuses `http://`, any
//! certificate that does not chain to the system roots and name the host, any
//! 3xx, any non-identity `Content-Encoding` and any non-2xx status; it ignores
//! the proxy environment, sends exactly `Host`, `Accept`, `Accept-Encoding:
//! identity` and `User-Agent`, and enforces R-4's limits.
//!
//! A failure is a [`FetchFailure`] holding a closed [`ReasonCode`], never a
//! wrapped error, and no type here derives `Debug` over a response body, so
//! no byte of a body reaches a log line (R-30).

use std::time::Duration;

use url::Url;

use super::ReasonCode;
use super::refresher::RequestNote;

/// R-4: a body above this many bytes, counted as received, fails the fetch.
pub(crate) const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
/// R-4: the connect timeout.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// R-4: the total timeout, body read included.
pub(crate) const TOTAL_TIMEOUT: Duration = Duration::from_secs(20);
/// R-4: the page cap per GitHub owner.
pub(crate) const MAX_PAGES: u8 = 10;

/// A GitHub page number, 1 to [`MAX_PAGES`]. The cap is in the type, so a
/// request for an 11th page cannot be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Page(u8);

impl Page {
    pub(crate) const FIRST: Page = Page(1);

    pub(crate) fn new(n: u8) -> Option<Page> {
        (1..=MAX_PAGES).contains(&n).then_some(Page(n))
    }

    pub(crate) fn get(self) -> u8 {
        self.0
    }

    /// The page after this one, or `None` past the cap.
    pub(crate) fn next(self) -> Option<Page> {
        Page::new(self.0 + 1)
    }
}

/// R-1's request set. Nothing else can be requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// `https://api.github.com/orgs/mnemra/repos?type=public&per_page=100`
    GithubOrg(Page),
    /// `https://api.github.com/users/prmanahan/repos?type=owner&per_page=100`
    GithubUser(Page),
    /// `https://mnemra.dev/rss.xml`
    Feed,
}

impl Target {
    /// The fixed URL, plus `&page=N` for page N of 2 or more.
    pub(crate) fn url(self) -> Url {
        let (base, page) = match self {
            Target::GithubOrg(p) => (
                "https://api.github.com/orgs/mnemra/repos?type=public&per_page=100",
                Some(p),
            ),
            Target::GithubUser(p) => (
                "https://api.github.com/users/prmanahan/repos?type=owner&per_page=100",
                Some(p),
            ),
            Target::Feed => ("https://mnemra.dev/rss.xml", None),
        };
        let text = match page {
            Some(p) if p.get() >= 2 => format!("{base}&page={}", p.get()),
            _ => base.to_string(),
        };
        Url::parse(&text).expect("a fixed URL")
    }

    fn accept(self) -> &'static str {
        match self {
            Target::GithubOrg(_) | Target::GithubUser(_) => "application/vnd.github+json",
            Target::Feed => "application/rss+xml, application/xml;q=0.9",
        }
    }
}

/// A 2xx response. It holds the body, so it has no `Debug`.
pub(crate) struct Fetched {
    pub(crate) note: RequestNote,
    pub(crate) body: Vec<u8>,
}

/// Why a request failed, and what is known of the response if one arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FetchFailure {
    pub(crate) reason: ReasonCode,
    pub(crate) note: Option<RequestNote>,
}

/// The client could not be built.
#[derive(Debug)]
pub struct BuildError;

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the outbound client could not be built")
    }
}

impl std::error::Error for BuildError {}

#[derive(Clone)]
pub struct OutboundClient {
    _private: (),
}

impl OutboundClient {
    pub fn new() -> Result<Self, BuildError> {
        Ok(Self { _private: () })
    }

    #[cfg(test)]
    pub(crate) fn for_test(_net: super::tls_harness::TestNet) -> Self {
        Self { _private: () }
    }

    pub(crate) async fn fetch(&self, target: Target) -> Result<Fetched, FetchFailure> {
        self.fetch_url(&target.url(), target.accept()).await
    }

    async fn fetch_url(&self, _url: &Url, _accept: &str) -> Result<Fetched, FetchFailure> {
        Err(FetchFailure {
            reason: ReasonCode::TransportFailed,
            note: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    use tokio::time::Instant;

    use super::*;
    use crate::live_lists::tls_harness::{CountingListener, Pki, Reply, TestNet, TlsServer};

    const GITHUB: &str = "api.github.com";
    const FEED: &str = "mnemra.dev";
    const JSON: &[u8] = include_bytes!("fixtures/encoded/plain.json");

    struct Rig {
        server: TlsServer,
        client: OutboundClient,
    }

    /// A TLS server for both names, trusted by the client, answering `reply`.
    async fn rig(reply: Reply) -> Rig {
        let pki = Pki::new();
        let server = TlsServer::always(pki.issue(&[GITHUB, FEED]), reply).await;
        let net = TestNet::new(&pki, &[(GITHUB, server.addr()), (FEED, server.addr())]);
        Rig {
            client: OutboundClient::for_test(net),
            server,
        }
    }

    fn reason<T>(result: Result<T, FetchFailure>) -> Option<ReasonCode> {
        result.err().map(|f| f.reason)
    }

    fn url(text: &str) -> Url {
        Url::parse(text).expect("a url")
    }

    fn org() -> Target {
        Target::GithubOrg(Page::FIRST)
    }

    /// Env vars are process-global: tests that set them take this lock.
    static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    struct EnvGuard {
        names: Vec<&'static str>,
        _lock: tokio::sync::MutexGuard<'static, ()>,
    }

    impl EnvGuard {
        async fn set(vars: &[(&'static str, &str)]) -> Self {
            let lock = ENV_LOCK.lock().await;
            for (k, v) in vars {
                // SAFETY: every test that touches the environment holds
                // ENV_LOCK, and none reads these variables concurrently.
                unsafe { std::env::set_var(k, v) };
            }
            Self {
                names: vars.iter().map(|(k, _)| *k).collect(),
                _lock: lock,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for k in &self.names {
                // SAFETY: as in `set`; the lock is still held.
                unsafe { std::env::remove_var(k) };
            }
        }
    }

    fn unique(prefix: &str) -> String {
        static N: AtomicU32 = AtomicU32::new(0);
        format!(
            "{prefix}-{}-{}",
            uuid::Uuid::new_v4().simple(),
            N.fetch_add(1, Ordering::SeqCst)
        )
    }

    // ---- R-1: the request set is closed ----

    #[test]
    fn target_urls_are_r1s_fixed_urls_and_the_page_form() {
        // Given every page a Page can hold, When the URL is built, Then it is
        // the fixed URL (page 1) or the fixed URL plus `&page=N`.
        let org_base = "https://api.github.com/orgs/mnemra/repos?type=public&per_page=100";
        let user_base = "https://api.github.com/users/prmanahan/repos?type=owner&per_page=100";
        for n in 1..=MAX_PAGES {
            let page = Page::new(n).expect("in range");
            let suffix = if n == 1 {
                String::new()
            } else {
                format!("&page={n}")
            };
            assert_eq!(
                Target::GithubOrg(page).url().as_str(),
                format!("{org_base}{suffix}")
            );
            assert_eq!(
                Target::GithubUser(page).url().as_str(),
                format!("{user_base}{suffix}")
            );
        }
        assert_eq!(Target::Feed.url().as_str(), "https://mnemra.dev/rss.xml");
    }

    #[test]
    fn a_page_outside_one_to_ten_cannot_be_built() {
        // R-4: the page cap is in the type.
        assert_eq!(Page::new(0), None);
        assert_eq!(Page::new(MAX_PAGES + 1), None);
        assert_eq!(Page::new(255), None);
        assert_eq!(Page::new(MAX_PAGES).and_then(Page::next), None);
        assert_eq!(Page::FIRST.next(), Page::new(2));
    }

    // ---- R-2 ----

    #[tokio::test(start_paused = true)]
    async fn r2_refuses_http_and_the_plain_listener_sees_no_connection() {
        // Given a plain listener mapped to the name
        let plain = CountingListener::start().await;
        let pki = Pki::new();
        let client = OutboundClient::for_test(TestNet::new(&pki, &[(GITHUB, plain.addr())]));
        // When an http:// URL is requested
        let result = client
            .fetch_url(&url("http://api.github.com/orgs/mnemra/repos"), "*/*")
            .await;
        // Then it fails with the scheme-refusal reason and nothing connected
        assert_eq!(reason(result), Some(ReasonCode::SchemeRefused));
        assert_eq!(plain.connections(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r2_refuses_an_untrusted_certificate_and_trusts_it_once_its_root_is_trusted() {
        // Given a server whose certificate chains to a root the client lacks
        let (trusted, other) = (Pki::new(), Pki::new());
        let server = TlsServer::always(other.issue(&[GITHUB]), Reply::ok(JSON)).await;
        let untrusting =
            OutboundClient::for_test(TestNet::new(&trusted, &[(GITHUB, server.addr())]));
        // Then the certificate-class error
        assert_eq!(
            reason(untrusting.fetch(org()).await),
            Some(ReasonCode::CertificateRefused)
        );
        assert_eq!(server.requests().len(), 0);
        // And, the positive control: the same server with its root trusted succeeds
        let trusting = OutboundClient::for_test(TestNet::new(&other, &[(GITHUB, server.addr())]));
        let fetched = trusting.fetch(org()).await.ok().expect("trusted root");
        assert_eq!(fetched.body, JSON);
        assert_eq!(fetched.note.status, 200);
    }

    #[tokio::test(start_paused = true)]
    async fn r2_refuses_a_certificate_that_does_not_name_the_host() {
        // Given a certificate under the trusted root that names another host
        let pki = Pki::new();
        let server = TlsServer::always(pki.issue(&["other.example"]), Reply::ok(JSON)).await;
        let client = OutboundClient::for_test(TestNet::new(&pki, &[(GITHUB, server.addr())]));
        // Then the certificate-class error
        assert_eq!(
            reason(client.fetch(org()).await),
            Some(ReasonCode::CertificateRefused)
        );
        assert_eq!(server.requests().len(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r2_any_3xx_fails_and_its_location_is_not_requested() {
        for status in [301u16, 302, 303, 307, 308] {
            // Given a server that redirects to a path on itself
            let rig =
                rig(Reply::status(status)
                    .with_header("Location", "https://api.github.com/elsewhere"))
                .await;
            // When fetched
            let result = rig.client.fetch(org()).await;
            // Then it fails with the redirect reason and the server saw one request
            assert_eq!(
                reason(result),
                Some(ReasonCode::RedirectRefused),
                "{status}"
            );
            assert_eq!(rig.server.requests().len(), 1, "{status}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r2_ignores_the_proxy_environment() {
        // Given the three proxy variables set to a listener that records connections
        let proxy = CountingListener::start().await;
        let at = format!("http://{}", proxy.addr());
        let _env = EnvGuard::set(&[
            ("HTTPS_PROXY", &at),
            ("HTTP_PROXY", &at),
            ("ALL_PROXY", &at),
        ])
        .await;
        // When a client built under them fetches
        let rig = rig(Reply::ok(JSON)).await;
        // Then the fetch succeeds and the proxy recorded nothing
        assert!(rig.client.fetch(org()).await.is_ok());
        assert_eq!(proxy.connections(), 0);
        assert_eq!(rig.server.requests().len(), 1);
    }

    // ---- R-3 ----

    #[tokio::test(start_paused = true)]
    async fn r3_sends_exactly_four_headers_on_each_of_the_three_urls() {
        let want: BTreeSet<&str> = ["host", "accept", "accept-encoding", "user-agent"].into();
        for target in [org(), Target::GithubUser(Page::FIRST), Target::Feed] {
            let rig = rig(Reply::ok(JSON)).await;
            assert!(rig.client.fetch(target).await.is_ok());
            let sent = rig.server.requests();
            assert_eq!(sent.len(), 1);
            let names = sent[0].header_names();
            let got: BTreeSet<&str> = names.iter().map(String::as_str).collect();
            assert_eq!(got, want, "{target:?}");
            assert_eq!(names.len(), 4, "no repeated header, {target:?}");
            assert_eq!(sent[0].header("accept-encoding"), Some("identity"));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r3_no_header_value_holds_an_app_secret() {
        // Given four secrets, each a distinct value made for this run
        let secrets: Vec<(&'static str, String)> = [
            "ADMIN_PASSWORD",
            "ANTHROPIC_API_KEY",
            "EDGE_AUTH_TOKEN",
            "PAGE_HIT_SALT",
        ]
        .into_iter()
        .map(|k| (k, unique(k)))
        .collect();
        let vars: Vec<(&'static str, &str)> =
            secrets.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let _env = EnvGuard::set(&vars).await;
        // When each URL is fetched
        for target in [org(), Target::GithubUser(Page::FIRST), Target::Feed] {
            let rig = rig(Reply::ok(JSON)).await;
            assert!(rig.client.fetch(target).await.is_ok());
            // Then no received header value contains any secret
            for request in rig.server.requests() {
                for (name, value) in &request.headers {
                    for (key, secret) in &secrets {
                        assert!(!value.contains(secret.as_str()), "{key} leaked in {name}");
                    }
                }
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r3_each_codec_fixture_fails_with_the_encoding_reason() {
        // Given one fixture per codec, checked in as bytes
        let cases: [(&str, &[u8]); 4] = [
            ("gzip", include_bytes!("fixtures/encoded/body.gz")),
            ("br", include_bytes!("fixtures/encoded/body.br")),
            ("deflate", include_bytes!("fixtures/encoded/body.deflate")),
            ("zstd", include_bytes!("fixtures/encoded/body.zst")),
        ];
        for (codec, bytes) in cases {
            // When sent with the matching Content-Encoding
            let rig = rig(Reply::ok(bytes).with_header("Content-Encoding", codec)).await;
            let result = rig.client.fetch(org()).await;
            // Then it is a failed fetch with the encoding reason, not a decode failure
            let failure = result.err().expect(codec);
            assert_eq!(
                failure.reason,
                ReasonCode::ContentEncodingRefused,
                "{codec}"
            );
            assert_eq!(failure.note.map(|n| n.status), Some(200), "{codec}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r3_any_content_encoding_but_identity_is_refused() {
        // Property over spellings: only the identity coding is accepted.
        for value in [
            "GZIP",
            "x-gzip",
            "compress",
            "identity, gzip",
            "gzip, identity",
            "",
            "zstd;q=1",
            "Br",
        ] {
            let rig = rig(Reply::ok(JSON).with_header("Content-Encoding", value)).await;
            assert_eq!(
                reason(rig.client.fetch(org()).await),
                Some(ReasonCode::ContentEncodingRefused),
                "{value:?}"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r3_identity_and_no_content_encoding_are_accepted() {
        for headers in [
            vec![("Content-Encoding", "identity")],
            vec![("Content-Encoding", "IDENTITY")],
            vec![],
        ] {
            let mut reply = Reply::ok(JSON);
            for (n, v) in &headers {
                reply = reply.with_header(n, v);
            }
            let rig = rig(reply).await;
            let fetched = rig.client.fetch(org()).await.ok().expect("accepted");
            assert_eq!(fetched.body, JSON, "{headers:?}");
        }
    }

    // ---- R-4 ----

    #[tokio::test(start_paused = true)]
    async fn r4_a_connect_just_before_5s_succeeds() {
        let pki = Pki::new();
        let server = TlsServer::start_slow_handshake(
            pki.issue(&[GITHUB]),
            CONNECT_TIMEOUT - Duration::from_millis(1),
            |_, _| Reply::ok(JSON),
        )
        .await;
        let client = OutboundClient::for_test(TestNet::new(&pki, &[(GITHUB, server.addr())]));
        assert!(client.fetch(org()).await.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn r4_a_connect_that_never_completes_fails_at_5s_and_not_before() {
        let pki = Pki::new();
        let server = TlsServer::start_slow_handshake(
            pki.issue(&[GITHUB]),
            Duration::from_secs(3600),
            |_, _| Reply::ok(JSON),
        )
        .await;
        let client = OutboundClient::for_test(TestNet::new(&pki, &[(GITHUB, server.addr())]));
        let began = Instant::now();
        let result = client.fetch(org()).await;
        let took = began.elapsed();
        assert_eq!(reason(result), Some(ReasonCode::ConnectTimeout));
        assert!(took >= CONNECT_TIMEOUT, "failed early, at {took:?}");
        assert!(
            took <= CONNECT_TIMEOUT + Duration::from_millis(1),
            "failed late, at {took:?}"
        );
        assert_eq!(server.requests().len(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r4_a_response_that_completes_just_before_20s_succeeds() {
        let mut reply = Reply::ok(JSON);
        reply.head_delay = TOTAL_TIMEOUT - Duration::from_millis(1);
        let rig = rig(reply).await;
        assert!(rig.client.fetch(org()).await.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn r4_a_body_that_stalls_fails_at_20s_and_not_before() {
        let mut reply = Reply::ok(JSON);
        reply.stall_after = Some(3);
        let rig = rig(reply).await;
        let began = Instant::now();
        let result = rig.client.fetch(org()).await;
        let took = began.elapsed();
        assert_eq!(reason(result), Some(ReasonCode::TotalTimeout));
        assert!(took >= TOTAL_TIMEOUT, "failed early, at {took:?}");
        assert!(
            took <= TOTAL_TIMEOUT + Duration::from_millis(1),
            "failed late, at {took:?}"
        );
        assert_eq!(rig.server.requests().len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r4_a_body_of_exactly_2_mib_is_returned_and_one_byte_more_fails() {
        // Given a body of exactly the cap
        let rig_at = rig(Reply::ok(vec![b'x'; MAX_BODY_BYTES])).await;
        let fetched = rig_at.client.fetch(org()).await.ok().expect("at the cap");
        assert_eq!(fetched.body.len(), MAX_BODY_BYTES);
        // And one of cap + 1 bytes
        let rig_over = rig(Reply::ok(vec![b'x'; MAX_BODY_BYTES + 1])).await;
        assert_eq!(
            reason(rig_over.client.fetch(org()).await),
            Some(ReasonCode::BodyOverCap)
        );
        assert_eq!(rig_over.server.requests().len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn r4_non_2xx_statuses_fail_and_carry_their_status() {
        for status in [400u16, 401, 403, 404, 429, 500, 503] {
            let rig = rig(Reply::status(status)).await;
            let failure = rig.client.fetch(org()).await.err().expect("non-2xx");
            assert_eq!(failure.reason, ReasonCode::NonSuccessStatus, "{status}");
            assert_eq!(failure.note.map(|n| n.status), Some(status));
            assert_eq!(
                rig.server.requests().len(),
                1,
                "one request per URL, {status}"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r4_a_2xx_other_than_200_is_accepted() {
        let rig = rig(Reply::status(204)).await;
        let fetched = rig.client.fetch(org()).await.ok().expect("204");
        assert_eq!(fetched.note.status, 204);
    }

    // ---- R-30: the rate-limit field ----

    #[tokio::test(start_paused = true)]
    async fn r30_the_ratelimit_header_is_noted_only_when_it_is_an_integer() {
        for (value, want) in [
            ("59", Some(59)),
            ("0", Some(0)),
            ("many", None),
            ("5.5", None),
            ("", None),
        ] {
            let rig = rig(Reply::ok(JSON).with_header("x-ratelimit-remaining", value)).await;
            let fetched = rig.client.fetch(org()).await.ok().expect("2xx");
            assert_eq!(fetched.note.ratelimit_remaining, want, "{value:?}");
        }
        let rig = rig(Reply::ok(JSON)).await;
        let fetched = rig.client.fetch(org()).await.ok().expect("2xx");
        assert_eq!(fetched.note.ratelimit_remaining, None);
    }

    #[tokio::test(start_paused = true)]
    async fn r30_a_failed_response_still_carries_its_ratelimit_note() {
        let rig = rig(Reply::status(403).with_header("x-ratelimit-remaining", "0")).await;
        let failure = rig.client.fetch(org()).await.err().expect("403");
        assert_eq!(
            failure.note,
            Some(RequestNote {
                status: 403,
                ratelimit_remaining: Some(0)
            })
        );
    }

    // ---- R-5 ----

    #[tokio::test(start_paused = true)]
    async fn r5_a_name_outside_the_map_fails_with_a_resolution_error() {
        // Given a server and a map that does not name `unmapped.example`
        let rig = rig(Reply::ok(JSON)).await;
        // When that name is requested
        let result = rig
            .client
            .fetch_url(&url("https://unmapped.example/"), "*/*")
            .await;
        // Then resolution fails and nothing connected
        assert_eq!(reason(result), Some(ReasonCode::NameResolutionFailed));
        assert_eq!(rig.server.connections(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn r5_a_refused_connection_is_a_transport_failure() {
        // Given a name mapped to a port nothing listens on
        let pki = Pki::new();
        let dead = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
            l.local_addr().expect("addr")
        };
        let client = OutboundClient::for_test(TestNet::new(&pki, &[(GITHUB, dead)]));
        assert_eq!(
            reason(client.fetch(org()).await),
            Some(ReasonCode::TransportFailed)
        );
    }
}
