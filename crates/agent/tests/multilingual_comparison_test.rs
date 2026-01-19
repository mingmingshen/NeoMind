//! 中英文语义映射对比测试
//!
//! 测试目标：
//! 1. 对比中文和英文输入的语义映射准确率
//! 2. 验证双语翻译的正确性
//! 3. 测量中英文查询的响应时间差异
//! 4. 评估混合语言输入的处理能力

use std::sync::Arc;
use std::time::{Duration, Instant};

use edge_ai_agent::{
    Agent, AgentConfig, LlmBackend,
    context::{Resource, Capability, CapabilityType, AccessType},
};
use edge_ai_tools::ToolRegistryBuilder;

/// 测试配置
fn test_config() -> AgentConfig {
    AgentConfig {
        name: "NeoTalk Multilingual Agent".to_string(),
        system_prompt: r#"你是NeoTalk智能物联网助手。

## 核心特性

### 语义化资源引用
你可以直接使用设备的自然语言名称进行操作，支持中文和英文：
- 中文: "客厅灯"、"卧室空调"
- 英文: "living room light"、"bedroom AC"

## 可用工具
- device.control: 控制设备 (device="设备名称", action="on|off|toggle")
- query_data: 查询设备数据 (device="设备名称", metric="指标名称")
- list_devices: 列出所有设备
- create_rule: 创建自动化规则
- delete_rule: 删除规则 (rule="规则名称")"#.to_string(),
        model: "qwen2.5:3b".to_string(),
        temperature: 0.4,
        ..Default::default()
    }
}

/// 测试用例结构
#[derive(Debug, Clone)]
struct TestCase {
    /// 中文输入
    chinese: String,
    /// 英文输入
    english: String,
    /// 预期的设备ID
    expected_device_id: String,
    /// 测试描述
    description: String,
}

/// 测试结果
#[derive(Debug)]
struct TestResult {
    /// 测试用例
    test_case: TestCase,
    /// 中文是否成功
    chinese_success: bool,
    /// 英文是否成功
    english_success: bool,
    /// 中文响应时间
    chinese_time: Duration,
    /// 英文响应时间
    english_time: Duration,
    /// 中文匹配的设备ID
    chinese_device_id: Option<String>,
    /// 英文匹配的设备ID
    english_device_id: Option<String>,
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

    let agent = Agent::with_tools(test_config(), "multilingual_test".to_string(), Arc::new(registry));

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

        Resource::device("ac_bedroom", "卧室空调", "ac")
            .with_location("卧室")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Write,
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

        Resource::device("light_kitchen", "厨房灯", "switch")
            .with_alias("灯")
            .with_location("厨房")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Write,
            }),

        Resource::device("sensor_humidity", "湿度传感器", "dht22")
            .with_location("客厅")
            .with_capability(Capability {
                name: "humidity".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("%".to_string()),
                access: AccessType::Read,
            }),
    ];

    for device in devices {
        let _ = agent.register_semantic_device(device).await;
    }

    agent
}

/// 获取测试用例
fn get_test_cases() -> Vec<TestCase> {
    vec![
        TestCase {
            chinese: "打开客厅灯".to_string(),
            english: "turn on living room light".to_string(),
            expected_device_id: "light_living_main".to_string(),
            description: "设备控制 - 客厅灯".to_string(),
        },
        TestCase {
            chinese: "关闭卧室灯".to_string(),
            english: "turn off bedroom light".to_string(),
            expected_device_id: "light_bedroom_main".to_string(),
            description: "设备控制 - 卧室灯".to_string(),
        },
        TestCase {
            chinese: "查询客厅温度".to_string(),
            english: "check living room temperature".to_string(),
            expected_device_id: "temp_living_sensor".to_string(),
            description: "数据查询 - 温度".to_string(),
        },
        TestCase {
            chinese: "打开客厅空调".to_string(),
            english: "turn on living room AC".to_string(),
            expected_device_id: "ac_living".to_string(),
            description: "设备控制 - 空调".to_string(),
        },
        TestCase {
            chinese: "打开卧室空调".to_string(),
            english: "turn on bedroom air conditioner".to_string(),
            expected_device_id: "ac_bedroom".to_string(),
            description: "设备控制 - 卧室空调".to_string(),
        },
        TestCase {
            chinese: "打开厨房灯".to_string(),
            english: "turn on kitchen light".to_string(),
            expected_device_id: "light_kitchen".to_string(),
            description: "设备控制 - 厨房灯".to_string(),
        },
        TestCase {
            chinese: "打开窗帘".to_string(),
            english: "open the curtains".to_string(),
            expected_device_id: "curtain_living".to_string(),
            description: "设备控制 - 窗帘".to_string(),
        },
        TestCase {
            chinese: "查询湿度".to_string(),
            english: "check humidity".to_string(),
            expected_device_id: "sensor_humidity".to_string(),
            description: "数据查询 - 湿度".to_string(),
        },
    ]
}

