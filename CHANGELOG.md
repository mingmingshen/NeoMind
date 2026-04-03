# Changelog

All notable changes to NeoMind will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
