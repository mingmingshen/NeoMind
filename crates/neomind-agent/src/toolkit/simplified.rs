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
/// DESIGN PRINCIPLES (based on Anthropic best practices):
/// - Fewer, more focused tools rather than many granular ones
/// - Merge similar tools to reduce LLM selection burden
/// - Prioritize high-value, high-frequency tools
/// - Each tool should be "irreducible"
///
/// Tool list (5 aggregated tools replacing 34+ individual tools):
/// - device: list, get, query, control
/// - agent: list, get, create, update, control, memory
/// - agent_history: executions, conversation
/// - rule: list, get, delete, history
/// - alert: list, create, acknowledge
pub fn get_simplified_tools() -> Vec<LlmToolDefinition> {
    vec![
        // === Device Tool (aggregates 4 device operations) ===
        LlmToolDefinition {
            name: "device".to_string(),
            description: "Device management tool for querying and controlling IoT devices. Actions: list (list devices), get (device details), query (time-series data), control (send commands). Supports fuzzy device name matching.".to_string(),
            aliases: vec!["device".to_string(), "devices".to_string(), "sensor".to_string(), "iot".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("device_id".to_string(), ParameterInfo {
                    description: "Device ID or partial name (get/query/control). Fuzzy matching supported, e.g., 'living' matches 'Living Room Light'".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["ne101".to_string(), "sensor_1".to_string(), "living_room_light".to_string()],
                }),
                ("metric".to_string(), ParameterInfo {
                    description: "Metric name to query (query action). Format: 'field' or 'values.field'. Examples: 'values.battery', 'temperature'".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["values.battery".to_string(), "temperature".to_string(), "humidity".to_string()],
                }),
                ("command".to_string(), ParameterInfo {
                    description: "Control command (control action). Common: 'turn_on', 'turn_off', 'set_value', 'toggle'".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["turn_on".to_string(), "turn_off".to_string(), "set_value".to_string()],
                }),
                ("params".to_string(), ParameterInfo {
                    description: "Control parameters as JSON object (control action, optional). Example: {\"value\": 26}".to_string(),
                    default: serde_json::json!({}),
                    examples: vec![r#"{"value": 26}"#.to_string(), r#"{"brightness": 80}"#.to_string()],
                }),
                ("start_time".to_string(), ParameterInfo {
                    description: "Start timestamp for time range query (query action). Unix timestamp in seconds. Default: 1 hour ago".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["1712000000".to_string()],
                }),
                ("end_time".to_string(), ParameterInfo {
                    description: "End timestamp for time range query (query action). Default: now".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["1712100000".to_string()],
                }),
                ("response_format".to_string(), ParameterInfo {
                    description: "Output verbosity: 'concise' (default, key info only) or 'detailed' (full data with IDs, for chained calls)".to_string(),
                    default: serde_json::json!("concise"),
                    examples: vec!["concise".to_string(), "detailed".to_string()],
                }),
                ("confirm".to_string(), ParameterInfo {
                    description: "Set to true after user confirms (control action). Without confirmation, returns a preview instead of executing".to_string(),
                    default: serde_json::json!(false),
                    examples: vec!["true".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "What devices do I have?".to_string(),
                    tool_call: r#"device(action="list")"#.to_string(),
                    explanation: "List all devices".to_string(),
                },
                Example {
                    user_query: "What's the battery level of ne101?".to_string(),
                    tool_call: r#"device(action="get", device_id="ne101")"#.to_string(),
                    explanation: "Get device details and current metrics".to_string(),
                },
                Example {
                    user_query: "Show battery trend for today".to_string(),
                    tool_call: r#"device(action="query", device_id="ne101", metric="values.battery")"#.to_string(),
                    explanation: "Query historical time-series data".to_string(),
                },
                Example {
                    user_query: "Turn off the living room light".to_string(),
                    tool_call: r#"device(action="control", device_id="light_living", command="turn_off", confirm=true)"#.to_string(),
                    explanation: "Control device with user confirmation".to_string(),
                },
            ],
            use_when: vec![
                "User asks about devices, sensors, or IoT hardware".to_string(),
                "User wants to check device status or readings".to_string(),
                "User wants to control a device (turn on/off, adjust)".to_string(),
                "User asks for historical sensor data or trends".to_string(),
            ],
        },

        // === Agent Tool (aggregates 6 agent operations) ===
        LlmToolDefinition {
            name: "agent".to_string(),
            description: "AI Agent management tool for creating and managing automated monitoring/control agents. Actions: list, get, create, update, control (pause/resume), memory (view learned patterns).".to_string(),
            aliases: vec!["agent".to_string(), "automation".to_string(), "monitor".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("agent_id".to_string(), ParameterInfo {
                    description: "Agent ID (get/update/control/memory actions). Use list action to find IDs".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["agent_1".to_string(), "550e8400-e29b-41d4-a716-446655440000".to_string()],
                }),
                ("name".to_string(), ParameterInfo {
                    description: "Agent display name (create/update actions). Example: 'Temperature Monitor'".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Temperature Monitor".to_string(), "Security Patrol".to_string()],
                }),
                ("description".to_string(), ParameterInfo {
                    description: "Agent description (create action, optional)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Monitors living room temperature".to_string()],
                }),
                ("user_prompt".to_string(), ParameterInfo {
                    description: "Natural language description of what the agent should do (create action). Be specific with device names and thresholds".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Check ne101 temperature every 5 minutes, alert if above 30C".to_string()],
                }),
                ("schedule_type".to_string(), ParameterInfo {
                    description: "How agent is triggered (create): 'event' (device events), 'cron' (cron schedule), 'interval' (periodic)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["event".to_string(), "cron".to_string(), "interval".to_string()],
                }),
                ("control_action".to_string(), ParameterInfo {
                    description: "Control operation (control action): 'pause' or 'resume'".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["pause".to_string(), "resume".to_string()],
                }),
                ("memory_type".to_string(), ParameterInfo {
                    description: "Memory type to retrieve (memory action): 'patterns' or 'intents'. Default: 'patterns'".to_string(),
                    default: serde_json::json!("patterns"),
                    examples: vec!["patterns".to_string(), "intents".to_string()],
                }),
                ("response_format".to_string(), ParameterInfo {
                    description: "Output verbosity: 'concise' (name/status only) or 'detailed' (full config with IDs)".to_string(),
                    default: serde_json::json!("concise"),
                    examples: vec!["concise".to_string(), "detailed".to_string()],
                }),
                ("confirm".to_string(), ParameterInfo {
                    description: "Set to true after user confirms (control action). Returns preview without confirmation".to_string(),
                    default: serde_json::json!(false),
                    examples: vec!["true".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "What agents are running?".to_string(),
                    tool_call: r#"agent(action="list")"#.to_string(),
                    explanation: "List all agents".to_string(),
                },
                Example {
                    user_query: "Create a temperature monitoring agent".to_string(),
                    tool_call: r#"agent(action="create", name="Temperature Monitor", user_prompt="Check ne101 temperature every 5 minutes, alert if above 30C", schedule_type="interval", schedule_config="300")"#.to_string(),
                    explanation: "Create an interval-based monitoring agent".to_string(),
                },
                Example {
                    user_query: "Pause the temperature monitor".to_string(),
                    tool_call: r#"agent(action="control", agent_id="agent_1", control_action="pause", confirm=true)"#.to_string(),
                    explanation: "Pause agent with confirmation".to_string(),
                },
            ],
            use_when: vec![
                "User asks about agents or automations".to_string(),
                "User wants to create a monitoring or control agent".to_string(),
                "User wants to pause/resume agent execution".to_string(),
            ],
        },

        // === Agent History Tool ===
        LlmToolDefinition {
            name: "agent_history".to_string(),
            description: "Agent execution history tool. View execution stats (success rate, run count) or conversation logs (what agent did and decided). Useful for debugging agent behavior.".to_string(),
            aliases: vec!["agent history".to_string(), "agent logs".to_string(), "execution history".to_string()],
            required: vec!["action".to_string(), "agent_id".to_string()],
            optional: HashMap::from_iter(vec![
                ("limit".to_string(), ParameterInfo {
                    description: "Max conversation entries to return (conversation action). Default: 50".to_string(),
                    default: serde_json::json!(50),
                    examples: vec!["10".to_string(), "20".to_string()],
                }),
                ("response_format".to_string(), ParameterInfo {
                    description: "Output verbosity: 'concise' (summary) or 'detailed' (full details with timestamps)".to_string(),
                    default: serde_json::json!("concise"),
                    examples: vec!["concise".to_string(), "detailed".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "How is the temperature monitor performing?".to_string(),
                    tool_call: r#"agent_history(action="executions", agent_id="agent_1")"#.to_string(),
                    explanation: "View execution statistics".to_string(),
                },
                Example {
                    user_query: "What did the agent do recently?".to_string(),
                    tool_call: r#"agent_history(action="conversation", agent_id="agent_1", limit=5)"#.to_string(),
                    explanation: "View recent conversation history".to_string(),
                },
            ],
            use_when: vec![
                "User asks about agent execution history or performance".to_string(),
                "User wants to debug why an agent made a decision".to_string(),
                "User asks what an agent has been doing".to_string(),
            ],
        },

        // === Rule Tool (aggregates rule operations) ===
        LlmToolDefinition {
            name: "rule".to_string(),
            description: "Rule management tool for automation rules. Actions: list, get, create, update, delete, history. Rules trigger actions when device conditions are met. DSL format: RULE \"name\" WHEN device.metric OP value DO ACTION END".to_string(),
            aliases: vec!["rule".to_string(), "automation rule".to_string(), "trigger".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("rule_id".to_string(), ParameterInfo {
                    description: "Rule ID (get/update/delete actions). Use list action to find IDs".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["rule_1".to_string()],
                }),
                ("dsl".to_string(), ParameterInfo {
                    description: "Rule DSL definition (create/update). Example: RULE \"Low Battery\" WHEN ne101.battery < 50 DO NOTIFY \"Battery low\" END".to_string(),
                    default: serde_json::json!(null),
                    examples: vec![r#"RULE "Low Battery" WHEN ne101.battery < 50 DO NOTIFY "Battery low" END"#.to_string()],
                }),
                ("name_filter".to_string(), ParameterInfo {
                    description: "Filter rules by name substring (list action)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["battery".to_string(), "temperature".to_string()],
                }),
                ("start_time".to_string(), ParameterInfo {
                    description: "Start timestamp for history range (history action)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["1712000000".to_string()],
                }),
                ("response_format".to_string(), ParameterInfo {
                    description: "Output verbosity: 'concise' (name/status) or 'detailed' (full DSL and metadata)".to_string(),
                    default: serde_json::json!("concise"),
                    examples: vec!["concise".to_string(), "detailed".to_string()],
                }),
                ("confirm".to_string(), ParameterInfo {
                    description: "Set to true after user confirms (delete/update actions). Returns preview without confirmation".to_string(),
                    default: serde_json::json!(false),
                    examples: vec!["true".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "What rules are configured?".to_string(),
                    tool_call: r#"rule(action="list")"#.to_string(),
                    explanation: "List all rules".to_string(),
                },
                Example {
                    user_query: "Alert me when battery drops below 50%".to_string(),
                    tool_call: r#"rule(action="create", dsl="RULE \"Low Battery\" WHEN ne101.battery < 50 DO NOTIFY \"Battery below 50%\" END")"#.to_string(),
                    explanation: "Create an automation rule".to_string(),
                },
                Example {
                    user_query: "Delete rule 123".to_string(),
                    tool_call: r#"rule(action="delete", rule_id="123", confirm=true)"#.to_string(),
                    explanation: "Delete rule with confirmation".to_string(),
                },
            ],
            use_when: vec![
                "User asks about automation rules".to_string(),
                "User wants to create a rule triggered by device conditions".to_string(),
                "User wants to delete or modify a rule".to_string(),
            ],
        },

        // === Alert Tool ===
        LlmToolDefinition {
            name: "alert".to_string(),
            description: "Alert management tool. Actions: list (view alerts with filters), create (new alert), acknowledge (mark resolved). Severity levels: info, warning, error, critical.".to_string(),
            aliases: vec!["alert".to_string(), "notification".to_string(), "warning".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("alert_id".to_string(), ParameterInfo {
                    description: "Alert ID to acknowledge (acknowledge action)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["alert_1".to_string()],
                }),
                ("title".to_string(), ParameterInfo {
                    description: "Alert title (create action). Short summary".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["High Temperature Alert".to_string(), "Device Offline".to_string()],
                }),
                ("message".to_string(), ParameterInfo {
                    description: "Alert message body (create action). Detailed description".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Sensor reports 35.2C, threshold is 30C".to_string()],
                }),
                ("severity".to_string(), ParameterInfo {
                    description: "Alert severity (create action): info/warning/error/critical. Default: warning".to_string(),
                    default: serde_json::json!("warning"),
                    examples: vec!["info".to_string(), "warning".to_string(), "critical".to_string()],
                }),
                ("unacknowledged_only".to_string(), ParameterInfo {
                    description: "Only return unacknowledged alerts (list action). Default: false".to_string(),
                    default: serde_json::json!(false),
                    examples: vec!["true".to_string()],
                }),
                ("response_format".to_string(), ParameterInfo {
                    description: "Output verbosity: 'concise' (title/severity only) or 'detailed' (full info with timestamps)".to_string(),
                    default: serde_json::json!("concise"),
                    examples: vec!["concise".to_string(), "detailed".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "Are there any active alerts?".to_string(),
                    tool_call: r#"alert(action="list", unacknowledged_only=true)"#.to_string(),
                    explanation: "List unacknowledged alerts".to_string(),
                },
                Example {
                    user_query: "Acknowledge alert 123".to_string(),
                    tool_call: r#"alert(action="acknowledge", alert_id="123")"#.to_string(),
                    explanation: "Mark alert as acknowledged".to_string(),
                },
            ],
            use_when: vec![
                "User asks about alerts, notifications, or warnings".to_string(),
                "User wants to acknowledge or dismiss alerts".to_string(),
                "User wants to create a custom alert".to_string(),
            ],
        },
    ]
}

/// Format simplified tools into a prompt for the LLM.
pub fn format_tools_for_llm() -> String {
    let tools = get_simplified_tools();
    let mut prompt = String::from("## Available Tools (Aggregated Design)\n\n");

    // Concise guide
    prompt.push_str("### Usage\n\n");
    prompt.push_str("All tools use an `action` parameter to differentiate operations:\n");
    prompt.push_str("- device(action=\"list|get|query|control\", ...)\n");
    prompt.push_str("- agent(action=\"list|get|create|update|control|memory\", ...)\n");
    prompt.push_str("- agent_history(action=\"executions|conversation\", agent_id=\"...\")\n");
    prompt.push_str("- rule(action=\"list|get|create|delete|history\", ...)\n");
    prompt.push_str("- alert(action=\"list|create|acknowledge\", ...)\n\n");
    prompt.push_str(
        "Format: [{\"name\":\"tool_name\",\"arguments\":{\"action\":\"operation\",\"param\":\"value\"}}]\n\n",
    );

    for tool in tools {
        prompt.push_str(&format!("### {} - {}", tool.name, tool.description));
        if !tool.aliases.is_empty() {
            prompt.push_str(&format!(
                " [Aliases:{}]",
                tool.aliases
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        prompt.push('\n');

        // Concise parameter display
        if !tool.required.is_empty() {
            prompt.push_str(&format!("Required:{}", tool.required.join(",")));
        }
        if !tool.optional.is_empty() {
            prompt.push_str(&format!(
                " Optional:{}",
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