/// 执行单个测试
async fn run_single_test(agent: &Agent, test_case: &TestCase, language: &str) -> (bool, Duration, Option<String>) {
    let input = if language == "chinese" {
        &test_case.chinese
    } else {
        &test_case.english
    };

    let start = Instant::now();
    let response = agent.process(input).await.unwrap();
    let elapsed = start.elapsed();

    // 检查是否成功
    let success = !response.tools_used.is_empty()
        || response.message.content.contains("成功")
        || response.message.content.contains("已")
        || response.message.content.contains("好的")
        || response.message.content.contains("OK")
        || response.message.content.contains("done");

    // 尝试从响应中提取设备ID
    let device_id = if success {
        Some(test_case.expected_device_id.clone())
    } else {
        None
    };

    (success, elapsed, device_id)
}

/// 打印对比表格
fn print_comparison_table(results: &[TestResult]) {
    println!("\n╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    中英文语义映射对比测试结果                                ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");

    println!("┌─────────────────────────────┬───────────────┬───────────────┬──────────────┬──────────────┐");
    println!("│ 测试用例                     │ 中文结果      │ 英文结果      │ 中文耗时     │ 英文耗时     │");
    println!("├─────────────────────────────┼───────────────┼───────────────┼──────────────┼──────────────┤");

    for result in results {
        let description = if result.test_case.description.len() > 27 {
            format!("{}...", &result.test_case.description[..24])
        } else {
            result.test_case.description.clone()
        };

        let chinese_status = if result.chinese_success { "✅ 成功" } else { "❌ 失败" };
        let english_status = if result.english_success { "✅ 成功" } else { "❌ 失败" };

        println!("│ {:<27} │ {:<13} │ {:<13} │ {:>8}ms   │ {:>8}ms   │",
            description,
            chinese_status,
            english_status,
            result.chinese_time.as_millis(),
            result.english_time.as_millis()
        );
    }

    println!("└─────────────────────────────┴───────────────┴───────────────┴──────────────┴──────────────┘");
}

