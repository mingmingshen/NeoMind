# Agent LLM Error Surfacing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop silently degrading to rule-based analysis when LLM calls fail; surface real errors (HTTP 403, quota exhausted, network, etc.) as Failed execution records with explicit error messages.

**Architecture:** Three layered changes: (1) `LlmError::Api { status, body }` variant preserves HTTP status code end-to-end, (2) cloud backends emit the new variant instead of stringified `Generation(format!(...))`, (3) analyzer removes 3 silent-fallback paths so LLM errors propagate as `Err` to the existing Failed-record handler. No frontend, scheduler, or state-machine changes — the existing `Failed` badge and `execution.error` text display already work.

**Tech Stack:** Rust (Axum, thiserror, tokio), neomind-core / neomind-agent crates

**Spec:** `docs/2026-06-14-agent-llm-error-surfacing-design.md`

---

## Plan-Specific Refinements Over Spec

The spec section 4.3-4.4 proposes changing `analyze_with_llm` / `execute_with_tools` signatures to `Result<_, LlmError>`. **This plan refines that to a simpler approach** because:

1. Per Option A (chosen by user), ALL LLM errors fail uniformly. The `is_permanent()` classification is **log-only** — it doesn't affect runtime behavior.
2. Logging classification at the inner LLM call site (where `LlmError` is still structured) achieves the same observability without signature changes.
3. `execute_with_tools` has non-LLM failure modes (tool registry missing) that don't fit `Result<_, LlmError>`.

**Refinement**: Keep all signatures as `AgentResult<...>`. Add a `tracing::warn!` with `permanent = e.is_permanent()` at each inner LLM call site. Propagate as `NeoMindError::Llm(format!("{}", e))` unchanged. Boundary in `analyze_situation_with_intent` simply propagates `Err` without classification.

This avoids: signature changes, new `NeoMindError::LlmStructured(LlmError)` variant, regex parsing of error strings. Same end-user behavior.

---

## File Structure

| File | Responsibility | Change Type |
|---|---|---|
| `crates/neomind-core/src/llm/backend.rs` | Add `Api` variant + `is_permanent()` + unit tests | Modify (additive) |
| `crates/neomind-agent/src/llm_backends/backends/openai.rs` | 5 HTTP-status sites: replace `Generation(format!("API error N: ..."))` with `Api { status, body }` | Modify |
| `crates/neomind-agent/src/llm_backends/backends/ollama.rs` | 2-3 sites: same pattern | Modify |
| `crates/neomind-agent/src/ai_agent/executor/analyzer.rs` | Remove silent fallbacks in `analyze_situation_with_intent`; add classification log in `analyze_with_llm` | Modify |
| `crates/neomind-agent/src/ai_agent/executor/mod.rs` | Add `last_llm_error` field to `ToolLoopOutput`; use it in `execute_with_tools` to surface real error | Modify |
| `crates/neomind-agent/src/ai_agent/executor/tool_loop.rs` | Capture `LlmError` into `ToolLoopOutput`; add `permanent =` field to warn log | Modify |
| `crates/neomind-agent/src/ai_agent/executor/tool_result.rs` | Update destructure to handle new `last_llm_error` field | Modify (1 line) |
| `docs/guides/en/03-agent.md` | Add note about Failed status on LLM errors | Modify (docs) |

**No changes to**: `crates/neomind-core/src/error/mod.rs` (NeoMindError unchanged), frontend (Failed badge + error text already exist), scheduler, agent struct.

---

## Task 1: Add `LlmError::Api` Variant and `is_permanent()` Method

**Files:**
- Modify: `crates/neomind-core/src/llm/backend.rs` (around line 416-461 for enum, end of file for impl + tests)

- [ ] **Step 1.1: Write the failing unit tests**

Append to `crates/neomind-core/src/llm/backend.rs` (after the existing `BackendMetrics` impl, before any `#[cfg(test)]` module or at end of file):

```rust
#[cfg(test)]
mod error_classification_tests {
    use super::LlmError;

    #[test]
    fn permanent_variants() {
        assert!(LlmError::BackendUnavailable("ollama".into()).is_permanent());
        assert!(LlmError::ModelNotFound("qwen3.5:4b".into()).is_permanent());
        assert!(LlmError::InvalidInput("bad request".into()).is_permanent());
        assert!(LlmError::ContextOverflow { prompt_tokens: 10000, max_context: 8000 }.is_permanent());
        assert!(LlmError::Serialization(serde_json::from_str::<i32>("not a number").unwrap_err()).is_permanent());
    }

    #[test]
    fn permanent_http_statuses() {
        assert!(LlmError::Api { status: 400, body: "".into() }.is_permanent());
        assert!(LlmError::Api { status: 401, body: "".into() }.is_permanent());
        assert!(LlmError::Api { status: 403, body: "quota exhausted".into() }.is_permanent());
        assert!(LlmError::Api { status: 404, body: "".into() }.is_permanent());
    }

    #[test]
    fn transient_http_statuses() {
        assert!(!LlmError::Api { status: 429, body: "rate limited".into() }.is_permanent());
        assert!(!LlmError::Api { status: 500, body: "".into() }.is_permanent());
        assert!(!LlmError::Api { status: 502, body: "".into() }.is_permanent());
        assert!(!LlmError::Api { status: 503, body: "".into() }.is_permanent());
    }

    #[test]
    fn transient_variants() {
        assert!(!LlmError::Timeout(60).is_permanent());
        assert!(!LlmError::Network("connection refused".into()).is_permanent());
        assert!(!LlmError::Generation("legacy fallback".into()).is_permanent());
        assert!(!LlmError::Unknown("something".into()).is_permanent());
    }

    #[test]
    fn api_variant_display_format() {
        let e = LlmError::Api { status: 403, body: "quota exhausted".into() };
        let s = format!("{}", e);
        assert!(s.contains("403"), "Display should include status: got {}", s);
        assert!(s.contains("quota exhausted"), "Display should include body: got {}", s);
    }
}
```

