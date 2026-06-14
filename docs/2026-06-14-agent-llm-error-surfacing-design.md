# Agent LLM Error Surfacing — Design Spec

**Date:** 2026-06-14
**Status:** Approved (brainstormed)
**Author:** Claude + shenmingming

## 1. Problem

When an AI agent's LLM call fails (e.g., quota exhausted → HTTP 403, network blip, API key invalid), the executor **silently falls back** to rule-based analysis (`crates/neomind-agent/src/ai_agent/executor/analyzer.rs:103-110` and `analyzer.rs:76-83`). The fallback returns:

- `situation_analysis` = `"Analyzing N data points for agent '...'"` (template)
- `decisions` = `[]` (no real analysis)
- `conclusion` = `"No actions required - conditions not met"` (template)

`execute_internal` then marks the execution as `Completed` with no error indication. The result: users see what looks like a successful agent run that "didn't trigger any conditions," while the real cause (quota, auth, network) is buried in server logs. This causes significant debugging wasted effort.

The current `LlmError` enum also lacks structured HTTP status — error codes are baked into `LlmError::Generation(format!("API error 403: ..."))` strings, making reliable classification impossible.

## 2. Goals

1. **Surface LLM failures explicitly** in agent execution results, not silently degrade.
2. **Distinguish permanent vs transient errors** so retry policy and user messaging differ appropriately.
3. **Auto-pause agents** that fail repeatedly with permanent errors (no point retrying quota-exhausted API).
4. **Push real-time alerts** through the existing Message/channel system.
5. **Zero data migration** (redb is schema-less; new fields default safely).

## 3. Non-Goals

- Chat streaming path error handling (chat already surfaces errors directly to the user — out of scope).
- Manual "恢复" (resume) UI button — recovery is automatic on next successful execution.
- Backoff/retry logic for transient errors (existing executor already has per-tool timeouts; scheduler-level retry is a separate concern).
- Tool execution error surfacing (existing tool-loop error handling is unchanged).
- Configurable threshold via env var — hard-coded `N=3` for v1.

## 4. Architecture

### 4.1 Error Classification

**New `LlmError::Api` variant** (`crates/neomind-core/src/llm/backend.rs:416`):

```rust
#[error("API error {status}: {body}")]
Api {
    status: u16,
    body: String,
},
```

**Classification helper**:

```rust
impl LlmError {
    /// Permanent errors require user action (top up quota, fix API key,
    /// change model, reduce context). Retrying will not help.
    pub fn is_permanent(&self) -> bool {
        match self {
            Self::BackendUnavailable(_)
            | Self::ModelNotFound(_)
            | Self::InvalidInput(_)
            | Self::ContextOverflow { .. }
            | Self::Serialization(_) => true,
            Self::Api { status, .. } => *status >= 400 && *status < 500 && *status != 429,
            // 429 (rate limit), 5xx, timeout, network, IO are transient
            Self::Timeout(_)
            | Self::Network(_)
            | Self::Io(_) => false,
            // Conservative: treat generic Generation/Unknown as transient
            // (worst case: we degrade instead of failing — reversible).
            Self::Generation(_) | Self::Unknown(_) => false,
        }
    }
}
```

**Classification matrix:**

| Variant | Permanent? | Example | Action |
|---|---|---|---|
| `BackendUnavailable` | ✅ | "ollama backend not available" | Fail |
| `ModelNotFound` | ✅ | "qwen3.5:4b not pulled" | Fail |
| `InvalidInput` | ✅ | malformed request | Fail |
| `ContextOverflow` | ✅ | prompt too long | Fail |
| `Serialization` | ✅ | bug | Fail |
| `Api { status: 401/403/404 }` | ✅ | quota, auth, model not found | Fail |
| `Api { status: 429 }` | ❌ | rate limited | Degrade |
| `Api { status: 5xx }` | ❌ | server error | Degrade |
| `Timeout` | ❌ | 300s LLM timeout | Degrade |
| `Network` | ❌ | connection refused | Degrade |
| `Io` | ❌ | local IO hiccup | Degrade |
| `Generation` | ❌ | legacy fallback (no status) | Degrade |
| `Unknown` | ❌ | unspecified | Degrade |

### 4.2 Cloud Backend Updates

Update ~10 error-emitting sites across 2-3 backend files (most cloud providers share the OpenAI-compatible code path in `openai.rs`):

