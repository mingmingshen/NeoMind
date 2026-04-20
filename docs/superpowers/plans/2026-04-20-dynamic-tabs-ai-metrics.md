# Dynamic Data Explorer Tabs & AI Metric Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Data Explorer tabs data-driven and add an `ai_metric` agent tool for writing/reading custom AI metrics.

**Architecture:** Add `DataSourceType::Ai` to the core enum, build an `AiMetricTool` as a standalone `Tool` impl registered in the `AggregatedToolsBuilder`, and refactor the frontend Data Explorer to generate tabs dynamically from API response data. A lightweight in-memory `AiMetricsRegistry` (DashMap) stores metadata (unit, description) for AI metrics.

**Tech Stack:** Rust (Axum, redb, async-trait, serde, dashmap), React 18 + TypeScript + Tailwind + i18next

**Spec:** `docs/superpowers/specs/2026-04-20-dynamic-tabs-ai-metrics.md`

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `crates/neomind-core/src/datasource/mod.rs` | `DataSourceType` enum, `DataSourceId` methods | Modify |
| `crates/neomind-agent/src/toolkit/ai_metric.rs` | `AiMetricsRegistry` + `AiMetricTool` impl | Create |
| `crates/neomind-agent/src/toolkit/mod.rs` | Export `ai_metric` module | Modify |
| `crates/neomind-agent/src/toolkit/aggregated.rs` | Register `AiMetricTool` in builder | Modify |
| `crates/neomind-api/src/handlers/data.rs` | `collect_ai_sources()` function | Modify |
| `web/src/pages/data-explorer.tsx` | Dynamic tabs, badge colors | Modify |
| `web/src/i18n/locales/en/data.json` | English i18n for data explorer | Create |
| `web/src/i18n/locales/zh/data.json` | Chinese i18n for data explorer | Create |
| `web/src/i18n/config.ts` | Register `data` namespace | Modify |

---

### Task 1: Add `Ai` variant to `DataSourceType`

**Files:**
- Modify: `crates/neomind-core/src/datasource/mod.rs`

- [ ] **Step 1: Add `Ai` variant and update all match arms**

In `DataSourceType` enum, add `Ai` with `#[serde(rename = "ai")]`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceType {
    #[serde(rename = "device")]
    Device,
    #[serde(rename = "extension")]
    Extension,
    #[serde(rename = "transform")]
    Transform,
    #[serde(rename = "ai")]
    Ai,
}
```

Update `DataSourceId` methods — add `Ai` arm to every match:

- `ai()` constructor:
```rust
pub fn ai(group: &str, field: &str) -> Self {
    Self {
        source_type: DataSourceType::Ai,
        source_id: group.to_string(),
        field_path: field.to_string(),
    }
}
```

- `parse()`: add `"ai" => DataSourceType::Ai` to the source_type match
- `storage_key()`: add `DataSourceType::Ai => format!("ai:{}:{}", self.source_id, self.field_path)`
- `display_name()`: add `DataSourceType::Ai => format!("AI {} / {}", self.source_id, self.field_path)`
- `device_part()`: add `DataSourceType::Ai => format!("ai:{}", self.source_id)`
- `from_storage_parts()`: replace the if-else chain with a single match:
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

If `DataSourceCatalog::by_type()` exists, add `DataSourceType::Ai => self.ai.iter().collect()` (with a new `ai: Vec<DataSourceInfo>` field).

- [ ] **Step 2: Build to verify compilation**

Run: `cargo build -p neomind-core`
Expected: Compiles with errors only in crates that match on `DataSourceType` (fix those too — the compiler will list them all).

- [ ] **Step 3: Fix all downstream compile errors**

Run: `cargo build 2>&1 | head -50`
Fix every `non-exhaustive patterns` error by adding `DataSourceType::Ai` arms. Typical locations:
- `crates/neomind-api/src/handlers/data.rs` — any match on `DataSourceType`
- `crates/neomind-api/src/event_services.rs` — if matching on `DataSourceType`

Run: `cargo build`
Expected: Clean build

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(core): add Ai variant to DataSourceType with all match arm updates"
```

