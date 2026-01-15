//! Strict test suite to verify:
//! 1. No infinite thinking loops after tool calls
//! 2. Tool execution results are properly saved
//! 3. State updates correctly after tool execution
//! 4. Long-term stability (30+ rounds)

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

/// Mock LLM backend with tracking capabilities.
struct MockLlmBackend {
    response_queue: Arc<RwLock<Vec<StreamChunk>>>,
    call_count: Arc<RwLock<usize>>,
}

impl MockLlmBackend {
    fn new() -> Self {
        Self {
            response_queue: Arc::new(RwLock::new(Vec::new())),
            call_count: Arc::new(RwLock::new(0)),
        }
    }

    async fn set_response_chunks(&self, chunks: Vec<StreamChunk>) {
        let mut queue = self.response_queue.write().await;
        *queue = chunks;
    }

    async fn get_call_count(&self) -> usize {
        *self.call_count.read().await
    }

    async fn reset_call_count(&self) {
        *self.call_count.write().await = 0;
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
        *self.call_count.write().await += 1;
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
        *self.call_count.write().await += 1;
        let chunks = self.response_queue.read().await;
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

/// Stateful mock tool that tracks its calls.
struct StatefulMockTool {
    name: String,
    state: Arc<RwLock<serde_json::Value>>,
}

#[async_trait::async_trait]
impl Tool for StatefulMockTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "A stateful mock tool for testing"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {"type": "string"},
                "value": {"type": "string"}
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult<ToolOutput> {
        let action = args["action"].as_str().unwrap_or("get");
        let value = args["value"].as_str().unwrap_or("");

        // Handle both string and number values
        let value_str = if value.is_empty() {
            // Try to get value as number
            if let Some(n) = args["value"].as_i64() {
                n.to_string()
            } else {
                "".to_string()
            }
        } else {
            value.to_string()
        };

        // Update state
        let mut state = self.state.write().await;
        let current_state = state.clone();

        let new_state = match action {
            "set" => {
                let updated = json!({
                    "last_action": "set",
                    "value": value_str,
                    "previous": current_state,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                *state = updated.clone();
                updated
            }
            "increment" => {
                let current_val = current_state["value"]
                    .as_str()
                    .unwrap_or("0")
                    .parse::<i32>()
                    .unwrap_or(0);
                let new_val = current_val + 1;
                let updated = json!({
                    "last_action": "increment",
                    "value": new_val.to_string(),
                    "previous": current_state,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });
                *state = updated.clone();
                updated
            }
            _ => {
                json!({
                    "last_action": "get",
                    "value": current_state["value"].as_str().unwrap_or("uninitialized"),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            }
        };

        Ok(ToolOutput {
            success: true,
            data: new_state,
            error: None,
            metadata: None,
        })
    }
}

impl StatefulMockTool {
    async fn get_state(&self) -> serde_json::Value {
        self.state.read().await.clone()
    }
}

/// Simple mock tool.
struct SimpleMockTool {
    name: String,
}

#[async_trait::async_trait]
impl Tool for SimpleMockTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "A simple mock tool"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({"type": "object", "properties": {}})
    }

    async fn execute(&self, _args: serde_json::Value) -> ToolResult<ToolOutput> {
        Ok(ToolOutput {
            success: true,
            data: json!({"result": "success", "tool": self.name}),
            error: None,
            metadata: None,
        })
    }
}

/// Create simple chunks.
fn create_simple_chunks(response: &str) -> Vec<StreamChunk> {
    vec![Ok((response.to_string(), false))]
}

/// Create tool call chunks.
fn create_tool_call_chunks(
    tool_name: &str,
    tool_args: serde_json::Value,
    final_response: &str,
) -> Vec<StreamChunk> {
    vec![
        Ok((format!("Let me use {} to help you.", tool_name), true)),
        Ok((
            format!("<tool_calls><invoke name=\"{}\">", tool_name),
            false,
        )),
        Ok((create_parameter_xml(&tool_args), false)),
        Ok((format!("</invoke></tool_calls>{}", final_response), false)),
    ]
}

/// Create parameter XML.
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
            params.push_str(&format!(
                "<parameter name=\"{}\" value=\"{}\"/>",
                key, value_str
            ));
        }
        params
    } else {
        String::new()
    }
}

/// Collect events and analyze for potential issues.
struct EventAnalyzer {
    events: Vec<AgentEvent>,
    thinking_chunks: usize,
    content_chunks: usize,
    tool_calls: Vec<String>,
    errors: Vec<String>,
    has_end: bool,
    max_thinking_sequence: usize,
    current_thinking_sequence: usize,
}

