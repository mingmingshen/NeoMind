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

### Chart Colors

Six chart-compatible tokens defined in `index.css`: `--chart-1` through `--chart-6`.

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

### Chat-Specific Colors

| Token | Purpose |
|-------|---------|
| `--msg-user-bg` / `--msg-user-text` | User message bubbles |
| `--msg-ai-bg` / `--msg-ai-text` | AI message bubbles |
| `--msg-system-bg` / `--msg-system-text` | System messages |
| `--tool-bg` / `--tool-border` / `--tool-header-bg` | Tool call display |
| `--thinking-bg` / `--thinking-border` / `--thinking-text` | Thinking state display |

### Hover State Tokens

| Token | Usage |
|-------|-------|
| `--primary-hover` | Primary button hover |
| `--secondary-hover` | Secondary button hover |
| `--destructive-hover` | Destructive button hover |
| `--card-hover-bg` | Card hover background |
| `--glass-border-hover` | Glass border hover state |

---

## 2. Typography

### Font Family

| Token | Class | Usage |
|-------|-------|-------|
| Sans | `font-sans` (default) | All UI text — Plus Jakarta Sans + Noto Sans SC + system-ui |
| Mono | `font-mono` | Code, device IDs, monospaced data |

For inline styles (CodeMirror, Recharts), use `fontMonoStack` from `@/design-system/tokens/typography`.

### Font Size Tokens

All custom font sizes are defined as semantic tokens in `@/design-system/tokens/typography`. **NEVER hardcode `text-[Xpx]` in components** — import the appropriate token instead.

| Token | Size | Use Case |
|-------|------|----------|
| `textMicro` | 9px | Extreme micro labels — data type badges in execution details |
| `textNano` | 10px | Timestamps, tiny metadata, compact badges |
| `textMini` | 11px | Badge text, secondary labels, tab labels |
| `textCode` | 12px | Inline code in markdown, code snippets |
| `textBody` | 13px | Chat messages, tool call text, markdown body |
| `textHeading` | 15px | Markdown headings within content |

Standard Tailwind sizes fill the remaining tiers:

| Class | Size | Use Case |
|-------|------|----------|
| `text-xs` | 12px | Small labels, secondary text, helper text |
| `text-sm` | 14px | Body text, form labels, descriptions |
| `text-base` | 16px | Standard body, primary content |
| `text-lg` | 18px | Section headings |
| `text-xl`+ | 20px+ | Page titles, hero text |

### Badge Size Presets

Use `badgeSize` from typography tokens for consistent badge text:

```tsx
import { badgeSize } from '@/design-system/tokens/typography'

<Badge className={cn(badgeSize.micro, "h-4 px-1")}>int</Badge>    // 9px
<Badge className={cn(badgeSize.small, "h-5 px-2")}>online</Badge> // 10px
<Badge className={cn(badgeSize.default, "h-5 px-2")}>Active</Badge> // 11px
```

### Visual Hierarchy

```
9px(micro) → 10px(nano) → 11px(mini) → 12px(xs/code) → 13px(body) → 14px(sm) → 15px(heading) → 16px(base)
```

The smallest sizes (9-11px) are reserved for **non-essential metadata** that users scan rather than read: timestamps, device IDs, status badges, data type labels. Primary content and interactive elements always use 12px or larger.

### Exceptions: When `text-[Xpx]` Must Stay

These patterns require literal static strings and cannot use tokens:

1. **Tailwind prose modifiers** — `prose-h1:text-[15px]` (JIT needs full string)
2. **CVA variant configs** — `sm: 'text-[10px]'` (must be static for type inference)
3. **Third-party library APIs** — Recharts `tick={{ fontSize: 10 }}`, CodeMirror `theme({ fontSize: '13px' })`

### Import Pattern

```tsx
import { textNano, textMini, textBody, cn } from '@/design-system/tokens/typography'
import { cn } from '@/lib/utils'

<span className={cn(textNano, "text-muted-foreground")}>2 min ago</span>
<p className={cn(textBody, "leading-relaxed")}>Chat message content</p>
```

---

## 3. Page Layout

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

## 4. Dialogs

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

### Confirmation Dialog: `useConfirm`

Promise-based confirmation dialog for destructive actions:

```tsx
import { useConfirm } from '@/components/ui/use-confirm'

const confirm = useConfirm()
const yes = await confirm({ title: 'Delete?', description: 'This cannot be undone.' })
if (yes) await api.deleteItem(id)
```

### Dialog State Hook: `useDialog`

```tsx
import { useDialog } from '@/hooks/useDialog'

const { open, data, openDialog, closeDialog } = useDialog<User>()
openDialog(user)  // passes user as data
```

---

## 5. Loading States

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

### Loading Button Hook

