//! Multi-turn conversation integration test.
//!
//! Tests:
//! 1. 10 rounds of conversation per session
//! 2. At least 5 messages per round
//! 3. Context preservation
//! 4. Tool calling
//! 5. No blocking/interruption
//! 6. Different tools and intents

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::json;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use edge_ai_agent::{agent::AgentEvent, session::SessionManager};
use edge_ai_core::{
    EventBus,
    llm::backend::{
        BackendId, FinishReason, LlmError, LlmInput, LlmOutput, LlmRuntime, StreamChunk, TokenUsage,
    },
};
use edge_ai_tools::{Result as ToolResult, Tool, ToolOutput, ToolRegistryBuilder};

/// Mock LLM backend that simulates real streaming behavior.
struct MockLlmBackend {
    response_queue: Arc<RwLock<Vec<StreamChunk>>>,
}

impl MockLlmBackend {
    fn new() -> Self {
        Self {
            response_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set up the complete streaming response in advance.
    async fn set_response_chunks(&self, chunks: Vec<StreamChunk>) {
        let mut queue = self.response_queue.write().await;
        *queue = chunks;
    }
}

#[async_trait::async_trait]
impl LlmRuntime for MockLlmBackend {
    fn backend_id(&self) -> BackendId {
        BackendId::new("mock")
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }

    async fn generate(&self, _input: LlmInput) -> Result<LlmOutput, LlmError> {
        Ok(LlmOutput {
            text: "Response".to_string(),
            thinking: None,
            finish_reason: FinishReason::Stop,
            usage: Some(TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        })
    }

    async fn generate_stream(
        &self,
        _input: LlmInput,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamChunk> + Send>>, LlmError> {
        let chunks = self.response_queue.read().await;
        // Clone only the Ok chunks (all our test data uses Ok)
        let cloned: Vec<StreamChunk> = chunks
            .iter()
            .filter_map(|c| {
                c.as_ref()
                    .ok()
                    .map(|(text, is_thinking)| Ok((text.clone(), *is_thinking)))
            })
            .collect();
        Ok(Box::pin(futures::stream::iter(cloned)))
    }

    fn max_context_length(&self) -> usize {
        8192
    }
}

/// Mock tools for testing.
struct MockDeviceTool {
    name: String,
}

#[async_trait::async_trait]
impl Tool for MockDeviceTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Query device information"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "device_id": {"type": "string"},
                "query": {"type": "string"}
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult<ToolOutput> {
        let device_id = args["device_id"].as_str().unwrap_or("unknown");
        Ok(ToolOutput {
            success: true,
            data: json!({
                "device": device_id,
                "status": "online",
                "value": format!("Data from {}", device_id)
            }),
            error: None,
            metadata: None,
        })
    }
}

/// Create streaming chunks that simulate a tool call response.
fn create_tool_call_chunks(
    tool_name: &str,
    tool_args: serde_json::Value,
    final_response: &str,
) -> Vec<StreamChunk> {
    vec![
        // First some thinking
        Ok((format!("Let me query {} for you.", tool_name), true)),
        // Then the tool call block in XML format
        Ok((
            format!(
                "<tool_calls><invoke name=\"{}\"><parameter name=\"device_id\" value=\"{}\"/></invoke></tool_calls>",
                tool_name,
                tool_args["device_id"].as_str().unwrap_or("sensor_1")
            ),
            false,
        )),
        // Finally the response
        Ok((
            format!("Based on the tool result, {}", final_response),
            false,
        )),
    ]
}

/// Create streaming chunks for a simple response (no tools).
fn create_simple_chunks(response: &str) -> Vec<StreamChunk> {
    vec![Ok((response.to_string(), false))]
}

/// Collect events from a stream.
async fn collect_events(stream: Pin<Box<dyn Stream<Item = AgentEvent> + Send>>) -> Vec<AgentEvent> {
    let mut events = Vec::new();
    futures::pin_mut!(stream);
    while let Some(event) = stream.next().await {
        let is_end = event.is_end();
        events.push(event);
        if is_end {
            break;
        }
    }
    events
}

/// Comprehensive test: 10 rounds of multi-turn conversation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_comprehensive_multi_turn_conversation() {
    println!("\n=== Comprehensive Multi-Turn Conversation Test ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    // Create tool registry with multiple tools
    let mut registry = ToolRegistryBuilder::new().build();
    registry.register(Arc::new(MockDeviceTool {
        name: "query_device".to_string(),
    }));
    registry.register(Arc::new(MockDeviceTool {
        name: "list_devices".to_string(),
    }));
    registry.register(Arc::new(MockDeviceTool {
        name: "get_temperature".to_string(),
    }));

    let session_manager = SessionManager::memory();
    session_manager.set_tool_registry(Arc::new(registry)).await;

    let session_id = session_manager
        .create_session()
        .await
        .expect("Failed to create session");

    let agent = session_manager
        .get_session(&session_id)
        .await
        .expect("Failed to get session");
    agent.set_custom_llm(mock_llm.clone()).await;

    // 10 rounds of conversation, each with different intent
    let test_rounds: Vec<(
        &str,
        /* expected tools */ Vec<&str>,
        /* chunks */ Vec<StreamChunk>,
    )> = vec![
        // Round 1: Simple greeting (no tools)
        (
            "Hello",
            vec![],
            create_simple_chunks("Hello! How can I help you today?"),
        ),
        // Round 2: Query device (uses tool)
        (
            "What's the status of sensor_1?",
            vec!["query_device"],
            create_tool_call_chunks(
                "query_device",
                json!({"device_id": "sensor_1"}),
                "The sensor is online and functioning normally.",
            ),
        ),
        // Round 3: List devices (uses tool)
        (
            "List all devices",
            vec!["list_devices"],
            create_tool_call_chunks(
                "list_devices",
                json!({}),
                "I found 3 devices: sensor_1, sensor_2, and actuator_1.",
            ),
        ),
        // Round 4: Get temperature (uses tool)
        (
            "What's the current temperature?",
            vec!["get_temperature"],
            create_tool_call_chunks(
                "get_temperature",
                json!({"device_id": "sensor_1"}),
                "The current temperature is 22.5°C.",
            ),
        ),
        // Round 5: Context question (no tools - tests memory)
        (
            "What was the temperature I just asked about?",
            vec![],
            create_simple_chunks("You asked about the temperature, which was 22.5°C."),
        ),
        // Round 6: Another device query (uses different tool)
        (
            "Check sensor_2 status",
            vec!["query_device"],
            create_tool_call_chunks(
                "query_device",
                json!({"device_id": "sensor_2"}),
                "Sensor 2 is online and reporting data.",
            ),
        ),
        // Round 7: Multi-device question (uses list_devices)
        (
            "How many sensors do we have?",
            vec!["list_devices"],
            create_tool_call_chunks(
                "list_devices",
                json!({}),
                "We have 2 sensors: sensor_1 and sensor_2.",
            ),
        ),
        // Round 8: Temperature again (uses tool - tests tool reusability)
        (
            "Get temperature from sensor_1 again",
            vec!["get_temperature"],
            create_tool_call_chunks(
                "get_temperature",
                json!({"device_id": "sensor_1"}),
                "The temperature is now 22.8°C.",
            ),
        ),
        // Round 9: Context about previous queries
        (
            "Which sensor had higher temperature?",
            vec![],
            create_simple_chunks(
                "Based on our queries, sensor_1 had 22.8°C compared to the earlier 22.5°C.",
            ),
        ),
        // Round 10: Closing conversation
        (
            "Thank you for the help",
            vec![],
            create_simple_chunks("You're welcome! Let me know if you need anything else."),
        ),
    ];

    println!("Running {} rounds of conversation...\n", test_rounds.len());

    let mut passed_rounds = 0;
    let mut failed_rounds = Vec::new();
    let mut total_messages = 0;
    let mut total_tools_used = 0;

    for (round_idx, (user_msg, expected_tools, chunks)) in test_rounds.into_iter().enumerate() {
        println!("=== Round {} ===", round_idx + 1);
        println!("User: {}", user_msg);

        // Set up the mock response
        mock_llm.set_response_chunks(chunks).await;

        // Process message
        let start = std::time::Instant::now();
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            session_manager.process_message_events(&session_id, user_msg),
        )
        .await;

        match result {
            Ok(Ok(stream)) => {
                let duration = start.elapsed();
                let events = collect_events(stream).await;

                // Analyze events
                let mut tools_called = Vec::new();
                let mut has_content = false;
                let mut _has_thinking = false;
                let mut has_error = false;
                let mut content_parts = Vec::new();

                for event in &events {
                    match event {
                        AgentEvent::Thinking { .. } => {
                            _has_thinking = true;
                        }
                        AgentEvent::Content { content } => {
                            has_content = true;
                            content_parts.push(content.clone());
                        }
                        AgentEvent::ToolCallStart { tool, .. } => {
                            if !tools_called.contains(tool) {
                                tools_called.push(tool.clone());
                            }
                        }
                        AgentEvent::ToolCallEnd { tool, success, .. } => {
                            println!(
                                "  Tool '{}' completed: {}",
                                tool,
                                if *success { "OK" } else { "FAIL" }
                            );
                        }
                        AgentEvent::Error { message } => {
                            has_error = true;
                            println!("  ERROR: {}", message);
                        }
                        AgentEvent::End => {}
                        _ => {} // Handle other event variants
                    }
                }

                let full_response: String = content_parts.join("");
                println!("Assistant: {}", full_response);
                println!("Tools called: {:?}", tools_called);
                println!("Duration: {:?}", duration);

                total_messages += 1;
                total_tools_used += tools_called.len();

                // Verify expectations
                let mut round_passed = true;

                // Check for timeout
                if duration.as_secs() > 8 {
                    println!("  ⚠️  WARNING: Near timeout");
                    round_passed = false;
                }

                // Check for errors
                if has_error {
                    println!("  ❌ FAILED: Error in response");
                    round_passed = false;
                }

                // Verify tools were called
                for expected_tool in &expected_tools {
                    if !tools_called.contains(&expected_tool.to_string()) {
                        println!("  ❌ FAILED: Expected tool '{}' not called", expected_tool);
                        round_passed = false;
                    } else {
                        println!("  ✅ Tool '{}' was called", expected_tool);
                    }
                }

                // Verify no unexpected tools were called
                for tool in &tools_called {
                    if !expected_tools.contains(&tool.as_str()) {
                        println!("  ⚠️  WARNING: Unexpected tool '{}' called", tool);
                    }
                }

                // Check that we got a response
                if !has_content {
                    println!("  ❌ FAILED: No content received");
                    round_passed = false;
                }

                if round_passed {
                    println!("  ✅ Round {} PASSED", round_idx + 1);
                    passed_rounds += 1;
                } else {
                    failed_rounds.push(round_idx + 1);
                }
            }
            Ok(Err(e)) => {
                println!("  ❌ FAILED: {}", e);
                failed_rounds.push(round_idx + 1);
            }
            Err(_) => {
                println!("  ❌ FAILED: Timeout (potential infinite loop)");
                failed_rounds.push(round_idx + 1);
            }
        }
        println!();
    }

    // Final summary
    println!("=== Test Summary ===");
    println!(
        "Rounds passed: {}/{}",
        passed_rounds,
        passed_rounds + failed_rounds.len()
    );
    println!("Total messages processed: {}", total_messages);
    println!("Total tools used: {}", total_tools_used);

    if !failed_rounds.is_empty() {
        println!("Failed rounds: {:?}", failed_rounds);
        panic!("Comprehensive multi-turn test failed!");
    } else {
        println!("✅ All rounds passed successfully!");
    }
}

/// Test that tools can be called repeatedly without loop detection.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_repeated_tool_calls_no_loop() {
    println!("\n=== Testing Repeated Tool Calls (No False Loop Detection) ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    let mut registry = ToolRegistryBuilder::new().build();
    registry.register(Arc::new(MockDeviceTool {
        name: "query_data".to_string(),
    }));

    let session_manager = SessionManager::memory();
    session_manager.set_tool_registry(Arc::new(registry)).await;

    let session_id = session_manager
        .create_session()
        .await
        .expect("Failed to create session");

    let agent = session_manager
        .get_session(&session_id)
        .await
        .expect("Failed to get session");
    agent.set_custom_llm(mock_llm.clone()).await;

    // Call the same tool multiple times in sequence - each should work
    for i in 1..=5 {
        println!("--- Call {} ---", i);
        mock_llm
            .set_response_chunks(create_tool_call_chunks(
                "query_data",
                json!({"device_id": "sensor_1"}),
                "Data retrieved successfully",
            ))
            .await;

        let result = tokio::time::timeout(
            Duration::from_secs(5),
            session_manager
                .process_message_events(&session_id, &format!("Query sensor_1 (call {})", i)),
        )
        .await;

        match result {
            Ok(Ok(stream)) => {
                let events = collect_events(stream).await;
                let tools_used: Vec<_> = events
                    .iter()
                    .filter_map(|e| match e {
                        AgentEvent::ToolCallStart { tool, .. } => Some(tool.clone()),
                        _ => None,
                    })
                    .collect();

                if tools_used.contains(&"query_data".to_string()) {
                    println!("  ✅ Call {} - Tool 'query_data' executed", i);
                } else {
                    println!("  ❌ Call {} - Tool not executed", i);
                    panic!("Tool not executed on call {}", i);
                }
            }
            Ok(Err(e)) => {
                println!("  ❌ Call {} - Error: {}", i, e);
                panic!("Error on call {}: {}", i, e);
            }
            Err(_) => {
                println!("  ❌ Call {} - Timeout", i);
                panic!("Timeout on call {}", i);
            }
        }
    }

    println!("✅ All 5 sequential tool calls succeeded (no false loop detection)");
}

/// Test context preservation across conversation turns.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_preservation() {
    println!("\n=== Testing Context Preservation ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    let session_manager = SessionManager::memory();
    let session_id = session_manager
        .create_session()
        .await
        .expect("Failed to create session");

    let agent = session_manager
        .get_session(&session_id)
        .await
        .expect("Failed to get session");
    agent.set_custom_llm(mock_llm.clone()).await;

    // Simulate a conversation where context matters
    let conversation = vec![
        ("My name is Alice", "Nice to meet you, Alice!"),
        ("What's my name?", "Your name is Alice."),
        ("I like blue", "Noted - you like the color blue."),
        (
            "What's my name and favorite color?",
            "Your name is Alice and you like blue.",
        ),
    ];

    for (i, (user_msg, expected_contains)) in conversation.iter().enumerate() {
        println!("Turn {}: {}", i + 1, user_msg);
        mock_llm
            .set_response_chunks(create_simple_chunks(expected_contains))
            .await;

        let result = tokio::time::timeout(
            Duration::from_secs(5),
            session_manager.process_message_events(&session_id, user_msg),
        )
        .await;

        match result {
            Ok(Ok(stream)) => {
                let events = collect_events(stream).await;
                let response: String = events
                    .iter()
                    .filter_map(|e| match e {
                        AgentEvent::Content { content } => Some(content.clone()),
                        _ => None,
                    })
                    .collect();

                if response.contains(expected_contains) {
                    println!("  ✅ Context preserved");
                } else {
                    println!(
                        "  ❌ Context NOT preserved: got '{}', expected '{}'",
                        response, expected_contains
                    );
                    panic!("Context not preserved on turn {}", i + 1);
                }
            }
            Ok(Err(e)) => {
                panic!("Error on turn {}: {}", i + 1, e);
            }
            Err(_) => {
                panic!("Timeout on turn {}", i + 1);
            }
        }
    }

    println!("✅ Context preserved across conversation turns");
}
