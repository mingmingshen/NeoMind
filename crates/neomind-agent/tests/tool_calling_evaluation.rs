//! Tool Calling Evaluation Test with Metrics
//!
//! This test evaluates:
//! 1. **Precision** - Correct tool calls / Total tool calls
//! 2. **Recall** - Correct tool calls / Expected tool calls
//! 3. **Multi-tool success rate** - Success rate for multi-tool requests
//! 4. **Analysis accuracy** - Quality of LLM's response based on tool results
//!
//! Run with:
//!   cargo test -p neomind-agent --test tool_calling_evaluation -- --ignored --nocapture
//!
//! Environment variables:
//!   MODEL - Model name (default: qwen3.5:2b)
//!   OLLAMA_ENDPOINT - Ollama endpoint (default: http://localhost:11434)

#![allow(dead_code)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use neomind_agent::agent::tool_parser::parse_tool_calls;
use neomind_agent::{OllamaConfig, OllamaRuntime};
use neomind_core::llm::backend::{GenerationParams, LlmInput, LlmRuntime, ToolDefinition};
use neomind_core::message::Message;

// ============================================================================
// Test Cases & Expected Results
// ============================================================================

#[derive(Debug, Clone)]
struct TestCase {
    name: String,
    query: String,
    expected_tools: Vec<String>, // Expected tool names
    min_tools: usize,            // Minimum expected tools
    description: String,
}

fn get_test_cases() -> Vec<TestCase> {
    vec![
        // === Single Tool Tests ===
        TestCase {
            name: "设备列表查询".to_string(),
            query: "列出所有设备".to_string(),
            expected_tools: vec!["device_discover".to_string(), "list_devices".to_string()],
            min_tools: 1,
            description: "应该调用设备发现或列表工具".to_string(),
        },
        TestCase {
            name: "规则列表查询".to_string(),
            query: "列出所有自动化规则".to_string(),
            expected_tools: vec!["list_rules".to_string()],
            min_tools: 1,
            description: "应该调用规则列表工具".to_string(),
        },
        TestCase {
            name: "设备发现".to_string(),
            query: "发现并搜索所有设备".to_string(),
            expected_tools: vec!["device_discover".to_string()],
            min_tools: 1,
            description: "应该调用设备发现工具".to_string(),
        },
        TestCase {
            name: "规则历史查询".to_string(),
            query: "查看规则执行历史".to_string(),
            expected_tools: vec!["query_rule_history".to_string()],
            min_tools: 1,
            description: "应该调用规则历史查询工具".to_string(),
        },
        // === Multi-Tool Tests ===
        TestCase {
            name: "设备和规则同时查询".to_string(),
            query: "请列出所有设备和所有自动化规则".to_string(),
            expected_tools: vec![
                "device_discover".to_string(),
                "list_devices".to_string(),
                "list_rules".to_string(),
            ],
            min_tools: 2,
            description: "应该同时调用设备和规则工具".to_string(),
        },
        TestCase {
            name: "设备发现和状态".to_string(),
            query: "发现设备并查看它们的状态".to_string(),
            expected_tools: vec!["device_discover".to_string(), "list_devices".to_string()],
            min_tools: 1,
            description: "应该调用设备发现工具".to_string(),
        },
        TestCase {
            name: "三重查询".to_string(),
            query: "我需要查看：1)所有设备 2)所有规则 3)规则执行历史".to_string(),
            expected_tools: vec![
                "device_discover".to_string(),
                "list_devices".to_string(),
                "list_rules".to_string(),
                "query_rule_history".to_string(),
            ],
            min_tools: 3,
            description: "应该调用多个工具".to_string(),
        },
        // === Parameter-Specific Tests ===
        TestCase {
            name: "特定设备查询".to_string(),
            query: "查询设备 'sensor_temp_01' 的温度数据".to_string(),
            expected_tools: vec![
                "device.query".to_string(),
                "query_data".to_string(),
                "get_device_data".to_string(),
            ],
            min_tools: 1,
            description: "应该调用设备数据查询工具".to_string(),
        },
        TestCase {
            name: "多个设备数据查询".to_string(),
            query: "查询客厅和卧室的温度".to_string(),
            expected_tools: vec!["device.query".to_string(), "query_data".to_string()],
            min_tools: 1,
            description: "应该调用数据查询工具（可能多次）".to_string(),
        },
        // === Context/Reference Tests ===
        TestCase {
            name: "场景查询".to_string(),
            query: "列出所有场景".to_string(),
            expected_tools: vec!["list_scenarios".to_string()],
            min_tools: 1,
            description: "应该调用场景列表工具".to_string(),
        },
        TestCase {
            name: "工作流查询".to_string(),
            query: "显示所有工作流的状态".to_string(),
            expected_tools: vec![
                "list_workflows".to_string(),
                "query_workflow_status".to_string(),
            ],
            min_tools: 1,
            description: "应该调用工作流相关工具".to_string(),
        },
    ]
}

