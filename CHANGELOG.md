# Changelog

All notable changes to NeoMind will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [v0.6.5] - 2025-04-09

### Changed

- **Token-Based Context Management** — Conversation history is now managed using token counting instead of message count, providing more accurate context window utilization and preventing overflow across different LLM backends.
- **Context Overflow Retry** — When a request fails due to context window overflow, the system automatically retries with progressively shorter context, improving resilience without user intervention.
- **Dashboard Grid Rewrite** — Replaced the controlled-mode `react-grid-layout` integration with a ref-based approach that avoids the feedback loop between layout state and component re-renders, eliminating jitter and positioning bugs.
- **Aggregated Tool Enhancements** — Improved tool descriptions and added `latest_execution` and `send_message` actions for better agent execution monitoring and control.

### Fixed

- **New Component Default Size** — Newly added dashboard components now appear at correct default sizes instead of minimum (1×1). Root cause: RGL v2's `synchronizeLayoutWithChildren` created `w:1, h:1` defaults for children not yet in internal layout state. Fixed by calculating actual y position instead of relying on `y:9999` compact, and adding `data-grid` props as safety fallback.
- **Config Data Refresh** — Component data now updates immediately when editing data binding in the config dialog, instead of requiring save and exit. Added `configVersion` tracking to force grid re-render on live config changes.
- **Chart Responsive Resize** — Chart components (LineChart, BarChart, PieChart, AreaChart) now properly fill their container when resized via drag handles. Replaced fixed pixel heights with flex-based layout (`flex-1 min-h-0`).
- **llama.cpp Multimodal Detection** — Automatically detect vision, tool calling, and context size capabilities from llama.cpp server's `/props` endpoint.

---

## [v0.6.4] - 2025-04-08

### New Features

- **Planning System** — New execution plan generation module (`agent/planner/`) with two planners: `KeywordPlanner` (fast, rule-based, zero LLM cost) and `LLMPlanner` (deep, LLM-generated for complex tasks). `PlanningCoordinator` routes between them based on intent confidence. Includes `ExecutionPlanCreated`, `PlanStepStarted`, `PlanStepCompleted` WebSocket events and `ExecutionPlanPanel` UI component.
- **EntityResolver** — Fuzzy entity name/ID matching for all LLM tool parameters. Resolves human-readable names to internal IDs using progressive matching: exact ID → exact name (case-insensitive) → substring match. Reduces tool round-trips by handling ambiguous references.
- **Device Info Enrichment** — Device query results now include live metrics and available commands alongside basic device info. Metric names are automatically resolved from user-friendly aliases.
- **AlertTool Get Action** — Added `get` action to `AlertTool` for retrieving individual alert details by ID.

### Changed

- **Aggregated Tools Refactor** — Optimized tool descriptions and prompts for the aggregated tool architecture (~8 tools instead of ~50), reducing token usage by 60%+.
- **Memory Extraction** — Memory extraction now runs as a background task instead of blocking the response pipeline. Chinese-language extraction prompts localized to English.
- **Version Bump** — Bumped version to 0.6.4 with UI refinements.

### Fixed

- **llama.cpp Multimodal Auto-Detection** — Automatically detect multimodal (vision), tool calling, and context size capabilities from llama.cpp server's `/props` endpoint. Capabilities are persisted to storage and updated at startup. Previously, llama.cpp backends always reported `supports_multimodal: false`.
- **llama.cpp Streaming Timeout** — Removed global HTTP client timeout that killed long-running streaming responses. Streaming requests now run without time limits; non-streaming requests use a 600s per-request timeout.
- **Context Window Overflow** — Conversation history is now automatically truncated to fit the model's context window. Older messages are dropped first when the total prompt exceeds 70% of `max_context`. This prevents `exceed_context_size_error` errors, especially with multimodal messages containing images.
- **Tool Loop Detection False Positive** — Tool loop detection now only blocks exact duplicate calls (same tool name + same arguments). Previously, calling the same tool 3+ times with different arguments (e.g., querying different devices) was incorrectly blocked as a loop.
- **Tool Loop Detection Non-Blocking** — Changed loop detection event from `Error` to `Warning` so it no longer interrupts the conversation flow. The LLM continues generating a text response when a duplicate tool call is skipped.
- **Memory Scheduler Auto-Start** — Fixed memory scheduler never starting when LLM backend becomes available after server startup. Replaced one-shot startup attempt with background retry task that polls every 30 seconds for LLM runtime availability. Added idempotency protection to prevent duplicate scheduler instances.
- **Scheduler Concurrency Bug** — Fixed race condition in agent scheduler that could cause duplicate executions. Added status reset retry, health check, and execution retry logic.
- **False Alert Notifications** — Prevented false alert notifications from being sent. LLM thinking tokens are now stripped from agent conclusions. Reduced redundant tool rounds.
- **Extension jsxRuntime** — Exposed `jsxRuntime` global for extension UMD bundles, fixing React JSX runtime resolution in frontend components.
- **Extension .nep Extraction** — Fixed extraction of bundled native libraries from `.nep` extension packages.

---

## [v0.6.3] - 2025-04-03

### Overview

Feature release focusing on **Memory System Integration** and **MQTT Security**.

**Highlights**:
- Memory Scheduler - LLM-powered automatic memory extraction and compression
- Category-based Memory - Reorganized memory with 4 categories (Profile, Knowledge, Tasks, Evolution)
- MQTT mTLS Support - Secure MQTT connections with client certificates
- Agent LLM Decoupling - Separate LLM backends for agents vs chat
- Memory Panel Redesign - Full-screen dialog with better UX

---

### New Features

#### Memory System
- **MemoryScheduler Integration** - Automatic memory extraction/compression on LLM backend activation
- **Scheduled Extraction** - Periodic extraction from chat sessions to memory
- **Manual Compression API** - `/api/memory/compress` endpoint for on-demand compression
- **MemoryCompressor** - LLM-based memory summarization with importance decay
- **Category-based Memory API** - REST endpoints for UserProfile, DomainKnowledge, TaskPatterns, SystemEvolution

#### MQTT Security
- **mTLS Support** - Client certificate authentication for MQTT brokers
- **CA Certificate** - Custom CA certificate configuration
- **Secure Connection** - Enhanced TLS/SSL options for device connections

#### Agent System
- **Per-step Result Field** - Track execution results for each reasoning step
- **LLM Backend Decoupling** - Agents use independent LLM backends, not tied to chat model
- **Capability Correction** - Consistent vision/reasoning capability detection

#### UI Improvements
- **MemoryPanel Redesign** - Full-screen dialog with category tabs
- **ResponsiveTable** - Better memory list display
- **CodeMirror Width Fix** - Editor fills full dialog width

---

### Bug Fixes

- Fixed chat model selection overwriting agent LLM backends
- Fixed LLM capability correction inconsistency
- Fixed CodeMirror width in edit mode
- Fixed memory extraction API response field

---

### Summary

- Memory system with LLM-powered extraction and compression
- MQTT mTLS for secure device connections
- Category-based memory organization
- Agent/chat LLM backend decoupling
- Improved memory panel UX

**Production-ready and recommended for all users.**

---

**Release Date**: April 3, 2025

---

**For issues or questions**: https://github.com/camthink-ai/NeoMind/issues
