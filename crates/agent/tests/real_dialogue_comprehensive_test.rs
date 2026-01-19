//! Real dialogue comprehensive test.
//!
//! This module tests the integrated agent improvements with realistic user conversations.

use std::sync::Arc;
use std::pin::Pin;

use edge_ai_agent::agent::intent_classifier::{
    IntentClassifier, IntentCategory, ProcessingStrategy, Entity, EntityType,
};
use edge_ai_agent::tools::automation::{
    CreateAutomationTool,
};
use edge_ai_agent::task_orchestrator::{
    TaskOrchestrator, TaskSession, TaskStep, TaskStatus, ResponseType,
};
use edge_ai_core::tools::{Tool, ToolOutput, Result as ToolResult};

// Mock LLM for testing
struct MockLlmForTest;

#[async_trait::async_trait]
impl edge_ai_core::llm::backend::LlmRuntime for MockLlmForTest {
    fn backend_id(&self) -> edge_ai_core::llm::backend::BackendId {
        edge_ai_core::llm::backend::BackendId::new("mock")
    }

    fn model_name(&self) -> &str {
        "test"
    }

    async fn generate(&self, _input: edge_ai_core::llm::backend::LlmInput) -> std::result::Result<
        edge_ai_core::llm::backend::LlmOutput,
        edge_ai_core::llm::backend::LlmError
    > {
        use edge_ai_core::llm::backend::{LlmOutput, FinishReason, TokenUsage};
        Ok(LlmOutput {
            text: "RULE \"温度控制\"\nWHEN sensor.temperature > 30\nFOR 1 minutes\nDO NOTIFY \"温度过高\"\nEND".to_string(),
            finish_reason: FinishReason::Stop,
            usage: Some(TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
            thinking: None,
        })
    }

    async fn generate_stream(&self, _input: edge_ai_core::llm::backend::LlmInput) -> std::result::Result<
        Pin<Box<dyn futures::Stream<Item = edge_ai_core::llm::backend::StreamChunk> + Send>>,
        edge_ai_core::llm::backend::LlmError
    > {
        use futures::{Stream, stream};
        Ok(Box::pin(stream::empty()))
    }

    fn max_context_length(&self) -> usize {
        4096
    }

    fn capabilities(&self) -> edge_ai_core::llm::backend::BackendCapabilities {
        edge_ai_core::llm::backend::BackendCapabilities::default()
    }
}

/// Comprehensive test scenario
#[derive(Debug)]
struct TestScenario {
    name: &'static str,
    user_input: &'static str,
    expected_intent: IntentCategory,
    expected_entities: Vec<(EntityType, &'static str)>,
    expected_strategy: ProcessingStrategy,
}

/// Test results tracking
#[derive(Debug)]
struct TestResults {
    total_scenarios: usize,
    intent_correct: usize,
    entities_correct: usize,
    strategy_correct: usize,
    failures: Vec<String>,
}

impl TestResults {
    fn new() -> Self {
        Self {
            total_scenarios: 0,
            intent_correct: 0,
            entities_correct: 0,
            strategy_correct: 0,
            failures: Vec::new(),
        }
    }

    fn record_success(&mut self, _scenario: &str) {
        self.intent_correct += 1;
        self.strategy_correct += 1;
        self.entities_correct += 1;
    }

    fn record_failure(&mut self, scenario: &str, error: String) {
        self.failures.push(format!("{}: {}", scenario, error));
    }

    fn accuracy(&self) -> f32 {
        if self.total_scenarios == 0 {
            return 0.0;
        }
        (self.intent_correct as f32) / (self.total_scenarios as f32)
    }

    fn summary(&self) -> String {
        format!(
            "总测试: {} | 意图准确: {:.1}% | 策略准确: {:.1}% | 失败: {}",
            self.total_scenarios,
            (self.intent_correct as f32 / self.total_scenarios as f32) * 100.0,
            (self.strategy_correct as f32 / self.total_scenarios as f32) * 100.0,
            self.failures.len()
        )
    }
}

#[cfg(test)]
mod comprehensive_tests {
    use super::*;

