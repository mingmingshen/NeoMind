//! Simplified tool wrappers with smart defaults and user-friendly responses.
//!
//! This module provides simplified versions of tools that:
//! 1. Have minimal required parameters
//! 2. Use smart defaults for common use cases
//! 3. Return user-friendly error messages
//! 4. Support natural language parameter names
//!
//! NOTE: Tool names here must match the actual tool names registered in the tool registry.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
                })
            )
        } else {
            ToolOutput::error_with_metadata(
                &self.message,
                serde_json::json!({
                    "suggestions": self.suggestions,
                    "type": "friendly_error"
                })
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
/// 工具清单 (12个工具，从原18个精简):
/// - 设备工具(4): device_discover, get_device_data, query_data, device_control
/// - 规则工具(3): list_rules, create_rule, delete_rule
/// - Agent工具(5): list_agents, get_agent, create_agent, execute_agent, control_agent
pub fn get_simplified_tools() -> Vec<LlmToolDefinition> {
    vec![
        // === 工具调用流程说明 ===
        //
        // **设备数据工具支持名称解析**：
        //   - get_device_data 和 query_data 的 device_id 参数支持设备名称、简称或完整ID
        //   - 工具内部会自动将 'ne101'、'ne101 test' 等名称解析为真实设备ID
        //   - 因此：device_discover 不是必须的，除非用户明确问"有哪些设备"
        //
        // **推荐调用顺序**:
        //   场景A - 用户询问特定设备数据：
        //     Step 1: get_device_data(device_id="设备名称") → 获取所有当前指标
        //     Step 2: query_data(device_id="设备名称", metric="实际指标名") → 仅当需要历史/趋势时
        //
        //   场景B - 用户问"有哪些设备"：
        //     Step 1: device_discover() → 显示所有设备列表
        //     Step 2: get_device_data(device_id) → 根据需要查询特定设备
        //
        // **常见错误**:
        //   ❌ 跳过 get_device_data 直接调用 query_data → 指标名可能不准确
        //   ❌ query_data 的 metric 参数用 'battery' → 应该是 'values.battery'
        //
        // **正确示例**:
        //   ✅ 用户说"ne101 test的数据" → get_device_data(device_id="ne101 test")
        //   ✅ 用户说"有哪些设备" → device_discover()
        //   ✅ 用户说"今天的电池趋势" → get_device_data() → query_data(metric="values.battery")
        //
        // ================================

        // === Device Tools (4个) ===

        // device_discover - 列出所有设备
        LlmToolDefinition {
            name: "device_discover".to_string(),
            description: "列出所有设备(id/name/type/status)和统计信息".to_string(),
            aliases: vec!["查看设备".to_string(), "有哪些设备".to_string(), "设备列表".to_string()],
            required: vec![],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "有哪些设备？".to_string(),
                    tool_call: "device_discover()".to_string(),
                    explanation: "显示设备列表".to_string(),
                },
            ],
            use_when: vec!["用户询问设备".to_string()],
        },

        // get_device_data - 获取设备数据（含趋势分析）
        LlmToolDefinition {
            name: "get_device_data".to_string(),
            description: "获取设备当前所有指标值。device_id支持名称/简称/ID自动解析。趋势分析: analysis='trend'".to_string(),
            aliases: vec!["设备数据".to_string(), "设备状态".to_string(), "趋势分析".to_string()],
            required: vec!["device_id".to_string()],
            optional: HashMap::from_iter(vec![
                ("analysis".to_string(), ParameterInfo {
                    description: "分析类型: 'trend'趋势分析".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["trend".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "ne101的电量？".to_string(),
                    tool_call: "get_device_data(device_id='ne101')".to_string(),
                    explanation: "查询设备数据".to_string(),
                },
                Example {
                    user_query: "电量趋势？".to_string(),
                    tool_call: "get_device_data(device_id='ne101', analysis='trend')".to_string(),
                    explanation: "趋势分析".to_string(),
                },
            ],
            use_when: vec!["用户询问设备数据/状态/趋势".to_string()],
        },

        // query_data - 查询历史时间序列数据
        LlmToolDefinition {
            name: "query_data".to_string(),
            description: "查询设备历史时间序列数据。需先get_device_data确认指标名(如values.battery)".to_string(),
            aliases: vec!["历史数据".to_string()],
            required: vec!["device_id".to_string(), "metric".to_string()],
            optional: HashMap::from_iter(vec![
                ("start_time".to_string(), ParameterInfo {
                    description: "起始时间戳(秒)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["今天0点".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "今天的详细电量数据".to_string(),
                    tool_call: "query_data(device_id='ne101', metric='values.battery')".to_string(),
                    explanation: "历史查询".to_string(),
                },
            ],
            use_when: vec!["用户查询详细历史数据".to_string()],
        },

        // device_control - 控制设备
        LlmToolDefinition {
            name: "device_control".to_string(),
            description: "控制设备: turn_on/turn_off/set_value".to_string(),
            aliases: vec!["打开".to_string(), "关闭".to_string(), "控制".to_string()],
            required: vec!["device_id".to_string(), "command".to_string()],
            optional: {
                let mut map = HashMap::new();
                map.insert("value".to_string(), ParameterInfo {
                    description: "命令值".to_string(),
                    default: Value::Null,
                    examples: vec!["26".to_string()],
                });
                map
            },
            examples: vec![
                Example {
                    user_query: "打开客厅灯".to_string(),
                    tool_call: "device_control(device_id='light_living', command='turn_on')".to_string(),
                    explanation: "控制设备".to_string(),
                },
            ],
            use_when: vec!["用户要打开/关闭/控制设备".to_string()],
        },

        // === Rule Tools (3个) ===

        // list_rules - actual tool name in registry
        LlmToolDefinition {
            name: "list_rules".to_string(),
            description: "列出所有自动化规则".to_string(),
            aliases: vec![
                "规则列表".to_string(),
                "有哪些规则".to_string(),
            ],
            required: vec![],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "有哪些规则？".to_string(),
                    tool_call: "list_rules()".to_string(),
                    explanation: "列出所有规则".to_string(),
                },
            ],
            use_when: vec!["用户询问规则".to_string()],
        },

        // create_rule - actual tool name in registry
        LlmToolDefinition {
            name: "create_rule".to_string(),
            description: "创建自动化规则。DSL: RULE\"名\"WHEN条件DO动作END。条件: device.metric>50、BETWEEN、AND/OR。动作: NOTIFY/EXECUTE/ALERT/SET/DELAY".to_string(),
            aliases: vec![
                "创建规则".to_string(),
                "添加规则".to_string(),
            ],
            required: vec!["name".to_string(), "dsl".to_string()],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "当ne101电量低于50%时告警".to_string(),
                    tool_call: "create_rule(name='低电量告警', dsl='RULE \"低电量告警\"\nWHEN ne101.battery_percent < 50\nDO NOTIFY \"设备ne101电量低于50%\"\nEND')".to_string(),
                    explanation: "创建低电量告警规则，设备ID和指标名用点连接，消息用引号".to_string(),
                },
                Example {
                    user_query: "温度过高时自动开启风扇".to_string(),
                    tool_call: "create_rule(name='高温开启风扇', dsl='RULE \"高温开启风扇\"\nWHEN sensor.temperature > 30\nDO EXECUTE sensor.fan(speed=100)\nEND')".to_string(),
                    explanation: "创建带设备控制的规则，使用EXECUTE动作".to_string(),
                },
                Example {
                    user_query: "温度异常时告警（过高或过低）".to_string(),
                    tool_call: "create_rule(name='温度异常', dsl='RULE \"温度异常\"\nWHEN (sensor.temp > 35) OR (sensor.temp < 10)\nDO NOTIFY \"温度超出安全范围\"\nEND')".to_string(),
                    explanation: "创建复杂条件规则，使用OR逻辑组合".to_string(),
                },
            ],
            use_when: vec!["用户要创建规则".to_string()],
        },

        // delete_rule - actual tool name in registry
        LlmToolDefinition {
            name: "delete_rule".to_string(),
            description: "删除指定的规则".to_string(),
            aliases: vec![
                "删除规则".to_string(),
                "移除规则".to_string(),
            ],
            required: vec!["rule_id".to_string()],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "删除规则123".to_string(),
                    tool_call: "delete_rule(rule_id='123')".to_string(),
                    explanation: "删除规则".to_string(),
                },
            ],
            use_when: vec!["用户要删除规则".to_string()],
        },

        // === Agent Tools (5个) ===

        // list_agents - actual tool name in registry
        LlmToolDefinition {
            name: "list_agents".to_string(),
            description: "列出所有Agent(id/状态/执行统计)。用户询问Agent时必先调用此工具获取ID".to_string(),
            aliases: vec![
                "列出Agent".to_string(),
                "有哪些Agent".to_string(),
                "Agent列表".to_string(),
            ],
            required: vec![],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "有哪些Agent？".to_string(),
                    tool_call: "list_agents()".to_string(),
                    explanation: "首先列出所有Agent获取ID".to_string(),
                },
                Example {
                    user_query: "显示所有智能体".to_string(),
                    tool_call: "list_agents()".to_string(),
                    explanation: "列出Agent及其状态".to_string(),
                },
            ],
            use_when: vec![
                "用户询问有哪些Agent".to_string(),
                "用户询问Agent列表".to_string(),
                "用户想查看所有Agent".to_string(),
                "用户询问某个Agent但未提供agent_id".to_string(),
            ],
        },

        // get_agent - actual tool name in registry
        LlmToolDefinition {
            name: "get_agent".to_string(),
            description: "获取Agent详情(执行统计/调度配置)。无agent_id时必先list_agents".to_string(),
            aliases: vec![
                "Agent详情".to_string(),
                "Agent信息".to_string(),
                "Agent执行情况".to_string(),
            ],
            required: vec!["agent_id".to_string()],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "温度监控Agent的执行情况".to_string(),
                    tool_call: "先list_agents获取agent_id，再用get_agent(agent_id='xxx')查询详情".to_string(),
                    explanation: "需要agent_id才能查询，先调用list_agents".to_string(),
                },
                Example {
                    user_query: "agent_1的详细信息".to_string(),
                    tool_call: "get_agent(agent_id='agent_1')".to_string(),
                    explanation: "已知agent_id时直接查询".to_string(),
                },
            ],
            use_when: vec![
                "用户询问Agent详情/状态".to_string(),
                "用户询问Agent执行情况/结果".to_string(),
                "用户提供agent_id名称".to_string(),
            ],
        },

        // execute_agent - actual tool name in registry
        LlmToolDefinition {
            name: "execute_agent".to_string(),
            description: "手动执行Agent".to_string(),
            aliases: vec![
                "执行Agent".to_string(),
                "运行Agent".to_string(),
            ],
            required: vec!["agent_id".to_string()],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "执行温度监控".to_string(),
                    tool_call: "execute_agent(agent_id='agent_1')".to_string(),
                    explanation: "执行Agent".to_string(),
                },
            ],
            use_when: vec!["用户要执行Agent".to_string()],
        },

        // control_agent - actual tool name in registry
        LlmToolDefinition {
            name: "control_agent".to_string(),
            description: "控制Agent（暂停/恢复/删除）".to_string(),
            aliases: vec![
                "暂停Agent".to_string(),
                "恢复Agent".to_string(),
            ],
            required: vec!["agent_id".to_string(), "action".to_string()],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "暂停温度监控".to_string(),
                    tool_call: "control_agent(agent_id='agent_1', action='pause')".to_string(),
                    explanation: "暂停Agent".to_string(),
                },
            ],
            use_when: vec!["用户要控制Agent".to_string()],
        },

        // create_agent - actual tool name in registry
        LlmToolDefinition {
            name: "create_agent".to_string(),
            description: "创建AI Agent。描述: 设备/指标/条件/动作/频率。类型: monitor/executor/analyst".to_string(),
            aliases: vec![
                "创建Agent".to_string(),
                "新建Agent".to_string(),
            ],
            required: vec!["description".to_string()],
            optional: HashMap::from_iter(vec![
                ("name".to_string(), ParameterInfo {
                    description: "可选，Agent名称".to_string(),
                    default: Value::Null,
                    examples: vec!["温度监控Agent".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "帮我创建一个监控ne101温度的Agent，每5分钟检查一次".to_string(),
                    tool_call: "create_agent(description='监控ne101设备(4t1vcbefzk)的温度指标，每5分钟检查一次，如果温度超过30度就发送告警通知')".to_string(),
                    explanation: "创建监控型Agent，先查询设备信息".to_string(),
                },
                Example {
                    user_query: "创建一个分析电池状态的Agent".to_string(),
                    tool_call: "create_agent(description='每天早上8点分析所有NE101设备的电池状态，识别电池电量低于20%的设备并生成报告')".to_string(),
                    explanation: "创建分析型Agent，按时间执行".to_string(),
                },
                Example {
                    user_query: "湿度低的时候自动开加湿器".to_string(),
                    tool_call: "create_agent(description='监控室内湿度，当湿度低于30%时自动打开加湿器，每分钟检查一次')".to_string(),
                    explanation: "创建执行型Agent，自动控制设备".to_string(),
                },
            ],
            use_when: vec!["用户要创建Agent".to_string(), "用户要添加智能体".to_string(), "用户要做自动化".to_string()],
        },

        // 注意: 已移除低频工具 - agent_memory, get_agent_executions, get_agent_execution_detail, get_agent_conversation
        // 这些功能可通过 get_agent 获取基础信息，详情可通过直接查询数据库获得
    ]
}

/// Format simplified tools into a prompt for the LLM.
pub fn format_tools_for_llm() -> String {
    let tools = get_simplified_tools();
    let mut prompt = String::from("## 可用工具\n\n");

    // 精简指导原则
    prompt.push_str("### 调用流程\n\n");
    prompt.push_str("设备数据: device_discover→get_device_data→query_data(如需历史)\n");
    prompt.push_str("Agent查询: list_agents→get_agent(需agent_id)\n");
    prompt.push_str("格式: [{\"name\":\"工具名\",\"arguments\":{\"参数\":\"值\"}}]\n\n");

    for tool in tools {
        prompt.push_str(&format!("### {} - {}", tool.name, tool.description));
        if !tool.aliases.is_empty() {
            prompt.push_str(&format!(" [{}]", tool.aliases.join(",")));
        }
        prompt.push('\n');

        // 精简参数展示
        if !tool.required.is_empty() {
            prompt.push_str(&format!("必参:{}", tool.required.join(",")));
        }
        if !tool.optional.is_empty() {
            prompt.push_str(&format!(" 可参:{}", tool.optional.keys().cloned().collect::<Vec<_>>().join(",")));
        }
        prompt.push('\n');
    }

    prompt
}

/// Format simplified tools as JSON for function calling.
pub fn format_tools_as_json() -> Vec<Value> {
    let tools = get_simplified_tools();
    tools.into_iter().map(|tool| {
        let mut properties = serde_json::Map::new();

        // Build properties from required and optional params
        for param in &tool.required {
            properties.insert(param.clone(), serde_json::json!({
                "type": "string",
                "description": format!("{} (必需)", param)
            }));
        }
        for (param, info) in &tool.optional {
            properties.insert(param.clone(), serde_json::json!({
                "type": "string",
                "description": info.description,
                "default": info.default
            }));
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
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tools_for_llm() {
        let prompt = format_tools_for_llm();
        assert!(prompt.contains("device_discover"));
        assert!(prompt.contains("get_device_data"));
        assert!(prompt.contains("device_control"));
        assert!(prompt.contains("create_rule"));
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
