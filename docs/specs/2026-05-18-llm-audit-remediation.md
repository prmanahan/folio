# Spec: LLM-surface security-audit remediation

> **Spec for:** task #1066 (LLM10 + hardening) + task #1067 (LLM07 data-classification decision, resolved)
> **Date:** 2026-05-18
> **Status:** locked (maintainer signed off 2026-05-18 — no changes without sign-off; re-review only if implementation surfaces something unexpected against the verification asks)
> **Target:** folio (`libs/site-core`, `cmd/server`, frontend untouched)
> **Parent commit:** `293e468a94b98bc8638ae0db83f6ac92fa1e79cc`
> **Source:** LLM-surface security audit (OWASP LLM Top 10 2025), verdict *approve-with-conditions*, zero Critical/High. Instruction/data separation, output sanitization (DOMPurify), and the no-tools posture were verified structurally sound. This spec remediates the three conditions of approval plus the optional hardening.
> **Implementation pipeline:** spec (this doc, → locked) → red-phase test author writes failing tests → implementer makes them green → test-integrity re-review → security-review of the diff → `/merge`.

## Changelog

- **v1 (2026-05-18):** Initial. Transcribed from the audit findings; scope and the LLM07 disposition were decided by the maintainer before drafting.

## Decisions already made (do not reopen)

- **LLM07 disposition: option (a) — narrow classify + drop.** The three most-sensitive private fields are removed from the prompt entirely. Not a post-generation scrub, not accept-and-document. The DB still stores these fields (admin-managed data); only the prompt-construction path stops reading them.
- **Scope:** all of M1, M3, L1, L2, Nit, the residual error-string note, and LLM07(a). One implementation pass.
- **Salary exclusion stays as-is.** `COMPENSATION_DEFLECTION` and the absence of any salary interpolation are a correct existing control — do not touch.

## Requirements

Each requirement is `file:line` anchored to the audit. Line numbers are at the parent commit; treat them as starting points, not guarantees — the implementer locates the current site.

### R1 — LLM10: upstream/request timeout (audit M1)

**Problem.** The model call has no deadline. The fit path awaits `agent.prompt(...).await` (`routes/ai.rs:262`) with no `tokio::time::timeout`. The chat path drives `source.next().await` in an unbounded `while let` (`ai/anthropic_stream.rs:248`). No `tower_http` `TimeoutLayer` is installed in `cmd/server/main.rs`. A hung or slow upstream pins a connection (chat additionally pins a spawned tokio task) indefinitely, bounded only by the per-IP rate quotas.

**Fix.**
1. Wrap the fit await in `tokio::time::timeout(Duration::from_secs(N), agent.prompt(...))`; on elapsed, map to the existing degraded-response error surface (a 504-shaped variant or `AppError::Internal` with a canned client message — match the existing refusal/context-exceeded pattern, no raw upstream text to the client).
2. For chat, add **both** a per-event idle deadline (timeout around `source.next()`) **and** a total wall-clock cap on `run_chat_stream`. On expiry, emit the existing generic `event: error` + `[DONE]` and discard the SSE buffer (same path as refusal/context-exceeded).
3. Install a `tower_http::timeout::TimeoutLayer` in `main.rs` as the outer backstop.

`N` and the chat idle/total values: pick defaults sane for a portfolio chat/fit feature (single-digit to low-tens of seconds idle; total bounded well under any proxy/LB timeout). State the chosen values and the reasoning in the completion report.

**Acceptance.**
- A mock upstream that never sends a terminal frame causes the fit handler to return the degraded error within the configured bound (asserted on wall-clock, not just "the timeout value is configured").
- A mock upstream that stalls mid-stream causes the chat handler to emit `event: error` + `[DONE]` within the idle bound and discard the partial buffer (no partial model text reaches the client).
- The `TimeoutLayer` is present in the router construction.

### R2 — LLM10: lower the token-output ceiling (audit M3)

**Problem.** `MAX_TOKENS_MAX = 200_000` (`db/config.rs:33`). `get_max_tokens` clamps the admin-configured value to `[1, 200_000]` and passes it straight to the model as `max_tokens`. For this feature a 200k upper bound is a sky-high per-request cost/latency ceiling; a bad config row (or compromised admin session) can request a 200k completion.

