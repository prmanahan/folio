---
status: draft
date: 2026-09-14
task: 3554
spec_type: code
reviewed_by: null
reviewed_date: null
---

# Spec: private content bundle — job builds it, folio's server loads it via a push separate from code deploys

> **Spec for:** task #3554 — the server-side content bundle: its contract, the import path, what the server does with it and without it, and the removal of the surfaces it replaces.
> **Date:** 2026-09-14 (last revised 2026-11-09)
> **Status:** draft — parked 2026-11-09; not ready to build; blocking findings are recorded on task `#3554`
> **Target:** folio (`libs/site-core`, `cmd/server`, `justfile`, `ops/token.just` (new), `contracts/` (new), `.github/workflows/`, `.githooks/` (new), `.dockerignore`, `.gitignore`, `Dockerfile`, `docker-compose.yml`, `fly.toml`, `CLAUDE.md`; frontend AI pane behaviour only)
> **Measured against:** `main` at `e9fe1d8ec4c2b9ea049934c4a403914f6048bf98` (2026-11-09). That tree carries `#3558` (`libs/site-core/app.rs` composes the router; `libs/site-core/middleware/origin_lock.rs` exists; migration `007` exists) and `#4236`. Facts dated 2026-11-09 were read from that commit. Line references marked "at the parent commit" name `e4ece8ed09a52d07a949e1231d0a370d4c222a66`; the implementer re-anchors any that remain at dispatch.
> **Frame:** `docs/intent/folio-simplification-frame.md` @ blob `1e30979eeb4ef863b2458a8b10d3dfe56941251d` (`git rev-parse HEAD:docs/intent/folio-simplification-frame.md`; locked; L-1…L-16, QA-F1…F13). The pin is a blob because a commit SHA does not survive this branch's rebases. The frame's only edits since its lock are MEASURED annotations to L-11 and QA-F7 and the removal of one job detail from its facts paragraph (`10695`, applied to the frame by Puck), with no lock changed (blob before them: `472b60b7765c5f45d0e2aace591538b2b71b8680`). This spec is closed against that frame. Nothing outside it enters the requirements; a gap returns to the frame, it is not filled here.
> **Rulings applied, by activity id** (all on `#3554` unless stated; every one re-read in this revision via `brain activity list --json`): `5217` (decisions 1–4), `5221` (5–8), `5224` (9, 10, 12), `5228` (11, 13, 14), `5331`, `5365`, `5404` (E-2: `subtle` direct; one-axis tier rule, superseded), `5419` (two-axis tier rule, supersedes `5404`'s tier half), `10632` (close-out scope and E-7, E-8, E-9; the option picks and the event-relative window are Peter's, the rest of each option's text is this spec's), `10695` (public-text detail removed; `5221` (8) extended to `reason` and `source`), `5311` on `#3470`, `5348` on `#3558` (cited only where it changes a requirement).
> **Self-consistency pass (standing step):** every acceptance criterion is walked against the artifacts this document specifies, and every mechanism's outside dependency (a man page, a CLI flag, a file in the tree, git history, how production is wired) is measured against this tree or the tool. What the walk changed is marked **[SC]**. A mechanism whose outside dependency could not be measured is not locked.
> **Source:** the frame + the rulings + code verification at the parent commit and, where dated 2026-11-09, at `e9fe1d8`. Tags such as `(Warden F-8)` name review findings; the reports are in `brain/projects/folio/`. Requirement keywords MUST / MUST NOT / SHOULD / MAY are RFC 2119. Evidence labels: **RULED** = Peter's words or a recorded ruling, by message number or activity id; **MEASURED** = checked at the parent commit or against a live CLI in a named dispatch; **INHERITED** = taken from a record and not re-checked; **INFERRED** = reasoning; **LEAN** = an item the `#3554` record marks "PUCK'S LEAN, not ruled".
> **Canon:** anchors cite `puck-0.7.40/plugin/canon` values (`V-*`), principles (`P-*`) and this repo's `decisions/DEFAULTS.md` (`G-*`, the project tier beneath canon — folio's `G-0003` is not canon's `G-0003`). Every cited value and principle resolves to a file in that release (checked 2026-11-09); those whose text differs from the release earlier revisions cited (`P-MinBlastRadius`, `P-TDDPairs`, `P-TrustThenRetro`, `V-Security`) and those newly leaned on here were read in this revision.
> **Public-repo rule (amended 2026-09-27; Puck's decision, applied here).** `#3558` is cited by number and nothing here states what the weakness *was*. What this document **may** describe is the shipped code's **health exemption** as its own doc comment describes it (`libs/site-core/middleware/origin_lock.rs:10-16`): that the outermost layer answers `GET /api/health` itself, without calling the next layer, through `routes::health_check_body`. That is the one property R-0011 depends on, and it is described nowhere else. Everything else about the fix — what it keys on, what it rejects, why — stays undescribed. **Superseded wording, kept with the reason it lost:** revision 2 read "no behaviour, keying detail or reachability property of the fix is described. The fix is written and reviewed but not deployed at the time of writing." Its premise lapsed: `#3558` deployed to production on 2026-09-17, and `origin_lock.rs` with its doc comment has been on the public `origin/main` since commit `99b4fd8` (MEASURED 2026-09-27: `git ls-tree origin/main libs/site-core/middleware/` lists it; `git branch -r --contains 99b4fd8` → `origin/main`). Under the old wording R-0011 could not name the function production actually serves health from, and an implementer following R-0011 literally would have shipped a health body production never serves (Bolt round-3 Critical; Warden W3-7, W3-14).
> **Implementation pipeline:** spec (this doc → locked at maintainer sign-off) → red-phase test author (security boundaries: R-0004, R-0005, R-0006, R-0007, R-0008, R-0014) → implementer → test-integrity review → code + security review of the diff → pre-merge operational gate (R-0019) → PR → merge (deploys) → post-deploy verification (R-0019).

## Frame directions this spec applies

The frame is the closed world; this section is a pointer table, not a restatement.

| Frame lock | Carried by |
|---|---|
| L-2 field-level public/AI-only split (RULED `5331`); mechanisms: a tracked pre-push hook (R-0014 layer 3) AND job's generator emits only schema-public fields (RULED `5217` (2)) | R-0002, R-0003, R-0014 |
| L-3 the server reads only the bundle; self-contained | R-0009 |
| L-5 private content never in repo or artifact; **a file on the volume** (RULED `5331`) | R-0008, R-0013 |
| L-6 two release streams | R-0005, R-0012, R-0019 |
| L-7 fork A (RULED `5229`) — everything here sits on Fly under one origin; the Astro page-side spec (`#3625`) owns the page half | Out of scope |
| L-8 admin, login, sessions, login limiter, CRUD removed; push token (RULED `5123`, `5217` (1), `5224` (9), `5228` (11)); **one bearer token, one actor, no second condition on a push that rewrites the prompt** — separation of privilege is absent by ruling and recorded as accepted (threat model) | R-0006, R-0007, R-0017 |
| L-9 three tiers — public / unpublished / private — rendered explicitly (RULED `5217` (4)); per-field rule RULED `5419` | R-0002, R-0009 |
| L-10 articles: summaries and links; mnemra.dev only | R-0002, R-0004, R-0009 |
| L-11 (a) private absent → public fallback; (b) no bundle → chat and fit unavailable, health degraded, pages unaffected (RULED `5331`) | R-0010, R-0011 |
| L-12 own checker, no schema library (RULED `5221` (5)); generation in tests, nothing new in the binary, `schemars` untouched (RULED `5331`, `5365`) | R-0003, R-0004 |
| L-13 `#3558` lands first; bundle release before Astro; open-ended overlap accepted (RULED `5217` (3), `5224` (10)) | R-0017, R-0019 |
| L-14 job's second copy is an encrypted off-machine backup (RULED `5224` (12)) | R-0020 |
| L-16 `agents` are page files; `site_config` stays in the DB (RULED `5311`) | R-0002 (absent list), R-0017 |

**Measured facts this spec rests on** (at the parent commit unless dated):

- `build_system_prompt` (`libs/site-core/ai/context.rs:31-349`) reads ten loaders and interpolates the field set in R-0002; `build_fit_prompt` (`:355`) wraps it. Values are interpolated raw, with no delimiting; `career_narrative` renders before `SECURITY_BLOCK` (`:67`, `:72`). **MEASURED** (Warden W-08).
- `DATA_CLASSIFICATION` (`libs/site-core/ai/prompt_templates.rs:43-49`) names five private labels: "Why joined", "Why left", "Manager would say", "Reports would say", "Honest notes" — two of them are the Tier-A drops that are not in the bundle, leaving three that are. **MEASURED** (Warden W-09).
- Not interpolated today: `would_do_differently` (`:169`), `manager_would_say` (`:181`), `honest_notes` (`:209`); salary fields never (`:343`). Every published article's full `content` is appended (`:326-340`). **MEASURED.**
- Server-wide request-body cap 64 KiB (`libs/site-core/app.rs:113`), guarded by `libs/site-core/tests/test_llm_audit_hardening.rs:397`; the AI router applies its own cap (`routes/ai.rs:40, :385, :395`). **MEASURED 2026-11-09.**
- The global limiter runs `extract_token` and an `admin_sessions` lookup under the DB mutex on every request carrying `Authorization: Bearer`, before the limit check (`middleware/global_rate_limit.rs:190-197`, `auth.rs:72`). **MEASURED 2026-11-09** (Warden W-06, F-15).
- `schemars = "0.8"` is a workspace dependency used by the fit types (`Cargo.toml:27`, `libs/site-core/Cargo.toml:22`, `ai/types.rs:1`). `sha2` is direct; `subtle v2.6.1` reaches `folio` on normal edges via `sha2 → digest`, `argon2` and `rustls` (Warden F-27 MEASURED, `cargo tree -e normal -i subtle`). No JSON Schema validator crate and no archive crate is in `Cargo.lock`. **MEASURED 2026-09-17.**
- `schemars 0.8.22` emits, for these types, `$schema`, `definitions`/`$ref`, `items` for every `Vec<T>`, `format` for numeric types, and array-valued or `anyOf` types for `Option<T>` and unions (Warden F-1 MEASURED from `gen.rs`, `schema.rs`). **INHERITED**; R-0003 derives the keyword set from the real output rather than fixing it here.
- **Health is produced by one function on two paths (MEASURED 2026-11-09 at `e9fe1d8`).** `routes::health_check_body(&DbState) -> (StatusCode, &'static str)` (`libs/site-core/routes/mod.rs:25-40`) returns the text `ok` / `db unavailable`; the routed handler `routes::health_check` (`:43-45`) calls it; and the outermost layer's health exemption (`libs/site-core/middleware/origin_lock.rs:82-84`) calls it too and returns without calling `next`, so **in production every `GET /api/health` is answered by that function through the exemption, never by the router**. `build_app` (`libs/site-core/app.rs:75-129`) wires the route at `:98` and the outermost layer at `:124-128`; `tests/test_origin_lock.rs:13,28` already builds through `build_app`. `fly.toml:23-29` and `deploy.yml:98` check HTTP status only. `middleware/global_rate_limit.rs` has no path exemption and allows 60 requests per 60 seconds (`:215-216`).
- `deploy.yml` runs on every push to `main` with no path filter (`on: push: branches: [main]`, `:18-20`); its last-touching commit is `99b4fd8` (2026-09-16), so `main`'s head is later than that commit. **MEASURED 2026-11-09.**
- The system block is sent with `"cache_control": {"type": "ephemeral"}` on chat (`libs/site-core/ai/anthropic_stream.rs:153`, tested at `:668`) and on fit through rig (`libs/site-core/routes/ai.rs:242`). Visitor text never enters the system block: chat passes `payload.message` separately (`routes/ai.rs:140`); fit passes the job description via `.prompt(...)` (`:259`) and the system prompt via `.preamble(...)` (`:248`). **MEASURED 2026-09-27.**
- `Dockerfile:67-69` copies `cmd/`, `libs/`, `migrations/` into the Rust builder; runtime copies the binary and `frontend/build/` (`:83-84`). `.dockerignore` excludes `data/*.db`, `docs/`, `*.md`, `tests/`, `.claude/`, `justfile`. `docker-compose.yml` passes `PORT`, `DATABASE_URL`, `ADMIN_PASSWORD`, `ANTHROPIC_API_KEY` only. **MEASURED 2026-09-17**; `Dockerfile` anchors 2026-11-09.
- `.env.example` ships `ADMIN_PASSWORD=changeme`; `config.rs:32` does not refuse it; `config.rs:19` loads `.env` unconditionally. **MEASURED.**
- **`libs/site-core/db/fixtures/live_profile.json` is tracked, embedded by `include_str!` at `db/seed.rs:68` for the `live` seed mode (`seed.rs:142`), and carries Peter's real email and phone** (Warden F-17; **MEASURED 2026-09-17**, values not repeated here). **PII baseline over every tracked file (MEASURED 2026-11-09 at `e9fe1d8`, `git grep -P` over all 236 tracked paths minus R-0015's three exclusions, values not recorded):** the phone regex `\b\d{3}[.\-\s]\d{3}[.\-\s]\d{4}\b` hits exactly four lines — `e2e/fixtures.ts:19` and `libs/site-core/db/seed.rs:20` (both the fictional `555-01xx` exchange), `e2e/hero-overflow.spec.ts:28` (a real number) and `libs/site-core/db/fixtures/live_profile.json:7` (real). The email regex `[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}`, ignoring `example.com`/`example.org`, hits `Cargo.toml:11` (the `noreply@anthropic.com` co-author address), `e2e/hero-overflow.spec.ts:25, :44` and `live_profile.json:4`. The two tracked image files, `frontend/static/favicon.png` and `frontend/static/textures/noise.png`, match neither regex when read as raw bytes. Tracked `*.json`/`*.yml`/`*.yaml` files carry **none** of the non-public field names as keys today, and no tracked `*.md` frontmatter does (**MEASURED 2026-11-09** at `c4b5235`, positive control: the same key extractor finds a non-public key in a synthetic document). **[SC]**
- Migrations `001`–`007` exist (`007_ai_hourly_ceilings.sql`). **MEASURED 2026-11-09.**
- `core.hooksPath` is unset (`#3569`, `#4547`, done 2026-11-09): `git rev-parse --git-path hooks` gives `.git/hooks`, which holds 15 samples and no live hook. **MEASURED 2026-11-09.** CI (`test.yml`) runs on `push` and `pull_request`, after a branch is on GitHub, and runs no `just` recipe. **INHERITED** (Warden W-04, Bolt C1).
- With `HOME` pointed at a temp directory, `security list-keychains` returns only the System keychain and `security default-keychain` fails; without the override the login keychain is listed. **INHERITED** (Warden F-6, MEASURED 2026-09-17, positive control both ways). A freshly created keychain is locked and prompts for its password on read (Warden F-7, INFERRED from `man security`). **`man security`, `add-generic-password` (MEASURED 2026-09-27, lines 245-247): "By default, the application which creates an item is trusted to access its data without warning. You can remove this default access by explicitly specifying an empty app pathname: -T ""."** An item created with no `-A`/`-T` therefore trusts `/usr/bin/security`, the same binary every reader uses; a keychain ACL is keyed on the reading application's binary, not on its parent process.
- **`fly` v0.4.104 with an empty `HOME` holds no credential (MEASURED 2026-09-27):** `env -i HOME=<empty temp dir> PATH=/usr/bin:/bin:/opt/homebrew/bin fly auth whoami` prints `Error: no access token available. Please login with 'flyctl auth login'` and exits 1, and creates no config under that `HOME`; positive control: the same command under the real `HOME` (where `~/.fly/config.yml` exists) prints the account identity. `FLY_API_TOKEN`/`FLY_ACCESS_TOKEN` are absent under `env -i` by construction.
- **`just 1.58.0` (MEASURED 2026-09-27, `just --help`; the two flags R-0007 passes re-confirmed on 1.59.0, 2026-11-09):** `--no-dotenv` ("Don't load `.env` file", env `JUST_NO_DOTENV`), `--dotenv-command` (runs a command and loads its output, env `JUST_DOTENV_COMMAND`), `--dotenv-path`, `--dotenv-filename`, and `-d, --working-directory`. The main `justfile` has no `dotenv` or `set` line today. `env -i` "ignores the inherited environment completely" (`man env`). `/usr/bin/shasum` is `#!/usr/bin/perl`, so Perl is a sixth real binary reached by absolute shebang. `bash` sources `$BASH_ENV` for non-interactive shells and `command -p` searches a default path (`man bash`). Under the fixed `PATH=/usr/bin:/bin:/opt/homebrew/bin`, `just` resolves to `/opt/homebrew/bin/just`, a symlink into `/opt/homebrew/Cellar/`, and `bash` to `/bin/bash` 3.2.57 (MEASURED 2026-11-09).
- **`curl 8.7.1` (MEASURED 2026-09-27, `man curl`):** `-H @-` "makes curl read the header file from stdin" (added in 7.55.0), so a bearer header can reach `curl` without touching argv.
- The Fly app is one machine on one volume at `/app/data`, `min_machines_running = 0`, daily snapshots with 5-day retention, and routinely sits `stopped`. **INHERITED** (Bolt).
- `CLAUDECODE=1` is set in a Claude Code shell. **MEASURED** (2026-09-14). `security add-generic-password -w` as the last option prompts on the tty; `-U` updates; `-A`/`-T` set the access list; `security -i` reads commands from stdin and prints a `security>` prompt; `find-generic-password -w` prints only the password. **MEASURED** (`man security`).
- `fly v0.4.99`: `fly secrets import` reads `NAME=VALUE` from stdin and starts a deployment without `--stage`. **MEASURED** (2026-09-14). `deploy.yml:98` uses `curl --retry 5 --retry-delay 5 --fail`. **INHERITED** (Bolt OP-3).
- mnemra.dev's blog collection requires `title` (min 1) and `summary` (1–280). **MEASURED** (sibling read). `#3560` is done. **MEASURED 2026-09-15.**
- job is agent-operated and ingests untrusted text. **INHERITED** (Warden W-01).
- Test harness anchors: `tests/common/mod.rs:28` `test_app()`, `:72` `test_app_with_state()` — both mount `/api/health` on a bare router (`:52-53`, `:96-97`), not through `build_app`; `tests/test_origin_lock.rs:25-35` builds through `build_app` with `tests/fixtures/static_site` as the static dir; `tests/common/ai_mock.rs:81`; outbound-body capture in `tests/test_ai_chat_endpoint.rs`. `axum_test::TestServer` cannot withhold part of a request body (Warden F-18); no test under `libs/site-core/tests/` drives a raw `TcpStream` today (Glitch L1). **MEASURED 2026-09-27 / INHERITED.**
- **Startup (MEASURED 2026-09-27):** `Config::from_env` (`libs/site-core/config.rs:53-54`) calls `dotenvy::dotenv().ok()` first; `dotenvy 0.15.7`'s finder **walks parent directories** from the process cwd until it finds `.env` or reaches the root (`dotenvy-0.15.7/src/find.rs:32-33`). every variable `Config::from_env` requires is enumerated by `config.rs`'s own unit tests (`:53-71`, `:86-140`); `PORT` must parse (`:68-71`); `main.rs:34` calls `validate_static_dir`; the listener binds at `main.rs:99` and logs `listening` with the port at `:103`. `cmd/server/` has no `tests/` directory; its `[[bin]]` is named `folio`.
- **Dependencies (MEASURED 2026-09-27):** workspace `resolver = "3"`; `tokio` is `1.52.3` in `Cargo.lock` with the `full` feature only, and `full` in tokio's own `Cargo.toml` (`:72-84`) does not include `test-util` (`:133`, a separate feature). `trybuild` is not in `Cargo.lock`. `rustdoc 1.98.0`'s documentation of `compile_fail` names no error-code check (the rustdoc book shipped with the toolchain, "compile_fail" section).
- Private-content sizes, for sizing only: ~12.7 KB across `experiences` AI-only columns (9 rows), ~2.1 KB `candidate_profile`, ~5 KB FAQ, ~1.1 KB values; `gaps_weaknesses` and `ai_instructions` empty. **INHERITED** (Sage F4, 2026-07-23 snapshot). No value from that snapshot appears here.

