//! Keyword-based planner - rule-based mapping from IntentCategory to execution plans.
//!
//! This planner provides zero-cost, sub-millisecond planning by mapping intent categories
//! to predefined plan templates. No LLM calls are made.

use crate::agent::planner::types::{ExecutionPlan, PlanStep, PlanningMode, StepId};
use crate::agent::staged::{IntentCategory, IntentResult};

/// Keyword-based planner that maps intents to execution plans.
#[derive(Clone, Debug)]
pub struct KeywordPlanner {
    /// Control keywords for device sub-intent detection
    control_keywords: Vec<String>,
}

impl Default for KeywordPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl KeywordPlanner {
    /// Create a new keyword planner.
    pub fn new() -> Self {
        Self {
            control_keywords: vec![
                "控制".to_string(),
                "打开".to_string(),
                "关闭".to_string(),
                "开关".to_string(),
                "设置".to_string(),
                "control".to_string(),
                "turn on".to_string(),
                "turn off".to_string(),
                "open".to_string(),
                "close".to_string(),
            ],
        }
    }

    /// Create a keyword planner with custom control keywords.
    pub fn with_control_keywords(keywords: Vec<String>) -> Self {
        Self {
            control_keywords: keywords,
        }
    }

    /// Generate a plan from intent classification result.
    ///
    /// Returns `None` if:
    /// - Intent is `System`, `Help`, or `General` (no planning needed)
    /// - Intent is `Workflow` (defer to LLM planner)
    pub fn plan_sync(&self, intent: &IntentResult, message: &str) -> Option<ExecutionPlan> {
        match intent.category {
            IntentCategory::Device => self.plan_device(intent, message),
            IntentCategory::Rule => self.plan_rule(),
            IntentCategory::Data => self.plan_data(),
            IntentCategory::Alert => self.plan_alert(),
            IntentCategory::System => None, // Skip planning
            IntentCategory::Help => None,   // Skip planning
            IntentCategory::General => None, // Skip planning
            IntentCategory::Workflow => None, // Defer to LLM planner
        }
    }

    /// Plan for device-related intents.
    /// Detects control vs query from message keywords.
    fn plan_device(&self, _intent: &IntentResult, message: &str) -> Option<ExecutionPlan> {
        let message_lower = message.to_lowercase();
        let is_control = self
            .control_keywords
            .iter()
            .any(|kw| message_lower.contains(&kw.to_lowercase()));

        let action = if is_control { "control" } else { "query" };
        let description = if is_control {
            "控制设备".to_string()
        } else {
            "查询设备状态".to_string()
        };

        let step = PlanStep {
            id: 0,
            tool_name: "device".to_string(),
            action: action.to_string(),
            params: serde_json::json!({ "message": message }),
            depends_on: vec![],
            description,
        };

        Some(ExecutionPlan {
            steps: vec![step],
            mode: PlanningMode::Keyword,
        })
    }

    /// Plan for rule-related intents.
    /// Single step: list rules.
    fn plan_rule(&self) -> Option<ExecutionPlan> {
        let step = PlanStep {
            id: 0,
            tool_name: "rule".to_string(),
            action: "list".to_string(),
            params: serde_json::json!({}),
            depends_on: vec![],
            description: "列出自动化规则".to_string(),
        };

        Some(ExecutionPlan {
            steps: vec![step],
            mode: PlanningMode::Keyword,
        })
    }

    /// Plan for data-related intents.
    /// Single step: query device data.
    fn plan_data(&self) -> Option<ExecutionPlan> {
        let step = PlanStep {
            id: 0,
            tool_name: "device".to_string(),
            action: "query".to_string(),
            params: serde_json::json!({}),
            depends_on: vec![],
            description: "查询设备数据".to_string(),
        };

        Some(ExecutionPlan {
            steps: vec![step],
            mode: PlanningMode::Keyword,
        })
    }

