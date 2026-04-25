# v0.7.0 Phase 5: Documentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update all documentation to reflect v0.7.0 — API docs, user guides (EN/ZH), extension guide, and migration guide.

**Architecture:** Three tracks — (A) API/Swagger docs, (B) user guides + extension guide, (C) migration guide + CHANGELOG.

**Tech Stack:** Markdown, OpenAPI/Swagger, bilingual (EN/ZH)

**Spec:** `docs/superpowers/specs/2026-04-26-v0.7.0-release-plan-design.md` Part 4

**Depends on:** All prior phases should be complete or near-complete before documentation is finalized.

---

## Track A: API Documentation

### Task A1: Update Swagger/OpenAPI Spec

**Files:**
- Modify: API docs endpoint (likely `crates/neomind-api/src/docs.rs` or swagger config)

**Context:** Swagger spec currently shows v0.5.9, codebase is at v0.6.12. Needs to catch up to v0.7.0.

- [ ] **Step 1: Audit current API routes vs documented routes**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && grep -rn '\.route\|\.get\|\.post\|\.put\|\.delete' crates/neomind-api/src/routes/ | grep -v test`

Compare against Swagger spec to find undocumented endpoints.

- [ ] **Step 2: Add missing endpoints to Swagger**

Known missing endpoints from v0.6.x:
- `POST /api/extensions/:id/push-metrics`
- `GET /api/telemetry` (generic telemetry API)
- `GET /api/data/sources` with pagination params (offset, limit, source_type, search)
- Agent execution mode changes (Focused/Free)
- Extension health and config metadata fields

- [ ] **Step 3: Update schemas**

- `ExecutionMode`: `Chat`/`React` → `Focused`/`Free`
- `DataSourceId`: `device_id` → `source_id` rename
- New fields: `health_status`, `last_error`, `config_parameters` on extensions

- [ ] **Step 4: Update version to v0.7.0**

- [ ] **Step 5: Verify Swagger UI**

Run the server and visit `http://localhost:9375/api/docs` to verify.

- [ ] **Step 6: Commit**

```bash
git add crates/neomind-api/src/
git commit -m "docs: update Swagger/OpenAPI spec to v0.7.0"
```

---

## Track B: User Guides + Extension Guide

### Task B1: Update English User Guides

**Files:**
- Modify: `docs/guides/en/*.md`

- [ ] **Step 1: Identify guides needing updates**

Run: `ls -la /Users/shenmingming/CamThink\ Project/NeoMind/docs/guides/en/`

Key guides to check:
- `02-llm.md` — LLM backend configuration
- `03-agent.md` — Agent modes (Focused/Free), skills, shell tool
- `04-devices.md` — Device management
- `10-storage.md` — Storage layer
- `extension-system.md` — Extension SDK, Push mode

- [ ] **Step 2: Update agent guide (03-agent.md)**

Document:
- Focused vs Free mode (replaces Chat/React)
- Skill system
- Shell tool
- Agent status sync

- [ ] **Step 3: Update extension guide (extension-system.md)**

Document:
- Push mode FFI callback
- Extension health status
- Config parameters
- Instance reset

- [ ] **Step 4: Update storage guide (10-storage.md)**

Document:
- Generic telemetry API
- `source_id` rename
- Server-side pagination for data sources

- [ ] **Step 5: Update remaining guides**

Check each guide for outdated information.

- [ ] **Step 6: Commit**

```bash
git add docs/guides/en/
git commit -m "docs: update English user guides for v0.7.0"
```

---

### Task B2: Update Chinese User Guides

**Files:**
- Modify: `docs/guides/zh/*.md`

- [ ] **Step 1: Sync Chinese guides with English updates**

Apply the same changes as Task B1 to the Chinese versions.

- [ ] **Step 2: Commit**

```bash
git add docs/guides/zh/
git commit -m "docs: update Chinese user guides for v0.7.0"
```

---

## Track C: Migration Guide + CHANGELOG

### Task C1: Write v0.6 → v0.7 Migration Guide

