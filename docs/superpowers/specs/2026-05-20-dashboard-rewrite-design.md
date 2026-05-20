# Dashboard Full Rewrite Design Spec

**Date:** 2026-05-20
**Status:** Approved
**Scope:** Feature-complete rewrite of the dashboard system

## Problem Statement

The current dashboard system has accumulated significant technical debt:

- **Monolithic files**: `UnifiedDataSourceConfig.tsx` (2,504 lines), `CustomLayer.tsx` (1,396 lines), `MapDisplay.tsx` (1,206 lines), `AgentMonitorWidget.tsx` (1,150 lines)
- **Complex state management**: Single `dashboardSlice.ts` (964 lines) handles CRUD, layout, UI state, and persistence
- **Manual data caching**: `useDataSource.ts` (28K tokens) with manual `fetchCache`/`markFetching`/`markFetched` pattern and global RAF batching
- **Type safety issues**: 29 `as any` casts in `UnifiedDataSourceConfig.tsx`, loose typing between API and frontend
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
| `dashboardCrudSlice` | Load/save/delete dashboards, share/unshare | ~200 lines |
| `dashboardLayoutSlice` | Grid layout positions, add/remove/move widgets | ~250 lines |
| `dashboardEditSlice` | Edit mode, selected widget, config dialog state | ~150 lines |
| `dashboardConfigSlice` | Widget config CRUD, data source binding, display options | ~200 lines |

**Removed from Zustand:**
- `fetchCache` / `markFetching` / `markFetched` → TanStack Query
- Telemetry data caching → TanStack Query
- WebSocket event dedup → TanStack Query `queryClient.setQueryData`

**Composition pattern:**
```typescript
const useDashboardStore = create(
  ...dashboardCrudSlice,
  ...dashboardLayoutSlice,
  ...dashboardEditSlice,
  ...dashboardConfigSlice,
);
```

## Data Layer (TanStack Query)

### Query Key Tree

```
['dashboards']                          → all dashboards list
['dashboards', id]                      → single dashboard document
['dashboards', id, 'widgets', wid]      → widget config
['telemetry', sourceId, timeWindow]     → time-series data
['devices']                             → device list
['devices', deviceId, 'metrics']        → device metrics
['extensions']                          → extension list
['extensions', extId, 'metrics']        → extension metrics
['system', metricType]                  → system metrics
```

### Per-Source-Type Hooks

Replace the monolithic `useDataSource.ts` with focused hooks (~100-200 lines each):

```typescript
useDeviceTelemetry(source, timeWindow)   // polling + WebSocket
useExtensionMetric(source)               // polling
useSystemMetric(source)                  // polling
useAiMetric(source)                      // polling
useStaticValue(source)                   // no fetching, returns value
useWidgetDataSource(source, timeWindow)  // unified wrapper
```

### Real-time Updates

WebSocket messages flow directly into TanStack Query cache:

```typescript
websocket.onMessage((msg) => {
  queryClient.setQueryData(
    ['telemetry', msg.sourceId, currentTimeWindow],
    (old) => mergeDataPoint(old, msg.data)
  );
});
```

No global RAF batching. TanStack Query's `structuralSharing` prevents unnecessary re-renders.

### Cache Invalidation

```typescript
queryClient.invalidateQueries({ queryKey: ['devices', deviceId] });
queryClient.invalidateQueries({ queryKey: ['dashboards', dashboardId] });
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
  queryKey: ['telemetry', sourceId, timeWindow],
  queryFn: () => api.fetchTelemetry(sourceId, timeWindow),
  staleTime: 10_000,
});
```

## Type Safety

### Discriminated Union Data Sources

```typescript
type DataSource =
  | { type: 'device_telemetry'; deviceId: string; field: string; transform?: TransformConfig }
  | { type: 'device_metric'; deviceId: string; field: string }
  | { type: 'device_command'; deviceId: string; commandId: string }
  | { type: 'extension_metric'; extensionId: string; field: string }
  | { type: 'extension_command'; extensionId: string; commandId: string }
  | { type: 'system'; metric: SystemMetricType }
  | { type: 'ai'; metric: AiMetricType }
  | { type: 'static'; value: string; unit?: string }
  // ... all 13 types
```

### Typed Widget Configs

```typescript
type WidgetConfigMap = {
  value_card: ValueCardConfig;
  line_chart: LineChartConfig;
  bar_chart: BarChartConfig;
  // ... all 18 types
};

type WidgetConfig<T extends WidgetType = WidgetType> = {
  type: T;
  dataSource: DataSource | null;
  display: WidgetConfigMap[T]['display'];
  actions: WidgetConfigMap[T]['actions'];
};
```

### Zod Schema Validation

API responses validated at the boundary with Zod schemas. No `as any` casts.

## File Structure

