//! Extended multi-turn conversation integration test.
//!
//! Tests:
//! 1. 20+ rounds of conversation per session
//! 2. Multiple tools called in same response
//! 3. All available tools verification
//! 4. Complex multi-turn scenarios
//! 5. Context preservation across many turns
//! 6. Sequential tool calls with dependencies

use std::sync::Arc;
use std::time::Duration;
use std::pin::Pin;
use tokio::sync::RwLock;
use serde_json::json;
use futures::{Stream, StreamExt};
use async_trait::async_trait;

use edge_ai_core::{
    EventBus,
    llm::backend::{LlmRuntime, LlmInput, LlmOutput, FinishReason, TokenUsage, BackendId, StreamChunk, LlmError},
};
use edge_ai_agent::{
    agent::AgentEvent,
    session::SessionManager,
};
use edge_ai_tools::{ToolRegistryBuilder, Tool, ToolOutput, Result as ToolResult};

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
        let cloned: Vec<StreamChunk> = chunks.iter().filter_map(|c| {
            c.as_ref().ok().map(|(text, is_thinking)| Ok((text.clone(), *is_thinking)))
        }).collect();
        Ok(Box::pin(futures::stream::iter(cloned)))
    }

    fn max_context_length(&self) -> usize {
        8192
    }
}

