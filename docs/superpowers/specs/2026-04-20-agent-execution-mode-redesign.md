# Agent Execution Mode Redesign: Focused Mode & Free Mode

**Date**: 2026-04-20
**Status**: Draft
**Affects**: `neomind-agent`, `neomind-api`, `web` (frontend)

## Problem

The current Chat Mode and React Mode lack clear differentiation in:
1. **Purpose**: Both modes feel similar to users
2. **Tool layer**: Chat Mode gets no tools, React Mode gets all — no middle ground
3. **Frontend form**: Same fields, same validation regardless of mode
4. **Resource binding**: Resources are stored but never effectively used in either mode
5. **LLM tool descriptions**: The mode descriptions are vague

## Design

### Mode Definitions

| | Focused Mode (was Chat) | Free Mode (was React) |
|---|---|---|
| **Purpose** | User defines scope, LLM works within boundaries | LLM explores freely with full capabilities |
| **Resources** | **Required** (>= 1) | Optional (recommended, not enforced) |
| **Tools for LLM** | None — structured prompt with data table + command list | Full 8 aggregated tools: `device`, `agent`, `rule`, `message`, `extension`, `transform`, `skill`, `shell` |
| **LLM calls** | Single pass | Multi-round (up to 3 rounds tool loop) |
| **Token cost** | Low (no tool definitions, concise prompt) | High (8 tool definitions + multi-round) |
| **Reliability** | High — limited scope, pre-collected data, template-driven | Depends on model capability — free exploration |
| **Multimodal** | Supported (images via multimodal LLM API: base64/URL as `ContentPart`) | Supported (images via multimodal LLM API in tool loop messages) |
| **Use cases** | Monitoring, alerts, data analysis, scheduled reports | Complex automation, device control, multi-step workflows |

### Backend Changes

#### 1. Executor: `should_use_tools()` — no logic change needed

Current behavior already correct:
- Focused Mode → `return false` (skip tool mode, use `analyze_with_llm` path)
- Free Mode → check LLM supports function calling + tool registry available

The rename from `Chat` to `Focused` is cosmetic at the `ExecutionMode` enum level.

#### 2. Focused Mode: Reliable Data & Command Execution

**Data injection** — improve `analyze_with_llm()` prompt building:

Replace the current loose text data summary with a structured table:

```
## Current Data (live from bound resources)
| Resource | Metric | Value | Time |
|----------|--------|-------|------|
| temp_living | temperature | 23.5°C | 10:30 |
| temp_living | humidity | 65% | 10:30 |

## Available Commands (only execute when needed)
| Display Name | Device | Command | Action Value |
|-------------|--------|---------|-------------|
| Turn on living room light | light_living | turn_on | `light_living:turn_on` |
| Turn off living room light | light_living | turn_off | `light_living:turn_off` |
```

This replaces the existing `build_available_commands_description()` and `build_available_data_sources_description()` output format.

**Decision template** — add fill-in-the-blank examples to prompt:

```
If you need to execute a command, output decisions in this format:
"decisions": [{"decision_type": "command", "action": "<copy from Available Commands>", "description": "<reason>"}]

If no action needed:
"decisions": []

Example - temperature exceeds threshold:
"decisions": [{"decision_type": "command", "action": "ac_living:turn_on", "description": "Temperature 32°C exceeds 30°C threshold"}]
```

**Three-layer command execution guarantee**:

1. **Prompt constraint** (existing, improved): Action values are explicitly listed in table, LLM copies them
2. **Fuzzy matching fallback** (enhance existing `handle_command_decision` in `command_executor.rs`): The current code already has `parse_command_from_action()` (lines 156-211) and fuzzy matching in `handle_command_decision()` (lines 322-367) for device commands and extension tools. Enhance this by:
   - Also matching against the command resource's `name` field (display name): "打开客厅灯" → match resource named "打开客厅灯" with resource_id "light_living:turn_on"
   - Also matching by command suffix across bound resources: "turn_on" → find any bound command resource ending with `:turn_on`
3. **Scope validation** (new in `execute_decisions`): In Focused Mode, only allow execution of commands that exist in `agent.resources` (type = Command or ExtensionTool). Reject out-of-scope commands with a warning log.

**Scope validation implementation**:

```rust
// In execute_decisions (command_executor.rs), add scope check BEFORE
// calling handle_command_decision for Focused Mode.
//
// Build allowed resource IDs from agent's command-type resources.
// Match using the same logic as handle_command_decision:
// - Device commands: resource_id format "device_id:command_name"
// - Extension tools: resource_id format "extension:ext_id:command_name"
if agent.execution_mode == ExecutionMode::Focused {
    let allowed_resource_ids: Vec<String> = agent.resources.iter()
        .filter(|r| matches!(r.resource_type, ResourceType::Command | ResourceType::ExtensionTool))
        .map(|r| r.resource_id.clone())
        .collect();

    for decision in decisions {
        if decision.decision_type == "command" {
            // Parse the action using existing parse_command_from_action logic,
            // then check if the resulting resource_id is in allowed list.
            // Also check fuzzy matches against allowed resources' name fields.
            let action = &decision.action;
            let is_allowed = allowed_resource_ids.iter().any(|rid| {
                action == rid                     // Exact match: "light_living:turn_on"
                || action.ends_with(rid.split(':').last().unwrap_or(""))  // Suffix: "turn_on"
                || allowed_resource_ids.iter().any(|r| {
                    // Name match: compare against resource display name
                    agent.resources.iter()
                        .find(|res| &res.resource_id == r)
                        .map(|res| res.name.contains(action))
                        .unwrap_or(false)
                })
            });

            if !is_allowed {
                tracing::warn!(
                    action = %decision.action,
                    "Focused Mode: rejecting out-of-scope command"
                );
                continue;
            }
        }
    }
}
```

#### 3. Free Mode: No Changes to Tool Loop

The existing `run_tool_loop()` with 8 tools works as-is. The only change is the mode name in enum and descriptions.

#### 4. Enum Rename

```rust
// crates/neomind-storage/src/agents.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutionMode {
    #[default]
    #[serde(rename = "focused", alias = "chat")]
    Focused,
    #[serde(rename = "free", alias = "react")]
    Free,
}
```

Serialization always outputs `"focused"` / `"free"`. Deserialization accepts both old (`chat`/`react`) and new (`focused`/`free`) values via serde `alias`.

**Migration strategy**:
- **Database**: No migration needed — redb stores serialized JSON, and serde alias handles old values transparently. When an agent is loaded, `chat` → `Focused`, `react` → `Free`. When saved back, the new names are used.
- **Frontend**: TypeScript types accept both old and new values (`'focused' | 'free' | 'chat' | 'react'`). The UI always displays/sends new names. Existing agents with old values are transparently handled.
- **API**: Response always uses new names (`"focused"` / `"free"`). Request accepts both.

#### 5. API Validation

In `crates/neomind-api/src/handlers/agents.rs`, add validation in `create_agent` handler (after building resources, before saving):

```rust
// After line ~966 (after resources Vec is built), add:
use neomind_storage::agents::ExecutionMode;
if let Some(ref mode) = request_body.execution_mode {
    if mode == "focused" && resources.is_empty() {
        return Err(ErrorResponse::with_message(
            "Focused Mode requires at least one resource binding".to_string()
        ));
    }
}
// Also add same check in update_agent handler after line ~1193
```

Error response format:
```json
{"success": false, "error": "Focused Mode requires at least one resource binding"}
```

### Frontend Changes

#### 1. Mode Selection UI

Update the two mode cards in `AgentEditorFullScreen.tsx`:

**Focused Mode card**:
- Icon: Target or Crosshair
- Title: "Focused Mode"
- Description: "精准模式 — 绑定资源和操作，快速精准分析。适合监控、告警、数据分析。"
- Badge: "省 Token"

**Free Mode card**:
- Icon: Zap
- Title: "Free Mode"
- Description: "自由模式 — LLM 自主探索和决策，支持多轮工具调用。适合复杂自动化和设备控制。"
- Badge: "推荐" (keep existing)

#### 2. Form Field Visibility

| Field | Focused Mode | Free Mode |
|---|---|---|
| Resource binding | **Required**, highlighted, validation error if empty | Optional |
| Data collection config per resource | **Shown** (time range, history, trend) | Hidden (LLM queries live) |
| Tool chaining toggle | Hidden | Shown |
| Max chain depth | Hidden | Shown (when chaining enabled) |
| Priority slider | Shown (advanced) | Shown (advanced) |
| Context window size | Shown (advanced) | Shown (advanced) |
| Prompt templates | Default to "analysis" template | Default to "automation" template |

#### 3. Form Validation

```typescript
// On save
if (executionMode === 'focused' && resources.length === 0) {
  toast({ title: t('validationError'), description: t('focusedModeRequiresResources') })
  scrollToResources()
  return
}
```

#### 4. Data Collection Configuration UI (Focused Mode Only)

The backend `collect_data()` already supports configurable data ranges via `resource.config.data_collection`:
- `time_range_minutes` (default: 60)
- `include_history` (default: false)
- `max_points` (default: 1000)
- `include_trend` (default: false)
- `include_baseline` (default: false)

**Currently this config is not exposed in the frontend.** For Focused Mode, this is critical because the LLM cannot query data itself — it only sees what's pre-collected.

**Frontend changes**: When a metric/extension_metric resource is selected in Focused Mode, show a collapsible "Data Collection" config section below each resource:

```
┌─ temperature (temp_living:temperature) ──────────────┐
│  Data Collection Settings                     ▼      │
│  ┌────────────────────────────────────────────────┐  │
│  │ Time Range: [60 ▾] minutes                     │  │
│  │ ☐ Include History Data                         │  │
│  │ ☐ Include Trend Analysis                       │  │
│  │ ☐ Include Baseline Comparison                  │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

Time range presets: 5min, 15min, 30min, 1h, 6h, 12h, 24h, 7d, custom.

This config is saved into the resource's `config.data_collection` field and used by `collect_data()` at execution time.

**Free Mode**: No data collection config needed — LLM queries data live via tools during execution.

#### 5. Type Updates

```typescript
// web/src/types/index.ts
execution_mode?: 'focused' | 'free' | 'chat' | 'react'  // Accept both old and new
```

#### 5. i18n Updates

Add new keys to the `agents` namespace in both locales:

`web/src/i18n/locales/en/agents.json`:
```json
{
  "focusedMode": "Focused Mode",
  "focusedModeDescription": "Bind specific resources and actions for fast, precise analysis. Best for monitoring, alerts, data analysis.",
  "freeMode": "Free Mode",
  "freeModeDescription": "LLM freely explores and decides with multi-round tool calling. Best for complex automation and device control.",
  "focusedModeRequiresResources": "Focused Mode requires at least one resource binding",
  "saveToken": "Save Tokens"
}
```

`web/src/i18n/locales/zh/agents.json`:
```json
{
  "focusedMode": "精准模式",
  "focusedModeDescription": "绑定特定资源和操作，快速精准分析。适合监控、告警、数据分析。",
  "freeMode": "自由模式",
  "freeModeDescription": "LLM 自主探索和决策，支持多轮工具调用。适合复杂自动化和设备控制。",
  "focusedModeRequiresResources": "精准模式需要至少绑定一个资源",
  "saveToken": "省 Token"
}
```

### LLM Tool Description Changes

**Consistency requirement**: The LLM tool descriptions in `simplified.rs` must stay consistent with the frontend form. Both are entry points for creating agents — one for LLM (chat), one for humans (UI). The same rules apply: Focused requires resources, Free is optional.

In `crates/neomind-agent/src/toolkit/simplified.rs`, update the `agent` tool's description for `execution_mode`, `resources`, and add `data_collection` config:

```
execution_mode:
  'focused' = Focused mode — must bind resources, LLM works within defined scope. Fast, precise, token-efficient. Best for monitoring, alerts, data analysis.
  'free' = Free mode — LLM freely explores with all tools, supports multi-round reasoning. Best for complex automation and device control.

resources:
  Required for Focused mode (at least 1), optional for Free mode.
  Format: [{"resource_type":"device|metric|command|extension_tool|extension_metric", "resource_id":"...", "config": {...}}]
  Focused mode: these define the exact scope of what the agent can read and control.
  Free mode: these are recommended focus areas, the agent can access anything.

  For metric resources in Focused mode, config.data_collection controls what data is pre-collected:
  {
    "data_collection": {
      "time_range_minutes": 60,    // How far back to look (default: 60)
      "include_history": false,    // Include historical time-series (default: false)
      "max_points": 1000,          // Max data points when include_history=true
      "include_trend": false,      // Include trend analysis (default: false)
      "include_baseline": false    // Compare against learned baselines (default: false)
    }
  }
  Note: data_collection only applies to Focused mode. Free mode agents query data live via tools.
```

### Files to Modify

| File | Changes |
|------|---------|
| `crates/neomind-storage/src/agents.rs` | Rename `ExecutionMode` variants, add serde alias for backward compat |
| `crates/neomind-agent/src/ai_agent/executor/analyzer.rs` | Structured data/command table in prompt, decision templates |
| `crates/neomind-agent/src/ai_agent/executor/command_executor.rs` | Fuzzy matching fallback, scope validation for Focused mode |
| `crates/neomind-agent/src/toolkit/simplified.rs` | Update tool descriptions for mode, resources, and data_collection params |
| `crates/neomind-api/src/handlers/agents.rs` | Validation: Focused mode requires resources |
| `web/src/pages/agents-components/AgentEditorFullScreen.tsx` | Mode card UI, form field visibility, data collection config, validation |
| `web/src/types/index.ts` | Update execution_mode type |
| `web/src/i18n/locales/en/agents.json` | New i18n keys |
| `web/src/i18n/locales/zh/agents.json` | New i18n keys |

### Not Changed

- **Chat dialogue page** (`/chat`): Uses a separate `Session` + `Agent` pipeline, not affected
- **Free Mode tool loop**: Works as-is, no changes to `run_tool_loop()`
- **Multimodal support**: Both modes handle images via LLM's multimodal API (`ContentPart::image_base64` / `ContentPart::image_url`), not by embedding image data into text prompts. No mode-specific changes needed.
- **Memory system**: Both modes use agent memory, no mode-specific changes
- **DynamicToolGenerator** (`context/dynamic_tools.rs`): Not used in this design, may be leveraged in future

### Testing Plan

1. **Unit tests**: Fuzzy matching, scope validation, enum deserialization (old + new names)
2. **Integration tests**: Create Focused agent without resources → 400 error; create with resources → success
3. **E2E test**: Focused agent with temperature sensor + light command, verify structured prompt output and command execution
4. **Backward compat**: Existing agents with `execution_mode: "chat"` should deserialize as `Focused`, `"react"` as `Free`
