# Agent Planning & Parallel Execution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a hybrid keyword/LLM planning stage to the agent pipeline that deliberately constructs multi-tool-call plans, improving latency for multi-device queries and giving users visibility into what the agent will do.

**Architecture:** The planner runs as a pre-processing step before the streaming loop. KeywordPlanner uses rule-based mapping from `IntentCategory`; LLMPlanner uses one lightweight LLM call for complex tasks. The planner feeds into the existing `join_all` parallel execution in `streaming.rs`. New `AgentEvent` variants are additive.

**Tech Stack:** Rust (tokio, serde, async-trait), TypeScript (React, Zustand), WebSocket events

**Spec:** `docs/superpowers/specs/2026-04-05-agent-planning-parallel-design.md`

---

## File Map

### New Files
| File | Responsibility |
|---|---|
| `crates/neomind-agent/src/agent/planner/mod.rs` | Planner trait, module re-exports |
| `crates/neomind-agent/src/agent/planner/types.rs` | `ExecutionPlan`, `PlanStep`, `PlanningMode`, `StepId` |
| `crates/neomind-agent/src/agent/planner/keyword.rs` | Keyword-based fast planner using `IntentCategory` |
| `crates/neomind-agent/src/agent/planner/llm_planner.rs` | LLM-based deep planner with structured output |
| `web/src/components/chat/ExecutionPlanPanel.tsx` | Plan visualization component |

### Modified Files
| File | Change |
|---|---|
| `crates/neomind-agent/src/agent/types.rs` | Add `PlanningConfig` to `AgentConfig`, add new `AgentEvent` variants |
| `crates/neomind-agent/src/agent/mod.rs` | Add `pub mod planner` and re-export types |
| `crates/neomind-agent/src/agent/staged.rs` | Integrate planner between intent classification and tool filtering |
| `crates/neomind-agent/src/agent/streaming.rs` | Accept optional `ExecutionPlan`, emit plan events |
| `web/src/types/index.ts` | Add new `ServerMessage` variants for plan events |
| `web/src/components/chat/ChatContainer.tsx` | Handle new plan events in reducer and WebSocket handler |
| `web/src/components/chat/MergedMessageList.tsx` | Render `ExecutionPlanPanel` for streaming messages |

---

## Task 1: Planner Types

**Files:**
- Create: `crates/neomind-agent/src/agent/planner/mod.rs`
- Create: `crates/neomind-agent/src/agent/planner/types.rs`
- Modify: `crates/neomind-agent/src/agent/types.rs` (add `PlanningConfig`)
- Modify: `crates/neomind-agent/src/agent/mod.rs` (add module + re-exports)

- [ ] **Step 1: Create `planner/types.rs` with core types**

```rust
// crates/neomind-agent/src/agent/planner/types.rs
//! Planner types for execution plan generation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Step identifier — index into the plan's step vector.
pub type StepId = usize;

/// How the plan was generated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanningMode {
    /// Rule-based mapping from IntentCategory (fast, zero LLM cost)
    Keyword,
    /// LLM-generated plan for complex multi-step tasks
    LLM,
}

/// A single step in an execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Unique step identifier (index in the plan).
    pub id: StepId,
    /// Tool name: "device", "agent", "agent_history", "rule", "alert", "extension"
    pub tool_name: String,
    /// Action within the tool: "list", "get", "query", "control", etc.
    pub action: String,
    /// Parameters for the tool call.
    pub params: Value,
    /// Steps that must complete before this one. Empty = parallelizable.
    #[serde(default)]
    pub depends_on: Vec<StepId>,
    /// Human-readable description for frontend display.
    pub description: String,
}

impl PlanStep {
    /// Whether this step is safe to run in parallel with others.
    /// Destructive operations (control, delete, create, update) are never parallel.
    pub fn is_safe_parallel(&self) -> bool {
        matches!(
            (self.tool_name.as_str(), self.action.as_str()),
            ("device", "list" | "get" | "query")
                | ("rule", "list" | "get")
                | ("agent", "list" | "get")
                | ("agent_history", "executions" | "conversation")
                | ("alert", "list")
                | ("extension", "list" | "get" | "status")
        )
    }
}

/// Execution plan produced by the planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Steps in the plan, ordered by intended execution sequence.
    pub steps: Vec<PlanStep>,
    /// How the plan was generated.
    pub mode: PlanningMode,
}

impl ExecutionPlan {
    /// Create an empty plan (signals "skip planning").
    pub fn empty(mode: PlanningMode) -> Self {
        Self {
            steps: Vec::new(),
            mode,
        }
    }

    /// Whether this plan has any steps.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Group steps into parallel batches based on dependencies and safety.
    /// Returns vec of batches, where each batch contains step IDs that can run in parallel.
    pub fn parallel_batches(&self) -> Vec<Vec<StepId>> {
        if self.steps.is_empty() {
            return Vec::new();
        }

        let mut completed: std::collections::HashSet<StepId> = std::collections::HashSet::new();
        let mut batches = Vec::new();

        while completed.len() < self.steps.len() {
            let mut batch = Vec::new();
            for step in &self.steps {
                if completed.contains(&step.id) {
                    continue;
                }
                // Check all dependencies are satisfied
                let deps_met = step.depends_on.iter().all(|dep| completed.contains(dep));
                if deps_met && step.is_safe_parallel() {
                    batch.push(step.id);
                } else if deps_met && !step.is_safe_parallel() {
                    // Destructive steps get their own batch (single item)
                    if batch.is_empty() {
                        batch.push(step.id);
                    }
                    // If batch already has safe steps, defer this one
                }
            }

            if batch.is_empty() {
                // Force-progress: pick the first unresolved step
                for step in &self.steps {
                    if !completed.contains(&step.id) {
                        batch.push(step.id);
                        break;
                    }
                }
            }

            for id in &batch {
                completed.insert(*id);
            }
            batches.push(batch);
        }

        batches
    }
}

/// Configuration for the planning system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningConfig {
    /// Enable planning stage (default: true). When false, uses existing pipeline.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Confidence threshold above which KeywordPlanner is used (default: 0.8).
    #[serde(default = "default_keyword_threshold")]
    pub keyword_threshold: f32,
    /// Maximum entities before falling back to LLM planner (default: 3).
    #[serde(default = "default_max_entities")]
    pub max_entities_for_keyword: usize,
    /// Timeout for LLM planner call in seconds (default: 2).
    #[serde(default = "default_llm_timeout_secs")]
    pub llm_timeout_secs: u64,
}

fn default_enabled() -> bool { true }
fn default_keyword_threshold() -> f32 { 0.8 }
fn default_max_entities() -> usize { 3 }
fn default_llm_timeout_secs() -> u64 { 2 }

impl Default for PlanningConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            keyword_threshold: default_keyword_threshold(),
            max_entities_for_keyword: default_max_entities(),
            llm_timeout_secs: default_llm_timeout_secs(),
        }
    }
}
```

