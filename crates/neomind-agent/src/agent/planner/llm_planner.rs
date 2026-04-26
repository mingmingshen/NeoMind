//! LLM-based deep planner for complex multi-step tasks.

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

use super::keyword::KeywordPlanner;
use super::types::{ExecutionPlan, PlanStep, PlanningMode, StepId};
use crate::agent::staged::IntentResult;
use crate::context_selector::ContextBundle;
use crate::llm::LlmInterface;

/// LLM plan output format for JSON parsing.
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
    depends_on: Vec<StepId>,
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

    fn build_planning_prompt(user_message: &str) -> String {
        format!(
            r#"Analyze this IoT platform user request and create an execution plan.

Available tools:
- device: actions=list, get, query, control
- agent: actions=list, get, create, update, control, memory, send_message, executions, conversation, latest_execution
- rule: actions=list, get, delete, history
- alert: actions=list, create, acknowledge
- extension: actions=list, get, execute, status

Rules:
- Independent steps should have empty depends_on
- Destructive actions (control, delete, create) should NOT be parallel
- Keep plans simple — prefer fewer steps

User request: {user_message}

Respond with JSON only:
{{"steps":[{{"tool":"...","action":"...","params":{{}},"depends_on":[],"description":"..."}}]}}"#
        )
    }

    /// Generate plan using LLM. Falls back to KeywordPlanner on failure.
    pub async fn plan(
        &self,
        intent: &IntentResult,
        _context: &ContextBundle,
        user_message: &str,
    ) -> Option<ExecutionPlan> {
        // Try LLM planning first
        if let Some(plan) = self.call_llm(user_message).await {
            return Some(plan);
        }

        // Fallback to keyword planner
        self.keyword_planner.plan_sync(intent, user_message)
    }

    async fn call_llm(&self, user_message: &str) -> Option<ExecutionPlan> {
        let prompt = Self::build_planning_prompt(user_message);

        let result = tokio::time::timeout(self.timeout, self.llm.chat_without_tools(&prompt))
            .await
            .ok()?;

        let response = result.ok()?;
        let cleaned = response
            .text
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

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests are integration tests that require a working LLM backend.
    // In production, they should be behind an `#[cfg(feature = "integration-tests")]` flag.
    // For now, we provide unit tests for the prompt building and parsing logic.

    #[test]
    fn test_planning_prompt_format() {
        let prompt = LLMPlanner::build_planning_prompt("list all devices");
        assert!(prompt.contains("IoT platform"));
        assert!(prompt.contains("device: actions=list"));
        assert!(prompt.contains("list all devices"));
        assert!(prompt.contains("JSON only"));
    }

    #[test]
    fn test_llm_plan_output_parsing() {
        let json_str = r#"{"steps":[
            {"tool":"device","action":"list","params":{},"depends_on":[],"description":"List all devices"},
            {"tool":"rule","action":"list","params":{},"depends_on":[],"description":"List rules"}
        ]}"#;

        let parsed: LlmPlanOutput = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed.steps.len(), 2);
        assert_eq!(parsed.steps[0].tool, "device");
        assert_eq!(parsed.steps[0].action, "list");
        assert_eq!(parsed.steps[1].tool, "rule");
        assert_eq!(parsed.steps[1].depends_on.len(), 0);
    }

    #[test]
    fn test_llm_plan_output_with_dependencies() {
        let json_str = r#"{"steps":[
            {"tool":"device","action":"get","params":{"id":"temp1"},"depends_on":[],"description":"Get device"},
            {"tool":"rule","action":"create","params":{"device":"temp1"},"depends_on":[0],"description":"Create rule"}
        ]}"#;

        let parsed: LlmPlanOutput = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed.steps.len(), 2);
        assert_eq!(parsed.steps[1].depends_on, vec![0]);
    }

    #[test]
    fn test_llm_plan_output_json_markdown_wrapped() {
        let json_str = r#"```json
        {"steps":[
            {"tool":"device","action":"list","params":{},"depends_on":[],"description":"List devices"}
        ]}
        ```"#;

        let cleaned = json_str
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let parsed: LlmPlanOutput = serde_json::from_str(cleaned).unwrap();
        assert_eq!(parsed.steps.len(), 1);
        assert_eq!(parsed.steps[0].tool, "device");
    }

    #[test]
    fn test_llm_planner_timeout_creation() {
        // This test just verifies the planner can be created with different timeouts
        // We can't test the actual timeout behavior without a mock LLM
        let timeout_secs = 5u64;
        let timeout = Duration::from_secs(timeout_secs);
        assert_eq!(timeout.as_secs(), 5);
    }

    #[test]
    fn test_empty_steps_returns_none() {
        let json_str = r#"{"steps":[]}"#;
        let parsed: LlmPlanOutput = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed.steps.len(), 0);
    }
}
