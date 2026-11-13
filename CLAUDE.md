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

<guardrails>
Project-specific rules. These extend the DEFAULTS.md projection.

- **90% line + function coverage is the target, not a gate.** Measured by `just backend-coverage` and `just frontend-coverage`; no recipe or CI step fails on a coverage figure (#2750).
- **No `rust_embed` or compile-time asset embedding.** Static files served via `ServeDir`, Docker multi-stage build copies both binary and frontend assets.
- **Deployment: Fly.io.** `fly.toml` at project root. A merge to `main` deploys to Fly via CI (`.github/workflows/deploy.yml`: tests, then `flyctl deploy`, then health and origin-closure checks; no path filter). Auto-deploy stands (Peter, 2026-10-25).
- **`ADMIN_PASSWORD` required at startup by design.** The app panics if unset — this is intentional, not a bug. Local dev handled via `.env` file. Never add a default.
</guardrails>

<tech-stack>
- **Backend:** Rust (edition 2024), Axum 0.8, tower-http, rusqlite
- **Frontend:** SvelteKit (Svelte 5), Tailwind CSS v4, DaisyUI v5, Vite
- **Database:** SQLite at `data/site.db` (gitignored). Migrations auto-run at startup from `migrations/`.
- **AI:** rig-core with Anthropic Claude. Optional — disabled if `ANTHROPIC_API_KEY` is unset.
- **Runtime config:** the `site_config` SQLite key/value table is the source of truth for `ai.model_id` and `ai.max_tokens`, read at request time.
- **Testing:** `cargo test` (unit + integration), `vitest` (frontend unit), Playwright (e2e; see `<gates>`)
- **Deploy target:** Fly.io (Docker multi-stage)
</tech-stack>

<entry-points>
All commands via `justfile`. Run from project root.

| Command | Purpose |
|---------|---------|
| `just run` | Build frontend + run backend (port 3000). Needs `frontend/build/projects.json` first: `GITHUB_TOKEN=<a token> just fetch-projects` |
| `just frontend-dev` | Vite dev server with hot-reload (port 5173, proxies to backend) |
| `just test` | Backend unit + integration tests |
| `just e2e` | Root Playwright suite over `e2e/` (starts Vite on :5173 itself; see `<gates>`) |
| `just check` | Pre-commit: fmt + clippy (-D warnings) + test |
| `just docker` | `docker compose up --build` |
| `just release` | Release binary |

Module layout: `cmd/server/` (Axum entry), `libs/site-core/` (core logic, DB, routes, AI, static file serving), `frontend/` (SvelteKit), `migrations/` (SQL), `e2e/` (Playwright), `data/` (SQLite + seed SQL).

**Backend/frontend decoupling:** `cargo build` and `cargo test` (unit tests) run standalone without any frontend build step — no `include_str!`/`include_bytes!`/`build.rs` coupling to frontend assets. The only frontend dependencies are at *runtime*: `validate_static_dir()` panics at startup if `STATIC_DIR` (default `frontend/build`) is missing, and the server refuses to start without a valid `frontend/build/projects.json` (a local run first needs `GITHUB_TOKEN=<a token> just fetch-projects`, then a frontend build). `ServeDir` reads files live from that path. This is the intended design.

**AI architecture is asymmetric (spec #572) — do not "unify" it:** the two AI paths deliberately take different routes.
- **chat** = `ai/anthropic_stream.rs` — a custom Anthropic SSE consumer (Path X). It bypasses rig-core's `CompletionModel` *on purpose: that path drops `stop_reason`*, which chat needs to map refusal/context-exceeded/truncated terminal conditions. Refactoring chat back through rig's `CompletionModel` silently loses `stop_reason` and reintroduces the bug this spec fixed.
- **fit** = `agent.prompt` + `ai/stop_reason.rs::StopReasonCapture` (a rig PromptHook) — fit stays on rig and recovers `stop_reason` via the hook instead of a custom SSE consumer.
</entry-points>

<conventions>
- **Test pyramid: inverted.** Heavy unit tests with mocks; integration tests only at real seams.
- **Pre-commit: `just check`** — fmt + clippy with `-D warnings` + test must all pass.
- **Worktrees:** harness-managed, under `.claude/worktrees/`, for all code work.
- **Rebase-merge to main** via `/merge` skill or PR (matches mnemra-core; flipped 2026-05-15). Squash and merge-commits disabled at the GitHub repo level. No local merges to main.
</conventions>

<skills>
Puck plugin skills to load into dispatches targeting this project.

- `review-core` + `rust-review` — Rust review
- `review-core` + `svelte-review` — Svelte/SvelteKit review
- `security` — anything touching auth, admin, or env vars
- `dev-loop` — test-first work (red-phase tests, assertion strength)
</skills>

<gates>
What green means, per surface. CI is GitHub Actions: `test.yml` on PRs to `main` and pushes to `main`; `deploy.yml` on pushes to `main` (the same test job, then deploy).

- **Backend:** `just check` from the root (fmt --check, clippy --all-targets -D warnings, cargo test). It covers no frontend code. There is no `ci` recipe.
- **Frontend:** from `frontend/`, `npm run check` (svelte-kit sync + svelte-check) and `npm run test:unit` (vitest run). CI runs both, plus `npm run build` and `cargo test --workspace`. `just check` and `just test` are cargo only.
- **Playwright runs in no CI job.** Two separate suites:
  - `npm run test:e2e` from `frontend/` runs `frontend/e2e/ai-pane.spec.ts` only, chromium.
  - `just e2e` from the root runs `playwright.config.ts` over `e2e/` (7 specs plus `fixtures.ts`) on chromium, firefox and webkit. One spec: `npx playwright test e2e/<file>`.
  - Both start Vite on :5173 themselves (`webServer`); Vite proxies `/api` to the backend on :3000. :8080 is the Fly/Docker port, not the e2e target.
  - `e2e/admin.spec.ts` posts `/api/admin/login` and needs the backend running; the other specs run against mocked API routes.
  - `e2e/accessibility.spec.ts` runs axe with a mocked API and fires in no automated gate.
</gates>

<references>
- **Project spec:** `spec.md` at project root — what folio is, goals, current state. Long-lived, serves both humans and AI.
- **Feature specs:** `docs/specs/*.md` — per-feature specs (e.g., `2026-04-09-hero-redesign.md`). Short-lived, scoped to one feature.
- **Decisions:** `decisions/DEFAULTS.md` (projected general standards); no project-specific decision files are tracked.
- **README:** `README.md` — human-facing getting-started doc.
- **Fly config:** `fly.toml`
- **Docker:** `Dockerfile`, `docker-compose.yml`
</references>
