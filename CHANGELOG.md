# Changelog

All notable changes to NeoMind will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [v0.7.0] - 2026-04-28

### Added

- **API Input Validation** — All POST/PUT endpoints validate parameters before processing
- **Settings Persistence** — Settings saved to redb database, survive server restarts
- **MQTT Topic Unsubscription** — Custom MQTT topics can be unsubscribed via API
- **Empty State Guidance** — All list pages show helpful guidance when empty
- **Confirmation Dialogs** — Destructive operations require explicit confirmation
- **Form Validation** — Agent, device, and rule editors validate input with inline error messages
- **Error Boundaries** — React Error Boundaries for graceful page failure handling
- **User-Friendly Error Messages** — Toast notifications show clear messages instead of raw errors
- **AI Analyst Display Title** — Agent name in dashboard widget linked to Display Title from agent config
- **JWT-Based Rate Limiting** — Per-user rate limiting with JWT client identification
- **Backend-Ready Event** — Tauri desktop startup uses event-based ready detection instead of polling
- **Aurora Background & Glass Morphism** — App-wide aurora gradient background layer with glass-style TopNav and PageLayout footer
- **OKLCH Color System** — CSS color tokens migrated from HSL to OKLCH for perceptually uniform color scales
- **Harmonized Accent Tokens** — OKLCH-based category accent colors (purple, orange, teal, rose) with consistent light/dark variants
- **Design System Tokens** — Centralized Tailwind config tokens for borders, radius, shadows, and layout spacing
- **Frontend Design Specification** — Comprehensive `DESIGN_SPEC.md` documenting all UI patterns, tokens, and conventions
- **Plus Jakarta Sans & Noto Sans SC Fonts** — New typography with Latin and CJK support
- **UnifiedFormDialog** — Centralized dialog component handling mobile/desktop, portal, escape key, backdrop click, and z-index extraction for backdrop sync
- **Chart Color Palette Redesign** — Visually distinct, accessible chart colors with better contrast

### Changed

- **Error Handling** — Replaced 1000+ hot-path `unwrap()` calls with safe error propagation across 8 crates
- **Pagination** — Standardized default page size to 10 across all pages
- **Loading States** — All page-level loading uses skeleton screens instead of spinners
- **Notifications** — Replaced `alert()` with toast notifications throughout the UI
- **Event Trigger Cooldown** — Default changed from 5s to 60s (configurable)
- **Frontend Visual Unification** — Unified visual style and component consistency across 109 frontend files
- **Centralized API Layer** — Standardized all frontend API calls through centralized `api.ts`, eliminating scattered `fetch()` calls
- **DashMap for Device Registry** — Replaced `RwLock<HashMap>` with `DashMap` for lock-free concurrent device operations
- **Lazy Telemetry Loading** — Telemetry data fetched on demand (detail view) instead of eagerly on page load
- **Rate Limit** — Raised to 5000/min for edge device workloads; frontend retries on 429
- **Design Token Migration** — All hardcoded Tailwind palette colors (blue-500, green-600, etc.) replaced with semantic design tokens (text-success, bg-error-light, text-accent-orange, etc.) across entire frontend
- **Dialog Consolidation** — 29 form dialogs migrated from raw Radix Dialog to UnifiedFormDialog with consistent behavior
- **Chat Welcome Page** — Redesigned welcome screen with improved layout
- **Checkbox Unification** — All checkbox components consolidated to use shared `Checkbox` from `ui/checkbox`
- **Vertical Stepper Redesign** — Improved step indicator with better visual hierarchy
- **Map Component** — Device icon click no longer navigates away; shows toast notification instead
- **Shared Layout Tokens** — Extracted reusable tokens for dashboard cards, dialog headers, and section layouts

### Performance

- **API Polling Storms** — Eliminated continuous polling from data explorer (debounced events), telemetry hooks (retry limit + throttle), and extension components (conditional polling)
- **N+1 Telemetry Queries** — Replaced N+1 pattern with single table scan in data sources API
- **Message Manager Lock Contention** — Write locks released before disk I/O, reducing p99 latency from 700ms
- **Session RwLock Contention** — Session resolution clones data and drops lock before async operations
- **Agent Execution Query** — Direct lookup by ID instead of fetching 100 records + linear search
- **Device Registry Concurrency** — `DashMap` eliminates lock contention for concurrent device reads/writes
- **Agent Editor Responsiveness** — Dialog opens immediately; resources loaded in background; validation on submit only
- **Blocking Call Chain Elimination** — Removed 25 blocking patterns across 28 files (frontend and backend)
- **Batch API Requests** — Frontend batches telemetry and data source requests to reduce HTTP overhead
- **Extension Polling** — YOLO device inference extension only polls when device binding is active
- **Fetch Deduplication** — TTL-based cache (10s) in Zustand store prevents redundant API calls on page remount; WebSocket device status events use optimistic updates instead of full refetch

### Fixed

