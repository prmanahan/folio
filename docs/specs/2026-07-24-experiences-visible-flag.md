---
status: approved
date: 2026-07-24
task: 2715
spec_type: code
reviewed_by: Peter Manahan
reviewed_date: 2026-07-24
---

# Spec: experiences.visible flag — page/AI split prerequisite

> **Spec for:** task #2715 — Track A of the content-alignment program (mechanism only)
> **Date:** 2026-07-24
> **Status:** approved — spec-exit gate confirmed by the maintainer
> **Locked:** 2026-07-24
> **Target:** folio (`libs/site-core`, `migrations/`, `frontend/` admin surface, `.github/workflows/` test gates only; public frontend untouched)
> **Parent commit:** `624cd55e77719a5d2abf1b4720066282e2df3719`
> **Intent / Frame:** `docs/intent/experiences-visible-flag.md` · `docs/intent/experiences-visible-flag-frame.md`
> **Source:** maintainer content decisions D1–D4 (ratified 2026-07-23) + code verification at the parent commit. Requirement keywords MUST / MUST NOT / SHOULD / MAY are RFC 2119.
> **Implementation pipeline:** spec (this doc → locked at maintainer sign-off) → red-phase test author writes failing tests + the mechanical compile-fix sweep → implementer makes them green + extends the in-file white-box tests (test-strength reviewed) → test-integrity re-review → code + security review of the diff → pre-merge operational gate (R-0008) → PR → merge (**merge deploys production DDL** — migrations auto-run at startup) → post-deploy verification (R-0010).

## Changelog

- **Locked (2026-07-24):** spec-exit gate confirmed by the maintainer. No content change from v2 — `status: draft → approved`, review marker stamped, `[audit_chain.spec]` written to the BOM sidecar. The "remains draft" note in the v2 entry below is historical, superseded by this line.
- **v2 (2026-07-24):** Revision against three parallel spec reviews (architecture/security, operational, testability — 32 findings, 10 high, unanimous verdict fix-then-ship, zero blockers). Substance: (1) R-0003 moved from white-box to the black-box outbound-model-call surface, with an ordered seeded-failure probe — a verifier never shown to fail on known-bad input is not yet a verifier; (2) R-0006 now names **both** duplicated form render sites (create + inline-edit) — the edit path is the operation this feature exists for, and the cited precedent's single-form shape steered straight into a create-only defect; (3) R-0008 rewritten as an ordered runbook with verifiable post-conditions over the real topology (one machine, one non-replicated volume, no canary possible); (4) new R-0009 (frontend type + unit gate wired into CI) and R-0010 (post-deploy verification) — both closed gates that could previously pass while the property they gate was false; (5) `visible` reframed as a **page-composition control** with an explicit non-guarantee — hidden rows stay reachable through the public AI surface by design; (6) test-work ownership split encoded so the green phase cannot deadlock on an unowned compile-break. Full finding disposition below. Status remains draft — pending maintainer spec-exit review.
- **v1 (2026-07-24):** Initial. Mechanism shape was ratified by the maintainer 2026-07-23; this spec encodes it with acceptance criteria, decides the one delegated open point (Decisions of note, D-1), and carries the operational deploy gate.

### v2 finding disposition (32/32 addressed; 0 deferred)

Reviews labeled **A** (architecture + security), **O** (operational), **T** (testability). T-1 carries two distinct sub-defects and is dispositioned as T-1a/T-1b, matching its own count of 7.