    /// Plan for alert-related intents.
    /// Single step: list alerts.
    fn plan_alert(&self) -> Option<ExecutionPlan> {
        let step = PlanStep {
            id: 0,
            tool_name: "alert".to_string(),
            action: "list".to_string(),
            params: serde_json::json!({}),
            depends_on: vec![],
            description: "列出告警信息".to_string(),
        };

        Some(ExecutionPlan {
            steps: vec![step],
            mode: PlanningMode::Keyword,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_device_query() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["设备".to_string()],
        };
        let plan = planner.plan_sync(&intent, "查看客厅温度传感器").unwrap();
        assert_eq!(plan.mode, PlanningMode::Keyword);
        assert!(plan.steps.len() >= 1);
        assert_eq!(plan.steps[0].tool_name, "device");
        assert_eq!(plan.steps[0].action, "query");
    }

    #[test]
    fn test_device_control() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["控制".to_string()],
        };
        let plan = planner.plan_sync(&intent, "控制客厅灯").unwrap();
        assert_eq!(plan.steps[0].action, "control");
    }

    #[test]
    fn test_device_control_english() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["control".to_string()],
        };
        let plan = planner
            .plan_sync(&intent, "Turn on the living room light")
            .unwrap();
        assert_eq!(plan.steps[0].action, "control");
    }

    #[test]
    fn test_general_skips() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::General,
            confidence: 0.5,
            keywords: vec![],
        };
        assert!(planner.plan_sync(&intent, "你好").is_none());
    }

    #[test]
    fn test_help_skips() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Help,
            confidence: 0.9,
            keywords: vec!["帮助".to_string()],
        };
        assert!(planner.plan_sync(&intent, "怎么用").is_none());
    }

    #[test]
    fn test_workflow_skips() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Workflow,
            confidence: 0.9,
            keywords: vec!["工作流".to_string()],
        };
        assert!(planner.plan_sync(&intent, "执行工作流").is_none());
    }

    #[test]
    fn test_system_skips() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::System,
            confidence: 0.9,
            keywords: vec!["系统".to_string()],
        };
        assert!(planner.plan_sync(&intent, "系统状态").is_none());
    }

    #[test]
    fn test_rule_plan() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Rule,
            confidence: 0.9,
            keywords: vec!["规则".to_string()],
        };
        let plan = planner.plan_sync(&intent, "查看规则列表").unwrap();
        assert_eq!(plan.mode, PlanningMode::Keyword);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].tool_name, "rule");
        assert_eq!(plan.steps[0].action, "list");
    }

    #[test]
    fn test_data_plan() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Data,
            confidence: 0.9,
            keywords: vec!["数据".to_string()],
        };
        let plan = planner.plan_sync(&intent, "查询温度数据").unwrap();
        assert_eq!(plan.mode, PlanningMode::Keyword);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].tool_name, "device");
        assert_eq!(plan.steps[0].action, "query");
    }

    #[test]
    fn test_alert_plan() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Alert,
            confidence: 0.9,
            keywords: vec!["告警".to_string()],
        };
        let plan = planner.plan_sync(&intent, "查看告警").unwrap();
        assert_eq!(plan.mode, PlanningMode::Keyword);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].tool_name, "alert");
        assert_eq!(plan.steps[0].action, "list");
    }

    #[test]
    fn test_custom_control_keywords() {
        let custom_keywords = vec!["切换".to_string(), "toggle".to_string()];
        let planner = KeywordPlanner::with_control_keywords(custom_keywords);
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["设备".to_string()],
        };
        let plan = planner.plan_sync(&intent, "切换客厅灯").unwrap();
        assert_eq!(plan.steps[0].action, "control");
    }

    #[test]
    fn test_plan_step_structure() {
        let planner = KeywordPlanner::new();
        let intent = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["设备".to_string()],
        };
        let plan = planner.plan_sync(&intent, "查看设备").unwrap();

        let step = &plan.steps[0];
        assert_eq!(step.id, 0);
        assert!(!step.tool_name.is_empty());
        assert!(!step.action.is_empty());
        assert!(step.depends_on.is_empty());
        assert!(!step.description.is_empty());
    }
}
