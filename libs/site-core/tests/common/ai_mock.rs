//! Test helpers for the AI-handler integration tests in spec #572 Task 4.
//!
//! Glitch's red-phase test home. Three concerns live here so the per-test
//! file stays focused on Given/When/Then bodies:
//!
//! 1. `ai_test_app_with_mock(base_url)` — builds an `AppState` whose
//!    `rig_client` is a real `anthropic::Client` pointed at a mockito server
//!    URL. Uses the same in-memory SQLite + migrations + seed pattern as the
//!    existing `test_app` helper. Per Sage Q1: `Client::builder().base_url(...)`
//!    is the supported public surface; the builder normalizes the URL via
//!    `normalize_anthropic_base_url` which strips `/v1/messages`, `/messages`,
//!    or `/v1` suffixes — pass mockito's base URL clean.
//!
//! 2. `parse_sse_frames(body)` — split a raw SSE response body into typed
//!    `(event_name, data)` tuples for assertion. Frame separator is `\n\n`
//!    per the SSE spec. The `[DONE]` sentinel is parsed as a frame with
//!    `event_name == None` and `data == "[DONE]"`.
//!
//! 3. `LogBuf` + `capture_logs` — thread-local tracing capture mirroring the
//!    pattern in `libs/site-core/db/config.rs` (lines 182-227). Copied (not
//!    imported) per the workspace pattern: each test binary owns its capture
//!    infrastructure so parallel tests don't share global tracing state.
//!    Used by R17/R23 sanitization tests.

use rusqlite::Connection;
use site_core::state::{AppState, DbState};
use std::io;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;

// ===========================================================================
// AppState fixture with mockito-pointed rig Client
// ===========================================================================

/// Build a test app whose `rig_client` is a real `anthropic::Client` pointed
/// at `base_url`. The intent is `mockito::Server::url()`; the URL is passed
/// through rig's `normalize_anthropic_base_url` so suffixes are stripped if
/// present.
///
/// Uses an in-memory SQLite seeded with all migrations (including 005 →
/// `site_config` rows for `ai.model_id` and `ai.max_tokens`).
pub fn ai_test_app_with_mock(base_url: &str) -> axum_test::TestServer {
    let conn = Connection::open_in_memory().expect("in-memory connection must open");
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size=-64000;
         PRAGMA temp_store=memory;",
    )
    .expect("PRAGMA setup must succeed");
    site_core::db::migrate(&conn).expect("migrations must succeed");
    site_core::db::seed::seed_test_data(&conn).expect("seed must succeed");

    let client = rig_core::providers::anthropic::Client::builder()
        .api_key("test-key-not-used-mockito-intercepts")
        .base_url(base_url)
        .build()
        .expect("rig anthropic Client must build with mockito base_url");

    let password_hash = site_core::auth::hash_password("testpass").expect("test password hash");
    let state: DbState = Arc::new(AppState {
        db: Arc::new(Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client: Some(client),
        trusted_ip_header: None,
        page_hit_salt: "test-salt".to_string(),
    });

    let app = axum::Router::new()
        .merge(site_core::routes::ai::routes())
        .with_state(state);

    axum_test::TestServer::new(app)
}

/// Variant of `ai_test_app_with_mock` returning the `DbState` so a test can
/// hand-edit the `site_config` table (e.g., override `ai.model_id` to verify
/// the chat handler reads from config, not a literal).
pub fn ai_test_app_with_mock_and_state(base_url: &str) -> (axum_test::TestServer, DbState) {
    let conn = Connection::open_in_memory().expect("in-memory connection must open");
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size=-64000;
         PRAGMA temp_store=memory;",
    )
    .expect("PRAGMA setup must succeed");
    site_core::db::migrate(&conn).expect("migrations must succeed");
    site_core::db::seed::seed_test_data(&conn).expect("seed must succeed");

    let client = rig_core::providers::anthropic::Client::builder()
        .api_key("test-key-not-used-mockito-intercepts")
        .base_url(base_url)
        .build()
        .expect("rig anthropic Client must build");

    let password_hash = site_core::auth::hash_password("testpass").expect("test password hash");
    let state: DbState = Arc::new(AppState {
        db: Arc::new(Mutex::new(conn)),
        admin_password_hash: password_hash,
        rig_client: Some(client),
        trusted_ip_header: None,
        page_hit_salt: "test-salt".to_string(),
    });

    let app = axum::Router::new()
        .merge(site_core::routes::ai::routes())
        .with_state(state.clone());

    let server = axum_test::TestServer::new(app);
    (server, state)
}

// ===========================================================================
// SSE response parsing
// ===========================================================================

