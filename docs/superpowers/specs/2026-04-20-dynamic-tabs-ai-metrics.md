# Dynamic Data Explorer Tabs & AI Metric Tool

Date: 2026-04-20

## Background

The Data Explorer currently uses hardcoded tabs (All, Devices, Extensions, Transforms) to categorize data sources. Adding new data source types requires frontend code changes. Additionally, there is no mechanism for AI agents to create and persist custom metrics during analysis, which limits the platform's ability to support AI-driven insights.

## Goals

1. Make Data Explorer tabs data-driven so new `DataSourceType` variants appear automatically without frontend changes.
2. Add an `Ai` data source type with a dedicated agent tool for writing and reading custom AI-generated metrics.

## Non-Goals

- Migrating existing device data to a unified key format (`device:` prefix).
- Adding count badges or statistics to tabs.
- Building a UI for manually creating custom metrics (may follow later).

## Design

### 1. DataSourceType: Add `Ai` Variant

**File**: `crates/neomind-core/src/datasource/mod.rs`

Add `Ai` to the `DataSourceType` enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceType {
    Device,
    Extension,
    Transform,
    Ai,  // new
}
```

Update `device_part()`:

```rust
pub fn device_part(&self) -> String {
    match &self.source_type {
        DataSourceType::Device    => self.source_id.clone(),
        DataSourceType::Extension => format!("extension:{}", self.source_id),
        DataSourceType::Transform => format!("transform:{}", self.source_id),
        DataSourceType::Ai        => format!("ai:{}", self.source_id),
    }
}
```

Update `from_storage_parts()` — add `ai` prefix handling, plus `device:` for future-proofing:

```rust
pub fn from_storage_parts(device_id: &str, metric: &str) -> Option<Self> {
    match device_id.split_once(':') {
        Some(("extension", id)) => Some(Self::extension(id, metric)),
        Some(("transform", id)) => Some(Self::transform(id, metric)),
        Some(("ai", id))        => Some(Self::ai(id, metric)),
        Some(("device", id))    => Some(Self::device(id, metric)), // future-proof
        _                       => Some(Self::device(device_id, metric)), // legacy
    }
}
```

Add constructor and constants:

```rust
impl DataSourceId {
    pub fn ai(group: &str, field: &str) -> Self {
        Self { source_type: DataSourceType::Ai, source_id: group.to_string(), field: field.to_string() }
    }
}

