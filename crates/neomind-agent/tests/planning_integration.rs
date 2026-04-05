//! Integration tests for the planning system.
//!
//! These tests verify the end-to-end behavior of:
//! - KeywordPlanner: Rule-based plan generation
//! - PlanningCoordinator: Routing between planners
//! - ExecutionPlan: Parallel batch computation
//!
//! All types used here are re-exported from the neomind-agent crate root.

use neomind_agent::{
    ContextBundle, ExecutionPlan, IntentCategory, IntentResult, KeywordPlanner, PlanningConfig,
    PlanningCoordinator, PlanningMode,
};

/// Helper to create an IntentResult for testing
fn create_intent(category: IntentCategory, confidence: f32, keywords: Vec<&str>) -> IntentResult {
    IntentResult {
        category,
        confidence,
        keywords: keywords.into_iter().map(String::from).collect(),
    }
}

/// Helper to create an empty ContextBundle
fn empty_context() -> ContextBundle {
    ContextBundle {
        device_types: vec![],
        rules: vec![],
        commands: vec![],
        estimated_tokens: 0,
    }
}

#[test]
fn test_keyword_planner_device_intent() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::Device, 0.9, vec!["设备"]);
    let message = "查看客厅温度传感器";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_some(), "Device intent should produce a plan");
    let plan = plan.unwrap();
    assert_eq!(plan.mode, PlanningMode::Keyword);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].tool_name, "device");
    assert_eq!(plan.steps[0].action, "query");
    assert!(plan.steps[0].description.contains("查询"));
}

#[test]
fn test_keyword_planner_device_control_intent() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::Device, 0.9, vec!["控制"]);
    let message = "控制客厅灯打开";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_some(), "Device control intent should produce a plan");
    let plan = plan.unwrap();
    assert_eq!(plan.steps[0].action, "control");
    assert!(plan.steps[0].description.contains("控制"));
}

#[test]
fn test_keyword_planner_device_control_english() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::Device, 0.9, vec!["control"]);
    let message = "Turn on the living room light";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_some());
    let plan = plan.unwrap();
    assert_eq!(plan.steps[0].action, "control");
}

#[test]
fn test_keyword_planner_rule_intent() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::Rule, 0.9, vec!["规则"]);
    let message = "查看规则列表";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_some(), "Rule intent should produce a plan");
    let plan = plan.unwrap();
    assert_eq!(plan.mode, PlanningMode::Keyword);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].tool_name, "rule");
    assert_eq!(plan.steps[0].action, "list");
}

#[test]
fn test_keyword_planner_data_intent() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::Data, 0.9, vec!["数据"]);
    let message = "查询温度数据";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_some(), "Data intent should produce a plan");
    let plan = plan.unwrap();
    assert_eq!(plan.mode, PlanningMode::Keyword);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].tool_name, "device");
    assert_eq!(plan.steps[0].action, "query");
}

#[test]
fn test_keyword_planner_alert_intent() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::Alert, 0.9, vec!["告警"]);
    let message = "查看告警信息";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_some(), "Alert intent should produce a plan");
    let plan = plan.unwrap();
    assert_eq!(plan.mode, PlanningMode::Keyword);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].tool_name, "alert");
    assert_eq!(plan.steps[0].action, "list");
}

#[test]
fn test_keyword_planner_general_skips() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::General, 0.5, vec![]);
    let message = "你好";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_none(), "General intent should skip planning");
}

#[test]
fn test_keyword_planner_help_skips() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::Help, 0.9, vec!["帮助"]);
    let message = "怎么使用这个系统";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_none(), "Help intent should skip planning");
}

#[test]
fn test_keyword_planner_workflow_skips() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::Workflow, 0.9, vec!["工作流"]);
    let message = "执行工作流";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_none(), "Workflow intent should skip planning (defer to LLM)");
}

#[test]
fn test_keyword_planner_system_skips() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::System, 0.9, vec!["系统"]);
    let message = "系统状态";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_none(), "System intent should skip planning");
}