- **Rule Engine** — Catch-all error recovery prevents scheduler crashes
- **Console Cleanup** — Removed 130+ non-essential console statements from frontend
- **Extension Runner** — Improved crash loop detection and panic handling
- **Session Flicker & Tab Jumping** — Fixed race conditions in chat session switching and tab state sync
- **Focus Management** — Proper auto-focus on dialog open, search input sync, CLS (Layout Shift) prevention
- **Delete Confirmation** — Consistent border-radius and confirmation dialogs for destructive actions
- **JWT Expiration** — Client-side token expiration check prevents 401 error storms from expired tokens
- **Base64 Image Handling** — Robust cleaning with re-encoding for Ollama compatibility
- **Thinking Model Compatibility** — Disabled thinking mode in agent analyzer; made `importance` field optional in memory compression response
- **Agent Editor Input Lag** — Validation runs on submit instead of every keystroke
- **Automation Page Duplicate Loading** — Prevented duplicate resource loading on automation page navigation
- **Recharts Console Warnings** — Suppressed width/height -1 warnings from responsive charts
- **Startup Health Check** — Uses HEAD method instead of GET; increased timeout for reliability
- **Telemetry Time Range** — Frontend time range aligned with backend 30-day limit
- **User Prompt Length** — Lowered minimum from 10 to 1 character for short messages
- **Dashboard First-Load Race Condition** — Components no longer show "Failed to Load Data" on initial load; deferred data fetching waits for device list to be available before showing error state
- **Nested Dialog Z-Index** — All dashboard child dialogs (Map Editor, Layer Editor, Center Picker, AI Analyst, Agent Monitor, Command Button) now render above FullScreenDialog (z:100) using z-[110]
- **Dialog Backdrop Z-Index** — UnifiedFormDialog extracts z-index from className and applies to backdrop, fixing misaligned layering
- **Dark Mode Dialog Border** — Added visible border to UnifiedFormDialog for clear edge distinction in dark mode
- **Tailwind v3 Opacity Modifiers** — Fixed all broken CSS variable opacity modifiers (bg-primary/10 silently fails); replaced with pre-defined tokens (bg-muted-30, bg-success-light) and inline styles
- **Select Text Alignment** — Fixed text alignment in Select/Combobox components
- **Dropdown Z-Index** — Fixed dropdown menus appearing behind other UI elements
- **Nav Z-Index Conflict** — Fixed TopNav layering conflict with content below
- **Aurora Background Rendering** — Fixed CSS selector issues and glass surface rendering

### Removed

- **Swagger/OpenAPI (utoipa)** — Removed unused utoipa dependencies and auto-generated spec code

### Testing

- Added comprehensive unit tests to neomind-storage (42+ new tests)
- Added comprehensive unit tests to neomind-agent (125+ tests in tools module)
- Added comprehensive unit tests to neomind-rules (93+ new tests for DSL parser and engine)
- Added comprehensive unit tests to neomind-messages (118+ total tests)
- Added comprehensive unit tests to neomind-extension-runner (79+ new tests)
- Added comprehensive unit tests to neomind-api (24 validation tests)

---

## [v0.6.12] - 2026-04-26

### Added

- **VLM Vision Dashboard Component** — New `vlm-vision` dashboard component for real-time visual analysis using VLM (Vision Language Model) models. Streams camera/video frames to LLM backends for scene understanding, object detection, and visual Q&A directly on the dashboard.
  - `useVlmSession` hook with WebSocket streaming for low-latency frame-by-frame analysis
  - `useVlmQueue` hook with drop-intermediate-frame strategy to keep only the latest frame
  - `useVlmModels` hook for listing available LLM backends as vision models
  - `VlmMessageBubble`, `VlmTimeline`, `VlmInputBar`, `VlmConfigPanel` UI components
  - Full Zustand slice for VLM session state management
  - Registry-based component library with automatic category grouping
  - Config dialog with data source binding (device metrics, extensions, AI metrics), model selector, system prompt, and context window settings
  - i18n support (English/Chinese)

- **Event-Driven Agent Triggers for Extensions** — Agents can now be triggered by extension output events, not just device metrics. This enables agents to react to AI analysis results, external API data, and custom extension outputs.
  - Unified `DataSourceRef` model (`source_type`, `source_id`, `field`) replaces device-only `EventTriggerData`
  - `check_and_trigger_data_event()` as unified entry point for all data source types
  - `matches_data_source_filter()` supporting `Device`, `Metric`, `ExtensionMetric`, `ExtensionTool` resource types
  - ExtensionOutput feedback loop prevention with source exclusion dispatch

- **Agent Status Sync** — Agent pause/activate actions now properly sync with the scheduler (pause → unschedule, activate → reschedule), ensuring UI state matches backend execution state.

- **Extension Push-Metrics API** — New `POST /api/extensions/:id/push-metrics` endpoint for device-initiated data push that immediately stores telemetry and publishes `ExtensionOutput` events to trigger downstream agents.

### Changed

- **Dashboard Component Registry** — Replaced hardcoded `getComponentLibrary()` with registry-driven approach using `groupComponentsByCategory()`, making it easier to add new component types.
- **Tauri Updater Version Comparison** — Version check now normalizes `v` prefix and whitespace before comparison, preventing duplicate update prompts when remote JSON uses `v0.6.12` format.
- **Data Source Loading Optimization** — Added `skip_telemetry` param to `/api/data/sources` to skip expensive telemetry population for bulk listing; frontend uses server-side `source_type` filtering and parallel requests; eliminated N+1 query pattern.
- **Event-Triggered Agent Cooldown** — Changed from 5s to 60s to prevent excessive LLM calls while keeping data fresh (collection stays at 60s).
- **API Retry Policy** — Frontend now retries only gateway errors (502/503/504), not 500 application errors.
- **Unified Data Source Config** — Migrated `UnifiedDataSourceConfig` from local state to Zustand store for consistency.
- **AI Analyst Session** — Enhanced `useAnalystSession` with improved data processing, multi-source value extraction, and unmount protection for API calls. Removed `useAnalystQueue` (merged into session hook).
- **Default Image Format** — Changed default camera frame format from PNG to JPEG for better bandwidth efficiency.

### Fixed

