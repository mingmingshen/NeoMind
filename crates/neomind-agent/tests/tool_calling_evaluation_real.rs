//! Real Tool Calling Evaluation Test (Updated with actual tools)
//!
//! This test evaluates tool calling with REAL LLM and REAL tool execution.
//! Tests are aligned with the actual available tools in the system.
//!
//! Run with:
//!   cargo test -p neomind-agent --test tool_calling_evaluation_real -- --ignored --nocapture

#![allow(dead_code)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use neomind_agent::session::SessionManager;
use neomind_agent::{OllamaConfig, OllamaRuntime};

// ============================================================================
// Test Cases - Aligned with actual available tools
// ============================================================================

/// Actual available tools in the system:
///
/// **Device Tools:**
///   - device_discover - 列出所有设备
///   - get_device_data - 获取设备当前所有数据
///   - query_data - 查询设备历史数据
///   - device_control - 控制设备
///   - device_analyze - 分析设备数据趋势
///
/// **Rule Tools:**
///   - list_rules - 列出所有自动化规则
///   - create_rule - 创建自动化规则
///   - delete_rule - 删除规则
///
/// **Agent Tools:**
///   - list_agents - 列出所有AI Agent
///   - get_agent - 获取Agent详细信息
///   - execute_agent - 执行Agent
///   - control_agent - 控制Agent（暂停/恢复/删除）
///   - create_agent - 创建Agent
///   - agent_memory - 查询Agent记忆
///   - get_agent_executions - 获取Agent执行历史
///   - get_agent_execution_detail - 获取单次执行详情
///   - get_agent_conversation - 获取Agent对话历史

#[derive(Debug, Clone)]
struct TestCase {
    name: String,
    query: String,
    expected_tools: Vec<String>,
    min_tools: usize,
    description: String,
}

fn get_test_cases() -> Vec<TestCase> {
    vec![
        // === Device Tools Tests ===
        TestCase {
            name: "设备发现".to_string(),
            query: "列出所有设备".to_string(),
            expected_tools: vec!["device_discover".to_string()],
            min_tools: 1,
            description: "应该调用设备发现工具".to_string(),
        },
        TestCase {
            name: "设备列表查询".to_string(),
            query: "查看有哪些设备".to_string(),
            expected_tools: vec!["device_discover".to_string()],
            min_tools: 1,
            description: "应该调用设备发现工具".to_string(),
        },
        TestCase {
            name: "设备数据查询".to_string(),
            query: "查询设备 abc123 的当前数据".to_string(),
            expected_tools: vec!["get_device_data".to_string()],
            min_tools: 1,
            description: "应该调用获取设备数据工具".to_string(),
        },
        TestCase {
            name: "设备历史数据".to_string(),
            query: "查看设备 abc123 的电池电量历史趋势".to_string(),
            expected_tools: vec!["query_data".to_string(), "get_device_data".to_string()],
            min_tools: 1,
            description: "应该调用数据查询工具".to_string(),
        },
        // === Rule Tools Tests ===
        TestCase {
            name: "规则列表".to_string(),
            query: "列出所有自动化规则".to_string(),
            expected_tools: vec!["list_rules".to_string()],
            min_tools: 1,
            description: "应该调用规则列表工具".to_string(),
        },
        TestCase {
            name: "查看规则".to_string(),
            query: "显示所有规则".to_string(),
            expected_tools: vec!["list_rules".to_string()],
            min_tools: 1,
            description: "应该调用规则列表工具".to_string(),
        },
        // === Agent Tools Tests ===
        TestCase {
            name: "Agent列表".to_string(),
            query: "列出所有AI Agent".to_string(),
            expected_tools: vec!["list_agents".to_string()],
            min_tools: 1,
            description: "应该调用Agent列表工具".to_string(),
        },
        TestCase {
            name: "Agent详情".to_string(),
            query: "查看Agent agent_1的详细信息".to_string(),
            expected_tools: vec!["get_agent".to_string()],
            min_tools: 1,
            description: "应该调用获取Agent详情工具".to_string(),
        },
        TestCase {
            name: "Agent执行历史".to_string(),
            query: "查看Agent agent_1的执行历史".to_string(),
            expected_tools: vec!["get_agent_executions".to_string(), "get_agent".to_string()],
            min_tools: 1,
            description: "应该调用Agent执行历史工具".to_string(),
        },
        // === Multi-Tool Tests ===
        TestCase {
            name: "设备+规则".to_string(),
            query: "请同时列出所有设备和所有自动化规则".to_string(),
            expected_tools: vec!["device_discover".to_string(), "list_rules".to_string()],
            min_tools: 2,
            description: "应该同时调用设备和规则工具".to_string(),
        },
        TestCase {
            name: "设备发现+数据".to_string(),
            query: "发现设备后查看第一个设备的详细数据".to_string(),
            expected_tools: vec!["device_discover".to_string(), "get_device_data".to_string()],
            min_tools: 1,
            description: "应该调用设备发现工具".to_string(),
        },
        TestCase {
            name: "全面查询".to_string(),
            query: "我需要查看：所有设备、所有规则、所有Agent".to_string(),
            expected_tools: vec![
                "device_discover".to_string(),
                "list_rules".to_string(),
                "list_agents".to_string(),
            ],
            min_tools: 2,
            description: "应该调用多个工具".to_string(),
        },
        // === Control Tests ===
        TestCase {
            name: "设备控制".to_string(),
            query: "打开设备 light_001".to_string(),
            expected_tools: vec!["device_control".to_string()],
            min_tools: 1,
            description: "应该调用设备控制工具".to_string(),
        },
        TestCase {
            name: "创建规则".to_string(),
            query: "创建一个温度超过30度时告警的规则".to_string(),
            expected_tools: vec!["create_rule".to_string()],
            min_tools: 1,
            description: "应该调用创建规则工具".to_string(),
        },
        TestCase {
            name: "创建Agent".to_string(),
            query: "创建一个监控温度的Agent".to_string(),
            expected_tools: vec!["create_agent".to_string()],
            min_tools: 1,
            description: "应该调用创建Agent工具".to_string(),
        },
    ]
}

