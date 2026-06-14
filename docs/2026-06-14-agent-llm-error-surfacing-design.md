# Agent LLM Error Surfacing — Design Spec

**Date:** 2026-06-14
**Status:** Approved (brainstormed, simplified Rev 4)
**Author:** Claude + shenmingming

> **Design philosophy (per user)**: "每一次的执行记录正确的就正确 错误的就错误 只是错误信息要明确 就解决很多问题了"
>
> Every execution record is either correct or failed. Make error messages explicit. No additional state machines, no auto-pause, no banners — the existing execution record + Failed badge + `error` field already gives users everything they need.

## 1. Problem

When an AI agent's LLM call fails (e.g., quota exhausted → HTTP 403, network blip, API key invalid), the executor **silently falls back** to rule-based analysis (`crates/neomind-agent/src/ai_agent/executor/analyzer.rs:103-110` and `analyzer.rs:76-83`). The fallback returns template text:

- `situation_analysis` = `"Analyzing N data points for agent '...'"`
- `decisions` = `[]`
- `conclusion` = `"No actions required - conditions not met"`

`execute_internal` then marks the execution as `Completed` with no error indication. Users see what looks like a successful agent run that "didn't trigger any conditions," while the real cause is buried in server logs.

The current `LlmError` enum also lacks structured HTTP status — error codes are baked into `LlmError::Generation(format!("API error 403: ..."))` strings, making reliable classification impossible.

## 2. Goals

1. **Surface LLM failures explicitly** as Failed execution records with the real error message.
2. **Preserve structured error info** (HTTP status code) end-to-end so logs can classify failures.
3. **Zero new state machines** — no auto-pause, no degraded status, no new agent struct fields.
4. **Minimal frontend changes** — existing Failed badge + existing `execution.error` text display already do the job.

## 3. Non-Goals

- Degrade mode (Partial status) — A vs B design discussion, user chose A: all LLM errors fail uniformly.
- Auto-pause after consecutive failures — user explicitly rejected as over-engineering.
- Banner UI components of any kind — existing UI surfaces suffice.
- Chat streaming path error handling (already surfaces errors directly).
- Tool execution error surfacing (existing tool-loop handling unchanged).
- Scheduler changes (no auto-pause means no scheduler integration).
- Recovery flow — no paused state means no recovery needed.
- Message push notifications — execution records are the source of truth.
- `NeoMindError::Llm(LlmError)` structural change — deferred (see §6 Bridge).

## 4. Architecture

### 4.1 LlmError Variant + Classification

**New `LlmError::Api` variant** (`crates/neomind-core/src/llm/backend.rs:416`):

```rust
#[error("API error {status}: {body}")]
Api {
    status: u16,
    body: String,
},
```

**Classification helper** (used in logs, optional for future scheduler improvements):

```rust
impl LlmError {
    /// Permanent errors require user action. Transient errors may succeed on retry.
    /// Note: classification is informational only in this design — both paths
    /// result in Failed execution. The distinction is preserved for logging
    /// clarity and any future retry/backoff work.
    pub fn is_permanent(&self) -> bool {
        match self {
            Self::BackendUnavailable(_)
            | Self::ModelNotFound(_)
            | Self::InvalidInput(_)
            | Self::ContextOverflow { .. }
            | Self::Serialization(_) => true,
            Self::Api { status, .. } => *status >= 400 && *status < 500 && *status != 429,
            Self::Timeout(_)
            | Self::Network(_)
            | Self::Io(_)
            | Self::Generation(_)
            | Self::Unknown(_) => false,
        }
    }
}
```

### 4.2 Cloud Backend Updates

Replace ~10 error-emitting sites in 2 files (most cloud providers share the OpenAI-compatible code path):

- `crates/neomind-agent/src/llm_backends/backends/openai.rs:677, 819, 1013, 1235`
- `crates/neomind-agent/src/llm_backends/backends/ollama.rs` (similar pattern, if applicable)

Before:
```rust
return Err(LlmError::Generation(format!("API error {}: {}", status, body)));
```

After:
```rust
return Err(LlmError::Api { status: status.as_u16(), body });
```

For non-HTTP errors (connection refused, timeout), keep existing `Network`/`Timeout` variants.

### 4.3 Inner Function Signatures