- [ ] **Step 2: Run tests to verify types compile**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo check -p neomind-agent 2>&1 | head -20`
Expected: Compile errors — module not yet registered (that's OK, types file is standalone for now)

- [ ] **Step 3: Create `planner/mod.rs` with trait and re-exports**

```rust
// crates/neomind-agent/src/agent/planner/mod.rs
//! Planner module for generating execution plans.
//!
//! Two planners:
//! - `KeywordPlanner` — fast, rule-based, zero LLM cost
//! - `LLMPlanner` — deep, LLM-generated, for complex tasks

pub mod keyword;
pub mod llm_planner;
pub mod types;

use async_trait::async_trait;
pub use types::{ExecutionPlan, PlanningConfig, PlanningMode, PlanStep, StepId};

use crate::agent::context_selector::ContextBundle;
use crate::agent::staged::IntentResult;

/// Planner trait — produce an execution plan from intent + context.
#[async_trait]
pub trait Planner: Send + Sync {
    /// Generate an execution plan.
    /// Returns `None` if planning should be skipped (e.g., general chat).
    async fn plan(
        &self,
        intent: &IntentResult,
        context: &ContextBundle,
        user_message: &str,
    ) -> Option<ExecutionPlan>;
}
```

- [ ] **Step 4: Add `planning` field to `AgentConfig` in `types.rs`**

In `crates/neomind-agent/src/agent/types.rs`, at the end of the `AgentConfig` struct (around line 233, before the closing `}`), add:

```rust
    /// Planning configuration
    #[serde(default)]
    pub planning: crate::agent::planner::types::PlanningConfig,
```

Update `Default` impl for `AgentConfig` (around line 274) to include:

```rust
            planning: crate::agent::planner::types::PlanningConfig::default(),
```

- [ ] **Step 5: Add `pub mod planner` to `agent/mod.rs`**

In `crates/neomind-agent/src/agent/mod.rs`, after line 33 (`pub mod types;`), add:

```rust
pub mod planner;
```

- [ ] **Step 6: Run cargo check**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo check -p neomind-agent 2>&1 | tail -5`
Expected: May have warnings about unused `planner` module, but no errors

- [ ] **Step 7: Commit**

```bash
cd "/Users/shenmingming/CamThink Project/NeoMind"
git add crates/neomind-agent/src/agent/planner/
git add crates/neomind-agent/src/agent/types.rs
git add crates/neomind-agent/src/agent/mod.rs
git commit -m "feat(agent): add planner types and PlanningConfig"
```

---

## Task 2: KeywordPlanner

**Files:**
- Create: `crates/neomind-agent/src/agent/planner/keyword.rs`

- [ ] **Step 1: Write keyword planner tests in the same file**

