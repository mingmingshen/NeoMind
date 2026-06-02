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

- Rename existing `SettingsTabValue` type to `SettingsSection` (or keep the name — implementer's choice)
- State tracked via `useState<SettingsSection>` in `SettingsPage`
- URL param `?tab=llm` still supported for deep-linking (backward compat, one-shot read on mount as current)
- Valid sections: `"llm" | "connections" | "preferences" | "about"`
- Default: `"preferences"` (same as current)

### Mobile & Global TopNav Interaction

The global mobile TopNav tab bar (with Settings entry) remains visible at all times. The in-page mobile back arrow sits within the Settings page content area, below the global TopNav. No conflict — two distinct navigation levels:
- **Global TopNav**: switches between top-level app pages (Dashboard, Chat, Settings, etc.)
- **In-page nav**: switches between settings sections

When a user taps a section on mobile and enters drill-down view:
- The back arrow + section title appears at the top of the page content
- The global TopNav remains visible above it
- URL updates to `?tab=<section>` to support deep-linking
- Pressing the in-page back arrow returns to the section list (URL reverts to `/settings`)

### Bottom Spacing

When removing `hasBottomNav` from `PageLayout`, add manual bottom padding on mobile via `pb-safe` or `pb-[calc(2rem+env(safe-area-inset-bottom))]` to the scroll container. Desktop needs no special bottom spacing beyond the default.

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

- Use `useIsMobile()` from `@/hooks/useMobile` for desktop/mobile detection (project convention, not `useMediaQuery`)
- Mobile uses a simple state machine: `"list" | "section"` — list shows section cards, section shows the component + back arrow
- Desktop sidebar always visible, content area switches components
- Keep `borderedHeader={false}` and remove `hasBottomNav` from PageLayout. Add manual mobile bottom padding (see Bottom Spacing section)
- The sidebar+content layout sits inside `PageLayout` children. The sidebar does NOT need edge-to-edge positioning — standard `PageLayout` padding is fine. Use `md:flex md:gap-6` inside the content area.
- No new dependencies required

### Accessibility

- Sidebar nav items use `role="tablist"` / `role="tab"` with `aria-selected` on active item
- Mobile section list uses `role="list"` / `role="listitem"` with `aria-label` on each row
- Keyboard navigation: arrow keys cycle through sidebar items, Enter activates

### Icons

| Section | Icon (lucide-react) | Label i18n Key |
|---------|---------------------|----------------|
| LLM | `Cpu` | `settings:llmBackends` |
| Connections | `Plug` | `settings:deviceConnections` |
| Preferences | `Sliders` | `settings:preferences` |
| About | `Info` | `settings:about` |

### Store Subscriptions

The LLM backend store selectors (`createBackend`, `updateBackend`, `deleteBackend`, `testBackend`) remain in `settings.tsx` and are passed as props to `UnifiedLLMBackendsTab` — same pattern as current code, no change needed.
