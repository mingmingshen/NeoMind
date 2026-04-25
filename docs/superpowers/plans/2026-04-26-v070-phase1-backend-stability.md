# v0.7.0 Phase 1: Backend Stability Hardening

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate crash risks in production by replacing hot-path `unwrap()` calls, adding API input validation, and hardening critical execution paths.

**Architecture:** Three parallel tracks — (A) unwrap elimination across 8 crates prioritized by crash risk, (B) API input validation for all mutating endpoints, (C) critical path robustness improvements. Each track produces independent, mergeable commits.

**Tech Stack:** Rust, Axum, redb, thiserror, tokio

**Spec:** `docs/superpowers/specs/2026-04-26-v0.7.0-release-plan-design.md` Part 1

---

## Track A: Hot-Path `unwrap()` Elimination

### Task A1: Agent Executor Unwrap Hardening (Critical)

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor/mod.rs`
- Modify: `crates/neomind-agent/src/ai_agent/executor/agent_loop.rs` (if exists)
- Modify: `crates/neomind-agent/src/error.rs`

**Context:** The agent executor is the hottest path — it runs on every LLM call, tool execution, and response cycle. 343 `unwrap()` calls in `neomind-agent`, with executor being the single highest-risk module.

- [ ] **Step 1: Audit executor unwraps**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && grep -rn '\.unwrap()' crates/neomind-agent/src/ai_agent/executor/ | grep -v '#\[cfg(test)\]' | grep -v 'mod tests'`

Record the count and categorize each as:
- **RwLock/Mutex**: `read().unwrap()`, `write().unwrap()`, `lock().unwrap()`
- **Option**: `some_field.unwrap()`
- **Result**: `operation().unwrap()`

- [ ] **Step 2: Replace RwLock unwraps with `unwrap_or_else`**

Pattern to apply for all `RwLock`/`Mutex` accesses:

```rust
// BEFORE (crashes on poisoned lock)
let registry = self.tool_registry.read().unwrap();

// AFTER (recovers from poisoned lock)
let registry = self.tool_registry.read().unwrap_or_else(|e| {
    tracing::error!("Tool registry lock poisoned, recovering: {}", e);
    e.into_inner()
});
```

Apply to every `self.xxx.read().unwrap()` and `self.xxx.write().unwrap()` in executor files.

- [ ] **Step 3: Replace Option unwraps with safe access**

```rust
// BEFORE
let config = self.config.unwrap();

// AFTER
let config = self.config.as_ref().ok_or_else(|| {
    AgentError::config_err("Agent config not initialized")
})?;
```

For fields that are guaranteed to exist after init, use `expect()` with a clear message instead of bare `unwrap()`.

- [ ] **Step 4: Replace Result unwraps with `?` or `map_err`**

```rust
// BEFORE
let response = llm_call.await.unwrap();

// AFTER
let response = llm_call.await.map_err(|e| {
    tracing::error!("LLM call failed: {}", e);
    NeoMindError::llm_err(format!("LLM call failed: {}", e))
})?;
```

