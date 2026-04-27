# NeoMind UI Design Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade NeoMind frontend to glass morphism + aurora design with OKLCH colors for both light and dark themes.

**Architecture:** Replace HSL CSS custom properties with OKLCH equivalents, add aurora background gradient layer and glass utility classes, update 4 key components (TopNav, PageLayout, Button, Badge) to use glass styling. All changes are CSS-first with minimal component code changes.

**Tech Stack:** CSS custom properties, Tailwind CSS v3 (class-based dark mode), OKLCH color space

**Spec:** `docs/superpowers/specs/2026-04-27-neomind-design-upgrade-design.md`

---

### Task 1: Update CSS Color Tokens to OKLCH (Dark Theme)

**Files:**
- Modify: `web/src/index.css` (lines 231-292, the `.dark { ... }` block)

This is the largest single change. Replace all HSL values in the `.dark` theme block with OKLCH equivalents per the spec. Keep the same variable names so Tailwind mappings (`hsl(var(--background))`) still work — but change the values to include `oklch(...)` directly, and update the Tailwind config to use raw `var()` instead of `hsl(var())`.

> **Important:** CSS variables that contain `oklch(...)` values cannot be wrapped in `hsl()`. We must update both the CSS values AND the Tailwind config in the same commit.

- [ ] **Step 1: Update the `.dark` block in `web/src/index.css`**

Replace the `.dark` block (lines 231-292) with:

```css
  .dark {
    /* Glass Aurora - Dark Theme */
    --background: oklch(0.13 0.008 270);
    --foreground: oklch(0.95 0.005 270);
    --card: oklch(0.16 0.008 270);
    --card-foreground: oklch(0.95 0.005 270);
    --popover: oklch(0.16 0.008 270);
    --popover-foreground: oklch(0.95 0.005 270);
    --primary: oklch(0.95 0.005 270);
    --primary-foreground: oklch(0.13 0.008 270);
    --secondary: oklch(0.20 0.008 270);
    --secondary-foreground: oklch(0.95 0.005 270);
    --muted: oklch(0.20 0.008 270);
    --muted-foreground: oklch(0.58 0.012 270);
    --accent: oklch(0.20 0.008 270);
    --accent-foreground: oklch(0.95 0.005 270);
    --destructive: oklch(0.577 0.245 27);
    --destructive-foreground: oklch(0.95 0.005 270);
    --border: oklch(1 0 0 / 10%);
    --input: oklch(1 0 0 / 10%);
    --ring: oklch(0.70 0.01 270);

    /* Semantic colors */
    --color-success: oklch(0.72 0.19 155);
    --color-success-bg: oklch(0.72 0.19 155 / 10%);
    --color-warning: oklch(0.72 0.16 85);
    --color-warning-bg: oklch(0.72 0.16 85 / 10%);
    --color-error: oklch(0.577 0.245 27);
    --color-error-bg: oklch(0.577 0.245 27 / 10%);
    --color-info: oklch(0.65 0.15 250);
    --color-info-bg: oklch(0.65 0.15 250 / 10%);

    /* Shadows - lighter */
    --shadow-sm: 0 1px 2px 0 oklch(0 0 0 / 20%);
    --shadow-md: 0 2px 6px -1px oklch(0 0 0 / 25%);
    --shadow-lg: 0 4px 16px -3px oklch(0 0 0 / 25%);
    --shadow-xl: 0 8px 24px -5px oklch(0 0 0 / 30%);

    /* Glass-specific tokens */
    --glass: oklch(0.20 0.008 270 / 60%);
    --glass-heavy: oklch(0.18 0.008 270 / 80%);
    --glass-border: oklch(1 0 0 / 10%);
    --glass-border-hover: oklch(1 0 0 / 18%);
    --shadow-glass: 0 2px 8px 0 oklch(0 0 0 / 20%);
    --shadow-glass-lg: 0 4px 16px 0 oklch(0 0 0 / 25%);
    --shadow-brand: 0 2px 12px 0 oklch(0.637 0.237 41 / 15%);

    /* Chart colors */
    --chart-1: oklch(0.65 0.18 300);
    --chart-2: oklch(0.65 0.17 155);
    --chart-3: oklch(0.72 0.16 85);
    --chart-4: oklch(0.637 0.237 41);
    --chart-5: oklch(0.65 0.17 20);
    --chart-6: oklch(0.65 0.12 210);

    /* Chat-specific colors - dark mode */
    --msg-user-bg: oklch(0.45 0.15 260);
    --msg-user-text: oklch(1 0 0);
    --msg-ai-bg: oklch(0.18 0.008 270);
    --msg-ai-text: oklch(0.90 0.005 270);
    --msg-system-bg: oklch(0.20 0.008 270);
    --msg-system-text: oklch(0.58 0.012 270);

    --tool-bg: oklch(0.20 0.008 270);
    --tool-border: oklch(1 0 0 / 15%);
    --tool-header-bg: oklch(0.22 0.008 270);

    --thinking-bg: oklch(0.20 0.008 270);
    --thinking-border: oklch(1 0 0 / 12%);
    --thinking-text: oklch(0.62 0.01 270);

    --card-hover-bg: oklch(0.18 0.008 270);
    --input-focus-bg: oklch(0.16 0.008 270);

    --session-drawer-bg: oklch(0.12 0.008 270);
    --session-drawer-border: oklch(1 0 0 / 8%);
    --session-item-hover: oklch(0.18 0.008 270);
    --session-item-active: oklch(0.22 0.008 270);
  }
```

