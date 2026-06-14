# Agent LLM Error Surfacing — Design Spec

**Date:** 2026-06-14
**Status:** Approved (brainstormed) — Rev 2 (post spec-review)
**Author:** Claude + shenmingming

> **Rev 2 changes** address spec-reviewer findings:
> - Inner LLM functions return `Result<_, LlmError>` to preserve structured error (was lost via `NeoMindError::Llm(String)` conversion)
> - Scheduler uses existing `Result<AgentExecutionRecord, NeoMindError>` shape, checks `record.status == Partial` in `Ok` arm (no new `Outcome` enum)
> - `auto_paused: bool` added to `ScheduledTask` (avoids batch-loading agents each tick)
> - Reuse existing `consecutive_failures` for transient retry; add `permanent_failure_count` for auto-pause counter (avoids parallel-counter confusion)
> - Permanent errors bypass existing retry/backoff logic (retrying 403 is pointless)
> - Frontend token corrected: `text-primary-foreground` per DESIGN_SPEC.md
> - Partial executions do NOT set `AgentStatus::Error` (rule-based still ran) — requires modifying the `Completed → Active, else → Error` guard at mod.rs:942-946
> - Auto-pause Message rate-limited per agent_id (60s cooldown)
>
> **Rev 3 changes** address round-2 spec-review findings:
> - Explicit code change shown for mod.rs:942-946 guard (was wrongly claimed as already-safe)
> - Field name in section 4.8 corrected to `permanent_failure_count` (was leftover `consecutive_permanent_failures`)
> - `Ok(None)` path no longer hard-fails — degrades instead (consistent with design philosophy; avoids breaking change when backend deleted)
> - `message_dedup` field on `AgentScheduler` explicitly noted as new

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

**Critical implementation note**: `NeoMindError::Llm(String)` currently stringifies the underlying `LlmError` at every callsite (e.g., analyzer.rs:272-274 uses `format!("LLM generation failed: {}", e)`). By the time the error reaches `analyze_situation_with_intent`, the structured `LlmError` is **lost** — `is_permanent()` cannot be called on a `NeoMindError`.

**Fix**: Inner LLM functions return `Result<_, LlmError>` and only convert at the outer `analyze_situation_with_intent` boundary. Update signatures:

- `analyze_with_llm` (analyzer.rs:143) → `Result<_, LlmError>` instead of `Result<_, NeoMindError>`
- `execute_with_tools` (mod.rs:424) → `Result<_, LlmError>` for the LLM-specific failure path
- `try_in_process_dispatch` paths unchanged (no LLM involvement)

Inside these functions, remove the `?` conversions that wrap as `NeoMindError::Llm(String)` and let `LlmError` propagate naturally.

`analyze_situation_with_intent` (analyzer.rs:42-135) becomes:

```rust
pub(crate) async fn analyze_situation_with_intent(
    &self, agent: &AiAgent, data: &[DataCollected],
    parsed_intent: Option<&ParsedIntent>, execution_id: &str,
    invocation_input: Option<&AgentInput>,
) -> AgentResult<AnalysisResult> {
    use neomind_core::llm::backend::LlmError;
    let mut degraded_reason: Option<String> = None;

    match self.get_llm_runtime_for_agent(agent).await {
        Ok(Some(llm)) => {
            if self.should_use_tools(agent, &llm) {
                match self.execute_with_tools(...).await {
                    Ok((dp, er)) => return Ok(AnalysisResult::Free { decision_process: dp, execution_result: er }),
                    Err(e) if e.is_permanent() => {
                        return Err(NeoMindError::Llm(format!("{}", e)));
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "transient tool error, will try direct LLM");
                        degraded_reason = Some(format!("LLM 工具调用瞬时失败: {}", e));
                    }
                }
            }
            match self.analyze_with_llm(...).await {  // Now returns Result<_, LlmError>
                Ok(r) => return Ok(AnalysisResult::Focused { ..r, degraded_reason: None }),
                Err(e) if e.is_permanent() => {
                    return Err(NeoMindError::Llm(format!("{}", e)));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "transient LLM error, degrading to rule-based");
                    degraded_reason = Some(format!("LLM 瞬时错误: {}", e));
                }
            }
        }
        Ok(None) => {
            // LLM runtime unavailable. This could be because:
            //  - Rule-only agent (no llm_backend_id) — normal, no degradation
            //  - Backend was deleted or unset — degrade for visibility
            //  - Default runtime not initialized — degrade for visibility
            if agent.llm_backend_id.is_some() {
                // Agent expected an LLM but runtime is gone — degrade (don't hard-fail).
                // This avoids breaking behavior change when user deletes a backend:
                // previously agents silently fell back to rule-based; now they mark
                // themselves Partial so the user knows LLM isn't running.
                degraded_reason = Some(format!(
                    "Agent '{}' 配置了 LLM 后端但运行时不可用（可能已被删除）",
                    agent.name
                ));
            }
            // Rule-only agent: proceed normally, no degradation flag
        }
        Err(e) => {
            // Backend store read error — transient (storage hiccup)
            tracing::warn!(error = %e, "LLM runtime load failed, degrading");
            degraded_reason = Some(format!("LLM 后端加载失败: {}", e));
        }
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

**Note on `Ok(None)` for `get_llm_runtime_for_agent` errors**: previously these fell through to rule-based silently. Now we treat them as transient (storage read errors are typically transient) and set `degraded_reason` — the agent will be marked Partial instead of Completed, surfacing the issue without hard-failing.

### 4.5 execute_internal Wiring

`crates/neomind-agent/src/ai_agent/executor/mod.rs` currently destructures `AnalysisResult::Focused` by name (around line 1260-1268). Update the destructure to bind `degraded_reason`:

```rust
AnalysisResult::Focused {
    situation_analysis,
    reasoning_steps,
    decisions,
    conclusion,
    degraded_reason,  // NEW binding
} => {
    // ... existing post-analysis steps (execute_decisions, report, memory update) ...

    // After all post-analysis steps, apply status override if degraded
    if let Some(reason) = &degraded_reason {
        execution_record.status = ExecutionStatus::Partial;
        execution_record.error = Some(reason.clone());
    }
}
```

**CRITICAL — also modify the AgentStatus guard at `mod.rs:942-946`**:

Current code (WRONG for Partial):
```rust
let new_status = if record.status == ExecutionStatus::Completed {
    neomind_storage::AgentStatus::Active
} else {
    neomind_storage::AgentStatus::Error  // ← Partial hits this branch
};
```

Change to:
```rust
let new_status = if record.status == ExecutionStatus::Completed
    || record.status == ExecutionStatus::Partial {
    neomind_storage::AgentStatus::Active
} else {
    neomind_storage::AgentStatus::Error  // Only Failed/Cancelled
};
```

This explicitly keeps `AgentStatus::Active` for Partial executions — the agent is still functionally working (rule-based analysis ran), just degraded. Only `Failed` and `Cancelled` should trigger `AgentStatus::Error`.

The status override happens **after** existing post-analysis steps so that:
1. Memory update still runs with rule-based decisions (learning continues)
2. The existing `if record.status == Failed` guard at mod.rs:945 (for `AgentStatus::Error` assignment) doesn't accidentally trigger on Partial
3. Notifications/reports still generate from rule-based decisions

The outer `Err(e)` branch (mod.rs outermost error handler) is extended to ensure:
1. `execution_record.status = Failed`
2. `execution_record.error = Some(format!("{}", e))`
3. Existing journal write of `success: false` entry (already in place per MEMORY.md)
4. Scheduler hook receives the failure for counter increment (see 4.6)

### 4.6 Scheduler Auto-Pause

**Existing field conflict**: `AiAgent` (`agents.rs:100-102`) already has `consecutive_failures: u32` used by scheduler retry logic (lines 444-484) for exponential backoff. To avoid parallel-counter confusion:

- **Keep** `consecutive_failures` for **transient** retry/backoff semantics
- **Add** `permanent_failure_count: u32` for permanent error auto-pause counter
- **Add** `auto_paused: bool` for pause flag

**New fields on `Agent`** (`crates/neomind-storage/src/agents.rs`):

```rust
pub struct Agent {
    // ...existing fields including consecutive_failures: u32...