- [ ] **Step 1.2: Run tests to verify they fail**

Run: `cargo test -p neomind-core --lib error_classification_tests`
Expected: FAIL — `variant `Api` not found`, method `is_permanent` not found.

- [ ] **Step 1.3: Add the `Api` variant to `LlmError`**

In `crates/neomind-core/src/llm/backend.rs`, find the `pub enum LlmError` (line 416). Insert after `Generation(String)` (line 431):

```rust
    /// HTTP API error with status code and response body.
    /// Used by cloud backends to preserve status code for classification.
    #[error("API error {status}: {body}")]
    Api {
        /// HTTP status code (e.g., 401, 403, 429, 500).
        status: u16,
        /// Raw response body (may be JSON, may be empty).
        body: String,
    },
```

- [ ] **Step 1.4: Add `is_permanent()` method**

Append a new `impl LlmError` block at the end of the file (or add to existing one). Check first if there's an existing `impl LlmError` to extend:

```rust
impl LlmError {
    /// Classify whether this error requires user action (permanent) or may
    /// succeed on retry (transient).
    ///
    /// Used for log clarity only — both classes propagate as Failed execution
    /// per the agent error surfacing design (Rev 4, Option A).
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

- [ ] **Step 1.5: Run tests to verify they pass**

Run: `cargo test -p neomind-core --lib error_classification_tests`
Expected: PASS — 5 tests pass.

- [ ] **Step 1.6: Run full workspace check**

Run: `cargo build -p neomind-core`
Expected: clean build (variant addition is purely additive).

- [ ] **Step 1.7: Commit**

```bash
git add crates/neomind-core/src/llm/backend.rs
git commit -m "feat(core): add LlmError::Api variant and is_permanent()

