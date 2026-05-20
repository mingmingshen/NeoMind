# Dashboard Full Rewrite Design Spec

**Date:** 2026-05-20
**Status:** Approved
**Scope:** Feature-complete rewrite of the dashboard system

## Problem Statement

The current dashboard system has accumulated significant technical debt:

- **Monolithic files**: `UnifiedDataSourceConfig.tsx` (2,504 lines), `CustomLayer.tsx` (1,396 lines), `MapDisplay.tsx` (1,206 lines), `AgentMonitorWidget.tsx` (1,150 lines)
- **Complex state management**: Single `dashboardSlice.ts` (964 lines) handles CRUD, layout, UI state, and persistence
- **Manual data caching**: `useDataSource.ts` (28K tokens) with manual `fetchCache`/`markFetching`/`markFetched` pattern and global RAF batching
- **Type safety issues**: 83 `as any` casts across 15 dashboard files, loose typing between API and frontend
- **Zero test coverage** for dashboard components
- **Performance**: Memory leak potential in shared event system, unnecessary re-renders

## Architecture

### Approach: TanStack Query + Zustand Hybrid

- **TanStack Query** owns all data fetching, caching, and real-time updates
- **Zustand** owns only UI state (layout positions, edit mode, dialog state)
- **Widgets** are self-contained islands that own their data subscriptions

```
┌─────────────────────────────────────────────────┐
│                   VisualDashboard                │
│         (Layout Orchestrator, drag-and-drop)     │
├─────────────────────────────────────────────────┤
│  DashboardGrid                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │ Widget A │ │ Widget B │ │ Widget C │  ...    │
│  │ (island) │ │ (island) │ │ (island) │        │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘        │
│       │              │              │            │
├───────┴──────────────┴──────────────┴────────────┤
│              Data Layer (TanStack Query)          │
│  ┌────────────────┐  ┌─────────────────────┐    │
│  │ query-key tree │  │ per-source-type      │    │
│  │ ['dashboard',  │  │ hooks:               │    │
│  │  id, 'data',   │  │ useDeviceTelemetry() │    │
│  │  source]       │  │ useExtensionMetric() │    │
│  └────────────────┘  │ useSystemMetric()    │    │
│                      └─────────────────────┘    │
├─────────────────────────────────────────────────┤
│              UI State (Zustand)                   │
│  dashboardCrudSlice  │ dashboardLayoutSlice      │
│  dashboardEditSlice  │ dashboardConfigSlice      │
└─────────────────────────────────────────────────┘
```

## State Management (Zustand)

Replace the single 964-line `dashboardSlice.ts` with 4 focused slices:

| Slice | Responsibility | Size estimate |
|-------|---------------|---------------|
| `dashboardCrudSlice` | Load/save/delete dashboards, share/unshare, persistence coordination | ~300 lines |
| `dashboardLayoutSlice` | Grid layout positions, add/remove/move widgets | ~250 lines |
| `dashboardEditSlice` | Edit mode, selected widget, config dialog state | ~150 lines |
| `dashboardConfigSlice` | Widget config CRUD, data source binding, display options | ~250 lines |

**Removed from Zustand:**
- `fetchCache` / `markFetching` / `markFetched` → TanStack Query
- Telemetry data caching → TanStack Query
- WebSocket event dedup → TanStack Query `queryClient.setQueryData`

**Composition pattern (valid Zustand API):**
```typescript
// Each slice is a StateCreator function
type DashboardStore = CrudSlice & LayoutSlice & EditSlice & ConfigSlice;

const useDashboardStore = create<DashboardStore>()(
  (...a) => ({
    ...dashboardCrudSlice(...a),
    ...dashboardLayoutSlice(...a),
    ...dashboardEditSlice(...a),
    ...dashboardConfigSlice(...a),
  })
);
```

### Persistence Strategy

The existing `DashboardStorage` abstraction (`web/src/store/persistence/`) with its DTO conversion layer (`fromDashboardDTO`/`toDashboardDTO`) is kept. The `dashboardCrudSlice` uses TanStack Query mutations for server sync, but the persistence layer provides:

1. **Hybrid storage**: localStorage for fast reads + API for authoritative state
2. **DTO conversion**: snake_case (API) ↔ camelCase (frontend) via `fromDashboardDTO()`
3. **ID management**: `handleIdChange` for server-assigned IDs after create