- [ ] **Step 2: Update the `:root` (default light) block in `web/src/index.css`**

Replace lines 158-229 (`:root` inside `@layer base`) with OKLCH light values:

```css
  :root {
    --background: oklch(1 0 0);
    --foreground: oklch(0.18 0.02 270);
    --card: oklch(1 0 0);
    --card-foreground: oklch(0.18 0.02 270);
    --popover: oklch(1 0 0);
    --popover-foreground: oklch(0.18 0.02 270);
    --primary: oklch(0.18 0.02 270);
    --primary-foreground: oklch(1 0 0);
    --secondary: oklch(0.96 0.003 270);
    --secondary-foreground: oklch(0.18 0.02 270);
    --muted: oklch(0.96 0.003 270);
    --muted-foreground: oklch(0.45 0.01 270);
    --accent: oklch(0.96 0.003 270);
    --accent-foreground: oklch(0.18 0.02 270);
    --destructive: oklch(0.577 0.245 27);
    --destructive-foreground: oklch(1 0 0);
    --border: oklch(0 0 0 / 10%);
    --input: oklch(0 0 0 / 10%);
    --ring: oklch(0.18 0.02 270);
    --radius: 0.625rem;

    /* Spacing system */
    --space-1: 0.25rem;
    --space-2: 0.5rem;
    --space-3: 0.75rem;
    --space-4: 1rem;
    --space-6: 1.5rem;
    --space-8: 2rem;

    /* Border radius */
    --radius-sm: 0.375rem;
    --radius-md: 0.5rem;
    --radius-lg: 0.625rem;
    --radius-xl: 0.75rem;
    --radius-2xl: 1rem;
    --radius-full: 9999px;

    /* Chart colors */
    --chart-1: oklch(0.65 0.18 300);
    --chart-2: oklch(0.65 0.17 155);
    --chart-3: oklch(0.72 0.16 85);
    --chart-4: oklch(0.637 0.237 41);
    --chart-5: oklch(0.65 0.17 20);
    --chart-6: oklch(0.65 0.12 210);

    /* Animation durations */
    --duration-fast: 150ms;
    --duration-normal: 200ms;
    --duration-slow: 300ms;

    /* Easing functions */
    --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
    --ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
    --ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);

    /* Semantic colors */
    --color-success: oklch(0.55 0.17 155);
    --color-success-bg: oklch(0.55 0.17 155 / 8%);
    --color-warning: oklch(0.68 0.17 65);
    --color-warning-bg: oklch(0.68 0.17 65 / 8%);
    --color-error: oklch(0.55 0.22 25);
    --color-error-bg: oklch(0.55 0.22 25 / 8%);
    --color-info: oklch(0.52 0.15 250);
    --color-info-bg: oklch(0.52 0.15 250 / 8%);

    /* Shadow system */
    --shadow-sm: 0 1px 2px 0 oklch(0 0 0 / 5%);
    --shadow-md: 0 2px 4px -1px oklch(0 0 0 / 8%);
    --shadow-lg: 0 4px 12px -2px oklch(0 0 0 / 8%);
    --shadow-xl: 0 8px 20px -4px oklch(0 0 0 / 10%);

    /* Glass-specific tokens (light) */
    --glass: oklch(1 0 0 / 65%);
    --glass-heavy: oklch(1 0 0 / 82%);
    --glass-border: oklch(0 0 0 / 7%);
    --glass-border-hover: oklch(0 0 0 / 14%);
    --shadow-glass: 0 1px 3px 0 oklch(0 0 0 / 5%);
    --shadow-glass-lg: 0 2px 8px 0 oklch(0 0 0 / 8%);
    --shadow-brand: 0 2px 12px 0 oklch(0.637 0.237 41 / 12%);

    /* Chat-specific colors - light mode */
    --msg-user-bg: oklch(0.45 0.18 260);
    --msg-user-text: oklch(1 0 0);
    --msg-ai-bg: oklch(0.97 0.003 270);
    --msg-ai-text: oklch(0.18 0.02 270);
    --msg-system-bg: oklch(0.95 0.003 270);
    --msg-system-text: oklch(0.45 0.01 270);

    --tool-bg: oklch(0.97 0.003 270);
    --tool-border: oklch(0.45 0.15 260);
    --tool-header-bg: oklch(0.94 0.005 270);

    --thinking-bg: oklch(0.97 0.01 80);
    --thinking-border: oklch(0.68 0.17 65);
    --thinking-text: oklch(0.40 0.01 270);

    --card-hover-bg: oklch(0.97 0.003 270);
    --input-focus-bg: oklch(1 0 0);

    --session-drawer-bg: oklch(0.99 0.002 270);
    --session-drawer-border: oklch(0 0 0 / 8%);
    --session-item-hover: oklch(0.95 0.003 270);
    --session-item-active: oklch(0.93 0.003 270);
  }
```

