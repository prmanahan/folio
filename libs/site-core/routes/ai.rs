use std::convert::Infallible;
use std::net::SocketAddr;

use axum::{
    Json, Router,
    extract::{ConnectInfo, DefaultBodyLimit, State},
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
    routing::post,
};
use futures::stream::Stream;
use rig_core::client::CompletionClient;
use rig_core::completion::Prompt;
use tracing;

use crate::ai::anthropic_stream::stream_chat;
use crate::ai::context::{build_fit_prompt, build_system_prompt};
use crate::ai::rate_limit::check_rate_limit;
use crate::ai::stop_reason::{StopReason, StopReasonCapture, from_anthropic_str};
use crate::ai::types::{ChatRequest, FitRequest, FitVerdict};
use crate::db::config::{get_max_tokens, get_model_id};
use crate::error::{AppError, sanitize_for_log};
use crate::state::DbState;

/// Cap for raw model text passed through `tracing::error!` from the fit
/// handler (spec R17). Mirrors the chat-handler cap.
const LOG_SANITIZE_MAX_CHARS: usize = 500;

/// LLM-audit L2 / R5: explicit request-body byte cap on the AI routes.
///
/// 64 KiB. The largest in-spec request is the fit handler's 15_000-char
/// job description; at up to 4 bytes/char (worst-case UTF-8) plus the JSON
/// envelope that is well under 64 KiB, so this leaves ~4x headroom over
/// the semantic cap while rejecting a ~1 MiB body pre-parse (axum emits
/// 413 before the handler allocates/parses) instead of relying on axum's
/// 2 MiB default and the post-parse char check.
const AI_BODY_LIMIT_BYTES: usize = 64 * 1024;

/// Generic opaque client message for AI-path `AppError::Internal` failures
/// whose detail (upstream/serde error text) must stay server-side only
/// (LLM-audit residual / R7). The detailed `{e}` is logged via
/// `tracing::error!` at each site; the client receives only this fixed
/// string so no upstream/serde text reaches the response body.
const AI_INTERNAL_OPAQUE_MESSAGE: &str = "AI request failed. Please try again later.";

/// LLM-audit M1 / R1: deadline on the fit handler's single buffered
/// `agent.prompt(...).await`.
///
/// 12 s. The spec band is "single-digit to low-tens of seconds"; the R1
/// acceptance test stalls the mock upstream for 40 s and asserts the
/// handler returns within a 25 s wall-clock bound. 12 s sits comfortably
/// under that bound while leaving enough room for a real (non-stalled)
/// Sonnet fit completion, which is a single non-streaming round-trip. On
/// expiry the handler returns the existing degraded `AppError::Internal`
/// surface (fixed opaque client string — never raw upstream text).
const FIT_PROMPT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(12);

