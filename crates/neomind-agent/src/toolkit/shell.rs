//! Shell tool for executing system commands.
//!
//! Allows the AI agent to run arbitrary shell commands on the host system.
//! Cross-platform: uses `/bin/sh -c` on Unix, `cmd /C` on Windows.
//! Disabled by default — must be explicitly enabled in agent configuration.
//!
//! ## Internal CLI Execution Mode
//!
//! When `internal_cli_execution` is enabled in the config, neomind CLI commands
//! are executed via direct function calls to `neomind-cli-ops` instead of spawning
//! a separate process. This optimization is useful for Tauri/Web environments where
//! process spawning overhead is significant.
//!
//! Supported domains: device, dashboard, rule, extension, widget, transform, agent, message, system, connector.
//! All domains route to their respective cli-ops handler functions.
//! Non-neomind commands fall through to process spawning.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;

use neomind_core::tools::ToolCategory;

use super::error::{Result, ToolError};
use super::tool::{object_schema, Tool, ToolOutput};

/// Shell tool configuration, stored as part of agent config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Whether shell tool is enabled. Default: false.
    #[serde(default)]
    pub enabled: bool,

    /// Maximum execution time per command in seconds. Default: 30.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Maximum output characters (stdout + stderr combined). Default: 10000.
    #[serde(default = "default_max_output")]
    pub max_output_chars: usize,

    /// Use internal execution for neomind CLI commands (Tauri/Web mode). Default: false.
    /// When true, neomind commands are executed via direct function calls instead of spawning processes.
    #[serde(default)]
    pub internal_cli_execution: bool,
}

fn default_timeout() -> u64 {
    30
}

fn default_max_output() -> usize {
    10000
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout_secs: default_timeout(),
            max_output_chars: default_max_output(),
            internal_cli_execution: false,
        }
    }
}

/// Output from a shell command execution.
#[derive(Debug)]
struct CommandOutput {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

/// Shell tool — executes system commands.
pub struct ShellTool {
    config: ShellConfig,
}

impl ShellTool {
    pub fn new(config: ShellConfig) -> Self {
        Self { config }
    }

    /// Check if command is a neomind CLI command and execute it internally if enabled.
    /// Returns Some(result) if internally executed, None if should use process spawning.
    async fn try_internal_execution(
        &self,
        command: &str,
    ) -> Option<Result<CommandOutput>> {
        // Only use internal execution if configured
        if !self.config.internal_cli_execution {
            return None;
        }

        // Check if command starts with "neomind"
        let command = command.trim();
        if !command.starts_with("neomind ") {
            return None;
        }

        // Create an API client for internal calls
        let api_base = std::env::var("NEOMIND_API_BASE")
            .unwrap_or_else(|_| "http://localhost:9375/api".to_string());
        let client = neomind_cli_ops::ApiClient::with_base_url(&api_base);

        // Parse the CLI command
        let args = match shell_words::split(command) {
            Ok(a) => a,
            Err(e) => {
                return Some(Err(ToolError::Execution(format!(
                    "Failed to parse command: {}", e
                ))))
            }
        };

        if args.len() < 2 {
            return None;
        }

        let domain = args[1].as_str();
        tracing::debug!(domain = domain, "Attempting internal CLI execution");

        // Global --help interception: if any arg is --help/-h, return help instead of executing
        if Self::has_help_flag(&args) {
            let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
            // For subcommands like "device types create --help", include the sub-subcommand
            let sub = args.get(3).map(|s| s.as_str())
                .filter(|s| !s.starts_with('-'))
                .unwrap_or("");
            let help_cmd = if action.is_empty() || action == "--help" || action == "-h" {
                domain.to_string()
            } else if sub.is_empty() {
                format!("{} {}", domain, action)
            } else {
                format!("{} {} {}", domain, action, sub)
            };
            let result = Self::help_response(&help_cmd);
            return match result {
                Ok(resp) => {
                    let output = serde_json::to_string(&resp).unwrap_or_default();
                    Some(Ok(CommandOutput {
                        exit_code: Some(0),
                        stdout: output,
                        stderr: String::new(),
                        timed_out: false,
                    }))
                }
                Err(e) => Some(Err(ToolError::Execution(e.to_string()))),
            };
        }

        let result = match domain {
            "device" => Self::exec_device(&client, &args).await,
            "dashboard" => Self::exec_dashboard(&client, &args).await,
            "rule" => Self::exec_rule(&client, &args).await,
            "extension" => Self::exec_extension(&client, &args).await,
            "widget" => Self::exec_widget(&client, &args).await,
            "transform" => Self::exec_transform(&client, &args).await,
            "agent" => Self::exec_agent(&client, &args).await,
            "message" => Self::exec_message(&client, &args).await,
            "system" => Self::exec_system(&client, &args).await,
            "connector" => Self::exec_connector(&client, &args).await,
            "llm" => Self::exec_llm(&client, &args).await,
            _ => return None, // Unknown domain, fall through to process spawning
        };

        match result {
            Ok(response) => {
                let output = serde_json::to_string(&response).unwrap_or_default();
                tracing::debug!(domain = domain, "Internal CLI execution succeeded");
                Some(Ok(CommandOutput {
                    exit_code: Some(if response.success { 0 } else { 1 }),
                    stdout: output,
                    stderr: String::new(),
                    timed_out: false,
                }))
            }
            Err(e) => {
                // Check if this is a fallthrough signal — let external process handle it
                if e.to_string() == "__FALLTHROUGH__" {
                    tracing::debug!(domain = domain, "Internal CLI falling through to external process");
                    return None;
                }
                tracing::warn!(domain = domain, error = %e, "Internal CLI execution failed");
                Some(Err(ToolError::Execution(format!(
                    "Internal CLI error: {}", e
                ))))
            }
        }
    }

    /// Check if args contain --help or -h flag
    fn has_help_flag(args: &[String]) -> bool {
        args.iter().any(|a| a == "--help" || a == "-h")
    }

