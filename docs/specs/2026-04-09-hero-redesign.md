# Hero Redesign Spec — Ironworks Evolution
**Spec ID:** 2026-04-09-hero-redesign
**Status:** Draft — awaiting Peter review
**Author:** Pixel
**Dispatch:** #159
**Task:** #295
**Date:** 2026-04-09

---

## 1. Design Rationale

### Who This Is For

Primary user: a hiring manager or technical leader with 90 seconds. They landed from a LinkedIn profile, a referral, or a job application. They scan left-to-right, top-to-bottom, and they're making a gut decision: does this person have the taste and craft I'd trust to run a technical program?

Secondary user: a developer or architect who found the site organically. They'll spend longer, they'll look at source, and they'll notice if the design is generic.

### What "Success" Looks Like

Within 10 seconds, both users should know: senior engineer, opinionated craftsman, not a template. Within 30 seconds they should have a credible first impression — location, availability, the kind of work he does.

### Why Gears and a Skills Banner

The gear animation isn't decoration — it's the Ironworks proof point. It says: "I built this." A subtle mechanical system running in the background signals craftsmanship without boasting. The 15–20 second runtime mimics machinery warming up; the freeze (not fade) signals mechanical precision. Gears don't gracefully dissolve — they stop.

The skills banner is a conveyor belt, a literal workshop metaphor. It keeps the eye moving in the direction of the page (left-to-right on desktop, continuous on mobile) and surfaces technical identity at a glance. Cinzel small-caps on a brass-tinted strip reinforces the brand.

### What Stays From the Current Hero

- Cinzel name at gold (`--color-gold`), scaled up
- Job title and elevator pitch paragraphs
- Meta line: location, availability dot, remote preference
- Brass ghost link buttons
- Brass gradient rule (integrated into skills banner bottom)

### What Changes

- Hero becomes full viewport (`100vh`) instead of padded section
- Background gains 3-tier layered depth + subtle noise texture
- Animated gear assembly in background (visible but non-distracting)
- Name gets letter-stamp entrance animation on load
- Skills banner replaces the static `<SkillsSection>` ticker (or supplements it)
- Content vertically centered in the hero viewport

---

## 2. Hero Layout

### Decision: Full Viewport (100vh)

Full viewport makes the first-impression deliberate. Visitors see exactly one thing before they scroll: who Peter is and what the Ironworks identity communicates. This is standard for "Opinionated Craftsman" portfolios that compete on identity.

