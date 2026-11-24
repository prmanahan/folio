---
plan: live-lists-and-blog-move
status: active
author: Wyrd
task: 4853
form: full
base: 87d9ebcca0143e2177de458cd7d94904938a51cb
specs:
  - path: docs/specs/2026-11-18-live-lists-and-blog-move.md
    spec_version: 2791887aeb8266df9d1e00621a7cd89c11a41f20 blob 4e8af5ed0aa02d44aa01aafbaad9d861070c5705
---

# Plan: live projects and articles lists, and the blog move to mnemra.dev

## Purpose

This plan implements folio task #4853 in two merges. The folio server will fetch the projects list from GitHub and the article list from https://mnemra.dev/rss.xml at runtime. It holds the last good copy of each in SQLite and serves them to /projects and to both AI prompts. folio's own article pages leave, and their URLs redirect to https://mnemra.dev/blog/.

It follows the locked spec `docs/specs/2026-11-18-live-lists-and-blog-move.md`:

- lock commit `2791887aeb8266df9d1e00621a7cd89c11a41f20`
- blob `4e8af5ed0aa02d44aa01aafbaad9d861070c5705`

Form: **full**. This plan does not use the short form.

**Lock test, run 2026-11-19 in this worktree:**

- **Limb 1:** `git rev-parse -q --verify 'refs/tags/brief/live-lists-and-blog-move/spec-locked^{commit}'` printed `2791887aeb8266df9d1e00621a7cd89c11a41f20`.
- **Limb 2:** `git diff --stat 'refs/tags/brief/live-lists-and-blog-move/spec-locked' -- <path>` printed nothing. The blob is `4e8af5ed0aa02d44aa01aafbaad9d861070c5705` both at the tag and at `HEAD`. Both are from `git rev-parse`.
  - ⚠ This session's shell did not surface the exit status of `git diff --quiet`. That command printed nothing even for a file that has drifted (the old projects spec, below). Limb 2 therefore rests on the empty `--stat` and the matching blobs.
  - Positive control for `--stat`: the same command on the old projects spec printed `32 +++---`.

**Spec set.** `git ls-files -- 'docs/specs/2026-11-18-live-lists-and-blog-move.md' 'docs/specs/2026-11-18-live-lists-and-blog-move--*.md'` lists one file, so the set has one member. `git tag -l 'brief/live-lists-and-blog-move/*'` lists the `intent-locked`, `frame-locked` and `spec-locked` tags.

**Cross-file references.** The set is derived, not hand-listed. Grepping the spec for ``R-[0-9]+ (`docs/specs/2026-11-11-projects-from-github.md`)`` finds nine ids: R-3, R-4, R-7, R-8, R-9, R-12, R-13, R-14 and R-16. Each resolves: `grep -c '^- \*\*R-N\b' docs/specs/2026-11-11-projects-from-github.md` returns 1 for every one.

That file belongs to another feature, so it is not in this set's `ls-files` enumeration. Under the spec's Purpose it is an adopted external source, not a member. Its own lock state is recorded in D21 and SG3.

**Drift anchors only, not planned members:**

- Frame `docs/intent/live-lists-and-blog-move-frame.md`: tag `frame-locked`, commit `17a7e70512b069f8033473468c5d18eb5169ed3e`. `--stat` against its tag is empty.
- Intake `docs/intent/live-lists-and-blog-move.md`: tag `intent-locked`, commit `73e4ffb2045798a0c260e984fad289db45d754fa`. `--stat` against its tag is empty.

**Adopted text.** The old projects spec's adopted clauses are read from blob `32f66b7547095822efe8a2418465850265c973c3`. That is the file at the spec's lock commit `2791887` and at base `87d9ebc`. It is not the blob at that spec's tag, `2c5c937ad34a8f578baffe2f790b114a93f01e03` (D21, SG3).

## Base re-check (go/no-go)

**Verdict: go.** No task is a no-go.

The spec's Base is `ac68586`; the trunk is `87d9ebc`, which equals `origin/main` per the dispatch. `git diff --stat ac68586 87d9ebc` touches only the spec file. Each assumption below was checked against `87d9ebc`.

| Assumption | Trunk | Holds |
|---|---|---|
| Migrations stop at `007` (`migrations/007_ai_hourly_ceilings.sql`), so `008` is free | `ls migrations` | yes |
| `MAX_AGE_AHEAD` is 5 minutes in `libs/site-core/projects_list.rs` | line 33, `Duration::minutes(5)` | yes. The dispatch named `libs/site-core/src/…`; the crate has no `src/`, and the spec's paths are the right ones |
| `percent_decode_utf8` is in `libs/site-core/static_files.rs` | line 42 | yes |
| `should_track` counts `/articles` and `/articles/` (`libs/site-core/middleware/page_hits.rs`) | line 12 | yes |
| `build_base_prompt` (`libs/site-core/ai/context.rs`) reads `article::list_all` and the loaded `ProjectsList` | lines 36 and 45 | yes |
| The admin dashboard response has an `articles` field (`libs/site-core/routes/admin/dashboard.rs`) | line 11 | yes |
| R-26's four sites link `/articles` | `Nav.svelte:10`, `Hero.svelte:107`, `HeroSkeleton.svelte:40`, `(public)/+page.svelte:49` | yes |
| `{@html` outside tests is in `ChatMessage.svelte` and `MarkdownEditor.svelte`, plus `(public)/articles/[slug]/+page.svelte`, which merge 1 removes | R-24's grep | yes |
| `MarkdownEditor.svelte` survives R-27, because `frontend/src/routes/admin/ai/+page.svelte` uses it | `git grep` | yes |
| The spawned-binary harness `cmd/server/tests/common/mod.rs` exists, with `run_serve` and `fake_edge_auth_token` | Read | yes |
| `fly.toml` has `release_command = "./folio check-projects"`, a 15 s health `grace_period` and path `/api/health` | Read | yes |
| `deploy.yml` has the `Fetch projects list` step and **no `workflow_dispatch`** trigger | Read | yes; see SG1 |
| `test.yml`'s `test` job runs `scripts/check-chrono-dev-only.sh` (no argument). `deploy.yml`'s `test` job has no chrono step. `deny.toml` names the script | Read | yes |
| `actions/setup-node@53b83947a5a98c8d113130e565377fae1a50d02f` (v6.3.0) with `node-version: 22` is the repository's pin | `test.yml`, `deploy.yml` | yes |
| The vitest include has `scripts/*.spec.mjs` (`frontend/vitest.config.ts:19`) | Read | yes |
| `audit.yml` has `workflow_dispatch`, a PR path filter on manifests and lockfiles, and an unflagged `cargo-audit audit` step. `.cargo/` holds only `config.toml`, so there is no `audit.toml`. `deny.toml`'s ignore list has one entry (RUSTSEC-2026-0543) | Read | yes |
| Every assertion title and seed label R-35 quotes is in `frontend/src/lib/__tests__/deploy-workflow.test.ts` | 29 strings `grep -F`'d. 28 hit; `in the fetch step run:` is a row of the parameterised `seed: GITHUB_TOKEN outside the fetch step env, %s` (line 612) | yes |
| `Cargo.lock` holds `reqwest` 0.13.2, `url` 2.5.8, `time` 0.3.47, `rustls` 0.23.46, `ring` 0.17.14 and `aws-lc-rs` 1.18.1, and does not hold `quick-xml` or `rcgen` | `Cargo.lock` | yes; see SG4 |
| R-6's parity fixtures exist: `frontend/scripts/projects-fixtures/api-responses.json` and `list-expected.json` | `ls` | yes |
| `frontend/scripts/projects-fixtures.spec.mjs` (the adopted `^fixture-` rule) imports `fixtureRoot` from `projects-test-support.mjs`, which R-29's grep hits | Read | yes; T16 handles it |
| `e2e/projects.spec.ts` route-mocks `**/projects.json` | line 24 | yes |
| `AppState {` construction sites | 19 sites in 13 files (`git grep -c 'AppState {' -- libs cmd`) | measured; T11 sizing |

## Not planned

None. The spec set has one member, and it is planned.

## Not covered

None. Every R-N from R-1 to R-36 is cited by at least one task's Acceptance Criteria. Each requirement that spans tasks is owned by a named cross-task constraint: R-5 (D12), R-16 (D13), R-23 (D6), R-24 and R-26 (D7), R-29 (D8), R-30 (D11), R-35 (D9) and R-36 (D10).

Every scenario is covered too. The spec's `## Scenarios` holds 27 `**Given**` lines: Lists 7, Break-glass switch 7, Daily check 7 and Post-deploy check 6.
- The seven Lists scenarios are cited by their quoted Given text in T8, T9, T10 and T11.
- The switch scenarios are in T8, with their prompt clauses in T11.
- The daily-check and post-deploy scenarios are in T13.

## Unwired tasks

No build task is unwired. The core lands first and wires itself, and each source plugs in behind it (D14).

These tasks make nothing reachable because they are not product code. None of them needs a wiring task:

- **T1, T7:** test plans.
- **T4:** the runbook document.
- **T5, T17:** edge tests.
- **T6, T18:** the landings and first real runs.
- **T19, T20:** post-merge observations.

## Landing map: when each requirement goes live

Every merge to `main` deploys (`.github/workflows/deploy.yml`, no path filter). Each delivery is one PR, rebase-merged (D1). Production changes only at the two merges.