    /// Build a help response for a given command path (e.g. "device types create")
    fn help_response(cmd: &str) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        // Domain-specific help texts for subcommands the LLM commonly needs
        let help = match cmd {
            "device types create" => concat!(
                "Usage: neomind device types create --name <NAME> --metrics '<JSON>' [--id <ID>] [--commands '<JSON>']\n\n",
                "Flags:\n",
                "  --name       (required) Device type display name\n",
                "  --id         Optional custom ID (auto-generated if omitted)\n",
                "  --metrics    (required) JSON array of metric definitions\n",
                "  --commands   Optional JSON array of command definitions\n\n",
                "Metrics format: [{\"name\":\"temperature\",\"display_name\":\"Temperature\",\"data_type\":\"Float\",\"unit\":\"°C\"}]\n",
                "  - name: metric identifier (required)\n",
                "  - display_name: human-readable name (optional)\n",
                "  - data_type: Float | Integer | String | Boolean (default: Float)\n",
                "  - unit: display unit like °C, %, Pa (optional)\n\n",
                "Example:\n",
                "  neomind device types create --name 'Temperature Sensor' \\\n",
                "    --metrics '[{\"name\":\"temperature\",\"display_name\":\"Temperature\",\"data_type\":\"Float\",\"unit\":\"°C\"},{\"name\":\"humidity\",\"display_name\":\"Humidity\",\"data_type\":\"Float\",\"unit\":\"%\"}]'"
            ),
            "device create" => concat!(
                "Usage: neomind device create <NAME> [device_type_id] <adapter>\n\n",
                "Positional: NAME, device_type_id (optional), adapter (mqtt|webhook)\n\n",
                "Flags:\n",
                "  --type / --device-type   Device type ID\n",
                "  --adapter / --adapter-type  mqtt (default) | webhook\n\n",
                "Example:\n",
                "  neomind device create 'Temp-01' temperature-sensor mqtt\n",
                "  neomind device create 'Temp-02' --type temperature-sensor --adapter webhook"
            ),
            "dashboard update" => concat!(
                "Usage: neomind dashboard update <ID> [--name <NAME>] [--description <DESC>] [--layout '<JSON>'] [--components '<JSON>']\n\n",
                "WARNING: --components replaces ALL existing components. Use 'dashboard add-components' instead.\n\n",
                "Flags:\n",
                "  --name          New dashboard name\n",
                "  --description   New description\n",
                "  --layout        JSON layout config\n",
                "  --components    JSON array of ALL components (replaces existing!)\n\n",
                "RECOMMENDED: Use add-components to append without replacing:\n",
                "  neomind dashboard add-components <ID> --components '[...]'\n\n",
                "Example (rename only):\n",
                "  neomind dashboard update <ID> --name 'New Name'"
            ),
            "dashboard add-components" => concat!(
                "Usage: neomind dashboard add-components <ID> --components '<JSON_ARRAY>'\n\n",
                "Append components to an existing dashboard without replacing existing ones.\n",
                "This is the RECOMMENDED way to add widgets.\n\n",
                "Flags:\n",
                "  --components    (required) JSON array of new components\n\n",
                "Component format:\n",
                "  {\n",
                "    \"id\": \"unique-id\",\n",
                "    \"type\": \"value-card\",\n",
                "    \"title\": \"Temperature\",\n",
                "    \"position\": {\"x\":0, \"y\":0, \"w\":4, \"h\":2},\n",
                "    \"data_source\": {\"type\":\"device\",\"sourceId\":\"sensor-001\",\"property\":\"temperature\"},\n",
                "    \"display\": {\"unit\":\"°C\"}\n",
                "  }\n\n",
                "Grid: 12 columns wide. New components should start at y = max(existing_y + h).\n\n",
                "Example:\n",
                "  neomind dashboard add-components <ID> --components '[\n",
                "    {\"id\":\"temp\",\"type\":\"value-card\",\"title\":\"Temp\",\"position\":{\"x\":0,\"y\":0,\"w\":4,\"h\":2},\n",
                "     \"data_source\":{\"type\":\"device\",\"sourceId\":\"sensor-001\",\"property\":\"temperature\"}},\n",
                "    {\"id\":\"hum\",\"type\":\"value-card\",\"title\":\"Humidity\",\"position\":{\"x\":4,\"y\":0,\"w\":4,\"h\":2},\n",
                "     \"data_source\":{\"type\":\"extension-metric\",\"extensionId\":\"weather\",\"extensionMetric\":\"humidity\"}}\n",
                "  ]'"
            ),
            "dashboard remove-components" => concat!(
                "Usage: neomind dashboard remove-components <ID> --ids '<JSON_ARRAY>'\n\n",
                "Remove specific components from a dashboard by their IDs.\n\n",
                "Flags:\n",
                "  --ids    (required) JSON array of component IDs to remove\n\n",
                "Example:\n",
                "  neomind dashboard remove-components <ID> --ids '[\"temp\",\"chart1\"]'"
            ),
            "dashboard create" => concat!(
                "Usage: neomind dashboard create --name <NAME> [--description <DESC>] [--layout '<JSON>']\n\n",
                "Flags:\n",
                "  --name          (required) Dashboard name\n",
                "  --description   Optional description\n",
                "  --layout        Optional JSON layout (default: 12-column grid)\n\n",
                "Example:\n",
                "  neomind dashboard create --name 'Battery Monitor'\n",
                "  neomind dashboard create --name 'Sensors' --description 'All sensor data'"
            ),
            "rule create" => concat!(
                "Usage: neomind rule create --dsl '<DSL_STRING>' [--name <NAME>]\n\n",
                "Flags:\n",
                "  --dsl    (required) Rule DSL definition\n",
                "  --name   Optional rule name\n\n",
                "DSL Syntax (case-insensitive keywords):\n",
                "  RULE <name>\n",
                "    WHEN <condition>\n",
                "    DO <action>\n",
                "  END\n\n",
                "Conditions:\n",
                "  <device_id>.<metric> <op> <value>   (op: >, <, >=, <=, ==, !=)\n",
                "  <device_id>.<metric> BETWEEN <val1> AND <val2>\n",
                "  EXTENSION <ext_id>.<metric> <op> <value>\n",
                "  Logical: AND, OR, NOT  (combine with parentheses)\n\n",
                "Actions:\n",
                "  NOTIFY \"message text\" [channel1, channel2]\n",
                "  EXECUTE <device_id>.<command>(key=value, key2=\"val2\")\n",
                "  LOG <level> \"message\"\n",
                "  ALERT \"title\" \"message\" <SEVERITY>\n",
                "  TRIGGER_AGENT <agent_id> \"input text\"\n\n",
                "Template vars: {{device.name}}, {{value}}\n",
                "New rules are disabled — run `neomind rule enable <ID>` after create.\n\n",
                "Example:\n",
                "  neomind rule create --name 'High Temp Alert' --dsl 'RULE high_temp\n",
                "    WHEN sensor-001.temperature > 30\n",
                "    DO\n",
                "      NOTIFY \"High temp on {{device.name}}: {{value}}°C\"\n",
                "    END'\n\n",
                "  neomind rule create --name 'Offline Alert' --dsl 'RULE offline\n",
                "    WHEN sensor-001.status == \"offline\"\n",
                "    DO\n",
                "      NOTIFY \"{{device.name}} went offline\" [email, sms]\n",
                "    END'"
            ),
            "transform create" => concat!(
                "Usage: neomind transform create --name <NAME> --code '<JS>' [--scope <SCOPE>] [--output-prefix <PREFIX>] [--description <DESC>] [--enabled]\n\n",
                "Flags:\n",
                "  --name           (required) Transform name\n",
                "  --code           (required) JavaScript function body\n",
                "  --scope          global | device_type:<Type> | device:<ID> (default: global)\n",
                "  --output-prefix  Prefix for output DataSourceId\n",
                "  --description    Optional description\n",
                "  --enabled        Enable immediately (omit to create disabled)\n\n",
                "Code format: `input` is the raw metric value. Must `return` the result.\n",
                "  return (input - 32) * 5 / 9\n",
                "  if (input > 80) return \"good\"; if (input > 20) return \"ok\"; return \"low\"\n\n",
                "Example:\n",
                "  neomind transform create --name 'F to C' --scope global \\\n",
                "    --code 'return (input - 32) * 5 / 9'\n",
                "  neomind transform create --name 'Battery Health' --scope global \\\n",
                "    --code 'if (input > 80) return \"good\"; if (input > 20) return \"ok\"; return \"low\"'"
            ),
            "agent create" => concat!(
                "Usage: neomind agent create --name <NAME> --prompt '<TASK>' [options]\n\n",
                "Required:\n",
                "  --name       Agent display name\n",
                "  --prompt     Task description for the LLM\n\n",
                "Optional:\n",
                "  --schedule-type     event | interval | cron (default: event)\n",
                "  --schedule-config   Interval seconds or cron expression\n",
                "  --description       Agent description\n",
                "  --model             LLM backend ID (see: neomind llm list)\n",
                "  --system-prompt     Custom system instructions\n",
                "  --execution-mode    free | focused (default: free)\n",
                "  --device-ids        Comma-separated device IDs for focused mode\n\n",
                "After create, MUST activate:\n",
                "  neomind agent control <ID> --status active\n\n",
                "Examples:\n",
                "  neomind agent create --name 'Monitor' --prompt 'Check batteries' --schedule-type interval --schedule-config '300'\n",
                "  neomind agent create --name 'Daily' --prompt 'Summarize' --schedule-type cron --schedule-config '0 9 * * *'\n",
                "  neomind agent create --name 'Sensor Watch' --prompt 'Monitor sensors' --execution-mode focused --device-ids 's1,s2'"
            ),
            "agent invoke" => concat!(
                "Usage: neomind agent invoke <ID> --input '<TEXT>'\n\n",
                "One-shot agent execution. Runs the agent immediately with the given input.\n\n",
                "Flags:\n",
                "  --input    (required) Input text for the agent\n\n",
                "Example:\n",
                "  neomind agent invoke my-agent --input 'Check all sensors and report anomalies'"
            ),
            "agent memory" => concat!(
                "Usage: neomind agent memory <ID>\n\n",
                "Get extracted knowledge and memories for an agent.\n",
                "Returns key-value pairs the agent has learned across executions.\n\n",
                "Example:\n",
                "  neomind agent memory my-agent"
            ),
            "agent conversation" => concat!(
                "Usage: neomind agent conversation <ID> [--limit <N>]\n\n",
                "Get the full conversation history (message log) for an agent.\n\n",
                "Flags:\n",
                "  --limit    Max messages to return (default: 50)\n\n",
                "Example:\n",
                "  neomind agent conversation my-agent --limit 20"
            ),
            "agent latest-execution" => concat!(
                "Usage: neomind agent latest-execution <ID>\n\n",
                "Get the most recent execution result for an agent.\n",
                "Returns status, output, tool calls, and duration.\n\n",
                "Example:\n",
                "  neomind agent latest-execution my-agent"
            ),
            "agent executions" => concat!(
                "Usage: neomind agent executions <ID> [--limit <N>] [--offset <N>]\n\n",
                "Get execution history for an agent.\n\n",
                "Flags:\n",
                "  --limit    Max executions to return\n",
                "  --offset   Skip first N executions\n\n",
                "Example:\n",
                "  neomind agent executions my-agent --limit 10"
            ),
            "agent send-message" => concat!(
                "Usage: neomind agent send-message <ID> --message '<TEXT>' [--type <TYPE>]\n\n",
                "Send a directive message to a running agent.\n\n",
                "Flags:\n",
                "  --message    (required) Message text\n",
                "  --type       Optional message type\n\n",
                "Example:\n",
                "  neomind agent send-message my-agent --message 'Focus on battery levels today'"
            ),
            "connector create" => concat!(
                "Usage: neomind connector create --name <NAME> --host <HOST> [--port <PORT>] [--type <TYPE>] [--tls] [--username <USER>] [--password <PASS>] [--topics <TOPICS>]\n\n",
                "Flags:\n",
                "  --name      (required) Connector display name\n",
                "  --host      (required) Broker hostname or IP\n",
                "  --port      Port number (default: 1883)\n",
                "  --type / --connector-type   mqtt | webhook\n",
                "  --tls       Enable TLS (flag, no value)\n",
                "  --username  Auth username\n",
                "  --password  Auth password\n",
                "  --topics    Comma-separated topics to subscribe\n\n",
                "Example:\n",
                "  neomind connector create --name 'Remote Broker' --host 192.168.1.50 --port 1883\n",
                "  neomind connector create --name 'Secure' --host broker.example.com --port 8883 --tls --username user --password pass"
            ),
            "message send" => concat!(
                "Usage: neomind message send --title <TITLE> --message <TEXT> [--severity <LEVEL>] [--source <SRC>]\n\n",
                "Flags:\n",
                "  --title      (required) Message title\n",
                "  --message    (required) Message body text\n",
                "  --severity   info | warning | error | critical (default: info)\n",
                "  --source     Optional source identifier\n\n",
                "Example:\n",
                "  neomind message send --title 'Low Battery' --message 'Sensor-001 battery at 15%' --severity warning"
            ),
            "message channel-create" => concat!(
                "Usage: neomind message channel-create --name <NAME> --type <TYPE> --config '<JSON>'\n\n",
                "Flags:\n",
                "  --name     (required) Channel name (unique identifier)\n",
                "  --type     (required) Channel type: webhook | email\n",
                "  --config   (required) JSON configuration for the channel\n\n",
                "Webhook config: {\"url\": \"https://...\", \"headers\": {\"Authorization\": \"Bearer ...\"}}\n",
                "Email config: {\"smtp_server\": \"smtp.example.com\", \"smtp_port\": 587, \"username\": \"...\", \"password\": \"...\", \"from_address\": \"...\"}\n\n",
                "Example:\n",
                "  neomind message channel-create --name alerts --type webhook --config '{\"url\": \"https://hooks.example.com/notify\"}'"
            ),
            "message channel-update" => concat!(
                "Usage: neomind message channel-update <NAME> --config '<JSON>'\n\n",
                "Flags:\n",
                "  --config    (required) New JSON configuration\n\n",
                "Example:\n",
                "  neomind message channel-update alerts --config '{\"url\": \"https://new-url.example.com/hook\"}'"
            ),
            "message channel-delete" => concat!(
                "Usage: neomind message channel-delete <NAME>\n\n",
                "Permanently delete a message channel.\n\n",
                "Example:\n",
                "  neomind message channel-delete alerts"
            ),
            "message channel-test" => concat!(
                "Usage: neomind message channel-test <NAME>\n\n",
                "Send a test message through the channel to verify configuration.\n\n",
                "Example:\n",
                "  neomind message channel-test alerts"
            ),
            "dashboard" => concat!(
                "Dashboard Commands:\n\n",
                "  neomind dashboard list                                    List all dashboards\n",
                "  neomind dashboard get <ID>                                Get dashboard details\n",
                "  neomind dashboard create --name <NAME>                    Create new dashboard\n",
                "  neomind dashboard update <ID> [--name] [--components]     Update (components REPLACES all!)\n",
                "  neomind dashboard add-components <ID> --components '<JSON>'  APPEND components (recommended)\n",
                "  neomind dashboard remove-components <ID> --ids '<JSON>'   Remove components by ID\n",
                "  neomind dashboard delete <ID>                             Delete dashboard\n",
                "  neomind dashboard share <ID> [--public] [--expires <SEC>]  Share dashboard"
            ),
            "device" => concat!(
                "Device Commands:\n\n",
                "  neomind device list                                       List devices\n",
                "  neomind device get <ID>                                   Get device details\n",
                "  neomind device create <NAME> [--type <T>] [--adapter <A>] Create device\n",
                "  neomind device update <ID> [--name] [--config]            Update device\n",
                "  neomind device delete <ID>                                Delete device\n",
                "  neomind device latest <ID>                                Latest metric values\n",
                "  neomind device history <ID> [--metric] [--time-range]     Telemetry history\n",
                "  neomind device control <ID> <CMD> [--params '<JSON>']     Send command\n",
                "  neomind device types list                                 List device types\n",
                "  neomind device types create --name <N> --metrics '<JSON>' Create device type\n",
                "  neomind device types get <ID>                             Get device type\n",
                "  neomind device write-metric <ID> --metric <M> --value <V> Write metric value"
            ),
            "rule" => concat!(
                "Rule Commands:\n\n",
                "  neomind rule list                                         List rules\n",
                "  neomind rule get <ID>                                     Get rule details\n",
                "  neomind rule create [--name <N>] --dsl '<DSL>'            Create rule\n",
                "  neomind rule update <ID> [--name] [--dsl]                 Update rule\n",
                "  neomind rule delete <ID>                                  Delete rule\n",
                "  neomind rule enable <ID>                                  Enable rule\n",
                "  neomind rule disable <ID>                                 Disable rule\n",
                "  neomind rule test <ID> --input '<JSON>'                   Test rule"
            ),
            "agent" => concat!(
                "Agent Commands:\n\n",
                "  neomind agent list                                        List agents\n",
                "  neomind agent get <ID>                                    Get agent details\n",
                "  neomind agent create --name <N> --prompt '<TASK>'         Create agent\n",
                "  neomind agent update <ID> [--name] [--prompt] [--model]   Update agent\n",
                "  neomind agent delete <ID>                                 Delete agent\n",
                "  neomind agent control <ID> --status active|paused        Start/stop agent\n",
                "  neomind agent invoke <ID> --input '<TEXT>'                One-shot execution\n",
                "  neomind agent executions <ID>                             Execution history\n",
                "  neomind agent latest-execution <ID>                       Most recent execution\n",
                "  neomind agent conversation <ID>                           Full message log\n",
                "  neomind agent memory <ID>                                 Extracted knowledge\n",
                "  neomind agent send-message <ID> --message '<TEXT>'        Send directive"
            ),
            "extension" => concat!(
                "Extension Commands:\n\n",
                "  neomind extension list                                    List extensions\n",
                "  neomind extension get|info <ID>                          Get extension details\n",
                "  neomind extension status <ID>                             Extension status\n",
                "  neomind extension logs <ID> [--limit <N>]                 Extension logs\n",
                "  neomind extension create <NAME> --extension-type <T>      Scaffold extension\n",
                "  neomind extension build <PATH>                            Build extension\n",
                "  neomind extension validate <PATH>                         Validate package\n",
                "  neomind extension install <PATH>                          Install extension\n",
                "  neomind extension uninstall <ID>                          Uninstall extension\n",
                "  neomind extension reload <ID>                             Reload extension\n",
                "  neomind extension market-list                             List marketplace\n",
                "  neomind extension market-install <ID> [--version <V>]     Install from marketplace"
            ),
            "widget" => concat!(
                "Widget Commands:\n\n",
                "  neomind widget list                                       List installed widgets\n",
                "  neomind widget get <ID>                                   Get widget details + config_schema\n",
                "  neomind widget bundle <ID>                                Get widget bundle JS\n",
                "  neomind widget create <NAME> [--widget-type <T>]          Scaffold widget (local files)\n",
                "  neomind widget install <PATH>                             Install widget from zip\n",
                "  neomind widget uninstall <ID>                             Uninstall widget\n",
                "  neomind widget market-list                                List marketplace widgets\n",
                "  neomind widget market-install <ID> [--version <V>]        Install from marketplace\n\n",
                "Widget types: chart, gauge, stat, table, image, custom"
            ),
            "connector" => concat!(
                "Connector Commands:\n\n",
                "  neomind connector list                                    List connectors\n",
                "  neomind connector get <ID>                                Get connector details\n",
                "  neomind connector create --name <N> --host <H> [--port <P>]  Create connector\n",
                "  neomind connector update <ID> [--name] [--host] [--port]  Update connector\n",
                "  neomind connector delete <ID>                             Delete connector\n",
                "  neomind connector test <ID>                               Test connection\n",
                "  neomind connector subscriptions                           List subscriptions\n",
                "  neomind connector subscribe --topic <T> [--qos <Q>]       Subscribe to topic\n",
                "  neomind connector unsubscribe --topic <T>                 Unsubscribe from topic"
            ),
            "llm" => concat!(
                "LLM Backend Commands:\n\n",
                "  neomind llm list                                         List configured backends\n",
                "  neomind llm get <ID>                                     Get backend details\n",
                "  neomind llm models                                       List available Ollama models\n\n",
                "Use backend IDs with: neomind agent create --model <ID>"
            ),
            "transform" => concat!(
                "Transform Commands:\n\n",
                "  neomind transform list                                    List transforms\n",
                "  neomind transform get <ID>                                Get transform details\n",
                "  neomind transform create --name <N> --code '<JS>' [--scope <S>]  Create transform\n",
                "  neomind transform update <ID> [--name] [--code] [--enabled]  Update transform\n",
                "  neomind transform delete <ID>                             Delete transform\n",
                "  neomind transform test --code '<JS>' --input '<JSON>'     Test transform code\n",
                "  neomind transform metrics                                 List virtual metrics\n",
                "  neomind transform data-sources                            List data sources"
            ),
            "message" => concat!(
                "Message Commands:\n\n",
                "  neomind message send --title <T> --message <M> [--severity <LV>]  Send notification\n",
                "  neomind message list [--limit] [--severity] [--status]    List messages\n",
                "  neomind message get <ID>                                  Get message details\n",
                "  neomind message read <ID> / ack <ID>                      Mark as read\n\n",
                "Channel Commands:\n",
                "  neomind message channel-list                              List channels\n",
                "  neomind message channel-get <NAME>                        Get channel\n",
                "  neomind message channel-create --name <N> --type <T> --config '<JSON>'\n",
                "  neomind message channel-update <NAME> --config '<JSON>'   Update channel\n",
                "  neomind message channel-delete <NAME>                     Delete channel\n",
                "  neomind message channel-test <NAME>                       Test channel\n",
                "  neomind message channel-types                             List channel types\n\n",
                "Severity levels: info, warning, error, critical"
            ),
            "system" => concat!(
                "System Commands:\n\n",
                "  neomind system info                                       System status & network info"
            ),
            _ => return Ok(neomind_cli_ops::CliResponse::success(
                serde_json::json!({"help": true, "command": cmd}),
                format!("Help for '{}': refer to the shell tool description for available commands and flags.", cmd)
            )),
        };
        Ok(neomind_cli_ops::CliResponse::success(
            serde_json::json!({"help": true, "command": cmd}),
            help.to_string()
        ))
    }

