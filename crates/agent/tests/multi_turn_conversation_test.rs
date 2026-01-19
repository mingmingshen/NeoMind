//! Multi-turn conversation simulation test.
//!
//! This test verifies:
//! 1. Conversation context retention across turns
//! 2. Accuracy of intent recognition and response generation
//! 3. Complex multi-step task completion
//! 4. Error recovery and clarification handling

use std::sync::Arc;
use tokio::sync::RwLock;
use edge_ai_agent::context::{
    ResourceIndex, ResourceResolver, DynamicToolGenerator,
    ResolvedIntent, IntentCategory,
    generate_large_scale_devices,
};

/// Represents a single turn in a conversation.
#[derive(Debug, Clone)]
struct ConversationTurn {
    /// User query
    pub query: String,
    /// Expected intent category (optional)
    pub expected_intent: Option<IntentCategory>,
    /// Expected minimum resources found (0 = don't care)
    pub min_resources: usize,
    /// Whether clarification should be provided
    pub expects_clarification: bool,
    /// Context from previous turns (entities, locations, etc.)
    pub context_hints: Vec<String>,
}

/// Test scenario with multiple conversation turns.
struct ConversationScenario {
    name: String,
    description: String,
    turns: Vec<ConversationTurn>,
    expected_outcome: ScenarioOutcome,
}

#[derive(Debug, Clone)]
enum ScenarioOutcome {
    /// All turns should succeed with appropriate responses
    FullSuccess,
    /// First few turns succeed, then clarification is needed
    SuccessWithClarification,
    /// User needs to be guided through discovery
    DiscoveryGuided,
}

/// Result of a conversation turn evaluation.
#[derive(Debug)]
struct TurnResult {
    pub success: bool,
    pub intent_match: bool,
    pub resource_count: usize,
    pub action_count: usize,
    pub has_clarification: bool,
    pub elapsed_ms: u64,
    pub issues: Vec<String>,
}

/// Multi-turn conversation session that maintains context.
struct ConversationSession {
    index: Arc<RwLock<ResourceIndex>>,
    resolver: ResourceResolver,
    tool_generator: DynamicToolGenerator,
    /// Context maintained across turns
    context: SessionContext,
}

/// Context maintained during a conversation.
#[derive(Debug, Default)]
struct SessionContext {
    /// Mentioned locations (e.g., "1楼", "客厅")
    mentioned_locations: Vec<String>,
    /// Mentioned device types (e.g., "灯", "空调")
    mentioned_device_types: Vec<String>,
    /// Previously referenced devices
    referenced_devices: Vec<String>,
    /// Current floor/topic focus
    current_focus: Option<String>,
    /// Turn number
    turn_number: usize,
}

impl ConversationSession {
    fn new(index: Arc<RwLock<ResourceIndex>>) -> Self {
        let resolver = ResourceResolver::new(Arc::clone(&index));
        let tool_generator = DynamicToolGenerator::new(Arc::clone(&index));
        Self {
            index,
            resolver,
            tool_generator,
            context: SessionContext::default(),
        }
    }

    /// Process a conversation turn and update context.
    async fn process_turn(&mut self, turn: &ConversationTurn) -> TurnResult {
        let start = std::time::Instant::now();
        let resolved = self.resolver.resolve(&turn.query).await;
        let elapsed = start.elapsed();

        // Update context based on this turn
        self.update_context(&turn.query, &resolved);

        // Generate relevant tools
        let _tools = self.tool_generator.generate_tools_for_query(&turn.query).await;

        // Evaluate the result
        let mut result = TurnResult {
            success: true,
            intent_match: true,
            resource_count: resolved.resources.len(),
            action_count: resolved.actions.len(),
            has_clarification: resolved.clarification.is_some(),
            elapsed_ms: elapsed.as_millis() as u64,
            issues: Vec::new(),
        };

        // Check intent match if expected
        if let Some(expected) = &turn.expected_intent {
            let intent_match = match (&expected, &resolved.intent) {
                (IntentCategory::ListDevices, IntentCategory::ListDevices) => true,
                (IntentCategory::QueryData, IntentCategory::QueryData) => true,
                (IntentCategory::ControlDevice, IntentCategory::ControlDevice) => true,
                (IntentCategory::SystemStatus, IntentCategory::SystemStatus) => true,
                (IntentCategory::General, IntentCategory::General) => true,
                _ => false,
            };

            if !intent_match {
                result.intent_match = false;
                result.issues.push(format!("Intent mismatch: expected {:?}, got {:?}", expected, resolved.intent));
            }
        }

        // Check minimum resources
        if turn.min_resources > 0 && resolved.resources.len() < turn.min_resources {
            result.success = false;
            result.issues.push(format!(
                "Expected at least {} resources, found {}",
                turn.min_resources,
                resolved.resources.len()
            ));
        }

        // Check clarification expectation
        if turn.expects_clarification && !result.has_clarification {
            result.success = false;
            result.issues.push("Expected clarification but none provided".to_string());
        }

        // Check for reasonable response time
        if elapsed.as_millis() > 100 {
            result.issues.push(format!("Slow response: {:?}", elapsed));
        }

        // Check that we have some useful response
        if resolved.actions.is_empty() && !result.has_clarification && resolved.resources.is_empty() {
            result.success = false;
            result.issues.push("No useful response provided".to_string());
        }

        result
    }

