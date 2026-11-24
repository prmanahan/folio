//! In-process harness: log capture, a per-test environment, and a started
//! server driven without a socket (R-5: "acceptance cases that need a running
//! server with fake sources drive the start function in-process").
//!
//! Test-only. The start function takes its clock, sources and store through
//! `StartInputs`, the inputs production fills, so these are doubles for that
//! seam and not a second way to construct the product.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::http::{HeaderName, HeaderValue};
use rusqlite::Connection;
use time::OffsetDateTime;

use super::ListName;
use super::refresher::Sources;
use super::start::{StartError, StartInputs, start};
use super::test_support::{FakeClock, T0, TempDir, projects_doc};
use crate::config::Config;

/// Collects what `tracing` writes while it is installed. Thread-local, so a
/// test that reads it runs on a current-thread runtime (`#[tokio::test]`),
/// where every spawned task shares the test's thread.
#[derive(Clone, Default)]
pub(crate) struct LogCapture(Arc<Mutex<Vec<u8>>>);

impl LogCapture {
    pub(crate) fn install() -> (Self, tracing::subscriber::DefaultGuard) {
        let capture = Self::default();
        let writer = capture.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .without_time()
            .with_writer(move || writer.clone())
            .finish();
        (capture, tracing::subscriber::set_default(subscriber))
    }

    pub(crate) fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().expect("log lock")).into_owned()
    }

    pub(crate) fn lines_containing(&self, needle: &str) -> Vec<String> {
        self.text()
            .lines()
            .filter(|l| l.contains(needle))
            .map(str::to_owned)
            .collect()
    }

    /// The one line holding `needle`; fails when there are none or several.
    pub(crate) fn one_line(&self, needle: &str) -> String {
        let mut lines = self.lines_containing(needle);
        assert_eq!(lines.len(), 1, "want one {needle:?} line, got {lines:?}");
        lines.remove(0)
    }
}

impl std::io::Write for LogCapture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("log lock").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Files and settings for one start: a static dir holding an `index.html`
/// with a CSP meta tag and a valid `projects.json` (the refusal stays until
/// T11), and a database file. Per-test, removed on drop.
pub(crate) struct Env {
    dir: TempDir,
    pub(crate) clock: Arc<FakeClock>,
    edge_token: String,
    admin_password: String,
}

impl Env {
    pub(crate) fn new() -> Self {
        let dir = TempDir::new();
        let static_dir = dir.path().join("static");
        std::fs::create_dir_all(&static_dir).expect("create static dir");
        std::fs::write(
            static_dir.join("index.html"),
            r#"<!doctype html><meta http-equiv="content-security-policy" content="default-src 'self'">"#,
        )
        .expect("write index.html");
        std::fs::write(static_dir.join("projects.json"), projects_doc(T0))
            .expect("write projects.json");
        // Generated per run: no credential literal in the source.
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        Self {
            dir,
            clock: FakeClock::at(T0),
            edge_token: format!("folio_edge_{nonce}{nonce}"),
            admin_password: format!("test-admin-{nonce}"),
        }
    }

    pub(crate) fn static_dir(&self) -> PathBuf {
        self.dir.path().join("static")
    }

    pub(crate) fn projects_json_path(&self) -> PathBuf {
        self.static_dir().join("projects.json")
    }

    pub(crate) fn db_path(&self) -> PathBuf {
        self.dir.path().join("site.db")
    }

    fn db_url(&self) -> String {
        self.db_path().to_str().expect("utf-8 path").to_string()
    }

    /// A migrated connection to the database file, for seeding and reading
    /// back. Opening it before a start is how a test seeds a row.
    pub(crate) fn db(&self) -> Connection {
        crate::db::connect(&self.db_url()).expect("open db")
    }

    pub(crate) fn seed(&self, list: ListName, document: &str) {
        self.db()
            .execute(
                "INSERT INTO live_lists (name, document) VALUES (?1, ?2) \
                 ON CONFLICT(name) DO UPDATE SET document = excluded.document",
                rusqlite::params![list.as_str(), document],
            )
            .expect("seed row");
    }

