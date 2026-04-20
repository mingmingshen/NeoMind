//! Tool Calling Chain Test - Testing dependent tool calls
//!
//! This tests scenarios where tools have dependencies:
//! 1. device_discover → get device_id → get_device_data(device_id)
//! 2. list_agents → get agent_id → get_agent_executions(agent_id)
//!
//! Run with:
//!   cargo test -p neomind-agent --test tool_chain_test -- --ignored --nocapture

use std::sync::Arc;
use std::time::{Duration, Instant};

use neomind_agent::session::SessionManager;
use neomind_agent::{OllamaConfig, OllamaRuntime};

#[tokio::test]
#[ignore]
async fn test_tool_calling_chain() -> anyhow::Result<()> {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 11434));
    if std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_err() {
        println!("⚠️  Ollama not available");
        return Ok(());
    }

    let model = std::env::var("MODEL").unwrap_or("qwen3.5:2b".to_string());
    let endpoint = std::env::var("OLLAMA_ENDPOINT").unwrap_or("http://localhost:11434".to_string());

    println!("\n🔗 Tool Calling Chain Test");
    println!("📦 Model: {}", model);

    let session_manager = SessionManager::memory();

    let config = OllamaConfig {
        endpoint,
        model,
        timeout_secs: 120,
    };

    // Test cases with dependency chains
    let test_cases = vec![
        // Chain 1: device (list → latest)
        (
            "设备数据链路",
            "查看 ne101 test 设备的当前数据",
            vec!["device", "device"],
        ),
        // Chain 2: device (list → history)
        (
            "设备历史趋势链路",
            "分析 ne101 test 设备今天的电池电量变化趋势",
            vec!["device", "device", "device"],
        ),
        // Chain 3: agent (list → executions)
        (
            "Agent执行历史链路",
            "查看第一个Agent的最近执行记录",
            vec!["agent", "agent"],
        ),
        // Chain 4: agent (list → detail → executions)
        (
            "Agent详情链路",
            "列出所有Agent，然后查看第一个Agent的详细信息和执行历史",
            vec!["agent", "agent", "agent"],
        ),
        // Chain 5: Multi-device data
        (
            "多设备数据链路",
            "获取前两个设备的详细数据",
            vec!["device", "device"],
        ),
    ];

    let mut results = Vec::new();

    for (name, query, expected_tools) in test_cases {
        println!("\n--- {} ---", name);
        println!("Query: {}", query);

        // Create new session for each test
        let session_id = session_manager.create_session().await?;
        let llm = Arc::new(OllamaRuntime::new(config.clone())?);
        let agent = session_manager.get_session(&session_id).await?;
        agent.set_custom_llm(llm).await;

        let start = Instant::now();
        let response = session_manager.process_message(&session_id, query).await?;
        let elapsed = start.elapsed();

        let tools: Vec<String> = response.tool_calls.iter().map(|t| t.name.clone()).collect();
        let tool_count = tools.len();

        println!("Tools called ({}): {:?}", tool_count, tools);
        println!("Time: {}ms", elapsed.as_millis());

        // Show tool results
        for tc in &response.tool_calls {
            if let Some(result) = &tc.result {
                let preview = serde_json::to_string(result).unwrap_or_default();
                let preview = if preview.len() > 200 {
                    &preview[..200]
                } else {
                    &preview
                };
                println!("  {} result: {}", tc.name, preview);
            }
        }

        // Check if chain was followed
        let chain_followed = expected_tools
            .iter()
            .all(|t| tools.iter().any(|c| c.contains(t)));
        let tool_count_ok = tool_count >= expected_tools.len().saturating_sub(1); // Allow missing 1

        results.push((
            name.to_string(),
            tool_count,
            expected_tools.len(),
            chain_followed || tool_count_ok,
        ));

        println!(
            "Response: {}",
            response
                .message
                .content
                .chars()
                .take(100)
                .collect::<String>()
        );
    }

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("📊 TOOL CHAIN SUMMARY");
    println!("{}", "=".repeat(60));

    let mut total_tools = 0;
    let mut total_expected = 0;
    let mut successful_chains = 0;

    for (name, actual, expected, success) in &results {
        let status = if *success { "✅" } else { "⚠️" };
        println!(
            "{} {}: {} tools (expected {})",
            status, name, actual, expected
        );
        total_tools += actual;
        total_expected += expected;
        if *success {
            successful_chains += 1;
        }
    }

    println!("\nTotal tools called: {}", total_tools);
    println!("Total expected: {}", total_expected);
    println!(
        "Chain success rate: {}/{}",
        successful_chains,
        results.len()
    );

    // Analyze multi-turn behavior
    println!("\n📈 Multi-turn Analysis:");
    println!("The Agent may call tools over multiple turns.");
    println!("Check if subsequent turns complete the chain...");

    // Test a multi-turn conversation explicitly
    println!("\n--- Multi-turn Conversation Test ---");
    let session_id = session_manager.create_session().await?;
    let llm = Arc::new(OllamaRuntime::new(config)?);
    let agent = session_manager.get_session(&session_id).await?;
    agent.set_custom_llm(llm).await;

    let mut total_turn_tools = 0;

    // Turn 1: Get devices
    let response1 = session_manager
        .process_message(&session_id, "列出所有设备")
        .await?;
    let tools1 = response1.tool_calls.len();
    println!("Turn 1 (列出所有设备): {} tools", tools1);
    for tc in &response1.tool_calls {
        println!("  - {}", tc.name);
    }
    total_turn_tools += tools1;

    // Turn 2: Get specific device data
    let response2 = session_manager
        .process_message(&session_id, "获取第一个设备的详细数据")
        .await?;
    let tools2 = response2.tool_calls.len();
    println!("Turn 2 (获取第一个设备详细数据): {} tools", tools2);
    for tc in &response2.tool_calls {
        println!("  - {}", tc.name);
    }
    total_turn_tools += tools2;

    println!("\nMulti-turn total tools: {}", total_turn_tools);

    Ok(())
}