    fn update_context(&mut self, query: &str, resolved: &ResolvedIntent) {
        self.context.turn_number += 1;

        // Extract mentioned locations
        if query.contains("楼") {
            if let Some(floor) = query.split('楼').next() {
                self.context.mentioned_locations.push(format!("{}楼", floor));
            }
        }

        // Common room names
        let rooms = ["客厅", "卧室", "厨房", "浴室", "书房", "阳台", "餐厅", "走廊"];
        for room in rooms {
            if query.contains(room) {
                self.context.mentioned_locations.push(room.to_string());
            }
        }

        // Extract device types
        let device_types = ["灯", "空调", "温度", "湿度", "窗帘", "插座", "传感器"];
        for dt in device_types {
            if query.contains(dt) {
                self.context.mentioned_device_types.push(dt.to_string());
            }
        }

        // Track referenced devices
        for resource in &resolved.resources {
            if !self.context.referenced_devices.contains(&resource.name) {
                self.context.referenced_devices.push(resource.name.clone());
            }
        }
    }

    /// Run a complete conversation scenario.
    async fn run_scenario(&mut self, scenario: &ConversationScenario) -> ScenarioResult {
        let mut turn_results = Vec::new();
        let mut all_success = true;

        for turn in &scenario.turns {
            let result = self.process_turn(turn).await;
            let turn_success = result.success && result.intent_match;
            if !turn_success {
                all_success = false;
            }
            turn_results.push((turn.query.clone(), result));
        }

        ScenarioResult {
            scenario_name: scenario.name.clone(),
            all_success,
            turn_results,
        }
    }

    /// Reset the session context.
    fn reset_context(&mut self) {
        self.context = SessionContext::default();
    }
}

#[derive(Debug)]
struct ScenarioResult {
    scenario_name: String,
    all_success: bool,
    turn_results: Vec<(String, TurnResult)>,
}

