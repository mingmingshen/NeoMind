# useDataSource Data Pipeline Rewrite

## Problem

The current `useDataSource` system is 3,297 lines (638 in main hook + 2,661 across 15 sub-files) for what is fundamentally a simple job: **fetch data from APIs, receive WebSocket updates, display in components**. This over-engineering has caused:

- **Hard-to-trace bugs**: Data flows through 5 sub-hooks + store subscription + dual event channels, making race conditions inevitable
- **Duplicate data paths**: Store subscription AND WebSocket events both push the same device data
- **Dead code**: eventBus.ts (162 lines), storeWatcher.ts (213 lines), network-perf-test.ts (32 lines) — zero external consumers
- **Premature optimization**: batchFetch.ts microtask batching, 3 separate TypedCache instances, RAF-based event batching
- **Fuzzy matching**: extractors.ts (283 lines) with property name heuristics that mask real bugs

## Target

~400 lines total. Single file for the hook, one small utility file for shared helpers.

## Architecture

### Core Principle: One data path per source type

Each `DataSource.type` has exactly **one** way data enters the component:

| Source Type | Initial Load | Real-time Updates |
|---|---|---|
| `device` / `command` / `device-info` / `metric` | Read from Zustand store | Store subscription (devices slice) |
| `telemetry` / `transform` / `ai-metric` | `fetchHistoricalTelemetry()` API call | Store subscription → merge latest point |
| `extension` | Extension output API or store | WebSocket `ExtensionOutput` event |
| `system` | `fetchSystemStats()` API call | Polling (interval from `refresh`) |

**No dual channels.** No event bus. No batch fetch. No RAF batching.

### New File Structure

```
web/src/hooks/useDataSource/
├── index.ts              (~30 lines)  Re-exports
├── useDataSource.ts      (~350 lines) Main hook — ALL logic here
├── fetch.ts              (~120 lines) API fetch functions (telemetry, system, extension)
├── helpers.ts            (~80 lines)  Pure utilities: dedup, extract value, sort
└── cache.ts              (~40 lines)  Simple Map<string, {data, expiry}> with TTL

DELETED FILES (no replacement):
├── batchFetch.ts         ❌ 200 lines — store subscription handles device data
├── eventBus.ts           ❌ 162 lines — dead code, zero consumers
├── storeWatcher.ts       ❌ 213 lines — dead code, zero consumers
├── extractors.ts         ❌ 283 lines — replaced by 30 lines in helpers.ts
├── network-perf-test.ts  ❌ 32 lines  — debug artifact
├── index.ts (old)        ❌ 48 lines  — merged into new index.ts

MERGED INTO useDataSource.ts:
├── useDeviceEventProcessing.ts    (415 lines → ~0 lines, deleted)
├── useExtensionEventProcessing.ts (180 lines → ~0 lines, deleted)
├── useTelemetryFetching.ts        (216 lines → ~0 lines, deleted)
├── useSystemFetching.ts           (104 lines → ~0 lines, deleted)
├── useExtensionFetching.ts        (169 lines → ~0 lines, deleted)
├── telemetryFetch.ts              (249 lines → merged into fetch.ts, ~60 lines)
├── systemFetch.ts                 (72 lines → merged into fetch.ts, ~20 lines)
└── dedup.ts                       (181 lines → merged into helpers.ts, ~40 lines)

OLD useDataSource.ts: 638 lines → 0 (merged into new useDataSource.ts)
```

### Data Flow (Simplified)

```
useDataSource(sources)
  │
  ├─ classify sources by type
  │
  ├─ [store-based sources] ────────────────────────────────────┐
  │   readDataFromStore() → setData()                          │
  │   store.subscribe() → onChange → readDataFromStore()       │
  │   (telemetry: also merge latest point into array)          │
  │                                                            │
  ├─ [telemetry sources] ──────────────────────────────────┐   │
  │   fetchHistoricalTelemetry() → sort/dedup → setData()   │   │
  │   setInterval for refresh (if ds.refresh set)           │   │
  │                                                        │   │
  ├─ [extension sources] ──────────────────────────────────────┤
  │   fetch extension output → setData()                       │
  │   useEvents('extension') → merge event into data array     │
  │                                                            │
  └─ [system sources] ──────────────────────────────────────┐  │
      fetchSystemStats() → setData()                        │  │
      setInterval for refresh                               │  │
                                                           │  │
  return { data, loading, error, lastUpdate, sendCommand } ◄─┘
```

### Hook Signature (Unchanged)

The public API stays exactly the same — all 15+ consumers work without changes:

```typescript
function useDataSource<T = unknown>(
  dataSource: DataSourceOrList | undefined,
  options?: {
    enabled?: boolean
    transform?: (data: unknown) => T
    fallback?: T
    preserveMultiple?: boolean
  }
): UseDataSourceResult<T>
```

### Implementation Details

#### 1. fetch.ts (~120 lines)

Three exported functions:

```typescript
// Fetch historical telemetry with in-flight dedup and cache
export async function fetchTelemetry(
  deviceId: string, metricId: string, timeRange: number, limit: number, aggregate: string
): Promise<{ data: unknown[]; success: boolean }>

// Fetch system stats with cache
export async function fetchSystemStats(): Promise<Record<string, unknown>>

// Fetch extension output
export async function fetchExtensionOutput(
  extensionId: string, metric: string
): Promise<unknown>
```

- Simple `Map<string, {data, expiry}>` cache (no TypedCache class)
- In-flight dedup via `Map<string, Promise>` (keep from telemetryFetch.ts)
- TTL: telemetry 30s, system 10s, extension 30s

