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
    /// Tool name: "device", "agent", "rule", "message", "extension"
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
                | ("agent", "list" | "get" | "memory" | "executions" | "conversation")
                | ("message", "list")
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
            let mut deferred = Vec::new();

            for step in &self.steps {
                if completed.contains(&step.id) {
                    continue;
                }
                // Check all dependencies are satisfied
                let deps_met = step.depends_on.iter().all(|dep| completed.contains(dep));
                if !deps_met {
                    continue;
                }

                if step.is_safe_parallel() {
                    // Safe steps can share a batch
                    batch.push(step.id);
                } else {
                    // Destructive steps get their own batch (single item)
                    // If batch already has safe steps, defer this one
                    if batch.is_empty() {
                        batch.push(step.id);
                    } else {
                        deferred.push(step.id);
                    }
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
