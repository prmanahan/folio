---
spec_type: code
frame_relevant: true
---

# Intake: experiences.visible flag — page/AI split prerequisite

**Stakes:** high
**Date:** 2026-07-24
**Status:** locked
**Consumer:** mixed — plan-tier decomposer + implementing/test agents; end beneficiary is the site maintainer

## JTBD

The maintainer needs page-visibility and AI-context membership to be **independently
controllable per experience row**, so that pre-2012 roles can come off the public resume
page while remaining fully available to the site's AI chat and job-fit context.

Today the two are welded together: `list_public`
(`libs/site-core/models/experience.rs:38`) has no `WHERE` clause, so every row in
`experiences` renders publicly. A row kept for the AI layer is necessarily a public row;
a row deleted to hide it is also gone from the AI's context. The maintainer's ratified
content decision (D2, 2026-07-23 — resume-shaped page, cut-line at Senior Development
Manager, 2012-08) is not expressible in the current schema.

## Non-goals

- **Applying the content cut.** Setting `visible = 0` on the six pre-2012 rows is DATA,
  applied later via the admin API in a separate content track. This work ships the
  mechanism only.
- **Authoring AI-layer content** for any role (e.g. the Installation Manager story into
  experience AI-context fields). Separate content track.
- **Patents section** (D3). Separate spec, not started.
- **Promoting instruction-only AI fields to hard-drop** (Tier-B → Tier-A review).
  Deferred to a separate security review track.
- **Public API response-shape changes.** The public experience JSON keeps its exact
  current shape.
- **Content-defect sweep** (`display_order` collisions, typos, casing). Rides the
  content track, not this change.
- **Retrofitting this repo's intent/Frame/ADR corpus.** Known backlog, logged
  separately.

## Success criteria

Each is an observable outcome a check can verify:

1. A row with `visible = 0` does not appear in the `GET /api/experience` response.
2. That same row's content still appears in the AI system prompt built by
   `build_system_prompt` (`libs/site-core/ai/context.rs:31`).
3. An admin `GET → PUT` round-trip of a full experience object preserves `visible = 0`
   and every other field.
4. Running migration 006 against a database with existing rows leaves every existing row
   visible — the rendered public site is byte-identical before and after deploy.
5. The admin UI exposes a per-experience visibility toggle that persists through save.
6. The repo's 90% line + function coverage floor still holds (repo
   `decisions/DEFAULTS.md` G-0001 — this repo's own ADR numbering).

## Hard constraints

- **Mechanism is ratified, not open.** Maintainer decisions of 2026-07-23 (D1–D4) and
  task #2715 fix the shape: migration `006` adds
  `visible INTEGER NOT NULL DEFAULT 1` to `experiences`; `list_public` filters
  `WHERE visible = 1`; `visible` threads through admin CRUD and the admin UI.
- **Cut-line (D2) is locked**: page shows Senior Development Manager (2012-08) forward;
  pre-2012 roles are AI-layer-only. Not re-litigated here.
- **Migrations are compiled-in and forward-only** (repo `decisions/DEFAULTS.md` G-0005):
  SQL files under `migrations/` are `include_str!`-embedded via a registration array in
  `libs/site-core/db/schema.rs` and auto-run at application startup. No down migrations.
- **Merge = deploy = production DDL.** Startup auto-run on the production host means the
  merge that lands this change executes the DDL against the live database.
- **Rollback is a volume snapshot restore**, not a down migration.
- **D4 stands**: private/AI fields stay private; the existing code-verified
  public/private boundary is not weakened.
- **Coverage floor**: 90% line + function (repo `decisions/DEFAULTS.md` G-0001).

## Evidence

- Live-vs-baseline gap analysis, maintainer-ratified 2026-07-23 (decisions D1–D4):
  identified the structural blocker verbatim — "`list_public` has no `WHERE` clause —
  every row in `experiences` renders publicly."
- Code verification at parent commit `624cd55` (2026-07-24) confirmed:
  `libs/site-core/models/experience.rs:38` (no filter);
  `libs/site-core/ai/context.rs:34` (`build_system_prompt` reads `list_all`, not
  `list_public`); `libs/site-core/db/schema.rs:5` (`MIGRATIONS` registration array).
- Precedent in this codebase: `projects` and `articles` already carry
  `published INTEGER NOT NULL DEFAULT 0` flags
  (`migrations/001_initial_schema.sql:134`, `:145`) filtered by
  `WHERE published = 1` on their public queries
  (`libs/site-core/models/project.rs:37`, `libs/site-core/models/article.rs:35`).

## Risk profile

Trust-boundary flag raised: this change modifies the **public-exposure boundary** of a
live public site — which rows the public API serves, and which rows the AI context
retains. Resolution deferred to the Frame's risk-profile section.

## Consultations

None. Stage 2a elicitation collapsed under `brownfield-extension` modulation: the
maintainer's 2026-07-23 rulings (D1–D4) plus the 2026-07-24 pre-spec code verification
are the elicitation substrate, transcribed into this run's brief.

## Dismissed review flags

None.