- **Recharts Chart Rendering** — Fixed "width(-1) and height(-1)" console warnings by introducing `ChartContainer` with `ResizeObserver` and explicit pixel-sized inner container, ensuring `ResponsiveContainer` always receives valid dimensions.
- **Race Condition in Agent Execution** — Fixed `get_latest_execution` querying by ID instead of potentially stale cache. Added atomic check-and-insert for scheduler concurrency. Handled `RwLock` poison gracefully instead of panicking.
- **MQTT Lock Contention** — Fixed `last_seen` read-write lock race with `try_write`; scoped dual write lock releases to prevent contention.
- **Event Bus CPU Busy-Loop** — Added `yield_now()` in `EventBusReceiver` to prevent CPU spinning.
- **Rule Engine Deadlock** — Reduced lock scope in rule engine to prevent potential deadlock.
- **Storage Consistency** — Cache updates now happen after successful DB commit, not before. LRU cache eviction optimized from O(n) to O(1).
- **Input Size Limits** — Added limits for push-metrics (100), telemetry metrics (50), extension queries (10K), agent input (100KB), and telemetry time range (30 days max).
- **Memory Leak Prevention** — Auto-cleanup for delivery logs exceeding 1000 entries. Clean empty skill index entries on removal. Extension stream clients properly cleaned on unregister.
- **Error Handling** — Return proper HTTP 500/504 for agent execution failures. Log data collection, AI metric event, and WebSocket handler errors instead of silently dropping. Handle closed semaphore gracefully.
- **AI Analyst Data Display** — Strip "produce:" prefix from extension metric field names for correct backend key matching. Extract per-metric values instead of showing raw arrays for multi-source data.
- **Data Explorer Crash** — Guard telemetry API response to prevent crash on 502/401 when `res.data` is undefined.
- **Metric Value Parsing** — Fix fallback from 0.0 to string for non-numeric metric values.
- **Console Log Cleanup** — Removed 63+ unnecessary `console.log/info/debug` calls across frontend.
- **Dead Code Removal** — Removed `DataSourceSelector`, `DataSourceSelectorContent` components, and unused system memory extraction code from agent executor.

---

## [v0.6.11] - 2026-04-21

### Added

- **Generic Telemetry API** — New `GET /api/telemetry` endpoint for querying time-series data from any source type (devices, AI metrics, transforms, extensions) using a unified interface. Accepts `source`, `metric`, `start`, `end`, `limit`, and `aggregate` (avg/min/max/sum/count) parameters. Returns data in a consistent format with `"source_id"` key. Independent of the device-specific `/api/devices/:id/telemetry` routes.
- **Server-side Pagination for Data Sources** — `GET /api/data/sources` now supports `offset`, `limit`, `source_type`, `source`, and `search` query parameters. `populate_latest_values` runs only on the paginated subset, significantly reducing DB queries for large deployments.
- **Data Explorer Redesign** — Frontend Data Explorer rewritten with server-side pagination, filtering by source type and source name, and search. Replaced client-side filtering with API-driven filtering for better performance.
- **Extension Push Mode** — Extensions can now push data to the host via a native FFI callback (`PushOutputWriterFn`), bypassing the JSON FFI round-trip. New `send_push_output()` SDK function and `neomind_extension_register_push_writer` FFI export.
- **Extension Instance Reset** — New `neomind_extension_reset_instance()` FFI export allows the runner to re-initialize extensions without restarting the process. Extension instance storage changed from `OnceLock` to `RwLock<Option<...>>` with double-checked locking.
- **CString Memory Safety** — `json_ptr()` now tracks the last 4 allocations per thread, automatically freeing the oldest when the buffer is full. Prevents memory leaks when the host doesn't call `free_string`.
- **IPC Event Subscription** — Extension runner now supports event subscription via IPC. New `event_handler.rs` and `ipc_routing.rs` modules provide channel-based stdin message routing and event state management.
- **IPC ConfigUpdate Message** — New `ConfigUpdate` IpcMessage and `ConfigUpdated` IpcResponse support hot-reloading extension configuration.
- **Extension Health & Config Metadata** — Extensions now expose `health_status`, `last_error`, `last_error_at`, and `config_parameters` fields. Frontend types updated accordingly.

### Changed

- **`device_id` → `source_id` Telemetry Renaming** — Renamed the first-level key in the telemetry time-series storage from `device_id` to `source_id` across the entire stack. This reflects the actual usage where telemetry stores data from multiple source types (devices, AI agents, transforms, extensions), not just devices. The rename covers 5 Rust crates and 20+ frontend files.
  - **Storage Layer** (`neomind-storage`): All `TimeSeriesStore` method parameters (`write`, `query_range`, `query_latest`, `delete_range`, `list_metrics`, etc.), struct fields (`BatchWriteRequest`, `TimeSeriesResult`), and internal DashMap keys renamed.
  - **Devices Wrapper** (`neomind-devices/telemetry`): `TimeSeriesStorage` and `MetricCache` methods updated. Method renames: `list_devices()` → `list_sources()`, `get_device()` → `get_source()`, `clear_device()` → `clear_source()`, `device_count()` → `source_count()`.
  - **Core Bridge** (`neomind-core/datasource`): `DataSourceId::device_part()` → `source_part()`, `from_storage_parts(device_id, ...)` → `from_storage_parts(source_id, ...)`. All internal tests updated.
  - **API Layer** (`neomind-api`): Extension metrics handlers, data source handlers, capability providers updated. Internal variable names aligned with new terminology.
  - **Agent Layer** (`neomind-agent`): AI metrics tool uses `source_id = format!("ai:{}", group)`. Tool output JSON key changed to `"source_id"`. Data collector uses `source_part()`.
  - **Extension State** (`extension_state`): `ExtensionMetricsStorage` method parameters and `ExtensionMetricsStorageAdapter` local variables renamed.
  - **Frontend Gradual Migration**: Added `sourceId` field to `DataSource` and `MapMarker` types (with `deviceId` deprecated). Introduced `getSourceId()` helper that prefers `sourceId` with `deviceId` fallback. All 20+ dashboard and config components updated to read via `getSourceId()` and write both fields.