/// Extract the client IP from request headers.
///
/// Priority: `trusted_header` (configured via `TRUSTED_IP_HEADER` env var, e.g.
/// `fly-client-ip` on Fly.io) → `x-forwarded-for` (fallback for local dev without
/// a proxy) → `peer_addr` (ConnectInfo peer addr) → `"unknown"`.
///
/// LLM-audit L1 / R4: mirrors the fix in
/// `middleware::global_rate_limit::extract_ip_for_rate_limit`. The
/// no-ConnectInfo handlers pass `None` (→ `"unknown"` only when no peer
/// addr); the `_with_addr` handlers pass `Some(addr)` so distinct
/// no-proxy clients get distinct rate-limit buckets instead of all
/// collapsing into one `"unknown"` bucket.
///
/// SECURITY NOTE: The trusted header is set by the reverse proxy and cannot be spoofed
/// by clients in production. `x-forwarded-for` is used only as a local-dev fallback
/// and is client-controlled; if this service is ever exposed directly (no proxy),
/// rate limiting by XFF IP is bypassable.
fn extract_ip(
    headers: &HeaderMap,
    trusted_header: Option<&str>,
    peer_addr: Option<SocketAddr>,
) -> String {
    // Prefer the configured trusted header (e.g. fly-client-ip, CF-Connecting-IP, etc.).
    if let Some(header_name) = trusted_header
        && let Some(val) = headers.get(header_name)
        && let Ok(val_str) = val.to_str()
    {
        let trimmed = val_str.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    // Fall back to X-Forwarded-For for local dev (no proxy).
    if let Some(forwarded) = headers.get("x-forwarded-for")
        && let Ok(val) = forwarded.to_str()
        && let Some(first_ip) = val.split(',').next()
    {
        let trimmed = first_ip.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    // R4: prefer the peer addr over the collapsing "unknown" bucket.
    match peer_addr {
        Some(addr) => addr.ip().to_string(),
        None => "unknown".to_string(),
    }
}

/// Chat handler that uses ConnectInfo (requires into_make_service_with_connect_info).
/// Used in production where ConnectInfo is available.
#[tracing::instrument(skip(state, headers, payload))]
pub async fn chat_with_addr(
    State(state): State<DbState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let ip = extract_ip(&headers, state.trusted_ip_header.as_deref(), Some(addr));
    chat_inner(state, &ip, payload).await
}

/// Chat handler without ConnectInfo (for testing or when ConnectInfo is not configured).
#[tracing::instrument(skip(state, headers, payload))]
pub async fn chat(
    State(state): State<DbState>,
    headers: HeaderMap,
    Json(payload): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let ip = extract_ip(&headers, state.trusted_ip_header.as_deref(), None);
    chat_inner(state, &ip, payload).await
}

async fn chat_inner(
    state: DbState,
    ip: &str,
    payload: ChatRequest,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>> + use<>>, AppError> {
    if payload.message.len() > 10_000 {
        return Err(AppError::BadRequest(
            "Message too long (max 10,000 characters)".into(),
        ));
    }

    // Lock DB, check rate limit, then — still inside the SAME single lock
    // acquisition (R6) — bind the non-`Option` rig client BEFORE the
    // config accessors run, then build the prompt and read config.
    //
    // Ordering is load-bearing on three constraints that collide here:
    //   - R6: rate-limit + accessors + prompt build share ONE lock.
    //   - Rate-limit-first: `test_ai_endpoints.rs` rate-limit tests run
    //     with `rig_client: None` and require the rate-limit check to
    //     fire BEFORE the AI-disabled guard (the 11th request must 429,
    //     not short-circuit on the missing client). So the guard cannot
    //     be hoisted above `check_rate_limit`.
    //   - R27 / Scenario 12: `get_model_id` / `get_max_tokens` MUST NOT
    //     be called when `rig_client` is `None` (wasted work; the spec
    //     names them explicitly).
    //
    // Binding `client` as a non-`Option` `&anthropic::Client` here makes
    // the accessors *structurally unreachable* on the no-key path — they
    // are textually below the `?` early-return and the type is no longer
    // `Option`, so a future refactor cannot silently reorder the bug
    // back in (parse-don't-validate at the boundary). The `&` borrows
    // `state`, not the `MutexGuard`, so it outlives the block.
    let (client, system_prompt, model, max_tokens) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        check_rate_limit(&conn, ip, "chat", 10)?;
        // R27: AI-disabled guard. Fires AFTER rate-limit, BEFORE the
        // config accessors — `?` returns the existing flat
        // `{"error":"AI features not configured"}` body unchanged.
        let client = state
            .rig_client
            .as_ref()
            .ok_or_else(|| AppError::Internal("AI features not configured".into()))?;
        let prompt = build_system_prompt(&conn)?;
        let model = get_model_id(&conn);
        let max_tokens = u64::from(get_max_tokens(&conn));
        (client, prompt, model, max_tokens)
    };

    // Path X: hand-rolled Anthropic SSE consumer (R11). `stream_chat`
    // returns the bridge channel receiver immediately and spawns the
    // consume + dispatch task in the background.
    let rx = stream_chat(client, &model, &system_prompt, &payload.message, max_tokens)?;
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// Fit analysis handler with ConnectInfo (production).
#[tracing::instrument(skip(state, headers, payload))]
pub async fn fit_analysis_with_addr(
    State(state): State<DbState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<FitRequest>,
) -> Result<Json<FitVerdict>, AppError> {
    fit_analysis_inner(
        state.clone(),
        &extract_ip(&headers, state.trusted_ip_header.as_deref(), Some(addr)),
        payload,
    )
    .await
}

/// Fit analysis handler without ConnectInfo (for testing).
#[tracing::instrument(skip(state, headers, payload))]
pub async fn fit_analysis(
    State(state): State<DbState>,
    headers: HeaderMap,
    Json(payload): Json<FitRequest>,
) -> Result<Json<FitVerdict>, AppError> {
    fit_analysis_inner(
        state.clone(),
        &extract_ip(&headers, state.trusted_ip_header.as_deref(), None),
        payload,
    )
    .await
}

/// Try to extract JSON from a response that may include markdown code fences or preamble text.
fn extract_json(text: &str) -> Option<&str> {
    // Try extracting from ```json ... ``` fences
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim());
        }
    }
    // Try extracting from ``` ... ``` fences
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim());
        }
    }
    // Try finding the first { ... } block
    if let Some(start) = text.find('{')
        && let Some(end) = text.rfind('}')
        && end > start
    {
        return Some(&text[start..=end]);
    }
    None
}