---

### Task 2: Create `AiMetricsRegistry` and `AiMetricTool`

**Files:**
- Create: `crates/neomind-agent/src/toolkit/ai_metric.rs`
- Modify: `crates/neomind-agent/src/toolkit/mod.rs`

- [ ] **Step 1: Create `ai_metric.rs` with registry and tool**

The file contains two things:

**AiMetricsRegistry** — in-memory metadata store:
```rust
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

/// Metadata for an AI metric, stored in-memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMetricMeta {
    pub unit: Option<String>,
    pub description: Option<String>,
}

/// Ephemeral registry for AI metric metadata.
/// Shared between AiMetricTool (writes metadata) and data handler (reads metadata).
#[derive(Debug, Default)]
pub struct AiMetricsRegistry {
    metrics: DashMap<(String, String), AiMetricMeta>, // key: (group, field)
}

impl AiMetricsRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn register(&self, group: &str, field: &str, meta: AiMetricMeta) {
        self.metrics.insert((group.to_string(), field.to_string()), meta);
    }

    pub fn get(&self, group: &str, field: &str) -> Option<AiMetricMeta> {
        self.metrics.get(&(group.to_string(), field.to_string())).map(|v| v.value().clone())
    }

    pub fn all_keys(&self) -> Vec<(String, String)> {
        self.metrics.iter().map(|e| e.key().clone()).collect()
    }
}
```