- **Extension SDK Unified Trait** — Removed `wasm_extension` module. The `Extension` trait is now identical across native and WASM targets, simplifying cross-platform extension development.
- **IPC InFlightRequests: Sync Mutex** — Replaced `tokio::sync::Mutex` with `std::sync::Mutex` in `InFlightRequests` so `complete()`, `cancel()`, etc. can be called from synchronous contexts (receiver thread) without `block_on`.
- **Extension State Enum Simplified** — `ExtensionStateEnum` reduced to 4 states: `Running`, `RunningIsolated`, `Stopped`, `Error`. Removed unused `Discovered`, `Loaded`, `Initialized` states and `ExtensionTypeEnum`.
- **Extension Execute Response Simplified** — `ExtensionExecuteResponse` changed from a structured interface to `Record<string, unknown>` — the raw JSON result from the extension is returned directly.
- **SDK Version Bumped** — `neomind-extension-sdk` updated to v0.6.1.

### Removed

- **HTTP_REQUEST & KV_STORAGE Capabilities** — Removed `HttpRequest` and `KvStorage` from `ExtensionCapability` enum, SDK bindings, API providers (`HttpCapabilityProvider`, `KvCapabilityProvider`), and storage layer (`ExtensionKvStore`). Extensions can make HTTP calls and manage key-value data natively.
- **PermissionDenied Error** — Removed `CapabilityError::PermissionDenied` and `required_capabilities` from `ExtensionContextConfig`. Capability access is now determined solely by provider registration.
- **Dead IPC Forwarder** — Removed `start_ipc_forwarder` thread (~150 lines) and `SyncIpcRequest`/`SyncIpcResponse` types. The stdin reader thread handles all IPC routing.

### Fixed

- **SDK Macro Compilation Error** — Fixed `expected *mut i8, found Option<_>` in `neomind_export!` macro. `Vec::remove()` returns `T`, not `Option<T>` — changed `if let Some(old) = buf.remove(0)` to `let old = buf.remove(0)`.
- **Debug Logging Cleanup** — Converted 47 `eprintln!` calls to structured `tracing` macros across extension runner (`main.rs`, `ipc_routing.rs`) and core (`process.rs`). Only the panic handler retains `eprintln!` for safety.
- **Extension Upload Dialog Animation** — Fixed Loader2 spinner jittering during upload by converting inline component function to a JSX variable, preventing React unmount/remount cycles on every progress update.
- **Extension Bundle Cache Stale Issue** — Fixed browser loading old UMD bundles after extension reinstall/update. Three fixes applied:
  - Store's `unregisterExtension` now clears `DynamicRegistry` caches and global variables.
  - Upload dialog clears extension caches before re-syncing component registry.
  - `syncComponents` detects `bundle_url`/`global_name`/`export_name` changes and clears stale module caches.
- **Loading State Improvements** — Skeleton screen patterns improved across `LoadingState` and `ResponsiveTable` components.
- **Tauri Version Mismatch** — Fixed `tauri.conf.json` showing stale version while Cargo.toml was already updated.

### Preserved (Not Changed)

- **Extension SDK Wire Protocol**: JSON parameter key `"device_id"` unchanged — avoids breaking external extensions.
- **Device Management Code**: Device register/unregister/status/config/command handlers use `device_id` semantically and correctly.
- **API URL Routes**: All existing HTTP routes (`/api/devices/:id/telemetry`, etc.) unchanged.
- **redb File Format**: Binary storage format unaffected — only variable names changed.
- **`device_type` Fields**: Retention policy fields in storage layer correctly preserved as a separate concept.

---

## [v0.6.10] - 2026-04-20

### Added

- **AI Metrics Tool** — New `ai_metric` tool enables LLM agents to create and query custom time-series metrics (anomaly scores, predictions, derived indicators). Actions: `write` (persist data point + metadata), `read` (list all metrics with latest values or query time-series for a specific metric). Metrics appear in the Data Explorer via `ai:{group}:{field}` data source IDs. Metadata persists across restarts via JSON file.
- **AI Metrics Registry** — `AiMetricsRegistry` provides shared metadata storage between `AiMetricTool` (writes) and the data sources handler (reads), with disk persistence in `data/ai_metrics_metadata.json`.
- **Dynamic Data Explorer Tabs** — Frontend Data Explorer now dynamically creates tabs for all registered data source types, including AI Metrics. Tab content auto-refreshes when new sources are discovered.
- **Unified Data Sources Collector** — `collect_ai_sources` handler collects AI metric data sources alongside device, extension, and transform sources for the unified data API.

### Changed

- **Agent Execution Mode Redesign** — Renamed Chat Mode → **Focused Mode** and React Mode → **Free Mode** with clear differentiation across all layers (backend, API, frontend, LLM tools).
  - **Focused Mode**: User binds resources (required), LLM works within defined scope using structured data tables and decision templates. Single-pass, token-efficient. Best for monitoring, alerts, data analysis.
  - **Free Mode**: LLM freely explores with all 8 tools (device, agent, rule, message, extension, transform, skill, shell), no resource binding needed. Multi-round reasoning. Best for complex automation and device control.