**Fix.** Lower `MAX_TOKENS_MAX` to a value appropriate for chat/fit (target band **8_000–16_000**; pick one, justify in the report). `DEFAULT_MAX_TOKENS = 5530` is unchanged. The clamp then doubles as the LLM10 cost ceiling.

**Acceptance.**
- A `site_config` row requesting a value above the new ceiling clamps down to the new ceiling (existing clamp test updated to the new bound).
- `DEFAULT_MAX_TOKENS` unchanged; `MAX_TOKENS_MIN` unchanged.

### R3 — LLM07(a): drop the three most-sensitive private fields from the prompt

**Problem.** The prompt embeds private candidate context whose only confidentiality barrier is natural-language instruction (`SECURITY_BLOCK`, `DATA_CLASSIFICATION` in `ai/prompt_templates.rs`). Prompt-only confidentiality is bypassable by adversarial phrasing / injection / jailbreak, and the model can paraphrase the content even without quoting it. Disposition (a) removes the three most-sensitive fields from the prompt entirely.

**The three fields and their current prompt-interpolation sites** (parent commit):
- `would_do_differently` — experience field, interpolated at `ai/context.rs:169`
- `manager_would_say` — experience field, interpolated at `ai/context.rs:181`
- `honest_notes` — **skill** field (`models/skill.rs:44`), interpolated at `ai/context.rs:210`

**Fix.** Remove every prompt-interpolation site of each of the three fields from the context-construction path. Do **not** remove the struct fields, the DB columns, or the admin CRUD — only the prompt-construction code stops reading them. If a future grep finds an additional interpolation site for any of the three, remove that too; the acceptance test (not the line list) is the authority.

**Acceptance — phrased as data-absence, not field-name-absence.**
- With a unique sentinel string stuffed into each of `experience.would_do_differently`, `experience.manager_would_say`, and `skill.honest_notes` in a test fixture, the fully constructed prompt string contains **none** of the three sentinels (and no high-similarity span of them).
- The prompt-construction code path does **not read** `exp.would_do_differently`, `exp.manager_would_say`, or `s.honest_notes` — asserted structurally (the field accessors do not appear on the context-construction path), not only by string search on the output.
- Fields that stay (`why_joined`, `why_left`, `actual_contributions`, `proudest_achievement`, `challenges_faced`, `lessons_learned`, `reports_would_say`, `title_progression`, `quantified_impact`) are unaffected — a regression fixture confirms a retained field's sentinel still appears.
- The natural-language `DATA_CLASSIFICATION` block may reference the dropped field labels harmlessly; updating its example list is optional polish, not required.

### R4 — LLM10: `"unknown"` IP fallback collapses no-header callers into one bucket (audit L1)

**Problem.** When the trusted header is unset and `x-forwarded-for` is absent, `extract_ip` returns the literal `"unknown"` (`routes/ai.rs:87`, mirrored `middleware/global_rate_limit.rs:87`); all such callers share one rate-limit bucket. Fails *safe* for throttling but a single no-proxy client can be DoS'd by another, and many distinct attackers behind a missing-header config share one quota. Production uses `routes_with_connect_info()` so prod is mostly fine — this is defense-in-depth for the non-`ConnectInfo` path.

**Fix.** When neither header resolves, prefer the `ConnectInfo` peer address where it is available on the handler (it is already passed as `fallback` for chat at `routes/ai.rs:74`); fall back to `"unknown"` only when no peer addr is available. Apply consistently to the mirrored site in `global_rate_limit.rs`.

**Acceptance.** With both headers absent but a `ConnectInfo` peer addr present, two distinct peer addrs get distinct rate-limit buckets; `"unknown"` is used only when no peer addr is available.

### R5 — Explicit `DefaultBodyLimit` (audit L2)

**Problem.** No `DefaultBodyLimit` configured; relies on axum's 2 MB default. Handler-level caps (chat 10_000 chars `routes/ai.rs:96`, fit 15_000 chars `routes/ai.rs:217`) reject oversized input only *after* full parse, so ~2 MB can be shipped before the post-parse length check returns 400.

**Fix.** Add an explicit `axum::extract::DefaultBodyLimit::max(64 * 1024)` (or a per-route limit on `/api/chat` and `/api/fit`) so the byte cap aligns with the semantic cap and rejection happens pre-allocation. Pick the byte value with headroom over the 15_000-char fit cap; justify in the report.

**Acceptance.** A request body above the configured byte limit is rejected before handler parse (413/400 as axum produces); a normal-sized request is unaffected.

