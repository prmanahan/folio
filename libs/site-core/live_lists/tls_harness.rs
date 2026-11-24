//! The TLS test harness (R-5, builder-in-code): a certificate authority and
//! a local TLS server, both made in memory per test.
//!
//! Test-only, and it holds no key or certificate file: `rcgen` generates
//! every key at run time, and nothing is written to disk. The server speaks
//! HTTP/1.1 only, so `Host` is a header on the wire (R-3). Only three things
//! differ from production in a test: the trust root, the name-to-address map
//! and the clock. The client under test is the production client, built from
//! the same policy (`OutboundClient::for_test`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    Issuer, KeyPair, KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

/// A certificate authority that lives only in memory.
pub(crate) struct Pki {
    params: CertificateParams,
    key: KeyPair,
    root_der: Vec<u8>,
}

/// A server certificate and its key.
pub(crate) struct Leaf {
    cert: CertificateDer<'static>,
    key: Vec<u8>,
}

impl Pki {
    pub(crate) fn new() -> Self {
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let mut name = DistinguishedName::new();
        name.push(DnType::CommonName, "folio test root");
        params.distinguished_name = name;
        let key = KeyPair::generate().expect("generate the root key");
        let root = params.self_signed(&key).expect("self-sign the root");
        Self {
            params,
            key,
            root_der: root.der().to_vec(),
        }
    }

    /// The root certificate, DER, for the client's trust store.
    pub(crate) fn root_der(&self) -> Vec<u8> {
        self.root_der.clone()
    }

    /// A server certificate naming exactly `names`, signed by this root.
    pub(crate) fn issue(&self, names: &[&str]) -> Leaf {
        let mut params =
            CertificateParams::new(names.iter().map(|n| n.to_string()).collect::<Vec<_>>())
                .expect("subject alt names");
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        let key = KeyPair::generate().expect("generate a leaf key");
        let issuer = Issuer::from_params(&self.params, &self.key);
        let cert = params.signed_by(&key, &issuer).expect("sign the leaf");
        Leaf {
            cert: cert.der().clone(),
            key: key.serialize_der(),
        }
    }
}

/// One request as the server received it.
#[derive(Clone, Debug)]
pub(crate) struct Request {
    pub(crate) target: String,
    /// Header names lower-cased, values as sent, in wire order.
    pub(crate) headers: Vec<(String, String)>,
}

impl Request {
    pub(crate) fn header_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.headers.iter().map(|(n, _)| n.clone()).collect();
        names.sort();
        names
    }

    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// What the server does with one request.
#[derive(Clone)]
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
    /// Slept on tokio's clock after the TLS handshake and the request head
    /// are read, before the response head is written.
    pub(crate) head_delay: Duration,
    /// Slept after the response head, before the body is written.
    pub(crate) body_delay: Duration,
    /// Write the head, then this many body bytes, then never finish.
    pub(crate) stall_after: Option<usize>,
}

impl Reply {
    pub(crate) fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: body.into(),
            head_delay: Duration::ZERO,
            body_delay: Duration::ZERO,
            stall_after: None,
        }
    }

    pub(crate) fn status(status: u16) -> Self {
        Self {
            status,
            ..Self::ok(Vec::new())
        }
    }

    pub(crate) fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

type Script = Arc<dyn Fn(&Request, usize) -> Reply + Send + Sync>;