- [ ] **Step 5: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-agent && cargo test -p neomind-agent --lib 2>&1 | tail -20`

Expected: All tests pass, no compilation errors.

- [ ] **Step 6: Commit**

```bash
git add crates/neomind-agent/src/
git commit -m "fix(agent): replace hot-path unwrap() in executor with safe error handling"
```

---

### Task A2: Agent LLM Backends Unwrap Hardening (Critical)

**Files:**
- Modify: `crates/neomind-agent/src/llm_backends/*.rs`

**Context:** LLM backend code handles external API calls (Ollama, OpenAI-compatible). Failures here should return errors, not panic.

- [ ] **Step 1: Audit llm_backends unwraps**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && grep -rn '\.unwrap()' crates/neomind-agent/src/llm_backends/ | grep -v test`

- [ ] **Step 2: Replace JSON parsing unwraps**

```rust
// BEFORE
let content = response["choices"][0]["message"]["content"].as_str().unwrap();

// AFTER
let content = response["choices"][0]["message"]["content"].as_str().ok_or_else(|| {
    NeoMindError::llm_err("Invalid LLM response: missing content field")
})?;
```

- [ ] **Step 3: Replace HTTP response unwraps**

All `reqwest` response handling should use `?` with proper error conversion.

- [ ] **Step 4: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-agent && cargo test -p neomind-agent --lib 2>&1 | tail -20`

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-agent/src/llm_backends/
git commit -m "fix(agent): harden LLM backend response parsing against malformed responses"
```

---

### Task A3: Agent Tools Unwrap Hardening (Critical)

**Files:**
- Modify: `crates/neomind-agent/src/tools/*.rs`

**Context:** Tool execution handles device control, metric reads, shell commands — all external I/O that can fail unexpectedly.

- [ ] **Step 1: Audit tool unwraps**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && grep -rn '\.unwrap()' crates/neomind-agent/src/tools/ | grep -v test`

- [ ] **Step 2: Replace unwraps in each tool file**

For each tool, wrap execution in a catch-all:
```rust
// For tool execute methods, ensure no unwrap in the hot path
async fn execute(&self, args: Value) -> Result<ToolOutput, ToolError> {
    // Use ? propagation throughout
    let param = args["name"].as_str().ok_or_else(|| {
        ToolError::InvalidParam("name is required")
    })?;
    // ...
}
```

- [ ] **Step 3: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-agent && cargo test -p neomind-agent --lib 2>&1 | tail -20`

- [ ] **Step 4: Commit**

```bash
git add crates/neomind-agent/src/tools/
git commit -m "fix(agent): harden tool execution against unwrap panics"
```

---

### Task A4: Storage Timeseries Unwrap Hardening (Critical)

**Files:**
- Modify: `crates/neomind-storage/src/timeseries.rs`
- Modify: `crates/neomind-storage/src/error.rs`

**Context:** 303 `unwrap()` calls in `neomind-storage`. The timeseries module handles all telemetry data — crashes here lose data. Uses `redb` with `TableDefinition` and composite keys `(source_id, metric, timestamp)`.

- [ ] **Step 1: Audit timeseries unwraps**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && grep -rn '\.unwrap()' crates/neomind-storage/src/timeseries.rs | grep -v test | head -40`

- [ ] **Step 2: Replace redb transaction unwraps**

```rust
// BEFORE
let table = db.open_table(TIMESERIES_TABLE).unwrap();

// AFTER
let table = db.open_table(TIMESERIES_TABLE).map_err(|e| {
    Error::Storage(format!("Failed to open timeseries table: {}", e))
})?;
```

- [ ] **Step 3: Replace RwLock unwraps**

All `self.inner.read().unwrap()` / `self.inner.write().unwrap()` → use `unwrap_or_else` with poison recovery.

- [ ] **Step 4: Replace JSON serialization unwraps**

```rust
// BEFORE
let bytes = serde_json::to_vec(&point).unwrap();

// AFTER
let bytes = serde_json::to_vec(&point).map_err(|e| {
    Error::Serialization(e.to_string())
})?;
```

- [ ] **Step 5: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-storage && cargo test -p neomind-storage --lib 2>&1 | tail -20`

- [ ] **Step 6: Commit**

```bash
git add crates/neomind-storage/src/
git commit -m "fix(storage): replace hot-path unwrap() in timeseries with proper error handling"
```

---

### Task A5: Core EventBus Unwrap Hardening (Critical)

**Files:**
- Modify: `crates/neomind-core/src/eventbus.rs`
- Modify: `crates/neomind-core/src/extension/isolated/*.rs`

**Context:** 81 `unwrap()` in `neomind-core`. EventBus is the central event dispatch — every device metric, extension output, and agent trigger flows through it. The `eventbus.rs` module alone has 21 unwraps.

- [ ] **Step 1: Audit eventbus unwraps**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && grep -rn '\.unwrap()' crates/neomind-core/src/eventbus.rs | grep -v test`

- [ ] **Step 2: Replace channel send/receive unwraps**

```rust
// BEFORE
self.sender.send(event).unwrap();

// AFTER
if let Err(e) = self.sender.send(event) {
    tracing::warn!("EventBus: failed to send event (receiver dropped): {}", e);
    // Don't panic — event bus is advisory, not critical
}
```

- [ ] **Step 3: Replace HashMap/RwLock unwraps in event handlers**

- [ ] **Step 4: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-core && cargo test -p neomind-core --lib 2>&1 | tail -20`

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-core/src/
git commit -m "fix(core): harden EventBus against unwrap panics in event dispatch"
```

---

### Task A6: API Handlers Unwrap Hardening (High)

**Files:**
- Modify: `crates/neomind-api/src/handlers/*.rs`
- Modify: `crates/neomind-api/src/handlers/**/*.rs`

**Context:** 129 `unwrap()` calls in `neomind-api`. Handlers are the user-facing surface — panics here return 500 errors. Handlers use `ErrorResponse` with variants: `bad_request`, `unauthorized`, `not_found`, `conflict`, `validation`, `internal`.

- [ ] **Step 1: Audit handler unwraps**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && grep -rn '\.unwrap()' crates/neomind-api/src/handlers/ | grep -v test | head -50`

