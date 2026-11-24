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

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, LINK};
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

    /// Whether the target is a GitHub page, the only kind whose `Link` is read.
    fn paginates(self) -> bool {
        matches!(self, Target::GithubOrg(_) | Target::GithubUser(_))
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
    /// Every `Link` header value, joined with `, `; `None` when the response
    /// has none. Read only for `rel="next"`: the URLs in it are never requested (R-1).
    pub(crate) link: Option<String>,
    pub(crate) body: Vec<u8>,
}

/// Why a request failed, and what is known of the response if one arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FetchFailure {
    pub(crate) reason: ReasonCode,
    pub(crate) note: Option<RequestNote>,
}

/// The client could not be built. The cause is not carried: it is not safe
/// to assume it holds no text from outside.
#[derive(Debug)]
pub struct BuildError;

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the outbound client could not be built")
    }
}

impl std::error::Error for BuildError {}

/// R-3's `User-Agent`. GitHub refuses a request without one.
const USER_AGENT: &str = "folio-live-lists (https://peter.manahan.io)";

/// The resolver's failure, as a type of this module's own so a resolution
/// error is recognised by type, never by the text of a wrapped error.
#[derive(Debug)]
struct Unresolved;

impl std::fmt::Display for Unresolved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the host name did not resolve")
    }
}

impl std::error::Error for Unresolved {}

fn unresolved() -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(Unresolved)
}

/// The production resolver: the system's, with failures typed as [`Unresolved`].
struct SystemResolver;

impl Resolve for SystemResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let found: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await
                .map_err(|_cause| unresolved())?
                .collect();
            if found.is_empty() {
                return Err(unresolved());
            }
            Ok(Box::new(found.into_iter()) as Addrs)
        })
    }
}

/// The test resolver: only the names in the test's map resolve (R-5).
#[cfg(test)]
struct MapResolver(std::collections::HashMap<String, SocketAddr>);

#[cfg(test)]
impl Resolve for MapResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let found = self.0.get(name.as_str()).copied();
        Box::pin(async move {
            match found {
                Some(addr) => Ok(Box::new(std::iter::once(addr)) as Addrs),
                None => Err(unresolved()),
            }
        })
    }
}

/// The environment variables rustls-native-certs, under the platform
/// verifier, reads trust roots from.
const TRUST_ROOT_VARIABLES: [&str; 2] = ["SSL_CERT_FILE", "SSL_CERT_DIR"];

/// The one policy, shared by the production and test clients. They differ
/// only in what they add to it: the resolver, and for tests a trust root.
fn policy() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        // R-2: the proxy environment is never read.
        .no_proxy()
        // R-2: a 3xx is a failed fetch and its `Location` is never requested.
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        // R-3: HTTP/1.1, so `Host` is a header. A body is never decompressed:
        // no decompression feature is compiled in, and these refuse one that
        // another dependency enables later. `Accept-Encoding: identity` is
        // sent per request.
        .http1_only()
        .no_gzip()
        .no_brotli()
        .no_zstd()
        .no_deflate()
        .user_agent(USER_AGENT)
        // R-4
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
}

/// The one client. Cheap to clone; clones share a connection pool.
#[derive(Clone)]
pub struct OutboundClient {
    inner: reqwest::Client,
}

impl OutboundClient {
    /// Build the production client. It takes no source URL, scheme, trust
    /// root, resolver or proxy (R-2): the system roots, the system resolver
    /// and no proxy are the only choices.
    ///
    /// The platform verifier on Linux also loads trust roots named by
    /// `SSL_CERT_FILE` and `SSL_CERT_DIR`, which would let the environment
    /// add a root. Either variable being set therefore fails the build, so
    /// the signed gap "TLS test hook by review" stays the only way in.
    ///
    /// # Errors
    /// [`BuildError`] when either trust-root variable is set, or the TLS
    /// stack cannot be initialised. Only a fixed message is logged, at `error`.
    pub fn new() -> Result<Self, BuildError> {
        for name in TRUST_ROOT_VARIABLES {
            if std::env::var_os(name).is_some() {
                tracing::error!(
                    variable = name,
                    "the outbound client refuses to build while a trust-root variable is set"
                );
                return Err(BuildError);
            }
        }
        Self::finish(policy().dns_resolver(Arc::new(SystemResolver)))
    }

