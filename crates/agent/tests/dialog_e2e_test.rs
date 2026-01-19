//! End-to-end dialogue test for core tools.
//!
//! This test simulates real user conversations to validate the new core tools.

use edge_ai_agent::agent::Agent;

/// Test a single dialogue turn
async fn test_dialog_turn(agent: &Agent, name: &str, user_message: &str) {
    println!("测试: {}", name);
    println!("问题: {}", user_message);

    match agent.process(user_message).await {
        Ok(response) => {
            let content = &response.message.content;
            let preview = if content.len() > 300 {
                format!("{}...", &content[..300])
            } else {
                content.clone()
            };
            println!("回答: {}", preview);
            println!("工具调用: {} 个", response.tool_calls.len());

            // Print tool call details
            for call in &response.tool_calls {
                println!("  - {}", call.name);
            }
            println!("状态: ✓ 通过\n");
        }
        Err(e) => {
            println!("错误: {}\n", e);
        }
    }
}

#[tokio::test]
async fn test_dialog_suite() {
    println!("\n");
    println!("═══════════════════════════════════════════════════════════");
    println!("              NeoTalk 核心工具对话测试");
    println!("═══════════════════════════════════════════════════════════\n");

    // Create agent with a session
    let agent = Agent::with_session("dialog_test_session".to_string());

    let tests = vec![
        ("device.discover - 发现所有设备", "系统中有哪些设备？"),
        ("device.discover - 按位置过滤", "客厅有哪些设备？"),
        ("device.query - 查询温度", "客厅温度多少度？"),
        ("device.control - 控制设备", "把客厅灯打开"),
        ("device.analyze - 数据分析", "分析一下温度趋势"),
        ("rule.from_context - 创建规则", "创建一个规则：温度超过50度时告警"),
        ("rule.from_context - 复杂规则", "温度持续5分钟超过30度时开启风扇"),
        ("device.control - 批量控制", "把所有灯都关掉"),
        ("device.discover - 查询离线", "哪些设备离线了？"),
        ("device.query - 湿度查询", "客厅的湿度是多少？"),
    ];

    let mut passed = 0;
    let total = tests.len();

    for (name, user_message) in tests {
        test_dialog_turn(&agent, name, user_message).await;

        // Check if response was successful
        match agent.process(user_message).await {
            Ok(_) => passed += 1,
            Err(_) => {}
        }

        // Small delay between tests
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("═══════════════════════════════════════════════════════════");
    println!("测试结果: 至少一半的测试应该产生有效响应");
    println!("═══════════════════════════════════════════════════════════\n");

    // At least half of the tests should succeed
    assert!(passed >= total / 2, "At least half of the dialog tests should pass");
}

#[tokio::test]
async fn test_single_dialog_device_discover() {
    let agent = Agent::with_session("test_discover".to_string());

    println!("\n测试: device.discover");
    println!("问题: 有哪些设备？");

    let response = agent.process("有哪些设备？").await.unwrap();
    println!("回答: {}\n", response.message.content);

    assert!(!response.message.content.is_empty());
}

#[tokio::test]
async fn test_single_dialog_device_query() {
    let agent = Agent::with_session("test_query".to_string());

    println!("\n测试: device.query");
    println!("问题: 客厅温度多少？");

    let response = agent.process("客厅温度多少？").await.unwrap();
    println!("回答: {}\n", response.message.content);

    assert!(!response.message.content.is_empty());
}

#[tokio::test]
async fn test_single_dialog_device_control() {
    let agent = Agent::with_session("test_control".to_string());

    println!("\n测试: device.control");
    println!("问题: 把客厅灯打开");

    let response = agent.process("把客厅灯打开").await.unwrap();
    println!("回答: {}\n", response.message.content);

    assert!(!response.message.content.is_empty());
}

#[tokio::test]
async fn test_single_dialog_device_analyze() {
    let agent = Agent::with_session("test_analyze".to_string());

    println!("\n测试: device.analyze");
    println!("问题: 分析一下温度趋势");

    let response = agent.process("分析一下温度趋势").await.unwrap();
    println!("回答: {}\n", response.message.content);

    assert!(!response.message.content.is_empty());
}

#[tokio::test]
async fn test_single_dialog_rule_from_context() {
    let agent = Agent::with_session("test_rule".to_string());

    println!("\n测试: rule.from_context");
    println!("问题: 创建一个规则：温度超过50度时告警");

    let response = agent.process("创建一个规则：温度超过50度时告警").await.unwrap();
    println!("回答: {}\n", response.message.content);

    assert!(!response.message.content.is_empty());
}

#[tokio::test]
async fn test_multi_turn_conversation() {
    let agent = Agent::with_session("test_multi_turn".to_string());

    println!("\n测试: 多轮对话");

    // Turn 1
    println!("第1轮 - 用户: 有哪些设备？");
    let r1 = agent.process("有哪些设备？").await.unwrap();
    println!("助手: {}\n", r1.message.content);

    // Turn 2
    println!("第2轮 - 用户: 客厅温度多少？");
    let r2 = agent.process("客厅温度多少？").await.unwrap();
    println!("助手: {}\n", r2.message.content);

    // Turn 3
    println!("第3轮 - 用户: 把客厅灯打开");
    let r3 = agent.process("把客厅灯打开").await.unwrap();
    println!("助手: {}\n", r3.message.content);

    assert!(r1.message.content.len() > 5);
    assert!(r2.message.content.len() > 5);
    assert!(r3.message.content.len() > 5);
}