```tsx
import { useLoadingButton } from '@/hooks/useLoadingButton'

const { isLoading, handleClick } = useLoadingButton(async () => { await api.save(data) })
<Button onClick={handleClick} disabled={isLoading}>
  {isLoading && <Loader2 className="h-4 w-4 animate-spin mr-2" />}Save
</Button>
```

**NEVER use `Loader2` spinners for page-level or table-level content loading.**

---

## 6. Data Display

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

### Virtual List: `VirtualList`

For rendering 1000+ items (sessions, logs, etc.), use `VirtualList` from `@/components/ui/virtual-list` for high-performance rendering.

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

### Bulk Actions: `BulkActionBar`

For multi-select operations:

```tsx
<BulkActionBar
  selectedCount={selected.length}
  onCancel={() => setSelected([])}
  actions={[{ label: 'Delete', variant: 'destructive', onClick: handleBulkDelete }]}
/>
```

### Stats Display: `StatsCard` / `MonitorStatsGrid`

```tsx
<StatsCard title="Devices" value={42} icon={Server} trend="+5%" />
<MonitorStatsGrid stats={stats} />
```

### Status Badges: `StatusBadge`

```tsx
<StatusBadge status="online" />
<StatusBadge variant="warning">Pending</StatusBadge>
```

### Empty States: `EmptyState`

```tsx
<EmptyState icon={Server} title="No devices" description="Add your first device" />
```

---

## 7. UI Components

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

| Component | Import Path | Notes |
|-----------|-------------|-------|
| Input | `@/components/ui/input` | Standard text input |
| Select | `@/components/ui/select` | Radix-based dropdown |
| Checkbox | `@/components/ui/checkbox` | Radix-based, use `onCheckedChange` |
| Switch | `@/components/ui/switch` | Toggle switch |
| Label | `@/components/ui/label` | Form label |
| Textarea | `@/components/ui/textarea` | Multi-line input |
| Slider | `@/components/ui/slider` | Range input |
| ColorPicker | `@/components/ui/color-picker` | Color selection |
| IconPicker | `@/components/ui/icon-picker` | Icon selection |
| EntityIconPicker | `@/components/ui/entity-icon-picker` | Entity icon selection |
| CodeEditor | `@/components/ui/code-editor` | Code/text editing |

**NEVER use raw HTML `<input>`, `<select>`, `<checkbox>` elements in pages.**

### Form Layout

| Component | Import Path | Purpose |
|-----------|-------------|---------|
| FormSection | `@/components/ui/form-section` | Grouped form sections with title |
| Field | `@/components/ui/field` | Label + input + error wrapper |

### Checkbox

All checkboxes must use the shared `Checkbox` from `@/components/ui/checkbox`. Do NOT import `@radix-ui/react-checkbox` directly.

```tsx
import { Checkbox } from '@/components/ui/checkbox'
<Checkbox checked={enabled} onCheckedChange={(checked) => setEnabled(!!checked)} />
```

### Other UI Primitives

| Component | Import Path | Purpose |
|-----------|-------------|---------|
| Card | `@/components/ui/card` | Content card containers |
| Badge | `@/components/ui/badge` | Labels and tags |
| Avatar | `@/components/ui/avatar` | User avatars |
| Tabs | `@/components/ui/tabs` | Content tabs (in-page, not page-level) |
| Accordion | `@/components/ui/accordion` | Collapsible sections |
| Sheet | `@/components/ui/sheet` | Side panel |
| Progress | `@/components/ui/progress` | Progress bar |
| Skeleton | `@/components/ui/skeleton` | Custom skeleton shapes |
| ScrollArea | `@/components/ui/scroll-area` | Custom scrollable area |
| Separator | `@/components/ui/separator` | Visual divider |
| Tooltip | `@/components/ui/tooltip` | Hover information |
| Toast | `@/components/ui/use-toast` | Notification toasts |
| Confirmer | `@/components/ui/confirmer` | Confirmation dialog |

### Status Colors

Use `getStatusColorClass()` and `getStatusBgClass()` from `@/design-system/utils/format` for status-based coloring:

```tsx
import { getStatusColorClass, getStatusBgClass } from '@/design-system/utils/format'

<span className={cn(getStatusBgClass(status), getStatusColorClass(status))}>
  {status}
</span>
```

---

## 8. Z-Index Stack

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

## 9. Store & Data Fetching

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

### Store Slices

All slices follow the same pattern with `fetchCache`:

