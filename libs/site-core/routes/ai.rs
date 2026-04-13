use std::convert::Infallible;
use std::net::SocketAddr;

use axum::{
    Json, Router,
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
    routing::post,
};
use futures::stream::{Stream, StreamExt};
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::streaming::StreamingPrompt;
use tracing;

use crate::ai::context::{build_fit_prompt, build_system_prompt};
use crate::ai::rate_limit::check_rate_limit;
use crate::ai::types::{ChatRequest, FitRequest, FitVerdict};
use crate::error::AppError;
use crate::state::DbState;

/// Extract the client IP from request headers.
///
/// Priority: `trusted_header` (configured via `TRUSTED_IP_HEADER` env var, e.g.
/// `fly-client-ip` on Fly.io) → `x-forwarded-for` (fallback for local dev without
/// a proxy) → `fallback` (ConnectInfo addr or "unknown").
///
/// SECURITY NOTE: The trusted header is set by the reverse proxy and cannot be spoofed
/// by clients in production. `x-forwarded-for` is used only as a local-dev fallback
/// and is client-controlled; if this service is ever exposed directly (no proxy),
/// rate limiting by XFF IP is bypassable.
fn extract_ip(headers: &HeaderMap, fallback: &str, trusted_header: Option<&str>) -> String {
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
    fallback.to_string()
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
    let ip = extract_ip(
        &headers,
        &addr.ip().to_string(),
        state.trusted_ip_header.as_deref(),
    );
    chat_inner(state, &ip, payload).await
}

/// Chat handler without ConnectInfo (for testing or when ConnectInfo is not configured).
#[tracing::instrument(skip(state, headers, payload))]
pub async fn chat(
    State(state): State<DbState>,
    headers: HeaderMap,
    Json(payload): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let ip = extract_ip(&headers, "unknown", state.trusted_ip_header.as_deref());
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

    // Lock DB, check rate limit, build system prompt, then drop lock
    let system_prompt = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        check_rate_limit(&conn, ip, "chat", 10)?;
        build_system_prompt(&conn)?
    };

    // Check rig client is configured
    let client = state
        .rig_client
        .as_ref()
        .ok_or_else(|| AppError::Internal("AI features not configured".into()))?;

    // Create a rig agent
    let agent = client
        .agent("claude-sonnet-4-20250514")
        .preamble(&system_prompt)
        .max_tokens(4096)
        .build();

    // Stream the response using the agent's streaming method
    let mut rig_stream = agent.stream_prompt(&payload.message).await;

    // Bridge rig stream to SSE events via an mpsc channel.
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(32);

    tokio::spawn(async move {
        use rig::agent::MultiTurnStreamItem;
        use rig::streaming::StreamedAssistantContent;

        let mut sent_content = false;

        while let Some(chunk) = rig_stream.next().await {
            match chunk {
                Ok(item) => {
                    if let MultiTurnStreamItem::StreamAssistantItem(
                        StreamedAssistantContent::Text(text),
                    ) = item
                    {
                        sent_content = true;
                        let event = Event::default().data(&text.text);
                        if tx.send(Ok(event)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Chat stream error: {:?}", e);
                    break;
                }
            }
        }

        // If stream errored without producing content, send an error event
        if !sent_content {
            tracing::error!("Chat stream produced no content — sending error to client");
            let _ = tx
                .send(Ok(Event::default()
                    .event("error")
                    .data("AI response failed. Please try again.")))
                .await;
        }

        // Send the [DONE] sentinel
        let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
    });

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
        &extract_ip(
            &headers,
            &addr.ip().to_string(),
            state.trusted_ip_header.as_deref(),
        ),
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
        &extract_ip(&headers, "unknown", state.trusted_ip_header.as_deref()),
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

    // Lock DB, check rate limit, build fit prompt, then drop lock
    let fit_prompt = {
        let conn = state
            .db
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        check_rate_limit(&conn, ip, "fit", 5)?;
        build_fit_prompt(&conn)?
    };

    // Check rig client is configured
    let client = state
        .rig_client
        .as_ref()
        .ok_or_else(|| AppError::Internal("AI features not configured".into()))?;

    // Create agent with fit preamble
    let agent = client
        .agent("claude-sonnet-4-20250514")
        .preamble(&fit_prompt)
        .max_tokens(4096)
        .build();

    // Call agent's non-streaming prompt method
    let response_text = agent
        .prompt(&payload.job_description)
        .await
        .map_err(|e| AppError::Internal(format!("AI prompt failed: {}", e)))?;

    // Parse the response as JSON into FitVerdict (with fallback extraction)
    let verdict: FitVerdict = serde_json::from_str(&response_text)
        .or_else(|_| {
            extract_json(&response_text)
                .map(serde_json::from_str)
                .unwrap_or_else(|| {
                    Err(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "No JSON found",
                    )))
                })
        })
        .map_err(|e| {
            AppError::Internal(format!("Failed to parse AI response as FitVerdict: {}", e))
        })?;

    Ok(Json(verdict))
}

/// Routes for use without ConnectInfo (e.g., in tests).
pub fn routes() -> Router<DbState> {
    Router::new()
        .route("/api/chat", post(chat))
        .route("/api/fit", post(fit_analysis))
}

/// Routes for use with ConnectInfo (production, where into_make_service_with_connect_info is used).
pub fn routes_with_connect_info() -> Router<DbState> {
    Router::new()
        .route("/api/chat", post(chat_with_addr))
        .route("/api/fit", post(fit_analysis_with_addr))
}