// ============================================================================
// Evaluation Metrics
// ============================================================================

#[derive(Debug, Default)]
struct EvaluationMetrics {
    total_tests: usize,
    tests_with_tool_calls: usize,
    total_expected_tool_calls: usize,
    total_actual_tool_calls: usize,
    correct_tool_calls: usize,
    multi_tool_requests: usize,
    successful_multi_tools: usize,
    empty_responses: usize,
    total_time_ms: u128,
    tool_execution_errors: usize,
}

impl EvaluationMetrics {
    fn precision(&self) -> f64 {
        if self.total_actual_tool_calls == 0 {
            0.0
        } else {
            self.correct_tool_calls as f64 / self.total_actual_tool_calls as f64
        }
    }

    fn recall(&self) -> f64 {
        if self.total_expected_tool_calls == 0 {
            0.0
        } else {
            self.correct_tool_calls as f64 / self.total_expected_tool_calls as f64
        }
    }

    fn f1_score(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }

    fn multi_tool_success_rate(&self) -> f64 {
        if self.multi_tool_requests == 0 {
            0.0
        } else {
            self.successful_multi_tools as f64 / self.multi_tool_requests as f64
        }
    }

    fn tool_call_detection_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            self.tests_with_tool_calls as f64 / self.total_tests as f64
        }
    }

    fn average_time_ms(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            self.total_time_ms as f64 / self.total_tests as f64
        }
    }
}

// ============================================================================
// Test Runner
// ============================================================================

