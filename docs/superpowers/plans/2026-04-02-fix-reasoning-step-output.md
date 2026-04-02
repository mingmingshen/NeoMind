# Fix ReasoningStep.output Duplication Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the bug where all reasoning steps share the same `output` (the entire situation_analysis) instead of step-specific results, by adding a dedicated `result` field to the LLM response schema and updating the prompt to request per-step results.

**Architecture:** Add a `result` field to `ReasoningFromLlm` struct and update the JSON prompt schema to include `"result"` in each reasoning step. Then update all 6 code paths in `analyze_with_llm` that map `ReasoningFromLlm → ReasoningStep` to use `step.result` instead of `situation_analysis.clone()`. For backward compatibility, fall back to `step.description` when `result` is absent.

**Tech Stack:** Rust, serde JSON deserialization, LLM prompt engineering

---

## File Structure

| File | Change | Purpose |
|------|--------|---------|
| `crates/neomind-agent/src/ai_agent/executor.rs:5794-5802` | Modify `ReasoningFromLlm` | Add `result: Option<String>` field |
| `crates/neomind-agent/src/ai_agent/executor.rs:5577-5599` | Modify system prompt templates | Add `"result"` to JSON schema example |
| `crates/neomind-agent/src/ai_agent/executor.rs:5835-5842` | Modify mapping (path 1: main) | Use `step.result` instead of `situation_analysis.clone()` |
| `crates/neomind-agent/src/ai_agent/executor.rs:5920-5930` | Modify mapping (path 2: extracted JSON) | Same fix |
| `crates/neomind-agent/src/ai_agent/executor.rs:5982-5992` | Modify mapping (path 3: truncated JSON) | Same fix |
| `crates/neomind-agent/src/ai_agent/executor.rs:6043-6066` | Modify lenient parsing (path 4) | Extract `result` field from raw JSON |
| `crates/neomind-agent/src/ai_agent/executor.rs:6113-6119` | Modify empty-steps fallback (path 5) | Use description as output |
| `crates/neomind-agent/src/ai_agent/executor.rs:6168-6178` | Modify raw text fallback (path 6) | Already correct (uses raw text) |

---