Structured HTTP error variant preserves status code end-to-end. Used by
cloud backends (next commit) to replace stringified
Generation(format!(\"API error N: ...\")) patterns. Classification
helper supports log-level permanent/transient distinction.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 2: Migrate OpenAI Backend Error Sites to `Api` Variant

**Files:**
- Modify: `crates/neomind-agent/src/llm_backends/backends/openai.rs` — 5 HTTP-status sites at lines 677, 819, 840, 1013, 1235 (plus line 1253 edge case)

- [ ] **Step 2.1: Locate all error sites by grepping**

Run: `grep -n 'LlmError::Generation(format!' crates/neomind-agent/src/llm_backends/backends/openai.rs`
Expected: 7 matches total. Of these:
- Lines 677, 819, 840, 1013, 1235: HTTP-status sites — **migrate to `Api` variant**
- Line 849: deserialization error (no HTTP status) — **leave as `Generation`**
- Line 1253: "unexpected JSON response" (Anthropic streaming, no clean status) — **leave as `Generation`** (edge case)

Also grep for the non-format rate-limit pattern:
Run: `grep -n 'LlmError::Generation("Rate' crates/neomind-agent/src/llm_backends/backends/openai.rs`
Expected: matches at lines 1005 and 1227. These can stay as `Generation("Rate limited by API".to_string())` — transient classification works regardless of variant, since `Generation` is classified as transient in `is_permanent()`.

- [ ] **Step 2.2: Read each migration site to confirm local variable names**

Read each migration location (677, 819, 840, 1013, 1235). The current pattern at each is:

```rust
return Err(LlmError::Generation(format!(
    "API error {}: {}",
    status.as_u16(),
    body
)));
```

The variable holding the body text is `body` (NOT `body_text`) at all 5 sites. Capture the exact text of each call to drive the Edit.

- [ ] **Step 2.3: Replace site at line 677**

In `crates/neomind-agent/src/llm_backends/backends/openai.rs`, replace the `LlmError::Generation(format!("API error {}: {}", status.as_u16(), body))` at line 677 with:

```rust
            return Err(LlmError::Api {
                status: status.as_u16(),
                body,
            });
```

(The key change: `LlmError::Generation(format!(...))` → `LlmError::Api { status, body }`. Variable `body` is moved into the struct.)

- [ ] **Step 2.4: Repeat for sites at 819, 840, 1013, 1235**

Each site follows the same `status.as_u16()` + `body` pattern. For each:

1. Read the surrounding 5-10 lines to confirm `status` is a `reqwest::StatusCode` and `body` is a `String`
2. Edit to use `LlmError::Api { status: status.as_u16(), body }`
3. **Line 840 special case**: This is the Anthropic "error payload inside HTTP 200" path. Verify that a `status` variable is in scope (it may need to be captured from `response.status()` before the Anthropic-specific parsing). If `status` is not readily available, construct `Api { status: 200, body }` — the body will contain the Anthropic error JSON.

- [ ] **Step 2.5: Build to verify no compilation errors**

Run: `cargo build -p neomind-agent`
Expected: clean build.

- [ ] **Step 2.6: Grep to confirm HTTP-status sites are migrated**

Run: `grep -n 'API error' crates/neomind-agent/src/llm_backends/backends/openai.rs`
Expected: zero matches in non-comment code (line 849 deserialization and 1253 unexpected-JSON sites are not HTTP-status sites and are correctly left as `Generation`).

- [ ] **Step 2.7: Run existing agent crate tests**

Run: `cargo test -p neomind-agent --lib`
Expected: PASS — no test should break (Display format `"API error {status}: {body}"` matches the prior format).

- [ ] **Step 2.8: Commit**

```bash
git add crates/neomind-agent/src/llm_backends/backends/openai.rs
git commit -m "refactor(agent): openai backend uses LlmError::Api

Replaces 5 sites of LlmError::Generation(format!(\"API error N: ...\"))
with structured LlmError::Api { status, body }. Display output is
unchanged (\"API error N: ...\"), so log/error consumers see no
difference. Status code now preserved for is_permanent() classification.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 3: Migrate Ollama Backend Error Sites to `Api` Variant

**Files:**
- Modify: `crates/neomind-agent/src/llm_backends/backends/ollama.rs` — sites at lines 632, 908, 948

- [ ] **Step 3.1: Locate all error sites**

Run: `grep -n 'LlmError::Generation(format!' crates/neomind-agent/src/llm_backends/backends/ollama.rs`
Expected: 3 matches (632, 908, 948).

- [ ] **Step 3.2: Read each site**

Lines 632-634 (non-streaming):
```rust
            return Err(LlmError::Generation(format!(
                "Ollama API error {}: {}",
                status.as_u16(),
                ...
            )));
```

Lines 908-910 (streaming):
```rust
                    .send(Err(LlmError::Generation(format!(
                        ...
                        status.as_u16(),
                        ...
```

Line 948 (streaming, different format):
```rust
                    .send(Err(LlmError::Generation(error_msg)).await;
```

- [ ] **Step 3.3: Replace line 632-634 site**

Edit to:
```rust
            return Err(LlmError::Api {
                status: status.as_u16(),
                body: <body_text_variable>,
            });
```

(Check the actual variable name for the body — might be `body`, `body_text`, `error_text`, etc.)

- [ ] **Step 3.4: Replace line 908-910 site**

Same pattern, but note the `tx.send(Err(...)).await` context — preserve the channel send.

- [ ] **Step 3.5: Replace line 948 site**

This site uses a pre-built `error_msg` string. Look up its definition to see if status code is captured. If yes, change to `Api { status: ..., body: error_msg }`. If not (just a generic message), leave as `Generation` — it's transient classification anyway and not worth refactoring.

- [ ] **Step 3.6: Build and verify**

Run: `cargo build -p neomind-agent`
Expected: clean build.

- [ ] **Step 3.7: Grep to confirm HTTP-status sites are migrated**

Run: `grep -n 'API error' crates/neomind-agent/src/llm_backends/backends/ollama.rs`
Expected: zero matches (or only the generic line 948 path).

- [ ] **Step 3.8: Commit**

```bash
git add crates/neomind-agent/src/llm_backends/backends/ollama.rs
git commit -m "refactor(agent): ollama backend uses LlmError::Api

Same migration as openai backend. Status code preserved for
is_permanent() classification.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 4: Remove Silent Fallbacks in Analyzer

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor/analyzer.rs` — `analyze_situation_with_intent` (lines 42-135) and `analyze_with_llm` LLM call site (around line 272)

This is the core behavioral change. Before this task: LLM failures silently fall through to rule-based analysis. After: LLM failures propagate as `Err` to the outer handler, which builds a Failed ExecutionRecord.

- [ ] **Step 4.1: Add a classification log at the LLM call site inside `analyze_with_llm`**

In `crates/neomind-agent/src/ai_agent/executor/analyzer.rs`, find the LLM call around line 256-275 (the `tokio::time::timeout(...)` block). The current failure mapping is:

```rust
let llm_result = match tokio::time::timeout(
    std::time::Duration::from_secs(LLM_TIMEOUT_SECS),
    llm.generate(input),
)
.await
{
    Ok(result) => result,
    Err(_) => {
        tracing::warn!(agent_id = %agent.id, "LLM timed out after {}s", LLM_TIMEOUT_SECS);
        return Err(NeoMindError::Llm(format!(
            "LLM timeout after {}s",
            LLM_TIMEOUT_SECS
        )));
    }
};

let output = llm_result.map_err(|e| {
    tracing::error!(agent_id = %agent.id, error = %e, "LLM generation failed");
    NeoMindError::Llm(format!("LLM generation failed: {}", e))
})?;
```

Replace the `map_err` block with classification logging:

```rust
let output = llm_result.map_err(|e| {
    tracing::warn!(
        agent_id = %agent.id,
        error = %e,
        permanent = e.is_permanent(),
        "LLM generation failed"
    );
    NeoMindError::Llm(format!("{}", e))
})?;
```

Changes:
- Log level: `error!` → `warn!` (warns are appropriate for expected failure modes; errors reserved for bugs)
- Added `permanent = e.is_permanent()` field for log classification
- Simplified message: removed redundant "LLM generation failed:" prefix (already in `error = %e`)

- [ ] **Step 4.2: Rewrite `analyze_situation_with_intent` to remove silent fallbacks**

In `crates/neomind-agent/src/ai_agent/executor/analyzer.rs`, find `analyze_situation_with_intent` (line 26). Replace lines 42-135 (the entire `match` block + fallthrough) with:

```rust
        match self.get_llm_runtime_for_agent(agent).await {
            Ok(Some(llm)) => {
                tracing::info!(
                    agent_id = %agent.id,
                    "LLM runtime available, performing LLM-based analysis"
                );

                // Check if tool/function-calling mode should be used
                if self.should_use_tools(agent, &llm) {
                    tracing::info!(
                        agent_id = %agent.id,
                        "Tool mode enabled - using function calling"
                    );
                    return self
                        .execute_with_tools(
                            agent,
                            data,
                            llm.clone(),
                            execution_id,
                            invocation_input,
                        )
                        .await
                        .map(|(dp, exec_result)| {
                            tracing::info!(
                                agent_id = %agent.id,
                                "Tool-based analysis completed successfully"
                            );
                            AnalysisResult::Free {
                                decision_process: dp,
                                execution_result: exec_result,
                            }
                        })
                        .map_err(|e| {
                            tracing::warn!(
                                agent_id = %agent.id,
                                error = %e,
                                "Tool-based analysis failed"
                            );
                            e
                        });
                }

                // Standard LLM-based analysis
                return self
                    .analyze_with_llm(llm, agent, data, parsed_intent, execution_id)
                    .await
                    .map(|(sa, rs, ds, con)| {
                        tracing::info!(
                            agent_id = %agent.id,
                            "LLM-based analysis completed successfully"
                        );
                        AnalysisResult::Focused {
                            situation_analysis: sa,
                            reasoning_steps: rs,
                            decisions: ds,
                            conclusion: con,
                        }
                    })
                    .map_err(|e| {
                        tracing::warn!(
                            agent_id = %agent.id,
                            error = %e,
                            "LLM analysis failed"
                        );
                        e
                    });
            }
            Ok(None) => {
                if agent.llm_backend_id.is_some() {
                    // Agent expected an LLM but runtime is unavailable.
                    // Fail explicitly rather than silently degrading.
                    tracing::warn!(
                        agent_id = %agent.id,
                        backend_id = ?agent.llm_backend_id,
                        "Agent has llm_backend_id but runtime is unavailable"
                    );
                    return Err(NeoMindError::Validation(format!(
                        "Agent '{}' 配置了 LLM 后端但运行时不可用（可能已被删除或 API key 未配置）",
                        agent.name
                    )));
                }
                tracing::info!(
                    agent_id = %agent.id,
                    "Rule-only agent (no llm_backend_id), proceeding to rule-based analysis"
                );
                // Fall through to rule-based analysis below
            }
            Err(e) => {
                tracing::warn!(
                    agent_id = %agent.id,
                    error = %e,
                    "Failed to load LLM runtime"
                );
                return Err(e);
            }
        }

        // Only reached for rule-only agents (no llm_backend_id).
        let (situation_analysis, reasoning_steps, decisions, conclusion) =
            self.analyze_rule_based(agent, data, parsed_intent).await?;
        Ok(AnalysisResult::Focused {
            situation_analysis,
            reasoning_steps,
            decisions,
            conclusion,
        })
    }
```

**Key behavioral changes vs the prior code**:

| Path | Before | After |
|---|---|---|
| `Ok(Some(llm))` + tool-mode fails | `tracing::warn!` + fall through to LLM direct | `return Err(e)` |
| `Ok(Some(llm))` + LLM direct fails | `tracing::warn!` + fall through to rule-based | `return Err(e)` |
| `Ok(None)` + has `llm_backend_id` | Silent fallthrough to rule-based | `return Err(Validation(...))` |
| `Ok(None)` + rule-only | Fallthrough to rule-based | Unchanged (fallthrough) |
| `Err(_)` from runtime load | Silent fallthrough | `return Err(e)` |

- [ ] **Step 4.3: Build to verify compilation**

Run: `cargo build -p neomind-agent`
Expected: clean build. If compiler complains about unused imports (e.g., `AnalysisResult` is still used; but `tracing::warn!` macro might already be imported), clean up.

- [ ] **Step 4.4: Run existing analyzer tests**

Run: `cargo test -p neomind-agent --lib analyzer`
Expected: existing tests should still pass (the rule-only path is unchanged). If any test relied on the silent fallback behavior (e.g., "LLM error → rule-based output"), update the test to expect `Err`.

- [ ] **Step 4.5: Search for tests that asserted silent fallback**

Run: `grep -rn 'falling back\|rule-based\|conditions not met' crates/neomind-agent/tests/ crates/neomind-agent/src/ai_agent/executor/ | grep -i test`
Expected: any test asserting fallback behavior should be updated. If found, change the assertion to expect `Err(...)`.

- [ ] **Step 4.6: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor/analyzer.rs
git commit -m "feat(agent): remove silent LLM fallbacks in analyzer

LLM failures now propagate as Err to execute_internal's outer error
handler, which builds a Failed ExecutionRecord with the real error
message. Previously these failures silently fell through to rule-based
analysis, producing misleading 'No actions required - conditions not
met' template output.

Behavior changes:
- LLM tool-mode failure: return Err (was: fall through to LLM direct)
- LLM direct failure: return Err (was: fall through to rule-based)
- Runtime load failure: return Err (was: fall through to rule-based)
- Ok(None) + has llm_backend_id: validation error (was: silent degrade)
- Ok(None) + rule-only agent: unchanged (no LLM expected)

Rule-only agents (no llm_backend_id) continue to work without LLM.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 5: Surface Real LLM Error from Tool Loop

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor/mod.rs` — `ToolLoopOutput` struct (line 52) + `execute_with_tools` degraded-output branch (lines 533-542)
- Modify: `crates/neomind-agent/src/ai_agent/executor/tool_loop.rs` — LLM error capture (lines 111-131) + struct construction (lines 567-574)
- Modify: `crates/neomind-agent/src/ai_agent/executor/tool_result.rs` — destructure update (line 303-307)

**Why this task exists**: The tool loop currently swallows `LlmError` into `final_text = "LLM generation failed during tool execution."` and `break`s. The caller `execute_with_tools` then replaces this with a generic `NeoMindError::Llm("LLM tool-calling failed — falling back to direct analysis")` at line 539-541. The real error (e.g., "API error 403: quota exhausted") is lost. After Task 4 propagates this generic error as a Failed execution, the user sees "LLM tool-calling failed" instead of the actual cause — defeating the design's goal of exposing real errors.

This task captures the original `LlmError` in `ToolLoopOutput` so `execute_with_tools` can construct a meaningful error message.

- [ ] **Step 5.1: Add `last_llm_error` field to `ToolLoopOutput`**

In `crates/neomind-agent/src/ai_agent/executor/mod.rs`, update the struct at line 52-57:

```rust
pub(crate) struct ToolLoopOutput {
    pub(crate) final_text: String,
    pub(crate) all_tool_results: Vec<crate::toolkit::ToolResult>,
    /// (thought, tool_calls) per round
    pub(crate) round_data_list_raw: Vec<(Option<String>, Vec<ToolCallRecord>)>,
    /// Captured LLM error if `generate()` failed during the loop.
    /// Used by `execute_with_tools` to surface the real error cause instead
    /// of a generic "tool-calling failed" message.
    pub(crate) last_llm_error: Option<neomind_core::llm::backend::LlmError>,
}
```

- [ ] **Step 5.2: Capture the `LlmError` in `run_tool_loop`**

In `crates/neomind-agent/src/ai_agent/executor/tool_loop.rs`, the LLM call at line 111-131 currently looks like:

```rust
let output = match llm_runtime.generate(input).await {
    Ok(o) => o,
    Err(e) => {
        let round_num = round + 1;
        // ... existing tracing::warn! with extensive context ...
        final_text = "LLM generation failed during tool execution.".to_string();
        break;
    }
};
```

Add a `let mut last_llm_error: Option<LlmError> = None;` declaration near the top of `run_tool_loop` (alongside `let mut final_text = String::new();` — find that declaration and add alongside). Then update the error branch:

```rust
let output = match llm_runtime.generate(input).await {
    Ok(o) => o,
    Err(e) => {
        let round_num = round + 1;
        let msg_count = messages.len();
        let has_images = messages.iter().any(|m| {
            matches!(&m.content, Content::Parts(parts) if parts.iter().any(|p| matches!(p, ContentPart::ImageBase64 { .. } | ContentPart::ImageUrl { .. })))
        });
        tracing::warn!(
            agent_id = %agent.id,
            error = %e,
            permanent = e.is_permanent(),
            round = round_num,
            msg_count,
            has_images,
            model = %llm_runtime.model_name(),
            "LLM generation failed in tool loop"
        );
        last_llm_error = Some(e);
        final_text = "LLM generation failed during tool execution.".to_string();
        break;
    }
};
```

Changes:
- Added `permanent = e.is_permanent()` field to existing warn log
- Added `last_llm_error = Some(e);` to capture the structured error before it's dropped
- Trimmed log message (the "— will trigger fallback..." suffix is now inaccurate since Task 4 stops falling back)

You'll also need to add `LlmError` to the imports at the top of `tool_loop.rs` (line 9 area):

```rust
use neomind_core::llm::backend::{LlmError, LlmRuntime};
```

- [ ] **Step 5.3: Initialize the field in struct construction**

In `crates/neomind-agent/src/ai_agent/executor/tool_loop.rs` at line 567-574, update the `ToolLoopOutput` construction:

```rust
ToolLoopOutput {
    final_text,
    all_tool_results,
    round_data_list_raw: round_data_list
        .into_iter()
        .map(|rd| (rd.thought, rd.tool_calls))
        .collect(),
    last_llm_error,
}
```

- [ ] **Step 5.4: Update destructuring in `tool_result.rs`**

In `crates/neomind-agent/src/ai_agent/executor/tool_result.rs` at line 303-307, the existing destructure must handle the new field. Use `..` to ignore it (this function doesn't need the LLM error — it only runs on the success path):

```rust
let ToolLoopOutput {
    final_text,
    all_tool_results,
    round_data_list_raw,
    last_llm_error: _,
} = loop_output;
```

(Or use `..` if the function already uses it elsewhere — check the surrounding code.)

- [ ] **Step 5.5: Use `last_llm_error` in `execute_with_tools` to surface real error**

In `crates/neomind-agent/src/ai_agent/executor/mod.rs` at lines 533-542, replace the generic error construction:

```rust
if no_tools_executed && (llm_generation_failed || has_malformed_output) {
    tracing::info!(
        agent_id = %agent.id,
        malformed_output = has_malformed_output,
        "Tool-calling produced no results — falling back to direct LLM analysis"
    );
    return Err(NeoMindError::Llm(
        "LLM tool-calling failed — falling back to direct analysis".to_string(),
    ));
}
```

with:

```rust
if no_tools_executed && (llm_generation_failed || has_malformed_output) {
    // If we have the real LLM error, surface it. Otherwise use a generic
    // message for the malformed-output path (no LLM error captured).
    let msg = match loop_output.last_llm_error {
        Some(ref e) => format!("LLM tool-calling failed: {}", e),
        None if has_malformed_output => {
            "LLM tool-calling produced malformed output".to_string()
        }
        _ => "LLM tool-calling failed".to_string(),
    };
    tracing::warn!(
        agent_id = %agent.id,
        malformed_output = has_malformed_output,
        has_llm_error = loop_output.last_llm_error.is_some(),
        "Tool-calling failed — propagating error"
    );
    return Err(NeoMindError::Llm(msg));
}
```

You'll need to add `LlmError` to the imports in `mod.rs` (line 8 area):

```rust
use neomind_core::llm::backend::{LlmError, LlmRuntime};
```

(But only if Step 5.5's code actually references `LlmError` by name — the `match` uses `ref e` and Display, so the import may not be strictly needed. Let the compiler tell you.)

- [ ] **Step 5.6: Build and verify**

Run: `cargo build -p neomind-agent`
Expected: clean build. If the compiler complains about unused imports, remove them. If it complains about the destructure in `tool_result.rs`, adjust per Step 5.4.

- [ ] **Step 5.7: Run tool-loop and executor tests**

Run: `cargo test -p neomind-agent --lib`
Expected: PASS (existing tests that asserted the generic "LLM tool-calling failed — falling back to direct analysis" message may break — if so, update them to the new message format).

- [ ] **Step 5.8: Grep for tests asserting the old generic message**

Run: `grep -rn 'falling back to direct analysis' crates/neomind-agent/`
Expected: any matches in test files should be updated to the new message format (e.g., `assert!(msg.contains("LLM tool-calling failed"))`).

- [ ] **Step 5.9: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor/mod.rs \
        crates/neomind-agent/src/ai_agent/executor/tool_loop.rs \
        crates/neomind-agent/src/ai_agent/executor/tool_result.rs
git commit -m "feat(agent): surface real LLM error from tool loop

Tool loop previously swallowed LlmError into a generic fallback string,
causing execute_with_tools to emit 'LLM tool-calling failed — falling
back to direct analysis' regardless of the real cause (403, timeout,
network, etc.). Now captures the structured LlmError in ToolLoopOutput
and uses it to construct a meaningful error message.

Also adds permanent=true/false classification field to the tool loop
warn log for observability.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 6: Add Integration Test for LLM Failure Path

**Files:**
- Create: `crates/neomind-agent/tests/llm_error_surfacing.rs`

This is a behavior-level test that verifies an LLM failure propagates as `Err` rather than silently degrading.

- [ ] **Step 6.1: Check existing test scaffolding for mock LLM runtime**

Run: `grep -rn 'MockLlm\|MockRuntime\|impl LlmRuntime' crates/neomind-agent/tests/ crates/neomind-agent/src/`
Expected: find any existing mock pattern, or confirm we need to create one.

**Note**: `LlmError` does NOT derive `Clone` (the `Io(#[from] std::io::Error)` and `Serialization(#[from] serde_json::Error)` variants wrap non-Clone types). Any mock that tries to clone an `LlmError` will not compile. The classification tests below avoid this by constructing the error in-place per assertion.

- [ ] **Step 6.2: Write the classification tests**

Create `crates/neomind-agent/tests/llm_error_surfacing.rs`:

```rust
//! Integration tests for agent LLM error surfacing.
//!
//! Verifies that LLM failures propagate as Failed execution records
//! instead of silently degrading to rule-based analysis.
//!
//! Note: These tests cover LlmError::is_permanent() classification.
//! End-to-end agent-execution verification is done via the manual
//! verification step (Task 8) because it requires a full AgentExecutor
//! bootstrap with mock stores and a mock LlmRuntime.

use neomind_core::llm::backend::LlmError;

#[test]
fn permanent_llm_error_classified_correctly() {
    // 4xx (except 429) are permanent — require user action.
    let quota_err = LlmError::Api {
        status: 403,
        body: "{\"error\":{\"message\":\"free tier exhausted\",\"type\":\"AllocationQuota\"}}".into(),
    };
    assert!(quota_err.is_permanent(), "403 should be permanent");

    assert!(LlmError::Api { status: 400, body: "".into() }.is_permanent());
    assert!(LlmError::Api { status: 401, body: "".into() }.is_permanent());
    assert!(LlmError::Api { status: 404, body: "".into() }.is_permanent());
}

#[test]
fn transient_llm_error_classified_correctly() {
    // 5xx, 429, and non-HTTP variants are transient — may succeed on retry.
    let timeout_err = LlmError::Timeout(60);
    assert!(!timeout_err.is_permanent(), "timeout should be transient");

    assert!(!LlmError::Api { status: 429, body: "rate limited".into() }.is_permanent());
    assert!(!LlmError::Api { status: 500, body: "".into() }.is_permanent());
    assert!(!LlmError::Api { status: 503, body: "".into() }.is_permanent());
    assert!(!LlmError::Network("connection refused".into()).is_permanent());
}

#[test]
fn api_variant_display_format() {
    // Display output must match the prior Generation(format!(...)) format
    // so log/error consumers see no difference.
    let e = LlmError::Api { status: 403, body: "quota exhausted".into() };
    let s = format!("{}", e);
    assert!(s.contains("403"), "Display should include status: got {}", s);
    assert!(s.contains("quota exhausted"), "Display should include body: got {}", s);
}
```

- [ ] **Step 6.3: Run the tests to verify they pass**

Run: `cargo test -p neomind-agent --test llm_error_surfacing`
Expected: PASS — 3 tests pass. (Requires Task 1 to be complete.)

- [ ] **Step 6.4: Attempt to extend with full executor integration test (optional)**

If the codebase has an executor bootstrap pattern in existing tests (check `crates/neomind-agent/tests/` for an `AgentExecutor::new(...)` pattern with in-memory stores), write a `llm_failure_propagates_as_failed_execution` test. A mock `LlmRuntime` for this must:
- Be `#[async_trait::async_trait]`
- Implement all 5 required methods: `backend_id`, `model_name`, `generate`, `generate_stream`, `max_context_length`
- NOT try to clone `LlmError` (store `status: u16` + `body: String` and construct the error fresh per `generate()` call)

If no bootstrap pattern exists, skip this step and rely on the manual verification (Task 8) — the classification tests + Display format test provide sufficient unit coverage.

- [ ] **Step 6.5: Commit**

```bash
git add crates/neomind-agent/tests/llm_error_surfacing.rs
git commit -m "test(agent): add LlmError classification integration tests

Verifies is_permanent() classification for HTTP status codes (4xx vs
5xx vs 429) and variant-based classification (BackendUnavailable,
ModelNotFound, Timeout, Network). Confirms Display format matches the
prior Generation(format!(...)) output for backward compatibility.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 7: Update Documentation

**Files:**
- Modify: `docs/guides/en/03-agent.md` (and `docs/guides/zh/03-agent.md` if Chinese version exists)

- [ ] **Step 7.1: Locate the agent execution section**

Run: `grep -n 'execution.*status\|Failed\|Completed\|LLM' docs/guides/en/03-agent.md | head -20`
Expected: find a section discussing execution status or LLM backends.

- [ ] **Step 7.2: Add documentation note**

Insert a short note (2-3 paragraphs) in the relevant section:

```markdown
### LLM Failure Handling

When an agent's LLM call fails (e.g., quota exhausted, API key invalid,
model not found, network timeout), the execution record is marked as
**Failed** with the real error message. Common failure scenarios:

- **HTTP 401/403**: API key invalid or quota exhausted — fix the API key
  or top up quota in the LLM backend settings.
- **HTTP 404**: Model name wrong or model not pulled — check the model
  name in the agent's LLM backend.
- **HTTP 429**: Rate limited — wait and retry; consider reducing
  schedule frequency.
- **HTTP 5xx**: Provider-side error — retry later.
- **Timeout**: LLM took longer than the configured timeout — consider
  using a faster model or simplifying the prompt.

Previously, LLM failures silently fell back to rule-based analysis,
producing misleading "No actions required - conditions not met" output.
As of this update, LLM failures are explicit so you can diagnose and
fix the underlying issue.
```

- [ ] **Step 7.3: Add Chinese translation if zh version exists**

Repeat Step 7.2 in `docs/guides/zh/03-agent.md` at the same section.

- [ ] **Step 7.4: Commit**

```bash
git add docs/guides/en/03-agent.md docs/guides/zh/03-agent.md
git commit -m "docs(agent): document LLM failure handling behavior

Explains that LLM failures now surface as Failed execution records
with real error messages, replacing the previous silent-fallback
behavior that produced misleading 'No actions required' output.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Task 8: Manual Verification (End-to-End)

This is the final acceptance test — verify the user's original bug is fixed.

- [ ] **Step 8.1: Build the server**

Run: `cargo build -p neomind-cli --release`
Expected: clean build.

- [ ] **Step 8.2: Start the server with an agent that uses GLM backend**

Run: `./target/release/neomind serve`
(Use an agent configured with a GLM backend whose quota is exhausted, or temporarily set an invalid API key.)

- [ ] **Step 8.3: Trigger agent execution**

Either wait for the scheduled trigger or use the UI's "Run now" button.

- [ ] **Step 8.4: Verify the execution record shows Failed with real error**

Open the agent execution history in the web UI. The latest execution should:
- Show **Failed** status (red badge) — not green "Completed"
- Show error message in detail dialog containing "API error 403" or "quota exhausted"
- NOT show the misleading "Analyzing N data points... No actions required" template

- [ ] **Step 8.5: Verify server logs show classification**

In the `neomind serve` output, grep for `permanent=`:
```bash
./target/release/neomind serve 2>&1 | grep "permanent="
```
Expected: log line like `WARN ... permanent=true ... "LLM analysis failed"` or `permanent=false` for transient errors.

- [ ] **Step 8.6: Test rule-only agent (no LLM backend)**

Configure an agent with no `llm_backend_id` set. Trigger execution.
Expected: agent runs normally using rule-based analysis, no error, no degradation flag (behavior unchanged from before).

- [ ] **Step 8.7: Test agent with deleted backend**

Configure an agent with `llm_backend_id = "some-id"`. Delete that backend. Trigger execution.
Expected: execution Failed with validation error `"Agent 'X' 配置了 LLM 后端但运行时不可用（可能已被删除或 API key 未配置）"`.

---

## Build Sequence Summary

| Task | Files | LoC | Behavior change? |
|---|---|---|---|
| 1: `LlmError::Api` + `is_permanent()` | backend.rs | +50 (with tests) | No (additive) |
| 2: OpenAI backend migration | openai.rs | ±25 (5 sites) | No (Display unchanged) |
| 3: Ollama backend migration | ollama.rs | ±15 | No (Display unchanged) |
| 4: Analyzer remove fallbacks | analyzer.rs | ±80 (rewrite branch) | **YES** — core change |
| 5: Tool loop error capture | mod.rs, tool_loop.rs, tool_result.rs | ±60 | **YES** — surfaces real LLM error |
| 6: Integration tests | tests/llm_error_surfacing.rs | +60 | No (tests) |
| 7: Documentation | 03-agent.md | +30 | No (docs) |
| 8: Manual verification | — | — | Verification |

**Total**: ~320 LoC across 8 files.

Each task produces a self-contained commit. Tasks 1-3 ship as no-behavior-change refactors (zero risk). Tasks 4-5 are the core behavior changes (analyzer fallbacks + tool-loop error capture). Tasks 6-7 are polish. Task 8 is acceptance.

---

## Risks and Mitigations

- **Risk**: Existing tests assert specific error message strings.
  **Mitigation**: `LlmError::Api`'s Display impl is `"API error {status}: {body}"` — same format as the prior `format!("API error {}: {}", status, body)`. Grep `crates/` for `"API error"` and verify tests still match.

- **Risk**: Some agent currently relies on silent fallback (treating "No actions required" as normal output).
  **Mitigation**: The fallback output was always semantically wrong (it lied about analysis). Behavior change is intentional. Users will see real errors and can fix them.

- **Risk**: Rule-only agents break if they accidentally have `llm_backend_id` set (e.g., via UI default).
  **Mitigation**: The check is `agent.llm_backend_id.is_some()`. UI should not set this field for rule-only agents. If issue arises in production, add UI guard in a follow-up.

- **Risk**: `analyze_with_llm` signature unchanged means classification happens only at inner call sites, not boundary.
  **Mitigation**: This is by design (Plan-Specific Refinements section). Boundary doesn't need classification because all errors fail uniformly. Logs at inner sites provide observability.

- **Risk**: Task 5 touches 3 files (mod.rs, tool_loop.rs, tool_result.rs) and adds a struct field — more invasive than originally planned.
  **Mitigation**: The change is additive (`Option<LlmError>` field defaults to `None`). The `tool_result.rs` change is a single-line destructure update. Without this task, tool-mode LLM failures would surface as a generic "LLM tool-calling failed" message — defeating the design's goal of exposing the real error cause. The task includes explicit grep step (5.8) to catch any test asserting the old generic message.
