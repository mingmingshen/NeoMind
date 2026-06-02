# Changelog

All notable changes to NeoMind will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [v0.8.4] - 2026-06-02

### Changed

- **System prompt slim-down (73% token reduction)** — Reduced AI agent system prompt from ~7,500 to ~1,800 tokens, freeing ~5,700 tokens per request for conversation history (+32% available context). Three-layer architecture: (1) system prompt for core decision rules, (2) CLI `--help` for command details loaded on demand, (3) skill tool for complex workflows loaded on demand. Removed redundant CLI reference table (already in shell tool JSON description), few-shot examples (modern tool-calling models don't need them), and consolidated duplicate rules across PRINCIPLES/RESPONSE_FORMAT/THINKING_GUIDELINES. Added Typical Workflows table, response format patterns, error handling hint, and vision hint. Integrated vision capability detection so multimodal models automatically receive image analysis instructions.

### Added

- **Vision tool** — AI agent can now analyze images from HTTP URLs, local files, data URLs, or raw base64 using a vision-language model (VLM). Auto-detects VLM backends via `supports_multimodal` capability and registers the tool automatically. Security hardened: SSRF protection with per-redirect validation, symlink-safe file reads via canonicalize-then-validate, MIME allowlist for data URLs, file extension whitelist with magic bytes validation, 10MB size limit. VLM backend selection follows priority: explicit config → active backend → first multimodal instance
- **4 new bridge extensions** — Home Assistant Bridge, LoRaWAN Bridge, Modbus Bridge, and Uink-RMS Bridge added to the extension marketplace for broader IoT protocol coverage
- **Layered multimodal capability detection** — Replace hardcoded heuristic with 3-tier resolution: LiteLLM registry (2,748 embedded model entries) → conservative heuristic → false. Add user override endpoint (`PATCH /api/llm-backends/:id/capabilities`), background refresh loop for Ollama instances (hourly `/api/show` polling), and source tracking (`user_override` > `runtime_api` > `registry` > `heuristic`). HTTP images pre-encoded to base64 for Ollama compatibility
- **i18n comprehensive standards** — Added section 12 to DESIGN_SPEC.md covering namespace rules, key naming convention (`{page}.{section}.{field}`), cross-namespace references, and common mistakes checklist
- **Dashboard tab bar layout mode** — Alternative to the left sidebar: a horizontal scrollable tab bar rendered inline in the toolbar header, freeing the full content width for the dashboard grid. Toggle via `PanelTop` button in the sidebar header or `PanelLeft` button on the tab bar; preference persisted in `localStorage` (`neomind_dashboard_layout_mode`). Active tab has a distinct `bg-muted` style with an elastically-expanding `⋮` action menu (cubic-bezier overshoot easing, 200ms) that reveals Rename/Delete on hover — no floating overlay. Left side has `[≡ sidebar][+]` controls; tab names truncate at 200px with full-name tooltip on hover. Layout mode is independent from the existing sidebar collapse state and fullscreen mode
- **Tooltips on dashboard toolbar action buttons** — Edit/Done, Add Component, Share, and Fullscreen buttons now use Radix `Tooltip` (300ms delay) instead of native `title=` attribute, matching the hover-label pattern used elsewhere in the dashboard chrome
- **i18n keys for tab bar** — Added `sidebar.switchToTabs` and a new `tabBar.*` namespace (`newDashboard`, `namePlaceholder`, `deleteTitle`, `deleteDescription`, `delete`, `rename`, `switchToSidebar`) in both `en` and `zh`

### Changed

- **Extensions page empty state redesign** — Replaced generic "No extensions found" with a rich ecosystem showcase: horizontal marquee scrolling 12 real extension preview cards (YOLO Video, Face Recognition, BACnet, Modbus, LoRaWAN, ONVIF, Home Assistant, Stream Player, Weather, OCR, OPC-UA, Image Analyzer), 8 category tags matching actual extension types, and CSS-only animation with edge fade and hover-pause

- **Messages page filter redesign** — Replaced heavy Sheet side-drawer with a lightweight Popover dropdown filter panel. Compact pill-style buttons replace bulky collapsible sections for Severity, Status, and Category filters. Removed CollapsibleSection component and unused Sheet/Separator/ChevronDown imports

- **Multimodal image upload avoids redundant vision tool call** — When a user uploads an image to a multimodal-capable model (e.g., GPT-4o, qwen-vl), the image is sent directly as native `Content::Parts` and the `vision` tool is filtered from the tool list. This prevents the model from calling the vision tool on images it can already see, eliminating a redundant LLM round-trip (following industry best practice: OpenAI, Anthropic, CrewAI all recommend native multimodal over tool-mediated vision)

- **Dashboard telemetry data split** — Separated real-time device telemetry (`deviceTelemetry` Record) from the `devices` array to eliminate cascading re-renders. Previously, every WebSocket metric update mapped over the entire `devices` array, causing all dashboard components to re-render. Now high-frequency metric writes only update a per-device telemetry map, while the `devices` array reference stays stable. Dashboard components use targeted selectors with `shallow` equality to re-render only when their bound device's telemetry changes. This reduces re-renders from O(n) per metric update to O(1).

- **Clippy cleanup** — Fixed 45 clippy warnings across 4 crates (`neomind-cli-ops`, `neomind-storage`, `neomind-agent`, `neomind-api`). Introduced `CredentialValidator` type alias for complex closure types, replaced `iter().cloned().collect()` with `to_vec()`, used `strip_prefix` instead of manual slicing, and resolved `await_holding_lock` in shutdown by cloning `Arc` before dropping the read guard
- **Dashboard list sorted by creation time** — Both sidebar and tab bar now display dashboards ordered by `createdAt` ascending (oldest first, newest at end), independent of backend fetch order or sync remapping. Newly created dashboards always appear at the end of the list
- **Sidebar collapsed-mode cleanup** — Removed the `+` (new dashboard) button and its divider from the collapsed sidebar view. The collapsed column now shows only the dashboard icon list; creation requires expanding the sidebar first or using the tab bar's `+` button
- **Sidebar item always shows component count** — Removed the `count > 0` guard so dashboards with 0 components display "0 components" instead of hiding the count row entirely

### Fixed

- **i18n ZH translations** — Batch translated 422 missing Chinese keys across 15 namespace files. Achieved EN/ZH parity (5,370 keys each, 17 active namespaces)
- **i18n reference consistency** — Fixed `ConfigFieldComponents.tsx` using default namespace instead of explicit `dashboardComponents`. Fixed 6 files using `t('common.xxx')` anti-pattern in default namespace context (`DeviceBindingConfig`, `MessageChannelsTab`, `MessagesTab`, `messages.tsx`). Fixed `settings.tsx` listing unregistered namespaces (`llm`, `connections`)
- **Create-dashboard navigation race** — `handleDashboardCreate` now `await flushSync()` and reads the final `currentDashboardId` from the store before navigating, so the URL receives the stable post-remap dashboard id. Previously the URL would hold the local temporary id while the store later updated to the backend-assigned id, causing the URL ↔ Store sync to bounce the user off the newly created dashboard

### Removed

- **~8,900 lines of dead frontend code** — Removed 30+ unused components, hooks, and utility modules that were superseded by page-level implementations:
  - `components/automation/` — AlertsTab, AutomationCreatorDialog, AutomationsTab, TransformsTabContent, TransformExecutionHistory (replaced by `pages/automation-components/`)
  - `components/devices/` — DeviceControl, DeviceRealtime, TemplatePreview (replaced by `pages/devices/`)
  - `components/extensions/` — ExtensionDataSourceSelector, ExtensionMetricSelector, ExtensionToolSelector, ExtensionTransformConfig (inlined into pages)
  - `components/shared/` — BulkActionBar, FullScreenEditor, KeepAlive, MonitorStatsGrid, SearchBar, SearchResultsDialog (unused)
  - `components/layout/` — SubPageHeader (unused)
  - `hooks/` — useApiData, useComponentPerf, useDialog, useInterval, useLoadingButton, useMessages (replaced by store-level fetchCache pattern)
  - `lib/` — extension-stream-hooks, fetch-with-timeout, react-query-hooks, status/utils, validation/utils, related test
- **Dead i18n namespaces** — Removed `commands.json` (58 keys, 0 references), `navigation.json` (12 keys, duplicated by `common.json` `nav`/`navShort`), `tools.json` (11 keys, unused), and orphaned camelCase `dashboardComponents.json` (merged into hyphenated version)
- **245 duplicate i18n keys from common.json** — Removed sections that existed identically in `dashboard-components.json`: visualDashboard, sizes, imageDisplay, imageHistory, layerDisplay, mapDisplay, markdownDisplay, placeholders, range, searchBar, videoDisplay, webDisplay, common

---

## [v0.8.3] - 2026-06-01

### Added

- **Docker deployment** — Production-ready multi-stage Dockerfile (Node 20 frontend + Rust 1.85 backend + Alpine runtime), docker-compose.yml with named volume persistence, health check, and `.env.example` configuration template. Single container includes API server, Web UI, embedded MQTT broker, and extension runner


- **Agent experience learning** — New per-execution insight extraction and LLM-driven task profile reflection. Agent memory now accumulates actionable knowledge over time instead of just recording what happened:
  - `MemorySummary.insight` — Inline insight from main LLM output (focused mode) or deterministic extraction (free mode: failure reasons, alert/command triggers, >20% baseline deviation, anomaly keywords). Zero extra LLM calls
  - `TaskProfile` — Evolving task-level knowledge summary (max 500 chars) generated by LLM reflection when ≥5 insights accumulated (first time) or 6-hour staleness (updates). Includes version, execution count, and freshness tracking
  - Task Knowledge injected as highest-priority section in `build_history_context()` for LLM decision-making
  - Short-term summary cards now display insights with lightbulb icon in agent detail panel
  - API DTOs (`MemorySummaryDto`, `AgentMemoryDto`) expose `insight` and `task_profile` fields
  - i18n support for Task Knowledge and Recent Key Findings (en/zh)
- **`web_fetch` tool** — AI agent can now fetch URL content directly. Returns cleaned text (HTML stripped) or raw content with configurable max length (default 5000, max 50000 chars). Security: SSRF protection blocks private/local IPs (localhost, 10.x, 192.168.x, 172.16-31.x, IPv6 unique local, link-local, IPv4-mapped IPv6), validates redirect targets, enforces 15s timeout and 1MB response limit. Content-Type media type parsing prevents binary bypass via parameter injection
- **`file_write` tool** — AI agent can create or overwrite files within allowed directories (data dir + `NEOMIND_ALLOWED_WRITE_DIRS` env var). Atomic writes via temp-file-then-rename. Supports all text file types (.rs, .toml, .py, .js, .json, .md, .conf, etc.). Blocks binary extensions (.so/.dll/.exe/.sys) and .env files. Content limit: 1MB. Auto-creates parent directories by default. Preserves file permissions on overwrite
- **`file_edit` tool** — AI agent can perform precise string replacement in existing files. Parameters: `old_string`/`new_string` with optional `replace_all`. CRLF/LF line ending normalization for cross-platform matching. File size limit: 10MB. Error messages include file preview for context when old_string not found. Atomic write preserves file permissions
- **`path_validator` module** — Shared security layer for file tools. Symlink escape prevention via `find_existing_ancestor()` + canonicalization. Path traversal (`..`) detection at component level. `NEOMIND_ALLOWED_WRITE_DIRS` env var for extension development directories
- **Memory tool 2-file API** — New file-based memory endpoints: `GET/PUT /memory/file/{category}` for direct file read/write. Memory tool now supports custom category files (`custom/{name}.md`) and per-request session binding via shared handle
- **Device list grouped by type** — `neomind device list` now groups devices by `device_type`, shows metric schema with example values from online devices (parallel enrichment), and truncates large lists (>50 devices) for token budget protection
- **LLM backend create via CLI** — `neomind llm create` registers new LLM backend instances from the command line
- **Thinking model loop detection** — Ollama backend detects and cuts off runaway thinking (loops, excessive length) for qwen3/deepseek-r1 models
- **Chat page context injection** — When the global chat FAB is opened from a page (dashboard, devices, automation, etc.), a short neutral context string (`[context] page:dashboard "name", N components`) is automatically prepended to the first user message so the AI knows which page the user is on. Context is reactive to route changes, injected only on the first message per session, and resets on new conversation
- **Dashboard community components split** — Component library now separates "My Components" (locally created / AI-generated) from "Marketplace" (installed from registry). Added `source` field to distinguish origins, with reinstall support for local components to refresh updated bundles
- **System context resource inventory** — Periodic background task gathers device/agent/extension/dashboard names and writes to KNOWLEDGE.md `<!-- system-context -->` marker section (800 char limit, 10min interval). AI now knows what resources exist without tool calls
- **LLM chat/agent summarization** — Periodic background task uses LLM to summarize recent chat sessions → `<!-- chat-summary -->` in USER.md (200 chars) and active agent execution patterns → `<!-- agent-summary -->` in KNOWLEDGE.md (300 chars). Configurable backend selection and 2h interval

### Changed

- **Agent context builder optimized** — Merged duplicate Execution History + Short-term Memory sections into single "Recent Execution History". Filtered low-value learned patterns. Baselines now show human-readable device names from resources instead of raw metric IDs
- **Agent reflection prompt language** — All LLM reflection prompts use English for consistency
- **Focused mode LLM fallback** — Deterministic fill from `situation_analysis` when small models omit `reasoning_steps`/`conclusion`/`decisions` fields. Uses `serde_json::Value` for `insight` to tolerate non-string LLM output (true, 0, null). No extra LLM calls, no circular risk
- **Memory system refactor** — Replaced old LLM-based chat extraction (`POST /api/memory/extract`) with marker-based periodic summarization. Removed dead extraction pipeline (compat stubs, category files). Memory writes are now: (1) user via memory tool, (2) background periodic summaries. Old `user_profile.md`/`task_patterns.md`/`domain_knowledge.md` files (417KB of noise) replaced by clean USER.md/KNOWLEDGE.md
- **Memory config defaults** — `agent_char_limit`: 500→1000, `summary_interval_secs`: 3600→7200, `system_context_interval_secs`: 300→600. Added `summary_backend_id` field for selecting LLM backend for summarization (defaults to active backend)
- **Agent short-term memory** — Capacity increased from 10→20 entries. `summarize_agent_context()` now includes both situation and conclusion for richer context. Learned patterns get time-based confidence decay (10%/week, removed after 28 days). Baselines pruned when data sources no longer present
- **Memory config dialog** — Replaced manual toggle switch with Radix UI Switch component. Added LLM backend selector for summarization. Removed Extract button from toolbar
- **Tool prompt architecture** — `builder.rs` now includes structured tool descriptions (Type 1: shell, Type 2: skill, Type 3: file/web) with parameter docs, security notes, and usage examples in the system prompt. `TOOL_STRATEGY` section guides LLM on when to use each tool type
- **Memory tool actions expanded** — Added `read_file`, `write_file`, `list_files` actions for direct file manipulation alongside existing category-based actions
- **Memory panel unified** — Custom memory files merged into the same table as user/knowledge files. Single unified dialog for view/edit. "Add File" button in tab actions bar. Eliminated ~200 lines of duplicate state and dialogs
- **Memory stats API unified** — `GET /api/memory/stats` now returns `{ files, custom_files }` using the new `store.stats()` API instead of deprecated `all_stats()`. Fixed stats display (was always showing 0 chars due to key mismatch)
- **Code formatting cleanup** — `cargo fmt` applied across agent, storage, API crates for consistent formatting
- **Table vertical alignment** — ResponsiveTable cells now use flex centering for consistent vertical alignment across rows with varying content heights
- **Global chat floating window** — Replaced full-screen backdrop overlay with a fixed-size floating window (380×560 desktop, 70vh mobile) anchored to bottom-right. Users can now chat while viewing Dashboard/device pages behind the window
- **Memory scheduler cleanup** — Removed system resource summary job that wrote stale "System Resources" sections to KNOWLEDGE.md every schedule interval, wasting the char budget on transient data queryable live via CLI tools

### Removed

- **`ai_metric` tool** — Removed the AI Metric tool and all related infrastructure. This tool allowed LLM agents to write custom time-series metrics (`ai:{group}:{field}`), but had no reliable use case — the Memory system already covers cross-session knowledge persistence. Full cleanup across backend, frontend, i18n, and docs:
  - **Rust**: Deleted `crates/neomind-agent/src/toolkit/ai_metric.rs` (614 lines). Removed `AiMetricsRegistry` from `AgentState`, `init_tools()`, `refresh_extension_tools()`. Removed `DataSourceType::Ai` enum variant and `DataSourceId::ai()` from `neomind-core`. Removed `collect_ai_sources()` from data handler. Removed `"ai:"` from `KNOWN_PREFIXES` in telemetry migration
  - **Frontend**: Removed `'ai-metric'` from `DataSourceType` union, `AIMetricDataSource` interface, `aiGroup` field. Cleaned all 6 config schema files, `UnifiedDataSourceConfig`, `DataSourceIndicator`, `DualModeSourceField`, `ComponentConfigBuilder`, `componentDataApi`
  - **i18n**: Removed `aiMetric`, `aiMetricDesc`, `noAiMetrics`, `aiGroupPlaceholder` from en/zh locales
  - **Docs**: Removed ai_metric references from agent (en/zh), tools (en/zh), storage (en/zh), and web dashboard (en/zh) documentation
- **`session_search` tool** — Removed conversation history search tool. LLM already has full conversation context in its prompt window, making self-search redundant. Memory system handles cross-session knowledge persistence. Deleted `crates/neomind-agent/src/toolkit/session_search.rs` (127 lines)
- **`think` tool** — Removed the explicit thinking tool (338 lines). Thinking models now handle reasoning internally via streaming. The `think` namespace removed from LLM tool routing and staged agent filter
- **`ToolFilter` dead code** — Removed unused `ToolFilter` struct, `filter_by_intent()`, `intent_prompt()` from `staged.rs` (~130 lines). Removed dead `classify_intent()`, `get_intent_prompt()`, `filter_tools_by_intent()` methods and `tool_filter` field from `LlmInterface` in `llm.rs` (~140 lines including tests). Removed unused `IntentCategory::namespace()` and `IntentClassifier::classify_category()`
- **5 unused agent components** — Deleted `AgentMemoryDialog`, `AgentExecutionsList`, `AgentListPanel`, `AgentLogicPreview`, `AgentsList` (0 references, ~1626 lines of dead code)
- **Chat memory toggle** — Memory is now always enabled (configurable via settings). The per-session toggle was redundant since the memory tool provides on-demand access regardless of snapshot preload
- **Chat skill selector** — LLM already auto-selects skills via the `skill` tool based on user intent. Manual preloading was redundant and added UI clutter
- **Memory extract endpoint** — Removed `POST /api/memory/extract` and frontend Extract button. Old LLM-based chat extraction produced 417KB of noisy data (3551 entries, mostly duplicates). Replaced by background periodic summarization
- **Dead memory modules** — Removed `compat.rs` (empty stubs), `lifecycle.rs` (unused hooks), `short_term.rs`, `mid_term.rs`, `long_term.rs`, `tiered.rs`, `bm25.rs`, `embeddings.rs` (all unused after refactor)
- **Unused `write_last_resource_summary_time`** — Removed dead method from `MarkdownMemoryStore`

### Fixed

- **Custom Layer background image UI redesign** — Merged awkward two-field layout (URL + separate file upload) into a single inline field with URL input + Upload button, matching ImageSourceField pattern
- **LayerEditorDialog save button i18n** — Added missing `common.save` translation key so save button shows localized text instead of raw key
- **Missing zh translations for spatial config** — Added `backgroundType`, `backgroundImageUrl`, `layerItemBinding`, `manageLayerItems` and related keys to Chinese locale
- **Memory tool write lock** — Write operations (add/replace/remove/create) now use `store.write().await` instead of `store.read().await` to prevent read-modify-write race conditions
- **Memory tool first-match-only** — `replace`/`remove` actions now use `.replacen(..., 1)` instead of `.replace()` to prevent multi-replace data corruption
- **Memory tool chars vs bytes** — All "X chars" messages now use `.chars().count()` instead of `.len()` for correct UTF-8/Chinese text reporting
- **Memory tool list action** — `target` parameter is now optional for `list` action (was incorrectly required)
- **Memory snapshot budget** — Added hard truncation fallback when user content alone exceeds 5000 char budget
- **Refresh extension tools** — Memory tool is now re-registered during `refresh_extension_tools()` to prevent it from disappearing after extension refresh
- **All compiler warnings resolved** — Zero warnings across neomind-storage, neomind-agent, neomind-api crates
- **Session file path traversal** — Added `validate_session_id()` to block `../` and `/` in session IDs, preventing arbitrary file access
- **Char counting consistency** — Fixed `write_file()`, `stats()`, and agent stats to use `.chars().count()` instead of `.len()` for correct UTF-8/Chinese text handling
- **Extraction lock resilience** — Extraction guard now uses `Drop` pattern to ensure lock is released even on panic, preventing permanent lock-out
- **Missing i18n keys** — Added `systemMemory.extract` and `systemMemory.custom.description` to en/zh locales
- **Session sidebar card overflow** — Fixed Radix ScrollArea Viewport injecting `display:table` + `min-width:100%` causing cards to overflow. Added CSS override to Viewport component and proper `min-w-0` flex constraints for text truncation
- **Session action buttons** — Edit/delete buttons now compact (`h-4 w-4`) and absolutely positioned floating on card right side with hover reveal, instead of inline layout
- **Dashboard stuck skeleton screens** — Fixed three root causes: loading counter leak on telemetry-only sources, retry storm (reduced to 1 retry at 500ms), and added 3s hard deadline force-clear
- **Dashboard cross-tab sync** — Emit `DashboardUpdated` event on CRUD operations. VisualDashboard subscribes for real-time sync across browser tabs
- **Dashboard chart tooltip crash** — Fixed crash when rendering telemetry point objects `{timestamp, time, value}` as React children. LineChart now correctly extracts numeric values
- **Community widget data flow** — Fixed `fetchData` prop not reaching community widgets due to missing `installedComponents.length` dependency in rendering useMemo. Removed 2.5s fetch delay for immediate registry sync
- **Data source editor binding** — Fixed `dataSourceToSelectedItems` not recognizing `type:"telemetry"` and `type:"device"` with metric fields, causing editor to not show bound state for AI-created data sources

---

## [v0.8.2] - 2026-05-29

### Changed

- **DataSource unified Source+Mode architecture** — Replaced 12 legacy `type`-based routing with 4 unified fields (`source`/`id`/`field`/`mode`). New `DataSourceSource` (device/extension/system/transform/ai) and `DataSourceMode` (latest/timeseries/command/info/list) types provide clean orthogonal dimensions. `migrateToUnified()` bidirectionally populates both old and new fields for zero-migration backward compatibility. Removed 14 type guard functions, legacy switch statements across 6 sub-hooks. All routing now uses mode-based logic with fallback to legacy fields
- **usePollingSource replaces useSystemSource** — New generic HTTP polling hook supporting latest, list, and timeseries accumulation modes. System metrics now support client-side historical accumulation (pruned by `timeRange`/`limit`). Deleted `useSystemSource.ts` entirely. `pollDataSource()` dispatch in fetch.ts provides extensible source routing for future data sources (rule lists, message lists, external APIs)
- **Config UI outputs unified fields** — `selectedItemsToDataSource` now outputs `source`/`id`/`field`/`mode` alongside legacy `type`. `suggestedMode` prop enables per-component mode hints (LED→latest, Chart→timeseries, Toggle→command, Map→info). Eliminates sourceTransform round-trips for new configurations
- **isImageDataSource refactored** — Changed from 3-arg `(params, transform, metricId)` to single-arg `(ds)` pattern. Updated 8 call sites across 4 files
- **Community/extension component fetchData API** — New `resolveDataSourceData()` utility and `fetchData` prop injection in ComponentRenderer for community/extension components. Provides mode-aware data fetching without React hook dependency

### Fixed

- **Instant telemetry initial rendering** — Telemetry-bound components (LED, ValueCard, ProgressBar, etc.) now read initial values from `store.current_values` instead of waiting for HTTP API. New `readTelemetryInitialValues` in `useStoreSource` creates synthetic data points from store, eliminating loading flash on dashboard open
- **Enhanced telemetry retry** — `useTelemetrySource` now retries with exponential backoff on transient failures instead of showing permanent error state
- **Dashboard component count mismatch** — Removed destructive `isDataSourceValid` filter in `fetchDashboards` that silently deleted components with incomplete data sources
- **Camera hardware lock leak** — `VideoDisplay` CameraAccess now properly stops MediaStream tracks on unmount via `streamRef` + cleanup
- **Dual/triple fullscreen rendering** — VideoDisplay, MapDisplay, CustomLayer no longer render content inline AND via portal simultaneously (`{!isFullscreen && content}` pattern)
- **useTelemetrySource timer leaks** — Retry setTimeout and fetch timeout promise now tracked via refs and cleaned up on unmount
- **LayerEditorDialog cancel data loss** — Cancel button now calls `onOpenChange(false)` instead of `onSave(undefined)` which wiped all layer bindings
- **Config save dataSource priority** — Simplified `handleSaveConfig` to 2 authoritative locations instead of 5, preventing restoration of intentionally-cleared data sources
- **Duplicate dashboard creation** — `HybridDashboardStorage.syncToApi` now only syncs dashboards with existing server ID mapping
- **Stack overflow on large telemetry arrays** — Replaced `Math.min(...array)` / `Math.max(...array)` with `.reduce()` pattern across 10 files to handle arrays >100K elements
- **createStableKey stack overflow** — Added depth limit (MAX_DEPTH=10) to prevent infinite recursion on deep/circular references
- **Sparkline crash on sparse data** — Added guard for `< 2` data points before rendering
- **getLinearGradient OKLCH handling** — Now uses proper `colorWithAlpha()` helper instead of raw string concatenation
- **normalizeDataSource empty array** — `[]` input no longer wrapped as `[[]]`
- **imageUtils cache memory bloat** — Inputs >10KB (base64 camera frames) skip caching to avoid multi-MB string retention
- **SharedDashboard i18n** — Replaced 6 hardcoded English error messages with `t()` calls
- **Video display config i18n** — Replaced hardcoded Chinese strings with `t()` calls
- **Chart useMemo stale data** — LineChart, BarChart, PieChart now include `sources`, `getSeriesName`, `getDeviceName` in dependency arrays
- **Renderers missing builtIn types** — Added `counter` and `metric-card` to builtInTypes Set and builtInComponentMap
- **DashboardGrid redundant data-grid** — Removed `data-grid` attribute from child elements (layouts prop is authoritative)
- **ImageDisplay fullscreen portal** — Fullscreen overlay now uses `getPortalRoot()` instead of inline rendering
- **Dashboard switch state cleanup** — `mobileSelectedId` and `mobileEditBarOpen` reset on dashboard switch
- **Deep clone on template apply** — `applyTemplate` now uses `JSON.parse(JSON.stringify())` for proper deep clone
- **configComponentId reset on delete** — `deleteDashboard` now clears `configComponentId` and `configPanelOpen`

### Fixed (Round 10)

- **Error Boundary for dashboard components** — Extension/community component runtime errors no longer crash the entire dashboard page; graceful error card with localized message
- **localStorage quota recovery** — `LocalStorageDashboardStorage.save()` now catches `QuotaExceededError`, clears stale data, and retries write
- **Hybrid storage sync race condition** — Rapid edits to a local dashboard before first server sync now preserve latest changes instead of overwriting with stale server state
- **Position validation** — `moveComponent` now clamps negative x/y to 0 and dimensions to minimum 1; `positionFromDTO` applies same validation to API responses
- **Registry validation** — Dynamic and community component registries reject types that shadow built-in widget types (e.g. registering `"line-chart"` as extension)
- **Missing type guards** — Added `isExtensionMetricSource()` and `isExtensionCommandSource()` type guards for discriminated union coverage

### Fixed (Round 11)

- **Mobile edit mode state leak** — Exiting edit mode on mobile now resets `mobileSelectedId` and `mobileEditBarOpen` instead of leaving stale mobile UI
- **Mobile drag/resize disabled** — Grid drag and resize disabled on touch devices to prevent conflicts with scrolling and touch interactions
- **Extension uninstall cleans all dashboards** — Unregistering an extension now removes its components from ALL dashboards, not just the current one
- **ComponentRenderer unmounted state updates** — Added mountedRef guard to prevent React warnings from async state updates after component unmount
- **Mobile touch targets** — Action buttons in mobile edit mode increased to 44px height (was 32px) for proper touch accessibility
- **Mobile selection overlay** — Split overlay into separate selected/unselected states; component content is now interactive when selected

### Changed

- **Dashboard configSchemas registry pattern** — Replaced 2982-line monolithic `configSchemas.tsx` switch statement with a modular registry pattern. Schema generators are now organized into `builtIn/` sub-modules (indicators, charts, controls, display, spatial, business) plus a `dynamic.tsx` handler for extension/community/custom components. No user-visible behavior changes
- **Dashboard store: eliminated slice circular dependencies** — Removed module-level `_scheduleSync`/`_flushSync` variable exports from `dashboardCrudSlice`. `scheduleSync()` and `flushSync()` are now proper slice methods accessed via `get()`, eliminating fragile module-level getter pattern
- **DataSource discriminated union types** — Added 12 type-specific interfaces (`DeviceDataSource`, `CommandDataSource`, `SystemDataSource`, etc.) with type guards (`isDeviceSource()`, `isRealtimeSource()`, `isPolledSource()`, etc.). Legacy flat `DataSource` interface preserved for backward compatibility. Updated `useDataSource` pipeline and `dashboardHelpers` to use type guards
- **useDataSource simplified state management** — Replaced 12-action `useReducer` state machine with flat `useState` + loading ref counter. Removed `activeFetchSource` tracking, `FETCH_EMPTY_RETRY`, and `FORCE_CLEAR_LOADING` actions. Loading state is now a simple counter (loading = counter > 0) managed by `startLoading`/`finishLoading` callbacks

---

## [v0.8.1] - 2026-05-27

### Added

- **Embedded MQTT broker auth & TLS management** — Redesigned `EmbeddedBroker` with `external_auth` callback for redb-backed credential validation, stop/restart lifecycle, and TLS support (cert/key paths). Broker now loads config from redb at startup and validates connections against stored credentials
- **MQTT credential storage** — New redb tables (`mqtt_credentials`, `mqtt_credentials_by_username`) for MQTT username/password management. Full CRUD methods with automatic index maintenance in `neomind-storage`
- **Embedded broker config API** — New endpoints `GET/PUT /api/settings/broker` for reading and updating embedded broker configuration (auth mode, TLS, credentials). Changes take effect on broker restart
- **Embedded broker config UI** — New `EmbeddedBrokerConfigDialog` component with auth mode toggle (anonymous/credential), credential management (add/delete), and TLS configuration (cert/key paths). Full en/zh i18n support
- **CLI: device drafts commands** — New `neomind device drafts` subcommand group (`list`, `get`, `approve`, `reject`, `config`) for managing auto-discovered device drafts. Full workflow: list pending → inspect samples → approve with name/type → or reject
- **CLI: device webhook-url** — New `neomind device webhook-url <ID>` command to retrieve the HTTP push URL for webhook adapter devices
- **CLI: extension config** — New `neomind extension config <ID>` to view config, `--set '<JSON>'` to update. Replaces manual API calls for extension configuration
- **CLI: API client auth retry** — All API client methods (GET/POST/PUT/DELETE/multipart) now automatically retry on 401 with refreshed API key from redb. API key stored in `RwLock` for thread-safe refresh
- **CLI: health check via API** — `neomind health` now queries actual LLM backend status via API instead of checking environment variables. Shows backend count, active backend ID, and setup hints
- **CLI: system info with TLS/auth/credentials** — `neomind system info` now exposes MQTT broker TLS status, auth mode, and credentials for AI agent onboarding guidance
- **Broker connection guide in Add Device dialog** — New step showing embedded broker connection details (host, port, credentials) to simplify device onboarding

### Changed

- **CLI: shell tool reference updates** — `transform test` renamed to `test-code`, `extension get` aliased to `info`, agents created as `active` by default (no longer need `control <ID> active`), push target type auto-detected from config
- **CLI: shell operator fallthrough** — Commands containing pipes (`|`), redirects (`>`), or stderr redirects (`2>`) now fall through to real shell execution instead of internal routing
- **CLI: DSL parser validation** — Rule engine now rejects function-call syntax (e.g., `device.metric(temperature)`) and empty source/metric with clear error messages
- **Session preview auto-extraction** — Session list now includes preview text auto-extracted from the first user message (50-char limit), improving session sidebar display
- **User guide improvements** — Updated documentation with Skills tab references, Data page guidance, and content fixes
- **Embedded broker migrated to rmqtt** — Replaced rumqttd with rmqtt for improved stability, plugin support, and standards compliance. Broker restart uses system credentials from redb

### Fixed

- **Storage lifetime issue** — Fixed lifetime annotation in `delete_mqtt_credential` preventing compilation
- **macOS resource limits** — Fixed macOS file descriptor limits for stable operation under high connection counts
- **MQTT InvalidAuth loop** — Resolved broker authentication loop caused by credential mismatch; parallelized broker startup for faster initialization
- **MQTT broker restart credentials** — Broker restart adapter now correctly uses system credentials from redb instead of stale values
- **Backend base64 image stripping reverted** — Reverted commit 49c1086 which stripped `data:image/...;base64,` prefix from metric/telemetry API responses, breaking all image consumers (dashboard widgets and external extensions). Backend now returns string values as-is
- **Base64 image detection** — Fixed `/9j/` (JPEG) rejection in `isPureBase64`/`isBase64Image` across ImageDisplay, ImageHistory, AgentMonitorWidget, and helpers. All components now correctly detect JPEG base64 data
- **Image URL normalization** — Fixed double-prefixed data URL handling and non-standard `data:` prefix cases using magic bytes detection in normalizeImageUrl
- **Image dynamic refresh** — Device→telemetry conversion in ImageDisplay and ImageHistory now includes `refresh` interval for live image updates
- **External placeholder SSL error** — Replaced external `via.placeholder.com` with local empty state, eliminating SSL errors for missing images
- **React setState-during-render warning** — Fixed `UnifiedDataSourceConfig` calling `onChange()` inside `setSelectedItems` updater; moved to useEffect
- **Floating chat session isolation** — PanelChatView and GlobalChatFab now share session key constant; added new conversation button; fixed session history loading on mount
- **Floating chat panel redesign** — Complete overhaul of the global floating chat panel: independent session with local state (no longer shares global store with chat page), proper LLM backend loading with "not configured" empty state, skeleton loading when reopening panel, session not found auto-recovery (silently creates new session)
- **AI response tool call rendering fixed** — `ToolCallVisualization` was deprecated (returned `null`), causing tool calls and execution process to be invisible in `MergedMessageList` and `MessageItem`. Replaced with `ToolProcessBlock` to match the main chat page's rendering
- **Floating panel card-style AI responses** — Added `assistantCard` prop to `MessageItem`/`MergedMessageList` for wrapping AI responses (thinking + tool calls + content) in a subtle card background, improving readability over the glass morphism panel background
- **Streaming cursor positioning** — Fixed floating cursor in streaming content caused by `relative inline` CSS on the wrapper; now uses proper `align-text-bottom` alignment
- **Streaming-to-saved message flash fix** — Panel's `"end"` handler now uses `currentStreamMessageId` as the saved message ID, enabling smooth transition from streaming block to persisted message without visual flash
- **Session cleanup on delete** — `deleteSession` in sessionSlice now clears the panel's persisted session ID from localStorage when the deleted session matches, preventing "Session not found" errors on next panel open
- **Missing i18n translations** — Added translations for "Edit Dashboard", "Internal Broker", "Built-in" labels in en/zh locales

---

## [v0.8.0] - 2026-05-26

### Added

- **Messaging system delivery retry** — Failed message deliveries are now automatically retried up to 3 times with a 2-minute interval scheduler. The existing `DeliveryLog` infrastructure (`can_retry`/`increment_retry`/`max_retries`) is now fully wired to a background retry loop in `AppState`
- **Webhook timeout configuration** — Webhook channels now support configurable request timeout (`timeout_secs`, default 30s) with a 10s connect timeout. Field exposed in the channel creation dialog in the UI, with en/zh i18n labels
- **Message deduplication** — Messages with the same title+source+severity within a 60-second window are automatically deduplicated. The message is still stored but channel delivery is skipped, preventing message bombing from high-frequency rule triggers
- **Automatic delivery log cleanup** — A background task now runs every 6 hours to clean up delivery logs older than 1 day and messages older than 30 days. Runs on startup and periodically via `tokio::select!` alongside the retry scheduler
- **Automatic updater fixes** — Fixed app restart and version placeholder replacement after in-app updates. Fixed service config, sudo handling, and upgrade support for the install/update flow
- **Global AI chat entry (FAB)** — Floating action button on all non-chat pages opens a full-screen glass-morphism chat overlay with smooth scale-up animation. Panel uses an independent session persisted via localStorage, shares WebSocket with the main `/chat` page. Brand orange styling, Bot icon for AI messages, i18n empty state
- **5 new notification channels** — Telegram (Bot API), WeCom (robot webhook), DingTalk (custom robot with HMAC-SHA256 sign), Slack (Incoming Webhook), Feishu (custom bot with HMAC-SHA256 sign). Each channel is feature-gated in `Cargo.toml` and registered via `ChannelFactory`. All use platform-native message formats (markdown, Block Kit, HTML)
- **Channel editor FullScreenDialog** — Replaced inline `UnifiedFormDialog` with dedicated `ChannelEditorDialog` component using `FullScreenDialog` + Sidebar layout. Left sidebar shows all 7 channel types with icons and descriptions; main area shows dynamic config form. Mobile-friendly with horizontal tab bar
- **Data push module** — New `neomind-data-push` crate for pushing device telemetry and extension output to external systems. Supports Webhook and MQTT targets with event-driven and interval-based scheduling, configurable retry with exponential backoff, data filtering, and Jinja-like template rendering. Full REST API and frontend management UI with `PushTargetDialog` and `DeliveryHistoryPanel`
- **Channel type registry** — Backend now exposes channel type schemas via `GET /api/messages/channels/types/:type/schema` with per-type JSON Schema for config validation. Frontend auto-discovers available types

### Changed

- **Email SMTP connection reuse** — `EmailChannel` now builds and caches the `SmtpTransport` at creation time via `Arc<Mutex>`, eliminating per-send SMTP connection setup overhead
- **Email recipients atomicity** — `add_recipient`/`remove_recipient` now recreate the email channel before persisting to storage, with automatic rollback on failure. Previously a failed recreation could leave `state.recipients` and `EmailChannel.to_addresses` out of sync
- **Chat message styling** — AI messages use Bot icon instead of logo image. User message bubbles use neutral black/white. User avatar uses brand orange accent. Streaming text internationalized
- **Messages page refactored** — Extracted ~500 lines of channel create/edit logic from `messages.tsx` into standalone `ChannelEditorDialog` component. Main page reduced by 40%
- **Delivery log removed** — Removed monolithic `delivery_log.rs` (591 lines). Delivery tracking now handled by channel-level retry in `ChannelFilter` with simpler dedup logic

### Fixed

- **Email TLS configuration dead code** — The `use_tls` field in `EmailChannel` was stored but never read in `send()`, which always used `Tls::Required`. Now correctly uses `builder_dangerous` when `use_tls` is false, enabling support for local mail servers (MailHog, etc.)
- **CLI robustness** — Fixed widget install multipart mismatch, added border styling to widget scaffolds, aligned CLI docs/skills/prompts with actual system behavior
- **CI build** — Fixed Tauri externalBin by building `neomind-cli` alongside `neomind-extension-runner`
- **Device auto-discovery** — Fixed `adapter_type` when registering auto-discovered devices
- **Channel config field alignment** — Email config now sends `smtp_server`/`username`/`password` (was `smtp_host`/`smtp_username`/`smtp_password`). Webhook timeout field now sends `timeout_secs` (was `timeout`). All fields match backend factory expectations
- **Channel edit form initialization** — Edit mode now correctly populates form via `useEffect` watching `open`/`editingChannel` instead of relying on `onOpenChange` callback which only fires on user actions
- **DingTalk dead code** — Removed unused `webhook_url` method that caused Rust compiler warning

---

## [v0.7.9] - 2026-05-25

### Added

- **Widget development skill** — New builtin skill `widget-development.md` with complete IIFE templates (ValueCard, Clock, Gauge, DevicePanel), jsxRuntime pattern documentation, props interface guide, manifest.json reference, and Tailwind styling rules. Based on patterns from real NeoMind-Dashboard-Components repository
- **Extension development skill** — Rewritten `extension-development.md` with complete working DataProcessor template, state management patterns (AtomicU64, RwLock, Mutex), Builder API reference, Cargo.toml requirements, and `ureq` sync HTTP guidance. Based on patterns from real NeoMind-Extensions repository
- **Transform metric discovery guidance** — Enhanced `transform-management.md` with "Discover Metrics Before Writing Code" section, auto-unwrap semantics documentation, `extensions.invoke()` usage, and three discovery paths (device metrics, extension metrics, existing transforms)
- **Extension reload command** — New `neomind extension reload <ID>` command in CLI, cli-ops, and shell.rs routing. Calls `POST /api/extensions/:id/reload` for hot-restarting extension processes
- **Agent create advanced flags** — Help text now documents all flags: `--resources`, `--metrics`, `--commands`, `--event-filter`, `--timezone`, `--enable-tool-chaining`, `--max-chain-depth`, `--priority`, `--context-window-size`
- **Shell help for extension/widget/transform** — Added detailed help entries for `extension create/build`, `widget create`, and `transform create` with workflow steps, parameter tables, and examples

### Changed

- **Dashboard add-components** — Shell help and tool description now prominently recommend `add-components` over `update --components` to prevent accidental full replacement of dashboard components
- **Rule DSL quotes** — Fixed tool description to use `RULE "<name>"` (quoted) matching the actual DSL parser requirement
- **Rule engine improvements** — Enhanced DSL parsing, validation, and generator for more robust rule creation
- **CLI error recovery** — Transform test command now flattens API error responses for clearer error messages

### Fixed

- **Webhook adapter auto-discovery** — Webhook adapter now emits `DeviceDiscovered` on every POST for unregistered devices (previously only on first POST), enabling proper sample collection for auto-onboarding
- **Webhook auto-onboarding single-trigger** — `create_draft_with_topic()` now triggers analysis immediately when `MIN_SAMPLES_FOR_ANALYSIS` samples are collected (was 1 but analysis only triggered in `add_sample_to_draft`). One webhook POST now creates draft + triggers analysis
- **Webhook URL format** — Fixed all frontend webhook URL generation from `/api/devices/webhook/{id}` to correct route `/api/devices/{id}/webhook` across 6 components
- **Webhook handler refactor** — Rewrote webhook handler from 650+ lines to ~200 lines, delegating to `WebhookAdapter.process_webhook()` instead of duplicating token verification, metric extraction, and event publishing
- **Webhook shared device registry** — Webhook adapter now receives the shared `DeviceRegistry` via `set_shared_device_registry()`, fixing token verification and device type lookup
- **Webhook token display** — Fixed `config_to_device_instance()` in compat.rs to include `connection_config.extra` fields (webhook_token, json_path, etc.) so tokens display correctly in Device Connections
- **Pending Devices WebSocket auto-update** — Fixed event handler to use correct field names (`custom_type`, `data.event_type`, snake_case values). Added `Custom` event arm in `extract_event_data()` to avoid double-wrapped serialization
- **Webhook routes** — Added 3 webhook routes to router.rs: `POST /api/devices/:id/webhook`, `POST /api/devices/webhook`, `GET /api/devices/:id/webhook-url`
- **Webhook token input** — Added webhook token generation and input to both AddDeviceDialog and ManualAddForm (AddDeviceGlobalDialog)
- **Webhook URL with real IP** — Device Connections webhook URLs now show server's real IP instead of localhost
- **Device Information webhook display** — DeviceDetail page now shows webhook URL and token for webhook adapter devices
- **Extension status/logs 500→404** — Fixed API returning 500 "IPC error" for non-existent extensions. Added existence check before IPC calls, returning proper 404
- **Boolean flag parsing** — Fixed `--tls` flag silently failing when it's the last argument. Changed from `get_flag_value()` to `args.iter().any()` for boolean flags
- **Severity level mismatch** — Fixed message send recovery hint from "error" to "emergency" to match actual API accepted values (info|warning|critical|emergency)
- **Transform auto-unwrap** — Single-key JSON input like `{"value": 42}` is now auto-unwrapped to scalar `42` for simpler transform code. Multi-key objects remain as-is
- **Extension reload routing** — `neomind extension reload` no longer falls through to `__FALLTHROUGH__` but properly calls the API endpoint
- **Marketplace dialog flickering (Windows)** — `ExtensionListContent` and `DetailContent` were defined as inline components inside `MarketplaceDialog`, causing React to unmount/remount the entire DOM subtree on every render. Replaced with stable inline JSX. Also removed duplicate `fetchExtensions()` call after install
- **EntityIconPicker flickering** — `IconPreview` was defined inside the component body, moved to module level to prevent React remounting
- **UnifiedDataSourceConfig flickering** — `ItemBadge` (2 instances) and `DataIndicator` defined inside component bodies caused unnecessary remounts. Extracted to module-level components with `t` passed via props

---

## [v0.7.9] - 2026-05-23

### Added

- **CLI command system (neomind-cli-ops)** — New shared library crate with typed API client, unified output formatting, and full CLI commands for all 8 domains: device, dashboard, rule, extension, widget, transform, agent, message. Each domain supports list/get/create/update/delete plus domain-specific actions (device control, rule testing, agent invocation, extension marketplace, etc.)
- **AI Build Mode foundation** — `neomind-cli` packaged as Tauri external binary, enabling the agent to execute CLI commands via shell tool. Full CLI command reference injected into agent system prompt for discoverability
- **System CLI** — New `system info` command aggregating MQTT broker status, network info, and webhook URL. Broker management and help modules added
- **Telemetry stats API** — New endpoint for telemetry statistics with improved telemetry handling in backend
- **Dashboard rewrite (Phase 1–4)** — Complete frontend dashboard architecture overhaul:
  - Phase 1: New type system, API client, and Zustand store slices (CRUD + data source)
  - Phase 2: Query hooks, data source abstractions, real-time event bridge
  - Phase 3: Grid layout, widget shell, config panel, component registries
  - Phase 4: Widget adapters for all chart types, feature module barrel export

### Changed

- **useDataSource pipeline rewrite** — Refactored from 16 files to 4 focused sub-hooks (useTelemetrySource, useExtensionSource, useStoreSource, useSystemSource). Fixed extension event dynamic updates and data flow bugs
- **Agent CLI integration** — Unified flag names between shell.rs and CLI for consistency. Improved CLI completeness and token efficiency in agent prompts

### Fixed

- **Dashboard scroll white screen** — Multiple fixes: debounced Recharts ResponsiveContainer, staggered chart rendering with memoization, skipped unchanged device updates, removed overflow-anchor suppression
- **Dashboard multi-widget performance** — Fixed lag, blank widgets, and unresponsive mouse in dashboards with many components
- **Dashboard config preview** — Fixed live preview not reflecting config changes, removed forced grid aspect ratio causing component distortion, preserved component aspect ratio
- **Dashboard data source config** — Improved data source selector and configuration UI
- **Extension crash diagnostics** — Improved error reporting and fixed Windows DLL search path
- **CLI compatibility** — Fixed short option conflicts in device commands, added `--json` flag, fixed output printing, added API key auth support
- **Extension runner** — Bumped to 0.7.5 with improved crash protection

## [v0.7.8] - 2026-05-16

### Changed

- **Extension marketplace dialogs** — Converted extension detail and install dialogs to `FullScreenDialog` for better layout on all screen sizes
- **Transform Builder toolbar** — Redesigned Code step toolbar, removed step titles for cleaner UI
- **Data Explorer detail view** — Optimized list layouts and detail panel styling
- **Telemetry storage identifiers** — Unified all storage source IDs with `device:` prefix for consistency

### Fixed

- **Dashboard telemetry data sorting** — Fixed time-series data returning oldest points instead of newest when storage limit push-down was used. Added `query_range_rev()` for efficient descending-order queries. Applied stable sort across all telemetry transform paths to prevent JavaScript's unstable `Array.sort` from shuffling equal-timestamp points
- **Image history cross-metric interference** — Tightened `eventMetricMatches()` to prevent `foo.image` matching `bar.image` via last-segment comparison. Image data sources in the store change path now use content-only deduplication (same image content at any timestamp is treated as duplicate) instead of timestamp+value pair matching
- **Image history stale data injection** — Added time range validation to WebSocket and SSE event merge paths — events with timestamps outside the component's configured time range are now rejected. Fixed `findMetricValue` step 4 to require structurally similar key names instead of matching any image-like value
- **Store merge data misalignment** — `fetchTelemetryData` now only merges store values when API returns empty, preventing stale `current_values` from being stamped with `now` and displacing real latest data
- **Timestamp consistency** — All telemetry paths now use `Math.floor(Date.now() / 1000)` (integer seconds) instead of `Date.now() / 1000` (float). Fixed `extractTimestamp` in `ImageHistory` to correctly normalize seconds↔milliseconds
- **Extension marketplace install timeout** — Increased HTTP request timeout from 30s to 120s and extension startup timeout from 30s to 120s to allow heavy extensions (e.g. stream-player with 70+ FFmpeg dylibs) to complete installation
- **Update dialog reappearing after restart** — Prevented version update dialog from showing again after the app has been restarted following an update
- **AI chat message flicker** — Eliminated brief content flash when AI streaming completes and the final message replaces the streaming state
- **CI build warnings** — Resolved event capability test timeout and remaining build warnings

## [v0.7.7] - 2026-05-15

### Added

- **Data retention configuration** — New `GET/PUT /api/settings/retention` and `POST /api/settings/retention/cleanup` endpoints for automatic telemetry cleanup. Configurable retention period (never–90 days), image data retention, cleanup interval, and manual trigger
- **Preferences UI — Data Management** — New data management section in Settings > Preferences with auto-cleanup toggle, retention period selector, image data retention selector, and manual cleanup button
- **Extension FFI timeout protection** — Added `safe_ffi_call_with_timeout` with 30-second limit for all extension FFI calls, preventing hung extensions from blocking the runner
- **Extension event queue backpressure** — Event queue now capped at 1000 entries; oldest events dropped with warning log when queue is full

### Changed

- **Server startup parallelization** — Split initialization into Phase A (parallel store opening via `spawn_blocking`) and Phase B (background services). All redb stores (rule, agent, dashboard, instance, extension) open concurrently, reducing cold-start time
- **Concurrent extension loading** — Extension loading now uses bounded parallelism (`Semaphore(4)`) instead of sequential loading
- **Lazy GPU detection** — GPU info collected on first `/api/stats` request instead of at startup, eliminating startup delay on systems without GPU
- **Frontend cache eviction** — `useDataSource` now enforces max cache sizes with FIFO eviction for system stats, telemetry, and extension data caches
- **Extension stream lifecycle** — Added `destroy()` method for complete client cleanup; proper subscription handler cleanup on reconnect
- **Robust dashboard conversion** — `positionFromDTO` returns safe defaults for missing/malformed position data; better validation of component DTOs

### Fixed

- **Integration test redb lock conflict** — `ExtensionStore::open` now supports `:memory:` mode (isolated temp DB per call); `new_for_testing()` uses `:memory:` to eliminate parallel test file lock failures (87/87 tests passing)
- **Backend switching race condition** — `set_active` now holds a DashMap guard to prevent concurrent instance removal during active backend switch
- **Channel handler error handling** — Replaced `expect("Just created")` / `expect("Just updated")` with proper `ok_or_else` error responses in channel CRUD handlers
- **Dashboard scroll white screen** — `ChartContainer` replaced ResizeObserver + useState with pure CSS (`minHeight: 120`), eliminating the first-frame blank render. Grid items use `content-visibility: auto` with `contain-intrinsic-size: 300px` to prevent GPU texture exhaustion during fast scrolling
- **Chart component deduplication** — Extracted shared `toTelemetrySource`, `getDeviceName`, `getPropertyDisplayName`, `getSeriesName`, and `ChartTooltip` from LineChart/BarChart/PieChart into shared modules (~300 lines removed)
- **Cache implementation unified** — `useDataSource` telemetry cache migrated from raw Map + manual TTL/eviction to `TypedCache` with metadata support, unified with system stats and extension caches (~70 lines removed)

### Removed (Dead Code Cleanup)

- **Legacy `LlmBackend` trait** — Removed unused trait from `neomind-core` along with `LlmConfig`, `GenerationResult`, `StopReason`, `GenerationStream` types (0 implementations, fully replaced by `LlmRuntime`)
- **`TokenizerWrapper`** — Removed empty placeholder module (`llm_backends/tokenizer.rs`), never had a real implementation
- **`ContextRelevance::Low`** — Removed unused enum variant that was never constructed or matched
- **`StorageResult.source`** — Removed unused `'local' | 'api' | 'cache'` field from frontend persistence types (set 28 times, never read)
- **Dead functions/constants** — Removed 10 `#[allow(dead_code)]` items: `filter_simplified_tools`, `AsyncThinkStorage`, `AggressiveMockLlm`, `COMPOUND_SEPARATORS`, `MAX_TOOL_CALLS_PER_REQUEST_DEFAULT`, `DEFAULT_CONTEXT_TOKENS`, `extract_conversation_entities_topics`, `build_memory_injection_hint`, `detect_complex_intent_with_llm`, `is_complex_multi_step_intent_fallback`
- **Dead struct fields** — Removed `MessageManager.data_dir`, `MqttMapping.capabilities`, `HttpPollingTask.error_count`, `ExtensionStreamEvent::Heartbeat` variant
- **Unused example files** — Removed 5 dead examples from `crates/neomind-devices/examples/`
- **Incorrect `#[allow(dead_code)]` annotations** — Cleaned from `IsolatedExtensionLoader.native_loader` (actively used), `StreamEvent`, `CloudDeviceTypesIndex`

### Fixed

- **Integration test redb lock conflict** — `ExtensionStore::open` now supports `:memory:` mode (isolated temp DB per call); `new_for_testing()` uses `:memory:` to eliminate parallel test file lock failures (87/87 tests passing)
- **Clippy warnings** — Auto-fixed ~57 clippy issues: unnecessary `to_string`, redundant closures, `and_then→map`, `filter_map→map`, `map_or` simplification, `strip_prefix`, `is_multiple_of`, empty lines after doc comments

## [v0.7.6] - 2026-05-14

### Performance

- **WKWebView dashboard rendering** — Replaced `translate3d(0,0,0)` with `content-visibility: auto` + `isolation: isolate` + `contain: layout paint` to prevent GPU compositing layer exhaustion during loading/scrolling, eliminating white screen flash on Tauri macOS
- **Sparkline render optimization** — Extracted `SparklineContent` to top-level `memo`-wrapped component to prevent remount on each parent render; wrapped `Sparkline` export in `React.memo` to skip reconciliation when props unchanged
- **DashboardGrid render optimization** — Removed `devicesLength` from `gridComponents` useMemo dependency to prevent 3-second full rebuild on unrelated device changes
- **Limit push-down to storage** — Added `limit: Option<usize>` parameter through `query_telemetry` → `query_limited` → `query_range` chain, capping data allocation at the storage layer instead of filtering after full read
- **N+1 query elimination** — Replaced per-metric `latest()` loops with single-transaction `latest_batch` in `get_current_metrics`, reducing storage transactions linearly with metric count
- **Cold-start metrics warmup** — `list_metrics` now caches results in `metrics_info` DashMap after the first cold-start range scan, skipping full-table scans on subsequent calls
- **Debounced dashboard persistence** — `storage.sync` debounced to 500ms trailing window to coalesce rapid drag/resize events into a single API call
- **HTTP timeout layers** — Added `RequestBodyTimeoutLayer(20s)` nested inside `TimeoutLayer(30s)` to prevent slow-client DoS while preserving proper LIFO semantics
- **Code deduplication** — Extracted `createStableKey` utility from 3 duplicate implementations into shared `@/lib/stable-key.ts`

### Fixed

- **Timeout layer ordering** — Swapped `TimeoutLayer(30s)` and `RequestBodyTimeoutLayer(60s)` so the body timeout (20s) fires before the overall request timeout (30s), per Tower LIFO middleware semantics
- **Cold-start `list_metrics` returning empty** — Removed early-return guard that prevented the fallback range scan from running after server restart; added `metrics_initialized = true` after both `list_metrics` and `list_all_metrics_grouped` fallback scans
- **`moveComponent` stale closure** — Replaced separate `moveDebounceTimer` with shared `scheduleSync()` mutable-ref pattern to capture latest dashboard state during rapid drag operations
- **`handleIdChange` dashboard overwrite** — Added `activeDashboardId` guard to only update `currentDashboard`/`currentDashboardId` when the user hasn't switched away during sync
- **Sparkline const between import blocks** — Moved `SVG_OVERFLOW_VISIBLE` style constant to after all imports to satisfy linter
- **LoadingState animation** — Restored missing `animate-pulse` on loading skeleton placeholder
- **Removed unused `AlertCircle` import** from `DefaultStates.tsx`
- **Flaky test** — Added `flush()` method to `TimeSeriesStorage`/`ExtensionMetricsStorage` and call it in `test_extension_storage_write_query` to drain write buffer before asserting query results

### Chore

- **Gitignore** — Added `.worktrees/` for git worktree isolation

## [v0.7.5] - 2026-05-13

### Added

- **Unified execution engine: Focused / Focused+ / Free** — Focused mode agents can now opt into tool calling via the `enable_tool_chaining` toggle, creating a "Focused+" mode that combines pre-collected data with multi-round tool queries. The `run_tool_loop` engine is shared across Free (30 rounds, full autonomy) and Focused+ (configurable rounds, recommended tool guidance). Original Focused JSON path preserved as fallback when tool chaining is disabled
- **ToolLoopConfig** — New configuration struct driving the tool loop with mode-specific parameters: `max_rounds` (30 for Free, `max_chain_depth` for Focused+) and `recommended_tools` (prompt guidance extracted from bound resources for Focused+, unrestricted for Free)
- **Focused mode tool chaining toggle** — Agent editor shows an "Enable Tool Chaining" switch under Focused mode, persisted via `enable_tool_chaining` field. Hidden when Free mode is selected
- **Focused+ grouped resource prompt** — Focused+ system prompt groups bound resources by type (metrics with current values, commands) and provides a lightweight snapshot table instead of dumping raw pre-collected JSON. LLM is guided to use `device(action="history")` for historical queries, eliminating the need for manual `time_range` / `include_history` configuration
- **Data Collection config hidden for Focused+** — When tool chaining is enabled, the per-resource Data Collection config panel (time range, include history) is hidden since the LLM queries what it needs via tools
- **Adaptive time-series compression for device history** — `device(action="history")` now returns one of two formats, automatically picking the smallest: compact values array (`{"values": [...]}`) or adaptive series (`{"series": [{"range": "...", "kept": 12.0}, {"range": "...", "fluctuated": [12.5, ...]}]}`). Stable periods compress to single `"kept"` entries, significantly reducing token usage for the LLM
- **Mid-task context compaction** — When agent memory exceeds 70% of the context budget during long ReAct loops, old tool execution rounds are automatically summarized into a structured progress summary. Keeps recent rounds intact, preventing context overflow mid-task
- **Actual prompt overhead measurement** — Context window budget now measures real system prompt + tool definition tokens instead of using fixed percentage heuristics. Allocates `model_capacity - overhead - 1024` for history with a 20% safety floor
- **Agent summary API** — New `GET /api/agents?view=summary` endpoint returning lightweight `{id, name, status}` for dashboard dropdowns, replacing full agent payload
- **LargeDataCache eviction** — Cache now enforces max 20 entries and 50MB total. Oldest entries evicted automatically when limits are exceeded
- **Release build profile** — Added LTO thin, codegen-units=1, strip, opt-level=3 for smaller optimized binaries

### Changed

- **Time-series write buffering** — Single-point writes are now batched in an in-memory buffer (200 points, 500ms flush interval) and flushed to redb as batched transactions, significantly improving high-frequency device telemetry throughput. Flush is offloaded to `spawn_blocking` to avoid blocking the async runtime
- **Async storage I/O** — `MessageStore` operations (`insert`, `update`, `delete`, `list`) now have `*_async` wrappers that offload blocking redb I/O to `spawn_blocking`, preventing tokio runtime stalls
- **Batch delivery log writes** — Message delivery logs are collected per send cycle and written in a single lock acquisition, reducing lock contention
- **Tool response ID naming** — All aggregated tool responses now use explicit field names (`device_id`, `agent_id`, `rule_id`, `message_id`, `extension_id`) instead of generic `"id"`, improving LLM clarity
- **Token estimation consolidation** — Unified `estimate_tokens` and `estimate_message_tokens` into `tokenizer` module. Thinking content is correctly excluded from token counts (not sent to LLM)
- **Tool result compaction thresholds** — Increased keep threshold from 4KB→8KB, data-action preview from 300→2048 chars, and `CompactionConfig.max_message_length` from 8K/6K→32K/16K to preserve compact time-series format intact
- **Ollama thinking timeout guard** — Added `!skip_remaining_thinking` check to prevent repeated timeout warnings. Added 180s hard limit after timeout — terminates stream if model is stuck in thinking loop
- **ExtensionStore singleton** — `ExtensionState` now holds a shared `Arc<ExtensionStore>` instead of opening the database per call in `load_from_storage` and error handling paths
- **Error handling improvements** — `IsolatedExtension::new` uses `ok_or_else()` instead of `expect()` for child process stdin/stdout/stderr. API handlers use `From` conversion with `?` instead of `.map_err()`
- **InFlightRequests lock optimization** — Send response outside the mutex critical section, reducing lock hold time
- **Shared `ExtensionStore` in state** — `ExtensionState` constructors now accept `Arc<ExtensionStore>`, eliminating redundant `open()` calls in `load_from_storage` and auto-discovery
- **Image insight extraction** — Rewritten to use char-level operations for UTF-8 safety. Image analyses deduplicated by content fingerprint to prevent memory bloat
- **Agent panic protection** — `execute_agent` now catches panics via `catch_unwind` and converts them to Failed execution records instead of crashing the scheduler

### Fixed

- **Dashboard widget loading flash** — All 8 generic dashboard components (ValueCard, LineChart, BarChart, PieChart, Sparkline, ProgressBar, LEDIndicator, AgentMonitorWidget) now use `showLoading = loading && !hasData` pattern, preventing skeleton flash during periodic telemetry refreshes
- **DashboardGrid blank first frame** — Initial container width measurement now uses `useLayoutEffect` instead of `useEffect`, eliminating the blank frame caused by width 0 → measure → re-render
- **Dashboard DTO type safety** — Refactored `fromDashboardDTO` / `toDashboardDTO` to eliminate all `any` casts. Proper `ComponentDTO` interface, discriminated `GenericComponent`/`BusinessComponent` handling via `isGenericComponent()`
- **i18n fallback** — Removed hardcoded `lng: 'en'` default, allowing proper browser language detection. Settings tab labels now correctly use `settings:` namespace prefix
- **Agent config state injection** — Removed fragile `_agentsList`/`_visionModelsList` injection pattern in `componentConfig`. Dashboard now reads agent/model lists directly from component state
- **Extension sync consolidation** — Merged three separate extension sync effects in `App.tsx` into two cleaner effects (immediate on auth + periodic 60s timer)
- **Pending devices broker check** — Now checks both built-in MQTT broker (`connected`) and external brokers, instead of only external
- **Export dialog tree-shaking** — `xlsx` and `jszip` now loaded via dynamic `import()`, reducing initial bundle size
- **useDataSource cache leak** — Added `beforeunload` cleanup for the telemetry cache interval, preventing HMR interval accumulation in development
- **UTF-8 safe truncation** — Text truncation in agent prompts now correctly handles multi-byte characters at sentence boundaries, preventing panics on non-ASCII content
- **Agent editor state reset** — Creating a new agent now correctly resets `enableToolChaining` to prevent stale state from previous edits

## [v0.7.4] - 2026-05-11

### Added

- **Extension device management API** — Extensions can now register device type templates and device instances via new capabilities `DeviceTemplateRegister`, `DeviceRegister`, `DeviceUnregister`. Enables extensions to act as virtual device adapters
- **Extension command routing** — `DeviceService` now routes commands for extension-registered devices (adapter_type="extension") back to the owning extension via an `ExtensionCommandRouter` callback
- **Extension log viewer** — New `GET/DELETE /api/extensions/:id/logs` endpoints. Extensions capture stderr into a ring buffer (500 lines) with structured log entries (timestamp, level, message), viewable from the frontend details dialog
- **Extension crash recovery with config restore** — After crash recovery restart, the system automatically re-applies the extension's saved configuration from the extension store
- **Extension config_parameters support** — Extension runner now parses `config_parameters` from metadata JSON, enabling extensions to declare their configuration schema
- **Device metric update sets last_seen** — Reporting metrics from an extension now updates the device's `last_seen` timestamp, preventing "Never Connected" false status
- **Extension details full-screen dialog** — `ExtensionDetailsDialog` redesigned as `FullScreenDialog` with sidebar navigation: Overview, Configuration, Logs, Metrics, Commands — replacing the old tabbed modal
- **Extension SDK v0.6.3** — New `register_template()`, `register_device()`, `unregister_device()` functions for device management from extensions
- **Dashboard sharing system** — Full-featured share link management for dashboards: create links with read-only or interactive permissions, set expiration (1h–30d), copy/revoke links. Backend proxy forwards API requests via `x-internal-proxy` header for auth bypass. Shared dashboards render using the same component pipeline as the main dashboard
- **ShareManagerDialog** — New full-screen dialog for managing share links with "Add Share" dashed card pattern. Creation form in nested `UnifiedFormDialog` (z-[110])
- **Dashboard DualModeSourceField** — New dual-mode data source selector supporting both extension metrics and device metrics. Video-display component supports device-metric binding
- **Component library FullScreenDialog** — Replaced Sheet-based component library picker with `FullScreenDialog` for better space and consistency
- **Community component marketplace** — Backend API for browsing, installing, and managing community dashboard components. Manual install via file upload supported. New `FrontendComponentStore` for filesystem-based component storage
- **Marketplace browser & import UI** — `ComponentMarketplace` full-screen dialog for browsing and installing marketplace components with one-click install/uninstall. `InstallComponentDialog` for manual component import via file upload (manifest.json + bundle.js)
- **Frontend component runtime** — `CommunityRegistry`, `ComponentRenderer`, Zustand store slice for frontend components. WebSocket event system and lifecycle hooks for community components
- **Device binding for components** — Dashboard components can bind to devices via `deviceBinding` config. Bound components receive `deviceContext` (device info, current values) and `sendDeviceCommand` function. `DeviceBindingConfig` panel for selecting bound device and command parameters
- **Extension `has_device_binding` flag** — Extension components declare device binding support via `has_device_binding` in component definition

### Changed

- **Migrate to parking_lot locks** — Replaced `std::sync::RwLock`/`Mutex` with `parking_lot` equivalents across all backend crates (~80 lock `.unwrap()` calls eliminated). parking_lot locks never poison, removing a class of potential panics
- **Replace ExtensionStats API with ExtensionLogs API** — Removed `GET /api/extensions/:id/stats` and `ExtensionStatsDto`. Replaced with the new log viewer endpoints. Frontend store updated accordingly
- **ExtensionCard redesign** — Simplified from 570-line component to 148 lines by extracting details into `ExtensionDetailsDialog`
- **Fix unsafe error handling** — `shell.rs` now checks return values of `killpg` (Unix) and `TerminateProcess` (Windows) with logging on failure
- **Fix business logic unwrap()** — Replaced ~25 `unwrap()` calls in production code with `expect()`, `unwrap_or()`, or proper error propagation
- **Fix agent semaphore panic** — Tool concurrency semaphore closure now returns an error instead of panicking
- **Fix clippy -D warnings** — Resolved `is_multiple_of`, `Default` impl, `or_insert_with`, `map_or`, wildcard pattern, and `from_str` → `parse_category` naming issues
- **Fix broken test** — `test_cursor_decode_invalid_utf8` assertion corrected
- **Fix extension uninstall dialog** — Uninstall confirmation now correctly shows the extension name instead of literal `{{name}}`
- **Fix extension grid props** — Corrected `onConfigure` → `onDetails` prop name to match `ExtensionGrid` API
- **Bump version to 0.7.4** — Updated workspace, extension-runner, web, Tauri versions. Bumped extension-sdk to 0.7.0
- **Dashboard header buttons reordered** — Edit → Add Component → Share (Share moved to rightmost position). All buttons use `rounded-md` for consistent smaller border radius
- **"Add" button label** — Changed from "Add" to "Add Component" for clarity
- **Device re-registration** — `DeviceRegistry::register()` now updates existing devices in-place instead of returning `AlreadyExists` error, enabling idempotent extension re-registration
- **Fix last_seen timestamp unit** — Extension metric updates now use seconds instead of milliseconds for `last_seen`, matching device registry expectations
- **Device command dialog spacing** — Increased spacing between form fields in command control dialog for better readability
- **Dashboard sidebar alignment** — Fixed header alignment and markdown content padding in dashboard sidebar
- **Security: protected routes** — Moved sensitive APIs (LLM backends list, etc.) from public to protected routes. Removed `skipAuth` from frontend API calls that should require authentication

## [v0.7.3] - 2026-05-08

### Added

- **Relative Time Range for Tool Queries** — New `time_range` parameter for device, rule, message, and ai_metric tools. Supports human-readable strings like `"30min"`, `"1h"`, `"1d"`, `"1w"`, `"2w"` instead of Unix timestamps, solving small model timestamp calculation errors
- **Guided Error Messages** — All tool errors now include natural language guidance (e.g., entity not found → suggest list action, unknown action → show valid actions, operation failures → suggest next steps)
- **Time-Range Query Prompt** — Prompt builder now includes explicit time-range guidance to help small models correctly choose `history` action with `time_range` for time-based queries

### Changed

- **Tighter ReAct Loop Duplicate Detection** — Stop after 1 consecutive duplicate round (was 2), lower already-executed threshold to 50% (was 60%), add message_id/extension_id to signature checks
- **Stronger Inter-Round Context** — Multi-round context prompt now uses "STOP AND THINK" pattern to prevent small models from re-calling same tools with identical arguments
- **Device Tool Description** — Enhanced with stronger time-range keywords and examples to improve small model action selection accuracy

### Fixed

- **Repeated Tool Calls** — Fixed small models repeatedly calling same tool (e.g., `message(list)` 3 times in a row) by tightening loop detection and improving inter-round prompts
- **Wrong Action for Time Queries** — Fixed models using `device(list)` instead of `device(history)` when user asks about trends or time ranges

### Removed

- **Dead Code** — Removed unused `ToolOutput::error_with_data()` method
- **Chinese Hardcoding** — Replaced all hardcoded Chinese text in code with English (aliases, error messages, examples, test assertions)

---

## [v0.7.2] - 2026-05-06

### Added

- **Multi-Instance Management** — Connect to and switch between multiple NeoMind backends (local + remote) with full-screen instance manager dialog, instance selector pill in navigation bar, and animated switch overlay
- **Instance CRUD API** — REST endpoints (`/api/instances`) for creating, listing, updating, deleting, and testing remote backend instances with API key authentication
- **Instance Storage** — Persistent storage for remote instance metadata in `instances.redb` (redb-backed)
- **Unified Auth Verification** — New `GET /api/auth/verify` endpoint that accepts both JWT and API key authentication, used for pre-switch key validation
- **API Key Pre-Validation** — Instance switching validates API keys against the remote backend before switching, preventing broken states with clear error messages
- **API Key Form Validation** — Instance add/edit form validates API keys in real-time against the remote instance before saving, with visual feedback (check/error icons)
- **Remote Instance UX** — Instance manager hides management actions (add/edit/delete) when connected to a remote instance, shows contextual hint banner
- **CLI API Key Management** — `neomind api-key create/list/delete` commands for managing API keys from the command line with custom data directory support
- **Auth Data Dir Support** — `AuthState::new_with_data_dir()` for CLI tools to use custom data directories for API key storage
- **Persistent Encryption Key** — Encryption key for API key storage auto-generated and persisted to `data/encryption_key` file, survives server restarts without needing `NEOMIND_ENCRYPTION_KEY` env var
- **Encryption Key Fallback Chain** — `CryptoService` now follows priority: env var → persistent file → generate + save, ensuring API keys remain valid across restarts

### Fixed

- **Infinite API Loop on Devices Page** — TransformsBadge and DeviceTransformsDialog fetched devices, device types, and transforms on every mount, causing N×3 redundant API calls per page load. Fixed with conditional dialog rendering (`{open && <Dialog />}`) and shared `fetchCache` for transform list queries
- **Mobile Content Top Padding** — Extensions and Settings pages had inconsistent top spacing compared to other pages. Unified mobile content padding to `pt-2` in PageLayout
- **Mobile Action Button Inconsistency** — Page action buttons used different sizes (`h-8 text-xs` vs `h-9 text-sm`) on mobile. Unified all page action buttons to use standard `size="sm"` for consistent appearance
- **Extensions Page Header Layout** — Moved Extensions page action buttons into `headerContent` slot for consistent fixed positioning with other tabbed pages
- **WebSocket Infinite Reconnect Loop** — Switching to a remote instance with an invalid API key caused WebSocket to repeatedly fail auth → reload page → fail again. Fixed by separating API key errors (disconnect without reload) from JWT errors (reload to re-login)
- **WebSocket Close Code for Auth** — Server now sends close code `4001` for WebSocket auth rejections, allowing the client to distinguish auth failures from normal disconnects
- **API Key Not Clearing on Edit** — Clearing the API key field in instance edit form didn't remove the key (empty string was sent as `undefined`). Fixed: frontend sends empty string, backend treats it as `api_key = None`
- **Stale Zustand Persist Cache** — Old `currentInstanceId` from Zustand persist could override localStorage-based instance selection after page refresh. Fixed with persist version bump (v2) and migration that removes the stale field
- **Validation Icon Layout Shift** — API key validation icon (checkmark/error/spinner) caused input field width to shift. Fixed by reserving space with `pr-8` padding on the input
- **Remote Instance Shows Offline** — Instance selector always showed offline for remote instances because `isAuthenticated` only checked JWT token, not API key. Fixed `checkAuthStatus` to recognize API key as valid authentication, enabling WebSocket connections for remote instances
- **Login Page Stuck on Remote Instance** — Switching to a remote instance with API key from login page stayed on login instead of redirecting to dashboard. Login page now detects API key auth and redirects immediately
- **Stale Instance Cache After Edit** — Editing an instance (e.g. clearing API key) updated the Zustand store but not the localStorage cache (`neomind_instance_cache`), causing login page to use stale data. Fixed: all instance CRUD operations now sync to localStorage cache immediately
- **API Key Stored in Plaintext in Browser** — Backend now returns masked API keys (e.g. `nmk_abc1****`) in list/get/update responses. Full keys are held only in JavaScript memory during the add/edit session and never persisted to localStorage. Edit form shows masked key with option to clear or replace
- **Failed Switch Doesn't Revert** — Dismissing the error overlay after a failed instance switch left `currentInstanceId` pointing to the unreachable target, causing reconnection attempts on next refresh. Fixed: `clearSwitchingError` now reverts to the previous instance
- **revertSwitch Could Get Stuck** — If the instance list was empty after switching to a remote instance, reverting failed silently. Fixed: `revertSwitch` now falls back to `getCachedInstances()` when the in-memory list is empty
- **Duplicated localStorage Key Constants** — Instance-related localStorage keys were defined independently in `instanceSlice.ts` and `login.tsx`. Extracted to shared `instance-constants.ts` module

### Changed

- **Dynamic API Base URL** — Refactored `getApiBase()` to support runtime URL switching via `setApiBase()` for multi-instance support, extracted URL/key utilities to `urls.ts`
- **WebSocket/SSE/Extension Stream Auth** — All real-time connections support both JWT token and API key authentication. API key sent as query parameter for WebSocket/SSE, enabling passwordless access to remote instances
- **ProtectedRoute Accepts API Key** — Frontend route guard allows access when either JWT token or API key is present, enabling passwordless remote instance access
- **Connection Status → Instance Selector** — TopNav connection status indicator replaced with instance selector pill showing current instance name and connectivity status
- **Instance Manager Full-Screen Dialog** — Instance list opens as full-screen dialog (replacing dropdown) for better usability on mobile and desktop
- **Login Page Instance Selector** — Login page includes instance selector dropdown using cached instance list, allowing connection to remote backends before authentication
- **Setup Wizard Split** — Setup wizard pages extracted into separate files under `web/src/pages/setup/` for maintainability

---

## [v0.7.1] - 2026-05-04

### Added

- **BLE Provisioning** — Zero-touch device setup via Bluetooth Low Energy with dual transport support (Tauri native BLE via btleplug + Web Bluetooth API)
- **BLE Device Config Read** — Read device info (MAC, SN, model, netmod type) from BLE characteristic on connect for pre-filling configuration
- **BLE Netmod Support** — Adapt provisioning UI based on device network module type (WiFi / HaLow / Cat.1 cellular), hide WiFi config for Cat.1 devices
- **BLE Re-provisioning** — Update existing device info (name, broker, MQTT config) when re-provisioning via BLE; show "Configuration Updated" success message
- **BLE Device Name Sync** — Write user-specified device name to firmware storage during BLE provisioning
- **BLE Preparation Guide** — Step-by-step instructions on scan page to guide users through the provisioning flow
- **Auto Discovery Broker Guidance** — Contextual empty state in Pending Devices that guides users to add MQTT broker in Settings
- **Network Info API** — `GET /api/system/network-info` returns WiFi SSID and LAN IP for BLE provisioning

### Fixed

- **Device Type Dropdown Loading** — Add Device dialog now fetches device types on open instead of relying on stale cache
- **WebSocket Not Auto-Recovering** — Added missing `online` event listener for network recovery and reset `isManualDisconnect` flag in `connect()`
- **WebSocket Disconnected After Page Refresh** — Auth state initially false caused disconnect flag to stick, blocking reconnect
- **About Page Memory Progress Bar** — Used `bg-*` classes instead of `text-*` for progress bar fill color
- **Layout Flicker on Page Switch** — Responsive hooks (`useIsDesktop`, `useIsMobile`, `useIsTouchDevice`, `useDeviceType`) now read `window.innerWidth` synchronously on first render
- **Focus Ring on Click** — Suppressed `:focus-visible` ring on mouse clicks in Tauri/Chromium
- **BLE WiFi SSID 404** — Fixed frontend calling non-existent `/system/wifi-ssid` endpoint → use registered `/system/network-info`
- **BLE Success Screen** — Deferred `onComplete` callback to done phase close button instead of closing dialog immediately on apply
- **BLE MQTT Characteristic Optional** — Handle older firmware without MQTT characteristic gracefully
- **BLE Empty WiFi Password** — Allow empty password for open WiFi networks

### Changed

- **BLE Two-Phase Provisioning** — Split into resolve-only (get MQTT config) → BLE write → register device, preventing phantom devices on BLE failure
- **BLE Scanned Device Cards** — Display MAC address instead of model name for easier device identification
- **Pending Devices Table** — Removed column header icons for cleaner appearance
- **Add Device Dialog Icons** — Updated tab and header icons for better semantic meaning

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