## Decisions already made (do not reopen)

- **Frame L-1…L-16** are rulings, not proposals.
- **Forward-only migrations** (G-0005). The table drop is a later release (R-0018).
- **No compile-time asset embedding** (G-0002): nothing here embeds content. The committed schema is embedded with `include_str!` following the migrations precedent (`db/schema.rs:5-36`).
- **Coverage is measured and reported, not gated** (`#2750`).
- **`#2753` disposition** (2026-07-24): the seven Tier-B experience fields stay in the prompt; the two Tier-A drops stand.
- **`#3558` lands before this spec merges and before any release built from it** (RULED `5217` (3), `5218` on `#3558`).
- **`subtle` becomes a direct dependency** (RULED `5404`, E-2). **Divergence recorded:** `#3558`'s implementation carries its own hand-rolled constant-time compare with its reason documented there; it is not changed by this spec, and the two are not unified (V-Consistency: the one-sentence reason is that `#3558`'s compare guards a value that is not a stored digest and was reviewed on its own terms; this spec's compare is where the compile-enforced newtype earns its cost).

## What the bundle is — and is not

The bundle is the **whole of what the AI knows about the candidate**: a single JSON document, pushed by an authenticated client, held in memory by the server and persisted as a file on the volume. Every field carries a **tier** — `public`, `unpublished` or `private` (frame L-9; RULED `5217` (4)) — fixed in the schema, never per instance:

- **`public`** — text already on the public pages, duplicated into the bundle so the server never reads page files (L-3).
- **`unpublished`** — text that reaches only the system prompt; the model **may use it in answers**, including verbatim; never on a page, never in a repository. FAQ answers are the canonical case.
- **`private`** — text the model **may be informed by but must never quote or paraphrase closely**. Why a role ended, and an employer's internal numbers, are the canonical cases.

**The tier rule has two axes (RULED `5419`, superseding `5404`'s one-axis rule).** A field is `private` if it is **self-critical** (today's sensitive labels, self-ratings, stated weaknesses) **or carries a third party's confidential detail** (an employer's unreleased work, internal numbers, named clients or people). Every other AI-only field is `unpublished`. The rule governs new fields too. **Superseded alternative, kept with its reason:** the one-axis rule (`5404`: self-critical only) put `quantified_impact`, `actual_contributions` and `challenges_faced` in the tier the model may quote verbatim — which is where employer-specific material lives (Warden F-26). Rejected in `5419`: one axis plus a manual audit of those three fields (safety would rest on that pass and every future edit); never quoting anything private (paraphrase everywhere).