impl EventAnalyzer {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            thinking_chunks: 0,
            content_chunks: 0,
            tool_calls: Vec::new(),
            errors: Vec::new(),
            has_end: false,
            max_thinking_sequence: 0,
            current_thinking_sequence: 0,
        }
    }

    fn analyze(&mut self, event: &AgentEvent) {
        self.events.push(event.clone());
        match event {
            AgentEvent::Thinking { .. } => {
                self.thinking_chunks += 1;
                self.current_thinking_sequence += 1;
                self.max_thinking_sequence = self
                    .max_thinking_sequence
                    .max(self.current_thinking_sequence);
            }
            AgentEvent::Content { .. } => {
                self.content_chunks += 1;
                self.current_thinking_sequence = 0;
            }
            AgentEvent::ToolCallStart { tool, .. } => {
                if !self.tool_calls.contains(tool) {
                    self.tool_calls.push(tool.clone());
                }
                self.current_thinking_sequence = 0;
            }
            AgentEvent::ToolCallEnd { .. } => {
                self.current_thinking_sequence = 0;
            }
            AgentEvent::Error { message } => {
                self.errors.push(message.clone());
            }
            AgentEvent::End => {
                self.has_end = true;
            }
        }
    }

    /// Check for potential infinite thinking loop.
    fn has_thinking_loop(&self) -> bool {
        // Criteria for potential thinking loop:
        // 1. More than 10 thinking chunks
        // 2. No tool calls or content after thinking
        // 3. No end event
        self.thinking_chunks > 10 && self.tool_calls.is_empty() && !self.has_end
    }

    /// Check if response completed properly.
    fn is_complete(&self) -> bool {
        self.has_end
            && !self
                .errors
                .iter()
                .any(|e| e.contains("infinite") || e.contains("loop") || e.contains("too long"))
    }
}

async fn collect_and_analyze(
    stream: Pin<Box<dyn Stream<Item = AgentEvent> + Send>>,
) -> EventAnalyzer {
    let mut analyzer = EventAnalyzer::new();
    futures::pin_mut!(stream);
    while let Some(event) = stream.next().await {
        analyzer.analyze(&event);
        if event.is_end() {
            break;
        }
    }
    analyzer
}

