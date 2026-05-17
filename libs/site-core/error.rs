//! Application-level error type and its `IntoResponse` mapping.
//!
//! Spec #572 (R15, R16, R17, R29) adds two AI-stop-reason-shaped variants —
//! `Refusal(Option<String>)` and `ContextExceeded(Option<String>)`. Both
//! carry an optional raw model-text payload for SERVER-SIDE logging only;
//! the client response body is a canned static string (R29 — never reflect
//! untrusted model output to the client).
//!
//! Heterogeneous response shape, by design:
//! - `Refusal`         → 422, `{"error":"refusal","message":"<canned>"}`
//! - `ContextExceeded` → 413, `{"error":"context_exceeded","message":"<canned>"}`
//! - all other variants → keep the existing flat `{"error":"<msg>"}` shape.
//!
//! The R17 sanitizer (`sanitize_for_log`) is exported for reuse by
//! `crate::ai::stop_reason` (which caps `Other(_)` payloads at 64 chars)
//! and by the AI handler error-logging paths (which cap raw model refusal
//! / context-exceeded text at 500 chars before emitting the `error!` log
//! record).

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// Canned client-facing message for `AppError::Refusal`. Defined here so
/// the variant's `IntoResponse` arm and any tests share the exact string.
pub const REFUSAL_CLIENT_MESSAGE: &str = "The assistant declined to respond to that request.";

/// Canned client-facing message for `AppError::ContextExceeded`.
pub const CONTEXT_EXCEEDED_CLIENT_MESSAGE: &str =
    "The request exceeded the model's context window. Try a shorter input.";

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Internal(String),
    Unauthorized(String),
    BadRequest(String),
    RateLimited(String),
    /// Anthropic returned `stop_reason = "refusal"`. The optional payload is
    /// the raw model refusal text for SERVER-SIDE logging only; the client
    /// response body uses the canned [`REFUSAL_CLIENT_MESSAGE`] (R29).
    Refusal(Option<String>),
    /// Anthropic returned `stop_reason = "model_context_window_exceeded"`.
    /// Same payload contract as `Refusal` — raw text is server-side-only.
    ContextExceeded(Option<String>),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, Json(json!({ "error": msg }))).into_response()
            }
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": msg })),
            )
                .into_response(),
            AppError::Unauthorized(msg) => {
                (StatusCode::UNAUTHORIZED, Json(json!({ "error": msg }))).into_response()
            }
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
            }
            AppError::RateLimited(msg) => {
                (StatusCode::TOO_MANY_REQUESTS, Json(json!({ "error": msg }))).into_response()
            }
            AppError::Refusal(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({
                    "error": "refusal",
                    "message": REFUSAL_CLIENT_MESSAGE,
                })),
            )
                .into_response(),
            AppError::ContextExceeded(_) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(json!({
                    "error": "context_exceeded",
                    "message": CONTEXT_EXCEEDED_CLIENT_MESSAGE,
                })),
            )
                .into_response(),
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

/// Sanitize a raw-model-text payload before it crosses into a tracing log
/// line. Spec R17.
///
/// Sequence (order matters — strip ANSI BEFORE replacing control chars,
/// because ANSI sequences begin with `\x1b` which is itself a control char):
///
/// 1. Strip ANSI escape sequences: `\x1b[...]<letter>` and the bare `\x1b`.
/// 2. Replace control characters (U+0000–U+001F, excluding `\t` which we
///    treat as printable; CR/LF handled in step 3) with `?`.
/// 3. Replace `\r` and `\n` with the placeholder `↵` (single Unicode arrow).
///    Raw newlines MUST NOT pass through — `tracing_subscriber::fmt()`
///    writes one log record per line, so a raw `\n` in the payload spoofs
///    a new log line on Fly.io's per-line stdout capture.
/// 4. Truncate to `max_chars` Unicode characters (NOT bytes) so the result
///    is UTF-8-safe.
///
/// `max_chars` is the cap on the *output* — by spec, sanitization runs to
/// completion before the truncate so adversarial padding (e.g. control-char
/// noise as a prefix) cannot push useful tail content beyond the window.
pub fn sanitize_for_log(input: &str, max_chars: usize) -> String {
    // Step 1: strip ANSI escape sequences via a tiny inline state machine.
    // ANSI shapes we care about for Fly.io / `fly logs` operator terminals:
    //   - CSI:  ESC '[' params* final-byte (final-byte in 0x40..=0x7E)
    //   - OSC:  ESC ']' ... (BEL | ESC '\\')
    //   - others: drop `ESC <next>` pair as a defensive fallback.
    let mut without_ansi = String::with_capacity(input.len());
    let mut iter = input.chars().peekable();
    while let Some(c) = iter.next() {
        if c == '\x1b' {
            match iter.next() {
                None => break,
                Some('[') => {
                    // CSI: consume until a final byte in 0x40..=0x7E.
                    for nested in iter.by_ref() {
                        if matches!(nested, '\x40'..='\x7e') {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC: consume until BEL (0x07) or ST (ESC '\').
                    let mut prev_esc = false;
                    for nested in iter.by_ref() {
                        if nested == '\x07' {
                            break;
                        }
                        if prev_esc && nested == '\\' {
                            break;
                        }
                        prev_esc = nested == '\x1b';
                    }
                }
                Some(_) => {
                    // Generic ESC + next: drop the pair (defensive).
                }
            }
            continue;
        }
        without_ansi.push(c);
    }

    // Steps 2 + 3: control-char and CR/LF substitution in one pass.
    let mut cleaned = String::with_capacity(without_ansi.len());
    for c in without_ansi.chars() {
        match c {
            '\r' | '\n' => cleaned.push('↵'),
            '\t' => cleaned.push('\t'),
            ch if (ch as u32) < 0x20 => cleaned.push('?'),
            ch => cleaned.push(ch),
        }
    }

    // Step 4: truncate to `max_chars` Unicode chars (chars, not bytes).
    if cleaned.chars().count() <= max_chars {
        cleaned
    } else {
        cleaned.chars().take(max_chars).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_ansi_csi() {
        let dirty = "before\x1b[31mRED\x1b[0mafter";
        assert_eq!(sanitize_for_log(dirty, 500), "beforeREDafter");
    }

    #[test]
    fn sanitize_replaces_control_chars() {
        let dirty = "a\x07b\x01c";
        assert_eq!(sanitize_for_log(dirty, 500), "a?b?c");
    }

    #[test]
    fn sanitize_replaces_crlf() {
        let dirty = "line1\r\nline2\nline3";
        assert_eq!(sanitize_for_log(dirty, 500), "line1↵↵line2↵line3");
    }

    #[test]
    fn sanitize_truncates_to_max_chars_unicode_safe() {
        let dirty = "X".repeat(1000);
        let cleaned = sanitize_for_log(&dirty, 500);
        assert_eq!(cleaned.chars().count(), 500);
    }

    #[test]
    fn sanitize_keeps_tab_and_printable() {
        let dirty = "col1\tcol2 col3";
        assert_eq!(sanitize_for_log(dirty, 500), "col1\tcol2 col3");
    }
}