**Neither non-public tier is confidential.** The system prompt is served to anonymous visitors through unauthenticated chat and fit endpoints (`routes::ai`, merged at `libs/site-core/app.rs:100`). Protection against a determined visitor is the instruction layer (R-0009's tier regions plus `SECURITY_BLOCK`) — the exposure Peter reviewed and accepted on 2026-07-24 (`#2753`). This spec changes **where the text lives** and **how it reaches the server**, not that exposure. Content that must not be on the public internet MUST NOT be in the bundle.

**job is a trusted producer** (Warden W-08). folio validates **structure only**. job ingests untrusted text and is agent-operated; **job MUST guarantee that no externally sourced text enters a bundle field without Peter's review**, that its page emitter is an allowlist of `x-tier: public` fields (RULED `5217` (2)), and that its push runs only from a terminal-only command that refuses under `CLAUDECODE` (R-0007 rule 3, adopted alongside the Keychain ACL — Warden F-8). All three are needs on `#3471`. A fourth is there too, **unverified and pending**: job's ingest-facing agent sessions cannot reach the Fly credential (T10). They are stated here so the trust boundary is on the page (T10, T11).

**What the split buys structurally.** A field absent from the schema cannot reach the prompt without a schema change reviewed in a PR (V-DataMinimization at the contract).

## Threat model

Required at design time (P-SecurityLayered).

**Assets.** (A1) the non-public tiers' text; (A2) the integrity of the system prompt; (A3) the paid Anthropic key; (A4) the push token; (A5) availability of the AI surface.

**Trust boundaries.** Internet → CDN edge → Fly → the Axum process. Peter's Mac (Keychain) → job's push client → the endpoint. job's source of truth ← untrusted external text.

**Separation of privilege is absent, by ruling.** One bearer token, held by one actor, is the only condition on a push that rewrites the prompt served to every visitor (frame L-8). Compensating controls: revoke-now and the kill switch (R-0007), and job's pre-push digest check — which is a cross-repo obligation, **pending on `#3471`**, not a control folio ships (Warden F-19). Recorded as accepted.

| # | Threat | Asset | Control(s) | Residual, stated |
|---|---|---|---|---|
| T1 | **Token theft** → attacker pushes a bundle. | A2, A1 | Token never printed or passed as argv (R-0007); `CLAUDECODE` refusal covers generation, rotation and revocation only; read path is T10; **revoke-now** and the **kill switch** (R-0007); every push logged with digest and version (R-0021); `previous.json` for rollback (R-0008); job's pre-push digest check (RULED `5221` (6); client contract) — **pending on `#3471`, unimplemented at the time of writing**; until it exists, folio-side detection between pushes is D-16's recorded gap. | A holder controls the prompt until revocation, and **revocation lands only when the deployment `fly secrets import` triggers has restarted the machine** (D-10; the same window applies to the kill switch — Warden F-23). The `CLAUDECODE` refusal is locally bypassable. Any free-text field can carry a link or an instruction the model repeats (T11). A push is identified by digest and version only; no source address is logged (D-28). |
| T2 | **Replay / interception.** A captured request replayed **rolls content back** to the captured generation and rotates the real previous generation away. | A4, A2 | HTTPS end to end; an *immediate* replay is a no-op (R-0005 idempotence). | Whoever captures the request holds the token; subsumed by T1, answered by revoke. The CDN sees the token in transit (R-0020). |
| T3 | **Malicious or oversized bundle.** | A3, A5 | Auth before body; route-scoped body cap 1 MiB (R-0005); checker with bounds (R-0004); rendered-prompt cap in UTF-8 bytes (R-0004); structural-token refusal in values (R-0004 (f)); nonce-bound delimiters (R-0009). | A valid bundle at the caps is the per-request spend ceiling, ~3× today's size (INFERRED). |
| T4 | **`#3558`** — cited by number. | A2 | Authorization is the token on every request path and depends on nothing outside this spec (D-9). | `#3558` lands first (RULED `5217` (3)); R-0019 step 1 carries an observable post-condition. |
| T5 | **Online token guessing.** | A4 | 256-bit random token; digest compared in constant time by construction (R-0006). | 2^256. A limiter adds nothing to confidentiality. |
| T6 | **Hash disclosure.** | A4 | Only the digest is stored; nothing about hashes on public health. | None material. |
| T7 | **Bundle at rest**; snapshots. | A1 | Plaintext file on an encrypted Fly volume — the same posture as SQLite today (RULED `5331`). | Fly staff / snapshot access unchanged. A snapshot restore serves stale content and reports healthy — runbook step in R-0019. |
| T8 | **Content in the artifact or the repo.** | A1 | No bundle in the build context (R-0013); synthetic fixtures with a PII check ranging over every fixture and test file (R-0015); page files guarded by an opt-out scan with a pinned exclusion list, a pre-push hook and job's allowlist emitter (R-0014). | A private **value** under a public key name is undetectable by key-name scanning (`#2753`). The hook runs only in a clone where `just setup-hooks` has run, and `--no-verify` skips it; `just check` fails when it is not installed (R-0014). |
| T9 | **Availability.** | A5 | Validate-before-swap; copy-then-rename so `current.json` is never absent (R-0008); `previous.json` fallback; N-1 format support; health reports state **on the path production serves** (R-0011); pages never depend on the bundle; the post-deploy check passes only on an allow-listed body and the check itself is tested (R-0019). | First deploy to first push is AI-unavailable by design. Degradation between deploys has no alert (RULED `5228` (13); D-16). |
| T10 | **Token read by an agent session** (Warden W-01). | A4, A2 | **Two layers with different reach** (Warden F-8, W3-1): (1) the Keychain item is created with `-T ""` so it prompts on every read by any application (RULED `5217` (1)), proven by the R-0007 rule 3 probe with a positive control, and the creation flags are checked by shim assertion (j); (2) job's push runs only from a terminal-only command that refuses under `CLAUDECODE` — adopted **alongside** (1) (need on `#3471`; client contract). **Layer (2) constrains job's push command only. Against a direct `security find-generic-password` call from an injected agent that can run commands, layer (1) is the only control on the push token.** | **Layer (1) covers the push-token path only; the Fly credential on the same Mac grants equal-or-greater authority with no prompt** (need on `#3471`, unverified and pending: job's ingest-facing agent sessions cannot reach it). Layer (1) holds only while the item's access list stays as created. Layer (2) is locally bypassable and does not cover a direct read. If the rule-3 probe fails, T10 has **no** layer against a direct read, and that is recorded as the residual rather than layer (2) being promoted. Neither stops Peter himself pasting the token into a session. |
| T11 | **Producer poisoning** (Warden W-08). | A2 | job's review obligation (`#3471`); values rendered inside **nonce-bound** delimiters the model is told are data (R-0009); a value containing a structural token is refused (R-0004 (f)); the refusal set and the delimiter constants are tied by a test. | Instruction-only at the model. folio cannot tell Peter's text from injected text. |

**Layers per P-SecurityLayered.** Local best-effort — the `CLAUDECODE` refusals, `just check`, the pre-push hook. Structural — the token (fails closed at startup, RULED `5224` (9)), auth-before-body, the checker, the route body cap, job's allowlist emitter. Backstop — CI after the push, the post-deploy health assertion. A control that errors denies.

## Requirements

### R-0001 — The bundle is one JSON document with a manifest and an integer format version

**Decision.** A single UTF-8 JSON document, `Content-Type: application/json`, top-level `manifest` plus the ten section keys of R-0002. No archive, no compression. *Anchors: V-Simplicity; P-StackDiscipline.*

| Field | Type | Rule |
|---|---|---|
| `format_version` | integer | `1` in this release. Dispatched on first (R-0012). R-0004 step 1 is the only place its supported values are enforced: the schema and the Rust type constrain it to an integer and nothing more. |
| `content_version` | string 1–128, **charset `[A-Za-z0-9._:+-]`** | Producer-assigned, opaque. Authenticated `GET` only. |
| `generated_at` | string 1–64 | When the producer built it. Opaque to folio: stored and returned, never parsed. RFC 3339 UTC is job's convention. |
| `producer` | string 1–128, **charset `[A-Za-z0-9._:+ -]`** | Never on public health. |

The charsets are cross-field rules (R-0004 step 4 (e)), not schema keywords, and are stated in the client contract.

**Acceptance.** Top level not an object, or `manifest` or any of its four fields missing → `422` naming the path. `format_version` not an integer or unsupported → `422` with `"supported_format_versions": [..]`. Wrong `Content-Type` → `415`. `content_version` with a space or a newline → `422` `/manifest/content_version` rule `charset`; `producer` with a newline → `422`; with a space → accepted. `generated_at` of `banana` → accepted. The committed fixture → `200`.

**Rejected alternatives.** Zip or tar; a SQLite file (`#3472` hazard travels); a monotonic `content_version` (blocks rollback-by-re-push).

### R-0002 — Sections, fields and tiers: the existing context, limited to what the builders use

**Decision.** Exactly the fields below. Array order is prompt order. Every key is **required** (present); "may be empty" means `""`, `[]`, `{}` or `null` where the type allows `null` (D-2). **Field names are unique across tiers** (Warden W-03). Tier per field follows the two-axis rule (RULED `5419`). *Anchors: L-9, L-12; V-DataMinimization; MEASURED field usage `context.rs:45-347`.*

| Section | Type | Field | Type / bound | Tier | Non-empty required? |
|---|---|---|---|---|---|
| `profile` | object | `name`, `title` | string ≤ 200 | public | **yes** |
| | | `pitch_short` | string ≤ 280 | public | no |
| | | `pitch_long` | string ≤ 1500 | public | no |
| | | `career_narrative` | string ≤ 10 000 | unpublished | no |
| | | `looking_for`, `not_looking_for`, `management_style`, `work_style` | string ≤ 2 000 | unpublished | no |
| | | `target_titles`, `target_company_stages` | array ≤ 50 of string ≤ 200 | unpublished | no |
| `experience` | array ≤ 50 | `company_name`, `title` | string ≤ 200 | public | **yes** |
| | | `location` | string ≤ 200 | public | no |
| | | `start_date` | string ≤ 32 | public | **yes** |
| | | `end_date` | string ≤ 32 or `null` | public | no |
| | | `is_current` | boolean | public | — |
| | | `summary` | string ≤ 2 000 | public | no |
| | | `bullet_points` | array ≤ 20 of string ≤ 500 | public | no |
| | | `proudest_achievement`, `lessons_learned`, `title_progression` | string ≤ 10 000 | unpublished | no |
| | | `why_joined`, `why_left`, `reports_would_say` | string ≤ 10 000 | private (self-critical axis) | no |
| | | `actual_contributions`, `challenges_faced` | string ≤ 10 000 | **private (third-party axis, RULED `5419`)** | no |
| | | `quantified_impact` | map ≤ 50 keys; keys `[A-Za-z0-9_ -]` ≤ 64; values string ≤ 500 **or** number | **private (third-party axis, RULED `5419`)** | no (`{}` = absent) |
| `skills` | array ≤ 200 | `skill_name` | string ≤ 200 | public | **yes** |
| | | `category` | enum `strong` / `moderate` / `gap` | public | — |
| | | `years_experience` | integer ≥ 0 | public | — |
| | | `evidence` | string ≤ 2 000 | unpublished | no |
| | | `self_rating` | integer 1–5 **or `null`** (`null` = absent) **[SC]** | private | no |
| `education` | array ≤ 20 | `degree`, `institution` | string ≤ 200 | public | **yes** |
| | | `location`, `start_year`, `end_year` | string ≤ 200 / ≤ 16 / ≤ 16 | public | no |
| `projects` | array ≤ 50 | `title` | string ≤ 200 | public | **yes** |
| | | `summary` | string ≤ 2 000 | public | no |
| | | `description` | string ≤ 10 000 | public | no |
| | | `tech_stack` | array ≤ 50 of string ≤ 100 | public | no |
| `articles` | array ≤ 200 | `title` | string ≤ 200 | public | **yes** |
| | | `summary` | string 1–280 | public | **yes** |
| | | `url` | string ≤ 500, `https://mnemra.dev/…` | public | **yes** |
| | | `published_at` | string `YYYY-MM-DD` | public | **yes** |
| `instructions` | array ≤ 50 | `instruction_type` | string ≤ 64 | unpublished | **yes** |
| | | `instruction` | string ≤ 2 000 | unpublished | **yes** |
| `values` | object | `must_haves`, `management_style_preferences`, `team_size_preferences`, `how_handle_conflict`, `how_handle_ambiguity`, `how_handle_failure`, `dealbreakers` | string ≤ 2 000 | unpublished (`dealbreakers` named unpublished in RULED `5404`; `5419` moves only the three experience fields — Warden W3-13) | no |
| `faq` | array ≤ 100 | `question` | string ≤ 500 | public | **yes** |
| | | `answer` | string ≤ 5 000 | unpublished | **yes** |
| `gaps` | array ≤ 50 | `gap_type` | string ≤ 64 | private | **yes** |
| | | `gap_description` | string ≤ 2 000 | private | **yes** |
| | | `why_its_a_gap` | string ≤ 2 000 | private | no |
| | | `interest_in_learning` | boolean | private | — |

**[SC] `self_rating` is nullable** because R-0010's "every non-public field of the section empty" must be satisfiable for `skills`; a required integer has no empty value, so the `public_only` fixture could never exist as written before.

**Deliberately absent** (rejected as unknown keys): `id`, `created_at`, `updated_at`, `visible`, `display_order`, `sort_order`, `priority`, `slug`, `url` (projects), `published`, `published_at` (projects), `content` (articles), `tags`, `email`, `phone`, `location` (profile), `linkedin_url`, `github_url`, `twitter_url`, `availability_status`, `availability_date`, `remote_preference`, `salary_min`, `salary_max`, `would_do_differently`, `manager_would_say`, `honest_notes`, `last_used`, `is_common_question`, `description` (gaps — renamed), everything in `links`, `agents` (RULED `5311`), `site_config` (RULED `5311`), `page_hits`, `rate_limits`.

**Acceptance.**
- A test enumerates the generated schema's properties per object and asserts set-equality with a hard-coded list per object.
- A test asserts every property carries exactly one `x-tier` and the three name sets are pairwise disjoint (positive control: rename `gap_description` to `description` in a test copy → fails naming the clash).
- A test asserts the `x-tier: private` set equals exactly `{why_joined, why_left, reports_would_say, actual_contributions, challenges_faced, quantified_impact, self_rating, gap_type, gap_description, why_its_a_gap, interest_in_learning}` (the `5404` set — which names `dealbreakers` unpublished — as corrected by `5419`; a tier change is a visible diff).
- **Schema-shape bullets (the three above) are Mechanical, in R-0003's bucket** (Glitch M1): they need in-process `schemars` regeneration and are the implementer's paired self-tests, not red-phase work.
- Any Deliberately-absent key at any level → `422` naming it (controls: `would_do_differently`, `salary_min`, `content` on an article, `description` on a gap).

### R-0003 — The published schema is generated from the Rust types, in tests; its keyword set and the checker's are the same set

**Decision.** The bundle's Rust types (`#[derive(Deserialize, Serialize, JsonSchema)]`, `#[serde(deny_unknown_fields)]`) are the source. A `#[cfg(test)]` test regenerates the schema with `schemars` and asserts byte-equality with the committed `contracts/content-bundle.v1.schema.json`; with `CONTRACT_UPDATE=1` it writes the file (`just contract-update`). The server binary gains nothing new for this; `schemars` is untouched (RULED `5331`, `5365`). Bounds are `schemars` attributes; each property carries `"x-tier"` (post-processed in the same test); every **struct** object declares `"additionalProperties": false`; the one **map** (`quantified_impact`) declares `"additionalProperties": <value schema>` and `"maxProperties": 50`.

**Keyword set (Warden F-1) [SC].** The count is not a spec constant. The implementer generates the real schema once and enumerates the keywords it contains; that enumeration becomes `CHECKER_KEYWORDS`, the set R-0004's checker implements. A test asserts **set-equality in both directions** between the keywords present in the committed schema and `CHECKER_KEYWORDS`, seeded both ways (a keyword in the schema the checker lacks → fail; a keyword the checker lists that never appears → fail, or is explicitly listed as reserved with a reason). Expected members, from `schemars 0.8.22`'s measured output, at least: `$schema`, `definitions`, `$ref` (or the generator is configured to inline subschemas, in which case these two are absent and the test says so), `type` (string or array-valued), `properties`, `required`, `additionalProperties` (boolean or schema), `items`, `maxItems`, `maxLength`, `minLength`, `minimum`, `maximum`, `format`, `enum`, `anyOf` (for `string | number`), `maxProperties`, `x-tier`. `description`/`title` appear only if doc comments are placed on the types — they are not, and the test asserts their absence. *Anchors: L-12 (RULED); P-LockContract; P-GuaranteeByMechanism.*

**Acceptance.**
- Editing one `maxLength` in the committed file fails the generation test naming the file (seeded, recorded).
- The both-way keyword test passes; seeded: add `"pattern"` to a test copy → fails naming `pattern`; remove `items` from `CHECKER_KEYWORDS` → fails naming `items`.
- **Every validating keyword is proved live (Warden W3-5, W4-3).** `CHECKER_KEYWORDS` is partitioned in source into **assertion** keywords (they reject a value themselves), **applicators** (`properties`, `items`, `$ref` and schema-valued `additionalProperties`: they only descend; `additionalProperties: false` is an assertion, so that keyword has one fixture per role) and **annotation-only** keywords, each annotation-only entry with a one-line reason (`format` is one: `type` plus `minimum`/`maximum` carry the bound). An assertion keyword has a file under `contracts/fixtures/invalid/` named `<keyword>-*.json` that fails with `rule == <keyword>`; an applicator has one that fails at a path reachable only through it, the inner keyword as `rule`. A reconciliation test fails if any validating keyword has no such fixture or any fixture names an unknown keyword. Seeded: make the checker's `minimum` branch a no-op → the test fails naming `minimum`; make it skip `items` descent → fails naming `items`.
- Every struct object has `"additionalProperties": false`; `quantified_impact` has a schema-valued `additionalProperties` and `maxProperties`; every property has `x-tier`.
- `just contract-update && git status --porcelain contracts/` prints nothing on a clean tree.
- `cargo tree -e normal -p site-core` diffed against the parent commit shows exactly one new direct edge, `subtle` (RULED `5404`) — nothing else (positive control: `schemars` listed both before and after). **[SC]** (the earlier "no dependency added" criterion contradicted E-2's ruling).

### R-0004 — A pushed document is validated in full, by folio's own checker against the committed schema plus cross-field rules, before anything is stored

**Decision.** Order; the first failure rejects the whole document with `422` and `{path, rule}` errors:
1. `manifest.format_version` from the raw document; unsupported → `422` (R-0012).
2. **folio's own checker** (RULED `5221` (5)) evaluates the schema set it was **constructed with**: an interpreter for exactly `CHECKER_KEYWORDS` (R-0003) that **fails closed** — a schema containing any other keyword is refused when the checker is constructed, and R-0003's test prevents one landing. **The schema set is a constructor input (Glitch H2; P-TestableByDesign):** `Checker::new(schemas: &[(FormatVersion, &str)]) -> Result<Checker, CheckerError>`; production passes the embedded set (R-0012's `include_str!` files, unchanged — nothing new enters the binary, RULED `5331`, `5365`); a test passes a synthetic set. There is no env var, flag or file override for the running server. `maxLength`/`minLength` count Unicode scalar values; tests include multi-byte strings at the limit. `type` arrays and `anyOf` are evaluated as JSON Schema draft-07 defines them for the shapes the schema uses (nullable, `string | number`); `$ref` resolves only within the document being checked (no remote or file refs — by construction, there is no resolver).
3. Deserialization into the Rust types (`deny_unknown_fields`).
4. Cross-field rules: (a) every non-empty-required field is non-empty after trimming; (b) `articles[].url` — absolute, `https`, host exactly `mnemra.dev`, no userinfo, no port; (c) `published_at` is a calendar date; (d) `end_date` is `null` when `is_current`; (e) manifest charsets (R-0001) and `quantified_impact` key charset and length; (f) **structural-token refusal (Warden F-2) [SC]:** no string value contains a line whose first non-blank character is `#`, a line whose first non-blank characters are `===`, or the substrings `<<` or `>>` anywhere — the set is `PROMPT_STRUCTURAL_TOKENS`, a constant that R-0009's delimiter constants are derived from, with a test asserting every delimiter R-0009 emits matches a refused form. **"Line" is defined (Warden W3-11):** a value is split at every Unicode line terminator — LF, CR, CRLF, U+000B, U+000C, U+0085, U+2028, U+2029 — by the checker's own splitter, `PROMPT_LINE_TERMINATORS`, never by `str::lines()` (which splits on LF and CRLF only — INFERRED from the Rust std documentation, not measured this round; the explicit set makes the claim independent of it); (g) the rendered system prompt (R-0009) is ≤ **120 000 UTF-8 bytes**.
5. Only then stored (R-0008).

**Error shape.** `422 {"error":"invalid_bundle","error_count":N,"errors":[{path, rule}, …]}`. `path`: JSON pointer, truncated at any map-typed field, ≤ 256 bytes, non-printables escaped. `rule`: keyword or rule name only. Never a value. **Several violations at once (Glitch M3):** the step that fails reports **every** violation it found in that step, in document order, **capped at five** entries in `errors` — the same cap and order as R-0021's log line — with `error_count` carrying the uncapped total; steps after the failing one do not run.

*Anchors: V-Security; P-GuaranteeByMechanism; T3, T11; L-10.*

**Acceptance (one fixture per case under `contracts/fixtures/invalid/`).**
- Unknown key at top level / `manifest` / `profile` / one experience → `422` naming it.
- `profile.name` empty → `422` `/profile/name`; `articles[0].summary` 281 chars → `422`; 51 experiences → `422` `/experience`; 200 four-byte code points at a 200 limit → accepted, 201 → `422`; `quantified_impact` with 51 keys → `422` `/experience/0/quantified_impact` rule `maxProperties`; a value of 501 chars under key `FIXTURE-qi-key` → `422` with path exactly `/experience/0/quantified_impact` and the sentinel absent from body and logs (positive control: the path is present); a numeric value → accepted.
- `articles[0].url` = `http://mnemra.dev/blog/x/` → `422`; `https://example.com/` → `422`; `https://mnemra.dev.evil.com/` → `422`; `https://user@mnemra.dev/` → `422`; `https://mnemra.dev/blog/hello-mnemra/` → accepted.
- `faq[0].answer` containing a line `## SECURITY` → `422` rule `structural_token`; containing a line `=== PRIVATE ===` → `422`; containing `<<end>>` mid-line → `422`; containing `<<` alone → `422`; the same characters not in those forms (`#` mid-line, `==` , `<`) → accepted. **Line terminators:** `"x\r=== PRIVATE ==="` → `422`; `"x\u{2028}## x"` → `422`; `"x\u{0085}=== y"` → `422`; `"x\n  # y"` → `422` (leading blanks); `"x=== y"` → accepted.
- `skills[0].self_rating` = `6` → `422` `/skills/0/self_rating` rule `maximum`; `0` → `422` rule `minimum`; `null` → accepted; `years_experience` = `-1` → `422` rule `minimum` (W3-5's no-op-proof cases).
- A document with seven step-2 violations → `422`, `error_count: 7`, `errors` has exactly five entries in document order; a document with two → `error_count: 2`, two entries; a document whose only violation is at step 4 (a) → `error_count: 1`.
- Rendered prompt 120 001 bytes → `422` rule `rendered_prompt_too_long`; 120 000 → accepted.
- After any `422`, the authenticated `GET`'s `sha256` and `current.json` are unchanged.
- **Mechanical (Glitch N1).** `Checker::new` over a synthetic set containing a keyword outside `CHECKER_KEYWORDS` (e.g. `pattern`) → `Err(CheckerError::UnknownKeyword("pattern"))`, never a checker that skips it; over the embedded set → `Ok`. This property has no black-box signal by design (D-24), so it is the implementer's paired self-test.

### R-0005 — The import endpoint: `PUT /api/content/bundle`, bearer-authorized before the body is read, route-scoped body cap, strict flags, idempotent, no caching

**Decision.** Router `routes::content`:

| Route | Auth | Behaviour |
|---|---|---|
| `PUT /api/content/bundle[?dry_run=true\|false][&allow_public_only=true\|false]` | Bearer | Validate → store and swap → `200 {"state","format_version","content_version","sha256","bytes","sections":{…},"empty_sections":[…],"unchanged":bool,"push_token_hashes_configured":n,"dry_run":bool}`. |
| `GET /api/content/bundle` | Bearer | Manifest + `state`, `source`, `loaded_at`, `sha256`, `empty_sections`, `push_token_hashes_configured`. `404 {"error":"no_bundle"}` when `absent`. |

Rules: **auth before body** — a `route_layer` on this router; the handler never receives the body until the token matched; `401 {"error":"unauthorized"}`. `DefaultBodyLimit::max(1_048_576)` on this router only. **Strict flags:** exactly `true`/`false`; any other value or unknown query key → `400 {"error":"bad_query"}`, nothing read. **Public-only guard** (RULED `5221` (7)): a push that would move `loaded`/`partial` → `public_only` → `409 {"error":"would_drop_to_public_only","empty_sections":[…]}` unless `allow_public_only=true`; not applied on a fresh server or one already `public_only`. Idempotent: `unchanged: true`, with nothing written, only when the body's `sha256` equals that of `current.json`'s bytes on disk. **Pushes serialized:** one push holds the store's write lock from validation through the `Arc` swap; a second push waits, then runs against the state the first left. **Reads never wait on a push (Bolt B4-2):** every read of the current bundle state — health on both paths, chat and fit, the authenticated `GET` — MUST NOT wait on this lock, and sees the pre-swap state until the swap. `Cache-Control: no-store` on both routes. No endpoint-specific limiter (D-9). Flat `{"error":…}` shape; new variants `Unavailable` (503), `Conflict` (409), `BadQuery` (400).

*Anchors: P-LeastAuthority; V-Consistency; V-Simplicity (D-6); P-TestableByDesign.*

**Acceptance.**
- **Auth-before-body, deterministic (Warden F-18) [SC]:** over a raw `TcpStream` (or a manually driven hyper body channel — the harness is named so `axum_test` is not silently substituted): with a **wrong** token, send headers with `Content-Length: 1048576` and no body → `401` arrives; with a **valid** token, send headers plus exactly half the declared body → assert no response bytes have arrived after the half is flushed and the server has acknowledged it (a read with a short deadline returns nothing), then send the remainder → `200`. No wall-clock bound is asserted on the first case beyond the test's own overall timeout. **This test does not run through `test_app_with_bundle`** (Glitch L1): it binds the composed app from `build_app` on a loopback `TcpListener` and drives the socket itself — a new harness; no test under `libs/site-core/tests/` does this today (MEASURED 2026-09-27).
- **Against the composed app from `site_core::app::build_app`, not the bare router (Glitch M5):** body 1 048 577 bytes with a valid token → `413`; 1 048 576 of a valid document → not `413`; 100 KiB → not `413`; `POST /api/chat` with 100 KiB → still `413`. The global 64 KiB cap lives in `build_app` (`app.rs:113`, MEASURED 2026-09-27), so these four lines prove the route-scoped override only when that layer is present.
- **Serialization observable (Glitch M6):** with bundle Z live, through the R-0008 filesystem seam, push A is blocked inside its first write; push B (a different body) is sent while A is blocked → no response to B arrives while A is blocked (a read with a short deadline returns nothing); A is released → A's response, then B's, both `200`; final state `current.json == B`, `previous.json == A`. While A is blocked, each of these completes before A is released: `GET /api/health` through `build_app` → `loaded`; the authenticated `GET` → `sha256(Z)`; a `POST /api/chat` whose outbound `system` carries Z's sentinel. Positive controls, recorded once (Verification item 9; Glitch N5): with the store lock removed, B's response arrives while A is still blocked; with a seeded health read taking that lock, the health response does not arrive before A's release.
- No `Authorization` → `401`; wrong → `401`; right + fixture → `200`, `sha256` = SHA-256 of the bytes.
- `?dry_run=true` on a fresh server → `200 dry_run:true`, then `GET` → `404 no_bundle`. `?dry_run=1`, `?dryrun=true`, `?dry_run=TRUE` → `400`, `sha256` unchanged.
- Loaded fixture live, push `public-only` without the flag → `409`; with it → `200 state: public_only`; fresh server, no flag → `200`.
- Two identical pushes → `unchanged: true`, `previous.json` absent; A, B, B → `previous.json` is A; from a `previous` boot (R-0008), pushing `previous.json`'s bytes → `unchanged: false` and `current.json` equals the body.
- Both routes carry `Cache-Control: no-store`. `test_llm_audit_hardening.rs` R5 passes unmodified.

### R-0006 — Push token: 256-bit bearer secret, digest-only on the server, constant-time by construction and closed to byte access, required at startup

**Decision.**
- **Token:** 32 CSPRNG bytes, base64 (44 chars); the client strips the trailing newline.
- **Server configuration:** `CONTENT_PUSH_TOKEN_SHA256` — one or two lowercase hex digests, comma-separated. Unset, empty, malformed, or more than two → **the process refuses to start** (RULED `5224` (9)). **Trade-off, stated (Warden W-30):** the refusal takes the pages down too; the alternative (start with the content router returning `503`) was rejected because R-0019 step 2 gates the secret before B1 and a silently unconfigured write path is what the `ADMIN_PASSWORD` precedent prevents.
- **Comparison (Warden W-24, F-14) [SC]:** `PushTokenDigest([u8; 32])` — the inner array is **private**; the type implements **no `PartialEq`/`Eq`, no `Deref`, no `AsRef<[u8]>`, no `as_bytes()` or any byte accessor**, and exactly one comparison method `ct_eq(&self, &Self) -> bool` over `subtle::ConstantTimeEq` (`subtle` is a **direct dependency**, RULED `5404`). `Config` holds `Vec<PushTokenDigest>` parsed at startup — never the hex strings — and the presented token is hashed into a `PushTokenDigest` before any comparison. The compiler enforces `==`-freedom; a source-text control enforces that no `String`/`&str`/`[u8]` equality occurs in the auth path.
- **Scope:** read only by `routes::content`. The global limiter's pre-limit `admin_sessions` lookup persists until R-0017 Part B (D-9).
- **Rotation overlap:** two digests; responses report `push_token_hashes_configured`.
- **Kill switch:** 64 zeros is well-formed with no known preimage; needs only `fly` auth. It takes effect **after the restart the secret change triggers** (F-23).
- **No dev token is committed.** `.env.example` carries the variable with an empty value and a comment; `just dev-push-token` writes a generated digest into the gitignored `.env`; a test reads `.env.example` and asserts its value fails validation. `just docker` needs `.env` populated first, and the compose file says so in a comment.
- **Keychain identity:** `io.manahan.folio.push-token` / `folio-prmanahan`; `-previous` during rotation. Created with an access list that trusts no application (RULED `5217` (1)) — **which means `add-generic-password … -T ""` and neither `-A` nor any non-empty `-T`** (Warden W3-1; `man security` MEASURED 2026-09-27: an item created without `-T ""` trusts the creating binary, `/usr/bin/security`, which is every reader's binary). The `-previous` item written by `rotate-token` carries the same flags.
- **Fly secret name:** `CONTENT_PUSH_TOKEN_SHA256` on `folio-prmanahan`.

*Anchors: L-8; P-LeastAuthority; P-SecurityLayered; V-Consistency; P-TDDPairs T5.*

**Acceptance.**
- **Startup refusal, by subprocess (Glitch H4).** Harness: a new `cmd/server/tests/startup.rs` (Cargo injects `CARGO_BIN_EXE_folio` only for the package that owns the `[[bin]]`; `cmd/server/` has no `tests/` today — MEASURED 2026-09-27). Each case spawns the binary with **cwd = a fresh empty temp directory** (so `dotenvy`'s parent-walking finder — MEASURED, `find.rs:32-33` — cannot reach a `.env`; the harness asserts no `.env` exists in that directory or any ancestor before spawning) and an **explicit, complete environment** (`env_clear()` then): `ADMIN_PASSWORD=<random>`, every other variable `Config::from_env` requires set to a value `libs/site-core/config.rs`'s own unit tests accept, `DATABASE_URL=<temp file>`, `STATIC_DIR=<temp dir containing index.html>`, `PORT=<a free port the harness picks by binding and releasing a loopback listener>`, `CONTENT_DIR=<temp dir>`, `ANTHROPIC_API_KEY` unset, and `CONTENT_PUSH_TOKEN_SHA256` per case. Cases: unset → exit non-zero within 10 s and stderr contains `CONTENT_PUSH_TOKEN_SHA256`; `abc` → non-zero, names the variable; three digests → non-zero; one or two → the process logs the `listening` line (`main.rs:103`) and `GET /api/health` on that port returns `200`, then the process is killed; 64 zeros → starts, and every push → `401`. Positive control: another required variable from `config.rs`'s own unit tests, one that survives R-0017 Part B (so not `ADMIN_PASSWORD`), unset with a valid digest → non-zero and stderr names that variable, not `CONTENT_PUSH_TOKEN_SHA256` — proving the refusal is attributable to the one variable under test.
- Two digests → either token `200`, a third `401`.
- **Type closure (Glitch H1) — implementer's paired self-test, reviewed for assertion strength, not red-phase work.** Four `compile_fail` doc-tests on `PushTokenDigest` (`a == b`, `a.0`, `a.as_ref()`, `&*a`) **plus one positive companion in the same doc block set that must compile and run**: `let a = PushTokenDigest::from_token(b"x"); let b = PushTokenDigest::from_token(b"x"); assert!(a.ct_eq(&b));`. The companion bounds the negatives: if the type is absent or not `pub`, the companion fails, so the four cannot pass on an unresolved-name error. A source-shape test additionally asserts `libs/site-core/routes/content/auth.rs` (or wherever the type lives; the test names the file) contains no `impl PartialEq`, `impl Eq`, `impl Deref`, `impl AsRef`, no `#[derive(` list containing `PartialEq` or `Eq` on that type, and that the struct's field is not `pub`. Which compiler error each negative fires is not pinned (RULED `10632`, E-9): the residual is a negative failing for a wrong reason, possible only if its own test line is malformed, which the reviewer reads (D-27).
- Source-text control (Warden W3-10, W4-7), computable: **no non-comment line** of `routes/content/auth.rs` contains the token `==`, `!=`, `.eq(`, `.ne(`, `.cmp(` or `hex::encode` (count comparisons live outside the auth module; the `compile_fail` doc-tests' `a == b` sits in a doc comment); `libs/site-core/config.rs` contains the line `push_token_digests: Vec<PushTokenDigest>` and no `push_token` field of type `String` or `Vec<String>`; positive control: seed a `presented_hex == configured_hex` line in `auth.rs` → fails; seed `Vec<String>` on the field → fails.
- `.env.example`'s `CONTENT_PUSH_TOKEN_SHA256` line fails `Config` validation; a generated digest passes.
- Token-leak: no event **or span field** contains any 8-byte-or-longer window of the token sentinel for `200`, `401`, `422` — with the **positive control (Warden F-9)** run once and recorded: a deliberately un-skipped instrumented argument whose value must appear in a captured span field, then removed.

### R-0007 — Justfile recipes turn the token runbook into a mechanism; the operational transitions live in a separate file

**Decision (RULED `5228` (11)).** `justfile`: `push-token-setup`, `push-token-verify` (idempotent), `push-bundle-seed` (Bolt F3) and a pointer comment. `ops/token.just` (never loaded by the main justfile; `just -f ops/token.just <recipe>`): `rotate-token`, `revoke-token`, `retire-previous` — `retire-previous` placed here as this spec's call under the ruling. All six are `@`-quiet `#!/usr/bin/env bash` recipes with `set -euo pipefail`. job never runs them.

| Recipe | Effect | Idempotent |
|---|---|---|
| `push-token-setup` | If the Keychain item exists: run the verify logic; exit 0 only on `accepted`, printing `push-token: already set up (fly: accepted)`; otherwise `(fly: rejected\|unreachable)` and non-zero (Warden W-16). Else: generate → store with the no-trust ACL → set the Fly secret → verify → `push-token: set`. | yes |
| `push-token-verify` | Read the token (prompts); `GET https://peter.manahan.io/api/content/bundle` with `curl --retry 5 --retry-delay 5 --max-time 20 -w '%{http_code}'` and **read the body** (Bolt B) **[SC]**; print exactly one of `push-token: accepted (hashes configured: N)` on `200`, `push-token: accepted (no bundle loaded)` on `404` **whose body is `{"error":"no_bundle"}`**, `push-token: rejected (401)`, `push-token: unreachable (<status or curl error>)` for anything else, including any other `404`. Exit 0 only on the two `accepted` forms. | yes |
| `rotate-token` | Scheduled rotation: current → `-previous`; new; store; Fly `<new>,<old>`; wait for health then verify; print `push-token: rotated (2 hashes configured — run retire-previous after the next successful push). For suspected theft run revoke-token.` | no |
| `revoke-token` | Incident response: new; store; delete `-previous`; Fly new digest only; wait then verify; `push-token: revoked; 1 hash configured; effective after the machine restarts`. | no |
| `retire-previous` | No `-previous` → `nothing to retire`, exit 0. Else Fly current only; delete previous; verify. | yes |
| `push-bundle-seed <file>` (`justfile`; Bolt F3) | The one-time seed push from the Mac (R-0019 step 6), under every rule below. Read the token (prompts); `curl --max-time 60 -w '%{http_code}' -X PUT -H 'Content-Type: application/json' -H @- --upload-file <file> https://peter.manahan.io/api/content/bundle` with the `Authorization: Bearer …` line fed on **stdin** (`-H @-`, MEASURED `man curl` 8.7.1); print the status and the response body (it carries `sha256` and counts, never the token); exit 0 only on `200`. The hostname is a literal in the recipe. | no (a second run is an idempotent re-push) |

Rules:
1. **Refuse inside a Claude session:** `CLAUDECODE` set → stderr refusal, exit `2`, before any call. Covers generation, rotation, revocation.
2. **The token never appears in a just variable, any process's argv, or output.** Generation `TOKEN="$(openssl rand -base64 32)"`; digest via `printf | shasum -a 256 | cut`; Keychain writes fed on stdin to `security -i` with its stdout and stderr redirected to `/dev/null`; Fly via `fly secrets import` on stdin. **Recipe bodies name no path (Warden F-4, W3-2, W4-6) [SC]:** the source scan reads, **below the shebang line**, the body of each recipe in rule 5's computed set (every `justfile` recipe whose body contains the token `security` or `fly`, plus every recipe in `ops/token.just`; the `justfile`'s other recipes legitimately name paths) and fails on **every** `/` outside the permitted literals: `/dev/null`, `application/json`, `https://peter.manahan.io/api/content/bundle`, `https://peter.manahan.io/api/health`. The permitted set is a constant in the scan and a test pins it to exactly those four entries. Its outside dependency is only the text of those two files (no shell tokenization is relied on).
3. **Two read-path layers with different reach (RULED `5217` (1); Warden F-8, W3-1) [SC].** (1) The item is created with `-T ""` and neither `-A` nor a non-empty `-T`, so that **every** reader — including `/usr/bin/security` itself — prompts (`man security` MEASURED 2026-09-27: without `-T ""` the creating binary is trusted silently, and `security` is the creating binary). (2) job's push runs only from a terminal-only command that refuses under `CLAUDECODE` — **alongside** (1), unconditionally (need on `#3471`). **Reach:** (2) constrains job's push command only; an injected agent that can run commands calls `security find-generic-password` directly, and against that read of the push token (1) is the only layer; T10's residual states what it does not cover. **The probe for (1), run by Peter at a terminal, never by an agent (Warden F-7):** `security create-keychain -p <pw> /tmp/probe.keychain` → `security unlock-keychain -p <pw> /tmp/probe.keychain` (so a locked-keychain prompt cannot be mistaken for the ACL) → add **two** items: `probe-open` with `-A`, and `probe-acl` created **by piping into `security -i` the exact `add-generic-password` line the recipe feeds it, with only the service/account names substituted** — so the probe also covers how `security -i` parses the quoted empty `-T ""` → read both with `find-generic-password -w`, once interactively and once non-interactively (`security … </dev/null`). The ACL is keyed on the reading application's binary, not on whether the parent is Terminal; both reads use `/usr/bin/security`, so no separate non-Terminal spawn is needed. **Pass condition, recorded verbatim (Warden W4-4):** `probe-open` reads with no prompt and prints its value (positive control). `probe-acl` **never prints its value without Peter's click**: on each read Peter clicks Deny if a dialog appears, and the exit status and message are pasted. The non-interactive read may end in a dialog or in `errSecInteractionNotAllowed`, and either passes (which one occurs is INFERRED; the probe measures it). If a recipe rewrites an existing item with `-U`, `probe-acl` is also updated once through that same line, and a further read must still not print without a click. **A silent print of `probe-acl`'s value on any read is the one failing outcome; a `probe-open` that does not print silently voids the probe (rerun it).** The probe's range is a throwaway keychain, stated as such; the login keychain is not probed. **Cleanup:** `security delete-keychain /tmp/probe.keychain` (`create-keychain` adds the keychain to the search list), pasted. If the probe fails, layer (1) is dropped from T10's control column and **T10 has no control against a direct read** — recorded as the residual, and layer (2) is not promoted to cover it. Runbook: the item's access list is never widened.
4. **Verify without revealing.** Only the fixed lines are printed.
5. **The shim test runs the real recipes and cannot reach the real world (Warden W-05, F-5, F-6, W3-8; rule-5 ruling conditions 1–3) [SC].** `scripts/test-push-token-recipes.sh` builds a temp shim directory and runs **`just -f <file> <recipe>` itself** — the real `just`, `bash`, `shasum`, `cut` and `sleep` are placed in the shim directory by the harness as exec-wrappers (never named by a recipe), alongside recording shims for `security`, `fly`, `curl` and `openssl`. **Environment (ruling condition 1):** every recipe is started with **`env -i`** plus an enumerated allowlist and nothing else — `HOME=<temp dir>`, `PATH=<shim dir>`, `TMPDIR=<temp dir>`, `LANG=C.UTF-8`, `JUST_NO_DOTENV=1`, and `CLAUDECODE=1` only for assertion (a) — and `just` is invoked with **`--no-dotenv`** as well (MEASURED: `just 1.58.0` honours `JUST_DOTENV_COMMAND`, which runs a command at load, and `--no-dotenv`/`JUST_NO_DOTENV`; `bash` sources `$BASH_ENV`; `/usr/bin/shasum` is Perl and honours `PERL5OPT`/`PERL5LIB` — every one an inherited-environment code path, all closed by `env -i`). The harness also runs `just` with `--working-directory <temp dir>`, so a relative path in a recipe lands in the temp directory. **Wrapper resolution (condition 2; Warden W4-2, Glitch N4, N8):** the five real binaries are resolved at setup from the fixed `PATH=/usr/bin:/bin:/opt/homebrew/bin`, never from the inherited `PATH`; the harness asserts each **resolved** path (before symlink resolution) sits directly in one of those three directories and each **canonical** path is under neither the repository root nor the shim directory (an identity check, not an existence check), and fails otherwise. The order pins `/bin/bash` 3.2 on purpose: recipe bodies stay within bash 3.2 syntax, the oldest `bash` that runs them. Recipe bodies call only bash builtins and the nine wrapped or shimmed commands (`just`, `bash`, `shasum`, `cut`, `sleep`, `security`, `fly`, `curl`, `openssl`); adding one extends this list and the identity check. **The isolation mechanism is `HOME` plus the empty environment:** with `HOME` redirected the login keychain is not in `security`'s search list (INHERITED, Warden F-6 MEASURED with positive control), and `fly` under an empty `HOME` holds no credential (**MEASURED 2026-09-27**, `fly` v0.4.104: `no access token available`, exit 1; positive control under the real `HOME`). **Residual (condition 3), stated:** real `bash` can bypass the *recorder* without a `/`-leading token — `command -p security …` searches a default path, `exec 3<>/dev/tcp/…` opens a socket (both `man bash`, MEASURED) — but not the *isolation*: such a call reaches a `HOME`-isolated `security` or a credential-less `fly`. Rule 2's scan catches the `/dev/tcp` form as text. A **reconciliation test** asserts the recipe set the script drives equals the **computed** set (Warden W3-9): every recipe in `justfile` whose body contains the token `security` or `fly`, plus every recipe in `ops/token.just` (`just --summary -f …`) — so a new recipe touching the Keychain or Fly fails the test until driven; a seeded sixth recipe in **each** file fails it. Assertions per recipe: (a) `CLAUDECODE=1` → exit `2`, empty shim log; (b) no 8-byte window of the sentinel token in any recorded argv or in stdout/stderr; (c) the sentinel in `security`'s recorded stdin where expected; (d) `fly`'s stdin matches `^CONTENT_PUSH_TOKEN_SHA256=[0-9a-f]{64}(,[0-9a-f]{64})?$`, one digest for `revoke-token`/`retire-previous`; (e) `revoke-token` records no `-previous` write and one delete; (f) `push-token-setup` twice → one Keychain write; (g) `setup` with a `curl` shim returning `401` → non-zero; (h) `verify` with a `curl` shim returning `404` and body `{"error":"no_bundle"}` → exit 0 `accepted (no bundle loaded)`; with `404` and an empty body → `unreachable`, non-zero; **(j) every `add-generic-password` line on `security`'s recorded stdin contains `-T ""` and contains neither `-A` nor a `-T` followed by a non-empty value** (Warden W3-1) — seeded: remove `-T ""` from one recipe → fails; **(k) `push-bundle-seed`:** `curl`'s recorded argv contains `-H` `@-`, `--upload-file <file>` and the literal URL, and the sentinel appears only on `curl`'s recorded stdin as `Authorization: Bearer <sentinel>`; with a `curl` shim returning `401` → non-zero. Wired into `just check`; CI wiring parked on `#3488`.

*Anchors: L-8; `5228` (11); P-GuaranteeByMechanism; P-SecurityLayered; P-TrustworthySignal.*

**Acceptance.**
- The shim script passes, and fails on each seeded defect, recorded: (i) remove a `CLAUDECODE` guard; (ii) `-w "$TOKEN"` on `security`'s argv; (iii) `echo "$TOKEN"`; (x) remove `-T ""` from `push-token-setup`'s `add-generic-password` line (assertion (j)); (xi) put the bearer header on `push-bundle-seed`'s `curl` argv (assertion (k)). The **source scan** fails on each of: (iv) `/usr/bin/security`; (v) `/bin/`; (vi) `/usr/local/bin/`; (vii) `PATH=/usr/bin:$PATH`; (viii) `$(/usr/bin/x)`; (ix) `x=/bin/y`; (xii) `\/usr/bin/x`; (xiii) `${X:-/usr/bin/fly}`; and passes the computed set's real recipes (whose only `/` characters sit in the four permitted literals, and whose shebang is above the scanned range); the permitted-set pin fails on a seeded fifth entry.
- The reconciliation test fails on a seeded sixth recipe in `justfile` that mentions `fly`, and on a seeded fourth recipe in `ops/token.just`; a seeded `justfile` recipe mentioning neither `security` nor `fly` is not required.
- The wrapper identity check fails when the harness is run with a shim named `just` placed under the repository root and that path prepended to the fixed resolution `PATH` (seeded).
- The probe record (rule 3) is in the PR before the token recipes merge, with both items' outcomes and the cleanup line.
- `just --list` hides the three transition recipes; `just -f ops/token.just --list` shows them.

### R-0008 — Storage on the volume: two files, one writer, `current.json` never absent, previous generation kept, boot-time fallback

**Decision.** `CONTENT_DIR` (env; default `data/content`; Fly `/app/data/content`) holds `current.json`, `previous.json`, transient `incoming.json.tmp` and `previous.json.tmp`. **One writer:** the store module `content/store.rs` is the only code that opens any of these names for writing. **Filesystem seam (Glitch H3; P-TestableByDesign — the filesystem is a true system boundary):** the store is constructed with a `ContentFs` collaborator exposing exactly the operations the sequence below uses — `write_and_fsync(path, bytes)`, `copy_and_fsync(from, to)`, `rename(from, to)`, `fsync_dir(dir)`, `exists(path)`, `read(path)` — production wires the `std::fs` implementation, tests wire a fake that wraps the real filesystem rooted at the test's `CONTENT_DIR` and fails or blocks at a named step (Glitch N6). No `#[cfg(test)]` hook, no env var.

Push sequence (Warden F-20) **[SC]**: write `incoming.json.tmp` + fsync → **only when the loaded generation is `current.json` (`source == current`)**, copy `current.json` → `previous.json.tmp` + fsync → rename `previous.json.tmp` → `previous.json` (both skipped on the very first push, so `previous.json` stays absent — Glitch M4 — and whenever the server is running from `previous.json` or is `absent` over an unloadable `current.json`, so an unloaded file never overwrites the only good generation) → rename `incoming.json.tmp` → `current.json` → fsync the directory → swap the `Arc`. `current.json` is never absent once it has existed; an error at any step before the final rename → `500 storage_error` with `current.json` byte-identical.

Boot: try `current.json` (R-0004 steps 1–4; rule (g) is a WARN at boot) → `loaded`/`current` → else ERROR, try `previous.json` → `loaded`/`previous` → else `absent` with `reason`. No runtime re-read (D-7).

*Anchors: L-5 (RULED `5331`); V-Reliability; V-Reversibility; D-5.*

**Acceptance.**
- Push A then B: `current.json` == B, `previous.json` == A; `GET` reports `sha256(B)`.
- Chat held open against the slow mock, push B: the held request's outbound `system` carries A's sentinel; the next carries B's (S3).
- `current.json` = `{not json`, valid `previous.json` → `loaded`, `source: previous`, file untouched.
- Neither file → `absent`, `reason: no_file`; `CONTENT_DIR` exists afterwards.
- `current.json` at an unsupported version, no `previous.json` → `absent`, `reason: unsupported_format_version`.
- `current.json` whose rendered prompt is 120 001 bytes → `loaded` with WARN `content.bundle.over_cap`; the same document pushed → `422`.
- DB path unwritable, valid `current.json` → health still reports the bundle `loaded`.
- Single push from `absent` → `200`, `state: loaded`, `current.json == body`, **`previous.json` absent**, `unchanged: false` (Glitch M4).
- **Fault injection through the seam (Glitch H3):** the fake `ContentFs` fails the `rename(incoming.json.tmp, current.json)` step → `500 storage_error`, `current.json == A`, `previous.json == A`, and the next push succeeds; the fake fails `copy_and_fsync` → `500`, `current.json == A`, `previous.json` unchanged from before; the fake blocks at `write_and_fsync` → used by R-0005's serialization test. **Positive control, recorded once as a design-review artifact, not automated:** the same `rename` fault against the rejected rename-first order (written once in a throwaway branch by the implementer, never merged) leaves `current.json` absent — pasted into the B1 PR.
- **Push from a `previous` boot:** with `current.json` corrupt, `previous.json` = A and `source: previous`, push B → `200`, `current.json == B`, `previous.json == A`, `source: current`; the fake failing the final rename instead → `500`, `previous.json == A`, and a restart boots `loaded`, `source: previous`.
- **Single-writer scan (Warden W3-12, Glitch M7, Bolt F2), range stated:** a source-text test ranges over every `.rs` file under `libs/site-core/` and `cmd/server/` **whose path has no `/tests/` component** (integration tests are out of range; inline `#[cfg(test)]` modules are in range and must use the store's constants) and asserts the four literals `current.json`, `previous.json`, `incoming.json.tmp`, `previous.json.tmp` each appear in exactly one file, `libs/site-core/content/store.rs`; the range is asserted non-empty and to contain `store.rs`. Seeded: add `File::create("current.json")` in `export.rs` → fails naming `export.rs`; add `"previous.json.tmp"` in `routes/content/mod.rs` → fails. **[SC]** (the earlier "referenced by one module" predicate was contradicted by R-0016's need to reference `CONTENT_DIR` to refuse it).

### R-0009 — The prompt builder reads the bundle only; tiers render as nonce-bound blocks; articles render as summary and link; absent sections are named

**Decision.**
- `build_system_prompt(bundle: &LoadedBundle) -> String`, `build_fit_prompt` likewise; neither takes a `Connection`. Handlers clone the `Arc<LoadedBundle>` outside the DB lock and without waiting on the push lock (R-0005). `LoadedBundle` carries its `LoadNonce`.
- **Tier regions (RULED `5217` (4))** in order after `CORE_DIRECTIVE` and `SECURITY_BLOCK`: public, unpublished, private, each opened and closed by a label line built from `PROMPT_STRUCTURAL_TOKENS` (R-0004) and a **per-load nonce** (Warden F-2, W3-6) **[SC]**: `=== BEGIN PUBLIC {nonce} ===` … `=== END PUBLIC {nonce} ===`; each field inside `<<field {nonce}>>` … `<<end {nonce}>>`. The nonce is 16 hex chars from the CSPRNG, **drawn once when the `Arc<LoadedBundle>` is built — at boot or at a push's swap — and reused for every render of that load**, so the system block is byte-identical across requests and the `cache_control: ephemeral` the chat and fit paths already set (MEASURED, `anthropic_stream.rs:153`, `routes/ai.rs:242`) can hit; a per-render nonce would make every system prompt unique and defeat the cache, raising T3's per-request spend. The producer still cannot predict the nonce (it is drawn after the push is accepted), and a visitor who extracts it gains nothing because visitor text never enters the system block (MEASURED, `routes/ai.rs:140, :248, :259`). A constant `PROMPT_NONCE_NOTICE`, rendered before the first region label and carrying this load's nonce, tells the model that only delimiters carrying that nonce are structure and everything between them is data. Because R-0004 (f) refuses `<<`, `>>` and `===`-led lines in values, **a value cannot contain any delimiter form the renderer emits**; a mid-line `===` or a `#` after the first non-blank character is accepted by design (R-0004's accepted cases), and the nonce is the load-bearing control against a value that resembles a delimiter without matching one (Warden W3-11). The nonce also makes a future delimiter change safe without a validator change. A test asserts every delimiter form the renderer emits is refused by rule (f) (the tie between the two constants).
- **Articles:** `### {title} ({published_at})`, summary, URL — inside the field block; never `content`.
- **Absence block** after the regions: a region in the same delimiter form, `=== BEGIN ABSENT {nonce} ===` … `=== END ABSENT {nonce} ===`, holding each empty non-public reporting section's R-0010 identifier (`instructions`, `values`, `faq`, `gaps`, `profile.nonpublic`, `experience.nonpublic`, `skills.nonpublic`) on a line of its own, with a decline instruction. Presence, position, framing and identifiers are normative; the instruction's wording is free (LEAN, frame §4).
- `ai/context.rs` has no `rusqlite` and no `models::` import.

*Anchors: L-3, L-9, L-10; P-MinBlastRadius; V-DataMinimization.*

**Acceptance (at the outbound-model-call boundary, `test_ai_chat_endpoint.rs` pattern).**
- Fixture loaded, `POST /api/chat`: the captured `system` contains the three region labels in order, with one nonce shared by every delimiter in the render; each prose sentinel sits inside `<<… {nonce}>>`…`<<end {nonce}>>` within the region its `x-tier` names (a test derives the expected region per sentinel from the schema, so a tier change moves the assertion); `SECURITY_BLOCK`'s first line precedes the first region label; **two renders of one load carry byte-identical `system` blocks; a second push (a different body) then a render carries a different nonce; two boots over the same `current.json` carry different nonces** (Warden W3-6).
- The article renders title, `published_at`, summary sentinel and URL on consecutive lines inside its field block.
- `values`, `gaps`, `instructions` empty, all else populated → the lines of the ABSENT region that equal an R-0010 identifier are exactly those three; all non-public empty → all seven; none empty → no ABSENT region.
- The captured `system` contains `PROMPT_NONCE_NOTICE` verbatim with this load's nonce before the first region label, and that nonce occurs exactly once outside delimiter lines; seeded: delete the notice → fails. This checks presence and position, not model behaviour.
- `POST /api/fit` → the same base prompt plus the fit wrapper.
- Source-text guard: no `rusqlite` token and no `models::` import in `context.rs`.
- The fixture's rendered prompt is ≤ 120 000 bytes.
- The delimiter/rule-(f) tie test passes; seeded: change a delimiter constant to `[[end]]` → the tie test fails.

### R-0010 — Missing data: four observable states

**Decision.** Non-public reporting sections: `instructions`, `values`, `faq`, `gaps`, `profile.nonpublic`, `experience.nonpublic`, `skills.nonpublic` — "empty" for a section-object means every unpublished and private field of every item is `""`, `[]`, `{}` or `null` **[SC]**.

| State | Definition | chat / fit |
|---|---|---|
| `absent` | No valid bundle (RULED `5331`) | `503 {"error":"ai_unavailable","reason":…}` after the rate-limit check, before any upstream call |
| `public_only` | All seven empty | proceeds; absence block names all; `X-Folio-Context: public_only` |
| `partial` | Some empty | proceeds; names the empty; `X-Folio-Context: partial` |
| `loaded` | None empty | proceeds; `X-Folio-Context: full` |

Key check precedes bundle check. Frontend: on `503`, `frontend/src/lib/components/ai/ChatTab.svelte` and `fitAnalysis` in `frontend/src/lib/api.ts` show the same unavailable message as their `500` path, with no automatic retry (MEASURED 2026-11-09: a `503` reads "try again" today). The `public_only`/`partial` one-line notice is `#3625`'s islands; here the header is the contract. **Reconciliation with `5221` (8) (Warden F-22):** the header discloses bundle state on an unauthenticated route; the ruling's minimization is scoped to health, and the ruled public-fallback notice (L-11 (a)) requires the client to know the state — recorded, not changed.

**Residual.** Fabrication, not leakage, under instruction-only control (L-11 (a); D-3).

**Acceptance.**
- Fresh server: `POST /api/chat` → `503`, `reason: no_file`; the mock upstream receives zero requests (positive control with a bundle loaded: one).
- Fresh server, 11 chat requests from one client → the 11th is `429`, not `503`.
- `public-only` fixture → `200`/SSE with `X-Folio-Context: public_only` on chat and fit; `partial` fixture → `partial`; full fixture → `full`.
- Key unset and no bundle → the existing `500`.
- Frontend unit tests under `frontend/src/lib/__tests__/`, with `$lib/api` mocked as `ChatTab.typed-events.test.ts` does: `api.chat()` resolving `503` → `ChatTab` shows its `500` path's message in its `role="alert"` element and calls `api.chat` once; `fitAnalysis` on a `503` rejects with that message.

### R-0011 — Health: JSON body; public shows state, reason, source and format version only; independent of the DB lock

**Decision.** `GET /api/health` → an `application/json` body `{status, db, bundle:{state, reason, source, format_version}}` (RULED `5221` (8), extended to `reason` and `source` by `10695`). `status: degraded` when `state != loaded` or `source == previous`; HTTP 200 regardless of bundle state (D-15); `503` only on `db: unavailable`, mapped, not unwrapped (`#3486`, D-11). No external consumer of the old text body is known; the implementation record states the check (Bolt OP-7).

**One producer, both paths (Bolt round-3 Critical; Warden W3-7; Bolt C superseded — Puck's decision 2026-09-27, applied here).** `routes::health_check_body` (`libs/site-core/routes/mod.rs:25-40`) is the **single producer** of the health body and gains the bundle state as an input: it takes `&DbState` and the current bundle state, and returns R-0011's JSON with its status code. Both callers use it: the routed handler `routes::health_check` (`:43-45`, the path tests reach through a bare router), **and the outermost layer's health exemption at `libs/site-core/middleware/origin_lock.rs:82-84`, which in production answers every `GET /api/health` itself without calling `next`** (its doc comment, `:10-16`; MEASURED 2026-11-09). The exemption therefore MUST be given the bundle state (through whatever state or `Extension` `build_app` already wires — an implementation choice), with a read that does not wait on the push lock (R-0005), and MUST keep returning without calling `next`: the exemption exists so health checks skip the limiter and page-hit tracking, and that stays. **Superseded (revision 2, Bolt C's remedy), kept with the reason it lost:** "the served path is exercised only by R-0019's post-deploy check … the deploy check is the one that proves production" — false on the tree: the deploy check hits the same exemption, which at that revision still rendered the text body, so the check could pass forever on a body with no `bundle` key (Bolt F1). **This requirement edits a trust-boundary control.** The implementation dispatch for it loads the `control-integrity` skill and the diff gets a Warden pre-push review before it lands. **The exemption's fail-closed shape has two halves:** a non-health request never skips the check, which `tests/test_origin_lock.rs`'s existing cases assert and which MUST stay green unmodified; a health request never reaches `next`, which those cases do not assert (they check status only), so the acceptance below adds it.

*Anchors: P-SecurityLayered — a control is itself software and fails closed, and the edit runs under security review; V-Observability — an instrument that reports success without verifying is worse than none, which is what a routed-only body would have been; V-Consistency — one producer, one shape on every path.*

```jsonc
// Illustrative — field names and value sets are normative; formatting is not.
{ "status": "ok",            // "ok" | "degraded"; HTTP 200 for both; 503 only when db == "unavailable"
  "db": "ok",                // "ok" | "unavailable"
  "bundle": { "state": "loaded",   // "loaded" | "partial" | "public_only" | "absent"
              "reason": null,      // when absent: "no_file" | "unreadable" | "invalid" | "unsupported_format_version"
              "source": "current", // "current" | "previous" | null
              "format_version": 1 } }
```

Everything else — `content_version`, `generated_at`, `loaded_at`, `sha256`, `empty_sections`, `producer`, hash count — is on the authenticated `GET /api/content/bundle` only. An absent bundle MUST NOT produce a non-200 (D-15).

*Anchors: V-Observability; P-InstrumentBefore; P-TrustworthySignal (`#3486`); V-DataMinimization (RULED `5221` (8)).*

**Acceptance.**
- **Every case below except the 61-request loop (`build_app` only, because the bare router has no limiter) runs against the app from `site_core::app::build_app` as a bare `GET /api/health` carrying no headers beyond `Host`** (the request shape Fly's health checker sends, per `origin_lock.rs:13-16`; `tests/test_origin_lock.rs:25-35` is the construction pattern), **and again through the bare-router harness**; both must return byte-identical bodies and `Content-Type: application/json` for the same state.
- Fresh server → `200`, `status: degraded`, `bundle.state: absent`, `reason: no_file`, `db: ok`; the body contains none of the keys `content_version`, `generated_at`, `loaded_at`, `sha256`, `empty_sections`, `producer`.
- After the fixture → `status: ok`, `state: loaded`; after `public-only` → `public_only`; after `partial` → `partial`.
- Poisoned DB lock → `503`, `db: unavailable`, bundle fields populated, no panic — on both paths.
- `curl --fail …/api/health` still succeeds on a fresh volume.
- **A health request never reaches `next` (Warden W4-5, Bolt B4-1, Glitch N2).** In `tests/test_health.rs`, through `build_app`: 61 bare `GET /api/health` → none is `429`. Positive control in the same test: 61 requests that the outermost layer passes to `next`, built the way `tests/test_origin_lock.rs` builds its accepted requests, reach `429` by the 61st. Seeded once (Verification item 9): replace the exemption's return with a call to `next` → the health loop returns `429` and the test fails.
- `tests/test_health.rs` asserts the JSON shape on both paths; its module doc names the two callers of `health_check_body`; the implementation record names the external-consumer check.

### R-0012 — Format-version compatibility runs both ways

**Decision.** `SUPPORTED_FORMAT_VERSIONS` is derived from the schema files embedded in the binary (one `include_str!` per `contracts/content-bundle.v{N}.schema.json`), and the server dispatches on and reports the set its `Checker` was constructed with (production: exactly the embedded set); a reconciliation test asserts every such file on disk is embedded and vice versa (P-TrustworthySignal TS2). Forward: unsupported → `422` with the set. Backward: a release introducing N keeps N-1; N-1 may be removed only in a release after production health reports `format_version: N` — a PR-body precondition (procedural, D-8) backed by the post-deploy check (R-0019) failing on `absent` for any reason but `no_file`. Version 2 is not designed (P-Defer; self-announcing: it arrives as a new schema file the reconciliation test refuses until embedded); the test-only synthetic schema below is a test double and designs nothing (D-26).

**Acceptance.** `422 supported_format_versions: [1]` for `format_version: 2` against the embedded set; the reconciliation test fails on an unembedded `v9` file (seeded); a `{1}` build over a stored v1 → `loaded`. **Through R-0004's seam (RULED `10632`, E-7 (a)):** a checker constructed with the embedded v1 schema plus a test-only schema labelled synthetic, v1's schema registered under format version 2, accepts a copy of the v1 fixture carrying `format_version: 2` (`200`, reachable because R-0001 leaves `format_version` an integer in the schema and the Rust type) and, over a stored v1, boots `loaded` with `format_version: 1` (S8). The synthetic schema is outside `contracts/`, so the reconciliation test does not see it, and is a string in Rust test source, not a tracked file in R-0014's range (Warden W4-9).

### R-0013 — The bundle never enters the image or the repository; the schema does

**Decision.** `.gitignore` `data/content/`; `.dockerignore` `data/` and `contracts/fixtures/`; `Dockerfile` `COPY contracts/content-bundle.v*.schema.json contracts/` in the Rust builder stage; fixtures read from disk by tests; `export-bundle` writes outside `CONTENT_DIR` (canonicalized, R-0016).

**Acceptance.**
- `git check-ignore data/content/current.json` prints the path; `git check-ignore contracts/fixtures/bundle.v1.json` prints nothing.
- `docker build .` succeeds; in the image `grep -r "<fixture sentinel>" /app` → no match; positive control `grep -c "format_version" /app/folio` ≥ 1.
- `strings target/release/folio | grep -c "<fixture sentinel>"` → `0`; `… | grep -c "format_version"` ≥ 1.
- Seeding check: create `data/content/current.json` with a sentinel and `contracts/fixtures/seed.json` with another, build, confirm both absent from every layer. Recorded once.

### R-0014 — A page file carrying a non-public field name fails before it can be pushed; the scan's range and exclusions are pinned

**Decision.** Layers by when they run (RULED `5217` (2)):
1. **Producer:** job's allowlist emitter (`#3471`).
2. **Before exposure, local:** the opt-out scan — a Rust test over every tracked `*.md`, `*.mdx`, `*.json`, `*.yaml`, `*.yml` (`git ls-files`) **minus a pinned exclusion list**: `contracts/fixtures/bundle.v1*.json` (they carry non-public names as data keys by design — Warden F-3) **[SC]**, `contracts/fixtures/invalid/`, `contracts/content-bundle.v*.schema.json`. `contracts/fixtures/pages/` stays in range. It fails on any frontmatter or data key at any depth in the unpublished ∪ private name set from the committed schema; asserts the positive control fails; asserts the range is non-empty; and **a test asserts the exclusion list is exactly those three entries**, so a widening is a visible diff. **Baseline (MEASURED 2026-11-09 at `c4b5235`, 28 tracked files):** no JSON, YAML or frontmatter key in the range is in that set, and `frontend/tsconfig.json`, which is not strict JSON, is read as text rather than skipped, so the scan's first run over the real tree is expected green apart from the fixtures this spec adds; any hit at implementation time is a real finding or a listed exclusion with a reason.
3. **Before exposure, on push:** `.githooks/pre-push` running the same scan, `core.hooksPath = .githooks` via `just setup-hooks`. The layer is per clone and nothing installs it automatically, so `just check` fails, naming `just setup-hooks`, when the directory `git rev-parse --git-path hooks` names holds no `pre-push`.
4. **After exposure:** CI.
Every Astro collection schema is `.strict()` (`#3625`).

*Anchors: L-2; P-SecurityLayered; P-GuaranteeByMechanism; P-CheckCoversTheClaim; P-TrustworthySignal.*

**Acceptance.** Control file fails; every in-range file passes; seeded `why_joined: x` on the fixture page → fails; seeded tracked `notes/anything.yaml` with `dealbreakers:` → fails; `summary:` passes; emptied range → `scan_range_empty`; name set equals the schema's non-public set; the exclusion-list pin fails on a seeded fourth entry; the hook file exists; after `just setup-hooks`, `git rev-parse --git-path hooks` names the directory holding it; with `core.hooksPath` unset, `just check` fails naming `just setup-hooks`.

### R-0015 — Synthetic fixtures, obviously fake, and no real personal data in any fixture or test file

**Decision.** Tree: `contracts/content-bundle.v1.schema.json`; `contracts/fixtures/bundle.v1.json` (every section populated; "Alex Rivera", the existing seed persona); `bundle.v1.public-only.json` (**every unpublished and private field empty, `self_rating: null`** **[SC]**); `bundle.v1.partial.json` (values/gaps/instructions empty); `invalid/` (one file per R-0004 negative case, named for the rule); `pages/` (fake Astro content-collection files plus the `.control` file). Every prose value is a sentinel `FIXTURE-<section>-<field>[-n]`. `just push-bundle-dev` pushes `bundle.v1.json` to `http://localhost:3000` with the token from the gitignored `.env` (R-0006's `dev-push-token`); no token or digest is committed. **PII check (Warden F-17, W3-4, W4-1) [SC]:** a test ranges over **every tracked file (`git ls-files`; content read as raw bytes, so a binary file is scanned and a read error fails the test) minus a pinned exclusion list** — `Cargo.lock`, `frontend/package-lock.json` (machine-generated), `docs/` (reviewed prose; this spec's own R-0004 text carries an illustrative `user@mnemra.dev`) — the same opt-out shape as R-0014, and fails on any match of the email pattern `[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}` whose domain is not `example.com`/`example.org` and whose full text is not on a pinned two-entry allow-list of non-personal strings — `noreply@anthropic.com` (`Cargo.toml:11`) and `user@mnemra.dev` (R-0004's userinfo fixture) — or any phone-shaped string (`\b\d{3}[.\-\s]\d{3}[.\-\s]\d{4}\b`) **except the reserved fictional exchange `555-01xx`** (`\b\d{3}[.\-\s]555[.\-\s]01\d{2}\b` is allowed; `e2e/fixtures.ts:19` and `db/seed.rs:20` use it — MEASURED 2026-11-09). Tests pin the exclusion list to exactly those three entries and the allow-list to exactly those two, and assert the range is non-empty. **Consequences in B1:** `libs/site-core/db/fixtures/live_profile.json` and the `live` seed mode (`seed.rs:68, :142`) are **deleted** — the repo copy of real contact data would otherwise outlive both removals this spec schedules; `e2e/hero-overflow.spec.ts`'s real-domain email (`:25`, `:44`) **and its real phone number (`:28`)** are replaced with `example.com` and a `555-01xx` number. **Positive control (MEASURED 2026-11-09 at `e9fe1d8`):** with the `555-01xx` allowance and the allow-list in place, the patterns hit exactly two files — `e2e/hero-overflow.spec.ts` and `libs/site-core/db/fixtures/live_profile.json` — and on no other. The `Manahan` check stays scoped to `contracts/` and the fixture directories with its reason: the name is public site content everywhere else in the tree (V-Consistency, one sentence).

**Acceptance.** No file under `contracts/` or `libs/site-core/db/fixtures/` contains `Manahan` (positive control `CLAUDE.md`); every fixture `profile.name` is `Alex Rivera`; `bundle.v1.json` → `200`; every `invalid/` file → `422` with the expected path (table-driven, unlisted file fails); `push-bundle-dev` works; `git grep 'FOLIO_DEV_PUSH_TOKEN='` finds no value; the PII test passes at B1 and fails at `e9fe1d8` on exactly the two named files; the exclusion-list pin fails on a seeded fourth entry and the allow-list pin on a seeded third; `git ls-files libs/site-core/db/fixtures/live_profile.json` prints nothing at B1.

### R-0016 — `folio export-bundle`: the one-time seed, written outside `CONTENT_DIR`, loaded through the normal push from the Mac

**Problem.** The private tier exists only in production SQLite (`#3471`). A direct write into `CONTENT_DIR` would be a second writer (Warden W-15, Bolt OP-1).

**Decision.** `folio export-bundle --database <path> --out <path>` opens the database as the server does (`db::connect`, WAL replayed), maps the ten loaders to the v1 types (R-0002 field set only; articles mapped to summary + `https://mnemra.dev/blog/<slug>/`), fills the manifest (`producer: "folio-export-<git sha>"`, `content_version: "export-<YYYYMMDDThhmmssZ>"`), runs R-0004 validation, and writes the document. `--out` is refused when its **canonicalized** path (the parent directory canonicalized when the file does not exist yet) is under the canonicalized `CONTENT_DIR` (Warden F-21); it never touches `current.json`. The seed enters production **through `PUT https://peter.manahan.io/api/content/bundle` from Peter's Mac** — never from the machine, whose command line must never carry the raw token (Warden F-10; Bolt A) **[SC]**. Exits non-zero on validation failure and writes nothing. Lives from B1 to the drop release.

**Acceptance.** Against a seeded DB the export → `200` on `PUT` with section counts equal to the seeded counts and no absent-list key; against a WAL-only DB it sees every row (positive control: `site.db` alone → empty); `--out /app/data/content/x.json` → exits non-zero `refused: path under CONTENT_DIR`; `--out data/content/../content/x.json` → refused; a symlink under `CONTENT_DIR` pointing elsewhere → refused; an empty `candidate_profile.name` → exits non-zero, no file.

### R-0017 — Removal of the admin, login, sessions, login limiter and the public content JSON routes

**Decision.** Two parts, precondition-gated (D-14); order is ruled: **the bundle release first, then Astro; the overlap is open-ended and accepted** (RULED `5224` (10)).

**Part A — in release B1:** delete `/admin/ai`, `routes/admin/{faq,gaps,instructions,values}.rs`, the write functions of those models, **`libs/site-core/db/fixtures/live_profile.json` and the `live` seed mode** (R-0015). The AI-only sections of the experience, profile and skills admin forms become **dead writes** until Part B; CLAUDE.md's `<landmines>` says so. The window's length is unbounded by this spec and accepted (RULED `5224` (10); Bolt OP-9).

**Part B — first release after** (1) production health reports `bundle.state != absent` and (2) the live pages are served from files (`#3625` deployed): delete `auth.rs`, `routes/admin/` in full, `frontend/src/routes/admin/`, the login limiter, `ADMIN_PASSWORD` from `Config` and the CLAUDE.md guardrail (reconcile every copy in one change, including the value `cmd/server/tests/startup.rs` sets; that test's attribution control stays), `test_password.rs`, `admin_login()`, the admin-session exemption and the pre-limit `admin_sessions` lookup in `global_rate_limit_middleware` (D-9's interim window closes here), `routes::public_router()` and every public content route, `serve_avatar` (agents become page files, RULED `5311`), and the orphaned `models::*` read paths.

**Acceptance.** Part A: `GET /api/admin/faq` (authenticated) → `404` after B1; `git ls-files libs/site-core/db/fixtures/live_profile.json` prints nothing. Part B: `POST /api/admin/login` → `404`; `grep -rn "ADMIN_PASSWORD" --include=*.rs --include=*.toml --include=CLAUDE.md --include=docker-compose.yml .` → zero (positive control at the parent commit → matches); the binary starts with `ADMIN_PASSWORD` unset; `GET /api/profile` → `404`; the PR body quotes the health line and names the Astro release SHA.

### R-0018 — The content tables are dropped in a later release, with the rollback horizon marked

**Decision.** Migration `NNN_drop_content_tables.sql` at **the next free number when authored** (`007` exists on `main`), registered in `db/schema.rs`'s `MIGRATIONS` array, drops `candidate_profile`, `experiences`, `skills`, `education`, `gaps_weaknesses`, `values_culture`, `faq_responses`, `ai_instructions`, `projects`, `articles`, `links`, `agents`, `admin_sessions`, and removes `export-bundle`, `seed_test_data()` and every orphaned `models::*` module. `site_config`, `rate_limits`, `page_hits`, `_migrations` stay.

**Preconditions, in the PR body:** (1) Part B live for at least one content push; (2) a WAL-inclusive DB copy taken after the last admin write, outside every repository (`<backup dir>/`), with `PRAGMA integrity_check` and the thirteen row counts pasted; (3) `previous.json` and `current.json` copied to the same location; (4) **the oldest image ref still safe against the post-drop schema** (the release that ships Part B) recorded in the PR body **and as a CLAUDE.md landmine** (Bolt OP-6): `flyctl deploy --image <older ref>` after this release is an outage, not a rollback.

**Acceptance.** Migration test from the prior schema with rows in every dropped table → tables gone, `site_config` unchanged, `_migrations` carries the new version; the PR body carries the four artefacts; `cargo build` succeeds with no `models::{profile,…}` module; CLAUDE.md contains the landmine line naming the image ref.

### R-0019 — Cutover runbook: the site never goes dark, and every check reads the body and is itself tested

**Before B1 merges (gate; `## First real run` adds the export dry run):**
1. `#3558` is `done` (`brain task get 3558`, the ruling's words, RULED `5217` (3)) **and** an observable production post-condition pasted (Warden F-16, W3-3) **[SC]**: the URL of the latest `deploy.yml` run on `main` showing its post-deploy check steps green, plus **`git merge-base --is-ancestor 99b4fd8d1672acc79b14600b84b4d1294e9bf717 <that run's head_sha>` exiting 0**, with both SHAs pasted — `99b4fd8` is the commit that last touched `deploy.yml` (MEASURED 2026-09-27), and `deploy.yml` runs on every push to `main` (`on: push: branches: [main]`), so the latest run's commit is `main`'s head, not the workflow's last-touch commit; ancestry is what proves the run executed a workflow version carrying those steps. Nothing about the check's content is stated here.
2. `push-token-setup` run from Terminal; the rule-3 probe record attached. *Post-condition:* `fly secrets list` shows the variable; `verify` says `unreachable` (expected until B1).
3. job's build passes `?dry_run=true` against a local B1 server; `200` body pasted (fallback: `## First real run`).
4. `fly.toml [env]` gains `CONTENT_DIR`; `docker-compose.yml` passes `CONTENT_PUSH_TOKEN_SHA256` and `CONTENT_DIR`. *Post-condition (Bolt F5):* the three added lines quoted from the B1 PR diff.

**B1 deploy — AI unavailable from step 5 to step 7; target under one hour of operator time; on any failure stop, pages keep serving.**
5. Merge → deploy → health `absent/no_file`, pages `200`.
6. `fly ssh console -a folio-prmanahan -C "./folio export-bundle --database /app/data/site.db --out /tmp/seed.json"`; `fly ssh sftp get /tmp/seed.json <backup dir>/seed.json`; then from the Mac, from Terminal, **`just push-bundle-seed <backup dir>/seed.json`** (R-0007; the token read from the Keychain with a prompt, fed to `curl` on stdin, never on argv — Bolt A, Bolt F3; the push targets the **public hostname**, a literal in the recipe) **[SC]**. *Post-condition:* the `200` body pasted; its `sections` counts beside the live public API's counts and equal; **its `sha256` recorded — it is job's first baseline (step 8)**.
7. Health `loaded`/`current` **on the public hostname**; chat answers from a browser.
8. **Delete the machine copy** (Warden F-11) **[SC]**: `fly ssh console -C "rm /tmp/seed.json"`; *post-condition:* `fly ssh console -C "ls /tmp/seed.json"` → "No such file", pasted. **Hand job, by the filed task, both the Mac copy's path and step 6's `sha256` as job's recorded last-push digest** (Bolt F4) — client contract §2 fails closed with no recorded baseline, so without this handoff step 9 stops on the mismatch check.
9. First real push from job: job's pre-push `GET` shows step 6's `sha256`, job proceeds; `content_version` changes; `sha256` equals job's digest.

**Post-deploy assertion (mechanism, `deploy.yml`; allow-list polarity — Bolt F1, Warden W3-7):** the step reads the `/api/health` body from the public hostname, parses it as JSON, and **passes only when all of** `.bundle.state ∈ {"loaded","partial","public_only"}` **and** `.bundle.source == "current"`, **or** `.bundle.state == "absent"` **and** `.bundle.reason == "no_file"`. Everything else fails the workflow: a body that is not JSON, a missing `bundle` key, a missing `state`/`source`/`reason` key, `source == "previous"`, any `absent` reason but `no_file`, any unlisted value. **The assertion is tested (Warden F-13, Glitch N7) [SC]:** health bodies are committed under `.github/fixtures/health/` — pass: `absent-no_file.json`, `loaded-current.json`, `partial-current.json`, `public_only-current.json` (one per allow-listed state); fail: `absent-unsupported_format_version.json`, `absent-invalid.json`, `absent-unreadable.json`, `loaded-previous.json`, `no-bundle-key.json` (`{"status":"ok","db":"ok"}`, the pre-B1 body shape), plus the non-JSON text file `ok.txt` (the pre-`#3554` text body) which must fail — and `scripts/test-deploy-assertion.sh`, wired into `just check`, runs the workflow's exact expression over every committed body, asserting each verdict.

**Post-restore step (Bolt OP-4):** job's §2 check stops on the restored digest by design. Peter runs the baseline adoption (client contract §2), then job re-pushes; confirm the authenticated `GET`'s `content_version`. The same step applies after a fallback boot (S4, S16).

**Acceptance.** PR body carries every post-condition; `deploy.yml` contains the assertion; `scripts/test-deploy-assertion.sh` passes on every committed body and fails on a seeded expression change (flip the `source` test to `!=` → `loaded-previous.json` passes → the script fails naming it) and on a seeded body with no listed verdict; the assertion text in `deploy.yml` and in the script are asserted identical (one source).

### R-0020 — Data-handling record

**Decision.** The record, at field granularity via the schema's `x-tier`. Subject: the candidate. Visitor data (`rate_limits`, `page_hits`) is unchanged by this spec and not re-recorded here.

| Category (fields) | Purpose | Retention | Recipients beyond this surface |
|---|---|---|---|
| Bundle **public** tier | Render the prompt without page files (L-3) | Until replaced; one previous generation; volume snapshots (daily, 5-day, INHERITED) | Anthropic (system prompt on every chat/fit request; retention per Anthropic's API data policy, not controlled here); CDN (transit); Fly (volume, snapshots); visitors via model output |
| Bundle **unpublished** and **private** tiers | Context the model may use / may be informed by (R-0009) | Same | Same; visitors receive it only through model output under instruction-only control (`#2753`) |
| **Seed export** — machine copy `/tmp/seed.json` (R-0019 step 6) | One-time seed | **Deleted at step 8, with the `ls` post-condition pasted** (Warden F-11) | None |
| **Seed export** — Mac copy `<backup dir>/seed.json` (R-0019 step 6) | job's initial source | Until `#3471`'s encrypted off-machine backup is recorded done (RULED `5224` (12)); **its deletion is an acceptance item on `#3471`**, fired by the backup landing (Warden F-12) | None |
| **WAL-inclusive DB copy** in `<backup dir>/` (R-0018) | Rollback material for the drop | Until the drop release has been live for one push cycle; deletion is a step in the drop PR's checklist | None |
| **Export dry run** — WAL-inclusive DB copy and its export in `<backup dir>` (`## First real run`) | Prove production content passes R-0004 before B1 removes its editors | Deleted when the dry run has passed, before B1 merges | None |
| Push token digest | Authorize the two content routes | Until rotated or revoked | Fly (secret store) |
| Push token (raw) | Authorize pushes | Until rotated/revoked; Keychain on Peter's Mac | CDN (TLS termination, transit only); Fly proxy |
| Import log lines (digest, `content_version`, counts, failed-auth **counts per window**) | Operate and audit the write path | Fly log retention (platform default) | Fly (logs) |
| Public health (`state`, `reason`, `source`, `format_version`) and the `X-Folio-Context` header | Operability; the ruled public-fallback notice (R-0010) | Live only | Anyone — none of it is content or timing (RULED `5221` (8), extended by `10695`; R-0010's reconciliation) |

**Fields the server no longer holds:** `would_do_differently`, `manager_would_say`, `honest_notes`, `salary_*`, `last_used`, `email`, `phone`, social URLs, availability, article `content`, `tags`, `links`, `agents` (page files). **Repo copy removed in B1:** `live_profile.json` (R-0015).

**Acceptance.** The private+unpublished field list equals the schema's non-public set (a test); the "removed" list is asserted absent from the schema; the drop PR checklist contains the WAL-copy deletion; `#3471` carries the Mac-copy deletion item (its filing is pasted in the B1 PR).

### R-0021 — Logging for the new surface

**Decision.** Structured `tracing` events:
- `INFO content.bundle.imported` — `content_version`, `format_version`, `sha256`, `bytes`, section counts, `empty_sections`, `dry_run`, `unchanged`. No client address (D-28).
- `WARN content.push.unauthorized_count` — one line per window with the count; never per attempt, never a header value. **The window is event-relative (RULED `10632`):** the first unauthorized attempt after idle opens a 60-second window, and the line is emitted at its close whether or not another attempt arrives. Idle means no window is open. An unauthorized attempt is any `401` from the content routes, `PUT` or `GET`, with a missing or a wrong token. A window is half-open: an attempt at exactly its close instant opens the next one. The aggregator takes its clock as a collaborator that both reports the time and waits until an instant, real in production; a test's fake releases pending waits when advanced (D-25).
- `WARN content.bundle.rejected` — `error_count`, first five `path`+`rule` pairs in R-0004's truncated form. Never values.
- `WARN content.push.rejected_public_only` — the `409` guard fired.
- Boot: `INFO content.bundle.loaded` (+ `source`), `ERROR content.bundle.load_failed` (`path`, `reason`), `WARN content.bundle.loaded_previous`, `WARN content.bundle.over_cap`.
- `WARN content.prompt.degraded` once per load naming empty sections.
- Handlers use `#[tracing::instrument(skip_all, fields(...))]` with fields listed explicitly; no argument is recorded by `Debug`.

**Acceptance.** A captured-subscriber test asserts each event fires on its trigger with the named fields; a push with a sentinel value produces no event or span field containing the sentinel (positive control: the `sha256` is present); through the fake clock, without waiting out the real window: 30 failed attempts (mixing `PUT` and `GET`, missing and wrong token) and then no further request → no line before the clock reaches the window's close, and exactly one `unauthorized_count` line with `count: 30` after it; an attempt at exactly the close instant is counted in the next window; no attempt → no line; a `200`, a `401` and a `422` each produce no span field containing any 8-byte window of the token sentinel, with the span-capture positive control of R-0006 recorded once.

## Scenarios

The Scenarios are a curated, illustrative subset; each requirement's own Acceptance list is the exhaustive matrix, and a red-phase author draws from both (Glitch §2). **S9, S12 and S14 narrate real Fly, Keychain, Mac and job behaviour that no `cargo test` in this repository executes** (Glitch L2): they are verified by proxy — S9 by R-0007's shim harness, S12 by R-0019's runbook post-conditions and `scripts/test-deploy-assertion.sh`, S14 by job's own tests on `#3471` — never by automation against real infrastructure.

**S1 — Happy push (R-0004, R-0005, R-0008).** Given a running server with a token configured and no bundle · When job `PUT`s the fixture with `Authorization: Bearer <token>` · Then `200`, `state: loaded`, `sha256` = SHA-256 of the body, `unchanged: false`; `current.json` equals the body.

**S2 — Invalid push leaves the live bundle alone (R-0004).** Given S1 · When a document with `salary_min` on `profile` is pushed · Then `422` with `path` `/profile/salary_min`; `GET`'s `sha256` unchanged.

**S3 — Push during a request (R-0008).** Given a chat held open against the slow mock with A live · When B is pushed · Then the held request's `system` carries A's sentinel; the next carries B's.

**S4 — Corrupt file at boot (R-0008).** Given `current.json` is `{not json` and `previous.json` valid · When the process starts · Then health `loaded`, `source: previous`, `status: degraded`; ERROR names `current.json`.

**S5 — Fresh volume (R-0010, R-0011; RULED `5331`).** Given no files · When a visitor loads a page and opens the chat · Then page `200`, health `200 absent/no_file`, `POST /api/chat` → `503 ai_unavailable`; the mock upstream receives nothing.

**S6 — Public-only bundle (R-0009, R-0010).** Given the public-only fixture live (every unpublished and private field empty, `self_rating: null`) · When a visitor chats · Then the outbound `system` carries public sentinels only, the absence block names all seven, and the response carries `X-Folio-Context: public_only`.

**S7 — Unauthorized push (R-0005, R-0006).** Given a valid bundle live · When a `PUT` arrives with a wrong token and another with none · Then both `401`, the body never read, and one `unauthorized_count` line per window.

**S8 — Code ahead of content (R-0012).** Given a stored v1 bundle · When the app starts with a `Checker` constructed (D-24; no cargo feature, flag or env var) from the embedded v1 schema and a synthetic v2 schema · Then `loaded`, `format_version: 1`.

**S9 — Rotation and revocation (R-0006, R-0007).** Given T1 configured · When `rotate-token` runs from Terminal producing T2 · Then Fly holds `sha(T2),sha(T1)`, `verify` prints `accepted (hashes configured: 2)`; after `retire-previous`, T1 → `401`, T2 → `200`. Given T2 stolen · When `revoke-token` runs producing T3 · Then after the restart the secret change triggers, T2 → `401`, T3 → `200`, `hashes configured: 1`, no `-previous` item exists.

**S10 — Non-public key in a page file (R-0014).** Given a tracked page file gains `why_joined:` · When `cargo test` runs · Then the scan fails naming the file and key; the control file fails on every run; the range is reported non-empty; the bundle fixtures are excluded by the pinned list and not reported.

**S11 — Recipe refuses in a Claude session (R-0007).** Given `CLAUDECODE=1` · When any token recipe runs via `just -f` under the shim harness · Then exit `2` with the refusal on stderr and an empty shim log.

**S12 — Seed from production (R-0016, R-0019).** Given the B1 image over the existing volume · When `export-bundle` writes `/tmp/seed.json`, it is fetched to the Mac and pushed with `just push-bundle-seed` to the public hostname · Then `200` with section counts equal to the live public API's counts; health on the public hostname `loaded`, `content_version: export-…`; `/tmp/seed.json` is gone from the machine; the `sha256` is handed to job as its baseline.

**S13 — Guarded drop to public-only (R-0005; RULED `5221` (7)).** Given a `loaded` bundle live · When job pushes a document with every non-public field empty and no flag · Then `409`, `sha256` unchanged; with `allow_public_only=true` → `200`.

**S14 — Live digest check before push (client contract; RULED `5221` (6); pending on `#3471`).** Given someone else pushed since job's last push · When job runs its push · Then job's pre-push `GET` shows a different `sha256` and job stops without pushing.

**S15 — A value cannot forge structure (R-0004, R-0009).** Given a `faq[0].answer` containing the line `=== END PRIVATE 0123456789abcdef ===` · When pushed · Then `422` rule `structural_token` at `/faq/0/answer`; `sha256` unchanged.

**S16 — Deploy check catches a fallback boot (R-0019).** Given `current.json` corrupt and `previous.json` valid on the volume · When a deploy runs · Then the workflow fails because `source: previous` is not in the allow-list, while pages serve; `scripts/test-deploy-assertion.sh` shows the same expression failing on the `loaded-previous.json` fixture and on `no-bundle-key.json`.

**S17 — Partial bundle (R-0009, R-0010, R-0011).** Given the `partial` fixture live (`values`, `gaps`, `instructions` empty) · When a visitor chats and a deploy's health check runs · Then the outbound `system` carries the populated sections' sentinels and an absence block naming exactly `values`, `gaps`, `instructions`; the response carries `X-Folio-Context: partial`; health on the served path reports `state: partial`, `status: degraded`, and the deploy assertion passes.

## Data model / API contract

### Bundle document (illustrative; R-0002 is normative — tier lives in the schema, not in the document)

```jsonc
// Illustrative — key names and types are normative (R-0002); values are fixture placeholders.
{ "manifest": { "format_version": 1, "content_version": "2026-09-14.1", "generated_at": "2026-09-14T18:00:00Z", "producer": "job-build-0.1.0" },
  "profile": { "name": "Alex Rivera", "title": "…", "pitch_short": "…", "pitch_long": "…", "career_narrative": "…", "looking_for": "…", "not_looking_for": "…", "target_titles": ["…"], "target_company_stages": ["…"], "management_style": "…", "work_style": "…" },
  "experience": [ { "company_name": "…", "title": "…", "location": "…", "start_date": "2022-01", "end_date": null, "is_current": true, "summary": "…", "bullet_points": ["…"], "proudest_achievement": "…", "lessons_learned": "", "title_progression": "", "why_joined": "…", "why_left": "", "reports_would_say": "", "actual_contributions": "…", "challenges_faced": "", "quantified_impact": {} } ],
  "skills": [ { "skill_name": "Rust", "category": "strong", "years_experience": 3, "evidence": "…", "self_rating": 4 } ],
  "education": [ { "degree": "…", "institution": "…", "location": "…", "start_year": "2010", "end_year": "2014" } ],
  "projects": [ { "title": "…", "summary": "…", "description": "…", "tech_stack": ["…"] } ],
  "articles": [ { "title": "…", "summary": "…", "url": "https://mnemra.dev/blog/hello-mnemra/", "published_at": "2026-04-28" } ],
  "instructions": [ { "instruction_type": "tone", "instruction": "…" } ],
  "values": { "must_haves": "…", "management_style_preferences": "…", "team_size_preferences": "…", "how_handle_conflict": "…", "how_handle_ambiguity": "…", "how_handle_failure": "…", "dealbreakers": "…" },
  "faq": [ { "question": "…", "answer": "…" } ],
  "gaps": [ { "gap_type": "skill", "gap_description": "…", "why_its_a_gap": "…", "interest_in_learning": true } ] }
```

### HTTP surface

| Route | Auth | Success | Failures |
|---|---|---|---|
| `PUT /api/content/bundle[?dry_run=…][&allow_public_only=…]` | Bearer | `200` R-0005 body | `400 bad_query` · `401` · `409 would_drop_to_public_only` · `413` · `415` · `422` `{path, rule}` list · `500 storage_error` · `429` (global layer) |
| `GET /api/content/bundle` | Bearer | `200` manifest + state + `sha256` + `empty_sections` | `401` · `404 {"error":"no_bundle"}` |
| `GET /api/health` | none | `200` R-0011 body (state, reason, source, format_version only) | `503` when `db: unavailable` |
| `POST /api/chat`, `POST /api/fit` | none | unchanged; header `X-Folio-Context: full \| partial \| public_only` | **new:** `503 ai_unavailable` when `absent` |

**One content-route `404` (Bolt D, Warden W4-8):** `GET /api/content/bundle` → `404 {"error":"no_bundle"}` means the request reached the content router and no bundle is loaded. Any other `404` is not an answer from that route.

### Client contract (normative for job's push client; mechanism is job's lane, `#3471`)

1. Read the token from Keychain item `io.manahan.folio.push-token` / `folio-prmanahan` with `security find-generic-password -s … -a … -w`, stripping the trailing newline. The read prompts by design; the reader MUST NOT create or widen the item's access list. **The push runs only from a terminal-only command that refuses under `CLAUDECODE`, and never under an agent session that processes untrusted input** (T10, both layers).
2. `GET https://peter.manahan.io/api/content/bundle` first; on a `sha256` different from job's last recorded push, stop and report (RULED `5221` (6)). **job's recorded baseline before its first push is the seed's `sha256` handed over at R-0019 step 8; with no recorded baseline at all, job stops — it never treats "no baseline" as "proceed"** (Bolt F4; fails closed). `404 {"error":"no_bundle"}` is not a mismatch; any other `404` is a routing failure — stop. **Baseline adoption (need on `#3471`, pending):** after an event that changes the live digest without a push from job (a snapshot restore or a fallback boot), the ruled stop still fires first, and the only way past it is an operation job provides: Peter, at a terminal, reads the live `GET` and confirms adopting its `sha256` as job's recorded baseline. It is never automatic and refuses under `CLAUDECODE`.
3. `PUT` to the **public hostname** with `Content-Type: application/json`; `?dry_run=true` to validate; `allow_public_only=true` only when intended. Compare and record `sha256`.
4. Retry on `429`, `5xx` and connection failures with backoff; not on other `4xx`.
5. Manifest charsets: `content_version` `[A-Za-z0-9._:+-]{1,128}`; `producer` `[A-Za-z0-9._:+ -]{1,128}`. **No string value may contain a line starting with `#` or `===`, or the substrings `<<` / `>>`** (R-0004 (f)).
6. job guarantees no externally sourced text enters a bundle field without Peter's review; its page emitter is an allowlist of `x-tier: public` fields; it deletes its copy of the seed export once its encrypted off-machine backup exists (R-0020); its ingest-facing agent sessions cannot reach the Fly credential (T10; unverified and pending).

### Environment and files

| Name | Where | Value |
|---|---|---|
| `CONTENT_DIR` | env; `fly.toml [env]`; `docker-compose.yml`; `.env.example` | `/app/data/content` (Fly, compose), `data/content` (dev); holds `current.json`, `previous.json`, `incoming.json.tmp`, `previous.json.tmp` |
| `CONTENT_PUSH_TOKEN_SHA256` | Fly secret; `docker-compose.yml` (from `.env`); `.env` (dev, gitignored, generated by `just dev-push-token`) | one or two lowercase hex digests; **required at startup**; 64 zeros = kill switch (effective after restart) |
| `contracts/content-bundle.v1.schema.json` | repo; embedded; copied into the Rust builder stage | generated in tests (R-0003) |
| `contracts/fixtures/**` | repo, tests and dev only; excluded from the build context; `bundle.v1*.json` excluded from the R-0014 scan by the pinned list; `pages/` in scan range | synthetic |
| `.githooks/pre-push` | repo | the R-0014 scan; active in a clone after `just setup-hooks` |
| `.github/fixtures/health/*` (`.json` bodies and `ok.txt`), `scripts/test-deploy-assertion.sh` | repo; `just check` | the health bodies and the test of `deploy.yml`'s allow-list assertion |
| `ops/token.just` | repo; not loaded by `justfile` | `rotate-token`, `revoke-token`, `retire-previous` |
| `<backup dir>` | Peter's Mac | a directory outside every repository |
| Keychain `io.manahan.folio.push-token[-previous]` / `folio-prmanahan` | Peter's Mac | raw token(s); no-trust access list (`-T ""`, R-0006) |
| `libs/site-core/db/fixtures/live_profile.json` | **removed in B1** (R-0015) | — |

### Recipes added

`justfile`: `push-token-setup`, `push-token-verify`, `push-bundle-seed`, `dev-push-token`, `contract-update`, `push-bundle-dev`, `setup-hooks`; `check` gains `scripts/test-push-token-recipes.sh`, the recipe source scan, the hook-installed check and `scripts/test-deploy-assertion.sh`. `ops/token.just`: `rotate-token`, `revoke-token`, `retire-previous`.

## Out of scope

- **The Astro page build** (`#3625`): collections, redirects, the AI pane as islands, F1/F2 fixes, `agents` page files with images (RULED `5311`). **Obligations handed to it:** (a) every collection schema is `.strict()`; (b) the page-side FAQ suggestions list (D-4); (c) the frontend behaviour in R-0010 carried by the islands; (d) a reverse-path guard for hand edits to generated page files (frame §4). *Tripwire: self-announcing — R-0017 Part B's precondition (2) cannot be met without that release.*
- **job's build and ingestion** (`#3471`): how it holds the source, produces both outputs, reads the mnemra.dev feed (`#3560`, done), reviews externally sourced text, keeps its encrypted off-machine backup (RULED `5224` (12)), runs its push from a terminal-only command, checks the live digest before every push, provides the baseline adoption (client contract §2), keeps its ingest-facing agent sessions from the Fly credential, and deletes its copy of the seed export. Told after this spec lands, with a reference.
- **`#3558`.** Lands first (RULED `5217` (3)). Not described here.
- **`#2753(c)`.** If reversed, `would_do_differently` and `manager_would_say` join R-0002 in a schema revision. *Self-announcing.*
- **A reliability target** for the import endpoint or the degraded AI: none stated — a recorded decision (P-ReliabilityTarget). Degradation between deploys has no alert (RULED `5228` (13); D-16).
- **A rollback endpoint.** Re-push is primary; `previous.json` plus a restart is secondary (D-7). *Self-announcing.*
- **CI running `just`** (`#3488`). Parked on that task.
- **`#3491`, `#3493`, `#3484`, `#3485`, `#3481`, `#3634`.** Untouched. `#3486` closes in R-0011 (D-11).
- **Format version 2.** Not designed. *Self-announcing (R-0012).*

## Verification (the verify contract)

1. `cargo build --workspace`; `cargo test --workspace` green; clippy `-D warnings`; `fmt --check`.
2. `just check` passes; its output shows `scripts/test-push-token-recipes.sh`, the recipe source scan and `scripts/test-deploy-assertion.sh` ran.
3. `just contract-update && git status --porcelain contracts/` → empty.
4. `cargo tree -e normal -p site-core` diffed against the parent commit: exactly `subtle` added.
5. Coverage measured and reported (`#2750`).
6. Frontend `npm run check && npm run test:unit`.
7. Image check (R-0013) pasted; `docker build .` succeeds.
8. Seeded probes, each quoted once: R-0003 (schema edit; `pattern` both ways; `minimum` no-op and `items` skip against the per-keyword fixture reconciliation), R-0004 (unknown keyword refused at `Checker::new`; the seven-violation body), R-0006 (four `compile_fail`s with their positive companion; `.env.example`; source-text control both seeds; span positive control; the subprocess attribution control), R-0007 (five shim defects (i)–(iii), (x), (xi); eight source-scan defects (iv)–(ix), (xii), (xiii); permitted-set fifth entry; unlisted recipe in each file; wrapper identity check), R-0008 (the seam faults, including the `previous` boot; second writer, two seeds), R-0009 (delimiter tie; `PROMPT_NONCE_NOTICE` deleted), R-0011 (the 61-request loop's positive control), R-0012 (`v9`; the synthetic `{1, 2}` case), R-0014 (private key; opt-out file; empty range; fourth exclusion; hook absent), R-0015 (PII test at `e9fe1d8`; fourth exclusion; third allow-list entry), R-0019 (assertion script: flipped `source` test; unlisted body), R-0021.
9. The rule-3 probe record, run by Peter at a terminal, with every read's outcome and the `delete-keychain` cleanup. Recorded once each, from a throwaway change never merged: R-0008's rename-first positive control, R-0005's two controls (store lock removed; a read taking that lock), R-0011's exemption seed.
10. Health: fresh → `absent`; after `push-bundle-dev` → `loaded`; public body has none of the authenticated-only keys — **read through `build_app` as a bare `GET /api/health` with no headers beyond `Host` — the path production serves — and identical through the bare router**.

**Build-capability check:** a sandbox or permission failure is reported as such, never as verified.

## Decision levels

Set here, never at runtime (*Anchors: P-TrustThenRetro — the level is a dispatch parameter; P-RaiseDontImprovise — a durable or hard-to-reverse call is raised before acting*). **Level 3:** recommend and wait for Peter. **Level 4:** act and tell him at once. **Level 5:** act and report at the routine update. `durable` marks a row closed before the build, stating its answer. No level-3 row is open for an answer at this revision. A decision no row lists follows `P-RaiseDontImprovise`: level 3 when it rests on context the builder lacks or is durable or hard to reverse, which includes anything touching a credential, a trust boundary, a tier or a pinned list; otherwise level 5. Either way it is recorded in the report that took it.

| Decision the build will meet | Level | Answer or default |
|---|---|---|
| A new dependency, dev-dependency or feature of one | 3 | None is needed. `subtle` direct is approved (RULED `5404`); nothing else is. |
| The first run on real data (**First real run**) | 3 | Stops when it happens, with Bolt walking the runbook first. |
| An actor, an accepted residual in the threat model, or separation of privilege | 3 | The table stands as written; raise before changing it. T10's residual stands with it: layer (1) covers the push-token path only and does not close the injected-agent actor, because the Fly credential on the same Mac is not behind it. |
| A failed rule-3 probe outcome (R-0007) | 3 | T10 loses layer (1) and says so; the token recipes do not merge until Peter has answered. |
| A control that cannot be built as written: auth-before-body, startup refusal, `PushTokenDigest`'s closure, the checker's fail-closed construction, the health exemption's shape, the recipes' `CLAUDECODE` refusal | 3 | Raise it. The requirement is never narrowed to get green. |
| Widening a pinned list: R-0007's permitted literals or command set, R-0014's exclusions, R-0015's exclusions or allow-list | 3 | Default: change the file or fixture, not the list. |
| A field's tier, or a field added to or removed from R-0002 | 3 | The two-axis rule (RULED `5419`) decides the tier; a new field is a schema change reviewed in a PR. |
| The v1 wire contract: R-0001 and R-0002 field names; R-0005 and R-0011 routes, status codes, error bodies and health field names | `durable` | As written. job and `deploy.yml` consume them. |
| Credential and storage identities: Keychain `io.manahan.folio.push-token[-previous]` / `folio-prmanahan`, Fly secret `CONTENT_PUSH_TOKEN_SHA256`, `CONTENT_DIR` and its four file names | `durable` | As written. |
| R-0018's table-drop migration | `durable` | Forward-only (G-0005), after its four preconditions. |
| Visitor-facing wording: the absence block's instruction, the unavailable, `public_only` and `partial` notices | 4 | R-0009 and R-0010 fix presence, position, framing and identifiers; the build picks the rest of the wording and tells Peter. |
| Seam and harness shapes: the `ContentFs` surface, the clock, the schema-set input, test helpers, how a scan or fixture is built, internal module layout, line-reference drift | 5 | The implementer's, inside the locked property; one line in the report. |

## First real run

- **Runs on:** the production content, exported by `folio export-bundle` from the Fly volume's `site.db` and pushed with `just push-bundle-seed` to the public hostname (R-0016, R-0019 steps 5–7).
- **Attended:** yes, by Peter at a terminal; the Keychain prompt and R-0019's post-conditions need him. Bolt walks the runbook first (level 3).
- **Smallest requirement set:** R-0001…R-0006, R-0007 (whole: `revoke-token` is T1's named compensating control), R-0008…R-0013, R-0016, R-0019, R-0021. R-0014, R-0015, R-0017 Part A and R-0020 ship in the same release; the run does not depend on them.
- **Precursor:** R-0019 step 3 sends job's build to a local B1 server with `?dry_run=true`, storing nothing. Whether that build carries real content is `#3471`'s; if it does, it takes the same level-3 stop.
- **If job's build does not exist when B1 merges:** R-0019 step 3 is met by `just push-bundle-dev` with the committed fixture against a local B1 server, `200` body pasted, and job's own `?dry_run=true` moves to just before step 9.
- **Export dry run before B1 merges:** B1 removes four sections' editors (R-0017 Part A), so `folio export-bundle` over a WAL-inclusive copy of the production database in `<backup dir>` MUST first exit 0, exit status pasted. It is first contact with real data and takes the same level-3 stop.
- **Off until later:** no toggle exists or is added (P-MinBlastRadius: a toggle only where a flip reaches users faster than a rollback; a re-push or redeploy is the rollback here). Carried later: R-0019 step 9 (job's first push, after `#3471`) and the releases carrying R-0017 Part B (`#3625`) and R-0018.
- **Scope:** Peter's close-out scope is "Lock it, no build" (RULED `10632`). This section names the run for whichever build follows and schedules nothing.

## Notes for the red-phase test author

- **Separate red phase (P-TDDPairs):** R-0004 validation (table-driven over `invalid/`, one fixture per validating keyword; `path` equality and truncation; structural tokens with the line-terminator set; the capped multi-violation body), R-0005 (auth-before-body over a raw `TcpStream` on a loopback listener — a new harness, not `test_app_with_bundle`; strict flags; the `409` guard; caps **against the composed app**; dry-run; idempotence; serialization and non-blocking reads through the seam; `no-store`), R-0006 (startup refusal by subprocess from the new `cmd/server/tests/startup.rs`, cwd an empty temp dir, `env_clear()` plus the listed variables; `.env.example`; the source-text control), R-0007 (the shim harness driving `just -f … --no-dotenv --working-directory <tmp>` under `env -i` with the fixed-PATH wrapper resolution and identity check; the source scan with its prefix set; the computed reconciliation), R-0008 (seam fault injection; first push from absent; push from a `previous` boot; single writer with its range), R-0009 (per-load nonce; the delimiter/rule-(f) tie), R-0011 (both paths through `build_app` and the bare router; the 61-request loop with its positive control), R-0012 (the synthetic-v2 line, through the harness's schema-set input), R-0014 (opt-out scan; positive control; empty range; exclusion-list pin), R-0015 (opt-out PII test with its positive control run at `e9fe1d8` before the two files change in the same PR), R-0019 (`scripts/test-deploy-assertion.sh` over every committed body), R-0021 (token windows over events and spans; the span positive control is a one-time seeded test, recorded then removed; the window test through the fake clock). **Mechanical (implementer self-tests, reviewed for assertion strength):** R-0003 including R-0002's three schema-shape bullets (Glitch M1) and R-0020's field-list-equals-schema line (Glitch M2); R-0006's four `compile_fail`s with their positive companion (Glitch H1); R-0004's `Checker::new` line (Glitch N1); R-0012's reconciliation.
- **Harness:** `test_app_with_bundle(fixture_name) -> (TestServer, DbState)` **builds through `site_core::app::build_app`** (reusing `tests/test_origin_lock.rs:25-35`'s `locked_server()` construction — `tests/fixtures/static_site` as the static dir and that file's throwaway test values — and, for every request except where a test says otherwise, the same per-request setup that file's passing cases use) with the mock upstream, `CONTENT_DIR` as a per-test temp dir, a `ContentFs` and a `Checker` schema set as constructor inputs (real by default), digests as constructor inputs (never process env inside handlers). The bare-router helper stays for the R-0011 "both paths" comparison. T4: tests never mutate process env.
- **Tokens in tests:** generated per run; digests computed in-test.
- **Red state:** before the router exists every HTTP test is `404`; the red-phase author lands the router skeleton (`501`) so assertions fail for the spec reason.
- **Sentinels:** `FIXTURE-<section>-<field>[-n]`; leak assertions grep for those and for token windows, never for names.

## Decisions of note

**D-1 — Single JSON document, no archive.** *Anchors: V-Simplicity; P-StackDiscipline.* Rejected: zip/tar; a SQLite file.

**D-2 — All keys required, no serde defaults; empties explicit (`""`, `[]`, `{}`, `null` where typed).** *Anchors: the 2026-07-24 spec's D-1; P-GuaranteeByMechanism.*

**D-3 — Public-data fallback when private content is absent; unavailable only with no bundle (RULED `5331`, frame L-11).** The rejected branch (unavailable on any missing private data) is kept: it fails closed on fabrication but makes the AI unavailable whenever job pushes a public-only build. A page-file fallback with no bundle was rejected by ruling; this spec's reasoning against it (a second parser of one content schema; a code-to-content coupling) is recorded as the spec's, not the ruling's.

**D-4 — FAQ suggestions become page content.** *Anchors: frame L-3.*

**D-5 — The bundle is a file on the volume, not a SQLite table (RULED `5331`).** The ruling's ground: both forms are equally exposed. The spec's additional reasoning: a whole document replaced atomically, never queried by row; the `#3472` WAL hazard makes a file the more backup-able form; `fly ssh console` can `mv` a file and the image has no `sqlite3`. Rejected: a `content_bundle` table.

**D-6 — Dry-run on the endpoint instead of a `validate-bundle` CLI.** *Anchors: V-Simplicity; P-LockContract.*

**D-7 — No rollback endpoint; one previous generation kept; the secondary path restarts the process.** Re-push from job is primary. The secondary — `mv previous.json current.json` over `fly ssh console` — takes effect only after `fly machine restart <id>`, a brief full-site interruption (Bolt OP-2). After a snapshot restore the bundle is valid, old and healthy-looking; R-0019's post-restore step re-pushes (Bolt OP-4). Rejected: a `POST …/rollback` route; N generations; a runtime file watcher.

**D-8 — The N-1 drop precondition is procedural, backed by the post-deploy assertion, which is itself tested.** *Anchors: P-Defer DF1.*

**D-10 — Digests read at startup; rotation, revocation and the kill switch take effect at the restart the secret change triggers.** *Anchors: V-Consistency with `Config::from_env`.* `push-token-verify` retries while the machine wakes (Bolt OP-3).

**D-11 — `#3486` closes in R-0011.**

**D-12 — Article URLs pinned to `https://mnemra.dev/`.** *Anchors: L-10.* The pin constrains one field; links in any free-text field are T11.

**D-13 — The page-file scan is a Rust test over the tracked tree, opt-out polarity, with a pinned exclusion list.** *Anchors: G-0002; P-TrustworthySignal; P-RustDefault.* Rejected: a shell script; an opt-in range parsed from the Astro config; an ad-hoc widening of the exclusions (the pin exists to make that a visible diff).

**D-15 — Health stays HTTP 200 with an absent bundle.** *Anchors: V-Reliability; P-TrustworthySignal.*

**D-17 — Rotation and revocation recipes live in `ops/token.just`, unloaded by the main justfile (RULED `5228` (11)).** Rejected: a G-0006 exception in a P-record; no `rotate` recipe; amending G-0006.

**D-18 — Own schema checker over the keyword set the generator emits, fail-closed (RULED `5221` (5)); generation in tests, nothing new in the binary (RULED `5331`, `5365`).** The keyword set is derived, not fixed (Warden F-1). Rejected: a validator crate; dropping field bounds; a fixed nine-keyword list that the real schema could not satisfy.

**D-21 — Prompt delimiters carry a per-load nonce, and values are refused if they contain any structural-token form.** *Anchors: P-GuaranteeByMechanism; T11; V-Simplicity — one draw per load is the smaller mechanism and keeps the prompt cache the existing paths already use (MEASURED).* Rejected: delimiters without a nonce guarded only by refusal (a future delimiter change would need a matching validator change — Warden F-2); refusal alone without a nonce (same reason); a nonce alone without refusal (a value could still carry a plausible-looking delimiter that a model may misread); **a per-render nonce (revision 2, Warden's F-2 option (b)) — superseded because every system block became byte-unique and defeated `cache_control: ephemeral`, raising T3's per-request spend with no security gain: the producer cannot observe a load-time draw either (Warden W3-6).**

**D-22 — One health producer on both paths; the outermost layer's health exemption keeps answering without `next`.** *Anchors: P-SecurityLayered — the exemption is a control surface, edited under security review and kept fail-closed; V-Observability — the deploy check must read a body that can carry the state; V-Consistency — one shape on every path.* Rejected: routing health through `next` so the router answers (reintroduces the limiter and page-hit tracking on the path Fly's health checker takes, which the exemption exists to skip — `origin_lock.rs:13-16`); an out-of-band note to the implementer instead of spec text (Bolt's round-3 suggestion — the public-repo rule that blocked spec text has lapsed, and a note is not a mechanism); leaving R-0019 as the proof of production (revision 2; false on the tree — R-0011); a source-text guard over the exemption branch and a `"bundle":` literal count (anchored on text that could not see the branch, and keyed on one serialization style — Warden W4-5, Glitch N2, N3; the 61-request loop observes the behaviour instead).

**D-23 — The store's filesystem is an injected collaborator (`ContentFs`); fault injection and serialization are observed through it.** *Anchors: P-TestableByDesign — substitute a double only at a true system boundary; a hidden test-only constructor signals under-injection.* Rejected: `#[cfg(test)]` fault hooks (hidden test-only path); process-kill timing (platform-fragile, unspecifiable); re-implementing the rejected rename-first order as an automated positive control (porting logic into the test suite — recorded once instead).

**D-24 — The checker's schema set is a constructor input; production passes the embedded set.** *Anchors: P-TestableByDesign; RULED `5331`/`5365` unchanged — no new binary content, no env or flag override.* Rejected: an env-var or file override for the running server (a runtime surface no operator needs, and a way to load an unreviewed schema in production); calling the loader white-box with a hand-built string (the seam then exists only in tests).


**D-25 — The failed-auth aggregator takes its clock as a collaborator, which supplies waiting as well as the time; the window is event-relative (RULED `10632`: E-8 option (b) and the window; that the collaborator waits is this spec's, because a clock that only reports time cannot emit at the close without another attempt).** *Anchors: P-TestableByDesign — a collaborator with testability value is a constructor argument, driven real in production.* Rejected, with this spec's reasons: `tokio`'s `test-util` as a dev-dependency feature (a dependency-feature ask with no need once the clock is injected); a configurable interval with a real one-second window in tests (flaky by construction, P-TrustworthySignal); a wall-clock-minute-aligned window (a real-time test can straddle a boundary).

**D-26 — R-0004's seam carries a test-only synthetic format-version-2 schema (RULED `10632`: E-7 option (a)).** *Anchors: P-TestableByDesign; P-Defer — version 2 itself stays undesigned and self-announcing.* Rejected, with this spec's reasons: deferring S8 and the synthetic line to a real version 2 (the N/N-1 path would ship once unverified); designing a minimal real version 2 now (V-Simplicity); the schema as a tracked `.json` under the test tree (R-0014's scan reads its property names and would need a fourth exclusion — Warden W4-9).

**D-27 — `PushTokenDigest`'s `compile_fail` negatives are bounded by a must-compile companion and a source-shape test; no `trybuild` (RULED `10632`: E-9 option (b)).** *Anchors: P-GuaranteeByMechanism — the compiler enforces the property either way.* Residual: a negative could fail for a wrong reason only if its own test line is malformed, which the reviewer reads. Rejected, with this spec's reason: `trybuild` with pinned `.stderr` (`Cargo.lock` carries none, MEASURED; a new dev-dependency and snapshot churn on each toolchain bump). rustdoc 1.98's `compile_fail` documents no error-code check (MEASURED), so no in-toolchain option existed.

**D-28 — The import log carries no client address.** *Anchors: V-Simplicity; V-DataMinimization.* The service has logged no per-request address since `#4236` (MEASURED 2026-11-09 at `e9fe1d8`), and this spec adds none of its own. Warden W-06 item 4's three-header request was superseded by ruling `5348` (3) on `#3558`. **Trade-off, stated:** V-Security loses a forensic signal, since a source address on a push would help tell a stolen-token push from Peter's; the audit record is digest and version (T1's residual). Rejected: an address field on this write path alone (an unwatched forensic signal, since nothing reads these logs (D-16), against the data-minimisation direction `#4236` set; `extract_ip_for_rate_limit` would supply the source, but the signal has no consumer).

**D-9 — No endpoint-specific rate limiter; the endpoint's authorization does not depend on `#3558`; the global limiter's pre-limit lookup persists for an unbounded period, accepted.** *Anchors: V-Simplicity; P-SecurityLayered's ceiling.* A 256-bit token makes online guessing infeasible; auth-before-body is the memory control. **The pre-limit `admin_sessions` lookup under the DB mutex on every Bearer request (MEASURED) persists until R-0017 Part B, whose timing is open-ended and accepted (RULED `5224` (10))** — Warden F-15; the earlier "not left open-ended" clause is withdrawn as contradicting D-14. What bounds that cost in the interim is outside this spec. Peter's push shares the global bucket and may see `429`; the client retries. Rejected: waiting on `#3558` for this endpoint's design; a per-endpoint limiter.

**D-14 — Removal precondition-gated; dead-write window and Part B's timing accepted as unbounded (RULED `5224` (10)).**

**D-16 — No degrade alert between deploys (RULED `5228` (13)).** Until job's pre-push digest check exists (`#3471`), no recurring compare exists at all.

**D-19 — Sage D4's alternative, a Fly-authenticated CLI push with no internet-exposed write route, was on the table and lost.** Peter ruled for an API push (`#3554` msgs 2, 4). This spec's statement of the cost: the CLI shape would have kept the write path off the internet at the price of a `fly ssh` step per content change and no path for job to push unattended.

**D-20 — Constant-time compare: `subtle` direct (RULED `5404`); decline branch and the `#3558` divergence.** Options considered: (a) `subtle` direct — ruled; (b) hand-rolled XOR-accumulate — **not a valid decline branch** (the "looks constant-time" class the optimizer can undo); (c) `digest::CtOutput` — **resolved, no longer unverified** (Warden F-27 MEASURED: public API via `sha2::digest`, `mac` feature active by unification) and **loses** because it implements `PartialEq`, forfeiting the compile-enforced guard, and rides another crate's feature. **Had (a) been declined, the destination would have been `digest = { version = "0.10", features = ["mac"] }` as the direct dependency inside the same no-`PartialEq` newtype** — same one-line cost, no hand-rolled arithmetic. `#3558`'s hand-rolled compare stays as documented there (Decisions already made).

## Escalations

**E-1 — RESOLVED (RULED `5404`, corrected by `5419`):** two-axis tier rule; the table in R-0002 carries it; the one-axis version is kept as superseded in "What the bundle is".
**E-2 — RESOLVED (RULED `5404`):** `subtle` direct; D-20 records the branches and the divergence.
**E-3 — RESOLVED (RULED `5224` (10)):** bundle release first, after `#3558`; open-ended overlap accepted (R-0017, D-14).
**E-4 — RESOLVED (RULED `5229`):** fork A; the Astro spec (`#3625`) carries the page half.
**E-5 — RESOLVED (RULED `5224` (12)):** job's second copy is an encrypted off-machine backup (R-0020; `#3471`).
**E-6 — RESOLVED (RULED `5228` (14)):** the frame is a standalone document, cited in the header by path and SHA.

**The rule-5 judgment call (revision 2) is RULED acceptable with three conditions** (Warden round 3, §2): real `just`/`bash`/`shasum`/`cut`/`sleep` wrappers in the shim directory, provided the harness starts from `env -i` plus an allowlist with `--no-dotenv`, resolves the wrappers from a fixed system `PATH` with an identity check, and states the recorder-bypass residual with the `fly` credential measurement recorded. All three are in R-0007 rule 5; the `fly` measurement is recorded in Measured facts.

**E-7 — RESOLVED (RULED `10632`):** option (a), a test-only synthetic format-version-2 schema through the D-24 seam, labelled synthetic, outside `contracts/`; R-0012's acceptance line and S8 stand (D-26).
**E-8 — RESOLVED (RULED `10632`):** option (b), a clock seam the aggregator takes as a collaborator; no `test-util`; the window is event-relative (R-0021, D-25).
**E-9 — RESOLVED (RULED `10632`):** option (b), the compile-must-pass companion plus source-shape test; residual recorded; no `trybuild` (R-0006, D-27).

**Resolved without escalation, recorded so they are not re-asked:** `dealbreakers` moved to unpublished because ruling `5404` names it there (Warden W3-13 — applying a ruling, not reopening one); the public-repo rule amended on Puck's decision (header); Bolt's round-3 question 1 (health) answered by Puck's decision in R-0011, not by a change to `#3558`'s exemption.