    /// Counter of consecutive PERMANENT failures (4xx auth/quota, model
    /// not found, etc.). Reset on any success. Drives auto-pause.
    /// Distinct from `consecutive_failures` which tracks transient retry.
    #[serde(default)]
    pub permanent_failure_count: u32,

    /// Set to true by scheduler after `permanent_failure_count` >= 3.
    /// Cleared by any successful execution (manual trigger or scheduled).
    /// When true, scheduled + event triggers are skipped; manual triggers
    /// still execute (so user can verify the fix).
    #[serde(default)]
    pub auto_paused: bool,
}
```

**Scheduler interaction with existing retry logic** (scheduler.rs:444-484):

The existing retry logic increments `consecutive_failures` and applies exponential backoff for ALL failures. This is wrong for permanent errors (retrying 403 is pointless). Update the retry decision:

```rust
// Inside scheduler.rs retry logic (around line 444):
let is_permanent = matches!(
    &result,
    Err(e) if is_permanent_llm_error(e)  // helper to extract from NeoMindError::Llm(String)
);

if is_permanent {
    // Permanent error: skip retry, increment permanent counter, maybe auto-pause
    agent.consecutive_failures = 0;  // reset transient counter
    agent.permanent_failure_count = agent.permanent_failure_count.saturating_add(1);

    if agent.permanent_failure_count >= 3 && !agent.auto_paused {
        agent.auto_paused = true;
        // Mark ScheduledTask.auto_paused for fast path skip
        if let Some(task) = self.tasks.get(&agent_id) {
            task.write().await.auto_paused = true;
        }
        // Emit Message (rate-limited per agent_id, see below)
        self.emit_auto_pause_message(&agent).await;
    }
} else {
    // Transient error: existing retry/backoff path (unchanged)
    agent.consecutive_failures += 1;
    // ... existing backoff logic ...
}
```

**Add `auto_paused: bool` to `ScheduledTask`** (`scheduler.rs:96-111`):

```rust
pub struct ScheduledTask {
    // ...existing fields...
    /// Cached `agent.auto_paused` flag for O(1) skip in selection loop.
    /// Synced when task is registered, updated, or when scheduler sets auto_paused.
    pub auto_paused: bool,
}
```

Sync points:
- `schedule_agent` (line 196): initialize `auto_paused = agent.auto_paused`
- After execution outcome modifies `agent.auto_paused`: update the cached task field
- Optional: periodic reconcile loop (e.g., every 60s) reads agent store and resyncs

**Task selection skip** (`scheduler.rs:282-300`):

```rust
// Inside the task selection loop (no need to load agents here):
if task.auto_paused {
    skipped.push((task_id, task.next_execution, "auto_paused"));
    continue;
}
```

**Partial execution handling** (inside `Ok(record)` arm at scheduler.rs:397+):

```rust
match result {
    Ok(record) => {
        // Reset transient failure counter on any non-failure outcome
        agent.consecutive_failures = 0;

        if record.status == ExecutionStatus::Partial {
            // Degrade — don't increment permanent counter (rule-based still ran)
            // Don't reset auto_paused either — need clean success to recover
            tracing::info!(
                agent_id = %agent_id,
                "Agent execution degraded (Partial), counters unchanged"
            );
        } else if record.status == ExecutionStatus::Completed {
            // Clean success — reset all counters and clear auto_paused
            agent.permanent_failure_count = 0;
            agent.auto_paused = false;
            if let Some(task) = self.tasks.get(&agent_id) {
                task.write().await.auto_paused = false;
            }
        }
        // ... existing record storage ...
    }
    Err(e) => {
        let is_permanent = is_permanent_llm_error(&e);
        if is_permanent {
            // ... permanent error path as above ...
        } else {
            // ... existing transient retry path ...
        }
    }
}
```

**Helper to extract LlmError from `NeoMindError::Llm(String)`**:

Since `NeoMindError::Llm(String)` loses structured info, but the error message includes the status code (e.g., `"API error 403: ..."`), classify with a regex fallback:

```rust
fn is_permanent_llm_error(e: &NeoMindError) -> bool {
    let NeoMindError::Llm(msg) = e else { return false; };
    // The error string format from LlmError::Api display: "API error {status}: {body}"
    // Conservative match: only treat clearly-permanent statuses as permanent.
    // Falls back to transient for ambiguous cases (safer).
    if let Some(status) = extract_status_from_message(msg) {
        return status >= 400 && status < 500 && status != 429;
    }
    // Also detect by keyword for non-HTTP errors
    msg.contains("Backend unavailable")
        || msg.contains("Model not found")
        || msg.contains("Context overflow")
        || msg.contains("quota")
        || msg.contains("exhausted")
        || msg.contains("AllocationQuota")
        || msg.contains("unauthorized")
        || msg.contains("invalid api key")
}
```

This is a bridge solution. Long-term: change `NeoMindError::Llm(String)` to `NeoMindError::Llm(LlmError)` to preserve structure end-to-end. Tracked as a follow-up.

**Rate-limited auto-pause Message** (avoid spam if many agents fail simultaneously):

Add new field to `AgentScheduler` (`scheduler.rs:113-131`):

```rust
pub struct AgentScheduler {
    // ...existing fields...
    /// Dedup window for auto-pause Messages: agent_id → last emit timestamp.
    /// Prevents spamming channels if many agents fail simultaneously.
    message_dedup: Arc<RwLock<HashMap<String, i64>>>,
}