Add at the bottom of `keyword.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::staged::IntentCategory;

    #[test]
    fn test_keyword_planner_single_device_query() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["设备".to_string()],
        };
        let ctx = ContextBundle::default();
        let plan = planner.plan_sync(&intent, &ctx, "查看客厅温度传感器的数据");

        assert!(plan.is_some());
        let plan = plan.unwrap();
        assert!(plan.steps.len() >= 1);
        assert_eq!(plan.mode, PlanningMode::Keyword);
    }

    #[test]
    fn test_keyword_planner_general_skips() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::General,
            confidence: 0.5,
            keywords: vec![],
        };
        let ctx = ContextBundle::default();
        let plan = planner.plan_sync(&intent, &ctx, "你好");

        assert!(plan.is_none());
    }

    #[test]
    fn test_keyword_planner_help_skips() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Help,
            confidence: 0.9,
            keywords: vec!["帮助".to_string()],
        };
        let ctx = ContextBundle::default();
        let plan = planner.plan_sync(&intent, &ctx, "怎么用");

        assert!(plan.is_none());
    }

    #[test]
    fn test_parallel_batches_independent_steps() {
        let plan = ExecutionPlan {
            steps: vec![
                PlanStep {
                    id: 0,
                    tool_name: "device".into(),
                    action: "query".into(),
                    params: serde_json::json!({"device_id": "temp_1"}),
                    depends_on: vec![],
                    description: "Query temp_1".into(),
                },
                PlanStep {
                    id: 1,
                    tool_name: "device".into(),
                    action: "query".into(),
                    params: serde_json::json!({"device_id": "temp_2"}),
                    depends_on: vec![],
                    description: "Query temp_2".into(),
                },
            ],
            mode: PlanningMode::Keyword,
        };

        let batches = plan.parallel_batches();
        assert_eq!(batches.len(), 1); // Both in one parallel batch
        assert_eq!(batches[0].len(), 2);
    }

    #[test]
    fn test_parallel_batches_serial_for_control() {
        let plan = ExecutionPlan {
            steps: vec![
                PlanStep {
                    id: 0,
                    tool_name: "device".into(),
                    action: "query".into(),
                    params: serde_json::json!({"device_id": "temp_1"}),
                    depends_on: vec![],
                    description: "Query temp_1".into(),
                },
                PlanStep {
                    id: 1,
                    tool_name: "device".into(),
                    action: "control".into(),
                    params: serde_json::json!({"device_id": "light_1", "command": "on"}),
                    depends_on: vec![],
                    description: "Turn on light_1".into(),
                },
            ],
            mode: PlanningMode::Keyword,
        };

        let batches = plan.parallel_batches();
        // query is safe, control is destructive → separate batches
        assert_eq!(batches.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo test -p neomind-agent keyword 2>&1 | tail -10`
Expected: Compile errors — `KeywordPlanner` not defined yet

- [ ] **Step 3: Implement KeywordPlanner**

```rust
// crates/neomind-agent/src/agent/planner/keyword.rs
//! Keyword-based fast planner.
//!
//! Maps IntentCategory to execution templates using rule-based logic.
//! Zero LLM cost, sub-millisecond execution.

use async_trait::async_trait;
use serde_json::json;

use super::types::{ExecutionPlan, PlanningMode, PlanStep};
use super::Planner;

use crate::agent::context_selector::ContextBundle;
use crate::agent::staged::{IntentCategory, IntentResult};

/// Keyword-based planner using rule mapping.
pub struct KeywordPlanner {
    /// Categories that skip planning entirely (just chat, no tools needed)
    skip_categories: Vec<IntentCategory>,
}

impl KeywordPlanner {
    pub fn new() -> Self {
        Self {
            skip_categories: vec![
                IntentCategory::General,
                IntentCategory::Help,
            ],
        }
    }

    /// Synchronous planning (no async needed for keyword matching).
    pub fn plan_sync(
        &self,
        intent: &IntentResult,
        _context: &ContextBundle,
        user_message: &str,
    ) -> Option<ExecutionPlan> {
        // Skip planning for chat/help categories
        if self.skip_categories.contains(&intent.category) {
            return None;
        }

        let steps = match intent.category {
            IntentCategory::Device => self.plan_device_steps(user_message),
            IntentCategory::Rule => self.plan_rule_steps(user_message),
            IntentCategory::Data => self.plan_data_steps(user_message),
            IntentCategory::Alert => self.plan_alert_steps(user_message),
            IntentCategory::System => self.plan_system_steps(user_message),
            IntentCategory::Workflow => return None, // Defer to LLM planner
            _ => return None,
        };

        if steps.is_empty() {
            return None;
        }

        Some(ExecutionPlan {
            steps,
            mode: PlanningMode::Keyword,
        })
    }

    fn plan_device_steps(&self, message: &str) -> Vec<PlanStep> {
        // Detect if this is a control action
        let is_control = ["控制", "打开", "关闭", "开关", "设置", "control", "turn on", "turn off", "open", "close"]
            .iter()
            .any(|kw| message.to_lowercase().contains(kw));

        if is_control {
            vec![PlanStep {
                id: 0,
                tool_name: "device".into(),
                action: "control".into(),
                params: json!({"message": message}),
                depends_on: vec![],
                description: format!("控制设备: {}", message),
            }]
        } else {
            vec![PlanStep {
                id: 0,
                tool_name: "device".into(),
                action: "query".into(),
                params: json!({"message": message}),
                depends_on: vec![],
                description: format!("查询设备: {}", message),
            }]
        }
    }

    fn plan_rule_steps(&self, message: &str) -> Vec<PlanStep> {
        vec![PlanStep {
            id: 0,
            tool_name: "rule".into(),
            action: "list".into(),
            params: json!({"message": message}),
            depends_on: vec![],
            description: "查询规则列表".into(),
        }]
    }

    fn plan_data_steps(&self, message: &str) -> Vec<PlanStep> {
        vec![PlanStep {
            id: 0,
            tool_name: "device".into(),
            action: "query".into(),
            params: json!({"message": message}),
            depends_on: vec![],
            description: format!("查询数据: {}", message),
        }]
    }

    fn plan_alert_steps(&self, _message: &str) -> Vec<PlanStep> {
        vec![PlanStep {
            id: 0,
            tool_name: "alert".into(),
            action: "list".into(),
            params: json!({}),
            depends_on: vec![],
            description: "查询告警列表".into(),
        }]
    }

    fn plan_system_steps(&self, _message: &str) -> Vec<PlanStep> {
        // System queries don't need planning — single tool call
        Vec::new()
    }
}

impl Default for KeywordPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Planner for KeywordPlanner {
    async fn plan(
        &self,
        intent: &IntentResult,
        context: &ContextBundle,
        user_message: &str,
    ) -> Option<ExecutionPlan> {
        self.plan_sync(intent, context, user_message)
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo test -p neomind-agent keyword 2>&1 | tail -20`
Expected: All keyword planner tests PASS