### R6 — `CORS_ORIGIN` fail-loud in production (audit Nit)

**Problem.** CORS origin silently defaults to `http://localhost:3000` when `CORS_ORIGIN` is unset (`cmd/server/main.rs:118-124`). Not a vuln (default is restrictive) but a missing prod env var yields a wrong-origin policy that breaks the site rather than failing loud.

**Fix.** Match the existing fail-loud pattern: require `CORS_ORIGIN` in production the way `ADMIN_PASSWORD` is required, or at minimum emit a `WARN` like `PAGE_HIT_SALT` does (`config.rs:21-24`) when it falls back to the default. Prefer the require-in-prod form; if that risks breaking local/dev ergonomics, use the WARN form and state the choice in the report.

**Acceptance.** In a production configuration with `CORS_ORIGIN` unset, the server either refuses to start with a clear message or logs a WARN naming the missing var (per the chosen form). Local/dev behavior unchanged.

### R7 — Opaque client strings for `AppError::Internal` on AI paths (audit residual note)

**Problem.** `AppError::Internal(msg)` maps to a 500 with `{"error":"<msg>"}` (`error.rs:57-61`). Several AI paths build `msg` from upstream/serde error text: `format!("AI prompt failed: {e}")` (`routes/ai.rs:265`), `format!("Failed to parse AI response as FitVerdict: {e}")` (`routes/ai.rs:338`), `format!("failed to serialize chat body: {err}")` (`ai/anthropic_stream.rs:172`). These reflect library/serde error strings (not raw model text, not the API key, not a stack trace) to the client. Low leak value and the frontend already remaps 500 to a generic message, but tightening closes the gap.

**Fix.** For these AI-path `AppError::Internal` sites, send an opaque generic client string while keeping the detailed error server-side via `tracing` (log at `error!`/`warn!` with the full `{e}`, return a fixed client message). Do not change `AppError::Internal`'s general contract — scope strictly to the three AI-path call sites named above and any sibling in the same paths the implementer finds.

**Acceptance.** The three named sites return a fixed opaque client string; the detailed error appears in server logs; no upstream/serde text appears in the HTTP response body. Existing frontend 500-remap behavior still works.

## Out of scope

- Full-repo audit (this was an LLM-surface audit only).
- The transitive `RUSTSEC-2026-0097` rand unsoundness warnings — informational, not reachable from folio's HTTP-REST AI paths; flag for a future dependency bump when upstream resolves, do not address here.
- LLM07 options (b) post-gen scrub and (c) accept-and-document — explicitly not chosen.
- Any change to instruction/data separation, DOMPurify output sanitization, or the no-tools posture — verified sound, leave alone.

## Verification (the implementer runs all of these locally before reporting done)

1. `cargo build` — workspace compiles.
2. `cargo test` — full suite green, including the new red-phase tests.
3. `cargo clippy` — no new warnings on touched crates.
4. `just fmt` (format-on-save is hook-owned; the implementer runs it to confirm a clean tree, does not hand-format).
5. `cargo audit --file Cargo.lock` — still vuln-clean (the two allowed rand unsoundness warnings are expected and unchanged).

**Build-capability check (explicit instruction to the implementer):** if `cargo build` or `cargo test` returns a *sandbox/permission* error (e.g. "Operation not permitted", denied write, blocked process) rather than a genuine *compile/test* failure, **STOP and report** — do not attempt to work around it, do not hand-wave the verification as "structurally fine". This is a deliberate probe of the dispatch's build capability; an honest "the build step was blocked by the sandbox" is the correct output in that case, not a worked-around green.

## Notes for the red-phase test author

- R1 timeout tests must assert on **observed behavior under a stalling mock upstream within a wall-clock bound**, not on "the timeout constant is set to N". A test that only checks the configured value passes against a no-op implementation.
- R3 is the highest-risk assertion-gaming surface. The strong test stuffs **unique sentinels** into the three fields via fixture and asserts sentinel-absence in the constructed prompt **plus** structural absence of the field reads on the construction path. A weak test that asserts `"manager_would_say" not in prompt` (the label, not the data) is insufficient and must not be the gate.
- R2/R5/R6 are straightforward boundary tests; R4 needs two distinct peer addrs to prove bucket separation; R7 asserts response-body opacity while the detail is present in captured logs.
