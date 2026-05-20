# Dashboard Full Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the dashboard system with TanStack Query for data fetching and modular Zustand slices for UI state, keeping all existing functionality working during migration.

**Architecture:** Build new `features/dashboard/` module alongside existing code. TanStack Query handles all data fetching/caching. Zustand split into 4 focused slices. Widgets are self-contained with Error Boundaries. Cutover replaces old code only after full verification.

**Tech Stack:** React 18, TypeScript, TanStack Query v5, Zustand, react-grid-layout, recharts, lucide-react, Zod v4

**Spec:** `docs/superpowers/specs/2026-05-20-dashboard-rewrite-design.md`

---

## File Structure Map

### New Files (features/dashboard/)

```
web/src/features/dashboard/
├── api/
│   ├── dashboards.ts            # Dashboard CRUD API calls
│   └── telemetry.ts             # Telemetry/history fetch functions
├── hooks/
│   ├── useWidgetDataSource.ts   # Unified data source wrapper
│   ├── useDeviceTelemetry.ts    # Device telemetry polling + WS
│   ├── useExtensionMetric.ts    # Extension metric polling
│   ├── useSystemMetric.ts       # System metric polling
│   ├── useDashboardLayout.ts    # Grid layout management
│   └── queries.ts               # Dashboard query key tree + hooks
├── store/
│   ├── dashboardCrudSlice.ts    # Dashboard CRUD + persistence
│   ├── dashboardLayoutSlice.ts  # Grid layout state
│   ├── dashboardEditSlice.ts    # Edit mode + selection
│   ├── dashboardConfigSlice.ts  # Widget config management
│   └── index.ts                 # Composed store
├── components/
│   ├── VisualDashboard.tsx      # Main page orchestrator
│   ├── DashboardGrid.tsx        # react-grid-layout wrapper
│   ├── DashboardEventBridge.tsx # WS/SSE → TanStack Query bridge
│   ├── WidgetShell.tsx          # Error boundary + chrome + loading
│   ├── WidgetErrorFallback.tsx  # Error fallback UI
│   ├── WidgetSkeleton.tsx       # Loading skeleton
│   ├── InstallWidgetDialog.tsx  # Add widget dialog
│   └── config/
│       ├── WidgetConfigPanel.tsx
│       ├── DataSourceSelector.tsx
│       ├── DataSourceField.tsx
│       ├── DisplayOptions.tsx
│       └── ActionConfig.tsx
├── widgets/                     # Migrated from components/dashboard/generic/
│   ├── ValueCard/
│   ├── LineChart/
│   ├── BarChart/
│   ├── PieChart/
│   ├── AreaChart/
│   ├── Sparkline/
│   ├── LedIndicator/
│   ├── ProgressBar/
│   ├── ToggleSwitch/
│   ├── ImageDisplay/
│   ├── ImageHistory/
│   ├── WebDisplay/
│   ├── MarkdownDisplay/
│   ├── MapDisplay/
│   ├── VideoDisplay/
│   ├── CustomLayer/
│   ├── AgentMonitor/
│   ├── AiAnalyst/
│   ├── registry.ts              # Static widget registry
│   ├── DynamicRegistry.ts       # Extension widget registry
│   └── CommunityRegistry.ts     # Marketplace widget registry
├── types/
│   ├── dashboard.ts             # Core dashboard types
│   ├── dataSources.ts           # Data source discriminated unions
│   └── widgets.ts               # Widget config types + WidgetConfigMap
└── utils/
    ├── telemetryTransform.ts    # Aggregation, time windows
    └── colorScales.ts           # Chart color handling
```

### Files Modified (cutover only)

- `web/src/App.tsx` or router config — route to new VisualDashboard
- `web/src/types/dashboard.ts` — re-export from new types

### Files Removed (cutover only)

- `web/src/store/slices/dashboardSlice.ts`
- `web/src/hooks/useDataSource.ts`
- `web/src/hooks/useDashboardPrefetch.ts`
- `web/src/components/dashboard/` (entire directory)
- `web/src/pages/dashboard-components/VisualDashboard.tsx`

---

## Phase 1: Foundation

### Task 1: Create directory structure + types

