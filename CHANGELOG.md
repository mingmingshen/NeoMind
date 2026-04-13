# Changelog

All notable changes to NeoMind will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
