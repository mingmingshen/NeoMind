//! Quick test for multiple tool calls in single request

use std::sync::Arc;
use neomind_core::message::Message;
use neomind_core::llm::backend::{LlmInput, GenerationParams, ToolDefinition, LlmRuntime};
use neomind_llm::{OllamaConfig, OllamaRuntime};

#[tokio::test]
#[ignore]
async fn test_multi_tools_single_request() -> anyhow::Result<()> {
    let model = std::env::var("MODEL").unwrap_or("qwen2.5:3b".to_string());
    let endpoint = std::env::var("OLLAMA_ENDPOINT").unwrap_or("http://localhost:11434".to_string());

    let config = OllamaConfig {
        endpoint,
        model: model.clone(),
        timeout_secs: 60,
    };
    let llm = Arc::new(OllamaRuntime::new(config)?);
    
    let tools = vec![
        ToolDefinition {
            name: "list_devices".to_string(),
            description: "List all devices".to_string(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        },
        ToolDefinition {
            name: "list_rules".to_string(),
            description: "List all rules".to_string(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        },
    ];
    
    // Test 1: Multiple tools with AND
    println!("=== Test 1: Multiple tools with AND ===");
    let messages = vec![
        Message::system("You are a helpful assistant. Use tools when needed. When user asks for multiple things, call ALL relevant tools."),
        Message::user("List all devices AND all rules."),
    ];
    
    let input = LlmInput {
        messages,
        params: GenerationParams { temperature: Some(0.1), max_tokens: Some(500), ..Default::default() },
        model: Some(model.clone()),
        stream: false,
        tools: Some(tools.clone()),
    };
    
    let output = llm.generate(input).await?;
    println!("Response: {}", output.text);
    let (_, calls) = neomind_agent::agent::tool_parser::parse_tool_calls(&output.text)?;
    println!("Tool calls detected: {}\n", calls.len());
    
    // Test 2: Explicitly ask for both
    println!("=== Test 2: Explicit request ===");
    let messages2 = vec![
        Message::system("You are a helpful assistant. Available tools: list_devices, list_rules. Call tools using JSON array format: [{\"name\":\"tool_name\",\"arguments\":{}}]"),
        Message::user("I need to see both: 1) all devices, 2) all rules"),
    ];
    
    let input2 = LlmInput {
        messages: messages2,
        params: GenerationParams { temperature: Some(0.0), max_tokens: Some(500), ..Default::default() },
        model: Some(model),
        stream: false,
        tools: Some(tools),
    };
    
    let output2 = llm.generate(input2).await?;
    println!("Response: {}", output2.text);
    let (_, calls2) = neomind_agent::agent::tool_parser::parse_tool_calls(&output2.text)?;
    println!("Tool calls detected: {}", calls2.len());
    
    Ok(())
}