**Files:**
- Create: `web/src/features/dashboard/types/dashboard.ts`
- Create: `web/src/features/dashboard/types/dataSources.ts`
- Create: `web/src/features/dashboard/types/widgets.ts`
- Create: `web/src/features/dashboard/types/index.ts`

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p web/src/features/dashboard/{api,hooks,store,components/config,widgets,types,utils}
```

- [ ] **Step 2: Write data source types** (`types/dataSources.ts`)

Create discriminated union types for all 13 data source types. Use the exact same `DataSourceType` string values from current `web/src/types/dashboard.ts` to maintain API compatibility. Key types:

- `DataSource` — discriminated union with `type` field for each of the 13 source types
- `ResolvedDataSource` — live data wrapper with `value`, `timeSeries`, `isLoading`, `error`, `unit`, `lastUpdated`
- `TimeWindow`, `AggregateMethod`, `TelemetryTransformConfig` — time-series config types

- [ ] **Step 3: Write dashboard core types** (`types/dashboard.ts`)

Port from existing `web/src/types/dashboard.ts` but with strict typing:
- `Dashboard` — keep same shape, reference new `DataSource` type
- `DashboardComponent` = `GenericComponent | BusinessComponent`
- `ComponentPosition`, `DisplayConfig`, `ActionConfig` — same as current
- `GenericComponent` — with typed `dataSource: DataSource | null` instead of `Record<string, unknown>`
- `BusinessComponent` — same pattern

- [ ] **Step 4: Write widget types** (`types/widgets.ts`)

- `WidgetType` — union of all 19 widget type strings
- `WidgetConfigMap` — mapped type from WidgetType to per-widget config
- `WidgetProps` — `{ widgetId, config, dataSource: ResolvedDataSource | null, isEditing }`
- `WidgetConfigProps` — `{ widgetId, config, onSave }`
- `WidgetDefinition` — static registry entry (type, icon, defaultSize, sizeConstraints, component, configComponent)
- `DynamicWidgetDefinition` — extension/community registry entry with loader functions

- [ ] **Step 5: Write barrel export** (`types/index.ts`)

Re-export all types from the three files.

- [ ] **Step 6: Verify TypeScript compilation**

Run: `cd web && npx tsc --noEmit`
Expected: No errors in new files. Existing code unaffected.

- [ ] **Step 7: Commit**

```bash
git add web/src/features/dashboard/types/
git commit -m "feat(dashboard): add rewrite type foundations — discriminated unions, strict widget types"
```

### Task 2: API client functions

**Files:**
- Create: `web/src/features/dashboard/api/dashboards.ts`
- Create: `web/src/features/dashboard/api/telemetry.ts`

- [ ] **Step 1: Write dashboard API client** (`api/dashboards.ts`)

Functions that call the existing API endpoints:
- `listDashboards(): Promise<Dashboard[]>` — GET `/dashboards` → `fromDashboardDTO()`
- `getDashboard(id: string): Promise<Dashboard>` — GET `/dashboards/:id` → `fromDashboardDTO()`
- `createDashboard(dashboard): Promise<Dashboard>` — POST `/dashboards` with `toCreateDashboardDTO()`
- `updateDashboard(id, updates): Promise<Dashboard>` — PUT `/dashboards/:id` with `toUpdateDashboardDTO()`
- `deleteDashboard(id): Promise<void>` — DELETE `/dashboards/:id`
- `shareDashboard(id, config): Promise<ShareConfig>` — POST `/dashboards/:id/share`
- `unshareDashboard(id): Promise<void>` — DELETE `/dashboards/:id/share`

Import `fetchAPI` from `@/lib/api`, DTO converters from `@/store/persistence/types`.

- [ ] **Step 2: Write telemetry API client** (`api/telemetry.ts`)

- `fetchTelemetry(sourceId: string, timeWindow: TimeWindow): Promise<TelemetryPoint[]>`
- `fetchDeviceMetrics(deviceId: string): Promise<Metric[]>`
- `fetchExtensionMetrics(extensionId: string): Promise<Metric[]>`
- `fetchSystemMetrics(type: SystemMetricType): Promise<MetricValue>`

- [ ] **Step 3: Verify TypeScript compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 4: Commit**

```bash
git add web/src/features/dashboard/api/
git commit -m "feat(dashboard): add API client functions for dashboards and telemetry"
```

### Task 3: Zustand store slices

**Files:**
- Create: `web/src/features/dashboard/store/dashboardCrudSlice.ts`
- Create: `web/src/features/dashboard/store/dashboardLayoutSlice.ts`
- Create: `web/src/features/dashboard/store/dashboardEditSlice.ts`
- Create: `web/src/features/dashboard/store/dashboardConfigSlice.ts`
- Create: `web/src/features/dashboard/store/index.ts`

- [ ] **Step 1: Write dashboardCrudSlice** (~300 lines)

State: `dashboards`, `currentDashboardId`, `dashboardsLoading`
Actions: `fetchDashboards`, `createDashboard`, `updateDashboard`, `deleteDashboard`, `setCurrentDashboard`, `shareDashboard`, `unshareDashboard`
Integration: Uses existing `createDashboardStorage()` for persistence, `fromDashboardDTO`/`toDashboardDTO` for conversion.
Debounced sync (500ms) to API via the storage layer.

- [ ] **Step 2: Write dashboardLayoutSlice** (~250 lines)

State: `layouts` (per breakpoint), `layoutVersion`
Actions: `addLayoutItem`, `removeLayoutItem`, `updateLayout`, `resetLayout`
Uses react-grid-layout layout format. Syncs layout changes back to `dashboardCrudSlice` on drag/resize stop.

- [ ] **Step 3: Write dashboardEditSlice** (~150 lines)

State: `editMode`, `selectedWidgetId`, `configPanelOpen`, `configWidgetId`, `componentLibraryOpen`, `isReadOnly`
Actions: `setEditMode`, `selectWidget`, `openConfig`, `closeConfig`, `openLibrary`, `closeLibrary`

- [ ] **Step 4: Write dashboardConfigSlice** (~250 lines)

State: (reads from current dashboard in crudSlice)
Actions: `addWidget`, `removeWidget`, `updateWidgetConfig`, `updateWidgetDataSource`, `updateWidgetDisplay`, `updateWidgetActions`
These modify the current dashboard's components array and trigger persistence.

- [ ] **Step 5: Write composed store** (`store/index.ts`)

```typescript
import { create, type StateCreator } from 'zustand'
import { createCrudSlice, type CrudSlice } from './dashboardCrudSlice'
import { createLayoutSlice, type LayoutSlice } from './dashboardLayoutSlice'
import { createEditSlice, type EditSlice } from './dashboardEditSlice'
import { createConfigSlice, type ConfigSlice } from './dashboardConfigSlice'

