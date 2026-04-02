//! Simplified tool wrappers with smart defaults and user-friendly responses.
//!
//! This module provides simplified versions of tools that:
//! 1. Have minimal required parameters
//! 2. Use smart defaults for common use cases
//! 3. Return user-friendly error messages
//! 4. Support natural language parameter names
//!
//! NOTE: Tool names here must match the actual tool names registered in the tool registry.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::tool::ToolOutput;

/// User-friendly error response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendlyError {
    /// User-facing error message in simple language
    pub message: String,
    /// Suggested fixes the user can try
    pub suggestions: Vec<String>,
    /// Whether this is a critical error or just a warning
    pub is_warning: bool,
}

impl FriendlyError {
    /// Create a new friendly error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            suggestions: Vec::new(),
            is_warning: false,
        }
    }

    /// Add a suggestion to the error.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    /// Set as warning instead of error.
    pub fn as_warning(mut self) -> Self {
        self.is_warning = true;
        self
    }

    /// Convert to ToolOutput.
    pub fn to_output(&self) -> ToolOutput {
        if self.is_warning {
            ToolOutput::warning_with_metadata(
                &self.message,
                serde_json::json!({
                    "suggestions": self.suggestions,
                    "type": "friendly_warning"
                }),
            )
        } else {
            ToolOutput::error_with_metadata(
                &self.message,
                serde_json::json!({
                    "suggestions": self.suggestions,
                    "type": "friendly_error"
                }),
            )
        }
    }
}

/// Common friendly error messages.
pub struct ErrorMessages;

impl ErrorMessages {
    /// Device not found error with helpful suggestions.
    pub fn device_not_found(device_id: &str) -> FriendlyError {
        FriendlyError::new(format!("找不到设备 '{}'", device_id))
            .with_suggestion("使用 'device_discover' 查看所有可用设备".to_string())
            .with_suggestion("检查设备ID是否正确".to_string())
            .with_suggestion(format!("可能是设备 '{}' 尚未添加到系统", device_id))
    }

    /// Parameter missing error with usage hint.
    pub fn parameter_missing(param_name: &str, usage_hint: &str) -> FriendlyError {
        FriendlyError::new(format!("缺少必要参数: {}", param_name))
            .with_suggestion(format!("用法: {}", usage_hint))
    }

    /// No data available error.
    pub fn no_data_available(device_id: &str) -> FriendlyError {
        FriendlyError::new(format!("设备 '{}' 暂无数据", device_id))
            .with_suggestion("设备可能刚添加，需要等待数据采集".to_string())
            .with_suggestion("检查设备是否在线".to_string())
            .as_warning()
    }

    /// Device offline error.
    pub fn device_offline(device_id: &str) -> FriendlyError {
        FriendlyError::new(format!("设备 '{}' 当前离线", device_id))
            .with_suggestion("检查设备电源连接".to_string())
            .with_suggestion("查看网络连接状态".to_string())
    }

    /// Rule not found error.
    pub fn rule_not_found(rule_name: &str) -> FriendlyError {
        FriendlyError::new(format!("找不到规则 '{}'", rule_name))
            .with_suggestion("使用 'list_rules' 查看所有可用规则".to_string())
            .with_suggestion("规则名称可能输入错误".to_string())
    }

    /// Invalid command error with valid commands.
    pub fn invalid_command(device_id: &str, valid_commands: &[String]) -> FriendlyError {
        FriendlyError::new(format!("设备 '{}' 不支持此命令", device_id))
            .with_suggestion(format!("支持的命令: {}", valid_commands.join(", ")))
    }

    /// Permission denied error.
    pub fn permission_denied(resource: &str) -> FriendlyError {
        FriendlyError::new(format!("没有权限访问: {}", resource))
            .with_suggestion("请联系管理员获取权限".to_string())
    }

    /// General error with context.
    pub fn general(context: &str, details: &str) -> FriendlyError {
        FriendlyError::new(format!("{}: {}", context, details))
            .with_suggestion("请稍后重试".to_string())
            .with_suggestion("如果问题持续，请联系技术支持".to_string())
    }
}