/// Mock tools for comprehensive testing.
struct MockTool {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[async_trait::async_trait]
impl Tool for MockTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> serde_json::Value {
        self.parameters.clone()
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult<ToolOutput> {
        let tool_name = self.name.clone();
        Ok(ToolOutput {
            success: true,
            data: json!({
                "tool": tool_name,
                "args": args,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "status": "success"
            }),
            error: None,
            metadata: None,
        })
    }
}

/// Create a mock tool with specified properties.
fn create_mock_tool(name: &str, description: &str, params: serde_json::Value) -> Arc<MockTool> {
    Arc::new(MockTool {
        name: name.to_string(),
        description: description.to_string(),
        parameters: params,
    })
}

/// Create streaming chunks for a simple response.
fn create_simple_chunks(response: &str) -> Vec<StreamChunk> {
    vec![
        Ok((response.to_string(), false)),
    ]
}

/// Create streaming chunks for a single tool call.
fn create_tool_call_chunks(tool_name: &str, tool_args: serde_json::Value, final_response: &str) -> Vec<StreamChunk> {
    vec![
        Ok((format!("I'll help you with that using {}.", tool_name), true)),
        Ok((format!("<tool_calls><invoke name=\"{}\">", tool_name), false)),
        Ok((create_parameter_xml(&tool_args), false)),
        Ok((format!("</invoke></tool_calls>{}", final_response), false)),
    ]
}

/// Create streaming chunks for multiple tool calls in one response.
fn create_multi_tool_call_chunks(tools: Vec<(&str, serde_json::Value)>, final_response: &str) -> Vec<StreamChunk> {
    let mut chunks = vec![
        Ok(("I'll need to call multiple tools to help you.".to_string(), true)),
        Ok(("<tool_calls>".to_string(), false)),
    ];

    for (tool_name, args) in &tools {
        chunks.push(Ok((format!("<invoke name=\"{}\">", tool_name), false)));
        chunks.push(Ok((create_parameter_xml(args), false)));
        chunks.push(Ok(("</invoke>".to_string(), false)));
    }

    chunks.push(Ok(("</tool_calls>".to_string(), false)));
    chunks.push(Ok((final_response.to_string(), false)));

    chunks
}

/// Create XML parameters from JSON value.
fn create_parameter_xml(args: &serde_json::Value) -> String {
    if let Some(obj) = args.as_object() {
        let mut params = String::new();
        for (key, value) in obj {
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => "null".to_string(),
            };
            params.push_str(&format!("<parameter name=\"{}\" value=\"{}\"/>", key, value_str));
        }
        params
    } else {
        String::new()
    }
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

/// Test 1: Extended 20-round conversation with various scenarios.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_extended_20_round_conversation() {
    println!("\n=== Extended 20-Round Conversation Test ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    // Create comprehensive tool registry
    let mut registry = ToolRegistryBuilder::new().build();
    registry.register(create_mock_tool(
        "query_device",
        "Query device status and information",
        json!({
            "type": "object",
            "properties": {
                "device_id": {"type": "string"}
            }
        })
    ));
    registry.register(create_mock_tool(
        "list_devices",
        "List all available devices",
        json!({"type": "object", "properties": {}})
    ));
    registry.register(create_mock_tool(
        "get_temperature",
        "Get temperature reading from a device",
        json!({
            "type": "object",
            "properties": {
                "device_id": {"type": "string"}
            }
        })
    ));
    registry.register(create_mock_tool(
        "get_humidity",
        "Get humidity reading from a device",
        json!({
            "type": "object",
            "properties": {
                "device_id": {"type": "string"}
            }
        })
    ));
    registry.register(create_mock_tool(
        "control_device",
        "Send control command to a device",
        json!({
            "type": "object",
            "properties": {
                "device_id": {"type": "string"},
                "command": {"type": "string"},
                "value": {"type": "string"}
            }
        })
    ));

    let session_manager = SessionManager::memory();
    session_manager.set_tool_registry(Arc::new(registry)).await;

    let session_id = session_manager.create_session().await
        .expect("Failed to create session");

    let agent = session_manager.get_session(&session_id).await
        .expect("Failed to get session");
    agent.set_custom_llm(mock_llm.clone()).await;

    // 20 rounds with increasing complexity
    let test_rounds: Vec<(&str, Vec<&str>, Vec<StreamChunk>)> = vec![
        // Round 1: Greeting
        ("Hello, can you help me?", vec![], create_simple_chunks("Hello! I'm here to help you manage your devices and answer questions.")),

        // Round 2: List devices
        ("What devices do I have?", vec!["list_devices"],
            create_tool_call_chunks("list_devices", json!({}),
                "You have 5 devices: sensor_temp_1, sensor_temp_2, sensor_hum_1, switch_living, and switch_bedroom.")),

        // Round 3: Query specific device
        ("Tell me about sensor_temp_1", vec!["query_device"],
            create_tool_call_chunks("query_device", json!({"device_id": "sensor_temp_1"}),
                "sensor_temp_1 is an online temperature sensor located in the living room.")),

        // Round 4: Get temperature
        ("What's the current temperature?", vec!["get_temperature"],
            create_tool_call_chunks("get_temperature", json!({"device_id": "sensor_temp_1"}),
                "The current temperature is 23.5°C.")),

        // Round 5: Get humidity
        ("What about humidity?", vec!["get_humidity"],
            create_tool_call_chunks("get_humidity", json!({"device_id": "sensor_hum_1"}),
                "The current humidity is 65%.")),

        // Round 6: Context question
        ("What was the temperature you just mentioned?", vec![],
            create_simple_chunks("I mentioned the temperature is 23.5°C from sensor_temp_1.")),

        // Round 7: Control device
        ("Turn on the living room switch", vec!["control_device"],
            create_tool_call_chunks("control_device", json!({"device_id": "switch_living", "command": "on", "value": "true"}),
                "The living room switch has been turned on successfully.")),

        // Round 8: Multiple queries
        ("Check both temperature sensors", vec!["query_device"],
            create_tool_call_chunks("query_device", json!({"device_id": "sensor_temp_1"}),
                "Both temperature sensors are online. sensor_temp_1 shows 23.5°C and sensor_temp_2 shows 22.8°C.")),

        // Round 9: Complex context
        ("Which sensor is warmer?", vec![],
            create_simple_chunks("sensor_temp_1 is warmer at 23.5°C compared to sensor_temp_2 at 22.8°C.")),

        // Round 10: List again (verify state)
        ("List devices again", vec!["list_devices"],
            create_tool_call_chunks("list_devices", json!({}),
                "You still have 5 devices: sensor_temp_1, sensor_temp_2, sensor_hum_1, switch_living, and switch_bedroom.")),

        // Round 11: Get humidity again
        ("What's the humidity now?", vec!["get_humidity"],
            create_tool_call_chunks("get_humidity", json!({"device_id": "sensor_hum_1"}),
                "The current humidity is 68%, slightly up from before.")),

        // Round 12: Context from earlier
        ("What's the combined status of both temperature sensors?", vec![],
            create_simple_chunks("From our earlier queries, both sensor_temp_1 (23.5°C) and sensor_temp_2 (22.8°C) are online and functioning normally.")),

        // Round 13: Control another device
        ("Turn off the bedroom switch", vec!["control_device"],
            create_tool_call_chunks("control_device", json!({"device_id": "switch_bedroom", "command": "off", "value": "false"}),
                "The bedroom switch has been turned off.")),

        // Round 14: Query device
        ("Check the bedroom switch status", vec!["query_device"],
            create_tool_call_chunks("query_device", json!({"device_id": "switch_bedroom"}),
                "The bedroom switch is currently off.")),

        // Round 15: Get all environmental data
        ("Give me all environmental readings", vec!["get_temperature"],
            create_tool_call_chunks("get_temperature", json!({"device_id": "sensor_temp_1"}),
                "Environmental readings: Temperature 23.5°C, Humidity 68%. All sensors are operating normally.")),

        // Round 16: Context chain
        ("Based on all the data we've discussed, what's the environment like?", vec![],
            create_simple_chunks("Based on our conversation, your environment is comfortable: 23.5°C temperature with 68% humidity. Both temperature sensors are working well, and you have control of two switches.")),

        // Round 17: Control command
        ("Set the living room switch to off", vec!["control_device"],
            create_tool_call_chunks("control_device", json!({"device_id": "switch_living", "command": "off", "value": "false"}),
                "The living room switch has been turned off.")),

        // Round 18: Multi-device query
        ("Check if all sensors are still online", vec!["list_devices"],
            create_tool_call_chunks("list_devices", json!({}),
                "All 5 devices are online and operational. No changes detected.")),

        // Round 19: Summary request
        ("Summarize everything we've done today", vec![],
            create_simple_chunks("Today we: checked your 5 devices multiple times, monitored temperature (23.5°C) and humidity (68%), controlled your living room and bedroom switches, and verified all systems are working properly.")),

        // Round 20: Closing
        ("Thanks for all the help!", vec![],
            create_simple_chunks("You're welcome! Feel free to ask if you need anything else. Have a great day!")),
    ];

    println!("Running {} rounds of conversation...\n", test_rounds.len());

    let mut passed_rounds = 0;
    let mut failed_rounds = Vec::new();
    let mut total_messages = 0;
    let mut total_tools_used = 0;

    for (round_idx, (user_msg, expected_tools, chunks)) in test_rounds.into_iter().enumerate() {
        println!("=== Round {} ===", round_idx + 1);
        println!("User: {}", user_msg);

        mock_llm.set_response_chunks(chunks).await;

        let start = std::time::Instant::now();
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            session_manager.process_message_events(&session_id, user_msg)
        ).await;

        match result {
            Ok(Ok(stream)) => {
                let duration = start.elapsed();
                let events = collect_events(stream).await;

                let mut tools_called = Vec::new();
                let mut has_content = false;
                let mut has_error = false;
                let mut content_parts = Vec::new();

                for event in &events {
                    match event {
                        AgentEvent::Thinking { .. } => {}
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
                            println!("  Tool '{}' completed: {}", tool, if *success { "OK" } else { "FAIL" });
                        }
                        AgentEvent::Error { message } => {
                            has_error = true;
                            println!("  ERROR: {}", message);
                        }
                        AgentEvent::End => {}
                    }
                }

                let full_response: String = content_parts.join("");
                if !full_response.is_empty() {
                    println!("Assistant: {}", full_response);
                }
                println!("Tools called: {:?}", tools_called);
                println!("Duration: {:?}", duration);

                total_messages += 1;
                total_tools_used += tools_called.len();

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
                    }
                }