```typescript
// dashboardCrudSlice integrates persistence
const dashboardCrudSlice: StateCreator<...> = (set, get) => ({
  dashboards: [],
  currentDashboardId: null,

  loadDashboards: async () => {
    // TanStack Query mutation with optimistic update
    const dashboards = await dashboardApi.list();
    const converted = dashboards.map(fromDashboardDTO);
    set({ dashboards: converted });
  },

  saveDashboard: useDebouncedMutation(async (dashboard) => {
    const dto = toDashboardDTO(dashboard);
    const saved = await dashboardApi.update(dto);
    // handleIdChange if server assigned new ID
    return fromDashboardDTO(saved);
  }, { delay: 500 }),
});
```

The existing `fetchCache` utility in `web/src/lib/utils/async.ts` is **kept** for non-dashboard code that still uses it. Only dashboard-internal usage is removed.

## Data Layer (TanStack Query)

> **Note:** `@tanstack/react-query@^5.90.21` is already installed with `QueryClientProvider` in `web/src/main.tsx` and existing hooks in `web/src/lib/react-query-hooks.ts`. This reuses that infrastructure.

### Query Key Tree

Keys are namespaced under `dashboard` to avoid collision with existing keys in `react-query-hooks.ts`:

```
['dashboard', 'list']                              → all dashboards list
['dashboard', id]                                  → single dashboard document
['dashboard', id, 'widget', wid]                   → widget config
['dashboard', 'telemetry', sourceId, timeWindow]   → time-series data
['dashboard', 'device', deviceId, 'metrics']       → device metrics
['dashboard', 'extension', extId, 'metrics']       → extension metrics
['dashboard', 'system', metricType]                → system metrics
```

Existing non-dashboard query keys (`['devices']`, `['extensions']`) in `react-query-hooks.ts` remain unchanged.

### Per-Source-Type Hooks

Replace the monolithic `useDataSource.ts` with focused hooks (~100-200 lines each):

```typescript
useDeviceTelemetry(source, timeWindow)   // polling + WebSocket
useDeviceMetric(source)                  // device metric polling
useDeviceCommand(source)                 // command status
useExtensionMetric(source)               // extension metric polling
useExtensionCommand(source)              // extension command
useSystemMetric(source)                  // system metric polling
useAiMetric(source)                      // AI metric polling
useTransformData(source)                 // transform automation output
useAgentData(source)                     // agent data
useStaticValue(source)                   // no fetching, returns value
useWidgetDataSource(source, timeWindow)  // unified wrapper
```

### Real-time Updates Bridge

A dedicated `DashboardEventBridge` component manages WebSocket → TanStack Query cache updates:

```typescript
// web/src/features/dashboard/components/DashboardEventBridge.tsx
// Mounts inside VisualDashboard, subscribes to relevant WebSocket events

function DashboardEventBridge({ dashboardId }: { dashboardId: string }) {
  const queryClient = useQueryClient();

  // Reuse existing useEvents hook for WS/SSE connection
  useEvents({
    onDeviceTelemetry: (msg) => {
      queryClient.setQueryData(
        ['dashboard', 'telemetry', msg.sourceId, currentTimeWindow],
        (old: TelemetryData[] | undefined) => appendDataPoint(old, msg.data)
      );
    },
    onExtensionMetric: (msg) => {
      queryClient.setQueryData(
        ['dashboard', 'extension', msg.extensionId, 'metrics'],
        (old) => mergeMetric(old, msg.data)
      );
    },
  });

  return null; // no UI
}
```

**Connection lifecycle:**
- Uses existing `useEvents` hook (supports both WebSocket and SSE via `useSSE` flag)
- Auth and reconnect handled by the existing event infrastructure
- Scoped per dashboard: only subscribes to events for the current dashboard's data sources
- Cleanup: when `VisualDashboard` unmounts, `DashboardEventBridge` unsubscribes

**Performance:** TanStack Query's `structuralSharing` (default) performs referential equality checks on data. Time-series data uses `appendDataPoint` which only creates a new array reference when new data arrives, preventing unnecessary re-renders.

### Cache Invalidation

```typescript
queryClient.invalidateQueries({ queryKey: ['dashboard', 'device', deviceId] });
queryClient.invalidateQueries({ queryKey: ['dashboard', dashboardId] });
// Wildcard: invalidate all dashboard data
queryClient.invalidateQueries({ queryKey: ['dashboard'] });
```

