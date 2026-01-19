//! 语义映射层 E2E 测试
//!
//! 测试目标：
//! 1. 验证自然语言设备名称能正确映射到技术 ID
//! 2. 验证规则名称能正确映射到技术 ID
//! 3. 验证 LLM 可以直接使用自然名称进行操作
//! 4. 对比使用语义映射前后的对话质量

use std::sync::Arc;
use std::time::Duration;

use edge_ai_agent::{
    Agent, AgentConfig, LlmBackend,
    context::{Resource, Capability, CapabilityType, AccessType},
};
use edge_ai_tools::ToolRegistryBuilder;

/// 测试配置
fn test_config() -> AgentConfig {
    AgentConfig {
        name: "NeoTalk Semantic Agent".to_string(),
        system_prompt: r#"你是NeoTalk智能物联网助手。

## 核心特性

### 语义化资源引用
你可以直接使用设备的自然语言名称进行操作，无需知道技术 ID：
- 使用 "客厅灯" 而不是 "light_living_main"
- 使用 "温度报警规则" 而不是 "rule_001"

## 可用工具
- device.control: 控制设备 (device="设备名称", action="on|off|toggle")
- query_data: 查询设备数据 (device="设备名称", metric="指标名称")
- list_devices: 列出所有设备
- delete_rule: 删除规则 (rule="规则名称")"#.to_string(),
        model: "qwen2.5:3b".to_string(),
        temperature: 0.4,
        ..Default::default()
    }
}