    /// Test suite for real dialogue scenarios
    #[test]
    fn test_real_dialogue_scenarios() {
        let classifier = IntentClassifier::new();

        let scenarios = vec![
            // Query Data scenarios
            TestScenario {
                name: "查询温度",
                user_input: "客厅温度多少",
                expected_intent: IntentCategory::QueryData,
                expected_entities: vec![
                    (EntityType::Location, "客厅"),
                    (EntityType::Value, "温度"),
                ],
                expected_strategy: ProcessingStrategy::FastPath,
            },
            TestScenario {
                name: "查询设备状态",
                user_input: "查看设备状态",
                expected_intent: IntentCategory::QueryData,
                expected_entities: vec![],
                expected_strategy: ProcessingStrategy::FastPath,
            },

            // Control Device scenarios
            TestScenario {
                name: "打开空调",
                user_input: "打开客厅的空调",
                expected_intent: IntentCategory::ControlDevice,
                expected_entities: vec![
                    (EntityType::Location, "客厅"),
                    (EntityType::Device, "空调"),
                ],
                expected_strategy: ProcessingStrategy::Standard,
            },
            TestScenario {
                name: "关闭灯光",
                user_input: "关闭所有灯",
                expected_intent: IntentCategory::ControlDevice,
                expected_entities: vec![
                    (EntityType::Action, "关闭"),
                ],
                expected_strategy: ProcessingStrategy::Standard,
            },

            // Create Automation scenarios
            TestScenario {
                name: "温度告警自动化",
                user_input: "当温度超过30度时打开空调",
                expected_intent: IntentCategory::CreateAutomation,
                expected_entities: vec![
                    (EntityType::Value, "30"),
                ],
                expected_strategy: ProcessingStrategy::MultiTurn,
            },
            TestScenario {
                name: "湿度控制自动化",
                user_input: "如果湿度低于40%就开启加湿器",
                expected_intent: IntentCategory::CreateAutomation,
                expected_entities: vec![
                    (EntityType::Value, "40"),
                ],
                expected_strategy: ProcessingStrategy::MultiTurn,
            },
            TestScenario {
                name: "定时任务",
                user_input: "每天早上8点打开客厅灯",
                expected_intent: IntentCategory::CreateAutomation,
                expected_entities: vec![
                    (EntityType::Value, "8"),
                    (EntityType::Location, "客厅"),
                ],
                expected_strategy: ProcessingStrategy::MultiTurn,
            },

            // Analyze Data scenarios
            TestScenario {
                name: "趋势分析",
                user_input: "分析最近一周的温度趋势",
                expected_intent: IntentCategory::AnalyzeData,
                expected_entities: vec![],
                expected_strategy: ProcessingStrategy::Quality,
            },

            // Summarize Info scenarios
            TestScenario {
                name: "设备汇总",
                user_input: "汇总所有设备的状态",
                expected_intent: IntentCategory::SummarizeInfo,
                expected_entities: vec![],
                expected_strategy: ProcessingStrategy::Quality,
            },

            // Out of Scope scenarios
            TestScenario {
                name: "硬件安装",
                user_input: "帮我安装一个新的温度传感器",
                expected_intent: IntentCategory::OutOfScope,
                expected_entities: vec![],
                expected_strategy: ProcessingStrategy::Fallback,
            },
        ];

        let mut results = TestResults::new();

        for scenario in &scenarios {
            results.total_scenarios += 1;

            let classification = classifier.classify(scenario.user_input);

            // Check intent
            if classification.intent != scenario.expected_intent {
                results.record_failure(
                    scenario.name,
                    format!(
                        "意图错误: 期望 {:?}, 实际 {:?}",
                        scenario.expected_intent, classification.intent
                    ),
                );
                continue;
            }

            // Check strategy
            if classification.strategy != scenario.expected_strategy {
                results.record_failure(
                    scenario.name,
                    format!(
                        "策略错误: 期望 {:?}, 实际 {:?}",
                        scenario.expected_strategy, classification.strategy
                    ),
                );
                continue;
            }

            // Check entities
            let entities_match = check_entities(&classification.entities, &scenario.expected_entities);
            if !entities_match {
                results.record_failure(
                    scenario.name,
                    format!("实体不匹配: {:?}", classification.entities),
                );
                continue;
            }

            results.record_success(scenario.name);
        }

        println!("\n=== 真实对话测试结果 ===");
        println!("{}", results.summary());

        if !results.failures.is_empty() {
            println!("\n失败的测试:");
            for failure in &results.failures {
                println!("  - {}", failure);
            }
        }

        // Assert overall accuracy threshold
        assert!(
            results.accuracy() >= 0.8,
            "整体准确率低于80%: {}",
            results.summary()
        );
    }

