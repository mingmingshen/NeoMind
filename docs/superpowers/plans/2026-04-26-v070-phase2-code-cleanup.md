# v0.7.0 Phase 2: Code Cleanup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Zero unresolved TODOs, remove dead code, clean console artifacts, enforce consistent style across frontend and backend.

**Architecture:** Three parallel tracks — (A) resolve 13 TODO/FIXME items, (B) dead code + style enforcement, (C) console/debug cleanup.

**Tech Stack:** Rust, TypeScript, React, ESLint, cargo fmt/clippy

**Spec:** `docs/superpowers/specs/2026-04-26-v0.7.0-release-plan-design.md` Part 3 (Sections 3.1, 3.2)

**Depends on:** Phase 1 (backend changes should be merged first to minimize merge conflicts)

---

## Track A: TODO/FIXME Zero-Out

### Task A1: Implement LLM Backend Create Dialog (Frontend TODO #1)

**Files:**
- Modify: `web/src/components/llm/LLMBackendConfigDialog.tsx`

**Context:** Line 68 — `if (mode === 'create') { // TODO: Implement create }`. The dialog currently only supports editing existing backends.

- [ ] **Step 1: Read current dialog code**

Read the full file to understand form fields, state management, and API calls used for edit mode.

- [ ] **Step 2: Implement create mode form**

```typescript
if (mode === 'create') {
  // Populate form with defaults for new backend
  // On submit, call POST /api/llm/backends with form data
  // On success, refresh backend list and close dialog
}
```

- [ ] **Step 3: Test in browser**

Navigate to Settings → LLM Backends → click "Add Backend". Verify form creates a new backend.

- [ ] **Step 4: Commit**

```bash
git add web/src/components/llm/LLMBackendConfigDialog.tsx
git commit -m "feat(llm): implement backend creation in config dialog"
```

---

### Task A2: Implement Agent Available Resources API (Frontend TODO #2)

**Files:**
- Modify: `crates/neomind-api/src/handlers/agents.rs` (or relevant agent handler file)
- Modify: `web/src/pages/agents-components/AgentDetailPanel.tsx`

**Context:** Line 179 — `// TODO: Implement /api/agents/{id}/available-resources endpoint`. The function returns early without fetching data.

- [ ] **Step 1: Create backend endpoint**

```rust
pub async fn get_available_resources(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<Vec<AvailableResource>> {
    let resources = state.resource_provider.list_available(&id).await
        .map_err(|e| ErrorResponse::internal(&e.to_string()))?;
    ok(resources)
}
```

- [ ] **Step 2: Register route in router**

Add `GET /api/agents/:id/available-resources` to the agent routes.

- [ ] **Step 3: Implement frontend integration**

```typescript
const loadAvailableResources = async () => {
  try {
    const res = await api.get(`/api/agents/${agentId}/available-resources`);
    setAvailableResources(res.data);
  } catch {
    // Endpoint may not be available in older versions
  }
}
```

- [ ] **Step 4: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-api`

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-api/src/ web/src/pages/agents-components/AgentDetailPanel.tsx
git commit -m "feat(agents): implement available-resources endpoint and frontend integration"
```

---

### Task A3: Dashboard Interactions — VisualDashboard (Frontend TODOs #3-5)

**Files:**
- Modify: `web/src/pages/dashboard-components/VisualDashboard.tsx`

**Context:** Lines 4372-4378 — Three TODOs for device details, metric tooltip, and command execution from dashboard.

- [ ] **Step 1: Read current VisualDashboard interaction code**

Understand the event handling structure around lines 4370-4380.

- [ ] **Step 2: Implement device detail panel**

On device marker click, show a slide-out panel with device status, last seen, recent metrics.

- [ ] **Step 3: Implement metric tooltip**

On metric chart hover, show a tooltip with current value, timestamp, and trend.

- [ ] **Step 4: Implement command execution**

On command action, open a command dialog with device selection and command input.

- [ ] **Step 5: Test in browser**

Navigate to Dashboard, click device markers, hover charts, trigger commands.

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/dashboard-components/VisualDashboard.tsx
git commit -m "feat(dashboard): implement device detail panel, metric tooltip, and command execution"
```

---

### Task A4: Dashboard Interactions — MapDisplay (Frontend TODOs #6-8)

**Files:**
- Modify: `web/src/components/dashboard/generic/MapDisplay.tsx`

**Context:** Lines 977-987 — Three TODOs mirroring the VisualDashboard TODOs for map component.

- [ ] **Step 1: Wire up device detail panel in map**

Reuse the panel component from Task A3.

- [ ] **Step 2: Wire up metric tooltip in map markers**

- [ ] **Step 3: Wire up command execution from map**

- [ ] **Step 4: Commit**

```bash
git add web/src/components/dashboard/generic/MapDisplay.tsx
git commit -m "feat(dashboard): wire up MapDisplay device details, metric tooltip, and commands"
```

---

### Task A5: Dashboard Config — Selector Dialogs (Frontend TODOs #9-11)

**Files:**
- Modify: `web/src/components/dashboard/config/DataSourceConfigSection.tsx`
- Create: `web/src/components/dashboard/config/DeviceDataSourceSelectorDialog.tsx`
- Create: `web/src/components/dashboard/config/MetricSelectorDialog.tsx`
- Create: `web/src/components/dashboard/config/CommandSelectorDialog.tsx`

**Context:** Lines 51, 62, 71 — Three TODOs for selector dialogs (device/data source, metric, command).

- [ ] **Step 1: Create DeviceDataSourceSelectorDialog**

A searchable dialog that lists all data sources with filtering by type.

- [ ] **Step 2: Create MetricSelectorDialog**

A dialog listing metrics for a selected data source.

- [ ] **Step 3: Create CommandSelectorDialog**

A dialog listing commands for a selected device.

- [ ] **Step 4: Wire into DataSourceConfigSection**

Replace the three TODO stubs with dialog open calls.

- [ ] **Step 5: Commit**

```bash
git add web/src/components/dashboard/config/
git commit -m "feat(dashboard): implement data source, metric, and command selector dialogs"
```

---

## Track B: Dead Code + Style Enforcement

### Task B1: Rust Code Cleanup

- [ ] **Step 1: Run cargo clippy and fix warnings**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo clippy --workspace -- -D warnings 2>&1 | head -50`

