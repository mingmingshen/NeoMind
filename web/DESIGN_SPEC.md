# NeoMind Frontend Design & Component Specification

This document defines the visual design system, component usage rules, and coding conventions for the NeoMind web frontend. All frontend changes must follow these standards.

---

## 1. Color System

### Design Token Architecture

All colors are defined as OKLCH CSS variables in `src/index.css` (`:root` for light, `.dark` for dark mode) and mapped in `tailwind.config.js`.

**NEVER use hardcoded Tailwind palette colors** (`bg-blue-500`, `text-green-600`, `bg-orange-100`, etc.).

### Semantic Color Tokens

| Purpose | Text Class | Background Class | CSS Variable |
|---------|-----------|-----------------|--------------|
| Success | `text-success` | `bg-success` / `bg-success-light` | `--color-success` |
| Warning | `text-warning` | `bg-warning` / `bg-warning-light` | `--color-warning` |
| Error | `text-error` | `bg-error` / `bg-error-light` | `--color-error` |
| Info | `text-info` | `bg-info` / `bg-info-light` | `--color-info` |

### Accent Category Tokens

| Category | Default | Light Background |
|----------|---------|-----------------|
| Purple | `text-accent-purple` | `bg-accent-purple-light` |
| Orange | `text-accent-orange` | `bg-accent-orange-light` |
| Cyan | `text-accent-cyan` | `bg-accent-cyan-light` |
| Emerald | `text-accent-emerald` | `bg-accent-emerald-light` |
| Indigo | `text-accent-indigo` | `bg-accent-indigo-light` |

### Text on Colored Backgrounds

When placing text/icons on colored backgrounds (buttons, badges, status pills), use `text-primary-foreground` — it resolves to white in both light and dark themes.

```tsx
// Good
<span className="bg-success text-primary-foreground">Active</span>
// Bad
<span className="bg-success text-white">Active</span>
```

### Exceptions: `text-white` / `bg-black` Are Allowed ONLY For

- **Media overlays**: Video player controls, image caption overlays (these sit on top of photos/videos where only pure white/black provides contrast)
- **Modal backdrops**: `bg-black/50`, `bg-black/80` for dialog overlays
- **Micro-contrast backgrounds**: `bg-black/[0.02]` / `bg-white/[0.02]` in builder layouts

### Opacity Limitation (Tailwind v3)

CSS variables defined as plain `oklch()` values do NOT support Tailwind's `/` opacity modifier. These silently fail:

```tsx
// BROKEN - silently produces no opacity
<div className="bg-primary/10" />
<div className="bg-muted-foreground/20" />
```

**Workarounds:**
- Use pre-defined alpha variables: `bg-muted-20`, `bg-muted-30`, `bg-muted-50`, `bg-bg-50`, `bg-bg-70`, etc.
- Use inline styles: `style={{ backgroundColor: 'oklch(0.18 0.02 270 / 10%)' }}`
- Use `bg-success-light` / `bg-error-light` etc. (pre-defined at 8-10% opacity)

---

## 2. Page Layout

### Mandatory Pattern: `PageLayout`

Every page must use `PageLayout` from `@/components/layout/PageLayout`.

```tsx
<PageLayout
  title="Page Title"
  actions={<Button>Add</Button>}
  footer={<Pagination ... />}
>
  {/* Content grows naturally; PageLayout handles scrolling */}
</PageLayout>
```

**Rules:**
- Content area uses `overflow-auto` via PageLayout's scroll container — do NOT add your own scroll
- Fixed headers (tabs) go in `headerContent` prop
- Fixed footers (pagination) go in `footer` prop
- Page-level loading MUST use skeleton screens, never spinners

### Tabs Pattern: `PageTabsBar` + `PageTabsContent`

```tsx
<PageLayout title="Title" headerContent={<PageTabsBar tabs={tabs} />}>
  <PageTabsContent />
</PageLayout>
```

---

## 3. Dialogs

### Standard Dialogs: `UnifiedFormDialog`

Use `UnifiedFormDialog` from `@/components/dialog/UnifiedFormDialog` for all form dialogs. Do NOT use raw `Dialog` from `@/components/ui/dialog`.