/// A single SSE frame: `(event_name, data)`. `event_name` is `None` when the
/// frame has only a `data:` line (the default-event case, used by folio for
/// content + `[DONE]` sentinels).
pub type SseFrame = (Option<String>, String);

/// Parse an SSE response body (bytes) into a vec of typed frames.
///
/// SSE frame separator is `\n\n`. Each frame may have any number of
/// `field: value` lines; this helper recognizes `event` and `data` and
/// concatenates multi-line `data:` per the SSE spec.
///
/// Empty trailing frames (from a final `\n\n`) are dropped. Lines starting
/// with `:` (comments) are ignored. The keep-alive `:` lines emitted by
/// axum's `KeepAlive` default are NOT expected in tests with fast mocks.
pub fn parse_sse_frames(body: &[u8]) -> Vec<SseFrame> {
    let text = std::str::from_utf8(body).expect("SSE body must be valid UTF-8");
    let mut frames = Vec::new();

    for raw_frame in text.split("\n\n") {
        if raw_frame.trim().is_empty() {
            continue;
        }
        let mut event_name: Option<String> = None;
        let mut data_parts: Vec<&str> = Vec::new();
        for line in raw_frame.lines() {
            if line.is_empty() || line.starts_with(':') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("event:") {
                event_name = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("data:") {
                data_parts.push(rest.trim_start());
            }
            // ignore id:, retry: etc.
        }
        if event_name.is_some() || !data_parts.is_empty() {
            frames.push((event_name, data_parts.join("\n")));
        }
    }

    frames
}

/// Convenience: count frames whose event name matches.
pub fn count_event(frames: &[SseFrame], event_name: &str) -> usize {
    frames
        .iter()
        .filter(|(name, _)| name.as_deref() == Some(event_name))
        .count()
}

/// Convenience: find the first frame matching event name.
pub fn first_event<'a>(frames: &'a [SseFrame], event_name: &str) -> Option<&'a SseFrame> {
    frames
        .iter()
        .find(|(name, _)| name.as_deref() == Some(event_name))
}

/// Convenience: assert the last frame is the `[DONE]` sentinel (event-less).
pub fn assert_terminates_with_done(frames: &[SseFrame]) {
    let last = frames
        .last()
        .expect("SSE stream must have at least one frame");
    assert!(
        last.0.is_none() && last.1 == "[DONE]",
        "expected stream to terminate with [DONE], got: {:?}",
        last
    );
}

// ===========================================================================
// Log capture (mirrors db/config.rs pattern at lines 182-227)
// ===========================================================================

#[derive(Clone, Default)]
pub struct LogBuf(Arc<Mutex<Vec<u8>>>);

impl LogBuf {
    pub fn captured(&self) -> String {
        let bytes = self.0.lock().expect("log buffer mutex poisoned").clone();
        String::from_utf8(bytes).expect("log output must be valid UTF-8")
    }

    pub fn count(&self, needle: &str) -> usize {
        self.captured().matches(needle).count()
    }
}

