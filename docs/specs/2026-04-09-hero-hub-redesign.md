# Feature: Hero Hub Redesign (Phase 1)

> **Spec for:** Folio redesign — task #292, branch `hero-redesign`
> **Date:** 2026-04-09
> **Design ref:** `scratch/folio-redesign-design-direction.md`, Stitch project `9166146696467023785`

## Purpose

Replace the traditional nav bar + scrolling hero with a card-based hub layout where the home page IS the navigation. Eliminates mobile nav wrapping issues (P0), adds skeleton loading (P0), and repositions availability status for recruiter visibility (P1).

## Requirements

### Navigation Model
- The home page SHALL NOT render a traditional navigation bar
- The home page SHALL present Projects, Articles, and Contact as interactive cards that link to their respective routes
- The home page SHALL present "Ask AI" as a visually prominent primary CTA card that opens the AI chat pane
- Inner pages (routes other than `/`) SHALL render a minimal header with the PM monogram linking home and a breadcrumb trail showing the current path
- The breadcrumb trail MUST NOT duplicate "PM" — the monogram is the home link, the text trail shows depth (e.g., `Projects / Redline`)
- On mobile viewports (<768px), detail pages SHOULD collapse the breadcrumb to a back link showing the parent page name (e.g., `← Projects`)

### Home Page Layout
- The home page SHALL display a two-column layout on desktop (≥768px): identity block left, content + cards right
- The identity block SHALL contain: name (large, bold), title, and availability badge in that order
- The availability badge SHALL appear immediately below the title, using teal accent color
- The content column SHALL contain: elevator pitch (2-3 sentences), metadata line, Ask AI card, then three navigation cards (Projects, Articles, Contact)
- The Ask AI card SHALL use a filled brass/gold background to distinguish it as the primary action
- The Projects, Articles, and Contact cards SHALL use brass outline borders (secondary treatment)
- The Contact card SHALL contain icon links for GitHub, LinkedIn, Resume, and Email
- On mobile (<768px), the layout SHALL stack vertically: identity → pitch → metadata → Ask AI → nav cards
- The home page SHALL render a SkillsBanner marquee at the bottom of the viewport
- The home page SHALL render a GearsBackground with a diagonal motif separating the identity and content columns