### Prefetch Strategy

Replace `useDashboardPrefetch.ts` with TanStack Query's `queryClient.prefetchQuery`:

```typescript
// In VisualDashboard, on mount or when dashboard loads
const prefetchTelemetry = useCallback(async (sources: DataSource[]) => {
  const CONCURRENCY = 3;
  // Batch prefetch with concurrency limit (same as current logic)
  for (let i = 0; i < sources.length; i += CONCURRENCY) {
    const batch = sources.slice(i, i + CONCURRENCY);
    await Promise.all(batch.map(s =>
      queryClient.prefetchQuery({
        queryKey: ['dashboard', 'telemetry', s.sourceId, s.timeWindow],
        queryFn: () => telemetryApi.fetch(s.sourceId, s.timeWindow),
      })
    ));
  }
}, [queryClient]);
```

### fetchCache Replacement

**Before:**
```typescript
if (shouldFetch(key)) {
  markFetching(key);
  const data = await api.fetch();
  markFetched(key, data);
}
```

**After:**
```typescript
const { data } = useQuery({
  queryKey: ['dashboard', 'telemetry', sourceId, timeWindow],
  queryFn: () => api.fetchTelemetry(sourceId, timeWindow),
  staleTime: 10_000,
});
```

## Type Safety

### Discriminated Union Data Sources

Complete list matching current `DataSourceType` values:

```typescript
type DataSource =
  | { type: 'device'; deviceId: string; field: string; transform?: TransformConfig }
  | { type: 'metric'; deviceId: string; field: string }
  | { type: 'command'; deviceId: string; commandId: string }
  | { type: 'telemetry'; deviceId: string; field: string; transform?: TransformConfig }
  | { type: 'device-info'; deviceId: string }
  | { type: 'extension'; extensionId: string; field: string }
  | { type: 'extension-metric'; extensionId: string; field: string }
  | { type: 'extension-command'; extensionId: string; commandId: string }
  | { type: 'system'; metric: SystemMetricType }
  | { type: 'transform'; transformId: string; outputField: string }
  | { type: 'ai-metric'; metric: AiMetricType }
  | { type: 'agent'; agentId: string }
  | { type: 'static'; value: string; unit?: string };
```

### Typed Widget Configs

Complete list of all 19 widget types (17 generic + 2 business):

```typescript
type WidgetConfigMap = {
  // Generic (17)
  value_card: ValueCardConfig;
  led_indicator: LedIndicatorConfig;
  sparkline: SparklineConfig;
  progress_bar: ProgressBarConfig;
  line_chart: LineChartConfig;
  bar_chart: BarChartConfig;
  pie_chart: PieChartConfig;
  area_chart: AreaChartConfig;
  toggle_switch: ToggleSwitchConfig;
  image_display: ImageDisplayConfig;
  image_history: ImageHistoryConfig;
  web_display: WebDisplayConfig;
  markdown_display: MarkdownDisplayConfig;
  map_display: MapDisplayConfig;
  video_display: VideoDisplayConfig;
  custom_layer: CustomLayerConfig;
  // Business (2)
  agent_monitor_widget: AgentMonitorConfig;
  ai_analyst: AiAnalystConfig;
};

type WidgetConfig<T extends WidgetType = WidgetType> = {
  type: T;
  dataSource: DataSource | null;
  display: WidgetConfigMap[T]['display'];
  actions: WidgetConfigMap[T]['actions'];
};
```

### ResolvedDataSource Type

The type that bridges raw config to live widget data:

```typescript
type ResolvedDataSource = {
  // The raw config from the dashboard document
  source: DataSource;
  // The current live value (single point for gauges, array for charts)
  value: number | string | null;
  // For time-series sources: historical data points
  timeSeries?: { timestamp: number; value: number }[];
  // Loading/error state from TanStack Query
  isLoading: boolean;
  error: Error | null;
  // For sources with a unit
  unit?: string;
  // Last updated timestamp
  lastUpdated?: number;
};
```

### Zod Schema Validation

API responses validated at the boundary with Zod schemas (`zod@^4.3.5` is already installed):

```typescript
const DashboardResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  widgets: z.array(WidgetConfigSchema),
  layout: z.array(LayoutItemSchema),
  // ...
});
```