    /// Test automation tool integration
    #[tokio::test]
    async fn test_automation_tool_integration() {
        let llm = Arc::new(MockLlmForTest);
        let classifier = Arc::new(IntentClassifier::new());
        let store = None;

        let create_tool = CreateAutomationTool::new(llm.clone(), store, classifier);

        // Test creating automation from natural language
        let result: ToolOutput = create_tool
            .execute(serde_json::json!({
                "description": "当温度超过30度时打开空调"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.data["status"], "created");

        let automation_id = result.data["automation_id"].as_str().unwrap();
        assert!(automation_id.starts_with("auto_"));

        let dsl = result.data["dsl"].as_str().unwrap();
        assert!(dsl.contains("RULE"));
        assert!(dsl.contains("WHEN"));
        assert!(dsl.contains("DO"));
    }

    /// Test task orchestrator multi-turn flow
    #[tokio::test]
    async fn test_task_orchestrator_flow() {
        let llm = Arc::new(MockLlmForTest);
        let classifier = Arc::new(IntentClassifier::new());
        let orchestrator = TaskOrchestrator::new(llm, classifier);

        // Start a complex task
        let response: edge_ai_agent::task_orchestrator::TaskResponse = orchestrator
            .start_task("创建一个温度控制自动化", "session_1")
            .await
            .unwrap();

        assert_eq!(response.response_type, ResponseType::TaskStarted);
        assert!(response.needs_input);
        assert!(!response.completed);

        let task_id = response.task_id;

        // Verify task was created
        let task_state = orchestrator.get_task_state(&task_id).await.unwrap();
        assert_eq!(task_state.status, TaskStatus::InProgress);
        assert!(task_state.steps.len() > 0);
    }

    /// Test end-to-end automation creation flow
    #[tokio::test]
    async fn test_end_to_end_automation_flow() {
        let llm = Arc::new(MockLlmForTest);
        let classifier = Arc::new(IntentClassifier::new());

        // Step 1: Classify intent
        let classification = classifier.classify("当温度超过30度时打开空调");
        assert_eq!(classification.intent, IntentCategory::CreateAutomation);
        assert_eq!(classification.strategy, ProcessingStrategy::MultiTurn);

        // Step 2: Verify entity extraction
        let has_temp_value = classification.entities.iter()
            .any(|e| e.entity_type == EntityType::Value && e.value.contains("30"));
        assert!(has_temp_value, "Should extract temperature value");

        // Step 3: Test automation creation
        let create_tool = CreateAutomationTool::new(llm.clone(), None, Arc::clone(&classifier));
        let result: ToolOutput = create_tool
            .execute(serde_json::json!({
                "description": "当温度超过30度时打开空调"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.data["status"], "created");
    }

    /// Test entity extraction accuracy
    #[test]
    fn test_entity_extraction_accuracy() {
        let classifier = IntentClassifier::new();

        let test_cases = vec![
            ("客厅温度25度", vec![(EntityType::Location, "客厅"), (EntityType::Value, "25")]),
            ("打开卧室灯", vec![(EntityType::Action, "打开"), (EntityType::Location, "卧室")]),
            ("设置温度为30", vec![(EntityType::Value, "30")]),
        ];

        for (input, expected_entities) in test_cases {
            let classification = classifier.classify(input);

            for (expected_type, expected_value) in &expected_entities {
                let found = classification.entities.iter().any(|e| {
                    e.entity_type == *expected_type && e.value.contains(expected_value)
                });

                assert!(
                    found,
                    "实体提取失败: 输入='{}', 期望实体={:?}, 实际={:?}",
                    input, expected_entities, classification.entities
                );
            }
        }
    }

    /// Test confidence scoring
    #[test]
    fn test_confidence_scoring() {
        let classifier = IntentClassifier::new();

        // High confidence inputs (clear intent)
        let high_confidence_inputs = vec![
            "当温度超过30度时打开空调",
            "客厅湿度低于40%时开启加湿器",
            "每天早上8点打开客厅灯",
        ];

        for input in high_confidence_inputs {
            let classification = classifier.classify(input);
            assert!(
                classification.confidence >= 0.5,
                "高置信度输入置信度低于50%: '{}' -> {}",
                input, classification.confidence
            );
        }

        // Low confidence inputs (ambiguous)
        let low_confidence_inputs = vec!["打开", "查询", "设置"];

        for input in low_confidence_inputs {
            let classification = classifier.classify(input);
            // Ambiguous inputs should either be Clarify or have low confidence
            assert!(
                classification.intent == IntentCategory::Clarify || classification.confidence < 0.5,
                "模糊输入应归类为Clarify或低置信度: '{}' -> intent={:?}, confidence={}",
                input, classification.intent, classification.confidence
            );
        }
    }

    /// Test strategy recommendation
    #[test]
    fn test_strategy_recommendation() {
        let classifier = IntentClassifier::new();

        let strategy_tests = vec![
            ("客厅温度多少", ProcessingStrategy::FastPath),
            ("分析最近一周的数据", ProcessingStrategy::Quality),
            ("打开客厅灯", ProcessingStrategy::Standard),
            ("当温度高时打开空调", ProcessingStrategy::MultiTurn),
        ];

        for (input, expected_strategy) in strategy_tests {
            let classification = classifier.classify(input);
            assert_eq!(
                classification.strategy, expected_strategy,
                "策略推荐错误: 输入='{}', 期望={:?}, 实际={:?}",
                input, expected_strategy, classification.strategy
            );
        }
    }
}

// Helper function to check entities
fn check_entities(
    actual_entities: &[Entity],
    expected_entities: &[(EntityType, &str)],
) -> bool {
    for (expected_type, expected_value) in expected_entities {
        let found = actual_entities.iter().any(|e| {
            e.entity_type == *expected_type && e.value.contains(expected_value)
        });
        if !found {
            return false;
        }
    }
    true
}
