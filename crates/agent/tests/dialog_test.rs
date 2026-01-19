//! Simple dialogue test for agent evaluation.
//!
//! Tests basic conversation flow with actual LLM.

use std::sync::Arc;
use edge_ai_agent::Agent;
use edge_ai_agent::agent::LlmBackend;

/// Test simple greeting
#[tokio::test]
async fn test_dialogue_greeting() {
    let agent = Agent::with_session("test_greeting".to_string());

    // Configure LLM
    let backend = LlmBackend::Ollama {
        endpoint: "http://localhost:11434".to_string(),
        model: "qwen2.5:3b".to_string(),
    };

    match agent.configure_llm(backend).await {
        Ok(_) => println!("LLM configured successfully"),
        Err(e) => {
            eprintln!("Failed to configure LLM: {}", e);
            return;
        }
    }

    // Test greeting
    let test_queries = vec![
        "你好",
        "列出设备",
        "有哪些规则",
    ];

    for query in test_queries {
        println!("\n=== Testing: '{}' ===", query);
        let start = std::time::Instant::now();

        match agent.process(query).await {
            Ok(response) => {
                let elapsed = start.elapsed();
                println!("Response ({}ms): {}", elapsed.as_millis(), response.message.content);
                println!("Tools used: {:?}", response.tools_used);
                println!("Processing time: {}ms", response.processing_time_ms);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

/// Test conversation with context
#[tokio::test]
async fn test_dialogue_with_context() {
    let agent = Agent::with_session("test_context".to_string());

    let backend = LlmBackend::Ollama {
        endpoint: "http://localhost:11434".to_string(),
        model: "qwen2.5:3b".to_string(),
    };

    if let Err(e) = agent.configure_llm(backend).await {
        eprintln!("Failed to configure LLM: {}", e);
        return;
    }

    // Multi-turn conversation
    let turns = vec![
        "列出所有设备",
        "第一条设备是什么？",
        "关闭第一条设备",
    ];

    for (i, query) in turns.iter().enumerate() {
        println!("\n=== Turn {}: '{}' ===", i + 1, query);
        let start = std::time::Instant::now();

        match agent.process(query).await {
            Ok(response) => {
                let elapsed = start.elapsed();
                println!("Response ({}ms): {}", elapsed.as_millis(), response.message.content);
                println!("Tools used: {:?}", response.tools_used);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

/// Test tool calling accuracy
#[tokio::test]
async fn test_tool_calling_accuracy() {
    let agent = Agent::with_session("test_tools".to_string());

    let backend = LlmBackend::Ollama {
        endpoint: "http://localhost:11434".to_string(),
        model: "qwen2.5:3b".to_string(),
    };

    if let Err(e) = agent.configure_llm(backend).await {
        eprintln!("Failed to configure LLM: {}", e);
        return;
    }

    // Test cases: (query, expected_tool)
    let test_cases = vec![
        ("列出所有设备", "list_devices"),
        ("列出所有规则", "list_rules"),
        ("查询温度数据", "query_data"),
    ];

    let mut passed = 0;
    let mut total = test_cases.len();

    for (query, expected_tool) in &test_cases {
        println!("\n=== Testing: '{}' (expect tool: {}) ===", query, expected_tool);

        match agent.process(query).await {
            Ok(response) => {
                let elapsed = response.processing_time_ms;
                let tool_used = response.tools_used.first().map(|s| s.as_str());

                if tool_used == Some(*expected_tool) {
                    println!("✓ PASS - Used correct tool: {}, Time: {}ms", expected_tool, elapsed);
                    println!("  Response: {}", response.message.content.chars().take(100).collect::<String>());
                    passed += 1;
                } else {
                    println!("✗ FAIL - Expected: {}, Got: {:?}", expected_tool, tool_used);
                    println!("  Response: {}", response.message.content);
                }
            }
            Err(e) => {
                eprintln!("✗ ERROR: {}", e);
            }
        }
    }

    println!("\n=== Summary: {}/{} passed ===", passed, total);
    assert!(passed >= total * 2 / 3, "At least 2/3 of tests should pass");
}

/// Test response time
#[tokio::test]
async fn test_response_time() {
    let agent = Agent::with_session("test_speed".to_string());

    let backend = LlmBackend::Ollama {
        endpoint: "http://localhost:11434".to_string(),
        model: "qwen2.5:3b".to_string(),
    };

    if let Err(e) = agent.configure_llm(backend).await {
        eprintln!("Failed to configure LLM: {}", e);
        return;
    }

    let queries = vec![
        "你好",
        "列出设备",
        "有哪些规则",
        "查询温度",
    ];

    let mut times = Vec::new();

    for query in queries {
        match agent.process(query).await {
            Ok(response) => {
                times.push(response.processing_time_ms);
                println!("'{}' -> {}ms", query, response.processing_time_ms);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    if !times.is_empty() {
        let avg = times.iter().sum::<u64>() / times.len() as u64;
        let max = *times.iter().max().unwrap();
        let min = *times.iter().min().unwrap();

        println!("\n=== Response Time Stats ===");
        println!("Average: {}ms", avg);
        println!("Min: {}ms", min);
        println!("Max: {}ms", max);

        // qwen2:1.5b should respond in under 5 seconds for simple queries
        assert!(avg < 5000, "Average response time should be under 5 seconds");
    }
}
