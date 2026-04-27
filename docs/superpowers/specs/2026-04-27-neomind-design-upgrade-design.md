# NeoMind UI Design Upgrade Spec

## Overview

Upgrade the NeoMind frontend to a glass morphism + aurora design language with OKLCH color space, supporting both light and dark themes. The upgrade is incremental, preserving existing component architecture (shadcn/ui + Radix UI) while updating visual tokens.

## Design Direction: Glass Aurora

**Chosen style**: Dark-first glass morphism with subtle aurora gradients, applicable to both light and dark themes.

**Core principles**:
- Glass cards with `backdrop-filter: blur(20px) saturate(120%)` and semi-transparent backgrounds
- Aurora background: radial gradients using brand orange + cool purple accents
- Brand color `#f24a00` (oklch(0.637 0.237 41)) used as restrained accent only (buttons, key data, progress bars)
- Light shadows, moderate border radius (12px/10px/6px)
- All colors defined in OKLCH for perceptual uniformity

## Color System

### Brand Color
- `#f24a00` = `oklch(0.637 0.237 41)` = `hsl(18 100% 47%)`
- Used only on: primary buttons, key data highlights, progress bars, AI badges

### Dark Theme Tokens

| Token | OKLCH Value | Usage |
|-------|-------------|-------|
| `--bg-deep` | `oklch(0.13 0.008 270)` | Page background |
| `--bg` | `oklch(0.16 0.008 270)` | Card/elevated surfaces |
| `--glass` | `oklch(0.20 0.008 270 / 60%)` | Glass card background |
| `--glass-heavy` | `oklch(0.18 0.008 270 / 80%)` | Nav/footer glass |
| `--glass-border` | `oklch(1 0 0 / 10%)` | White transparency borders |
| `--glass-border-hover` | `oklch(1 0 0 / 18%)` | Hover state borders |
| `--fg` | `oklch(0.95 0.005 270)` | Primary text |
| `--fg-muted` | `oklch(0.58 0.012 270)` | Secondary text |
| `--fg-subtle` | `oklch(0.40 0.010 270)` | Tertiary text |
| `--success` | `oklch(0.72 0.19 155)` | Online status |
| `--warning` | `oklch(0.72 0.16 85)` | Alert/warning |
| `--error` | `oklch(0.577 0.245 27)` | Error/critical |
| `--info` | `oklch(0.65 0.15 250)` | Info messages |
| `--purple` | `oklch(0.62 0.20 300)` | Aurora accent |

### Light Theme Tokens

| Token | OKLCH Value | Usage |
|-------|-------------|-------|
| `--bg-deep` | `oklch(0.98 0.003 270)` | Page background |
| `--bg` | `oklch(0.96 0.004 270)` | Subtle surfaces |
| `--glass` | `oklch(1 0 0 / 65%)` | Glass card background |
| `--glass-heavy` | `oklch(1 0 0 / 82%)` | Nav/footer glass |
| `--glass-border` | `oklch(0 0 0 / 7%)` | Black transparency borders |
| `--glass-border-hover` | `oklch(0 0 0 / 14%)` | Hover state borders |
| `--fg` | `oklch(0.18 0.02 270)` | Primary text |
| `--fg-muted` | `oklch(0.45 0.01 270)` | Secondary text |
| `--fg-subtle` | `oklch(0.62 0.008 270)` | Tertiary text |
| `--success` | `oklch(0.55 0.17 155)` | Online status |
| `--warning` | `oklch(0.68 0.17 65)` | Alert/warning |
| `--error` | `oklch(0.55 0.22 25)` | Error/critical |
| `--info` | `oklch(0.52 0.15 250)` | Info messages |

### Semantic Colors (Brand)

| Token | Dark | Light | Usage |
|-------|------|-------|-------|
| `--primary` | brand gradient | brand gradient | CTA buttons |
| `--primary-foreground` | white | white | Text on primary |
| `--secondary` | `oklch(1 0 0 / 6%)` | `oklch(0 0 0 / 4%)` | Secondary actions |
| `--muted` | `oklch(0.20 0.008 270)` | `oklch(0.95 0.003 270)` | Muted backgrounds |
| `--accent` | `oklch(0.637 0.237 41 / 8%)` | `oklch(0.637 0.237 41 / 5%)` | Accent highlights |

## Shadows

Dark theme shadows use dark color bases, light theme uses light shadows:

