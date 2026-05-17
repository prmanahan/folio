# Feature: site_config table + Sonnet 4 → 4.6 migration + Anthropic SSE consumer (chat) + PromptHook (fit)

> **Spec for:** task #572 — Folio model migration: Sonnet 4-20250514 → Sonnet 4.6
> **Date:** 2026-05-15 (v3)
> **Status:** locked (trust-and-lock — v1 and v2 each had a full reviewer pass; v3 is a targeted architectural reset documented below; re-review only if implementation surfaces something unexpected against the verification asks)
> **Target:** `/Users/manahan/claude_workspace/repos/folio`
> **Implementation pipeline:** spec v3 (this revision, locked) → Glitch-first implementation (red-phase tests → Forge impl → Glitch test-integrity re-review → Warden code+security review) → 90% coverage gate → `/merge` (rebase-merge)

## Changelog

- **v4 (2026-05-17):** Scenario-1/R29 contradiction reconciled after Warden code review (#562). Anthropic delivers `stop_reason` only in the terminal `message_delta` frame; R29 (no raw refusal text to client under any path) therefore requires the chat handler to buffer all content deltas until that terminal frame — streaming-as-arrives is incompatible with refusal compliance. v3's Scenario 1 ("emits as they arrive") and R13's MaxTokens row ("content already streamed") presumed streaming and were internally contradictory with R29. v4 formalizes server-side buffering as the chat contract (R11a). The shipped implementation already conforms; this is a spec correction, not an implementation change.

- **v3 (2026-05-15):** Architectural reset after v2 reviewers confirmed that rig-core 0.37 streaming still drops `stop_reason` to the caller — same blocker shape as v1, surfaced one abstraction layer down. Path X locked.
  - **Chat handler — custom Anthropic SSE consumer.** New module `libs/site-core/ai/anthropic_stream.rs` consumes Anthropic SSE directly via `rig::http_client::sse::GenericEventSource` over a request built with `rig::providers::anthropic::Client::post_sse("/v1/messages")`. The `Client` continues to supply auth (`x-api-key`), `anthropic-version`, optional `anthropic-beta`, base-URL normalization, and `Content-Type` insertion; the handler owns body construction (using rig's pub `Message`/`Content`/`SystemContent`/`CacheControl` types) and the SSE event-match loop (where `MessageDelta.stop_reason` is captured). The surface was verified against rig-core 0.37 source with file:line citations prior to v3 lock.
  - **Fit handler — `agent.prompt()` + PromptHook.** Stays on the high-level Agent surface (buffered, non-streaming) and attaches a `PromptHook<M>` that captures `response.raw_response.stop_reason` into shared state (`Arc<Mutex<Option<String>>>`). The non-streaming `CompletionResponse` on Anthropic exposes `stop_reason` natively; the hook is the natural capture site.
  - **R12 split:** v2's "shared helper" framing conflated two return types and two call patterns. v3 specifies two named helpers, one per handler shape — no fiction of unification.
  - **StopReason enum expanded** to cover all seven Anthropic-documented variants: `EndTurn | StopSequence | PauseTurn | MaxTokens | ToolUse | Refusal | ContextExceeded | Other(String)`. v2 was missing `StopSequence` and `PauseTurn` — normal `stop_sequence` completions would 500 per v2's degradation rule. Enum is `#[non_exhaustive]` to permit future variant additions without breaking dependent matches.
  - **R17 sanitization (log injection mitigation):** raw model text MUST have ANSI escape sequences stripped, control characters replaced, and CR/LF replaced with a visible placeholder before the 500-char truncate. Folio uses text-format `tracing_subscriber::fmt()`; raw newlines in model output spoof log lines on Fly.io's per-line stdout capture.
  - **MaxTokens behavior specified** (was undefined in v2). Chat: stream content already received, emit `event: truncated\ndata: <canned>` then `[DONE]`. Fit: log `warn!` and attempt JSON parse — existing parse-error surface handles incomplete JSON.
  - **`Other(String)` logging specified.** Mapping function emits `tracing::warn!` naming the (sanitized) unrecognized value before falling through to `AppError::Internal`. Closes the "future stop_reason invisible in production" gap.
  - **Rollback footgun warned:** controlled rollback via DROP migration + later re-deploy of new code leaves `_migrations` claiming v5 applied while the table is absent. Service runs on compiled-in defaults (degraded but not crashing). Mitigation: ship a `007_restore_site_config.sql` in the upgrade chain.
  - **R14 split (asymmetric cache_control):** chat hand-rolls `cache_control: Some(CacheControl::ephemeral())` on the `SystemContent::Text` block when building the body (no `CompletionModel` involved). Fit uses `model.with_prompt_caching()` on the `CompletionModel` before constructing the Agent.
  - **"Verification asks for Forge"** section added — three items that the rig-core 0.37 source investigation did not pin down at investigation time and that gate impl: (a) exact `with_hook` method name on `PromptRequest` builder, (b) Anthropic SSE `error` event payload shape, (c) `rig::http_client::retry::*` default policy (likely `ExponentialBackoff` — needs explicit `NoRetry` to prevent double-streaming).

- **v2 (2026-05-15):** Substantial architectural revision after v1 review.
  - rig-core bumped 0.33 → 0.37 (new constants, prompt-caching API, PromptHook trait).
  - Attempted to unify both AI handlers to the lower-level `CompletionModel.stream(...)` API on the assumption that the lower-level streaming surface exposed `stop_reason`. **v2 reviewers verified against rig-core 0.37 source that the streaming `StreamingCompletionResponse` carries only `usage` — `stop_reason` is read inside rig and discarded before return.** v3 supersedes this approach with Path X.
  - Module path locked to `libs/site-core/db/config.rs` — `libs/site-core/config.rs` already exists (env-var Config struct).
  - Migration runner: explicit MIGRATIONS array registration step added to Task 2.
  - Frontend `sse.ts` parser update added (new Task 5) — without it, new event types render as content text.
  - Refusal/context-exceeded payloads: client receives canned static messages; raw model text logged server-side only (security: no untrusted LLM content reflected to client body).
  - R3/R4 log levels split: `warn!` for missing row, `error!` for infrastructure failure.
  - `max_tokens` clamped at 200 000 upper bound.
  - Scenario 10 (idempotency) rewritten to test the migration-runner skip mechanism (load-bearing) plus `INSERT OR IGNORE` (belt-and-braces).
  - New scenarios: empty-string config value, whitespace-trim behavior, max_tokens clamp at upper bound.
  - Operational Requirements section added (per Bolt v1 review).
  - Spec header updated: rebase-merge (folio merge convention flipped 2026-05-15 to match mnemra-core).
  - Glitch-first locked in for implementation (per Glitch v1 verdict).

## Purpose

Anthropic is removing `claude-sonnet-4-20250514` from production. Folio uses it as a hardcoded literal in two `routes/ai.rs` call sites. This spec migrates the model ID to a SQLite-backed runtime config table so future model swaps no longer require recompilation, bumps rig-core 0.33 → 0.37, splits the two AI handlers onto architecturally appropriate rig-core surfaces (custom Anthropic SSE consumer for the chat handler, where rig discards `stop_reason`; `agent.prompt()` + `PromptHook` for the fit handler, where the non-streaming `CompletionResponse` exposes `stop_reason` natively), adds prompt-caching for cost savings, and adds typed error mapping for refusal and context-window-exceeded model responses.

## Requirements

### Config storage

- **R1.** A new SQLite table `site_config` SHALL exist after migration. Schema is a string-typed key/value store: `key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL, updated_at TEXT NOT NULL DEFAULT (datetime('now'))`. Typed parsing happens in Rust accessors, not in SQL.
- **R2.** The migration SHALL seed two rows on first apply via `INSERT OR IGNORE`:
  - `key='ai.model_id', value='claude-sonnet-4-6'`
  - `key='ai.max_tokens', value='5530'`
- **R3.** A typed accessor `get_model_id(&Connection) -> String` SHALL return the value of `ai.model_id` from the table after stripping leading/trailing whitespace, OR return the compiled-in default `"claude-sonnet-4-6"` if the row is missing OR the trimmed value is empty. The accessor MUST NOT panic.
  - Missing row OR empty-after-trim → emit `tracing::warn!` naming the key and the fallback used.
  - SQL connection error (file-locked, schema missing, etc.) → emit `tracing::error!` naming the key and the underlying error; still return the compiled-in default rather than propagate.
- **R4.** A typed accessor `get_max_tokens(&Connection) -> u32` SHALL return the parsed integer value of `ai.max_tokens` after whitespace-trim, OR return the compiled-in default `5530` if the row is missing, the value is empty after trim, OR the value cannot be parsed as `u32`. The accessor SHALL clamp the returned value to the inclusive range `[1, 200_000]` — values outside the range are replaced with the nearest bound and a `tracing::warn!` is emitted naming the offending value.
  - Missing row, empty after trim, parse failure → `tracing::warn!`.
  - SQL connection error → `tracing::error!`.
  - Out-of-range value → `tracing::warn!` after clamp.
- **R5.** Both accessors SHALL be reachable from `crate::ai::*` and from `routes/ai.rs` without requiring a new struct field on `AppState`. Accessors take a `&Connection` and read on demand.
- **R6.** Both accessors SHALL be invoked inside the existing per-request DB lock in each AI handler (the same lock that wraps `check_rate_limit` and `build_*_prompt`). They MUST NOT acquire a second lock.
- **R7.** The accessors SHALL live at `libs/site-core/db/config.rs` (a new module). The path `libs/site-core/config.rs` is taken — it MUST NOT be reused.

### Migration registration

- **R8.** The new file `migrations/005_site_config.sql` SHALL be added AND the corresponding entry SHALL be added to the `MIGRATIONS` const array in `libs/site-core/db/schema.rs` (or wherever the runner reads its migration list). A new migration file without a corresponding registration entry is dead code; both changes ship together.

### Dependency: rig-core bump

- **R9.** `rig-core` SHALL be bumped from `0.33` to `0.37` in the workspace `Cargo.toml`. The bump MUST be accompanied by a verification pass: build clean, all existing tests pass, no behavior regression in chat or fit endpoints under manual smoke test (or equivalent automated coverage).
- **R10.** Breaking changes between 0.33 and 0.37 (4 minor versions, ~2 months) SHALL be enumerated in the PR description. The implementer MUST NOT silently work around a breaking change without naming it; if a rename or signature change forces an internal API shift, that shift is documented.

### AI handler architecture (v3 — Path X, asymmetric)

- **R11. Chat handler — custom Anthropic SSE consumer.** `chat_inner` SHALL consume the Anthropic Server-Sent Events stream directly via `rig::http_client::sse::GenericEventSource` over a request constructed with `rig::providers::anthropic::Client::post_sse("/v1/messages")`. The handler MUST NOT route through `rig::CompletionModel::stream(...)` or `agent.stream_prompt(...)` for the chat path. Rationale: rig's streaming surface (both `Agent::stream_prompt` and `CompletionModel::stream`) reads `MessageDelta.stop_reason` internally and discards it before returning to the caller; consuming SSE directly is the only path that preserves `stop_reason`. The `Client` continues to supply auth (`x-api-key`), `anthropic-version`, optional `anthropic-beta`, base-URL normalization, and `Content-Type` injection — the handler only owns body construction (using rig's public Anthropic request types) and the event-match loop.

- **R11a. Chat content buffering is the R29 enforcement mechanism.** Because Anthropic delivers `stop_reason` only in the terminal `message_delta` SSE frame (the reason R11/Path X exists at all), `chat_inner` SHALL buffer all `content_block_delta` text server-side until that terminal frame is decoded. On `Refusal`/`ContextExceeded` the buffer is discarded and only the canned terminal event is emitted (R29). On `EndTurn`/`StopSequence`/`PauseTurn` the buffer is flushed in order as default-event `data:` frames then `[DONE]`. On `MaxTokens` the buffer is flushed then `event: truncated` is appended (R13). The streaming-latency cost of buffering is the accepted contract: R29 on a streaming API is unachievable without terminal-gating, since the refusal signal arrives last. Token-incremental delivery to the client is explicitly out of scope for this spec.

- **R12. Fit handler — `agent.prompt()` + PromptHook.** `fit_analysis_inner` SHALL call `agent.prompt(...)` (non-streaming, buffered) on the existing rig Agent surface, attaching a `PromptHook<M>` whose `on_completion_response(_, response)` MUST capture `response.raw_response.stop_reason` into shared state (`Arc<Mutex<Option<String>>>`) and return `HookAction::Continue`. After `await`, the handler reads the captured value and maps it through the same `StopReason::from_anthropic` function used by chat. Rationale: fit is buffered-parse-then-return; the non-streaming `CompletionResponse` on Anthropic carries `stop_reason` natively via `raw_response.stop_reason`, so the hook is the natural capture site and rig's high-level surface stays intact for fit.

- **R12a. Two named helper modules, no shared helper.** v2 specified a "shared streaming helper" returning either `mpsc::Receiver` or `String` — two return types and two consumption patterns, never actually shared. v3 specifies two named modules:
  - `libs/site-core/ai/anthropic_stream.rs` — chat handler's SSE consumer. Entry point shape (exact names at implementer's call): `pub async fn stream_chat(client: &anthropic::Client, model: &str, system: &str, user: &str, max_tokens: u64) -> Result<(mpsc::Receiver<String>, oneshot::Receiver<StopReason>), AppError>`. Owns body construction (~10 LOC using rig's pub `Message`/`Content`/`SystemContent`/`CacheControl`), the SSE event-match loop (~50 LOC), and result-channel plumbing (~15 LOC). Total module ~120–170 LOC.
  - `libs/site-core/ai/stop_reason.rs` — the `StopReason` enum (R13), the `from_anthropic_str(&str) -> StopReason` mapping function (with sanitization for the `Other` payload), and the `StopReasonCapture` struct that implements `PromptHook<M>` for fit.
  - The two modules MUST NOT cross-depend beyond `anthropic_stream.rs` importing `StopReason` from `stop_reason.rs`. There is no shared streaming helper.

- **R13. `StopReason` enum.** `libs/site-core/ai/stop_reason.rs` SHALL define:

  ```rust
  #[non_exhaustive]
  pub enum StopReason {
      EndTurn,
      StopSequence,
      PauseTurn,
      MaxTokens,
      ToolUse,
      Refusal,
      ContextExceeded,
      Other(String),
  }
  ```

  Mapping from Anthropic's wire strings (per `https://platform.claude.com/docs/en/api/handling-stop-reasons`):

  | Wire string | Variant |
  |-------------|---------|
  | `end_turn` | `EndTurn` |
  | `stop_sequence` | `StopSequence` |
  | `pause_turn` | `PauseTurn` |
  | `max_tokens` | `MaxTokens` |
  | `tool_use` | `ToolUse` |
  | `refusal` | `Refusal` |
  | `model_context_window_exceeded` | `ContextExceeded` |
  | (anything else) | `Other(<sanitized>)` |

  The `Other(String)` payload MUST be sanitized inside `from_anthropic_str` before construction: control characters (U+0000–U+001F) replaced with `?`, CR/LF replaced with space, then truncated to 64 chars (chars, not bytes). When `Other(_)` is produced, the mapping function SHALL emit `tracing::warn!` naming the sanitized value (e.g., `unrecognized stop_reason: <value>`) before returning. The `#[non_exhaustive]` attribute exists so future Anthropic variants can be added without breaking dependent matches.

  **Handler behavior by variant:**

  | Variant | Chat handler | Fit handler |
  |---------|--------------|-------------|
  | `EndTurn` | Stream completes; emit `[DONE]` | Parse buffered JSON; return 200 |
  | `StopSequence` | Treat as `EndTurn` (folio passes no `stop_sequences` today; content at the sentinel is valid) | Same as `EndTurn` |
  | `PauseTurn` | Treat as `EndTurn` (folio uses no server tools today; continuation logic out of scope) | Same as `EndTurn` |
  | `MaxTokens` | Flush the buffered partial content as default-event `data:` frames, then emit `event: truncated\ndata: The response was cut off due to length.` then `[DONE]`. The partial content received before truncation is delivered to the user. Log `warn!` server-side naming the model. | Log `warn!` server-side; attempt `extract_json` + `serde_json::from_str` on the buffered string. If parse succeeds, return 200 with the verdict (the model output happened to fit before truncation). If parse fails, fall through to the existing parse-error code path. No new `AppError` variant. |
  | `ToolUse` | `AppError::Internal("unexpected tool_use; folio is no-tools")` | Same |
  | `Refusal` | Emit `event: refusal\ndata: <canned>` then `[DONE]`; `error!` log with sanitized raw text (R17) | HTTP 422 with canned body (R16); `error!` log with sanitized raw text (R17) |
  | `ContextExceeded` | Emit `event: context_exceeded\ndata: <canned>` then `[DONE]`; `error!` log with sanitized raw text (R17) | HTTP 413 with canned body (R16); `error!` log with sanitized raw text (R17) |
  | `Other(_)` | `AppError::Internal`; `warn!` log already emitted by the mapping function | Same |

### Cache control (v3 — asymmetric)

- **R14. Cache_control mechanism is asymmetric by handler.** The architectural split in R11/R12 dictates two different mechanisms:
  - **Chat handler (Path X — bypasses CompletionModel):** SHALL hand-roll `cache_control: Some(CacheControl::ephemeral())` on the `SystemContent::Text` block when constructing the JSON request body. Use rig's pub `rig::providers::anthropic::completion::CacheControl::ephemeral()` constructor. Default TTL (5 min — `Ephemeral { ttl: None }`). NO top-level `cache_control` field on the body. Folio's chat is single-turn per request; only the system preamble is cacheable.
  - **Fit handler (still using rig's Agent + CompletionModel):** SHALL call `.with_prompt_caching()` on the `CompletionModel` before constructing the Agent. This sets `cache_control` on the system block and the last message content block automatically per rig-core 0.37's `apply_cache_control`. NO `with_automatic_caching()` — folio is single-turn; per-block control is sufficient.
  - **No 1-hour TTL.** The `extended-cache-ttl-2025-04-11` beta header is out of scope; default 5 min is the right knob for a portfolio site.
  - **Verification:** Task 4 ACs (both chat and fit) require an outbound-request assertion that the system block carries `cache_control: { "type": "ephemeral" }`. Glitch's mock-Anthropic-HTTP harness covers this for both paths.

### Error mapping

- **R15.** New `AppError` variants SHALL be added: `AppError::Refusal(Option<String>)` and `AppError::ContextExceeded(Option<String>)`. The `Option<String>` carries raw model text for **server-side logging only**. The client-facing message MUST be a canned static string defined in code, not the captured model text. Rationale: model output is untrusted input — never reflect it verbatim to a client response body.
- **R16.** A custom `IntoResponse` impl SHALL be added (or extended) for the new variants:
  - `AppError::Refusal(_)` → HTTP 422 Unprocessable Entity, body `{"error":"refusal","message":"The assistant declined to respond to that request."}` (canned).
  - `AppError::ContextExceeded(_)` → HTTP 413 Payload Too Large, body `{"error":"context_exceeded","message":"The request exceeded the model's context window. Try a shorter input."}` (canned).
  - Existing `AppError` variants keep their current `{"error": <msg>}` flat shape — the new two-field `{"error":"<code>","message":"<canned>"}` shape applies ONLY to the two new variants. This is a deliberate heterogeneous response shape; documented here so future operators don't homogenize one direction without considering both.
- **R17. Server-side logging with sanitization (log injection mitigation).** When `AppError::Refusal(Some(text))` or `AppError::ContextExceeded(Some(text))` fires, an `error!`-level `tracing` record SHALL be emitted server-side that includes the captured raw model text, **sanitized then truncated**. The sanitization sequence is:
  1. Strip ANSI escape sequences (any byte sequence starting with `\x1b[` and ending in a letter).
  2. Replace control characters (U+0000–U+001F except `\t` and printable space) with `?`.
  3. Replace literal `\r` and `\n` with a visible placeholder (`↵` or a single space — implementer's call, but raw newlines MUST NOT pass through).
  4. Truncate to 500 chars (Unicode chars, not bytes — UTF-8-safe).

  Rationale: folio runs `tracing_subscriber::fmt()` text format (not JSON). Raw newlines in model text become spoofed log lines on Fly.io's per-line stdout capture; ANSI escapes corrupt `fly logs` operator terminals. The 500-char cap also bounds log-volume amplification from an adversary who can trigger repeated refusals. The client never sees this text under any code path (R29).

  The same sanitize-and-truncate helper SHALL be applied to the `Other(String)` log payload (R13) and reused for any future log-bound raw-model-text path.

- **R18.** The chat handler SHALL emit typed terminal events before closing the SSE stream for the following conditions; the `data:` payload is the canned message defined here (or in R16), never raw model text:
  - `event: refusal\ndata: The assistant declined to respond to that request.` then `[DONE]`.
  - `event: context_exceeded\ndata: The request exceeded the model's context window. Try a shorter input.` then `[DONE]`.
  - `event: truncated\ndata: The response was cut off due to length.` then `[DONE]` — fires on `StopReason::MaxTokens` (R13).

### Frontend SSE handling

- **R19.** `frontend/src/lib/sse.ts` (or wherever the SSE parser lives) SHALL be updated to recognize three new event types (`refusal`, `context_exceeded`, `truncated`) and route them to a typed error path in the chat UI. Without this change, the new server-emitted event frames are silently treated as content text, which is a visible regression. Implementer MAY route all three to the existing `event: error` rendering path; either way, the new event types MUST NOT render as content. The `truncated` event is informational rather than a hard error — the implementer MAY display it with distinct styling (e.g., a subtle banner under the truncated response) but is not required to.
- **R20.** The chat UI's display of `refusal` vs `context_exceeded` vs `truncated` vs general error MAY differ visually but MUST NOT render the canned message as if it came from the AI assistant (no avatar, no chat-bubble styling — error/notice treatment).

### Prohibitions (load-bearing under Opus 4.7's literal-reading regime)

- **R21.** This change SHALL NOT introduce an admin UI for editing `site_config`. The table is admin-edited via SQL or seed scripts in this spec; UI is a separate spec.
- **R22.** This change SHALL NOT migrate any other hardcoded value (rate-limit thresholds, message-length caps, prompt-string fragments, IP-header names, salt) into `site_config`. Only the two AI parameters in scope.
- **R23.** This change SHALL NOT add a fallback that retries against the old `claude-sonnet-4-20250514` model on failure. The old model is being deprecated; a fallback to it would mask production breakage.
- **R24.** This change SHALL NOT add an in-process cache of `site_config` values. Each request reads fresh from SQLite under the existing lock.
- **R25.** This change SHALL NOT add new dependencies beyond the rig-core bump explicitly authorized by R9.
- **R26.** This change SHALL NOT modify, rename, or restructure the `ai_instructions`, `candidate_profile`, or `values_culture` tables, or any prompt-construction code in `libs/site-core/ai/context.rs` beyond what is strictly required to thread the system preamble through the new helper modules (`anthropic_stream.rs` for chat, `stop_reason.rs` for fit). Prompt content is out of scope.
- **R27.** This change SHALL NOT change the `ANTHROPIC_API_KEY` env-var contract or the `rig_client: Option<...>` pattern in `AppState`. The "AI features disabled when key absent" behavior MUST be preserved exactly. The `get_*` accessors SHALL NOT be called when `rig_client` is `None` — both handlers already early-return in that case before reaching the model setup.
- **R28.** This change SHALL NOT alter the `cargo test` / `just check` / coverage gates. The 90% line + function coverage hard floor (per project G-0001) applies to all new code.
- **R29.** Refusal and context-exceeded handling SHALL NOT propagate raw model text to the client response body under any code path. Server-side logging of raw text is permitted (R17); client reflection is not.

## Out of Scope

- Admin UI for `site_config` (`/admin/config` or similar) — separate spec.
- Migrating rate-limit caps (currently `10` and `5` in `routes/ai.rs:102, 249`) into `site_config` — separate spec.
- Migrating message-length caps (`10_000`, `15_000` in `routes/ai.rs:90, 237`) into `site_config` — separate spec.
- Per-request model override via API parameter.
- Provider abstraction (OpenAI / local model fallback) — folio is Anthropic-only by design.
- Audit log of config changes — there is no UI to log; SQL edits are out-of-band.
- Schema validation library, JSONSchema, or typed config DSL.
- Backwards-compat shims for the old model ID anywhere in code, tests, fixtures, or docs.
- Changes to the `_migrations` machinery itself.
- Tool-calling, multi-turn memory, or agent abstractions in either handler — folio's chat and fit are both single-turn, no tools, no memory. The chat handler's custom SSE consumer assumes no tool_use blocks in the response stream; fit's PromptHook fires once per `prompt()` call, not per agentic turn.
- Hot-reload of `site_config` values — operator-edited rows take effect on next request because reads are per-request; this is by design, not a feature requiring a "reload" path.

## Scenarios

### Scenario 1: Happy path — chat with seeded config (custom Anthropic SSE consumer)

**Given** a fresh database with migration 005 applied
**And** `ANTHROPIC_API_KEY` is set
**When** a client POSTs `/api/chat` with a valid message
**Then** the chat handler issues an SSE POST to `/v1/messages` via `rig::providers::anthropic::Client::post_sse(...)`
**And** the outbound request body uses `model = "claude-sonnet-4-6"`, `max_tokens = 5530`, `stream = true`, and carries `cache_control: { "type": "ephemeral" }` on the `SystemContent::Text` block
**And** the chat handler buffers content deltas server-side until the terminal `message_delta` frame; on the `EndTurn` non-refusal stop it flushes the buffered content in order as default-event `data:` frames, followed by `[DONE]`
**And** the captured `stop_reason` is `EndTurn` (no terminal error event)

### Scenario 2: Happy path — fit with seeded config (agent.prompt + PromptHook)

**Given** a fresh database with migration 005 applied
**And** `ANTHROPIC_API_KEY` is set
**When** a client POSTs `/api/fit` with a valid job description
**Then** the fit handler constructs a rig Agent with `model = "claude-sonnet-4-6"`, `max_tokens = 5530`, and `with_prompt_caching()` enabled on the underlying `CompletionModel`
**And** `agent.prompt(<job_description>).with_hook(StopReasonCapture)` is invoked
**And** the outbound non-streaming request body carries `cache_control: { "type": "ephemeral" }` on the system block per rig's `apply_cache_control`
**And** the `StopReasonCapture` hook fires once and stores `stop_reason = "end_turn"` into shared state
**And** the captured value maps to `StopReason::EndTurn`
**And** `extract_json` + `serde_json::from_str` parse the buffered response into a valid `FitVerdict`
**And** the response is `200 OK` with the verdict JSON

### Scenario 3: Config row missing — accessor falls back

**Given** a database where row `key='ai.model_id'` has been deleted
**When** a client POSTs `/api/chat`
**Then** `get_model_id` returns the compiled-in default `"claude-sonnet-4-6"`
**And** a `tracing::warn!` is emitted naming `ai.model_id` and the fallback path

### Scenario 4: Config value malformed

**Given** a database where row `ai.max_tokens` has `value = "not-a-number"`
**When** a client POSTs `/api/chat`
**Then** `get_max_tokens` returns the compiled-in default `5530`
**And** a `tracing::warn!` is emitted naming `ai.max_tokens` and the parse failure

### Scenario 5: Empty-string config value

**Given** a database where row `ai.model_id` has `value = ""`
**When** a client POSTs `/api/chat`
**Then** `get_model_id` returns the compiled-in default `"claude-sonnet-4-6"`
**And** a `tracing::warn!` is emitted

### Scenario 6: Whitespace-only / leading-trailing whitespace

**Given** a database where row `ai.model_id` has `value = "  claude-sonnet-4-6  "`
**When** a client POSTs `/api/chat`
**Then** `get_model_id` returns `"claude-sonnet-4-6"` (trimmed)
**And** no warn is emitted

### Scenario 7: max_tokens out of range — clamp behavior

**Given** a database where row `ai.max_tokens` has `value = "999999"` (above 200_000)
**When** a client POSTs `/api/chat`
**Then** `get_max_tokens` returns `200_000`
**And** a `tracing::warn!` is emitted naming the offending value and the clamp

### Scenario 8: Refusal stop_reason

**Given** the model returns a `refusal` stop reason
**When** the client POSTs `/api/chat`
**Then** the SSE stream emits `event: refusal\ndata: The assistant declined to respond to that request.` followed by `[DONE]`
**And** an `error!`-level log record is written server-side containing the raw model refusal text (truncated to 500 chars)
**And** the client never receives the raw model text

**And** when the same condition occurs on `/api/fit`:
**Then** the response is HTTP 422 with body `{"error":"refusal","message":"The assistant declined to respond to that request."}`
**And** the same server-side `error!` log is written

### Scenario 9: Context window exceeded

**Given** the model returns a `model_context_window_exceeded` stop reason
**When** the client POSTs `/api/chat`
**Then** the SSE stream emits `event: context_exceeded\ndata: The request exceeded the model's context window. Try a shorter input.` followed by `[DONE]`
**And** an `error!`-level log record is written server-side
**And** when the same condition occurs on `/api/fit`, the response is HTTP 413 with the canned `context_exceeded` body

### Scenario 10: Migration idempotency — runner skip is load-bearing

**Given** a database where migration 005 has been previously applied
**And** an operator has hand-edited `ai.model_id` to a different value (e.g., a future model name)
**When** the application restarts
**Then** the migration runner sees `_migrations` already records version 005 and SKIPS re-running 005
**And** the hand-edited value is preserved
**And** no error or warn is logged

**Additionally** as a belt-and-braces test:
**Given** a hypothetical scenario where the runner does re-run 005
**When** the SQL executes
**Then** `INSERT OR IGNORE` causes seed rows to be no-ops
**And** the hand-edited value is still preserved

### Scenario 11: Old model literal absent (post-merge sweep)

**Given** the change has been merged to main
**When** `rg "claude-sonnet-4-20250514"` is run across the entire repo, excluding:
  - `target/`, `node_modules/`, `data/`, `.worktrees/`
  - `docs/specs/2026-05-15-sonnet-migration-config-table.md` (this file — it records the migration history)
**Then** zero matches are returned

### Scenario 12: AI features disabled (no API key)

**Given** `ANTHROPIC_API_KEY` is unset (so `rig_client` is `None`)
**When** a client POSTs `/api/chat`
**Then** the existing `AppError::Internal("AI features not configured")` early-return fires
**And** `get_model_id` and `get_max_tokens` are NEVER called (they'd be wasted work)

### Scenario 13: rig-core bump verification

**Given** rig-core has been bumped from 0.33 to 0.37
**When** `cargo build --workspace` is run
**Then** the build succeeds with no errors and no new warnings (above existing baseline)
**And** all existing tests pass
**And** the new mock-Anthropic-HTTP tests confirm: outbound chat request carries `cache_control` on the system block; outbound fit request carries `cache_control` on the system block; chat handler reads `MessageDelta.stop_reason` from the SSE stream; fit `PromptHook` reads `raw_response.stop_reason` from the buffered `CompletionResponse`
**And** the manual smoke test of chat + fit (or equivalent live-API integration test, if available) confirms behavior parity against the real Anthropic endpoint

### Scenario 14: Stop_sequence stop_reason (success path)

**Given** the model returns `stop_reason = "stop_sequence"` (rare in folio today; folio passes no `stop_sequences`)
**When** the client POSTs `/api/chat`
**Then** the SSE stream emits all content received before the sentinel
**And** the captured `stop_reason` maps to `StopReason::StopSequence` and is treated as `EndTurn`-equivalent
**And** the stream terminates with `[DONE]` (no terminal error event)

**And** when the same condition occurs on `/api/fit`:
**Then** the `StopReasonCapture` hook stores `"stop_sequence"`
**And** mapping yields `StopReason::StopSequence`, treated as `EndTurn`-equivalent
**And** the buffered response is parsed as JSON and a successful verdict is returned

### Scenario 15: Max_tokens stop_reason (truncation path)

**Given** the model returns `stop_reason = "max_tokens"`
**When** the client POSTs `/api/chat`
**Then** the SSE stream emits all content received before truncation
**And** the stream emits `event: truncated\ndata: The response was cut off due to length.` followed by `[DONE]`
**And** a `tracing::warn!` is logged server-side naming `max_tokens` and the model

**And** when the same condition occurs on `/api/fit`:
**Then** a `tracing::warn!` is logged server-side
**And** the buffered response is passed to `extract_json` + `serde_json::from_str`
**And** if parse succeeds: HTTP 200 with the parsed verdict (the model output happened to fit before truncation)
**And** if parse fails: existing parse-error code path returns (no new variant added for this case)

### Scenario 16: Unrecognized stop_reason — Other variant

**Given** the model returns `stop_reason = "some_future_anthropic_string"`
**When** the client POSTs `/api/chat`
**Then** the value is sanitized (control chars stripped, CR/LF→space, ≤64 chars) and stored as `StopReason::Other`
**And** a `tracing::warn!` is logged naming the sanitized value
**And** the response is treated as `AppError::Internal` — chat emits the existing generic SSE error frame then `[DONE]`

**And** when the same condition occurs on `/api/fit`:
**Then** the same sanitization, warn log, and `AppError::Internal` (HTTP 500) path runs

### Scenario 17: SSE transport error from Anthropic (chat path)

**Given** Anthropic returns an SSE frame with `event: error` and data payload `{"type":"error","error":{"type":"overloaded_error","message":"<text>"}}`
**When** the client POSTs `/api/chat`
**Then** the SSE consumer deserializes the data payload into a typed `{ kind, message }` struct
**And** the kind is mapped to an appropriate `AppError` variant (e.g., `Internal("upstream overloaded")` for `overloaded_error`)
**And** the SSE stream emits the existing generic error frame then `[DONE]`
**And** the message field is sanitized per R17 before any server-side logging

## Data Model

**Entity: `site_config`**

| Field | Type | Constraints | Notes |
|-------|------|-------------|-------|
| `key` | TEXT | PRIMARY KEY, NOT NULL | Dotted namespace (`ai.model_id`, `ai.max_tokens`). |
| `value` | TEXT | NOT NULL | Stringified value; Rust accessors trim and parse. |
| `updated_at` | TEXT | NOT NULL, DEFAULT `datetime('now')` | ISO-8601 UTC. Updated on row replacement. |

**Seed rows (inserted by migration 005 via `INSERT OR IGNORE`):**

| key | value |
|-----|-------|
| `ai.model_id` | `claude-sonnet-4-6` |
| `ai.max_tokens` | `5530` |

**Accessor return-type contract:**

| Accessor | Returns | Fallback |
|----------|---------|----------|
| `get_model_id(&Connection)` | `String` (whitespace-trimmed) | `"claude-sonnet-4-6"` on missing/empty/error |
| `get_max_tokens(&Connection)` | `u32` (clamped to `[1, 200_000]`) | `5530` on missing/empty/parse-error |

## API Contract

No new HTTP endpoints. Existing endpoints unchanged in URL, request shape, and success response shape.

### `POST /api/fit` — new error responses

- **422 Unprocessable Entity** — model returned a refusal. Body: `{"error":"refusal","message":"The assistant declined to respond to that request."}`. Body is a canned static string; raw model text is logged server-side only.
- **413 Payload Too Large** — model returned context-window-exceeded. Body: `{"error":"context_exceeded","message":"The request exceeded the model's context window. Try a shorter input."}`. Same canned-string contract.

### `POST /api/chat` — new SSE event types

In addition to the existing data-event stream and `[DONE]` sentinel:

- `event: refusal\ndata: <canned message>` — terminal event before `[DONE]` when the model refuses.
- `event: context_exceeded\ndata: <canned message>` — terminal event before `[DONE]` when the model trips the context window.

The existing `event: error` frame remains the catch-all for unclassified errors.

## UI Behavior

Frontend changes scoped to SSE event-type recognition:

- The chat UI's SSE parser SHALL recognize `event: refusal` and `event: context_exceeded` and route them to a typed error path. They MUST NOT render as content text in the chat transcript (no AI-bubble styling, no avatar).
- Visual treatment of the new error types MAY mirror the existing `event: error` styling, OR MAY be visually distinct (designer's call). Either way, error treatment, not content treatment.

## Constraints

- Must integrate with existing `tower_http::services::ServeDir` static-serving model (per G-0002 — no asset embedding).
- Must integrate with the existing migration runner (the `_migrations` table + MIGRATIONS const array in `libs/site-core/db/schema.rs`).
- `AppState` shape may grow new fields; the `rig_client: Option<...>` semantics MUST NOT change.
- 90% line + function coverage hard floor on new code (per G-0001).
- Pre-commit gate `just check` (fmt + clippy `-D warnings` + test) MUST pass before merge.
- All work in a worktree under `.worktrees/`. Rebase-merge to main via `/merge` skill or PR (folio merge convention flipped 2026-05-15 to match mnemra-core).
- Per `<dependency-approval>`, the rig-core bump 0.33 → 0.37 is the only authorized dependency change. No other crates added.

## Operational Requirements

(Per Bolt v1 review — six categories.)

### Deployment Impact

- New binary, new migration. Standard `fly deploy` flow.
- `data/site.db` is gitignored and survives across deploys via the persistent volume. Migration 005 applies on first restart after deploy. New deploys to a fresh instance seed both rows; upgrade deploys also seed both rows because the table is new.

### State Changes

- Write paths: `_migrations` row (one INSERT per migration on first apply), `site_config` rows (two INSERTs on first apply, no further writes from this change).
- Migration is forward-only. Rollback strategy below.

### Health & Availability

- Existing `/healthz` (or equivalent) is unaffected. The DB-readiness check it already performs is sufficient — the new table is created within the same transaction guard.
- No new background tasks, no new ports, no new external dependencies at runtime.

### Observability

- New `tracing::warn!` events: missing config row, empty/malformed value, max_tokens clamp, `MaxTokens` stop reason (chat + fit), `Other(_)` unrecognized stop_reason (named with sanitized value), `StopSequence`/`PauseTurn` (info-level acceptable; warn if implementer prefers — these are normal for future tool use but unexpected for folio today).
- New `tracing::error!` events: SQL connection failure during config read, `Refusal` stop reason, `ContextExceeded` stop reason. The error logs include the **sanitized** raw model text (R17 — ANSI/control/CRLF stripped, 500 char cap), NOT the warn logs.
- No new metrics counters in this spec. (Future spec MAY add `chat_refusal_total` / `chat_context_exceeded_total` / `chat_truncated_total` counters; deferred.)
- **Tracing exposes the system preamble for the fit handler.** When fit's `CompletionModel` has `with_prompt_caching()` enabled, rig-core 0.37 records the system preamble in each completion's tracing span (`gen_ai.system_instructions` field). The chat handler under Path X does NOT route through rig's `CompletionModel`, so it does NOT emit this span automatically — implementer MAY add an equivalent manual span for parity, or accept that chat traces look different from fit traces. Folio's preamble contains profile data and instruction prompts (non-secret per project guardrails); operators forwarding tracing to external aggregators should treat preamble content as logged.

### Degradation Behavior

- If `ANTHROPIC_API_KEY` is absent → existing behavior preserved (R27): handlers return `AppError::Internal("AI features not configured")` before reaching config or model setup.
- If the SQL connection errors during config read → fall back to compiled-in defaults; log at error level. No request failure cascades from a config read failure.
- If rig-core returns a transient error → propagated as `AppError::Internal` (existing behavior). No retry layer added.
- If a stop_reason value is unrecognized → mapped to `StopReason::Other(String)` and treated as `AppError::Internal`. The Anthropic-documented strings are the recognized set; future Anthropic API additions degrade gracefully.

### Operational Test Scenarios (for QA / Glitch)

- `[chat with default seeded model] → success, content streamed`
- `[chat with hand-edited ai.model_id row] → success using edited value`
- `[chat with deleted ai.model_id row] → success using compiled-in default + warn log`
- `[chat with malformed ai.max_tokens row] → success using compiled-in default + warn log`
- `[chat hitting model refusal] → typed event:refusal, [DONE], canned message, server-side error log with truncated raw text`
- `[fit hitting context exceeded] → HTTP 413, canned body, server-side error log`
- `[no ANTHROPIC_API_KEY → both endpoints return existing 'AI features not configured' error]`
- `[migration runs once on fresh DB, skipped on subsequent boots]`

### Rollback Procedure

- **Fastest (no DB change):** `fly deploy --image <previous-sha>`. Reverts code; the `site_config` table remains in the DB harmlessly (unused by the previous code). No data corruption. This rollback also reverts the rig-core 0.33 → 0.37 bump and the handler architectural changes (Tasks 1 + 4 land in the same image); rollback is atomic at the image level.
- **Controlled (with DB change):** Issue a new migration `006_revert_site_config.sql` that DROPs the table, plus a code revert. Heavier but cleaner.
- **⚠ Controlled-rollback footgun on re-deploy.** If the controlled path is taken (migration 006 drops the table) and the new code is later redeployed, the migration runner sees `_migrations` already records version 5 and SKIPS re-running migration 005. The `site_config` table will be ABSENT but `_migrations` claims it was applied. Every config accessor call will hit the fallback path on every request (with a warn log on first call per accessor per process boot). Service runs on compiled-in defaults — degraded but not crashing.

  **Mitigation:** before redeploying new code after a controlled rollback, ship `007_restore_site_config.sql` in the upgrade chain that re-creates the table and seeds the rows. The forward path is: rollback (006 drops) → upgrade (007 restores) → new code. Do NOT redeploy new code without 007 in between.
- **In-flight model swap:** to swap models without a deploy, the operator can hand-edit `ai.model_id` in the DB (interim procedure below). To revert that swap, edit it back. No deploy needed.

### Interim Production Config-Edit Procedure (until admin UI ships)

`sqlite3` is not in the runtime image. To edit `site_config` in production:

1. **Preferred:** issue a new migration file `migrations/006_<reason>.sql` that `INSERT OR REPLACE` the row, and deploy. Auditable, reversible via revert.
2. **Emergency-only:** SSH into the Fly machine (`fly ssh console`), invoke `sqlite3` from a temporary install or via a pre-built shim binary. Operator-discretion; document the change in `decisions/` post-hoc.

---

## Verification asks for Forge

The pre-v3 rig-core 0.37 source investigation verified the bulk of the public surface with file:line citations. Three items remained unverified at investigation time. **Forge MUST verify each one before writing code that depends on it.** If any answer diverges from the v3 design assumptions, surface the divergence before continuing.

1. **Exact method name to attach a `PromptHook` to `PromptRequest`.** The hook trait shape and its `on_completion_response` signature are verified (`rig-core-0.37.0/src/agent/prompt_request/hooks.rs:12-90`); the method to *attach* a hook to a `PromptRequest` builder is not. Candidate names: `with_hook`, `hook`, `add_hook`. Verify against `rig-core-0.37.0/src/agent/prompt_request/mod.rs` before writing fit-handler code. If the attach API requires boxing (`Pin<Box<dyn ...>>`) rather than `impl PromptHook`, adjust the capture struct accordingly.

2. **Anthropic SSE `error` event payload shape.** The v3 chat-handler design needs to deserialize SSE frames where `event == "error"` into a typed `AnthropicSseError { kind: String, message: String }` rather than treating `data` as opaque text. Anthropic's documented shape is `{"type":"error","error":{"type":"<reason>","message":"<text>"}}`. Verify against an Anthropic doc page or rig test fixture. Map `kind` to existing `AppError` variants where possible (e.g., `overloaded_error` → `AppError::Internal("upstream overloaded")`); for `rate_limit_error`, prefer an existing `TooManyRequests`-shaped variant if folio has one, otherwise `Internal`. Sanitize the `message` field per R17 before logging.

3. **`rig::http_client::retry::*` default policy and `NoRetry` availability.** `GenericEventSource` defaults to `ExponentialBackoff` retries (`rig-core-0.37.0/src/http_client/sse.rs:75-80`). For a single-shot chat request, a mid-stream retry would re-issue the POST and the user would see content twice (or the partial first stream would be lost in favor of a duplicate from-scratch second stream). Folio chat is NOT idempotent at the model level (same prompt → potentially different output). Forge MUST explicitly disable retry on the chat SSE stream via `GenericEventSource::with_retry_policy(NoRetry)` or the equivalent API. Verify the exact method name and that a `NoRetry` (or `None`) policy exists. If the API has no such option, file a follow-up task documenting the workaround and confirm the decision with the spec owner before merging.

These three items gate chat-handler impl. Verify, then proceed.

---

## Tasks

### Task 1: Bump rig-core 0.33 → 0.37 + verify breaking changes

**Files:** `Cargo.toml` (workspace), `libs/site-core/Cargo.toml`, `Cargo.lock`, possibly any source file that hits a renamed/changed API.
**Type:** dependency
**Depends on:** None.

**What:** Bump rig-core in the workspace `Cargo.toml` to 0.37. Run `cargo build --workspace` and `cargo test --workspace`. Enumerate any breaking changes encountered (renames, signature changes, removed APIs) in the PR description. Touch only the minimum source needed to restore green build/tests; this task does NOT include the architectural unification — that's Task 4.

**Acceptance Criteria:**
- [ ] `Cargo.toml` shows `rig-core = "0.37"`.
- [ ] `cargo build --workspace` succeeds with no new warnings above baseline.
- [ ] `cargo test --workspace` is green.
- [ ] PR description has a "rig-core 0.33 → 0.37 breaking changes" section listing every API that needed adjustment, or "none" with a build-output citation.

**Test Expectations:**
- Existing test suite green is the gate. No new tests in this task.

---

### Task 2: Add `site_config` migration + register in MIGRATIONS array

**Files:** `migrations/005_site_config.sql` (new); `libs/site-core/db/schema.rs` (modify — add to MIGRATIONS const); `libs/site-core/tests/test_schema.rs` (modify).
**Type:** backend
**Depends on:** None.

**What:** Create the SQL migration per §Data Model and register it in the runner's MIGRATIONS array. Without registration, the migration is dead code. Extend the schema test to assert table presence, column shape, and seed-row values.

**Acceptance Criteria:**
- [ ] `migrations/005_site_config.sql` exists per §Data Model.
- [ ] `libs/site-core/db/schema.rs` MIGRATIONS array includes the new entry in the correct slot.
- [ ] On a fresh DB, after the migration runner runs, `SELECT key, value FROM site_config ORDER BY key` returns exactly two rows: `ai.max_tokens=5530` and `ai.model_id=claude-sonnet-4-6`.
- [ ] On a DB where the seed rows have been edited and `_migrations` records 005 as applied, restart leaves the edited values untouched (Scenario 10 main path).
- [ ] Test asserting the MIGRATIONS array entry exists by name.

**Test Expectations:**
- Schema test: table exists, columns + types + NOT NULL flags + PRIMARY KEY correct.
- Seed test: both rows present with expected values.
- Idempotency test (Scenario 10): runner-skip path AND `INSERT OR IGNORE` belt-and-braces path.
- Registration test: MIGRATIONS array contains a 005 entry.

---

### Task 3: Add typed config accessors at `libs/site-core/db/config.rs`

**Files:** `libs/site-core/db/config.rs` (new); `libs/site-core/db/mod.rs` (modify — register module); tests in same module or `tests/test_config.rs`.
**Type:** backend
**Depends on:** Task 2.

**What:** Implement `get_model_id(&Connection) -> String` and `get_max_tokens(&Connection) -> u32` per R3, R4. Both wrap a private `get_config_str_trimmed(&Connection, &str) -> Option<String>` raw accessor. Both apply the warn/error log split from R3, R4. `get_max_tokens` enforces the clamp from R4.

**Acceptance Criteria:**
- [ ] Module path is `libs/site-core/db/config.rs` (NOT `libs/site-core/config.rs` — that path is taken).
- [ ] `get_model_id` returns the seeded value on a fresh DB.
- [ ] `get_model_id` returns `"claude-sonnet-4-6"` on missing row, with `warn` log.
- [ ] `get_model_id` returns `"claude-sonnet-4-6"` on empty-after-trim value, with `warn` log.
- [ ] `get_model_id` strips leading/trailing whitespace from a non-empty value.
- [ ] `get_max_tokens` returns `5530` on a fresh DB.
- [ ] `get_max_tokens` returns `5530` on missing row, with `warn` log.
- [ ] `get_max_tokens` returns `5530` on empty/malformed value, with `warn` log.
- [ ] `get_max_tokens` clamps a value above 200_000 to 200_000, with `warn` log.
- [ ] `get_max_tokens` clamps `0` to `1`, with `warn` log.
- [ ] On simulated SQL connection error, both accessors fall back to defaults and log at `error` level.
- [ ] Neither accessor panics under any tested condition.

**Test Expectations:**
- Round-trip: insert, read, verify.
- Each fallback path: trigger the condition, assert default + correct log level.
- Boundary: `u32::MAX` value (gets clamped to 200_000), value exactly `200_000` (passthrough), value exactly `1` (passthrough), value `0` (clamped to 1).
- Whitespace: leading, trailing, both.

---

### Task 4: Implement chat SSE consumer + fit PromptHook + shared StopReason

**Files:**
- `libs/site-core/ai/stop_reason.rs` (new) — `StopReason` enum, `from_anthropic_str` mapping (with sanitization for `Other`), `StopReasonCapture` PromptHook struct, the sanitize-and-truncate helper for R13/R17 raw-text logging.
- `libs/site-core/ai/anthropic_stream.rs` (new) — chat handler's custom Anthropic SSE consumer per R11. Owns body construction (rig's pub `Message`/`Content`/`SystemContent`/`CacheControl` types), `GenericEventSource` plumbing, and the SSE event-match loop.
- `libs/site-core/ai/mod.rs` (modify) — register the two new modules.
- `libs/site-core/routes/ai.rs` (rewrite) — `chat_inner` calls `anthropic_stream::stream_chat(...)`; `fit_analysis_inner` keeps `agent.prompt(...).with_hook(StopReasonCapture::new())` (or equivalent — see Verification ask #1).
- `libs/site-core/error.rs` (modify) — add `AppError::Refusal(Option<String>)`, `AppError::ContextExceeded(Option<String>)`, custom `IntoResponse` arms per R16. Add helper `AppError::log_sanitized_error(...)` reusing the R17 sanitizer.
- `libs/site-core/tests/test_ai_endpoints.rs` (rewrite) — drop existing `rig_client: None`-only tests; add mock-Anthropic-HTTP coverage exercising both handlers end-to-end.

**Type:** backend
**Depends on:** Task 1, Task 3, all three Verification asks (must be answered before code).

**What:** Implement Path X per R11/R12/R12a/R13/R14/R15/R16/R17/R18. Chat handler runs over a custom SSE consumer atop `rig::providers::anthropic::Client::post_sse(...)`; fit handler stays on `agent.prompt(...)` with a `PromptHook<M>` capturing `stop_reason`. Shared `StopReason` enum (`#[non_exhaustive]`, 7 named variants + sanitized `Other(String)`). Cache_control is asymmetric: chat hand-rolls `CacheControl::ephemeral()` on the `SystemContent` block; fit uses `with_prompt_caching()` on the `CompletionModel`. New `AppError::Refusal`/`ContextExceeded` variants with canned client bodies per R16; sanitized server-side raw-text logging per R17. Chat SSE emits typed terminal events per R18 (`refusal`, `context_exceeded`, `truncated`).

**Acceptance Criteria:**

Architecture conformance:
- [ ] `rg "claude-sonnet-4-20250514" libs/ cmd/ frontend/ migrations/ tests/` returns zero matches.
- [ ] `rg "max_tokens\(4096\)" libs/ cmd/` returns zero matches.
- [ ] `rg "stream_prompt\|CompletionModel::stream\|completion_model\.stream" libs/site-core/routes/ai.rs libs/site-core/ai/anthropic_stream.rs` returns zero matches (chat does NOT use rig's streaming surface).
- [ ] `rg "anthropic::Client::post_sse\|client\.post_sse" libs/site-core/ai/anthropic_stream.rs` returns at least one match (chat uses Path X).
- [ ] `rg "agent\.prompt\|\.with_hook" libs/site-core/routes/ai.rs` shows fit using both (exact method name per Verification ask #1).
- [ ] `StopReason` enum is `#[non_exhaustive]` with exactly the 8 documented variants (7 named + `Other(String)`).

Security / data-handling (R15/R17/R29):
- [ ] `AppError::Refusal` returns HTTP 422 with the canned body; `AppError::ContextExceeded` returns HTTP 413 with the canned body. No raw model text in either body.
- [ ] Server-side `error!` logs for refusal/context-exceeded include the **sanitized** raw model text (ANSI/control/CRLF stripped, ≤500 chars).
- [ ] `rg` verification: `AppError::Refusal(text)` / `AppError::ContextExceeded(text)` payloads only flow into `tracing::error!` or `tracing::warn!`, never into `Json(...)`, `Event::default().data(...)`, `Sse`, or response builder calls.
- [ ] `StopReason::Other(value)` is constructed only via `from_anthropic_str`, which sanitizes the value before storage.

Behavior coverage:
- [ ] Chat SSE emits `event: refusal\ndata: <canned>` then `[DONE]` on refusal stop_reason.
- [ ] Chat SSE emits `event: context_exceeded\ndata: <canned>` then `[DONE]` on context_window_exceeded stop_reason.
- [ ] Chat SSE emits `event: truncated\ndata: <canned>` then `[DONE]` on max_tokens stop_reason.
- [ ] `StopReason::StopSequence` and `StopReason::PauseTurn` are treated as `EndTurn`-equivalent (no terminal error event for chat; successful JSON parse path for fit).
- [ ] `StopReason::Other(_)` emits `warn!` log with the sanitized value and falls through to `AppError::Internal`.

Cache_control verification:
- [ ] Mock-HTTP harness asserts: outbound chat request body has `cache_control: { "type": "ephemeral" }` on the `SystemContent::Text` block.
- [ ] Mock-HTTP harness asserts: outbound fit request body has `cache_control: { "type": "ephemeral" }` on the system block per rig's `apply_cache_control`.

Test rigor:
- [ ] All new tests exercise the path beyond the `rig_client: None` early-exit. Mock-Anthropic-HTTP stub drives end-to-end behavior assertions.
- [ ] Coverage: 90% floor on new code in `error.rs`, `routes/ai.rs`, `ai/anthropic_stream.rs`, `ai/stop_reason.rs`.

**Test Expectations:**

Glitch's red-phase tests SHALL cover (Given/When/Then names; full bodies in red-phase dispatch):

- `chat handler issues SSE POST via anthropic::Client::post_sse with model and max_tokens from config`
- `chat handler outbound body carries cache_control ephemeral on SystemContent block`
- `chat handler reads MessageDelta.stop_reason from the SSE stream and maps to StopReason variant`
- `chat handler emits event:refusal frame with canned data on refusal stop_reason`
- `chat handler emits event:context_exceeded frame with canned data on context_window_exceeded stop_reason`
- `chat handler emits event:truncated frame with canned data on max_tokens stop_reason`
- `chat handler treats stop_sequence as EndTurn-equivalent — no terminal error frame`
- `chat handler treats pause_turn as EndTurn-equivalent — no terminal error frame`
- `chat handler logs warn naming the sanitized value on Other stop_reason and returns Internal`
- `chat handler deserializes Anthropic SSE event:error data into typed AnthropicSseError and maps kind to AppError`
- `chat handler disables GenericEventSource retry — no double-stream on transient failures`
- `fit handler uses agent.prompt with StopReasonCapture hook`
- `fit handler outbound body carries cache_control ephemeral on system block via with_prompt_caching`
- `fit handler StopReasonCapture stores raw_response.stop_reason after await`
- `fit handler returns HTTP 422 with canned refusal body on refusal stop_reason`
- `fit handler returns HTTP 413 with canned context_exceeded body on context_window_exceeded stop_reason`
- `fit handler logs warn on max_tokens stop_reason and attempts JSON parse on buffered response`
- `server-side error log for refusal/context_exceeded contains sanitized raw model text (no ANSI no CR/LF) truncated to 500 chars`
- `client response body NEVER contains raw model text on any stop_reason`
- `StopReason::Other payload is sanitized at construction (control chars stripped, ≤64 chars)`
- `ANTHROPIC_API_KEY absent preserves existing AI-disabled behavior — config accessors never called, neither helper module is reached`
- `outbound chat request reused fresh rig::anthropic::Client from AppState — no new Client construction per request`

---

### Task 5: Frontend SSE parser update for new event types

**Files:** `frontend/src/lib/sse.ts` (or wherever the parser lives — implementer locates); chat UI component handling the parsed events; frontend tests covering the new event types; Playwright e2e tests under `e2e/`.
**Type:** frontend
**Depends on:** Task 4.

**What:** Update the SSE parser to recognize three new event types: `event: refusal`, `event: context_exceeded`, `event: truncated`. Route `refusal` and `context_exceeded` to the chat UI's error display path (existing `event: error` handler is acceptable; a distinct visual treatment is preferred but not required). `truncated` is informational rather than a hard error — route it to a notice/banner path or to the error path with distinct copy. None of the three event types MAY render as content text under any code path.

**Acceptance Criteria:**
- [ ] SSE parser recognizes all three new event types and exposes them to the chat UI as typed errors/notices.
- [ ] Chat UI does NOT render any of the three event types with AI-bubble styling.
- [ ] Frontend unit tests cover each new event type's parser routing.
- [ ] Playwright e2e tests under `e2e/` cover the new event types end-to-end (folio's e2e infra is Playwright per project CLAUDE.md — this is no longer conditional on test-infra availability).

**Test Expectations:**
- Frontend unit tests for the SSE parser: feed each new event type, assert correct dispatch.
- Component test for the chat UI: assert error/notice treatment (whatever shape) for each new event.
- Playwright e2e: trigger each new event type via the mock-Anthropic-HTTP harness (or a frontend stub if the e2e suite cannot reach the backend mock), assert the rendered DOM uses error/notice classes rather than the AI-bubble class.

---

### Task 6: Sweep stale references + update CLAUDE.md

**Files:** `CLAUDE.md` (modify), `README.md` (modify if it cites the model name), any other doc file the `rg` sweep finds.
**Type:** docs
**Depends on:** Task 4.

**What:** Final pass to remove stale references to the old model ID across non-spec documentation. Update `CLAUDE.md` `<current-phase>` to mark task #572 as done and note `site_config` as the canonical home for `ai.model_id` and `ai.max_tokens`. Add a brief note to `CLAUDE.md` `<tech-stack>` or `<entry-points>` mentioning the asymmetric AI architecture introduced by v3 (chat = `ai/anthropic_stream.rs` custom SSE consumer; fit = `agent.prompt` + `ai/stop_reason.rs::StopReasonCapture` hook), so future maintainers don't accidentally refactor chat back through rig's CompletionModel and lose `stop_reason` again. Verify the `<conventions>` line about rebase-merge still reads correctly post-merge.

**Acceptance Criteria:**
- [ ] No non-spec file references `claude-sonnet-4-20250514`.
- [ ] `CLAUDE.md` `<current-phase>` reflects #572 done and `site_config` table present.
- [ ] `CLAUDE.md` notes the chat/fit architectural asymmetry and the reason rig's CompletionModel is bypassed for chat (drops `stop_reason`).
- [ ] If README cites a model name, it cites `claude-sonnet-4-6` and references the config table as the runtime source of truth.
- [ ] `CLAUDE.md` `<conventions>` rebase-merge line is intact.

**Test Expectations:**
- Manual: `rg` sweep across the repo confirms cleanup. (Scenario 11.)