No `as any` casts — discriminated unions enable TypeScript narrowing.

## Widget Registry

### Static Widget Registry (built-in)

```typescript
interface WidgetDefinition {
  type: WidgetType;
  displayName: string;
  icon: LucideIcon;
  defaultSize: { w: number; h: number };
  sizeConstraints: SizeConstraints;
  component: React.LazyExoticComponent<WidgetProps>;
  configComponent: React.LazyExoticComponent<WidgetConfigProps>;
}
```

### Dynamic Registry (extension-provided widgets)

Extension widgets loaded at runtime via IIFE bundles served by the backend. The `WidgetDefinition` interface is extended:

```typescript
type StaticWidgetDefinition = {
  source: 'static';
  component: React.LazyExoticComponent<WidgetProps>;
  configComponent: React.LazyExoticComponent<WidgetConfigProps>;
  // ...standard fields
};

type DynamicWidgetDefinition = {
  source: 'dynamic';
  loader: () => Promise<{ default: React.ComponentType<WidgetProps> }>;
  configLoader: () => Promise<{ default: React.ComponentType<WidgetConfigProps> }>;
  // Lifecycle hooks
  onMount?: () => void;
  onUnmount?: () => void;
  // ...standard fields
};

type WidgetDefinition = StaticWidgetDefinition | DynamicWidgetDefinition;
```

**Extension lifecycle:**
- `useExtensionLifecycle` hook manages registration/cleanup when extensions are installed/removed
- Dynamic widgets inject `window.React` / `window.ReactDOM` globals for IIFE bundles
- Extension widgets are wrapped in `React.Suspense` + `ErrorBoundary` (see WidgetShell below)
- Community marketplace widgets use `CommunityRegistry` with the same `DynamicWidgetDefinition` pattern

### Widget Props Interface

```typescript
interface WidgetProps {
  widgetId: string;
  config: WidgetConfig;
  dataSource: ResolvedDataSource | null;
  isEditing: boolean;
}
```

## Component Architecture

### WidgetShell (Error Isolation)

Every widget is wrapped in `WidgetShell` which provides:

1. **React Error Boundary** — if a widget crashes (especially dynamic extension widgets), only that widget shows a fallback UI, not the entire dashboard
2. **Loading state** — skeleton while the widget lazy-loads
3. **Widget chrome** — title bar, resize handle, config button (in edit mode)
4. **Drag handle** — in edit mode

```typescript
function WidgetShell({ widgetId, isEditing }: { widgetId: string; isEditing: boolean }) {
  const definition = useWidgetDefinition(widgetId);
  const config = useWidgetConfig(widgetId);
  const dataSource = useWidgetDataSource(config.dataSource);

  return (
    <ErrorBoundary
      fallback={<WidgetErrorFallback widgetId={widgetId} />}
    >
      <Suspense fallback={<WidgetSkeleton />}>
        <definition.component
          widgetId={widgetId}
          config={config}
          dataSource={dataSource}
          isEditing={isEditing}
        />
      </Suspense>
    </ErrorBoundary>
  );
}
```

### Mobile / Responsive Layout

The rewrite preserves the existing responsive behavior:

- `useIsMobile()` and `useTouchHover()` hooks are reused from the existing codebase
- `DashboardGrid` uses react-grid-layout responsive breakpoints (`lg`, `md`, `sm`, `xs`)
- Mobile layout uses compact layout with single-column fallback
- Touch interactions for drag-and-drop are preserved
- `CustomLayer` canvas touch events are preserved in the decomposed `LayerCanvas`

### Shared Dashboard (Read-only) Mode

The `dashboardEditSlice` tracks a `isReadOnly` flag set when viewing a shared dashboard:

- No edit mode toggle available
- No config dialog
- `WidgetShell` hides chrome (title bar actions, resize handle)
- Data sources still fetch live data (read access to telemetry)
- `ShareManagerDialog` is preserved and moved to `features/dashboard/components/`

## File Structure

