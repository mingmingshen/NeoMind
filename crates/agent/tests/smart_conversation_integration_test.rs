//! SmartConversationManager 集成测试
//!
//! 测试应用层智能对话拦截功能
//!
//! ## 测试场景
//!
//! 1. 信息不足拦截 - 用户说"打开灯"时应该追问
//! 2. 危险操作拦截 - 用户说"删除所有规则"时应该确认
//! 3. 意图模糊拦截 - 用户说"温度"时应该澄清
//! 4. 正常执行 - 用户说"列出设备"时应该直接执行

use std::sync::Arc;
use std::time::Duration;

use edge_ai_agent::{
    Agent, AgentConfig, LlmBackend,
    smart_conversation::{Device, Rule},
};
use edge_ai_tools::ToolRegistryBuilder;

/// 创建带有智能对话功能的 Agent
async fn create_agent_with_smart_conversation() -> Agent {
    let registry = ToolRegistryBuilder::new()
        .with_query_data_tool()
        .with_control_device_tool()
        .with_list_devices_tool()
        .with_create_rule_tool()
        .with_list_rules_tool()
        .build();

    let agent = Agent::with_tools(
        AgentConfig::default(),
        "test_smart_session".to_string(),
        Arc::new(registry)
    );

    // 配置 Ollama 后端
    let backend = LlmBackend::Ollama {
        endpoint: "http://localhost:11434".to_string(),
        model: "qwen2.5:3b".to_string(),
    };

    agent.configure_llm(backend).await.unwrap();

    agent
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_smart_conversation_missing_info() {
    println!("\n=== 测试信息不足拦截 ===\n");

    let agent = create_agent_with_smart_conversation().await;

    // 测试用例1: "打开灯" - 应该拦截并追问
    let response = agent.process("打开灯").await.unwrap();

    println!("输入: 打开灯");
    println!("输出: {}", response.message.content);
    println!("工具调用: {:?}", response.tools_used);

    // 验证：响应应该包含问号或追问
    assert!(
        response.message.content.contains("请问") ||
        response.message.content.contains("哪个") ||
        response.message.content.contains("?"),
        "应该追问哪个位置的灯，但实际输出: {}",
        response.message.content
    );

    // 验证：不应该调用任何设备控制工具
    assert!(
        !response.tools_used.iter().any(|t| t.contains("control") || t.contains("device")),
        "信息不足时不应该调用设备控制工具"
    );
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_smart_conversation_dangerous_operation() {
    println!("\n=== 测试危险操作拦截 ===\n");

    let agent = create_agent_with_smart_conversation().await;

    // 测试用例2: "删除所有规则" - 应该拦截并确认
    let response = agent.process("删除所有规则").await.unwrap();

    println!("输入: 删除所有规则");
    println!("输出: {}", response.message.content);
    println!("工具调用: {:?}", response.tools_used);

    // 验证：响应应该包含确认提示
    assert!(
        response.message.content.contains("确认") ||
        response.message.content.contains("确定"),
        "应该要求用户确认，但实际输出: {}",
        response.message.content
    );

    // 验证：不应该调用删除工具
    assert!(
        !response.tools_used.iter().any(|t| t.contains("delete") || t.contains("remove")),
        "危险操作必须先确认，不应该直接执行删除"
    );
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_smart_conversation_ambiguous_intent() {
    println!("\n=== 测试意图模糊拦截 ===\n");

    let agent = create_agent_with_smart_conversation().await;

    // 测试用例3: "温度" - 应该澄清意图
    let response = agent.process("温度").await.unwrap();

    println!("输入: 温度");
    println!("输出: {}", response.message.content);
    println!("工具调用: {:?}", response.tools_used);

    // 验证：响应应该询问意图
    assert!(
        response.message.content.contains("查询") ||
        response.message.content.contains("设置") ||
        response.message.content.contains("哪个房间"),
        "应该澄清用户是想查询还是设置温度，但实际输出: {}",
        response.message.content
    );
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_smart_conversation_normal_execution() {
    println!("\n=== 测试正常执行 ===\n");

    let agent = create_agent_with_smart_conversation().await;

    // 测试用例4: "列出设备" - 应该直接执行
    let response = agent.process("列出设备").await.unwrap();

    println!("输入: 列出设备");
    println!("输出: {}", response.message.content);
    println!("工具调用: {:?}", response.tools_used);

    // 验证：应该调用了 list_devices 或类似工具
    assert!(
        response.tools_used.iter().any(|t| t.contains("list") || t.contains("device")),
        "应该直接调用列出设备的工具"
    );
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_smart_conversation_comprehensive() {
    println!("\n=== 智能对话综合测试 ===\n");

    let agent = create_agent_with_smart_conversation().await;

    let test_cases = vec![
        ("打开灯", "应该追问位置"),
        ("关闭所有设备", "应该要求确认"),
        ("温度", "应该澄清意图"),
        ("列出设备", "应该直接执行"),
        ("删除所有规则", "应该要求确认"),
        ("查看湿度", "应该追问房间"),
        ("客厅灯打开", "应该直接执行"), // 这个有明确位置，应该执行
    ];

    for (i, (input, expected)) in test_cases.iter().enumerate() {
        println!("\n--- 测试用例 {}: {} ---", i + 1, input);
        println!("预期: {}", expected);

        let response = agent.process(input).await.unwrap();
        println!("实际: {}", response.message.content.chars().take(100).collect::<String>());

        if !response.tools_used.is_empty() {
            println!("工具: {:?}", response.tools_used);
        }

        // 短暂延迟避免请求过快
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n=== 测试完成 ===");
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_smart_conversation_with_context_update() {
    println!("\n=== 测试上下文更新后的智能对话 ===\n");

    let agent = create_agent_with_smart_conversation().await;

    // 更新设备列表，让智能对话管理器知道有哪些设备
    let devices = vec![
        Device {
            id: "light-1".to_string(),
            name: "客厅灯".to_string(),
            location: "客厅".to_string(),
            device_type: "light".to_string(),
        },
        Device {
            id: "light-2".to_string(),
            name: "卧室灯".to_string(),
            location: "卧室".to_string(),
            device_type: "light".to_string(),
        },
    ];

    agent.update_smart_context_devices(devices).await;

    // 更新规则列表
    let rules = vec![
        Rule {
            id: "rule-1".to_string(),
            name: "温度报警规则".to_string(),
            enabled: true,
        },
    ];

    agent.update_smart_context_rules(rules).await;

    // 现在测试：说"打开灯"应该仍然追问（因为有多个灯）
    let response1 = agent.process("打开灯").await.unwrap();
    println!("输入: 打开灯");
    println!("输出: {}", response1.message.content);
    assert!(response1.message.content.contains("请问") || response1.message.content.contains("哪个"));

    // 但说"打开客厅灯"应该直接执行
    let response2 = agent.process("打开客厅灯").await.unwrap();
    println!("\n输入: 打开客厅灯");
    println!("输出: {}", response2.message.content.chars().take(100).collect::<String>());
    println!("工具: {:?}", response2.tools_used);
}