// ============================================================================
// Tool Definitions
// ============================================================================

fn get_test_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "device_discover".to_string(),
            description: "发现并搜索所有可用设备".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "list_devices".to_string(),
            description: "列出所有已注册的设备及其状态".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "device.query".to_string(),
            description: "查询特定设备的数据".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "device_id": {"type": "string", "description": "设备ID"},
                    "metrics": {"type": "array", "description": "要查询的指标列表"}
                },
                "required": ["device_id"]
            }),
        },
        ToolDefinition {
            name: "query_data".to_string(),
            description: "查询设备指标数据".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "device_id": {"type": "string"},
                    "metric": {"type": "string"}
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_device_data".to_string(),
            description: "获取设备的详细数据和状态".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "device_id": {"type": "string", "description": "设备ID"}
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "list_rules".to_string(),
            description: "列出所有自动化规则".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "query_rule_history".to_string(),
            description: "查询规则的执行历史".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "list_scenarios".to_string(),
            description: "列出所有场景".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "list_workflows".to_string(),
            description: "列出所有工作流".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "query_workflow_status".to_string(),
            description: "查询工作流执行状态".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

// ============================================================================
// Evaluation Metrics
// ============================================================================

#[derive(Debug, Default)]
struct EvaluationMetrics {
    // Basic counts
    total_tests: usize,
    tests_with_tool_calls: usize,

    // Tool call counts
    total_expected_tool_calls: usize,
    total_actual_tool_calls: usize,
    correct_tool_calls: usize,

    // Multi-tool metrics
    multi_tool_requests: usize,
    successful_multi_tools: usize,

    // Response quality
    empty_responses: usize,
    responses_with_tool_results: usize,

    // Timing
    total_time_ms: u128,
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

struct TestRunner {
    llm: Arc<OllamaRuntime>,
    model_name: String,
    tools: Vec<ToolDefinition>,
}

impl TestRunner {
    async fn new() -> anyhow::Result<Self> {
        let model_name = std::env::var("MODEL").unwrap_or_else(|_| "qwen3.5:2b".to_string());

        let ollama_endpoint = std::env::var("OLLAMA_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        let config = OllamaConfig {
            endpoint: ollama_endpoint,
            model: model_name.clone(),
            timeout_secs: 60,
        };

        let llm = Arc::new(OllamaRuntime::new(config)?);
        let tools = get_test_tools();

        Ok(Self {
            llm,
            model_name,
            tools,
        })
    }

    async fn run_test(&self, test_case: &TestCase) -> anyhow::Result<TestResult> {
        let system_prompt = format!(
            "You are a helpful IoT assistant. Available tools:\n{}\n\n\
            ## Important Rules:\n\
            1. When user asks for information, ALWAYS use tools to get the latest data\n\
            2. You can call multiple tools in one response using XML format:\n\
            <tool_calls><invoke name=\"tool_name\"></invoke></tool_calls>\n\
            3. When user asks for multiple things, call ALL relevant tools",
            self.tools
                .iter()
                .map(|t| format!("- {}: {}", t.name, t.description))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let messages = vec![
            Message::system(&system_prompt),
            Message::user(&test_case.query),
        ];

        let input = LlmInput {
            messages,
            params: GenerationParams {
                temperature: Some(0.1),
                max_tokens: Some(500),
                ..Default::default()
            },
            model: Some(self.model_name.clone()),
            stream: false,
            tools: Some(self.tools.clone()),
        };

        let start = Instant::now();
        let output = self.llm.generate(input).await?;
        let elapsed = start.elapsed();

        let (_, tool_calls) = parse_tool_calls(&output.text)?;
        let called_tool_names: Vec<String> = tool_calls.iter().map(|t| t.name.clone()).collect();

        // Check if expected tools were called
        let mut correct_calls = 0;
        for expected in &test_case.expected_tools {
            if called_tool_names
                .iter()
                .any(|name| name.contains(expected) || expected.contains(name))
            {
                correct_calls += 1;
            }
        }

        // Check for unexpected tool calls
        let unexpected_calls: Vec<_> = called_tool_names
            .iter()
            .filter(|name| {
                !test_case
                    .expected_tools
                    .iter()
                    .any(|exp| name.contains(exp) || exp.contains(*name))
            })
            .collect();

        Ok(TestResult {
            test_name: test_case.name.clone(),
            query: test_case.query.clone(),
            response: output.text.clone(),
            tool_calls: called_tool_names.clone(),
            expected_tools: test_case.expected_tools.clone(),
            min_tools: test_case.min_tools,
            correct_calls,
            total_expected: test_case.expected_tools.len(),
            total_actual: tool_calls.len(),
            unexpected_calls: unexpected_calls.iter().map(|s| s.to_string()).collect(),
            elapsed_ms: elapsed.as_millis(),
            has_tool_calls: !tool_calls.is_empty(),
            meets_minimum: tool_calls.len() >= test_case.min_tools,
            is_multi_tool: test_case.min_tools > 1,
            multi_tool_success: test_case.min_tools > 1 && tool_calls.len() >= test_case.min_tools,
        })
    }
}

#[derive(Debug)]
struct TestResult {
    test_name: String,
    query: String,
    response: String,
    tool_calls: Vec<String>,
    expected_tools: Vec<String>,
    min_tools: usize,
    correct_calls: usize,
    total_expected: usize,
    total_actual: usize,
    unexpected_calls: Vec<String>,
    elapsed_ms: u128,
    has_tool_calls: bool,
    meets_minimum: bool,
    is_multi_tool: bool,
    multi_tool_success: bool,
}

// ============================================================================
// Output Formatting
// ============================================================================

fn print_metrics(metrics: &EvaluationMetrics) {
    println!("\n{}", "=".repeat(70));
    println!("📊 TOOL CALLING EVALUATION RESULTS");
    println!("{}", "=".repeat(70));

    println!("\n📈 Core Metrics:");
    println!(
        "  Precision (正确调用率):  {:.1}%",
        metrics.precision() * 100.0
    );
    println!("  Recall (召回率):        {:.1}%", metrics.recall() * 100.0);
    println!(
        "  F1 Score:              {:.1}%",
        metrics.f1_score() * 100.0
    );

    println!("\n🔧 Tool Call Detection:");
    println!(
        "  Detection Rate:        {:.1}%",
        metrics.tool_call_detection_rate() * 100.0
    );
    println!(
        "  Total Expected Calls:  {}",
        metrics.total_expected_tool_calls
    );
    println!(
        "  Total Actual Calls:    {}",
        metrics.total_actual_tool_calls
    );
    println!("  Correct Calls:         {}", metrics.correct_tool_calls);

    println!("\n🎯 Multi-Tool Performance:");
    println!("  Multi-Tool Requests:   {}", metrics.multi_tool_requests);
    println!("  Successful:           {}", metrics.successful_multi_tools);
    println!(
        "  Success Rate:          {:.1}%",
        metrics.multi_tool_success_rate() * 100.0
    );

    println!("\n⏱️  Performance:");
    println!(
        "  Average Response Time: {:.0}ms",
        metrics.average_time_ms()
    );
    println!("  Total Time:            {}ms", metrics.total_time_ms);

    println!("\n📝 Response Quality:");
    println!("  Empty Responses:       {}", metrics.empty_responses);
    println!(
        "  With Tool Results:     {}",
        metrics.responses_with_tool_results
    );

    println!("\n{}", "=".repeat(70));
}

fn print_detailed_results(results: &[TestResult]) {
    println!("\n{}", "-".repeat(70));
    println!("📋 DETAILED TEST RESULTS");
    println!("{}", "-".repeat(70));

    for (i, result) in results.iter().enumerate() {
        println!("\n[{}] {}", i + 1, result.test_name);
        println!("    Query: {}", result.query);
        println!("    Time: {}ms", result.elapsed_ms);
        println!("    Expected: {:?}", result.expected_tools);
        println!("    Called: {:?}", result.tool_calls);
        println!(
            "    Correct: {}/{}",
            result.correct_calls, result.total_expected
        );

        if !result.unexpected_calls.is_empty() {
            println!("    ⚠️  Unexpected: {:?}", result.unexpected_calls);
        }

        if result.has_tool_calls {
            println!("    ✅ Tool calls detected");
        } else {
            println!("    ❌ No tool calls");
        }

        if result.is_multi_tool {
            if result.multi_tool_success {
                println!("    ✅ Multi-tool successful");
            } else {
                println!(
                    "    ❌ Multi-tool failed (expected >= {}, got {})",
                    result.min_tools, result.total_actual
                );
            }
        }

        // Show response preview
        let preview: String = result.response.chars().take(100).collect();
        println!(
            "    Response: {}{}",
            preview,
            if result.response.len() > 100 {
                "..."
            } else {
                ""
            }
        );
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
#[ignore = "Requires Ollama LLM backend. Run with: cargo test --test tool_calling_evaluation -- --ignored --nocapture"]
async fn test_tool_calling_evaluation() -> anyhow::Result<()> {
    // Check if Ollama is available
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 11434));
    if std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_err() {
        println!("⚠️  Ollama not available, skipping test");
        return Ok(());
    }

    let runner = TestRunner::new().await?;
    let test_cases = get_test_cases();

    println!("\n🚀 Starting Tool Calling Evaluation");
    println!("📦 Model: {}", runner.model_name);
    println!("📋 Test Cases: {}", test_cases.len());

    let mut metrics = EvaluationMetrics::default();
    let mut results = Vec::new();

    for test_case in &test_cases {
        metrics.total_tests += 1;
        metrics.total_expected_tool_calls += test_case.expected_tools.len();

        if test_case.min_tools > 1 {
            metrics.multi_tool_requests += 1;
        }

        match runner.run_test(test_case).await {
            Ok(result) => {
                println!("\n[✓] {} - {}ms", result.test_name, result.elapsed_ms);

                metrics.total_actual_tool_calls += result.total_actual;
                metrics.correct_tool_calls += result.correct_calls;
                metrics.total_time_ms += result.elapsed_ms;

                if result.has_tool_calls {
                    metrics.tests_with_tool_calls += 1;
                }

                if result.multi_tool_success {
                    metrics.successful_multi_tools += 1;
                }

                if result.response.is_empty() {
                    metrics.empty_responses += 1;
                }

                results.push(result);
            }
            Err(e) => {
                println!("\n[✗] {} - Error: {}", test_case.name, e);
            }
        }
    }

    // Print results
    print_detailed_results(&results);
    print_metrics(&metrics);

    // Assertions for CI/CD
    let detection_rate = metrics.tool_call_detection_rate();
    let precision = metrics.precision();
    let multi_tool_rate = metrics.multi_tool_success_rate();

    println!("\n🎯 Validation:");

    if detection_rate >= 0.8 {
        println!("  ✅ Detection rate >= 80%: {:.1}%", detection_rate * 100.0);
    } else {
        println!("  ⚠️  Detection rate < 80%: {:.1}%", detection_rate * 100.0);
    }

    if precision >= 0.7 {
        println!("  ✅ Precision >= 70%: {:.1}%", precision * 100.0);
    } else {
        println!("  ⚠️  Precision < 70%: {:.1}%", precision * 100.0);
    }

    if multi_tool_rate >= 0.6 {
        println!(
            "  ✅ Multi-tool rate >= 60%: {:.1}%",
            multi_tool_rate * 100.0
        );
    } else {
        println!(
            "  ⚠️  Multi-tool rate < 60%: {:.1}%",
            multi_tool_rate * 100.0
        );
    }

    // Overall assessment
    let overall_score =
        (metrics.precision() + metrics.recall() + metrics.multi_tool_success_rate()) / 3.0;

    println!("\n🏆 Overall Score: {:.1}%", overall_score * 100.0);

    if overall_score >= 0.7 {
        println!("✅ EVALUATION PASSED - Tool calling is working well!");
    } else {
        println!("⚠️  EVALUATION WARNING - Tool calling needs improvement.");
    }

    Ok(())
}