- [ ] **Step 5: Commit**

```bash
cd "/Users/shenmingming/CamThink Project/NeoMind"
git add crates/neomind-agent/src/agent/planner/keyword.rs
git commit -m "feat(agent): add KeywordPlanner with rule-based mapping"
```

---

## Task 3: LLMPlanner

**Files:**
- Create: `crates/neomind-agent/src/agent/planner/llm_planner.rs`

- [ ] **Step 1: Implement LLMPlanner**

```rust
// crates/neomind-agent/src/agent/planner/llm_planner.rs
//! LLM-based deep planner for complex multi-step tasks.
//!
//! Uses one lightweight LLM call to produce a structured execution plan.
//! Falls back to KeywordPlanner on failure or timeout.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::keyword::KeywordPlanner;
use super::types::{ExecutionPlan, PlanningMode, PlanStep};
use super::Planner;

use crate::agent::context_selector::ContextBundle;
use crate::agent::staged::IntentResult;
use crate::llm::LlmInterface;

/// LLM output format for plan parsing.
#[derive(Debug, Deserialize)]
struct LlmPlanOutput {
    steps: Vec<LlmPlanStep>,
}

#[derive(Debug, Deserialize)]
struct LlmPlanStep {
    tool: String,
    action: String,
    params: serde_json::Value,
    #[serde(default)]
    depends_on: Vec<super::types::StepId>,
    #[serde(default)]
    description: String,
}

/// LLM-based planner with fallback to KeywordPlanner.
pub struct LLMPlanner {
    llm: Arc<LlmInterface>,
    keyword_planner: KeywordPlanner,
    timeout: Duration,
}

impl LLMPlanner {
    pub fn new(llm: Arc<LlmInterface>, timeout_secs: u64) -> Self {
        Self {
            llm,
            keyword_planner: KeywordPlanner::new(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    fn build_planning_prompt(user_message: &str, _context: &ContextBundle) -> String {
        format!(
            r#"You are a task planner for an IoT platform. Analyze the user request and create an execution plan.

Available tools:
- device: actions=list, get, query, control
- agent: actions=list, get, create, update, control, memory
- agent_history: actions=executions, conversation
- rule: actions=list, get, delete, history
- alert: actions=list, create, acknowledge
- extension: actions=list, get, execute, status

Rules:
- Mark independent steps with empty depends_on
- Destructive actions (control, delete, create) should NOT be parallel
- Keep plans simple — prefer fewer steps

User request: {}

Respond with JSON only:
{{"steps":[{{"tool":"...","action":"...","params":{{}},"depends_on":[],"description":"..."}}]}}"#,
            user_message
        )
    }

    async fn call_llm_plan(&self, user_message: &str, context: &ContextBundle) -> Option<ExecutionPlan> {
        let prompt = Self::build_planning_prompt(user_message, context);
        let system = "You are a task planner. Output only valid JSON. No explanation.";

        let result = tokio::time::timeout(
            self.timeout,
            self.llm.chat(system, &prompt),
        ).await.ok()?;

        let response = result.ok()?;
        let cleaned = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let parsed: LlmPlanOutput = serde_json::from_str(cleaned).ok()?;

        let steps: Vec<PlanStep> = parsed
            .steps
            .into_iter()
            .enumerate()
            .map(|(i, s)| PlanStep {
                id: i,
                tool_name: s.tool,
                action: s.action,
                params: s.params,
                depends_on: s.depends_on,
                description: s.description,
            })
            .collect();

        if steps.is_empty() {
            return None;
        }

        Some(ExecutionPlan {
            steps,
            mode: PlanningMode::LLM,
        })
    }
}

#[async_trait]
impl Planner for LLMPlanner {
    async fn plan(
        &self,
        intent: &IntentResult,
        context: &ContextBundle,
        user_message: &str,
    ) -> Option<ExecutionPlan> {
        // Try LLM planning first
        if let Some(plan) = self.call_llm_plan(user_message, context).await {
            return Some(plan);
        }

        // Fallback to keyword planner
        self.keyword_planner.plan(intent, context, user_message).await
    }
}
```