- `crates/neomind-agent/src/llm_backends/backends/openai.rs:677, 819, 1013, 1235`
- `crates/neomind-agent/src/llm_backends/backends/ollama.rs` (similar pattern)

Replace:
```rust
return Err(LlmError::Generation(format!("API error {}: {}", status, body)));
```

With:
```rust
return Err(LlmError::Api { status: status.as_u16(), body });
```

For non-HTTP errors (connection, timeout) keep existing `Network`/`Timeout` variants.

### 4.3 AnalysisResult Extension

`crates/neomind-agent/src/ai_agent/executor/analyzer.rs:12-23`:

```rust
pub(crate) enum AnalysisResult {
    Focused {
        situation_analysis: String,
        reasoning_steps: Vec<ReasoningStep>,
        decisions: Vec<Decision>,
        conclusion: String,
        degraded_reason: Option<String>, // NEW: None = clean, Some = degraded with reason
    },
    Free { /* unchanged */ },
}
```

### 4.4 Analyzer Branch Logic

`analyze_situation_with_intent` (analyzer.rs:42-135) becomes:

```rust
pub(crate) async fn analyze_situation_with_intent(
    &self, agent: &AiAgent, data: &[DataCollected],
    parsed_intent: Option<&ParsedIntent>, execution_id: &str,
    invocation_input: Option<&AgentInput>,
) -> AgentResult<AnalysisResult> {
    let mut degraded_reason: Option<String> = None;

    match self.get_llm_runtime_for_agent(agent).await {
        Ok(Some(llm)) => {
            if self.should_use_tools(agent, &llm) {
                match self.execute_with_tools(...).await {
                    Ok((dp, er)) => return Ok(AnalysisResult::Free { decision_process: dp, execution_result: er }),
                    Err(e) if e.is_permanent() => return Err(e.into()),
                    Err(e) => {
                        tracing::warn!(error = %e, "transient tool error, will try direct LLM");
                        degraded_reason = Some(format!("LLM 工具调用瞬时失败: {}", e));
                    }
                }
            }
            match self.analyze_with_llm(...).await {
                Ok(r) => return Ok(AnalysisResult::Focused { ..r, degraded_reason: None }),
                Err(e) if e.is_permanent() => return Err(e.into()),
                Err(e) => {
                    tracing::warn!(error = %e, "transient LLM error, degrading to rule-based");
                    degraded_reason = Some(format!("LLM 瞬时错误: {}", e));
                }
            }
        }
        Ok(None) => {
            // Only fail hard if user explicitly bound a backend that's now missing.
            // Rule-only agents (no llm_backend_id) proceed without LLM — backwards compat.
            if agent.llm_backend_id.is_some() {
                return Err(NeoMindError::Validation(format!(
                    "Agent '{}' 绑定的 LLM 后端不可用", agent.name
                )));
            }
            // Rule-only agent: proceed normally, no degradation flag
        }
        Err(e) => return Err(e),
    }

    // Transient fallback path
    let (sa, rs, ds, con) = self.analyze_rule_based(agent, data, parsed_intent).await?;
    Ok(AnalysisResult::Focused {
        situation_analysis: format!(
            "⚠️ 已降级为规则分析\n{}\n原因: {}",
            sa,
            degraded_reason.clone().unwrap_or_default()
        ),
        reasoning_steps: rs,
        decisions: ds,
        conclusion: con,
        degraded_reason, // Some(...) — signals Partial status downstream
    })
}
```

### 4.5 execute_internal Wiring

`crates/neomind-agent/src/ai_agent/executor/mod.rs:1184-1192`:

```rust
let analysis = self.analyze_situation_with_intent(...).await?;

match analysis {
    AnalysisResult::Focused { degraded_reason: Some(reason), .. } => {
        // Mark execution as Partial + populate error field
        execution_record.status = ExecutionStatus::Partial;
        execution_record.error = Some(reason);
        // Still update memory with rule-based decisions (existing path)
    }
    AnalysisResult::Focused { degraded_reason: None, .. } => {
        // Normal Completed path (existing)
    }
    AnalysisResult::Free { .. } => { /* existing */ }
}

// Err bubbles up to outer handler which already writes success:false journal
// entry (per MEMORY.md "Error path journal write"). We additionally:
//   1. Set execution_record.status = Failed
//   2. Set execution_record.error = format!("{}", e)
//   3. Notify scheduler of permanent failure (see 4.6)
```

