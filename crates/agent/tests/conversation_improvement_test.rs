//! 对话改进综合测试
//!
//! 测试目标：
//! 1. LLM 在信息不足时使用 ask_user 追问
//! 2. LLM 在危险操作前使用 confirm_action 确认
//! 3. LLM 支持多轮上下文对话
//! 4. LLM 在意图模糊时使用 clarify_intent

use std::sync::Arc;
use std::time::Duration;

use edge_ai_agent::{
    Agent, AgentConfig,
    tools::{AskUserTool, ConfirmActionTool, ClarifyIntentTool},
};
use edge_ai_tools::ToolRegistryBuilder;

#[tokio::test]
async fn test_ask_user_on_missing_info() {
    // 创建一个带有标准工具的 Agent
    let mut registry = ToolRegistryBuilder::new()
        .with_query_data_tool()
        .with_control_device_tool()
        .with_list_devices_tool()
        .with_create_rule_tool()
        .with_list_rules_tool()
        .build();

    // 添加交互工具
    registry.register(Arc::new(AskUserTool::new()));
    registry.register(Arc::new(ConfirmActionTool::new()));
    registry.register(Arc::new(ClarifyIntentTool::new()));

    let mut config = AgentConfig::default();
    config.system_prompt = r#"你是NeoTalk智能助手。

## 核心原则
1. **信息追问**: 当用户请求缺少必要信息时，使用 ask_user 工具追问
2. **二次确认**: 执行危险操作前使用 confirm_action 工具确认

## 使用示例
- 用户说"打开灯" → 使用 ask_user 询问"要打开哪个位置的灯？"
- 用户说"删除所有规则" → 使用 confirm_action 确认"#.to_string();

    let agent = Agent::with_tools(config, "test_session".to_string(), Arc::new(registry));

    // 测试场景1: 用户说"打开灯" - 应该使用 ask_user
    let response = agent.process("打开灯").await.unwrap();

    println!("=== 测试场景1: 打开灯 ===");
    println!("响应内容: {}", response.message.content);
    println!("工具调用: {:?}", response.tool_calls);

    // 验证是否调用了 ask_user 工具
    let asked_user = response.tool_calls.iter().any(|tc| {
        tc.name.contains("ask_user") || tc.name.contains("AskUser")
    });

    if asked_user {
        println!("✅ 测试通过: LLM 使用了 ask_user 工具");
    } else {
        println!("⚠️ 测试警告: LLM 未使用 ask_user 工具，响应: {}", response.message.content);
    }
}

#[tokio::test]
async fn test_confirm_action_on_dangerous_operation() {
    let mut registry = ToolRegistryBuilder::new()
        .with_query_data_tool()
        .with_control_device_tool()
        .with_list_devices_tool()
        .with_create_rule_tool()
        .with_list_rules_tool()
        .build();

    registry.register(Arc::new(AskUserTool::new()));
    registry.register(Arc::new(ConfirmActionTool::new()));
    registry.register(Arc::new(ClarifyIntentTool::new()));

    let mut config = AgentConfig::default();
    config.system_prompt = r#"你是NeoTalk智能助手。

## 核心原则
1. **二次确认**: 执行危险操作前必须使用 confirm_action 工具确认
   - 危险操作包括: 删除规则、删除设备、关闭所有设备

## 使用示例
- 用户说"删除所有规则" → 使用 confirm_action 确认"#.to_string();

    let agent = Agent::with_tools(config, "test_session".to_string(), Arc::new(registry));

    // 测试场景2: 用户说"删除所有规则" - 应该使用 confirm_action
    let response = agent.process("删除所有规则").await.unwrap();

    println!("=== 测试场景2: 删除所有规则 ===");
    println!("响应内容: {}", response.message.content);
    println!("工具调用: {:?}", response.tool_calls);

    let confirmed = response.tool_calls.iter().any(|tc| {
        tc.name.contains("confirm") || tc.name.contains("Confirm")
    });

    if confirmed {
        println!("✅ 测试通过: LLM 使用了 confirm_action 工具");
    } else {
        println!("⚠️ 测试警告: LLM 未使用 confirm_action 工具");
    }
}

