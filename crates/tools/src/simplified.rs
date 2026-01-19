//! Simplified tool wrappers with smart defaults and user-friendly responses.
//!
//! This module provides simplified versions of tools that:
//! 1. Have minimal required parameters
//! 2. Use smart defaults for common use cases
//! 3. Return user-friendly error messages
//! 4. Support natural language parameter names

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{Result, ToolError};
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
        let mut output = if self.is_warning {
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
        };
        output
    }
}

/// Common friendly error messages.
pub struct ErrorMessages;

impl ErrorMessages {
    /// Device not found error with helpful suggestions.
    pub fn device_not_found(device_id: &str) -> FriendlyError {
        FriendlyError::new(format!("找不到设备 '{}'", device_id))
            .with_suggestion("使用 'list_devices' 查看所有可用设备".to_string())
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
    /// Tool name (short, memorable)
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
pub fn get_simplified_tools() -> Vec<LlmToolDefinition> {
    vec![
        // === Core Tool: device.discover ===
        LlmToolDefinition {
            name: "device.discover".to_string(),
            description: "发现系统中的所有设备。支持按位置、类型、状态过滤和分组。这是探索系统设备能力的入口工具。".to_string(),
            aliases: vec![
                "发现设备".to_string(),
                "设备列表".to_string(),
                "查看设备".to_string(),
                "所有设备".to_string(),
                "有什么设备".to_string(),
                "有哪些设备".to_string(),
                "设备有哪些".to_string(),
                "list_devices".to_string(),
                "devices".to_string(),
                "discover".to_string(),
            ],
            required: vec![],
            optional: {
                let mut map = HashMap::new();
                map.insert("location".to_string(), ParameterInfo {
                    description: "位置过滤，如 '客厅' '卧室' '厨房'".to_string(),
                    default: Value::Null,
                    examples: vec!["客厅".to_string(), "卧室".to_string(), "厨房".to_string()],
                });
                map.insert("type".to_string(), ParameterInfo {
                    description: "设备类型，如 'sensor' 传感器, 'actuator' 执行器".to_string(),
                    default: Value::Null,
                    examples: vec!["sensor".to_string(), "actuator".to_string(), "light".to_string()],
                });
                map.insert("status".to_string(), ParameterInfo {
                    description: "状态筛选，如 'online' 在线, 'offline' 离线".to_string(),
                    default: Value::Null,
                    examples: vec!["online".to_string(), "offline".to_string()],
                });
                map.insert("group_by".to_string(), ParameterInfo {
                    description: "分组方式：'type'按类型、'location'按位置、'status'按状态".to_string(),
                    default: serde_json::json!("none"),
                    examples: vec!["type".to_string(), "location".to_string(), "status".to_string()],
                });
                map
            },
            examples: vec![
                Example {
                    user_query: "系统有哪些设备？".to_string(),
                    tool_call: "device.discover()".to_string(),
                    explanation: "显示所有设备列表和摘要".to_string(),
                },
                Example {
                    user_query: "客厅有哪些设备？".to_string(),
                    tool_call: "device.discover(filter={location:'客厅'}, group_by='type')".to_string(),
                    explanation: "显示客厅的设备，按类型分组".to_string(),
                },
                Example {
                    user_query: "哪些设备离线了？".to_string(),
                    tool_call: "device.discover(filter={status:'offline'})".to_string(),
                    explanation: "只显示离线设备".to_string(),
                },
            ],
            use_when: vec![
                "用户询问设备".to_string(),
                "用户问有哪些".to_string(),
                "用户想查看设备列表".to_string(),
                "用户询问特定位置的设备".to_string(),
            ],
        },

        // === Core Tool: device.query ===
        LlmToolDefinition {
            name: "device.query".to_string(),
            description: "查询设备的实时或历史数据。支持查询单个或多个指标，可指定时间范围和聚合方式。".to_string(),
            aliases: vec![
                "查询数据".to_string(),
                "获取数据".to_string(),
                "设备数据".to_string(),
                "数据".to_string(),
                "温度".to_string(),
                "湿度".to_string(),
                "温度多少".to_string(),
                "当前温度".to_string(),
                "query_data".to_string(),
                "data".to_string(),
                "query".to_string(),
            ],
            required: vec!["device_id".to_string()],
            optional: {
                let mut map = HashMap::new();
                map.insert("metrics".to_string(), ParameterInfo {
                    description: "要查询的指标列表，如 ['temperature', 'humidity']。不指定则返回所有指标".to_string(),
                    default: Value::Null,
                    examples: vec!["['temperature']".to_string(), "['temperature', 'humidity']".to_string()],
                });
                map.insert("limit".to_string(), ParameterInfo {
                    description: "返回数据点数量限制，默认24个点".to_string(),
                    default: serde_json::json!(24),
                    examples: vec!["10".to_string(), "24".to_string(), "100".to_string()],
                });
                map
            },
            examples: vec![
                Example {
                    user_query: "客厅温度多少？".to_string(),
                    tool_call: "device.query(device_id='sensor_temp_living', metrics=['temperature'])".to_string(),
                    explanation: "查询客厅温度传感器的当前温度".to_string(),
                },
                Example {
                    user_query: "过去24小时的温度数据".to_string(),
                    tool_call: "device.query(device_id='sensor_temp_living', metrics=['temperature'], limit=24)".to_string(),
                    explanation: "查询24小时的历史温度数据".to_string(),
                },
                Example {
                    user_query: "传感器有哪些数据？".to_string(),
                    tool_call: "device.query(device_id='sensor_temp_living')".to_string(),
                    explanation: "查询传感器的所有指标".to_string(),
                },
            ],
            use_when: vec![
                "用户询问温度/湿度/数据".to_string(),
                "用户问现在多少度".to_string(),
                "用户想查看读数".to_string(),
                "用户询问历史数据".to_string(),
                "用户询问趋势".to_string(),
            ],
        },

        // === Core Tool: device.control ===
        LlmToolDefinition {
            name: "device.control".to_string(),
            description: "控制单个或多个设备。支持打开、关闭、设置值等操作。".to_string(),
            aliases: vec![
                "控制设备".to_string(),
                "打开".to_string(),
                "关闭".to_string(),
                "开启".to_string(),
                "调节".to_string(),
                "设置".to_string(),
                "turn_on".to_string(),
                "turn_off".to_string(),
                "open".to_string(),
                "close".to_string(),
                "control".to_string(),
            ],
            required: vec!["command".to_string()],
            optional: {
                let mut map = HashMap::new();
                map.insert("device_id".to_string(), ParameterInfo {
                    description: "单个设备ID，支持模糊匹配".to_string(),
                    default: Value::Null,
                    examples: vec!["light_living".to_string(), "sensor_temp".to_string()],
                });
                map.insert("value".to_string(), ParameterInfo {
                    description: "命令值，如温度26、亮度50".to_string(),
                    default: Value::Null,
                    examples: vec!["26".to_string(), "50".to_string(), "100".to_string()],
                });
                map
            },
            examples: vec![
                Example {
                    user_query: "打开客厅的灯".to_string(),
                    tool_call: "device.control(device_id='light_living_main', command='turn_on')".to_string(),
                    explanation: "打开指定设备".to_string(),
                },
                Example {
                    user_query: "关闭空调".to_string(),
                    tool_call: "device.control(device_id='ac', command='turn_off')".to_string(),
                    explanation: "关闭指定设备".to_string(),
                },
                Example {
                    user_query: "把空调设为26度".to_string(),
                    tool_call: "device.control(device_id='ac_bedroom', command='set_temperature', value={temperature:26})".to_string(),
                    explanation: "设置空调温度".to_string(),
                },
                Example {
                    user_query: "打开所有灯".to_string(),
                    tool_call: "device.control(device_id='light', command='turn_on')".to_string(),
                    explanation: "批量控制所有匹配的设备".to_string(),
                },
            ],
            use_when: vec![
                "用户要打开/关闭设备".to_string(),
                "用户要调节设备".to_string(),
                "用户说太亮/太暗".to_string(),
                "用户要设置温度".to_string(),
            ],
        },

        // === Rule Management Tools ===
        LlmToolDefinition {
            name: "create_rule".to_string(),
            description: "创建自动化规则。当满足条件时自动执行操作。".to_string(),
            aliases: vec![
                "创建规则".to_string(),
                "新建规则".to_string(),
                "添加规则".to_string(),
                "自动化".to_string(),
                "告警".to_string()
            ],
            required: vec!["name".to_string(), "condition".to_string(), "action".to_string()],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "创建高温告警规则".to_string(),
                    tool_call: "create_rule(name='高温告警', condition='温度>50', action='通知我')".to_string(),
                    explanation: "温度超过50度时发送通知".to_string(),
                },
                Example {
                    user_query: "温度超过30度时开风扇".to_string(),
                    tool_call: "create_rule(name='自动降温', condition='温度>30', action='打开风扇')".to_string(),
                    explanation: "高温自动开启风扇".to_string(),
                },
            ],
            use_when: vec![
                "用户要创建规则".to_string(),
                "用户要设置自动化".to_string(),
                "用户要添加告警".to_string(),
            ],
        },

        LlmToolDefinition {
            name: "list_rules".to_string(),
            description: "列出所有规则。查看已创建的自动化规则。".to_string(),
            aliases: vec![
                "规则列表".to_string(),
                "显示规则".to_string(),
                "所有规则".to_string(),
                "查看规则".to_string()
            ],
            required: vec![],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "显示所有规则".to_string(),
                    tool_call: "list_rules()".to_string(),
                    explanation: "列出所有自动化规则".to_string(),
                },
            ],
            use_when: vec![
                "用户询问规则".to_string(),
                "用户想查看已创建的自动化".to_string(),
            ],
        },

        LlmToolDefinition {
            name: "disable_rule".to_string(),
            description: "禁用规则。暂停指定的自动化规则。".to_string(),
            aliases: vec![
                "禁用规则".to_string(),
                "停用规则".to_string(),
                "暂停规则".to_string(),
                "关闭规则".to_string()
            ],
            required: vec!["rule".to_string()],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "禁用高温告警规则".to_string(),
                    tool_call: "disable_rule(rule='高温告警')".to_string(),
                    explanation: "暂停指定的自动化规则".to_string(),
                },
            ],
            use_when: vec![
                "用户要禁用/停用规则".to_string(),
                "用户要暂停自动化".to_string(),
            ],
        },

        LlmToolDefinition {
            name: "enable_rule".to_string(),
            description: "启用规则。恢复已禁用的自动化规则。".to_string(),
            aliases: vec![
                "启用规则".to_string(),
                "激活规则".to_string(),
                "开启规则".to_string(),
                "恢复规则".to_string()
            ],
            required: vec!["rule".to_string()],
            optional: HashMap::new(),
            examples: vec![
                Example {
                    user_query: "启用刚才的规则".to_string(),
                    tool_call: "enable_rule(rule='高温告警')".to_string(),
                    explanation: "恢复指定的自动化规则".to_string(),
                },
            ],
            use_when: vec![
                "用户要启用/激活规则".to_string(),
                "用户要恢复自动化".to_string(),
            ],
        },

        // === Core Tool: device.analyze ===
        LlmToolDefinition {
            name: "device.analyze".to_string(),
            description: "使用LLM分析设备数据，发现趋势、异常和模式。支持趋势分析、异常检测、数据摘要等多种分析类型。".to_string(),
            aliases: vec![
                "分析数据".to_string(),
                "数据分析".to_string(),
                "趋势分析".to_string(),
                "异常检测".to_string(),
                "数据异常".to_string(),
                "analyze".to_string(),
                "analysis".to_string(),
                "trend".to_string(),
            ],
            required: vec!["device_id".to_string()],
            optional: {
                let mut map = HashMap::new();
                map.insert("metric".to_string(), ParameterInfo {
                    description: "要分析的指标名称，如 'temperature'。不指定则分析所有指标".to_string(),
                    default: Value::Null,
                    examples: vec!["temperature".to_string(), "humidity".to_string()],
                });
                map.insert("analysis_type".to_string(), ParameterInfo {
                    description: "分析类型：'trend' 趋势、'anomaly' 异常、'summary' 摘要".to_string(),
                    default: serde_json::json!("summary"),
                    examples: vec!["trend".to_string(), "anomaly".to_string(), "summary".to_string()],
                });
                map.insert("limit".to_string(), ParameterInfo {
                    description: "要分析的数据点数量，默认24".to_string(),
                    default: serde_json::json!(24),
                    examples: vec!["24".to_string(), "48".to_string(), "100".to_string()],
                });
                map
            },
            examples: vec![
                Example {
                    user_query: "分析温度趋势".to_string(),
                    tool_call: "device.analyze(device_id='sensor_temp_living', metric='temperature', analysis_type='trend')".to_string(),
                    explanation: "分析温度变化趋势，判断是否上升/下降/稳定".to_string(),
                },
                Example {
                    user_query: "检测数据异常".to_string(),
                    tool_call: "device.analyze(device_id='sensor_temp_living', analysis_type='anomaly')".to_string(),
                    explanation: "使用统计方法检测数据中的异常点".to_string(),
                },
                Example {
                    user_query: "数据摘要".to_string(),
                    tool_call: "device.analyze(device_id='sensor_temp_living')".to_string(),
                    explanation: "生成统计摘要，包括最大值、最小值、平均值等".to_string(),
                },
            ],
            use_when: vec![
                "用户要求分析数据".to_string(),
                "用户询问趋势".to_string(),
                "用户检测异常".to_string(),
                "用户要数据摘要".to_string(),
            ],
        },

        // === Core Tool: rule.from_context ===
        LlmToolDefinition {
            name: "rule.from_context".to_string(),
            description: "从自然语言描述中提取规则信息，生成结构化的规则定义和DSL。支持理解用户的自然语言描述并生成规则代码。".to_string(),
            aliases: vec![
                "创建规则".to_string(),
                "新建规则".to_string(),
                "添加规则".to_string(),
                "生成规则".to_string(),
                "规则定义".to_string(),
                "自动化规则".to_string(),
                "create_rule".to_string(),
                "new_rule".to_string(),
                "add_rule".to_string(),
            ],
            required: vec!["description".to_string()],
            optional: {
                let mut map = HashMap::new();
                map.insert("context_devices".to_string(), ParameterInfo {
                    description: "可选：上下文中的设备ID列表，用于验证".to_string(),
                    default: Value::Null,
                    examples: vec!["['sensor_temp_living', 'sensor_humidity_living']".to_string()],
                });
                map.insert("confirm".to_string(), ParameterInfo {
                    description: "是否确认创建规则，默认false仅预览".to_string(),
                    default: serde_json::json!(false),
                    examples: vec!["true".to_string(), "false".to_string()],
                });
                map
            },
            examples: vec![
                Example {
                    user_query: "温度超过50度时告警".to_string(),
                    tool_call: "rule.from_context(description='温度超过50度时告警')".to_string(),
                    explanation: "生成高温告警规则的定义和DSL".to_string(),
                },
                Example {
                    user_query: "温度持续5分钟超过30度时开风扇".to_string(),
                    tool_call: "rule.from_context(description='温度持续5分钟超过30度时开风扇')".to_string(),
                    explanation: "生成带持续时间和动作的规则".to_string(),
                },
                Example {
                    user_query: "湿度低于30%时告警并开启加湿器".to_string(),
                    tool_call: "rule.from_context(description='湿度低于30%时告警并开启加湿器')".to_string(),
                    explanation: "生成多动作规则".to_string(),
                },
            ],
            use_when: vec![
                "用户要创建规则".to_string(),
                "用户描述自动化场景".to_string(),
                "用户设置阈值告警".to_string(),
                "用户要条件触发动作".to_string(),
            ],
        },
    ]
}