The outer `Err(e)` branch (mod.rs around line 1380+) is extended to record `Failed` status and propagate the error to the scheduler hook.

### 4.6 Scheduler Auto-Pause

**New fields on `Agent`** (`crates/neomind-storage/src/agents.rs`):

```rust
pub struct Agent {
    // ...existing fields...
    /// Counter of consecutive permanent failures (reset on any success).
    /// Used by scheduler to trigger auto-pause.
    #[serde(default)]
    pub consecutive_permanent_failures: u32,

    /// Set to true by scheduler after N consecutive permanent failures.
    /// Cleared by any successful execution (manual trigger or scheduled).
    /// When true, scheduled + event triggers are skipped; manual triggers
    /// still execute (so user can verify the fix).
    #[serde(default)]
    pub auto_paused: bool,
}
```

**Scheduler skip logic** (`crates/neomind-agent/src/ai_agent/scheduler.rs`):

```rust
// In task selection (around line 282-300):
if task.agent.auto_paused {
    // Skip scheduled/event triggers. Manual API trigger bypasses scheduler
    // entirely (calls executor directly), so auto_pause doesn't block recovery.
    skipped.push((task_id, task.next_execution, "auto_paused"));
    continue;
}
```

**Failure tracking** (after execution completes in scheduler):

```rust
match execution_outcome {
    Outcome::Failed(err) if err.is_permanent_llm_error() => {
        agent.consecutive_permanent_failures += 1;
        if agent.consecutive_permanent_failures >= 3 && !agent.auto_paused {
            agent.auto_paused = true;
            // Push critical Message via existing channel system
            self.emit_message(Message {
                source_type: "system".into(),
                category: "agent_paused".into(),
                severity: Severity::Critical,
                content: format!(
                    "Agent '{}' 已连续 3 次永久错误，自动暂停。最后错误: {}\n\
                     修复后请手动触发一次执行以恢复。",
                    agent.name, err
                ),
                metadata: serde_json::json!({"agent_id": agent.id}),
                ..
            }).await;
        }
    }
    Outcome::Failed(_) => {
        // Transient failure — don't increment counter
    }
    Outcome::Partial(_) => {
        // Degrade — don't increment counter (rule-based still ran)
    }
    Outcome::Success => {
        // Clear failure state on any success
        agent.consecutive_permanent_failures = 0;
        agent.auto_paused = false;
    }
}
```

**Recovery flow (no UI button):**

1. User sees Message: "Agent X 已连续 3 次永久错误，自动暂停"
2. User fixes root cause (top up quota, fix API key, change model)
3. User clicks existing "Run now" button on agent detail page
4. "Run now" bypasses scheduler → calls executor directly → executes
5. If success: `consecutive_permanent_failures = 0`, `auto_paused = false`
6. Agent resumes normal scheduling

If "Run now" still fails, user gets another normal Failed execution record and can iterate.

### 4.7 Frontend Changes

**`web/src/types/index.ts:2065`** — add Partial to ExecutionStatus:

```typescript
export type ExecutionStatus = 'Running' | 'Completed' | 'Failed' | 'Cancelled' | 'Partial'
```

**`web/src/pages/agents-components/AgentExecutionTimeline.tsx:207-220`** — add Partial case:

```typescript
case 'Partial':
    return {
        icon: AlertCircle,
        color: 'text-warning',
        bg: 'bg-warning-light border-warning',
        label: t('agents:executionStatus.partial')
    }
```

Add `partial` key to i18n locale files (`web/src/i18n/locales/{en,zh}/agents.json`): `"partial": "Degraded"` / `"partial": "已降级"`.

**`web/src/pages/agents-components/ExecutionDetailDialog.tsx`** — top banner above existing content:

```tsx
{execution.status === 'Partial' && execution.error && (
    <div className="mb-4 p-3 rounded-lg border bg-warning-light border-warning text-warning-foreground">
        <div className="flex items-center gap-2 font-medium">
            <AlertCircle className="h-4 w-4" />
            {t('agents:execution.degradedBanner')}
        </div>
        <p className="text-sm mt-1 opacity-90">{execution.error}</p>
    </div>
)}
{execution.status === 'Failed' && execution.error && (
    <div className="mb-4 p-3 rounded-lg border bg-error-light border-error text-error-foreground">
        <div className="flex items-center gap-2 font-medium">
            <XCircle className="h-4 w-4" />
            {t('agents:execution.failedBanner')}
        </div>
        <p className="text-sm mt-1 opacity-90 font-mono break-words">{execution.error}</p>
    </div>
)}
```