**AiMetricTool** — agent tool implementation:
```rust
use async_trait::async_trait;
use neomind_devices::TimeSeriesStorage;
use serde_json::{Value, json};
use std::sync::Arc;

use super::tool::{Tool, ToolOutput, ToolError};

pub struct AiMetricTool {
    storage: Arc<TimeSeriesStorage>,
    registry: Arc<AiMetricsRegistry>,
}

impl AiMetricTool {
    pub fn new(storage: Arc<TimeSeriesStorage>, registry: Arc<AiMetricsRegistry>) -> Self {
        Self { storage, registry }
    }
}

#[async_trait]
impl Tool for AiMetricTool {
    fn name(&self) -> &str { "ai_metric" }

    fn description(&self) -> &str {
        "Write or read custom AI-generated metrics. Use this to persist analysis results, anomaly scores, predictions, or any derived data as time-series metrics that appear in the Data Explorer."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["write", "read"],
                    "description": "Action to perform"
                },
                "group": {
                    "type": "string",
                    "description": "Logical grouping (e.g. 'anomaly', 'trend', 'prediction')"
                },
                "field": {
                    "type": "string",
                    "description": "Metric field name (e.g. 'score', 'direction')"
                },
                "value": {
                    "description": "The metric value (number, string, boolean, or JSON). Required for write action."
                },
                "unit": {
                    "type": "string",
                    "description": "Unit of measurement (e.g. '%', '°C', '0-1')"
                },
                "description": {
                    "type": "string",
                    "description": "Human-readable description of this metric"
                },
                "query": {
                    "type": "string",
                    "enum": ["list", "data"],
                    "description": "Query type for read action. 'list' returns all AI metrics, 'data' returns time-series."
                },
                "hours": {
                    "type": "number",
                    "description": "Lookback window in hours for 'data' query (default: 1)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput, ToolError> {
        let action = args["action"].as_str().unwrap_or("");
        match action {
            "write" => self.execute_write(&args).await,
            "read" => self.execute_read(&args).await,
            _ => Err(ToolError::InvalidArguments("action must be 'write' or 'read'".into())),
        }
    }
}

impl AiMetricTool {
    fn validate_name(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }

    fn json_to_metric_value(value: &Value) -> neomind_devices::mdl::MetricValue {
        match value {
            Value::Null => neomind_devices::mdl::MetricValue::Null,
            Value::Bool(b) => neomind_devices::mdl::MetricValue::Boolean(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    neomind_devices::mdl::MetricValue::Integer(i)
                } else {
                    neomind_devices::mdl::MetricValue::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            Value::String(s) => neomind_devices::mdl::MetricValue::String(s.clone()),
            Value::Array(arr) => neomind_devices::mdl::MetricValue::Array(
                arr.iter().map(|v| Self::json_to_metric_value(v)).collect()
            ),
            other => neomind_devices::mdl::MetricValue::String(other.to_string()),
        }
    }

    async fn execute_write(&self, args: &Value) -> Result<ToolOutput, ToolError> {
        let group = args["group"].as_str().unwrap_or("");
        let field = args["field"].as_str().unwrap_or("");

        if !Self::validate_name(group) {
            return Err(ToolError::InvalidArguments(
                "Invalid 'group': must be non-empty alphanumeric (hyphens/underscores allowed)".into(),
            ));
        }
        if !Self::validate_name(field) {
            return Err(ToolError::InvalidArguments(
                "Invalid 'field': must be non-empty alphanumeric (hyphens/underscores allowed)".into(),
            ));
        }
        if args.get("value").is_none_or(|v| v.is_null()) {
            return Err(ToolError::InvalidArguments("Missing required parameter: value".into()));
        }

        let value = &args["value"];
        let metric_value = Self::json_to_metric_value(value);

        let point = neomind_devices::DataPoint {
            timestamp: chrono::Utc::now().timestamp_millis(),
            value: metric_value,
            quality: Some(1.0),
        };

        let device_id = format!("ai:{}", group);
        self.storage
            .write(&device_id, field, point)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to write metric: {}", e)))?;

        // Store metadata
        let meta = AiMetricMeta {
            unit: args.get("unit").and_then(|v| v.as_str()).map(String::from),
            description: args.get("description").and_then(|v| v.as_str()).map(String::from),
        };
        if meta.unit.is_some() || meta.description.is_some() {
            self.registry.register(group, field, meta);
        }

        Ok(ToolOutput::success(json!({
            "status": "written",
            "id": format!("ai:{}:{}", group, field)
        })))
    }

    async fn execute_read(&self, args: &Value) -> Result<ToolOutput, ToolError> {
        let query = args["query"].as_str().unwrap_or("list");

        match query {
            "list" => {
                let keys = self.registry.all_keys();
                let mut entries = Vec::new();
                for (group, field) in keys {
                    let device_id = format!("ai:{}", group);
                    let latest = self.storage.latest(&device_id, &field).await;
                    let meta = self.registry.get(&group, &field);
                    entries.push(json!({
                        "id": format!("ai:{}:{}", group, field),
                        "group": group,
                        "field": field,
                        "unit": meta.as_ref().and_then(|m| m.unit.as_deref()),
                        "description": meta.as_ref().and_then(|m| m.description.as_deref()),
                        "current_value": latest.ok().flatten().map(|dp| dp.value.to_json_value()),
                    }));
                }
                Ok(ToolOutput::success(json!({ "metrics": entries })))
            }
            "data" => {
                let group = args["group"].as_str().unwrap_or("");
                let field = args["field"].as_str().unwrap_or("");
                if group.is_empty() || field.is_empty() {
                    return Err(ToolError::InvalidArguments(
                        "'group' and 'field' required for data query".into(),
                    ));
                }
                let hours = args["hours"].as_u64().unwrap_or(1);
                let now = chrono::Utc::now().timestamp_millis();
                let start = now - (hours * 3_600_000) as i64;
                let device_id = format!("ai:{}", group);
                let result = self.storage
                    .query(&device_id, field, start, now)
                    .await
                    .map_err(|e| ToolError::Execution(format!("Query failed: {}", e)))?;
                let points: Vec<Value> = result.iter().map(|dp| {
                    json!({
                        "timestamp": dp.timestamp,
                        "value": dp.value.clone().to_json_value(),
                        "quality": dp.quality,
                    })
                }).collect();
                Ok(ToolOutput::success(json!({ "data_points": points })))
            }
            _ => Err(ToolError::InvalidArguments("query must be 'list' or 'data'".into())),
        }
    }
}
```