/// Simplified tool configuration.
///
/// This configures how a tool should be simplified for LLM use.
#[derive(Debug, Clone)]
pub struct SimplifiedConfig {
    /// Tool name (used by LLM)
    pub name: String,
    /// Natural language description
    pub description: String,
    /// Natural language aliases for this tool
    pub aliases: Vec<String>,
    /// Required parameters with natural language names
    pub required_params: Vec<String>,
    /// Optional parameters with smart defaults
    pub optional_params: HashMap<String, Value>,
    /// Example calls in natural language
    pub examples: Vec<String>,
}

impl SimplifiedConfig {
    /// Create a new simplified config.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            aliases: Vec::new(),
            required_params: Vec::new(),
            optional_params: HashMap::new(),
            examples: Vec::new(),
        }
    }

    /// Set description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add an alias.
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    /// Add a required parameter.
    pub fn with_required_param(mut self, param: impl Into<String>) -> Self {
        self.required_params.push(param.into());
        self
    }

    /// Add an optional parameter with default value.
    pub fn with_optional_param(mut self, param: impl Into<String>, default: Value) -> Self {
        self.optional_params.insert(param.into(), default);
        self
    }

    /// Add an example.
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }
}

/// Tool definition optimized for LLM consumption.
///
/// This is a simplified version of the tool schema that:
/// 1. Uses natural language parameter names
/// 2. Provides clear examples
/// 3. Includes common use cases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolDefinition {
    /// Tool name (short, memorable) - MUST match actual tool name in registry
    pub name: String,
    /// Natural language description
    pub description: String,
    /// Natural language aliases
    pub aliases: Vec<String>,
    /// Required parameters (simplified names)
    pub required: Vec<String>,
    /// Optional parameters with defaults
    pub optional: HashMap<String, ParameterInfo>,
    /// Usage examples
    pub examples: Vec<Example>,
    /// When to use this tool
    pub use_when: Vec<String>,
}

/// Parameter information for LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    /// Natural language description
    pub description: String,
    /// Default value
    pub default: Value,
    /// Example values
    pub examples: Vec<String>,
}

/// Usage example for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    /// User query that triggers this tool
    pub user_query: String,
    /// Expected tool call (simplified format)
    pub tool_call: String,
    /// Description of what this does
    pub explanation: String,
}