### Task 1: Add `result` field to `ReasoningFromLlm`

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs:5794-5802`

- [ ] **Step 1: Add `result` field to the struct**

Change the struct at line 5794 from:

```rust
#[derive(serde::Deserialize)]
struct ReasoningFromLlm {
    #[serde(alias = "step_number", default)]
    step: serde_json::Value,
    #[serde(alias = "output", default)]
    description: Option<String>,
    #[serde(default)]
    confidence: f32,
}
```

To:

```rust
#[derive(serde::Deserialize)]
struct ReasoningFromLlm {
    #[serde(alias = "step_number", default)]
    step: serde_json::Value,
    #[serde(alias = "output", default)]
    description: Option<String>,
    /// Step-specific result/finding (distinct from the overall situation_analysis)
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    confidence: f32,
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p neomind-agent 2>&1 | head -30`
Expected: May show warnings about unused field, but no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "feat(agent): add result field to ReasoningFromLlm for step-specific output"
```

---

### Task 2: Update system prompt templates to include `result` in JSON schema

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs:5577-5599`

- [ ] **Step 1: Update all 4 prompt template variants**

There are 4 prompt templates (Chinese image, English image, Chinese non-image, English non-image). Each has a `reasoning_steps` example like:

```
"reasoning_steps": [{"step": 1, "description": "...", "confidence": 0.9}]
```

Change each to include `result`:

**Chinese image template (line ~5580):**
```
"reasoning_steps": [{{\"step\": 1, \"description\": \"分析步骤\", \"result\": \"该步骤的具体发现\", \"confidence\": 0.9}}]
```

**English image template (line ~5585):**
```
"reasoning_steps": [{{\"step\": 1, \"description\": \"Analysis step\", \"result\": \"Specific finding from this step\", \"confidence\": 0.9}}]
```

**Chinese non-image template (line ~5591):**
```
"reasoning_steps": [{{\"step\": 1, \"description\": \"步骤\", \"result\": \"该步骤的具体发现\", \"confidence\": 0.9}}]
```

**English non-image template (line ~5596):**
```
"reasoning_steps": [{{\"step\": 1, \"description\": \"Step\", \"result\": \"Specific finding from this step\", \"confidence\": 0.9}}]
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p neomind-agent 2>&1 | head -30`
Expected: Clean compilation.

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "feat(agent): update LLM prompt schema to request per-step result field"
```

---

### Task 3: Fix main parsing path (path 1) to use `step.result`

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs:5835-5842`

- [ ] **Step 1: Update the mapping at line 5840**

Change:
```rust
.map(|(_i, step)| neomind_storage::ReasoningStep {
    step_number: extract_step_number(&step.step, (_i + 1) as u32),
    description: step.description.unwrap_or_default(),
    step_type: "llm_analysis".to_string(),
    input: Some(text_data_summary.join("\n")),
    output: situation_analysis.clone(),
    confidence: step.confidence,
})
```

To:
```rust
.map(|(_i, step)| neomind_storage::ReasoningStep {
    step_number: extract_step_number(&step.step, (_i + 1) as u32),
    description: step.description.unwrap_or_default(),
    step_type: "llm_analysis".to_string(),
    input: Some(text_data_summary.join("\n")),
    output: step.result
        .or(step.description.clone())
        .unwrap_or_default(),
    confidence: step.confidence,
})
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p neomind-agent 2>&1 | head -30`
Expected: Clean compilation.

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "fix(agent): use step-specific result in main LLM parsing path"
```

---

### Task 4: Fix extracted JSON fallback path (path 2)

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs:5920-5930`

- [ ] **Step 1: Update the mapping at line 5928**

Change `output: situation_analysis.clone(),` to:
```rust
output: step.result
    .or(step.description.clone())
    .unwrap_or_default(),
```

Full context - the block should become:
```rust
.map(|(_i, step)| neomind_storage::ReasoningStep {
    step_number: extract_step_number(
        &step.step,
        (_i + 1) as u32,
    ),
    description: step.description.unwrap_or_default(),
    step_type: "llm_analysis".to_string(),
    input: Some(text_data_summary.join("\n")),
    output: step.result
        .or(step.description.clone())
        .unwrap_or_default(),
    confidence: step.confidence,
})
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p neomind-agent 2>&1 | head -30`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "fix(agent): use step-specific result in extracted JSON fallback"
```

---

### Task 5: Fix truncated JSON recovery path (path 3)

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs:5982-5992`

- [ ] **Step 1: Update the mapping at line 5990**

Change `output: situation_analysis.clone(),` to:
```rust
output: step.result
    .or(step.description.clone())
    .unwrap_or_default(),
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p neomind-agent 2>&1 | head -30`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "fix(agent): use step-specific result in truncated JSON recovery path"
```

---

### Task 6: Fix lenient JSON parsing path (path 4)

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs:6043-6066`

- [ ] **Step 1: Extract `result` field from raw JSON and use it**

The lenient path iterates over JSON values directly. Update the block starting at line 6043:

Change:
```rust
for (i, item) in arr.iter().enumerate() {
    let step_num = (i + 1) as u32;
    let description: String = item
        .get("description")
        .and_then(|v| v.as_str())
        .or_else(|| item.get("output").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();
    if description.is_empty() {
        continue;
    }
    let confidence = item
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.8)
        as f32;
    reasoning_steps.push(neomind_storage::ReasoningStep {
        step_number: step_num,
        description,
        step_type: "llm_analysis".to_string(),
        input: Some(text_data_summary.join("\n")),
        output: situation_analysis.clone(),
        confidence,
    });
}
```

To:
```rust
for (i, item) in arr.iter().enumerate() {
    let step_num = (i + 1) as u32;
    let description: String = item
        .get("description")
        .and_then(|v| v.as_str())
        .or_else(|| item.get("output").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();
    if description.is_empty() {
        continue;
    }
    let step_result = item
        .get("result")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| Some(description.clone()))
        .unwrap_or_default();
    let confidence = item
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.8)
        as f32;
    reasoning_steps.push(neomind_storage::ReasoningStep {
        step_number: step_num,
        description,
        step_type: "llm_analysis".to_string(),
        input: Some(text_data_summary.join("\n")),
        output: step_result,
        confidence,
    });
}
```

Note: The fallback chain is `result → description → empty`, consistent with Tasks 3-5.

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p neomind-agent 2>&1 | head -30`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "fix(agent): use step-specific result in lenient JSON parsing fallback"
```

---

### Task 7: Fix empty-steps fallback path (path 5)

**Files:**
- Modify: `crates/neomind-agent/src/ai_agent/executor.rs:6113-6119`

- [ ] **Step 1: Update the single-step fallback**

Change:
```rust
vec![neomind_storage::ReasoningStep {
    step_number: 1,
    description: "LLM analysis completed".to_string(),
    step_type: "llm_analysis".to_string(),
    input: Some(format!("{} data sources", data.len())),
    output: situation_analysis.clone(),
    confidence: 0.7,
}]
```

To:
```rust
vec![neomind_storage::ReasoningStep {
    step_number: 1,
    description: "LLM analysis completed".to_string(),
    step_type: "llm_analysis".to_string(),
    input: Some(format!("{} data sources", data.len())),
    output: situation_analysis.chars().take(200).collect::<String>(),
    confidence: 0.7,
}]
```

Note: This fallback fires when `reasoning_steps` was empty. Using a truncated situation_analysis as output is reasonable here since there's only one step. But we truncate to avoid duplication in case this step gets merged with others.

- [ ] **Step 2: Verify compilation**

Run: `cargo check -p neomind-agent 2>&1 | head -30`

- [ ] **Step 3: Commit**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "fix(agent): truncate output in empty-steps fallback to avoid duplication"
```

---

### Task 8: Verify raw text fallback (path 6) — no change needed

**Files:**
- Inspect: `crates/neomind-agent/src/ai_agent/executor.rs:6168-6178`

- [ ] **Step 1: Verify the raw text path is already correct**

This path creates a single `ReasoningStep` with `output: situation_analysis.clone()`. Since this path only fires when all JSON parsing fails and produces a single step (not multiple), using the full text as output is correct behavior — there's no duplication across steps.

No code change needed. Verify the block at line 6168:
```rust
let reasoning_steps = vec![neomind_storage::ReasoningStep {
    step_number: 1,
    description: if situation_analysis.chars().count() > 200 {
        situation_analysis.chars().take(200).collect::<String>() + "..."
    } else {
        situation_analysis.clone()
    },
    step_type: "llm_analysis".to_string(),
    input: Some(format!("{} data sources", data.len())),
    output: situation_analysis.clone(),
    confidence: 0.7,
}];
```

This is acceptable: single-step, raw text fallback. No duplication issue.

---

### Task 9: Full compilation and test

**Files:**
- All modified files

- [ ] **Step 1: Run full cargo check**

Run: `cargo check 2>&1 | tail -20`
Expected: Clean compilation, no errors.

- [ ] **Step 2: Run cargo clippy**

Run: `cargo clippy -p neomind-agent 2>&1 | tail -20`
Expected: No new warnings related to changes.

- [ ] **Step 3: Run cargo test**

Run: `cargo test -p neomind-agent 2>&1 | tail -30`
Expected: All tests pass.

- [ ] **Step 4: Run cargo fmt**

Run: `cargo fmt`
Expected: No formatting changes needed (or auto-fixed).

- [ ] **Step 5: Commit (if any formatting changes)**

```bash
git add crates/neomind-agent/src/ai_agent/executor.rs
git commit -m "style(agent): apply cargo fmt"
```

---

## Testing Notes

**Why no unit tests are proposed:** `ReasoningFromLlm` and `LlmResponse` are private structs defined inside the `analyze_with_llm` function body. They cannot be directly unit-tested from an external test module. The serde backward-compatibility of `Option<String>` with `#[serde(default)]` is well-established and low-risk.

**Practical verification:** After implementation, run an agent with an LLM backend and inspect the reasoning steps in the UI to confirm each step has a unique, step-specific `output` instead of the shared `situation_analysis`.

**Future improvement:** Consider extracting `ReasoningFromLlm` to module scope for unit testability.

## Future Considerations

**Output truncation:** The cleanup pass at `create_conversation_turn` (line ~7526-7533) truncates `step.description` but not `step.output`. After this fix, `output` will contain LLM-generated per-step results which could be arbitrarily long. Consider adding `step.output = clean_and_truncate_text(&step.output, 200)` to the same cleanup pass as a follow-up.