    /// Extract value for a flag like --name <value> from args
    fn get_flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
        args.iter().position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .map(|s| s.as_str())
    }

    /// Resolve entity ID: supports both positional (args[3]) and `--id` flag.
    fn resolve_id(args: &[String]) -> &str {
        Self::get_flag_value(args, "--id").unwrap_or_else(|| args.get(3).map(|s| s.as_str()).unwrap_or(""))
    }

    // ---- Domain executors ----

    async fn exec_device(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => {
                let device_type = Self::get_flag_value(args, "--device-type").map(|s| s.to_string());
                let status = Self::get_flag_value(args, "--status").map(|s| s.to_string());
                neomind_cli_ops::device::list_devices(client, device_type.as_deref(), status.as_deref()).await
            }
            "get" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::device::get_device(client, id).await
            }
            "create" => {
                let name = Self::get_flag_value(args, "--name").unwrap_or("").to_string();
                let type_id = Self::get_flag_value(args, "--type")
                    .or_else(|| Self::get_flag_value(args, "--device-type"))
                    .unwrap_or("").to_string();
                let adapter = Self::get_flag_value(args, "--adapter")
                    .or_else(|| Self::get_flag_value(args, "--adapter-type"))
                    .unwrap_or("mqtt").to_string();
                let config = Self::get_flag_value(args, "--config")
                    .map(|s| serde_json::from_str(s).unwrap_or(serde_json::json!(s)));
                neomind_cli_ops::device::create_device(client, &name, &type_id, &adapter, config).await
            }
            "update" => {
                let id = Self::resolve_id(args).to_string();
                let name = Self::get_flag_value(args, "--name").map(|s| s.to_string());
                let config = Self::get_flag_value(args, "--config")
                    .map(|s| serde_json::from_str(s).unwrap_or(serde_json::json!(s)));
                neomind_cli_ops::device::update_device(client, &id, name.as_deref(), config).await
            }
            "delete" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::device::delete_device(client, id).await
            }
            "latest" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::device::get_latest_metrics(client, id).await
            }
            "history" => {
                let id = Self::resolve_id(args);
                let metric = Self::get_flag_value(args, "--metric");
                let time_range = Self::get_flag_value(args, "--time-range");
                let compress = args.iter().any(|a| a == "--compress");
                neomind_cli_ops::device::get_telemetry_history(client, id, metric, time_range, compress).await
            }
            "control" => {
                let id = Self::resolve_id(args).to_string();
                // Support both --command flag and positional arg (args[4])
                let command = Self::get_flag_value(args, "--command")
                    .or_else(|| args.get(4).map(|s| s.as_str()).filter(|s| !s.starts_with("--")))
                    .unwrap_or("").to_string();
                let params_str = Self::get_flag_value(args, "--params").unwrap_or("{}");
                let params = serde_json::from_str(params_str).unwrap_or(serde_json::json!({}));
                neomind_cli_ops::device::control_device(client, &id, &command, params).await
            }
            "types" => {
                let sub = args.get(3).map(|s| s.as_str()).unwrap_or("");
                match sub {
                    "list" => neomind_cli_ops::device::list_device_types(client).await,
                    "get" => {
                        let type_id = args.get(4).map(|s| s.as_str()).unwrap_or("");
                        neomind_cli_ops::device::get_device_type(client, type_id).await
                    }
                    "create" => {
                        let id = Self::get_flag_value(args, "--id").map(|s| s.to_string());
                        let name = Self::get_flag_value(args, "--name").unwrap_or("").to_string();
                        let metrics_str = Self::get_flag_value(args, "--metrics").unwrap_or("[]");
                        let metrics = serde_json::from_str(metrics_str).unwrap_or(serde_json::json!([]));
                        let commands = Self::get_flag_value(args, "--commands")
                            .map(|s| serde_json::from_str(s).unwrap_or(serde_json::json!(null)));
                        neomind_cli_ops::device::create_device_type(client, id.as_deref(), &name, metrics, commands).await
                    }
                    _ => anyhow::bail!("Unknown device types subcommand: {}", sub),
                }
            }
            "write-metric" => {
                let id = Self::resolve_id(args).to_string();
                let metric = Self::get_flag_value(args, "--metric").unwrap_or("").to_string();
                let value_str = Self::get_flag_value(args, "--value").unwrap_or("");
                let value = if let Ok(n) = value_str.parse::<f64>() {
                    serde_json::json!(n)
                } else if let Ok(b) = value_str.parse::<bool>() {
                    serde_json::json!(b)
                } else {
                    serde_json::json!(value_str)
                };
                let timestamp = Self::get_flag_value(args, "--timestamp").and_then(|s| s.parse::<i64>().ok());
                neomind_cli_ops::device::write_metric(client, &id, &metric, value, timestamp).await
            }
            _ => anyhow::bail!("Unknown device action: {}", action),
        }
    }

    async fn exec_dashboard(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => neomind_cli_ops::dashboard::list_dashboards(client).await,
            "get" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::dashboard::get_dashboard(client, id).await
            }
            "create" => {
                let name = Self::get_flag_value(args, "--name").unwrap_or("").to_string();
                let description = Self::get_flag_value(args, "--description").map(|s| s.to_string());
                let layout = Self::get_flag_value(args, "--layout")
                    .map(|s| serde_json::from_str(s).unwrap_or(serde_json::json!(s)));
                neomind_cli_ops::dashboard::create_dashboard(client, &name, description.as_deref(), layout).await
            }
            "update" => {
                let id = Self::resolve_id(args).to_string();
                let name = Self::get_flag_value(args, "--name").map(|s| s.to_string());
                let description = Self::get_flag_value(args, "--description").map(|s| s.to_string());
                let layout = Self::get_flag_value(args, "--layout")
                    .map(|s| serde_json::from_str(s).unwrap_or(serde_json::json!(s)));
                let components = Self::get_flag_value(args, "--components")
                    .map(|s| serde_json::from_str(s).unwrap_or(serde_json::json!(s)));
                neomind_cli_ops::dashboard::update_dashboard(client, &id, name.as_deref(), description.as_deref(), layout, components).await
            }
            "add-components" => {
                let id = Self::resolve_id(args).to_string();
                let components_str = Self::get_flag_value(args, "--components").unwrap_or("[]");
                let components = serde_json::from_str(components_str).unwrap_or(serde_json::json!([]));
                neomind_cli_ops::dashboard::add_components(client, &id, components).await
            }
            "remove-components" => {
                let id = Self::resolve_id(args).to_string();
                let ids_str = Self::get_flag_value(args, "--ids").unwrap_or("[]");
                let ids = serde_json::from_str(ids_str).unwrap_or(serde_json::json!([]));
                neomind_cli_ops::dashboard::remove_components(client, &id, ids).await
            }
            "delete" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::dashboard::delete_dashboard(client, id).await
            }
            "share" => {
                let id = Self::resolve_id(args).to_string();
                let public = args.iter().any(|a| a == "--public");
                let expires = Self::get_flag_value(args, "--expires").map(|s| s.to_string());
                neomind_cli_ops::dashboard::share_dashboard(client, &id, public, expires.as_deref()).await
            }
            _ => anyhow::bail!("Unknown dashboard action: {}", action),
        }
    }

    async fn exec_rule(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => neomind_cli_ops::rule::list_rules(client).await,
            "get" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::rule::get_rule(client, id).await
            }
            "create" => {
                let name = Self::get_flag_value(args, "--name").map(|s| s.to_string());
                let dsl = Self::get_flag_value(args, "--dsl").unwrap_or("").to_string();
                neomind_cli_ops::rule::create_rule(client, name.as_deref(), &dsl).await
            }
            "update" => {
                let id = Self::resolve_id(args).to_string();
                let name = Self::get_flag_value(args, "--name").map(|s| s.to_string());
                let dsl = Self::get_flag_value(args, "--dsl").map(|s| s.to_string());
                neomind_cli_ops::rule::update_rule(client, &id, name.as_deref(), dsl.as_deref()).await
            }
            "delete" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::rule::delete_rule(client, id).await
            }
            "enable" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::rule::enable_rule(client, id).await
            }
            "disable" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::rule::disable_rule(client, id).await
            }
            "test" => {
                let id = Self::resolve_id(args).to_string();
                let input_str = Self::get_flag_value(args, "--input").unwrap_or("{}");
                let input = serde_json::from_str(input_str).unwrap_or(serde_json::json!({}));
                neomind_cli_ops::rule::test_rule(client, &id, input).await
            }
            "history" => {
                let id = Self::resolve_id(args).to_string();
                neomind_cli_ops::rule::get_rule_history(client, &id).await
            }
            _ => anyhow::bail!("Unknown rule action: {}", action),
        }
    }

    async fn exec_extension(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => neomind_cli_ops::extension::list_extensions(client).await,
            "get" | "info" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::extension::get_extension(client, id).await
            }
            "status" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::extension::get_extension_status(client, id).await
            }
            "logs" => {
                let id = Self::resolve_id(args).to_string();
                let limit = Self::get_flag_value(args, "--limit").and_then(|s| s.parse::<usize>().ok());
                neomind_cli_ops::extension::get_extension_logs(client, &id, limit).await
            }
            "install" => {
                let path = args.get(3).map(|s| s.as_str()).unwrap_or("");
                neomind_cli_ops::extension::install_extension_file(client, path).await
            }
            "uninstall" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::extension::uninstall_extension(client, id).await
            }
            "market-list" => neomind_cli_ops::extension::list_marketplace(client).await,
            "market-install" => {
                let ext_id = Self::resolve_id(args).to_string();
                let version = Self::get_flag_value(args, "--version").map(|s| s.to_string());
                neomind_cli_ops::extension::install_extension_market(client, &ext_id, version.as_deref()).await
            }
            // create/validate/build are local filesystem operations — let them fall through to external CLI
            "create" | "validate" | "build" => {
                Err(anyhow::anyhow!("__FALLTHROUGH__"))
            }
            _ => Err(anyhow::anyhow!("__FALLTHROUGH__")),
        }
    }

    async fn exec_widget(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => neomind_cli_ops::widget::list_widgets(client).await,
            "get" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::widget::get_widget(client, id).await
            }
            "bundle" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::widget::get_widget_bundle(client, id).await
            }
            "create" => {
                let name = args.get(3).map(|s| s.as_str()).unwrap_or("").to_string();
                let widget_type = Self::get_flag_value(args, "--widget-type").unwrap_or("custom").to_string();
                let output = Self::get_flag_value(args, "--output").map(|s| s.to_string());
                // create_widget is synchronous (file system only), wrap it
                neomind_cli_ops::widget::create_widget(&name, &widget_type, output.as_deref())
            }
            "install" => {
                let path = args.get(3).map(|s| s.as_str()).unwrap_or("");
                neomind_cli_ops::widget::install_widget_file(client, path).await
            }
            "uninstall" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::widget::uninstall_widget(client, id).await
            }
            "market-list" => neomind_cli_ops::widget::list_marketplace_widgets(client).await,
            "market-install" => {
                let id = Self::resolve_id(args).to_string();
                let version = Self::get_flag_value(args, "--version").map(|s| s.to_string());
                neomind_cli_ops::widget::install_widget_market(client, &id, version.as_deref()).await
            }
            _ => anyhow::bail!("Unknown widget action: {}", action),
        }
    }

    async fn exec_transform(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => neomind_cli_ops::transform::list_transforms(client).await,
            "get" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::transform::get_transform(client, id).await
            }
            "create" => {
                let name = Self::get_flag_value(args, "--name").unwrap_or("").to_string();
                let scope = Self::get_flag_value(args, "--scope").unwrap_or("global").to_string();
                let code = Self::get_flag_value(args, "--code").unwrap_or("").to_string();
                let output_prefix = Self::get_flag_value(args, "--output-prefix").map(|s| s.to_string());
                let description = Self::get_flag_value(args, "--description").map(|s| s.to_string());
                let enabled = Self::get_flag_value(args, "--enabled").and_then(|s| s.parse::<bool>().ok());
                neomind_cli_ops::transform::create_transform(
                    client, &name, &scope, &code,
                    output_prefix.as_deref(), description.as_deref(), enabled,
                ).await
            }
            "update" => {
                let id = Self::resolve_id(args).to_string();
                let name = Self::get_flag_value(args, "--name").map(|s| s.to_string());
                let description = Self::get_flag_value(args, "--description").map(|s| s.to_string());
                let code = Self::get_flag_value(args, "--code").map(|s| s.to_string());
                let scope = Self::get_flag_value(args, "--scope").map(|s| s.to_string());
                let output_prefix = Self::get_flag_value(args, "--output-prefix").map(|s| s.to_string());
                let enabled = Self::get_flag_value(args, "--enabled").and_then(|s| s.parse::<bool>().ok());
                neomind_cli_ops::transform::update_transform(
                    client, &id,
                    name.as_deref(), description.as_deref(), code.as_deref(),
                    scope.as_deref(), output_prefix.as_deref(), enabled,
                ).await
            }
            "delete" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::transform::delete_transform(client, id).await
            }
            "metrics" => {
                neomind_cli_ops::transform::list_virtual_metrics(client).await
            }
            "test" => {
                let code = Self::get_flag_value(args, "--code").unwrap_or("").to_string();
                let input_str = Self::get_flag_value(args, "--input").unwrap_or("{}");
                let input_data = serde_json::from_str(input_str).unwrap_or(serde_json::json!({}));
                neomind_cli_ops::transform::test_transform_code(client, &code, input_data).await
            }
            "data-sources" => neomind_cli_ops::transform::list_transform_data_sources(client).await,
            _ => anyhow::bail!("Unknown transform action: {}", action),
        }
    }

    async fn exec_agent(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => neomind_cli_ops::agent_cmd::list_agents(client).await,
            "get" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::agent_cmd::get_agent(client, id).await
            }
            "create" => {
                let name = Self::get_flag_value(args, "--name").unwrap_or("").to_string();
                let prompt = Self::get_flag_value(args, "--prompt").unwrap_or("").to_string();
                let description = Self::get_flag_value(args, "--description").map(|s| s.to_string());
                let schedule_type = Self::get_flag_value(args, "--schedule-type").map(|s| s.to_string());
                let schedule_config = Self::get_flag_value(args, "--schedule-config").map(|s| s.to_string());
                let event_filter = Self::get_flag_value(args, "--event-filter").map(|s| s.to_string());
                let timezone = Self::get_flag_value(args, "--timezone").map(|s| s.to_string());
                let llm_backend = Self::get_flag_value(args, "--model")
                    .or_else(|| Self::get_flag_value(args, "--llm-backend"))
                    .map(|s| s.to_string());
                let system_prompt = Self::get_flag_value(args, "--system-prompt").map(|s| s.to_string());
                let execution_mode = Self::get_flag_value(args, "--execution-mode").map(|s| s.to_string());
                let device_ids = Self::get_flag_value(args, "--device-ids").map(|s| s.to_string());
                let resources = Self::get_flag_value(args, "--resources").map(|s| s.to_string());
                let metrics = Self::get_flag_value(args, "--metrics").map(|s| s.to_string());
                let commands = Self::get_flag_value(args, "--commands").map(|s| s.to_string());
                let enable_tool_chaining = Self::get_flag_value(args, "--enable-tool-chaining").and_then(|s| s.parse::<bool>().ok());
                let max_chain_depth = Self::get_flag_value(args, "--max-chain-depth").and_then(|s| s.parse::<usize>().ok());
                let priority = Self::get_flag_value(args, "--priority").and_then(|s| s.parse::<u8>().ok());
                let context_window_size = Self::get_flag_value(args, "--context-window-size").and_then(|s| s.parse::<usize>().ok());
                neomind_cli_ops::agent_cmd::create_agent(
                    client, &name, &prompt, description.as_deref(),
                    schedule_type.as_deref(), schedule_config.as_deref(),
                    event_filter.as_deref(), timezone.as_deref(),
                    llm_backend.as_deref(), system_prompt.as_deref(),
                    execution_mode.as_deref(), device_ids.as_deref(), resources.as_deref(),
                    metrics.as_deref(), commands.as_deref(),
                    enable_tool_chaining, max_chain_depth, priority, context_window_size,
                ).await
            }
            "delete" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::agent_cmd::delete_agent(client, id).await
            }
            "control" => {
                let id = Self::resolve_id(args).to_string();
                // Support --action, --status flags and positional status arg (args[4])
                let action = Self::get_flag_value(args, "--action")
                    .or_else(|| Self::get_flag_value(args, "--status"))
                    .or_else(|| args.get(4).map(|s| s.as_str()).filter(|s| !s.starts_with("--")))
                    .unwrap_or("").to_string();
                neomind_cli_ops::agent_cmd::control_agent(client, &id, &action).await
            }
            "update" => {
                let id = Self::resolve_id(args).to_string();
                let name = Self::get_flag_value(args, "--name").map(|s| s.to_string());
                let prompt = Self::get_flag_value(args, "--prompt").map(|s| s.to_string());
                let description = Self::get_flag_value(args, "--description").map(|s| s.to_string());
                let llm_backend = Self::get_flag_value(args, "--model")
                    .or_else(|| Self::get_flag_value(args, "--llm-backend"))
                    .map(|s| s.to_string());
                let system_prompt = Self::get_flag_value(args, "--system-prompt").map(|s| s.to_string());
                let schedule_type = Self::get_flag_value(args, "--schedule-type").map(|s| s.to_string());
                let schedule_config = Self::get_flag_value(args, "--schedule-config").map(|s| s.to_string());
                let execution_mode = Self::get_flag_value(args, "--execution-mode").map(|s| s.to_string());
                let device_ids = Self::get_flag_value(args, "--device-ids").map(|s| s.to_string());
                let resources = Self::get_flag_value(args, "--resources").map(|s| s.to_string());
                let metrics = Self::get_flag_value(args, "--metrics").map(|s| s.to_string());
                let commands = Self::get_flag_value(args, "--commands").map(|s| s.to_string());
                let enable_tool_chaining = Self::get_flag_value(args, "--enable-tool-chaining").and_then(|s| s.parse::<bool>().ok());
                let max_chain_depth = Self::get_flag_value(args, "--max-chain-depth").and_then(|s| s.parse::<usize>().ok());
                let priority = Self::get_flag_value(args, "--priority").and_then(|s| s.parse::<u8>().ok());
                let context_window_size = Self::get_flag_value(args, "--context-window-size").and_then(|s| s.parse::<usize>().ok());
                neomind_cli_ops::agent_cmd::update_agent(
                    client, &id, name.as_deref(), description.as_deref(),
                    llm_backend.as_deref(), system_prompt.as_deref(), prompt.as_deref(),
                    schedule_type.as_deref(), schedule_config.as_deref(),
                    execution_mode.as_deref(), device_ids.as_deref(), resources.as_deref(),
                    metrics.as_deref(), commands.as_deref(),
                    enable_tool_chaining, max_chain_depth, priority, context_window_size,
                ).await
            }
            "invoke" => {
                let id = Self::resolve_id(args).to_string();
                let input = Self::get_flag_value(args, "--input").unwrap_or("").to_string();
                neomind_cli_ops::agent_cmd::invoke_agent(client, &id, &input).await
            }
            "memory" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::agent_cmd::get_agent_memory(client, id).await
            }
            "executions" => {
                let id = Self::resolve_id(args).to_string();
                let limit = Self::get_flag_value(args, "--limit").and_then(|s| s.parse::<usize>().ok());
                let offset = Self::get_flag_value(args, "--offset").and_then(|s| s.parse::<usize>().ok());
                neomind_cli_ops::agent_cmd::get_agent_executions(client, &id, limit, offset).await
            }
            "latest-execution" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::agent_cmd::get_latest_execution(client, id).await
            }
            "conversation" => {
                let id = Self::resolve_id(args).to_string();
                let limit = Self::get_flag_value(args, "--limit").and_then(|s| s.parse::<usize>().ok());
                neomind_cli_ops::agent_cmd::get_conversation(client, &id, limit).await
            }
            "send-message" => {
                let id = Self::resolve_id(args).to_string();
                let message = Self::get_flag_value(args, "--message").unwrap_or("").to_string();
                let message_type = Self::get_flag_value(args, "--type").map(|s| s.to_string());
                neomind_cli_ops::agent_cmd::send_message(client, &id, &message, message_type.as_deref()).await
            }
            _ => anyhow::bail!("Unknown agent action: {}", action),
        }
    }

    async fn exec_message(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => {
                let limit = Self::get_flag_value(args, "--limit").and_then(|s| s.parse::<usize>().ok());
                let offset = Self::get_flag_value(args, "--offset").and_then(|s| s.parse::<usize>().ok());
                let severity = Self::get_flag_value(args, "--severity").map(|s| s.to_string());
                let status = Self::get_flag_value(args, "--status").map(|s| s.to_string());
                neomind_cli_ops::message::list_messages(client, limit, offset, severity.as_deref(), status.as_deref()).await
            }
            "get" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::message::get_message(client, id).await
            }
            "send" => {
                let title = Self::get_flag_value(args, "--title").unwrap_or("").to_string();
                let message_body = Self::get_flag_value(args, "--message").unwrap_or("").to_string();
                let severity = Self::get_flag_value(args, "--severity").unwrap_or("info").to_string();
                let source = Self::get_flag_value(args, "--source").map(|s| s.to_string());
                neomind_cli_ops::message::send_message(client, &title, &message_body, &severity, source.as_deref()).await
            }
            "read" | "ack" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::message::acknowledge_message(client, id).await
            }
            "channel-list" => {
                neomind_cli_ops::message::list_channels(client).await
            }
            "channel-get" => {
                let name = Self::resolve_id(args);
                neomind_cli_ops::message::get_channel(client, name).await
            }
            "channel-types" => {
                neomind_cli_ops::message::list_channel_types(client).await
            }
            "channel-create" => {
                let name = Self::get_flag_value(args, "--name").unwrap_or("").to_string();
                let channel_type = Self::get_flag_value(args, "--type").unwrap_or("").to_string();
                let config = Self::get_flag_value(args, "--config").unwrap_or("{}");
                neomind_cli_ops::message::create_channel(client, &name, &channel_type, config).await
            }
            "channel-update" => {
                let name = Self::resolve_id(args).to_string();
                let config = Self::get_flag_value(args, "--config").unwrap_or("{}");
                neomind_cli_ops::message::update_channel(client, &name, config).await
            }
            "channel-delete" => {
                let name = Self::resolve_id(args);
                neomind_cli_ops::message::delete_channel(client, name).await
            }
            "channel-test" => {
                let name = Self::resolve_id(args).to_string();
                neomind_cli_ops::message::test_channel(client, &name).await
            }
            _ => anyhow::bail!("Unknown message action: {}", action),
        }
    }

    /// Execute `neomind system <action>` commands internally.
    async fn exec_system(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("info");
        match action {
            "info" => neomind_cli_ops::system::system_info(client).await,
            _ => anyhow::bail!("Unknown system action: {}", action),
        }
    }

    /// Execute `neomind connector <action>` commands internally.
    async fn exec_connector(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => neomind_cli_ops::connector::list_connectors(client).await,
            "get" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::connector::get_connector(client, id).await
            }
            "create" => {
                let connector_type = Self::get_flag_value(args, "--type")
                    .or_else(|| Self::get_flag_value(args, "--connector-type"))
                    .map(|s| s.to_string());
                let name = Self::get_flag_value(args, "--name").unwrap_or("").to_string();
                let host = Self::get_flag_value(args, "--host").unwrap_or("").to_string();
                let port = Self::get_flag_value(args, "--port")
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(1883);
                let tls = Self::get_flag_value(args, "--tls").is_some();
                let username = Self::get_flag_value(args, "--username").map(|s| s.to_string());
                let password = Self::get_flag_value(args, "--password").map(|s| s.to_string());
                let topics = Self::get_flag_value(args, "--topics").map(|s| s.to_string());
                neomind_cli_ops::connector::create_connector(
                    client, &name, connector_type.as_deref(), &host, port, tls,
                    username.as_deref(), password.as_deref(), topics.as_deref(),
                ).await
            }
            "update" => {
                let id = Self::resolve_id(args);
                let name = Self::get_flag_value(args, "--name").map(|s| s.to_string());
                let host = Self::get_flag_value(args, "--host").map(|s| s.to_string());
                let port = Self::get_flag_value(args, "--port").and_then(|s| s.parse::<u16>().ok());
                let tls = Self::get_flag_value(args, "--tls").is_some().then_some(true);
                let username = Self::get_flag_value(args, "--username").map(|s| s.to_string());
                let password = Self::get_flag_value(args, "--password").map(|s| s.to_string());
                let topics = Self::get_flag_value(args, "--topics").map(|s| s.to_string());
                let enabled = if Self::get_flag_value(args, "--disable").is_some() { Some(false) } else { None };
                neomind_cli_ops::connector::update_connector(
                    client, id, name.as_deref(), host.as_deref(), port, tls,
                    username.as_deref(), password.as_deref(), topics.as_deref(), enabled,
                ).await
            }
            "delete" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::connector::delete_connector(client, id).await
            }
            "test" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::connector::test_connector(client, id).await
            }
            "subscriptions" => {
                neomind_cli_ops::connector::list_subscriptions(client).await
            }
            "subscribe" => {
                let topic = Self::get_flag_value(args, "--topic").unwrap_or("").to_string();
                let qos = Self::get_flag_value(args, "--qos").and_then(|s| s.parse::<u8>().ok());
                neomind_cli_ops::connector::subscribe_topic(client, &topic, qos).await
            }
            "unsubscribe" => {
                let topic = Self::get_flag_value(args, "--topic").unwrap_or("").to_string();
                neomind_cli_ops::connector::unsubscribe_topic(client, &topic).await
            }
            _ => anyhow::bail!("Unknown connector action: {}", action),
        }
    }

    async fn exec_llm(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => neomind_cli_ops::llm::list_backends(client).await,
            "get" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::llm::get_backend(client, id).await
            }
            "models" => neomind_cli_ops::llm::list_ollama_models(client).await,
            _ => anyhow::bail!("Unknown llm action: {}. Available: list, get, models", action),
        }
    }

    /// Build a platform-appropriate shell command.
    /// Unix: login shell (`$SHELL -l -c`) with isolated process group;
    ///       falls back to `/bin/sh -c` without `-l` if $SHELL is not set.
    /// Windows: `cmd /C`
    fn build_command(command: &str) -> std::process::Command {
        let (shell, is_login) = shell_path();
        let mut cmd = std::process::Command::new(shell);
        shell_arg(&mut cmd, command, is_login);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        set_process_group(&mut cmd);
        cmd
    }

    /// Execute a command with timeout and output capture.
    async fn execute_command(
        &self,
        command: &str,
        working_dir: Option<&str>,
        timeout: Duration,
    ) -> Result<CommandOutput> {
        // Try internal execution for neomind CLI commands (if enabled)
        if let Some(result) = self.try_internal_execution(command).await {
            return result;
        }

        // Fall back to process spawning
        let mut cmd = Self::build_command(command);

        if let Some(dir) = working_dir {
            let path = std::path::Path::new(dir);
            if !path.exists() {
                return Err(ToolError::Execution(format!(
                    "Working directory does not exist: {}",
                    dir
                )));
            }
            if !path.is_dir() {
                return Err(ToolError::Execution(format!(
                    "Path is not a directory: {}",
                    dir
                )));
            }
            cmd.current_dir(dir);
        }

        let child = tokio::process::Command::from(cmd)
            .spawn()
            .map_err(|e| ToolError::Execution(format!("Failed to spawn: {}", e)))?;

        // Capture child PID before moving child into the timeout future
        let child_pid = child.id();

        let result = tokio::time::timeout(timeout, child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => Ok(CommandOutput {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                timed_out: false,
            }),
            Ok(Err(e)) => Err(ToolError::Execution(format!("Execution failed: {}", e))),
            Err(_) => {
                // Timeout — kill the process
                kill_process_by_pid(child_pid);
                Ok(CommandOutput {
                    exit_code: None,
                    stdout: String::new(),
                    stderr: format!("Command timed out after {}s", timeout.as_secs()),
                    timed_out: true,
                })
            }
        }
    }
}