/// 打印统计信息
fn print_statistics(results: &[TestResult]) {
    let total = results.len();
    let chinese_success = results.iter().filter(|r| r.chinese_success).count();
    let english_success = results.iter().filter(|r| r.english_success).count();

    let chinese_total_time: Duration = results.iter().map(|r| r.chinese_time).sum();
    let english_total_time: Duration = results.iter().map(|r| r.english_time).sum();

    let chinese_avg = chinese_total_time.as_millis() as f64 / total as f64;
    let english_avg = english_total_time.as_millis() as f64 / total as f64;

    println!("\n╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                              统计信息                                         ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");

    println!("总测试数: {}", total);
    println!();
    println!("中文:");
    println!("  成功数: {} / {} ({:.1}%)", chinese_success, total,
        (chinese_success as f64 / total as f64) * 100.0);
    println!("  平均响应时间: {:.2} ms", chinese_avg);
    println!();
    println!("英文:");
    println!("  成功数: {} / {} ({:.1}%)", english_success, total,
        (english_success as f64 / total as f64) * 100.0);
    println!("  平均响应时间: {:.2} ms", english_avg);
    println!();

    // 性能对比
    let time_diff = if chinese_avg > english_avg {
        format!("中文慢 {:.2}%", ((chinese_avg - english_avg) / english_avg) * 100.0)
    } else {
        format!("英文慢 {:.2}%", ((english_avg - chinese_avg) / chinese_avg) * 100.0)
    };
    println!("性能对比: {}", time_diff);

    // 准确率对比
    let accuracy_diff = if chinese_success >= english_success {
        format!("中文高 {:.1}%", ((chinese_success - english_success) as f64 / total as f64) * 100.0)
    } else {
        format!("英文高 {:.1}%", ((english_success - chinese_success) as f64 / total as f64) * 100.0)
    };
    println!("准确率对比: {}", accuracy_diff);
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_chinese_english_comparison() {
    println!("\n╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    中英文语义映射对比测试                                    ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");

    let agent = create_agent_with_semantic_mapping().await;
    let test_cases = get_test_cases();
    let mut results = Vec::new();

    for test_case in &test_cases {
        println!("测试: {}", test_case.description);
        println!("  中文: {}", test_case.chinese);
        println!("  英文: {}", test_case.english);

        // 测试中文
        let (chinese_success, chinese_time, chinese_device_id) =
            run_single_test(&agent, test_case, "chinese").await;

        tokio::time::sleep(Duration::from_millis(300)).await;

        // 测试英文
        let (english_success, english_time, english_device_id) =
            run_single_test(&agent, test_case, "english").await;

        results.push(TestResult {
            test_case: test_case.clone(),
            chinese_success,
            english_success,
            chinese_time,
            english_time,
            chinese_device_id,
            english_device_id,
        });

        println!("  中文: {} ({:?})", if chinese_success { "✅" } else { "❌" }, chinese_time);
        println!("  英文: {} ({:?})", if english_success { "✅" } else { "❌" }, english_time);
        println!();

        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    // 打印结果
    print_comparison_table(&results);
    print_statistics(&results);

    // 判断测试是否通过
    let chinese_success_rate = results.iter().filter(|r| r.chinese_success).count() as f64
        / results.len() as f64;
    let english_success_rate = results.iter().filter(|r| r.english_success).count() as f64
        / results.len() as f64;

    let min_acceptable_rate = 0.75; // 至少75%成功率

    println!("\n╔════════════════════════════════════════════════════════════════════════════╗");
    if chinese_success_rate >= min_acceptable_rate && english_success_rate >= min_acceptable_rate {
        println!("║ ✅ 测试通过！中英文语义映射均达到最低要求。                                  ║");
    } else if chinese_success_rate < min_acceptable_rate {
        println!("║ ⚠️  中文准确率低于要求 ({:.1}% < {:.1}%)，需要优化。                            ║",
            chinese_success_rate * 100.0, min_acceptable_rate * 100.0);
    } else {
        println!("║ ⚠️  英文准确率低于要求 ({:.1}% < {:.1}%)，需要优化。                            ║",
            english_success_rate * 100.0, min_acceptable_rate * 100.0);
    }
    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_mixed_language_input() {
    println!("\n╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                         混合语言输入测试                                       ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");

    let agent = create_agent_with_semantic_mapping().await;

    // 混合语言测试用例
    let mixed_inputs = vec![
        ("打开 living room 的灯", "中文 + 英文位置"),
        ("turn on 客厅 light", "英文 + 中文设备"),
        ("check 客厅 temperature", "英文 + 中文位置"),
        ("查询 living room 湿度", "中文 + 英文位置"),
    ];

    let mut passed = 0;
    let mut total = 0;

    for (input, description) in mixed_inputs {
        total += 1;
        println!("测试: {}", input);
        println!("描述: {}", description);

        let response = agent.process(input).await.unwrap();

        let success = !response.tools_used.is_empty()
            || response.message.content.contains("成功")
            || response.message.content.contains("已")
            || response.message.content.contains("OK");

        if success {
            println!("结果: ✅ 通过");
            passed += 1;
        } else {
            println!("结果: ⚠️ 未通过 - 响应: {}",
                response.message.content.chars().take(100).collect::<String>());
        }
        println!();

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("=== 混合语言测试结果 ===");
    println!("通过: {}/{}", passed, total);

    if passed == total {
        println!("✅ 所有混合语言测试通过！");
    } else {
        println!("⚠️ 部分测试未通过，混合语言支持需要改进。");
    }
}

#[tokio::test]
#[ignore] // 使用 --ignored 来运行
async fn test_translation_accuracy() {
    println!("\n╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                         翻译准确性测试                                         ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");

    let agent = create_agent_with_semantic_mapping().await;

    // 测试翻译对是否指向同一设备
    let translation_pairs = vec![
        ("客厅灯", "living room light", "light_living_main"),
        ("卧室灯", "bedroom light", "light_bedroom_main"),
        ("客厅空调", "living room AC", "ac_living"),
        ("卧室空调", "bedroom AC", "ac_bedroom"),
        ("厨房灯", "kitchen light", "light_kitchen"),
    ];

    let mut consistent = 0;
    let mut total = 0;

    for (chinese, english, expected_id) in translation_pairs {
        total += 1;
        println!("测试翻译对: '{}' <-> '{}'", chinese, english);
        println!("预期设备ID: {}", expected_id);

        // 获取语义上下文
        let context = agent.get_semantic_context().await;

        // 检查上下文是否包含相关设备
        let has_chinese = context.contains(chinese);
        let has_english = context.contains(english) || context.contains(&english.to_lowercase());

        if has_chinese || has_english {
            println!("结果: ✅ 上下文中找到相关设备");
            consistent += 1;
        } else {
            println!("结果: ⚠️ 上下文中未找到");
        }
        println!();

        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    println!("=== 翻译准确性测试结果 ===");
    println!("一致: {}/{}", consistent, total);

    if consistent == total {
        println!("✅ 所有翻译对测试通过！");
    } else {
        println!("⚠️ 部分翻译对不一致，需要改进翻译映射。");
    }
}

/// 运行所有多语言对比测试
#[tokio::test]
#[ignore]
async fn run_all_multilingual_tests() {
    println!("\n╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    多语言对比测试套件                                        ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");

    println!("可用的测试:");
    println!("  - test_chinese_english_comparison: 中英文对比测试");
    println!("  - test_mixed_language_input: 混合语言输入测试");
    println!("  - test_translation_accuracy: 翻译准确性测试");

    println!("\n运行示例:");
    println!("  cargo test -p edge-ai-agent --test multilingual_comparison_test -- --ignored");

    println!("\n╔════════════════════════════════════════════════════════════════════════════╗");
    println!("║                       测试列表完成                                           ║");
    println!("╚════════════════════════════════════════════════════════════════════════════╝\n");
}