#[test]
fn test_execution_plan_parallel_batches_single_step() {
    let plan = ExecutionPlan {
        steps: vec![],
        mode: PlanningMode::Keyword,
    };

    let batches = plan.parallel_batches();

    assert_eq!(batches.len(), 0, "Empty plan should have no batches");
}

#[test]
fn test_execution_plan_parallel_batches_all_safe_parallel() {
    use neomind_agent::PlanStep;

    let plan = ExecutionPlan {
        steps: vec![
            PlanStep {
                id: 0,
                tool_name: "device".to_string(),
                action: "list".to_string(),
                params: serde_json::json!({}),
                depends_on: vec![],
                description: "List devices".to_string(),
            },
            PlanStep {
                id: 1,
                tool_name: "rule".to_string(),
                action: "list".to_string(),
                params: serde_json::json!({}),
                depends_on: vec![],
                description: "List rules".to_string(),
            },
            PlanStep {
                id: 2,
                tool_name: "alert".to_string(),
                action: "list".to_string(),
                params: serde_json::json!({}),
                depends_on: vec![],
                description: "List alerts".to_string(),
            },
        ],
        mode: PlanningMode::Keyword,
    };

    let batches = plan.parallel_batches();

    assert_eq!(batches.len(), 1, "All safe-parallel steps should be in one batch");
    assert_eq!(batches[0].len(), 3, "All three steps should be in the batch");
}

#[test]
fn test_execution_plan_parallel_batches_with_dependencies() {
    use neomind_agent::PlanStep;

    let plan = ExecutionPlan {
        steps: vec![
            PlanStep {
                id: 0,
                tool_name: "device".to_string(),
                action: "list".to_string(),
                params: serde_json::json!({}),
                depends_on: vec![],
                description: "List devices".to_string(),
            },
            PlanStep {
                id: 1,
                tool_name: "device".to_string(),
                action: "query".to_string(),
                params: serde_json::json!({}),
                depends_on: vec![0], // Depends on step 0
                description: "Query device".to_string(),
            },
        ],
        mode: PlanningMode::Keyword,
    };

    let batches = plan.parallel_batches();

    assert_eq!(batches.len(), 2, "Should have two batches due to dependency");
    assert_eq!(batches[0], vec![0], "First batch should have step 0");
    assert_eq!(batches[1], vec![1], "Second batch should have step 1");
}

#[test]
fn test_execution_plan_parallel_batches_mixed_safe_unsafe() {
    use neomind_agent::PlanStep;

    let plan = ExecutionPlan {
        steps: vec![
            PlanStep {
                id: 0,
                tool_name: "device".to_string(),
                action: "list".to_string(),
                params: serde_json::json!({}),
                depends_on: vec![],
                description: "List devices".to_string(),
            },
            PlanStep {
                id: 1,
                tool_name: "rule".to_string(),
                action: "list".to_string(),
                params: serde_json::json!({}),
                depends_on: vec![],
                description: "List rules".to_string(),
            },
            PlanStep {
                id: 2,
                tool_name: "device".to_string(),
                action: "control".to_string(),
                params: serde_json::json!({}),
                depends_on: vec![],
                description: "Control device".to_string(),
            },
        ],
        mode: PlanningMode::Keyword,
    };

    let batches = plan.parallel_batches();

    assert_eq!(batches.len(), 2, "Should have two batches");
    assert_eq!(batches[0].len(), 2, "First batch should have two safe-parallel steps");
    assert_eq!(batches[1].len(), 1, "Second batch should have one unsafe step");
    assert!(batches[1].contains(&2), "Second batch should contain the control step");
}

#[test]
fn test_execution_plan_is_empty() {
    let plan = ExecutionPlan {
        steps: vec![],
        mode: PlanningMode::Keyword,
    };

    assert!(plan.is_empty());
}

#[test]
fn test_execution_plan_not_empty() {
    use neomind_agent::PlanStep;

    let plan = ExecutionPlan {
        steps: vec![PlanStep {
            id: 0,
            tool_name: "device".to_string(),
            action: "list".to_string(),
            params: serde_json::json!({}),
            depends_on: vec![],
            description: "List devices".to_string(),
        }],
        mode: PlanningMode::Keyword,
    };

    assert!(!plan.is_empty());
}