```
web/src/features/dashboard/
├── api/
│   ├── dashboards.ts                  # CRUD API calls
│   └── telemetry.ts                   # telemetry fetch functions
├── hooks/
│   ├── useWidgetDataSource.ts         # unified data source hook
│   ├── useDeviceTelemetry.ts          # device polling + WS
│   ├── useDeviceMetric.ts             # device metric polling
│   ├── useDeviceCommand.ts            # device command status
│   ├── useExtensionMetric.ts          # extension metric polling
│   ├── useExtensionCommand.ts         # extension command
│   ├── useSystemMetric.ts             # system metric polling
│   ├── useAiMetric.ts                 # AI metric polling
│   ├── useTransformData.ts            # transform output
│   ├── useStaticValue.ts              # static value (no fetch)
│   └── useDashboardLayout.ts          # grid layout management
├── store/
│   ├── dashboardCrudSlice.ts
│   ├── dashboardLayoutSlice.ts
│   ├── dashboardEditSlice.ts
│   ├── dashboardConfigSlice.ts
│   └── index.ts
├── components/
│   ├── VisualDashboard.tsx            # main page orchestrator
│   ├── DashboardGrid.tsx              # react-grid-layout wrapper
│   ├── DashboardEventBridge.tsx       # WS/SSE → TanStack Query bridge
│   ├── WidgetShell.tsx                # error boundary + chrome + loading
│   ├── WidgetErrorFallback.tsx        # error fallback UI
│   ├── WidgetSkeleton.tsx             # loading skeleton
│   ├── InstallWidgetDialog.tsx        # add widget dialog
│   ├── ShareManagerDialog.tsx         # share dashboard dialog
│   └── config/
│       ├── WidgetConfigPanel.tsx      # config panel shell
│       ├── DataSourceSelector.tsx     # picks source type + target
│       ├── DataSourceField.tsx        # single field picker
│       ├── DisplayOptions.tsx         # display config
│       └── ActionConfig.tsx           # action config
├── widgets/
│   ├── ValueCard/
│   │   ├── ValueCard.tsx
│   │   └── ValueCardConfig.tsx
│   ├── LedIndicator/
│   ├── Sparkline/
│   ├── ProgressBar/
│   ├── LineChart/
│   ├── BarChart/
│   ├── PieChart/
│   ├── AreaChart/
│   ├── ToggleSwitch/
│   ├── ImageDisplay/
│   ├── ImageHistory/
│   ├── WebDisplay/
│   ├── MarkdownDisplay/
│   ├── MapDisplay/
│   │   ├── MapDisplay.tsx
│   │   ├── MapMarkerLayer.tsx
│   │   ├── MapCommandHandler.tsx
│   │   └── MapDisplayConfig.tsx
│   ├── VideoDisplay/
│   ├── CustomLayer/
│   │   ├── CustomLayer.tsx
│   │   ├── LayerCanvas.tsx
│   │   ├── LayerToolbar.tsx
│   │   └── CustomLayerConfig.tsx
│   ├── AgentMonitor/
│   │   ├── AgentMonitorWidget.tsx
│   │   ├── AgentMessageList.tsx
│   │   └── AgentMonitorConfig.tsx
│   ├── AiAnalyst/
│   ├── registry.ts                    # static built-in registry
│   ├── DynamicRegistry.ts             # extension-provided widgets
│   └── CommunityRegistry.ts           # marketplace widgets
├── types/
│   ├── dashboard.ts                   # core dashboard types
│   ├── dataSources.ts                 # all 13 data source types
│   └── widgets.ts                     # widget config types + WidgetConfigMap
├── utils/
│   ├── telemetryTransform.ts          # aggregation, time windows
│   └── colorScales.ts                 # chart color handling
└── __tests__/
    ├── hooks/
    │   ├── useDeviceTelemetry.test.ts
    │   ├── useWidgetDataSource.test.ts
    │   └── useDashboardLayout.test.ts
    ├── store/
    │   └── dashboardCrudSlice.test.ts
    └── widgets/
        ├── ValueCard.test.tsx
        └── LineChart.test.tsx
```

## Migration Plan

Build new system alongside the old. No breaking changes until final cutover.

### Phase 1: Foundation
1. Create `web/src/features/dashboard/` directory structure
2. Rewrite dashboard types with strict generics (no `as any`)
3. Build API client functions (`api/dashboards.ts`, `api/telemetry.ts`)
4. Build 4 Zustand slices (integrate with existing `DashboardStorage` persistence)
5. **Test deliverable:** Type compilation passes, store unit tests pass

