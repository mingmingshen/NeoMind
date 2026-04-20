# Agent Execution Mode Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign AI Agent Chat/React modes into Focused/Free modes with clear differentiation in tools, frontend form, API validation, and data collection.

**Architecture:** Rename the `ExecutionMode` enum variants with serde aliases for backward compatibility. Enhance Focused Mode's prompt with structured data tables and command templates. Add scope validation to command execution. Differentiate the frontend form by mode.

**Tech Stack:** Rust (serde enum alias), React/TypeScript (form conditional rendering), Axum (API validation)

**Spec:** `docs/superpowers/specs/2026-04-20-agent-execution-mode-redesign.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/neomind-storage/src/agents.rs` | Modify | Rename `ExecutionMode` enum variants with serde aliases |
| `crates/neomind-agent/src/ai_agent/executor/mod.rs` | Modify | Update `should_use_tools()` to use `Focused` variant |
| `crates/neomind-agent/src/ai_agent/executor/analyzer.rs` | Modify | Structured table prompt, decision templates for Focused Mode |
| `crates/neomind-agent/src/ai_agent/executor/command_executor.rs` | Modify | Scope validation + enhanced fuzzy matching |
| `crates/neomind-agent/src/toolkit/simplified.rs` | Modify | Update LLM tool descriptions for mode/resources/data_collection |
| `crates/neomind-api/src/handlers/agents.rs` | Modify | Mode name mapping + Focused requires resources validation |
| `web/src/types/index.ts` | Modify | Update `execution_mode` type |
| `web/src/pages/agents-components/AgentEditorFullScreen.tsx` | Modify | Mode cards, form visibility, data collection config, validation |
| `web/src/i18n/locales/en/agents.json` | Modify | New i18n keys |
| `web/src/i18n/locales/zh/agents.json` | Modify | New i18n keys |
| `crates/neomind-storage/src/agents.rs` (tests) | Modify | Update test references |
| `crates/neomind-agent/tests/*.rs` | Modify | Update test references |

---

### Task 1: Rename ExecutionMode Enum (Backend Core)

**Files:**
- Modify: `crates/neomind-storage/src/agents.rs:229-235` (enum definition)
- Modify: `crates/neomind-storage/src/agents.rs:1922,1967,2045,2113` (test references)
- Modify: `crates/neomind-agent/src/ai_agent/executor/mod.rs:309-337` (`should_use_tools`)
- Modify: `crates/neomind-api/src/handlers/agents.rs:985-988,1210-1215` (mode parsing)

- [ ] **Step 1: Rename enum variants with serde aliases**

In `crates/neomind-storage/src/agents.rs`, replace lines 226-235:

```rust
/// Agent execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Focused mode — user-defined scope, single-pass analysis with bound resources
    #[default]
    #[serde(rename = "focused", alias = "chat")]
    Focused,
    /// Free mode — LLM freely explores with full tool access, multi-round reasoning
    #[serde(rename = "free", alias = "react")]
    Free,
}
```

- [ ] **Step 2: Update `should_use_tools()` in executor**

In `crates/neomind-agent/src/ai_agent/executor/mod.rs`, update line 317:

```rust
// Before:
if agent.execution_mode == ExecutionMode::Chat {
// After:
if agent.execution_mode == ExecutionMode::Focused {
```

And update the tracing message on line 320:
```rust
// Before:
"Execution mode is Chat - using direct LLM analysis"
// After:
"Execution mode is Focused - using direct LLM analysis within bound resources"
```

- [ ] **Step 3: Update API mode parsing**

In `crates/neomind-api/src/handlers/agents.rs`, update create handler (line ~985-988):

```rust
// Before:
execution_mode: match request.execution_mode.as_deref() {
    Some("react") => neomind_storage::agents::ExecutionMode::React,
    _ => neomind_storage::agents::ExecutionMode::Chat,
},
// After:
execution_mode: match request.execution_mode.as_deref() {
    Some("free") | Some("react") => neomind_storage::agents::ExecutionMode::Free,
    _ => neomind_storage::agents::ExecutionMode::Focused,
},
```