async fn run_test_case(
    session_manager: &SessionManager,
    session_id: &str,
    test_case: &TestCase,
) -> anyhow::Result<TestResult> {
    let start = Instant::now();

    // Use REAL agent with REAL tool execution
    let response = session_manager
        .process_message(session_id, &test_case.query)
        .await?;

    let elapsed = start.elapsed();

    // Check if tools were called
    let tool_calls = &response.tool_calls;
    let called_tool_names: Vec<String> = tool_calls.iter().map(|t| t.name.clone()).collect();

    // Check if expected tools were called (partial match)
    let mut correct_calls = 0;
    for expected in &test_case.expected_tools {
        if called_tool_names.iter().any(|name| name.contains(expected)) {
            correct_calls += 1;
        }
    }

    // Check for tool execution errors
    let tool_errors: Vec<String> = tool_calls
        .iter()
        .filter_map(|t| {
            if let Some(result) = &t.result {
                if result.get("success").and_then(|s| s.as_bool()) == Some(false) {
                    Some(
                        result
                            .get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("Unknown error")
                            .to_string(),
                    )
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    Ok(TestResult {
        test_name: test_case.name.clone(),
        query: test_case.query.clone(),
        response_content: response.message.content.clone(),
        tool_calls: called_tool_names.clone(),
        expected_tools: test_case.expected_tools.clone(),
        min_tools: test_case.min_tools,
        correct_calls,
        total_expected: test_case.expected_tools.len(),
        total_actual: tool_calls.len(),
        elapsed_ms: elapsed.as_millis(),
        has_tool_calls: !tool_calls.is_empty(),
        meets_minimum: tool_calls.len() >= test_case.min_tools,
        is_multi_tool: test_case.min_tools > 1,
        multi_tool_success: test_case.min_tools > 1 && tool_calls.len() >= test_case.min_tools,
        tool_errors,
        tools_used: response.tools_used,
    })
}

#[derive(Debug)]
struct TestResult {
    test_name: String,
    query: String,
    response_content: String,
    tool_calls: Vec<String>,
    expected_tools: Vec<String>,
    min_tools: usize,
    correct_calls: usize,
    total_expected: usize,
    total_actual: usize,
    elapsed_ms: u128,
    has_tool_calls: bool,
    meets_minimum: bool,
    is_multi_tool: bool,
    multi_tool_success: bool,
    tool_errors: Vec<String>,
    tools_used: Vec<String>,
}

// ============================================================================
// Output Formatting
// ============================================================================

fn print_metrics(metrics: &EvaluationMetrics) {
    println!("\n{}", "=".repeat(70));
    println!("📊 REAL TOOL CALLING EVALUATION RESULTS (Updated Tools)");
    println!("{}", "=".repeat(70));

    println!("\n📈 Core Metrics:");
    println!(
        "  Precision (精确度):   {:.1}%",
        metrics.precision() * 100.0
    );
    println!("  Recall (召回率):      {:.1}%", metrics.recall() * 100.0);
    println!("  F1 Score:             {:.1}%", metrics.f1_score() * 100.0);

    println!("\n🔧 Tool Call Detection:");
    println!(
        "  Detection Rate:       {:.1}%",
        metrics.tool_call_detection_rate() * 100.0
    );
    println!(
        "  Total Expected:       {}",
        metrics.total_expected_tool_calls
    );
    println!(
        "  Total Actual:         {}",
        metrics.total_actual_tool_calls
    );
    println!("  Correct Matches:      {}", metrics.correct_tool_calls);
    println!("  Tool Errors:          {}", metrics.tool_execution_errors);

    println!("\n🎯 Multi-Tool Performance:");
    println!("  Multi-Tool Requests:  {}", metrics.multi_tool_requests);
    println!("  Successful:          {}", metrics.successful_multi_tools);
    println!(
        "  Success Rate:         {:.1}%",
        metrics.multi_tool_success_rate() * 100.0
    );

    println!("\n⏱️  Performance:");
    println!("  Average Response:     {:.0}ms", metrics.average_time_ms());
    println!("  Total Time:           {}ms", metrics.total_time_ms);

    println!("\n{}", "=".repeat(70));
}

fn print_detailed_results(results: &[TestResult]) {
    println!("\n{}", "-".repeat(70));
    println!("📋 DETAILED TEST RESULTS");
    println!("{}", "-".repeat(70));

    for (i, result) in results.iter().enumerate() {
        println!(
            "\n[{}] {} - {}ms",
            i + 1,
            result.test_name,
            result.elapsed_ms
        );
        println!("    Query: {}", result.query);
        println!("    Tools called: {:?}", result.tool_calls);
        println!("    Expected patterns: {:?}", result.expected_tools);
        println!(
            "    Correct matches: {}/{}",
            result.correct_calls, result.total_expected
        );

        if !result.tool_errors.is_empty() {
            println!("    ⚠️  Tool errors: {:?}", result.tool_errors);
        }

        if result.has_tool_calls {
            println!("    ✅ Tools executed");
        } else {
            println!("    ❌ No tools called");
        }

        if result.is_multi_tool {
            if result.multi_tool_success {
                println!("    ✅ Multi-tool successful");
            } else {
                println!(
                    "    ⚠️  Multi-tool partial (expected >= {}, got {})",
                    result.min_tools, result.total_actual
                );
            }
        }

        // Response preview
        let preview: String = result.response_content.chars().take(150).collect();
        println!(
            "    Response: {}{}",
            preview,
            if result.response_content.len() > 150 {
                "..."
            } else {
                ""
            }
        );
    }
}

// ============================================================================
// Test
// ============================================================================

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_evaluation_real -- --ignored --nocapture"]
async fn test_real_tool_calling_evaluation() -> anyhow::Result<()> {
    // Check Ollama availability
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 11434));
    if std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_err() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    let model_name = std::env::var("MODEL").unwrap_or_else(|_| "qwen3.5:2b".to_string());

    let ollama_endpoint =
        std::env::var("OLLAMA_ENDPOINT").unwrap_or_else(|_| "http://localhost:11434".to_string());

    println!("\n🚀 Real Tool Calling Evaluation (Updated Tool List)");
    println!("📦 Model: {}", model_name);
    println!("🔗 Endpoint: {}", ollama_endpoint);

    // Create session manager with REAL tools
    let session_manager = SessionManager::memory();
    let session_id = session_manager.create_session().await?;

    // Configure LLM
    let ollama_config = OllamaConfig {
        endpoint: ollama_endpoint,
        model: model_name.clone(),
        timeout_secs: 120,
    };

    let llm_runtime = Arc::new(OllamaRuntime::new(ollama_config.clone())?);
    let agent = session_manager.get_session(&session_id).await?;
    agent.set_custom_llm(llm_runtime).await;

    println!("✅ Session created: {}", session_id);
    println!("🔧 Tools: REAL (device_discover, list_rules, list_agents, etc.)");

    println!("\n📋 Available Tools:");
    println!(
        "  Device: device_discover, get_device_data, query_data, device_control, device_analyze"
    );
    println!("  Rule:   list_rules, create_rule, delete_rule");
    println!("  Agent:  list_agents, get_agent, execute_agent, control_agent, create_agent, etc.");


    let test_cases = get_test_cases();
    println!("\n📋 Test cases: {}", test_cases.len());

    let mut metrics = EvaluationMetrics::default();
    let mut results = Vec::new();

    for test_case in &test_cases {
        metrics.total_tests += 1;
        metrics.total_expected_tool_calls += test_case.expected_tools.len();

        if test_case.min_tools > 1 {
            metrics.multi_tool_requests += 1;
        }

        print!("\n[Testing] {}...", test_case.name);

        // Create a new session for each test to avoid cached results
        let test_session_id = session_manager.create_session().await?;
        let llm_runtime = Arc::new(OllamaRuntime::new(ollama_config.clone())?);
        let agent = session_manager.get_session(&test_session_id).await?;
        agent.set_custom_llm(llm_runtime).await;

        match run_test_case(&session_manager, &test_session_id, test_case).await {
            Ok(result) => {
                println!(
                    " ✓ ({}ms, {} tools)",
                    result.elapsed_ms, result.total_actual
                );

                metrics.total_actual_tool_calls += result.total_actual;
                metrics.correct_tool_calls += result.correct_calls;
                metrics.total_time_ms += result.elapsed_ms;

                if result.has_tool_calls {
                    metrics.tests_with_tool_calls += 1;
                }

                if result.multi_tool_success {
                    metrics.successful_multi_tools += 1;
                }

                if result.response_content.is_empty() {
                    metrics.empty_responses += 1;
                }

                metrics.tool_execution_errors += result.tool_errors.len();

                results.push(result);
            }
            Err(e) => {
                println!(" ✗ Error: {}", e);
            }
        }
    }

    // Print results
    print_detailed_results(&results);
    print_metrics(&metrics);

    // Validation
    println!("\n🎯 Validation:");
    let detection_rate = metrics.tool_call_detection_rate();
    let precision = metrics.precision();
    let multi_tool_rate = metrics.multi_tool_success_rate();

    if detection_rate >= 0.8 {
        println!("  ✅ Detection rate >= 80%: {:.1}%", detection_rate * 100.0);
    } else {
        println!("  ⚠️  Detection rate < 80%: {:.1}%", detection_rate * 100.0);
    }

    if precision >= 0.6 {
        println!("  ✅ Precision >= 60%: {:.1}%", precision * 100.0);
    } else {
        println!("  ⚠️  Precision < 60%: {:.1}%", precision * 100.0);
    }

    if multi_tool_rate >= 0.5 {
        println!(
            "  ✅ Multi-tool rate >= 50%: {:.1}%",
            multi_tool_rate * 100.0
        );
    } else {
        println!(
            "  ⚠️  Multi-tool rate < 50%: {:.1}%",
            multi_tool_rate * 100.0
        );
    }

    if metrics.tool_execution_errors == 0 {
        println!("  ✅ No tool execution errors");
    } else {
        println!(
            "  ⚠️  Tool execution errors: {}",
            metrics.tool_execution_errors
        );
    }

    let overall_score =
        (metrics.precision() + metrics.recall() + metrics.multi_tool_success_rate()) / 3.0;
    println!("\n🏆 Overall Score: {:.1}%", overall_score * 100.0);

    if overall_score >= 0.65 {
        println!("✅ EVALUATION PASSED");
    } else {
        println!("⚠️  EVALUATION NEEDS IMPROVEMENT");
    }

    Ok(())
}