- [ ] **Step 3: Remove the `[data-theme="light"]` block**

The `[data-theme="light"]` block (lines 295-355) duplicates `:root` values. Since `:root` now has proper light-theme OKLCH values, delete the entire `[data-theme="light"]` block. The app uses `class="dark"` switching, so this block is unnecessary.

- [ ] **Step 4: Update Tailwind config to use `var()` instead of `hsl(var())`**

Modify `web/tailwind.config.js` — replace all `"hsl(var(--xxx))"` with `"var(--xxx)"`:

```js
      colors: {
        border: "var(--border)",
        input: "var(--input)",
        ring: "var(--ring)",
        background: "var(--background)",
        foreground: "var(--foreground)",
        primary: {
          DEFAULT: "var(--primary)",
          foreground: "var(--primary-foreground)",
        },
        secondary: {
          DEFAULT: "var(--secondary)",
          foreground: "var(--secondary-foreground)",
        },
        destructive: {
          DEFAULT: "var(--destructive)",
          foreground: "var(--destructive-foreground)",
        },
        muted: {
          DEFAULT: "var(--muted)",
          foreground: "var(--muted-foreground)",
        },
        accent: {
          DEFAULT: "var(--accent)",
          foreground: "var(--accent-foreground)",
        },
        popover: {
          DEFAULT: "var(--popover)",
          foreground: "var(--popover-foreground)",
        },
        card: {
          DEFAULT: "var(--card)",
          foreground: "var(--card-foreground)",
        },
        success: {
          DEFAULT: "var(--color-success)",
          light: "var(--color-success-bg)",
        },
        warning: {
          DEFAULT: "var(--color-warning)",
          light: "var(--color-warning-bg)",
        },
        error: {
          DEFAULT: "var(--color-error)",
          light: "var(--color-error-bg)",
        },
        info: {
          DEFAULT: "var(--color-info)",
          light: "var(--color-info-bg)",
        },
      },
```

- [ ] **Step 5: Verify build compiles**

Run: `cd web && npm run build`
Expected: Build succeeds with no errors.

- [ ] **Step 6: Visual check**