And update handler (line ~1210-1215):

```rust
// Before:
if let Some(mode) = request.execution_mode {
    agent.execution_mode = match mode.as_str() {
        "react" => neomind_storage::agents::ExecutionMode::React,
        _ => neomind_storage::agents::ExecutionMode::Chat,
    };
}
// After:
if let Some(mode) = request.execution_mode {
    agent.execution_mode = match mode.as_str() {
        "free" | "react" => neomind_storage::agents::ExecutionMode::Free,
        _ => neomind_storage::agents::ExecutionMode::Focused,
    };
}
```

- [ ] **Step 4: Update test references**

In all test files, replace `ExecutionMode::Chat` with `ExecutionMode::Focused`:
- `crates/neomind-storage/src/agents.rs`: lines 1922, 1967, 2045, 2113
- `crates/neomind-agent/tests/llm_integration_test.rs`: line 111
- `crates/neomind-agent/tests/realistic_performance_test.rs`: lines 156, 381
- `crates/neomind-agent/tests/conversation_integration.rs`: line 90
- `crates/neomind-agent/tests/real_world_simulation_test.rs`: line 201
- `crates/neomind-agent/tests/load_test.rs`: line 234
- `crates/neomind-agent/tests/full_integration_test.rs`: lines 203, 288

- [ ] **Step 5: Build and test**

Run: `cargo build && cargo test`
Expected: All existing tests pass with new enum names

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "refactor: rename ExecutionMode Chat/React to Focused/Free with backward compat aliases"
```

---

### Task 2: Add API Validation — Focused Mode Requires Resources

**Files:**
- Modify: `crates/neomind-api/src/handlers/agents.rs:~966` (create handler, after resources built)
- Modify: `crates/neomind-api/src/handlers/agents.rs:~1193` (update handler, after resources built)

- [ ] **Step 1: Add validation in create agent handler**

In `crates/neomind-api/src/handlers/agents.rs`, after the `resources` Vec is built (around line 966), before the agent is constructed:

```rust
// Validate: Focused mode requires at least one resource
let execution_mode = match request.execution_mode.as_deref() {
    Some("free") | Some("react") => neomind_storage::agents::ExecutionMode::Free,
    _ => neomind_storage::agents::ExecutionMode::Focused,
};
if execution_mode == neomind_storage::agents::ExecutionMode::Focused && resources.is_empty() {
    return Err(ErrorResponse::with_message(
        "Focused mode requires at least one resource binding".to_string(),
    ));
}
```

Then use `execution_mode` variable in the agent construction instead of the inline match.

- [ ] **Step 2: Add validation in update agent handler**

In the update handler (around line 1193), after resources are rebuilt:

```rust
// Validate: Focused mode requires at least one resource
if has_resources_update {
    let new_mode = match request.execution_mode.as_deref() {
        Some("free") | Some("react") => neomind_storage::agents::ExecutionMode::Free,
        None => agent.execution_mode, // keep existing
        _ => neomind_storage::agents::ExecutionMode::Focused,
    };
    if new_mode == neomind_storage::agents::ExecutionMode::Focused && resources.is_empty() {
        return Err(ErrorResponse::with_message(
            "Focused mode requires at least one resource binding".to_string(),
        ));
    }
}
```

- [ ] **Step 3: Build and test**

Run: `cargo build -p neomind-api && cargo test -p neomind-api`
Expected: Builds, existing tests pass

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(api): validate Focused mode requires resource binding on create/update"
```

---

### Task 3: Update LLM Tool Descriptions

**Files:**
- Modify: `crates/neomind-agent/src/toolkit/simplified.rs:385-402`

- [ ] **Step 1: Update execution_mode parameter description**

Replace lines 385-389:

```rust
("execution_mode".to_string(), ParameterInfo {
    description: "Agent mode (create): 'focused' = must bind resources, LLM works within defined scope. Fast, precise, token-efficient. Best for monitoring, alerts, data analysis. 'free' = LLM freely explores with all tools, multi-round reasoning. Best for complex automation and device control.".to_string(),
    default: serde_json::json!("focused"),
    examples: vec!["focused".to_string(), "free".to_string()],
}),
```