    pub(crate) fn stored(&self, list: ListName) -> Option<String> {
        self.db()
            .query_row(
                "SELECT document FROM live_lists WHERE name = ?1",
                [list.as_str()],
                |r| r.get(0),
            )
            .ok()
    }

    pub(crate) fn config(&self) -> Config {
        Config {
            port: 0,
            database_url: self.db_url(),
            admin_password: self.admin_password.clone(),
            anthropic_api_key: None,
            trusted_ip_header: None,
            static_dir: self.static_dir().to_str().expect("utf-8 path").to_string(),
            page_hit_salt: "test-salt".to_string(),
            edge_auth_token: self.edge_token.clone(),
        }
    }

    pub(crate) fn inputs(&self, switch: Option<&str>, sources: Sources) -> StartInputs {
        self.inputs_raw(switch.map(OsString::from), sources)
    }

    pub(crate) fn inputs_raw(&self, switch: Option<OsString>, sources: Sources) -> StartInputs {
        StartInputs {
            config: self.config(),
            cors_origin: Some("http://localhost:3000".to_string()),
            switch,
            clock: self.clock.clone(),
            sources,
            store: None,
        }
    }

    fn edge_header(&self) -> (HeaderName, HeaderValue) {
        (
            HeaderName::from_static("x-folio-edge-auth"),
            HeaderValue::from_str(&self.edge_token).expect("header value"),
        )
    }
}

/// A start that reached step 7. The router is driven through axum-test's
/// in-memory transport, so no socket stands between a request and the
/// handlers and tokio's paused clock is never advanced by I/O.
pub(crate) struct Running {
    pub(crate) server: axum_test::TestServer,
    /// The wall clock's instant when the serve step was entered.
    pub(crate) serving_began_at: OffsetDateTime,
    /// How far tokio's clock moved between the call and the serve step. On a
    /// paused clock that only happens when the runtime sat idle on a timer,
    /// which is what an awaited refresh against a silent source would cause.
    pub(crate) tokio_time_before_serving: std::time::Duration,
    handle: tokio::task::JoinHandle<Result<(), StartError>>,
}

impl Running {
    /// GET with the edge secret, so the origin lock lets it through.
    pub(crate) async fn get(&self, env: &Env, path: &str) -> axum_test::TestResponse {
        let (name, value) = env.edge_header();
        self.server.get(path).add_header(name, value).await
    }

    /// GET `path`, require `status`, and parse the JSON body.
    pub(crate) async fn get_json(
        &self,
        env: &Env,
        path: &str,
        status: axum::http::StatusCode,
    ) -> serde_json::Value {
        let response = self.get(env, path).await;
        response.assert_status(status);
        serde_json::from_str(&response.text()).expect("a JSON body")
    }

    /// Whether the start task is still running (serving).
    pub(crate) fn is_serving(&self) -> bool {
        !self.handle.is_finished()
    }
}

/// Run `start` on a task, with a serve step that hands its router to the
/// test and then never returns. Returns the `StartError` when `start` ends
/// before serving.
pub(crate) async fn try_launch(inputs: StartInputs) -> Result<Running, StartError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let clock = inputs.clock.clone();
    let called_at = tokio::time::Instant::now();
    let handle = tokio::spawn(start(inputs, move |router| async move {
        // A send can only fail if the test gave up waiting.
        let _ = tx.send((router, clock.now(), called_at.elapsed()));
        std::future::pending::<std::io::Result<()>>().await
    }));
    match rx.await {
        Ok((router, serving_began_at, tokio_time_before_serving)) => Ok(Running {
            server: axum_test::TestServer::new(router),
            serving_began_at,
            tokio_time_before_serving,
            handle,
        }),
        Err(_) => match handle.await.expect("the start task ran to its end") {
            Err(e) => Err(e),
            Ok(()) => panic!("start returned Ok without serving"),
        },
    }
}

/// Let every task that can run without the clock moving run to its next
/// wait. A fixed count of yields, not a sleep: nothing here waits on time.
pub(crate) async fn settle() {
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
}

pub(crate) async fn launch(inputs: StartInputs) -> Running {
    try_launch(inputs)
        .await
        .unwrap_or_else(|e| panic!("start failed before serving: {e:?}"))
}