- [ ] **Step 2: Replace unwraps with ErrorResponse returns**

```rust
// BEFORE
let agent = state.agent_manager.get(&id).unwrap();

// AFTER
let agent = state.agent_manager.get(&id).ok_or_else(|| {
    ErrorResponse::not_found(&format!("Agent '{}' not found", id))
})?;
```

- [ ] **Step 3: Replace state access unwraps**

All `State(state).xxx.read().unwrap()` patterns → `unwrap_or_else` with poison recovery.

- [ ] **Step 4: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-api && cargo test -p neomind-api --lib 2>&1 | tail -20`

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-api/src/handlers/
git commit -m "fix(api): replace handler unwrap() with proper ErrorResponse returns"
```

---

### Task A7: Rules Engine Unwrap Hardening (High)

**Files:**
- Modify: `crates/neomind-rules/src/engine.rs`
- Modify: `crates/neomind-rules/src/dsl.rs`

**Context:** 93 `unwrap()` calls. The rule engine scheduler runs periodically evaluating conditions against device metrics — a crash here stops all automated rule processing.

- [ ] **Step 1: Audit engine unwraps**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && grep -rn '\.unwrap()' crates/neomind-rules/src/engine.rs | grep -v test`

- [ ] **Step 2: Replace scheduler RwLock unwraps**

```rust
// BEFORE (engine.rs:421)
let mut rule_store = self.rule_store.write().unwrap();