    /// The test client: the production policy, with `roots` added to the
    /// trust store and the name map as the only resolver (R-5). Gated by
    /// `#[cfg(test)]`, so only a source change can put a trust root or
    /// resolver seam into the shipped client.
    #[cfg(test)]
    pub(crate) fn for_test(net: super::tls_harness::TestNet) -> Self {
        let mut builder = policy().dns_resolver(Arc::new(MapResolver(net.names)));
        for der in &net.roots {
            builder = builder
                .add_root_certificate(reqwest::Certificate::from_der(der).expect("a DER root"));
        }
        Self::finish(builder).expect("the test client builds")
    }

    fn finish(builder: reqwest::ClientBuilder) -> Result<Self, BuildError> {
        // The cause is dropped: a fixed message is all that is logged (R-30),
        // as [`BuildError`] says.
        let inner = builder.build().map_err(|_cause| {
            tracing::error!("the outbound client could not be built");
            BuildError
        })?;
        Ok(Self { inner })
    }

    /// Fetch one of R-1's URLs.
    pub(crate) async fn fetch(&self, target: Target) -> Result<Fetched, FetchFailure> {
        self.fetch_url(&target.url(), target.accept(), target.paginates())
            .await
    }

    /// Fetch `url`. Private: the one way in from outside this module is
    /// [`OutboundClient::fetch`], so R-1's request set is closed. Tests reach
    /// it for R-2's `http://` and unmapped-name cases.
    async fn fetch_url(
        &self,
        url: &Url,
        accept: &str,
        read_link: bool,
    ) -> Result<Fetched, FetchFailure> {
        let failed = |reason, note| FetchFailure { reason, note };
        if url.scheme() != "https" {
            return Err(failed(ReasonCode::SchemeRefused, None));
        }
        let mut response = self
            .inner
            .get(url.clone())
            .header(ACCEPT, accept)
            .header(ACCEPT_ENCODING, "identity")
            .send()
            .await
            .map_err(|e| failed(classify(&e), None))?;

        let status = response.status();
        let note = RequestNote {
            status: status.as_u16(),
            ratelimit_remaining: response
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<i64>().ok()),
        };
        if status.is_redirection() {
            return Err(failed(ReasonCode::RedirectRefused, Some(note)));
        }
        if !status.is_success() {
            return Err(failed(ReasonCode::NonSuccessStatus, Some(note)));
        }
        // R-3: a body is never decoded. Only `identity` is accepted; an
        // unreadable or empty value is not `identity`, so it is refused.
        let identity_only = response
            .headers()
            .get_all(CONTENT_ENCODING)
            .iter()
            .all(|v| {
                v.to_str()
                    .is_ok_and(|v| v.trim().eq_ignore_ascii_case("identity"))
            });
        if !identity_only {
            return Err(failed(ReasonCode::ContentEncodingRefused, Some(note)));
        }

        // R-1: pagination reads `rel="next"` from here. A `Link` value that is
        // not plain text is ambiguous, so the fetch fails rather than ending
        // the list early.
        // The feed never paginates, so its `Link` is not read at all.
        let mut link_values = Vec::new();
        for value in response
            .headers()
            .get_all(LINK)
            .iter()
            .filter(|_| read_link)
        {
            match value.to_str() {
                Ok(text) => link_values.push(text.to_string()),
                Err(_unreadable) => return Err(failed(ReasonCode::LinkMalformed, Some(note))),
            }
        }
        let link = (!link_values.is_empty()).then(|| link_values.join(", "));

        let mut body = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    // R-4: counted on bytes received, stopping at the first
                    // chunk that passes the cap.
                    if body.len() + chunk.len() > MAX_BODY_BYTES {
                        return Err(failed(ReasonCode::BodyOverCap, Some(note)));
                    }
                    body.extend_from_slice(&chunk);
                }
                Ok(None) => return Ok(Fetched { note, link, body }),
                Err(e) => return Err(failed(classify(&e), Some(note))),
            }
        }
    }
}

/// Map a transport error to its closed reason code, by type. The error itself
/// is dropped here: its text may name a host or carry response bytes.
fn classify(error: &reqwest::Error) -> ReasonCode {
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(err) = cause {
        if err.is::<Unresolved>() {
            return ReasonCode::NameResolutionFailed;
        }
        if is_certificate_error(err) {
            return ReasonCode::CertificateRefused;
        }
        cause = err.source();
    }
    match (error.is_timeout(), error.is_connect()) {
        (true, true) => ReasonCode::ConnectTimeout,
        (true, false) => ReasonCode::TotalTimeout,
        (false, _) => ReasonCode::TransportFailed,
    }
}