/// Generate simplified tool definitions for common tools.
///
/// IMPORTANT: Tool names MUST match the actual tool names registered in the tool registry.
/// See real.rs, agent_tools.rs, and system_tools.rs for actual tool names.
///
/// DESIGN PRINCIPLES (基于 Anthropic 最佳实践):
/// - 更少、更聚焦的工具，而非大量细粒度工具
/// - 合并功能相似的工具，减少 LLM 选择负担
/// - 优先考虑高价值、高使用频率的工具
/// - 每个工具应该是"不可删减"的
///
/// 工具清单 (5个聚合工具，替代原来的34+个独立工具):
/// - device: list, get, query, control
/// - agent: list, get, create, update, control, memory
/// - agent_history: executions, conversation
/// - rule: list, get, delete, history
/// - alert: list, create, acknowledge
pub fn get_simplified_tools() -> Vec<LlmToolDefinition> {
    vec![
        // === 工具调用流程说明 ===
        //
        // **聚合工具设计原则**:
        //   - 所有工具使用 action 参数区分具体操作
        //   - 大幅减少工具定义的 token 消耗 (~60%)
        //   - 输出格式与原工具保持兼容
        //
        // **推荐调用方式**:
        //   场景A - 用户询问设备:
        //     device(action="list") → 列出所有设备
        //     device(action="get", device_id="xxx") → 获取设备详情
        //     device(action="query", device_id="xxx", metric="xxx") → 查询数据
        //     device(action="control", device_id="xxx", command="xxx") → 控制设备
        //
        //   场景B - 用户管理Agent:
        //     agent(action="list") → 列出所有Agent
        //     agent(action="get", agent_id="xxx") → 获取Agent详情
        //     agent(action="create", name="xxx", user_prompt="xxx") → 创建Agent
        //
        //   场景C - 用户管理规则:
        //     rule(action="list") → 列出所有规则
        //     rule(action="get", rule_id="xxx") → 获取规则详情
        //     rule(action="delete", rule_id="xxx") → 删除规则
        //
        // ================================

        // === Device Tool (聚合4个设备操作) ===
        LlmToolDefinition {
            name: "device".to_string(),
            description: "设备管理工具。action: list(列出设备)|get(获取详情)|query(查询数据)|control(控制设备)".to_string(),
            aliases: vec!["设备".to_string(), "device_discover".to_string(), "get_device_data".to_string(), "query_data".to_string(), "device_control".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("device_id".to_string(), ParameterInfo {
                    description: "设备ID (get/query/control需要)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["ne101".to_string(), "sensor_1".to_string()],
                }),
                ("metric".to_string(), ParameterInfo {
                    description: "指标名称 (query需要，如values.battery)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["values.battery".to_string(), "temperature".to_string()],
                }),
                ("command".to_string(), ParameterInfo {
                    description: "控制命令 (control需要: turn_on/turn_off/set_value)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["turn_on".to_string(), "turn_off".to_string()],
                }),
                ("params".to_string(), ParameterInfo {
                    description: "控制参数 (control可选)".to_string(),
                    default: serde_json::json!({}),
                    examples: vec!["{\"value\": 26}".to_string()],
                }),
                ("start_time".to_string(), ParameterInfo {
                    description: "起始时间戳 (query可选)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["今天0点".to_string()],
                }),
                ("end_time".to_string(), ParameterInfo {
                    description: "结束时间戳 (query可选)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["现在".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "有哪些设备？".to_string(),
                    tool_call: "device(action=\"list\")".to_string(),
                    explanation: "列出所有设备".to_string(),
                },
                Example {
                    user_query: "ne101的电量？".to_string(),
                    tool_call: "device(action=\"get\", device_id=\"ne101\")".to_string(),
                    explanation: "获取设备当前所有指标".to_string(),
                },
                Example {
                    user_query: "今天的电池趋势".to_string(),
                    tool_call: "device(action=\"query\", device_id=\"ne101\", metric=\"values.battery\")".to_string(),
                    explanation: "查询历史数据".to_string(),
                },
                Example {
                    user_query: "打开客厅灯".to_string(),
                    tool_call: "device(action=\"control\", device_id=\"light_living\", command=\"turn_on\")".to_string(),
                    explanation: "控制设备".to_string(),
                },
            ],
            use_when: vec!["用户询问设备".to_string(), "用户要控制设备".to_string()],
        },

        // === Agent Tool (聚合6个Agent操作) ===
        LlmToolDefinition {
            name: "agent".to_string(),
            description: "智能体管理工具。action: list|get|create|update|control|memory".to_string(),
            aliases: vec!["智能体".to_string(), "list_agents".to_string(), "get_agent".to_string(), "create_agent".to_string(), "execute_agent".to_string(), "control_agent".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("agent_id".to_string(), ParameterInfo {
                    description: "智能体ID (get/update/control/memory需要)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["agent_1".to_string()],
                }),
                ("name".to_string(), ParameterInfo {
                    description: "智能体名称 (create/update需要)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["温度监控".to_string()],
                }),
                ("description".to_string(), ParameterInfo {
                    description: "智能体描述 (create可选)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["监控温度变化".to_string()],
                }),
                ("user_prompt".to_string(), ParameterInfo {
                    description: "用户需求描述 (create需要)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["每5分钟检查一次温度".to_string()],
                }),
                ("control_action".to_string(), ParameterInfo {
                    description: "控制动作 (control需要: pause/resume/start/stop)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["pause".to_string(), "resume".to_string()],
                }),
                ("memory_type".to_string(), ParameterInfo {
                    description: "记忆类型 (memory可选: patterns/intents)".to_string(),
                    default: serde_json::json!("patterns"),
                    examples: vec!["patterns".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "有哪些智能体？".to_string(),
                    tool_call: "agent(action=\"list\")".to_string(),
                    explanation: "列出所有Agent".to_string(),
                },
                Example {
                    user_query: "创建一个温度监控Agent".to_string(),
                    tool_call: "agent(action=\"create\", name=\"温度监控\", user_prompt=\"每5分钟检查一次ne101的温度，超过30度告警\")".to_string(),
                    explanation: "创建Agent".to_string(),
                },
                Example {
                    user_query: "暂停温度监控".to_string(),
                    tool_call: "agent(action=\"control\", agent_id=\"agent_1\", control_action=\"pause\")".to_string(),
                    explanation: "控制Agent".to_string(),
                },
            ],
            use_when: vec!["用户询问Agent".to_string(), "用户要创建/控制Agent".to_string()],
        },

        // === Agent History Tool ===
        LlmToolDefinition {
            name: "agent_history".to_string(),
            description: "智能体历史工具。action: executions(执行统计)|conversation(对话记录)".to_string(),
            aliases: vec!["执行历史".to_string(), "get_agent_executions".to_string(), "get_agent_conversation".to_string()],
            required: vec!["action".to_string(), "agent_id".to_string()],
            optional: HashMap::from_iter(vec![
                ("limit".to_string(), ParameterInfo {
                    description: "返回数量限制".to_string(),
                    default: serde_json::json!(10),
                    examples: vec!["20".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "温度监控Agent的执行情况".to_string(),
                    tool_call: "agent_history(action=\"executions\", agent_id=\"agent_1\")".to_string(),
                    explanation: "查看执行统计".to_string(),
                },
            ],
            use_when: vec!["用户询问Agent执行历史".to_string()],
        },

        // === Rule Tool (聚合4个规则操作) ===
        LlmToolDefinition {
            name: "rule".to_string(),
            description: "规则管理工具。action: list|get|delete|history".to_string(),
            aliases: vec!["规则".to_string(), "list_rules".to_string(), "get_rule".to_string(), "delete_rule".to_string(), "create_rule".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("rule_id".to_string(), ParameterInfo {
                    description: "规则ID (get/delete需要)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["rule_1".to_string()],
                }),
                ("name".to_string(), ParameterInfo {
                    description: "规则名称 (create需要)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["低电量告警".to_string()],
                }),
                ("dsl".to_string(), ParameterInfo {
                    description: "规则DSL (create需要)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["RULE \"低电量\" WHEN ne101.battery < 50 DO NOTIFY \"电量低\" END".to_string()],
                }),
                ("start_time".to_string(), ParameterInfo {
                    description: "起始时间戳 (history可选)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["今天0点".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "有哪些规则？".to_string(),
                    tool_call: "rule(action=\"list\")".to_string(),
                    explanation: "列出所有规则".to_string(),
                },
                Example {
                    user_query: "当ne101电量低于50%时告警".to_string(),
                    tool_call: "rule(action=\"create\", name=\"低电量告警\", dsl=\"RULE \\\"低电量告警\\\" WHEN ne101.battery < 50 DO NOTIFY \\\"电量低于50%\\\" END\")".to_string(),
                    explanation: "创建规则".to_string(),
                },
                Example {
                    user_query: "删除规则123".to_string(),
                    tool_call: "rule(action=\"delete\", rule_id=\"123\")".to_string(),
                    explanation: "删除规则".to_string(),
                },
            ],
            use_when: vec!["用户询问规则".to_string(), "用户要创建/删除规则".to_string()],
        },

        // === Alert Tool ===
        LlmToolDefinition {
            name: "alert".to_string(),
            description: "告警管理工具。action: list|create|acknowledge".to_string(),
            aliases: vec!["告警".to_string(), "list_alerts".to_string(), "create_alert".to_string(), "acknowledge_alert".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("alert_id".to_string(), ParameterInfo {
                    description: "告警ID (acknowledge需要)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["alert_1".to_string()],
                }),
                ("title".to_string(), ParameterInfo {
                    description: "告警标题 (create需要)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["温度异常".to_string()],
                }),
                ("message".to_string(), ParameterInfo {
                    description: "告警消息 (create需要)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["温度超过阈值".to_string()],
                }),
                ("severity".to_string(), ParameterInfo {
                    description: "严重程度 (create可选: info/warning/error/critical)".to_string(),
                    default: serde_json::json!("warning"),
                    examples: vec!["warning".to_string(), "critical".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "有哪些告警？".to_string(),
                    tool_call: "alert(action=\"list\")".to_string(),
                    explanation: "列出所有告警".to_string(),
                },
                Example {
                    user_query: "确认告警123".to_string(),
                    tool_call: "alert(action=\"acknowledge\", alert_id=\"123\")".to_string(),
                    explanation: "确认告警".to_string(),
                },
            ],
            use_when: vec!["用户询问告警".to_string(), "用户要确认告警".to_string()],
        },
    ]
}

/// Format simplified tools into a prompt for the LLM.
pub fn format_tools_for_llm() -> String {
    let tools = get_simplified_tools();
    let mut prompt = String::from("## 可用工具 (聚合设计)\n\n");

    // 精简指导原则
    prompt.push_str("### 调用方式\n\n");
    prompt.push_str("所有工具使用 action 参数区分操作类型:\n");
    prompt.push_str("- device(action=\"list|get|query|control\", ...)\n");
    prompt.push_str("- agent(action=\"list|get|create|update|control|memory\", ...)\n");
    prompt.push_str("- agent_history(action=\"executions|conversation\", agent_id=\"...\")\n");
    prompt.push_str("- rule(action=\"list|get|create|delete|history\", ...)\n");
    prompt.push_str("- alert(action=\"list|create|acknowledge\", ...)\n\n");
    prompt.push_str(
        "格式: [{\"name\":\"工具名\",\"arguments\":{\"action\":\"操作\",\"参数\":\"值\"}}]\n\n",
    );

    for tool in tools {
        prompt.push_str(&format!("### {} - {}", tool.name, tool.description));
        if !tool.aliases.is_empty() {
            prompt.push_str(&format!(
                " [别名:{}]",
                tool.aliases
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        prompt.push('\n');

        // 精简参数展示
        if !tool.required.is_empty() {
            prompt.push_str(&format!("必参:{}", tool.required.join(",")));
        }
        if !tool.optional.is_empty() {
            prompt.push_str(&format!(
                " 可参:{}",
                tool.optional
                    .keys()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        prompt.push('\n');
    }

    prompt
}

/// Format simplified tools as JSON for function calling.
pub fn format_tools_as_json() -> Vec<Value> {
    let tools = get_simplified_tools();
    tools
        .into_iter()
        .map(|tool| {
            let mut properties = serde_json::Map::new();

            // Build properties from required and optional params
            for param in &tool.required {
                properties.insert(
                    param.clone(),
                    serde_json::json!({
                        "type": "string",
                        "description": format!("{} (必需)", param)
                    }),
                );
            }
            for (param, info) in &tool.optional {
                properties.insert(
                    param.clone(),
                    serde_json::json!({
                        "type": "string",
                        "description": info.description,
                        "default": info.default
                    }),
                );
            }

            serde_json::json!({
                "name": tool.name,
                "description": tool.description,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": tool.required
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tools_for_llm() {
        let prompt = format_tools_for_llm();
        // Check for aggregated tool names
        assert!(prompt.contains("device"));
        assert!(prompt.contains("agent"));
        assert!(prompt.contains("rule"));
        assert!(prompt.contains("alert"));
    }

    #[test]
    fn test_get_simplified_tools_count() {
        let tools = get_simplified_tools();
        // Should have 5 aggregated tools
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn test_aggregated_tool_names() {
        let tools = get_simplified_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"device"));
        assert!(names.contains(&"agent"));
        assert!(names.contains(&"agent_history"));
        assert!(names.contains(&"rule"));
        assert!(names.contains(&"alert"));
    }

    #[test]
    fn test_friendly_error() {
        let err = ErrorMessages::device_not_found("sensor_1");
        assert_eq!(err.message, "找不到设备 'sensor_1'");
        assert_eq!(err.suggestions.len(), 3);
        assert!(!err.is_warning);
    }

    #[test]
    fn test_friendly_warning() {
        let err = ErrorMessages::no_data_available("sensor_1");
        assert!(err.is_warning);
    }

    #[test]
    fn test_simplified_config() {
        let config = SimplifiedConfig::new("test_tool")
            .with_description("A test tool")
            .with_alias("测试工具")
            .with_required_param("input")
            .with_example("test_tool(input='hello')");

        assert_eq!(config.name, "test_tool");
        assert_eq!(config.description, "A test tool");
        assert_eq!(config.aliases.len(), 1);
        assert_eq!(config.required_params.len(), 1);
        assert_eq!(config.examples.len(), 1);
    }
}