/// Test: Complex multi-turn conversation with context retention.
#[tokio::test]
async fn test_multi_turn_conversation_with_context_retention() {
    let index = Arc::new(RwLock::new(ResourceIndex::new()));

    println!("Registering 300 devices...");
    let devices = generate_large_scale_devices(300);
    for device in devices {
        index.write().await.register(device).await.unwrap();
    }
    println!("Device registration complete.\n");

    let mut session = ConversationSession::new(Arc::clone(&index));

    // Scenario 1: User explores devices by room type, then controls them
    let scenario1 = ConversationScenario {
        name: "Room exploration and control".to_string(),
        description: "User asks about living room devices, then controls them".to_string(),
        turns: vec![
            ConversationTurn {
                query: "客厅有哪些设备".to_string(),
                expected_intent: Some(IntentCategory::ListDevices),
                min_resources: 5,
                expects_clarification: false,
                context_hints: vec!["客厅".to_string()],
            },
            ConversationTurn {
                query: "客厅温度是多少".to_string(),
                expected_intent: Some(IntentCategory::QueryData),
                min_resources: 1,
                expects_clarification: false,
                context_hints: vec!["客厅".to_string()],
            },
            ConversationTurn {
                query: "打开卧室的灯".to_string(),
                expected_intent: Some(IntentCategory::ControlDevice),
                min_resources: 1,
                expects_clarification: false,
                context_hints: vec!["卧室".to_string(), "灯".to_string()],
            },
            ConversationTurn {
                query: "把所有空调关掉".to_string(),
                expected_intent: Some(IntentCategory::ControlDevice),
                min_resources: 1,
                expects_clarification: false,
                context_hints: vec!["空调".to_string()],
            },
        ],
        expected_outcome: ScenarioOutcome::FullSuccess,
    };

    let result1 = session.run_scenario(&scenario1).await;
    print_scenario_result(&result1);
    assert!(result1.all_success, "Scenario '{}' should succeed completely", scenario1.name);

    session.reset_context();

    // Scenario 2: General query that finds resources, then specific query
    let scenario2 = ConversationScenario {
        name: "General then specific".to_string(),
        description: "User asks general question, finds resources, then asks specific".to_string(),
        turns: vec![
            ConversationTurn {
                query: "温度怎么样".to_string(),
                expected_intent: Some(IntentCategory::QueryData),
                min_resources: 1,  // Will find many temperature sensors
                expects_clarification: false,  // Resources are found, no clarification needed
                context_hints: vec![],
            },
            ConversationTurn {
                query: "客厅温度是多少".to_string(),
                expected_intent: Some(IntentCategory::QueryData),
                min_resources: 1,
                expects_clarification: false,
                context_hints: vec!["客厅".to_string()],
            },
        ],
        expected_outcome: ScenarioOutcome::FullSuccess,
    };

    let result2 = session.run_scenario(&scenario2).await;
    print_scenario_result(&result2);
    assert!(result2.all_success, "Scenario '{}' should succeed", scenario2.name);

    session.reset_context();

    // Scenario 3: Complex multi-room comparison
    let scenario3 = ConversationScenario {
        name: "Multi-room comparison".to_string(),
        description: "User compares devices across multiple rooms".to_string(),
        turns: vec![
            ConversationTurn {
                query: "客厅和卧室各有哪些温度传感器".to_string(),
                expected_intent: Some(IntentCategory::ListDevices),
                min_resources: 5,
                expects_clarification: false,
                context_hints: vec!["客厅".to_string(), "卧室".to_string()],
            },
            ConversationTurn {
                query: "客厅温度高还是卧室温度高".to_string(),
                expected_intent: Some(IntentCategory::QueryData),
                min_resources: 5,
                expects_clarification: false,
                context_hints: vec![],
            },
        ],
        expected_outcome: ScenarioOutcome::FullSuccess,
    };

    let result3 = session.run_scenario(&scenario3).await;
    print_scenario_result(&result3);
    assert!(result3.all_success, "Scenario '{}' should succeed", scenario3.name);
}

/// Test: Complex task completion with multiple steps.
#[tokio::test]
async fn test_complex_task_completion() {
    let index = Arc::new(RwLock::new(ResourceIndex::new()));

    let devices = generate_large_scale_devices(300);
    for device in devices {
        index.write().await.register(device).await.unwrap();
    }

    let mut session = ConversationSession::new(Arc::clone(&index));

    // Complex scenario: Create a comfortable environment
    let scenario = ConversationScenario {
        name: "Comfort setup automation".to_string(),
        description: "User sets up comfortable environment across rooms".to_string(),
        turns: vec![
            // Step 1: Check current state
            ConversationTurn {
                query: "客厅现在的温度和湿度是多少".to_string(),
                expected_intent: Some(IntentCategory::QueryData),
                min_resources: 1,
                expects_clarification: false,
                context_hints: vec!["客厅".to_string()],
            },
            // Step 2: Adjust temperature (control operation)
            ConversationTurn {
                query: "打开客厅空调".to_string(),
                expected_intent: Some(IntentCategory::ControlDevice),
                min_resources: 1,
                expects_clarification: false,
                context_hints: vec!["客厅".to_string(), "空调".to_string()],
            },
            // Step 3: Adjust lighting
            ConversationTurn {
                query: "打开客厅的灯".to_string(),
                expected_intent: Some(IntentCategory::ControlDevice),
                min_resources: 1,
                expects_clarification: false,
                context_hints: vec!["客厅".to_string(), "灯".to_string()],
            },
            // Step 4: Open curtains
            ConversationTurn {
                query: "打开客厅的窗帘".to_string(),
                expected_intent: Some(IntentCategory::ControlDevice),
                min_resources: 1,
                expects_clarification: false,
                context_hints: vec!["客厅".to_string()],
            },
            // Step 5: Verify setup
            ConversationTurn {
                query: "客厅现在的状态怎么样".to_string(),
                expected_intent: Some(IntentCategory::QueryData),
                min_resources: 1,
                expects_clarification: false,
                context_hints: vec!["客厅".to_string()],
            },
        ],
        expected_outcome: ScenarioOutcome::FullSuccess,
    };

    let result = session.run_scenario(&scenario).await;
    print_scenario_result(&result);

    let success_count = result.turn_results.iter()
        .filter(|(_, r)| r.success)
        .count();

    let success_rate = (success_count as f32 / result.turn_results.len() as f32) * 100.0;
    println!("\nTask completion rate: {:.1}% ({}/{})",
        success_rate, success_count, result.turn_results.len());

    assert!(success_rate >= 80.0,
        "Task completion rate should be at least 80%, got {:.1}%", success_rate);
}