/// Format simplified tools into a prompt for the LLM.
pub fn format_tools_for_llm() -> String {
    let tools = get_simplified_tools();
    let mut prompt = String::from("## 可用工具\n\n");
    prompt.push_str("你可以调用以下工具来完成任务。工具调用格式：\n");
    prompt.push_str("```\n[{\"name\":\"工具名\",\"arguments\":{\"参数\":\"值\"}}]\n```\n\n");

    for tool in tools {
        prompt.push_str(&format!("### {} ({})\n", tool.name, tool.description));

        if !tool.aliases.is_empty() {
            prompt.push_str(&format!("**别名**: {}\n", tool.aliases.join(", ")));
        }

        prompt.push_str("**参数**:\n");
        if tool.required.is_empty() && tool.optional.is_empty() {
            prompt.push_str("  无需参数\n");
        } else {
            for param in &tool.required {
                prompt.push_str(&format!("  - **{}** (必需)\n", param));
            }
            for (param, info) in &tool.optional {
                prompt.push_str(&format!("  - **{}** (可选，默认: {}) - {}\n",
                    param, info.default, info.description));
            }
        }

        if !tool.examples.is_empty() {
            prompt.push_str("\n**示例**:\n");
            for ex in &tool.examples {
                prompt.push_str(&format!("  - 用户: \"{}\"\n", ex.user_query));
                prompt.push_str(&format!("    → `{}`\n", ex.tool_call));
            }
        }

        if !tool.use_when.is_empty() {
            prompt.push_str(&format!("\n**使用场景**: {}\n", tool.use_when.join("、")));
        }

        prompt.push_str("\n");
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
        assert!(prompt.contains("device.discover") || prompt.contains("list_devices"));
        assert!(prompt.contains("device.query") || prompt.contains("query_data"));
        assert!(prompt.contains("device.control") || prompt.contains("control_device"));
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
