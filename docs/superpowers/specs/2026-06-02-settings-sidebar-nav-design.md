# Settings Page: Sidebar Navigation Redesign

**Date:** 2026-06-02
**Status:** Approved
**Scope:** Navigation structure only — no visual refresh of section content

## Problem

The Settings page uses 4 horizontal tabs (LLM, Connections, Preferences, About) inside `PageTabsBar`. This feels flat and doesn't scale well — adding new sections would crowd the tab bar. The tab structure also hides content behind equal-weight tabs, making it hard to see what's available at a glance.

## Solution

Replace horizontal tabs with a **sidebar navigation** on desktop and a **card-based list** on mobile.

## Design

### Desktop Layout (md+ breakpoint)

```
┌──────────────────────────────────────────────────┐
│  Settings                                 header │
├───────────────┬──────────────────────────────────┤
│               │                                  │
│  ○ LLM        │                                  │
│  ○ Connect.   │   Active Section Content         │
│  ● Pref.  ←──│   (rendered directly, no tabs)    │
│  ○ About      │                                  │
│               │                                  │
│               │                                  │
└───────────────┴──────────────────────────────────┘
```

- Left sidebar: ~200px (`w-52`), full height, vertical list of nav items
- Each nav item: icon + label, hover/active states
- Active item: `bg-muted` background + left border accent (`border-l-2 border-primary`)
- Right area: renders the active section component directly
- No `PageTabsBar` in header

### Mobile Layout (< md breakpoint)

```
┌──────────────────────┐
│  ← Settings          │  (back arrow shown when inside section)
├──────────────────────┤
│ [Cpu] LLM Backends  >│
│ [Plug] Connections   >│
│ [Sliders] Pref.      >│
│ [Info] About         >│
└──────────────────────┘
```

- Section list: vertical card-style rows with icon, label, and chevron
- Tap navigates into the section (full-page view)
- Back arrow in header returns to the list
- No `PageTabsBottomNav` on the settings page

### Navigation State

- State tracked via `useState<SettingsSection>` in `SettingsPage`
- URL param `?tab=llm` still supported for deep-linking (backward compat)
- Valid sections: `"llm" | "connections" | "preferences" | "about"`
- Default: `"preferences"` (same as current)

### Section Components (Unchanged)

| Section | Component | Changes |
|---------|-----------|---------|
| LLM | `UnifiedLLMBackendsTab` | None |
| Connections | `UnifiedDeviceConnectionsTab` | None |
| Preferences | `PreferencesTab` | None |
| About | `AboutTab` | None |

## File Changes

### Modified Files

1. **`web/src/pages/settings.tsx`**
   - Remove `PageTabsBar` from `headerContent`
   - Add sidebar/content split layout for desktop
   - Add mobile section list + drill-down navigation
   - Keep `PageLayout` wrapper
   - Remove `PageTabsBottomNav` usage

### New Files

2. **`web/src/pages/settings/SettingsNav.tsx`**
   - Desktop sidebar navigation component
   - Props: `sections`, `activeSection`, `onSectionChange`
   - Renders icon + label for each section
   - Active state styling with left border accent

3. **`web/src/pages/settings/SettingsSectionList.tsx`**
   - Mobile card-list component
   - Props: `sections`, `onSectionSelect`
   - Each row: icon + label + `ChevronRight`
   - Uses design token colors (`text-muted-foreground`, `bg-card`, etc.)

### Unchanged Files

- `PreferencesTab.tsx` — zero changes
- `AboutTab.tsx` — zero changes
- `UnifiedLLMBackendsTab.tsx` — zero changes
- `UnifiedDeviceConnectionsTab.tsx` — zero changes
- `PageLayout.tsx` — zero changes
- `PageTabs.tsx` — zero changes (still used by other pages)

## Implementation Notes

- Use `useMediaQuery` or `md:` responsive classes for desktop/mobile split
- Mobile uses a simple state machine: `"list" | "section"` — list shows section cards, section shows the component + back arrow
- Desktop sidebar always visible, content area switches components
- Keep `borderedHeader={false}` and remove `hasBottomNav` from PageLayout (settings handles its own mobile nav)
- No new dependencies required