- [ ] **Step 2: Update resources parameter description**

Replace lines 390-397:

```rust
("resources".to_string(), ParameterInfo {
    description: "Resources to bind (create, multi-select). REQUIRED for focused mode (at least 1), optional for free mode. JSON array: [{\"type\":\"...\",\"id\":\"...\",\"config\":{...}}]. Types: 'device', 'metric' (id='device_id:metric'), 'command' (id='device_id:cmd'), 'extension_metric' (id='extension:ext_id:metric'), 'extension_tool' (id='extension:ext_id:tool'). Focused mode: these define the exact scope. Free mode: recommended focus areas. For metrics in focused mode, config.data_collection controls pre-collection: {\"data_collection\":{\"time_range_minutes\":60,\"include_history\":false,\"include_trend\":false}}".to_string(),
    default: serde_json::json!(null),
    examples: vec![
        "[{\"type\":\"metric\",\"id\":\"sensor_001:temperature\"}]".to_string(),
        "[{\"type\":\"device\",\"id\":\"camera_001\"},{\"type\":\"extension_tool\",\"id\":\"extension:image_analyzer:detect\"}]".to_string(),
        "[{\"type\":\"metric\",\"id\":\"sensor_001:temperature\",\"config\":{\"data_collection\":{\"time_range_minutes\":360}}}]".to_string(),
    ],
}),
```

- [ ] **Step 3: Update enable_tool_chaining description**

Replace line 399 (the `enable_tool_chaining` description):

```rust
description: "Allow tool output chaining in free mode (create, optional). Default: false. Set true for complex automation. Only applies to free mode.".to_string(),
```

- [ ] **Step 4: Build and test**

Run: `cargo build -p neomind-agent && cargo test -p neomind-agent --lib`
Expected: Builds, tests pass

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(tools): update agent tool descriptions for Focused/Free modes and data_collection"
```

---

### Task 4: Improve Focused Mode Prompt (Structured Tables + Templates)

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor/analyzer.rs:25-173` (command/data source builders)
- Modify: `crates/neomind-agent/src/ai_agent/executor/analyzer.rs:834-846` (resources_info assembly)

- [ ] **Step 1: Add `build_focused_data_table()` method**

Insert a new method in `analyzer.rs` after `build_available_data_sources_description()` (after line ~343):

```rust
/// Build structured data table for Focused Mode prompt.
/// Returns markdown tables of current data and available commands.
pub(crate) fn build_focused_data_table(
    agent: &AiAgent,
    data: &[DataCollected],
) -> String {
    let mut sections = Vec::new();

    // --- Current Data Table ---
    let data_entries: Vec<&DataCollected> = data.iter()
        .filter(|d| {
            d.source != "system"
            && !d.values.get("_is_image").and_then(|v| v.as_bool()).unwrap_or(false)
        })
        .take(15)
        .collect();

    if !data_entries.is_empty() {
        sections.push("## Current Data (live from bound resources)".to_string());
        sections.push("| Resource | Type | Value |".to_string());
        sections.push("|----------|------|-------|".to_string());
        for d in &data_entries {
            let value = if let Some(v) = d.values.get("value") {
                format!("{}", v)
            } else {
                let json_str = serde_json::to_string(&d.values).unwrap_or_default();
                if json_str.len() > 100 { json_str[..100].to_string() + "..." } else { json_str }
            };
            sections.push(format!("| {} | {} | {} |", d.source, d.data_type, value));
        }
    }

    // --- Available Commands Table ---
    let commands: Vec<&AgentResource> = agent.resources.iter()
        .filter(|r| matches!(r.resource_type, ResourceType::Command | ResourceType::ExtensionTool))
        .collect();

    if !commands.is_empty() {
        sections.push(String::new());
        sections.push("## Available Commands (only execute when needed)".to_string());
        sections.push("| Name | Action Value |".to_string());
        sections.push("|------|-------------|".to_string());
        for cmd in &commands {
            let display_name = if cmd.name.is_empty() { &cmd.resource_id } else { &cmd.name };
            sections.push(format!("| {} | `{}` |", display_name, cmd.resource_id));
        }
    }

    // --- Decision Template ---
    if !commands.is_empty() {
        sections.push(String::new());
        sections.push("### Decision Format".to_string());
        sections.push("If you need to execute a command:".to_string());
        sections.push("`\"decisions\": [{\"decision_type\": \"command\", \"action\": \"<copy Action Value>\", \"description\": \"<reason>\"}]`".to_string());
        sections.push("If no action needed: `\"decisions\": []`".to_string());
    }

    sections.join("\n")
}
```

- [ ] **Step 2: Use `build_focused_data_table` when mode is Focused**

In `analyze_with_llm()`, around line 834-846, replace the `resources_info` assembly:

```rust
// Before:
let available_commands = Self::build_available_commands_description(agent);
let available_data_sources = Self::build_available_data_sources_description(agent);
let resources_info = if available_data_sources.is_empty() {
    available_commands
} else {
    format!("{}\n\n{}", available_commands, available_data_sources)
};

