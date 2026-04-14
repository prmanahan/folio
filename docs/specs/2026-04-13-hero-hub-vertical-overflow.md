# Fix: Hero hub vertical overflow on long profile content

> **Spec for:** task #382 — Folio hero hub vertical-overflow fix + content length split + bounds testing
> **Source design:** `scratch/pixel-folio-hero-vertical-review.md` (Pixel, 2026-04-13, Option B chosen)
> **Date:** 2026-04-13
> **Severity:** high (live on production)
> **Target:** `/Users/manahan/claude_workspace/projects/folio`
> **Assigned:** Rune (Sonnet)
> **Review pipeline:** Warden → Glitch → 90% coverage gate → merge

---

## Bug

**Observed:** On the deployed hero hub (`peter.manahan.io`), when `profile.elevator_pitch` is long (~700 chars, single paragraph), the `.hero` section grows past the viewport. The skills ticker and footer are pushed below the fold because `.skills-banner` is `position: absolute; bottom: 0` *inside* `.hero` — it is anchored to the hero box, not the viewport. Nothing is hidden; the page just scrolls to reveal the overflowed content.

**Expected:** The hub fits within one small-viewport height (`100svh` minus nav) on every supported breakpoint. The skills ticker and footer sit at predictable positions regardless of profile content length. Long-form biographical content is routed to `/resume`, not crammed into the hub.

**Impact:** Peter's real profile triggers the bug on iPhone SE (~2.1× over budget), iPhone 14, iPad landscape, and 13" laptops. It does not reproduce on 16" or 27" displays, which is why the Alex Rivera seed fixture missed it. The hub's "frozen tableau" visual effect is compromised whenever the pitch exceeds ~4 lines of wrapped text.

---

## Reproduction