#### 2. helpers.ts (~80 lines)

```typescript
// Extract a value from device.current_values by property name (exact match only)
export function extractValue(currentValues: Record<string, unknown>, property: string): unknown

// Sort telemetry points newest-first, dedup by timestamp (1s tolerance)
export function sortAndDedup(points: unknown[], maxLimit: number): unknown[]

// Safe value extraction with fallback
export function safeExtract(value: unknown, fallback: unknown): unknown
```

No fuzzy matching. No `findPropertyValue` with regex. No virtual metric detection. Just exact property lookup.

#### 3. cache.ts (~40 lines)

```typescript
const cache = new Map<string, { data: unknown; expiry: number }>()
const inFlight = new Map<string, Promise<unknown>>()

export function getCached(key: string): unknown | null
export function setCache(key: string, data: unknown, ttlMs: number): void
export function getInFlight(key: string): Promise<unknown> | null
export function setInFlight(key: string, promise: Promise<unknown>): void
export function deleteCache(key: string): void  // for event-driven invalidation
export function clearAll(): void
```

#### 4. useDataSource.ts (~350 lines)

Single hook with inline logic organized in 4 sections:

**Section A: Setup** (~30 lines)
- State: `[data, loading, error, lastUpdate]`
- Normalize dataSources, classify by type
- Compute stable keys

**Section B: Store-based sources** (~80 lines)
- `readFromStore()` function: handles `device`, `command`, `device-info`, `metric` types
- Store subscription with `useStore.subscribe()` — only fires when relevant device changes
- Telemetry merge: when store update contains telemetry-relevant device, prepend latest point
- Command handling: `sendCommand` callback

**Section C: Fetch-based sources** (~100 lines)
- Telemetry: `useEffect` calls `fetchTelemetry()` on mount + interval
- Extension: `useEffect` calls `fetchExtensionOutput()` on mount + interval
- System: `useEffect` calls `fetchSystemStats()` on mount + interval
- All use `setData(prev => ...)` for safe concurrent updates
- Empty result retry: max 3 attempts with 3s delay

**Section D: Extension WebSocket** (~60 lines)
- `useEvents({ category: 'extension' })` subscription
- Filter events by matching `extensionId` + `extensionMetric`
- Merge event value into existing data array (prepend, sort, dedup)
- Invalidate fetch cache on event

### What We Lose (And Why It's Fine)

| Removed | Reason |
|---|---|
| Dual channel (store + WebSocket for devices) | Store subscription IS the WebSocket consumer. `useEvents` for devices updates the store, which triggers subscription. One path. |
| TypedCache with metadata | Simple `Map<string, {data, expiry}>` is sufficient. No size limits needed — dashboards have <20 components. |
| batchFetch.ts | Was only used for initial device data fetch. Store handles this via device list fetch on mount. |
| extractors.ts fuzzy matching | Exact property name match only. If property name is wrong, the fix is in config, not code. |
| RAF batching | React batches setState calls automatically in React 18. No manual batching needed. |
| eventBus.ts | Dead code. Zero consumers. |
| storeWatcher.ts | Dead code. Zero consumers. |
| network-perf-test.ts | Debug artifact. |

### Key Bug Fixes Inherent in Rewrite

1. **No more stale dataRef race**: All `setData` calls use callback form `setData(prev => ...)`
2. **No dual-path duplication**: One data path per source type
3. **Extension output_name mismatch**: Built into event matching logic from the start
4. **Scalar value re-render**: Always wrap as `[{timestamp, value}]` array
5. **Cache invalidation**: Events invalidate cache before updating state

### Migration Strategy

Since the public API is unchanged, migration is:

1. Write new files alongside old (new directory: `useDataSource.new/`)
2. Update import in `useDataSource.ts` to use new implementation
3. Run type check (`npx tsc --noEmit`)
4. Test all 15 dashboard components render correctly
5. Test real-time updates (device + extension)
6. Delete old files

### Estimated Result

| Metric | Before | After |
|---|---|---|
| Total lines | 3,297 | ~620 |
| Number of files | 16 | 4 |
| Sub-hooks | 5 | 0 |
| Dead code lines | 407 | 0 |
| Cache systems | 3 (TypedCache x3) | 1 (simple Map) |
| Data paths per source | 2-3 | 1 |

## Implementation Steps

### Step 1: Create new files (fetch.ts, helpers.ts, cache.ts)
- Pure functions, no React dependencies
- Copy and simplify from existing: telemetryFetch.ts → fetch.ts, dedup.ts → helpers.ts, cache.ts → simplified

### Step 2: Write new useDataSource.ts
- Single hook file, ~350 lines
- Inline all logic that was in 5 sub-hooks
- Same public API signature

### Step 3: Update index.ts
- Re-export public API from new files
- Keep backward-compatible re-exports (`fetchHistoricalTelemetry`)

### Step 4: Switch over
- Update the main import in `web/src/hooks/useDataSource.ts`
- Run `npx tsc --noEmit` to verify types

### Step 5: Test
- Dashboard renders all component types
- Device data updates in real-time
- Extension data updates via WebSocket
- Telemetry charts show historical data
- Command send works

### Step 6: Delete old files
- Remove: eventBus.ts, storeWatcher.ts, network-perf-test.ts, batchFetch.ts, extractors.ts, index.ts (old)
- Remove: useDeviceEventProcessing.ts, useExtensionEventProcessing.ts, useTelemetryFetching.ts, useSystemFetching.ts, useExtensionFetching.ts, telemetryFetch.ts, systemFetch.ts, dedup.ts