pub const AI_SOURCE_TYPE: &str = "ai";
```

### 2. Unified Data Sources API

**File**: `crates/neomind-api/src/handlers/data.rs`

Add a `collect_ai_sources()` function following the pattern of `collect_extension_sources()`:

- Query telemetry.redb for keys with `"ai:"` prefix via `list_metrics("ai:")` or a prefix scan.
- For each discovered `(group, field)`, build a `UnifiedDataSourceInfo` with `source_type: "ai"`.
- Integrate into `list_unified_data_sources()` alongside the existing three collectors.

The display name for AI metrics should use the group name in title case, with the field name as-is. Description comes from metadata stored at write time (see tool design below).

### 3. AiMetricTool (Agent Tool)

**File**: `crates/neomind-api/src/server/tools.rs` (or a new file alongside existing tools)

A new `Tool` implementation with two actions:

#### Tool Definition

- **Name**: `ai_metric`
- **Description**: "Write or read custom AI-generated metrics. Use this to persist analysis results, anomaly scores, predictions, or any derived data as time-series metrics that appear in the Data Explorer."

#### Action: `write`

Writes a single data point to telemetry.redb under `ai:{group}:{field}`.

**Parameters**:
```json
{
  "action": "write",
  "group": "anomaly",
  "field": "score",
  "value": 0.85,
  "unit": "0-1",
  "description": "Anomaly score computed from temperature trend analysis"
}
```

| Parameter     | Type   | Required | Description |
|--------------|--------|----------|-------------|
| action       | string | yes      | Must be `"write"` |
| group        | string | yes      | Logical grouping (e.g. "anomaly", "trend", "prediction") |
| field        | string | yes      | Metric field name (e.g. "score", "direction") |
| value        | any    | yes      | The metric value (number, string, boolean) |
| unit         | string | no       | Unit of measurement (e.g. "%", "°C", "0-1") |
| description  | string | no       | Human-readable description of this metric |

**Behavior**:
1. Validate group and field are non-empty alphanumeric strings (hyphens/underscores allowed).
2. Construct `DataSourceId::ai(group, field)`.
3. Convert value to `MetricValue`.
4. Create `DataPoint { timestamp: now, value, quality: Some(1.0) }`.
5. Write via `TimeSeriesStorage::write(&id.device_part(), &id.metric_part(), point)`.
6. If `unit` or `description` provided, store as metadata for display in Data Explorer.
7. Return `{ "status": "written", "id": "ai:anomaly:score" }`.

#### Action: `read`

Queries existing AI metrics or specific time-series data.

**Parameters**:
```json
{
  "action": "read",
  "query": "list"
}
```
or
```json
{
  "action": "read",
  "query": "data",
  "group": "anomaly",
  "field": "score",
  "hours": 24
}
```

| Parameter | Type   | Required | Description |
|-----------|--------|----------|-------------|
| action    | string | yes      | Must be `"read"` |
| query     | string | yes      | `"list"` for all AI metrics, `"data"` for time-series |
| group     | string | no       | Required when query is `"data"` |
| field     | string | no       | Required when query is `"data"` |
| hours     | number | no       | Lookback window in hours (default: 1) |

**Behavior**:
- `"list"`: Return all discovered `ai:*` metrics with their latest values.
- `"data"`: Return time-series data for `ai:{group}:{field}` over the specified time range.

### 4. Tool Registration

**File**: `crates/neomind-api/src/server/tools.rs`

Register `AiMetricTool` in the tool builder pipeline:

```rust
// In ToolRegistryBuilder::with_aggregated_tools_full() or similar
registry.register(Arc::new(AiMetricTool::new(time_series_storage.clone())));
```

The tool requires `SharedTimeSeriesStorage` as a dependency, same as existing tools.

### 5. Frontend: Dynamic Tabs

**File**: `web/src/pages/data-explorer.tsx`

#### Changes

1. **Remove hardcoded `SourceType` union**:
   ```typescript
   // Before
   type SourceType = 'all' | 'device' | 'extension' | 'transform'
   // After
   type SourceType = string
   ```

2. **Generate tabs dynamically from source data**:
   ```typescript
   const tabs = useMemo(() => {
     const typeSet = new Set(sources.map(s => s.source_type))
     return [
       { value: 'all', label: t('data:tabs.all', 'All'), icon: <Database /> },
       ...Array.from(typeSet).sort().map(type => ({
         value: type,
         label: t(`data:tabs.${type}`, typeLabel(type)),
         icon: iconForType(type),
       }))
     ]
   }, [sources, t])
   ```

3. **Add type-to-icon and type-to-label maps**:
   ```typescript
   const iconMap: Record<string, LucideIcon> = {
     device: Cpu, extension: Puzzle, transform: Workflow, ai: Brain, system: Monitor
   }
   const defaultIcon = Database

   function typeLabel(type: string): string {
     return t(`data:tabs.${type}`, type.charAt(0).toUpperCase() + type.slice(1))
   }
   ```

4. **Remove hardcoded `PageTabsContent` blocks** — the `dataTable` is the same across all tabs, so a single conditional render suffices (filtering is already handled by `filteredSources`).

5. **Add i18n keys** for the new `ai` tab:
   - `en/data.json`: `"tabs.ai": "AI Metrics"`
   - `zh/data.json`: `"tabs.ai": "AI 指标"`

6. **Update `SourceTypeBadge`** color map to include `ai`:
   ```typescript
   ai: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20'
   ```

### 6. Metadata Storage for AI Metrics

AI metrics need `unit` and `description` metadata that persists beyond the data point. Two options:

**Option A (Recommended): In-memory registry.** A simple `DashMap<(String, String), AiMetricMeta>` keyed by `(group, field)` that stores `{ display_name, unit, description }`. Populated on write, used when building `UnifiedDataSourceInfo`. Lost on restart, but AI metrics are ephemeral by nature and the tool can re-register metadata on each write.

**Option B: Metadata in DataPoint.** Use the existing `metadata` field on `DataPoint`. This persists but only the latest write carries the metadata, and it's per-data-point rather than per-metric.

Option A is simpler and separates concerns cleanly.

## Impact Analysis

- **Device data**: Zero impact. `device_part()` unchanged, old data fully readable.
- **Extension data**: Zero impact. No changes to extension write/read paths.
- **Transform data**: Zero impact. No changes to transform write/read paths.
- **Agent tool token cost**: One additional tool definition (~200 tokens). Only loaded when agent session is active.
- **telemetry.redb**: New keys under `"ai:*"` namespace. No schema changes.

## File Changes Summary

| File | Change |
|------|--------|
| `crates/neomind-core/src/datasource/mod.rs` | Add `Ai` variant, update `device_part()`, `from_storage_parts()`, add `ai()` constructor |
| `crates/neomind-api/src/handlers/data.rs` | Add `collect_ai_sources()`, integrate into unified listing |
| `crates/neomind-api/src/server/tools.rs` | Add `AiMetricTool` struct and `Tool` impl, register in builder |
| `crates/neomind-api/src/server/router.rs` | No changes needed (ai metrics served by existing `/api/data/sources`) |
| `web/src/pages/data-explorer.tsx` | Dynamic tabs, remove hardcoded types, add ai badge color |
| `web/src/types/index.ts` | No changes needed (`source_type` is already `string`) |
| `web/src/i18n/locales/en/data.json` | Add `tabs.ai` key |
| `web/src/i18n/locales/zh/data.json` | Add `tabs.ai` key |