| Slice | File | Key Resources |
|-------|------|---------------|
| device | `store/slices/deviceSlice.ts` | Devices, adapters |
| agent | `store/slices/agentSlice.ts` | AI agents |
| llmBackend | `store/slices/llmBackendSlice.ts` | LLM backends |
| extension | `store/slices/extensionSlice.ts` | Extensions |
| session | `store/slices/sessionSlice.ts` | Chat sessions |
| alert | `store/slices/alertSlice.ts` | Alerts |
| dashboard | `store/slices/dashboardSlice.ts` | Dashboards |
| settings | `store/slices/settingsSlice.ts` | System settings |
| auth | `store/slices/authSlice.ts` | Authentication |
| ui | `store/slices/uiSlice.ts` | UI state |
| update | `store/slices/updateSlice.ts` | Update checks |
| aiAnalyst | `store/slices/aiAnalystSlice.ts` | AI analyst |

---

## 10. Portal System

All modal/popover content must render through `getPortalRoot()` from `@/lib/portal`. This ensures correct z-index stacking.

```tsx
import { getPortalRoot } from '@/lib/portal'

<SomePrimitive.Portal container={getPortalRoot()}>
  {children}
</SomePrimitive.Portal>
```

---

## 11. Error Handling

### Centralized Error System

All error handling uses `@/lib/errors.ts` and `useErrorHandler` hook. **NEVER use bare `console.error` + `toast`** — always use the centralized system.

```tsx
import { useErrorHandler } from '@/hooks/useErrorHandler'

const { handleError, showSuccess, withErrorHandling } = useErrorHandler()
```

### Error Handler API

| Method | Purpose |
|--------|---------|
| `handleError(error, options?)` | Log + show toast. Options: `{ showToast, userMessage, operation }` |
| `withErrorHandling(fn, options?)` | Async wrapper, auto-catches and handles |
| `showSuccess(message)` | Success toast |
| `getErrorMessage(error)` | Extract user-friendly message |

### Usage Patterns

```tsx
// Manual handling
try {
  await api.deleteDevice(id)
  showSuccess('Device deleted')
} catch (error) {
  handleError(error, { operation: 'Delete device' })
}

// Automatic handling
const result = await withErrorHandling(
  () => api.createDevice(data),
  { operation: 'Create device' }
)
```

### Error Classification

Import from `@/lib/errors`:

| Function | Purpose |
|----------|---------|
| `isNetworkError(error)` | Network/connection failures |
| `isAuthError(error)` | 401/authentication errors |
| `isValidationError(error)` | 400/422 validation errors |
| `isNotFoundError(error)` | 404 errors |
| `isConflictError(error)` | 409 conflicts |
| `logError(error, context?)` | Structured error logging |

### Form Submission Hook

```tsx
import { useFormSubmit } from '@/hooks/useErrorHandler'

const { isSubmitting, handleSubmit } = useFormSubmit({
  successMessage: 'Saved',
  errorOperation: 'Save settings',
})

<form onSubmit={handleSubmit(async () => await api.save(data))}>
```

---

## 12. Internationalization (i18n)

All user-visible text must use the translation system. Translation files are in `src/i18n/locales/{en,zh}/`.

```tsx
const { t } = useTranslation(['common', 'devices'])
<span>{t('devices:title')}</span>
```

**NEVER hardcode strings** in components — always use `t()` with appropriate key.

---

## 13. Glass Morphism & Surfaces

| Surface | Class | Use Case |
|---------|-------|----------|
| Glass light | `bg-glass` | Floating panels |
| Glass heavy | `bg-glass-heavy` | Fixed headers/footers |
| Surface glass | `bg-surface-glass` | Overlays with backdrop-blur |
| Glass border | `border-glass-border` | Subtle borders |
| Glass border hover | `border-glass-border-hover` | Hover state |
| Card | `bg-card` | Content cards |
| Muted | `bg-muted` | Subtle backgrounds |

Fixed headers and footers should use `bg-surface-glass backdrop-blur` for the frosted glass effect.

---

## 14. Mobile & Responsive

### Breakpoints (Tailwind defaults)

| Prefix | Width | Target |
|--------|-------|--------|
| (none) | < 640px | Mobile |
| `sm:` | >= 640px | Large phones |
| `md:` | >= 768px | Tablets |
| `lg:` | >= 1024px | Laptops |
| `xl:` | >= 1280px | Desktops |

### Mobile Detection Hooks

Import from `@/hooks/useMobile`:

| Hook | Returns | Purpose |
|------|---------|---------|
| `useIsMobile()` | `boolean` | Viewport < 768px |
| `useIsTouchDevice()` | `boolean` | Touch capability |
| `useDeviceType()` | `'mobile' \| 'tablet' \| 'desktop'` | Full device classification |

### Touch Interaction Hooks

| Hook | Purpose |
|------|---------|
| `useTouchHover(options?)` | Touch-friendly hover state (toggle on tap) |
| `useLongPress(options)` | Long-press gesture (500ms default, 10px threshold) |
| `useTouchHandler()` | Touch event management |