export type DashboardStore = CrudSlice & LayoutSlice & EditSlice & ConfigSlice

export const useDashboardStore = create<DashboardStore>()(
  (...a) => ({
    ...createCrudSlice(...a),
    ...createLayoutSlice(...a),
    ...createEditSlice(...a),
    ...createConfigSlice(...a),
  })
)
```

- [ ] **Step 6: Verify TypeScript compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 7: Commit**

```bash
git add web/src/features/dashboard/store/
git commit -m "feat(dashboard): add Zustand store with 4 focused slices"
```

---

## Phase 2: Data Layer

### Task 4: Dashboard query hooks

**Files:**
- Create: `web/src/features/dashboard/hooks/queries.ts`

- [ ] **Step 1: Write query key tree**

Namespaced under `['dashboard', ...]` to avoid collision with existing `react-query-hooks.ts`:

```typescript
export const dashboardKeys = {
  all: ['dashboard'] as const,
  lists: () => [...dashboardKeys.all, 'list'] as const,
  list: (filters?) => [...dashboardKeys.lists(), filters] as const,
  detail: (id: string) => [...dashboardKeys.all, id] as const,
  telemetry: (sourceId: string, window: string) =>
    [...dashboardKeys.all, 'telemetry', sourceId, window] as const,
  deviceMetrics: (deviceId: string) =>
    [...dashboardKeys.all, 'device', deviceId, 'metrics'] as const,
  extensionMetrics: (extId: string) =>
    [...dashboardKeys.all, 'extension', extId, 'metrics'] as const,
  systemMetric: (type: string) =>
    [...dashboardKeys.all, 'system', type] as const,
}
```

- [ ] **Step 2: Write dashboard query hooks**

- `useDashboardList()` — queries `listDashboards()`, 5min staleTime
- `useDashboard(id)` — queries `getDashboard(id)`, 2min staleTime
- `useDashboardMutations()` — create/update/delete with automatic invalidation

- [ ] **Step 3: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 4: Commit**

```bash
git add web/src/features/dashboard/hooks/queries.ts
git commit -m "feat(dashboard): add TanStack Query key tree and dashboard query hooks"
```

### Task 5: Per-source-type data hooks

**Files:**
- Create: `web/src/features/dashboard/hooks/useDeviceTelemetry.ts`
- Create: `web/src/features/dashboard/hooks/useExtensionMetric.ts`
- Create: `web/src/features/dashboard/hooks/useSystemMetric.ts`
- Create: `web/src/features/dashboard/hooks/useWidgetDataSource.ts`

- [ ] **Step 1: Write useDeviceTelemetry** (~150 lines)

Uses `useQuery` with `dashboardKeys.telemetry(sourceId, timeWindow)`. Polling interval from config. Returns `ResolvedDataSource`.

- [ ] **Step 2: Write useExtensionMetric** (~100 lines)

Uses `useQuery` with `dashboardKeys.extensionMetrics(extId)`. Returns `ResolvedDataSource`.

- [ ] **Step 3: Write useSystemMetric** (~100 lines)

Uses `useQuery` with `dashboardKeys.systemMetric(type)`. Returns `ResolvedDataSource`.

- [ ] **Step 4: Write useWidgetDataSource** (~200 lines)

Unified wrapper that inspects `DataSource.type` and delegates to the correct per-source hook. Also handles `static` type (no fetch, returns value directly). Returns `ResolvedDataSource | null`.

- [ ] **Step 5: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 6: Commit**

```bash
git add web/src/features/dashboard/hooks/
git commit -m "feat(dashboard): add per-source-type data hooks with TanStack Query"
```

### Task 6: Event bridge + telemetry utils

**Files:**
- Create: `web/src/features/dashboard/components/DashboardEventBridge.tsx`
- Create: `web/src/features/dashboard/utils/telemetryTransform.ts`

- [ ] **Step 1: Write telemetryTransform utils** (~200 lines)

Port aggregation logic from current `useDataSource.ts`:
- `aggregateData(points, method, window)` — 10 aggregation methods
- `applyTimeWindow(points, window)` — 9 predefined windows + custom
- `mergeDataPoint(existing, newPoint)` — append to time-series
- `formatTelemetryValue(value, config)` — display formatting

- [ ] **Step 2: Write DashboardEventBridge** (~150 lines)

Component that mounts inside VisualDashboard. Subscribes to WebSocket events via existing `useEvents` hook. Routes events to `queryClient.setQueryData()` for the relevant query keys. Returns null (no UI).

- [ ] **Step 3: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 4: Commit**

```bash
git add web/src/features/dashboard/components/DashboardEventBridge.tsx web/src/features/dashboard/utils/telemetryTransform.ts
git commit -m "feat(dashboard): add event bridge and telemetry transform utilities"
```

---

## Phase 3: Core Components

### Task 7: Widget shell with error boundary

**Files:**
- Create: `web/src/features/dashboard/components/WidgetShell.tsx`
- Create: `web/src/features/dashboard/components/WidgetErrorFallback.tsx`
- Create: `web/src/features/dashboard/components/WidgetSkeleton.tsx`

- [ ] **Step 1: Write WidgetErrorFallback** (~30 lines)

Simple error display with retry button. Uses design tokens.

- [ ] **Step 2: Write WidgetSkeleton** (~30 lines)

Skeleton matching typical widget dimensions. Uses existing Skeleton component.

- [ ] **Step 3: Write WidgetShell** (~150 lines)

Wraps each widget with:
1. React Error Boundary (class component)
2. React.Suspense with WidgetSkeleton fallback
3. Widget chrome: title bar with icon, resize handle, config button (edit mode only)
4. Calls `useWidgetDataSource(config.dataSource)` and passes result to widget
5. Handles drag handle in edit mode

- [ ] **Step 4: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 5: Commit**

```bash
git add web/src/features/dashboard/components/WidgetShell.tsx web/src/features/dashboard/components/WidgetErrorFallback.tsx web/src/features/dashboard/components/WidgetSkeleton.tsx
git commit -m "feat(dashboard): add WidgetShell with error boundary and skeleton"
```

### Task 8: Dashboard grid + visual dashboard

**Files:**
- Create: `web/src/features/dashboard/components/DashboardGrid.tsx`
- Create: `web/src/features/dashboard/components/VisualDashboard.tsx`
- Create: `web/src/features/dashboard/hooks/useDashboardLayout.ts`

- [ ] **Step 1: Write useDashboardLayout** (~100 lines)

Hook that manages react-grid-layout state: layout, responsive breakpoints, compact type. Syncs layout changes to Zustand store on drag/resize stop.

- [ ] **Step 2: Write DashboardGrid** (~200 lines)

Port from existing `web/src/components/dashboard/DashboardGrid.tsx` (329 lines) but simplified:
- Uses `useDashboardLayout` for layout management
- Renders `WidgetShell` for each widget
- Responsive breakpoints (lg/md/sm/xs)
- Touch device support
- "Settle" mechanism for new components

- [ ] **Step 3: Write VisualDashboard** (~300 lines)

Port from existing `web/src/pages/dashboard-components/VisualDashboard.tsx` but cleaner:
- Mounts `DashboardEventBridge`
- Dashboard header with name, edit toggle, settings
- Renders `DashboardGrid` in the main area
- Edit mode sidebar with `InstallWidgetDialog`
- Config panel slide-out when widget selected
- Uses new Zustand store for all state

- [ ] **Step 4: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 5: Commit**

```bash
git add web/src/features/dashboard/components/DashboardGrid.tsx web/src/features/dashboard/components/VisualDashboard.tsx web/src/features/dashboard/hooks/useDashboardLayout.ts
git commit -m "feat(dashboard): add DashboardGrid and VisualDashboard orchestrator"
```

### Task 9: Widget config components

**Files:**
- Create: `web/src/features/dashboard/components/config/DataSourceSelector.tsx`
- Create: `web/src/features/dashboard/components/config/DataSourceField.tsx`
- Create: `web/src/features/dashboard/components/config/DisplayOptions.tsx`
- Create: `web/src/features/dashboard/components/config/ActionConfig.tsx`
- Create: `web/src/features/dashboard/components/config/WidgetConfigPanel.tsx`
- Create: `web/src/features/dashboard/components/InstallWidgetDialog.tsx`

- [ ] **Step 1: Write DataSourceSelector** (~300 lines)

Replaces the bulk of `UnifiedDataSourceConfig.tsx`. Picks source type (device/extension/system/etc) and target entity. Uses TanStack Query hooks for entity lists.

- [ ] **Step 2: Write DataSourceField** (~200 lines)

Field-level picker within a data source: select metric/field/command. Adapts based on source type.

- [ ] **Step 3: Write DisplayOptions** (~200 lines)

Widget display configuration: colors, thresholds, formatting, units.

- [ ] **Step 4: Write ActionConfig** (~150 lines)

Action configuration: commands to execute on tap/click.

- [ ] **Step 5: Write WidgetConfigPanel** (~200 lines)

Shell that combines all config sections. Slide-out panel in edit mode.

- [ ] **Step 6: Write InstallWidgetDialog** (~200 lines)

Widget library browser: grouped by category, search, preview. Uses registry metadata.

- [ ] **Step 7: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 8: Commit**

```bash
git add web/src/features/dashboard/components/
git commit -m "feat(dashboard): add widget config components and install dialog"
```

### Task 10: Widget registries

**Files:**
- Create: `web/src/features/dashboard/widgets/registry.ts`
- Create: `web/src/features/dashboard/widgets/DynamicRegistry.ts`
- Create: `web/src/features/dashboard/widgets/CommunityRegistry.ts`

- [ ] **Step 1: Write static registry** (`registry.ts`, ~400 lines)

Port from existing `registry.ts`. Each built-in widget entry:
```typescript
{
  type: 'value_card',
  displayName: 'Value Card',
  icon: Gauge,
  defaultSize: { w: 3, h: 2 },
  sizeConstraints: { minW: 2, minH: 2, maxW: 6, maxH: 4 },
  component: React.lazy(() => import('./ValueCard/ValueCard')),
  configComponent: React.lazy(() => import('./ValueCard/ValueCardConfig')),
}
```
Include `groupComponentsByCategory()` and `getCategoryInfo()` utilities.

- [ ] **Step 2: Write DynamicRegistry** (~200 lines)

Port from existing `DynamicRegistry.ts`. Manages extension-provided widgets:
- Dynamic registration/unregistration
- Loader functions for IIFE bundles
- `window.React` / `window.ReactDOM` injection
- Concurrent load limiting (max 3)
- Lifecycle hooks (mount/unmount)

- [ ] **Step 3: Write CommunityRegistry** (~200 lines)

Port from existing `CommunityRegistry.ts`. Same pattern as DynamicRegistry but for marketplace components.

- [ ] **Step 4: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 5: Commit**

```bash
git add web/src/features/dashboard/widgets/registry.ts web/src/features/dashboard/widgets/DynamicRegistry.ts web/src/features/dashboard/widgets/CommunityRegistry.ts
git commit -m "feat(dashboard): add static, dynamic, and community widget registries"
```

---

## Phase 4: Widget Migration

Each widget is migrated independently. Pattern for each:

1. Create widget directory under `widgets/`
2. Port the render component from `components/dashboard/generic/`
3. Create config component (extract config UI from existing config system)
4. Update registry entry to point to new files
5. Verify the widget renders in isolation

### Task 11: ValueCard + LedIndicator + Sparkline + ProgressBar (simple widgets)

**Files:**
- Create: `web/src/features/dashboard/widgets/ValueCard/ValueCard.tsx` + `ValueCardConfig.tsx`
- Create: `web/src/features/dashboard/widgets/LedIndicator/LedIndicator.tsx` + `LedIndicatorConfig.tsx`
- Create: `web/src/features/dashboard/widgets/Sparkline/Sparkline.tsx` + `SparklineConfig.tsx`
- Create: `web/src/features/dashboard/widgets/ProgressBar/ProgressBar.tsx` + `ProgressBarConfig.tsx`

- [ ] **Step 1: Port ValueCard** (~400 lines)

From `components/dashboard/generic/ValueCard.tsx` (543 lines). Receives `WidgetProps`, uses `dataSource` from props instead of calling `useDataSource` directly. Keep all existing display features: trend, raw data, styling presets.

- [ ] **Step 2: Port LedIndicator** (~250 lines)

From `components/dashboard/generic/LEDIndicator.tsx` (324 lines). LED state rules preserved.

- [ ] **Step 3: Port Sparkline** (~450 lines)

From `components/dashboard/generic/Sparkline.tsx` (592 lines). Time-series aggregation preserved.

- [ ] **Step 4: Port ProgressBar** (~300 lines)

From `components/dashboard/generic/ProgressBar.tsx` (379 lines).

- [ ] **Step 5: Create config components for each**

Simple config UIs extracted from `UnifiedDataSourceConfig` patterns.

- [ ] **Step 6: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 7: Commit**

```bash
git add web/src/features/dashboard/widgets/{ValueCard,LedIndicator,Sparkline,ProgressBar}/
git commit -m "feat(dashboard): migrate simple widgets — ValueCard, LedIndicator, Sparkline, ProgressBar"
```

### Task 12: Chart widgets (LineChart + BarChart + PieChart + AreaChart)

**Files:**
- Create: `web/src/features/dashboard/widgets/LineChart/LineChart.tsx` + `LineChartConfig.tsx`
- Create: `web/src/features/dashboard/widgets/BarChart/BarChart.tsx` + `BarChartConfig.tsx`
- Create: `web/src/features/dashboard/widgets/PieChart/PieChart.tsx` + `PieChartConfig.tsx`
- Create: `web/src/features/dashboard/widgets/AreaChart/AreaChart.tsx` + `AreaChartConfig.tsx`

- [ ] **Step 1: Port LineChart** (~600 lines)

From `components/dashboard/generic/LineChart.tsx` (789 lines). Key: receives `ResolvedDataSource` with `timeSeries` data. Keep all view modes (timeseries/snapshot/comparison).

- [ ] **Step 2: Port AreaChart** (~300 lines)

Currently part of `LineChart.tsx` (exported as AreaChart). Extract into its own widget.

- [ ] **Step 3: Port BarChart** (~400 lines)

From `components/dashboard/generic/BarChart.tsx` (504 lines).

- [ ] **Step 4: Port PieChart** (~350 lines)

From `components/dashboard/generic/PieChart.tsx` (427 lines).

- [ ] **Step 5: Create chart config components**

Time window selector, aggregation method, color scale, axis config.

- [ ] **Step 6: Create color scale utils** (`utils/colorScales.ts`, ~100 lines)

Extract color scale logic shared across charts.

- [ ] **Step 7: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 8: Commit**

```bash
git add web/src/features/dashboard/widgets/{LineChart,BarChart,PieChart,AreaChart}/ web/src/features/dashboard/utils/colorScales.ts
git commit -m "feat(dashboard): migrate chart widgets — LineChart, BarChart, PieChart, AreaChart"
```

### Task 13: Display widgets (ImageDisplay + ImageHistory + WebDisplay + MarkdownDisplay + ToggleSwitch)

**Files:**
- Create directories under `widgets/` for each

- [ ] **Step 1: Port ToggleSwitch** (~350 lines)

From `components/dashboard/generic/ToggleSwitch.tsx` (465 lines). Command execution preserved.

- [ ] **Step 2: Port ImageDisplay** (~500 lines)

From `components/dashboard/generic/ImageDisplay.tsx` (724 lines). Zoom and presets preserved.

- [ ] **Step 3: Port ImageHistory** (~450 lines)

From `components/dashboard/generic/ImageHistory.tsx` (664 lines). Timeline navigation preserved.

- [ ] **Step 4: Port WebDisplay** (~200 lines)

From `components/dashboard/generic/WebDisplay.tsx` (287 lines).

- [ ] **Step 5: Port MarkdownDisplay** (~150 lines)

From `components/dashboard/generic/MarkdownDisplay.tsx` (212 lines).

- [ ] **Step 6: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 7: Commit**

```bash
git add web/src/features/dashboard/widgets/{ToggleSwitch,ImageDisplay,ImageHistory,WebDisplay,MarkdownDisplay}/
git commit -m "feat(dashboard): migrate display widgets — ToggleSwitch, ImageDisplay, ImageHistory, WebDisplay, MarkdownDisplay"
```

### Task 14: MapDisplay (decomposed)

**Files:**
- Create: `web/src/features/dashboard/widgets/MapDisplay/MapDisplay.tsx`
- Create: `web/src/features/dashboard/widgets/MapDisplay/MapMarkerLayer.tsx`
- Create: `web/src/features/dashboard/widgets/MapDisplay/MapCommandHandler.tsx`
- Create: `web/src/features/dashboard/widgets/MapDisplay/MapDisplayConfig.tsx`

- [ ] **Step 1: Extract MapMarkerLayer** (~300 lines)

Marker rendering logic extracted from the 1206-line MapDisplay.

- [ ] **Step 2: Extract MapCommandHandler** (~200 lines)

Device command execution on map markers.

- [ ] **Step 3: Write MapDisplay** (~400 lines)

Main map component composing the extracted layers. Leaflet integration preserved.

- [ ] **Step 4: Write MapDisplayConfig** (~150 lines)

Map configuration: center, zoom, marker bindings.

- [ ] **Step 5: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 6: Commit**

```bash
git add web/src/features/dashboard/widgets/MapDisplay/
git commit -m "feat(dashboard): migrate MapDisplay — decomposed into MapMarkerLayer + MapCommandHandler"
```

### Task 15: VideoDisplay + CustomLayer (decomposed)

**Files:**
- Create: `web/src/features/dashboard/widgets/VideoDisplay/VideoDisplay.tsx` + `VideoDisplayConfig.tsx`
- Create: `web/src/features/dashboard/widgets/CustomLayer/CustomLayer.tsx`
- Create: `web/src/features/dashboard/widgets/CustomLayer/LayerCanvas.tsx`
- Create: `web/src/features/dashboard/widgets/CustomLayer/LayerToolbar.tsx`
- Create: `web/src/features/dashboard/widgets/CustomLayer/CustomLayerConfig.tsx`

- [ ] **Step 1: Port VideoDisplay** (~500 lines)

From `components/dashboard/generic/VideoDisplay.tsx` (807 lines). HLS, MJPEG, keep-alive preserved. Clean up event listener lifecycle.

- [ ] **Step 2: Extract LayerCanvas** (~400 lines)

Canvas rendering logic from 1396-line CustomLayer.

- [ ] **Step 3: Extract LayerToolbar** (~200 lines)

Drawing tools toolbar.

- [ ] **Step 4: Write CustomLayer** (~400 lines)

Main component composing canvas + toolbar.

- [ ] **Step 5: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 6: Commit**

```bash
git add web/src/features/dashboard/widgets/{VideoDisplay,CustomLayer}/
git commit -m "feat(dashboard): migrate VideoDisplay + CustomLayer — decomposed into focused sub-components"
```

### Task 16: AgentMonitor + AiAnalyst (business widgets)

**Files:**
- Create: `web/src/features/dashboard/widgets/AgentMonitor/AgentMonitorWidget.tsx`
- Create: `web/src/features/dashboard/widgets/AgentMonitor/AgentMessageList.tsx`
- Create: `web/src/features/dashboard/widgets/AgentMonitor/AgentMonitorConfig.tsx`
- Create: `web/src/features/dashboard/widgets/AiAnalyst/` (port from ai-analyst/)

- [ ] **Step 1: Extract AgentMessageList** (~300 lines)

Message rendering from 1150-line AgentMonitorWidget.

- [ ] **Step 2: Write AgentMonitorWidget** (~400 lines)

Status visualization + performance metrics.

- [ ] **Step 3: Port AiAnalyst** (~600 lines)

Port from `components/dashboard/generic/ai-analyst/` (7 files, ~1673 lines total). Agent execution, timeline, input preserved.

- [ ] **Step 4: Verify compilation**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 5: Commit**

```bash
git add web/src/features/dashboard/widgets/{AgentMonitor,AiAnalyst}/
git commit -m "feat(dashboard): migrate business widgets — AgentMonitor, AiAnalyst"
```

---

## Phase 5: Cutover

### Task 17: Route migration + cleanup

**Files:**
- Modify: Router config (wherever dashboard route is defined)
- Remove: Old dashboard files

- [ ] **Step 1: Update router to use new VisualDashboard**

Change the dashboard route to import from `@/features/dashboard/components/VisualDashboard` instead of old path.

- [ ] **Step 2: Test the new dashboard in browser**

Run: `cd web && npm run dev`

Verify:
- Dashboard list loads
- Dashboard grid renders with widgets
- Edit mode toggle works
- Widget config panel opens
- Data sources fetch live data
- Real-time updates via WebSocket
- Responsive layout works
- Shared dashboard (read-only) works
- Extension widgets load dynamically

- [ ] **Step 3: Remove old dashboard files**

```bash
git rm -r web/src/components/dashboard/
git rm web/src/store/slices/dashboardSlice.ts
git rm web/src/hooks/useDataSource.ts
git rm web/src/hooks/useDashboardPrefetch.ts
git rm web/src/pages/dashboard-components/VisualDashboard.tsx
```

Keep: `web/src/lib/utils/async.ts` (fetchCache used elsewhere)

- [ ] **Step 4: Fix any broken imports across the codebase**

Search for imports referencing deleted files and update them.

- [ ] **Step 5: Final TypeScript check**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 6: Final build check**

Run: `cd web && npm run build`

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(dashboard): cutover to new dashboard system, remove legacy code"
```

---

## Notes

- **Backward compatibility**: All new code lives under `features/dashboard/`. Existing dashboard works until Phase 5 cutover.
- **Test command**: `cd web && npx tsc --noEmit` after every task.
- **No new npm packages needed**: TanStack Query v5 and Zod v4 are already installed.
- **Persistence**: Existing `DashboardStorage` abstraction is reused, not replaced.