**Note:** `MetricValue` uses `to_json_value()` method to convert to `serde_json::Value`. The enum variants are: `Integer(i64)`, `Float(f64)`, `String(String)`, `Boolean(bool)`, `Array(Vec<MetricValue>)`, `Binary(Vec<u8>)`, `Null`.

- [ ] **Step 2: Export module in `mod.rs`**

Add to `crates/neomind-agent/src/toolkit/mod.rs`:
```rust
pub mod ai_metric;
pub use ai_metric::{AiMetricTool, AiMetricsRegistry, AiMetricMeta};
```

- [ ] **Step 3: Build to check compilation**

Run: `cargo build -p neomind-agent`
Fix any compile errors (likely `MetricValue` conversion, `into_json()` method name, import paths).

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(agent): add AiMetricsRegistry and AiMetricTool with write/read actions"
```

---

### Task 3: Register `AiMetricTool` in the builder

**Files:**
- Modify: `crates/neomind-agent/src/toolkit/aggregated.rs`

- [ ] **Step 1: Add `ai_metrics_registry` field to `AggregatedToolsBuilder`**

Add field to struct:
```rust
pub struct AggregatedToolsBuilder {
    // ... existing fields ...
    ai_metrics_registry: Option<Arc<super::ai_metric::AiMetricsRegistry>>,
}
```

Initialize as `None` in `new()`.

Add builder method:
```rust
pub fn with_ai_metrics_registry(mut self, registry: Arc<super::ai_metric::AiMetricsRegistry>) -> Self {
    self.ai_metrics_registry = Some(registry);
    self
}
```

- [ ] **Step 2: Register tool in `build()`**

In the `build()` method, after the Skill tool block (~line 3586), add:

```rust
// AI metric tool
if let (Some(storage), Some(registry)) = (&self.time_series_storage, &self.ai_metrics_registry) {
    tools.push(Arc::new(super::ai_metric::AiMetricTool::new(
        storage.clone(),
        registry.clone(),
    )));
}
```

- [ ] **Step 3: Build to verify**

Run: `cargo build -p neomind-agent`

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(agent): register AiMetricTool in AggregatedToolsBuilder"
```

---

### Task 4: Wire `AiMetricsRegistry` into ServerState

**Files:**
- Modify: `crates/neomind-api/src/server/types.rs`

- [ ] **Step 1: Add registry to ServerState**

The `ServerState` has sub-structs (`AgentState`, `DeviceState`, etc.). Add the registry to `AgentState` since it's AI-related. Check `crates/neomind-api/src/server/types.rs` for the exact sub-struct.

```rust
use neomind_agent::toolkit::ai_metric::AiMetricsRegistry;

// In AgentState struct:
pub ai_metrics_registry: Arc<AiMetricsRegistry>,
```

Initialize in `ServerState::new()`:
```rust
ai_metrics_registry: AiMetricsRegistry::new(),
```

