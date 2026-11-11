pub mod anthropic_stream;
pub mod context;
mod invisible_table;
pub mod prompt_templates;
pub mod prompts;
pub mod rate_limit;
pub mod stop_reason;
pub mod types;

use crate::error::AppError;

/// Generic opaque client message for AI-path `AppError::Internal` failures
/// whose detail (upstream/serde/SQLite error text) must stay server-side only
/// (LLM-audit residual / R7). The detail is logged at each site; the client
/// receives only this fixed string.
pub(crate) const AI_INTERNAL_OPAQUE_MESSAGE: &str = "AI request failed. Please try again later.";

/// Map an internal failure on an AI path (SQLite error, poisoned mutex) to the
/// opaque client error, logging `detail` server-side (#4711 F4).
///
/// Scoped to AI paths on purpose: the global `From<rusqlite::Error> for
/// AppError` still renders `err.to_string()` into the body for every other
/// surface.
pub(crate) fn opaque_internal(site: &'static str, detail: impl std::fmt::Display) -> AppError {
    tracing::error!(site, error = %detail, "AI path internal failure");
    AppError::Internal(AI_INTERNAL_OPAQUE_MESSAGE.to_string())
}