**Critical**: `NeoMindError::Llm(String)` currently stringifies `LlmError` at every callsite (e.g., analyzer.rs:272-274 uses `format!("LLM generation failed: {}", e)`). The structured `LlmError` is lost before reaching `analyze_situation_with_intent`, so `is_permanent()` cannot be called.

**Fix**: Inner LLM functions return `Result<_, LlmError>` and only convert at the outer `analyze_situation_with_intent` boundary.

Update signatures:
- `analyze_with_llm` (`analyzer.rs:143`): `Result<_, LlmError>` instead of `Result<_, NeoMindError>`
- `execute_with_tools` (`mod.rs:424`): LLM-specific failure path returns `Result<_, LlmError>`

Inside these functions, remove `?` conversions that wrap as `NeoMindError::Llm(String)` and let `LlmError` propagate naturally.

### 4.4 Analyzer Branch Logic (Simplified)

`analyze_situation_with_intent` (`analyzer.rs:42-135`) — **all silent fallbacks removed**:

```rust
pub(crate) async fn analyze_situation_with_intent(
    &self, agent: &AiAgent, data: &[DataCollected],
    parsed_intent: Option<&ParsedIntent>, execution_id: &str,
    invocation_input: Option<&AgentInput>,
) -> AgentResult<AnalysisResult> {
    use neomind_core::llm::backend::LlmError;

    match self.get_llm_runtime_for_agent(agent).await {
        Ok(Some(llm)) => {
            if self.should_use_tools(agent, &llm) {
                match self.execute_with_tools(...).await {
                    Ok((dp, er)) => return Ok(AnalysisResult::Free { decision_process: dp, execution_result: er }),
                    Err(e) => {
                        tracing::warn!(error = %e, permanent = e.is_permanent(), "LLM tool-based analysis failed");
                        return Err(NeoMindError::Llm(format!("{}", e)));
                    }
                }
            }
            match self.analyze_with_llm(...).await {  // Now returns Result<_, LlmError>
                Ok(r) => return Ok(AnalysisResult::Focused { ..r }),
                Err(e) => {
                    tracing::warn!(error = %e, permanent = e.is_permanent(), "LLM analysis failed");
                    return Err(NeoMindError::Llm(format!("{}", e)));
                }
            }
        }
        Ok(None) => {
            // Distinguish rule-only agents (expected) from misconfigured agents
            if agent.llm_backend_id.is_some() {
                return Err(NeoMindError::Validation(format!(
                    "Agent '{}' 配置了 LLM 后端但运行时不可用（可能已被删除或 API key 未配置）",
                    agent.name
                )));
            }
            // Rule-only agent: proceed to rule-based analysis below
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to load LLM runtime");
            return Err(e);
        }
    }

    // Only reached for rule-only agents (no llm_backend_id)
    let (sa, rs, ds, con) = self.analyze_rule_based(agent, data, parsed_intent).await?;
    Ok(AnalysisResult::Focused {
        situation_analysis: sa,
        reasoning_steps: rs,
        decisions: ds,
        conclusion: con,
    })
}
```

**Key behavioral changes vs current code**:

| Scenario | Before | After |
|---|---|---|
| LLM tool mode fails | Warn, fall through to LLM direct | **Fail with error** |
| LLM direct analysis fails | Warn, fall through to rule-based | **Fail with error** |
| `get_llm_runtime_for_agent` errors | Warn, fall through | **Fail with error** |
| `Ok(None)` + agent has `llm_backend_id` | Warn, fall through | **Fail with validation error** |
| `Ok(None)` + rule-only agent | Fall through (normal) | Fall through (unchanged) |

Rule-only agents (no `llm_backend_id`) continue to work without LLM — backward compatibility preserved.

### 4.5 execute_internal — Existing Error Path is Sufficient

The existing outer `Err(e)` handler in `execute_internal` already:
1. Sets execution status to `Failed`
2. Writes `success: false` journal entry (per MEMORY.md "Error path journal write")
3. Returns Err to caller

**No new code needed in `execute_internal`** — just verify the outer error handler populates `execution_record.error = Some(format!("{}", e))` before storing. If it doesn't, add a one-line fix.

The existing mod.rs:942-946 guard `if Completed {Active} else {Error}` is correct as-is — Failed correctly maps to `AgentStatus::Error`.