| R | Delivery, task | Why it is safe there |
|---|---|---|
| R-25, R-28, R-23 (merge-1 clause) | `blog-move`, T2 | The redirect is a route inside `build_app`, so `origin_lock` wraps it (`libs/site-core/app.rs`, #3558 layer order). The SPA article pages become unreachable before T3 removes them. |
| R-26, R-27 | `blog-move`, T3 | Commit order is after T2 (D3), so `/articles` never falls through to the SPA. The prompt keeps reading the `articles` table until T11; only route-only model functions are orphaned here. |
| R-5, R-15 to R-19, R-21, R-22, R-23 (merge-2 clause) | `live-lists`, T8 | Adds the table and the readers. No production source exists yet, so both lists read no data yet. The `projects.json` load, the refusal and the prompts stay as on trunk. |
| R-1 to R-4, R-6 | `live-lists`, T9 | The projects source plugs in, and `/api/lists/projects` serves live GitHub data. |
| R-7 to R-14 | `live-lists`, T10 | The articles source plugs in. |
| R-20, R-29 (startup refusal) | `live-lists`, T11 | The prompts move to the held copies, and the file load and refusal go. |
| R-24 | `live-lists`, T12 | `/projects` reads `/api/lists/projects`. |
| R-31 to R-34 | `live-lists`, T13 | Landings step 2. |
| R-29 (release command) | `live-lists`, T14 | Step 3. The fetch step still runs, so nothing refuses. |
| R-29 (fetch step, static file) | `live-lists`, T15 | Step 4, after the release command is gone. |
| R-29 (script deletion, superseded line) | `live-lists`, T16 | Step 5, after R-6's parity oracle passes (T9). |
| R-36 | every landing (D10) | An invariant, plus one audit run per merge. |

**Between the merges,** the chat and fit prompts still carry the `articles` table's published titles, summaries and bodies (`libs/site-core/ai/context.rs`, section 15). They hold no folio links. This is as the spec's Landings place R-20 in merge 2.

## Tasks

### T1 — Test plan: blog move

- **Files** (advisory): `docs/test-plans/2026-11-18-blog-move-test-plan.md` (new; D4).
- **Type:** test
- **Depends on:** None
- **Delivery:** `blog-move`
- **What:** Glitch derives the edge-test plan for merge 1 from the spec alone. It names each way in once:
  - the redirect: one path from R-25's set, and one non-GET method;
  - the redirect's origin-lock refusal;
  - the removed public and admin API paths;
  - the four Articles links in a browser;
  - the e2e specs that visit `/articles` today.
- **Reaches:** nothing; a test plan.
- **Acceptance Criteria:**
  - [ ] The plan exists at `docs/test-plans/2026-11-18-blog-move-test-plan.md`. It names one area per way in for R-23 (merge-1 clause), R-25, R-26, R-27 and R-28's surface, and states the run command for the Python suite and for `just e2e`. (D4)
  - [ ] It was written before T2's code and from the spec alone, and cites no file under `libs/` or `frontend/src/`. (D4)
- **Test Expectations:** none of its own.
- **Anchors:** D4

### T2 — `/articles` redirect and page-hit exclusion

- **Files** (advisory):
  - `libs/site-core/app.rs`: the route, inside the router that `origin_lock` wraps;
  - a new redirect module in `libs/site-core`, with its own percent-decode (R-25 rules out reusing `percent_decode_utf8` unchanged);
  - `libs/site-core/middleware/page_hits.rs`.
- **Type:** backend
- **Depends on:** T1
- **Delivery:** `blog-move`
- **What:** Answer R-25's path set with the fixed 301 for every method, behind the origin lock. Stop counting `/articles` and `/articles/*` as page hits.
- **Reaches:** `folio serve` answers `/articles`, `/articles/` and every path whose percent-decoded form starts `/articles/` with the 301.
- **Acceptance Criteria:**
  - [ ] `/articles`, `/articles/`, `/articles/x`, `/articles%2Fx` and `/articles?next=https://example.com/` each answer 301, with `Location` exactly `https://mnemra.dev/blog/` and `Cache-Control: public, max-age=3600`. Each also holds under a forged `Host`, and under each of `X-Forwarded-Host`, `X-Forwarded-Proto`, `X-Forwarded-For` and `X-Forwarded-Prefix`. (R-25)
  - [ ] POST, HEAD, OPTIONS and PATCH on `/articles/x` answer the same 301. (R-25)
  - [ ] `/articlesx` and `//articles` do not answer 301. `/articles/%zz` and `/articles/%FF` answer neither 301 nor any 5xx. (R-25)
  - [ ] Through `build_app`, `/articles` requested without a valid `X-Folio-Edge-Auth` gets the origin lock's 404, not the 301. (R-23, D6)
  - [ ] `should_track` returns false for `/articles` and `/articles/x`, and true for `/projects`. (R-28)
- **Test Expectations:**
  - Unit tests in the module (D5), written red first: the decoder is a parser on a security path.
  - A property: for generated paths whose strict percent-decoding starts `/articles/`, the answer is the fixed 301, and no request byte appears in the response.
  - The entry-point cases run through `build_app`, the composed router `cmd/server/main.rs` serves. T5 drives the binary.
  - Mutation target: the redirect matcher and its decoder (convention, D19).
- **Anchors:** D3, D5, D6, D19

### T3 — Articles links and article-surface removal

- **Files** (advisory):
  - `frontend/src/lib/components/Nav.svelte`, `Hero.svelte`, `HeroSkeleton.svelte`;
  - `frontend/src/routes/(public)/+page.svelte`;
  - `frontend/src/routes/(public)/articles/` (removed);
  - `frontend/src/routes/admin/articles/` (removed);
  - `frontend/src/routes/admin/+layout.svelte` and `+page.svelte`;
  - `frontend/src/lib/api.ts` and `admin-api.ts`;
  - `frontend/src/lib/components/ArticleCard.svelte` and the `Article` types (orphans);
  - `libs/site-core/routes/articles.rs` and `routes/admin/articles.rs` (removed);
  - `libs/site-core/routes/mod.rs`, `routes/admin/mod.rs` and `routes/admin/dashboard.rs`;
  - `libs/site-core/models/article.rs`: route-only functions only. `list_all` stays, because the prompt reads it until T11;
  - the frontend tests these edits break;
  - `libs/site-core/tests/test_articles.rs` (removed) and `libs/site-core/tests/admin_dashboard_tests.rs` (touched; D5).
- **Type:** full-stack
- **Depends on:** T2
- **Delivery:** `blog-move`
- **What:**
  - Point the four Articles links at https://mnemra.dev/blog/ in a new tab.
  - Remove the article pages, the public and admin article APIs, the admin article editor, its sidebar and dashboard items, and the dashboard's `articles` field.
  - Keep the `articles` table and its rows (intake NG-2).
- **Reaches:** the four Articles links and the admin UI, through `/` and `/admin`. The removed API paths reach the API-miss 404 through `folio serve`.
- **Sub-run hint:** expect about 2 agent runs: (1) the frontend links and removals; (2) the backend removals and dashboard field. Each commit builds and runs.
- **Acceptance Criteria:**
  - [ ] There is one component test per site: Nav, Hero, HeroSkeleton and the hub-error fallback. Each asserts `href` exactly `https://mnemra.dev/blog/`, `target="_blank"` and `rel="noopener noreferrer"`, with no extra accessible cue. (R-26)
  - [ ] Intake SC-9's search returns no line: ``git grep -nE "['\"\`]/(admin/)?articles" -- frontend/src ':!frontend/src/**/__tests__/**'``. (R-26)
  - [ ] `git grep -n '/articles' -- frontend/src ':!frontend/src/**/__tests__/**'` returns no line. A test under `frontend/src` that names `/articles` only asserts absence or the redirect. (R-26, D7)
  - [ ] GET, POST, PUT and DELETE on `/api/articles`, `/api/articles/x` and `/api/admin/articles` (and a path under it) each answer 404 `application/json` with body `{"error":"Not found"}`, with and without an authenticated admin session. (R-27)
  - [ ] The admin sidebar and dashboard hold no element whose `href` starts `/admin/articles`, and no Articles label or count. (R-27)
  - [ ] The admin dashboard API response has no `articles` key. (R-27)
  - [ ] The existing admin login tests pass unchanged. (R-27)
- **Test Expectations:**
  - Component tests for the links. Assert the removed-path 404s through `build_app`, with a real admin session from the existing login helper and a generated password (no literal; dev-loop `<tests>`).
  - `git diff 87d9ebc -- migrations/` stays empty.
  - Mutation: none. These are removals and attribute changes with no decision logic.
- **Anchors:** D3, D5, D7

### T4 — Runbook in the repository

- **Files** (advisory): `docs/runbooks/live-lists-and-blog-move.md` (new).
- **Type:** infra
- **Depends on:** None
- **Delivery:** `blog-move`
- **What:**
  - Copy the spec's `**Rollback, merge 1.**`, `**Rollback, merge 2.**` and `**Break-glass runbook**` blocks verbatim into the place D15 names.
  - Add one header line naming the source spec path and blob, and stating that the break-glass runbook and the merge-2 rollback apply only once merge 2 has deployed.
- **Reaches:** nothing; a document. It is in the repository before merge 1, so Bolt's merge-1 walk has a written runbook (carried DD-review F-8), and before merge 2, as the spec's First real run requires.
- **Acceptance Criteria:**
  - [ ] The three blocks equal the spec's text at blob `4e8af5e`, byte for byte, apart from heading level. (DSP-54)
  - [ ] The header line names `docs/specs/2026-11-18-live-lists-and-blog-move.md`, blob `4e8af5ed0aa02d44aa01aafbaad9d861070c5705`, and the merge-2 applicability. (DSP-54)
- **Test Expectations:** none. Mechanical copy; the reviewer diffs it against the spec.
- **Anchors:** D15

### T5 — Edge tests: blog move

- **Files** (advisory):
  - `tests/edge/` (new): a stdlib `unittest` suite run through `uv`, plus a small harness that starts the built `folio` binary with a temp `STATIC_DIR`, a temp `DATABASE_URL`, a list file (merge 1 still needs one), and a generated `ADMIN_PASSWORD` and `EDGE_AUTH_TOKEN`;
  - `e2e/public-pages.spec.ts`, `e2e/responsive.spec.ts`, `e2e/accessibility.spec.ts`.
- **Type:** test
- **Depends on:** T1, T2, T3
- **Delivery:** `blog-move`
- **What:** Glitch writes the edge tests from T1's plan. They are black-box, against the built artifact: Python for the binary over HTTP, Playwright for the browser (D4). Glitch edits the three e2e specs that visit `/articles` or assert a link to it.
- **Reaches:** nothing; tests.
- **Acceptance Criteria:**
  - [ ] The Python suite drives only the built `folio` binary over HTTP and fails, not skips, when the binary is absent. It covers each way in once:
    - one redirect path;
    - one non-GET redirect;
    - the redirect's origin-lock refusal;
    - one removed public API path;
    - one removed admin API path with an admin session;
    - one undecodable path that must not 5xx. (D4)
  - [ ] The suite's run is recorded in merge 1's report: command, binary commit and result. (D4)
  - [ ] `git grep -n '/articles' -- e2e frontend/e2e` returns only lines in tests that assert absence or the redirect. No test there visits `/articles` or asserts a link to it. (R-26, D7)
  - [ ] The edited e2e specs pass on a local `just e2e` run, recorded in merge 1's report, because no CI job runs Playwright. (R-26, D7)
- **Test Expectations:**
  - No snapshot or overwrite-generated expectations.
  - Credentials are generated per run.
  - The binary's static dir and list file are written by the harness, never read from the checkout.
- **Anchors:** D4, D7

### T6 — Merge 1: landing and first real run

- **Files** (advisory): none. This is the delivery's PR and its records.
- **Type:** infra
- **Depends on:** T2, T3, T4, T5
- **Delivery:** `blog-move`
- **What:** This is the convergence task named in Sequencing. Land merge 1 as one PR, in D3's commit order, and run the spec's merge-1 first run, attended, at level 3.
- **Reaches:** production, through the deploy that the merge triggers.
- **Acceptance Criteria:**
  - [ ] **Before the merge, Bolt checks the fast-path flag.** Bolt runs `fly deploy --help` with the `flyctl` that would run the merge-1 fast rollback, and records that `flyctl`'s version and whether `--skip-release-command` is listed. CI pins `flyctl` 0.5.10; the local one may differ.
    - If the flag is absent, the merge stops, and the lead decides at level 3 whether the merge-1 fast path in the runbook is void. (D16)
  - [ ] Before the merge, the current release's image (`fly releases --image -a folio-prmanahan`) is recorded as the rollback target. (D16)
  - [ ] An `audit.yml` run on the PR head passes, and the result is recorded. Merge 1 changes no path in the filter, so the run is dispatched by hand: `workflow_dispatch` on the PR branch. (R-36, D10)
  - [ ] `test.yml` is green on the PR head. T5's Python suite and `just e2e` runs are in the report. (D1)
  - [ ] Bolt walks the merge-1 rollback from `docs/runbooks/live-lists-and-blog-move.md`. Peter's level-3 go is recorded before the merge. (D16)
  - [ ] After the deploy, each item is recorded as seen:
    - `/articles` and `/articles/x` answer 301 to https://mnemra.dev/blog/;
    - `/api/articles` and `/api/admin/articles` answer the JSON 404;
    - the four Articles links open https://mnemra.dev/blog/ in a new tab;
    - the admin UI shows no articles page, link or count;
    - `/articles` sent to `https://folio-prmanahan.fly.dev` without the edge token gets the origin lock's 404. (D16)
- **Test Expectations:** none of its own.
- **Anchors:** D1, D3, D10, D16

### T7 — Test plan: live lists

- **Files** (advisory): `docs/test-plans/2026-11-18-live-lists-test-plan.md` (new; D4).
- **Type:** test
- **Depends on:** T6
- **Delivery:** `live-lists`
- **What:** Glitch derives merge 2's edge-test plan from the spec alone.
  - Python, against the binary:
    - `/api/lists/projects` and `/api/lists/freshness`: status, exact shape and `no-store`;
    - each route's origin-lock and rate-limit refusal;
    - the switch through the environment;
    - start with no list file;
    - `folio check-projects` gone;
    - `/projects.json` no longer special.
  - Playwright: `/projects` against a mocked `/api/lists/projects`.
- **Reaches:** nothing; a test plan.
- **Acceptance Criteria:**
  - [ ] The plan exists at `docs/test-plans/2026-11-18-live-lists-test-plan.md`. It names one area per way in, and states that the binary has no source override, so every binary-driven assertion holds whether or not the real sources answer (R-5's note). (D4)
  - [ ] It was written before T8's code and from the spec alone. (D4)
- **Test Expectations:** none of its own.
- **Anchors:** D2, D4

### T8 — Live-lists core: start function, store, switch, refresher, readers

- **Files** (advisory):
  - `migrations/008_live_lists.sql` (new);
  - `cmd/server/main.rs`: config read and one call;
  - `libs/site-core`: a start function, the store, the held copies, the switch parser, the refresher with an injected source seam and the test transport, the articles document validator, and the two `/api/lists/` routes;
  - `libs/site-core/app.rs`;
  - `libs/site-core/state.rs`;
  - `libs/site-core/Cargo.toml`: `url` direct at 2.5.8, and `time` `formatting` at runtime;
  - `cmd/server/tests/` spawned-binary cases (R-5 pins this harness; SG2).
- **Type:** backend
- **Depends on:** T7
- **Delivery:** `live-lists`
- **What:**
  - Build the seam everything else plugs into: R-5's start function and its seven steps, migration `008`, the store and held copies, R-18 and R-19's switch, the refresher with its 6-hour tick and single flight, and the two `/api/lists/` routes.
  - The refresher takes injected sources. No production source is registered yet; T9 and T10 add them.
  - Today's other startup work moves inside the start function in today's order, the `projects.json` load and refusal included, until T11 retires them.
- **Reaches:** `folio serve` answers `/api/lists/projects` (503, no data yet) and `/api/lists/freshness` (null times and `started_at`). `LIVE_LISTS_DISABLED` is parsed at start and logs its start line. Migration `008` runs at startup.
- **Live state:** at merge 2's deploy, migration `008` adds the `live_lists` table. No existing table changes. Rollback is per the spec: the old release ignores the table.
- **Sub-run hint:** expect about 3 agent runs.
  1. The start function, migration `008`, the switch, the delete and the load (R-5, R-16, R-18, R-19).
  2. The refresher and persist-then-swap (R-15, R-17), with fake sources.
  3. The routes (R-21, R-22, R-23), R-30's summary line, and the spawned-binary cases.
- **Acceptance Criteria:**
  - [ ] One start function in `site-core` runs R-5's seven steps in order. Today's other startup work runs inside it, in today's order, before step 7. It takes an injected wall clock for every timestamp and the skew comparison; its timers run on tokio time. (R-5, D12)
  - [ ] The start function receives the switch as the raw OS value (`Option<OsString>` or a raw lookup). `cmd/server/main.rs` only reads configuration and calls it; that is review-checked. (R-5, D12)
  - [ ] In-process, through the start function with the test transport, both clocks and a database file:
    - against a source that never answers, the first request is served before the first refresh completes;
    - against a pre-seeded store, the first request receives the seeded copies;
    - with the switch set and a seeded row, the row is gone before the refresher's first write. (R-5, D12)
  - [ ] Through the spawned binary with `cmd/server/tests/common/`:
    - `GET /api/health` answers 200 within 15 s;
    - `/api/lists/freshness` has exactly its three keys;
    - `/articles` answers the 301;
    - `/api/articles` answers the JSON 404.

    Only source-independent properties are asserted. (R-5, D12)
  - [ ] Migration `008` creates `live_lists` `STRICT`: `name` TEXT primary key with `CHECK (name IN ('projects', 'articles'))`, and `document` TEXT NOT NULL. The schemas of existing tables are unchanged after migration. (R-15)
  - [ ] A successful fetch is validated, persisted in one transaction, then swapped in.
    - A forced persist failure leaves the held copy and its time unchanged, and logs a failed fetch.
    - After a success, the stored document equals the held one. (R-15)
  - [ ] Stored rows are re-validated at start: projects by the existing validator, articles by the Data Model with R-11's field rules.
    - A row edited to break validation loads as no data yet.
    - A row dated exactly 5 minutes ahead loads; one 1 ms beyond is no data yet.
    - With the sources hanging, health answers 200 within 15 s through the start function. (R-16, D13)
  - [ ] After a restart with both sources failing, `/api/lists/projects` and `/api/lists/freshness` return the copies and times held before the restart, from the first request. This is Scenarios, Lists: "held copies of both lists and both sources failing"; its prompt clause is T11's. (R-16, D13)
  - [ ] The refresher, with fake sources:
    - after the start refresh, advancing the tokio test clock 6 h − 1 s causes no second fetch, and the last second causes exactly one. The server then serves the source's changed content without a restart. This is Scenarios, Lists: "the interval elapses and a description changed at the source", through `/api/lists/projects`; the page renders whatever that route serves (T12);
    - a failing projects source beside a good articles source advances only the articles time;
    - requests to `/api/health`, both `/api/lists/` routes, the `/articles` redirect and one fallback path cause no fetch;
    - after a forced panic in the persist step, health answers 200, the readers serve the held copy, and an `error` line records the refresher's end. (R-17)
  - [ ] Each switch scenario passes in-process through the start function, the non-Unicode value included. Each scenario's prompt clauses are T11's. (R-18)
  - [ ] Each disabled list's row is deleted before any refresher write, and the list is never fetched, written, loaded or served.
    - Every start logs one line naming the parsed set and, per disabled list, `deleted`, `absent` or `failed`.
    - With the delete forced to fail, the list is neither loaded nor served, and the line records `failed` at `error`. (R-19)
  - [ ] `GET /api/lists/projects`: with a held copy, 200, `Cache-Control: no-store`, and a body equal to the held document. With no data yet, 503, `no-store` and exactly `{"error":"Projects list unavailable"}`. (R-21)
  - [ ] `GET /api/lists/freshness` answers 200 and `no-store`, with keys exactly `projects`, `articles` and `started_at`, and null for a missing time.
    - The test reads two instants from the injected wall clock, one before the call and one after serving begins, then advances the clock before the first request.
    - `started_at` lies between the two instants. (R-22)
  - [ ] Each `/api/lists/` route requested without a valid `X-Folio-Edge-Auth` gets the origin lock's refusal. With the global limiter exhausted, each gets the limiter's refusal. (R-23, D6)
  - [ ] Each refresh writes one summary line, never rate-limited: `info` when every enabled list succeeded, otherwise `warn`. It carries the trigger, and per list the outcome, count, skipped posts by reason code and elapsed time. A failed persist or delete logs its own reason code. (R-30, D11)
  - [ ] The report records the lockfile delta. If enabling `time`'s `formatting` at runtime adds a package, the task stops at level 3 (Decision levels). (R-36, D10)
- **Test Expectations:**
  - Unit tests in-module, red first: the switch parser and the start sequence are a control and a contract.
  - Property tests on the switch grammar: an allowlisted-token set parses to that set; any unknown token or non-Unicode value disables both.
  - Serialize any test that sets process environment.
  - The R-15 and R-19 forced failures may use a SQLite trigger on the test database file (Constraints).
  - Carried, not filled: R-17's single flight has no acceptance case (DD-review G-5), and the spawned-binary properties cannot detect an awaited refresh (G-6).
  - Mutation targets: the switch parser, the start-time re-validation and skew check, and the refresher's advance-or-keep step (D19).
- **Anchors:** D5, D6, D10, D11, D12, D13, D14, D17, D19

### T9 — Outbound client, TLS harness and the projects source

- **Files** (advisory):
  - `libs/site-core/Cargo.toml`: `reqwest` direct at 0.13.2 with rig-core's TLS features (Frame 2a-R SLOT-1). `rcgen` as a dev-dependency only (D18);
  - the client module, the GitHub adapter and the TLS test harness under `#[cfg(test)]`;
  - `scripts/check-chrono-dev-only.sh` → `scripts/check-dev-only.sh`;
  - `.github/workflows/test.yml`, `.github/workflows/deploy.yml`: guard steps only;
  - `frontend/src/lib/__tests__/deploy-workflow.test.ts`;
  - the comments in `deny.toml` and `test.yml` that name the old script (DD-review L-4).
- **Type:** backend
- **Depends on:** T8
- **Delivery:** `live-lists`
- **Ruling:** Peter, 2026-11-24, "Accept all (Recommended)": rcgen 0.14.10 with its lock-only optional-dependency entries, tokio-rustls 0.26.4 (dev) and rustls 0.23.46 direct.
- **What:**
  - The one-constructor HTTPS client: R-1's fixed URLs, R-2's refusals, R-3's header set and no decompression, and R-4's limits.
  - It is tested through the production constructor against an in-memory `rcgen` TLS server (R-5).
  - The projects source (R-6) plugs into T8's refresher.
  - The dev-only guard is generalised, and runs for `rcgen` in the same commit that first adds `rcgen` to `Cargo.lock`.
- **Reaches:** after the start refresh, `/api/lists/projects` through `folio serve` serves the live GitHub list, and the freshness `projects` time advances.
  - From this task on, the R-5 spawned-binary test's refresher calls GitHub unauthenticated on every CI run (carried DD-review F-12).
- **Sub-run hint:** expect about 3 agent runs.
  1. The constructor, the TLS harness and the resolver map, then R-2 and R-3.
  2. R-4's timing probe and limits, R-1's pagination, the R-6 adapter and its parity oracle.
  3. `rcgen`'s lockfile measurement, the guard script, the workflow steps and their R-35 assertions, and R-30's codes.
- **Acceptance Criteria:**
  - [ ] A two-page owner yields requests for exactly the fixed URL and the fixed URL plus `&page=2`. A `Link` naming another host, path or query adds no request. (R-1)
  - [ ] Through the production-built client:
    - an `http://` URL fails with the scheme-refusal reason, and a plain listener mapped to that name records zero connections;
    - an untrusted certificate is refused with a certificate-class error, and the same server with its root trusted succeeds;
    - a wrong-hostname certificate under the trusted root is refused with a certificate-class error;
    - a 301 and a 302 each fail the fetch, and the server sees one request;
    - with `HTTPS_PROXY`, `HTTP_PROXY` and `ALL_PROXY` set to a recording listener, a fetch succeeds and the listener records none. (R-2)
  - [ ] On each of the three URLs, the set of header names received equals `Host`, `Accept`, `Accept-Encoding` and `User-Agent`, with `Accept-Encoding: identity`.
    - With `ADMIN_PASSWORD`, `ANTHROPIC_API_KEY`, `EDGE_AUTH_TOKEN` and `PAGE_HIT_SALT` set to distinct values, no header value contains any of them.
    - The gzip, br, deflate and zstd fixtures, checked in as bytes with no encoder crate added, each fail with the content-encoding reason.
    - The same JSON with `identity` and with no `Content-Encoding` is accepted. (R-3)
  - [ ] R-4's cases pass: connect, total and refresh-bound timing, the 2 MiB cap both sides, the 10-page cap both sides, 403 and 429, and one request per URL attempted.
    - Each two-sided timing case is first run repeatedly on paused tokio time against the `rcgen` server, and the run count and result are recorded.
    - A case that is not deterministic stops at level 3 with the row's three options. (R-4; Decision levels row "Test clock for R-4's timing cases"; carried DD-review F-1 and G-3)
    - [x] Ruling: Peter, 2026-11-24, "(c) Real clock (Recommended)": the R-4 two-sided timing cases run on the real clock with stated margins; not yet built.
  - [ ] Every R-2, R-3 and R-4 case runs through the production constructor against the local TLS server. Only the trust root, the name map and the clock differ.
    - A request for a name outside the test's map fails with a resolution error.
    - The one-constructor and `#[cfg(test)]` seam clauses are review-checked (signed known gap "TLS test hook by review"). (R-5, D12)
  - [ ] Scenarios, Lists: "a fetch returns one repo whose description contains U+202E". With a held projects copy, the held copy and its time are unchanged, and a `warn` line names `projects`. (R-6, D21)
  - [ ] Each R-6 condition, seeded from a held copy, leaves that copy and its time unchanged. For `frontend/scripts/projects-fixtures/api-responses.json`, the produced document equals `list-expected.json` except `fetched_at`; this is the parity oracle. (R-6, D21)
  - [ ] Each failure logs a reason-code member, never a wrapped error: scheme, certificate, redirect, encoding, resolution, the three timeouts, the two caps, non-2xx, and one per R-6 condition.
    - The summary line carries per-request status, and per GitHub response `x-ratelimit-remaining` when it parses as an integer, otherwise no field. (R-30, D11)
  - [ ] `scripts/check-dev-only.sh <package>` replaces the chrono script. With the package absent from `Cargo.lock`, it exits 1 with an instruction to remove that guard step and any ignore that depends on it.
    - `test.yml`'s chrono step runs exactly `scripts/check-dev-only.sh chrono`.
    - A step named `Check rcgen stays dev-only` runs exactly `scripts/check-dev-only.sh rcgen` in `test.yml`'s `test` job and in `deploy.yml`'s `test` job. (D18)
  - [ ] In the same commit, `deploy-workflow.test.ts` asserts each of these, each with a seeded violation that fails:
    - the chrono step's name and `run:`;
    - both `rcgen` steps' names and `run:`;
    - each dev-only guard step has exactly the keys `name` and `run`. (R-35, D9)
  - [ ] `rcgen` is a dev-dependency of `site-core` only, with default features off, a crypto provider already in `Cargo.lock`, no `pem` feature, and keys in memory only. The report records the measured `Cargo.lock` delta. Any package beyond `rcgen` and its own dependencies under those features stops the task at level 3. (D18)
  - [ ] At this task's commits, no `.cargo/audit.toml` exists, `deny.toml`'s ignore list is unchanged, and `audit.yml`'s `cargo-audit audit` step is unchanged. (R-36, D10)
- **Test Expectations:**
  - Red first: the client is a security boundary.
  - Properties: for any URL outside R-1's three plus GitHub's `&page=N` forms, no request is made. For any `Link` value, the next request is the fixed URL plus `&page=N` or none.
  - No private key or certificate file is committed (the gitleaks pre-commit hook; P-TDDPairs rule T5, no hardcoded secrets).
  - Mutation targets: the client policy (scheme, redirect, encoding, caps, pagination) and the R-6 adapter (D19).
- **Anchors:** D5, D9, D10, D11, D12, D14, D18, D19, D21

### T10 — Feed parser and the articles source

- **Files** (advisory):
  - `libs/site-core/Cargo.toml`: `quick-xml` 0.42.x, the 0.42 minor pinned, plain `Reader` (Frame D-3, 2a-R SLOT-2);
  - the feed parser module and its fixtures, among them a verbatim capture of `https://mnemra.dev/rss.xml` with its capture date.
- **Type:** backend
- **Depends on:** T8, T9
- **Delivery:** `live-lists`
- **What:** Parse the feed under R-7 to R-12's fail-closed rules, reusing T8's articles field rules and the rejected class. Plug the articles source into the refresher through T9's client.
- **Reaches:** the start refresh fetches https://mnemra.dev/rss.xml through `folio serve`, and the freshness `articles` time advances. The held articles copy reaches the prompts in T11.
- **Sub-run hint:** expect about 3 agent runs.
  1. The parser module and R-7 to R-9's fixtures. This can start in parallel with T9, because the parser needs no client.
  2. R-10 to R-12, R-14's per-alternative fixtures and the live capture.
  3. The articles source wiring, R-13's live-refresher run, and R-30's feed codes.
- **Acceptance Criteria:**
  - [ ] There is one negative fixture per R-7 bullet, each a failed fetch, with a DOCTYPE fixture at each named position: prolog, inside a field, inside discarded content, and after the 20th item. A document nested exactly 16 deep is accepted. An item whose `title` is replaced by `<atom:title>` is rejected as missing `title`. (R-7)
  - [ ] Each of these holds:
    - an `<item>` nested in an unknown channel child yields no post;
    - CDATA or a comment inside `<language>` is discarded and the posts still update;
    - an `<item>` inside a rejected `<item>` yields no post;
    - on `<rss>`, `<channel>`, `<item>` and `<guid>`, a duplicate attribute name, an unquoted value and `&bogus;` each leave the feed accepted;
    - an unknown entity inside a discarded channel child leaves the feed accepted. (R-8)
  - [ ] There is one negative fixture per R-9 bullet, plus an `<atom:link>` child, a duplicate `guid`, an empty `guid` and CDATA in `guid`, each rejecting only that post. `<guid isPermaLink="true">https://mnemra.dev/blog/hello-mnemra/</guid>` is accepted. (R-9)
  - [ ] There is one negative fixture per R-10 bullet. A title holding `&amp;` and `&gt;` is accepted. (R-10)
  - [ ] There is one negative fixture per R-11 rule. A 200-code-point title and a 600-code-point summary are accepted. (R-11)
  - [ ] Each of these holds:
    - a 22-item feed whose 3rd item is bad holds 19 posts, and neither the 21st nor the 22nd;
    - a 21-item feed with a DOCTYPE in the 21st item fails;
    - a feed whose posts are all bad keeps the held copy. (R-12)
  - [ ] Every negative fixture of R-6 to R-12 is run through a live refresher, and the refresher then completes a valid refresh. (R-13)
  - [ ] Every SHALL in R-7 to R-11, and each listed alternative, has a fixture. The committed capture of `https://mnemra.dev/rss.xml`, dated, parses to its expected posts. (R-14)
  - [ ] Feed failures and post rejections log one reason-code member per rule. Each skipped post logs a `warn` line holding only its item index and a reason code. (R-30, D11)
  - [ ] The scenarios "three posts and one has a title of 201 code points" and "every post in the feed is bad" pass (Scenarios, Lists). (R-12)
  - [ ] The report records the `Cargo.lock` delta for `quick-xml`. Any package beyond `quick-xml` and its own dependencies stops the task at level 3 (Constraints). (R-36, D10)
- **Test Expectations:**
  - Red first: a parser on a trust boundary.
  - Properties: no input panics the parser. Any text placed in a discarded child, an attribute value, or a rejected post's description never appears in the output document.
  - Carried, not filled: R-30 lists no role for R-12's "no valid post among the first 20" (DD-review F-11). The member's name is level 5 (Decision levels, "Log reason-code names"). R-14's capture checks form, not content (G-9).
  - Mutation target: the parser's event dispatch and its R-10 and R-11 checks (D19).
- **Anchors:** D5, D10, D11, D14, D19

### T11 — Prompts from the held copies; retire the startup refusal

- **Files** (advisory):
  - `libs/site-core/ai/context.rs`, `libs/site-core/ai/prompt_templates.rs`;
  - `libs/site-core/state.rs`: the `ProjectsList` plumbing goes;
  - the start function's file load;
  - `libs/site-core/models/article.rs`: `list_all` is orphaned here;
  - the tests these touch (D5): 19 `AppState {` sites in 13 files.
- **Type:** backend
- **Depends on:** T8, T9, T10
- **Delivery:** `live-lists`
- **What:**
  - Build both prompts from one snapshot of the held copies, with the adopted projects preamble and R-20's articles preamble.
  - Stop reading the `articles` table and the list file.
  - Retire the startup refusal on a missing or invalid list.
  - Land R-30's sentinel suite now that every source exists.
- **Reaches:** `/api/chat` and `/api/fit` carry the held lists through `folio serve`. The binary starts with no `projects.json`.
- **Sub-run hint:** expect about 2 agent runs: (1) `AppState` plumbing and the R-20 builder; (2) retire the refusal, then the sentinel suite.
- **Acceptance Criteria:**
  - [ ] For one snapshot, the chat prompt, the fit prompt and `/api/lists/projects` carry the same repos and descriptions. (R-20, D21)
  - [ ] With a seeded `articles` table row, neither prompt contains its title or `content`. (R-20)
  - [ ] The articles preamble is exactly R-20's sentence and appears once, directly after its heading. Each article entry is one line holding title, summary, link and publication date, in held order. A list with no data yet has no heading and no preamble. (R-20)
  - [ ] After a restart with both sources failing, both prompts carry the copies held before the restart, from the first request. (R-16, D13)
  - [ ] Driven with no list file and an empty store, the start function serves, and both lists read as no data yet. (R-29, D8)
  - [ ] None of R-30's seven sentinels appears anywhere in the log output, and the summary line carries each listed field. (R-30, D11)
  - [ ] Each of these passes, each through the start function:
    - Scenarios, Lists: "an empty store and both sources valid": the first request sees no data yet for both lists, and within the refresh bound both prompts and the freshness read show both lists, with times later than `started_at`. The `/projects` clause is R-24's 200 case (T12);
    - Scenarios, Lists: "no held copy and every fetch failing": the chat answers, and neither prompt has a projects or articles section;
    - Scenarios, Break-glass switch: the prompt clauses of the `articles` and `projects` cases. (R-20)
- **Test Expectations:**
  - Red first on the section builder: a prompt-injection holding layer (KG-6).
  - Mutation target: the R-20 section builder (D19).
  - The adopted preamble and one-line rule are read from blob `32f66b7` (D21).
- **Anchors:** D5, D8, D11, D13, D14, D19, D21

### T12 — `/projects` from `/api/lists/projects`

- **Files** (advisory):
  - `frontend/src/routes/(public)/projects/+page.svelte`;
  - `frontend/src/lib/__tests__/ProjectsPage.test.ts` and `ProjectsPageShapes.test.ts`.
- **Type:** frontend
- **Depends on:** T8
- **Delivery:** `live-lists`
- **What:** Load `GET /api/lists/projects`, with the 15 s abort. Render by the adopted rendering and R-14 (`docs/specs/2026-11-11-projects-from-github.md`). On any other outcome, show R-24's unavailable alert with the two GitHub links.
- **Reaches:** `/projects` in the browser, through `folio serve`.
- **Acceptance Criteria:**
  - [ ] Component tests cover each outcome. A never-resolving fetch shows neither the unavailable line nor `No projects yet.` before 15 s, and shows the unavailable line after 15 s. (R-24)
  - [ ] A valid 200 that would arrive after 15 s leaves the unavailable line showing, with no entries, and the request was aborted at 15 s. (R-24)
  - [ ] The line has `role="alert"`. The two links have `href`s exactly https://github.com/mnemra and https://github.com/prmanahan, distinct text naming the owner, `target="_blank"` and `rel="noopener noreferrer"`. (R-24)
  - [ ] A description `<img src=x onerror=alert(1)> **b**` served by the route creates no `img` or `strong` element. (R-24, D21)
  - [ ] `git grep -n '{@html' -- frontend/src ':!frontend/src/**/__tests__/**'` returns lines only in `frontend/src/lib/components/ai/ChatMessage.svelte` and `frontend/src/lib/components/admin/MarkdownEditor.svelte`. (R-24)
- **Test Expectations:**
  - Fake timers for the 15 s abort. Assert the exact line text.
  - Mutation target: the page's load-and-shape check (D19).
- **Anchors:** D7, D14, D19, D21

### T13 — Freshness checks and their workflows

- **Files** (advisory):
  - `frontend/scripts/check-freshness-daily.mjs`, `check-freshness-post-deploy.mjs`, and their `.spec.mjs` tests;
  - a new daily workflow file under `.github/workflows/`;
  - `.github/workflows/deploy.yml`: one new job, `needs: deploy`;
  - `frontend/src/lib/__tests__/deploy-workflow.test.ts`;
  - a skew-parity test;
  - `CLAUDE.md`.
- **Type:** infra
- **Depends on:** T11, T12
- **Delivery:** `live-lists`
- **What:**
  - This is Landings step 2: the daily and post-deploy checks, their jobs and R-35's check-job assertions.
  - The skew-parity test (R-34).
  - The P-RustDefault departure line for the two Node checks, recorded in folio's `CLAUDE.md` (D20).
- **Reaches:** after the merge, the daily check runs on its schedule and by `workflow_dispatch`, and the post-deploy job runs on every deploy.
- **Sub-run hint:** expect about 2 agent runs: (1) both scripts and their tests; (2) the workflows, R-35's assertions, R-34 and `CLAUDE.md`.
- **Acceptance Criteria:**
  - [ ] Each script reads only `https://peter.manahan.io/api/lists/freshness` with a 30 s timeout. Its tests assert:
    - the target is that exact `https://` URL;
    - the `fetch` init object has exactly the keys `redirect` (set to `'manual'`) and `signal`;
    - a 301 reads red;
    - a rejected `fetch` exits non-zero.

    On red, the script prints the body or the error and exits non-zero. Each check job installs Node with `actions/setup-node@53b83947a5a98c8d113130e565377fae1a50d02f` and `node-version: 22`, and sets a 10-minute limit. Carried, not filled: setup-node has no R-35 assertion (DD-review F-6). (R-31)
  - [ ] The daily check is its own workflow, on a daily schedule and `workflow_dispatch`, with `permissions` exactly `contents: read`. It reads, waits 150 s, and reads again, and judges only the second read against [now − 48 h, now + skew]. Each daily-check scenario passes. (R-32)
  - [ ] The post-deploy check is a job in `deploy.yml` with `needs: deploy` and no `permissions`. It reads every 10 s from the first read's start, with no read starting at or after 150 s. Each read is judged against its own `started_at`, and the check is green on the first read that settles green. Each post-deploy scenario passes. (R-33)
  - [ ] The skew allowance is 5 minutes in both checks. A test fails when the Node value and `MAX_AGE_AHEAD` in `libs/site-core/projects_list.rs` differ, shown by a seeded difference. (R-34)
  - [ ] `deploy-workflow.test.ts` asserts each of R-35's check-job bullets, each with a seeded violation that fails:
    - the daily workflow's `schedule` and `workflow_dispatch`;
    - the daily job's `permissions`;
    - no `environment`, secret or `env` on either check job;
    - no workflow-level `env` in either file;
    - each `run:` is exactly `node <script>`;
    - `timeout-minutes` is set;
    - the job and step key allowlists, with `continue-on-error`, `if`, `shell` and `working-directory` seeded at job and step level;
    - the literal `node:timers/promises`-only import allowlist;
    - the forbidden-token scan, with one seed per token.

    The action-pin and checkout site counts are raised to include the new file and job. (R-35, D9)
  - [ ] `CLAUDE.md` records the granted P-RustDefault departure for the two check scripts: that they are Node, why ("Node script + tests", Frame OPC-9), and a citation of P-RustDefault. (DSP-56, D20)
- **Test Expectations:**
  - Red first: both scripts are checks with in-scope consequence.
  - Fake timers for the 150 s window and the 10 s cadence.
  - The scripts read no environment, so their tests double `fetch` only.
  - Mutation targets: both scripts' predicates (D19).
- **Anchors:** D3, D9, D19, D20

### T14 — Retire the release command and `check-projects`

- **Files** (advisory):
  - `fly.toml`: the `[deploy]` table;
  - `cmd/server/main.rs`: the subcommand, and its `fly.toml` test module;
  - `cmd/server/Cargo.toml`: `toml_edit` if orphaned;
  - `cmd/server/tests/check_projects.rs` (removed);
  - the age-behind check in `libs/site-core/projects_list.rs` and its tests, if orphaned. `MAX_AGE_AHEAD` stays (R-16, R-34).
- **Type:** backend
- **Depends on:** T13
- **Delivery:** `live-lists`
- **What:** Landings step 3. Remove `fly.toml`'s release command and the `check-projects` subcommand, and their orphans.
- **Reaches:** the Fly release path runs no release command.
- **Live state:** at merge 2's deploy, Fly runs no release command. The spec's merge-2 rollback depends on deploying from a post-merge-2 checkout because of this.
- **Acceptance Criteria:**
  - [ ] `fly.toml` has no `release_command`. (R-29, D8)
  - [ ] `cmd/server/main.rs` has no `check-projects` subcommand. `./folio` with no subcommand still serves. (R-29, D8)
- **Test Expectations:**
  - Mechanical; tests may follow the code.
  - The report records any `Cargo.lock` shrink.
- **Anchors:** D3, D5, D8

### T15 — Retire the deploy fetch step and the static `/projects.json`

- **Files** (advisory):
  - `.github/workflows/deploy.yml`;
  - `frontend/src/lib/__tests__/deploy-workflow.test.ts`;
  - `libs/site-core/static_files.rs`: `NO_CACHE_FILE` and its rule;
  - `libs/site-core/tests/test_projects_json_cache.rs` (removed).
- **Type:** infra
- **Depends on:** T14
- **Delivery:** `live-lists`
- **What:** Landings step 4. Remove the `Fetch projects list` step and the `/projects.json` no-cache rule. In the same commit, apply R-35's fetch-step changes to the workflow test.
- **Reaches:** the deploy job runs without a GitHub token. `/projects.json` gets no special handling.
- **Acceptance Criteria:**
  - [ ] `deploy.yml` has no fetch step, and `static_files.rs` gives `/projects.json` no special handling. (R-29, D8)
  - [ ] In the same commit, `deploy-workflow.test.ts` makes R-35's fetch-step changes, every kept seed still failing on its seeded violation:
    - the secret rule becomes "outside the Deploy step, no secret is referenced", with the six `seed: %s` secret seeds, the install-step and test-job `GITHUB_TOKEN` seeds, and the `deploy-env-not-pinned` and `deploy-step-unexpected-key` allowlist seeds kept and re-pointed off the fetch step;
    - `github-context-token-reference` stays its own rule with its three seeds;
    - Deploy's `env` is exactly `FLY_API_TOKEN`;
    - the dropped assertions and seeds R-35 lists are gone. (R-35, D9)
- **Test Expectations:**
  - Carried, not filled: the `toJSON(github)` seed is not re-pointed, and the one `run:`-position secret seed is dropped (DD-review F-10, L-2 closure).
- **Anchors:** D3, D8, D9

### T16 — Delete the Node fetch script; add the superseded-by line

- **Files** (advisory):
  - remove `frontend/scripts/fetch-projects.mjs`, `fetch-projects.spec.mjs`, `projects-fetch-double.mjs` and `invisible-table.mjs`;
  - trim `projects-test-support.mjs`, or move `fixtureRoot` into `projects-fixtures.spec.mjs`, so the adopted `^fixture-` test survives;
  - `scripts/gen-invisible.mjs`: drop that output;
  - `justfile`, `README.md`, `CLAUDE.md`, `.gitignore`, `libs/site-core/config.rs`, `libs/site-core/projects_list.rs`;
  - `docs/specs/2026-11-11-projects-from-github.md`: one line.
- **Type:** infra
- **Depends on:** T15, T9
- **Delivery:** `live-lists`
- **What:** Landings step 5. Delete the fetch script and what only it and its tests used, after R-6's parity oracle passes. Clear every remaining hit of R-29's grep. Add the superseded-by line to the old projects spec.
- **Reaches:** nothing new. It removes a now-unused local path (`just fetch-projects`).
- **Acceptance Criteria:**
  - [ ] T9's R-6 parity oracle passes at this task's base, before any deletion. (R-29, D8)
  - [ ] ``git grep -nE 'projects\.json|fetch-projects|check-projects' -- ':!docs' ':!**/__tests__/**' ':!tests/**' ':!**/tests/**' ':!e2e/**' ':!frontend/e2e/**'`` returns no line outside an inline `#[cfg(test)]` module. That exemption is review-checked (carried R4-G4, G-8). (R-29, D8)
  - [ ] `frontend/scripts/projects-fixtures.spec.mjs` still runs under `npm run test:unit`, finds at least one fixture of each kind, and passes. `node scripts/gen-invisible.mjs --check` exits 0 on the remaining outputs. (R-29, D8, D21)
  - [ ] `docs/specs/2026-11-11-projects-from-github.md` carries exactly one added line: "Superseded by `docs/specs/2026-11-18-live-lists-and-blog-move.md` except for the clauses that spec adopts". No other edit is made to that file. (R-29, D8)
- **Test Expectations:** mechanical; no new tests beyond keeping the adopted fixture test green.
- **Anchors:** D3, D8, D21

### T17 — Edge tests: live lists

- **Files** (advisory): `tests/edge/` (extended); `e2e/projects.spec.ts`.
- **Type:** test
- **Depends on:** T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
- **Delivery:** `live-lists`
- **What:** Glitch writes the edge tests from T7's plan against the built binary and the built page.
- **Reaches:** nothing; tests.
- **Acceptance Criteria:**
  - [ ] The Python suite drives only the built binary, fails when it is absent, and asserts only source-independent properties. It covers each way in once:
    - the binary starts with no list file and health answers 200 within 15 s;
    - `/api/lists/projects` answers either 200 with a document or 503 with the exact body, `no-store` either way;
    - `/api/lists/freshness` has exactly its three keys and `no-store`;
    - each route without the edge token gets the origin-lock 404;
    - `LIVE_LISTS_DISABLED=projects` gives 503 on `/api/lists/projects` and a null `projects` time;
    - an unknown token logs an `error` line and disables both;
    - `folio check-projects` exits non-zero;
    - `/projects.json` carries no `no-cache`. (D4)
  - [ ] The suite's run is recorded in merge 2's report. (D4)
  - [ ] `e2e/projects.spec.ts` mocks `/api/lists/projects` instead of `projects.json`, and passes on a local `just e2e` run recorded in merge 2's report. (R-24, D7)
- **Test Expectations:**
  - Credentials and the edge token are generated per run.
  - Carried: the local `just e2e` run is unscoped and cannot fail mechanically (DD-review G-4).
- **Anchors:** D4, D7

### T18 — Merge 2: landing and first real run

- **Files** (advisory): none. This is the delivery's PR and its records.
- **Type:** infra
- **Depends on:** T17
- **Delivery:** `live-lists`
- **What:** This is the convergence task named in Sequencing. Land merge 2 as one PR in D3's commit order, then run the spec's merge-2 first run, attended, at level 3.
- **Reaches:** production.
- **Acceptance Criteria:**
  - [ ] **Before the merge:**
    - `docs/runbooks/live-lists-and-blog-move.md` is on `main` (T4);
    - merge 1's image (`fly releases --image -a folio-prmanahan`) is recorded as the rollback target;
    - the newest volume snapshot is recorded, labelled unverified because no restore has been demonstrated;
    - whether the projects fetch can succeed from Fly's egress is recorded. If that cannot be measured, the triage rule is recorded instead: a red post-deploy check whose summary line shows 403 or 429 is read as the parked GitHub budget (Frame FQ-2). (D16)
  - [ ] An `audit.yml` run on the PR head passes, by its pull-request trigger since `Cargo.lock` changes, and the result is recorded. `test.yml` is green. T17's Python suite and `just e2e` runs are in the report. (R-36, D10)
  - [ ] Bolt walks the break-glass runbook and the merge-2 rollback. Peter's level-3 go is recorded before the merge. (D16)
  - [ ] After the deploy, each item is recorded as seen:
    - the post-deploy check goes green;
    - `/api/lists/freshness` shows both times later than `started_at`;
    - `/projects` shows the list;
    - `fly logs` shows one refresh summary line carrying R-30's fields;
    - `fly secrets list` holds no GitHub token (intake SC-13). (D16)
  - [ ] A log-retention marker line is written at the run, for T19. (D16)
- **Test Expectations:** none of its own.
- **Anchors:** D1, D2, D3, D10, D16

### T19 — Parked readings, 24 to 48 hours after merge 2

- **Files** (advisory): none. Merge 2's report.
- **Type:** infra
- **Depends on:** T18
- **Delivery:** `live-lists` (post-merge; lands nothing)
- **What:** Record the spec's three parked readings. They are **parked, not deferred**: nothing fires them mechanically (P-Defer DF1). Cadence: merge 2's report stays open until all three are recorded, and Peter checks it at the next routine update after merge + 48 h.
- **Reaches:** nothing.
- **Acceptance Criteria:**
  - [ ] **Intake SC-1.** After a GitHub description change with no deploy, a freshness read made 24 h or more later wakes the machine. A second read, once the refresh bound has passed, shows a `projects` time later than the change, and `/projects` shows it. (D16)
  - [ ] **Intake SC-14.** Two of the daily check's judged second reads, made more than 24 h apart, show both lists' times advanced between them, with no human action. (D16)
  - [ ] **Log retention.** T18's marker line is read back with `fly logs` 48 h later, and the result is recorded. PK-2 stays unverified until then. (D16)
- **Test Expectations:** none.
- **Anchors:** D16

### T20 — Alert-path drill (Warden F-10)

- **Files** (advisory): none. Merge 2's report.
- **Type:** infra
- **Depends on:** T19
- **Delivery:** `live-lists` (post-merge; lands nothing)
- **What:** Drill the break-glass runbook on `articles`, so that one red run of each check is seen reaching Peter by GitHub notification. The drill is placed after T19 so that its human action and its red reads fall outside SC-1's and SC-14's windows (D16).
- **Reaches:** nothing.
- **Acceptance Criteria:**
  - [ ] Runbook steps 1 and 2 on `articles`: the start line names `articles` with `deleted` or `absent`, and freshness reports `articles` as null. (D16)
  - [ ] The switch stays set across the first **scheduled** daily run, and that cron-fired red is seen reaching Peter by notification. Until a scheduled red is seen, the report labels the scheduled alert path unverified. (D16)
  - [ ] held: SG1. A `deploy.yml` run while the switch is set has no named trigger. Once the lead rules, the post-deploy job's red is seen reaching Peter by notification. (D16)
  - [ ] Runbook step 5: after the unset, the `articles` time in the freshness read is later than the unset. (D16)
- **Test Expectations:** none.
- **Anchors:** D16

## Decisions

| D | Decision | Rationale | Canon anchor |
|---|---|---|---|
| D1 | **One plan, two deliveries.** `blog-move` (merge 1: T1 to T6) and `live-lists` (merge 2: T7 to T20). Each delivery is one PR from a stack of task branches, rebase-merged; every commit builds and runs. | The two merges go live separately, but Peter ruled them one spec with two landings ("Two, blog move first (Rec.)"). So they are one plan with two `Delivery` values, not a split spec. | Spec `## Requirements`, Landings. *P-IncrementalSlicing:* "every commit on main also runs, each commit of a multi-commit landing included." |
| D2 | **Merge 2 waits on merge 1's first run.** T7, merge 2's first task, depends on T6, so every merge-2 task depends on it transitively. Merge 2's branch is cut from `main` after T6. | The spec cuts merge 2 "from `main` after merge 1 has deployed and merge 1's first run has been observed". | Spec Landings. |
| D3 | **Commit order.** Merge 1: T2's commits before T3's, so R-25 lands no later than R-27. T4 and T5 may sit anywhere before T6. Merge 2 follows the five Landings steps: step 1 is T8 to T12 (T8 first; T9 before T10; T10 before T11); step 2 T13; step 3 T14; step 4 T15; step 5 T16. R-36's and R-35's commit rules are D10 and D9. | Rebase-merge puts each commit on `main`. The spec fixes the order: `/articles` must never be served the SPA page, a release command must not outlive the fetch step it reads, and the Node script goes only after the Rust parity oracle passes. | Spec Landings. *P-IncrementalSlicing:* as D1. |
| D4 | **Test plans and the edge suite.** Test plans: `docs/test-plans/2026-11-18-blog-move-test-plan.md` and `docs/test-plans/2026-11-18-live-lists-test-plan.md`. Edge tests that drive the binary over HTTP are Python stdlib `unittest`, run through `uv`, under `tests/edge/`. Browser edge tests are Playwright in `e2e/`. Each edge-test task depends on its test plan and every build task it drives. The edge suites run locally against the built artifact and each run is recorded in the delivery's report. This plan adds no CI job for them (SG5). | This is folio's first Python. The path sits under R-29's `tests/**` exclusion, so an absence test may name a removed path. A CI job would change the workflow shapes R-35 pins. | *P-TDDPairs:* "Edge tests that run a binary or call a server over its protocol are Python, run through uv, in every repository; edge tests that drive a page in a browser are TypeScript with Playwright." |
| D5 | **Test layout: migrate on touch.** New unit tests go in the module they test. A `tests/*.rs` file that a task touches moves in that task, into its module or into the Python edge suite. Exceptions:<br>• `cmd/server/tests/`, which R-5 pins (SG2);<br>• a shared helper (`libs/site-core/tests/common/`) whose move would force moving files the task does not otherwise touch. That one stays, and the report says so.<br>A file deleted with its feature is just deleted. Measured exposure: 19 test files match the touch-pattern grep in the report. | The canon statement rules both halves. | *P-TDDPairs:* "An existing one moves, into the module or into the Python edge suite, when a change already touches it, never in a sweep." |
| D6 | **R-23 spans both merges.** The `/articles` clause is T2's; the two `/api/lists/` route clauses are T8's. | R-23 says which clause lands in which merge. | Spec R-23. |
| D7 | **R-26's and R-24's acceptance is split by author.** Component tests and the `frontend/src` greps are the implementer's (T3, T12). The e2e spec edits, the `e2e` grep and the local `just e2e` record are the edge author's (T5, T17). | Playwright specs are browser edge tests, written by the separate edge author. | *P-TDDPairs:* edge layer language, quoted in D4. |
| D8 | **R-29 is split across Landings steps.** The startup-refusal bullet is T11 (step 1). The release command and `check-projects` are T14 (step 3). The fetch step and the static `/projects.json` are T15 (step 4). The script deletion, R-29's grep and the superseded-by line are T16 (step 5). | The spec places each bullet in a step. | Spec Landings and R-29. |
| D9 | **R-35 is split by bullet.** The guard-step bullets are T9, the commit that first adds `rcgen` to `Cargo.lock`. The check-job bullets and raised site counts are T13. The fetch-step re-points and drops are T15. | "Each bullet of R-35 lands in the same commit as the workflow, script or lockfile change it asserts." | Spec R-35. |
| D10 | **R-36 holds at every commit.** There is no `.cargo/audit.toml`, `deny.toml`'s ignore list is unchanged, and `audit.yml`'s step is unchanged. Each task that changes `Cargo.lock` (T8 possibly, T9, T10) records the delta. One `audit.yml` run per merge is recorded: T6 dispatched by hand; T18 by the pull-request trigger. | R-36 lands "with the commit that first adds a dependency to `Cargo.lock`", and its acceptance is per merge. | Spec R-36. *P-SecurityLayered:* "supply chain is its own layer" (spec's anchor). |
| D11 | **R-30 is split by failure role.** Each task adds the reason-code members for the failures it introduces: T8 persist, delete and the summary line; T9 transport and projects; T10 feed and post. The sentinel suite runs in T11, once every source exists. | The enum's roles belong to different modules, and the sentinels need every source. | Spec R-30. |
| D12 | **R-5 is split.** T8 owns the start function, its order, the raw switch value, `main.rs`, the in-process cases and the spawned-binary cases. T9 owns the TLS-harness clauses: production constructor, local TLS server, and the resolver refusing names outside the map. | The start function is the seam; the harness belongs with the client. | Spec R-5. *P-MinBlastRadius:* "the first commit introduces the seam, later commits change behaviour behind it." |
| D13 | **R-16 is split.** T8 owns the load, re-validation and route restart case. T11 owns the prompt clause of the restart case, because prompts read the held copies only from R-20. | R-16's first acceptance case names both the routes and the prompts, and the prompts move to the held copies in a later task. | Spec R-16, R-20. |
| D14 | **Wire first; no layer.** T8 lands the start function, the store, the routes and the refresher with an injected source seam. T9, T10 and T11 each plug one capability in behind it, and each is reachable through `folio serve` when it lands. No recorded layer is needed. | A pure-library-first order would stack three unwired tasks. | *P-IncrementalSlicing:* "A feature is wired through the shipped entry point from its first task." *P-MinBlastRadius:* as D12. |
| D15 | **The runbook's place.** `docs/runbooks/live-lists-and-blog-move.md` holds the spec's break-glass runbook and both rollbacks verbatim, landed in merge 1 (T4). A header line names the source blob and says the break-glass part and the merge-2 rollback apply from merge 2. **Rejected:**<br>• the spec section itself, because a short-lived feature spec is not where an operator looks;<br>• landing it inside merge 2's PR, which is not "in the repository before the merge". | The spec says the runbook must be "in the repository, in the place the plan names", before merge 2. Merge 1 then also gives Bolt's merge-1 walk a written runbook (carried DD-review F-8). | Spec `## First real run`. *V-DualAudience:* "Durable artifacts are written to be read cold." |
| D16 | **First real runs and the drill.** Merge 1's first run is T6, with Bolt's `--skip-release-command` check first. Merge 2's first run is T18. The parked readings are T19. The drill is T20, after T19, so its human action and red reads fall outside SC-1's and SC-14's "no human action" windows. **Rejected:** the drill before the readings, which would contaminate SC-14's pair and push it past 48 h either way. | Each merge's first run happens at the merge, attended, at level 3 (Peter: "First run at each merge"). The spec leaves the drill's place to the plan. | Spec `## First real run` and Decision levels row 1. *P-Defer DF1:* "human-noticed via a named cadence (valid only when relabelled parked, with the cadence stated)." |
| D17 | **No toggle.** `LIVE_LISTS_DISABLED` is a product setting, a spec-named break-glass control that Peter sets with `fly secrets`, so it has no removal event. T8's switch scenarios prove that off is off, including an unknown value and a non-Unicode one. | The spec forbids a toggle or dark launch (NG-10) and names the switch "a break-glass control, not a feature toggle". | *P-IncrementalSlicing:* "It is a product setting if the spec names who sets it after delivery." |
| D18 | **`rcgen` and the guard.** `rcgen` is a dev-dependency of `site-core` only, with default features off, a crypto provider already in `Cargo.lock`, no `pem`, and keys in memory only. Its acceptance is the measured lockfile delta, not a named provider (SG4). `scripts/check-dev-only.sh <package>` replaces the chrono script in T9, with the `rcgen` steps in both test jobs. | Peter's answer and Warden's conditions, in the spec's TLS-harness row. | Spec Decision levels, "TLS test harness for R-2 to R-5". |
| D19 | **Mutation gate: not in force.** The targets named per task are convention. No mutation tool is wired: `grep -i 'mutant\|stryker'` over `justfile`, `frontend/package.json`, `.github/workflows/*.yml` and `Cargo.toml` printed nothing, while the same files' `coverage` hit `justfile:68`. | No timed run has priced a gate, and none is wired, so a target named here enforces nothing. | *P-TDDPairs:* "until then, never report a mutation gate as in force." |
| D20 | **The P-RustDefault departure is recorded in `CLAUDE.md`**, in T13, beside the code that deviates and in the same delivery. | The spec routes this to the plan (DSP-56). | *P-RustDefault:* "Once a departure is granted, it is recorded where the deviation lives — in that project's own instructions." |
| D21 | **Adopted clauses are read from blob `32f66b7`.** That is `docs/specs/2026-11-11-projects-from-github.md` at the spec's lock commit `2791887` and at base. It is not the blob at that spec's tag (`2c5c937`). The adopted R-4 lone-surrogate rule, R-8 no-twin rule and R-12 preamble sentence exist only in `32f66b7`, so it is the only reading in which every clause the spec adopts exists (SG3). | An implementer reading the tag's blob would miss three adopted rules, and the no-twin and preamble criteria in T9 and T11 would have no source. | *P-PreserveDecisionSpace:* "A provenance fact is copied verbatim from its source." Spec `## Purpose`, the adopted-clause list. |

## Sequencing

- **Merge 1 (`blog-move`):**
  - T1, then T2, then T3. T4 can run in parallel at any point.
  - T5 after T1, T2 and T3.
  - **T6 is the convergence task**: the landing and the first run.
  - Critical path: T1, T2, T3, T5, T6.
- **Merge 2 (`live-lists`):**
  - T7, then T8.
  - After T8: T9 and T12 in parallel. T10's parser sub-run may start in parallel with T9, but T10 lands after T9.
  - Then T11 after T9 and T10.
  - Then the strict chain: T13, T14, T15, T16 (Landings steps 2 to 5).
  - Then T17, then **T18, the convergence task**: the landing and the first run.
  - Post-merge: T19, then T20.
  - **Critical path:** T7, T8, T9, T10, T11, T13, T14, T15, T16, T17, T18. T8 and T9 are the largest.
- **Sizes, anchored on #4726's landed plan** (`docs/plans/2026-11-13-projects-from-github-plan.md`). Its deliveries were:
  - `f6ad209`: validator, subcommand and harness, 13 files, +2207;
  - `b0ba08b`: fetch script, 14 files, +1838;
  - `b9da148`: go-live convergence, estimated at about 4 runs, 56 files, +2498/−1017;
  - `74142f6`: one header, 3 files, +323.

  Estimates here:
  - T8 and T9: each at or above `f6ad209`, about 3 runs;
  - T10: about `b0ba08b`, about 3 runs;
  - T3, T11, T13: about 2 runs each;
  - T2, T5, T12, T15, T16, T17: one run each;
  - T4, T14: small.

  #4726 then took 13 follow-up commits over 5 days after go-live (`git log --grep='#4726'`), so expect review rounds beyond these.
- **Review gates**, each at its task branch before it is stacked on the delivery branch. The lenses come from folio's `CLAUDE.md` `<skills>`:
  - T2, T8, T9, T10, T11, T14: `review-core` and `rust-review`;
  - T3, T12: add `svelte-review` and `typescript-review`;
  - `security` and `control-integrity` on T2 (path decoder), T8 (switch, re-validation), T9 (client policy, guard script), T10 (parser) and T13 (checks, workflow permissions);
  - `bolt-review` on T8 (migration, start), T14 and T15 (deploy configuration);
  - each test-plan and edge-test task gets an assertion review against the spec;
  - before each merge: Bolt's runbook walk and Peter's level-3 go (T6, T18).

## Test surface

- **Unit (in-module, D5):** the redirect decoder, switch parser, start sequence, store, refresher, client policy, GitHub adapter, feed parser, prompt builder, page component and check scripts.
- **In-process start function (R-5):** the test transport, the injected wall clock, the tokio test clock and a database file. This covers R-13, R-15 to R-22 and R-30.
- **TLS harness (T9):** an in-memory `rcgen` CA and server under `#[cfg(test)]` in `site-core`, with a name-to-address map that refuses unmapped names. No key file is committed.
- **Spawned binary (R-5):** `cmd/server/tests/common/`, pinned by the spec (SG2).
- **Fixtures:**
  - feed fixtures with the parser, including a dated verbatim capture of https://mnemra.dev/rss.xml;
  - codec bodies as bytes;
  - `frontend/scripts/projects-fixtures/` for the parity oracle and the adopted `^fixture-` test.
- **Config-shape:** `frontend/src/lib/__tests__/deploy-workflow.test.ts`, with seeded violations.
- **Edge (D4):** `tests/edge/` (Python through `uv`) and `e2e/` (Playwright). Both run locally, and each run is recorded in the delivery's report.
- **Mutation gate:** not in force (D19).
- **Coverage:** 90% lines and functions is a target, not a gate (folio `CLAUDE.md`).

## Release binding

- **Targets:** merge 1's PR merge and merge 2's PR merge. Each deploys to Fly through `.github/workflows/deploy.yml`, and each is that merge's first real run. There is no versioned release.
- **Branch posture:**
  - Each task works on its own worktree branch.
  - The lead stacks the branches in D3's order on one delivery branch per merge.
  - One PR per merge, rebase-merged.
  - Merge 2's branch is cut from `main` after T6 (D2).
- **Gate per landing:**
  - `test.yml`'s required "Test & Lint" is green on the PR.
  - The local gate is `just check`, then from `frontend/`: `npm ci`, `npm run check`, `npm run test:unit`, `npm run build`.
  - The `audit.yml` result is read by a person (R-36, D10).
  - The local edge and `just e2e` runs are recorded (D4, D7).
  - Peter's level-3 go comes after Bolt's walk.
- **Mitigation:** the spec's rollback blocks, as placed by D15. After merge 2, a rollback for bad content sets `LIVE_LISTS_DISABLED` for that list before rolling forward, or the poisoned row loads from the first request (spec, "Rolling back for bad content").

## Spec gaps surfaced

None became a task by inference. These are for the lead.

- **SG1: the drill's post-deploy red has no trigger (holds T20's third criterion).** The spec's alert-path drill needs "the post-deploy job, on any deploy-workflow run while the switch is set". `deploy.yml` triggers only on a push to `main` and has no `workflow_dispatch`.

  Options:
  - **(a)** Re-run merge 2's `Deploy` workflow run while the switch is set. This redeploys the same commit to Fly.
  - **(b)** Land a trivial change to `main` while the switch is set.
  - **(c)** Add `workflow_dispatch` to `deploy.yml`. This is a spec change, and it moves the workflow shape that R-35 pins.

  **Recommendation: (a).** It adds no commit and no workflow change, and it redeploys a known image. It is a real Fly action, so it is attended.
- **SG2: R-5 pins a Rust integration-test harness that canon now rules out.** R-5's spawned-binary cases use `cmd/server/tests/common/`, a `tests/*.rs` target. P-TDDPairs says "no separate integration-test binary is built (in Rust, no `tests/*.rs` target)", and that binary-driving edge tests are Python. The plan follows the locked text (*P-LockContract:* "Lock the contract; vary the implementation").

  **Recommendation:** accept for this feature. The Python edge suite (T17) also drives the binary. File a follow-up to migrate `cmd/server/tests/` when it is next touched outside this spec.
- **SG3: the adopted projects spec has drifted from its own lock tag.** `docs/specs/2026-11-11-projects-from-github.md` differs from `brief/projects-from-github/spec-locked` (`d1a2633`) by 16 lines. The commits are `83f1d71`, `05aabbd`, `2306db0`, `addcea0`, `d0cc37f` and `d84ae51`, all 2026-11-18.
  - The drift includes three clauses this spec adopts: R-4's lone-surrogate rule, R-8's no-twin rule ("strictly increasing" within an owner) and R-12's preamble sentence. Each carries "agreed by Peter, 2026-11-18" inline.
  - The spec calls that file "the locked projects spec", and says its merge-2 edit "intentionally drifts the old spec's lock tag". The tag had already drifted when this spec locked.
  - The plan reads the adopted text from blob `32f66b7` (D21), the only blob holding every adopted clause.

  **Recommendation:** the lead confirms D21. Re-cutting `brief/projects-from-github/spec-locked` is a #4726 matter for Peter, not this plan's. Nothing here waits on it.
- **SG4: "the crypto provider already in `Cargo.lock`" names no single provider.** Both `ring` 0.17.14 and `aws-lc-rs` 1.18.1 are in the lockfile; `rustls` 0.23.46 depends on `aws-lc-rs`. T9's acceptance is the measured delta, so either provider passes. The builder records which one it used (level 5). No hold.
- **SG5: the Python edge suite runs in no CI job,** as Playwright does today. Each run is recorded in the delivery's report (D4). A CI job would change the workflow files and the action-pin site counts that R-35 pins.

  **Recommendation:** file a follow-up task to wire `tests/edge/` into CI with its own R-35-style assertions. This is parked, not deferred: nothing fires it. It resurfaces at T18's report.