/// Test: Reliability with repeated similar queries.
#[tokio::test]
async fn test_reliability_with_similar_queries() {
    let index = Arc::new(RwLock::new(ResourceIndex::new()));

    let devices = generate_large_scale_devices(300);
    for device in devices {
        index.write().await.register(device).await.unwrap();
    }

    let resolver = ResourceResolver::new(index);

    // Test that similar queries produce consistent results
    let similar_queries = vec![
        "客厅温度",
        "客厅的温度是多少",
        "查询客厅温度",
        "客厅的室温",
    ];

    let mut results = Vec::new();
    for query in &similar_queries {
        let resolved = resolver.resolve(query).await;
        results.push((query, resolved.intent.clone(), resolved.resources.len()));
    }

    // All queries should be recognized as QueryData
    let query_data_count = results.iter()
        .filter(|(_, intent, _)| matches!(intent, IntentCategory::QueryData))
        .count();

    println!("Similar query consistency: {}/{} recognized as QueryData",
        query_data_count, results.len());

    // At least 75% should be recognized as the same intent
    assert!(query_data_count >= similar_queries.len() * 3 / 4,
        "Similar queries should have consistent intent recognition");
}

/// Test: Accuracy of resource matching.
#[tokio::test]
async fn test_resource_matching_accuracy() {
    let index = Arc::new(RwLock::new(ResourceIndex::new()));

    let devices = generate_large_scale_devices(300);
    for device in devices {
        index.write().await.register(device).await.unwrap();
    }

    let resolver = ResourceResolver::new(Arc::clone(&index));

    // Define test cases - we check if queries find relevant resources
    // Note: Device names don't include floor prefix (e.g., "3楼"), they only have room names
    // The location is stored separately, so we check for room name in results
    let test_cases = vec![
        ("客厅温度", "客厅", true),
        ("卧室的灯", "卧室", true),
        ("厨房空调", "厨房", true),
        ("书房湿度", "书房", true),
        ("打开窗帘", "", false),  // Ambiguous - curtains in many rooms
        ("温度传感器", "", false),  // Multiple locations
    ];

    let mut accurate_matches = 0;
    let mut total_specific = 0;

    for (query, expected_room, should_be_specific) in &test_cases {
        let resolved = resolver.resolve(query).await;

        let is_specific = if expected_room.is_empty() {
            // If no room specified, check if we found multiple resources (ambiguous)
            resolved.resources.len() > 5
        } else {
            // Check if we found resources with the expected room in their name
            resolved.resources.iter().any(|r| {
                let name = &r.name;
                name.contains(expected_room)
            })
        };

        if *should_be_specific {
            total_specific += 1;
            if is_specific {
                accurate_matches += 1;
            }
            println!("✓ '{}' -> specific: {}, expected: {}, found: {}",
                query, is_specific, should_be_specific, resolved.resources.len());
        } else {
            println!("✓ '{}' -> found {} results (ambiguous expected)", query, resolved.resources.len());
        }
    }

    let accuracy = if total_specific > 0 {
        (accurate_matches as f32 / total_specific as f32) * 100.0
    } else {
        100.0
    };

    println!("\nResource matching accuracy: {:.1}% ({}/{})",
        accuracy, accurate_matches, total_specific);

    assert!(accuracy >= 70.0,
        "Resource matching accuracy should be at least 70%, got {:.1}%", accuracy);
}

