# Changelog

All notable changes to NeoMind will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [v0.6.4] - 2025-04-08

### Fixed

- **llama.cpp Multimodal Auto-Detection** — Automatically detect multimodal (vision), tool calling, and context size capabilities from llama.cpp server's `/props` endpoint. Capabilities are persisted to storage and updated at startup. Previously, llama.cpp backends always reported `supports_multimodal: false`.
- **llama.cpp Streaming Timeout** — Removed global HTTP client timeout that killed long-running streaming responses. Streaming requests now run without time limits; non-streaming requests use a 600s per-request timeout.
- **Context Window Overflow** — Conversation history is now automatically truncated to fit the model's context window. Older messages are dropped first when the total prompt exceeds 70% of `max_context`. This prevents `exceed_context_size_error` errors, especially with multimodal messages containing images.
- **Tool Loop Detection False Positive** — Tool loop detection now only blocks exact duplicate calls (same tool name + same arguments). Previously, calling the same tool 3+ times with different arguments (e.g., querying different devices) was incorrectly blocked as a loop.
- **Tool Loop Detection Non-Blocking** — Changed loop detection event from `Error` to `Warning` so it no longer interrupts the conversation flow. The LLM continues generating a text response when a duplicate tool call is skipped.
- **Memory Scheduler Auto-Start** — Fixed memory scheduler never starting when LLM backend becomes available after server startup. Replaced one-shot startup attempt with background retry task that polls every 30 seconds for LLM runtime availability. Added idempotency protection to prevent duplicate scheduler instances.

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
