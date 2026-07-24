---
modulation: brownfield-extension
scope: scoped frame-let — experiences read-path only
---

# Frame: experiences.visible flag — page/AI split (scoped frame-let)

**Date:** 2026-07-24
**Status:** locked
**Intake:** `docs/intent/experiences-visible-flag.md`

> **Scope note.** This is a scoped frame-let covering the experiences read-path only —
> the public list route, the AI context assembly, and the admin CRUD surface. It is
> deliberately **not** a full-architecture Frame retrofit of this project: the change
> adds one INTEGER column and one WHERE clause and crosses no architectural surface
> (no boundary, transport, storage-shape, or auth change). Forcing a cold-start Frame
> for that would be over-serving. The project's missing intent/Frame/ADR corpus is a
> known retrofit backlog tracked separately.

## Operating constraints

Each constraint names the canon it derives from. Workspace principles are cited by name
with an inline gloss so this document reads standalone.

1. **Two-point migration registration, forward-only, auto-run** (repo
   `decisions/DEFAULTS.md` G-0005 — this repo's own ADR numbering). Migrations are not
   directory-scanned: dropping `migrations/006_*.sql` on disk does nothing until the
   compiled `MIGRATIONS` array in `libs/site-core/db/schema.rs:5` registers it. They
   auto-run at startup, so **merge = deploy = production DDL**, and there are no down
   migrations — the rollback path is a hosting-volume snapshot restore. The migration
   runner applies each pending migration atomically (DDL + bookkeeping row in one
   transaction, `libs/site-core/db/schema.rs:72`), so a failed 006 rolls back cleanly.
2. **Coverage floor** (repo `decisions/DEFAULTS.md` G-0001): 90% line + function, hard
   gate, inverted pyramid (unit-heavy).
3. **Guarantees carried by mechanism, not convention**
   (P-GuaranteeByMechanism — a guarantee enforced by a
   loud-failing mechanism, not a convention a future reader must remember). `visible`
   is a **page-composition control** — it governs what the rendered page shows; hidden
   rows deliberately remain in the AI context, and the spec carries the explicit
   non-guarantee ("What `visible` is — and is not"). Every failure mode of this change
   must either fail loudly (a 422, a failing test) or be pinned by a test that fails
   when the invariant breaks. No silent-flip paths.
4. **Behaviour-preserving deploy; mitigation path named before commit** (workspace
   Reversibility value). `DEFAULT 1` means every existing row is visible
   post-migration — the deploy changes nothing rendered. The visible change arrives
   later as data. The named mitigation path is the pre-merge volume snapshot.
5. **Match the established precedent shape** (P-LockContract / MB3 vocabulary
   consistency — one shape per concept across the codebase). `projects` and `articles`
   already carry `published INTEGER NOT NULL DEFAULT 0`
   (`migrations/001_initial_schema.sql:134`, `:145`), a `WHERE published = 1` public
   query (`libs/site-core/models/project.rs:37`, `libs/site-core/models/article.rs:35`),
   a required `published: bool` on the input struct with **no serde default**
   (`libs/site-core/models/project.rs:96`, `libs/site-core/models/article.rs:91`), and
   an admin checkbox (`frontend/src/routes/admin/projects/+page.svelte:196`). The
   experiences flag follows this shape.
6. **Minimum blast radius** (P-MinBlastRadius — a change reaches as far as the
   architecture lets it, no further). One column, one WHERE clause, one field threaded
   through the admin surface. No rename of `list_public`, no public response-shape
   change, no refactor of the prompt-assembly path.
7. **Testability decided at spec time; exposure boundary gets a separate test author**
   (P-TestableByDesign; P-TDDPairs — non-trivial security/data-validation surfaces get
   a red-phase test task dispatched separately from implementation). The
   highest-risk assertions are reachable black-box over HTTP; the AI-inclusion
   invariant is asserted black-box at the outbound-model-call boundary (mocked
   upstream — spec v2). The spec states the split so scenario authorship lands on
   the right surface.

## Rationale chain

Intent → constraints → decisions, traceable:

1. D2 (ratified): resume-shaped page, pre-2012 roles AI-layer-only.
2. Structural gap: `list_public` (`libs/site-core/models/experience.rs:38`) serves every
   row; page-visibility and AI-membership cannot be separated (intake, Evidence).
3. Constraint 5 (precedent shape) → a per-row INTEGER flag filtered in the public
   query, not a second table, not a config list, not soft-delete.
4. Constraint 4 (behaviour-preserving) → `DEFAULT 1`: migrate now, cut later as data.
5. Constraint 1 (two-point registration, merge=deploy) → the spec encodes BOTH
   registration steps and the pre-merge snapshot gate as requirements, because each is
   invisible-from-a-casual-read and silently fatal if dropped.
6. D2's second half (AI keeps hidden rows) is satisfied **naturally** today —
   `build_system_prompt` reads `list_all` (`libs/site-core/ai/context.rs:34`) — which is
   exactly why it needs a pinning test (constraint 3): nothing else stops a future
   refactor from repointing it at `list_public` and silently stripping the AI's pre-2012
   context.
7. Constraint 3 (fail loud) + constraint 5 (precedent) → `visible` is a **required**
   field on the admin input struct, no serde default; a stale client 422s instead of
   silently flipping visibility in either direction. Full reasoning locked in the spec's
   Decisions of note.
8. Constraint 6 → everything else stays untouched: public struct, route signatures,
   prompt formatting, `get_by_id`.

## Consultations

None (Mode A consultations: zero). Elicitation collapsed under brownfield-extension;
the maintainer's D1–D4 rulings and the orchestrator's pre-spec code verification were
injected as substrate.

## Routine decisions (batched)

Within-principle decisions taken inline, reported at the spec-exit gate:

- **Column name `visible`, not `published`.** The ratified task names it, and the
  semantics differ: `published` on projects/articles is a content-lifecycle state
  (draft → published); `visible` is page-placement — a hidden experience is still
  live, load-bearing content for the AI layer. Shape matches the precedent; the name
  says what the flag does.
- **`ExperiencePublic` does not carry `visible`.** On the public surface the value is
  `1` by construction (the query filters on it); serializing it adds surface for zero
  information. Public JSON shape is unchanged.
- **`list_public` keeps its name.** The function's contract ("the rows the public
  sees") is unchanged; only its implementation gains the filter.
- **Admin UI matches the projects checkbox pattern**
  (`frontend/src/routes/admin/projects/+page.svelte:196`), including a list-row
  visible/hidden badge per the projects published/draft pill (`:139`).
- **Schema-shape test follows the migration-005 precedent**
  (`libs/site-core/tests/test_schema.rs:64` — pragma-based column assertions per
  migration).

## Escalated decisions

None fired. The one decision canon did not pre-decide — the serde-default posture for
`visible` on the admin input — was explicitly delegated to this Frame/spec author by the
orchestrator, decided in the spec (Decisions of note, D-1), and is gated by the
maintainer's spec review rather than escalated mid-stage.

## Risk profile (resolved)

The intake's trust-boundary flag resolves to three failure directions, each pinned by a
named test in the spec:

| Direction | Failure | Pin |
|---|---|---|
| **Leak** | A `visible = 0` row is served by the public API | Black-box: `GET /api/experience` excludes the hidden row (spec R2) |
| **Starve** | The AI context loses hidden rows (a future refactor points prompt assembly at the filtered query) | Black-box: the captured outbound model-call `system` payload contains a `visible = 0` row's sentinel (spec R3 — the single most load-bearing test; validity proven by a seeded-failure probe) |
| **Silent flip** | An admin write mutates visibility without anyone asking for it (serde default in either direction) | Required field: PUT omitting `visible` → 422; round-trip test preserves `visible = 0` (spec R4/R5) |

Boundary review: the existing Tier-A hard-drop and public/private column split are
untouched — no private field's exposure changes in either direction. No new data is
collected (data-minimization posture unchanged). No new routes; the only new writable
surface is one field on the already-authenticated admin API. The deploy itself is
behaviour-preserving (every existing row migrates to `visible = 1`), which is what makes
production DDL on a live site low-risk here; the operational residue (snapshot-before-
merge, restore-as-rollback) is encoded as spec requirement R8 rather than left as
process lore.

## Intent self-report

1. **JTBD interpretation.** I read the job-to-be-done as: *make page-visibility and
   AI-context membership independently controllable per experience row — mechanism
   only, with the actual cut applied later as data.* Everything in the spec serves that
   single split; no content moves.
2. **Intent strain.** Two declared:
   - The intake's success criterion 4 says the deploy is behaviour-preserving. That is
     true of the **public** surface, but the admin **write contract** intentionally
     breaks compatibility: a client that PUTs the pre-006 object shape (20 fields, no
     `visible`) receives a 422 until updated. This is deliberate fail-loud (constraint
     3) and is small in practice — the input struct already rejects any missing field,
     so only fully-conforming clients work today — but it strains a literal reading of
     "behaviour-preserving," so it is declared here and in the spec rather than left
     implicit.
   - R8 (pre-merge snapshot) is an operational gate, not a code change — the edge of
     what a code spec normally carries. Encoded anyway: merge = deploy = prod DDL makes
     the operational step inseparable from the change's safety argument, and an
     unstated gate is a silent one.

## Changelog

- **v2 (2026-07-24):** Spec-review round 1 alignment. `visible` relabeled a
  page-composition control (constraint 3) — the explicit non-guarantee lives in the
  spec; the R3 pin moved to the black-box outbound-model-call surface (constraint 7,
  risk table). The locked decisions themselves are unchanged.
- **v1 (2026-07-24):** Initial lock.