### Skeleton Loading
- The home page SHALL display a skeleton loading state while API data loads
- Skeleton placeholders SHALL use `--raw-bg-surface` (#2a2219) on the base background (#1a1612)
- Skeleton elements SHALL pulse/animate to indicate loading
- The skeleton layout SHALL match the structure of the loaded page (placeholder shapes for name, title, badge, pitch, cards)

### Responsive Behavior
- All interactive elements MUST have a minimum touch target of 44px
- The layout MUST NOT produce horizontal overflow on viewports ≥320px
- Card content MUST remain readable at all supported viewport widths
- The `--nav-height` CSS variable SHALL be removed or set to 0 on the home page (no nav bar consuming viewport space)
- The home page hero SHALL fill the full viewport height (`100svh`) since no nav bar consumes space

### Data Validation
- Contact card link URLs MUST be validated against a protocol whitelist: `https://`, `http://`, `mailto:`, `tel:`. Reject `javascript:`, `data:`, `vbscript:` protocols. Invalid links SHALL be silently omitted.
- If the `links` array is empty or the API fails, the Contact card SHALL still render with a static fallback (e.g., "Contact links unavailable" or email-only fallback)

### Breadcrumb Label Resolution
- Breadcrumb segments SHALL be derived from the URL path, with the first character of each segment capitalized (e.g., `/projects/redline` → "Projects / Redline")
- For `[slug]` route parameters, the breadcrumb label SHALL use the slug value, title-cased. If the page loads entity data with a display name (e.g., project title), the breadcrumb MAY update to the display name after load.
- Breadcrumb text MUST use Svelte text interpolation (`{text}`), never `{@html}`, to prevent XSS from route segments

### Accessibility
- The PM monogram link SHALL have an accessible label (e.g., `aria-label="Home"`)
- Navigation cards SHALL be keyboard-navigable
- The skeleton loading state SHALL include `aria-busy="true"` on the container
- Reduced-motion preferences SHALL disable skeleton pulse animation and any entrance animations
- The breadcrumb nav element MUST have `aria-label="Breadcrumb"`
- The final breadcrumb segment MUST have `aria-current="page"`
- Breadcrumb separators (`/`) MUST have `aria-hidden="true"`

## Scenarios

### Scenario: Home page loads successfully

**Given** the user navigates to `/`
**When** the API returns profile, links, and skills data
**Then** the page displays the identity block with name, title, and availability badge
**And** the content column displays pitch, metadata, Ask AI card, and three nav cards
**And** the SkillsBanner displays at the bottom of the viewport
**And** no traditional nav bar is rendered

### Scenario: Home page loading state

**Given** the user navigates to `/`
**When** the API call is in progress
**Then** the page displays skeleton placeholders matching the hub layout structure
**And** the skeleton container has `aria-busy="true"`

### Scenario: Clicking Ask AI card

**Given** the home page is loaded
**When** the user clicks the Ask AI card
**Then** the AI chat pane opens (same behavior as current Ask AI button)

### Scenario: Clicking a nav card

**Given** the home page is loaded
**When** the user clicks the Projects card
**Then** the browser navigates to `/projects`

### Scenario: Inner page header with breadcrumb

**Given** the user is on `/projects`
**When** the page renders
**Then** the header shows the PM monogram (linking to `/`) and the text "Projects"
**And** no traditional nav links (Home, Projects, Agents, Articles) are shown

### Scenario: Nested inner page breadcrumb

**Given** the user is on `/projects/redline`
**When** the page renders
**Then** the header shows PM monogram, "Projects" (linking to `/projects`), separator, and "Redline"

### Scenario: Mobile breadcrumb collapse

**Given** the user is on `/projects/redline` on a viewport <768px
**When** the page renders
**Then** the header shows PM monogram and "← Projects" (linking to `/projects`)

### Scenario: Mobile home page layout

**Given** the user is on `/` on a viewport <768px
**When** the page loads
**Then** the layout stacks vertically: identity block, pitch, metadata, Ask AI card, nav cards, skills marquee
**And** no content overflows horizontally

### Scenario: Home page API error

**Given** the user navigates to `/`
**When** the API call fails
**Then** the page displays an error message in the identity/pitch region
**And** the nav cards (Projects, Articles, Contact) still render below the error (they are static, not data-dependent)
**And** the Contact card renders with a static fallback if links data is unavailable

### Scenario: Contact card with empty links

**Given** the API returns an empty `links` array
**When** the home page renders
**Then** the Contact card still renders with a fallback message or email-only link
**And** the card maintains its position in the nav card row/stack

### Scenario: Contact card rejects unsafe URLs

**Given** the API returns a link with `url: "javascript:alert(1)"`
**When** the home page renders
**Then** that link is omitted from the Contact card
**And** other valid links still render normally

## UI Behavior

- When the user hovers a nav card on desktop, the card border brightens to brass-light and a subtle background fill appears
- The Ask AI card opens the existing AiPane component (same `aiPaneOpen` toggle mechanism)
- The SkillsBanner component is reused from the current implementation
- The GearsBackground component is reused but repositioned for the diagonal motif (currently top-right, needs to work with the two-column split)
- On mobile, cards are full-width and stacked with consistent spacing
- The skeleton loading state transitions smoothly to the loaded content (no layout shift)

## Constraints

- Prefer reusing existing API endpoints — no backend changes in this phase unless a reviewer identifies a strong reason (e.g., missing data for breadcrumb labels, tag/filter support). If backend changes are warranted, document them as a sub-task rather than deferring silently.
- Must reuse existing `AiPane`, `SkillsBanner`, and `GearsBackground` components where possible (modify, don't rewrite)
- Must preserve the existing `(public)` route group structure
- Must preserve admin routes — no changes to `/admin/*`
- The existing Projects, Articles, and Agents listing page components (`/projects/+page.svelte`, `/articles/+page.svelte`, `/agents/+page.svelte`) receive the new header/breadcrumb but their content is NOT redesigned in this phase
- No new npm dependencies without approval
- All existing CSS custom properties in `app.css` remain the source of truth for design tokens

---

## Amendment: Displaced Section IA (2026-04-13)

**Context:** Phase 1 removed Experience, Education, and Skills sections from the home page per Task 2. The spec did not say where that content should live. This amendment resolves the gap before Phase 1.5 implementation. **This section describes what the user experiences. Implementation structure is left to the implementing agent.**

### User-facing Destination

- Experience, Education, and Skills SHALL be reachable from a single navigation destination that the user perceives as "Resume."
- From the user's perspective, Resume is one place — one card on the hub, one label in the breadcrumb, one reading flow covering skills, work history, and education in that order.
- How this destination is structured at the URL, file, or component level is an implementation decision. The design commitment is that the user never has to navigate between separate destinations to build the full picture.

### Content Order on the Resume Destination

- Skills SHALL appear first.
- Experience SHALL appear after Skills.
- Education SHALL appear after Experience.
- Each of the three SHALL be visually distinct as its own labeled section, scannable in isolation (a user landing near "Experience" should not have to infer where Experience starts).

**Rationale:** The visitor arriving at the Resume destination has already absorbed the identity block on the home hub — they know who Peter is. The click to Resume is a filter action, not a discovery action. The first-pass question is "does this person match what I need" (skills keyword scan), not "who is this person" (already answered). Skills first respects the visitor's time budget and rewards a specific technical checklist. Experience follows as the proof layer — depth and context for the skills that registered. Education sits last as confirming context for a senior candidate on a first-contact web surface; it does not earn prime vertical real estate on a public portfolio. This inverts the PDF-resume convention, which serves a different moment in the hiring funnel.

### Hub Card Treatment

The home page hub SHALL change from four cards to five. The Resume card is added as a secondary (brass outline) card, not a primary CTA.

**Desktop (≥ 768px):** Three visual zones — primary CTA, navigation row, contact block.

```
[        Ask AI — full width, filled brass        ]
[ Projects ]  [ Articles ]  [ Resume ]
[        Contact — full width, brass outline       ]
```

- Ask AI remains the filled brass primary CTA at the top.
- Projects, Articles, and Resume occupy a three-up row of equal-weight secondary cards.
- Contact moves to a full-width card below, visually separated from the navigation row. Contact is functionally different from the nav cards (it is a collection of external links, not an internal route), and the layout SHALL reflect that distinction.

**Mobile (< 768px):** All five cards stack full-width in this order — Ask AI, Projects, Articles, Resume, Contact.

### Naming

- Hub card label: **Resume**
- Page heading on the Resume destination: **Resume**
- Breadcrumb label when the user is on the Resume destination: **Resume**
- The existing "Resume PDF" link inside the Contact card SHALL be labeled **Resume PDF** (not "Resume") so it does not collide with the hub card label.

Rejected alternatives: "About" (overloaded web furniture; doesn't describe the content honestly), "Experience" as a destination name (names only one of three sections).

### Skills Surface

The canonical skills surface SHALL live on the Resume destination under the Skills section. The existing SkillsBanner marquee on the home page SHALL remain as a decorative brand element — it is NOT a replacement for an accessible, scannable skills surface.

**Skills layout requirements:**

- Skills SHALL be grouped by proficiency into three labeled bands: **Strong**, **Moderate**, **Learning**.
- Skills within each band SHALL render as compact pill chips (not a prose list, not a table, not a bar chart, not a tag cloud).
- Pill styling SHALL reuse the existing Ironworks token palette in `app.css`. Strong skills use the strong-border accent treatment; Moderate and Learning use their respective semantic colors already defined in the token set.
- Skill counts and proficiency percentages SHALL NOT be displayed. Category labels are the only proficiency signal.

### Deep Linking

- A user or external site SHALL be able to link directly to the Skills section of the Resume destination and have the browser land scrolled to that section.
- The same applies to Experience and Education sections.
- The specific URL shape used to achieve this is an implementation decision.

### Accessibility

- The Resume destination SHALL be fully accessible — no content hidden behind `aria-hidden`.
- Each of the three sections (Skills, Experience, Education) SHALL have a semantic heading reachable by screen reader section navigation.
- SkillsBanner's decorative marquee status (currently `aria-hidden="true"`) SHALL be preserved — the accessible skills surface lives on the Resume destination, not on the home page.

### Data Requirements and Scope

This amendment is a frontend-only information architecture change. The existing backend endpoints (`getExperience`, `getEducation`, `getSkills` via `libs/site-core/routes/experience.rs`, `education.rs`, `skills.rs`) already satisfy the Resume destination's data needs — no new fields, no new routes, no new shape. No backend changes are required for Phase 1.5. The affected layers are strictly: SvelteKit routes, Svelte components, CSS tokens, and E2E/unit tests.

### Out of Scope for Phase 1.5

Deliberately not committed in this amendment:

- **Section-level UX enhancements** — sticky section nav, active-section highlighting, scroll progress indicators. Phase 2.
- **Timeline visualization for Experience** — Phase 1.5 ships the existing chronological list treatment. Visual timeline is Phase 2.
- **Resume PDF auto-generation** — the Contact card Resume PDF link points at whatever the current static/external target is. Generating a PDF from the live Resume page is separate work.
- **Skills search/filter** — the pill matrix is read-only. Filter input is a Phase 2 enhancement when skill volume justifies it.

---

## Tasks

### Task 1: Header/Breadcrumb component

**Files:** `frontend/src/lib/components/Header.svelte` (new), `frontend/src/lib/components/Nav.svelte` (retire from public layout)
**Type:** frontend
**Depends on:** None

**What:** Create a new `Header` component that renders the PM monogram (home link) and a breadcrumb trail derived from the current route. This replaces `Nav.svelte` in the `(public)` layout for inner pages. The home page renders only the monogram (no breadcrumb text).

**Acceptance Criteria:**
- [ ] PM monogram links to `/` with `aria-label="Home"`
- [ ] Breadcrumb shows path segments after home (e.g., `Projects / Redline`)
- [ ] Each breadcrumb segment except the last is a link to its route
- [ ] On mobile (<768px) for depth >1, breadcrumb collapses to `← {parent}` back link
- [ ] Home page (`/`) renders monogram only, no breadcrumb text
- [ ] Styled with existing Ironworks CSS variables

**Test Expectations (Vitest + @testing-library/svelte unit tests):**
- Breadcrumb renders correct segments for `/projects`, `/projects/redline`, `/articles/my-post`
- Home page renders no breadcrumb text
- Mobile collapse triggers at the correct breakpoint
- All links navigate to correct routes
- ARIA attributes: `aria-label="Breadcrumb"` on nav, `aria-current="page"` on last segment, `aria-hidden="true"` on separators
- PM monogram has `aria-label="Home"`

### Task 2: Home page hub layout

**Files:** `frontend/src/routes/(public)/+page.svelte` (rewrite), `frontend/src/lib/components/Hero.svelte` (rewrite), `frontend/src/lib/components/HeroSkeleton.svelte` (new)
**Type:** frontend
**Depends on:** Task 1

**What:** Rewrite the home page to the Forge & Vision hub layout. Two-column on desktop (identity left, content + cards right), single column on mobile. Remove the current scrolling sections (Experience, Education, Skills sections below the hero) — the home page becomes a focused hub.

**Acceptance Criteria:**
- [ ] Desktop: two-column layout with identity block left, content + cards right
- [ ] Name, title, and teal availability badge render in the identity block
- [ ] Ask AI card has filled brass background, visually dominant
- [ ] Projects, Articles, Contact cards have brass outline borders
- [ ] Contact card contains icon links for GitHub, LinkedIn, Resume, Email (from API `links` data)
- [ ] Elevator pitch and metadata line render in the content column
- [ ] SkillsBanner renders at the bottom of the viewport
- [ ] GearsBackground renders with diagonal motif
- [ ] Mobile: stacks vertically with no horizontal overflow
- [ ] All cards have 44px minimum touch targets
- [ ] Clicking Ask AI opens AiPane
- [ ] Clicking Projects navigates to `/projects`
- [ ] Clicking Articles navigates to `/articles`

**Test Expectations (Vitest unit + Playwright E2E):**
- Home page renders all content sections from API data
- Cards link to correct routes
- Ask AI card toggles the AI pane
- Layout switches from two-column to single-column at 768px breakpoint
- No horizontal overflow at 320px viewport
- Unit test: Contact card omits links with unsafe protocols (`javascript:`, `data:`)
- Unit test: Contact card renders fallback when links array is empty
- Unit test: All card elements have ≥44px computed height

### Task 3: Skeleton loading state

**Files:** `frontend/src/lib/components/HeroSkeleton.svelte` (new), `frontend/src/routes/(public)/+page.svelte` (integrate)
**Type:** frontend
**Depends on:** Task 2

**What:** Create a skeleton loading component that mirrors the hub layout structure. Pulsing placeholder blocks for name, title, badge, pitch, metadata, and card areas.

**Acceptance Criteria:**
- [ ] Skeleton renders matching the hub layout structure (two-column desktop, stacked mobile)
- [ ] Placeholder blocks use `--raw-bg-surface` (#2a2219) color
- [ ] Pulse animation on skeleton elements
- [ ] `aria-busy="true"` on the skeleton container
- [ ] Reduced-motion: pulse animation disabled, static placeholders shown
- [ ] Smooth transition to loaded content with no layout shift
- [ ] Nav cards (Projects, Articles, Contact) render immediately even during loading (they're static)

**Test Expectations:**
- Skeleton appears when profile data is null/loading
- Skeleton disappears when data loads
- Static nav cards render during loading state
- No layout shift on transition

### Task 4: Update public layout to use Header

**Files:** `frontend/src/routes/(public)/+layout.svelte` (modify)
**Type:** frontend
**Depends on:** Task 1

**What:** Replace the `Nav` component import with the new `Header` component in the public layout. The home page will conditionally hide the header (or show monogram only), while inner pages get the breadcrumb treatment. The AiPane toggle mechanism must be preserved.

**Acceptance Criteria:**
- [ ] `Nav` component no longer rendered in the public layout
- [ ] `Header` component renders on inner pages with breadcrumb
- [ ] Home page shows monogram-only header (or no header, with monogram in Hero)
- [ ] AiPane still opens/closes correctly from both the Ask AI card (home) and any future trigger
- [ ] `--nav-height` set to 0 or removed for home page (full viewport hero)
- [ ] Footer still renders on all public pages
- [ ] No changes to admin layout

**Test Expectations:**
- Inner pages render header with correct breadcrumb
- Home page does not show traditional nav
- AiPane toggle works from home page Ask AI card
- Admin routes unaffected

### Task 5: Update E2E tests for new navigation model

**Files:** `e2e/public-pages.spec.ts`, `e2e/responsive.spec.ts`, `e2e/accessibility.spec.ts`, `e2e/ai-chat.spec.ts`
**Type:** frontend (test)
**Depends on:** Tasks 2, 4

**What:** Update all E2E tests that assert on the old nav bar, nav links, and home page sections. The nav bar is gone — tests must assert on the new card-based navigation, breadcrumb header, and hub layout instead. Remove assertions for Experience, Education, and Skills sections on the home page.

**Acceptance Criteria:**
- [ ] All tests referencing `<nav>` element or nav link text updated to assert on card-based navigation or breadcrumb header
- [ ] Tests asserting "Ask AI button in nav" updated to assert on Ask AI card
- [ ] Tests asserting Experience, Education, Skills sections on home page removed or relocated
- [ ] New assertions for: breadcrumb renders on inner pages, cards link to correct routes, Ask AI card opens pane
- [ ] Responsive tests updated: verify mobile stacked layout, breadcrumb collapse at 768px
- [ ] Touch target verification: assert card elements have ≥44px computed height/width
- [ ] Skeleton loading test: mock slow API response, verify skeleton appears before data
- [ ] Reduced-motion test: verify skeleton pulse disabled with `prefers-reduced-motion: reduce`
- [ ] All E2E tests pass with no regressions
- [ ] Tests cover viewports: 320px, 768px, 1440px

**Test Expectations:**
- Full E2E suite passes against the new navigation model
- No tests reference the removed Nav component
- Responsive breakpoint boundaries verified (767px vs 768px)
- Skeleton → loaded transition verified with network throttling

### Post-implementation: Dead code audit

After all tasks complete, verify that removed home page component imports (ExperienceSection, EducationSection, SkillsSection) are either:
- Still used on other routes (keep them)
- Unused anywhere (remove the imports, keep the component files for Phase 2-3)
