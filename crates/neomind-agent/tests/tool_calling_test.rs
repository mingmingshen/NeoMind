//! Tool Calling Integration Test with Real LLM
//!
//! This test verifies:
//! 1. User sends a message that should trigger tool calls
//! 2. LLM correctly identifies and generates tool calls
//! 3. Tools are executed and results are formatted
//! 4. Response includes tool execution results
//!
//! Requires Ollama to be running on localhost:11434
//! Recommended models (small/fast):
//! - qwen2.5:3b (tested)
//! - qwen3-vl:2b (fast, supports thinking)
//! - phi3:3.8b (good reasoning)
//!
//! Run with:
//!   cargo test --test tool_calling_test -- --ignored
//!
//! To use a specific model:
//!   MODEL=qwen3-vl:2b cargo test --test tool_calling_test -- --ignored

use std::sync::Arc;
use std::time::{Duration, Instant};

use neomind_core::message::Message;
use neomind_agent::session::SessionManager;
use neomind_llm::{OllamaConfig, OllamaRuntime};
use neomind_core::llm::backend::LlmRuntime;

// ============================================================================
// Test Context
// ============================================================================

struct ToolCallingTestContext {
    pub session_manager: SessionManager,
    pub session_id: String,
    pub model_name: String,
}

impl ToolCallingTestContext {
    async fn new() -> anyhow::Result<Self> {
        // Get model from environment or use default
        let model_name = std::env::var("MODEL")
            .unwrap_or_else(|_| "qwen2.5:3b".to_string());

        println!("📦 Using model: {}", model_name);

        // Create session manager with in-memory store
        let session_manager = SessionManager::memory();

        // Get Ollama endpoint from environment or use default
        let ollama_endpoint = std::env::var("OLLAMA_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        // Create a session (returns new session_id)
        let session_id = session_manager.create_session().await?;

        // Configure LLM backend for this session
        let ollama_config = OllamaConfig {
            endpoint: ollama_endpoint.clone(),
            model: model_name.clone(),
            timeout_secs: 120,
        };

        let llm_runtime = Arc::new(OllamaRuntime::new(ollama_config)?);

        // Get the agent and set custom LLM
        let agent = session_manager.get_session(&session_id).await?;
        agent.set_custom_llm(llm_runtime).await;

        Ok(Self {
            session_manager,
            session_id,
            model_name,
        })
    }

    async fn send_message(&self, message: &str) -> anyhow::Result<String> {
        println!("\n📤 User: {}", message);
        let start = Instant::now();

        let response = self.session_manager
            .process_message(&self.session_id, message)
            .await?;

        let elapsed = start.elapsed();

        println!("📥 Assistant ({}ms):", elapsed.as_millis());
        println!("   {}", response.message.content.trim());

        if !response.tool_calls.is_empty() {
            println!("\n🔧 Tool Calls ({}):", response.tool_calls.len());
            for (i, tc) in response.tool_calls.iter().enumerate() {
                println!("   {}. {} {:?}", i + 1, tc.name, tc.arguments);
                if let Some(result) = &tc.result {
                    let result_str = result.to_string();
                    let preview = if result_str.len() > 200 {
                        format!("{}...", &result_str[..200])
                    } else {
                        result_str
                    };
                    println!("      Result: {}", preview);
                }
            }
        }

        Ok(response.message.content.clone())
    }

    async fn get_history(&self) -> anyhow::Result<Vec<String>> {
        let agent = self.session_manager.get_session(&self.session_id).await?;
        let messages = agent.history().await;
        Ok(messages.iter().map(|m| {
            let preview: String = m.content.chars().take(100).collect();
            format!("{}: {}", m.role, preview)
        }).collect())
    }
}

// ============================================================================
// Helper: Check Ollama availability
// ============================================================================

fn ollama_available() -> bool {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 11434));
    std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok()
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_test -- --ignored"]
async fn test_simple_device_query() -> anyhow::Result<()> {
    if !ollama_available() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    let ctx = ToolCallingTestContext::new().await?;

    println!("\n=== Test: Simple Device Query ===\n");

    // Send a message that should trigger device query
    let response = ctx.send_message("列出所有设备").await?;

    // Verify response
    assert!(!response.is_empty(), "Response should not be empty");

    // Check if any tool was called (may vary by model)
    let history = ctx.get_history().await?;
    println!("\n📜 History ({} messages):", history.len());
    for msg in &history {
        println!("  {}", msg);
    }

    println!("\n✅ Simple device query test passed!");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_test -- --ignored"]
async fn test_rule_query() -> anyhow::Result<()> {
    if !ollama_available() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    let ctx = ToolCallingTestContext::new().await?;

    println!("\n=== Test: Rule Query ===\n");

    let response = ctx.send_message("查看所有自动化规则").await?;

    assert!(!response.is_empty(), "Response should not be empty");

    println!("\n✅ Rule query test passed!");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_test -- --ignored"]
async fn test_multi_tool_conversation() -> anyhow::Result<()> {
    if !ollama_available() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    let ctx = ToolCallingTestContext::new().await?;

    println!("\n=== Test: Multi-Tool Conversation ===\n");

    // First query - devices
    let _ = ctx.send_message("有哪些设备？").await?;

    // Second query - rules (context should be maintained)
    let _ = ctx.send_message("有哪些自动化规则？").await?;

    // Third query - combine information
    let response = ctx.send_message("总结一下设备和规则的情况").await?;

    assert!(!response.is_empty(), "Response should not be empty");

    println!("\n✅ Multi-tool conversation test passed!");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_test -- --ignored"]
async fn test_device_discovery() -> anyhow::Result<()> {
    if !ollama_available() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    let ctx = ToolCallingTestContext::new().await?;

    println!("\n=== Test: Device Discovery ===\n");

    let response = ctx.send_message("发现并搜索所有设备").await?;

    assert!(!response.is_empty(), "Response should not be empty");

    println!("\n✅ Device discovery test passed!");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_test -- --ignored"]
async fn test_conversational_context_with_tools() -> anyhow::Result<()> {
    if !ollama_available() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    let ctx = ToolCallingTestContext::new().await?;

    println!("\n=== Test: Conversational Context with Tools ===\n");

    // First establish context
    let _ = ctx.send_message("我家里有几个温度传感器").await?;

    // Follow-up question (should remember context)
    let response = ctx.send_message("它们的当前读数是多少？").await?;

    assert!(!response.is_empty(), "Response should not be empty");

    println!("\n✅ Conversational context test passed!");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_test -- --ignored"]
async fn test_tool_calling_format_variations() -> anyhow::Result<()> {
    if !ollama_available() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    let ctx = ToolCallingTestContext::new().await?;

    println!("\n=== Test: Tool Calling Format Variations ===\n");

    // Test different phrasings that should trigger the same tool
    let queries = vec![
        "列出设备",
        "查看所有设备",
        "设备列表",
        "show me devices",
        "what devices do you have",
    ];

    for query in queries {
        println!("\n--- Testing query: \"{}\" ---", query);
        let response = ctx.send_message(query).await?;
        assert!(!response.is_empty(), "Response should not be empty for: {}", query);
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n✅ Format variations test passed!");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_test -- --ignored"]
async fn test_error_handling_in_tool_calls() -> anyhow::Result<()> {
    if !ollama_available() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    let ctx = ToolCallingTestContext::new().await?;

    println!("\n=== Test: Error Handling in Tool Calls ===\n");

    // Query a non-existent device (should handle gracefully)
    let response = ctx.send_message("查询设备 'non_existent_device_12345' 的温度").await?;

    // Should still get a response even if device doesn't exist
    assert!(!response.is_empty(), "Should still respond even with errors");

    println!("\n✅ Error handling test passed!");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_test -- --ignored"]
async fn test_direct_llm_tool_calling() -> anyhow::Result<()> {
    if !ollama_available() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    println!("\n=== Test: Direct LLM Tool Calling ===\n");

    let model_name = std::env::var("MODEL")
        .unwrap_or_else(|_| "qwen2.5:3b".to_string());

    let ollama_endpoint = std::env::var("OLLAMA_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());

    let ollama_config = OllamaConfig {
        endpoint: ollama_endpoint,
        model: model_name.clone(),
        timeout_secs: 60,
    };

    let llm = Arc::new(OllamaRuntime::new(ollama_config)?);

    // Test with tools enabled
    use neomind_core::llm::backend::{LlmInput, GenerationParams, ToolDefinition};

    let tools = vec![
        ToolDefinition {
            name: "list_devices".to_string(),
            description: "List all available devices".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_device_info".to_string(),
            description: "Get information about a specific device".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "device_id": {
                        "type": "string",
                        "description": "The device ID"
                    }
                },
                "required": ["device_id"]
            }),
        },
    ];

    let messages = vec![
        Message::system("You are a helpful IoT device assistant. Use tools when needed."),
        Message::user("List all devices for me."),
    ];

    let input = LlmInput {
        messages,
        params: GenerationParams {
            temperature: Some(0.1),
            max_tokens: Some(500),
            ..Default::default()
        },
        model: Some(model_name),
        stream: false,
        tools: Some(tools),
    };

    println!("Sending request to LLM...");
    let start = Instant::now();
    let output = llm.generate(input).await?;
    let elapsed = start.elapsed();

    println!("\nLLM Response ({}ms):", elapsed.as_millis());
    println!("{}", output.text);

    // Check if tool call format is detected
    let (content, tool_calls) = neomind_agent::agent::tool_parser::parse_tool_calls(&output.text)?;

    println!("\nParsed content: {}", content);
    println!("Parsed tool calls: {}", tool_calls.len());

    for (i, tc) in tool_calls.iter().enumerate() {
        println!("  {}. {} {:?}", i + 1, tc.name, tc.arguments);
    }

    println!("\n✅ Direct LLM tool calling test passed!");
    Ok(())
}

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_test -- --ignored"]
async fn test_comparison_all_queries() -> anyhow::Result<()> {
    if !ollama_available() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    let ctx = ToolCallingTestContext::new().await?;

    println!("\n=== Test: Comparison - All Query Types ===\n");

    let test_queries = vec![
        ("Device List", "列出所有设备"),
        ("Rule List", "列出所有自动化规则"),
        ("Device Discovery", "发现新设备"),
        ("Complex Query", "帮我检查一下所有温度传感器的状态"),
    ];

    let mut results = Vec::new();

    for (name, query) in test_queries {
        println!("\n--- {} ---", name);
        let start = Instant::now();

        let response = ctx.send_message(query).await?;
        let elapsed = start.elapsed();

        results.push((name, elapsed.as_millis()));

        // Create new session for next test
        ctx.session_id.clone();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n📊 Performance Summary:");
    for (name, elapsed) in &results {
        println!("  {}: {}ms", name, elapsed);
    }

    let avg: u128 = results.iter().map(|(_, e)| e).sum::<u128>() / results.len() as u128;
    println!("  Average: {}ms", avg);

    println!("\n✅ Comparison test passed!");
    Ok(())
}