### 4.6 Frontend Changes

**None.**

- `web/src/types/index.ts:2065` — `ExecutionStatus` already has `'Failed'`. No Partial addition.
- `web/src/pages/agents-components/AgentExecutionTimeline.tsx:213-214` — Failed badge (red XCircle) already exists.
- `web/src/pages/agents-components/ExecutionDetailDialog.tsx:293-297` — Already displays `execution.error` text via `break-words`.

The only thing the user will see differently: failed executions now have **real error messages** instead of falling through to "No actions required - conditions not met" template text.

### 4.7 Data Migration

None.

## 5. Testing

### 5.1 Unit Tests (`crates/neomind-core/src/llm/backend.rs`)

- `is_permanent()` for every `LlmError` variant
- `Api { status: 401/403/404 }` → permanent
- `Api { status: 429/500/503 }` → transient

### 5.2 Integration Tests (`crates/neomind-agent/`)

- **LLM error propagation**: Mock LLM runtime returns `Api { status: 403 }`. Assert execution `Failed` with error message containing "API error 403".
- **Tool-mode failure**: Mock `execute_with_tools` returns `LlmError::Timeout`. Assert execution `Failed`.
- **Rule-only agent**: No `llm_backend_id`. Assert agent runs normally without LLM, no error.
- **Misconfigured agent**: Has `llm_backend_id` but backend deleted. Assert validation error.

### 5.3 Manual Verification

1. Configure agent with GLM backend
2. Set invalid API key or wait for quota exhaustion
3. Trigger agent execution
4. Verify: execution record status=`Failed`, error message in detail dialog shows real cause (e.g., "API error 403: ... free tier exhausted")
5. Verify: timeline shows red badge with no false "no actions required" output

## 6. Build Sequence

1. **LlmError variant + is_permanent()** — pure type change. CI must pass.
2. **Cloud backend updates** — replace `Generation(format!(...))` with `Api { status, body }`. CI must pass.
3. **Inner function signatures** — `analyze_with_llm` / `execute_with_tools` return `Result<_, LlmError>`. Most invasive but isolated to 2-3 files.
4. **Analyzer branch logic** — remove silent fallbacks, propagate errors. Integration tests per scenario.
5. **Verify execute_internal error path** — ensure `execution_record.error` is populated. One-line fix if missing.
6. **Documentation note** — add a paragraph to `docs/guides/en/03-agent.md` explaining that LLM failures now surface as Failed executions.

## 7. Bridge Solution / Future Work

The `NeoMindError::Llm(String)` shape loses structured error info at the boundary. This design uses `format!("{}", e)` as a bridge — the Display impl of `LlmError::Api` produces `"API error {status}: {body}"`, which is human-readable and parseable.

**Future improvement** (out of scope here): change `NeoMindError::Llm(String)` to `NeoMindError::Llm(LlmError)` to preserve structure end-to-end. Tracked as a separate refactor. Not needed for this design because no consumer currently pattern-matches on the error structure beyond logging.

## 8. Risks

- **Risk**: Existing tests assert specific error message strings (e.g., `"API error 403"`).
  **Mitigation**: The Display impl of `LlmError::Api` produces the same format. Grep for `"API error"` in tests; the format string `[error("API error {status}: {body}")]` matches.

- **Risk**: Changing inner function signatures is invasive.
  **Mitigation**: Build sequence stages this as step 3 with integration tests. If refactor proves too large, fall back to regex-based extraction in `analyze_situation_with_intent` (parse `"API error (\\d+)"` from the stringified `NeoMindError::Llm`). Less clean but unblocks the design without signature changes.

- **Risk**: Rule-only agents break if they accidentally have `llm_backend_id` set.
  **Mitigation**: The check is `agent.llm_backend_id.is_some()`. Legacy rule-only agents created before LLM backend support will have `None`. New agents created via UI typically get a backend ID; if user wants rule-only, they should not select a backend. Document this in the agent creation UI.

- **Risk**: Transient errors (network blip, 5xx) now cause Failed executions instead of silent degrade. Users may see more "Failed" entries than before.
  **Mitigation**: This is the intended behavior — user explicitly chose transparency over silent degradation. The error message distinguishes transient vs permanent causes so users can prioritize.