**Given** a profile where `elevator_pitch` is a single paragraph of ≥500 characters
**And** the viewport height is ≤900px (e.g. iPhone SE 667px, 13" laptop landscape ~800px)
**When** the user loads `/` (the hero hub)
**Then** the `.hero` section is taller than `100svh`
**And** `.skills-banner` is positioned below the initial visible viewport
**And** the user must scroll to see the ticker and the footer

---

## Fix Requirements

### Layout

- R1. The `.hero` section SHALL have a hard ceiling of `100svh - nav-height`, not a `min-height`. Its height MUST NOT grow past that ceiling regardless of content length.
- R2. The `.skills-banner` element SHALL be rendered as a DOM sibling of `.hero`, not as a descendant. It MUST NOT use `position: absolute` to anchor itself to the hero box. Its position on the page is determined by document flow relative to `.hero`.
- R3. The choice of viewport unit SHALL be `svh` (small viewport height). `dvh` and `lvh` are explicitly rejected — `svh` is preferred because it does not reflow the layout as the mobile browser chrome collapses/expands on scroll.
- R4. On every breakpoint in the test matrix (see §Test Expectations), the hub SHALL render without page scroll being required to reveal the ticker.

### Data model — pitch split

- R5. The `profile` schema SHALL expose two distinct pitch fields: `pitch_short` and `pitch_long`. The existing `elevator_pitch` column is superseded and MUST be migrated.
- R6. `pitch_short` MUST be enforced to ≤280 characters at the API/schema layer (not only in CSS or the admin UI). Validation failures MUST return a clear error, not silently truncate.
- R7. `pitch_long` MUST be enforced to ≤1500 characters at the same layer, same error semantics.
- R8. The hub (`Hero.svelte`) SHALL render `pitch_short`. The hub MUST NOT fall back to `pitch_long` if `pitch_short` is missing; missing `pitch_short` is a validation error upstream, not a rendering concern.
- R9. The `/resume` route SHALL render `pitch_long` where the full-form biographical content currently lives. If `/resume` previously consumed `elevator_pitch`, that consumption MUST be updated in the same change.
- R10. There SHALL be no "Read more" affordance on the hub. The hub renders `pitch_short` in full; the long form lives exclusively on `/resume`.

### Content length limits (API/schema enforcement)

- R11. The following fields SHALL have hard maximum lengths enforced at the validation layer. Admin UI MUST reflect the same limits with `maxlength` attributes and a visible character counter for `pitch_short` and `pitch_long`.

  | Field | Max chars |
  |---|---|
  | `name` | 32 |
  | `title` | 48 |
  | `pitch_short` | 280 |
  | `pitch_long` | 1500 |
  | `location` | 48 |
  | `remote_preference` | 64 |
  | `availability_status` | 32 |
  | `links[].label` | 16 |

- R12. The number of `links` rendered on the hub SHALL be capped at 5. Additional links remain in the data model but are only rendered on `/resume`. This is a render-time rule, not a validation rule — storing more than 5 links MUST remain allowed.

### Bounds-testing fixture

- R13. A new seed fixture SHALL be added that mirrors Peter's real profile data pulled from the live `/api/profile` endpoint on `peter.manahan.io`. The fixture MUST live under `libs/site-core/db/seed.rs` (or an equivalent fixture file) and MUST be selectable via environment variable or feature flag so the existing Alex Rivera seed remains the default.
- R14. The pull SHOULD be a one-time manual capture committed as static data — NOT a runtime fetch from the live site during build or test. The spec agent (Rune) MUST use `curl` or equivalent to capture the JSON once, save it to the fixture, and commit the static copy.
- R15. The live-profile fixture MUST be used in the bounds tests called out below. It SHOULD NOT replace the Alex Rivera fixture in unrelated tests.

---

## Acceptance Criteria

- [ ] The bug no longer reproduces: on all viewports in the test matrix, the hero hub with Peter's real profile fixture renders without vertical overflow, and the skills ticker is visible without scrolling.
- [ ] `profile.pitch_short` and `profile.pitch_long` exist as distinct columns in the schema; the migration is forward-only (no rollback required), and existing data is migrated according to the strategy in §Migration.
- [ ] `elevator_pitch` is removed from the `profile` model, API responses, types (`frontend/src/lib/types.ts`, `admin-types.ts`), admin UI, and every consumer identified by grep. No dangling references remain.
- [ ] Length limits in R11 are enforced server-side. Attempting to POST a profile with an overlength field returns a structured validation error with the offending field name and the limit.
- [ ] Admin UI shows live character counters for `pitch_short` and `pitch_long` and blocks submission when over limit.
- [ ] The `.skills-banner` element is a DOM sibling of `.hero` in `Hero.svelte` (or the enclosing layout); visual inspection shows the ticker at the bottom of the visible viewport on the hub page on every breakpoint tested.
- [ ] The live-profile fixture exists and can be loaded via documented mechanism (env var or feature flag); loading it produces a working hub without overflow.
- [ ] All existing tests still pass. New tests listed in §Test Expectations pass.
- [ ] 90% line coverage gate holds on the files touched (existing repo standard).

---

## Migration

The existing `elevator_pitch` column is Peter's 700-char pitch. The migration:

1. Add new columns `pitch_short TEXT NOT NULL DEFAULT ''` and `pitch_long TEXT NOT NULL DEFAULT ''` to `profile`.
2. For existing rows: copy `elevator_pitch` into `pitch_long` verbatim; leave `pitch_short` empty (Peter will supply the ≤280-char version as content work after the migration lands).
3. Drop `elevator_pitch`.
4. Seed data (Alex Rivera) MUST be updated to supply both fields inline rather than relying on migration. Rune writes an appropriate short pitch for the Alex Rivera seed that fits within 280 chars.
5. Temporarily allow empty `pitch_short` for seed purposes OR ship the migration paired with Peter's content update — Rune's call, document the choice. **Preferred:** enforce NOT NULL and `LENGTH(pitch_short) > 0` at the app/validation layer, not the DB, so Peter can land content changes without a second migration.

---

## Test Expectations

### Regression (the bug itself)

- A frontend component test for `Hero.svelte` SHALL render the hero with the live-profile fixture at the following simulated viewports and assert that the hero's computed height does not exceed the viewport height:
  - 375×667 (iPhone SE)
  - 390×844 (iPhone 14)
  - 820×1180 (iPad portrait)
  - 1180×820 (iPad landscape)
  - 1440×900 (13" MBP)
  - 1728×1117 (16" MBP)
  - 2560×1440 (27" monitor)
- A Playwright E2E test SHALL load the hub with the live-profile fixture and assert that the `.skills-banner` element's bounding rect sits within the initial viewport (no scroll required).

### Bounds

- `pitch_short` exactly 280 chars: renders without clipping or overflow.
- `pitch_short` at 281 chars: API returns validation error; admin UI blocks submission.
- `pitch_short` empty: API returns validation error (R8).
- `pitch_long` at 1500/1501 chars: same boundary behavior.
- `name`, `title`, `location`, `remote_preference`, `availability_status`, `links[].label` at their respective max+1: API rejects with field-specific error.
- Profile with 6 links: hub renders exactly 5; `/resume` renders all 6.
- Profile with 0 links: hub renders without empty-pill artifacts.
- Empty `pitch_long`: `/resume` renders the page without a broken bio section.

### Structural

- Unit test verifying `SkillsBanner` is rendered as a sibling of the hero, not a descendant (component tree assertion).
- Snapshot/DOM test confirming `.hero` has no `min-height` rule and uses `100svh`-based constraint.

### Do not test

- Visual pixel-regression screenshots are out of scope for this task — those belong in the broader #101 work. This task focuses on measurable layout constraints, not pixel-diff.

---

## Files

Not a directive — a map of expected touch points. Rune may add or skip files as needed.

**Backend (Rust):**
- `migrations/004_profile_pitch_split.sql` (new) — add `pitch_short`, `pitch_long`; drop `elevator_pitch`.
- `libs/site-core/models/profile.rs` — field rename/split, length validators.
- `libs/site-core/routes/profile.rs` — response shape.
- `libs/site-core/routes/admin/profile.rs` — validation, error responses.
- `libs/site-core/ai/context.rs` — swap `elevator_pitch` consumer to appropriate new field (`pitch_long` likely, since AI context benefits from richer text).
- `libs/site-core/db/seed.rs` — Alex Rivera updated with both pitch fields; live-profile fixture added.
- `libs/site-core/tests/test_profile.rs` — bounds tests for validators.

**Frontend (Svelte/TS):**
- `frontend/src/lib/types.ts` — type updates.
- `frontend/src/lib/admin-types.ts` — admin type updates.
- `frontend/src/lib/components/Hero.svelte` — remove `min-height`, add `100svh` ceiling, restructure so `SkillsBanner` is a sibling (may require moving the banner mount point into the parent layout).
- `frontend/src/lib/components/SkillsBanner.svelte` — remove absolute-positioning coupling if it lives in the component itself.
- `frontend/src/routes/admin/profile/+page.svelte` — split pitch inputs, char counters, maxlength.
- `frontend/src/routes/+page.svelte` (or wherever the landing layout lives) — host `SkillsBanner` as sibling of `<Hero />`.
- `frontend/src/routes/resume/+page.svelte` (wherever /resume lives) — switch to `pitch_long`.
- `frontend/src/lib/__tests__/Hero.test.ts` — regression + bounds tests.
- `frontend/e2e/*.spec.ts` — update any fixture expectations; add hub overflow E2E.
- `e2e/fixtures.ts` — update if pitch field references exist.
- `e2e/resume.spec.ts` — update.

**Ref only:** `scratch/pixel-folio-hero-vertical-review.md` — design rationale.

---

## Open risks / pre-dispatch notes

1. **Worktree conflict surface.** Three active worktrees touch files this spec needs: `feat/backend-coverage` (profile.rs, context.rs, seed.rs), `feat/frontend-coverage` (types.ts, Hero.svelte), `fix/workflow-permissions` (Hero.svelte, e2e). Puck MUST decide before dispatch whether Rune waits for these to merge or rebases after. This is a Peter decision, not a Rune decision.
2. **Content work is Peter's.** Rune writes the Alex Rivera short pitch for seeds. Peter writes his own `pitch_short` and the updated `pitch_long` as content work after the migration lands.
3. **`SkillsBanner` restructure may ripple.** Moving the banner out of `.hero` could affect any other place that relies on its current positioning. Rune greps for `SkillsBanner` usage before restructuring.
4. **AI context field choice.** `ai/context.rs` currently feeds `elevator_pitch` into the AI pane. Rune picks `pitch_long` as the replacement (richer context) unless there's a reason not to; document the choice in the PR.
