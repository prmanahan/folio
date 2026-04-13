# Ironworks Design System

> **Source:** Google Stitch project `9166146696467023785`, asset `assets/11870742886852404047`
> **Date:** 2026-04-09

Industrial-inspired dark theme for a senior software architect's portfolio site.

## Aesthetic

- Dark, warm backgrounds (#1a1612 base, #2a2219 surface, #352c20 elevated)
- Brass/gold metallic accents (#b08d57 primary, #d4af70 light, #e8c97a gold)
- Teal for links and functional color (#7ab8b8)
- Copper for warnings/gaps (#b56a4a)
- Subtle noise texture overlay on hero backgrounds
- Decorative gear/mechanical motifs at very low opacity (0.06-0.10)
- Tight corner radii (2-3px) — industrial, not playful
- Text: warm cream (#e8e0d0 primary, #c8b898 secondary, #a09080 muted)

## Color Palette

### Raw Tokens (defined in `frontend/src/app.css`)

| Token | Hex | Usage |
|-------|-----|-------|
| `--raw-bg-base` | #1a1612 | Page background |
| `--raw-bg-surface` | #2a2219 | Card backgrounds, elevated panels |
| `--raw-bg-elevated` | #352c20 | Hover states, active surfaces |
| `--raw-bg-navy` | #1c2a3a | Accent backgrounds |
| `--raw-bg-deeper` | #12100e | Deepest background layer |
| `--raw-text-primary` | #e8e0d0 | Primary text, headings |
| `--raw-text-secondary` | #c8b898 | Subtitles, metadata |
| `--raw-text-muted` | #a09080 | Ghost text, placeholders |
| `--raw-brass` | #b08d57 | Primary accent, borders |
| `--raw-brass-light` | #d4af70 | Hover accent, emphasis |
| `--raw-brass-dark` | #8a6a35 | Pressed states |
| `--raw-copper` | #b56a4a | Warning/gap accent |
| `--raw-copper-light` | #d4876a | Warning light variant |
| `--raw-gold` | #e8c97a | Premium accent, highlights |
| `--raw-teal` | #4d8a8a | Functional dark teal |
| `--raw-teal-light` | #7ab8b8 | Links, availability badge |
| `--raw-error` | #c0544a | Error states |
| `--raw-warning` | #d4a04a | Warning states |
| `--raw-success` | #4a8a6a | Success states |

### Semantic Aliases

| Token | Maps to | Usage |
|-------|---------|-------|
| `--color-bg` | `--raw-bg-base` | Page background |
| `--color-surface` | `--raw-bg-surface` | Card/panel backgrounds |
| `--color-text` | `--raw-text-primary` | Body text |
| `--color-text-muted` | `--raw-text-secondary` | Secondary text |
| `--color-accent` | `--raw-brass` | Primary accent |
| `--color-accent-light` | `--raw-brass-light` | Hover accent |
| `--color-link` | `--raw-teal-light` | Links |
| `--color-gold` | `--raw-gold` | Premium highlights |

## Typography

- **Font:** Source Sans 3 (headlines and body)
- **Headlines:** Semibold/bold weights
- **Body:** Regular weight
- **Responsive scaling:** `clamp()` functions for fluid sizing

## Shape & Spacing

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-sm` | 2px | Small elements |
| `--radius-md` | 3px | Default |
| `--radius-lg` | 6px | Large elements |
| `--radius-btn` | 2px | Buttons |
| `--nav-height` | 3.5rem | Nav bar (0 on home page in hub layout) |

## Interactive Elements

### Buttons
- Brass border with transparent background
- 44px minimum touch targets
- 2px border radius
- Hover: brass-light border + subtle background fill (`rgba(176, 141, 87, 0.10)`)

### Cards
- Brass outline border (`rgba(176, 141, 87, 0.30)`)
- Dark surface background (`--raw-bg-surface`)
- Hover: border brightens to `--raw-brass`, subtle background fill
- Ask AI card (primary): filled brass background, not outline

### Status Indicators
- Active: teal (`--raw-teal-light`)
- In Progress: gold (`--raw-gold`)
- Archived: muted (`--raw-text-muted`)
- Availability badge: teal pill with subtle teal border

## Stitch Configuration

```
Display Name: Ironworks
Color Mode: DARK
Color Variant: TONAL_SPOT
Headline Font: SOURCE_SANS_THREE
Body Font: SOURCE_SANS_THREE
Roundness: ROUND_FOUR
Custom Color: #b08d57
Primary Override: #b08d57
Secondary Override: #e8c97a
Tertiary Override: #4d8a8a
Neutral Override: #1a1612
```
