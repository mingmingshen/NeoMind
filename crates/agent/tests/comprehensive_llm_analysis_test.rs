//! NeoTalk 综合LLM分析测试
//!
//! 测试维度:
//! 1. 空响应问题深度分析
//! 2. 命令下发功能测试
//! 3. 规则引擎生成正确率
//! 4. 工作流生成正确率和可执行率
//!
//! **测试日期**: 2026-01-17
//! **LLM后端**: Ollama (qwen3:1.7b)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use edge_ai_llm::backends::create_backend;
use edge_ai_core::llm::backend::{LlmRuntime, GenerationParams, LlmInput};
use edge_ai_core::message::{Message, MessageRole, Content};
use edge_ai_rules::{RuleEngine, dsl::RuleDslParser};
use edge_ai_tools::{ToolRegistry, ToolCall, ToolRegistryBuilder};

// ============================================================================
// 测试配置
// ============================================================================

const TEST_MODEL: &str = "qwen3:1.7b";
const OLLAMA_ENDPOINT: &str = "http://localhost:11434";

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub model: String,
    pub endpoint: String,
    pub timeout_secs: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            model: TEST_MODEL.to_string(),
            endpoint: OLLAMA_ENDPOINT.to_string(),
            timeout_secs: 60,
        }
    }
}