- [ ] **Step 2: Run cargo check**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo check -p neomind-agent 2>&1 | tail -5`
Expected: Compiles with warnings about unused fields (OK, integration comes later)

- [ ] **Step 3: Commit**

```bash
cd "/Users/shenmingming/CamThink Project/NeoMind"
git add crates/neomind-agent/src/agent/planner/llm_planner.rs
git commit -m "feat(agent): add LLMPlanner with structured output parsing"
```

---

## Task 4: New AgentEvent Variants

**Files:**
- Modify: `crates/neomind-agent/src/agent/types.rs` (lines 9-89, the `AgentEvent` enum)

- [ ] **Step 1: Add new event variants to `AgentEvent`**

In `crates/neomind-agent/src/agent/types.rs`, after the `Plan { step, stage }` variant (around line 72) and before `Heartbeat` (line 74), add:

```rust
    /// Execution plan created (richer replacement for Plan in multi-step scenarios)
    ExecutionPlanCreated {
        /// The execution plan with all steps
        plan: crate::agent::planner::types::ExecutionPlan,
        /// Session ID
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
    },
    /// A single step in the execution plan has started
    PlanStepStarted {
        /// Step index in the plan
        step_id: crate::agent::planner::types::StepId,
        /// Human-readable step description
        description: String,
    },
    /// A single step in the execution plan has completed
    PlanStepCompleted {
        /// Step index in the plan
        step_id: crate::agent::planner::types::StepId,
        /// Whether the step succeeded
        success: bool,
        /// Brief result summary
        summary: String,
    },
```

- [ ] **Step 2: Add helper constructors**

After the existing `pub fn plan()` method (around line 172), add:

```rust
    /// Create an execution plan created event.
    pub fn execution_plan_created(plan: crate::agent::planner::types::ExecutionPlan) -> Self {
        Self::ExecutionPlanCreated {
            plan,
            session_id: None,
        }
    }

    /// Create a plan step started event.
    pub fn plan_step_started(step_id: usize, description: impl Into<String>) -> Self {
        Self::PlanStepStarted {
            step_id,
            description: description.into(),
        }
    }

    /// Create a plan step completed event.
    pub fn plan_step_completed(step_id: usize, success: bool, summary: impl Into<String>) -> Self {
        Self::PlanStepCompleted {
            step_id,
            success,
            summary: summary.into(),
        }
    }
```

- [ ] **Step 3: Run cargo check**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo check -p neomind-agent 2>&1 | tail -5`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
cd "/Users/shenmingming/CamThink Project/NeoMind"
git add crates/neomind-agent/src/agent/types.rs
git commit -m "feat(agent): add ExecutionPlanCreated, PlanStepStarted, PlanStepCompleted events"
```

---

## Task 5: Integrate Planner into Staged Pipeline

**Files:**
- Modify: `crates/neomind-agent/src/agent/staged.rs` (integrate planner after intent classification)

- [ ] **Step 1: Add planner integration to `staged.rs`**

Add at the top of `staged.rs` (after existing imports):

```rust
use super::planner::keyword::KeywordPlanner;
use super::planner::llm_planner::LLMPlanner;
use super::planner::types::PlanningConfig;
use super::planner::Planner;
```

Add a new struct that wraps the path selection logic:

```rust
/// Planning coordinator that selects between KeywordPlanner and LLMPlanner.
pub struct PlanningCoordinator {
    config: PlanningConfig,
    keyword_planner: KeywordPlanner,
    // LLM planner created lazily when needed
    llm_interface: Option<std::sync::Arc<crate::llm::LlmInterface>>,
}

impl PlanningCoordinator {
    pub fn new(config: PlanningConfig) -> Self {
        Self {
            config,
            keyword_planner: KeywordPlanner::new(),
            llm_interface: None,
        }
    }

    pub fn with_llm(mut self, llm: std::sync::Arc<crate::llm::LlmInterface>) -> Self {
        self.llm_interface = Some(llm);
        self
    }

    /// Decide which planner to use based on intent confidence and complexity.
    pub fn should_use_keyword_planner(&self, intent: &IntentResult) -> bool {
        intent.confidence >= self.config.keyword_threshold
            && intent.category != IntentCategory::Workflow
        // Workflow always gets LLM planner (multi-domain)
    }