Also expose via `ServerState` if needed (check if there's an accessor pattern):
```rust
// If ServerState has accessor methods, add:
pub fn ai_metrics_registry(&self) -> Arc<AiMetricsRegistry> {
    self.agents.ai_metrics_registry.clone()
}
```

- [ ] **Step 2: Pass registry to builder in `init_tools()` and `refresh_extension_tools()`**

Both methods in `crates/neomind-api/src/server/types.rs` build tools via `ToolRegistryBuilder`. The `with_ai_metrics_registry()` call goes on the `AggregatedToolsBuilder` (called inside `with_aggregated_tools_full()`). But since `with_aggregated_tools_full()` already takes `storage` as a parameter, add the registry as a new parameter to `with_aggregated_tools_full()`:

In `crates/neomind-agent/src/toolkit/registry.rs`, update `with_aggregated_tools_full()`:
```rust
pub fn with_aggregated_tools_full(
    mut self,
    // ... existing params ...
    ai_metrics_registry: Option<Arc<super::ai_metric::AiMetricsRegistry>>,  // new param
) -> Self {
    let mut builder = AggregatedToolsBuilder::new()
        // ... existing ...
        .with_ai_metrics_registry(registry);  // new
    // ...
}
```

Then in `types.rs` `init_tools()` and `refresh_extension_tools()`, pass `Some(self.agents.ai_metrics_registry.clone())` as the new parameter.

- [ ] **Step 3: Build and fix**

Run: `cargo build -p neomind-api`
Fix any compile errors.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(api): wire AiMetricsRegistry into ServerState and tool builder"
```

---

### Task 5: Add `collect_ai_sources()` to data handler

**Files:**
- Modify: `crates/neomind-api/src/handlers/data.rs`

- [ ] **Step 1: Add `collect_ai_sources()` function**

Following the pattern of `collect_extension_sources()`, add a function that:

1. Gets `AiMetricsRegistry` from state
2. Iterates `registry.all_keys()` to get `(group, field)` pairs
3. For each pair, queries `telemetry.latest(&format!("ai:{}", group), field)` for current value
4. Builds `UnifiedDataSourceInfo` with `source_type: "ai"`
5. Uses metadata from registry for `unit` and `description`

```rust
async fn collect_ai_sources(state: &ServerState, sources: &mut Vec<UnifiedDataSourceInfo>) {
    let registry = &state.xxx.ai_metrics_registry; // adjust path
    let telemetry = state.time_series_storage();
    let keys = registry.all_keys();

    for (group, field) in keys {
        let device_id = format!("ai:{}", group);
        let id = format!("ai:{}:{}", group, field);
        let meta = registry.get(&group, &field);

        let (current_value, last_update, quality) = match telemetry.latest(&device_id, &field).await {
            Ok(Some(dp)) => (Some(dp.value.to_json_value()), Some(dp.timestamp), dp.quality),
            _ => (None, None, None),
        };

        sources.push(UnifiedDataSourceInfo {
            id,
            source_type: "ai".to_string(),
            source_name: group.clone(),
            source_display_name: format!("AI {}", title_case(&group)),
            field: field.clone(),
            field_display_name: field,
            data_type: infer_data_type(&current_value),
            unit: meta.and_then(|m| m.unit),
            description: meta.and_then(|m| m.description),
            current_value,
            last_update,
            quality,
        });
    }
}
```

**Helper functions needed:**
- `title_case(s: &str) -> String` — capitalize first letter, replace hyphens with spaces. Simple implementation:
  ```rust
  fn title_case(s: &str) -> String {
      let mut result = String::new();
      let mut capitalize = true;
      for c in s.chars() {
          if c == '-' || c == '_' { result.push(' '); capitalize = true; }
          else if capitalize { result.extend(c.to_uppercase()); capitalize = false; }
          else { result.push(c); }
      }
      result
  }
  ```
- `infer_data_type(v: &Option<Value>) -> String` — return "float", "integer", "string", "boolean" based on JSON value type. Check existing code in `data.rs` — `populate_latest_values` already does `MetricValue` → JSON conversion. Reuse the same pattern for `data_type` inference.

- [ ] **Step 2: Integrate into `list_all_data_sources_handler`**

Add after the transform collection:
```rust
// 4. Collect AI metrics
collect_ai_sources(&state, &mut sources).await;
```

Update the numbering of `populate_latest_values` to step 5.

- [ ] **Step 3: Build and test**

Run: `cargo build -p neomind-api`

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(api): add collect_ai_sources to unified data sources handler"
```

---

### Task 6: Frontend — Dynamic tabs

**Files:**
- Modify: `web/src/pages/data-explorer.tsx`
- Create: `web/src/i18n/locales/en/data.json`
- Create: `web/src/i18n/locales/zh/data.json`
- Modify: `web/src/i18n/config.ts`

- [ ] **Step 1: Create i18n files**

Create `web/src/i18n/locales/en/data.json`:
```json
{
  "title": "Data Explorer",
  "subtitle": "Browse all data sources across devices, extensions, and transforms",
  "tabs": {
    "all": "All",
    "device": "Devices",
    "extension": "Extensions",
    "transform": "Transforms",
    "ai": "AI Metrics"
  },
  "columns": {
    "type": "Type",
    "source": "Source",
    "field": "Field",
    "dataType": "Data Type",
    "updated": "Updated"
  },
  "noResults": "No data sources match your search",
  "noSources": "No data sources found",
  "search": "Search data sources...",
  "filterSource": "Filter source...",
  "allSources": "All Sources"
}
```

Create `web/src/i18n/locales/zh/data.json`:
```json
{
  "title": "数据浏览器",
  "subtitle": "浏览设备、扩展和转换中的所有数据源",
  "tabs": {
    "all": "全部",
    "device": "设备",
    "extension": "扩展",
    "transform": "转换",
    "ai": "AI 指标"
  },
  "columns": {
    "type": "类型",
    "source": "来源",
    "field": "字段",
    "dataType": "数据类型",
    "updated": "更新时间"
  },
  "noResults": "没有匹配的数据源",
  "noSources": "未找到数据源",
  "search": "搜索数据源...",
  "filterSource": "筛选来源...",
  "allSources": "全部来源"
}
```

- [ ] **Step 2: Register `data` namespace in i18n config**

In `web/src/i18n/config.ts`, add the `data` namespace:
1. Import the JSON files at the top
2. Add `data: en_data` / `data: zh_data` to the resources objects
3. Add `'data'` to the `ns` array

- [ ] **Step 3: Refactor `data-explorer.tsx`**

Key changes:

1. **Update imports** — add `Brain` from lucide-react:
```typescript
import { Search, Database, RefreshCw, Cpu, Puzzle, Workflow, Brain } from 'lucide-react'
```

2. **Relax `SourceType`**:
```typescript
type SourceType = 'all' | string
```

3. **Update `SourceTypeBadge`** — add `ai` color:
```typescript
const colorMap: Record<string, string> = {
  device: 'bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20',
  extension: 'bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20',
  transform: 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20',
  ai: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20',
}
const iconMap: Record<string, typeof Database> = {
  device: Cpu, extension: Puzzle, transform: Workflow, ai: Brain,
}
```

4. **Make tabs dynamic**:
```typescript
const tabs = useMemo(() => {
  const typeSet = new Set(sources.map(s => s.source_type))
  return [
    { value: 'all', label: t('data:tabs.all', 'All'), icon: <Database className="h-4 w-4" /> },
    ...Array.from(typeSet).sort().map(type => {
      const Icon = iconMap[type] || Database
      const label = t(`data:tabs.${type}`, type.charAt(0).toUpperCase() + type.slice(1))
      return { value: type, label, icon: <Icon className="h-4 w-4" /> }
    })
  ]
}, [sources, t])
```

5. **Remove hardcoded `PageTabsContent` blocks** — replace the 4 separate blocks with one:
```tsx
<PageTabsContent value={activeType} activeTab={activeType}>
  {dataTable}
</PageTabsContent>
```

6. **Update `onTabChange` type** — remove `as SourceType` cast, since `SourceType` is now `string`.

- [ ] **Step 4: Build frontend**

Run: `cd web && npm run build`
Expected: Clean build with no type errors.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(web): dynamic Data Explorer tabs, add AI Metrics i18n"
```

---

### Task 7: Full build and smoke test

- [ ] **Step 1: Full Rust build**

Run: `cargo build`
Expected: Clean build

- [ ] **Step 2: Run tests**

Run: `cargo test`
Expected: All existing tests pass

- [ ] **Step 3: Start server and verify**

Run: `cargo run -p neomind-cli -- serve`
Test: `curl http://localhost:9375/api/data/sources` — should return empty `ai` sources (or none if no AI metrics written yet)

- [ ] **Step 4: Final commit if any fixes needed**

```bash
git add -A && git commit -m "fix: address build/test issues from integration"
```