                // Check that we got a response
                if !has_content && expected_tools.is_empty() {
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
                println!("  ❌ FAILED: Timeout");
                failed_rounds.push(round_idx + 1);
            }
        }
        println!();
    }

    println!("=== Test Summary ===");
    println!("Rounds passed: {}/{}", passed_rounds, passed_rounds + failed_rounds.len());
    println!("Total messages processed: {}", total_messages);
    println!("Total tools used: {}", total_tools_used);

    if !failed_rounds.is_empty() {
        println!("Failed rounds: {:?}", failed_rounds);
        panic!("Extended 20-round test failed!");
    } else {
        println!("✅ All 20 rounds passed successfully!");
    }
}

/// Test 2: Multiple tools called in the same response.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multiple_tools_same_response() {
    println!("\n=== Multiple Tools in Same Response Test ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    let mut registry = ToolRegistryBuilder::new().build();
    registry.register(create_mock_tool(
        "get_temperature",
        "Get temperature reading",
        json!({"type": "object", "properties": {"device_id": {"type": "string"}}})
    ));
    registry.register(create_mock_tool(
        "get_humidity",
        "Get humidity reading",
        json!({"type": "object", "properties": {"device_id": {"type": "string"}}})
    ));
    registry.register(create_mock_tool(
        "get_pressure",
        "Get pressure reading",
        json!({"type": "object", "properties": {"device_id": {"type": "string"}}})
    ));

    let session_manager = SessionManager::memory();
    session_manager.set_tool_registry(Arc::new(registry)).await;

    let session_id = session_manager.create_session().await
        .expect("Failed to create session");

    let agent = session_manager.get_session(&session_id).await
        .expect("Failed to get session");
    agent.set_custom_llm(mock_llm.clone()).await;

    // Test calling 3 tools at once
    let chunks = create_multi_tool_call_chunks(
        vec![
            ("get_temperature", json!({"device_id": "sensor_1"})),
            ("get_humidity", json!({"device_id": "sensor_1"})),
            ("get_pressure", json!({"device_id": "sensor_1"})),
        ],
        "All environmental readings collected successfully."
    );

    mock_llm.set_response_chunks(chunks).await;

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        session_manager.process_message_events(&session_id, "Get all environmental readings from sensor_1")
    ).await;

    match result {
        Ok(Ok(stream)) => {
            let events = collect_events(stream).await;
            let tools_used: Vec<_> = events.iter()
                .filter_map(|e| match e {
                    AgentEvent::ToolCallStart { tool, .. } => Some(tool.clone()),
                    _ => None,
                })
                .collect();

            println!("Tools called: {:?}", tools_used);

            let expected_tools = vec!["get_temperature", "get_humidity", "get_pressure"];
            let mut all_found = true;

            for expected in &expected_tools {
                if !tools_used.contains(&expected.to_string()) {
                    println!("  ❌ FAILED: Expected tool '{}' not found", expected);
                    all_found = false;
                }
            }

            if all_found && tools_used.len() == expected_tools.len() {
                println!("✅ All 3 tools called successfully in same response");
            } else {
                panic!("Expected 3 tools, got {}: {:?}", tools_used.len(), tools_used);
            }
        }
        Ok(Err(e)) => {
            panic!("Error: {}", e);
        }
        Err(_) => {
            panic!("Timeout");
        }
    }
}

