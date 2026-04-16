# Changelog

All notable changes to NeoMind will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
