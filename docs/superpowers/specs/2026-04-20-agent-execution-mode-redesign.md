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
| **Tools for LLM** | None — structured prompt with data table + command list | Full 7 aggregated tools: `device`, `agent`, `rule`, `message`, `extension`, `transform`, `skill`, `shell` |
| **LLM calls** | Single pass | Multi-round (up to 3 rounds tool loop) |
| **Token cost** | Low (no tool definitions, concise prompt) | High (7 tool definitions + multi-round) |
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
2. **Fuzzy matching fallback** (new in `execute_decisions`): When exact action match fails, attempt fuzzy match:
   - Match by display name: "打开客厅灯" → find command resource with similar name
   - Match by command suffix: "turn_on" → find command resource ending with `:turn_on`
   - Match by resource ID substring
3. **Scope validation** (new in `execute_decisions`): In Focused Mode, only allow execution of commands that exist in `agent.resources` (type = Command or ExtensionTool). Reject out-of-scope commands with a warning log.

**Scope validation implementation**:

```rust
// In execute_decisions, add scope check for Focused Mode
if agent.execution_mode == ExecutionMode::Focused {
    let allowed_actions: Vec<&str> = agent.resources.iter()
        .filter(|r| matches!(r.resource_type, ResourceType::Command | ResourceType::ExtensionTool))
        .map(|r| &r.resource_id[..])
        .collect();

    for decision in decisions {
        if decision.decision_type == "command" {
            if !allowed_actions.iter().any(|a| decision.action.contains(a)) {
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

The existing `run_tool_loop()` with 7 tools works as-is. The only change is the mode name in enum and descriptions.

#### 4. Enum Rename

```rust
// crates/neomind-storage/src/agents.rs
pub enum ExecutionMode {
    #[default]
    Focused,  // was Chat
    Free,     // was React
}

// Keep backward compatibility in serde deserialization
#[serde(rename_all = "lowercase")]
// Accept both old and new names:
// "focused" | "chat" → Focused
// "free" | "react" → Free
```

The serde deserialization should accept both old (`chat`/`react`) and new (`focused`/`free`) values for backward compatibility. Serialization uses new names.

#### 5. API Validation

In `crates/neomind-api/src/handlers/agents.rs`:

```rust
// Create agent
if execution_mode == Focused && resources.is_empty() {
    return Err(ErrorResponse::with_message(
        "Focused Mode requires at least one resource binding"
    ));
}
// Free Mode: resources optional, no validation
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

#### 4. Type Updates

```typescript
// web/src/types/index.ts
execution_mode?: 'focused' | 'free' | 'chat' | 'react'  // Accept both old and new
```

#### 5. i18n Updates

Add new keys in both `en/agents.json` and `zh/agents.json`:
- `focusedMode` / `focusedModeDescription`
- `freeMode` / `freeModeDescription`
- `focusedModeRequiresResources` (validation message)
- `saveToken` (badge text)

### LLM Tool Description Changes

In `crates/neomind-agent/src/toolkit/simplified.rs`, update the `agent` tool's description for `execution_mode` and `resources` parameters:

```
execution_mode:
  'focused' = Focused mode — must bind resources, LLM works within defined scope. Fast, precise, token-efficient. Best for monitoring, alerts, data analysis.
  'free' = Free mode — LLM freely explores with all tools, supports multi-round reasoning. Best for complex automation and device control.

resources:
  Required for Focused mode (at least 1), optional for Free mode.
  Format: [{"resource_type":"device|metric|command|extension_tool|extension_metric", "resource_id":"..."}]
  Focused mode: these define the exact scope of what the agent can read and control.
  Free mode: these are recommended focus areas, the agent can access anything.
```

### Files to Modify

| File | Changes |
|------|---------|
| `crates/neomind-storage/src/agents.rs` | Rename `ExecutionMode` variants, add serde alias for backward compat |
| `crates/neomind-agent/src/ai_agent/executor/analyzer.rs` | Structured data/command table in prompt, decision templates |
| `crates/neomind-agent/src/ai_agent/executor/command_executor.rs` | Fuzzy matching fallback, scope validation for Focused mode |
| `crates/neomind-agent/src/toolkit/simplified.rs` | Update tool descriptions for mode and resources params |
| `crates/neomind-api/src/handlers/agents.rs` | Validation: Focused mode requires resources |
| `web/src/pages/agents-components/AgentEditorFullScreen.tsx` | Mode card UI, form field visibility, validation |
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