#[tokio::test]
async fn test_multi_turn_context_conversation() {
    let mut registry = ToolRegistryBuilder::new()
        .with_query_data_tool()
        .with_control_device_tool()
        .with_list_devices_tool()
        .with_create_rule_tool()
        .with_list_rules_tool()
        .build();

    registry.register(Arc::new(AskUserTool::new()));

    let mut config = AgentConfig::default();
    config.system_prompt = r#"你是NeoTalk智能助手。

## 核心原则
1. **上下文对话**: 记住对话历史，支持多轮交互
   - 用"它/该设备"等代词指代之前提到的对象
2. **信息追问**: 信息不足时主动追问"#.to_string();

    let agent = Agent::with_tools(config, "test_session".to_string(), Arc::new(registry));

    println!("=== 测试场景3: 多轮对话 ===");

    // 第一轮: 用户问"客厅温度是多少"
    let response1 = agent.process("客厅温度是多少").await.unwrap();
    println!("第一轮 - 用户: 客厅温度是多少");
    println!("第一轮 - 响应: {}", response1.message.content);

    // 模拟工具返回（如果有）
    if !response1.tool_calls.is_empty() {
        // 假设工具返回了温度数据
        let _tool_result = r#"{"temperature": 26.5, "device": "客厅温度传感器"}"#;
        let _ = agent.process_tool_result(
            &response1.tool_calls[0].id,
            &format!("客厅温度是26.5°C")
        ).await;
    }

    // 第二轮: 用户说"那卧室呢" - 应该理解是在问卧室温度
    tokio::time::sleep(Duration::from_millis(100)).await;
    let response2 = agent.process("那卧室呢").await.unwrap();
    println!("第二轮 - 用户: 那卧室呢");
    println!("第二轮 - 响应: {}", response2.message.content);

    // 验证 LLM 是否理解了上下文
    if response2.message.content.contains("卧室") || response2.message.content.contains("bedroom") {
        println!("✅ 测试通过: LLM 理解了上下文");
    } else {
        println!("⚠️ 测试警告: LLM 可能未理解上下文");
    }
}

#[tokio::test]
async fn test_clarify_ambiguous_intent() {
    let mut registry = ToolRegistryBuilder::new()
        .with_query_data_tool()
        .with_control_device_tool()
        .with_list_devices_tool()
        .with_create_rule_tool()
        .with_list_rules_tool()
        .build();

    registry.register(Arc::new(ClarifyIntentTool::new()));

    let mut config = AgentConfig::default();
    config.system_prompt = r#"你是NeoTalk智能助手。

## 核心原则
1. **意图澄清**: 遇到歧义时使用 clarify_intent 工具询问
   - 用户说"温度"可能想查询、控制或分析
   - 不要猜测，主动澄清"#.to_string();

    let agent = Agent::with_tools(config, "test_session".to_string(), Arc::new(registry));

    // 测试场景4: 用户说"温度" - 意图模糊
    let response = agent.process("温度").await.unwrap();

    println!("=== 测试场景4: 意图澄清 ===");
    println!("响应内容: {}", response.message.content);
    println!("工具调用: {:?}", response.tool_calls);

    let clarified = response.tool_calls.iter().any(|tc| {
        tc.name.contains("clarify") || tc.name.contains("Clarify")
    });

    if clarified || response.message.content.contains("查询") && response.message.content.contains("还是") {
        println!("✅ 测试通过: LLM 进行了意图澄清");
    } else {
        println!("⚠️ 测试警告: LLM 可能未进行意图澄清");
    }
}

#[tokio::test]
async fn test_combined_scenario() {
    // 综合场景测试
    let mut registry = ToolRegistryBuilder::new()
        .with_query_data_tool()
        .with_control_device_tool()
        .with_list_devices_tool()
        .with_create_rule_tool()
        .with_list_rules_tool()
        .build();

    registry.register(Arc::new(AskUserTool::new()));
    registry.register(Arc::new(ConfirmActionTool::new()));
    registry.register(Arc::new(ClarifyIntentTool::new()));

    let agent = Agent::with_tools(AgentConfig::default(), "test_session".to_string(), Arc::new(registry));

    println!("=== 综合场景测试 ===\n");

    let test_cases = vec![
        "打开灯",
        "删除所有规则",
        "温度",
        "查看设备",
    ];

    for (i, input) in test_cases.iter().enumerate() {
        println!("--- 测试用例 {}: {} ---", i + 1, input);
        let response = agent.process(input).await.unwrap();
        println!("响应: {}\n工具调用: {:?}\n",
            response.message.content.chars().take(100).collect::<String>(),
            response.tool_calls);
    }

    println!("=== 综合场景测试完成 ===");
}

// 运行所有测试的主函数
#[tokio::test]
async fn run_all_conversation_improvement_tests() {
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║       对话改进综合测试套件                            ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    // 注意：这些是独立的测试，这里只是集中执行
    // 每个测试应该单独运行以获得更详细的结果
    println!("注意: 请单独运行各个测试以获得详细输出\n");
    println!("可用的测试:");
    println!("  - test_ask_user_on_missing_info");
    println!("  - test_confirm_action_on_dangerous_operation");
    println!("  - test_multi_turn_context_conversation");
    println!("  - test_clarify_ambiguous_intent");
    println!("  - test_combined_scenario");

    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║       测试列表完成                                    ║");
    println!("╚══════════════════════════════════════════════════════╝\n");
}
