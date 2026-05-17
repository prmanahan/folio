//! Anthropic `stop_reason` mapping and PromptHook capture (spec #572 R13).
//!
//! This module owns three things:
//!
//! 1. [`StopReason`] — a `#[non_exhaustive]` enum mirroring the seven
//!    Anthropic-documented `stop_reason` wire strings plus a sanitized
//!    `Other(String)` fallback for forward compatibility (R13).
//! 2. [`from_anthropic_str`] — the wire-string-to-variant mapping function.
//!    `Other(_)` payloads are sanitized at construction (control chars
//!    stripped, CR/LF replaced, truncated to 64 chars) so log lines and
//!    in-process diagnostics can carry the value without re-introducing the
//!    log-injection vector that R17 closes for refusal/context-exceeded
//!    paths. An `Other(_)` construction emits exactly one `tracing::warn!`
//!    event naming the sanitized value (R13).
//! 3. [`StopReasonCapture`] — a [`rig_core::agent::prompt_request::hooks::PromptHook`]
//!    bound against any `M: CompletionModel<Response = anthropic::CompletionResponse>`.
//!    The hook captures `response.raw_response.stop_reason` into shared state
//!    (`Arc<Mutex<Option<String>>>`). Used by the fit handler (R12) — chat is
//!    on Path X (`anthropic_stream.rs`) and reads `stop_reason` off the SSE
//!    `message_delta` frame directly.
//!
//! Visibility: `pub enum StopReason`, `pub fn from_anthropic_str`,
//! `pub struct StopReasonCapture`. The mapping function is named in the spec
//! (R13) and is reachable from both handler paths; making it `pub` keeps the
//! two paths from diverging on what counts as `Refusal` vs `Other`.

use std::sync::{Arc, Mutex};

use rig_core::agent::{HookAction, PromptHook};
use rig_core::completion::{CompletionModel, CompletionResponse};
use rig_core::message::Message;
use rig_core::providers::anthropic::completion::CompletionResponse as AnthropicCompletionResponse;

use crate::error::sanitize_for_log;

/// Maximum number of Unicode characters retained in a sanitized
/// [`StopReason::Other`] payload. Capped tighter than the R17 log path
/// (500 chars) because this value travels in-process as the variant payload
/// and may be matched against in tests / error messages — bounded length
/// keeps panic / debug output readable.
const OTHER_MAX_CHARS: usize = 64;

/// Anthropic `stop_reason` variants per the API docs. Marked
/// [`non_exhaustive`] so future Anthropic additions (or string-typed
/// values we haven't seen yet) can be added without breaking downstream
/// match coverage.
///
/// Wire-string mapping is owned by [`from_anthropic_str`] — DO NOT impl
/// `FromStr` here; the mapping has side effects (sanitization + warn log
/// on `Other`) that don't compose cleanly with `FromStr::Err = Infallible`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// `end_turn` — model produced a normal terminal response.
    EndTurn,
    /// `stop_sequence` — model hit a configured stop sequence. Folio
    /// configures none today; treat as `EndTurn`-equivalent (R13).
    StopSequence,
    /// `pause_turn` — Anthropic server-tool pause checkpoint. Folio uses
    /// no server tools; treat as `EndTurn`-equivalent (R13).
    PauseTurn,
    /// `max_tokens` — model output truncated at the request's max_tokens.
    MaxTokens,
    /// `tool_use` — model emitted a tool_use block. Folio is no-tools;
    /// surfacing this is an internal error (R13).
    ToolUse,
    /// `refusal` — model declined to respond. R29 path: server-side raw
    /// text logged with R17 sanitization, client receives the canned
    /// refusal body.
    Refusal,
    /// `model_context_window_exceeded` — input exceeded the context window.
    /// Same R29 / canned-body treatment as `Refusal`.
    ContextExceeded,
    /// Any wire string not in the recognized set above. Payload is
    /// sanitized at construction (control chars → `?`, CR/LF → `↵`,
    /// truncated to 64 Unicode chars).
    Other(String),
}

