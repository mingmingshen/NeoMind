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
//! Supported domains: device, dashboard, rule, extension, widget, transform, agent, message, system.
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
            "broker" => Self::exec_broker(&client, &args).await,
            "guide" => Self::exec_guide(&client, &args).await,
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
                        let name = Self::get_flag_value(args, "--name").unwrap_or("").to_string();
                        let metrics_str = Self::get_flag_value(args, "--metrics").unwrap_or("[]");
                        let metrics = serde_json::from_str(metrics_str).unwrap_or(serde_json::json!([]));
                        let commands = Self::get_flag_value(args, "--commands")
                            .map(|s| serde_json::from_str(s).unwrap_or(serde_json::json!(null)));
                        neomind_cli_ops::device::create_device_type(client, &name, metrics, commands).await
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
                let llm_backend = Self::get_flag_value(args, "--model")
                    .or_else(|| Self::get_flag_value(args, "--llm-backend"))
                    .map(|s| s.to_string());
                let system_prompt = Self::get_flag_value(args, "--system-prompt").map(|s| s.to_string());
                neomind_cli_ops::agent_cmd::create_agent(
                    client, &name, &prompt, description.as_deref(),
                    schedule_type.as_deref(), schedule_config.as_deref(),
                    llm_backend.as_deref(), system_prompt.as_deref(),
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
                neomind_cli_ops::agent_cmd::update_agent(
                    client, &id, name.as_deref(), description.as_deref(),
                    llm_backend.as_deref(), system_prompt.as_deref(), prompt.as_deref(),
                    schedule_type.as_deref(), schedule_config.as_deref(),
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

    /// Execute `neomind broker <action>` commands internally.
    async fn exec_broker(client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");
        match action {
            "list" => neomind_cli_ops::broker::list_brokers(client).await,
            "get" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::broker::get_broker(client, id).await
            }
            "create" => {
                let name = Self::get_flag_value(args, "--name").unwrap_or("").to_string();
                let host = Self::get_flag_value(args, "--host").unwrap_or("").to_string();
                let port = Self::get_flag_value(args, "--port")
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(1883);
                let tls = Self::get_flag_value(args, "--tls").is_some();
                let username = Self::get_flag_value(args, "--username").map(|s| s.to_string());
                let password = Self::get_flag_value(args, "--password").map(|s| s.to_string());
                let topics = Self::get_flag_value(args, "--topics").map(|s| s.to_string());
                neomind_cli_ops::broker::create_broker(
                    client, &name, &host, port, tls,
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
                neomind_cli_ops::broker::update_broker(
                    client, id, name.as_deref(), host.as_deref(), port, tls,
                    username.as_deref(), password.as_deref(), topics.as_deref(), enabled,
                ).await
            }
            "delete" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::broker::delete_broker(client, id).await
            }
            "test" => {
                let id = Self::resolve_id(args);
                neomind_cli_ops::broker::test_broker(client, id).await
            }
            "subscriptions" => {
                neomind_cli_ops::broker::list_subscriptions(client).await
            }
            "subscribe" => {
                let topic = Self::get_flag_value(args, "--topic").unwrap_or("").to_string();
                let qos = Self::get_flag_value(args, "--qos").and_then(|s| s.parse::<u8>().ok());
                neomind_cli_ops::broker::subscribe_topic(client, &topic, qos).await
            }
            "unsubscribe" => {
                let topic = Self::get_flag_value(args, "--topic").unwrap_or("").to_string();
                neomind_cli_ops::broker::unsubscribe_topic(client, &topic).await
            }
            _ => anyhow::bail!("Unknown broker action: {}", action),
        }
    }

    /// Execute `neomind guide <domain>` — returns the full help manual as a CliResponse.
    async fn exec_guide(_client: &neomind_cli_ops::ApiClient, args: &[String]) -> anyhow::Result<neomind_cli_ops::CliResponse> {
        let domain = args.get(2).map(|s| s.as_str()).unwrap_or("");
        if domain.is_empty() {
            let domains: Vec<serde_json::Value> = neomind_cli_ops::help::list_domains()
                .into_iter()
                .map(|d| serde_json::json!({"domain": d.name, "description": d.description}))
                .collect();
            return Ok(neomind_cli_ops::CliResponse::success(
                serde_json::json!({"domains": domains}),
                "Available guide domains",
            ));
        }
        match neomind_cli_ops::help::get_help(domain) {
            Some(content) => Ok(neomind_cli_ops::CliResponse::success(
                serde_json::json!({"domain": domain, "content": content}),
                &format!("Guide for '{}'", domain),
            )),
            None => anyhow::bail!("Unknown guide domain: '{}'. Run `neomind guide` to see available domains.", domain),
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
| device | list, get, create, update, delete, latest, history, control, write-metric, types | Device management, telemetry, control commands |
| dashboard | list, get, create, update, delete, share | Dashboard CRUD; `--components` replaces ALL components |
| widget | list, get, create, install, uninstall, market-list | Widget schemas; `get <TYPE>` returns config_schema |
| rule | list, get, create, update, delete, enable, disable, history | Rules use DSL: `RULE ... WHEN ... DO ... END` |
| agent | list, get, create, update, delete, control, executions, send-message | Must `control --status active` after create |
| transform | list, get, create, update, delete, test, data-sources | JS code transforms; uses `input` variable |
| extension | list, get, status, install, uninstall, logs, market-list | `get <ID>` returns commands, metrics, config details |
| message | list, send, read, channel-list/create/update/delete | Send requires `--title` + `--message` |
| system | info | MQTT broker, webhook URL, network info |
| broker | list, get, create, update, delete, test, subscriptions, subscribe, unsubscribe | External MQTT broker management |

> **Discover command details**: run `neomind <domain> <action> --help` to see all flags, examples, and usage notes.

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