```tsx
<UnifiedFormDialog
  open={open}
  onOpenChange={setOpen}
  title="Create Device"
  width="md"
  onSubmit={handleSubmit}
  isSubmitting={saving}
  submitLabel={t('common:create')}
>
  {/* Form fields */}
</UnifiedFormDialog>
```

**Width options:** `sm` (max-w-md) | `md` (max-w-lg) | `lg` (max-w-xl) | `xl` (max-w-2xl) | `2xl` (max-w-3xl) | `3xl` (max-w-5xl)

**Features:** Mobile full-screen, loading overlay, safe area insets, escape/backdrop close, body scroll lock on mobile.

### Full-Screen Builder Dialogs: `FullScreenDialog`

For multi-step builders (rule creation, data transform), use `FullScreenDialog` from `@/components/automation/dialog/FullScreenDialog`.

---

## 4. Loading States

### Page-Level: Skeleton Screens

```tsx
// MUST use for page-level loading
<LoadingState variant="page" />
```

`ResponsiveTable` has built-in skeleton rows when `loading={true}` — no extra loading component needed.

### Inline / Button / Dialog: Spinner

```tsx
// OK for button-level or inline loading
<Button disabled><Loader2 className="h-4 w-4 animate-spin mr-2" />Saving...</Button>
```

### Dialog Loading: Use Props

```tsx
<UnifiedFormDialog loading={initialLoad} isSubmitting={saving} />
```

**NEVER use `Loader2` spinners for page-level or table-level content loading.**

---

## 5. Data Display

### Tables: `ResponsiveTable`

```tsx
<ResponsiveTable
  columns={columns}
  data={items}
  renderCell={(key, row) => <CellComponent />}
  rowKey={(row) => row.id}
  actions={[{ label: 'Delete', onClick: handleDelete, variant: 'destructive' }]}
  loading={loading}
  emptyState={<EmptyState />}
/>
```

**Mobile behavior:** First column becomes card title, remaining columns become key-value pairs. Skeleton rows shown during loading.

### Pagination: `Pagination`

Default page size is **10** across all pages.

```tsx
<Pagination
  total={total}
  pageSize={10}
  currentPage={page}
  onPageChange={setPage}
  hideOnMobile  // enables infinite scroll on mobile
  onLoadMore={loadMore}
/>
```

---

## 6. UI Components

### Button Variants

| Variant | Use Case |
|---------|----------|
| `default` | Primary action (one per section) |
| `destructive` | Delete, dangerous operations |
| `outline` | Secondary actions |
| `secondary` | Tertiary actions |
| `ghost` | Toolbar/icon buttons |
| `link` | Text-only navigation |

### Form Controls

Always import from `@/components/ui/`:

| Component | Import Path |
|-----------|-------------|
| Input | `@/components/ui/input` |
| Select | `@/components/ui/select` (Radix) |
| Checkbox | `@/components/ui/checkbox` (Radix) |
| Switch | `@/components/ui/switch` |
| Label | `@/components/ui/label` |
| Textarea | `@/components/ui/textarea` |

**NEVER use raw HTML `<input>`, `<select>`, `<checkbox>` elements in pages.**

### Checkbox

All checkboxes must use the shared `Checkbox` from `@/components/ui/checkbox`. Do NOT import `@radix-ui/react-checkbox` directly.

```tsx
import { Checkbox } from '@/components/ui/checkbox'
<Checkbox checked={enabled} onCheckedChange={(checked) => setEnabled(!!checked)} />
```

### Status Colors

Use `getStatusColorClass()` and `getStatusBgClass()` from `@/design-system/utils/format` for status-based coloring:

```tsx
import { getStatusColorClass, getStatusBgClass } from '@/design-system/utils/format'

<span className={cn(getStatusBgClass(status), getStatusColorClass(status))}>
  {status}
</span>
```

---

## 7. Z-Index Stack

| Level | Value | Usage |
|-------|-------|-------|
| Base | `z-0` | Normal content |
| Sticky | `z-10` | Sticky headers |
| Dropdowns | `z-40` | Mobile nav, sidebars |
| Overlay | `z-50` | Dialog overlays |
| Full Screen | `z-[100]` | Full-screen dialogs |
| Full Screen Header | `z-[110]` | Full-screen dialog headers |
| Popovers | `z-[200]` | Select, DropdownMenu, Popover, Tooltip |