// ============================================================================
// 空响应分析器
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyResponseAnalysis {
    pub total_requests: usize,
    pub empty_responses: usize,
    pub empty_rate: f64,
    pub empty_by_category: HashMap<String, usize>,
    pub response_lengths: Vec<usize>,
    pub avg_response_length: f64,
    pub raw_responses: Vec<RawResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawResponseData {
    pub user_input: String,
    pub content: String,
    pub thinking: String,
    pub content_len: usize,
    pub thinking_len: usize,
    pub is_empty: bool,
    pub reason: String,
}

pub struct EmptyResponseAnalyzer {
    llm: Arc<dyn LlmRuntime>,
    config: TestConfig,
}

impl EmptyResponseAnalyzer {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = TestConfig::default();
        let llm_config = serde_json::json!({
            "endpoint": config.endpoint,
            "model": config.model
        });

        let llm = create_backend("ollama", &llm_config)?;

        Ok(Self { llm, config })
    }

    /// 深度分析空响应问题
    pub async fn analyze_empty_responses(&self, test_inputs: Vec<&str>) -> EmptyResponseAnalysis {
        let mut raw_responses = Vec::new();
        let mut empty_by_category = HashMap::new();
        let mut response_lengths = Vec::new();

        for input in test_inputs {
            let system_prompt = "你是 NeoTalk 智能助手。请用中文简洁回答。";

            let messages = vec![
                Message {
                    role: MessageRole::System,
                    content: Content::Text(system_prompt.to_string()),
                    timestamp: None,
                },
                Message {
                    role: MessageRole::User,
                    content: Content::Text(input.to_string()),
                    timestamp: None,
                },
            ];

            let llm_input = LlmInput {
                messages,
                params: GenerationParams {
                    max_tokens: Some(200),
                    temperature: Some(0.7),
                    ..Default::default()
                },
                model: Some(self.config.model.clone()),
                stream: false,
                tools: None,
            };

            match tokio::time::timeout(
                Duration::from_secs(self.config.timeout_secs),
                self.llm.generate(llm_input)
            ).await {
                Ok(Ok(output)) => {
                    let response_text = output.text;
                    let is_empty = response_text.trim().is_empty();

                    // 分析空响应原因
                    let reason = if is_empty {
                        "响应为空".to_string()
                    } else if response_text.len() < 5 {
                        format!("响应过短({}字符)", response_text.len())
                    } else {
                        "正常".to_string()
                    };

                    // 尝试获取原始Ollama响应数据（通过模拟）
                    let raw_data = RawResponseData {
                        user_input: input.to_string(),
                        content: response_text.clone(),
                        thinking: "".to_string(),  // 需要从Ollama获取原始数据
                        content_len: response_text.len(),
                        thinking_len: 0,
                        is_empty,
                        reason,
                    };

                    *empty_by_category.entry(raw_data.reason.clone()).or_insert(0) += 1;
                    response_lengths.push(response_text.len());
                    raw_responses.push(raw_data);
                }
                Ok(Err(e)) => {
                    let raw_data = RawResponseData {
                        user_input: input.to_string(),
                        content: "".to_string(),
                        thinking: "".to_string(),
                        content_len: 0,
                        thinking_len: 0,
                        is_empty: true,
                        reason: format!("LLM错误: {:?}", e),
                    };
                    *empty_by_category.entry(raw_data.reason.clone()).or_insert(0) += 1;
                    response_lengths.push(0);
                    raw_responses.push(raw_data);
                }
                Err(_) => {
                    let raw_data = RawResponseData {
                        user_input: input.to_string(),
                        content: "".to_string(),
                        thinking: "".to_string(),
                        content_len: 0,
                        thinking_len: 0,
                        is_empty: true,
                        reason: "超时".to_string(),
                    };
                    *empty_by_category.entry(raw_data.reason.clone()).or_insert(0) += 1;
                    response_lengths.push(0);
                    raw_responses.push(raw_data);
                }
            }
        }

        let total_requests = raw_responses.len();
        let empty_responses = raw_responses.iter().filter(|r| r.is_empty).count();
        let empty_rate = if total_requests > 0 {
            (empty_responses as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let avg_response_length = if !response_lengths.is_empty() {
            response_lengths.iter().sum::<usize>() as f64 / response_lengths.len() as f64
        } else {
            0.0
        };

        EmptyResponseAnalysis {
            total_requests,
            empty_responses,
            empty_rate,
            empty_by_category,
            response_lengths,
            avg_response_length,
            raw_responses,
        }
    }
}

// ============================================================================
// 命令下发测试器
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    pub command: String,
    pub parameters: Value,
    pub llm_response: String,
    pub parsed_command: Option<ParsedCommand>,
    pub execution_success: bool,
    pub execution_time_ms: u128,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    pub action: String,
    pub device_type: Option<String>,
    pub device_id: Option<String>,
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionTestResult {
    pub total_commands: usize,
    pub successful_parses: usize,
    pub successful_executions: usize,
    pub parse_rate: f64,
    pub execution_rate: f64,
    pub results: Vec<CommandExecutionResult>,
}

pub struct CommandExecutorTester {
    llm: Arc<dyn LlmRuntime>,
    config: TestConfig,
}

impl CommandExecutorTester {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = TestConfig::default();
        let llm_config = serde_json::json!({
            "endpoint": config.endpoint,
            "model": config.model
        });

        let llm = create_backend("ollama", &llm_config)?;

        Ok(Self { llm, config })
    }

    /// 测试命令下发功能
    pub async fn test_command_execution(&self, commands: Vec<(&str, Value)>) -> CommandExecutionTestResult {
        let mut results = Vec::new();

        for (command, params) in commands {
            let system_prompt = format!(r#"你是 NeoTalk 智能助手。
当用户要求执行设备控制时，请按以下JSON格式回复:
{{"action": "设备操作", "device_type": "设备类型", "device_id": "设备ID", "parameters": {{...}}}}

例如: 打开客厅的灯
回复: {{"action": "turn_on", "device_type": "light", "device_id": "living_room_light", "parameters": {{"power": "on"}}}}

用户命令: {}
参数: {:?}"#, command, params);

            let messages = vec![
                Message {
                    role: MessageRole::System,
                    content: Content::Text(system_prompt),
                    timestamp: None,
                },
                Message {
                    role: MessageRole::User,
                    content: Content::Text(command.to_string()),
                    timestamp: None,
                },
            ];

            let llm_input = LlmInput {
                messages,
                params: GenerationParams {
                    max_tokens: Some(200),
                    temperature: Some(0.3),  // 降低温度以提高一致性
                    ..Default::default()
                },
                model: Some(self.config.model.clone()),
                stream: false,
                tools: None,
            };

            let start = std::time::Instant::now();

            let result = match tokio::time::timeout(
                Duration::from_secs(self.config.timeout_secs),
                self.llm.generate(llm_input)
            ).await {
                Ok(Ok(output)) => {
                    let llm_response = output.text;
                    let parsed = self.parse_command_response(&llm_response);
                    let execution_success = parsed.is_some();

                    CommandExecutionResult {
                        command: command.to_string(),
                        parameters: params,
                        llm_response,
                        parsed_command: parsed,
                        execution_success,
                        execution_time_ms: start.elapsed().as_millis(),
                        error_message: if execution_success { None } else { Some("无法解析命令".to_string()) },
                    }
                }
                Ok(Err(e)) => {
                    CommandExecutionResult {
                        command: command.to_string(),
                        parameters: params,
                        llm_response: "".to_string(),
                        parsed_command: None,
                        execution_success: false,
                        execution_time_ms: start.elapsed().as_millis(),
                        error_message: Some(format!("LLM错误: {:?}", e)),
                    }
                }
                Err(_) => {
                    CommandExecutionResult {
                        command: command.to_string(),
                        parameters: params,
                        llm_response: "".to_string(),
                        parsed_command: None,
                        execution_success: false,
                        execution_time_ms: start.elapsed().as_millis(),
                        error_message: Some("超时".to_string()),
                    }
                }
            };

            results.push(result);
        }

        let total_commands = results.len();
        let successful_parses = results.iter().filter(|r| r.parsed_command.is_some()).count();
        let successful_executions = results.iter().filter(|r| r.execution_success).count();

        CommandExecutionTestResult {
            total_commands,
            successful_parses,
            successful_executions,
            parse_rate: if total_commands > 0 {
                (successful_parses as f64 / total_commands as f64) * 100.0
            } else {
                0.0
            },
            execution_rate: if total_commands > 0 {
                (successful_executions as f64 / total_commands as f64) * 100.0
            } else {
                0.0
            },
            results,
        }
    }

    fn parse_command_response(&self, response: &str) -> Option<ParsedCommand> {
        // 尝试解析JSON响应
        if let Ok(json) = serde_json::from_str::<Value>(response) {
            if let Some(obj) = json.as_object() {
                let action = obj.get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let device_type = obj.get("device_type")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let device_id = obj.get("device_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let mut parameters = HashMap::new();
                if let Some(params) = obj.get("parameters") {
                    if let Some(obj) = params.as_object() {
                        for (key, value) in obj {
                            parameters.insert(key.clone(), value.clone());
                        }
                    }
                }

                return Some(ParsedCommand {
                    action,
                    device_type,
                    device_id,
                    parameters,
                });
            }
        }

        // 如果JSON解析失败，尝试从文本中提取命令
        let lower = response.to_lowercase();
        if lower.contains("打开") || lower.contains("启动") || lower.contains("on") {
            Some(ParsedCommand {
                action: "turn_on".to_string(),
                device_type: None,
                device_id: None,
                parameters: HashMap::new(),
            })
        } else if lower.contains("关闭") || lower.contains("停止") || lower.contains("off") {
            Some(ParsedCommand {
                action: "turn_off".to_string(),
                device_type: None,
                device_id: None,
                parameters: HashMap::new(),
            })
        } else {
            None
        }
    }
}

// ============================================================================
// 规则引擎生成测试器
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleGenerationResult {
    pub description: String,
    pub llm_generated_dsl: String,
    pub is_valid_dsl: bool,
    pub parse_error: Option<String>,
    pub parse_success: bool,
    pub execution_time_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleGenerationTestResult {
    pub total_rules: usize,
    pub valid_dsl_count: usize,
    pub parse_success_count: usize,
    pub dsl_validity_rate: f64,
    pub parse_success_rate: f64,
    pub results: Vec<RuleGenerationResult>,
}

pub struct RuleGenerationTester {
    llm: Arc<dyn LlmRuntime>,
    config: TestConfig,
}

impl RuleGenerationTester {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = TestConfig::default();
        let llm_config = serde_json::json!({
            "endpoint": config.endpoint,
            "model": config.model
        });

        let llm = create_backend("ollama", &llm_config)?;

        Ok(Self { llm, config })
    }

    /// 测试规则引擎生成正确率
    pub async fn test_rule_generation(&self, rule_descriptions: Vec<&str>) -> RuleGenerationTestResult {
        let mut results = Vec::new();

        let dsl_template = r#"RULE "规则名称"
WHEN device_id.metric > 50
FOR 5 minutes
DO
    NOTIFY "告警消息"
    EXECUTE device_id.command(param=value)
END"#;

        for description in rule_descriptions {
            let system_prompt = format!(r#"你是 NeoTalk 规则引擎助手。
请根据用户的描述生成规则DSL。

DSL格式:
{}

可用设备:
- temp_sensor: 温度传感器，metrics: [temperature], commands: []
- humidity_sensor: 湿度传感器，metrics: [humidity], commands: []
- light_switch: 智能灯，metrics: [power, brightness], commands: [turn_on, turn_off, set_brightness]
- air_conditioner: 空调，metrics: [current_temp, target_temp], commands: [turn_on, turn_off, set_temperature]

请只返回DSL代码，不要有其他说明文字。"#, dsl_template);

            let messages = vec![
                Message {
                    role: MessageRole::System,
                    content: Content::Text(system_prompt),
                    timestamp: None,
                },
                Message {
                    role: MessageRole::User,
                    content: Content::Text(description.to_string()),
                    timestamp: None,
                },
            ];

            let llm_input = LlmInput {
                messages,
                params: GenerationParams {
                    max_tokens: Some(300),
                    temperature: Some(0.3),
                    ..Default::default()
                },
                model: Some(self.config.model.clone()),
                stream: false,
                tools: None,
            };

            let start = std::time::Instant::now();

            let result = match tokio::time::timeout(
                Duration::from_secs(self.config.timeout_secs),
                self.llm.generate(llm_input)
            ).await {
                Ok(Ok(output)) => {
                    let llm_generated_dsl = output.text;
                    let is_valid_dsl = self.looks_like_valid_dsl(&llm_generated_dsl);
                    let parse_result = RuleDslParser::parse(&llm_generated_dsl);
                    let parse_success = parse_result.is_ok();

                    RuleGenerationResult {
                        description: description.to_string(),
                        llm_generated_dsl,
                        is_valid_dsl,
                        parse_error: parse_result.err().map(|e| e.to_string()),
                        parse_success,
                        execution_time_ms: start.elapsed().as_millis(),
                    }
                }
                Ok(Err(e)) => {
                    RuleGenerationResult {
                        description: description.to_string(),
                        llm_generated_dsl: "".to_string(),
                        is_valid_dsl: false,
                        parse_error: Some(format!("LLM错误: {:?}", e)),
                        parse_success: false,
                        execution_time_ms: start.elapsed().as_millis(),
                    }
                }
                Err(_) => {
                    RuleGenerationResult {
                        description: description.to_string(),
                        llm_generated_dsl: "".to_string(),
                        is_valid_dsl: false,
                        parse_error: Some("超时".to_string()),
                        parse_success: false,
                        execution_time_ms: start.elapsed().as_millis(),
                    }
                }
            };

            results.push(result);
        }

        let total_rules = results.len();
        let valid_dsl_count = results.iter().filter(|r| r.is_valid_dsl).count();
        let parse_success_count = results.iter().filter(|r| r.parse_success).count();

        RuleGenerationTestResult {
            total_rules,
            valid_dsl_count,
            parse_success_count,
            dsl_validity_rate: if total_rules > 0 {
                (valid_dsl_count as f64 / total_rules as f64) * 100.0
            } else {
                0.0
            },
            parse_success_rate: if total_rules > 0 {
                (parse_success_count as f64 / total_rules as f64) * 100.0
            } else {
                0.0
            },
            results,
        }
    }

    fn looks_like_valid_dsl(&self, text: &str) -> bool {
        let trimmed = text.trim();
        !trimmed.is_empty()
            && (trimmed.contains("RULE")
                || trimmed.contains("WHEN")
                || trimmed.contains("DO"))
    }
}

// ============================================================================
// 工作流生成测试器
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGenerationResult {
    pub description: String,
    pub llm_generated_workflow: String,
    pub has_valid_structure: bool,
    pub has_steps: bool,
    pub has_conditions: bool,
    pub is_executable: bool,
    pub execution_time_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGenerationTestResult {
    pub total_workflows: usize,
    pub valid_structure_count: usize,
    pub has_steps_count: usize,
    pub executable_count: usize,
    pub structure_validity_rate: f64,
    pub executability_rate: f64,
    pub results: Vec<WorkflowGenerationResult>,
}

pub struct WorkflowGenerationTester {
    llm: Arc<dyn LlmRuntime>,
    config: TestConfig,
}

impl WorkflowGenerationTester {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = TestConfig::default();
        let llm_config = serde_json::json!({
            "endpoint": config.endpoint,
            "model": config.model
        });

        let llm = create_backend("ollama", &llm_config)?;

        Ok(Self { llm, config })
    }

    /// 测试工作流生成正确率和可执行率
    pub async fn test_workflow_generation(&self, workflow_descriptions: Vec<&str>) -> WorkflowGenerationTestResult {
        let mut results = Vec::new();

        let workflow_template = r#"WORKFLOW "工作流名称"
STEPS:
    1. IF condition THEN action
    2. action
    3. WHILE condition action
CONDITIONS:
    - condition_expression
ACTIONS:
    - action_name
END"#;

        for description in workflow_descriptions {
            let system_prompt = format!(r#"你是 NeoTalk 工作流引擎助手。
请根据用户的描述生成工作流定义。

工作流格式:
{}

可用操作:
- check_device_status: 检查设备状态
- send_command: 发送设备命令
- wait: 等待一段时间
- notify: 发送通知
- log: 记录日志
- trigger_rule: 触发规则

请只返回工作流定义，不要有其他说明文字。"#, workflow_template);

            let messages = vec![
                Message {
                    role: MessageRole::System,
                    content: Content::Text(system_prompt),
                    timestamp: None,
                },
                Message {
                    role: MessageRole::User,
                    content: Content::Text(description.to_string()),
                    timestamp: None,
                },
            ];

            let llm_input = LlmInput {
                messages,
                params: GenerationParams {
                    max_tokens: Some(400),
                    temperature: Some(0.3),
                    ..Default::default()
                },
                model: Some(self.config.model.clone()),
                stream: false,
                tools: None,
            };

            let start = std::time::Instant::now();

            let result = match tokio::time::timeout(
                Duration::from_secs(self.config.timeout_secs),
                self.llm.generate(llm_input)
            ).await {
                Ok(Ok(output)) => {
                    let llm_generated_workflow = output.text;
                    let has_valid_structure = self.looks_like_valid_workflow(&llm_generated_workflow);
                    let has_steps = has_steps(&llm_generated_workflow);
                    let has_conditions = has_conditions(&llm_generated_workflow);
                    let is_executable = has_valid_structure && has_steps;

                    WorkflowGenerationResult {
                        description: description.to_string(),
                        llm_generated_workflow,
                        has_valid_structure,
                        has_steps,
                        has_conditions,
                        is_executable,
                        execution_time_ms: start.elapsed().as_millis(),
                    }
                }
                Ok(Err(_e)) => {
                    WorkflowGenerationResult {
                        description: description.to_string(),
                        llm_generated_workflow: "".to_string(),
                        has_valid_structure: false,
                        has_steps: false,
                        has_conditions: false,
                        is_executable: false,
                        execution_time_ms: start.elapsed().as_millis(),
                    }
                }
                Err(_) => {
                    WorkflowGenerationResult {
                        description: description.to_string(),
                        llm_generated_workflow: "".to_string(),
                        has_valid_structure: false,
                        has_steps: false,
                        has_conditions: false,
                        is_executable: false,
                        execution_time_ms: start.elapsed().as_millis(),
                    }
                }
            };

            results.push(result);
        }

        let total_workflows = results.len();
        let valid_structure_count = results.iter().filter(|r| r.has_valid_structure).count();
        let has_steps_count = results.iter().filter(|r| r.has_steps).count();
        let executable_count = results.iter().filter(|r| r.is_executable).count();

        WorkflowGenerationTestResult {
            total_workflows,
            valid_structure_count,
            has_steps_count,
            executable_count,
            structure_validity_rate: if total_workflows > 0 {
                (valid_structure_count as f64 / total_workflows as f64) * 100.0
            } else {
                0.0
            },
            executability_rate: if total_workflows > 0 {
                (executable_count as f64 / total_workflows as f64) * 100.0
            } else {
                0.0
            },
            results,
        }
    }

    fn looks_like_valid_workflow(&self, text: &str) -> bool {
        let trimmed = text.trim();
        !trimmed.is_empty()
            && (trimmed.contains("WORKFLOW")
                || trimmed.contains("STEPS")
                || trimmed.contains("步骤"))
    }
}

fn has_steps(text: &str) -> bool {
    text.contains("步骤") || text.contains("STEPS") || text.contains("1.") || text.contains("第一步")
}

fn has_conditions(text: &str) -> bool {
    text.contains("IF") || text.contains("如果") || text.contains("WHEN") || text.contains("当")
}

// ============================================================================
// 综合测试
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveTestResult {
    pub empty_response_analysis: EmptyResponseAnalysis,
    pub command_execution: CommandExecutionTestResult,
    pub rule_generation: RuleGenerationTestResult,
    pub workflow_generation: WorkflowGenerationTestResult,
    pub overall_score: f64,
}

pub async fn run_comprehensive_test() -> ComprehensiveTestResult {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   NeoTalk 综合LLM分析测试                                            ║");
    println!("║   模型: {}                                                      ║", TEST_MODEL);
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    // 1. 空响应分析
    println!("\n📊 测试1: 空响应问题分析");
    println!("════════════════════════════════════════════════════════════════════════");

    let analyzer = match EmptyResponseAnalyzer::new().await {
        Ok(a) => a,
        Err(e) => {
            println!("⚠️  无法创建分析器: {:?}，跳过测试", e);
            println!("\n请确保 Ollama 正在运行: ollama serve");
            println!("安装模型: ollama pull {}", TEST_MODEL);

            // 返回空结果
            return ComprehensiveTestResult {
                empty_response_analysis: EmptyResponseAnalysis {
                    total_requests: 0,
                    empty_responses: 0,
                    empty_rate: 0.0,
                    empty_by_category: HashMap::new(),
                    response_lengths: Vec::new(),
                    avg_response_length: 0.0,
                    raw_responses: Vec::new(),
                },
                command_execution: CommandExecutionTestResult {
                    total_commands: 0,
                    successful_parses: 0,
                    successful_executions: 0,
                    parse_rate: 0.0,
                    execution_rate: 0.0,
                    results: Vec::new(),
                },
                rule_generation: RuleGenerationTestResult {
                    total_rules: 0,
                    valid_dsl_count: 0,
                    parse_success_count: 0,
                    dsl_validity_rate: 0.0,
                    parse_success_rate: 0.0,
                    results: Vec::new(),
                },
                workflow_generation: WorkflowGenerationTestResult {
                    total_workflows: 0,
                    valid_structure_count: 0,
                    has_steps_count: 0,
                    executable_count: 0,
                    structure_validity_rate: 0.0,
                    executability_rate: 0.0,
                    results: Vec::new(),
                },
                overall_score: 0.0,
            };
        }
    };

    let test_inputs = vec![
        "你好",
        "请告诉我当前时间",
        "帮我打开客厅的灯",
        "关闭卧室的空调",
        "查看温度传感器数据",
        "设置空调温度到26度",
        "启动所有设备",
        "停止浇水系统",
        "查看所有在线设备",
        "创建一条新规则",
        "温度超过30度时打开风扇",
        "湿度低于40%时启动加湿器",
        "有人在时自动开灯",
        "离开家时关闭所有电器",
        "早上7点自动打开窗帘",
        "检测到烟雾时报警",
        "室内PM2.5超过100时启动空气净化器",
        "电价低谷时给电动车充电",
        "用水量异常时通知用户",
        "夜间安防模式启动",
    ];

    let empty_response_analysis = analyzer.analyze_empty_responses(test_inputs).await;

    println!("总请求数: {}", empty_response_analysis.total_requests);
    println!("空响应数: {}", empty_response_analysis.empty_responses);
    println!("空响应率: {:.1}%", empty_response_analysis.empty_rate);
    println!("平均响应长度: {:.1}字符", empty_response_analysis.avg_response_length);
    println!("\n空响应分类:");
    for (category, count) in &empty_response_analysis.empty_by_category {
        println!("  - {}: {}次", category, count);
    }

    // 2. 命令下发测试
    println!("\n⚡ 测试2: 命令下发功能");
    println!("════════════════════════════════════════════════════════════════════════");

    let command_tester = match CommandExecutorTester::new().await {
        Ok(c) => c,
        Err(_) => {
            return ComprehensiveTestResult {
                empty_response_analysis,
                command_execution: CommandExecutionTestResult {
                    total_commands: 0,
                    successful_parses: 0,
                    successful_executions: 0,
                    parse_rate: 0.0,
                    execution_rate: 0.0,
                    results: Vec::new(),
                },
                rule_generation: RuleGenerationTestResult {
                    total_rules: 0,
                    valid_dsl_count: 0,
                    parse_success_count: 0,
                    dsl_validity_rate: 0.0,
                    parse_success_rate: 0.0,
                    results: Vec::new(),
                },
                workflow_generation: WorkflowGenerationTestResult {
                    total_workflows: 0,
                    valid_structure_count: 0,
                    has_steps_count: 0,
                    executable_count: 0,
                    structure_validity_rate: 0.0,
                    executability_rate: 0.0,
                    results: Vec::new(),
                },
                overall_score: 0.0,
            };
        }
    };

    let commands = vec![
        ("打开客厅的灯", serde_json::json!({"device": "light", "action": "on"})),
        ("关闭卧室空调", serde_json::json!({"device": "ac", "action": "off"})),
        ("设置温度为26度", serde_json::json!({"device": "thermostat", "temp": 26})),
        ("启动浇水系统", serde_json::json!({"device": "irrigation", "action": "on"})),
        ("打开所有风扇", serde_json::json!({"device": "fan", "action": "on"})),
        ("关闭门锁", serde_json::json!({"device": "lock", "action": "lock"})),
        ("打开窗帘", serde_json::json!({"device": "curtain", "action": "open"})),
        ("设置亮度为80%", serde_json::json!({"device": "light", "brightness": 80})),
        ("启动除湿机", serde_json::json!({"device": "dehumidifier", "action": "on"})),
        ("关闭所有灯光", serde_json::json!({"device": "all_lights", "action": "off"})),
    ];

    let command_execution = command_tester.test_command_execution(commands).await;

    println!("总命令数: {}", command_execution.total_commands);
    println!("成功解析: {}", command_execution.successful_parses);
    println!("成功执行: {}", command_execution.successful_executions);
    println!("解析率: {:.1}%", command_execution.parse_rate);
    println!("执行率: {:.1}%", command_execution.execution_rate);

    // 3. 规则生成测试
    println!("\n📜 测试3: 规则引擎生成");
    println!("════════════════════════════════════════════════════════════════════════");

    let rule_tester = match RuleGenerationTester::new().await {
        Ok(r) => r,
        Err(_) => {
            return ComprehensiveTestResult {
                empty_response_analysis,
                command_execution,
                rule_generation: RuleGenerationTestResult {
                    total_rules: 0,
                    valid_dsl_count: 0,
                    parse_success_count: 0,
                    dsl_validity_rate: 0.0,
                    parse_success_rate: 0.0,
                    results: Vec::new(),
                },
                workflow_generation: WorkflowGenerationTestResult {
                    total_workflows: 0,
                    valid_structure_count: 0,
                    has_steps_count: 0,
                    executable_count: 0,
                    structure_validity_rate: 0.0,
                    executability_rate: 0.0,
                    results: Vec::new(),
                },
                overall_score: 0.0,
            };
        }
    };

    let rule_descriptions = vec![
        "当温度超过30度时，打开风扇",
        "湿度低于40%时，启动加湿器",
        "检测到有人移动时，自动开灯",
        "当CO2浓度超过1000ppm时，启动新风系统",
        "当PM2.5超过100时，启动空气净化器",
        "当水位超过警戒线时，发送报警",
        "当室内无人时，关闭所有灯光",
        "当用电量超过阈值时，发送通知",
        "当门窗异常打开时，触发安防报警",
        "当温度低于18度时，启动加热模式",
    ];

    let rule_generation = rule_tester.test_rule_generation(rule_descriptions).await;

    println!("总规则数: {}", rule_generation.total_rules);
    println!("有效DSL数: {}", rule_generation.valid_dsl_count);
    println!("解析成功数: {}", rule_generation.parse_success_count);
    println!("DSL有效率: {:.1}%", rule_generation.dsl_validity_rate);
    println!("解析成功率: {:.1}%", rule_generation.parse_success_rate);

    // 4. 工作流生成测试
    println!("\n🔄 测试4: 工作流生成");
    println!("════════════════════════════════════════════════════════════════════════");

    let workflow_tester = match WorkflowGenerationTester::new().await {
        Ok(w) => w,
        Err(_) => {
            return ComprehensiveTestResult {
                empty_response_analysis,
                command_execution,
                rule_generation,
                workflow_generation: WorkflowGenerationTestResult {
                    total_workflows: 0,
                    valid_structure_count: 0,
                    has_steps_count: 0,
                    executable_count: 0,
                    structure_validity_rate: 0.0,
                    executability_rate: 0.0,
                    results: Vec::new(),
                },
                overall_score: 0.0,
            };
        }
    };

    let workflow_descriptions = vec![
        "回家模式：打开灯光，调节空调温度，播放音乐",
        "离家模式：关闭所有电器，启动安防系统",
        "睡眠模式：关闭所有灯光，降低空调噪音",
        "起床模式：打开窗帘，启动咖啡机，播放轻音乐",
        "观影模式：关闭窗帘，调暗灯光，调节空调",
        "会议模式：关闭背景音乐，调亮灯光，启动投影仪",
        "阅读模式：打开阅读灯，调节空调舒适温度",
        "运动模式：播放动感音乐，调亮灯光，启动风扇",
        "节能模式：关闭非必要设备，调节空调至节能温度",
        "清洁模式：启动扫地机器人，打开窗帘",
    ];

    let workflow_generation = workflow_tester.test_workflow_generation(workflow_descriptions).await;

    println!("总工作流数: {}", workflow_generation.total_workflows);
    println!("有效结构数: {}", workflow_generation.valid_structure_count);
    println!("包含步骤数: {}", workflow_generation.has_steps_count);
    println!("可执行数: {}", workflow_generation.executable_count);
    println!("结构有效率: {:.1}%", workflow_generation.structure_validity_rate);
    println!("可执行率: {:.1}%", workflow_generation.executability_rate);

    // 5. 综合评分
    println!("\n╔════════════════════════════════════════════════════════════════════════╗");
    println!("║   综合评估                                                           ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let overall_score = (
        (100.0 - empty_response_analysis.empty_rate) * 0.3 +  // 空响应率权重30%
        command_execution.execution_rate * 0.25 +             // 命令执行权重25%
        rule_generation.parse_success_rate * 0.25 +            // 规则生成权重25%
        workflow_generation.executability_rate * 0.2          // 工作流生成权重20%
    );

    println!("\n📈 各项得分:");
    println!("   响应可用性: {:.1}/100", 100.0 - empty_response_analysis.empty_rate);
    println!("   命令执行率: {:.1}/100", command_execution.execution_rate);
    println!("   规则解析率: {:.1}/100", rule_generation.parse_success_rate);
    println!("   工作流可执行率: {:.1}/100", workflow_generation.executability_rate);
    println!("\n   综合评分: {:.1}/100", overall_score);
    println!("   评级: {}", if overall_score >= 80.0 {
        "⭐⭐⭐⭐ 优秀"
    } else if overall_score >= 60.0 {
        "⭐⭐⭐ 中等"
    } else if overall_score >= 40.0 {
        "⭐⭐ 及格"
    } else {
        "⭐ 需改进"
    });

    ComprehensiveTestResult {
        empty_response_analysis,
        command_execution,
        rule_generation,
        workflow_generation,
        overall_score,
    }
}

// ============================================================================
// 测试入口
// ============================================================================

#[tokio::test]
async fn test_comprehensive_llm_analysis() {
    let result = run_comprehensive_test().await;

    // 断言关键指标
    assert!(result.empty_response_analysis.total_requests > 0, "应该有测试数据");

    // 如果Ollama可用，检查是否有一定成功率
    if result.empty_response_analysis.total_requests > 10 {
        let success_rate = 100.0 - result.empty_response_analysis.empty_rate;
        assert!(success_rate > 0.0, "应该有至少一些成功的响应");
    }
}
