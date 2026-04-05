//! Planning coordinator — routes between KeywordPlanner and LLMPlanner.

use std::sync::Arc;

use super::keyword::KeywordPlanner;
use super::llm_planner::LLMPlanner;
use super::types::{ExecutionPlan, PlanningConfig};
use crate::agent::staged::{IntentCategory, IntentResult};
use crate::context_selector::ContextBundle;
use crate::llm::LlmInterface;

/// Coordinates planning between keyword and LLM planners.
pub struct PlanningCoordinator {
    config: PlanningConfig,
    keyword_planner: KeywordPlanner,
    llm_interface: Option<Arc<LlmInterface>>,
}

impl PlanningCoordinator {
    pub fn new(config: PlanningConfig) -> Self {
        Self {
            config,
            keyword_planner: KeywordPlanner::new(),
            llm_interface: None,
        }
    }

    pub fn with_llm(mut self, llm: Arc<LlmInterface>) -> Self {
        self.llm_interface = Some(llm);
        self
    }

    /// Whether to use the keyword planner for this intent.
    pub fn should_use_keyword_planner(&self, intent: &IntentResult) -> bool {
        intent.confidence >= self.config.keyword_threshold
            && intent.category != IntentCategory::Workflow
    }

    /// Generate an execution plan using the appropriate planner.
    pub async fn plan(
        &self,
        intent: &IntentResult,
        context: &ContextBundle,
        user_message: &str,
    ) -> Option<ExecutionPlan> {
        if !self.config.enabled {
            return None;
        }

        if self.should_use_keyword_planner(intent) {
            self.keyword_planner.plan_sync(intent, user_message)
        } else if let Some(llm) = &self.llm_interface {
            let llm_planner = LLMPlanner::new(llm.clone(), self.config.llm_timeout_secs);
            llm_planner.plan(intent, context, user_message).await
        } else {
            // No LLM available, try keyword planner as fallback
            self.keyword_planner.plan_sync(intent, user_message)
        }
    }
}

impl Default for PlanningCoordinator {
    fn default() -> Self {
        Self::new(PlanningConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_use_keyword_high_confidence() {
        let coord = PlanningCoordinator::default();
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["设备".into()],
        };
        assert!(coord.should_use_keyword_planner(&intent));
    }

    #[test]
    fn test_should_use_llm_for_workflow() {
        let coord = PlanningCoordinator::default();
        let intent = IntentResult {
            category: IntentCategory::Workflow,
            confidence: 0.95,
            keywords: vec!["工作流".into()],
        };
        assert!(!coord.should_use_keyword_planner(&intent));
    }

    #[test]
    fn test_should_use_llm_for_low_confidence() {
        let coord = PlanningCoordinator::default();
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.5,
            keywords: vec![],
        };
        assert!(!coord.should_use_keyword_planner(&intent));
    }

    #[tokio::test]
    async fn test_planning_disabled() {
        let mut config = PlanningConfig::default();
        config.enabled = false;
        let coord = PlanningCoordinator::new(config);
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["设备".into()],
        };
        let context = ContextBundle {
            device_types: vec![],
            rules: vec![],
            commands: vec![],
            estimated_tokens: 0,
        };
        let plan = coord.plan(&intent, &context, "查询设备").await;
        assert!(plan.is_none());
    }

    #[tokio::test]
    async fn test_keyword_fallback_when_no_llm() {
        let coord = PlanningCoordinator::default(); // No LLM
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.5, // Low confidence → would normally use LLM
            keywords: vec![],
        };
        let context = ContextBundle {
            device_types: vec![],
            rules: vec![],
            commands: vec![],
            estimated_tokens: 0,
        };
        // Falls back to keyword planner since no LLM available
        let plan = coord.plan(&intent, &context, "查询所有设备").await;
        assert!(plan.is_some());
        let plan = plan.unwrap();
        assert_eq!(plan.mode, super::super::types::PlanningMode::Keyword);
    }
}