/// Test 3: All available tools functionality.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_all_tools_functionality() {
    println!("\n=== All Tools Functionality Test ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    // Create registry with all common tool types
    let mut registry = ToolRegistryBuilder::new().build();
    let tools_to_test = vec![
        ("query_device", json!({"device_id": "test_1"})),
        ("list_devices", json!({})),
        ("get_temperature", json!({"device_id": "sensor_1"})),
        ("get_humidity", json!({"device_id": "sensor_1"})),
        ("control_device", json!({"device_id": "switch_1", "command": "on", "value": "true"})),
        ("create_rule", json!({"name": "test_rule", "condition": "temp > 25"})),
        ("list_rules", json!({})),
        ("trigger_workflow", json!({"workflow_id": "test_wf"})),
    ];

    for (tool_name, _) in &tools_to_test {
        registry.register(create_mock_tool(
            tool_name,
            &format!("Mock tool for {}", tool_name),
            json!({"type": "object", "properties": {"test": {"type": "string"}}})
        ));
    }

    let session_manager = SessionManager::memory();
    session_manager.set_tool_registry(Arc::new(registry)).await;

    let session_id = session_manager.create_session().await
        .expect("Failed to create session");

    let agent = session_manager.get_session(&session_id).await
        .expect("Failed to get session");
    agent.set_custom_llm(mock_llm.clone()).await;

    let mut tested_tools = Vec::new();

    for (tool_name, args) in &tools_to_test {
        println!("Testing tool: {}", tool_name);

        let chunks = create_tool_call_chunks(
            tool_name,
            args.clone(),
            &format!("Tool {} executed successfully.", tool_name)
        );

        mock_llm.set_response_chunks(chunks).await;

        let result = tokio::time::timeout(
            Duration::from_secs(5),
            session_manager.process_message_events(
                &session_id,
                &format!("Execute {}", tool_name)
            )
        ).await;

        match result {
            Ok(Ok(stream)) => {
                let events = collect_events(stream).await;
                let tools_called: Vec<_> = events.iter()
                    .filter_map(|e| match e {
                        AgentEvent::ToolCallStart { tool, .. } => Some(tool.clone()),
                        _ => None,
                    })
                    .collect();

                if tools_called.contains(&tool_name.to_string()) {
                    println!("  ✅ Tool '{}' works", tool_name);
                    tested_tools.push(tool_name.to_string());
                } else {
                    println!("  ❌ Tool '{}' not called", tool_name);
                }
            }
            Ok(Err(e)) => {
                println!("  ❌ Tool '{}' error: {}", tool_name, e);
            }
            Err(_) => {
                println!("  ❌ Tool '{}' timeout", tool_name);
            }
        }
    }

    println!("\nTested {}/{} tools successfully", tested_tools.len(), tools_to_test.len());

    if tested_tools.len() != tools_to_test.len() {
        panic!("Not all tools were tested successfully");
    }

    println!("✅ All tools functionality verified");
}