#[tokio::test]
async fn test_planning_coordinator_default_config() {
    let coord = PlanningCoordinator::default();
    let intent = create_intent(IntentCategory::Device, 0.9, vec!["设备"]);
    let context = empty_context();
    let message = "查看设备列表";

    let plan = coord.plan(&intent, &context, message).await;

    assert!(plan.is_some(), "High confidence intent should produce a plan");
    let plan = plan.unwrap();
    assert_eq!(plan.mode, PlanningMode::Keyword);
}

#[tokio::test]
async fn test_planning_coordinator_disabled_returns_none() {
    let mut config = PlanningConfig::default();
    config.enabled = false;
    let coord = PlanningCoordinator::new(config);

    let intent = create_intent(IntentCategory::Device, 0.9, vec!["设备"]);
    let context = empty_context();
    let message = "查看设备列表";

    let plan = coord.plan(&intent, &context, message).await;

    assert!(plan.is_none(), "Disabled planning should return None");
}

#[tokio::test]
async fn test_planning_coordinator_high_confidence_uses_keyword() {
    let coord = PlanningCoordinator::default();
    let intent = create_intent(IntentCategory::Device, 0.95, vec!["设备"]);
    let context = empty_context();
    let message = "列出所有设备";

    let plan = coord.plan(&intent, &context, message).await;

    assert!(plan.is_some());
    let plan = plan.unwrap();
    assert_eq!(plan.mode, PlanningMode::Keyword, "High confidence should use Keyword planner");
}

#[tokio::test]
async fn test_planning_coordinator_low_confidence_falls_back_to_keyword() {
    // No LLM configured, so it should fall back to keyword planner
    let coord = PlanningCoordinator::default();
    let intent = create_intent(IntentCategory::Device, 0.5, vec![]);
    let context = empty_context();
    let message = "查看设备";

    let plan = coord.plan(&intent, &context, message).await;

    assert!(plan.is_some(), "Should fall back to keyword planner when no LLM");
    let plan = plan.unwrap();
    assert_eq!(plan.mode, PlanningMode::Keyword);
}

#[tokio::test]
async fn test_planning_coordinator_workflow_defers_to_llm() {
    let coord = PlanningCoordinator::default();
    let intent = create_intent(IntentCategory::Workflow, 0.95, vec!["工作流"]);
    let context = empty_context();
    let message = "执行工作流";

    // Without LLM, it should fall back to keyword planner which returns None for Workflow
    let plan = coord.plan(&intent, &context, message).await;

    // Workflow returns None from keyword planner, and we have no LLM
    assert!(plan.is_none() || plan.unwrap().steps.is_empty());
}

#[test]
fn test_planning_config_default_values() {
    let config = PlanningConfig::default();

    assert!(config.enabled, "Planning should be enabled by default");
    assert_eq!(config.keyword_threshold, 0.8);
    assert_eq!(config.max_entities_for_keyword, 3);
    assert_eq!(config.llm_timeout_secs, 2);
}

#[test]
fn test_keyword_planner_custom_control_keywords() {
    let custom_keywords = vec!["切换".to_string(), "toggle".to_string()];
    let planner = KeywordPlanner::with_control_keywords(custom_keywords);
    let intent = create_intent(IntentCategory::Device, 0.9, vec!["设备"]);
    let message = "切换客厅灯";

    let plan = planner.plan_sync(&intent, message);

    assert!(plan.is_some());
    let plan = plan.unwrap();
    assert_eq!(plan.steps[0].action, "control");
}

#[test]
fn test_plan_step_structure() {
    let planner = KeywordPlanner::new();
    let intent = create_intent(IntentCategory::Device, 0.9, vec!["设备"]);
    let message = "查看设备";

    let plan = planner.plan_sync(&intent, message).unwrap();
    let step = &plan.steps[0];

    assert_eq!(step.id, 0);
    assert!(!step.tool_name.is_empty());
    assert!(!step.action.is_empty());
    assert!(step.depends_on.is_empty());
    assert!(!step.description.is_empty());
}