async fn fit_analysis_inner(
    state: DbState,
    ip: &str,
    payload: FitRequest,
) -> Result<Json<FitVerdict>, AppError> {
    if payload.job_description.len() > 15_000 {
        return Err(AppError::BadRequest(
            "Job description too long (max 15,000 characters)".into(),
        ));
    }

    // Lock DB, check rate limit, bind rig client, build fit prompt, read
    // config — all inside ONE lock acquisition (R6). The `rig_client`
    // guard fires AFTER `check_rate_limit` (rate-limit-first; the
    // `test_ai_endpoints.rs` fit rate-limit test runs with
    // `rig_client: None`) and BEFORE the config accessors (R27 /
    // Scenario 12: `get_model_id` / `get_max_tokens` MUST NOT run on the
    // no-key path). Binding the non-`Option` client here makes the
    // accessors structurally unreachable when AI is disabled. See the
    // matching block in `chat_inner` for the full constraint rationale.
    let (client, fit_prompt, model_id, max_tokens) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        check_rate_limit(&conn, ip, "fit", 5)?;
        let client = state
            .rig_client
            .as_ref()
            .ok_or_else(|| AppError::Internal("AI features not configured".into()))?;
        let prompt = build_fit_prompt(&conn)?;
        let model = get_model_id(&conn);
        let max_tokens = u64::from(get_max_tokens(&conn));
        (client, prompt, model, max_tokens)
    };

    // R12 / R14 fit half: build a `CompletionModel` with prompt-caching
    // enabled (so the system block + last message-content block carry
    // `cache_control: { "type": "ephemeral" }` via rig's
    // `apply_cache_control`) and attach the `StopReasonCapture` hook so
    // the buffered, non-streaming response's `raw_response.stop_reason`
    // is observable after `.await`.
    let model = client.completion_model(model_id).with_prompt_caching();
    let agent = rig_core::agent::AgentBuilder::new(model)
        .preamble(&fit_prompt)
        .max_tokens(max_tokens)
        .build();

    let capture = StopReasonCapture::new();
    // R1 (LLM10): bound the single buffered upstream round-trip. Without
    // this, a hung/slow upstream pins the connection indefinitely
    // (bounded only by the per-IP rate quota). `tokio::time::timeout`
    // drops the in-flight future on expiry; the degraded surface is the
    // existing opaque `AppError::Internal` (no raw upstream text).
    let prompt_fut = agent
        .prompt(payload.job_description.as_str())
        .with_hook(capture.clone());
    let response_text = match tokio::time::timeout(FIT_PROMPT_TIMEOUT, prompt_fut).await {
        Ok(Ok(text)) => text,
        Ok(Err(e)) => {
            // R7: the upstream/rig error text (`{e}`) can carry library
            // and HTTP-surface detail. Log it server-side at error level;
            // the client receives only the fixed opaque message so no
            // upstream text reaches the response body.
            tracing::error!(error = %e, "fit: AI prompt failed");
            return Err(AppError::Internal(AI_INTERNAL_OPAQUE_MESSAGE.to_string()));
        }
        Err(_elapsed) => {
            // R1: deadline exceeded. Degraded surface — no upstream text.
            tracing::error!(
                timeout_secs = FIT_PROMPT_TIMEOUT.as_secs(),
                "fit: upstream prompt timed out; returning degraded error"
            );
            return Err(AppError::Internal(AI_INTERNAL_OPAQUE_MESSAGE.to_string()));
        }
    };

    // Map the captured stop_reason. Missing => treat as EndTurn (rig's
    // hook fires after a successful completion; if the wire response
    // omits stop_reason entirely we fall through to the parse path).
    let mapped = capture
        .captured()
        .map(|s| from_anthropic_str(&s))
        .unwrap_or(StopReason::EndTurn);

    match mapped {
        StopReason::EndTurn | StopReason::StopSequence | StopReason::PauseTurn => {
            parse_verdict(&response_text)
        }
        StopReason::MaxTokens => {
            // Spec R13 fit half: warn + attempt parse. If parse succeeds
            // the model happened to fit before truncation; return 200
            // with the verdict. If parse fails, fall through to the
            // existing parse-error path (`AppError::Internal`).
            tracing::warn!(
                stop_reason = "max_tokens",
                "fit response truncated by max_tokens; attempting JSON parse on buffered text"
            );
            parse_verdict(&response_text)
        }
        StopReason::Refusal => {
            // R17: server-side sanitized log; R29: client receives canned
            // body via `AppError::Refusal`'s `IntoResponse` arm. The
            // payload field threads the raw text to the log site only.
            let sanitized = sanitize_for_log(&response_text, LOG_SANITIZE_MAX_CHARS);
            tracing::error!(
                raw_model_text = %sanitized,
                "fit stop_reason=refusal; client receives canned 422 body"
            );
            Err(AppError::Refusal(Some(response_text)))
        }
        StopReason::ContextExceeded => {
            let sanitized = sanitize_for_log(&response_text, LOG_SANITIZE_MAX_CHARS);
            tracing::error!(
                raw_model_text = %sanitized,
                "fit stop_reason=context_exceeded; client receives canned 413 body"
            );
            Err(AppError::ContextExceeded(Some(response_text)))
        }
        StopReason::ToolUse => Err(AppError::Internal(
            "unexpected tool_use; folio is no-tools".into(),
        )),
        StopReason::Other(value) => {
            // R7: `value` is upstream-derived (Anthropic stop_reason
            // string). The mapping function already emitted a `warn!`
            // naming the sanitized value; emit one error! with the value
            // for the server-side trail, then return the fixed opaque
            // client string (no upstream text in the response body).
            tracing::error!(
                stop_reason = %value,
                "fit: unrecognized upstream stop_reason"
            );
            Err(AppError::Internal(AI_INTERNAL_OPAQUE_MESSAGE.to_string()))
        }
    }
}