All popover-type components (Select, DropdownMenu, Popover, Tooltip) render via portals to `#dialog-root` and use `z-[200]` to avoid being trapped behind full-screen dialog stacking contexts.

---

## 8. Store & Data Fetching

### Fetch Deduplication Pattern

Every store slice must use `fetchCache` from `@/lib/utils/async` to prevent redundant API calls:

```typescript
import { fetchCache } from '@/lib/utils/async'

fetchItems: async () => {
  if (!fetchCache.shouldFetch('items')) return
  fetchCache.markFetching('items')

  set({ loading: true })
  try {
    const data = await api.getItems()
    set({ items: data })
    fetchCache.markFetched('items')
  } catch (error) {
    fetchCache.invalidate('items')
  } finally {
    set({ loading: false })
  }
}
```

**Rules:**
- TTL is 10 seconds — check before fetching
- Always `invalidate()` after mutations (add/update/delete)
- WebSocket events use optimistic updates, NOT full refetch

### WebSocket Optimistic Updates

```typescript
updateDeviceStatus: (id: string, status: string) => {
  set((state) => ({
    devices: state.devices.map(d => d.id === id ? { ...d, status } : d)
  }))
}
```

---

## 9. Portal System

All modal/popover content must render through `getPortalRoot()` from `@/lib/portal`. This ensures correct z-index stacking.

```tsx
import { getPortalRoot } from '@/lib/portal'

<SomePrimitive.Portal container={getPortalRoot()}>
  {children}
</SomePrimitive.Portal>
```

---

## 10. Internationalization (i18n)

All user-visible text must use the translation system. Translation files are in `src/i18n/locales/{en,zh}/`.

```tsx
const { t } = useTranslation(['common', 'devices'])
<span>{t('devices:title')}</span>
```

**NEVER hardcode strings** in components — always use `t()` with appropriate key.

---

## 11. Glass Morphism & Surfaces

| Surface | Class | Use Case |
|---------|-------|----------|
| Glass light | `bg-glass` | Floating panels |
| Glass heavy | `bg-glass-heavy` | Fixed headers/footers |
| Surface glass | `bg-surface-glass` | Overlays with backdrop-blur |
| Glass border | `border-glass-border` | Subtle borders |
| Card | `bg-card` | Content cards |
| Muted | `bg-muted` | Subtle backgrounds |

Fixed headers and footers should use `bg-surface-glass backdrop-blur` for the frosted glass effect.

---

## 12. Spacing & Radius

Use Tailwind's standard spacing utilities (`p-2`, `gap-4`, etc.) and the predefined radius tokens:

| Token | Value | Class |
|-------|-------|-------|
| sm | 6px | `rounded-sm` |
| md | 8px | `rounded-md` |
| lg | 10px | `rounded-lg` |
| xl | 15px | `rounded-xl` |
| 2xl | 16px | `rounded-2xl` |
| full | circle | `rounded-full` |

---

## 13. Animation

| Name | Duration | Use Case |
|------|----------|----------|
| `animate-fade-in` | 200ms | General appearance |
| `animate-fade-in-up` | 300ms | Content appearing from below |
| `animate-scale-in` | 200ms | Dialogs, popovers |
| `animate-slide-in` | 200ms | Slide transitions |
| `animate-slide-in-from-*` | 300ms | Directional slides |
| `animate-pulse-slow` | 3s | Status indicators |
| `animate-spin-slow` | 3s | Slow loading |

**Timing:**
- Fast (150ms): hover, focus states
- Normal (200ms): general transitions
- Slow (300ms): layout changes, page transitions

---

## Quick Reference: File Locations

| Resource | Path |
|----------|------|
| CSS Variables | `web/src/index.css` |
| Tailwind Config | `web/tailwind.config.js` |
| Page Layout | `web/src/components/layout/PageLayout.tsx` |
| Shared Components | `web/src/components/shared/` |
| UI Primitives | `web/src/components/ui/` |
| Form Dialog | `web/src/components/dialog/UnifiedFormDialog.tsx` |
| Full Screen Dialog | `web/src/components/automation/dialog/FullScreenDialog.tsx` |
| Portal Utility | `web/src/lib/portal.tsx` |
| Fetch Cache | `web/src/lib/utils/async.ts` |
| Status Colors | `web/src/design-system/utils/format.ts` |
| i18n Locales | `web/src/i18n/locales/{en,zh}/` |