**Files:**
- Create: `docs/guides/en/migration-0.6-to-0.7.md`
- Create: `docs/guides/zh/migration-0.6-to-0.7.md`

- [ ] **Step 1: Write English migration guide**

Cover:
- Breaking changes (if any)
- New API endpoints
- Configuration changes
- Behavior changes (e.g., event trigger cooldown 5s → 60s)
- Updated dependencies

```markdown
# Migration Guide: v0.6 → v0.7

## Overview
v0.7.0 is a quality-focused release with stability improvements, UI polish,
and comprehensive testing. There are no breaking API changes from v0.6.12.

## What's New
### Backend Stability
- Hot-path error handling prevents production panics
- API input validation on all mutating endpoints
- Settings now persist across restarts (redb-backed)
- MQTT custom topic unsubscription

### Frontend
- Consistent skeleton loading across all pages
- Pagination standardized to 10 items per page
- Toast notifications replace alert() dialogs
- Form validation with inline error messages
- Confirmation dialogs for destructive operations
- Dashboard device details, metric tooltips, command execution

### Testing
- Comprehensive unit tests across 6 core crates

## Breaking Changes
None. v0.7.0 is fully backward-compatible with v0.6.x.

## Configuration Changes
- Settings are now persisted in `data/settings.redb`
- Event trigger cooldown changed from 5s to 60s (configurable)

## API Changes
- New: `GET /api/agents/:id/available-resources`
- Updated: All POST/PUT endpoints return 400 on invalid input
```

- [ ] **Step 2: Write Chinese version**

- [ ] **Step 3: Commit**

```bash
git add docs/guides/en/migration-0.6-to-0.7.md docs/guides/zh/migration-0.6-to-0.7.md
git commit -m "docs: add v0.6 to v0.7 migration guide (EN/ZH)"
```

---

### Task C2: Write CHANGELOG for v0.7.0

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Draft CHANGELOG entry**

```markdown
## [v0.7.0] - 2026-XX-XX

### Added
- **API Input Validation** — All POST/PUT endpoints validate parameters...
- **Settings Persistence** — Settings saved to redb, survive restarts...
- **MQTT Topic Unsubscription** — Custom topics can now be unsubscribed...
- **Dashboard Interactions** — Device detail panel, metric tooltips, command execution...
- **Selector Dialogs** — Data source, metric, and command selector dialogs for dashboard config...
- **Empty State Guidance** — All list pages show helpful guidance when empty...
- **Confirmation Dialogs** — Destructive operations require explicit confirmation...
- **Form Validation** — Agent, device, and rule editors validate input in real-time...
- **Error Boundaries** — React Error Boundaries for graceful failure handling...

### Changed
- **Error Handling** — Replaced 1000+ hot-path `unwrap()` calls with safe error propagation across 8 crates
- **Pagination** — Standardized default page size to 10 across all pages
- **Loading States** — All page-level loading uses skeleton screens instead of spinners
- **Notifications** — Replaced `alert()` with toast notifications throughout the UI
- **Error Messages** — User-friendly error messages for API failures

### Fixed
- **Rule Engine** — Catch-all error recovery prevents scheduler crashes
- **Console Cleanup** — Removed 130+ non-essential console statements

### Testing
- Added comprehensive unit tests to neomind-agent, neomind-storage, neomind-rules, neomind-messages, neomind-extension-runner, neomind-api
```

- [ ] **Step 2: Update version in Cargo.toml**

```bash
# Update workspace version from 0.6.12 to 0.7.0
```

- [ ] **Step 3: Update version in web/package.json** (if applicable)

- [ ] **Step 4: Commit**

```bash
git add CHANGELOG.md Cargo.toml web/package.json
git commit -m "docs: update CHANGELOG and bump version to v0.7.0"
```

---

## Completion Checklist

- [ ] Swagger/OpenAPI spec updated to v0.7.0 with all endpoints
- [ ] English user guides updated for all new features
- [ ] Chinese user guides synced with English
- [ ] Migration guide (EN/ZH) created
- [ ] CHANGELOG.md updated for v0.7.0
- [ ] Version bumped to 0.7.0 in Cargo.toml and package.json