    /// Generate an execution plan using the appropriate planner.
    pub async fn plan(
        &self,
        intent: &IntentResult,
        context: &super::context_selector::ContextBundle,
        user_message: &str,
    ) -> Option<super::planner::types::ExecutionPlan> {
        if !self.config.enabled {
            return None;
        }

        if self.should_use_keyword_planner(intent) {
            self.keyword_planner.plan(intent, context, user_message).await
        } else if let Some(llm) = &self.llm_interface {
            let llm_planner = LLMPlanner::new(llm.clone(), self.config.llm_timeout_secs);
            llm_planner.plan(intent, context, user_message).await
        } else {
            // No LLM available, try keyword planner as fallback
            self.keyword_planner.plan(intent, context, user_message).await
        }
    }
}
```

- [ ] **Step 2: Add tests for PlanningCoordinator**

```rust
#[cfg(test)]
mod planning_coordinator_tests {
    use super::*;

    #[test]
    fn test_should_use_keyword_high_confidence() {
        let coord = PlanningCoordinator::new(PlanningConfig::default());
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["设备".into()],
        };
        assert!(coord.should_use_keyword_planner(&intent));
    }

    #[test]
    fn test_should_use_llm_for_workflow() {
        let coord = PlanningCoordinator::new(PlanningConfig::default());
        let intent = IntentResult {
            category: IntentCategory::Workflow,
            confidence: 0.95,
            keywords: vec!["工作流".into()],
        };
        assert!(!coord.should_use_keyword_planner(&intent));
    }

    #[test]
    fn test_should_use_llm_for_low_confidence() {
        let coord = PlanningCoordinator::new(PlanningConfig::default());
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.5,
            keywords: vec![],
        };
        assert!(!coord.should_use_keyword_planner(&intent));
    }

    #[test]
    fn test_planning_disabled() {
        let mut config = PlanningConfig::default();
        config.enabled = false;
        let coord = PlanningCoordinator::new(config);
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["设备".into()],
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let plan = rt.block_on(coord.plan(&intent, &ContextBundle::default(), "查询设备"));
        assert!(plan.is_none());
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo test -p neomind-agent planning_coordinator 2>&1 | tail -15`
Expected: All 4 coordinator tests PASS

- [ ] **Step 4: Commit**

```bash
cd "/Users/shenmingming/CamThink Project/NeoMind"
git add crates/neomind-agent/src/agent/staged.rs
git commit -m "feat(agent): add PlanningCoordinator with path selection logic"
```

---

## Task 6: Emit Plan Events in Streaming

**Files:**
- Modify: `crates/neomind-agent/src/agent/streaming.rs`

- [ ] **Step 1: Add plan event emission helpers**

Add near the top of `streaming.rs` (after existing imports):

```rust
use super::planner::types::ExecutionPlan;
```

Add a helper function (near other helper functions in the file):

```rust
/// Emit plan events from an ExecutionPlan through the event channel.
pub fn emit_plan_events(plan: &ExecutionPlan, tx: &tokio::sync::mpsc::UnboundedSender<super::types::AgentEvent>) {
    // Emit the full plan for frontend rendering
    let _ = tx.send(super::types::AgentEvent::execution_plan_created(plan.clone()));
}
```

- [ ] **Step 2: Add `ExecutionPlan` parameter to stream processing functions**

This is a lightweight change — add an `Option<ExecutionPlan>` parameter to the main stream entry points. Find the primary public-facing stream function signature and add the parameter. The plan is only used to emit the `ExecutionPlanCreated` event at the start of processing.

In the actual integration, the caller (from `staged.rs`) will:
1. Run planner → get `Option<ExecutionPlan>`
2. If plan exists, emit `ExecutionPlanCreated` event
3. Pass plan (if any) to stream function
4. Stream function emits `PlanStepStarted`/`PlanStepCompleted` around tool calls

- [ ] **Step 3: Run cargo check**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo check -p neomind-agent 2>&1 | tail -5`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
cd "/Users/shenmingming/CamThink Project/NeoMind"
git add crates/neomind-agent/src/agent/streaming.rs
git commit -m "feat(agent): add plan event emission to streaming pipeline"
```

---

## Task 7: Frontend Types

**Files:**
- Modify: `web/src/types/index.ts` (around line 538-573, `ServerMessage` type)

- [ ] **Step 1: Add new plan event types to `ServerMessage`**

In `web/src/types/index.ts`, after the existing `Plan` variant in `ServerMessage` (around line 561), add:

```typescript
  // Execution plan created - full plan with all steps
  | { type: 'ExecutionPlanCreated'; plan: ExecutionPlan; sessionId: string }
  // A plan step has started executing
  | { type: 'PlanStepStarted'; stepId: number; description: string; sessionId: string }
  // A plan step has completed
  | { type: 'PlanStepCompleted'; stepId: number; success: boolean; summary: string; sessionId: string }
```

Also add the TypeScript types for the plan data (add before `ServerMessage`):

```typescript
/** Planning mode */
export type PlanningMode = 'keyword' | 'llm'

/** A single step in an execution plan */
export interface PlanStep {
  id: number
  tool_name: string
  action: string
  params: Record<string, unknown>
  depends_on: number[]
  description: string
}

/** An execution plan */
export interface ExecutionPlan {
  steps: PlanStep[]
  mode: PlanningMode
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind/web" && npx tsc --noEmit 2>&1 | head -10`
Expected: No type errors related to new types

- [ ] **Step 3: Commit**

```bash
cd "/Users/shenmingming/CamThink Project/NeoMind"
git add web/src/types/index.ts
git commit -m "feat(web): add ExecutionPlan types and ServerMessage variants"
```

---

## Task 8: Frontend Event Handling

**Files:**
- Modify: `web/src/components/chat/ChatContainer.tsx`

- [ ] **Step 1: Add plan state to `StreamState` and `StreamAction`**

In `ChatContainer.tsx`, update `StreamState` interface (around line 55-62):

```typescript
interface StreamState {
  isStreaming: boolean
  streamingContent: string
  streamingThinking: string
  streamingToolCalls: any[]
  streamProgress: StreamProgressType
  currentPlanStep: string
  // NEW: execution plan state
  executionPlan: ExecutionPlan | null
  planStepStates: Map<number, 'pending' | 'running' | 'completed' | 'failed'>
}
```

Add to `StreamAction` union (around line 64-75):

```typescript
  | { type: 'EXECUTION_PLAN'; plan: ExecutionPlan }
  | { type: 'PLAN_STEP_STARTED'; stepId: number; description: string }
  | { type: 'PLAN_STEP_COMPLETED'; stepId: number; success: boolean; summary: string }
```

Update initial state and reducer to handle these new actions.

- [ ] **Step 2: Add WebSocket event handlers**

In the `handleMessage` function's `switch` statement (around line 272-382), add new cases:

```typescript
    case "ExecutionPlanCreated":
      dispatch({ type: 'EXECUTION_PLAN', plan: data.plan })
      break

    case "PlanStepStarted":
      dispatch({ type: 'PLAN_STEP_STARTED', stepId: data.stepId, description: data.description })
      break

    case "PlanStepCompleted":
      dispatch({ type: 'PLAN_STEP_COMPLETED', stepId: data.stepId, success: data.success, summary: data.summary })
      break
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind/web" && npx tsc --noEmit 2>&1 | head -10`
Expected: No type errors

- [ ] **Step 4: Commit**

```bash
cd "/Users/shenmingming/CamThink Project/NeoMind"
git add web/src/components/chat/ChatContainer.tsx
git commit -m "feat(web): handle ExecutionPlan events in ChatContainer reducer"
```

---

## Task 9: ExecutionPlanPanel Component

**Files:**
- Create: `web/src/components/chat/ExecutionPlanPanel.tsx`
- Modify: `web/src/components/chat/MergedMessageList.tsx`

- [ ] **Step 1: Create `ExecutionPlanPanel` component**

```tsx
// web/src/components/chat/ExecutionPlanPanel.tsx
import { useState } from 'react'
import type { ExecutionPlan } from '../../types'

interface PlanStepState {
  status: 'pending' | 'running' | 'completed' | 'failed'
  summary?: string
}

interface ExecutionPlanPanelProps {
  plan: ExecutionPlan
  stepStates: Map<number, PlanStepState>
}

export function ExecutionPlanPanel({ plan, stepStates }: ExecutionPlanPanelProps) {
  const [collapsed, setCollapsed] = useState(false)
  const allDone = plan.steps.every(
    (_, i) => stepStates.get(i)?.status === 'completed' || stepStates.get(i)?.status === 'failed'
  )

  const statusIcon = (status: PlanStepState['status']) => {
    switch (status) {
      case 'completed': return '✅'
      case 'running': return '⏳'
      case 'failed': return '❌'
      default: return '⬜'
    }
  }

  return (
    <div className="my-2 border border-border rounded-lg overflow-hidden">
      <button
        className="w-full flex items-center justify-between px-3 py-2 bg-muted/50 text-sm hover:bg-muted/70 transition-colors"
        onClick={() => setCollapsed(!collapsed)}
      >
        <span className="font-medium">
          Execution Plan ({plan.steps.length} steps, {plan.mode === 'keyword' ? 'fast' : 'detailed'})
        </span>
        <span className="text-xs text-muted-foreground">
          {collapsed ? '▶' : '▼'} {allDone ? 'Done' : 'Running...'}
        </span>
      </button>

      {!collapsed && (
        <div className="px-3 py-2 space-y-1.5">
          {plan.steps.map((step) => {
            const state = stepStates.get(step.id) ?? { status: 'pending' as const }
            return (
              <div key={step.id} className="flex items-start gap-2 text-sm">
                <span className="mt-0.5">{statusIcon(state.status)}</span>
                <div className="flex-1 min-w-0">
                  <div className="truncate">{step.description}</div>
                  {state.summary && (
                    <div className="text-xs text-muted-foreground mt-0.5 truncate">
                      {state.summary}
                    </div>
                  )}
                </div>
                <span className="text-xs text-muted-foreground shrink-0">
                  {step.tool_name}:{step.action}
                </span>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Integrate into MergedMessageList**

In `MergedMessageList.tsx`, import and render `ExecutionPlanPanel` when an execution plan is available in the streaming state. Pass `executionPlan` and `planStepStates` from the parent `ChatContainer`.

- [ ] **Step 3: Verify it compiles**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind/web" && npx tsc --noEmit 2>&1 | head -10`
Expected: No type errors

- [ ] **Step 4: Commit**

```bash
cd "/Users/shenmingming/CamThink Project/NeoMind"
git add web/src/components/chat/ExecutionPlanPanel.tsx
git add web/src/components/chat/MergedMessageList.tsx
git commit -m "feat(web): add ExecutionPlanPanel component with step progress"
```

---

## Task 10: End-to-End Integration Test

**Files:**
- Create: `crates/neomind-agent/tests/planning_integration.rs`

- [ ] **Step 1: Write integration test**

```rust
// crates/neomind-agent/tests/planning_integration.rs
//! Integration tests for the planning system.

use neomind_agent::agent::planner::types::{ExecutionPlan, PlanningConfig, PlanningMode, PlanStep};
use neomind_agent::agent::staged::{IntentCategory, IntentResult, PlanningCoordinator};
use neomind_agent::agent::context_selector::ContextBundle;

#[tokio::test]
async fn test_keyword_planner_device_query() {
    let coord = PlanningCoordinator::new(PlanningConfig::default());
    let intent = IntentResult {
        category: IntentCategory::Device,
        confidence: 0.9,
        keywords: vec!["设备".into()],
    };

    let plan = coord.plan(&intent, &ContextBundle::default(), "查询客厅温度").await;
    assert!(plan.is_some());
    let plan = plan.unwrap();
    assert_eq!(plan.mode, PlanningMode::Keyword);
    assert!(!plan.steps.is_empty());
}

#[tokio::test]
async fn test_planning_disabled_skips() {
    let mut config = PlanningConfig::default();
    config.enabled = false;
    let coord = PlanningCoordinator::new(config);
    let intent = IntentResult {
        category: IntentCategory::Device,
        confidence: 0.9,
        keywords: vec!["设备".into()],
    };

    let plan = coord.plan(&intent, &ContextBundle::default(), "查询设备").await;
    assert!(plan.is_none());
}

#[tokio::test]
async fn test_general_intent_skips_planning() {
    let coord = PlanningCoordinator::new(PlanningConfig::default());
    let intent = IntentResult {
        category: IntentCategory::General,
        confidence: 0.5,
        keywords: vec![],
    };

    let plan = coord.plan(&intent, &ContextBundle::default(), "你好").await;
    assert!(plan.is_none());
}

#[test]
fn test_execution_plan_parallel_batches() {
    let plan = ExecutionPlan {
        steps: vec![
            PlanStep {
                id: 0, tool_name: "device".into(), action: "query".into(),
                params: serde_json::json!({}), depends_on: vec![], description: "Step 0".into(),
            },
            PlanStep {
                id: 1, tool_name: "device".into(), action: "query".into(),
                params: serde_json::json!({}), depends_on: vec![], description: "Step 1".into(),
            },
            PlanStep {
                id: 2, tool_name: "device".into(), action: "control".into(),
                params: serde_json::json!({}), depends_on: vec![0, 1], description: "Step 2".into(),
            },
        ],
        mode: PlanningMode::Keyword,
    };

    let batches = plan.parallel_batches();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0], vec![0, 1]); // Two queries in parallel
    assert_eq!(batches[1], vec![2]);     // Control waits
}
```

- [ ] **Step 2: Run integration tests**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo test -p neomind-agent --test planning_integration 2>&1 | tail -15`
Expected: All 4 tests PASS

- [ ] **Step 3: Commit**

```bash
cd "/Users/shenmingming/CamThink Project/NeoMind"
git add crates/neomind-agent/tests/planning_integration.rs
git commit -m "test(agent): add planning integration tests"
```

---

## Task 11: Final Validation

- [ ] **Step 1: Run full Rust test suite**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo test -p neomind-agent 2>&1 | tail -20`
Expected: All tests PASS

- [ ] **Step 2: Run cargo clippy**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind" && cargo clippy -p neomind-agent 2>&1 | tail -10`
Expected: No new warnings

- [ ] **Step 3: Run frontend type check**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind/web" && npx tsc --noEmit 2>&1 | head -10`
Expected: No type errors

- [ ] **Step 4: Run frontend build**

Run: `cd "/Users/shenmingming/CamThink Project/NeoMind/web" && npm run build 2>&1 | tail -10`
Expected: Build succeeds
