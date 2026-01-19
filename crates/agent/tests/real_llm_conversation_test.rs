//! 真实 LLM 对话测试
//!
//! 使用真实的 Ollama LLM 后端测试对话改进功能

use std::sync::Arc;
use std::time::Duration;

use edge_ai_agent::{Agent, AgentConfig, LlmBackend, tools::{AskUserTool, ConfirmActionTool, ClarifyIntentTool}};
use edge_ai_tools::ToolRegistryBuilder;

/// 测试配置
fn test_config() -> AgentConfig {
    AgentConfig {
        name: "NeoTalk Agent".to_string(),
        system_prompt: r#"你是NeoTalk智能物联网助手。

## 核心原则

### 1. 信息追问
当用户请求缺少必要信息时，使用 ask_user 工具追问：
- 用户说"打开灯" → 问"要打开哪个位置的灯？"
- 用户说"查看温度" → 问"要查看哪个房间的温度？"

### 2. 二次确认
执行以下操作前必须使用 confirm_action 工具确认：
- 删除规则/设备
- 关闭所有设备
- 批量操作

### 3. 意图澄清
遇到歧义时使用 clarify_intent 工具询问

## 可用工具
- ask_user: 向用户询问缺失信息
- confirm_action: 二次确认危险操作
- clarify_intent: 澄清模糊意图
- list_devices: 列出所有设备
- control_device: 控制设备"#.to_string(),
        model: "gpt-oss:20b".to_string(),
        temperature: 0.4,
        ..Default::default()
    }
}

/// 创建带有交互工具的 Agent
async fn create_agent_with_interaction_tools() -> Agent {
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

    let agent = Agent::with_tools(test_config(), "test_session".to_string(), Arc::new(registry));

    // 配置 Ollama 后端
    let backend = LlmBackend::Ollama {
        endpoint: "http://localhost:11434".to_string(),
        model: "gpt-oss:20b".to_string(),
    };

    agent.configure_llm(backend).await.unwrap();

    agent
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn real_llm_comprehensive_test() {
    println!("\n=== 真实 LLM 综合对话测试 ===\n");

    let agent = create_agent_with_interaction_tools().await;

    let test_cases = vec![
        ("打开灯", "应该追问哪个位置的灯"),
        ("删除所有规则", "应该要求二次确认"),
        ("温度", "应该澄清意图"),
        ("列出设备", "应该直接执行"),
    ];

    for (i, (input, expected)) in test_cases.iter().enumerate() {
        println!("--- 测试用例 {}: {} ---", i + 1, input);
        println!("预期: {}", expected);

        let response = agent.process(input).await.unwrap();
        println!("实际: {}", response.message.content.chars().take(150).collect::<String>());

        if !response.tool_calls.is_empty() {
            println!("工具: {:?}", response.tool_calls);
        }
        println!();

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("=== 测试完成 ===");
}