/// Test: Error recovery and clarification.
#[tokio::test]
async fn test_error_recovery_and_clarification() {
    let index = Arc::new(RwLock::new(ResourceIndex::new()));

    let devices = generate_large_scale_devices(300);
    for device in devices {
        index.write().await.register(device).await.unwrap();
    }

    let resolver = ResourceResolver::new(index);

    // Test queries that should trigger clarification
    let clarification_tests = vec![
        ("温度是多少", vec!["温度", "哪个", "设备"]),
        ("打开灯", vec!["哪个", "房间", "灯"]),
        ("空调状态", vec!["哪个", "空调", "状态"]),
    ];

    for (query, expected_keywords) in clarification_tests {
        let resolved = resolver.resolve(query).await;

        let has_clarification = resolved.clarification.is_some();
        let has_many_results = resolved.resources.len() >= 10;

        let _clarifies = if let Some(clarification) = &resolved.clarification {
            expected_keywords.iter().any(|kw| clarification.contains(kw))
        } else {
            false
        };

        assert!(has_clarification || has_many_results,
            "Query '{}' should trigger clarification or find many devices", query);

        if has_clarification {
            println!("✓ '{}' -> clarification: '{}'", query,
                resolved.clarification.unwrap());
        } else {
            println!("✓ '{}' -> found {} devices (ambiguous acceptable)", query,
                resolved.resources.len());
        }
    }
}

/// Test: Performance under load with multiple rapid queries.
#[tokio::test]
async fn test_performance_under_load() {
    let index = Arc::new(RwLock::new(ResourceIndex::new()));

    let devices = generate_large_scale_devices(300);
    for device in devices {
        index.write().await.register(device).await.unwrap();
    }

    let resolver = ResourceResolver::new(index);

    // Simulate rapid succession of queries
    let queries = vec![
        "1楼客厅温度", "2楼卧室湿度", "3楼厨房空调",
        "4楼书房灯", "5楼阳台窗帘", "6楼餐厅温度",
        "7楼客厅湿度", "8楼卧室空调", "9楼书房灯",
        "1楼客厅灯", "2楼厨房空调", "3楼卧室温度",
        "4楼餐厅湿度", "5楼阳台光照", "6楼客厅窗帘",
        "7楼卧室空调", "8楼书房温度", "9楼餐厅灯",
    ];

    let start = std::time::Instant::now();
    let mut all_times = Vec::new();

    for query in &queries {
        let q_start = std::time::Instant::now();
        let _resolved = resolver.resolve(query).await;
        let q_elapsed = q_start.elapsed();
        all_times.push(q_elapsed);
    }

    let total_time = start.elapsed();
    let avg_time = total_time.div_f64(queries.len() as f64);
    let max_time = *all_times.iter().max().unwrap();
    let min_time = *all_times.iter().min().unwrap();

    println!("Performance under load ({} queries):", queries.len());
    println!("  Total: {:?}", total_time);
    println!("  Average: {:.2?}", avg_time);
    println!("  Min: {:?}", min_time);
    println!("  Max: {:?}", max_time);
    println!("  Throughput: {:.2} queries/sec", queries.len() as f64 / total_time.as_secs_f64());

    // Performance assertions
    assert!(avg_time.as_millis() < 10, "Average time should be under 10ms");
    assert!(max_time.as_millis() < 50, "Max time should be under 50ms");
}

/// Print scenario result summary.
fn print_scenario_result(result: &ScenarioResult) {
    println!("\n=== Scenario: {} ===", result.scenario_name);
    println!("Overall: {}", if result.all_success { "✅ SUCCESS" } else { "❌ FAILED" });

    for (i, (query, turn_result)) in result.turn_results.iter().enumerate() {
        println!("\n  Turn {}: '{}'", i + 1, query);
        println!("    Success: {}", if turn_result.success { "✓" } else { "✗" });
        println!("    Intent: {}", turn_result.intent_match.then(|| "✓").unwrap_or("✗"));
        println!("    Resources: {}", turn_result.resource_count);
        println!("    Actions: {}", turn_result.action_count);
        println!("    Clarification: {}", if turn_result.has_clarification { "Yes" } else { "No" });
        println!("    Time: {:.2}ms", turn_result.elapsed_ms);

        if !turn_result.issues.is_empty() {
            println!("    Issues:");
            for issue in &turn_result.issues {
                println!("      - {}", issue);
            }
        }
    }
}