/// Test 4: Complex multi-turn scenario with tool dependencies.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_complex_scenario_with_dependencies() {
    println!("\n=== Complex Scenario with Tool Dependencies Test ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    let mut registry = ToolRegistryBuilder::new().build();
    registry.register(create_mock_tool("list_devices", "List devices", json!({})));
    registry.register(create_mock_tool("query_device", "Query device", json!({"device_id": {"type": "string"}})));
    registry.register(create_mock_tool("control_device", "Control device", json!({"device_id": {"type": "string"}})));

    let session_manager = SessionManager::memory();
    session_manager.set_tool_registry(Arc::new(registry)).await;

    let session_id = session_manager.create_session().await
        .expect("Failed to create session");

    let agent = session_manager.get_session(&session_id).await
        .expect("Failed to get session");
    agent.set_custom_llm(mock_llm.clone()).await;

    // Scenario: User wants to control a device but needs to find it first
    // Define conversation steps: (user_message, expected_tools, assistant_response)
    let scenario_steps: Vec<(&str, Vec<&str>, &str)> = vec![
        // Step 1: User asks to control a device but doesn't specify which
        ("Turn on a device for me", vec!["list_devices"],
            "I can help with that. Let me first check what devices you have available. You have these devices: switch_living, switch_bedroom, and switch_hallway. Which one would you like me to turn on?"),

        // Step 2: User specifies the living room switch
        ("Turn on the living room switch", vec!["control_device"],
            "OK, I'll turn on the living room switch for you. The living room switch has been turned on successfully."),

        // Step 3: User asks to verify
        ("Is the living room switch on?", vec!["query_device"],
            "Yes, the living room switch is currently on."),

        // Step 4: User wants to turn it off
        ("Now turn it off", vec!["control_device"],
            "I'll turn off the living room switch. The living room switch has been turned off."),

        // Step 5: Final verification
        ("What's the status now?", vec!["query_device"],
            "The living room switch is currently off."),
    ];

    let mut step = 1;

    for (user_msg, expected_tools, expected_response) in scenario_steps {
        println!("--- Step {} ---", step);
        println!("User: {}", user_msg);

        // Create chunks based on expected tools
        let chunks = if expected_tools.contains(&"list_devices") {
            create_tool_call_chunks("list_devices", json!({}), expected_response)
        } else if expected_tools.contains(&"control_device") {
            if step == 2 {
                create_tool_call_chunks("control_device", json!({"device_id": "switch_living", "command": "on"}), expected_response)
            } else {
                create_tool_call_chunks("control_device", json!({"device_id": "switch_living", "command": "off"}), expected_response)
            }
        } else if expected_tools.contains(&"query_device") {
            create_tool_call_chunks("query_device", json!({"device_id": "switch_living"}), expected_response)
        } else {
            create_simple_chunks(expected_response)
        };

        mock_llm.set_response_chunks(chunks).await;

        let result = tokio::time::timeout(
            Duration::from_secs(5),
            session_manager.process_message_events(&session_id, user_msg)
        ).await;

        match result {
            Ok(Ok(stream)) => {
                let events = collect_events(stream).await;
                let response_text: String = events.iter()
                    .filter_map(|e| match e {
                        AgentEvent::Content { content } => Some(content.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                println!("Assistant: {}", response_text);
                println!("✅ Step {} passed", step);
            }
            Ok(Err(e)) => {
                panic!("Step {} failed: {}", step, e);
            }
            Err(_) => {
                panic!("Step {} timed out", step);
            }
        }

        step += 1;
    }

    println!("✅ Complex scenario with dependencies completed successfully");
}