/// Parse a fit-handler response into a `FitVerdict`, attempting a direct
/// JSON parse first and falling back to the `extract_json` heuristic for
/// responses framed in markdown code fences.
fn parse_verdict(response_text: &str) -> Result<Json<FitVerdict>, AppError> {
    let verdict: FitVerdict = serde_json::from_str(response_text)
        .or_else(|_| {
            extract_json(response_text)
                .map(serde_json::from_str)
                .unwrap_or_else(|| {
                    Err(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "No JSON found",
                    )))
                })
        })
        .map_err(|e| {
            // R7: serde's parse error embeds surrounding model-input
            // context. Log the full detail server-side; the client gets
            // the fixed opaque string only.
            tracing::error!(error = %e, "fit: failed to parse AI response as FitVerdict");
            AppError::Internal(AI_INTERNAL_OPAQUE_MESSAGE.to_string())
        })?;
    Ok(Json(verdict))
}

/// Routes for use without ConnectInfo (e.g., in tests).
///
/// R5: `DefaultBodyLimit::max(AI_BODY_LIMIT_BYTES)` is applied here (not
/// only in `main.rs`) so the pre-parse 413 fires for apps that merge only
/// `routes::ai::routes()` — the behavioral R5 test builds its app that way.
pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/chat", post(chat))
        .route("/api/fit", post(fit_analysis))
        .layer(DefaultBodyLimit::max(AI_BODY_LIMIT_BYTES))
}