### Safe Area Insets (iPhone X+ notches)

CSS utility classes: `.safe-top`, `.safe-bottom`, `.safe-left`, `.safe-right`, `.pt-safe`, `.pb-safe`

Hook: `useSafeAreaInsets()` returns `{ top, right, bottom, left }` pixel values.

CSS variables:
```css
--topnav-height: calc(4rem + env(safe-area-inset-top, 0px));
--chat-content-padding-top: calc(6.5rem + env(safe-area-inset-top, 0px));
```

### Touch Targets

Minimum 44x44px per iOS/Android guidelines. Small icon buttons (h-6 through h-9) are acceptable exceptions.

### Body Scroll Lock

Use `useMobileBodyScrollLock()` when a modal/dialog opens on mobile to prevent background scrolling.

---

## 15. Form Validation

The codebase uses a **custom `useForm` hook** (not react-hook-form or zod):

```tsx
import { useForm } from '@/hooks/useForm'

const { values, errors, setValue, handleSubmit, isSubmitting } = useForm({
  initialValues: { name: '', type: 'mqtt' },
  onSubmit: async (values) => { await api.createDevice(values) },
  validate: (values) => {
    const errors: Record<string, string> = {}
    if (!values.name) errors.name = 'Name is required'
    return Object.keys(errors).length > 0 ? errors : undefined
  }
})
```

**Features:**
- Live validation on value change
- Custom validation function (returns `Record<string, string>` or `undefined`)
- `isSubmitting` state
- `submitError` for submission-level errors
- `setError(key, msg)` / `clearError(key)` for manual error management
- `reset()` to restore initial values

---

## 16. Spacing & Radius

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

## 17. Animation

| Name | Duration | Use Case |
|------|----------|----------|
| `animate-fade-in` | 200ms | General appearance |
| `animate-fade-out` | 200ms | Disappearance |
| `animate-fade-in-up` | 300ms | Content appearing from below |
| `animate-scale-in` | 200ms | Dialogs, popovers |
| `animate-scale-out` | 200ms | Closing animations |
| `animate-slide-in` | 200ms | Slide transitions |
| `animate-slide-in-from-*` | 300ms | Directional slides (top/bottom/left/right) |
| `animate-pulse-slow` | 3s | Status indicators |
| `animate-spin-slow` | 3s | Slow loading |
| `animate-bounce-subtle` | 2s | Subtle bounce (5px Y) |
| `animate-shimmer` | 2s | Skeleton loading shimmer |
| `animate-typewriter` | 2s | Typewriter effect |
| `animate-blink` | 1s | Blinking cursor |

**Stagger delays:** `delay-0`, `delay-100`, `delay-150`, `delay-200`, `delay-300`, `delay-400`, `delay-500`

**Timing CSS variables:**
- `--duration-fast`: 150ms (hover, focus)
- `--duration-normal`: 200ms (general transitions)
- `--duration-slow`: 300ms (layout changes)

**Easing:** `--ease-out`, `--ease-in-out`, `--ease-spring`

---

## 18. Infinite Scroll

For pages with infinite scroll on mobile:

```tsx
import { useInfiniteScroll } from '@/hooks/useInfiniteScroll'

const { ref, isLoading } = useInfiniteScroll({
  hasMore: page * pageSize < total,
  isLoading: loadingMore,
  onLoadMore: () => setPage(p => p + 1),
  threshold: 200,
})
```

---

## 19. Polling

For data that needs periodic refresh:

```tsx
import { useVisiblePolling } from '@/hooks/useVisiblePolling'

// Only polls when tab is visible
useVisiblePolling(fetchData, { interval: 30000, enabled: true })
```

---

## 20. Search

### SearchBar: Global search component

```tsx
import { SearchBar } from '@/components/shared'

<SearchBar
  placeholder="Search..."
  onSearch={handleSearch}
  resultTypes={[
    { key: 'device', label: 'Device' },
    { key: 'rule', label: 'Rule' },
  ]}
/>
```

### SearchResultsDialog

Full search results overlay with categorized results.

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
| Error Handling | `web/src/lib/errors.ts` |
| Error Hook | `web/src/hooks/useErrorHandler.ts` |
| Fetch Cache | `web/src/lib/utils/async.ts` |
| Status Colors | `web/src/design-system/utils/format.ts` |
| i18n Locales | `web/src/i18n/locales/{en,zh}/` |
| Mobile Hooks | `web/src/hooks/useMobile.ts` |
| Form Hook | `web/src/hooks/useForm.ts` |
| Dialog Hook | `web/src/hooks/useDialog.ts` |
| Infinite Scroll | `web/src/hooks/useInfiniteScroll.ts` |
| Store Slices | `web/src/store/slices/` |