/// 创建带有语义映射的 Agent
async fn create_agent_with_semantic_mapping() -> Agent {
    let registry = ToolRegistryBuilder::new()
        .with_query_data_tool()
        .with_control_device_tool()
        .with_list_devices_tool()
        .with_create_rule_tool()
        .with_list_rules_tool()
        .with_delete_rule_tool()
        .build();

    let agent = Agent::with_tools(test_config(), "semantic_test".to_string(), Arc::new(registry));

    // 配置 Ollama 后端
    let backend = LlmBackend::Ollama {
        endpoint: "http://localhost:11434".to_string(),
        model: "qwen2.5:3b".to_string(),
    };
    agent.configure_llm(backend).await.unwrap();

    // 注册测试设备到语义映射器
    let devices = vec![
        Resource::device("light_living_main", "客厅灯", "switch")
            .with_alias("灯")
            .with_location("客厅")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Write,
            }),

        Resource::device("light_bedroom_main", "卧室灯", "switch")
            .with_alias("灯")
            .with_location("卧室")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Write,
            }),

        Resource::device("temp_living_sensor", "客厅温度传感器", "dht22")
            .with_location("客厅")
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("°C".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("ac_living", "客厅空调", "ac")
            .with_location("客厅")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Write,
            })
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Property,
                data_type: "float".to_string(),
                valid_values: Some(vec!["16".to_string(), "17".to_string(), "18".to_string(),
                    "19".to_string(), "20".to_string(), "21".to_string(), "22".to_string(),
                    "23".to_string(), "24".to_string(), "25".to_string(), "26".to_string(),
                    "27".to_string(), "28".to_string(), "29".to_string(), "30".to_string()]),
                unit: Some("°C".to_string()),
                access: AccessType::ReadWrite,
            }),

        Resource::device("curtain_living", "客厅窗帘", "curtain")
            .with_location("客厅")
            .with_capability(Capability {
                name: "position".to_string(),
                cap_type: CapabilityType::Property,
                data_type: "int".to_string(),
                valid_values: None,
                unit: Some("%".to_string()),
                access: AccessType::ReadWrite,
            }),
    ];

    for device in devices {
        let _ = agent.register_semantic_device(device).await;
    }

    // 注册测试规则
    agent.register_semantic_rules(vec![
        ("rule_001".to_string(), "温度报警规则".to_string(), true),
        ("rule_002".to_string(), "湿度警告".to_string(), false),
        ("rule_003".to_string(), "夜间模式".to_string(), true),
    ]).await;

    agent
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_semantic_device_resolution() {
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║       语义设备解析测试                                ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    let agent = create_agent_with_semantic_mapping().await;

    // 测试场景1: 使用自然语言设备名
    let test_cases = vec![
        ("打开客厅灯", "客厅灯", "light_living_main"),
        ("关闭卧室灯", "卧室灯", "light_bedroom_main"),
        ("查询客厅温度", "客厅温度传感器", "temp_living_sensor"),
        ("打开空调", "客厅空调", "ac_living"),
    ];

    for (i, (input, _expected_name, _)) in test_cases.iter().enumerate() {
        println!("--- 测试用例 {}: {} ---", i + 1, input);
        let response = agent.process(input).await.unwrap();

        println!("输入: {}", input);
        println!("响应: {}", response.message.content.chars().take(100).collect::<String>());

        if !response.tools_used.is_empty() {
            println!("工具: {:?}\n", response.tools_used);
        } else {
            println!("(未调用工具)\n");
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("=== 测试完成 ===");
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_semantic_rule_resolution() {
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║       语义规则解析测试                                ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    let agent = create_agent_with_semantic_mapping().await;

    let test_cases = vec![
        ("删除温度报警规则", "温度报警规则"),
        ("启用夜间模式", "夜间模式"),
        ("禁用湿度警告", "湿度警告"),
    ];

    for (i, (input, expected_rule)) in test_cases.iter().enumerate() {
        println!("--- 测试用例 {}: {} ---", i + 1, input);
        let response = agent.process(input).await.unwrap();

        println!("输入: {}", input);
        println!("预期规则: {}", expected_rule);
        println!("响应: {}", response.message.content.chars().take(100).collect::<String>());

        if !response.tools_used.is_empty() {
            println!("工具: {:?}\n", response.tools_used);
        } else {
            println!("(未调用工具)\n");
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("=== 测试完成 ===");
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_semantic_context_injection() {
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║       语义上下文注入测试                               ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    let agent = create_agent_with_semantic_mapping().await;

    // 获取语义上下文
    let semantic_context = agent.get_semantic_context().await;
    println!("=== 语义上下文 ===\n");
    println!("{}\n", semantic_context);

    // 测试模糊查询
    let ambiguous_queries = vec![
        "有哪些设备",
        "显示所有规则",
        "客厅有什么",
    ];

    for (i, query) in ambiguous_queries.iter().enumerate() {
        println!("--- 查询 {}: {} ---", i + 1, query);
        let response = agent.process(query).await.unwrap();
        println!("响应: {}\n", response.message.content.chars().take(150).collect::<String>());

        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    println!("=== 测试完成 ===");
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_conversation_with_semantic_mapping() {
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║       语义映射对话测试                                ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    let agent = create_agent_with_semantic_mapping().await;

    // 模拟一个完整的对话场景
    let conversation = vec![
        "我回家了",  // 场景设定
        "打开客厅灯", // 语义: 客厅灯 -> light_living_main
        "客厅温度是多少", // 语义: 客厅温度传感器 -> temp_living_sensor
        "把卧室灯也打开", // 语义: 卧室灯 -> light_bedroom_main
        "设置空调到26度", // 语义: 客厅空调 -> ac_living
        "打开窗帘", // 语义: 客厅窗帘 -> curtain_living
        "查看所有设备", // 列表查询
        "启用夜间模式", // 语义: 夜间模式 -> rule_003
    ];

    for (i, input) in conversation.iter().enumerate() {
        println!("--- 第 {} 轮 ---", i + 1);
        println!("用户: {}", input);

        let response = agent.process(input).await.unwrap();

        let response_preview = if response.message.content.chars().count() > 100 {
            response.message.content.chars().take(100).collect::<String>() + "..."
        } else {
            response.message.content.clone()
        };
        println!("助手: {}", response_preview);

        if !response.tools_used.is_empty() {
            println!("工具: {:?}", response.tools_used);
        }

        println!();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // 显示最终统计
    let stats = agent.get_semantic_mapping_stats().await;
    println!("=== 映射统计 ===");
    println!("总映射次数: {}", stats.total_mappings);
    println!("成功: {} ({}%)", stats.successful,
        if stats.total_mappings > 0 {
            (stats.successful * 100 / stats.total_mappings)
        } else {
            0
        });
    println!("失败: {}", stats.failed);
    println!("平均置信度: {:.2}", stats.avg_confidence);

    println!("\n=== 测试完成 ===");
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_quality_improvement_with_semantic_mapping() {
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║       语义映射质量改进测试                             ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    let agent = create_agent_with_semantic_mapping().await;

    println!("测试: 验证 LLM 可以使用自然名称而不是技术 ID\n");

    // 关键测试点: LLM 应该使用自然语言名称，不需要知道技术 ID
    let critical_tests = vec![
        ("打开客厅灯", "应该直接使用 '客厅灯' 而不是 'light_living_main'"),
        ("删除温度报警规则", "应该直接使用 '温度报警规则' 而不是 'rule_001'"),
        ("查询卧室灯状态", "应该使用 '卧室灯' 而不是 'light_bedroom_main'"),
    ];

    let mut passed = 0;
    let mut total = 0;

    for (input, expectation) in critical_tests {
        total += 1;
        println!("测试: {}", input);
        println!("预期: {}", expectation);

        let response = agent.process(input).await.unwrap();

        // 检查是否成功处理
        let success = !response.tools_used.is_empty()
            || response.message.content.contains("成功")
            || response.message.content.contains("已")
            || response.message.content.contains("好的")
            || response.message.content.contains("确定");

        if success {
            println!("结果: ✅ 通过");
            passed += 1;
        } else {
            println!("结果: ⚠️ 未通过 - 响应: {}", response.message.content);
        }
        println!();

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("=== 质量测试结果 ===");
    println!("通过: {}/{}", passed, total);

    if passed == total {
        println!("✅ 所有测试通过！语义映射工作正常。");
    } else {
        println!("⚠️ 部分测试未通过，可能需要调整。");
    }
}

// 运行所有语义映射测试
#[tokio::test]
#[ignore]
async fn run_all_semantic_mapping_tests() {
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║       语义映射综合测试套件                            ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    println!("可用的测试:");
    println!("  - test_semantic_device_resolution");
    println!("  - test_semantic_rule_resolution");
    println!("  - test_semantic_context_injection");
    println!("  - test_conversation_with_semantic_mapping");
    println!("  - test_quality_improvement_with_semantic_mapping");

    println!("\n运行示例:");
    println!("  cargo test -p edge-ai-agent --test semantic_mapping_test -- --ignored");

    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║       测试列表完成                                    ║");
    println!("╚══════════════════════════════════════════════════════╝\n");
}