/// Routes for use with ConnectInfo (production, where into_make_service_with_connect_info is used).
///
/// R5: same explicit body cap as `routes()`.
pub fn routes_with_connect_info() -> Router<DbState> {
    Router::new()
        .route("/api/chat", post(chat_with_addr))
        .route("/api/fit", post(fit_analysis_with_addr))
        .layer(DefaultBodyLimit::max(AI_BODY_LIMIT_BYTES))
}

// ===========================================================================
// M1 — Warden #562 Medium (conf 86): config accessors MUST NOT run on the
// AI-disabled (no `ANTHROPIC_API_KEY` → `rig_client: None`) path.
// R27 / Scenario 12. White-box, fails-on-regression.
//
// Capture mechanism: `get_model_id` / `get_max_tokens` emit a `WARN` (via
// `tracing::warn!`) when their config row is missing. We construct a DB
// whose `ai.model_id` / `ai.max_tokens` rows are deleted, call
// `chat_inner` / `fit_analysis_inner` with `rig_client: None`, and assert
// ZERO `WARN ` lines were captured. The pre-fix code ran the accessors
// inside the lock block BEFORE the `rig_client` guard, so on the no-key
// path both accessors fired against the missing rows and emitted two
// WARN lines — this test FAILS on that code. The structural fix binds
// the client before the accessors, so they never run → no WARN → pass.
// This is NOT vacuous: the response body is identical pre/post fix
// (`AppError::Internal("AI features not configured")`), so only the
// side-effect (accessor WARN) discriminates buggy-vs-fixed.
// ===========================================================================
#[cfg(test)]
mod m1_no_key_path_tests {
    use super::*;
    use crate::state::{AppState, DbState};
    use rusqlite::Connection;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;
    use tracing_subscriber::layer::SubscriberExt;

    #[derive(Clone, Default)]
    struct LogBuf(Arc<Mutex<Vec<u8>>>);

    impl LogBuf {
        fn captured(&self) -> String {
            String::from_utf8(self.0.lock().expect("log mutex").clone())
                .expect("log output is UTF-8")
        }
        fn warn_count(&self) -> usize {
            self.captured().matches("WARN ").count()
        }
    }

    impl io::Write for LogBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .map_err(|_| io::Error::other("log mutex poisoned"))?
                .extend_from_slice(buf);
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