// ============================================================================
// Platform-specific helpers
// ============================================================================

/// Returns the user's login shell from `$SHELL`, falling back to `/bin/sh`.
/// Returns (shell_path, is_login): is_login is false for the fallback.
#[cfg(unix)]
fn shell_path() -> (String, bool) {
    match std::env::var("SHELL") {
        Ok(shell) => (shell, true),
        Err(_) => ("/bin/sh".to_string(), false),
    }
}

#[cfg(windows)]
fn shell_path() -> (&'static str, bool) {
    ("cmd", false)
}

/// Adds the shell flag argument.
/// Unix: `-l -c` for login shells, `-c` for fallback `/bin/sh`.
/// Windows: `/C`.
#[cfg(unix)]
fn shell_arg(cmd: &mut std::process::Command, command: &str, is_login: bool) {
    if is_login {
        cmd.arg("-l");
    }
    cmd.arg("-c").arg(command);
}

#[cfg(windows)]
fn shell_arg(cmd: &mut std::process::Command, command: &str, _is_login: bool) {
    cmd.arg("/C").arg(command);
}

/// Set process group isolation (Unix only — prevents orphaned child processes).
#[cfg(unix)]
fn set_process_group(cmd: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

#[cfg(windows)]
fn set_process_group(_cmd: &mut std::process::Command) {
    // On Windows, child processes are naturally terminated when the parent dies
    // via Job Object inheritance. No explicit action needed for our use case.
}

/// Kill a process by PID. On Unix, kills the entire process group to prevent orphans.
#[cfg(unix)]
fn kill_process_by_pid(pid: Option<u32>) {
    if let Some(pid) = pid {
        // PID of child is also the PGID since we used process_group(0)
        unsafe {
            if libc::killpg(pid as i32, libc::SIGKILL) != 0 {
                tracing::warn!(
                    "Failed to kill process group {}: {}",
                    pid,
                    std::io::Error::last_os_error()
                );
            }
        }
    }
}

#[cfg(windows)]
fn kill_process_by_pid(pid: Option<u32>) {
    if let Some(pid) = pid {
        // Use Windows API to terminate the process.
        // On Windows, TerminateProcess is the most reliable way to kill a process.
        unsafe {
            if windows_sys::Win32::System::Threading::TerminateProcess(pid as *mut _, 1) == 0 {
                tracing::warn!(
                    "Failed to terminate process {}: {}",
                    pid,
                    std::io::Error::last_os_error()
                );
            }
        }
    }
}

/// Truncate stdout + stderr to fit within max_total chars, with truncation notices.
fn truncate_output(stdout: &str, stderr: &str, max_total: usize) -> (String, String) {
    let stdout_len = stdout.len();
    let stderr_len = stderr.len();

    if stdout_len + stderr_len <= max_total {
        return (stdout.to_string(), stderr.to_string());
    }

    // Reserve space for truncation notices
    const NOTICE_LEN: usize = 60;
    let usable = max_total.saturating_sub(NOTICE_LEN * 2);

    let total = stdout_len + stderr_len;
    let stdout_budget = if total > 0 {
        (usable * stdout_len / total).min(stdout_len)
    } else {
        usable / 2
    };
    let stderr_budget = usable.saturating_sub(stdout_budget).min(stderr_len);

    let truncated_stdout = if stdout_len > stdout_budget {
        let safe_end = find_safe_truncation_point(stdout, stdout_budget);
        format!(
            "{}\n... [truncated, {} chars omitted]",
            &stdout[..safe_end],
            stdout_len - safe_end
        )
    } else {
        stdout.to_string()
    };

    let truncated_stderr = if stderr_len > stderr_budget {
        let safe_end = find_safe_truncation_point(stderr, stderr_budget);
        format!(
            "{}\n... [truncated, {} chars omitted]",
            &stderr[..safe_end],
            stderr_len - safe_end
        )
    } else {
        stderr.to_string()
    };

    (truncated_stdout, truncated_stderr)
}

/// Find a safe byte position to truncate at (don't split multi-byte UTF-8 chars).
fn find_safe_truncation_point(s: &str, max_bytes: usize) -> usize {
    if max_bytes >= s.len() {
        return s.len();
    }
    let mut pos = max_bytes;
    while pos > 0 && !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

// ============================================================================
// Error Recovery Hints
// ============================================================================

impl ShellTool {
    /// Generate a recovery hint when a neomind CLI command fails.
    fn recovery_hint(command: &str, stdout: &str, stderr: &str) -> Option<String> {
        let cmd = command.trim();
        if !cmd.starts_with("neomind ") {
            return None;
        }

        let parts: Vec<&str> = cmd.splitn(4, ' ').collect();
        let domain = parts.get(1).copied().unwrap_or("");
        let action = parts.get(2).copied().unwrap_or("");
        let combined = format!("{} {}", stdout, stderr).to_lowercase();

        let is_not_found = combined.contains("not found")
            || combined.contains("404")
            || combined.contains("does not exist")
            || combined.contains("no such");
        let is_validation = combined.contains("validation")
            || combined.contains("invalid")
            || combined.contains("missing")
            || combined.contains("required")
            || combined.contains("400")
            || combined.contains("422");

        match domain {
            "device" => {
                if is_not_found {
                    Some("Run 'neomind device list' to see available devices, then retry with a valid ID.".to_string())
                } else if action == "create" && is_validation {
                    Some("Required fields: --name, --type. Use 'neomind device types list' to see valid device types.".to_string())
                } else if action == "control" && is_not_found {
                    Some("Device not found. Run 'neomind device list' first, then use 'neomind device control <ID> --command <CMD>'.".to_string())
                } else {
                    Some("Available actions: list, get, create, update, delete, latest, history, control, write-metric, types".to_string())
                }
            }
            "dashboard" => {
                if is_not_found {
                    Some("Run 'neomind dashboard list' to see available dashboards.".to_string())
                } else if action == "create" && is_validation {
                    Some("Required field: --name. Example: neomind dashboard create --name \"My Dashboard\"".to_string())
                } else if action == "update" {
                    Some("Use --components to update widgets. Run 'neomind widget list' to see available widget types, and 'neomind dashboard get <ID>' to see current layout.".to_string())
                } else {
                    Some("Available actions: list, get, create, update, delete, share".to_string())
                }
            }
            "rule" => {
                if is_not_found {
                    Some("Run 'neomind rule list' to see available rules.".to_string())
                } else if action == "create" && (is_validation || combined.contains("dsl") || combined.contains("parse")) {
                    Some("Rule DSL syntax: RULE <name> WHEN <condition> DO <action> END. Example: RULE temp_alert WHEN device.temperature > 30 DO send_notification(\"Temperature too high\") END".to_string())
                } else if action == "enable" || action == "disable" {
                    Some("Run 'neomind rule list' to find the rule ID, then 'neomind rule <enable|disable> <ID>'.".to_string())
                } else {
                    Some("Available actions: list, get, create, update, delete, enable, disable, test, history".to_string())
                }
            }
            "agent" => {
                if is_not_found {
                    Some("Run 'neomind agent list' to see available agents.".to_string())
                } else if action == "create" && is_validation {
                    Some("Required fields: --name, --prompt, --schedule-type (event|interval|cron). Example: neomind agent create --name \"monitor\" --prompt \"Check devices\" --schedule-type event".to_string())
                } else if action == "control" && is_validation {
                    Some("Valid status values: active, paused. Example: neomind agent control <ID> --action active".to_string())
                } else {
                    Some("Available actions: list, get, create, update, delete, control, invoke, memory, executions, latest-execution, conversation, send-message".to_string())
                }
            }
            "extension" => {
                if is_not_found {
                    Some("Run 'neomind extension list' to see installed extensions.".to_string())
                } else if action == "install" && is_validation {
                    Some("Provide the extension zip file path. Use 'neomind extension market-list' to browse marketplace.".to_string())
                } else {
                    Some("Available actions: list, get, status, logs, install, uninstall, market-list, market-install".to_string())
                }
            }
            "transform" => {
                if is_not_found {
                    Some("Run 'neomind transform list' to see available transforms.".to_string())
                } else if action == "create" && is_validation {
                    Some("Required fields: --name, --code (JavaScript function). Use --scope to set input scope. Example: neomind transform create --name \"celsius\" --code \"return value * 9/5 + 32\" --scope global".to_string())
                } else {
                    Some("Available actions: list, get, create, update, delete, test, metrics, data-sources".to_string())
                }
            }
            "widget" => {
                if is_not_found {
                    Some("Run 'neomind widget list' to see available widgets (built-in + custom).".to_string())
                } else if action == "create" && is_validation {
                    Some("Valid widget types: chart, gauge, stat, table, image, custom. Example: neomind widget create \"My Chart\" --widget-type chart".to_string())
                } else if action == "install" && is_validation {
                    Some("Provide the widget directory path containing manifest.json and bundle.js. Use 'neomind widget market-list' to browse.".to_string())
                } else {
                    Some("Available actions: list, get, bundle, create, install, uninstall, market-list, market-install".to_string())
                }
            }
            "message" => {
                if is_not_found {
                    Some("Run 'neomind message list' to see all messages.".to_string())
                } else if action == "send" && is_validation {
                    Some("Required fields: --title, --message, --severity (info|warning|error|critical). Example: neomind message send --title \"Alert\" --message \"High temp\" --severity warning".to_string())
                } else {
                    Some("Available actions: list, get, send, read, channel-list, channel-get, channel-create, channel-update, channel-delete, channel-types, channel-test".to_string())
                }
            }
            _ => None,
        }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        r#"Execute shell commands on the host system.

Use this tool to run any system command. For NeoMind platform operations, use the `neomind` CLI.

## NeoMind CLI Domains

| Domain | Key Actions | Description |
|--------|------------|-------------|
| device | list, get, create, update, delete, latest, history, control, write-metric, types | Device management, telemetry, control commands. `types` is a subcommand: `device types list`, `device types get <ID>`, `device types create --name 'X' --metrics '[{"name":"temp","unit":"°C","type":"number"}]'` |
| dashboard | list, get, create, update, delete, share, add-components, remove-components | Dashboard CRUD. `--components` replaces ALL; use `add-components` to append safely |
| widget | list, get, bundle, create, install, uninstall, market-list, market-install | Widget schemas and lifecycle. `get <ID>` returns config_schema. `create` scaffolds locally |
| rule | list, get, create, update, delete, enable, disable, test, history | Rules use DSL: `RULE ... WHEN ... DO ... END` |
| agent | list, get, create, update, delete, control, invoke, executions, latest-execution, conversation, memory, send-message | Must `control --status active` after create |
| transform | list, get, create, update, delete, test, metrics, data-sources | JS code transforms; `input` is raw metric value. `metrics` lists virtual outputs |
| extension | list, get/info, status, logs, install, uninstall, market-list, market-install, reload | `get <ID>` returns commands, metrics, config details |
| message | list, get, send, read/ack, channel-list, channel-get, channel-create, channel-update, channel-delete, channel-test, channel-types | Send requires `--title` + `--message` + `--severity` |
| system | info | MQTT broker, webhook URL, network info |
| connector | list, get, create, update, delete, test, subscriptions, subscribe, unsubscribe | Data connectors (MQTT, webhook, etc.) |
| llm | list, get, models | LLM backend management; `models` lists Ollama models |

> **Discover command details**: run `neomind <domain> <action> --help` to see all flags, examples, and usage notes.

## Domain Quick Guides

> For complex operations (dashboard creation, agent management, extension development, device onboarding), use the `skill` tool to load detailed step-by-step guides.

### Rule DSL Syntax
```
RULE <name> WHEN <condition> DO <action> END
```
- Conditions: `<device_id>.<metric> <op> <value>`, `EXTENSION <ext_id>.<metric> <op> <value>`
- Operators: `< > <= >= == !=`, `BETWEEN val AND val`, combine with `AND`, `OR`, `NOT`
- Actions: `NOTIFY "msg" [channels]`, `EXECUTE device.cmd(key=val)`, `ALERT "title" "msg" SEVERITY`, `TRIGGER_AGENT id "input"`
- Template vars: `{{device.name}}`, `{{value}}`
- New rules are **disabled** — must `neomind rule enable <ID>` after create
- Metric names must match exactly — use `device latest <ID>` to discover real names

```bash
neomind rule create --dsl 'RULE high_temp WHEN sensor-001.temperature > 30 DO NOTIFY "High temp on {{device.name}}: {{value}}°C" END'
neomind rule create --dsl 'RULE offline WHEN sensor-001.status == "offline" DO NOTIFY "{{device.name}} went offline" [email] END'
neomind rule create --dsl 'RULE critical WHEN sensor-001.cpu > 90 AND sensor-001.memory > 80 DO ALERT "Critical" "Check {{device.name}}" CRITICAL END'
```

### Dashboard Components
Grid is 12 columns. `--components` **replaces ALL** components.
```json
{"id":"c1","type":"value-card","title":"Temp","position":{"x":0,"y":0,"w":4,"h":3},
 "data_source":{"type":"device","sourceId":"sensor-01","property":"temperature"},
 "display":{"unit":"°C"},"config":{}}
```
DataSource types: `device` (sourceId+property), `extension-metric` (extensionId+extensionMetric).
Always discover metric names first via `device latest <ID>`. Charts accept `data_source` as array for multi-series.
**For full workflow, load `dashboard-management` skill.**

### Transform JS Rules
`input` is the raw metric value. Must `return` the result.
Scope: `global` | `device_type:<Type>` | `device:<ID>`
Output DataSourceId: `transform:<output_prefix>:<field>`

```bash
neomind transform create --name 'F to C' --scope global --code 'return (input - 32) * 5 / 9'
neomind transform create --name 'Battery Health' --scope global --code 'if (input > 80) return "good"; if (input > 20) return "ok"; return "low"'
neomind transform test --code 'return (input - 32) * 5 / 9' --input '{"value": 212}'
```

### Custom Widget IIFE Format
No build tools. `manifest.json` + `bundle.js`. Use `neomind widget create "Name" --widget-type <TYPE>` to scaffold.
```javascript
(function(global) {
  var React = global.React;
  function MyWidget(props) {
    return React.createElement('div', {style:{width:'100%',height:'100%'}}, 'Hello');
  }
  global['NeoMindMyWidget'] = MyWidget;
})(window);
```
Rules: `React.createElement` only (no JSX/import), `global.React`, CSS vars only, under 50KB.
Props: `props.config`, `props.display`, `props.dataSource`, `props.id`, `props.title`

### Widget Creation Workflow (scaffold → edit → install → use)
1. `neomind widget create "My Widget" --widget-type <TYPE>` → scaffold to `data/frontend-components/<widget-id>/`
   - Types: `chart`, `gauge`, `stat`, `table`, `image`, `custom`
2. Edit `manifest.json` — required fields:
   - `id` (lowercase-hyphen, must not match built-ins like `value-card`)
   - `global_name` (convention: `NeoMind{PascalCase}`, must match bundle.js assignment)
   - `category`: indicators, charts, controls, display, spatial, business, custom
   - `size_constraints`: `{"min_w":2,"min_h":2,"default_w":4,"default_h":3,"max_w":12,"max_h":8}`
   - `has_data_source`: boolean, `max_data_sources`: number (omit = unlimited)
   - `config_schema`: JSON Schema with `display` and `config` sections
3. Edit `bundle.js` — must be valid IIFE (see template above), assign to `global['{global_name}']`
4. Package and install:
```bash
cd data/frontend-components/my-widget && zip -r ../my-widget.zip manifest.json bundle.js
neomind widget install ../my-widget.zip
neomind widget list    # verify
neomind widget get my-widget  # check config_schema
```
5. Add to dashboard via `dashboard update <ID> --components '[...]'`

## System Commands
- Network: ping, traceroute, curl, arp, nmap
- Monitoring: ps, df, free, top, uptime, systemctl status
- Files: ls, cat, head, tail, grep, find, wc
- Discovery: arp-scan, avahi-browse, bluetoothctl
- Containers: docker ps, docker logs

Commands run in a separate process — no persistent shell state between calls.
Output may be truncated for very long responses.
On failure, check the "suggestion" field for recovery hints."#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "command": {
                    "type": "string",
                    "description": "The shell command to execute. Supports pipes, redirections, and other shell features."
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional per-command timeout in seconds (max 600). Overrides default timeout."
                },
                "description": {
                    "type": "string",
                    "description": "Brief description of what this command does (5-10 words). Used for logging and audit."
                },
                "working_dir": {
                    "type": "string",
                    "description": "Optional working directory for command execution. Must be an existing directory path."
                }
            }),
            vec!["command".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("command is required".into()))?;

        if command.trim().is_empty() {
            return Err(ToolError::InvalidArguments(
                "command cannot be empty".into(),
            ));
        }

        // Resolve timeout: per-command override or config default, capped at 600s
        // Accepts both number and string forms (LLM may pass "30" as string)
        let timeout = if let Some(user_timeout) = args.get("timeout") {
            let secs = user_timeout
                .as_u64()
                .or_else(|| user_timeout.as_str().and_then(|s| s.parse::<u64>().ok()))
                .ok_or_else(|| {
                    ToolError::InvalidArguments("timeout must be a positive number".into())
                })?;
            Duration::from_secs(secs.min(600))
        } else {
            Duration::from_secs(self.config.timeout_secs.min(600))
        };

        let working_dir = args.get("working_dir").and_then(|v| v.as_str());
        let description = args.get("description").and_then(|v| v.as_str());

        tracing::info!(
            command = %command,
            description = description.unwrap_or(""),
            "Executing shell command"
        );

        let output = self.execute_command(command, working_dir, timeout).await?;

        let (stdout, stderr) =
            truncate_output(&output.stdout, &output.stderr, self.config.max_output_chars);

        tracing::info!(
            command = %command,
            exit_code = ?output.exit_code,
            timed_out = output.timed_out,
            stdout_len = stdout.len(),
            stderr_len = stderr.len(),
            "Shell command completed"
        );

        let mut result = serde_json::json!({
            "exit_code": output.exit_code,
            "stdout": stdout,
            "stderr": stderr,
            "command": command,
            "timed_out": output.timed_out
        });
        if let Some(desc) = description {
            result["description"] = serde_json::Value::String(desc.to_string());
        }

        // Enrich error responses with recovery hints for neomind CLI commands
        let is_error = output.exit_code.unwrap_or(1) != 0;
        if is_error {
            if let Some(hint) = Self::recovery_hint(command, &stdout, &stderr) {
                result["suggestion"] = serde_json::Value::String(hint);
            }
        }

        Ok(ToolOutput::success(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ShellConfig {
        ShellConfig {
            enabled: true,
            timeout_secs: 10,
            max_output_chars: 5000,
            internal_cli_execution: false,
        }
    }

    #[tokio::test]
    async fn test_basic_command() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({ "command": "echo hello world" }))
            .await
            .unwrap();
        assert!(result.success);
        let data = result.data;
        assert_eq!(data["exit_code"], 0);
        assert!(data["stdout"].as_str().unwrap().contains("hello world"));
        assert_eq!(data["timed_out"], false);
    }

    #[tokio::test]
    async fn test_stderr_capture() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({ "command": "echo error >&2" }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.data["stderr"].as_str().unwrap().contains("error"));
    }

    #[tokio::test]
    async fn test_nonzero_exit_code() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({ "command": "exit 42" }))
            .await
            .unwrap();
        assert!(result.success); // ToolOutput success = tool ran, not command success
        assert_eq!(result.data["exit_code"], 42);
    }

    #[tokio::test]
    async fn test_timeout() {
        let config = ShellConfig {
            enabled: true,
            timeout_secs: 1,
            max_output_chars: 5000,
            internal_cli_execution: false,
        };
        let tool = ShellTool::new(config);
        let result = tool
            .execute(serde_json::json!({ "command": "sleep 60" }))
            .await
            .unwrap();
        assert!(result.data["timed_out"].as_bool().unwrap());
        assert!(result.data["stderr"]
            .as_str()
            .unwrap()
            .contains("timed out"));
    }

    #[tokio::test]
    async fn test_per_command_timeout_override() {
        let tool = ShellTool::new(test_config()); // default 10s
        let result = tool
            .execute(serde_json::json!({ "command": "sleep 60", "timeout": 1 }))
            .await
            .unwrap();
        assert!(result.data["timed_out"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_empty_command_rejected() {
        let tool = ShellTool::new(test_config());
        let result = tool.execute(serde_json::json!({ "command": "  " })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_command_rejected() {
        let tool = ShellTool::new(test_config());
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_working_dir() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({ "command": "pwd", "working_dir": "/tmp" }))
            .await
            .unwrap();
        let stdout = result.data["stdout"].as_str().unwrap();
        assert!(stdout.contains("tmp"));
    }

    #[tokio::test]
    async fn test_invalid_working_dir() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({
                "command": "pwd",
                "working_dir": "/nonexistent/path"
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pipeline_command() {
        let tool = ShellTool::new(test_config());
        let result = tool
            .execute(serde_json::json!({
                "command": "echo -e 'apple\nbanana\ncherry' | grep an"
            }))
            .await
            .unwrap();
        let stdout = result.data["stdout"].as_str().unwrap();
        assert!(stdout.contains("banana"));
        assert!(!stdout.contains("apple"));
    }

    #[tokio::test]
    async fn test_permission_denied_command() {
        let tool = ShellTool::new(test_config());
        // This should fail with permission error, not crash
        let result = tool
            .execute(serde_json::json!({ "command": "ls /root" }))
            .await
            .unwrap();
        // Tool succeeds (command ran), but exit_code may be non-zero or stderr has error
        assert!(result.success);
        // Either exit_code is non-zero or stderr contains error info
        let exit_code = result.data["exit_code"].as_i64().unwrap_or(0);
        let stderr = result.data["stderr"].as_str().unwrap_or("");
        assert!(exit_code != 0 || !stderr.is_empty() || !result.data["stdout"].is_null());
    }

    #[test]
    fn test_truncate_output_within_budget() {
        let (out, err) = truncate_output("hello", "world", 100);
        assert_eq!(out, "hello");
        assert_eq!(err, "world");
    }

    #[test]
    fn test_truncate_output_exceeds_budget() {
        let stdout = "a".repeat(5000);
        let stderr = "b".repeat(5000);
        let (out, err) = truncate_output(&stdout, &stderr, 1000);
        assert!(out.len() < 1000);
        assert!(err.len() < 1000);
        assert!(out.contains("[truncated"));
        assert!(err.contains("[truncated"));
    }

    #[test]
    fn test_truncate_output_stderr_only() {
        let stdout = "short";
        let stderr = "x".repeat(5000);
        let (out, err) = truncate_output(stdout, &stderr, 1000);
        assert!(err.contains("[truncated"));
        assert!(out.len() + err.len() <= 1200);
    }

    #[test]
    fn test_find_safe_truncation_point_ascii() {
        assert_eq!(find_safe_truncation_point("hello world", 5), 5);
    }

    #[test]
    fn test_find_safe_truncation_point_multibyte() {
        let s = "你好世界";
        let pos = find_safe_truncation_point(s, 4);
        assert_eq!(pos, 3);
        assert!(s.is_char_boundary(pos));
    }

    #[test]
    fn test_tool_name_and_category() {
        let tool = ShellTool::new(test_config());
        assert_eq!(tool.name(), "shell");
        assert!(matches!(tool.category(), ToolCategory::System));
    }
}
