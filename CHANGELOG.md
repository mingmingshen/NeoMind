# Changelog

All notable changes to NeoMind will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