    /// Migrate + seed, then DELETE the `ai.model_id` / `ai.max_tokens`
    /// config rows so the accessors WOULD emit a `WARN` IF reached.
    fn db_with_missing_config_rows() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory");
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .expect("PRAGMA");
        crate::db::migrate(&conn).expect("migrate");
        crate::db::seed::seed_test_data(&conn).expect("seed");
        // Remove the rows the accessors read. If the accessors run, each
        // emits exactly one `WARN` (config.rs missing-row arm).
        conn.execute("DELETE FROM site_config WHERE key = 'ai.model_id'", [])
            .expect("delete model_id row");
        conn.execute("DELETE FROM site_config WHERE key = 'ai.max_tokens'", [])
            .expect("delete max_tokens row");
        conn
    }

    fn no_key_state(conn: Connection) -> DbState {
        let password_hash = crate::test_password::password_hash();
        Arc::new(AppState {
            db: Arc::new(Mutex::new(conn)),
            admin_password_hash: password_hash,
            rig_client: None,
            trusted_ip_header: None,
            page_hit_salt: "test-salt".to_string(),
        })
    }

    /// Sanity precondition: prove the WARN signal is real — calling the
    /// accessors directly against the missing-row DB DOES emit `WARN`s.
    /// If this did not fire, the no-key tests below would be vacuous
    /// (passing because the signal never exists, not because the guard
    /// blocks it). This is the anti-gaming guard for the pair.
    #[test]
    fn precondition_accessors_emit_warn_against_missing_config_rows() {
        let conn = db_with_missing_config_rows();
        let buf = LogBuf::default();
        let layer = tracing_subscriber::fmt::Layer::default()
            .with_writer(buf.clone())
            .with_ansi(false)
            .with_target(false)
            .without_time();
        let subscriber = tracing_subscriber::Registry::default().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            let _ = get_model_id(&conn);
            let _ = get_max_tokens(&conn);
        });
        assert!(
            buf.warn_count() >= 2,
            "precondition: missing-row accessors MUST emit WARN (else the \
             no-key tests are vacuous); captured={:?}",
            buf.captured()
        );
    }

    /// `chat_inner` with `rig_client: None` MUST return the AI-disabled
    /// error WITHOUT having called the config accessors → zero WARN.
    #[tokio::test]
    async fn chat_no_key_path_does_not_invoke_config_accessors() {
        let state = no_key_state(db_with_missing_config_rows());
        let payload = ChatRequest {
            message: "Hi".to_string(),
        };

        let buf = LogBuf::default();
        let layer = tracing_subscriber::fmt::Layer::default()
            .with_writer(buf.clone())
            .with_ansi(false)
            .with_target(false)
            .without_time();
        let subscriber = tracing_subscriber::Registry::default().with(layer);
        // `set_default` returns a thread-scoped guard (not a closure), so
        // it spans an `.await`. The no-key path never crosses a thread
        // boundary (it returns before `stream_chat` spawns), so the
        // thread-local subscriber captures any accessor WARN reliably.
        let result = {
            let _guard = tracing::subscriber::set_default(subscriber);
            chat_inner(state, "1.2.3.4", payload).await
        };

        // Behavior unchanged: AI-disabled error still surfaces. (`Sse<_>`
        // is not `Debug`, so match the error arm explicitly.)
        match result {
            Err(AppError::Internal(msg)) => {
                assert_eq!(msg, "AI features not configured")
            }
            Err(other) => panic!("expected AI-disabled Internal error, got {other:?}"),
            Ok(_) => panic!("expected AI-disabled error on the no-key path, got Ok"),
        }
        // Fail-on-regression: pre-fix code ran the accessors before the
        // guard → WARN lines present. Structural fix → accessors never
        // reached → zero WARN.
        assert_eq!(
            buf.warn_count(),
            0,
            "M1/R27: config accessors MUST NOT run on the no-key path; \
             a WARN means an accessor executed before the rig_client \
             guard. captured={:?}",
            buf.captured()
        );
    }

    /// `fit_analysis_inner` symmetry — same invariant on the fit handler.
    #[tokio::test]
    async fn fit_no_key_path_does_not_invoke_config_accessors() {
        let state = no_key_state(db_with_missing_config_rows());
        let payload = FitRequest {
            job_description: "Senior engineer".to_string(),
        };

        let buf = LogBuf::default();
        let layer = tracing_subscriber::fmt::Layer::default()
            .with_writer(buf.clone())
            .with_ansi(false)
            .with_target(false)
            .without_time();
        let subscriber = tracing_subscriber::Registry::default().with(layer);
        let result = {
            let _guard = tracing::subscriber::set_default(subscriber);
            fit_analysis_inner(state, "1.2.3.4", payload).await
        };

        // `FitVerdict` is not `Debug` (production type), so match the
        // error arm without debug-formatting the success value.
        match result {
            Err(AppError::Internal(msg)) => {
                assert_eq!(msg, "AI features not configured")
            }
            Err(other) => panic!("expected AI-disabled Internal error, got {other:?}"),
            Ok(_) => panic!("expected AI-disabled error on the no-key path, got Ok"),
        }
        assert_eq!(
            buf.warn_count(),
            0,
            "M1/R27: fit config accessors MUST NOT run on the no-key \
             path. captured={:?}",
            buf.captured()
        );
    }
}
