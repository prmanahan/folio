<project>
Name: Folio
Project ID: 1
Slug: folio
</project>

<purpose>
Peter's personal portfolio and resume site. Showcases work, serves the resume, hosts AI chat + job-fit analysis features. Deployed to peter.manahan.io.
</purpose>

<status>
active
</status>

<landmines>
Drift-prone facts that have tripped Puck before. Verify against code, not README or memory, when in doubt.

- **Static file serving: `ServeDir`, NOT `rust_embed`.** The README currently claims "SPA baked into the binary at compile time via rust-embed", but the code uses `tower_http::services::ServeDir` (`libs/site-core/static_files.rs:9`). Per G-0002 (no compile-time asset embedding), the code is correct — the README is stale. **Fix dispatched 2026-04-13 (task pending).** Once README is updated, this landmine can be removed.
</landmines>

<guardrails>
Project-specific rules. These extend the DEFAULTS.md projection.

- **90% line + function coverage** (per G-0001). Hard gate before merge.
- **No `rust_embed` or compile-time asset embedding** (per G-0002). Static files served via `ServeDir`, Docker multi-stage build copies both binary and frontend assets.
- **Deployment: Fly.io.** `fly.toml` at project root. Never deploy without Peter's explicit approval.
- **`ADMIN_PASSWORD` required at startup by design.** The app panics if unset — this is intentional, not a bug. Local dev handled via `.env` file. Never add a default.
</guardrails>

<tech-stack>
- **Backend:** Rust (edition 2024), Axum 0.8, tower-http, rusqlite
- **Frontend:** SvelteKit (Svelte 5), Tailwind CSS v4, DaisyUI v5, Vite
- **Database:** SQLite at `data/site.db` (gitignored). Migrations auto-run at startup from `migrations/`.
- **AI:** rig-core with Anthropic Claude. Optional — disabled if `ANTHROPIC_API_KEY` is unset.
- **Testing:** `cargo test` (unit + integration), Playwright (e2e under `e2e/`)
- **Deploy target:** Fly.io (Docker multi-stage)
</tech-stack>

<entry-points>
All commands via `justfile`. Run from project root.

| Command | Purpose |
|---------|---------|
| `just run` | Build frontend + run backend (port 3000) |
| `just frontend-dev` | Vite dev server with hot-reload (port 5173, proxies to backend) |
| `just test` | Backend unit + integration tests |
| `just e2e` | Playwright end-to-end tests (requires app on :8080) |
| `just check` | Pre-commit: fmt + clippy (-D warnings) + test |
| `just docker` | `docker compose up --build` |
| `just release` | Release binary |

Module layout: `cmd/server/` (Axum entry), `libs/site-core/` (core logic, DB, routes, AI, static file serving), `frontend/` (SvelteKit), `migrations/` (SQL), `e2e/` (Playwright), `data/` (SQLite + seed SQL).

**Backend/frontend decoupling:** Since removing `rust_embed`, `cargo build` and `cargo test` (unit tests) run standalone without any frontend build step — no `include_str!`/`include_bytes!`/`build.rs` coupling to frontend assets. The only frontend dependency is at *runtime*: `validate_static_dir()` panics at startup if `STATIC_DIR` (default `frontend/build`) is missing, and `ServeDir` reads files live from that path. This is the correct G-0002 design.

**AI architecture is asymmetric (spec #572) — do not "unify" it:** the two AI paths deliberately take different routes.
- **chat** = `ai/anthropic_stream.rs` — a custom Anthropic SSE consumer (Path X). It bypasses rig-core's `CompletionModel` *on purpose: that path drops `stop_reason`*, which chat needs to map refusal/context-exceeded/truncated terminal conditions. Refactoring chat back through rig's `CompletionModel` silently loses `stop_reason` and reintroduces the bug this spec fixed.
- **fit** = `agent.prompt` + `ai/stop_reason.rs::StopReasonCapture` (a rig PromptHook) — fit stays on rig and recovers `stop_reason` via the hook instead of a custom SSE consumer.
</entry-points>

<conventions>
- **Test pyramid: inverted** (per G-0001). Heavy unit tests with mocks; integration tests only at real seams.
- **Coverage: 90%** hard floor on both line and function coverage.
- **Pre-commit: `just check`** — fmt + clippy with `-D warnings` + test must all pass.
- **Worktrees in `.worktrees/`** for all feature work.
- **Rebase-merge to main** via `/merge` skill or PR (matches mnemra-core; flipped 2026-05-15). Squash and merge-commits disabled at the GitHub repo level. No local merges to main.
</conventions>

<skills>
Skills Puck should load into dispatches targeting this project.

- `skills/rust.md` — Rust conventions
- `skills/rust-review.md` — Warden's Rust review checklist
- `skills/sveltekit.md` — SvelteKit conventions
- `skills/svelte-review.md` — Warden's Svelte review checklist
- `skills/tdd.md` — test-driven workflow for new features
- `skills/security.md` — for anything touching auth, admin, or env vars
</skills>

<current-phase>
State that changes as work progresses. Update when it drifts.

**As of 2026-05-15:**
- `main` clean, synced with `origin/main`.
- Active worktree: `.worktrees/spec572-impl` on branch `feat/572-sonnet-migration` — task #572 implementation.
- Stale local branches present (no worktrees): `feat/backend-coverage`, `feat/frontend-coverage`, `chore/defaults-g0006`, `fix/admin-notebook-phase3-4`, `fix/workflow-permissions`, `ultra` — needs triage cleanup pass.
- **Done:** task #572 — Sonnet 4 deprecation migration shipped (spec `docs/specs/2026-05-15-sonnet-migration-config-table.md`). Delivered: new `site_config` SQLite table (key/value, typed) — now the **canonical runtime source of truth** for `ai.model_id` and `ai.max_tokens` (read from the DB at request time, not hardcoded) + swap to `claude-sonnet-4-6` + rig-core 0.33 → 0.37 bump + chat handler rewrite to custom Anthropic SSE consumer (Path X, bypasses rig's CompletionModel which drops `stop_reason`) + fit handler PromptHook for `stop_reason` capture + typed refusal/context-exceeded error mapping.
</current-phase>

<references>
- **Project spec:** `spec.md` at project root — what folio is, goals, current state. Long-lived, serves both humans and AI.
- **Feature specs:** `docs/specs/*.md` — per-feature specs generated by the `/spec` skill (e.g., `2026-04-09-hero-redesign.md`). Short-lived, scoped to one feature.
- **Decisions:** `decisions/` (project-specific `P-NNNN-*.md`) and `decisions/DEFAULTS.md` (projected general standards).
- **README:** `README.md` — human-facing getting-started doc.
- **Fly config:** `fly.toml`
- **Docker:** `Dockerfile`, `docker-compose.yml`
</references>