async fn emit_auto_pause_message(&self, agent: &AiAgent) {
    // Per-agent_id cooldown (60s), same pattern as event_trigger dedup
    let key = format!("auto_pause:{}", agent.id);
    let now = chrono::Utc::now().timestamp();
    {
        let dedup = self.message_dedup.read().await;
        if let Some(&last) = dedup.get(&key) {
            if now - last < 60 {
                return;  // Already notified recently
            }
        }
    }
    self.message_dedup.write().await.insert(key, now);
    // ... emit Message via existing channel system ...
}
```

**Recovery flow (no UI button)**:

1. User sees Message: "Agent X 已连续 3 次永久错误，自动暂停"
2. User fixes root cause (top up quota, fix API key, change model)
3. User clicks existing "Run now" button on agent detail page
4. "Run now" bypasses scheduler → calls executor directly → executes (auto_paused check skipped for manual)
5. If success: scheduler sees `ExecutionStatus::Completed`, resets `permanent_failure_count = 0`, `auto_paused = false`
6. Agent resumes normal scheduling

If "Run now" still fails, user gets another normal Failed execution record and can iterate.

### 4.7 Frontend Changes

**Backend already has `Partial`** in `ExecutionStatus` (`agents.rs:375-384`) — no backend enum change needed. Frontend-only work.

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
    <div className="mb-4 p-3 rounded-lg border bg-warning-light border-warning">
        <div className="flex items-center gap-2 font-medium text-warning">
            <AlertCircle className="h-4 w-4" />
            {t('agents:execution.degradedBanner')}
        </div>
        <p className="text-sm mt-1 text-muted-foreground">{execution.error}</p>
    </div>
)}
{execution.status === 'Failed' && execution.error && (
    <div className="mb-4 p-3 rounded-lg border bg-error-light border-error">
        <div className="flex items-center gap-2 font-medium text-error">
            <XCircle className="h-4 w-4" />
            {t('agents:execution.failedBanner')}
        </div>
        <p className="text-sm mt-1 text-muted-foreground font-mono break-words">{execution.error}</p>
    </div>
)}
```

**Token note**: Per DESIGN_SPEC.md, text on colored backgrounds should use `text-primary-foreground`. For inline colored icons/labels, use the semantic color directly (`text-warning`, `text-error`). The `*-foreground` suffixed tokens (`text-warning-foreground`, `text-error-foreground`) are not in the standard set — using `text-muted-foreground` for body text on tinted backgrounds is the established pattern. Verify final classes against existing components in the codebase.

**`AiAgent` type extension** (`web/src/types/index.ts`):

```typescript
export interface AiAgent {
    // ...existing fields...
    permanent_failure_count?: number;
    auto_paused?: boolean;
}
```

**Agent detail page** — show auto-paused banner:

```tsx
{agent.auto_paused && (
    <div className="mb-4 p-3 rounded-lg border bg-error-light border-error">
        <div className="flex items-center gap-2 font-medium text-error">
            <PauseCircle className="h-4 w-4" />
            {t('agents:detail.autoPausedBanner', {
                count: agent.permanent_failure_count,
                defaultValue: '已自动暂停（连续 {{count}} 次永久错误）'
            })}
        </div>
        <p className="text-sm mt-1 text-muted-foreground">
            {t('agents:detail.autoPausedHint', {
                defaultValue: '修复后点击"立即运行"，成功后将自动恢复调度'
            })}
        </p>
    </div>
)}
```

The existing "立即运行 / Run now" button already exists and bypasses scheduler — no new UI affordance needed for recovery.

**No changes to Message/channel UI** — existing components already render messages from any source.

### 4.8 Data Migration

None. redb is schema-less. New fields `permanent_failure_count` and `auto_paused` on `Agent` use `#[serde(default)]`, so loading legacy agent records yields `0` and `false`. The new `auto_paused: bool` on `ScheduledTask` is transient (rebuilt on each scheduler start from agent store).

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

1. **LlmError variant + is_permanent()** — pure type change, no behavior change. CI must pass. (`crates/neomind-core/src/llm/backend.rs`)
2. **Cloud backend updates** — replace `Generation(format!("API error N: ..."))` with `Api { status, body }`. ~10 sites across `openai.rs` and `ollama.rs`. CI must pass (error message format slightly changes).
3. **Inner LLM function signatures** — `analyze_with_llm` and `execute_with_tools` return `Result<_, LlmError>` instead of `Result<_, NeoMindError>`. Boundary conversion happens in `analyze_situation_with_intent`. This is the most invasive step but isolated to 2-3 files.
4. **AnalysisResult extension + analyzer branch logic** — add `degraded_reason`, change fallthrough paths. Integration tests for permanent-fail, transient-degrade, rule-only-passthrough, runtime-load-fail.
5. **execute_internal wiring** — propagate `degraded_reason` to `ExecutionRecord.status = Partial` + `error` field. Verify `AgentStatus::Error` guard doesn't trigger for Partial.
6. **Helper `is_permanent_llm_error(&NeoMindError)`** — regex-based extraction (bridge solution) for scheduler. Add unit tests for all common error message patterns.
7. **Scheduler auto-pause** — add `permanent_failure_count`/`auto_paused` to Agent + `auto_paused` to ScheduledTask. Implement skip logic, counter, retry-permanent-bypass, rate-limited Message emission.
8. **Frontend Partial status + banners + agent detail auto-paused banner + i18n** — visual polish only, last.
9. **Documentation update** — add note to `docs/guides/en/03-agent.md` about auto-pause behavior and recovery flow.

Step 1-2 ship first as a no-behavior-change refactor (zero risk). Steps 3-5 ship together as the core behavior change (most risk). Steps 6-7 ship as scheduler integration. Step 8-9 are polish.

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

- **Risk** (added Rev 2): Changing `analyze_with_llm` / `execute_with_tools` signatures to return `Result<_, LlmError>` is a larger refactor than initially scoped.
  **Mitigation**: Build sequence stages this as step 3, after no-behavior-change LlmError variant work. Alternative: keep `NeoMindError::Llm(String)` and use the regex bridge (section 4.6 helper) throughout — less clean but less invasive. Decision deferred to implementation phase based on actual refactor scope.

- **Risk** (added Rev 2): `is_permanent_llm_error(&NeoMindError)` regex bridge is fragile (depends on error message format).
  **Mitigation**: Long-term fix is changing `NeoMindError::Llm(String)` → `NeoMindError::Llm(LlmError)` to preserve structure. Tracked as follow-up. The bridge has unit tests for all known message patterns.

- **Risk** (added Rev 2): Cached `auto_paused: bool` on `ScheduledTask` could drift from `agent.auto_paused` if the agent is updated through non-scheduler paths.
  **Mitigation**: All agent updates go through `executor.store().update_agent()` which triggers `schedule_agent` re-registration. Sync point is in `schedule_agent`. Optional 60s reconcile loop adds safety.