/// A rustls certificate error, bare or inside `io::Error`s. tokio-rustls
/// wraps its handshake error in an `io::Error`, hyper-util wraps that in
/// another, and `io::Error::source` skips the wrapped error itself, so the
/// wrappers are unwrapped by hand.
fn is_certificate_error(err: &(dyn std::error::Error + 'static)) -> bool {
    if let Some(e) = err.downcast_ref::<rustls::Error>() {
        return matches!(
            e,
            rustls::Error::InvalidCertificate(_) | rustls::Error::NoCertificatesPresented
        );
    }
    err.downcast_ref::<std::io::Error>()
        .and_then(std::io::Error::get_ref)
        .is_some_and(|inner| is_certificate_error(inner))
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

    // Tests that need an environment variable run as a child of this test
    // binary with the variable set by the parent on the child's command line
    // (`Command::env`), so no variable is ever set in this process and no
    // test races another on the process environment.
    const CHILD_MARK: &str = "FOLIO_OUTBOUND_TEST_CHILD";

    /// True only in a child this binary's own `run_child` spawned: the marker
    /// holds the spawning process's id, and that id must be this process's
    /// parent. A marker left in a developer's shell, or inherited from an
    /// outer test run, names no such parent, so it cannot make a test run its
    /// child body in the parent, where it would assert nothing.
    fn is_child() -> bool {
        std::env::var(CHILD_MARK).is_ok_and(|mark| {
            mark.split_once(':')
                .is_some_and(|(pid, _)| pid == std::os::unix::process::parent_id().to_string())
        })
    }

    /// Rerun test `name` in a child with `vars` set, and require that it ran
    /// (`--exact` on a wrong name runs nothing and exits 0) and passed. The
    /// two trust-root variables are removed first, so the parent's shell
    /// cannot decide a case.
    async fn run_child(name: &str, vars: &[(&str, &str)]) {
        let module = module_path!().split_once("::").expect("a crate prefix").1;
        let mark = format!("{}:{}", std::process::id(), uuid::Uuid::new_v4().simple());
        // Printed (and so visible under --nocapture) to show this process took
        // the parent path.
        println!("spawned-child:{name}");
        let out = tokio::process::Command::new(std::env::current_exe().expect("this binary"))
            .args(["--exact", &format!("{module}::{name}"), "--test-threads=1"])
            .env(CHILD_MARK, &mark)
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR")
            .envs(vars.iter().copied())
            .output()
            .await
            .expect("spawn the child test");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success() && stdout.contains("1 passed"),
            "the child test {name} did not pass
stdout: {stdout}
stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn a_stale_child_marker_does_not_make_a_test_run_as_its_own_child() {
        // Given a marker in the environment that names no spawning parent
        let out = std::process::Command::new(std::env::current_exe().expect("this binary"))
            .args([
                "--exact",
                "live_lists::outbound::tests::r2_ignores_the_proxy_environment",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD_MARK, "1")
            .output()
            .expect("spawn");
        let stdout = String::from_utf8_lossy(&out.stdout);
        // Then the test still took the parent path and spawned a real child
        assert!(out.status.success(), "{stdout}");
        assert!(
            stdout.contains("spawned-child:r2_ignores_the_proxy_environment"),
            "the stale marker made the test its own child: {stdout}"
        );
    }

    /// `der` as a PEM certificate. A public certificate, generated per run.
    fn pem_of(der: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut b64 = String::new();
        for chunk in der.chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
            for i in 0..4 {
                if i <= chunk.len() {
                    b64.push(TABLE[((n >> (18 - 6 * i)) & 63) as usize] as char);
                } else {
                    b64.push('=');
                }
            }
        }
        let lines: Vec<&str> = b64
            .as_bytes()
            .chunks(64)
            .map(|c| std::str::from_utf8(c).expect("ascii"))
            .collect();
        format!(
            "-----BEGIN CERTIFICATE-----
{}
-----END CERTIFICATE-----
",
            lines.join(
                "
"
            )
        )
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

    #[tokio::test]
    async fn r2_refuses_http_and_the_plain_listener_sees_no_connection() {
        // Given a plain listener mapped to the name
        let plain = CountingListener::start().await;
        let pki = Pki::new();
        let client = OutboundClient::for_test(TestNet::new(&pki, &[(GITHUB, plain.addr())]));
        // When an http:// URL is requested
        let result = client
            .fetch_url(
                &url("http://api.github.com/orgs/mnemra/repos"),
                "*/*",
                false,
            )
            .await;
        // Then it fails with the scheme-refusal reason and nothing connected
        assert_eq!(reason(result), Some(ReasonCode::SchemeRefused));
        assert_eq!(plain.connections(), 0);
    }

    #[tokio::test]
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
        let fetched = trusting.fetch(org()).await.expect("trusted root");
        assert_eq!(fetched.body, JSON);
        assert_eq!(fetched.note.status, 200);
    }

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
    async fn r2_ignores_the_proxy_environment() {
        if is_child() {
            // When a client built under the proxy variables fetches
            let rig = rig(Reply::ok(JSON)).await;
            // Then the fetch succeeds
            assert!(rig.client.fetch(org()).await.is_ok());
            assert_eq!(rig.server.requests().len(), 1);
            return;
        }
        // Given the three proxy variables set, for the child, to a listener that records connections
        let proxy = CountingListener::start().await;
        let at = format!("http://{}", proxy.addr());
        run_child(
            "r2_ignores_the_proxy_environment",
            &[
                ("HTTPS_PROXY", &at),
                ("HTTP_PROXY", &at),
                ("ALL_PROXY", &at),
            ],
        )
        .await;
        // Then the proxy recorded nothing
        assert_eq!(proxy.connections(), 0);
    }

    // ---- Warden F-1: no trust root enters through the environment ----

    #[tokio::test]
    async fn r2_the_production_client_refuses_to_build_when_a_trust_root_variable_is_set() {
        use crate::live_lists::harness::LogCapture;
        if is_child() {
            let (log, _guard) = LogCapture::install();
            let set = |k: &str| std::env::var_os(k).is_some();
            if set("SSL_CERT_FILE") || set("SSL_CERT_DIR") {
                // Then the build fails closed, and says which variable, never its value
                assert!(OutboundClient::new().is_err());
                let text = log.text();
                assert!(text.contains("ERROR"), "{text}");
                assert!(
                    text.contains("SSL_CERT_FILE") || text.contains("SSL_CERT_DIR"),
                    "{text}"
                );
                for k in ["SSL_CERT_FILE", "SSL_CERT_DIR"] {
                    if let Some(v) = std::env::var_os(k) {
                        let v = v.to_string_lossy().into_owned();
                        assert!(!text.contains(&v), "{k}'s value leaked: {text}");
                    }
                }
            } else {
                // The positive control: with neither variable it builds
                assert!(OutboundClient::new().is_ok());
            }
            return;
        }
        // Given a real PEM root, in a file and in a directory
        let dir = crate::live_lists::test_support::TempDir::new();
        let pem = dir.path().join("root.pem");
        std::fs::write(&pem, pem_of(&Pki::new().root_der())).expect("write the PEM");
        let (file, directory) = (
            pem.to_str().expect("utf-8"),
            dir.path().to_str().expect("utf-8"),
        );
        let name = "r2_the_production_client_refuses_to_build_when_a_trust_root_variable_is_set";
        run_child(name, &[("SSL_CERT_FILE", file)]).await;
        run_child(name, &[("SSL_CERT_DIR", directory)]).await;
        run_child(name, &[]).await;
    }

    // ---- R-3 ----

    #[tokio::test]
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

    #[tokio::test]
    async fn r3_no_header_value_holds_an_app_secret() {
        const KEYS: [&str; 4] = [
            "ADMIN_PASSWORD",
            "ANTHROPIC_API_KEY",
            "EDGE_AUTH_TOKEN",
            "PAGE_HIT_SALT",
        ];
        if !is_child() {
            // Given four secrets, each a distinct value made for this run, set for the child
            let secrets: Vec<(&str, String)> = KEYS.into_iter().map(|k| (k, unique(k))).collect();
            let vars: Vec<(&str, &str)> = secrets.iter().map(|(k, v)| (*k, v.as_str())).collect();
            run_child("r3_no_header_value_holds_an_app_secret", &vars).await;
            return;
        }
        let secrets: Vec<(&str, String)> = KEYS
            .into_iter()
            .map(|k| (k, std::env::var(k).expect("set by the parent")))
            .collect();
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

    // ---- Cerberus L6: Link is read only for GitHub's targets ----

    #[tokio::test]
    async fn l6_the_link_header_is_read_only_for_the_github_targets() {
        // Given a Link value that is not plain text
        let unreadable = Reply::ok(JSON).with_header("Link", "<https://x/>; rel=\"n\u{e9}xt\"");
        // Then a GitHub target fails the fetch, since pagination would be ambiguous
        let github = rig(unreadable.clone()).await;
        assert_eq!(
            reason(github.client.fetch(org()).await),
            Some(ReasonCode::LinkMalformed)
        );
        // and the feed, which never paginates, ignores it, and carries no link
        let feed = rig(unreadable).await;
        let fetched = feed.client.fetch(Target::Feed).await.expect("the feed");
        assert_eq!(fetched.link, None);
        // and a readable Link on the feed is not carried either
        let feed = rig(Reply::ok(JSON).with_header("Link", "<https://x/>; rel=\"next\"")).await;
        let fetched = feed.client.fetch(Target::Feed).await.expect("the feed");
        assert_eq!(fetched.link, None);
    }

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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
            let fetched = rig.client.fetch(org()).await.expect("accepted");
            assert_eq!(fetched.body, JSON, "{headers:?}");
        }
    }

    // ---- R-4 ----

    // R-4's two-sided timing cases run on the real clock (Peter, 2026-11-24:
    // "(c) Real clock (Recommended)"). Paused tokio time against a real socket
    // is not deterministic: the clock jumps while I/O is in flight (measured,
    // 50 runs each, 12 to 26 early timeouts). Each case asserts the outcome and
    // an elapsed window, so a wrong constant (2 s or 10 s for the 5 s connect
    // limit, say) fails one side. Margins are generous on the late side for a
    // loaded CI runner, and tight enough to exclude the neighbouring constants.

    /// Connect, success side. The server completes the handshake after 3 s,
    /// which is inside the 5 s limit. Must succeed, and have taken at least
    /// 3 s (the delay was real) and under 5 s (a 5 s timer never fired). A
    /// connect limit of 2 s fails the outcome; one of 10 s passes this side
    /// and fails the next test.
    #[tokio::test]
    async fn r4_a_connect_that_completes_at_3s_succeeds() {
        let pki = Pki::new();
        let server = TlsServer::start_slow_handshake(
            pki.issue(&[GITHUB]),
            Duration::from_secs(3),
            |_, _| Reply::ok(JSON),
        )
        .await;
        let client = OutboundClient::for_test(TestNet::new(&pki, &[(GITHUB, server.addr())]));
        let began = Instant::now();
        let result = client.fetch(org()).await;
        let took = began.elapsed();
        assert_eq!(reason(result), None);
        assert!(
            took >= Duration::from_secs(3),
            "finished early, at {took:?}"
        );
        assert!(took < CONNECT_TIMEOUT, "finished late, at {took:?}");
    }

    /// Connect, failure side. The server never completes the handshake. Must
    /// fail with `ConnectTimeout`, no earlier than 5 s (a 2 s limit fails) and
    /// before 7 s (a 10 s limit fails; 2 s of slack for a loaded runner). The
    /// server saw no request.
    #[tokio::test]
    async fn r4_a_connect_that_never_completes_fails_between_5s_and_7s() {
        let pki = Pki::new();
        let server = TlsServer::start_slow_handshake(
            pki.issue(&[GITHUB]),
            Duration::from_secs(3600),
            |_, _| Reply::ok(JSON),
        )
        .await;
        let client = OutboundClient::for_test(TestNet::new(&pki, &[(GITHUB, server.addr())]));
        let began = Instant::now();
        // The outer timeout turns a deleted `.connect_timeout` into a failure with
        // a message, not a CI job that hangs.
        let result = tokio::time::timeout(
            CONNECT_TIMEOUT + Duration::from_secs(5),
            client.fetch(org()),
        )
        .await
        .expect("the connect outlived its limit: the connect timeout is gone");
        let took = began.elapsed();
        assert_eq!(reason(result), Some(ReasonCode::ConnectTimeout));
        assert!(took >= CONNECT_TIMEOUT, "failed early, at {took:?}");
        assert!(
            took < CONNECT_TIMEOUT + Duration::from_secs(2),
            "failed late, at {took:?}"
        );
        assert_eq!(server.requests().len(), 0);
    }

    /// Total, success side. The response head arrives after 15 s, inside the
    /// 20 s limit. Must succeed, taking at least 15 s and under 20 s (a total
    /// limit of 10 s fails the outcome).
    #[tokio::test]
    async fn r4_a_response_that_completes_at_15s_succeeds() {
        let mut reply = Reply::ok(JSON);
        reply.head_delay = Duration::from_secs(15);
        let rig = rig(reply).await;
        let began = Instant::now();
        let result = rig.client.fetch(org()).await;
        let took = began.elapsed();
        assert_eq!(reason(result), None);
        assert!(
            took >= Duration::from_secs(15),
            "finished early, at {took:?}"
        );
        assert!(took < TOTAL_TIMEOUT, "finished late, at {took:?}");
    }

    /// Total, failure side. The body stalls after 3 bytes. Must fail with
    /// `TotalTimeout`, no earlier than 20 s (a 10 s limit fails) and before
    /// 23 s (a 30 s limit fails; 3 s of slack for a loaded runner). One
    /// request reached the server.
    #[tokio::test]
    async fn r4_a_body_that_stalls_fails_between_20s_and_23s() {
        let mut reply = Reply::ok(JSON);
        reply.stall_after = Some(3);
        let rig = rig(reply).await;
        let began = Instant::now();
        // The outer timeout turns a deleted `.timeout` into a failure with a
        // message, not a CI job that hangs.
        let result = tokio::time::timeout(
            TOTAL_TIMEOUT + Duration::from_secs(5),
            rig.client.fetch(org()),
        )
        .await
        .expect("the fetch outlived its limit: the total timeout is gone");
        let took = began.elapsed();
        assert_eq!(reason(result), Some(ReasonCode::TotalTimeout));
        assert!(took >= TOTAL_TIMEOUT, "failed early, at {took:?}");
        assert!(
            took < TOTAL_TIMEOUT + Duration::from_secs(3),
            "failed late, at {took:?}"
        );
        assert_eq!(rig.server.requests().len(), 1);
    }

    #[tokio::test]
    async fn r4_a_body_of_exactly_2_mib_is_returned_and_one_byte_more_fails() {
        // Given a body of exactly the cap
        let rig_at = rig(Reply::ok(vec![b'x'; MAX_BODY_BYTES])).await;
        let fetched = rig_at.client.fetch(org()).await.expect("at the cap");
        assert_eq!(fetched.body.len(), MAX_BODY_BYTES);
        // And one of cap + 1 bytes
        let rig_over = rig(Reply::ok(vec![b'x'; MAX_BODY_BYTES + 1])).await;
        assert_eq!(
            reason(rig_over.client.fetch(org()).await),
            Some(ReasonCode::BodyOverCap)
        );
        assert_eq!(rig_over.server.requests().len(), 1);
    }

    #[tokio::test]
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

    #[tokio::test]
    async fn r4_a_2xx_other_than_200_is_accepted() {
        let rig = rig(Reply::status(204)).await;
        let fetched = rig.client.fetch(org()).await.expect("204");
        assert_eq!(fetched.note.status, 204);
    }

    // ---- R-30: the rate-limit field ----

    #[tokio::test]
    async fn r30_the_ratelimit_header_is_noted_only_when_it_is_an_integer() {
        for (value, want) in [
            ("59", Some(59)),
            ("0", Some(0)),
            ("many", None),
            ("5.5", None),
            ("", None),
        ] {
            let rig = rig(Reply::ok(JSON).with_header("x-ratelimit-remaining", value)).await;
            let fetched = rig.client.fetch(org()).await.expect("2xx");
            assert_eq!(fetched.note.ratelimit_remaining, want, "{value:?}");
        }
        let rig = rig(Reply::ok(JSON)).await;
        let fetched = rig.client.fetch(org()).await.expect("2xx");
        assert_eq!(fetched.note.ratelimit_remaining, None);
    }

    #[tokio::test]
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

    #[tokio::test]
    async fn r5_a_name_outside_the_map_fails_with_a_resolution_error() {
        // Given a server and a map that does not name `unmapped.example`
        let rig = rig(Reply::ok(JSON)).await;
        // When that name is requested
        let result = rig
            .client
            .fetch_url(&url("https://unmapped.example/"), "*/*", false)
            .await;
        // Then resolution fails and nothing connected
        assert_eq!(reason(result), Some(ReasonCode::NameResolutionFailed));
        assert_eq!(rig.server.connections(), 0);
    }

    #[tokio::test]
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