- **Structured Prompt for Focused Mode** — Focused Mode prompt now uses structured Markdown tables (data table + command table + decision template) instead of loose text, improving LLM reliability for command execution.
- **Scope Validation** — Focused Mode command execution validates that commands are within bound resources, rejecting out-of-scope commands with warning logs.
- **Data Collection Config UI** — Focused Mode metric resources now show configurable data collection settings (time range, include history, trend analysis, baseline comparison) in the agent editor.
- **Notification/Alert in Focused Mode** — Focused Mode can send notifications and alerts without binding, as inherent agent capabilities.
- **Focused Mode API Validation** — Create/update agent API returns 400 error if Focused Mode has no resource binding.
- **ExecutionMode Enum** — `Chat`/`React` renamed to `Focused`/`Free` with serde aliases for backward compatibility. Old values (`"chat"`, `"react"`) still accepted via deserialization.
- **Frontend Mode Cards** — Agent editor mode selection updated with new names, icons, descriptions, and "Required" badge for Focused Mode.
- **Free Mode Resource Binding Removed** — Free Mode no longer shows resource binding section. Resources cleared when switching to Free Mode.
- **LLM Tool Descriptions** — Agent tool parameter descriptions (`execution_mode`, `resources`, `enable_tool_chaining`) in both `aggregated.rs` and `simplified.rs` updated to reflect Focused/Free semantics and resource binding rules.
- **Internal Naming Unified** — `AnalysisResult` enum variants, all doc comments, tracing messages, and log strings updated from Chat/React to Focused/Free across `neomind-agent`, `neomind-storage`, and `neomind-api`.
- **Shell Tool** — New `shell` tool enables AI agents to execute system commands on the host. Features: login shell (`$SHELL -l -c`) for full user environment (PATH, aliases), cross-platform support (Unix/macOS/Windows), configurable timeout (max 600s), output truncation (10K chars), UTF-8 safe truncation, process group isolation for clean timeout kill. Parameters: `command` (required), `timeout`, `working_dir`, `description` (audit log).
- **Agent Skill System** — User-defined skill management via the `skill` tool. Actions: `search`, `list`, `get`, `create`, `update`, `delete`. Skills are YAML frontmatter + Markdown files that provide scenario-driven operation guides for the AI agent. Includes keyword matching, token budget injection, and persistence.
- **Skills Panel UI** — Frontend panel in agent settings for creating, editing, and deleting user skills with a code editor. Supports YAML frontmatter syntax highlighting.
- **Action Enum Constraints** — LLM tool definitions now include `enum` constraints on the `action` parameter for all aggregated tools, so the LLM knows exactly which actions are available (e.g., `device` supports `list|latest|history|control|write_metric`).
- **Removed Builtin Skills** — Removed 8 hardcoded builtin skills (753 lines) that duplicated tool descriptions. The skill system now focuses on user-defined multi-tool workflow skills only.
- **Enhanced Tool Descriptions** — All 6 aggregated tool descriptions (device, agent, rule, message, extension, transform) enhanced with critical workflow hints (confirm flow, list-first pattern, required fields) to compensate for removed builtin skills.
- **Login Shell for Shell Tool** — Uses `$SHELL` environment variable with `-l` flag for full user environment; falls back to `/bin/sh -c` without `-l` in minimal environments (Docker, IoT edge).
- **Adaptive Tool Timeout** — Outer tool execution timeout in `execute_with_retry_impl` now adapts to shell tool's internal timeout (`shell_timeout + 5s` buffer) instead of hardcoded 30s.
- **Tool Name Mapper** — Added `skill` and `shell` with Chinese/English aliases (命令行, 终端, bash, cli, 技能, 指南, etc.) for fuzzy tool name resolution.
- **Non-Simplified Tool Registration** — `update_tool_definitions` now registers ALL tools from the registry (not just extension tools) that aren't already in simplified definitions, fixing shell tool not being visible to the LLM.
- **Automation Simplified** — Removed complex automation modes, simplified to transform-only workflow. Unified loading states across frontend components.

### Fixed

- **Tool Result Compaction Echoing** — The old `[Called: tool(args) → result]` compaction format was being echoed verbatim by smaller LLMs instead of generating new tool calls. Replaced with natural language sentences that clearly indicate past results and instruct the model not to repeat them.
- **AI Metric Discoverability** — `ai_metric` `read_list` returned empty when metrics were written without optional `unit`/`description` fields because metadata was only registered conditionally. Now always registers metadata on write so all metrics are discoverable.
- **AI Metric Tool Description** — Improved `ai_metric` tool description with clear examples for write and read actions, making it easier for LLMs to use correctly.
- **AI Metric Metadata Persistence** — AI metrics metadata now persists to `data/ai_metrics_metadata.json` across server restarts via `AiMetricsRegistry` disk persistence.
- **Shell Timeout Parameter** — `timeout` parameter now accepts both number (`30`) and string (`"30"`) forms, fixing LLM passing string values through simplified schema.
- **Simplified Tool Description Accuracy** — Fixed `device` tool description: `get` → `latest`, added missing `write_metric` action. Fixed `message` tool: added missing `get` action.
- **Cross-Platform Shell Dependencies** — `libc` moved to Unix-only target dependency, `windows-sys` added as Windows-only dependency for proper cross-compilation.

### Added

- **Agent Execution Mode Redesign** — Renamed Chat Mode → **Focused Mode** and React Mode → **Free Mode** with clear differentiation across all layers (backend, API, frontend, LLM tools).
  - **Focused Mode**: User binds resources (required), LLM works within defined scope using structured data tables and decision templates. Single-pass, token-efficient. Best for monitoring, alerts, data analysis.
  - **Free Mode**: LLM freely explores with all 8 tools (device, agent, rule, message, extension, transform, skill, shell), no resource binding needed. Multi-round reasoning. Best for complex automation and device control.
