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
    /// Available action values (for tools with action parameter)
    pub actions: Vec<String>,
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
/// See aggregated.rs for actual tool names.
///
/// DESIGN PRINCIPLES (based on Anthropic best practices):
/// - Fewer, more focused tools rather than many granular ones
/// - Merge similar tools to reduce LLM selection burden
/// - Prioritize high-value, high-frequency tools
/// - Each tool should be "irreducible"
///
/// Tool list (5 aggregated tools replacing 34+ individual tools):
/// - device: list, get, query, control
/// - agent: list, get, create, update, control, memory, send_message, executions, conversation, latest_execution
/// - rule: list, get, delete, history
/// - message: list, send, read/acknowledge (aliases: alert, notification)
/// - extension: list, get, execute, status
pub fn get_simplified_tools() -> Vec<LlmToolDefinition> {
    vec![
        // === Device Tool (aggregates 4 device operations) ===
        LlmToolDefinition {
            name: "device".to_string(),
            description: "Device management tool. Actions: list (list devices), latest (all current metric values), history (historical time-series data for one metric), control (send commands), write_metric (write a data point). Supports fuzzy device name matching.".to_string(),
            aliases: vec!["device".to_string(), "devices".to_string(), "sensor".to_string(), "iot".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("device_id".to_string(), ParameterInfo {
                    description: "Device ID or partial name (get/query/control). Fuzzy matching supported, e.g., 'living' matches 'Living Room Light'".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["sensor_01".to_string(), "living_room_light".to_string(), "thermostat_02".to_string()],
                }),
                ("metric".to_string(), ParameterInfo {
                    description: "Metric name (history action). Format: 'field' or 'values.field'. Examples: 'values.battery', 'temperature'".to_string(),
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
                    description: "Start timestamp for history time range (history action). Unix timestamp in seconds. Default: 1 hour ago".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["1712000000".to_string()],
                }),
                ("end_time".to_string(), ParameterInfo {
                    description: "End timestamp for history time range (history action). Default: now".to_string(),
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
                    user_query: "How is the office temperature sensor doing?".to_string(),
                    tool_call: r#"device(action="latest", device_id="office_temp_sensor")"#.to_string(),
                    explanation: "Get device's latest data with all current metric values".to_string(),
                },
                Example {
                    user_query: "What devices do I have?".to_string(),
                    tool_call: r#"device(action="list")"#.to_string(),
                    explanation: "List all devices".to_string(),
                },
                Example {
                    user_query: "What's the battery level of sensor_01?".to_string(),
                    tool_call: r#"device(action="latest", device_id="sensor_01")"#.to_string(),
                    explanation: "Get device's latest data and current metrics".to_string(),
                },
                Example {
                    user_query: "Show battery trend for today".to_string(),
                    tool_call: r#"device(action="history", device_id="sensor_01", metric="values.battery")"#.to_string(),
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
                "User asks about a device's overall status or data summary".to_string(),
                "User wants to control a device (turn on/off, adjust)".to_string(),
                "User asks for historical sensor data or trends".to_string(),
            ],
            actions: vec!["list".into(), "latest".into(), "history".into(), "control".into(), "write_metric".into()],
        },
        LlmToolDefinition {
            name: "agent".to_string(),
            description: "AI Agent management tool for creating and managing automated monitoring/control agents. Actions: list, get, create, update, control (pause/resume), memory (view learned patterns), send_message (send instruction to agent), executions (execution stats), conversation (conversation log), latest_execution (most recent execution details).".to_string(),
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
                    description: "DETAILED agent instructions (create action, REQUIRED). Write a structured prompt: what to check, thresholds, actions on trigger, output format. NOT just user's words — expand into proper agent system prompt. Example: 'You are a temperature monitoring agent. Every execution: 1) Query all temperature sensors for latest readings. 2) If any sensor reads above 30°C, send an urgent notification with device name and current value. 3) Generate a brief summary of all sensor statuses. Respond in Chinese.'".to_string(),
                    default: serde_json::json!(null),
                    examples: vec![
                        "You are a temperature monitoring agent. Every execution: 1) Query all temperature sensors for latest readings. 2) If any sensor reads above 30°C, send an urgent notification with device name and current value. 3) Generate a brief summary of all sensor statuses. Respond in Chinese.".to_string(),
                        "Check device online status. If any device goes offline, immediately send an important notification. Provide a daily health summary.".to_string(),
                    ],
                }),
                ("schedule_type".to_string(), ParameterInfo {
                    description: "How agent is triggered (create, REQUIRED): 'cron' (time schedule like daily 8am), 'interval' (every N seconds), 'event' (when device data changes). User says 'every X minutes/hours' → interval. 'daily/weekly at X' → cron. 'when X happens' → event.".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["interval".to_string(), "cron".to_string(), "event".to_string()],
                }),
                ("schedule_config".to_string(), ParameterInfo {
                    description: "Schedule config (create): For cron: 5-field expression ('0 8 * * *'=daily 8am, '*/30 * * * *'=every 30min). For interval: seconds ('300'=5min, '3600'=1hour). For event: comma-separated DataSourceIds to watch ('device:sensor_001:temperature,extension:weather:humidity')".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["0 8 * * *".to_string(), "300".to_string(), "3600".to_string(), "device:sensor_001:temperature".to_string()],
                }),
                ("execution_mode".to_string(), ParameterInfo {
                    description: "Agent mode (create): 'chat' = single-pass (default, for monitoring/reporting), 'react' = multi-round tool loop (for automation needing device control or multi-step actions)".to_string(),
                    default: serde_json::json!("chat"),
                    examples: vec!["chat".to_string(), "react".to_string()],
                }),
                ("resources".to_string(), ParameterInfo {
                    description: "Resources to bind (create, multi-select, finest granularity preferred). JSON array: [{\"type\":\"...\",\"id\":\"...\"}]. Types: 'device' (id=device_id), 'metric' (id='device_id:metric_name'), 'command' (id='device_id:cmd'), 'extension_metric' (id='extension:ext_id:metric'), 'extension_tool' (id='extension:ext_id:tool'). Prefer specific metrics over whole devices. Example: [{\"type\":\"metric\",\"id\":\"sensor_001:temperature\"},{\"type\":\"extension_tool\",\"id\":\"extension:weather:forecast\"}]".to_string(),
                    default: serde_json::json!(null),
                    examples: vec![
                        "[{\"type\":\"metric\",\"id\":\"sensor_001:temperature\"}]".to_string(),
                        "[{\"type\":\"device\",\"id\":\"camera_001\"},{\"type\":\"extension_tool\",\"id\":\"extension:image_analyzer:detect\"}]".to_string(),
                    ],
                }),
                ("enable_tool_chaining".to_string(), ParameterInfo {
                    description: "Allow tool output chaining in react mode (create, optional). Default: false. Set true for complex automation.".to_string(),
                    default: serde_json::json!(false),
                    examples: vec!["true".to_string(), "false".to_string()],
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
                ("message".to_string(), ParameterInfo {
                    description: "Message content to send to the agent (send_message action). The agent will see this in its next execution. Example: 'Focus on monitoring the front door area'".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Focus on monitoring the front door area".to_string(), "Switch to monitoring humidity instead".to_string()],
                }),
                ("message_type".to_string(), ParameterInfo {
                    description: "Optional message type/tag for categorization (send_message action). Example: 'instruction', 'correction', 'update'".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["instruction".to_string(), "correction".to_string(), "update".to_string()],
                }),
                ("confirm".to_string(), ParameterInfo {
                    description: "Set to true after user confirms (control action). Returns preview without confirmation".to_string(),
                    default: serde_json::json!(false),
                    examples: vec!["true".to_string()],
                }),
                ("limit".to_string(), ParameterInfo {
                    description: "Max conversation entries to return (conversation action). Default: 50".to_string(),
                    default: serde_json::json!(50),
                    examples: vec!["10".to_string(), "20".to_string()],
                }),
                ("history_format".to_string(), ParameterInfo {
                    description: "Output verbosity for history actions (executions/conversation/latest_execution): 'concise' (summary) or 'detailed' (full details with timestamps)".to_string(),
                    default: serde_json::json!("concise"),
                    examples: vec!["concise".to_string(), "detailed".to_string()],
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
                    tool_call: r#"agent(action="create", name="Temperature Monitor", user_prompt="You are a temperature monitoring agent. Every execution: 1) Query all temperature sensors for latest readings. 2) If any sensor reads above 30C, send an urgent notification. 3) Provide a brief status summary.", schedule_type="interval", schedule_config="300")"#.to_string(),
                    explanation: "Create an interval-based monitoring agent with detailed prompt".to_string(),
                },
                Example {
                    user_query: "Create a daily patrol agent that runs at 8am".to_string(),
                    tool_call: r#"agent(action="create", name="Daily Patrol", user_prompt="You are a daily device patrol agent. Check all devices status, verify online/offline, report any anomalies. Send a summary notification.", schedule_type="cron", schedule_config="0 8 * * *", execution_mode="react", enable_tool_chaining=true)"#.to_string(),
                    explanation: "Create a cron-based agent with react mode for multi-step automation".to_string(),
                },
                Example {
                    user_query: "Pause the temperature monitor".to_string(),
                    tool_call: r#"agent(action="control", agent_id="agent_1", control_action="pause", confirm=true)"#.to_string(),
                    explanation: "Pause agent with confirmation".to_string(),
                },
                Example {
                    user_query: "How is the temperature monitor performing?".to_string(),
                    tool_call: r#"agent(action="executions", agent_id="agent_1")"#.to_string(),
                    explanation: "View execution statistics".to_string(),
                },
            ],
            use_when: vec![
                "User asks about agents or automations".to_string(),
                "User wants to create a monitoring or control agent".to_string(),
                "User wants to pause/resume agent execution".to_string(),
                "User wants to send an instruction or message to an agent".to_string(),
                "User wants to guide, correct, or update an agent's behavior".to_string(),
                "User asks about agent execution history or performance".to_string(),
                "User wants to debug why an agent made a decision".to_string(),
                "User asks about the latest execution result or whether it succeeded".to_string(),
            ],
            actions: vec!["list".into(), "get".into(), "create".into(), "update".into(), "control".into(), "memory".into(), "send_message".into(), "executions".into(), "conversation".into(), "latest_execution".into()],
        },

        // === Rule Tool (aggregates rule operations) ===
        LlmToolDefinition {
            name: "rule".to_string(),
            description: "Rule management tool for automation rules. Actions: list (with status), get, create, update, delete, history, enable (pause/resume). Rules use DSL: RULE \"name\" WHEN condition [FOR duration] DO actions END. Conditions: device/extension metrics, BETWEEN range, AND/OR/NOT logic. Actions: NOTIFY, EXECUTE, SET, LOG, ALERT, HTTP, DELAY.".to_string(),
            aliases: vec!["rule".to_string(), "automation rule".to_string(), "trigger".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("rule_id".to_string(), ParameterInfo {
                    description: "Rule ID (get/update/delete/enable actions). Use list action to find IDs".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["rule_1".to_string()],
                }),
                ("dsl".to_string(), ParameterInfo {
                    description: "Rule DSL definition (create/update). Syntax: RULE \"name\" WHEN condition [FOR duration] DO actions END. Conditions: device.metric OP value, EXTENSION ext.metric OP value, BETWEEN min AND max, AND/OR/NOT. Actions: NOTIFY \"msg\", EXECUTE dev.cmd(k=v), SET dev.prop=v, LOG level \"msg\", ALERT \"title\" \"msg\", HTTP METHOD url, DELAY duration.".to_string(),
                    default: serde_json::json!(null),
                    examples: vec![
                        r#"RULE "Low Battery" WHEN sensor_01.battery < 20 DO NOTIFY "Battery critical" END"#.to_string(),
                        r#"RULE "Temp Control" WHEN sensor_01.temperature > 30 FOR 5 minutes DO SET ac_01.power = "on" END"#.to_string(),
                        r#"RULE "Weather Alert" WHEN EXTENSION weather.temperature > 35 DO ALERT "Heat Wave" "Hot" severity=CRITICAL END"#.to_string(),
                        r#"RULE "Safety" WHEN (smoke_01.level > 50) AND (temp_01.temperature > 60) DO EXECUTE alarm_01.trigger(mode=emergency) END"#.to_string(),
                    ],
                }),
                ("enabled".to_string(), ParameterInfo {
                    description: "For 'enable' action: true to resume rule, false to pause it (default: true)".to_string(),
                    default: serde_json::json!(true),
                    examples: vec!["true".to_string(), "false".to_string()],
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
                    explanation: "List all rules with status".to_string(),
                },
                Example {
                    user_query: "Alert me when battery drops below 20%".to_string(),
                    tool_call: r#"rule(action="create", dsl="RULE \"Low Battery\" WHEN sensor_01.battery < 20 DO NOTIFY \"Battery critical\" END")"#.to_string(),
                    explanation: "Create a device condition rule with NOTIFY action".to_string(),
                },
                Example {
                    user_query: "Turn on AC when temperature is above 30 for 5 minutes".to_string(),
                    tool_call: r#"rule(action="create", dsl="RULE \"Temp Control\" WHEN sensor_01.temperature > 30 FOR 5 minutes DO SET ac_01.power = \"on\" END")"#.to_string(),
                    explanation: "Create rule with FOR duration and SET action".to_string(),
                },
                Example {
                    user_query: "Create a weather alert rule".to_string(),
                    tool_call: r#"rule(action="create", dsl="RULE \"Weather Alert\" WHEN EXTENSION weather.temperature > 35 DO ALERT \"Heat Wave\" \"Too hot\" severity=CRITICAL END")"#.to_string(),
                    explanation: "Create rule with Extension condition and ALERT action".to_string(),
                },
                Example {
                    user_query: "Disable the low battery rule".to_string(),
                    tool_call: r#"rule(action="enable", rule_id="Low Battery Alert", enabled=false)"#.to_string(),
                    explanation: "Pause a rule by setting enabled=false".to_string(),
                },
                Example {
                    user_query: "Delete rule 123".to_string(),
                    tool_call: r#"rule(action="delete", rule_id="123", confirm=true)"#.to_string(),
                    explanation: "Delete rule with confirmation".to_string(),
                },
            ],
            use_when: vec![
                "User asks about automation rules or triggers".to_string(),
                "User wants to create a rule triggered by device or extension conditions".to_string(),
                "User wants to delete or modify a rule".to_string(),
                "User wants to pause or resume a rule".to_string(),
                "User wants to control devices automatically based on conditions".to_string(),
                "User wants multi-condition logic (AND/OR) for automation".to_string(),
            ],
            actions: vec!["list".into(), "get".into(), "create".into(), "update".into(), "delete".into(), "history".into(), "enable".into()],
        },

        // === Message Tool ===
        LlmToolDefinition {
            name: "message".to_string(),
            description: "Message, alert and notification tool. Actions: list (view messages with filters), get (get message details), send (new message/alert), read (mark as read/acknowledge). Priority levels: info, notice, important, urgent.".to_string(),
            aliases: vec!["message".to_string(), "alert".to_string(), "notification".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("message_id".to_string(), ParameterInfo {
                    description: "Message ID for read/get action".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["msg_1".to_string()],
                }),
                ("title".to_string(), ParameterInfo {
                    description: "Message title (send action). Short summary".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Device Offline".to_string(), "Battery Low".to_string()],
                }),
                ("message".to_string(), ParameterInfo {
                    description: "Message body (send action). Detailed description".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Sensor reports 35.2C, threshold is 30C".to_string()],
                }),
                ("level".to_string(), ParameterInfo {
                    description: "Priority level (send action): info/notice/important/urgent. Default: notice".to_string(),
                    default: serde_json::json!("notice"),
                    examples: vec!["info".to_string(), "notice".to_string(), "urgent".to_string()],
                }),
                ("unacknowledged_only".to_string(), ParameterInfo {
                    description: "Only return unread messages (list action). Default: false".to_string(),
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
                    user_query: "Are there any unread messages?".to_string(),
                    tool_call: r#"message(action="list", unacknowledged_only=true)"#.to_string(),
                    explanation: "List unread messages".to_string(),
                },
                Example {
                    user_query: "Mark message 123 as read".to_string(),
                    tool_call: r#"message(action="read", message_id="123")"#.to_string(),
                    explanation: "Mark message as read".to_string(),
                },
                Example {
                    user_query: "Send an urgent alert about device offline".to_string(),
                    tool_call: r#"message(action="send", title="Device Offline", message="Sensor #5 is not responding", level="urgent")"#.to_string(),
                    explanation: "Send an urgent message/alert".to_string(),
                },
            ],
            use_when: vec![
                "User asks about messages, alerts, or notifications".to_string(),
                "User wants to acknowledge, dismiss, or read messages".to_string(),
                "User wants to send a message or create an alert".to_string(),
            ],
            actions: vec!["list".into(), "get".into(), "send".into(), "read".into()],
        },

        // === Extension Tool (management only - execute via direct extension-id:command format) ===
        LlmToolDefinition {
            name: "extension".to_string(),
            description: "Extension (plugin) management tool. Actions: list (show extensions), get (extension details and commands), status (health check). To execute extension commands, first use list/get to discover available extensions, then call directly: extension-id:command(params)".to_string(),
            aliases: vec!["extension".to_string(), "plugin".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("extension_id".to_string(), ParameterInfo {
                    description: "Extension ID (get/status actions). Use list first to discover available extensions".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["use list action to get real IDs".to_string()],
                }),
                ("response_format".to_string(), ParameterInfo {
                    description: "Output verbosity: 'concise' (summary only) or 'detailed' (full info)".to_string(),
                    default: serde_json::json!("concise"),
                    examples: vec!["concise".to_string(), "detailed".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "What extensions are installed?".to_string(),
                    tool_call: r#"extension(action="list")"#.to_string(),
                    explanation: "List all installed extensions".to_string(),
                },
                Example {
                    user_query: "What can extension X do?".to_string(),
                    tool_call: r#"extension(action="get", extension_id="<id from list>")"#.to_string(),
                    explanation: "Get extension details including available commands".to_string(),
                },
                Example {
                    user_query: "Is extension X healthy?".to_string(),
                    tool_call: r#"extension(action="status", extension_id="<id from list>")"#.to_string(),
                    explanation: "Check extension health and status".to_string(),
                },
            ],
            use_when: vec![
                "User asks about installed extensions, plugins, or add-ons".to_string(),
                "User wants to check if an extension is working properly".to_string(),
            ],
            actions: vec!["list".into(), "get".into(), "status".into()],
        },

        // === Transform Tool (data transformation rules) ===
        LlmToolDefinition {
            name: "transform".to_string(),
            description: "Data transformation tool. Creates JavaScript-based transforms that process raw device data into new metrics. Actions: list, get, create, update, delete, test. JS code receives `input` (device data) and can use `extensions.invoke()` for external API data. Return object → metrics named {prefix}.{key}. Scope: global, device_type:Type, device:DeviceId".to_string(),
            aliases: vec!["transform".to_string(), "数据转换".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("id".to_string(), ParameterInfo {
                    description: "Transform ID (for get/update/delete/test)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["transform_xxx".to_string()],
                }),
                ("name".to_string(), ParameterInfo {
                    description: "Display name (required for create)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Temperature Conversion".to_string()],
                }),
                ("description".to_string(), ParameterInfo {
                    description: "What the transform does".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Converts Celsius to Fahrenheit".to_string()],
                }),
                ("scope".to_string(), ParameterInfo {
                    description: "Scope: global, device_type:TypeName, device:DeviceId".to_string(),
                    default: serde_json::json!("global"),
                    examples: vec!["global".to_string(), "device_type:temperature_sensor".to_string(), "device:sensor_1".to_string()],
                }),
                ("intent".to_string(), ParameterInfo {
                    description: "Natural language description of transformation goal".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Count detections by class".to_string()],
                }),
                ("js_code".to_string(), ParameterInfo {
                    description: "JavaScript code. Receives `input`. Return value becomes metrics. Can use extensions.invoke() for external data.".to_string(),
                    default: serde_json::json!(null),
                    examples: vec![
                        "return (input.temperature * 9/5) + 32".to_string(),
                        "const c={}; for(const i of input.detections||[]){c[i.cls]=(c[i.cls]||0)+1} return c".to_string(),
                    ],
                }),
                ("output_prefix".to_string(), ParameterInfo {
                    description: "Prefix for output metric names (default: transform)".to_string(),
                    default: serde_json::json!("transform"),
                    examples: vec!["temp_conv".to_string(), "detection_count".to_string()],
                }),
                ("enabled".to_string(), ParameterInfo {
                    description: "Enable or disable the transform".to_string(),
                    default: serde_json::json!(true),
                    examples: vec!["true".to_string(), "false".to_string()],
                }),
                ("input_data".to_string(), ParameterInfo {
                    description: "Test input data (for test action only)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec![r#"{"temperature": 25}"#.to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "List all transforms".to_string(),
                    tool_call: r#"transform(action="list")"#.to_string(),
                    explanation: "List all data transformation rules".to_string(),
                },
                Example {
                    user_query: "Create a transform to convert Celsius to Fahrenheit".to_string(),
                    tool_call: r#"transform(action="create", name="Celsius to Fahrenheit", scope="global", js_code="return (input.temperature * 9/5) + 32", output_prefix="temp_conv")"#.to_string(),
                    explanation: "Create a simple unit conversion transform".to_string(),
                },
                Example {
                    user_query: "Create a transform that counts YOLO detections by class".to_string(),
                    tool_call: r#"transform(action="create", name="Detection Counter", scope="global", js_code="const c={}; for(const i of input.detections||[]){c[i.cls||'x']=(c[i.cls||'x']||0)+1} return c", output_prefix="det_count")"#.to_string(),
                    explanation: "Count detections by class name".to_string(),
                },
                Example {
                    user_query: "Create a transform that fetches weather and compares with device temperature".to_string(),
                    tool_call: r#"transform(action="create", name="Temp Comparison", scope="global", js_code="const w=extensions.invoke('weather.ext','get_current',{location:'Beijing'}); return {temp_diff: input.temp - w.temp_c, outdoor: w.temp_c}", output_prefix="weather")"#.to_string(),
                    explanation: "Use extension to get external weather data and compare".to_string(),
                },
                Example {
                    user_query: "Update transform output prefix to temp_conv".to_string(),
                    tool_call: r#"transform(action="update", id="transform_xxx", output_prefix="temp_conv")"#.to_string(),
                    explanation: "Update a transform's output prefix".to_string(),
                },
                Example {
                    user_query: "Delete that transform".to_string(),
                    tool_call: r#"transform(action="delete", id="transform_xxx")"#.to_string(),
                    explanation: "Delete a transform by ID".to_string(),
                },
            ],
            use_when: vec![
                "convert units".to_string(),
                "process data".to_string(),
                "calculate derived metrics".to_string(),
                "transform sensor data".to_string(),
                "call extension API".to_string(),
                "aggregate detection data".to_string(),
                "create data transformation rule".to_string(),
                "manage transforms".to_string(),
            ],
            actions: vec!["list".into(), "get".into(), "create".into(), "update".into(), "delete".into(), "test".into()],
        },

        // === Skill Tool (operation guides & skill management) ===
        LlmToolDefinition {
            name: "skill".to_string(),
            description: "Query and manage operation guides (skills). Search for relevant step-by-step guides before complex operations, or create/update/delete user-defined skills. Skills contain best practices and tool call examples for specific scenarios.".to_string(),
            aliases: vec!["skill".to_string(), "skills".to_string(), "guide".to_string(), "指南".to_string(), "技能".to_string()],
            required: vec!["action".to_string()],
            optional: HashMap::from_iter(vec![
                ("query".to_string(), ParameterInfo {
                    description: "Search query for keyword matching (search action). Example: 'delete rule', 'device control'".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["删除规则".to_string(), "device control".to_string(), "create agent".to_string()],
                }),
                ("id".to_string(), ParameterInfo {
                    description: "Skill ID for exact lookup, update, or delete".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["rule-management".to_string(), "device-control".to_string()],
                }),
                ("content".to_string(), ParameterInfo {
                    description: "Full skill file content for create/update. Format: YAML frontmatter (---) + Markdown body. Only id and name are required in frontmatter. Body can be empty.".to_string(),
                    default: serde_json::json!(null),
                    examples: vec![
                        "---\nid: my-skill\nname: My Skill\ncategory: general\ntriggers:\n  keywords: [keyword1]\n---\n\n# My Skill\n\nStep-by-step guide.".to_string(),
                        "---\nid: minimal\nname: Minimal\n---\n".to_string(),
                    ],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "Search for guides about rule management".to_string(),
                    tool_call: r#"skill(action="search", query="rule management")"#.to_string(),
                    explanation: "Search for relevant skill guides by keywords".to_string(),
                },
                Example {
                    user_query: "List all available skills".to_string(),
                    tool_call: r#"skill(action="list")"#.to_string(),
                    explanation: "List all available operation guides".to_string(),
                },
                Example {
                    user_query: "Get the full content of a specific skill".to_string(),
                    tool_call: r#"skill(action="get", id="rule-management")"#.to_string(),
                    explanation: "Get full skill content by ID".to_string(),
                },
                Example {
                    user_query: "Create a new skill guide".to_string(),
                    tool_call: r#"skill(action="create", content="---\nid: my-guide\nname: My Guide\ncategory: general\npriority: 50\ntoken_budget: 500\ntriggers:\n  keywords: [my keyword, example]\nanti_triggers:\n  keywords: []\n---\n\n# My Guide\n\nStep-by-step instructions here.")"#.to_string(),
                    explanation: "Create a new user-defined skill. Only id and name are required; body can be empty for minimal skills.".to_string(),
                },
            ],
            use_when: vec![
                "user asks about available skills or guides".to_string(),
                "user wants to create a skill or guide".to_string(),
                "need step-by-step instructions for complex operations".to_string(),
                "user asks what skills or capabilities are available".to_string(),
                "search for best practices before executing complex workflows".to_string(),
            ],
            actions: vec!["search".into(), "list".into(), "get".into(), "create".into(), "update".into(), "delete".into()],
        },

        // === Shell Tool (system command execution) ===
        LlmToolDefinition {
            name: "shell".to_string(),
            description: "Execute shell commands on the host system. Network diagnostics, system monitoring, file inspection, device discovery, container management.".to_string(),
            aliases: vec!["shell".to_string(), "cli".to_string(), "command".to_string(), "终端".to_string(), "命令行".to_string()],
            required: vec!["command".to_string()],
            optional: HashMap::from_iter(vec![
                ("timeout".to_string(), ParameterInfo {
                    description: "Per-command timeout in seconds (max 600, default: 30)".to_string(),
                    default: serde_json::json!(30),
                    examples: vec!["10".to_string(), "60".to_string()],
                }),
                ("working_dir".to_string(), ParameterInfo {
                    description: "Working directory for command execution".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["/tmp".to_string(), "/home/user".to_string()],
                }),
                ("description".to_string(), ParameterInfo {
                    description: "Brief description of what this command does (for logging)".to_string(),
                    default: serde_json::json!(null),
                    examples: vec!["Check disk usage".to_string(), "Ping gateway".to_string()],
                }),
            ]),
            examples: vec![
                Example {
                    user_query: "Check network connectivity to 192.168.1.1".to_string(),
                    tool_call: r#"shell(command="ping -c 3 192.168.1.1")"#.to_string(),
                    explanation: "Ping a device to check network connectivity".to_string(),
                },
                Example {
                    user_query: "Show disk usage".to_string(),
                    tool_call: r#"shell(command="df -h")"#.to_string(),
                    explanation: "Check disk space usage".to_string(),
                },
                Example {
                    user_query: "List running Docker containers".to_string(),
                    tool_call: r#"shell(command="docker ps")"#.to_string(),
                    explanation: "List active containers".to_string(),
                },
                Example {
                    user_query: "Find devices on the local network".to_string(),
                    tool_call: r#"shell(command="arp -a")"#.to_string(),
                    explanation: "List devices in ARP table".to_string(),
                },
            ],
            use_when: vec![
                "network diagnostics (ping, traceroute, curl)".to_string(),
                "system monitoring (ps, df, top, systemctl)".to_string(),
                "file inspection (ls, cat, grep, find)".to_string(),
                "device discovery (arp, avahi-browse, bluetoothctl)".to_string(),
                "container management (docker, podman)".to_string(),
                "user explicitly asks to run a command".to_string(),
            ],
            actions: vec![], // shell has no action parameter
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
    prompt.push_str("- device(action=\"list|latest|get|history|control|write_metric\", ...)\n");
    prompt.push_str("- agent(action=\"list|get|create|update|control|memory|send_message|executions|conversation|latest_execution\", ...) — use send_message to contact an agent\n");
    prompt.push_str("- rule(action=\"list|get|create|update|delete|history\", ...)\n");
    prompt.push_str("- message(action=\"list|send|read\", ...) — for system messages/notifications only, NOT for contacting agents\n");
    prompt.push_str("- extension(action=\"list|get|status\", ...)\n");
    prompt.push_str("- transform(action=\"list|get|create|update|delete|test\", ...)\n");
    prompt.push_str("- skill(action=\"search|list|get|create|update|delete\", ...) — operation guides & skill management\n");
    prompt.push_str("- shell(command=\"...\") — execute system commands (network, disk, processes, files)\n\n");
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
        assert!(prompt.contains("message"));
    }

    #[test]
    fn test_get_simplified_tools_count() {
        let tools = get_simplified_tools();
        // Should have 8 aggregated tools (device, agent, rule, message, extension, transform, skill, shell)
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn test_aggregated_tool_names() {
        let tools = get_simplified_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"device"));
        assert!(names.contains(&"agent"));
        assert!(names.contains(&"rule"));
        assert!(names.contains(&"message"));
        assert!(names.contains(&"extension"));
        assert!(names.contains(&"transform"));
        assert!(names.contains(&"skill"));
        assert!(names.contains(&"shell"));
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