Tradeoff: at small viewport heights (e.g., a 13" laptop in browser chrome), the content must still fit without overflow. Spec uses `min-height: 100vh` with `overflow: hidden` on the background layer and `overflow: visible` on content so long elevator pitches don't clip.

### Overall Structure

```
┌─────────────────────────────────────────────────────────────┐
│  NAV (sticky, --nav-height: 3.5rem)                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  [GEAR LAYER — absolute, full bleed, z-index: 0]           │
│  [NOISE TEXTURE OVERLAY — absolute, full bleed, z-index: 1]│
│                                                             │
│  [CONTENT — relative, z-index: 2]                          │
│                                                             │
│    Peter Manahan          ← h1, left-aligned               │
│    Principal Architect    ← .title                         │
│                                                             │
│    [elevator pitch]       ← max-width: 52ch                │
│                                                             │
│    ● Open · San Francisco · Remote-first                   │
│                                                             │
│    [GitHub] [LinkedIn] [Resume]                            │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  [SKILLS BANNER — bottom of hero, full bleed]              │
└─────────────────────────────────────────────────────────────┘
```

### Breakpoint Layouts

#### 375px (Mobile)

```
min-height: 100svh  /* svh avoids mobile browser chrome jumpiness */
padding-top: calc(var(--nav-height) + 2rem)
padding-bottom: 5rem  /* space for skills banner */
padding-left/right: 1.25rem

Content: single column, left-aligned
h1: clamp(2.25rem, 8vw, 3rem), Cinzel 700
.title: 1rem, Lato 400, --color-text-muted
pitch paragraphs: 0.9375rem, max-width: none (full column)
meta: 0.8125rem, flex-wrap: wrap, gap: 0.625rem
links: flex-wrap: wrap, gap: 0.5rem, full-width eligible

Gear layer: simplified — 2 gears only, positioned top-right corner
  opacity: 0.06 (more subtle on small screens — less visual noise)
```

#### 768px (Tablet)

```
min-height: 100svh
padding-top: calc(var(--nav-height) + 3rem)
padding-bottom: 5.5rem
padding-left/right: 2rem

h1: clamp(2.5rem, 5vw, 3.5rem), Cinzel 700
.title: 1.125rem
pitch paragraphs: 1rem, max-width: 52ch
Gear layer: 3 gears, positioned top-right quadrant
  opacity: 0.08
```

#### 1024px (Desktop)

```
min-height: 100vh
padding-top: calc(var(--nav-height) + 4rem)
padding-bottom: 6rem
max-width: var(--max-width) [960px], centered

h1: clamp(2.75rem, 4vw, 3.75rem), Cinzel 700
.title: 1.25rem
pitch paragraphs: 1rem, max-width: 52ch
Gear layer: full assembly — 4–5 interlocking gears, top-right
  opacity: 0.10
```

#### 1440px (Wide Desktop)

```
min-height: 100vh
max-width: var(--max-width) [960px], centered
Content layout unchanged — max-width contains it

h1: 3.75rem (clamp ceiling)
Gear layer: 5 gears, scaled 1.2x at this breakpoint, extend further right
  opacity: 0.10
Note: gears bleed past the content max-width into the right margin — intentional
```

---

## 3. Gears Animation Concept

### Approach: Inline SVG + CSS Animation

**Not canvas, not a library, not a video.**

SVG gives clean vector scaling, DOM accessibility (`aria-hidden`), and trivial `prefers-reduced-motion` handling via a single CSS media query. Canvas would require JavaScript for the stop behavior and motion detection. A CSS-only approach means zero JavaScript, zero dependencies, zero runtime errors.

Implementation structure:

```svelte
<!-- GearsBackground.svelte -->
<div class="gears-bg" aria-hidden="true" role="presentation">
  <svg class="gears-svg" viewBox="0 0 600 500" xmlns="http://www.w3.org/2000/svg">
    <!-- Gear A: large, 80px radius, 12 teeth -->
    <g class="gear gear-a" style="transform-origin: 300px 200px">
      <use href="#gear-large" />
    </g>
    <!-- Gear B: medium, 50px radius, 8 teeth, meshes with A -->
    <g class="gear gear-b" style="transform-origin: 420px 150px">
      <use href="#gear-medium" />
    </g>
    <!-- Gear C: small, 32px radius, 5 teeth, meshes with B -->
    <g class="gear gear-c" style="transform-origin: 490px 210px">
      <use href="#gear-small" />
    </g>
    <!-- Additional decorative: chain link connector, pipe segments -->
    <defs>
      <!-- Gear tooth paths defined here as reusable symbols -->
    </defs>
  </svg>
</div>
```

### Gear Shapes

Industrial gears, not cartoon gears. Use the standard involute tooth profile approximation: rectangular teeth with slightly chamfered corners (2px radius via `rx` attribute). 8–14 teeth per gear depending on radius. Center hole (axle) as a filled circle in `--raw-bg-deeper`. No decorative spoke cutouts — industrial solid disc with teeth.

Color: `--raw-brass-dark` (`#8a6a35`) at 60% opacity for primary gear bodies. Tooth highlights at `--raw-brass` (`#b08d57`) at 30% opacity. This keeps them visible against the dark background without competing with the content.

### Timing and Motion

Gear rotation speeds must be physically correct — teeth interlock means angular velocities are inversely proportional to radius.

```
Gear A (large, r=80): 1 full rotation / 20s → -18deg/s (counterclockwise)
Gear B (medium, r=50): (80/50) * 20s = 12.5s per rotation (clockwise)
Gear C (small, r=32): (80/32) * 20s = 8s per rotation (counterclockwise)
```

Easing: `linear` for continuous rotation. Gears rotate at constant angular velocity — they don't ease in or out. The mechanical feel comes from the physics-correct speed relationships, not from easing curves.

### The 15–20 Second Stop Behavior

This is the design's most distinctive moment. The animation runs for 15–20 seconds after page load, then freezes.

**Implementation:**

```css
.gear {
  animation-play-state: running;
  animation-timing-function: linear;
  animation-fill-mode: forwards;
}

.gears-bg.stopped .gear {
  animation-play-state: paused;
}
```

```javascript
// In GearsBackground.svelte
import { onMount } from 'svelte';

let stopped = $state(false);

onMount(() => {
  const timer = setTimeout(() => {
    stopped = true;
  }, 17000); // 17 seconds — midpoint of 15-20s range
  
  return () => clearTimeout(timer);
});
```

The class toggle `stopped` sets `animation-play-state: paused` on all gears simultaneously. Because `paused` freezes at the current keyframe, the gears halt instantly — no easing, no fade, no graceful exit. It looks like a machine stopping.

**Why not `animation-iteration-count: 1` with a very long duration?** That approach would animate from 0° to 360° exactly once, but the stop position would be predetermined (always at 360°). The `paused` approach freezes wherever the gear is when the timer fires — which is correct. A real machine doesn't always stop at the same position.

**Why 17 seconds?** Long enough to be noticed, short enough to not become annoying for users who land and immediately start scrolling. The first 3–5 seconds are the critical impression window — gears are already moving when eyes land.

### Gear Positioning

Gears occupy the top-right quadrant of the hero, bleeding to the viewport edge. They are NOT centered behind the content — they flank it, reinforcing the left-aligned text layout.

At 375px: gears visible in top-right corner only, clipped by viewport.
At 768px+: gear assembly extends from top-right, largest gear partially visible at the far right.
At 1440px: gears have room to breathe — the assembly sits comfortably in the margin right of the content max-width.

Z-index: `0` (behind everything). Content is `z-index: 2`.

### Decorative Supporting Elements

Beyond the rotating gears, add static (non-animated) industrial elements for depth:

- **Pipe segments:** Horizontal and L-shaped pipe silhouettes connecting gear positions. `stroke: --raw-brass-dark`, `stroke-width: 4`, `opacity: 0.12`
- **Rivets:** Small circles (`r=3`) at pipe joints. Same opacity as pipes
- **Pressure gauge circle:** Large partial circle arc (not a full gear) in the lower-right of the SVG. Suggests a gauge face. `opacity: 0.06`

These provide scale reference without requiring animation.

---

## 4. Skills Banner

### Concept: The Conveyor Belt

A horizontal strip anchored to the bottom of the hero. It scrolls continuously, looping its content. Cinzel small-caps on a brass-tinted background. The metaphor: raw materials moving through the workshop.

### Layout

```
Position: at the bottom of the hero section (not fixed to viewport)
Height: 2.75rem (44px minimum touch target, good visual weight)
Width: 100vw (full bleed, bleeds past container max-width)
Background: rgba(176, 141, 87, 0.10) with a top border of 1px solid rgba(176, 141, 87, 0.25)
```

The strip is full-width — it breaks out of the `container` max-width intentionally. The industrial conveyor runs wall-to-wall.

### Scroll Behavior

Infinite CSS marquee using the `translate` transform approach (not `margin-left`, which triggers layout recalculation):

```css
.banner-track {
  display: flex;
  width: max-content;
  animation: conveyor 40s linear infinite;
}

@keyframes conveyor {
  from { transform: translateX(0); }
  to   { transform: translateX(-50%); }
}
```

The trick: duplicate the skills list twice in the DOM. The track is twice the visual width. At -50% translation, the second copy has slid exactly to where the first started — seamless loop.

Speed: 40s for a full pass at desktop. Adjust for mobile: `40s * (375/1440)` = ~10.4s, but this feels too fast. Use a fixed character-based speed instead: approximately `0.8rem per second`. At 375px with ~30 items × ~8rem each = 240rem total ÷ 0.8 = 300s. In practice, use `60s` on mobile and `40s` on desktop — validated by feel, not math.

**Pause on hover (desktop only):**

```css
.banner-track:hover {
  animation-play-state: paused;
}
```

This lets users read a skill name without it scrolling away.

### Typography

```
Font: Cinzel, 600 weight (not 400 — thins out on dark)
Size: 0.6875rem (11px) — small enough for a tag strip, large enough to read
Text transform: uppercase
Letter-spacing: 0.14em (generous — small-caps at this size need air)
Color: --color-text-muted (#a09080)
```

**Separator between items:** A small brass diamond `◆` at 50% opacity, or a vertical pipe `|` in brass. The diamond fits the industrial aesthetic better — it evokes a rivet or bolt head.

### Item Structure

Each skill item:

```
[separator] SKILL NAME [separator] SKILL NAME ...
```

Example content (pulled from existing skills data):
```
◆ Rust ◆ SvelteKit ◆ PostgreSQL ◆ Distributed Systems ◆ Technical Leadership ◆ API Design ◆ Docker ◆ System Architecture ◆ Axum ◆ TypeScript ◆ Python ◆ Team Building ◆ Technical Strategy
```

Items are all skills, flat — no category grouping. The banner is about breadth and quick scanning, not the strong/moderate/gap hierarchy (that's the SkillsSection's job).

### Integration with Existing SkillsSection

The banner and `<SkillsSection>` serve different purposes:

- **Banner:** Quick, always-visible, brand-reinforcing. Shows breadth.
- **SkillsSection:** Structured, categorized, shows depth and recency.

They coexist. The banner is part of the hero; `<SkillsSection>` remains below the fold as a dedicated section.

---

## 5. Content Hierarchy

### Visual Reading Order (Mobile First)

1. **Name** (gold, large, Cinzel 700) — immediate personal identification
2. **Title** (muted, Lato 400) — context: what kind of person
3. **Elevator pitch** (primary text, Lato 400) — why this person matters
4. **Meta line** (ghost text, smaller) — logistics: where, status, remote
5. **Links** (brass ghost buttons) — actions
6. **Skills banner** (strip, bottom of viewport) — reinforces technical identity

### Name Entrance Animation

On page load, the name stamps in — letters appear left-to-right with staggered delay:

```css
h1 .char {
  opacity: 0;
  transform: translateY(4px) scaleY(1.05);
  animation: stamp-in 400ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
  animation-delay: calc(var(--char-index) * 40ms);
}

@keyframes stamp-in {
  to {
    opacity: 1;
    transform: translateY(0) scaleY(1);
  }
}
```

The `scaleY(1.05)` gives a slight press-into-surface feel (type being pressed onto paper). Full name "Peter Manahan" is 13 characters × 40ms = 520ms total stagger. The animation completes within 920ms (520ms + 400ms). Clean.

Rune implementation note: Cinzel doesn't support `SplitText` directly — wrap each character in a `<span class="char" style="--char-index: {i}">` in the Svelte template using a derived store or computed array.

### Title and Pitch Fade-In

Title and pitch paragraphs fade in after the name completes:

```
Title: opacity 0 → 1, translateY(6px) → 0, delay: 600ms, duration: 400ms
Pitch: opacity 0 → 1, delay: 800ms, duration: 500ms
Meta + Links: opacity 0 → 1, delay: 1000ms, duration: 400ms
```

All entrance animations use `cubic-bezier(0.25, 1, 0.5, 1)` (ease-out-quart). Nothing bounces.

### Existing Content Changes

**What stays unchanged:**
- Elevator pitch content (the text itself — this is Peter's voice, not ours to redesign)
- Meta line structure (location, status dot, remote preference)
- Link button structure and labels

**What changes:**
- `h1` font-weight: 900 → 700 (900 is too heavy on dark at large sizes; 700 has cleaner counters in Cinzel)
- `h1` letter-spacing: 0.08em → 0.06em (less exaggerated at larger sizes)
- `.title` font-size: 1.175rem → context-fluid (see breakpoint specs above)
- `.pitch` max-width: 640px → 52ch (character-based measure is correct for readability)
- `.pitch` line-height: 1.7 → 1.75 (slightly more air on dark background, per typography guidance)
- Hero `padding` → replaced by `min-height: 100svh` with flexbox vertical centering

---

## 6. Responsive Behavior

### 375px — Mobile

- Single column, full width
- Gear assembly: 2 gears, top-right corner, `opacity: 0.06`
- Skills banner: `height: 2.5rem`, font-size `0.625rem`, scroll speed `60s`
- Name: `font-size: clamp(2.25rem, 8vw, 2.75rem)`
- Pitch: no `max-width` constraint (full column width)
- Links: `flex-wrap: wrap` — each button may take full width if label is long
- Meta: `flex-wrap: wrap`, `gap: 0.5rem 0.875rem`
- Gear entrance animation: gears start immediately (same timing as desktop)
- Content padding: `pt: calc(3.5rem + 2rem)`, `pb: 5rem`

### 768px — Tablet

- Single column, `padding: 0 2rem`
- Gear assembly: 3 gears, visible in top-right, `opacity: 0.08`
- Skills banner: `height: 2.75rem`, font-size `0.6875rem`, scroll speed `45s`
- Name: `clamp(2.5rem, 5vw, 3.25rem)`
- Pitch: `max-width: 52ch`
- Links row fits in one line for typical link count (3 buttons)

### 1024px — Desktop

- Content centered within `max-width: 960px`
- Gear assembly: 4–5 gears, top-right quadrant, `opacity: 0.10`
- Skills banner: `height: 2.75rem`, scroll speed `40s`, hover-pause enabled
- Name: `clamp(2.75rem, 4vw, 3.75rem)`

### 1440px — Wide

- Content unchanged from 1024px (contained by `max-width: 960px`)
- Gear assembly: 5 gears, scaled `1.2×`, bleed into right viewport margin
- Banner remains 100vw — no change

---

## 7. States

### Default State (Page Load)

- Gears layer: hidden (`opacity: 0`)
- Content: hidden (`opacity: 0`)
- Skills banner: visible but `animation-play-state: paused` (no scroll until ready)

Page load sequence (orchestrated in `GearsBackground.svelte` + `Hero.svelte`):

```
0ms    → page visible, background color shows
100ms  → gear layer fades in (opacity 0 → gears' base opacity, 300ms)
200ms  → name stamp animation begins
600ms  → title fades in
800ms  → pitch fades in
1000ms → meta + links fade in
1200ms → skills banner starts scrolling
```

### Animation Running (0–17s)

- Gears rotating at correct relative speeds
- Skills banner scrolling
- All content visible
- No interaction required from user

### Animation Stopped (17s+)

- `animation-play-state: paused` on all gear elements
- Gears frozen in place — no visual indication this happened (no fade, no pulse)
- Skills banner continues scrolling (it's not part of the "machinery startup" metaphor)
- From this point, page behaves like a standard static hero

### Reduced Motion State

When `prefers-reduced-motion: reduce` is set:

```css
@media (prefers-reduced-motion: reduce) {
  .gear { animation: none; }
  .banner-track { animation: none; }
  h1 .char {
    opacity: 1;
    transform: none;
    animation: none;
  }
  .hero-content > * {
    opacity: 1;
    transform: none;
    animation: none;
  }
}
```

- Gears: static, no rotation. Still visible at base opacity (they're part of the background composition).
- Skills banner: static, overflows with `overflow: hidden`. First set of skills visible at the left edge. No scrolling.
- Name: visible immediately, no stamp effect
- Content: visible immediately, no fade-in sequence

The page is fully usable — no information is hidden behind animation.

### Focus State

Link buttons use the global `:focus-visible` ring: `outline: 2px solid var(--raw-brass-light)`, `outline-offset: 2px`. This is already defined in `app.css`.

Tab order: Nav → h1 (non-interactive, skip) → link buttons → first section below fold.

---

## 8. Accessibility

### Contrast Ratios

| Element | Color | Background | Ratio | WCAG |
|---------|-------|-----------|-------|------|
| h1 (name) | `#e8c97a` (--color-gold) | `#1a1612` | ~8.3:1 | AAA |
| .title | `#a09080` (--color-text-muted) | `#1a1612` | ~4.6:1 | AA |
| .pitch | `#e8e0d0` (--color-text) | `#1a1612` | ~12.1:1 | AAA |
| .meta | `#a09080` (--color-text-ghost) | `#1a1612` | ~4.6:1 | AA |
| Link text | `#a09080` (--color-text-muted) | transparent | via button border ~4.6:1 | AA |
| Banner text | `#a09080` | `rgba(176,141,87,0.10)` blended ≈ `#1e1b14` | ~4.5:1 | AA (borderline — verify) |

**Note on banner contrast:** The `rgba(176,141,87,0.10)` background blended over `#1a1612` produces approximately `#1e1b14`. The `#a09080` text against `#1e1b14` needs verification with a contrast tool before shipping. If it fails, use `--raw-text-secondary` (`#c8b898`) for banner text instead (yields ~7.1:1).

### Touch Targets

- Link buttons: `min-height: 44px` (already set in current styles — keep)
- Skills banner: not interactive (no touch targets required)
- Meta items: non-interactive text

### Motion Sensitivity

- `prefers-reduced-motion: reduce` disables all animation (see State 4 above)
- The global `app.css` rule already covers this (`animation: none !important`) — the redesign must NOT override this global rule
- Rune should test with Chrome DevTools → Rendering → Emulate prefers-reduced-motion

### Reading Order

Screen reader order matches visual order:

1. h1: "Peter Manahan" (no split-character span confusion — use `aria-label="Peter Manahan"` on the h1, with the char spans marked `aria-hidden="true"`)
2. `.title`: job title
3. `.pitch`: paragraph content
4. `.meta`: location, status, remote preference
5. `.links`: link buttons with descriptive labels
6. `.gears-bg`: `aria-hidden="true"` (decorative)
7. `.skills-banner`: `aria-hidden="true"` (duplicate of SkillsSection content — don't double-announce)

### Zoom

Content must be readable at 200% zoom. No fixed-height containers that clip text. The `52ch` max-width for pitch paragraphs will naturally reflow at zoom. Test at 200% in Firefox (most aggressive zoom implementation).

---

## 9. Implementation Notes for Rune

### Technology Choice: Inline SVG + CSS

Use inline SVG for the gear assembly. Not a separate SVG file, not `<img>` — inline, so CSS classes can be applied directly to `<g>` elements.

The SVG should live in `GearsBackground.svelte`. Import it as a Svelte component (not a static asset) so the JS timer that triggers `animation-play-state: paused` has direct DOM access.

### Gear SVG Construction

Build reusable gear symbols in `<defs>`:

```svg
<defs>
  <symbol id="gear-lg" viewBox="-90 -90 180 180">
    <!-- 12 rectangular teeth around a central disc -->
    <!-- Each tooth: rect, width 14, height 28, positioned at radius 76, rotated -->
    <!-- Central disc: circle r=62 -->
    <!-- Axle hole: circle r=12, fill: var(--raw-bg-deeper) -->
  </symbol>
</defs>
```

Tooth count formula: `N = floor(2π × r / tooth_pitch)`. Use `tooth_pitch ≈ 20px` for aesthetic spacing. At `r=80`: N=25 (too many — use 12 for readability at this visual scale). At `r=50`: N=8. At `r=32`: N=5.

**Rune's task:** Create 3 gear symbols (large/medium/small). Position them so teeth interlock visually (gap between gear centers = sum of pitch radii). The exact tooth-to-tooth mesh doesn't need to be mathematically perfect — visually plausible is sufficient for a background element.

### Animation Stop: CSS Class Toggle

```svelte
<script lang="ts">
  import { onMount } from 'svelte';

  let stopped = $state(false);

  onMount(() => {
    const STOP_DELAY_MS = 17_000;
    const id = setTimeout(() => { stopped = true; }, STOP_DELAY_MS);
    return () => clearTimeout(id);
  });
</script>

<div class="gears-bg" class:stopped aria-hidden="true" role="presentation">
  <!-- SVG inline -->
</div>
```

```css
.gear-a { animation: rotate-ccw 20s linear infinite; }
.gear-b { animation: rotate-cw  12.5s linear infinite; }
.gear-c { animation: rotate-ccw 8s   linear infinite; }

.stopped .gear-a,
.stopped .gear-b,
.stopped .gear-c {
  animation-play-state: paused;
}

@keyframes rotate-cw  { to { transform: rotate(360deg);  } }
@keyframes rotate-ccw { to { transform: rotate(-360deg); } }
```

**Do not** use `animation-iteration-count` to control the stop. `paused` at a timer-driven moment is the correct pattern.

### Skills Banner Implementation

The banner requires the skills data from the API. Two approaches:

**Option A (recommended):** Pass skills as a prop from `+page.svelte`. The banner lives inside `Hero.svelte` (or a `SkillsBanner.svelte` child). Same API call already fetches skills.

**Option B:** Hardcode a curated list of 15–20 skill names directly in the banner component. Simpler, no async dependency, but requires manual updates when skills change.

Option A is more maintainable. Flatten the skills array to `skill.skill_name` strings, filter to `category: 'strong'` and `category: 'moderate'` only (omit `gap` — the banner should show what Peter knows, not what he's learning).

Duplicate the list in the DOM for the seamless loop:

```svelte
<div class="banner-track" role="marquee" aria-label="Skills">
  {#each [...skills, ...skills] as skill, i}
    <span class="banner-item">
      <span class="separator" aria-hidden="true">◆</span>
      {skill.skill_name}
    </span>
  {/each}
</div>
```

The `role="marquee"` is technically valid WCAG but note: assistive technologies may announce this as live content. The `aria-hidden="true"` approach (marking the banner hidden and relying on `<SkillsSection>` for AT) is safer. Discuss with Peter which tradeoff he prefers.

### Noise Texture

Add a noise overlay via CSS pseudo-element on `.hero`:

```css
.hero::before {
  content: '';
  position: absolute;
  inset: 0;
  background-image: url('/textures/noise.png');  /* 200×200px, tileable, 8% opacity PNG */
  opacity: 0.04;
  pointer-events: none;
  z-index: 1;
}
```

The noise texture file should be a small (200×200px), high-contrast noise PNG at low base opacity — the `opacity: 0.04` in CSS makes it subtle. Generate with a tool like [Noiseform](https://noiseform.io/) or create programmatically. File size target: under 8KB (it tiles, so tiny is fine).

### Svelte 5 Patterns

This codebase uses Svelte 5 with `$state()`, `$derived()`, `$props()`. Use these throughout:

- `let stopped = $state(false)` for the gear stop state
- `let { skills } = $props()` for the banner
- No `onDestroy` needed — `onMount` return value handles cleanup

### Performance Checklist

- [ ] Gear SVG uses `will-change: transform` only when animation is running (add via JS class, remove after `stopped = true`)
- [ ] Banner track uses `will-change: transform` for GPU compositing
- [ ] Gear SVG marked `aria-hidden` so screen readers skip it
- [ ] Noise texture loaded as a static asset (not base64 inline)
- [ ] `prefers-reduced-motion` tested in DevTools before shipping
- [ ] Entrance animations use `animation-fill-mode: forwards` so elements stay visible after animating in
- [ ] No `requestAnimationFrame` loops — all animation via CSS

### Integration Points

`Hero.svelte` needs to:
1. Accept a `skills` prop (already fetched in `+page.svelte`)
2. Render `<GearsBackground />` as the first child (behind content)
3. Render `<SkillsBanner {skills} />` as the last child (bottom of section)
4. Apply `position: relative` to itself so absolute-positioned children anchor correctly

The existing `<SkillsSection>` below the fold is NOT replaced — the banner is additive.

---

## Peter's Decisions (2026-04-09)

1. **Skills banner accessibility:** `aria-hidden` — simpler, SkillsSection handles AT users with a clean list. Accessibility is a hard requirement across the site.
2. **Gear stop timing:** 10s — less annoying on repeat visits.
3. **Noise texture:** Static asset. Peter wants to eventually create a custom image (e.g., working at a desk).
4. **Skills content in banner:** API-sourced — site is white-labeled for content, skills change over time.
5. **Body font:** Experiment with Source Sans 3 in this iteration.
6. **Animation stop behavior:** Freeze (mechanical halt) — approved as specced.
7. **Skills banner loop:** Stops when scrolled off-screen. Banner acts as dividing line between hero and rest of page. Comes back when scrolling back up.
8. **Hero size:** `calc(100svh - var(--nav-height))` — confirmed via scaffold preview (localhost:5174). Full viewport minus nav works.

## Additional Feedback from Scaffold Review

- **Nav alignment:** Review whether nav content should be centered or left-aligned (→ task #299)
- **Nav buttons:** Need to "pop" and be more engaging — current ghost buttons are too subtle (→ task #299)