- **Structured Prompt for Focused Mode** — Focused Mode prompt now uses structured Markdown tables (data table + command table + decision template) instead of loose text, improving LLM reliability for command execution.
- **Scope Validation** — Focused Mode command execution validates that commands are within bound resources, rejecting out-of-scope commands with warning logs.
- **Data Collection Config UI** — Focused Mode metric resources now show configurable data collection settings (time range, include history, trend analysis, baseline comparison) in the agent editor.
- **Notification/Alert in Focused Mode** — Focused Mode can send notifications and alerts without binding, as inherent agent capabilities.
- **Focused Mode API Validation** — Create/update agent API returns 400 error if Focused Mode has no resource binding.

### Changed

- **ExecutionMode Enum** — `Chat`/`React` renamed to `Focused`/`Free` with serde aliases for backward compatibility. Old values (`"chat"`, `"react"`) still accepted via deserialization.
- **Frontend Mode Cards** — Agent editor mode selection updated with new names, icons, descriptions, and "Required" badge for Focused Mode.
- **Free Mode Resource Binding Removed** — Free Mode no longer shows resource binding section. Resources cleared when switching to Free Mode.
- **LLM Tool Descriptions** — Agent tool parameter descriptions (`execution_mode`, `resources`, `enable_tool_chaining`) in both `aggregated.rs` and `simplified.rs` updated to reflect Focused/Free semantics and resource binding rules.
- **Internal Naming Unified** — `AnalysisResult` enum variants, all doc comments, tracing messages, and log strings updated from Chat/React to Focused/Free across `neomind-agent`, `neomind-storage`, and `neomind-api`.

- **Shell Tool** — New `shell` tool enables AI agents to execute system commands on the host. Features: login shell (`$SHELL -l -c`) for full user environment (PATH, aliases), cross-platform support (Unix/macOS/Windows), configurable timeout (max 600s), output truncation (10K chars), UTF-8 safe truncation, process group isolation for clean timeout kill. Parameters: `command` (required), `timeout`, `working_dir`, `description` (audit log).
- **Agent Skill System** — User-defined skill management via the `skill` tool. Actions: `search`, `list`, `get`, `create`, `update`, `delete`. Skills are YAML frontmatter + Markdown files that provide scenario-driven operation guides for the AI agent. Includes keyword matching, token budget injection, and persistence.
- **Skills Panel UI** — Frontend panel in agent settings for creating, editing, and deleting user skills with a code editor. Supports YAML frontmatter syntax highlighting.
- **Action Enum Constraints** — LLM tool definitions now include `enum` constraints on the `action` parameter for all aggregated tools, so the LLM knows exactly which actions are available (e.g., `device` supports `list|latest|history|control|write_metric`).

### Changed

- **Removed Builtin Skills** — Removed 8 hardcoded builtin skills (753 lines) that duplicated tool descriptions. The skill system now focuses on user-defined multi-tool workflow skills only.
- **Enhanced Tool Descriptions** — All 6 aggregated tool descriptions (device, agent, rule, message, extension, transform) enhanced with critical workflow hints (confirm flow, list-first pattern, required fields) to compensate for removed builtin skills.
- **Login Shell for Shell Tool** — Uses `$SHELL` environment variable with `-l` flag for full user environment; falls back to `/bin/sh -c` without `-l` in minimal environments (Docker, IoT edge).
- **Adaptive Tool Timeout** — Outer tool execution timeout in `execute_with_retry_impl` now adapts to shell tool's internal timeout (`shell_timeout + 5s` buffer) instead of hardcoded 30s.
- **Tool Name Mapper** — Added `skill` and `shell` with Chinese/English aliases (命令行, 终端, bash, cli, 技能, 指南, etc.) for fuzzy tool name resolution.
- **Non-Simplified Tool Registration** — `update_tool_definitions` now registers ALL tools from the registry (not just extension tools) that aren't already in simplified definitions, fixing shell tool not being visible to the LLM.

### Fixed

- **Shell Timeout Parameter** — `timeout` parameter now accepts both number (`30`) and string (`"30"`) forms, fixing LLM passing string values through simplified schema.
- **Simplified Tool Description Accuracy** — Fixed `device` tool description: `get` → `latest`, added missing `write_metric` action. Fixed `message` tool: added missing `get` action.
- **Cross-Platform Shell Dependencies** — `libc` moved to Unix-only target dependency, `windows-sys` added as Windows-only dependency for proper cross-compilation.

---

## [v0.6.9] - 2025-04-16

### Added

- **Transform Aggregated Tool** — New `transform` tool enables LLM agents to manage JavaScript-based data transforms through natural conversation. Actions: `list`, `get`, `create`, `update`, `delete`, `test`. Supports scope-based targeting (global, device type, specific device), extension invocation via `extensions.invoke()`, and custom output prefixes. Full multilingual support (English/Chinese).
- **TransformStore Trait Abstraction** — `TransformStore` trait in `neomind-agent` with async CRUD methods using `serde_json::Value` for cross-crate data transfer, implemented for `SharedAutomationStore` in `neomind-api`. Avoids circular dependency between crates.
- **Virtual Metrics in Device Tool** — `device(action="list")` (detailed mode) now includes `virtual_metrics` field showing metrics from Transform/extension writes not in the device template. `device(action="latest")` appends virtual metrics with latest values into the metrics array, so the LLM can see and query all available metrics.
- **Device Write Metric Action** — New `device(action="write_metric")` action allows the AI agent to write values to device metrics. Accepts `device_id`, `metric`, `value` (string/number/boolean/null), and optional `timestamp`. Enables calibration values, status flags, computed results, and any AI-generated data to be persisted on devices.
- **Dynamic Context Compaction** — Context compaction parameters (`keep_recent`, `history_share`, `message_length`) now adapt to model capacity (>16k/8k-16k/<8k). Large models get 95% effective context allocation.
- **LLM Default Context Length** — Default max context token increased from 4096/8192 to 128000 across all backends (Ollama, llama.cpp, mock), matching modern model capabilities.
- **GLM & MiniMax Model Detection** — Added context length detection for GLM (128k) and MiniMax/abab (512k) models.