(Uses design tokens per DESIGN_SPEC.md — no hardcoded Tailwind palette colors.)

**No changes to Message/channel UI** — existing components already render messages from any source.

### 4.8 Data Migration

None. redb is schema-less. New fields `consecutive_permanent_failures` and `auto_paused` use `#[serde(default)]`, so loading legacy agent records yields `0` and `false`.

## 5. Testing

### 5.1 Unit Tests (`crates/neomind-core/src/llm/backend.rs`)

- `is_permanent()` for every `LlmError` variant
- `Api { status: 401 }` → permanent
- `Api { status: 403 }` → permanent
- `Api { status: 404 }` → permanent
- `Api { status: 429 }` → transient
- `Api { status: 500 }` → transient
- `Api { status: 503 }` → transient

### 5.2 Integration Tests (`crates/neomind-agent/`)

- **Permanent LLM error path**: Mock LLM runtime returns `Api { status: 403 }`. Assert `ExecutionStatus::Failed`, `error` field populated, scheduler increments counter.
- **Transient LLM error path**: Mock returns `Timeout`. Assert `ExecutionStatus::Partial`, `degraded_reason` populated, counter unchanged.
- **Auto-pause trigger**: Simulate 3 consecutive `Api { status: 403 }` failures. Assert `auto_paused == true` and Message emitted.
- **Auto-pause recovery**: Auto-paused agent + manual trigger + success. Assert `auto_paused == false` and counter reset.
- **Rule-only agent (no LLM bound)**: Assert no degradation flag, runs normally without LLM.

### 5.3 Manual Verification

1. Configure agent with GLM backend (the one in user's bug report)
2. Set invalid API key or wait for quota exhaustion
3. Trigger agent execution
4. Verify: execution record status=Failed, error message visible in detail dialog
5. Verify: timeline shows red badge
6. After 3 consecutive failures: verify Message appears in channels, agent shows auto_paused state
7. Fix API key, click "Run now", verify agent recovers

## 6. Build Sequence

1. **LlmError variant + is_permanent()** — pure type change, no behavior change. CI must pass.
2. **Cloud backend updates** — replace `Generation(format!(...))` with `Api { status, body }`. CI must pass (error messages slightly change format).
3. **AnalysisResult + analyzer branch logic** — add degraded_reason, change fallthrough paths. Integration test for each branch.
4. **execute_internal wiring** — propagate degraded_reason to ExecutionRecord. Status now varies.
5. **Scheduler auto-pause** — add Agent fields, skip logic, counter, Message emission.
6. **Frontend Partial status + banners + i18n** — visual polish, can ship behind feature flag if needed.
7. **Documentation update** — add note to `docs/guides/en/03-agent.md` about auto-pause behavior.

## 7. Open Questions Resolved

- **N=3 for auto-pause?** Yes, hard-coded. Can be made configurable later if needed.
- **Recovery UI button?** No — automatic on next successful execution (manual trigger).
- **Chat streaming path?** Out of scope — already surfaces errors directly.
- **Tool execution errors?** Out of scope — existing tool-loop handling unchanged.
- **Backward compat for rule-only agents?** Yes — agents without `llm_backend_id` continue to work without LLM, no degradation flag.

## 8. Risks

- **Risk**: Existing tests/integrations depend on the `Generation("API error N: ...")` string format.
  **Mitigation**: Grep for `"API error"` in tests; update assertions to use `Api { status: N, .. }` pattern match.

- **Risk**: Auto-pause false positives (e.g., GLM provider has a long outage, agent gets paused).
  **Mitigation**: User can always recover via manual "Run now". Auto-pause is reversible.

- **Risk**: `Generation(_)` conservatively classified as transient — could mask permanent errors from non-HTTP sources.
  **Mitigation**: Once all cloud backends are migrated to `Api {}`, `Generation` should rarely occur. Re-evaluate classification after telemetry shows real-world distribution.

- **Risk**: Frontend Partial status might confuse users who haven't seen the term before.
  **Mitigation**: Banner text explains "已降级为规则分析" with the reason. i18n labels are descriptive.