| Token | Dark | Light |
|-------|------|-------|
| `--shadow-glass` | `0 2px 8px oklch(0 0 0 / 20%)` | `0 1px 3px oklch(0 0 0 / 5%)` |
| `--shadow-glass-lg` | `0 4px 16px oklch(0 0 0 / 25%)` | `0 2px 8px oklch(0 0 0 / 8%)` |
| `--shadow-brand` | `0 2px 12px oklch(0.637 0.237 41 / 15%)` | `0 2px 12px oklch(0.637 0.237 41 / 12%)` |

## Border Radius

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-lg` | `12px` | Cards, panels |
| `--radius` | `10px` | Buttons, inputs |
| `--radius-sm` | `6px` | Badges, small elements |

## Aurora Background

Fixed position, non-interactive gradient layer behind all content:

```css
/* Dark theme */
.aurora {
  background:
    radial-gradient(ellipse 600px 400px at 15% 10%, oklch(0.637 0.237 41 / 10%) 0%, transparent 70%),
    radial-gradient(ellipse 500px 500px at 85% 90%, oklch(0.62 0.20 300 / 6%) 0%, transparent 70%),
    radial-gradient(ellipse 800px 300px at 50% 50%, oklch(0.55 0.10 250 / 4%) 0%, transparent 70%);
}

/* Light theme */
.aurora {
  background:
    radial-gradient(ellipse 600px 400px at 10% 5%, oklch(0.637 0.237 41 / 5%) 0%, transparent 70%),
    radial-gradient(ellipse 500px 500px at 90% 95%, oklch(0.55 0.12 280 / 4%) 0%, transparent 70%),
    radial-gradient(ellipse 800px 300px at 50% 40%, oklch(0.95 0.02 80 / 30%) 0%, transparent 70%);
}
```

## Glass Card Pattern

```css
.glass {
  background: var(--glass);
  backdrop-filter: blur(20px) saturate(120%);
  -webkit-backdrop-filter: blur(20px) saturate(120%);
  border: 1px solid var(--glass-border);
  box-shadow: var(--shadow-glass);
}

/* Top edge highlight */
.glass::before {
  content: '';
  position: absolute;
  top: 0; left: 0; right: 0;
  height: 1px;
  background: linear-gradient(90deg, transparent, oklch(1 0 0 / 12%), transparent);
}
```

Applied to: stat cards, data tables, activity feeds, nav bar, footer.

## Components to Update

### Phase 1: Color Foundation (index.css)
- Replace all HSL CSS custom properties with OKLCH equivalents
- Add aurora background layer
- Add glass utility classes
- Update semantic color tokens for both themes

### Phase 2: Core Components
- **TopNav**: glass background, updated active states with brand dot indicator
- **Button (btn-primary)**: brand gradient + light brand shadow
- **Card/Panel**: glass mixin, top edge highlight
- **Badge**: glass background, semantic color borders
- **Table**: transparent borders, glass hover states

### Phase 3: Layout & Surfaces
- **PageLayout**: aurora background integration
- **Footer**: glass background
- **Sidebar/Sheet**: glass-heavy for overlay panels
- **Dialog/Modal**: glass-heavy with blur backdrop

### Phase 4: Micro-interactions (future)
- Card hover: border brighten + shadow lift
- Button press: scale(0.98)
- Theme toggle: smooth 300ms transition on all tokens

## Typography

Already implemented: Plus Jakarta Sans (Latin) + Noto Sans SC (Chinese)

No further typography changes needed.

## Chart Colors (OKLCH)

```css
--chart-1: oklch(0.65 0.18 300);  /* Purple */
--chart-2: oklch(0.65 0.17 155);  /* Green */
--chart-3: oklch(0.72 0.16 85);   /* Yellow */
--chart-4: oklch(0.637 0.237 41); /* Brand orange */
--chart-5: oklch(0.65 0.17 20);   /* Red-orange */
--chart-6: oklch(0.65 0.12 210);  /* Teal */
```

## Performance Considerations

- `backdrop-filter` has GPU cost; limit to visible cards only
- Aurora background is pure CSS (no animation), zero runtime cost
- All transitions use `transform` and `opacity` where possible
- `will-change` not needed; browser compositing handles blur efficiently

## Files to Modify

1. `web/src/index.css` — color tokens, glass utilities, aurora layer
2. `web/src/components/layout/TopNav.tsx` — glass nav
3. `web/src/components/layout/PageLayout.tsx` — aurora background
4. `web/src/components/ui/button.tsx` — brand gradient primary
5. `web/tailwind.config.js` — extend theme with new tokens

## Out of Scope

- Component structure changes (keep shadcn/ui pattern)
- New components or features
- Animation/AI-generated visual effects
- Mobile-specific redesign