### Changed

- **Keyword Planner** — Rule intent planner now distinguishes transform-related queries from rule queries, routing to the correct tool (transform vs rule) based on message keywords (convert, transform, data processing, 数据转换, 数据解析, etc.).
- **Unified Alert/Message Tools** — Alert tool merged into message tool with consistent descriptions and examples.
- **Anti-Hallucination Tool Formatting** — Tool result summaries now use structured markers (`**[ToolResult:agent]** preview...`) instead of predictable "✓ tool executed successfully" patterns, making it harder for the LLM to memorize and hallucinate responses in long conversations.

### Fixed

- **Tool Result Cache Invalidation** — Cache not invalidated on write actions (create/update/delete/control) across all tools, causing stale data on subsequent reads. Now properly invalidated after all mutations.
- **`_raw` Metric Filtering** — `_raw` and `*_raw` metrics (containing large base64 images, full MQTT payloads) now replaced with `[raw payload, {size}]` in tool output, preventing token waste in LLM context. Virtual metrics discovery also skips these noise fields.
- **Duplicate Round Content** — Last tool-call round's content was displayed twice: once in the tool round block and once as the final message. Fixed in both backend (no longer storing `final_response_content` in `round_contents_map`) and frontend (no longer saving last round content on stream end).
- **Message List Detection** — `message(list)` output was misidentified as "Conversation Log". Added message-object detection (title/level/read fields) for correct formatting.
- **User Message Preservation** — User messages now always preserved in context window (User priority >= System), preventing critical context loss during compaction.

---

## [v0.6.8] - 2025-04-15

### Added

- **Per-Round Thinking Persistence** — Backend now tracks and stores thinking content per tool-call round (`round_thinking` field on `AgentMessage`), enabling grouped rendering in the frontend with visual round labels and color-coded badges.
- **Thinking Deduplication** — Frontend detects and hides thinking content that duplicates the final response (Phase 2 LLM echo), avoiding redundant display.
- **Streaming Loading Indicator** — Consistent loading dots shown during streaming when content hasn't arrived yet, replacing the previous empty-gap behavior after tool calls or thinking blocks.

### Changed

- **LLM Pipeline Optimization** — Removed deprecated `is_likely_thinking` filter in Ollama paths (Ollama already separates content/thinking correctly); removed keyword-based thinking control overrides — thinking now respects user/instance `thinking_enabled` setting directly (`Instance setting → LlmInterface → Ollama backend`).
- **Unified LLM Defaults** — Standardized parameters across configs: temperature 0.3, top_p 0.7, top_k 40, repeat_penalty 1.05 for better tool-calling determinism.
- **Prompt Cleanup** — Removed Quick Reference table and tool description double-injection from system prompts (~284 lines of deprecated constants removed from `builder.rs`); tool definitions now handled entirely by `PromptBuilder`.
- **Unified Chat Text Sizing** — All chat message block font sizes unified to 13px (thinking content, tool call content, markdown body, round content), with labels at 11px. Previously ranged from 10px–14px across different blocks.
- **Softer Block Styling** — Thinking and tool-call blocks now use borderless rounded backgrounds (`bg-muted/30`) instead of hard borders, for a cleaner visual appearance.
- **Tool Call Block Spacing** — Tool call block uses `mb-4` bottom margin to create clear separation from the final response content below.

### Fixed