/// Map an Anthropic wire-string `stop_reason` (e.g. `"end_turn"`,
/// `"refusal"`, `"model_context_window_exceeded"`) to a [`StopReason`].
///
/// Unrecognized values land in `Other(_)` with sanitization applied to the
/// payload (R13). A single `tracing::warn!` is emitted naming the
/// sanitized value — useful for the "future Anthropic variant appeared in
/// production" detection case.
pub fn from_anthropic_str(value: &str) -> StopReason {
    match value {
        "end_turn" => StopReason::EndTurn,
        "stop_sequence" => StopReason::StopSequence,
        "pause_turn" => StopReason::PauseTurn,
        "max_tokens" => StopReason::MaxTokens,
        "tool_use" => StopReason::ToolUse,
        "refusal" => StopReason::Refusal,
        "model_context_window_exceeded" => StopReason::ContextExceeded,
        other => {
            let sanitized = sanitize_for_log(other, OTHER_MAX_CHARS);
            tracing::warn!(
                stop_reason = %sanitized,
                "unrecognized Anthropic stop_reason; mapping to StopReason::Other"
            );
            StopReason::Other(sanitized)
        }
    }
}

/// PromptHook that captures `raw_response.stop_reason` into shared state.
///
/// rig's `with_hook(...)` clones the hook into the request; depending on
/// the runtime path the hook may be cloned again into a per-callback
/// handler. Wrapping the slot in `Arc<Mutex<...>>` makes all clones see
/// the same `Option<String>` — without the `Arc`, the captured value
/// would land in a clone the caller never reads.
///
/// Concrete `M::Response = anthropic::CompletionResponse` bound is what
/// gives us access to `raw_response.stop_reason: Option<String>`; the
/// generic `CompletionResponse<M::Response>` from rig's `completion` mod
/// exposes only `choice` and `usage` directly. R12.
#[derive(Debug, Clone, Default)]
pub struct StopReasonCapture {
    slot: Arc<Mutex<Option<String>>>,
}

impl StopReasonCapture {
    /// New capture with an empty shared slot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the captured wire-string `stop_reason`. Returns `None` if the
    /// hook never fired (e.g., the prompt failed before reaching the
    /// model) or the model omitted `stop_reason` entirely.
    ///
    /// Lock-poisoning falls back to `None` — the poisoned-mutex path is
    /// unreachable on the single-task fit handler today, but a
    /// non-panicking read keeps the consumer surface honest.
    pub fn captured(&self) -> Option<String> {
        self.slot.lock().ok().and_then(|guard| guard.clone())
    }
}

impl<M> PromptHook<M> for StopReasonCapture
where
    M: CompletionModel<Response = AnthropicCompletionResponse>,
{
    fn on_completion_response(
        &self,
        _prompt: &Message,
        response: &CompletionResponse<M::Response>,
    ) -> impl Future<Output = HookAction> + Send {
        // `raw_response` is anthropic's `CompletionResponse` with
        // `stop_reason: Option<String>`. Clone the string (cheap — short)
        // into our slot under the mutex.
        let slot = self.slot.clone();
        let stop_reason = response.raw_response.stop_reason.clone();
        async move {
            if let Ok(mut guard) = slot.lock() {
                *guard = stop_reason;
            }
            HookAction::cont()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_wire_strings() {
        assert_eq!(from_anthropic_str("end_turn"), StopReason::EndTurn);
        assert_eq!(
            from_anthropic_str("stop_sequence"),
            StopReason::StopSequence
        );
        assert_eq!(from_anthropic_str("pause_turn"), StopReason::PauseTurn);
        assert_eq!(from_anthropic_str("max_tokens"), StopReason::MaxTokens);
        assert_eq!(from_anthropic_str("tool_use"), StopReason::ToolUse);
        assert_eq!(from_anthropic_str("refusal"), StopReason::Refusal);
        assert_eq!(
            from_anthropic_str("model_context_window_exceeded"),
            StopReason::ContextExceeded
        );
    }

    #[test]
    fn other_sanitizes_payload() {
        let dirty = "weird\x07\x1b[31m\nvalue";
        match from_anthropic_str(dirty) {
            StopReason::Other(payload) => {
                assert!(!payload.contains('\x07'));
                assert!(!payload.contains('\x1b'));
                assert!(!payload.contains('\n'));
                assert!(!payload.contains('\r'));
            }
            other => panic!("expected Other, got {:?}", other),
        }
    }

    #[test]
    fn other_truncates_at_64_chars() {
        let dirty = "x".repeat(200);
        match from_anthropic_str(&dirty) {
            StopReason::Other(payload) => {
                assert!(payload.chars().count() <= 64);
            }
            other => panic!("expected Other, got {:?}", other),
        }
    }

    #[test]
    fn capture_starts_empty() {
        let cap = StopReasonCapture::new();
        assert_eq!(cap.captured(), None);
    }
}