impl io::Write for LogBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut inner = self
            .0
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "log buffer mutex poisoned"))?;
        inner.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogBuf {
    type Writer = LogBuf;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Run `f` with a thread-local tracing subscriber that captures all events
/// into a `LogBuf`. Returns `(f's result, captured LogBuf)`.
pub fn capture_logs_async<F, Fut, R>(f: F) -> (R, LogBuf)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = R>,
{
    let buf = LogBuf::default();
    let layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(buf.clone())
        .with_ansi(false)
        .with_target(false)
        .without_time();
    let subscriber = tracing_subscriber::Registry::default().with(layer);
    // Use a fresh tokio runtime so the subscriber outlives the futures.
    // We rely on with_default for sync wrapping; for async tests, the
    // subscriber must be set BEFORE the future is polled. Test functions
    // use `#[tokio::test]` so the caller pattern is:
    //   let (result, buf) = capture_logs_async(|| async { ... }).await; — INVALID
    // Use `capture_logs_sync_block` for sync; for async tests, set the
    // subscriber via `tracing::subscriber::set_default` (returns a guard)
    // and run the future on the existing tokio reactor.
    let _guard = tracing::subscriber::set_default(subscriber);
    let result = futures::executor::block_on(f());
    (result, buf)
}

/// Synchronous log capture. Sets a thread-local default subscriber for the
/// duration of `f`.
pub fn capture_logs_sync<R>(f: impl FnOnce() -> R) -> (R, LogBuf) {
    let buf = LogBuf::default();
    let layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(buf.clone())
        .with_ansi(false)
        .with_target(false)
        .without_time();
    let subscriber = tracing_subscriber::Registry::default().with(layer);
    let result = tracing::subscriber::with_default(subscriber, f);
    (result, buf)
}

/// Set a thread-local default tracing subscriber for the rest of the current
/// scope and return both a `LogBuf` (read captured output) and a
/// `DefaultGuard` (drop to restore the previous subscriber). For async tests
/// that want capture across `.await` points on a single task.
pub fn install_log_capture() -> (LogBuf, tracing::subscriber::DefaultGuard) {
    let buf = LogBuf::default();
    let layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(buf.clone())
        .with_ansi(false)
        .with_target(false)
        .without_time();
    let subscriber = tracing_subscriber::Registry::default().with(layer);
    let guard = tracing::subscriber::set_default(subscriber);
    (buf, guard)
}

// ===========================================================================
// SSE fixture builders for mockito responses
// ===========================================================================

/// Build a minimal Anthropic SSE response body that streams two text deltas
/// then a terminal `message_delta` with the supplied `stop_reason`, followed
/// by `message_stop`.
///
/// Used as the body in `mockito::Mock::with_body(...)`.
///
/// Forge T4-impl assumption: the chat handler reads
/// `content_block_delta { type: "text_delta", text: "..." }` frames and
/// emits the `.text` field as default-event SSE `data:` lines.
pub fn anthropic_sse_response(text_deltas: &[&str], stop_reason: &str) -> String {
    let mut out = String::new();

    // message_start (minimal)
    out.push_str("event: message_start\n");
    out.push_str(
        r#"data: {"type":"message_start","message":{"id":"msg_test","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-6","stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":1,"output_tokens":1}}}"#,
    );
    out.push_str("\n\n");

    // content_block_start
    out.push_str("event: content_block_start\n");
    out.push_str(
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
    );
    out.push_str("\n\n");

    // content_block_delta * N
    for delta in text_deltas {
        out.push_str("event: content_block_delta\n");
        let escaped = delta.replace('\\', "\\\\").replace('"', "\\\"");
        out.push_str(&format!(
            r#"data: {{"type":"content_block_delta","index":0,"delta":{{"type":"text_delta","text":"{}"}}}}"#,
            escaped
        ));
        out.push_str("\n\n");
    }

    // content_block_stop
    out.push_str("event: content_block_stop\n");
    out.push_str(r#"data: {"type":"content_block_stop","index":0}"#);
    out.push_str("\n\n");

    // message_delta — carries stop_reason
    out.push_str("event: message_delta\n");
    out.push_str(&format!(
        r#"data: {{"type":"message_delta","delta":{{"stop_reason":"{}","stop_sequence":null}},"usage":{{"output_tokens":2}}}}"#,
        stop_reason
    ));
    out.push_str("\n\n");

    // message_stop
    out.push_str("event: message_stop\n");
    out.push_str(r#"data: {"type":"message_stop"}"#);
    out.push_str("\n\n");

    out
}

/// Build an Anthropic SSE response where the stream emits `event: error`
/// with the canonical Anthropic error-event payload. Used to drive
/// Scenario 17.
pub fn anthropic_sse_error_response(kind: &str, message: &str) -> String {
    let mut out = String::new();
    out.push_str("event: error\n");
    let escaped_message = message.replace('\\', "\\\\").replace('"', "\\\"");
    out.push_str(&format!(
        r#"data: {{"type":"error","error":{{"type":"{}","message":"{}"}}}}"#,
        kind, escaped_message
    ));
    out.push_str("\n\n");
    out
}

/// Build a buffered JSON response body for the fit handler. Matches the
/// Anthropic Messages API non-streaming shape. The `text_content` argument
/// should be a string suitable for the model's `content[0].text` slot —
/// typically a serialized `FitVerdict`.
pub fn anthropic_messages_response(text_content: &str, stop_reason: &str) -> String {
    let escaped = text_content.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"{{"id":"msg_test","type":"message","role":"assistant","content":[{{"type":"text","text":"{}"}}],"model":"claude-sonnet-4-6","stop_reason":"{}","stop_sequence":null,"usage":{{"input_tokens":1,"output_tokens":1}}}}"#,
        escaped, stop_reason
    )
}

/// Canonical valid `FitVerdict` JSON string for fit handler happy-path
/// tests. Matches the `FitVerdict` shape in `libs/site-core/ai/types.rs`.
pub fn valid_fit_verdict_json() -> &'static str {
    r#"{"verdict":"strong-fit","headline":"Strong match","opening":"Your background aligns well","gaps":[],"transfers":[{"skill":"systems thinking","relevance":"high"}],"recommendation":"Pursue this role"}"#
}