### Phase 2: Data Layer
6. Implement `useDeviceTelemetry` hook
7. Implement `useDeviceMetric` hook
8. Implement `useDeviceCommand` hook
9. Implement `useExtensionMetric` hook
10. Implement `useExtensionCommand` hook
11. Implement `useSystemMetric` hook
12. Implement `useAiMetric` hook
13. Implement `useTransformData` hook
14. Implement `useStaticValue` hook
15. Implement `useWidgetDataSource` wrapper
16. Build `DashboardEventBridge` (WS/SSE → queryClient bridge)
17. Implement `telemetryTransform.ts` utils
18. Implement prefetch strategy
19. **Test deliverable:** Hook unit tests with mocked queryClient, data layer integration test

### Phase 3: Core Components
20. Build `WidgetShell` with Error Boundary + Suspense + Skeleton
21. Build `VisualDashboard` orchestrator
22. Build `DashboardGrid` (react-grid-layout with responsive breakpoints)
23. Build `InstallWidgetDialog`
24. Build `DataSourceSelector` config component
25. Build remaining config components (`DataSourceField`, `DisplayOptions`, `ActionConfig`)
26. Build `WidgetConfigPanel` shell
27. Build `ShareManagerDialog`
28. Build `DynamicRegistry` + `CommunityRegistry` with lifecycle hooks
29. **Test deliverable:** Component render tests, config panel interaction tests

### Phase 4: Widget Migration (one at a time, each independently testable)
30. ValueCard
31. LedIndicator
32. Sparkline
33. ProgressBar
34. LineChart
35. BarChart
36. PieChart
37. AreaChart
38. ToggleSwitch
39. ImageDisplay
40. ImageHistory
41. WebDisplay
42. MarkdownDisplay
43. MapDisplay (decomposed: MapDisplay + MapMarkerLayer + MapCommandHandler)
44. VideoDisplay
45. CustomLayer (decomposed: CustomLayer + LayerCanvas + LayerToolbar)
46. AgentMonitor (decomposed: AgentMonitorWidget + AgentMessageList)
47. AiAnalyst
48. **Test deliverable:** Each widget has render test + data binding test before moving to next

### Phase 5: Cutover
49. Wire new dashboard to routing (replace old VisualDashboard)
50. Migrate shared dashboard routes
51. Remove old `components/dashboard/` directory
52. Remove old `dashboardSlice.ts`
53. Remove old `useDataSource.ts`, `useDashboardPrefetch.ts`
54. Keep `fetchCache` utility (used by non-dashboard code)
55. **Test deliverable:** E2E manual testing, visual regression check

## Key Decompositions

| Current monolith | Decomposed into |
|-----------------|-----------------|
| `UnifiedDataSourceConfig.tsx` (2,504 lines) | `DataSourceSelector` + `DataSourceField` + `DisplayOptions` + `ActionConfig` (~300 lines each) |
| `CustomLayer.tsx` (1,396 lines) | `CustomLayer` + `LayerCanvas` + `LayerToolbar` (~400 lines each) |
| `MapDisplay.tsx` (1,206 lines) | `MapDisplay` + `MapMarkerLayer` + `MapCommandHandler` (~350 lines each) |
| `AgentMonitorWidget.tsx` (1,150 lines) | `AgentMonitorWidget` + `AgentMessageList` (~400 lines each) |
| `useDataSource.ts` (28K tokens) | 10 focused hooks (~150 lines each) |
| `dashboardSlice.ts` (964 lines) | 4 focused slices (~250 lines each) |

## Dependencies

- `@tanstack/react-query@^5.90.21` — already installed
- `zod@^4.3.5` — already installed
- Existing: `react-grid-layout`, `recharts`, `lucide-react`, `zustand`

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| TanStack Query learning curve | Phase 2 builds hooks incrementally, starting with simplest |
| Data parity with old system | Each widget tested individually in Phase 4 |
| Cutover regression | Phase 5 keeps old code until new system is verified; can rollback |
| Scope creep | Each phase has clear deliverables; no new features during rewrite |
| Dynamic extension widget compatibility | DynamicRegistry + CommunityRegistry ported in Phase 3 with lifecycle hooks |
| Persistence layer integration risk | Existing DashboardStorage abstraction kept, CRUD slice wraps it |
| Shared dashboard mode | Addressed in dashboardEditSlice with isReadOnly flag |