```
web/src/features/dashboard/
├── api/
│   ├── dashboards.ts                  # CRUD API calls
│   └── telemetry.ts                   # telemetry fetch functions
├── hooks/
│   ├── useWidgetDataSource.ts         # unified data source hook
│   ├── useDeviceTelemetry.ts          # device polling + WS
│   ├── useExtensionMetric.ts          # extension metric polling
│   ├── useSystemMetric.ts             # system metric polling
│   └── useDashboardLayout.ts          # grid layout management
├── store/
│   ├── dashboardCrudSlice.ts
│   ├── dashboardLayoutSlice.ts
│   ├── dashboardEditSlice.ts
│   ├── dashboardConfigSlice.ts
│   └── index.ts
├── components/
│   ├── VisualDashboard.tsx
│   ├── DashboardGrid.tsx
│   ├── WidgetShell.tsx
│   ├── InstallWidgetDialog.tsx
│   └── config/
│       ├── WidgetConfigPanel.tsx
│       ├── DataSourceSelector.tsx
│       ├── DataSourceField.tsx
│       ├── DisplayOptions.tsx
│       └── ActionConfig.tsx
├── widgets/
│   ├── ValueCard/
│   ├── LineChart/
│   ├── BarChart/
│   ├── PieChart/
│   ├── Sparkline/
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
│   └── registry.ts
├── types/
│   ├── dashboard.ts
│   ├── dataSources.ts
│   └── widgets.ts
└── utils/
    ├── telemetryTransform.ts
    └── colorScales.ts
```

## Widget Interface

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

interface WidgetProps {
  widgetId: string;
  config: WidgetConfig;
  dataSource: ResolvedDataSource | null;
  isEditing: boolean;
}
```

## Migration Plan

Build new system alongside the old. No breaking changes until final cutover.

### Phase 1: Foundation
1. Create `web/src/features/dashboard/` directory structure
2. Install `@tanstack/react-query` dependency
3. Set up QueryClient provider in app root
4. Rewrite dashboard types with strict generics (no `as any`)
5. Build API client functions (`api/dashboards.ts`, `api/telemetry.ts`)
6. Build 4 Zustand slices

### Phase 2: Data Layer
7. Implement `useDeviceTelemetry` hook
8. Implement `useExtensionMetric` hook
9. Implement `useSystemMetric` hook
10. Implement `useAiMetric` hook
11. Implement `useStaticValue` hook
12. Implement `useWidgetDataSource` wrapper
13. Add WebSocket → `queryClient.setQueryData` bridge
14. Implement `telemetryTransform.ts` utils

### Phase 3: Core Components
15. Build `VisualDashboard` orchestrator
16. Build `DashboardGrid` (react-grid-layout wrapper)
17. Build `WidgetShell` (chrome around every widget)
18. Build `InstallWidgetDialog`
19. Build `DataSourceSelector` config component
20. Build remaining config components (`DataSourceField`, `DisplayOptions`, `ActionConfig`)
21. Build `WidgetConfigPanel` shell

### Phase 4: Widget Migration (one at a time, each independently testable)
22. ValueCard
23. Sparkline
24. LineChart
25. BarChart
26. PieChart
27. ProgressBar
28. ToggleSwitch
29. LedIndicator
30. ImageDisplay
31. WebDisplay
32. MarkdownDisplay
33. MapDisplay (decomposed: MapDisplay + MapMarkerLayer + MapCommandHandler)
34. VideoDisplay
35. CustomLayer (decomposed: CustomLayer + LayerCanvas + LayerToolbar)
36. AgentMonitor (decomposed: AgentMonitorWidget + AgentMessageList)
37. AiAnalyst

### Phase 5: Cutover
38. Wire new dashboard to routing (replace old VisualDashboard)
39. Remove old `components/dashboard/` directory
40. Remove old `dashboardSlice.ts`
41. Remove old `useDataSource.ts`
42. E2E testing and polish

## Key Decompositions

| Current monolith | Decomposed into |
|-----------------|-----------------|
| `UnifiedDataSourceConfig.tsx` (2,504 lines) | `DataSourceSelector` + `DataSourceField` + `DisplayOptions` + `ActionConfig` (~300 lines each) |
| `CustomLayer.tsx` (1,396 lines) | `CustomLayer` + `LayerCanvas` + `LayerToolbar` (~400 lines each) |
| `MapDisplay.tsx` (1,206 lines) | `MapDisplay` + `MapMarkerLayer` + `MapCommandHandler` (~350 lines each) |
| `AgentMonitorWidget.tsx` (1,150 lines) | `AgentMonitorWidget` + `AgentMessageList` (~400 lines each) |
| `useDataSource.ts` (28K tokens) | 5 focused hooks (~150 lines each) |
| `dashboardSlice.ts` (964 lines) | 4 focused slices (~200 lines each) |

## Dependencies

- `@tanstack/react-query` — new dependency for data fetching
- `zod` — new dependency for API response validation
- Existing: `react-grid-layout`, `recharts`, `lucide-react`, `zustand`

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| TanStack Query learning curve | Phase 2 builds hooks incrementally, starting with simplest |
| Data parity with old system | Each widget tested individually in Phase 4 |
| Cutover regression | Phase 5 keeps old code until new system is verified; can rollback |
| Scope creep | Each phase has clear deliverables; no new features during rewrite |