/// Test 1: Verify tool execution doesn't cause thinking loops.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_no_thinking_loop_after_tool_call() {
    println!("\n=== Test: No Thinking Loop After Tool Call ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    let mut registry = ToolRegistryBuilder::new().build();
    registry.register(Arc::new(StatefulMockTool {
        name: "stateful_tool".to_string(),
        state: Arc::new(RwLock::new(json!({}))),
    }));
    registry.register(Arc::new(SimpleMockTool {
        name: "simple_tool".to_string(),
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

    // Test multiple rounds with tool calls
    let test_cases = vec![
        (
            "Use stateful tool to set value to 5",
            "stateful_tool",
            json!({"action": "set", "value": "5"}),
        ),
        ("Use simple tool", "simple_tool", json!({})),
        (
            "Increment the value",
            "stateful_tool",
            json!({"action": "increment"}),
        ),
        (
            "Get current value",
            "stateful_tool",
            json!({"action": "get"}),
        ),
        ("Use simple tool again", "simple_tool", json!({})),
    ];

    for (i, (user_msg, expected_tool, args)) in test_cases.iter().enumerate() {
        println!("--- Round {} ---", i + 1);
        println!("User: {}", user_msg);

        mock_llm.reset_call_count().await;

        let chunks = create_tool_call_chunks(
            expected_tool,
            args.clone(),
            &format!("{} executed successfully.", expected_tool),
        );

        mock_llm.set_response_chunks(chunks).await;

        let start = std::time::Instant::now();
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            session_manager.process_message_events(&session_id, user_msg),
        )
        .await;

        match result {
            Ok(Ok(stream)) => {
                let duration = start.elapsed();
                let analyzer = collect_and_analyze(stream).await;

                println!("  Thinking chunks: {}", analyzer.thinking_chunks);
                println!("  Content chunks: {}", analyzer.content_chunks);
                println!("  Tools called: {:?}", analyzer.tool_calls);
                println!("  Has end: {}", analyzer.has_end);
                println!("  Duration: {:?}", duration);

                // Verify no thinking loop
                if analyzer.has_thinking_loop() {
                    panic!("Round {}: Detected thinking loop!", i + 1);
                }

                // Verify completion
                if !analyzer.is_complete() {
                    panic!("Round {}: Response did not complete properly", i + 1);
                }

                // Verify tool was called
                if !analyzer.tool_calls.contains(&expected_tool.to_string()) {
                    panic!(
                        "Round {}: Expected tool '{}' not called",
                        i + 1,
                        expected_tool
                    );
                }

                // Verify LLM was called reasonable times (initial + follow-up, not infinite)
                let call_count = mock_llm.get_call_count().await;
                println!("  LLM calls: {}", call_count);

                // Should be called exactly 2 times: Phase 1 (detect tools) + Phase 2 (follow-up without tools)
                if call_count > 3 {
                    panic!(
                        "Round {}: Too many LLM calls ({}), possible loop detected",
                        i + 1,
                        call_count
                    );
                }

                println!("  ✅ Round {} passed", i + 1);
            }
            Ok(Err(e)) => {
                panic!("Round {}: Error: {}", i + 1, e);
            }
            Err(_) => {
                panic!("Round {}: Timeout", i + 1);
            }
        }
    }

    println!("\n✅ No thinking loop detected after tool calls");
}

/// Test 2: Verify tool state updates are persisted.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_tool_state_persistence() {
    println!("\n=== Test: Tool State Persistence ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    let stateful_tool = Arc::new(StatefulMockTool {
        name: "state_tool".to_string(),
        state: Arc::new(RwLock::new(json!({"value": "0"}))),
    });

    let mut registry = ToolRegistryBuilder::new().build();
    registry.register(stateful_tool.clone());

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

    // Sequence of operations that should update state
    let operations = vec![
        (
            "Set value to 10",
            json!({"action": "set", "value": "10"}),
            "10",
        ),
        ("Increment", json!({"action": "increment"}), "11"),
        ("Increment again", json!({"action": "increment"}), "12"),
        (
            "Set to 100",
            json!({"action": "set", "value": "100"}),
            "100",
        ),
        ("Increment", json!({"action": "increment"}), "101"),
    ];

    // Pre-execute the tool once to initialize its state properly
    {
        let initial_args = json!({"action": "set", "value": "0"});
        let _ = stateful_tool.execute(initial_args).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    for (i, (user_msg, args, expected_value)) in operations.iter().enumerate() {
        println!("--- Step {}: {} ===", i + 1, user_msg);

        let chunks = create_tool_call_chunks(
            "state_tool",
            args.clone(),
            &format!("Value is now {}", expected_value),
        );

        mock_llm.set_response_chunks(chunks).await;

        let result = tokio::time::timeout(
            Duration::from_secs(5),
            session_manager.process_message_events(&session_id, user_msg),
        )
        .await;

        match result {
            Ok(Ok(stream)) => {
                let _events = collect_and_analyze(stream).await;

                // Give tool execution time to complete
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Check the actual tool state
                let state = stateful_tool.get_state().await;
                println!(
                    "  Full state: {}",
                    serde_json::to_string_pretty(&state).unwrap_or_else(|_| "N/A".to_string())
                );
                let actual_value = state["value"].as_str().unwrap_or("not found");

                println!("  Expected value: {}", expected_value);
                println!("  Actual value: {}", actual_value);

                if actual_value != *expected_value {
                    panic!(
                        "Step {}: State not updated correctly. Expected {}, got {}",
                        i + 1,
                        expected_value,
                        actual_value
                    );
                }

                println!("  ✅ Step {} passed - state persisted correctly", i + 1);
            }
            Ok(Err(e)) => {
                panic!("Step {}: Error: {}", i + 1, e);
            }
            Err(_) => {
                panic!("Step {}: Timeout", i + 1);
            }
        }
    }

    println!("\n✅ Tool state persistence verified");
}

/// Test 3: Long-term stability test (30 rounds).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_long_term_stability_30_rounds() {
    println!("\n=== Test: Long-term Stability (30 Rounds) ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    let mut registry = ToolRegistryBuilder::new().build();
    registry.register(Arc::new(SimpleMockTool {
        name: "tool_a".to_string(),
    }));
    registry.register(Arc::new(SimpleMockTool {
        name: "tool_b".to_string(),
    }));
    registry.register(Arc::new(SimpleMockTool {
        name: "tool_c".to_string(),
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

    let mut total_rounds = 30;
    let mut passed_rounds = 0;
    let mut failed_rounds = Vec::new();
    let mut total_duration = Duration::from_secs(0);

    let tools = vec!["tool_a", "tool_b", "tool_c"];

    for round in 0..total_rounds {
        let tool = tools[round % tools.len()];
        let user_msg = format!("Round {}: Use {}", round + 1, tool);

        mock_llm.reset_call_count().await;

        let chunks = create_tool_call_chunks(
            tool,
            json!({}),
            &format!("Round {} complete with {}", round + 1, tool),
        );

        mock_llm.set_response_chunks(chunks).await;

        let start = std::time::Instant::now();
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            session_manager.process_message_events(&session_id, &user_msg),
        )
        .await;

        let duration = start.elapsed();
        total_duration += duration;

        match result {
            Ok(Ok(stream)) => {
                let analyzer = collect_and_analyze(stream).await;

                // Check for issues
                let mut round_passed = true;

                if analyzer.has_thinking_loop() {
                    println!("  ❌ Round {}: Thinking loop detected!", round + 1);
                    round_passed = false;
                }

                if !analyzer.is_complete() {
                    println!("  ❌ Round {}: Incomplete response", round + 1);
                    round_passed = false;
                }

                if !analyzer.tool_calls.contains(&tool.to_string()) {
                    println!(
                        "  ❌ Round {}: Expected tool '{}' not called",
                        round + 1,
                        tool
                    );
                    round_passed = false;
                }

                let call_count = mock_llm.get_call_count().await;
                if call_count > 3 {
                    println!(
                        "  ❌ Round {}: Too many LLM calls ({})",
                        round + 1,
                        call_count
                    );
                    round_passed = false;
                }

                if duration.as_secs() > 8 {
                    println!("  ⚠️  Round {}: Slow response ({:?})", round + 1, duration);
                    round_passed = false;
                }

                if round_passed {
                    passed_rounds += 1;
                    if (round + 1) % 10 == 0 {
                        println!("  ✅ Round {}: OK (duration: {:?})", round + 1, duration);
                    }
                } else {
                    failed_rounds.push(round + 1);
                }
            }
            Ok(Err(e)) => {
                println!("  ❌ Round {}: Error: {}", round + 1, e);
                failed_rounds.push(round + 1);
            }
            Err(_) => {
                println!("  ❌ Round {}: Timeout", round + 1);
                failed_rounds.push(round + 1);
            }
        }
    }

    println!("\n=== Long-term Stability Test Results ===");
    println!("Total rounds: {}", total_rounds);
    println!("Passed: {}", passed_rounds);
    println!("Failed: {}", failed_rounds.len());
    println!("Total duration: {:?}", total_duration);
    println!(
        "Average per round: {:?}",
        total_duration / total_rounds as u32
    );

    if !failed_rounds.is_empty() {
        println!("Failed rounds: {:?}", failed_rounds);
        panic!("Long-term stability test failed!");
    }

    println!("✅ Long-term stability verified (30 rounds passed)");
}

/// Test 4: Rapid consecutive tool calls don't cause issues.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rapid_consecutive_tool_calls() {
    println!("\n=== Test: Rapid Consecutive Tool Calls ===\n");

    let _event_bus = EventBus::new();
    let mock_llm = Arc::new(MockLlmBackend::new());

    let mut registry = ToolRegistryBuilder::new().build();
    for i in 1..=6 {
        registry.register(Arc::new(SimpleMockTool {
            name: format!("rapid_tool_{}", i),
        }));
    }

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

    // Call 6 different tools in rapid succession
    for i in 1..=6 {
        let tool_name = format!("rapid_tool_{}", i);
        let user_msg = format!("Execute {}", tool_name);

        let chunks = create_tool_call_chunks(
            &tool_name,
            json!({}),
            &format!("Tool {} executed", tool_name),
        );

        mock_llm.set_response_chunks(chunks).await;

        let start = std::time::Instant::now();
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            session_manager.process_message_events(&session_id, &user_msg),
        )
        .await;

        let duration = start.elapsed();

        match result {
            Ok(Ok(stream)) => {
                let analyzer = collect_and_analyze(stream).await;

                if analyzer.has_thinking_loop() {
                    panic!("Tool {}: Thinking loop detected!", i);
                }

                if !analyzer.is_complete() {
                    panic!("Tool {}: Incomplete response", i);
                }

                if !analyzer.tool_calls.contains(&tool_name) {
                    panic!("Tool {}: Not called", i);
                }

                println!("  Tool {}: OK ({:?})", i, duration);
            }
            Ok(Err(e)) => {
                panic!("Tool {}: Error: {}", i, e);
            }
            Err(_) => {
                panic!("Tool {}: Timeout", i);
            }
        }
    }

    println!("\n✅ Rapid consecutive tool calls handled correctly");
}