- **Multi-Round Thinking Display** — Thinking content now accumulates across all tool-call rounds instead of resetting on each round transition, so all rounds' thinking is visible during streaming.
- **Duplicate Loading Indicators** — Removed legacy standalone loading dots that conflicted with the new inline loading, preventing double indicators on empty streaming messages.
- **Rule Builder Extension Support** — Fixed validation in rule creation that blocked "Next" when selecting an extension as data source (only checked `device_id`, ignored `extension_id`). Fixed trigger building for extension conditions (was always empty `device_id`). Fixed `RuleAction::Set` on backend not routing to extension executor — Set actions targeting extensions now correctly execute via `ExtensionActionExecutor`.
- **Model Selector Overflow** — Added `max-h-[50vh] overflow-y-auto` to LLM model dropdown to prevent long model lists from overflowing the viewport.
- **Embedded Tool Call JSON in Display** — Small models (e.g. 4B) often output tool call JSON (`[{"name":"device",...}]`) as plain text mixed with markdown code blocks. Three-layer fix:
  - **Backend hold-back**: Streaming buffer now also detects `{"`, `{"name"`, and ```json``` patterns — not just `[` — to prevent partial JSON fragments from being yielded to the frontend.
  - **Backend storage cleaning**: `remove_tool_calls_from_response` applied at all 4 message storage points (main tool path, multimodal path, no-tool paths) and enhanced with ```json code block regex cleaning. `content_before_tools` is also cleaned before storing as round content.
  - **Frontend display cleaning**: `cleanToolCallJson()` applied to both `round_contents` and message content during rendering, covering streaming and persisted messages.

### Changed

- **Dead Chinese Prompt Code Removed** — Removed 481 lines of unused Chinese prompt constants (`*_ZH`) and associated methods from `builder.rs`. The `LANGUAGE_POLICY` header already instructs models to respond in the user's language, making separate Chinese prompts unnecessary. Only `CONVERSATION_CONTEXT_ZH` retained (still used by agent executor memory system).

---

## [v0.6.7] - 2025-04-14

### Added

- **Ollama Capabilities-Based Vision Detection** — Vision detection now prioritizes the Ollama API `capabilities` array (authoritative source) over `model_info` heuristic, with fallback for older Ollama versions.
- **qwen3.5 Multimodal Support** — Full qwen3.5 series (including `qwen3.5:4b` local models) now correctly detected as multimodal across all detection paths.
- **Agent Thinking Panel Collapsible** — Agent thinking panel now supports collapse/expand with a preview line, reducing visual clutter during execution monitoring.
- **Tauri Keyboard Fix** — Prevent Backspace/Delete from triggering browser back navigation in Tauri WebView.

### Changed

- **Agent Card Layout** — Simplified footer layout; executing status shown inline with spinner instead of separate thinking block.
- **Agent Detail Panel** — Executions are preloaded on agent selection instead of waiting for history tab; auto-reload on execution completion.
- **Unified Vision Detection** — All backend vision detection now uses `neomind-core`'s `detect_vision_capability()` for consistency.
- **Capability Upgrade Logic** — Backend capability detection only upgrades (false→true), never downgrades API-detected values that are already persisted.

### Fixed

- **Dashboard LineChart Stale Data** — Removed React.memo from LineChart component that prevented data updates.
- **DevicesPage Performance** — Grouped selectors with `shallow` equality to reduce unnecessary re-renders.
- **Telemetry Query Concurrency** — Added semaphore to limit concurrent telemetry queries to 16, preventing resource exhaustion.
- **Storage Performance** — Single DB query for device state instead of double lookup; paginated scan avoids loading all results; range query replaces full table scan.
- **UTF-8 Key Safety** — Safe `increment_prefix` for UTF-8 keys in storage, with semaphore error logging.

---

## [v0.6.6] - 2025-04-14

### Added

- **Token Usage Reporting & Context Summarization** — Agent streaming now reports token usage per turn. Sessions auto-summarize when context exceeds model limits, preserving conversation continuity across long sessions.
- **Context Summarization API** — New `POST /api/sessions/:id/summarize` endpoint for manual context compression.

### Changed

- **Agent Toolkit Consolidation** — Merged and simplified tool definitions, removed unused system tools (DSL, MDL, rule-gen) for cleaner agent context and faster tool resolution (~3400 lines removed).
- **Streaming Refactor** — Agent streaming handler restructured for better error recovery and token tracking.

### Fixed

- **Memory Compression Safety** — Compression now preserves high-importance entries instead of sending all entries to LLM. Only entries exceeding category limits are compressed, and the top half is always kept intact.
- **Over-Aggressive Merge Protection** — New safety threshold blocks compression when LLM returns fewer than 20% of the entries it was given, preventing catastrophic memory loss from small models over-merging.
- **Extract/Compress Decoupling** — `POST /api/memory/extract` no longer auto-triggers compression on all categories. Compression runs only via the scheduler or manual `POST /api/memory/compress` trigger.
- **Default Context Length** — Use 8192 as default `max_context` instead of 0, preventing context overflow on backends that don't report model limits.
- **Ollama Model Context Detection** — Correct context size detection for ministral and other models that report context length differently in the Ollama API.
- **Tauri Updater CI** — Fixed artifact paths and auto-generation of `latest-update.json` in GitHub Actions workflow.

---

## [v0.6.5] - 2025-04-13

### Added

- **Token-Based Context Management** — Conversation history managed using token counting instead of message count, with automatic context overflow retry for resilience across LLM backends.
- **Dashboard Grid Rewrite** — Ref-based `react-grid-layout` integration eliminates feedback loops between layout state and re-renders, fixing jitter and positioning bugs.
- **Config Data Refresh** — Component data updates immediately when editing data binding in config dialog, with `configVersion` tracking for live re-renders.
- **Chart Responsive Resize** — Chart components (LineChart, BarChart, PieChart, AreaChart) properly fill their container via flex-based layout.
- **New Component Default Size** — Dashboard components appear at correct default sizes instead of 1×1 minimum.
- **Aggregated Tool Enhancements** — Added `latest_execution` and `send_message` tool actions for agent execution monitoring and control.
- **Agent Execution Timeline** — Refactored timeline with tool thinking event support and improved event rendering.
- **React/Chat Dual-Path Execution** — Agents support both React reasoning loop and direct chat execution paths with background API.
- **Concise React Prompts** — Optimized agent React prompts and UTF-8 truncation safety.
- **Execution Detail Layout** — Improved execution detail dialog layout.

### Fixed

- **Streaming Tool Calls** — Fixed tool call streaming event handling in chat interface.
- **Sidebar Scroll** — Fixed sidebar scroll behavior and chat layout issues.
- **Scheduler Panic** — Fixed agent scheduler panic on concurrent access.
- **Thinking Model Compatibility** — Memory extraction and compression LLM calls now disable thinking (`thinking_enabled: Some(false)`), preventing token waste on reasoning models (qwen3.x, deepseek-r1).
- **Memory Config Alignment** — Backend `ExtractionConfig` now matches frontend Config UI fields.
- **Memory Extraction Returns Zero** — Fixed extraction returning 0 entries when using thinking-capable models.
- **llama.cpp Multimodal Detection** — Auto-detect vision, tool calling, and context size from `/props` endpoint.