| ID | Sev | Disposition |
|---|---|---|
| A-H1 | High | Fixed — non-guarantee section added; D-1 and Frame relabeled page-composition control; `id`/`display_order` gap noted there |
| A-H2 | High | Fixed — R-0003 rewritten: black-box surface + ordered seeded-failure probe; `build_fit_prompt` transitivity noted |
| A-H3 | High | Fixed — R-0006 AC names `npm run check`; new R-0009 wires svelte-check + vitest into both CI workflow files |
| A-H4 | High | Fixed — R-0008 runbook: snapshot verified non-pending (a); write-quiesce replaces the unevaluable timestamp clause (b); snapshot taken with the machine stopped (c); gate-time ID resolution, spelled-out restore, data-loss window (d) |
| A-H5 | High | Fixed — R-0006 names both render sites; edit-path AC + scenario S7 added; sweep list gains both FormSection anchors |
| A-M1 | Med | Fixed — "verify" hedge replaced with the verified answer: full-object equality, no exclusions |
| A-L1 | Low | Fixed — D-6 records the CHECK-constraint omission as a decision |
| A-L2 | Low | Fixed — permitted unfiltered reads enumerated under R-0002 |
| A-L3 | Low | Fixed — distinct-`display_order` seeding note in the red-phase notes |
| A-L4 | Low | Fixed — seeding locked to in-test insert via `test_app_with_state`; shared fixture untouched |
| A-L5 | Low | Fixed — D-1 names the stale-tab manifestation and the reload remedy |
| A-L6 | Low | Fixed with correction — `create`/`update`/llm07a anchors updated (`:159`/`:201`/`:79`); the two `list_published` rows of the finding were themselves off by one (the spec's `:37`/`:35` land on the `WHERE published = 1` lines — re-verified at the parent commit); citations now name fn + filter lines so no ambiguity survives |
| A-N1 | Nit | Fixed — "exactly one SQL statement; a 005-style header comment is expected and is not a statement" |
| A-N2 | Nit | Fixed — `dflt_value`-returns-text clause added to R-0001 acceptance |
| A-N3 | Nit | Fixed — pre-existing toast defect recorded in Out of scope so it is not mistaken for a regression |
| O-H1 | High | Fixed — restore runbook written out (new-volume-from-snapshot + machine re-point), one-shot restore drill required, rollback data-loss window stated |
| O-H2 | High | Fixed — topology stated honestly in R-0008: no canary/blue-green possible; failed migration = site down; no auto-revert assumed |
| O-H3 | High | Fixed — new R-0010 post-deploy verification (public baseline diff + admin column check; the `SELECT 1` health probe named necessary-but-insufficient) |
| O-M1 | Med | Fixed — R-0001 now requires the dedicated registration test per the repo's own 005 precedent |
| O-M2 | Med | Fixed (recorded decision) — D-7: the operational gate stays procedural, with a named tripwire to a CI PR-body backstop |
| O-M3 | Med/Low | Fixed — deploy-downtime baseline named in R-0008 |
| O-L1 | Low/Nit | Fixed — atomic frontend+backend image noted in D-1's declared consequence |
| O-N1 | Nit | Fixed — grace-period note in R-0008 (pre-existing margin, unchanged by 006) |
| O-N2 | Nit | Fixed — same clause replacement as A-H4(b) |
| O-N3 | Nit | Fixed — per-migration transaction anchor corrected to `schema.rs:86` |
| T-1a | High | Fixed — the "not reachable over HTTP" claim removed; R-0003 moved onto the crate's proven outbound-capture pattern |
| T-1b | High | Fixed — schema-red accepted as the pre-006 red state; behavioral validity proven post-green by the seeded-failure probe with the failure message recorded |
| T-2 | High | Fixed — ownership split encoded (mechanical compile-fix = red-phase sweep; in-file white-box extension = implementer under test-strength review); S4's raw-JSON-only nature stated |
| T-3 | Med | Fixed — `test_app_with_state` variant specified; shared-fixture blast radius contained |
| T-4 | Med | Fixed — vitest AC pins `emptyForm()` default; `npm run test:unit` added to verification and CI |
| T-5 | Low | Fixed — admin-auth helper named as new work (primitives exist, helper does not) |
| T-6 | Nit | Fixed — "migration-005 *pattern*" rewording |

## Decisions already made (do not reopen)

- **D2 cut-line (maintainer, 2026-07-23):** the public page is resume-shaped — roles from Senior Development Manager (2012-08) forward show; pre-2012 roles are AI-layer-only. This spec delivers the *mechanism* for that split. It does **not** apply the cut.
- **Mechanism shape (task #2715):** migration 006 adds `visible INTEGER NOT NULL DEFAULT 1` to `experiences`; `list_public` filters `WHERE visible = 1`; `visible` threads through admin CRUD and the admin UI. Precedent: the `published` flag on `projects` and `articles`.
- **`DEFAULT 1` is load-bearing:** the deploy is behaviour-preserving; the visibility cut is applied later as data through the admin API (separate content track).
- **Forward-only migrations** (this repo's `decisions/DEFAULTS.md` G-0005 — note: this repo's own ADR numbering): no down migration. Rollback for a bad 006 is a hosting-volume snapshot restore (R-0008).
- **D4 (maintainer, 2026-07-23):** private/AI fields stay private. The existing Tier-A hard-drop and the public/private column split are untouched by this change.

## What `visible` is — and is not

`visible = 0` removes a row from the **rendered public page** only. It is a **page-composition control, not a confidentiality boundary**:

- Hidden rows remain in the AI system prompt **by design** — that is D2's second half, and R-0003 exists to pin it. The chat and fit endpoints are public and unauthenticated: `cmd/server/main.rs:159` mounts the AI routes as a sibling of the admin router, and only the admin router carries auth (`libs/site-core/routes/admin/mod.rs:34`). So **when AI is enabled** (`ANTHROPIC_API_KEY` set — the live site advertises the feature; when unset the path returns the AI-disabled error and nothing is reachable), an anonymous visitor can elicit a hidden role's content by asking the chat. The only guards are per-window rate limits (`libs/site-core/routes/ai.rs:168`, `:273`).
- Secondarily, the public JSON serializes `id` (AUTOINCREMENT) and `display_order` (`libs/site-core/models/experience.rs:5-16`), so a hidden row leaves an observable gap in both sequences — a second, weaker reason the flag is not concealment.

Content that must not be publicly disclosed MUST NOT live in `experiences` at all, or must sit in a Tier-A hard-dropped field (D4). Do not set `visible = 0` on an NDA'd role or a role under a separation agreement and believe it is off the public internet. It is not.

## Requirements

R-IDs are stable identifiers for the implementation record. Line numbers are anchored at the parent commit; treat them as starting points — the implementer locates the current site.

### R-0001 — Migration 006: column added AND migration registered (two steps, both mandatory)

**Problem.** Migrations in this repo are NOT directory-scanned. `libs/site-core/db/schema.rs:5` holds a compiled `MIGRATIONS` array of `(version, name, sql)` tuples where each `sql` is `include_str!("../../../migrations/NNN_*.sql")`. Dropping a file into `migrations/` is necessary but **not sufficient** — a migration missing its array entry never runs, silently.

**Fix.**
1. Create `migrations/006_experience_visibility.sql` containing exactly one SQL statement:
   `ALTER TABLE experiences ADD COLUMN visible INTEGER NOT NULL DEFAULT 1;`
   A header comment block per the 005 precedent (`migrations/005_site_config.sql`) is expected and is not a statement.
2. Register it in the `MIGRATIONS` array in `libs/site-core/db/schema.rs` as
   `(6, "experience_visibility", include_str!("../../../migrations/006_experience_visibility.sql"))`.

The migration MUST contain no other statement — no UPDATE, INSERT, or DELETE (R-0007). The non-idempotent `ALTER TABLE` is guarded by the `_migrations` version-tracking table, per this repo's `decisions/DEFAULTS.md` G-0005.

**Acceptance.**
- A schema-shape test (following the migration-005 *pattern* — `libs/site-core/tests/test_schema.rs:64` tests `site_config`, not `experiences`; reuse its `PRAGMA table_info` + `ColInfo` approach) asserts on a freshly migrated database: column `visible` exists, type `INTEGER`, `NOT NULL`, and `dflt_value` is the **string** `"1"` — the pragma returns defaults as text, and the 005 precedent reads it as `Option<String>`.
- A migration-on-legacy-data test: build a database at the 005 schema, insert experience rows, run migrations; every pre-existing row has `visible = 1` and all other column values are unchanged.
- A **dedicated registration test** asserts the `MIGRATIONS` array contains the version-6 entry, following the repo's own per-migration precedent (`migrations_const_array_registers_005_site_config`, `libs/site-core/tests/test_schema.rs:212`). The shape test also only passes through the real array (`libs/site-core/tests/common/mod.rs:18` migrates via it), but the dedicated test keeps the established pattern and gives a distinct, named failure for "never registered" vs "wrong column".

### R-0002 — Public read path excludes hidden rows; public response shape unchanged

**Problem.** `list_public` (`libs/site-core/models/experience.rs:38`) has no `WHERE` clause — every row in `experiences` is served by the public route `GET /api/experience` (`libs/site-core/routes/experience.rs:18`).

**Fix.** `list_public` adds `WHERE visible = 1`, matching the precedent form of `list_published` (`libs/site-core/models/project.rs:34`, filter at `:37`; `libs/site-core/models/article.rs:32`, filter at `:35`). `ExperiencePublic` MUST NOT gain a `visible` field — on the public surface the value is `1` by construction, and the public JSON shape is part of the deploy's behaviour-preservation claim.

**Reads that deliberately stay unfiltered** (the permitted-path list, so a reviewer checks rather than re-derives it):
- `list_all` (`libs/site-core/models/experience.rs:133`) — admin list + AI prompt assembly (D-4; R-0003).
- `get_by_id` (`libs/site-core/models/experience.rs:146`) — admin detail.
- `SELECT COUNT(*) FROM experiences` (`libs/site-core/routes/admin/dashboard.rs:29`) — admin sidebar count; counts hidden rows, by design.

All are behind `require_auth` (`libs/site-core/routes/admin/mod.rs:34`) except prompt assembly, which is R-0003's whole point — and which is why the non-guarantee section above exists. `list_public` is the only unauthenticated JSON read of `experiences` (verified crate-wide at the parent commit).

**Acceptance (black-box over HTTP).**
- Given one row with `visible = 1` and one with `visible = 0`, `GET /api/experience` returns the visible row and does NOT return the hidden row (asserted on row identity/content, not on count alone).
- Response objects carry no `visible` key, and the existing public field set is unchanged (extends `libs/site-core/tests/test_experience.rs`, seeding the hidden row in-test via the `test_app_with_state` helper — see Notes).
- `get_by_id` and the admin list remain unfiltered (covered by R-0004).

### R-0003 — AI-context invariant: the model still receives hidden rows (the load-bearing test)

**Problem.** `build_system_prompt` (`libs/site-core/ai/context.rs:31`) reads `experience::list_all` at `ai/context.rs:34` — NOT `list_public` — so D2's "pre-2012 roles stay available to the AI" is satisfied *naturally* by today's code. That is exactly what makes it fragile: nothing prevents a future refactor from repointing prompt assembly at the filtered query and silently stripping the AI's hidden-role context. No behaviour change is required here; this requirement exists to **pin** the invariant.

**Fix.** No production-code change. The AI surface MUST continue to receive experiences regardless of `visible`. A black-box test at the outbound-model-call boundary MUST lock it. Placing this test on the black-box surface is deliberate: it is the spec's most game-able assertion, and the black-box author is the party who cannot game it.

**Acceptance (black-box: POST `/api/chat` → captured outbound model-call body).**
- Test harness: the crate's existing mocked-upstream chat pattern — `libs/site-core/tests/common/ai_mock.rs::ai_test_app_with_mock_and_state` (`ai_mock.rs:81`; returns `(TestServer, DbState)` so the test seeds rows through the exposed connection), with the outbound-body capture via `.match_request` + shared buffer proven at `libs/site-core/tests/test_ai_chat_endpoint.rs:45-103`.
- Given two seeded rows — one `visible = 1`, one `visible = 0`, each carrying a distinct sentinel company/content string — when the test POSTs `/api/chat`, the captured outbound Anthropic request body's `system` payload contains BOTH sentinels. `system` is an array of blocks (see the cache-control assertion at `test_ai_chat_endpoint.rs:147-160`) — assert over the concatenated block text. Assert on row *data*, not on which function is called.
- **Red-phase form.** Before migration 006 exists, this test can only fail on seeding (`no such column: visible`). That schema-red is the accepted red state for pipeline ordering (precedent: this repo's #572 red phase used compile-red as its red state). The behavioral validity of the assertion is established by the seeded-failure probe below — not by the red phase. The red-phase author SHOULD NOT chase a behavioral red before the column exists; none is possible.
- **Seeded-failure probe (ordered; run once, after the implementation is green).** The implementer temporarily repoints the experience read in `build_system_prompt` (`libs/site-core/ai/context.rs:34`) from `experience::list_all` to `experience::list_public`, runs this test, and confirms it fails **on the sentinel assertion** — not on compile or setup. The observed failure message is quoted verbatim in the implementation record. The seed edit is then reverted and the full suite re-run green. A verifier that has never been shown to fail on a known-bad input is not yet a verifier.
- `build_fit_prompt` (`libs/site-core/ai/context.rs:355`) delegates to `build_system_prompt` (`:357`), so the fit surface is transitively pinned — no second test is needed. Tripwire: if that delegation is ever split, the fit path needs its own pin.

### R-0004 — Admin data surface carries `visible` as a required field (no serde default)

**Problem.** `ExperienceFull` and `ExperienceInput` (`libs/site-core/models/experience.rs:48`, `:109`) do not carry `visible`; `create` (`:159`) and `update` (`:201`) name their columns explicitly, so until the field is added the admin surface cannot express the flag at all.

**Fix.**
1. `ExperienceFull` gains `visible: bool`, read in `from_row`, selected in `list_all` (`:133`) and `get_by_id` (`:146`). The admin routes (`libs/site-core/routes/admin/experience.rs`) therefore return it on GET, and the admin list continues to return hidden rows.
2. `ExperienceInput` gains `visible: bool` as a **required field — no `#[serde(default)]`, no default-true function** (decision D-1 below; this is the same contract as every other field on the struct and as `ProjectInput.published` / `ArticleInput.published`, `libs/site-core/models/project.rs:96`, `libs/site-core/models/article.rs:91`).
3. `create` and `update` add the `visible` column to their explicit column lists and bind `input.visible as i64`.

**Acceptance.**
- Admin `GET /api/admin/experience/{id}` and the admin list return `visible` for every row, including hidden rows.
- Admin `PUT` with `"visible": false` persists it: subsequent public list excludes the row; subsequent admin GET returns `visible: false`.
- Admin `POST` with `"visible": false` creates a row absent from the public list and present in the admin list.
- A `PUT` or `POST` whose body omits `visible` is rejected with the deserialization failure status the admin API already produces for any missing required field (422-class; asserted as the same status the current API returns for a body missing `company_name`). It MUST NOT succeed with an implied value. This rejection can only be expressed as a raw-JSON HTTP request — a Rust struct literal cannot omit a required field and compile (see Notes, ownership split).

### R-0005 — Admin round-trip preserves `visible = 0` (the real-world write path)

**Problem.** The admin update path is a full-object overwrite: `update` (`libs/site-core/models/experience.rs:201`) SETs every column it names from the input. The operating protocol for content edits is GET → mutate → PUT the whole object → re-GET and diff. If `visible` survives that cycle wrongly (dropped, defaulted, or flipped), a routine content edit un-hides or hides a role as a side effect.

**Fix.** Behaviour follows from R-0004; this requirement pins the end-to-end cycle.

**Acceptance (black-box over HTTP).**
- Given a row with `visible = 0`: admin `GET` the full object → `PUT` it back **unmodified** → re-`GET`. The re-fetched object has `visible = 0` and every other field byte-equal to the first GET. Full equality holds with **no exclusions**: `update`'s SET list (`libs/site-core/models/experience.rs:210-218`) does not write `created_at` or `id`, and the JSON columns (`bullet_points`, `quantified_impact`) re-serialize canonically from `serde_json::Value` on both GETs.
- The same cycle mutating one unrelated field (e.g. `summary`) still preserves `visible = 0`.

### R-0006 — Admin UI toggle — in BOTH form renderings

**Problem.** The admin experience page renders the form **twice — duplicated markup, not a shared component**: a create branch (`{#if creatingNew}`, `frontend/src/routes/admin/experience/+page.svelte:165`, its "Public Information" FormSection at `:176`) and a separate inline-edit branch (`{#if editingId === item.id}`, `:316`, its own "Public Information" FormSection at `:327`). The projects precedent is structurally different — a **single shared form** switched by `editingId` (`frontend/src/routes/admin/projects/+page.svelte:159-161`), where one checkbox (`:196`) covers both operations. Followed literally here, that precedent yields a create-only toggle: it type-checks, passes every backend test — and the maintainer cannot hide the six existing pre-2012 rows, which is the entire purpose of this feature. Additionally, with R-0004's required-field contract, an un-updated form could not save at all.

**Fix.** Specify behaviour, not implementation:
1. **Both form renderings** — the create branch (FormSection at `+page.svelte:176`) AND the inline-edit branch (FormSection at `:327`) — MUST render a visibility toggle (checkbox) bound to the form's `visible` value, in their Public Information sections, following the projects checkbox *control* pattern (`projects/+page.svelte:196`) at each site.
2. `emptyForm()` (new-entry state, `+page.svelte:20`) MUST default `visible` to `true`.
3. The edit-load mapping (`startEdit`, `+page.svelte:71`) MUST populate the toggle from the fetched value; create/update submissions MUST send `visible`.
4. The TypeScript admin types (`frontend/src/lib/admin-types.ts:74` `ExperienceFull`, `:99` `ExperienceInput`) MUST gain `visible: boolean` (required, not optional — mirrors the backend contract).
5. The experience list rows SHOULD show a visible/hidden badge, following the projects published/draft pill (`projects/+page.svelte:139`). Single render site (`{#each items as item}`, `+page.svelte:282`) — the dual-render trap does not apply to the badge.
6. A frontend unit test (vitest — `npm run test:unit`, already wired at `frontend/package.json:13`) MUST assert the new-entry form state defaults `visible` to `true`. A unit test is not browser automation and is not covered by the e2e carve-out (Out of scope).

**Acceptance.**
- Both FormSections (`:176` and `:327`) contain a checkbox bound to `visible` — the reviewer checks both anchors. A toggle present only in the create branch does NOT satisfy this requirement.
- **The edit path can toggle an existing row (S7 — the Track C operation):** open an existing row inline, uncheck, save → the row disappears from `GET /api/experience` and shows `visible: false` on admin GET; re-check and save → it reappears. The persistence half is asserted at the HTTP layer (R-0004); the binding half by the dual-anchor review above.
- A new entry saves with `visible = true` unless unchecked (S6); the vitest test pins `emptyForm().visible === true`.
- `npm run check` (svelte-check) fails on an object literal omitting `visible` from either interface. **Note:** `npm run build` is `vite build` (`frontend/package.json:8`) — esbuild strips types without checking them, so the *build* does NOT enforce this. The gate that actually runs the check is R-0009.

### R-0007 — The deploy is behaviour-preserving; the cut is data, not code

**Problem.** Merge = deploy = production DDL on a live public site. A reviewer who misses that the migration changes nothing visible will over-rate the risk; an implementer who "helpfully" applies the D2 cut in the migration would turn a low-risk deploy into an unreviewed content change.

**Fix.** After migration 006 runs in production, every existing row has `visible = 1` and the rendered site is unchanged. The migration MUST NOT write data (no UPDATE/INSERT/DELETE); this change MUST NOT set `visible = 0` on any production row. Applying the D2 cut (six pre-2012 rows → `visible = 0`) happens later, via the admin API, in the content track, after the maintainer approves the change-set.

**Acceptance.**
- The legacy-data migration test of R-0001 (all pre-existing rows `visible = 1`).
- `migrations/006_experience_visibility.sql` contains only the single `ALTER TABLE` statement (plus the conventional header comment, per R-0001).
- The public list on a migrated legacy database returns exactly the rows it returned pre-migration (same identities, same order).

### R-0008 — Pre-merge operational gate: verified-restorable snapshot; rollback is a runbook, not a strategy name

**Topology (verified against the live app 2026-07-24 — the safety argument must reflect it honestly).** The app runs **exactly one machine** (at authoring time `879206f0774ed8`, region yyz, shared-cpu-1x/256MB) on **one non-replicated volume** (mount `site_data`, `fly.toml:13-15`); `min_machines_running = 0` and `auto_stop_machines = "stop"` (`fly.toml:9-11`), so the machine is stopped much of the time and every deploy replaces the sole machine. Consequences:

- **No canary or blue-green is possible.** A second machine cannot attach to the non-replicated volume.
- **A failed migration takes the site down — it does not degrade.** `run_server` panics if the DB fails to open (`cmd/server/main.rs:100`), and `connect` runs migrations before returning (`libs/site-core/db/mod.rs:22`) — the listener never binds and `/api/health` never passes. Do NOT assume the platform auto-reverts to the previous image at this topology; treat a failed deploy as an outage requiring intervention (rollback or fix-forward).
- **Brief downtime is the baseline for any deploy** of this app (the sole machine is replaced). Pre-existing; this spec does not change it.
- The health-check `grace_period = "15s"` (`fly.toml:24`) covers DB connect + migrations + Argon2 hashing today; 006's single `ALTER TABLE` is sub-millisecond and does not materially change cold-start timing (pre-existing margin unmeasured; out of scope).
- Migrations are forward-only (this repo's `decisions/DEFAULTS.md` G-0005) — there is no down migration. The volume's automatic daily snapshots (5-day retention) are not a pre-deploy backup; the gate below creates one.

**Runbook.** Executed by the orchestrating operator at merge time; the implementer does not run it. It is a requirement of this spec so the deploy's safety argument is carried by the governing artifact, not by process memory. Each step has a verifiable post-condition. Command shapes are from the platform CLI's help output at review time — **re-check `--help` at gate time before running each command.**

1. **Resolve live IDs — do not trust literals in this spec.** `flyctl volumes list -a folio-prmanahan` and `flyctl machines list -a folio-prmanahan`. Volume and machine IDs drift when infrastructure is rebuilt; a stale literal means snapshotting the wrong volume. (At authoring time: volume `vol_v3lo2z522gm2qllv`, machine `879206f0774ed8`.) *Post-condition:* both resolved IDs recorded in the PR body.
2. **Quiesce writes.** No admin content writes from this point until post-deploy verification (R-0010) completes. This replaces the previous "snapshot later than the last production content write" clause, which was unevaluable — there is no `updated_at` column and no audit log to evaluate it against. *Post-condition:* an explicit attestation line in the PR body.
3. **Stop the machine** (`flyctl machine stop <machine-id>`). A block-level snapshot of a live SQLite file can capture a torn / mid-WAL state, and a torn snapshot is not a rollback; with the process down the file is quiescent. The site is routinely stopped anyway (`min_machines_running = 0`). *Post-condition:* machine state `stopped` in `flyctl machines list`.
4. **Create the snapshot:** `flyctl volumes snapshots create <vol-id>`.
5. **Verify the snapshot EXISTS — creation is asynchronous and can fail.** `flyctl volumes snapshots list <vol-id>` MUST show the new snapshot in a non-pending state with nonzero size. A requested-but-unconfirmed snapshot does not satisfy this gate. *Post-condition:* that output line (ID + state + created-at) pasted into the PR body. *Fallback:* if snapshot creation fails against a stopped machine (platform behavior not empirically confirmed), start the machine, keep the write-quiesce, re-snapshot, and record the residual torn-write risk in the PR body as accepted.
6. **Restore drill (one-shot; required before THIS merge — subsequent migration PRs may cite it).** Prove the snapshot is restorable without touching the live machine: `flyctl volumes create site_data_drill --snapshot-id <snap-id> -a folio-prmanahan --region yyz` → verify the new volume materializes with plausible size → destroy it (`flyctl volumes destroy <drill-vol-id>`). *Post-condition:* drill volume ID + its verification line in the PR body. This exercises the restore half that can be drilled safely; the machine re-point half (step 8b) remains a written-but-unexercised procedure and is recorded as such.
7. **Merge and watch.** The deploy pipeline (`.github/workflows/deploy.yml` — test job, then `flyctl deploy`) replaces and starts the machine. Watch the workflow run and `flyctl logs`. Success = the automated health check passes AND R-0010 passes. Failure = go to step 8, or fix-forward; do not assume the old version is still serving (see Topology).
8. **Rollback procedure — restore is NOT in-place.** Restoring means **provisioning a new volume from the snapshot and re-pointing the machine**:
   a. `flyctl volumes create site_data --snapshot-id <snap-id> -a folio-prmanahan --region yyz` — the new volume MUST carry the mount name `site_data` (`fly.toml:14`), which is how the mount binds.
   b. Re-point the single machine at the new volume. The CLI exposes no simple mount-swap; in practice this means stopping and destroying the machine, then re-deploying (or cloning a machine against the new volume) — confirm exact steps against current CLI help at execution time. This is the unexercised half named in step 6.
   c. **Data-loss window:** the restore discards every write made after the snapshot. With step 2's quiesce the intended loss is page-hit counters only; record the actual loss in the incident note.
   The migration runner's per-migration transaction (`libs/site-core/db/schema.rs:86`) additionally rolls back a *failing* 006 atomically — the snapshot covers what the transaction cannot: a migration that succeeds but is wrong, or damage beyond the migration itself.

**Enforcement posture:** this gate is procedural — no CI step verifies the PR body before the deploy job runs. That is a recorded decision with a tripwire: D-7.

**Acceptance.**
- The PR body contains: the resolved volume + machine IDs (step 1); the write-quiesce attestation (step 2); the machine-stopped-at-snapshot state (step 3); the `snapshots list` line showing the non-pending snapshot (step 5); the restore-drill record (step 6); and the rollback path stated as this runbook's step 8.
- Post-deploy, R-0010 passes and its results are recorded.

### R-0009 — Frontend type + unit gate runs in CI (mechanism, not convention)

**Problem.** The frontend half of D-1's fail-loud contract is currently a convention, not a mechanism. `npm run build` is `vite build` (`frontend/package.json:8`) — esbuild strips types without checking them, so an object literal omitting a required property transpiles cleanly. The only type-check is `npm run check` (`svelte-kit sync && svelte-check`, `package.json:11`), and **no gate runs it**: CI (`.github/workflows/test.yml:35-45`) runs `npm ci && npm run build`, `cargo test`, `cargo clippy` — no svelte-check, no vitest — and `just check` has no frontend step at all. A gate the implementer must remember to run locally contradicts the Frame's constraint 3 (guarantees carried by mechanism, not convention) — the same principle D-1 leans on to justify the required-field posture.

**Fix.** Add two steps to **both** workflow files — `.github/workflows/test.yml` AND the deliberately-duplicated test job in `.github/workflows/deploy.yml` (updating only `test.yml` would gate PRs but not the deploy pipeline):
1. `npm run check` in `frontend/` (svelte-check / tsc), and
2. `npm run test:unit` in `frontend/` (vitest — carries R-0006.6's `emptyForm()` default test),
both ordered after `npm ci`. `just check` stays backend-only; local frontend verification remains items 5–6 of the Verification section.

**Acceptance.**
- Both workflow files contain both steps.
- With the required `visible: boolean` on both interfaces (R-0006.4), an object literal omitting `visible` fails `npm run check`. The implementer confirms this once during development by temporarily removing the field from `emptyForm()` and observing the check fail (the frontend analogue of R-0003's seeded-failure probe; the observation goes in the implementation record).

### R-0010 — Post-deploy verification of the deployed feature

**Problem.** The deploy pipeline's only post-deploy check is `curl --fail https://peter.manahan.io/api/health` (`.github/workflows/deploy.yml:73-74`), which executes `SELECT 1` (`libs/site-core/routes/mod.rs:20-22`) — DB liveness only. It reports healthy with `visible` filtering completely broken. Every other verification item is pre-merge and local; nothing checks the real deploy against real production data.

**Fix (operator steps, immediately after the deploy completes).**
1. **Baseline:** during R-0008 step 2 (pre-merge, writes quiesced), save the response body of `GET https://peter.manahan.io/api/experience`.
2. **Public surface unchanged:** post-deploy, fetch the same URL. The JSON MUST be identical to the baseline — same rows, same order, no `visible` key. R-0007's behaviour-preservation claim is the oracle: `DEFAULT 1` means nothing is hidden yet, so any diff is a defect.
3. **Feature live in production:** authenticated `GET /api/admin/experience` returns `visible: true` on every row — proving the column exists and serializes in production, i.e. migration 006 actually ran end-to-end.
4. `/api/health` returns 200 (already automated; necessary, NOT sufficient).

**Acceptance.**
- The three checks recorded with the merge (PR body or merge note): empty public diff, the all-rows-`visible: true` admin observation, health 200.
- Any mismatch is an incident: go to R-0008 step 8 (rollback) or fix-forward — decided then, not silently absorbed.

## Scenarios

**S1 — Public exclusion (R-0002)**
Given experiences "Alpha Corp" (`visible = 1`) and "Hidden Corp" (`visible = 0`)
When an unauthenticated client requests `GET /api/experience`
Then the response contains "Alpha Corp", does not contain "Hidden Corp", and no object carries a `visible` key.

**S2 — AI inclusion (R-0003)**
Given the same two rows
When a client POSTs `/api/chat` (mocked upstream) and the outbound model-call body is captured
Then the captured `system` payload contains both "Alpha Corp" and "Hidden Corp" sentinels.

**S3 — Round-trip preservation (R-0005)**
Given "Hidden Corp" (`visible = 0`)
When an authenticated admin GETs the full object and PUTs it back unmodified
Then the re-fetched object still has `visible = false` and no other field changed.

**S4 — Stale client fails loud (R-0004)**
Given an authenticated admin client that PUTs the pre-006 object shape (no `visible` key)
When the request is deserialized
Then the API rejects it with the same missing-required-field status it returns today for a body missing `company_name`, and the stored row is unchanged.

**S5 — Legacy database migrates behaviour-preservingly (R-0001, R-0007)**
Given a database at the 005 schema containing N experience rows
When migrations run at startup
Then all N rows have `visible = 1` and the public list returns the same N rows in the same order as before the migration.

**S6 — New entry defaults visible (R-0006)**
Given the admin form opened for a new experience
When the maintainer saves without touching the toggle
Then the created row has `visible = true` and appears in the public list.

**S7 — Hiding an existing role from the page (R-0006 — the operation this feature exists for)**
Given an existing row currently on the public page
When the maintainer opens it in the inline-edit branch, unchecks the visibility toggle, and saves
Then the row disappears from `GET /api/experience`, remains in the admin list with `visible: false`, and remains in the AI context (R-0003).

## Data model

`experiences` table delta (schema after migration 006):

| Column | Type | Constraints | Notes |
|---|---|---|---|
| `visible` | `INTEGER` | `NOT NULL DEFAULT 1` | 1 = on the public page; 0 = AI-layer-only. Shape matches `projects.published` / `articles.published` (`migrations/001_initial_schema.sql:134`, `:145`); default differs deliberately (1, not 0) so the migration is behaviour-preserving. |

No other schema change. No index: the table is a handful of rows read whole; a partial index on a nine-row table is mechanism without evidence (D-5). No CHECK constraint on the 0/1 domain — a recorded decision, D-6. Semantics: see "What `visible` is — and is not."

## API contract

| Surface | Change |
|---|---|
| `GET /api/experience` (public) | **Response shape unchanged.** Row set now excludes `visible = 0` rows. No `visible` key is serialized. |
| `GET /api/admin/experience` (list) | Each object gains `visible: boolean`. Hidden rows remain included. |
| `GET /api/admin/experience/{id}` | Object gains `visible: boolean`. |
| `POST /api/admin/experience` / `PUT /api/admin/experience/{id}` | Request body gains **required** `visible: boolean`. A body omitting it is rejected exactly as a body omitting any other required field is today (breaking change for stale admin clients — deliberate; see Decisions of note D-1). Response objects gain `visible: boolean`. |
| AI prompt construction (internal) | No contract change; R-0003 pins inclusion of hidden rows. The AI surface is public — see the non-guarantee section. |

## UI behavior (admin only)

- Visibility checkbox in the Public Information section of **both** experience form renderings — create branch and inline-edit branch (R-0006.1); checked = on the page. New entries default checked.
- Editing loads the stored value; save always transmits it.
- List rows SHOULD carry a visible/hidden badge (projects pill pattern; single render site).
- No public-frontend change: filtering is server-side, and the public page renders whatever `GET /api/experience` returns.

## Existing row-construction sites (red-phase sweep list)

The new column and required input field touch every place a full-shape `experiences` row or `ExperienceInput` is constructed. Enumerated crate-wide at the parent commit so the red-phase author sweeps them in one pass. Ownership tags per the split in Notes.

**Compile-breaks (required struct field):**
- `libs/site-core/routes/admin/experience.rs:100` — `make_input()` `ExperienceInput` literal (in-file unit tests). **Owner: red-phase author** — adding `visible: true` to a fixture literal is mechanical compile repair, not behavior authoring; the green phase must not inherit a broken build.

**Raw-SQL fixtures (keep working via the column default, but need review for visibility coverage):**
- `libs/site-core/db/seed.rs:27` — `seed_test_data()` "Meridian Systems" row; feeds every `common::test_app()` HTTP test. **Stays untouched** — R-0002's hidden row is inserted in-test via the new `test_app_with_state` helper (see Notes); mutating the shared fixture is a standing trap whose ripple is unscoped.
- `libs/site-core/ai/context.rs:396` — `seed_experience()` unit-test fixture; unaffected functionally (defaults to visible). R-0003's test now lives on the black-box surface and does not extend this fixture.
- `libs/site-core/tests/test_llm07a_private_field_drop.rs:79` — sentinel-row INSERT; unaffected functionally (explicit column list against a defaulted column), confirm no assertion breaks.

**Row-set-sensitive assertions:**
- `libs/site-core/tests/test_experience.rs:10` — `assert_eq!(body.len(), 1)` against the seed fixture; unchanged under the locked in-test seeding approach (the shared fixture gains no row), but re-verify after the sweep.
- `libs/site-core/routes/admin/experience.rs:126` (`test_crud_cycle`), `:180` (`test_public_list_lacks_admin_fields`) — extend for `visible` (round-trip + public-struct absence). **Owner: implementer, under the reviewer's test-strength pass** — these call model functions directly (white-box); the black-box HTTP tests remain the authoritative coverage.
- `libs/site-core/tests/test_schema.rs` — add the migration-006 shape test and the dedicated registration test beside the 005 precedents (`:64`, `:212`); `:21` carries a table-presence list (unchanged, `experiences` already listed).

**Frontend:**
- `frontend/src/lib/admin-types.ts:74`, `:99` — interfaces gain required `visible: boolean`.
- `frontend/src/routes/admin/experience/+page.svelte:20` (`emptyForm()`), `:71` (edit-load mapping) — both construct `ExperienceInput`-shaped objects and fail `npm run check` until updated.
- `frontend/src/routes/admin/experience/+page.svelte:176` AND `:327` — the TWO "Public Information" FormSection render sites (create branch, inline-edit branch); each gains the checkbox (R-0006.1).
- `frontend/src/routes/admin/experience/+page.svelte:282` — list-row render site for the R-0006.5 badge (single site).

**CI:**
- `.github/workflows/test.yml` and `.github/workflows/deploy.yml` (test job) — gain the `npm run check` + `npm run test:unit` steps (R-0009).

## Out of scope

- **Applying the D2 cut** (`visible = 0` on the six pre-2012 rows) and **authoring AI-layer content** (the Installation Manager story into experience AI fields). Data changes via the admin API, in the content track, after maintainer approval of the change-set. *Tripwire back into scope: none needed — the content track is already scheduled and blocked on this spec merging.*
- **Patents section** (maintainer decision D3). Separate spec; not started.
- **Tier-B → Tier-A private-field promotion review.** Deferred security review of which instruction-only AI fields become hard-dropped. *Tripwire: fires with the content track, which writes new sensitive AI-layer content.*
- **Public frontend changes.** None required; filtering is server-side.
- **A down migration.** Forbidden by this repo's `decisions/DEFAULTS.md` G-0005; rollback is R-0008's runbook.
- **New e2e (browser) coverage for the admin toggle.** The exposure boundary is fully asserted at the HTTP layer (R-0002/R-0004/R-0005); the toggle is a checkbox binding whose type-checked wiring plus the HTTP tests cover the risk. Adding browser automation for it is cost without matching risk reduction. The R-0006.6 vitest unit test is NOT browser automation and is in scope. *Tripwire: if a binding defect ships despite type-check + unit + HTTP coverage, add the e2e case then.*
- **Content-defect sweep** (`display_order` collisions, typos, casing). Rides the content track.
- **Pre-existing admin toast defect** (`frontend/src/routes/admin/experience/+page.svelte:105-108` — `editingId` is nulled before the toast ternary reads it, so the save toast always says "created"). Pre-existing at the parent commit; recorded here so it is not later mistaken for a regression of the `visible` work. Not fixed by this spec.
- **Repo intent/Frame/ADR retrofit and the DEFAULTS G-numbering drift.** Known backlog, tracked outside this spec.

## Verification (the implementer runs all of these locally before reporting done)

1. `cargo build --workspace` — compiles.
2. `cargo test --workspace` — full suite green, including the new red-phase tests.
3. `cargo clippy -- -D warnings` — clean (matches CI, `.github/workflows/test.yml`).
4. `just check` — fmt + clippy + test composite passes (backend-only; the frontend gates are items 5–6 locally and R-0009 in CI).
5. `npm run check` in `frontend/` (svelte-check/tsc) — frontend types pass with the required `visible: boolean`.
6. `npm run test:unit` in `frontend/` (vitest) — including the R-0006.6 `emptyForm()` default test.
7. Coverage: the 90% line + function floor (repo `decisions/DEFAULTS.md` G-0001) is measured locally (e.g. `cargo llvm-cov --workspace`) and reported with the completion report. Known drift: CI carries no mechanical coverage gate and the justfile has no `verify-coverage` recipe — tracked separately (maintainer task #2750); this spec's obligation is the measured number in the report.

**Build-capability check:** if `cargo build` / `cargo test` fails with a sandbox or permission error rather than a genuine compile/test failure, STOP and report it as such — do not work around it and do not report verification as structurally satisfied.

## Notes for the red-phase test author

- **Surface split:** R-0002, R-0004, R-0005 are black-box over HTTP (`common::test_app()` / the `test_app_with_state` variant below); R-0003 is also black-box, at the outbound-model-call boundary via the mocked-upstream chat harness (`libs/site-core/tests/common/ai_mock.rs`). v1 of this spec claimed R-0003 was "not reachable over HTTP" — that was wrong: `tests/test_ai_chat_endpoint.rs:45-103` already captures and asserts on the outbound `system` payload, and R-0003 reuses exactly that pattern. Note `common::test_app()` (`tests/common/mod.rs:28`) merges the public + admin routers but NOT the AI routes — use the ai_mock harness for R-0003, not `test_app`.
- **Harness work the red-phase author owns (write once, before the scenario tests):**
  - A `test_app_with_state()` variant in `tests/common/mod.rs`, mirroring `ai_test_app_with_mock_and_state` (`tests/common/ai_mock.rs:81`), returning `(TestServer, DbState)`. R-0002/R-0004/R-0005 seed the hidden row **in-test** through the exposed connection; the shared `seed_test_data()` fixture stays untouched.
  - A shared admin-auth helper. **It does not exist yet** — only the primitives do (`tests/common/test_password.rs`); existing admin tests hand-roll the login dance locally in `auth_tests.rs`. Write `common::admin_login(&server)` (or equivalent) once; `admin_router` mounts `/api/admin/login` unprotected, so `test_app`-based servers can authenticate.
- **Ownership split for existing test code (decided — neither party waits on the other):**
  - **Mechanical compile-fixes ride the red-phase author's sweep**, even inside production source files: adding `visible: true` to the `make_input()` literal (`routes/admin/experience.rs:100`) is fixture repair, not behavior authoring.
  - **Extending the in-file white-box tests** (`test_crud_cycle` `:126`, `test_public_list_lacks_admin_fields` `:180`) with `visible` assertions is the **implementer's** work, under the reviewer's test-strength pass.
  - S4's missing-field rejection cannot be expressed in the in-file suite at all — a Rust struct literal cannot omit a required field and compile. It requires a raw-JSON HTTP POST; it lives with the black-box tests.
- **R-0003's red state is a schema error, and that is accepted.** Before migration 006, seeding a `visible = 0` row fails with `no such column: visible` — no behavioral red is possible when the requirement mandates zero production-code change. Do not chase one. Behavioral validity is proven post-green by R-0003's seeded-failure probe, which is the implementer's ordered step and produces a recorded artifact.
- **Do not assert SQL text.** Testing that `list_public`'s query string contains `WHERE visible = 1` is implementation-derived and survives behavioral regressions (e.g. a broken bind). Assert served row sets.
- **The 422 assertion (S4) reuses the API's existing missing-field behavior as its oracle:** send a body missing `company_name` today, record the status, and assert the missing-`visible` body produces the same status. The oracle is sound: no custom JSON-rejection layer exists anywhere in `libs/site-core`, so every missing-required-field rejection flows through the same default mechanism.
- **Round-trip (R-0005) asserts full-object equality with no exclusions** — see R-0005's acceptance for why none are needed. The requirement exists because full-object overwrite makes *any* field droppable.
- **Seed distinct `display_order` values** in the R-0001/R-0007 legacy-data test: `ORDER BY display_order` (`libs/site-core/models/experience.rs:42`) has no defined tie-break in SQLite, and the live data is known to carry collisions — colliding seeds make the same-order assertion flaky.
- The sweep list above is at the parent commit; re-grep before writing (`ExperienceInput`, `INSERT INTO experiences`) — do not path-filter to `src/`.

## Decisions of note

**D-1 — `visible` on `ExperienceInput` is a required field: no serde default in either direction.** *Anchors: the struct's own uniform contract + the `published` precedent (lock the established shape); fail-loud-over-silent-mutation (workspace principle P-GuaranteeByMechanism — a guarantee is carried by a mechanism that fails loudly, not a convention).*

The three candidate postures fail in three different directions:
- **No default (chosen):** a client that PUTs without `visible` gets a deserialization rejection (422-class). Fails **loud**, mutates nothing.
- **`#[serde(default)]`:** `bool::default()` is `false` — every save from a stale client silently **hides** the role. Fails silent, destructive to page content.
- **Default-true fn:** a stale client silently **un-hides** a deliberately hidden role — publishes content the maintainer chose to keep off the page. Fails silent, destructive to the D2 boundary.

The tiebreaker is already in the code: `ExperienceInput` (`libs/site-core/models/experience.rs:109`) carries **zero serde defaults** — every field except `end_date: Option<String>` is required, so any client that successfully PUTs today already sends the full object, and a client omitting *any* field already 422s. Adding `visible` as required breaks no client that works today; it makes exactly one class of client fail — one not yet updated for the new schema — and it fails them loudly at the boundary instead of silently flipping the page-composition bit. A lenient default would make `visible` the *only* forgiving field on the struct, and a silent flip in either direction is an unrequested content change to the public page. `ProjectInput.published` and `ArticleInput.published` (`libs/site-core/models/project.rs:96`, `libs/site-core/models/article.rs:91`) use the same required-bool contract.

Declared consequence: the admin *write* contract is intentionally not backward-compatible for stale clients (see S4); the public surface remains behaviour-preserving. In practice this fires only for an admin browser tab left open across the deploy — frontend and backend ship as one atomic image (multi-stage Dockerfile; the binary serves the built frontend from the same release), so a fresh page load cannot skew — and it surfaces as the generic "Failed to save" toast (`frontend/src/routes/admin/experience/+page.svelte:111`); the remedy is a page reload.

**D-2 — Column named `visible`, not `published`.** *Anchors: ratified task shape; vocabulary says what the flag does.* Shape (INTEGER flag + WHERE-filter + required input bool + admin checkbox) matches the `published` precedent exactly; the *name* differs because the semantics differ — `published` is content lifecycle (a draft is not yet content), `visible` is page placement (a hidden experience is live, load-bearing AI content). Reusing `published` here would import draft semantics the row does not have.

**D-3 — `ExperiencePublic` omits `visible`.** *Anchors: minimum blast radius; behaviour-preserving public shape.* The public query filters on the flag, so the serialized value would be constant `true` — zero information, one more public-surface field to freeze. The public JSON is byte-compatible with today's.

**D-4 — `list_public` keeps its name; `list_all` remains the single unfiltered read.** *Anchors: minimum blast radius; the shared-function coupling is the point.* `list_all` serves both the admin list and `build_system_prompt` — that shared read is what makes "admin sees everything" and "AI sees everything" the same invariant, and R-0003's test guards both consumers at once. Splitting or renaming reads would be mechanism without a forcing need. The full permitted-unfiltered-read list is enumerated under R-0002.

**D-5 — No index on `visible`.** *Anchors: smallest mechanism (workspace Simplicity value); defer until evidence forces (P-Defer).* The table holds single-digit rows and is read whole; an index is speculative. Tripwire: none needed — table growth to a scale where a scan matters is self-announcing in query behavior, and the schema can gain an index in a later migration.

**D-6 — No `CHECK (visible IN (0,1))` constraint (recorded omission).** *Anchors: precedent-shape consistency (Frame constraint 5 — `published` and `is_current` are equally unconstrained); smallest mechanism.* The only writers are `create`/`update` binding a Rust `bool`. The SQLite driver maps `0 → false` and any non-zero integer → `true`, so a non-0/1 value written by manual DB editing reads as **visible** — the safe direction for a page-composition control (nothing confidential rides on the flag; see the non-guarantee section). Tripwire: a non-Rust writer, or any reframing of `visible` toward confidentiality, reopens this decision.

**D-7 — R-0008's gate enforcement stays procedural (recorded omission).** *Anchors: proportionate mechanism for a solo-operator repo (smallest mechanism); the runbook's PR-body post-conditions are the gate artifact.* No CI step verifies the PR body carries the snapshot record before the deploy job runs; the maintainer's merge review is the check. Tripwire: the first migration-bearing PR that merges **without** the R-0008 PR-body artifacts fires the structural fix — a CI step that verifies a snapshot record in the PR body before the deploy job may run. The tripwire is checked at merge review, where the PR body is already in front of the reviewer.