/// A local TLS server on 127.0.0.1. Dropped, it stops.
pub(crate) struct TlsServer {
    addr: SocketAddr,
    requests: Arc<Mutex<Vec<Request>>>,
    connections: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl TlsServer {
    /// Serve `script(request, index)` for each request, presenting `leaf`.
    pub(crate) async fn start(
        leaf: Leaf,
        script: impl Fn(&Request, usize) -> Reply + Send + Sync + 'static,
    ) -> Self {
        Self::start_slow_handshake(leaf, Duration::ZERO, script).await
    }

    /// As [`TlsServer::start`], sleeping `handshake_delay` on tokio's clock
    /// after each TCP accept, before the TLS handshake (R-4's connect cases).
    pub(crate) async fn start_slow_handshake(
        leaf: Leaf,
        handshake_delay: Duration,
        script: impl Fn(&Request, usize) -> Reply + Send + Sync + 'static,
    ) -> Self {
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let mut config = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("protocol versions")
            .with_no_client_auth()
            .with_single_cert(
                vec![leaf.cert],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf.key)),
            )
            .expect("server certificate");
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let connections = Arc::new(AtomicUsize::new(0));
        let script: Script = Arc::new(script);
        let task = tokio::spawn({
            let (requests, connections) = (requests.clone(), connections.clone());
            async move {
                loop {
                    let Ok((tcp, _)) = listener.accept().await else {
                        return;
                    };
                    connections.fetch_add(1, Ordering::SeqCst);
                    tokio::spawn(serve_connection(
                        tcp,
                        handshake_delay,
                        acceptor.clone(),
                        script.clone(),
                        requests.clone(),
                    ));
                }
            }
        });
        Self {
            addr,
            requests,
            connections,
            task,
        }
    }

    /// Answer every request with `reply`.
    pub(crate) async fn always(leaf: Leaf, reply: Reply) -> Self {
        Self::start(leaf, move |_, _| reply.clone()).await
    }

    pub(crate) fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Every request received so far, in arrival order.
    pub(crate) fn requests(&self) -> Vec<Request> {
        self.requests.lock().expect("requests lock").clone()
    }

    /// TCP connections accepted, requests or not.
    pub(crate) fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }
}

impl Drop for TlsServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve_connection(
    tcp: tokio::net::TcpStream,
    handshake_delay: Duration,
    acceptor: TlsAcceptor,
    script: Script,
    requests: Arc<Mutex<Vec<Request>>>,
) {
    if !handshake_delay.is_zero() {
        tokio::time::sleep(handshake_delay).await;
    }
    let Ok(mut tls) = acceptor.accept(tcp).await else {
        return;
    };
    let mut head = Vec::new();
    let mut chunk = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        match tls.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&chunk[..n]),
        }
    }
    let request = parse_head(&head);
    let index = {
        let mut seen = requests.lock().expect("requests lock");
        seen.push(request.clone());
        seen.len() - 1
    };
    let reply = script(&request, index);
    if !reply.head_delay.is_zero() {
        tokio::time::sleep(reply.head_delay).await;
    }
    let mut out = format!(
        "HTTP/1.1 {} X\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.body.len()
    );
    for (name, value) in &reply.headers {
        out.push_str(&format!("{name}: {value}\r\n"));
    }
    out.push_str("\r\n");
    if tls.write_all(out.as_bytes()).await.is_err() {
        return;
    }
    if !reply.body_delay.is_zero() {
        tokio::time::sleep(reply.body_delay).await;
    }
    if let Some(n) = reply.stall_after {
        let _ = tls.write_all(&reply.body[..n.min(reply.body.len())]).await;
        let _ = tls.flush().await;
        std::future::pending::<()>().await;
    }
    let _ = tls.write_all(&reply.body).await;
    let _ = tls.shutdown().await;
}

fn parse_head(head: &[u8]) -> Request {
    let text = String::from_utf8_lossy(head);
    let mut lines = text.split("\r\n");
    let target = lines
        .next()
        .and_then(|l| l.split(' ').nth(1))
        .unwrap_or_default()
        .to_string();
    let headers = lines
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    Request { target, headers }
}

/// A plain TCP listener that only counts connections and serves nothing
/// (R-2's `http://` case and the proxy listener).
pub(crate) struct CountingListener {
    addr: SocketAddr,
    connections: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}

impl CountingListener {
    pub(crate) async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let connections = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn({
            let connections = connections.clone();
            async move {
                // Held open and never read, so a client that did connect would
                // wait on it rather than see a refusal.
                let mut held = Vec::new();
                while let Ok((tcp, _)) = listener.accept().await {
                    connections.fetch_add(1, Ordering::SeqCst);
                    held.push(tcp);
                }
            }
        });
        Self {
            addr,
            connections,
            task,
        }
    }

    pub(crate) fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub(crate) fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }
}

impl Drop for CountingListener {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// The trust root and the name map a test client is given, and nothing else.
pub(crate) struct TestNet {
    pub(crate) roots: Vec<Vec<u8>>,
    pub(crate) names: HashMap<String, SocketAddr>,
}

impl TestNet {
    pub(crate) fn new(root: &Pki, names: &[(&str, SocketAddr)]) -> Self {
        Self {
            roots: vec![root.root_der()],
            names: names.iter().map(|(n, a)| (n.to_string(), *a)).collect(),
        }
    }
}