// AFTER
let mut rule_store = self.rule_store.write().unwrap_or_else(|e| {
    tracing::error!("Rule store lock poisoned, recovering: {}", e);
    e.into_inner()
});
```

- [ ] **Step 3: Replace DSL parser unwraps**

Ensure parser returns `Result<Rule, RuleError>` instead of panicking on malformed input.

- [ ] **Step 4: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-rules && cargo test -p neomind-rules --lib 2>&1 | tail -20`

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-rules/src/
git commit -m "fix(rules): harden rule engine and DSL parser against unwrap panics"
```

---

### Task A8: Devices MQTT Unwrap Hardening (High)

**Files:**
- Modify: `crates/neomind-devices/src/mqtt/*.rs`
- Modify: `crates/neomind-devices/src/telemetry/*.rs`

**Context:** 58 `unwrap()` calls. MQTT module handles real-time device connections — crashes disconnect devices and lose telemetry data.

- [ ] **Step 1: Audit MQTT unwraps**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && grep -rn '\.unwrap()' crates/neomind-devices/src/mqtt/ crates/neomind-devices/src/telemetry/ | grep -v test`

- [ ] **Step 2: Replace all hot-path unwraps following the established patterns**

- [ ] **Step 3: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-devices && cargo test -p neomind-devices --lib 2>&1 | tail -20`

- [ ] **Step 4: Commit**

```bash
git add crates/neomind-devices/src/
git commit -m "fix(devices): harden MQTT and telemetry against unwrap panics"
```

---

### Task A9: Messages + Extension Runner Unwrap Hardening (Standard)

**Files:**
- Modify: `crates/neomind-messages/src/channels/*.rs`
- Modify: `crates/neomind-messages/src/manager.rs`
- Modify: `crates/neomind-messages/src/delivery_log.rs`
- Modify: `crates/neomind-extension-runner/src/main.rs`
- Modify: `crates/neomind-extension-runner/src/ipc_routing.rs`

**Context:** 20 unwraps in messages, 14 in extension-runner. Lower volume but notification delivery and IPC crashes affect reliability.

- [ ] **Step 1: Audit and fix all remaining hot-path unwraps**

- [ ] **Step 2: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-messages -p neomind-extension-runner && cargo test -p neomind-messages -p neomind-extension-runner --lib 2>&1 | tail -20`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-messages/src/ crates/neomind-extension-runner/src/
git commit -m "fix(messages,extension-runner): replace hot-path unwrap with safe error handling"
```

---

### Task A10: Verify Zero Hot-Path Unwraps

- [ ] **Step 1: Run unwrap audit across all 8 crates**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && for crate in neomind-agent neomind-storage neomind-api neomind-rules neomind-devices neomind-core neomind-messages neomind-extension-runner; do echo "=== $crate ==="; grep -rn '\.unwrap()' crates/$crate/src/ | grep -v test | grep -v '_test.rs' | wc -l; done`

Compare with baseline counts from spec. Verify significant reduction in hot-path modules.

- [ ] **Step 2: Run full cargo build + test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build && cargo test --workspace --lib 2>&1 | tail -30`

Expected: All workspace tests pass.

- [ ] **Step 3: Commit any final fixes**

```bash
git commit -m "chore: final unwrap audit and cleanup for v0.7.0 stability hardening"
```

---

## Track B: API Input Validation

### Task B1: Create Validation Helper Module

**Files:**
- Create: `crates/neomind-api/src/handlers/validation.rs`
- Modify: `crates/neomind-api/src/handlers/mod.rs`

- [ ] **Step 1: Create validation module with reusable helpers**

```rust
// crates/neomind-api/src/handlers/validation.rs
use crate::models::error::ErrorResponse;

pub struct Validator;

impl Validator {
    pub fn required_string(value: &str, field: &str) -> Result<(), ErrorResponse> {
        if value.trim().is_empty() {
            return Err(ErrorResponse::validation(&format!("{} is required", field)));
        }
        Ok(())
    }

    pub fn string_length(value: &str, field: &str, min: usize, max: usize) -> Result<(), ErrorResponse> {
        let len = value.trim().len();
        if len < min || len > max {
            return Err(ErrorResponse::validation(&format!(
                "{} must be between {} and {} characters", field, min, max
            )));
        }
        Ok(())
    }

    pub fn numeric_range(value: f64, field: &str, min: f64, max: f64) -> Result<(), ErrorResponse> {
        if value < min || value > max {
            return Err(ErrorResponse::validation(&format!(
                "{} must be between {} and {}", field, min, max
            )));
        }
        Ok(())
    }

    pub fn identifier(value: &str, field: &str) -> Result<(), ErrorResponse> {
        Self::required_string(value, field)?;
        if !value.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ':') {
            return Err(ErrorResponse::validation(&format!(
                "{} contains invalid characters (only alphanumeric, underscore, hyphen, colon allowed)", field
            )));
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Register module in mod.rs**

```rust
pub mod validation;
```

- [ ] **Step 3: Build and verify**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-api 2>&1 | tail -10`

- [ ] **Step 4: Commit**

```bash
git add crates/neomind-api/src/handlers/validation.rs crates/neomind-api/src/handlers/mod.rs
git commit -m "feat(api): add reusable validation helper module"
```

---

### Task B2: Validate Agent CRUD Endpoints

**Files:**
- Modify: `crates/neomind-api/src/handlers/agents.rs` (or `agents/*.rs`)

- [ ] **Step 1: Add validation to agent create handler**

```rust
use super::validation::Validator;

pub async fn create_agent(
    State(state): State<ServerState>,
    Json(req): Json<CreateAgentRequest>,
) -> HandlerResult<Agent> {
    Validator::required_string(&req.name, "name")?;
    Validator::string_length(&req.name, "name", 1, 100)?;

    if req.mode == "focused" && req.resources.is_empty() {
        return Err(ErrorResponse::validation("Focused mode requires at least one resource binding"));
    }
    // ... existing logic
}
```

- [ ] **Step 2: Add validation to agent update handler**

Validate name, mode transitions, resource bindings.

- [ ] **Step 3: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-api && cargo test -p neomind-api --lib 2>&1 | tail -20`

- [ ] **Step 4: Commit**

```bash
git add crates/neomind-api/src/handlers/agents/
git commit -m "feat(api): add input validation to agent CRUD endpoints"
```

---

### Task B3: Validate Device + Extension + Rule + MQTT + Telemetry Endpoints

**Files:**
- Modify: `crates/neomind-api/src/handlers/devices/mdl.rs`
- Modify: `crates/neomind-api/src/handlers/extensions.rs` (or `extensions/*.rs`)
- Modify: `crates/neomind-api/src/handlers/rules.rs`
- Modify: `crates/neomind-api/src/handlers/mqtt/subscriptions.rs`
- Modify: `crates/neomind-api/src/handlers/telemetry.rs` (or data handlers)

- [ ] **Step 1: Add validation to device handlers**

Validate `device_id` format, `device_type`, and config fields.

- [ ] **Step 2: Add validation to extension upload handler**

Validate file size (max 50MB), extension name format, metadata presence.

- [ ] **Step 3: Add validation to rule handlers**

Validate condition syntax (basic check), action list non-empty, rule name.

- [ ] **Step 4: Add validation to MQTT subscription handler**

Validate topic format (non-empty, valid MQTT topic pattern), QoS range (0-2).

- [ ] **Step 5: Add validation to telemetry query handler**

Validate time range (max 30 days), metric name format, limit bounds.

- [ ] **Step 6: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-api && cargo test -p neomind-api --lib 2>&1 | tail -20`

- [ ] **Step 7: Commit**

```bash
git add crates/neomind-api/src/handlers/
git commit -m "feat(api): add input validation to device, extension, rule, MQTT, and telemetry endpoints"
```

---

## Track C: Critical Path Robustness

### Task C1: Implement Settings Persistent Storage (TODO #1)

**Files:**
- Modify: `crates/neomind-api/src/handlers/setup.rs`

**Context:** Line 268 has `// TODO: Save to persistent settings storage`. Settings currently don't survive restarts.

- [ ] **Step 1: Read current setup handler code**

Read the file around line 268 to understand the current settings structure and what needs to be persisted.

- [ ] **Step 2: Implement redb-backed settings store**

Create a settings table in redb and persist key-value settings on update. Load on startup.

```rust
// Use existing redb infrastructure
const SETTINGS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("settings");

pub fn save_setting(db: &Database, key: &str, value: &Value) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(SETTINGS_TABLE)?;
        let bytes = serde_json::to_vec(value)?;
        table.insert(key, bytes.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}
```

- [ ] **Step 3: Wire into setup handler**

Replace the TODO with actual persistence call.

- [ ] **Step 4: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-api && cargo test -p neomind-api --lib 2>&1 | tail -20`

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-api/src/handlers/setup.rs
git commit -m "feat(api): implement persistent settings storage with redb backend"
```

---

### Task C2: Implement MQTT Custom Topic Unsubscription (TODO #2)

**Files:**
- Modify: `crates/neomind-api/src/handlers/mqtt/subscriptions.rs`

**Context:** Line 82 returns `success: false` with "not yet implemented" message.

- [ ] **Step 1: Read current subscription code**

Understand how subscriptions are tracked (likely a HashMap or set).

- [ ] **Step 2: Implement unsubscribe logic**

```rust
pub async fn unsubscribe_custom_topic(topic: &str) -> Result<()> {
    // Remove from subscription tracking
    // Send UNSUBSCRIBE to MQTT broker
    // Return success/failure
}
```

- [ ] **Step 3: Wire into handler**

Replace the stub with actual implementation.

- [ ] **Step 4: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-api && cargo test -p neomind-api --lib 2>&1 | tail -20`

- [ ] **Step 5: Commit**

```bash
git add crates/neomind-api/src/handlers/mqtt/subscriptions.rs
git commit -m "feat(mqtt): implement custom topic unsubscription"
```

---

### Task C3: Rule Engine Error Recovery

**Files:**
- Modify: `crates/neomind-rules/src/engine.rs`

- [ ] **Step 1: Wrap condition evaluation in catch-all**

```rust
// In the scheduler loop, wrap each rule evaluation
let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    self.evaluate_condition(&rule.condition, &values)
}));

match result {
    Ok(Ok(true)) => { /* execute actions */ }
    Ok(Ok(false)) => { /* condition not met */ }
    Ok(Err(e)) => {
        tracing::error!("Rule '{}' evaluation error: {}", rule.name, e);
        // Continue to next rule, don't crash scheduler
    }
    Err(panic_payload) => {
        tracing::error!("Rule '{}' panicked during evaluation", rule.name);
        // Continue to next rule
    }
}
```

- [ ] **Step 2: Build and test**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build -p neomind-rules && cargo test -p neomind-rules --lib 2>&1 | tail -20`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-rules/src/engine.rs
git commit -m "fix(rules): add catch-all error recovery in rule evaluation scheduler"
```

---

### Task C4: Full Workspace Build Verification

- [ ] **Step 1: Clean build all crates**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo build --workspace 2>&1 | tail -20`

- [ ] **Step 2: Run all workspace tests**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo test --workspace --lib 2>&1 | tail -30`

- [ ] **Step 3: Run cargo clippy**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo clippy --workspace 2>&1 | grep "warning:" | head -20`

Fix any new warnings introduced by the changes.

- [ ] **Step 4: Final commit**

```bash
git commit -m "chore: Phase 1 complete — backend stability hardening for v0.7.0"
```

---

## Completion Checklist

- [ ] All hot-path `unwrap()` calls in 8 crates audited and replaced
- [ ] API validation module created and applied to all POST/PUT endpoints
- [ ] Settings persistent storage implemented (TODO #1)
- [ ] MQTT custom topic unsubscription implemented (TODO #2)
- [ ] Rule engine error recovery in place
- [ ] `cargo build --workspace` passes
- [ ] `cargo test --workspace --lib` passes
- [ ] `cargo clippy --workspace` has no new warnings