Fix all warnings.

- [ ] **Step 2: Run cargo fmt**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo fmt --all -- --check 2>&1`

If any files need formatting, run: `cargo fmt --all`

- [ ] **Step 3: Remove unused imports**

Run: `cargo clippy --workspace 2>&1 | grep "unused import" | head -20`

Fix each warning by removing unused imports.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: rust code cleanup — clippy fixes, fmt, unused imports"
```

---

### Task B2: Frontend Dead Code Removal

- [ ] **Step 1: Run ESLint with no-unused-vars**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind/web && npx eslint src/ --rule 'no-unused-vars: error' --rule '@typescript-eslint/no-unused-vars: error' 2>&1 | head -50`

- [ ] **Step 2: Remove confirmed dead code**

- Unused imports
- Unused type definitions
- Unused helper functions
- Commented-out code blocks

- [ ] **Step 3: Run TypeScript type check**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind/web && npx tsc --noEmit 2>&1 | head -30`

Fix all type errors.

- [ ] **Step 4: Commit**

```bash
git add web/src/
git commit -m "chore: remove unused frontend code and fix type errors"
```

---

## Track C: Console/Debug Cleanup

### Task C1: Remove Non-Essential Console Statements

**Files:** 46 frontend files with ~132 console calls

**Top priority files (highest counts):**
- `web/src/store/slices/dashboardSlice.ts` (15 calls)
- `web/src/components/dashboard/registry/DynamicRegistry.ts` (14 calls)
- `web/src/lib/extension-stream.ts` (10 calls)
- `web/src/store/persistence/implementations.ts` (8 calls)
- `web/src/components/dashboard/generic/ai-analyst/useAnalystSession.ts` (6 calls)

- [ ] **Step 1: Audit console calls by severity**

```bash
cd /Users/shenmingming/CamThink\ Project/NeoMind/web
echo "=== console.log ===" && grep -rn 'console\.log' src/ | wc -l
echo "=== console.debug ===" && grep -rn 'console\.debug' src/ | wc -l
echo "=== console.warn ===" && grep -rn 'console\.warn' src/ | wc -l
echo "=== console.error ===" && grep -rn 'console\.error' src/ | wc -l
echo "=== console.info ===" && grep -rn 'console\.info' src/ | wc -l
```

**Rules:**
- `console.error` in catch blocks: **KEEP** (legitimate error logging)
- `console.warn` for deprecation notices: **KEEP**
- `console.log/debug/info`: **REMOVE** (unless in a clearly justified error path)
- DEBUG comments: **REMOVE**

- [ ] **Step 2: Clean dashboard slice (15 calls)**

Remove all `console.log/debug/info` calls in `dashboardSlice.ts`.

- [ ] **Step 3: Clean DynamicRegistry (14 calls)**

Remove debug logging in component registration.

- [ ] **Step 4: Clean extension-stream (10 calls)**

Remove debug logging in stream management.

- [ ] **Step 5: Clean remaining top-10 files**

Address the remaining high-count files.

- [ ] **Step 6: Clean all other files**

Remove non-essential console calls across remaining files.

- [ ] **Step 7: Remove DEBUG comment**

In `web/src/hooks/useDataSource.ts:1324`, remove the `// DEBUG:` comment and any associated debug code.

- [ ] **Step 8: Verify build**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind/web && npm run build 2>&1 | tail -10`

- [ ] **Step 9: Commit**

```bash
git add web/src/
git commit -m "chore: remove non-essential console statements and debug artifacts"
```

---

### Task C2: Replace `alert()` with Toast Notifications

**Files:**
- Modify: `web/src/components/devices/DeviceControl.tsx:320`
- Modify: `web/src/pages/chat.tsx:572,655,672`
- Modify: `web/src/pages/devices.tsx:466,472,475`

**Context:** 7 `alert()` calls in 3 files. The project uses a custom Radix UI toast via `useToast()` from `@/hooks/use-toast`.

- [ ] **Step 1: Replace alert() in DeviceControl.tsx**

```typescript
// BEFORE
alert(t('devices:control.jsonFormatError'))

// AFTER
toast({ title: t('devices:control.jsonFormatError'), variant: "destructive" })
```

- [ ] **Step 2: Replace alert() in chat.tsx**

Replace all 3 instances with `toast()`.

- [ ] **Step 3: Replace alert() in devices.tsx**

Replace all 3 instances with `toast()`.

- [ ] **Step 4: Verify no remaining alert() calls**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind/web && grep -rn "alert(" src/ | grep -v node_modules`

Expected: Zero results (or only legitimate non-blocking uses).

- [ ] **Step 5: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind/web && npm run build 2>&1 | tail -10`

- [ ] **Step 6: Commit**

```bash
git add web/src/
git commit -m "fix: replace alert() with toast notifications across all pages"
```

---

## Completion Checklist

- [ ] All 13 TODO/FIXME items resolved (2 backend + 11 frontend)
- [ ] `cargo clippy --workspace` passes with no warnings
- [ ] `cargo fmt --all -- --check` passes
- [ ] `npx tsc --noEmit` passes in web/
- [ ] Zero `alert()` calls in frontend
- [ ] Non-essential console statements removed
- [ ] DEBUG comments removed