// After:
let resources_info = match agent.execution_mode {
    neomind_storage::agents::ExecutionMode::Focused => {
        Self::build_focused_data_table(agent, data)
    }
    neomind_storage::agents::ExecutionMode::Free => {
        let commands = Self::build_available_commands_description(agent);
        let data_sources = Self::build_available_data_sources_description(agent);
        if data_sources.is_empty() { commands } else { format!("{}\n\n{}", commands, data_sources) }
    }
};
```

Note: `analyze_with_llm` already has `data: &[DataCollected]` parameter (confirmed at line 456). No signature change needed.

- [ ] **Step 3: Build and test**

Run: `cargo build -p neomind-agent && cargo test -p neomind-agent --lib`
Expected: Builds, tests pass

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(agent): add structured data table and decision template for Focused Mode prompt"
```

---

### Task 5: Add Scope Validation to Command Execution

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor/command_executor.rs:482-530` (`execute_decisions`)

- [ ] **Step 1: Add scope validation in `execute_decisions()`**

At the top of the `for decision in decisions` loop (after line 493), add scope check:

```rust
pub(crate) async fn execute_decisions(
    &self,
    agent: &AiAgent,
    decisions: &[Decision],
) -> AgentResult<(Vec<ActionExecuted>, Vec<NotificationSent>)> {
    let mut actions_executed = Vec::new();
    let mut notifications_sent = Vec::new();

    // Pre-build allowed actions for Focused mode scope validation
    let allowed_command_ids: Vec<String> = if agent.execution_mode == neomind_storage::agents::ExecutionMode::Focused {
        agent.resources.iter()
            .filter(|r| matches!(r.resource_type, neomind_storage::ResourceType::Command | neomind_storage::ResourceType::ExtensionTool))
            .map(|r| r.resource_id.clone())
            .collect()
    } else {
        Vec::new() // Empty = no scope restriction for Free mode
    };

    for decision in decisions {
        // Scope validation for Focused mode
        if !allowed_command_ids.is_empty() && decision.decision_type == "command" {
            let action = &decision.action;
            let is_allowed = allowed_command_ids.iter().any(|rid| {
                // 1. Exact match (most reliable)
                if action == rid { return true; }
                // 2. Suffix match: action "turn_on" matches rid "light_living:turn_on"
                if let Some(cmd_suffix) = rid.split(':').last() {
                    if action == cmd_suffix { return true; }
                    if action.ends_with(&format!(":{}", cmd_suffix)) { return true; }
                }
                // 3. Extension tool format: action contains the full rid
                if action.contains(rid.as_str()) { return true; }
                false
            });

            if !is_allowed {
                tracing::warn!(
                    action = %decision.action,
                    allowed = ?allowed_command_ids,
                    "Focused Mode: rejecting out-of-scope command"
                );
                actions_executed.push(neomind_storage::ActionExecuted {
                    action: decision.action.clone(),
                    target: String::new(),
                    success: false,
                    result: Some("Rejected: command not in bound resources".to_string()),
                    timestamp: chrono::Utc::now().timestamp(),
                });
                continue;
            }
        }

        // ... existing decision handling (query, command, alert, etc.) remains unchanged
```

- [ ] **Step 2: Build and test**

Run: `cargo build -p neomind-agent && cargo test -p neomind-agent --lib`
Expected: Builds, tests pass

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat(agent): add Focused Mode scope validation to command execution"
```

---

### Task 6: Frontend — Update Types and i18n

**Files:**
- Modify: `web/src/types/index.ts:1997,2267` (execution_mode type)
- Modify: `web/src/i18n/locales/en/agents.json`
- Modify: `web/src/i18n/locales/zh/agents.json`

- [ ] **Step 1: Update TypeScript types**

In `web/src/types/index.ts`, find `execution_mode?: 'chat' | 'react'` (appears at ~lines 1997 and 2267) and replace:

```typescript
execution_mode?: 'focused' | 'free' | 'chat' | 'react'
```

- [ ] **Step 2: Add i18n keys**

Add to `web/src/i18n/locales/en/agents.json`:
```json
"focusedMode": "Focused Mode",
"focusedModeDescription": "Bind specific resources and actions for fast, precise analysis. Best for monitoring, alerts, data analysis.",
"freeMode": "Free Mode",
"freeModeDescription": "LLM freely explores and decides with multi-round tool calling. Best for complex automation and device control.",
"focusedModeRequiresResources": "Focused Mode requires at least one resource binding",
"saveToken": "Save Tokens",
"dataCollection": "Data Collection",
"timeRange": "Time Range",
"includeHistory": "Include History",
"includeTrend": "Include Trend",
"includeBaseline": "Include Baseline"
```

Add to `web/src/i18n/locales/zh/agents.json`:
```json
"focusedMode": "精准模式",
"focusedModeDescription": "绑定特定资源和操作，快速精准分析。适合监控、告警、数据分析。",
"freeMode": "自由模式",
"freeModeDescription": "LLM 自主探索和决策，支持多轮工具调用。适合复杂自动化和设备控制。",
"focusedModeRequiresResources": "精准模式需要至少绑定一个资源",
"saveToken": "省 Token",
"dataCollection": "数据采集",
"timeRange": "时间范围",
"includeHistory": "包含历史数据",
"includeTrend": "包含趋势分析",
"includeBaseline": "包含基线对比"
```

- [ ] **Step 3: Verify frontend builds**

Run: `cd web && npm run build`
Expected: Builds without TypeScript errors

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(web): update types and i18n for Focused/Free mode rename"
```

---

### Task 7: Frontend — Agent Editor Mode Cards and Form Visibility

**Files:**
- Modify: `web/src/pages/agents-components/AgentEditorFullScreen.tsx`

- [ ] **Step 1: Update mode state default**

Line 306: Change `useState<'chat' | 'react'>('chat')` to `useState<'focused' | 'free' | 'chat' | 'react'>('focused')`

Line 350: Change fallback `?? 'chat'` to `?? 'focused'`

- [ ] **Step 2: Update mode card UI**

Replace the mode selection section (lines ~1110-1174) with:

**Focused Mode card:**
- Icon: Crosshair (import from lucide-react)
- Title: `tAgent('focusedMode', 'Focused Mode')`
- Description: `tAgent('focusedModeDescription', 'Bind resources for precise analysis')`
- Badge when selected: `tAgent('saveToken', 'Save Tokens')`

**Free Mode card:**
- Icon: Zap (keep existing)
- Title: `tAgent('freeMode', 'Free Mode')`
- Description: `tAgent('freeModeDescription', 'LLM explores freely with tools')`
- Badge when selected: `tAgent('recommended', 'Recommended')`

- [ ] **Step 3: Add conditional form field visibility**

Update the form rendering:
- `executionMode === 'focused'` (also handle `'chat'` for legacy): Show data collection config per resource
- `executionMode === 'free'` (also handle `'react'` for legacy): Show tool chaining + chain depth

Add a helper for mode detection:
```typescript
const isFocusedMode = executionMode === 'focused' || executionMode === 'chat'
const isFreeMode = executionMode === 'free' || executionMode === 'react'
```

- [ ] **Step 4: Add form validation**

In the save handler (around line 1032), before calling `onSave`:

```typescript
if (isFocusedMode && selectedResources.length === 0) {
  toast({
    title: tCommon('error'),
    description: tAgent('focusedModeRequiresResources'),
    variant: 'destructive',
  })
  return
}
```

- [ ] **Step 5: Send new mode value to backend**

In the save data object (line 1032):
```typescript
execution_mode: isFocusedMode ? 'focused' : 'free',
```

- [ ] **Step 6: Verify frontend builds**

Run: `cd web && npm run build`
Expected: Builds without errors

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat(web): update Agent Editor mode cards, form visibility, and validation"
```

---

### Task 8: Frontend — Data Collection Config UI (Focused Mode)

**Files:**
- Modify: `web/src/pages/agents-components/AgentEditorFullScreen.tsx` (resource item rendering)

- [ ] **Step 1: Add data collection config to metric resources**

When `isFocusedMode` and a selected resource is type `metric` or `extension_metric`, show a collapsible config section below the resource:

```tsx
{isFocusedMode && (resource.resource_type === 'metric' || resource.resource_type === 'extension_metric') && (
  <div className="ml-6 mt-1 border-l-2 border-muted pl-3 space-y-2">
    <Collapsible>
      <CollapsibleTrigger className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground">
        <ChevronRight className="h-3 w-3" />
        {tAgent('dataCollection', 'Data Collection')}
      </CollapsibleTrigger>
      <CollapsibleContent className="space-y-2 pt-2">
        <div className="flex items-center gap-2">
          <Label className="text-xs">{tAgent('timeRange', 'Time Range')}</Label>
          <Select
            value={resource.config?.data_collection?.time_range_minutes?.toString() || '60'}
            onValueChange={(v) => updateResourceConfig(resource.resource_id, { data_collection: { ...resource.config?.data_collection, time_range_minutes: parseInt(v) } })}
          >
            <SelectTrigger className="h-7 w-24 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="5">5 min</SelectItem>
              <SelectItem value="15">15 min</SelectItem>
              <SelectItem value="30">30 min</SelectItem>
              <SelectItem value="60">1 hour</SelectItem>
              <SelectItem value="360">6 hours</SelectItem>
              <SelectItem value="720">12 hours</SelectItem>
              <SelectItem value="1440">24 hours</SelectItem>
              <SelectItem value="10080">7 days</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="flex items-center gap-4">
          <label className="flex items-center gap-1 text-xs">
            <input type="checkbox" ... /> {tAgent('includeHistory', 'Include History')}
          </label>
          <label className="flex items-center gap-1 text-xs">
            <input type="checkbox" ... /> {tAgent('includeTrend', 'Include Trend')}
          </label>
          <label className="flex items-center gap-1 text-xs">
            <input type="checkbox" ... /> {tAgent('includeBaseline', 'Include Baseline')}
          </label>
        </div>
      </CollapsibleContent>
    </Collapsible>
  </div>
)}
```

- [ ] **Step 2: Include data_collection config in save payload**

Ensure the resource's `config` field (including `data_collection`) is passed to the backend in the save handler.

- [ ] **Step 3: Verify frontend builds**

Run: `cd web && npm run build`
Expected: Builds without errors

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(web): add data collection config UI for Focused Mode metric resources"
```

---

### Task 9: Final Verification

- [ ] **Step 1: Full Rust build + test**

Run: `cargo build && cargo test`
Expected: All tests pass

- [ ] **Step 2: Frontend build**

Run: `cd web && npm run build`
Expected: No TypeScript errors

- [ ] **Step 3: Verify backward compatibility**

Run: `cargo test -p neomind-storage -- --test-threads=1`
Expected: Existing agent data with `execution_mode: "chat"` or `"react"` deserializes correctly via serde alias