Run: `cd web && npm run dev`
Check: Open browser, verify both light and dark themes render correctly. Colors should look very similar to before but slightly cleaner (OKLCH is perceptually uniform).

- [ ] **Step 7: Commit**

```bash
git add web/src/index.css web/tailwind.config.js
git commit -m "feat: migrate CSS color tokens from HSL to OKLCH

- Replace all HSL values in :root and .dark with OKLCH equivalents
- Update Tailwind config to use var() instead of hsl(var())
- Remove redundant [data-theme='light'] block
- Add glass-specific CSS custom properties"
```

---

### Task 2: Add Aurora Background and Glass Utilities

**Files:**
- Modify: `web/src/index.css` (add after the theme blocks, before `@layer base`)
- Modify: `web/src/index.css` (add in `@layer components` or `@layer utilities`)

- [ ] **Step 1: Add aurora background CSS**

Add after the closing `}` of `.dark { ... }` but still inside `@layer base`:

```css
  /* Aurora background - fixed gradient layer */
  .aurora-bg {
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    transition: background 300ms ease;
  }

  .dark .aurora-bg {
    background:
      radial-gradient(ellipse 600px 400px at 15% 10%, oklch(0.637 0.237 41 / 8%) 0%, transparent 70%),
      radial-gradient(ellipse 500px 500px at 85% 90%, oklch(0.62 0.20 300 / 5%) 0%, transparent 70%),
      radial-gradient(ellipse 800px 300px at 50% 50%, oklch(0.55 0.10 250 / 3%) 0%, transparent 70%);
  }

  .aurora-bg:not(.dark *) {
    background:
      radial-gradient(ellipse 600px 400px at 10% 5%, oklch(0.637 0.237 41 / 4%) 0%, transparent 70%),
      radial-gradient(ellipse 500px 500px at 90% 95%, oklch(0.55 0.12 280 / 3%) 0%, transparent 70%),
      radial-gradient(ellipse 800px 300px at 50% 40%, oklch(0.95 0.02 80 / 20%) 0%, transparent 70%);
  }
```

- [ ] **Step 2: Add glass utility classes**

Add in `@layer utilities`:

```css
  /* Glass morphism utilities */
  .glass {
    background: var(--glass);
    backdrop-filter: blur(20px) saturate(120%);
    -webkit-backdrop-filter: blur(20px) saturate(120%);
    border: 1px solid var(--glass-border);
    box-shadow: var(--shadow-glass);
  }

  .glass-heavy {
    background: var(--glass-heavy);
    backdrop-filter: blur(24px) saturate(130%);
    -webkit-backdrop-filter: blur(24px) saturate(130%);
    border: 1px solid var(--glass-border);
    box-shadow: var(--shadow-glass-lg);
  }
```

- [ ] **Step 3: Verify build**

Run: `cd web && npm run build`
Expected: Success.

- [ ] **Step 4: Commit**

```bash
git add web/src/index.css
git commit -m "feat: add aurora background and glass morphism CSS utilities"
```

---

### Task 3: Update TopNav to Glass Style

**Files:**
- Modify: `web/src/components/layout/TopNav.tsx`

- [ ] **Step 1: Update the nav container className**

In `TopNav.tsx` line 160, change:

```
className="fixed top-0 left-0 right-0 min-h-16 bg-background/95 backdrop-blur flex items-center px-4 sm:px-6 shadow-sm z-50"
```

To:

```
className="fixed top-0 left-0 right-0 min-h-16 bg-background/70 backdrop-blur-xl saturate-120 border-b border-border/60 flex items-center px-4 sm:px-6 z-50"
```

This gives the nav a glass background with increased blur and slight transparency.

- [ ] **Step 2: Update active nav item style**

In the desktop nav items (line 182-186), change the active state from:

```tsx
isActive
  ? "bg-foreground text-background hover:bg-foreground hover:text-background"
  : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
```

To:

```tsx
isActive
  ? "bg-primary/10 text-primary hover:bg-primary/15"
  : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
```

This uses brand primary color for the active indicator instead of inverting black/white.

- [ ] **Step 3: Update mobile drawer active item style (same change)**

In the mobile nav items (line 237-240), same change:

```tsx
isActive
  ? "bg-primary/10 text-primary"
  : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
```

- [ ] **Step 4: Verify in browser**

Run: `cd web && npm run dev`
Check: Nav should have slight transparency, glass blur effect. Active nav item shows with primary color tint.

- [ ] **Step 5: Commit**

```bash
git add web/src/components/layout/TopNav.tsx
git commit -m "feat: update TopNav to glass style with transparent background"
```

---

### Task 4: Update PageLayout Footer Glass Effect

**Files:**
- Modify: `web/src/components/layout/PageLayout.tsx`

- [ ] **Step 1: Update footer glass morphism**

In `PageLayout.tsx` line 108, change:

```tsx
<div className="fixed bottom-0 left-0 right-0 bg-gradient-to-t from-background via-background/95 to-background/80 backdrop-blur-md border-t border-border/30">
```

To:

```tsx
<div className="fixed bottom-0 left-0 right-0 bg-background/70 backdrop-blur-xl saturate-120 border-t border-border/60">
```

- [ ] **Step 2: Verify in browser**

Check: Footer should have matching glass effect with the nav bar.

- [ ] **Step 3: Commit**

```bash
git add web/src/components/layout/PageLayout.tsx
git commit -m "feat: update PageLayout footer to glass style"
```

---

### Task 5: Add Aurora Background to App Root

**Files:**
- Modify: `web/src/App.tsx` or the root layout component that wraps all pages

- [ ] **Step 1: Find the root layout component**

Search for the component that renders `<TopNav>` and the main content area. This is likely `App.tsx` or a layout component.

Run: `grep -r "TopNav" web/src/ --include="*.tsx" -l`

- [ ] **Step 2: Add aurora-bg div as a sibling to the main content wrapper**

Add a `<div className="aurora-bg" />` element as the first child of the root container, before `<TopNav />`. This provides the fixed background gradient that the glass cards will blur over.

Example change in the root layout:
```tsx
<div className="relative flex h-full flex-col">
  <div className="aurora-bg" />
  <TopNav ref={topNavRef} />
  {/* ... rest of content with relative z-10 */}
</div>
```

Ensure the main content area has `relative z-10` so it sits above the aurora background.

- [ ] **Step 3: Verify in browser**

Check: Subtle aurora gradient visible behind content. More noticeable in dark theme (warm orange glow top-left, cool purple glow bottom-right).

- [ ] **Step 4: Commit**

```bash
git add web/src/App.tsx
git commit -m "feat: add aurora background gradient layer to app root"
```

---

### Task 6: Final Visual Verification and Cleanup

**Files:**
- Potentially adjust: `web/src/index.css` (fine-tuning values)
- Potentially adjust: Any component that looks off

- [ ] **Step 1: Run full build**

Run: `cd web && npm run build`
Expected: Clean build, no warnings.

- [ ] **Step 2: Test both themes**

Toggle between light and dark themes. Check:
- All pages load without visual breaks
- Text is readable (contrast ratios sufficient)
- Glass effects visible on nav, footer, cards
- Aurora background subtle and not distracting
- Chart colors render correctly
- Chat messages still readable
- Mobile responsive layout intact

- [ ] **Step 3: Fix any visual regressions**

If any component looks wrong (colors off, contrast too low, etc.), adjust the relevant CSS variable values. Common issues:
- If text is hard to read: adjust `--foreground` / `--muted-foreground` lightness
- If borders are too visible: reduce `--glass-border` opacity
- If aurora is too strong: reduce gradient opacity values

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "fix: fine-tune OKLCH color values after visual verification"
```

---

## Summary

| Task | Description | Est. Changes |
|------|-------------|-------------|
| 1 | CSS tokens HSL → OKLCH | ~120 lines in index.css, ~30 lines in tailwind.config.js |
| 2 | Aurora + glass CSS utilities | ~30 lines in index.css |
| 3 | TopNav glass style | ~6 lines in TopNav.tsx |
| 4 | PageLayout footer glass | ~1 line in PageLayout.tsx |
| 5 | Aurora background in app root | ~2 lines in App.tsx |
| 6 | Visual verification & cleanup | As needed |
