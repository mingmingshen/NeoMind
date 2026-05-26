//! Command-line interface for NeoMind.

use std::io::Read;
use std::net::SocketAddr;
use std::path::Path;
use std::time::SystemTime;

use anyhow::Result;
use clap::{Parser, Subcommand};
use neomind_agent::{LlmBackend, SessionManager};
use neomind_core::config::{
    endpoints, env_vars, models, normalize_ollama_endpoint, normalize_openai_endpoint,
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

/// NeoMind AI Agent - Run LLMs on edge devices.
#[derive(Parser, Debug)]
#[command(name = "neomind")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Action to perform.
    #[command(subcommand)]
    command: Command,

    /// Model path or identifier.
    #[arg(short, long, global = true)]
    model: Option<String>,

    /// Verbose output.
    #[arg(short, long, global = true)]
    verbose: bool,
}

/// Available commands.
#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Start the web server.
    Serve {
        /// Host to bind to.
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        /// Port to bind to.
        #[arg(short, long, default_value_t = 9375)]
        port: u16,
    },
    /// Run a single prompt and exit.
    Prompt {
        /// The prompt to process.
        prompt: String,
        /// Maximum tokens to generate.
        #[arg(long, default_value_t = usize::MAX)]
        max_tokens: usize,
        /// Temperature.
        #[arg(short, long, default_value_t = 0.7)]
        temperature: f32,
    },
    /// Chat mode (interactive REPL with session persistence).
    Chat {
        /// Session ID to resume (optional).
        #[arg(short, long)]
        session: Option<String>,
    },
    /// List available models from Ollama.
    ListModels {
        /// Ollama endpoint.
        #[arg(long, default_value = "http://localhost:11434")]
        endpoint: String,
    },
    /// Check system health and status.
    Health,
    /// View system logs.
    Logs {
        /// Number of lines to show (default: 50).
        #[arg(long, default_value_t = 50)]
        tail: usize,
        /// Follow log output (like tail -f).
        #[arg(long)]
        follow: bool,
        /// Filter by log level (ERROR, WARN, INFO, DEBUG).
        #[arg(long)]
        level: Option<String>,
        /// Show logs from the last duration (e.g., "1h", "30m").
        #[arg(long)]
        since: Option<String>,
    },
    /// Check for updates.
    CheckUpdate,
    /// LLM backend management commands.
    Llm {
        #[command(subcommand)]
        llm_cmd: LlmCommand,
    },
    /// Extension management commands.
    Extension {
        #[command(subcommand)]
        extension_cmd: ExtensionCommand,
    },
    /// API key management commands.
    ApiKey {
        #[command(subcommand)]
        key_cmd: ApiKeyCommand,
    },
    /// Device management commands.
    Device {
        #[command(subcommand)]
        device_cmd: DeviceCommand,
    },
    /// Dashboard management commands.
    Dashboard {
        #[command(subcommand)]
        dashboard_cmd: DashboardCommand,
    },
    /// Rule management commands.
    Rule {
        #[command(subcommand)]
        rule_cmd: RuleCommand,
    },
    /// Transform management commands.
    Transform {
        #[command(subcommand)]
        transform_cmd: TransformCommand,
    },
    /// Agent management commands.
    Agent {
        #[command(subcommand)]
        agent_cmd: AgentCommand,
    },
    /// Message management commands.
    Message {
        #[command(subcommand)]
        message_cmd: MessageCommand,
    },
    /// Data push management commands.
    ///
    /// Forward device metrics and extension outputs to external systems.
    /// Target types: webhook, mqtt. Schedule types: event (real-time), interval.
    Push {
        #[command(subcommand)]
        push_cmd: PushCommand,
    },
    /// Widget management commands.
    Widget {
        #[command(subcommand)]
        widget_cmd: WidgetCommand,
    },
    /// System information and infrastructure.
    System {
        #[command(subcommand)]
        system_cmd: SystemCommand,
    },
    /// Data connector management (MQTT, webhook, HTTP, etc.).
    Connector {
        #[command(subcommand)]
        connector_cmd: ConnectorCommand,
    },
    /// Deprecated: use 'connector' instead.
    #[command(hide = true)]
    Broker {
        #[command(subcommand)]
        broker_cmd: BrokerAliasCommand,
    },
}

/// API key subcommands.
#[derive(Subcommand, Debug)]
enum ApiKeyCommand {
    /// Create a new API key.
    ///
    /// Generates an API key for authenticating external requests.
    /// Example: `neomind api-key create --name my-app`
    Create {
        /// Name for the key.
        #[arg(short, long, default_value = "default")]
        name: String,
        /// Data directory path.
        #[arg(long, default_value = "data")]
        data_dir: String,
    },
    /// List all API keys.
    ///
    /// Shows all registered API key names (values are masked).
    /// Example: `neomind api-key list`
    List {
        /// Data directory path.
        #[arg(long, default_value = "data")]
        data_dir: String,
    },
    /// Delete an API key by name.
    ///
    /// Removes the key immediately; all requests using it will be rejected.
    /// Example: `neomind api-key delete my-app`
    Delete {
        /// Key name to delete.
        name: String,
        /// Data directory path.
        #[arg(long, default_value = "data")]
        data_dir: String,
    },
}

/// LLM backend subcommands.
#[derive(Subcommand, Debug)]
enum LlmCommand {
    /// List configured LLM backends.
    ///
    /// Shows all registered LLM backend instances with their ID, type, and model.
    /// Use the ID as --llm-backend value in agent create/update.
    ///
    /// Example: `neomind llm list`
    List {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Get LLM backend details.
    ///
    /// Shows full backend config including endpoint, model, and parameters.
    ///
    /// Example: `neomind llm get ollama-default`
    Get {
        /// Backend ID.
        #[arg(required = true)]
        id: String,
    },
    /// List available models from Ollama.
    ///
    /// Queries the Ollama server for models that can be pulled and used.
    /// Use model names when configuring LLM backends.
    ///
    /// Example: `neomind llm models`
    Models {
        /// Ollama endpoint.
        #[arg(long, default_value = "http://localhost:11434")]
        endpoint: String,
    },
}

/// Extension subcommands.
#[derive(Subcommand, Debug)]
enum ExtensionCommand {
    /// Validate a .nep extension package.
    ///
    /// Checks manifest, binary compatibility, and schema before installing.
    /// Always validate before `extension install` to catch issues early.
    /// Use -v for detailed output including all metrics and commands.
    /// Example: `neomind extension validate ./my-extension.nep -v`
    Validate {
        /// Path to the .nep file.
        #[arg(required = true)]
        path: std::path::PathBuf,
        /// Show detailed output.
        #[arg(short, long)]
        verbose: bool,
    },
    /// List installed extensions.
    ///
    /// Shows all installed extensions with their ID, version, and status.
    /// Use -v for detailed metrics and commands info.
    /// Example: `neomind extension list -v`
    List {
        /// Show detailed information.
        #[arg(short, long)]
        verbose: bool,
    },
    /// Show extension information.
    ///
    /// Displays manifest details: version, description, metrics, commands.
    /// Example: `neomind extension info weather-forecast`
    Info {
        /// Extension ID or .nep file path.
        #[arg(required = true)]
        id_or_path: String,
    },
    /// Install a .nep extension package.
    ///
    /// Installs from a local file path. The extension is loaded immediately.
    /// Example: `neomind extension install ./weather-forecast-v2.nep`
    Install {
        /// Path to the .nep file or URL.
        #[arg(required = true)]
        package: String,
    },
    /// Uninstall an extension.
    ///
    /// Stops the extension process and removes all files. This is irreversible.
    /// Example: `neomind extension uninstall weather-forecast`
    Uninstall {
        /// Extension ID.
        #[arg(required = true)]
        id: String,
    },
    /// Create a new extension scaffold.
    ///
    /// Generates a complete extension project with Cargo.toml, lib.rs, and manifest.
    /// Example: `neomind extension create my-extension --extension-type tool -o ./extensions`
    Create {
        /// Extension ID (lowercase, hyphens only).
        #[arg(required = true)]
        name: String,
        /// Extension type.
        #[arg(short, long, default_value = "tool")]
        extension_type: String,
        /// Output directory.
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Get extension health status.
    ///
    /// Shows process status, uptime, last error, and resource usage.
    /// Example: `neomind extension status weather-forecast`
    Status {
        /// Extension ID.
        #[arg(required = true)]
        id: String,
    },
    /// Get extension logs.
    ///
    /// Retrieves recent log output from the extension's isolated process.
    /// Example: `neomind extension logs weather-forecast --lines 50`
    Logs {
        /// Extension ID.
        #[arg(required = true)]
        id: String,
        /// Number of log lines to show.
        #[arg(short, long)]
        lines: Option<usize>,
    },
    /// Build an extension from source.
    ///
    /// Compiles the extension in release mode and produces a .nep package.
    ///
    /// Workflow:
    ///   1. `extension create my-extension` — scaffold
    ///   2. Edit code in the generated project
    ///   3. `extension build ./my-extension` — compile
    ///   4. `extension validate ./my-extension.nep` — check
    ///   5. `extension install ./my-extension.nep` — deploy
    ///
    ///   Example: `neomind extension build ./extensions/weather-forecast`
    Build {
        /// Extension directory path.
        #[arg(required = true)]
        path: std::path::PathBuf,
    },
    /// Install from marketplace.
    ///
    /// Downloads and installs an extension from the marketplace registry.
    /// Example: `neomind extension market-install weather-forecast --version 2.0.0`
    MarketInstall {
        /// Extension ID in marketplace.
        #[arg(required = true)]
        extension_id: String,
        /// Version to install (optional, defaults to latest).
        #[arg(long)]
        version: Option<String>,
    },
    /// List marketplace extensions.
    ///
    /// Shows all extensions available in the marketplace registry.
    /// Example: `neomind extension market-list`
    MarketList,
    /// Reload an extension.
    ///
    /// Restarts the extension process from its installed files.
    /// Useful after config changes or when the extension is unresponsive.
    /// Example: `neomind extension reload weather-forecast`
    Reload {
        /// Extension ID.
        #[arg(required = true)]
        id: String,
    },
}

/// Device subcommands.
#[derive(Subcommand, Debug)]
enum DeviceCommand {
    /// List all devices.
    ///
    /// Shows device ID, name, type, status, and last seen time.
    /// Use --device-type or --status to filter results.
    /// Use --json for structured output suitable for scripting.
    ///
    /// Workflow: Use this to find device IDs needed by other commands
    /// (get, update, delete, latest, history, control, write-metric).
    ///
    /// Example: `neomind device list --device-type temp_sensor --status online`
    List {
        /// Filter by device type.
        #[arg(short, long)]
        device_type: Option<String>,
        /// Filter by status.
        #[arg(short, long)]
        status: Option<String>,
        /// Output in JSON format.
        #[arg(long)]
        json: bool,
    },
    /// Get device details.
    ///
    /// Shows full device info including type, connection config, metrics, and commands.
    /// Use this to check which commands a device supports before using `device control`.
    ///
    /// Workflow: Find ID with `device list`, then inspect with `device get <ID>`.
    ///
    /// Example: `neomind device get device-001`
    Get {
        /// Device ID.
        #[arg(required = true)]
        id: String,
        /// Output in JSON format.
        #[arg(long)]
        json: bool,
    },
    /// Create a new device.
    ///
    /// First run `neomind device types list` to see available device types,
    /// or use any custom name. Default adapter is "mqtt".
    ///
    /// Workflow:
    ///   1. `neomind device types list` — pick a device type
    ///   2. `neomind device create --name "My Sensor" --device-type temp_sensor --adapter-type mqtt`
    ///   3. Send data via MQTT to the configured topic, or use `device write-metric`
    ///   4. Verify with `device latest <ID>`
    Create {
        /// Device name.
        #[arg(short, long)]
        name: String,
        /// Device type name. Run `neomind device types list` to see built-in types,
        /// or use any custom name (e.g., "sensor", "camera", "switch").
        #[arg(short, long)]
        device_type: String,
        /// Adapter type: mqtt (default) | webhook.
        #[arg(short, long)]
        adapter_type: String,
        /// Connection config JSON. For MQTT: '{"topic":"sensor/data"}'.
        /// For webhook: omit (auto-generated URL).
        #[arg(short, long)]
        config: Option<String>,
        /// Output in JSON format.
        #[arg(long)]
        json: bool,
    },
    /// Update device.
    ///
    /// Modify device name or connection config.
    ///
    /// Workflow: Find ID with `device list`, update fields as needed.
    /// To change MQTT topic: `device update <ID> --config '{"topic":"new/topic"}'`.
    ///
    /// Example: `neomind device update device-001 --name "Kitchen Sensor"`
    Update {
        /// Device ID.
        #[arg(required = true)]
        id: String,
        /// New name.
        #[arg(short, long)]
        name: Option<String>,
        /// Connection config JSON.
        #[arg(short, long)]
        config: Option<String>,
        /// Output in JSON format.
        #[arg(long)]
        json: bool,
    },
    /// Delete device.
    ///
    /// Removes the device and ALL associated telemetry data. This is irreversible.
    /// Consider exporting data first with `device history` if needed.
    ///
    /// Example: `neomind device delete device-001`
    Delete {
        /// Device ID.
        #[arg(required = true)]
        id: String,
        /// Output in JSON format.
        #[arg(long)]
        json: bool,
    },
    /// Get latest metrics.
    ///
    /// Returns the most recent metric values reported by the device.
    /// Useful for quick health checks or dashboards.
    /// For historical data, use `device history` instead.
    ///
    /// Example: `neomind device latest device-001`
    Latest {
        /// Device ID.
        #[arg(required = true)]
        id: String,
        /// Output in JSON format.
        #[arg(long)]
        json: bool,
    },
    /// Get telemetry history.
    ///
    /// Retrieves time-series metric data. Use --time-range to specify period
    /// (1h, 24h, 7d, 30d). Use --compress for AI-friendly compressed series
    /// that retains trends and allows up to 90 days of data.
    ///
    /// Workflow:
    ///   1. `device history <ID>` — all metrics, last 24h
    ///   2. `device history <ID> --metric temperature --time-range 7d` — specific metric
    ///   3. `device history <ID> --compress` — compact for AI consumption
    ///
    /// Example: `neomind device history device-001 --metric temperature --time-range 24h`
    History {
        /// Device ID.
        #[arg(required = true)]
        id: String,
        /// Metric name to filter (optional, returns all metrics if omitted).
        #[arg(long)]
        metric: Option<String>,
        /// Time range: "1h", "24h", "7d", "30d" (default: 24h).
        #[arg(short, long)]
        time_range: Option<String>,
        /// AI compression mode: lossless adaptive series (kept/fluctuated).
        /// Allows up to 90 days. Designed for AI consumption, not frontend charts.
        #[arg(long)]
        compress: bool,
        /// Output in JSON format.
        #[arg(long)]
        json: bool,
    },
    /// Send control command.
    ///
    /// Sends a command to a device (e.g., toggle switch, set speed, reboot).
    /// Check available commands first: `neomind device get <ID>` shows supported commands.
    ///
    /// Workflow:
    ///   1. `device get <ID>` — check available commands
    ///   2. `device control <ID> <command> --params '<json>'`
    ///
    /// Example: `neomind device control <ID> toggle --params '{"state":true}'`
    Control {
        /// Device ID.
        #[arg(required = true)]
        id: String,
        /// Command name (e.g., "toggle", "set_speed", "reboot").
        #[arg(required = true)]
        command: String,
        /// Command parameters JSON. Example: '{"state":true}'
        #[arg(short, long)]
        params: Option<String>,
        /// Output in JSON format.
        #[arg(long)]
        json: bool,
    },
    /// Device type management.
    Types {
        #[command(subcommand)]
        type_cmd: DeviceTypeCommand,
    },
    /// Write a metric data point.
    ///
    /// Manually push a metric value to a device. Useful for testing or
    /// feeding data from custom sources. The metric name must match the
    /// device type definition.
    ///
    /// Workflow:
    ///   1. `device types get <type>` — check valid metric names
    ///   2. `device write-metric <ID> --metric temp --value 23.5`
    ///   3. `device latest <ID>` — verify the value was recorded
    ///
    /// Example: `neomind device write-metric <ID> --metric temperature --value 25.5`
    WriteMetric {
        /// Device ID.
        #[arg(required = true)]
        id: String,
        /// Metric name (must match device type definition).
        #[arg(long)]
        metric: String,
        /// Value (number, string, or "true"/"false").
        #[arg(long)]
        value: String,
        /// Timestamp in milliseconds (defaults to now).
        #[arg(long)]
        timestamp: Option<i64>,
        /// Output in JSON format.
        #[arg(long)]
        json: bool,
    },
}

/// Device type subcommands.
#[derive(Subcommand, Debug)]
enum DeviceTypeCommand {
    /// List all device types.
    ///
    /// Shows all registered device types including built-in and custom ones.
    /// Use this before `device create` to pick the right --device-type value.
    ///
    /// Example: `neomind device types list`
    List,
    /// Get device type details.
    ///
    /// Shows metrics, commands, and sample data for a device type.
    /// Use this to discover valid metric names for `device write-metric`
    /// or available commands for `device control`.
    ///
    /// Example: `neomind device types get temp_sensor`
    Get {
        /// Type ID.
        #[arg(required = true)]
        id: String,
    },
    /// Create a new device type.
    ///
    /// Example:
    ///   neomind device types create --name "TempSensor" --metrics '[{"name":"temp","display_name":"Temperature","data_type":"Float","unit":"°C"}]'
    ///   neomind device types create --id temp_humidity --name "TempHumidity Sensor" --metrics '[{"name":"temp","display_name":"Temperature","data_type":"Float","unit":"°C"},{"name":"rh","display_name":"Humidity","data_type":"String","unit":"%"}]'
    Create {
        /// Unique type ID (auto-generated from name if omitted, e.g., "TempSensor" -> "temp_sensor").
        #[arg(short, long)]
        id: Option<String>,
        /// Type display name (e.g., "Temperature Sensor").
        #[arg(short, long)]
        name: String,
        /// Metrics JSON array. Each: {"name":"temp","display_name":"Temperature","data_type":"Float","unit":"°C"}
        #[arg(long)]
        metrics: String,
        /// Commands JSON array (optional). Each: {"id":"on","name":"Turn On","params":[]}
        #[arg(long)]
        commands: Option<String>,
    },
    /// Delete device type.
    ///
    /// Removes the type template. Devices already using this type will continue
    /// to work, but new devices cannot be created with this type.
    /// Check usage first: `neomind device list --device-type <type>`.
    ///
    /// Example: `neomind device types delete temp_sensor`
    Delete {
        /// Type ID.
        #[arg(required = true)]
        id: String,
    },
}

/// Dashboard subcommands.
#[derive(Subcommand, Debug)]
enum DashboardCommand {
    /// List all dashboards.
    ///
    /// Shows dashboard ID, name, description, and component count.
    /// Use --json for structured output.
    ///
    /// Workflow: Use this to find dashboard IDs for get/update/delete/share commands.
    ///
    /// Example: `neomind dashboard list --json`
    List {
        /// Output format (json flag for structured output).
        #[arg(long)]
        json: bool,
    },
    /// Get dashboard details.
    ///
    /// Shows full dashboard config including layout and all widget components.
    /// Use --json to get the exact format needed for `dashboard update --components`.
    ///
    /// Example: `neomind dashboard get dash-001`
    Get {
        /// Dashboard ID.
        #[arg(required = true)]
        id: String,
        /// Output format (json flag for structured output).
        #[arg(long)]
        json: bool,
    },
    /// Create a new dashboard.
    ///
    /// Creates an empty dashboard. Add widgets in a second step using
    /// `dashboard update --components`.
    ///
    /// Workflow:
    ///   1. `neomind dashboard create --name "My Dashboard"`
    ///   2. `neomind widget list` — see available widget types
    ///   3. `neomind dashboard update <ID> --components '[...]'` — add widgets
    Create {
        /// Dashboard name.
        #[arg(short, long)]
        name: String,
        /// Dashboard description.
        #[arg(short, long)]
        description: Option<String>,
        /// Layout configuration JSON (optional, auto-generated if omitted).
        #[arg(short, long)]
        layout: Option<String>,
        /// Output format (json flag for structured output).
        #[arg(long)]
        json: bool,
    },
    /// Update dashboard.
    ///
    /// Modify name, description, layout, or components.
    /// WARNING: --components replaces ALL existing components.
    /// Workflow: `dashboard get <ID>` → edit JSON → `dashboard update <ID> --components '...'`
    Update {
        /// Dashboard ID.
        #[arg(required = true)]
        id: String,
        /// New name.
        #[arg(short, long)]
        name: Option<String>,
        /// New description.
        #[arg(short, long)]
        description: Option<String>,
        /// New layout configuration JSON.
        #[arg(short, long)]
        layout: Option<String>,
        /// Dashboard components JSON array. WARNING: Replaces ALL existing components.
        /// Get current with `dashboard get <ID>`, modify the array, then pass back.
        /// Each component: {"type":"widget-type","data_source":{"..."},"display":{...},"config":{...}}
        #[arg(short, long)]
        components: Option<String>,
        /// Output format (json flag for structured output).
        #[arg(long)]
        json: bool,
    },
    /// Add components to dashboard (append mode).
    ///
    /// Appends new widgets without replacing existing ones.
    /// This is the RECOMMENDED way to add widgets.
    ///
    /// Example: `neomind dashboard add-components <ID> --components '[{"id":"c1","type":"value-card",...}]'`
    AddComponents {
        /// Dashboard ID.
        #[arg(required = true)]
        id: String,
        /// JSON array of new components to append.
        #[arg(short, long)]
        components: String,
        /// Output format (json flag for structured output).
        #[arg(long)]
        json: bool,
    },
    /// Remove components from dashboard by ID.
    ///
    /// Removes specific widgets by their component IDs.
    ///
    /// Example: `neomind dashboard remove-components <ID> --ids '["c1","c2"]'`
    RemoveComponents {
        /// Dashboard ID.
        #[arg(required = true)]
        id: String,
        /// JSON array of component IDs to remove.
        #[arg(short, long)]
        ids: String,
        /// Output format (json flag for structured output).
        #[arg(long)]
        json: bool,
    },
    /// Delete dashboard.
    ///
    /// Removes the dashboard and all its widget configurations. This is irreversible.
    /// Shared links will stop working immediately.
    ///
    /// Example: `neomind dashboard delete dash-001`
    Delete {
        /// Dashboard ID.
        #[arg(required = true)]
        id: String,
    },
    /// Share dashboard.
    ///
    /// Generates a shareable link for the dashboard. Use --public for open access
    /// or --expires to set a time-limited link.
    /// Example: `neomind dashboard share dash-001 --public --expires "2025-12-31"`
    Share {
        /// Dashboard ID.
        #[arg(required = true)]
        id: String,
        /// Make public.
        #[arg(short, long)]
        public: bool,
        /// Expiration date/time.
        #[arg(short, long)]
        expires: Option<String>,
        /// Output format (json flag for structured output).
        #[arg(long)]
        json: bool,
    },
}

/// Rule subcommands.
#[derive(Subcommand, Debug)]
enum RuleCommand {
    /// List all rules.
    ///
    /// Shows rule ID, name, status (enabled/disabled), and trigger count.
    /// Use this to find rule IDs for get/update/enable/disable commands.
    ///
    /// Example: `neomind rule list`
    List,
    /// Get rule details.
    ///
    /// Shows the full DSL definition, condition, action, and execution stats.
    /// Use this to inspect a rule before modifying or testing it.
    ///
    /// Example: `neomind rule get rule-001`
    Get {
        /// Rule ID.
        #[arg(required = true)]
        id: String,
    },
    /// Create a new rule.
    ///
    /// Uses NeoMind DSL syntax. Must include RULE...WHEN...DO...END structure.
    /// Example: `neomind rule create --dsl 'RULE "Alert" WHEN device.temperature > 30 DO NOTIFY "Too hot" END'`
    Create {
        /// Rule name (optional, can be set in DSL).
        #[arg(short, long)]
        name: Option<String>,
        /// Rule DSL definition. Syntax: RULE "name" WHEN <condition> DO <action> END
        /// Conditions: device.<metric> <op> <value>, AND/OR for compound.
        /// Actions: NOTIFY "message", device.<metric>.write(<value>)
        #[arg(short, long)]
        dsl: String,
    },
    /// Update rule.
    ///
    /// Modify rule name or DSL definition. The rule is re-evaluated immediately.
    /// Test first with `rule test <ID> --input '...'` to verify new conditions.
    ///
    /// Example: `neomind rule update rule-001 --dsl 'RULE "Alert" WHEN device.temp > 25 DO NOTIFY "Warm" END'`
    Update {
        /// Rule ID.
        #[arg(required = true)]
        id: String,
        /// New name.
        #[arg(short, long)]
        name: Option<String>,
        /// New rule DSL definition.
        #[arg(short, long)]
        dsl: Option<String>,
    },
    /// Delete rule.
    ///
    /// Removes the rule permanently. This is irreversible.
    /// Consider disabling first with `rule disable <ID>` if unsure.
    ///
    /// Example: `neomind rule delete rule-001`
    Delete {
        /// Rule ID.
        #[arg(required = true)]
        id: String,
    },
    /// Enable rule.
    ///
    /// Activates a disabled rule so it starts evaluating conditions again.
    /// After enabling, verify with `rule list` to confirm status change.
    ///
    /// Example: `neomind rule enable rule-001`
    Enable {
        /// Rule ID.
        #[arg(required = true)]
        id: String,
    },
    /// Disable rule.
    ///
    /// Pauses rule evaluation without deleting it. Can be re-enabled later.
    /// Prefer this over deleting if you want to temporarily stop a rule.
    ///
    /// Example: `neomind rule disable rule-001`
    Disable {
        /// Rule ID.
        #[arg(required = true)]
        id: String,
    },
    /// Test rule.
    ///
    /// Evaluates a rule against sample input data without triggering actions.
    /// Input must be a JSON object with metric values matching device field names.
    /// Use before enabling a new rule to verify it behaves correctly.
    ///
    /// Workflow:
    ///   1. `rule test <ID> --input '{"temperature": 32}'` — should trigger
    ///   2. `rule test <ID> --input '{"temperature": 20}'` — should not trigger
    ///   3. `rule enable <ID>` — activate if tests pass
    ///
    /// Example: `neomind rule test rule-001 --input '{"temperature": 32}'`
    Test {
        /// Rule ID.
        #[arg(required = true)]
        id: String,
        /// Input data JSON.
        #[arg(short, long)]
        input: String,
    },
    /// Get rule execution history.
    ///
    /// Shows recent rule evaluations with timestamps, input data, and results.
    /// Useful for debugging why a rule did or did not trigger.
    ///
    /// Example: `neomind rule history rule-001`
    History {
        /// Rule ID.
        #[arg(required = true)]
        id: String,
    },
}

/// Transform subcommands.
#[derive(Subcommand, Debug)]
enum TransformCommand {
    /// List all transforms.
    ///
    /// Shows transform ID, name, scope, and enabled status.
    /// Use this to find transform IDs for get/update/delete commands.
    ///
    /// Example: `neomind transform list`
    List,
    /// Get transform details.
    ///
    /// Shows the full JavaScript code, input/output mapping, and execution stats.
    /// Use this to inspect code before modifying with `transform update`.
    ///
    /// Example: `neomind transform get transform-001`
    Get {
        /// Transform ID.
        #[arg(required = true)]
        id: String,
    },
    /// Create a new transform.
    ///
    /// Code receives `input` object with metric data, must return transformed value.
    /// Example: --code 'return input * 1.8 + 32' (Celsius to Fahrenheit).
    Create {
        /// Transform name.
        #[arg(short, long)]
        name: String,
        /// Scope: "global" (all devices) or "device_type:TypeName" or "device:DeviceId".
        /// Most transforms use "global".
        #[arg(short, long)]
        scope: String,
        /// JavaScript transform code. Use `input` to access the value.
        /// Example: 'return input * 1.8 + 32'
        #[arg(short, long)]
        code: String,
        /// Output prefix for virtual metrics.
        #[arg(short, long)]
        output_prefix: Option<String>,
        /// Description.
        #[arg(short, long)]
        description: Option<String>,
        /// Whether to enable immediately.
        #[arg(long)]
        enabled: Option<bool>,
    },
    /// Update transform.
    ///
    /// Modify transform code, scope, or enabled status. Changes take effect immediately.
    /// Test new code first with `transform test-code` before applying.
    ///
    /// Example: `neomind transform update transform-001 --code 'return input * 2' --enabled true`
    Update {
        /// Transform ID.
        #[arg(required = true)]
        id: String,
        /// New name.
        #[arg(short, long)]
        name: Option<String>,
        /// New description.
        #[arg(short, long)]
        description: Option<String>,
        /// New JavaScript code.
        #[arg(short, long)]
        code: Option<String>,
        /// New scope.
        #[arg(short, long)]
        scope: Option<String>,
        /// New output prefix.
        #[arg(short, long)]
        output_prefix: Option<String>,
        /// Enable/disable.
        #[arg(long)]
        enabled: Option<bool>,
    },
    /// Delete transform.
    ///
    /// Removes the transform and its virtual metrics permanently. This is irreversible.
    /// Dashboards using the virtual metrics will show errors.
    ///
    /// Example: `neomind transform delete transform-001`
    Delete {
        /// Transform ID.
        #[arg(required = true)]
        id: String,
    },
    /// List virtual metrics from transforms.
    ///
    /// Shows all metrics produced by transforms with their data source mappings.
    /// Use this to discover available data source IDs for dashboards or rules.
    ///
    /// Example: `neomind transform metrics`
    Metrics,
    /// Test transform code.
    ///
    /// Evaluates JavaScript code against sample input without creating a transform.
    /// Always test before creating: write code, test, then `transform create`.
    ///
    /// Workflow:
    ///   1. `transform test-code --code 'return input * 1.8 + 32' --input '{"value": 100}'`
    ///   2. Verify output is correct
    ///   3. `transform create --name "Celsius to Fahrenheit" --scope global --code 'return input * 1.8 + 32'`
    ///
    /// Example: `neomind transform test-code --code 'return input * 1.8 + 32' --input '{"value": 100}'`
    TestCode {
        /// Transform code (JavaScript).
        #[arg(short, long)]
        code: String,
        /// Input data JSON.
        #[arg(short, long)]
        input: String,
    },
    /// List transform data sources.
    ///
    /// Shows available data source IDs and their types (device, extension, etc.).
    /// Example: `neomind transform data-sources`
    DataSources,
}

/// Agent subcommands.
#[derive(Subcommand, Debug)]
enum AgentCommand {
    /// List all agents.
    ///
    /// Shows agent ID, name, status, schedule type, and last execution time.
    /// Use this to find agent IDs for all other agent commands.
    ///
    /// Example: `neomind agent list`
    List,
    /// Get agent details.
    ///
    /// Shows full agent config: prompt, schedule, LLM backend, and resources.
    /// Use this to inspect an agent before updating or debugging.
    ///
    /// Example: `neomind agent get agent-001`
    Get {
        /// Agent ID.
        #[arg(required = true)]
        id: String,
    },
    /// Create a new agent.
    ///
    /// Creates an agent in paused state. You MUST activate it with
    /// `agent control <ID> active` after creation.
    ///
    /// Schedule is required. Use --schedule-type and --schedule-config together:
    ///   - interval: `--schedule-type interval --schedule-config "300"` (every 5 min)
    ///   - cron:     `--schedule-type cron --schedule-config "0 8 * * *"` (daily 8am)
    ///   - event:    `--schedule-type event` (triggered by device data)
    ///
    /// Workflow:
    ///   1. `agent create --name "Monitor" --prompt "Check sensors" --schedule-type interval --schedule-config "300"`
    ///   2. `agent control <ID> active` — start the agent
    ///   3. `agent latest-execution <ID>` — check results
    Create {
        /// Agent name (1-100 chars).
        #[arg(short, long)]
        name: String,
        /// What the agent should do (natural language, 1-10000 chars). Required.
        #[arg(short, long)]
        prompt: String,
        /// Description (0-500 chars).
        #[arg(short, long)]
        description: Option<String>,
        /// Schedule type: interval | cron | event. Required.
        ///   - interval: runs every N seconds (--schedule-config "300" = every 5min)
        ///   - cron:     runs on schedule (--schedule-config "0 8 * * *" = daily 8am)
        ///   - event:    runs when device data arrives (--event-filter optional)
        #[arg(long)]
        schedule_type: Option<String>,
        /// Schedule config: interval seconds or cron expression.
        /// Example: "300" (5min interval) or "0 8 * * *" (daily 8am).
        #[arg(long)]
        schedule_config: Option<String>,
        /// Shortcut: --every "5m" = interval/300, "1h" = interval/3600, "30s" = interval/30
        /// Overrides --schedule-type and --schedule-config.
        #[arg(long)]
        every: Option<String>,
        /// Event filter for event schedule type (e.g., "device_type:temp_sensor").
        #[arg(long)]
        event_filter: Option<String>,
        /// Timezone for cron schedule (e.g., "Asia/Shanghai", "UTC"). Default: system timezone.
        #[arg(long)]
        timezone: Option<String>,
        /// LLM backend ID. Run `neomind system info` to see available backends.
        #[arg(short, long)]
        llm_backend: Option<String>,
        /// System prompt for the LLM (overrides default agent system prompt).
        #[arg(short, long)]
        system_prompt: Option<String>,
        /// Execution mode: "free" (multi-round tool calling) or "focused" (single-pass with bound resources).
        /// "focused" requires --resources or --device-ids. Default: "free".
        #[arg(long)]
        execution_mode: Option<String>,
        /// Device IDs to bind (comma-separated). Used in focused mode.
        /// Example: --device-ids "device-001,device-002"
        #[arg(long)]
        device_ids: Option<String>,
        /// Resources JSON array (unified format). Each: {"resource_id":"...","resource_type":"device|extension","name":"..."}
        /// Prefer this over --device-ids for new agents.
        /// Example: --resources '[{"resource_id":"device-001","resource_type":"device","name":"Temp Sensor"}]'
        #[arg(long)]
        resources: Option<String>,
        /// Metrics to bind (JSON array). Each: {"device_id":"...","metric_name":"...","display_name":"..."}
        /// Example: --metrics '[{"device_id":"sensor-001","metric_name":"temperature","display_name":"Temperature"}]'
        #[arg(long)]
        metrics: Option<String>,
        /// Commands to bind (JSON array). Each: {"device_id":"...","command_name":"...","display_name":"...","parameters":{}}
        /// Example: --commands '[{"device_id":"switch-001","command_name":"toggle","display_name":"Toggle","parameters":{}}]'
        #[arg(long)]
        commands: Option<String>,
        /// Enable tool chaining (agent can call multiple tools in sequence). Default: false.
        #[arg(long)]
        enable_tool_chaining: Option<bool>,
        /// Maximum tool chain depth (1-20). Only used when --enable-tool-chaining is true. Default: 3.
        #[arg(long)]
        max_chain_depth: Option<usize>,
        /// Agent priority (0-255, higher = more important). Default: 128.
        #[arg(long)]
        priority: Option<u8>,
        /// Context window size (number of conversation turns to keep). Default: 10.
        #[arg(long)]
        context_window_size: Option<usize>,
    },
    /// Update agent.
    ///
    /// Modify agent configuration. Changes apply to the next scheduled execution.
    /// Check current config first with `agent get <ID>`.
    ///
    /// Example: `neomind agent update agent-001 --prompt "Monitor temperature and alert if above 30"`
    Update {
        /// Agent ID.
        #[arg(required = true)]
        id: String,
        /// New name.
        #[arg(short, long)]
        name: Option<String>,
        /// New user prompt.
        #[arg(short, long)]
        prompt: Option<String>,
        /// New description.
        #[arg(short, long)]
        description: Option<String>,
        /// LLM backend ID.
        #[arg(short, long)]
        llm_backend: Option<String>,
        /// System prompt.
        #[arg(short, long)]
        system_prompt: Option<String>,
        /// New schedule type: interval | cron | event.
        #[arg(long)]
        schedule_type: Option<String>,
        /// New schedule config (seconds for interval, cron expression for cron).
        #[arg(long)]
        schedule_config: Option<String>,
        /// New execution mode: "free" or "focused".
        #[arg(long)]
        execution_mode: Option<String>,
        /// New device IDs (comma-separated). Replaces existing bindings.
        #[arg(long)]
        device_ids: Option<String>,
        /// New resources JSON array. Replaces existing resources.
        #[arg(long)]
        resources: Option<String>,
        /// Metrics to bind (JSON array). Each: {"device_id":"...","metric_name":"...","display_name":"..."}
        /// Example: --metrics '[{"device_id":"sensor-001","metric_name":"temperature","display_name":"Temperature"}]'
        #[arg(long)]
        metrics: Option<String>,
        /// Commands to bind (JSON array). Each: {"device_id":"...","command_name":"...","display_name":"...","parameters":{}}
        /// Example: --commands '[{"device_id":"switch-001","command_name":"toggle","display_name":"Toggle","parameters":{}}]'
        #[arg(long)]
        commands: Option<String>,
        /// Enable/disable tool chaining.
        #[arg(long)]
        enable_tool_chaining: Option<bool>,
        /// Max tool chain depth.
        #[arg(long)]
        max_chain_depth: Option<usize>,
        /// Agent priority (0-255).
        #[arg(long)]
        priority: Option<u8>,
        /// Context window size.
        #[arg(long)]
        context_window_size: Option<usize>,
    },
    /// Delete agent.
    ///
    /// Removes the agent and ALL execution history. This is irreversible.
    /// Consider pausing first with `agent control <ID> paused` if unsure.
    ///
    /// Example: `neomind agent delete agent-001`
    Delete {
        /// Agent ID.
        #[arg(required = true)]
        id: String,
    },
    /// Control agent status.
    ///
    /// Status values: "active" (start running) or "paused" (stop).
    /// New agents are created in paused state — you MUST run this to start them.
    ///
    /// Workflow:
    ///   - Start: `agent control <ID> active`
    ///   - Stop:  `agent control <ID> paused`
    ///   - Verify: `agent list` or `agent get <ID>`
    Control {
        /// Agent ID.
        #[arg(required = true)]
        id: String,
        /// Status (active or paused).
        #[arg(required = true)]
        status: String,
    },
    /// Invoke agent with input.
    ///
    /// Runs a one-time agent execution with custom input, bypassing the schedule.
    /// Returns the agent's response directly. Useful for testing or ad-hoc queries.
    ///
    /// Workflow: `agent invoke <ID> "your question"` → see immediate response.
    ///
    /// Example: `neomind agent invoke agent-001 "Check all sensor readings"`
    Invoke {
        /// Agent ID.
        #[arg(required = true)]
        id: String,
        /// Input prompt.
        #[arg(required = true)]
        input: String,
    },
    /// Get agent memory.
    ///
    /// Shows extracted facts and context stored by the agent across executions.
    /// Memory helps the agent maintain context between runs.
    /// Check this if the agent seems to forget important information.
    ///
    /// Example: `neomind agent memory agent-001`
    Memory {
        /// Agent ID.
        #[arg(required = true)]
        id: String,
    },
    /// Get agent execution history.
    ///
    /// Shows past execution records with timestamps, status, and duration.
    /// Use --limit and --offset for pagination.
    /// For just the latest result, use `agent latest-execution` instead.
    ///
    /// Example: `neomind agent executions agent-001 --limit 10`
    Executions {
        /// Agent ID.
        #[arg(required = true)]
        id: String,
        /// Limit number of results.
        #[arg(short, long)]
        limit: Option<usize>,
        /// Offset for pagination.
        #[arg(short, long)]
        offset: Option<usize>,
    },
    /// Get latest agent execution.
    ///
    /// Shows the most recent execution result including tool calls and response.
    /// Quick way to check if the agent is working correctly.
    ///
    /// Example: `neomind agent latest-execution agent-001`
    LatestExecution {
        /// Agent ID.
        #[arg(required = true)]
        id: String,
    },
    /// Get agent conversation (messages).
    ///
    /// Shows the full conversation thread between the agent and LLM.
    /// Use --limit to control the number of messages returned.
    /// Useful for debugging: see exactly what the agent sent and received.
    ///
    /// Example: `neomind agent conversation agent-001 --limit 20`
    Conversation {
        /// Agent ID.
        #[arg(required = true)]
        id: String,
        /// Limit number of messages.
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Send message to agent.
    ///
    /// Sends an inline message to an active agent. The agent processes it
    /// in its next execution cycle. Use --message-type to categorize
    /// (instruction, correction, etc.).
    ///
    /// Workflow:
    ///   - Correction: `agent send-message <ID> "The threshold should be 35 not 30" --message-type correction`
    ///   - Instruction: `agent send-message <ID> "Focus on temperature sensors only" --message-type instruction`
    ///
    /// Example: `neomind agent send-message agent-001 "Increase alert threshold to 35"`
    SendMessage {
        /// Agent ID.
        #[arg(required = true)]
        id: String,
        /// Message content.
        #[arg(required = true)]
        message: String,
        /// Message type (e.g., instruction, correction).
        #[arg(long)]
        message_type: Option<String>,
    },
}

/// Message subcommands.
#[derive(Subcommand, Debug)]
enum MessageCommand {
    /// List messages.
    ///
    /// Shows system notifications, alerts, and user messages.
    /// Use --severity or --status to filter. Default limit is 20.
    ///
    /// Workflow: `message list` → `message get <ID>` → `message read <ID>` (acknowledge).
    ///
    /// Example: `neomind message list --severity error --limit 50`
    List {
        /// Limit number of results.
        #[arg(short, long)]
        limit: Option<usize>,
        /// Offset for pagination.
        #[arg(short, long)]
        offset: Option<usize>,
        /// Filter by severity.
        #[arg(long)]
        severity: Option<String>,
        /// Filter by status.
        #[arg(long)]
        status: Option<String>,
    },
    /// Get message details.
    ///
    /// Shows full message content, metadata, and acknowledgment status.
    /// Use this to read the full body of truncated messages from `message list`.
    ///
    /// Example: `neomind message get msg-001`
    Get {
        /// Message ID.
        #[arg(required = true)]
        id: String,
    },
    /// Send a new message.
    ///
    /// Creates a system message with severity level. Supports markdown content.
    /// Severity levels: info, warning, error, critical.
    ///
    /// Workflow: `message send --title "Alert" --message "Check sensor #3" --severity warning`
    /// Messages appear in the UI notification center and can trigger rules.
    ///
    /// Example: `neomind message send --title "Deploy Notice" --message "Version 2.0 deployed" --severity info`
    Send {
        /// Message title.
        #[arg(short, long)]
        title: String,
        /// Message content (supports markdown).
        #[arg(long)]
        message: String,
        /// Severity level: info | warning | error | critical.
        #[arg(short, long, default_value = "info")]
        severity: String,
        /// Source.
        #[arg(long)]
        source: Option<String>,
    },
    /// Acknowledge/read a message.
    ///
    /// Marks a message as read. Used to clear unread notifications.
    /// Batch acknowledge: list IDs from `message list`, then read each one.
    ///
    /// Example: `neomind message read msg-001`
    Read {
        /// Message ID.
        #[arg(required = true)]
        id: String,
    },
    /// List message channels.
    ///
    /// Shows all notification channels (webhook, email, etc.) and their status.
    /// Example: `neomind message channel-list`
    ChannelList,
    /// Get channel details.
    ///
    /// Shows channel type, config, and delivery status.
    /// Example: `neomind message channel-get slack-alerts`
    ChannelGet {
        /// Channel name.
        #[arg(required = true)]
        name: String,
    },
    /// List available channel types.
    ///
    /// Shows channel types that can be created (webhook, email, telegram, wecom, dingtalk, slack, feishu).
    /// Example: `neomind message channel-types`
    ChannelTypes,
    /// Get config schema and examples for a channel type.
    ///
    /// Shows required/optional fields and full config JSON examples.
    /// Run before `channel-create` to know what --config needs.
    /// Example: `neomind message channel-type-schema telegram`
    ChannelTypeSchema {
        /// Channel type (webhook, email, telegram, wecom, dingtalk, slack, feishu).
        #[arg(required = true)]
        channel_type: String,
    },
    /// Create a message channel.
    ///
    /// Workflow:
    ///   1. `message channel-types` — see available types
    ///   2. `message channel-type-schema <TYPE>` — get config fields & examples
    ///   3. `message channel-create --name <N> --type <TYPE> --config '<JSON>'`
    ///   4. `message channel-test <NAME>` — verify it works
    ///
    /// Config examples by type:
    ///   webhook:  '{"url":"https://example.com/webhook","headers":{"Authorization":"Bearer TOKEN"},"timeout_secs":30}'
    ///   email:    '{"smtp_server":"smtp.example.com","smtp_port":587,"username":"user","password":"pass","from_address":"noreply@example.com","use_tls":true}'
    ///   telegram: '{"token":"123456:ABCdefGHIjklMNO","chat_id":"-1001234567890"}'
    ///   wecom:    '{"key":"xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"}'
    ///   dingtalk: '{"access_token":"xxxx","secret":"SECxxxx"}'
    ///   slack:    '{"webhook_url":"https://hooks.slack.com/services/T00/B00/xxx"}'
    ///   feishu:   '{"hook_id":"xxxxxxxx","secret":"optional_sign_secret"}'
    ///
    /// Example: `neomind message channel-create --name "alerts" --type webhook --config '{"url":"https://hooks.slack.com/..."}'`
    ChannelCreate {
        /// Channel name (unique identifier).
        #[arg(long)]
        name: String,
        /// Channel type. Run `channel-types` to see available types.
        #[arg(long, visible_alias = "type")]
        channel_type: String,
        /// Channel config as JSON. Run `channel-type-schema <TYPE>` for field details.
        #[arg(long)]
        config: String,
    },
    /// Update channel configuration.
    ///
    /// Updates the channel's configuration. Does not change the channel type.
    /// Example: `neomind message channel-update --name "alerts" --config '{"url":"https://new-url.com"}'`
    ChannelUpdate {
        /// Channel name.
        #[arg(long)]
        name: String,
        /// New config (JSON string).
        #[arg(long)]
        config: String,
    },
    /// Delete a message channel.
    ///
    /// Removes the channel. Pending messages will not be delivered.
    /// Example: `neomind message channel-delete slack-alerts`
    ChannelDelete {
        /// Channel name.
        #[arg(required = true)]
        name: String,
    },
    /// Test a message channel.
    ///
    /// Sends a test message through the channel to verify configuration.
    /// Example: `neomind message channel-test slack-alerts`
    ChannelTest {
        /// Channel name.
        #[arg(required = true)]
        name: String,
    },
}

/// Push subcommands.
#[derive(Subcommand, Debug)]
enum PushCommand {
    /// List push targets.
    ///
    /// Shows all data push targets and their status.
    /// Example: `neomind push list`
    List,
    /// Get push target details.
    ///
    /// Shows target config, schedule, delivery stats.
    /// Example: `neomind push get <ID>`
    Get {
        /// Target ID.
        #[arg(required = true)]
        id: String,
    },
    /// Create a push target.
    ///
    /// Forwards device metrics / extension outputs to an external system.
    ///
    /// Target types & config:
    ///   webhook: '{"url":"https://example.com/api","headers":{"Authorization":"Bearer TOKEN"}}'
    ///   mqtt:    '{"broker":"tcp://broker:1883","topic":"neomind/data","username":"user","password":"pass"}'
    ///
    /// Example: `neomind push create --name my-webhook --type webhook --config '{"url":"https://httpbin.org/post"}'`
    Create {
        /// Target name (unique identifier).
        #[arg(long)]
        name: String,
        /// Target type (webhook, mqtt).
        #[arg(long, visible_alias = "target-type")]
        target_type: Option<String>,
        /// Target config as JSON.
        #[arg(long)]
        config: Option<String>,
        /// Schedule type: event (real-time) or interval (every 60s). Default: event.
        #[arg(long)]
        schedule: Option<String>,
        /// Comma-separated source patterns to filter (e.g., "device:sensor-001:temperature").
        #[arg(long)]
        sources: Option<String>,
    },
    /// Update a push target.
    ///
    /// Updates target name, config, or enabled status.
    /// Example: `neomind push update <ID> --config '{"url":"https://new-url.com"}'`
    Update {
        /// Target ID.
        #[arg(required = true)]
        id: String,
        /// New name.
        #[arg(long)]
        name: Option<String>,
        /// New config as JSON.
        #[arg(long)]
        config: Option<String>,
        /// Enable or disable.
        #[arg(long)]
        enabled: Option<bool>,
    },
    /// Delete a push target.
    ///
    /// Removes the target and its delivery history.
    /// Example: `neomind push delete <ID>`
    Delete {
        /// Target ID.
        #[arg(required = true)]
        id: String,
    },
    /// Start a push target.
    ///
    /// Enables real-time or scheduled data forwarding.
    /// Example: `neomind push start <ID>`
    Start {
        /// Target ID.
        #[arg(required = true)]
        id: String,
    },
    /// Stop a push target.
    ///
    /// Pauses data forwarding without deleting the target.
    /// Example: `neomind push stop <ID>`
    Stop {
        /// Target ID.
        #[arg(required = true)]
        id: String,
    },
    /// Test a push target.
    ///
    /// Sends a test payload to verify the target works.
    /// Example: `neomind push test <ID>`
    Test {
        /// Target ID.
        #[arg(required = true)]
        id: String,
    },
    /// Show delivery logs for a target.
    ///
    /// Example: `neomind push logs <ID> --limit 20`
    Logs {
        /// Target ID.
        #[arg(required = true)]
        id: String,
        /// Max log entries to return.
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Show push statistics.
    ///
    /// Displays aggregated delivery stats across all targets.
    /// Example: `neomind push stats`
    Stats,
}

/// Widget subcommands.
#[derive(Subcommand, Debug)]
enum WidgetCommand {
    /// List installed widgets.
    ///
    /// Shows widget ID, name, type, and version.
    /// Example: `neomind widget list --json`
    List {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Get widget details.
    ///
    /// Shows widget manifest, config schema, and supported data sources.
    /// Example: `neomind widget get line-chart`
    Get {
        /// Widget ID.
        #[arg(required = true)]
        id: String,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Get widget bundle.
    ///
    /// Returns the compiled JavaScript bundle for a widget.
    /// Example: `neomind widget bundle line-chart`
    Bundle {
        /// Widget ID.
        #[arg(required = true)]
        id: String,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Scaffold a new widget component (generates manifest.json + bundle.js).
    ///
    /// Creates a widget project with all required files.
    /// Widget types: chart, gauge, stat, table, image, custom.
    /// Example: `neomind widget create "My Chart" --widget-type chart -o ./my-widgets`
    Create {
        /// Widget display name.
        #[arg(required = true)]
        name: String,
        /// Widget type: chart, gauge, stat, table, image, custom.
        #[arg(long, default_value = "custom")]
        widget_type: String,
        /// Output directory (defaults to widget ID).
        #[arg(long)]
        output: Option<String>,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Install widget from file.
    ///
    /// Installs a widget from a .tgz package file.
    /// Example: `neomind widget install ./my-chart.tgz`
    Install {
        /// Path to widget file (.tgz).
        #[arg(required = true)]
        file: String,
    },
    /// Uninstall widget.
    ///
    /// Removes the widget. Dashboards using this widget will show an error.
    /// Example: `neomind widget uninstall my-chart`
    Uninstall {
        /// Widget ID.
        #[arg(required = true)]
        id: String,
    },
    /// List marketplace widgets.
    ///
    /// Shows all widgets available in the marketplace registry.
    /// Example: `neomind widget market-list`
    MarketList {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Install widget from marketplace.
    ///
    /// Downloads and installs a widget from the marketplace registry.
    /// Example: `neomind widget market-install line-chart --version 1.2.0`
    MarketInstall {
        /// Widget ID.
        #[arg(required = true)]
        id: String,
        /// Version (optional).
        #[arg(long)]
        version: Option<String>,
    },
}

/// System information subcommands.
#[derive(Subcommand, Debug)]
enum SystemCommand {
    /// Show system infrastructure info (MQTT broker, webhook URL, network).
    Info {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Data connector subcommands (MQTT, webhook, HTTP, etc.).
#[derive(Subcommand, Debug)]
enum ConnectorCommand {
    /// List all data connectors.
    ///
    /// Shows connector ID, name, host, port, type, and connection status.
    /// Example: `neomind connector list --json`
    List {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Get connector details and connection status.
    ///
    /// Shows connection state, subscriptions, and message statistics.
    /// Example: `neomind connector get connector-001`
    Get {
        /// Connector ID.
        #[arg(required = true)]
        id: String,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Create a new data connector.
    ///
    /// Registers an external data source for data ingestion.
    /// Currently supports MQTT connectors; more types coming soon.
    ///
    /// Workflow:
    ///   1. `connector create --name "Factory MQTT" --host 192.168.1.100 --port 1883`
    ///   2. `connector test <ID>` — verify connectivity
    ///   3. Devices sending to the connector's topics will auto-appear in NeoMind
    ///
    ///   Example: `neomind connector create --type mqtt --name "Factory MQTT" --host 192.168.1.100 --port 1883 --topics "sensor/#,device/#"`
    Create {
        /// Connector type (mqtt, webhook, http). Default: mqtt.
        #[arg(long, default_value = "mqtt")]
        connector_type: String,
        /// Connector display name.
        #[arg(long)]
        name: String,
        /// Connector hostname or IP.
        #[arg(long)]
        host: String,
        /// Connector port (default: 1883).
        #[arg(long, default_value_t = 1883)]
        port: u16,
        /// Enable TLS.
        #[arg(long)]
        tls: bool,
        /// Username for authentication.
        #[arg(long)]
        username: Option<String>,
        /// Password for authentication.
        #[arg(long)]
        password: Option<String>,
        /// Comma-separated topic subscriptions (default: # for all).
        #[arg(long)]
        topics: Option<String>,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Update connector configuration.
    ///
    /// Modify connector settings. Use --disable to stop, omit to keep current values.
    /// Example: `neomind connector update connector-001 --topics "sensor/+/data" --password "newpass"`
    Update {
        /// Connector ID.
        #[arg(required = true)]
        id: String,
        /// Connector display name.
        #[arg(long)]
        name: Option<String>,
        /// Connector hostname or IP.
        #[arg(long)]
        host: Option<String>,
        /// Connector port.
        #[arg(long)]
        port: Option<u16>,
        /// Enable TLS.
        #[arg(long)]
        tls: bool,
        /// Username for authentication.
        #[arg(long)]
        username: Option<String>,
        /// Password for authentication.
        #[arg(long)]
        password: Option<String>,
        /// Comma-separated topic subscriptions.
        #[arg(long)]
        topics: Option<String>,
        /// Disable the connector (stop connection).
        #[arg(long)]
        disable: bool,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Delete a connector.
    ///
    /// Removes the connector and disconnects. All subscriptions are lost.
    /// Example: `neomind connector delete connector-001`
    Delete {
        /// Connector ID.
        #[arg(required = true)]
        id: String,
    },
    /// Test connector connectivity with real MQTT handshake.
    ///
    /// Attempts to connect, subscribe, and publish a test message.
    /// Run this after creating or updating a connector to verify settings.
    /// If it fails, check host, port, credentials, and firewall rules.
    /// Example: `neomind connector test connector-001`
    Test {
        /// Connector ID.
        #[arg(required = true)]
        id: String,
    },
    /// List all MQTT topic subscriptions.
    ///
    /// Shows all active topic subscriptions across all connectors.
    /// Example: `neomind connector subscriptions`
    Subscriptions {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Subscribe to a custom MQTT topic.
    ///
    /// Adds a new topic subscription to the embedded broker.
    /// Supports MQTT wildcards: + (single level) and # (multi level).
    /// Example: `neomind connector subscribe --topic "factory/+/temperature" --qos 1`
    Subscribe {
        /// Topic pattern to subscribe to.
        #[arg(long)]
        topic: String,
        /// QoS level (0, 1, or 2). Default: 1.
        #[arg(long, default_value_t = 1)]
        qos: u8,
    },
    /// Unsubscribe from an MQTT topic.
    ///
    /// Removes a topic subscription from the embedded broker.
    /// Example: `neomind connector unsubscribe --topic "factory/+/temperature"`
    Unsubscribe {
        /// Topic to unsubscribe from.
        #[arg(long)]
        topic: String,
    },
}

/// Hidden backward-compatible alias — delegates to ConnectorCommand.
#[derive(Subcommand, Debug)]
#[command(hide = true)]
enum BrokerAliasCommand {
    #[command(flatten)]
    Connector(ConnectorCommand),
}

/// Parse human duration like "30s", "5m", "1h", "2d" to seconds
fn parse_duration(s: &str) -> u64 {
    let s = s.trim();
    if let Some(num) = s.strip_suffix('s') {
        num.parse::<u64>().unwrap_or(30)
    } else if let Some(num) = s.strip_suffix('m') {
        num.parse::<u64>().unwrap_or(1) * 60
    } else if let Some(num) = s.strip_suffix('h') {
        num.parse::<u64>().unwrap_or(1) * 3600
    } else if let Some(num) = s.strip_suffix('d') {
        num.parse::<u64>().unwrap_or(1) * 86400
    } else {
        s.parse::<u64>().unwrap_or(300)
    }
}

// Custom runtime with increased worker threads for better concurrent performance
// Default is num_cpus, but we use more to handle block_in_place alternatives
#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() -> Result<()> {
    // Install extension panic hook to catch and log extension panics
    // This provides better error messages and prevents some panics from crashing the server
    // IMPORTANT: This cannot catch all panic types (e.g., foreign exceptions from C/C++ code)
    neomind_core::extension::safety::install_extension_panic_hook();

    let args = Args::parse();

    // Initialize logging
    let json_logging = std::env::var("NEOMIND_LOG_JSON")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(false);

    // Build the env filter for log level control
    // -v/--verbose sets debug level; RUST_LOG env var takes precedence
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if args.verbose {
            tracing_subscriber::EnvFilter::new("neomind=debug")
                .add_directive(tracing::Level::DEBUG.into())
        } else {
            tracing_subscriber::EnvFilter::new("neomind=info")
                .add_directive(tracing::Level::INFO.into())
                .add_directive(tracing::Level::WARN.into())
        }
    });

    // For serve command: dual output (stdout + file); for others: stdout only
    let file_logging = matches!(args.command, Command::Serve { .. });

    if file_logging {
        let log_dir = Path::new("data/logs");
        let file_appender = tracing_appender::rolling::daily(log_dir, "neomind.log");

        let stdout_layer = if json_logging {
            tracing_subscriber::fmt::layer()
                .json()
                .with_target(true)
                .with_filter(env_filter.clone())
                .boxed()
        } else {
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .compact()
                .with_level(false)
                .with_filter(env_filter.clone())
                .boxed()
        };

        let file_layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_writer(file_appender)
            .with_filter(env_filter);

        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(file_layer)
            .init();
    } else if json_logging {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .with_target(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .compact()
            .with_level(false)
            .init();
    }

    // Run the appropriate command
    match args.command {
        Command::Serve { host, port } => run_server(host, port).await,
        Command::Prompt {
            prompt,
            max_tokens: _,
            temperature: _,
        } => run_prompt(&prompt).await,
        Command::Chat { session } => run_chat(session).await,
        Command::ListModels { endpoint } => list_models(endpoint).await,
        Command::Health => run_health().await,
        Command::Logs {
            tail,
            follow,
            level,
            since,
        } => run_logs(tail, follow, level, since).await,
        Command::Extension { extension_cmd } => run_extension_cmd(extension_cmd).await,
        Command::CheckUpdate => run_check_update().await,
        Command::ApiKey { key_cmd } => run_api_key_cmd(key_cmd).await,
        Command::Llm { llm_cmd } => run_llm_cmd(llm_cmd).await,
        Command::Device { device_cmd } => run_device_cmd(device_cmd).await,
        Command::Dashboard { dashboard_cmd } => run_dashboard_cmd(dashboard_cmd).await,
        Command::Rule { rule_cmd } => run_rule_cmd(rule_cmd).await,
        Command::Transform { transform_cmd } => run_transform_cmd(transform_cmd).await,
        Command::Agent { agent_cmd } => run_agent_cmd(agent_cmd).await,
        Command::Message { message_cmd } => run_message_cmd(message_cmd).await,
        Command::Push { push_cmd } => run_push_cmd(push_cmd).await,
        Command::Widget { widget_cmd } => run_widget_cmd(widget_cmd).await,
        Command::System { system_cmd } => run_system_cmd(system_cmd).await,
        Command::Connector { connector_cmd } => run_connector_cmd(connector_cmd).await,
        Command::Broker { broker_cmd } => {
            // Hidden backward-compatible alias: delegate to connector handler
            match broker_cmd {
                BrokerAliasCommand::Connector(cmd) => run_connector_cmd(cmd).await,
            }
        }
    }
}

/// Initialize LLM backend from available config sources.
async fn init_llm_backend(session_manager: &SessionManager) -> Result<()> {
    // Try config.toml, then llm_config.json, then environment variables
    let backend = load_llm_backend_from_env()?;
    // Only set as default for new sessions
    session_manager.set_default_llm_backend(backend).await;
    Ok(())
}

/// Load LLM backend from environment variables.
fn load_llm_backend_from_env() -> Result<LlmBackend> {
    // Check for Ollama
    if let Ok(endpoint) = std::env::var(env_vars::OLLAMA_ENDPOINT) {
        let endpoint = normalize_ollama_endpoint(endpoint);
        let model = std::env::var(env_vars::LLM_MODEL)
            .unwrap_or_else(|_| models::OLLAMA_DEFAULT.to_string());
        eprintln!("Using Ollama: endpoint={}, model={}", endpoint, model);
        return Ok(LlmBackend::Ollama {
            endpoint,
            model,
            capabilities: None,
        });
    }

    // Check for OpenAI
    if let Ok(api_key) = std::env::var(env_vars::OPENAI_API_KEY) {
        let endpoint = std::env::var(env_vars::OPENAI_ENDPOINT)
            .unwrap_or_else(|_| endpoints::OPENAI.to_string());
        let endpoint = normalize_openai_endpoint(endpoint);
        let model = std::env::var(env_vars::LLM_MODEL)
            .unwrap_or_else(|_| models::OPENAI_DEFAULT.to_string());
        eprintln!("Using OpenAI: endpoint={}, model={}", endpoint, model);
        return Ok(LlmBackend::OpenAi {
            api_key,
            endpoint,
            model,
            capabilities: None,
        });
    }

    Err(anyhow::anyhow!(
        "No LLM backend configured. Set OLLAMA_ENDPOINT or OPENAI_API_KEY environment variable."
    ))
}

/// Run a single prompt.
async fn run_prompt(prompt: &str) -> Result<()> {
    println!("NeoMind Edge AI - Prompt Mode");
    println!("==============================\n");
    println!("Prompt: {}", prompt);
    println!("\nGenerating response...\n");

    // Create session manager
    let session_manager = SessionManager::new()
        .map_err(|e| anyhow::anyhow!("Failed to create session manager: {}", e))?;

    // Initialize LLM backend
    init_llm_backend(&session_manager).await?;

    // Create a temporary session
    let session_id = session_manager
        .create_session()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create session: {}", e))?;

    // Process the prompt
    let response = session_manager
        .process_message(&session_id, prompt)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to process message: {}", e))?;

    println!("{}", response.message.content);
    println!("\nProcessing time: {}ms", response.processing_time_ms);

    // Clean up the temporary session
    let _ = session_manager.remove_session(&session_id).await;

    Ok(())
}

/// Run interactive chat mode.
async fn run_chat(session_id: Option<String>) -> Result<()> {
    println!("NeoMind Edge AI - Chat Mode");
    println!("===========================\n");

    // Create session manager
    let session_manager = SessionManager::new()
        .map_err(|e| anyhow::anyhow!("Failed to create session manager: {}", e))?;

    // Initialize LLM backend
    init_llm_backend(&session_manager).await?;

    // Use existing session or create new one
    let session_id = if let Some(sid) = session_id {
        println!("Resuming session: {}", sid);
        sid
    } else {
        let sid = session_manager
            .create_session()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create session: {}", e))?;
        println!("New session: {}", sid);
        sid
    };

    println!("\nType your message and press Enter to send.");
    println!("Type 'quit' or 'exit' to quit.");
    println!("Type 'clear' to clear conversation history.\n");

    let mut input = String::new();
    let stdin = std::io::stdin();

    loop {
        print!("> ");
        use std::io::Write;
        std::io::stdout().flush()?;

        input.clear();
        stdin.read_line(&mut input)?;

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "quit" || input == "exit" {
            println!("Goodbye!");
            break;
        }

        if input == "clear" {
            if session_manager.clear_history(&session_id).await.is_ok() {
                println!("Conversation history cleared.\n");
            }
            continue;
        }

        println!();

        // Process with streaming events
        match session_manager
            .process_message_events(&session_id, input)
            .await
        {
            Ok(mut stream) => {
                use futures::stream::StreamExt;
                let mut response_text = String::new();

                while let Some(event) = stream.next().await {
                    match event {
                        neomind_agent::AgentEvent::Thinking { content } => {
                            if !content.is_empty() {
                                print!("\x1b[90m[Thinking: {}...]\x1b[0m", content.trim());
                                use std::io::Write;
                                std::io::stdout().flush()?;
                            }
                        }
                        neomind_agent::AgentEvent::Content { content } => {
                            if !response_text.is_empty() && response_text != content {
                                // Print incremental content
                                let added =
                                    content.strip_prefix(&response_text).unwrap_or(&content);
                                print!("{}", added);
                                use std::io::Write;
                                std::io::stdout().flush()?;
                            }
                            response_text = content;
                        }
                        neomind_agent::AgentEvent::ToolCallStart { tool, .. } => {
                            println!("\n\n[Calling tool: {}]", tool);
                        }
                        neomind_agent::AgentEvent::ToolCallEnd { tool, success, .. } => {
                            if success {
                                println!("[Tool {} completed]\n", tool);
                            } else {
                                println!("[Tool {} failed]\n", tool);
                            }
                        }
                        neomind_agent::AgentEvent::Error { message } => {
                            eprintln!("\nError: {}", message);
                        }
                        neomind_agent::AgentEvent::Intent {
                            display_name,
                            confidence,
                            ..
                        } => {
                            println!(
                                "\n[Intent: {} (confidence: {:.0}%)]",
                                display_name,
                                confidence.unwrap_or(0.0) * 100.0
                            );
                        }
                        neomind_agent::AgentEvent::Plan { step, stage } => {
                            println!("[Plan: {} - {}]", stage, step);
                        }
                        neomind_agent::AgentEvent::ExecutionPlanCreated { plan, .. } => {
                            println!(
                                "[ExecutionPlan: {} steps ({:?})]",
                                plan.steps.len(),
                                plan.mode
                            );
                        }
                        neomind_agent::AgentEvent::PlanStepStarted {
                            step_id,
                            description,
                        } => {
                            println!("[PlanStep {} started: {}]", step_id, description);
                        }
                        neomind_agent::AgentEvent::PlanStepCompleted {
                            step_id,
                            success,
                            summary,
                        } => {
                            println!(
                                "[PlanStep {} {}: {}]",
                                step_id,
                                if success { "done" } else { "failed" },
                                summary
                            );
                        }
                        neomind_agent::AgentEvent::Progress { message, .. } => {
                            println!("[Progress: {}]", message);
                        }
                        neomind_agent::AgentEvent::Heartbeat { .. } => {
                            // Ignore heartbeat events in CLI
                        }
                        neomind_agent::AgentEvent::Warning { message } => {
                            eprintln!("[Warning] {}", message);
                        }
                        neomind_agent::AgentEvent::IntermediateEnd => {
                            // Intermediate end - more content coming, don't break
                            println!("[Continuing...]");
                        }
                        neomind_agent::AgentEvent::End { .. } => {
                            break;
                        }
                    }
                }

                println!(); // Final newline

                // Persist history
                let _ = session_manager.persist_history(&session_id).await;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}

/// List available models from Ollama.
async fn list_models(endpoint: String) -> Result<()> {
    println!("Available Models from Ollama:");
    println!("==============================\n");

    let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
    let client = reqwest::Client::new();

    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to Ollama: {}", e))?;

    if !response.status().is_success() {
        eprintln!("Ollama returned status: {}", response.status());
        eprintln!("\nMake sure Ollama is running at: {}", endpoint);
        return Ok(());
    }

    #[derive(serde::Deserialize)]
    struct OllamaModelsResponse {
        models: Vec<OllamaModel>,
    }

    #[derive(serde::Deserialize)]
    struct OllamaModel {
        name: String,
        size: Option<u64>,
    }

    let ollama_response: OllamaModelsResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

    if ollama_response.models.is_empty() {
        println!("No models found. Pull a model using: ollama pull <model>");
    } else {
        for model in &ollama_response.models {
            println!("  {}", model.name);
            if let Some(size) = model.size {
                let size_gb = size as f64 / (1024.0 * 1024.0 * 1024.0);
                println!("    Size: {:.2} GB", size_gb);
            }
        }
    }

    println!("\nTo use a model, set LLM_MODEL environment variable:");
    println!("  export LLM_MODEL=<model_name>");

    Ok(())
}

/// Check for updates by comparing current version with latest GitHub release.
async fn run_check_update() -> Result<()> {
    use neomind_core::brand::APP_VERSION;

    println!("NeoMind {}", APP_VERSION);
    println!("Checking for updates...\n");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(neomind_core::brand::user_agent())
        .build()?;

    let response = client
        .get("https://api.github.com/repos/camthink-ai/NeoMind/releases/latest")
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to check for updates: {}", e))?;

    if !response.status().is_success() {
        eprintln!("Failed to fetch release info (HTTP {})", response.status());
        return Ok(());
    }

    let release: serde_json::Value = response.json().await?;
    let latest_tag = release["tag_name"]
        .as_str()
        .unwrap_or("unknown")
        .trim_start_matches('v');

    let current = semver(APP_VERSION);
    let latest = semver(latest_tag);

    if current[0] < latest[0]
        || (current[0] == latest[0] && current[1] < latest[1])
        || (current[0] == latest[0] && current[1] == latest[1] && current[2] < latest[2])
    {
        println!("Update available: {} → v{}", APP_VERSION, latest_tag);

        if let Some(notes) = release["body"].as_str() {
            println!();
            // Show first 5 lines of release notes
            for line in notes.lines().take(5) {
                println!("  {}", line.trim());
            }
            let total_lines = notes.lines().count();
            if total_lines > 5 {
                println!("  ... ({} more lines)", total_lines - 5);
            }
        }

        if let Some(url) = release["html_url"].as_str() {
            println!("\nDownload: {}", url);
        }

        // Show relevant download assets
        if let Some(assets) = release["assets"].as_array() {
            let os_type = if cfg!(target_os = "macos") {
                "darwin"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else {
                "windows"
            };
            let arch_type = if cfg!(target_arch = "aarch64") {
                "arm64"
            } else {
                "amd64"
            };

            let server_asset = assets.iter().find(|a| {
                a["name"]
                    .as_str()
                    .map(|n| n.contains(&format!("server-{}", os_type)) && n.contains(arch_type))
                    .unwrap_or(false)
            });

            if let Some(asset) = server_asset {
                if let (Some(name), Some(size)) = (asset["name"].as_str(), asset["size"].as_u64()) {
                    let size_mb = size as f64 / (1024.0 * 1024.0);
                    println!(
                        "\nServer binary for your platform: {} ({:.1} MB)",
                        name, size_mb
                    );
                }
            }

            println!("\nUpdate command:");
            println!(
                "  curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | sh"
            );
        }
    } else {
        println!("Already up to date (v{})", APP_VERSION);
    }

    Ok(())
}

/// Parse a semver string like "0.6.6" into [u32, u32, u32].
fn semver(v: &str) -> [u32; 3] {
    let parts: Vec<u32> = v.split('.').filter_map(|s| s.parse().ok()).collect();
    match parts.as_slice() {
        [a, b, c] => [*a, *b, *c],
        [a, b] => [*a, *b, 0],
        [a] => [*a, 0, 0],
        _ => [0, 0, 0],
    }
}

/// Run the web server.
async fn run_server(host: String, port: u16) -> Result<()> {
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid address: {}:{}", host, port))?;

    // Startup cleanup of old log files
    cleanup_old_logs();

    // Spawn periodic log cleanup (every 24 hours)
    tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            cleanup_old_logs();
        }
    });

    neomind_api::run(addr).await
}

/// Clean up log files older than 7 days in data/logs/.
fn cleanup_old_logs() {
    use std::fs;

    let log_dir = Path::new("data/logs");
    if !log_dir.exists() {
        return;
    }

    let max_age_secs: i64 = 7 * 24 * 60 * 60; // 7 days
    let now = chrono::Utc::now();

    let mut removed = 0u32;
    let mut kept = 0u32;

    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let should_remove =
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if let Some(date_str) = filename.strip_prefix("neomind.log.") {
                        // Parse date from filename: neomind.log.YYYY-MM-DD
                        if let Ok(file_date) =
                            chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                        {
                            let file_datetime = file_date
                                .and_time(chrono::NaiveTime::default())
                                .and_utc();
                            (now - file_datetime).num_seconds() > max_age_secs
                        } else {
                            // Non-date suffix, fall back to mtime
                            is_file_older_than(&path, max_age_secs)
                        }
                    } else {
                        // Not a rotated log file (e.g., current neomind.log), skip
                        false
                    }
                } else {
                    false
                };

            if should_remove {
                if fs::remove_file(&path).is_ok() {
                    removed += 1;
                }
            } else {
                kept += 1;
            }
        }
    }

    if removed > 0 {
        tracing::info!(
            "Log cleanup: removed {} old file(s), kept {}",
            removed,
            kept
        );
    }
}

/// Check if a file is older than the given number of seconds (by mtime).
fn is_file_older_than(path: &Path, max_age_secs: i64) -> bool {
    let now = SystemTime::now();
    path.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|mtime| now.duration_since(mtime).ok())
        .map(|age| age.as_secs() as i64 > max_age_secs)
        .unwrap_or(false)
}

/// Run health check command.
async fn run_health() -> Result<()> {
    println!("NeoMind System Health Check");
    println!("==========================\n");

    // Check if server is running
    println!("🔍 Checking server status...");
    let api_base = std::env::var("NEOMIND_API_BASE")
        .unwrap_or_else(|_| "http://localhost:9375/api".to_string());
    let check_url = format!("{}/health", api_base);
    match reqwest::get(&check_url).await {
        Ok(resp) if resp.status().is_success() => {
            println!("  ✅ Server is running");

            // Get detailed health status
            if let Ok(health_json) = resp.json::<serde_json::Value>().await {
                if let Some(status) = health_json.get("status").and_then(|v| v.as_str()) {
                    println!("  Status: {}", status);
                }
            }
        }
        Ok(resp) => {
            println!("  ⚠️  Server returned status: {}", resp.status());
        }
        Err(e) => {
            println!("  ❌ Server is not responding: {}", e);
            println!("  Hint: Start the server with 'neomind serve'");
        }
    }

    println!();

    // Check database files
    println!("🔍 Checking databases...");
    let data_dir = std::path::PathBuf::from("./data");
    if data_dir.exists() {
        let db_files = [
            "telemetry.redb",
            "sessions.redb",
            "devices.redb",
            "extensions.redb",
        ];
        for db in &db_files {
            let db_path = data_dir.join(db);
            if db_path.exists() {
                let metadata = std::fs::metadata(&db_path)?;
                let size_kb = metadata.len() / 1024;
                println!("  ✅ {} ({} KB)", db, size_kb);
            } else {
                println!("  ⚠️  {} not found (will be created on first use)", db);
            }
        }
    } else {
        println!("  ℹ️  Data directory not found (will be created on first use)");
    }

    println!();

    // Check LLM backend
    println!("🔍 Checking LLM backend...");
    if std::env::var("OLLAMA_ENDPOINT").is_ok() || std::env::var("OPENAI_API_KEY").is_ok() {
        if std::env::var("OLLAMA_ENDPOINT").is_ok() {
            let endpoint = std::env::var("OLLAMA_ENDPOINT").unwrap_or_default();
            println!("  ✅ Ollama configured: {}", endpoint);
        }
        if std::env::var("OPENAI_API_KEY").is_ok() {
            println!("  ✅ OpenAI configured");
        }
    } else {
        println!("  ⚠️  No LLM backend configured");
        println!("  Hint: Set OLLAMA_ENDPOINT or OPENAI_API_KEY environment variable");
    }

    println!();

    // Check extensions directory
    println!("🔍 Checking extensions...");
    let extensions_dir = std::path::PathBuf::from("./extensions");
    if extensions_dir.exists() {
        let entries = std::fs::read_dir(&extensions_dir)?;
        let count = entries.filter_map(|e| e.ok()).count();
        println!("  ✅ Extensions directory found ({} items)", count);
    } else {
        println!("  ℹ️  Extensions directory not found");
    }

    println!();
    println!("Health check complete.");

    Ok(())
}

/// Run logs command.
async fn run_logs(
    tail: usize,
    follow: bool,
    level: Option<String>,
    _since: Option<String>,
) -> Result<()> {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Seek};

    // Search log directories: project data/logs/, macOS Tauri, Linux Tauri, Windows Tauri
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    let appdata = std::env::var("APPDATA").unwrap_or_default();

    let log_dirs = [
        std::path::PathBuf::from("data/logs"),
        // macOS: ~/Library/Application Support/com.neomind.neomind/logs/
        std::path::PathBuf::from(&home)
            .join("Library/Application Support/com.neomind.neomind/logs"),
        // Linux: ~/.local/share/com.neomind.neomind/logs/
        std::path::PathBuf::from(&home)
            .join(".local/share/com.neomind.neomind/logs"),
        // Windows: %APPDATA%/com.neomind.neomind/logs/
        std::path::PathBuf::from(&appdata)
            .join("com.neomind.neomind/logs"),
        // Fallback: ~/.neomind/logs/
        std::path::PathBuf::from(&home).join(".neomind/logs"),
    ];

    let log_dir = log_dirs
        .iter()
        .find(|d| d.exists())
        .ok_or_else(|| {
            let paths: String = log_dirs
                .iter()
                .filter(|d| !d.as_os_str().is_empty())
                .map(|d| format!("  {}", d.display()))
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::anyhow!(
                "Log directory not found. Searched in:\n{}\n\
                 Hint: Start the server with 'neomind serve' or run the Tauri desktop app.",
                paths
            )
        })?;

    // Find the newest log file
    let log_path = find_newest_log(log_dir)?;

    if follow {
        // Follow mode (like tail -f)
        println!("Following log file: {} (Ctrl+C to stop)\n", log_path.display());

        let file = File::open(&log_path)?;
        let mut reader = BufReader::new(file);
        reader.seek(std::io::SeekFrom::End(0))?;

        let mut line = String::new();
        loop {
            match reader.read_line(&mut line) {
                Ok(0) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    continue;
                }
                Ok(_) => {
                    if matches_level(&line, &level) {
                        print!("{}", line);
                    }
                    line.clear();
                }
                Err(e) => {
                    eprintln!("Error reading log: {}", e);
                    break;
                }
            }
        }
    } else {
        // Tail mode - show last N lines
        let file = File::open(&log_path)?;
        let reader = BufReader::new(file);

        let lines: Vec<String> = reader
            .lines()
            .map_while(Result::ok)
            .filter(|l| matches_level(l, &level))
            .collect();

        let start = if lines.len() > tail {
            lines.len() - tail
        } else {
            0
        };

        println!(
            "Log file: {} (showing last {} lines)\n",
            log_path.display(),
            lines.len() - start
        );

        for line in lines.iter().skip(start) {
            println!("{}", line);
        }
    }

    Ok(())
}

/// Find the newest log file in the log directory.
fn find_newest_log(log_dir: &Path) -> Result<std::path::PathBuf> {
    let mut newest: Option<(std::path::PathBuf, SystemTime)> = None;

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        // Only consider neomind log files
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if !name.starts_with("neomind.log") {
                continue;
            }
        }
        let mtime = path.metadata().ok().and_then(|m| m.modified().ok());
        if let Some(mtime) = mtime {
            if newest.as_ref().is_none_or(|(_, prev)| mtime > *prev) {
                newest = Some((path, mtime));
            }
        }
    }

    newest
        .map(|(p, _)| p)
        .ok_or_else(|| anyhow::anyhow!("No log files found in data/logs/"))
}

/// Check if a log line matches the requested level filter.
fn matches_level(line: &str, level: &Option<String>) -> bool {
    match level {
        Some(lvl) => line.contains(lvl) || line.to_uppercase().contains(&lvl.to_uppercase()),
        None => true,
    }
}

/// Run extension management commands.
async fn run_extension_cmd(cmd: ExtensionCommand) -> Result<()> {
    match cmd {
        ExtensionCommand::Validate { path, verbose } => validate_nep_package(&path, verbose).await,

        ExtensionCommand::List { verbose } => list_extensions(verbose).await,

        ExtensionCommand::Info { id_or_path } => show_extension_info(&id_or_path).await,

        ExtensionCommand::Install { package } => install_extension(&package).await,

        ExtensionCommand::Uninstall { id } => uninstall_extension(&id).await,

        ExtensionCommand::Create {
            name,
            extension_type,
            output,
        } => {
            create_extension_scaffold(&name, &extension_type, output)?;
            Ok(())
        }

        ExtensionCommand::Status { id } => {
            let client = neomind_cli_ops::ApiClient::new();
            let response = neomind_cli_ops::extension::get_extension_status(&client, &id).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }

        ExtensionCommand::Logs { id, lines } => {
            let client = neomind_cli_ops::ApiClient::new();
            let response = neomind_cli_ops::extension::get_extension_logs(&client, &id, lines).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }

        ExtensionCommand::Build { path } => {
            build_extension(&path)?;
            Ok(())
        }

        ExtensionCommand::MarketInstall { extension_id, version } => {
            let client = neomind_cli_ops::ApiClient::new();
            let response = neomind_cli_ops::extension::install_extension_market(
                &client,
                &extension_id,
                version.as_deref(),
            ).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }

        ExtensionCommand::MarketList => {
            let client = neomind_cli_ops::ApiClient::new();
            let response = neomind_cli_ops::extension::list_marketplace(&client).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        ExtensionCommand::Reload { id } => {
            let client = neomind_cli_ops::ApiClient::new();
            let response = neomind_cli_ops::extension::reload_extension(&client, &id).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
    }
}

/// Validate a .nep extension package.
async fn validate_nep_package(path: &std::path::PathBuf, verbose: bool) -> Result<()> {
    use std::fs::File;
    use zip::ZipArchive;

    if !path.exists() {
        anyhow::bail!("Extension package not found: {}", path.display());
    }

    if path.extension().is_none_or(|e| e != "nep") {
        anyhow::bail!(
            "Invalid extension package. Expected .nep file, got: {}",
            path.extension().unwrap_or_default().display()
        );
    }

    println!("Validating .nep package: {}", path.display());
    println!();

    // Open the ZIP archive
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    // Check for manifest.json - collect names first to avoid borrow issues
    let manifest_names: Vec<String> = archive
        .file_names()
        .filter(|n| n.ends_with("manifest.json"))
        .map(|s| s.to_string())
        .collect();

    if manifest_names.is_empty() {
        println!("❌ Validation FAILED");
        println!("   Missing manifest.json in package");
        std::process::exit(1);
    }

    // Read and parse manifest
    let manifest_path = &manifest_names[0];
    let mut manifest_file = archive.by_name(manifest_path)?;
    let mut manifest_content = String::new();
    manifest_file.read_to_string(&mut manifest_content)?;

    let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse manifest.json: {}", e))?;

    // Validate required fields
    let required_fields = ["id", "name", "version", "format_version"];
    let mut missing = Vec::new();

    for field in &required_fields {
        if manifest.get(field).is_none() {
            missing.push(*field);
        }
    }

    if !missing.is_empty() {
        println!("❌ Validation FAILED");
        println!("   Missing required fields: {}", missing.join(", "));
        std::process::exit(1);
    }

    // Display package info
    println!("✅ Validation PASSED");
    println!();
    println!(
        "ID:              {}",
        manifest["id"].as_str().unwrap_or("unknown")
    );
    println!(
        "Name:            {}",
        manifest["name"].as_str().unwrap_or("unknown")
    );
    println!(
        "Version:         {}",
        manifest["version"].as_str().unwrap_or("unknown")
    );
    println!(
        "Format Version:  {}",
        manifest["format_version"].as_str().unwrap_or("unknown")
    );

    if let Some(abi) = manifest.get("abi_version").and_then(|v| v.as_u64()) {
        println!("ABI Version:     {}", abi);
    }

    if let Some(desc) = manifest.get("description").and_then(|v| v.as_str()) {
        println!("Description:     {}", desc);
    }

    // List binaries
    if let Some(binaries) = manifest.get("binaries").and_then(|v| v.as_object()) {
        println!();
        println!("Binaries:");
        for (platform, path) in binaries {
            println!("  {}: {}", platform, path.as_str().unwrap_or("unknown"));
        }
    }

    if verbose {
        // manifest_file is no longer used, drop it to release archive borrow
        drop(manifest_file);

        println!();
        println!("--- Verbose Details ---");
        println!("Package size:    {} bytes", path.metadata()?.len());
        println!("Package path:    {}", path.display());
        println!("Files in package: {}", archive.len());

        println!();
        println!("Package contents:");
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            println!("  {}", file.name());
        }
    }

    Ok(())
}

/// List installed extensions.
async fn list_extensions(_verbose: bool) -> Result<()> {
    let client = neomind_cli_ops::ApiClient::new();
    let response = neomind_cli_ops::extension::list_extensions(&client).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

/// Show extension information.
async fn show_extension_info(id_or_path: &str) -> Result<()> {
    let path = std::path::PathBuf::from(id_or_path);

    if path.exists() {
        // It's a file path, validate it
        validate_nep_package(&path, true).await?;
        return Ok(());
    }

    // Try API first (shows runtime info: status, commands, metrics)
    let client = neomind_cli_ops::ApiClient::new();
    if let Ok(response) = neomind_cli_ops::extension::get_extension(&client, id_or_path).await {
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }

    // Fallback: search local filesystem for .nep files
    use std::fs;

    let search_dirs = [
        std::path::PathBuf::from("./data/extensions"),
        std::path::PathBuf::from("./extensions"),
    ];

    let mut found = None;
    'search: for search_dir in &search_dirs {
        if let Ok(entries) = fs::read_dir(search_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                if entry_path.extension().is_some_and(|e| e == "nep") {
                    if let Some(stem) = entry_path.file_stem() {
                        if stem.to_str().unwrap_or("").contains(id_or_path) {
                            found = Some(entry_path);
                            break 'search;
                        }
                    }
                }
            }
        }
    }

    if let Some(found_path) = found {
        validate_nep_package(&found_path, true).await?;
        Ok(())
    } else {
        anyhow::bail!("Extension not found: {}", id_or_path);
    }
}

/// Install an extension from .nep package.
async fn install_extension(package: &str) -> Result<()> {
    let source_path = std::path::PathBuf::from(package);

    if !source_path.exists() {
        anyhow::bail!("Package file not found: {}", package);
    }

    println!("Installing extension from: {}", package);

    // Validate first
    validate_nep_package(&source_path, false).await?;

    // Create target directory
    let target_dir = std::path::PathBuf::from("./data/extensions");
    std::fs::create_dir_all(&target_dir)?;

    let target_path = target_dir.join(source_path.file_name().unwrap());

    // Copy package
    std::fs::copy(&source_path, &target_path)?;

    println!();
    println!("✅ Extension installed successfully!");
    println!("   Location: {}", target_path.display());
    println!();
    println!("Note: The extension will be loaded on next server restart.");
    println!("      Or use the Web UI to load it dynamically.");

    Ok(())
}

/// Uninstall an extension.
async fn uninstall_extension(id: &str) -> Result<()> {
    use std::fs;

    let search_dirs = [
        std::path::PathBuf::from("./data/extensions"),
        std::path::PathBuf::from("./extensions"),
    ];

    let mut found = Vec::new();

    for search_dir in &search_dirs {
        if let Ok(entries) = fs::read_dir(search_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "nep") {
                    if let Some(stem) = path.file_stem() {
                        if stem.to_str().unwrap_or("").contains(id) {
                            found.push(path);
                        }
                    }
                }
            }
        }
    }

    if found.is_empty() {
        anyhow::bail!("Extension not found: {}", id);
    }

    if found.len() > 1 {
        println!("Found multiple extensions matching '{}':", id);
        for (i, path) in found.iter().enumerate() {
            println!("  {}. {}", i + 1, path.display());
        }
        anyhow::bail!("Please be more specific");
    }

    let path = &found[0];

    println!("Uninstalling extension: {}", path.display());
    println!("This will delete the extension package.");
    print!("Confirm? [y/N] ");
    use std::io::Write;
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if !input.trim().to_lowercase().starts_with('y') {
        println!("Cancelled.");
        return Ok(());
    }

    fs::remove_file(path)?;

    println!("✅ Extension uninstalled successfully!");

    Ok(())
}

/// Create a new extension scaffold with a complete, compilable project.
fn create_extension_scaffold(
    name: &str,
    _extension_type: &str,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    use std::fs;

    // Validate name is kebab-case (lowercase, digits, hyphens, no spaces)
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        anyhow::bail!(
            "Extension name must be kebab-case (lowercase letters, digits, and hyphens only). Got: '{}'",
            name
        );
    }
    if name.starts_with('-') || name.ends_with('-') {
        anyhow::bail!("Extension name must not start or end with a hyphen. Got: '{}'", name);
    }

    // Derive Rust identifiers
    let crate_name = format!("neomind_{}", name.replace('-', "_"));
    // Strip optional "neomind-" prefix for struct name, then PascalCase
    let struct_base = name.strip_prefix("neomind-").unwrap_or(name);
    let struct_name = struct_base
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<String>();

    // Resolve output directory
    let out_dir = output.unwrap_or_else(|| std::path::PathBuf::from(name));
    if out_dir.exists() {
        anyhow::bail!("Output directory already exists: {}", out_dir.display());
    }

    // SDK version — use the version of this binary's SDK dependency
    let sdk_version = "0.6.3";

    // Create directory structure
    let src_dir = out_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // --- Cargo.toml ---
    let cargo_toml = format!(
        r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2021"

[workspace]

[lib]
crate-type = ["cdylib"]

[dependencies]
neomind-extension-sdk = "{sdk_version}"
async-trait = "0.1"
serde_json = "1.0"
tokio = {{ version = "1", features = ["rt-multi-thread", "sync"] }}
"#,
    );
    fs::write(out_dir.join("Cargo.toml"), cargo_toml)?;

    // --- src/lib.rs ---
    let lib_rs = format!(
        r#"use async_trait::async_trait;
use neomind_extension_sdk::{{
    neomind_export, Extension, ExtensionCommand, ExtensionError, ExtensionMetadata,
    ExtensionMetricValue, MetricDataType, MetricDescriptor, ParamMetricValue,
    ParameterDefinition, Result,
}};
use serde_json::json;

pub struct {struct_name};

impl {struct_name} {{
    pub fn new() -> Self {{
        Self
    }}
}}

impl Default for {struct_name} {{
    fn default() -> Self {{
        Self::new()
    }}
}}

#[async_trait]
impl Extension for {struct_name} {{
    fn metadata(&self) -> &ExtensionMetadata {{
        static META: std::sync::OnceLock<ExtensionMetadata> = std::sync::OnceLock::new();
        META.get_or_init(|| {{
            ExtensionMetadata::new("{name}", "{struct_name}", "0.1.0")
                .with_description("A NeoMind extension")
        }})
    }}

    fn commands(&self) -> Vec<ExtensionCommand> {{
        vec![ExtensionCommand {{
            name: "hello".to_string(),
            display_name: "Hello".to_string(),
            description: "Returns a greeting".to_string(),
            payload_template: String::new(),
            parameters: vec![ParameterDefinition {{
                name: "name".to_string(),
                display_name: "Name".to_string(),
                description: "Who to greet".to_string(),
                param_type: MetricDataType::String,
                required: true,
                default_value: None,
                min: None,
                max: None,
                options: Vec::new(),
            }}],
            fixed_values: std::collections::HashMap::new(),
            samples: vec![json!({{"name": "world"}})],
            parameter_groups: Vec::new(),
        }}]
    }}

    fn metrics(&self) -> Vec<MetricDescriptor> {{
        vec![MetricDescriptor {{
            name: "invocations".to_string(),
            display_name: "Invocations".to_string(),
            data_type: MetricDataType::Integer,
            unit: "count".to_string(),
            min: Some(0.0),
            max: None,
            required: false,
        }}]
    }}

    async fn execute_command(
        &self,
        command: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value> {{
        match command {{
            "hello" => {{
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("world");
                Ok(json!({{"greeting": format!("Hello, {{}}!", name)}}))
            }}
            _ => Err(ExtensionError::CommandNotFound(command.to_string())),
        }}
    }}

    fn produce_metrics(&self) -> Result<Vec<ExtensionMetricValue>> {{
        Ok(vec![ExtensionMetricValue::new(
            "invocations",
            ParamMetricValue::Integer(1),
        )])
    }}

    fn as_any(&self) -> &dyn std::any::Any {{
        self
    }}
}}

neomind_export!({struct_name});
"#,
    );
    fs::write(src_dir.join("lib.rs"), lib_rs)?;

    // --- manifest.json ---
    let manifest = serde_json::json!({
        "format": "neomind-extension-package",
        "abi_version": 3,
        "type": "native",
        "id": name,
        "name": struct_name,
        "version": "0.1.0",
        "capabilities": {
            "commands": ["hello"],
            "metrics": ["invocations"]
        }
    });
    fs::write(
        out_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;

    // --- .gitignore ---
    fs::write(out_dir.join(".gitignore"), "/target/\n")?;

    println!("Extension created: {}", out_dir.display());
    println!();
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  cargo build");
    println!("  neomind extension build ./{}", name);

    Ok(())
}

/// Build an extension from source.
fn build_extension(path: &std::path::PathBuf) -> Result<()> {
    use std::process::Command;

    if !path.exists() {
        anyhow::bail!("Extension path not found: {}", path.display());
    }

    println!("Building extension: {}", path.display());
    println!();

    let cargo_toml = path.join("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!("Cargo.toml not found in extension path. Is this a Rust project?");
    }

    // Run cargo build
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(path)
        .status()?;

    if status.success() {
        println!("✅ Extension built successfully!");
        Ok(())
    } else {
        anyhow::bail!("Extension build failed");
    }
}

/// Run API key management commands.
/// Run LLM backend management commands.
async fn run_llm_cmd(cmd: LlmCommand) -> Result<()> {
    use neomind_cli_ops::llm::*;
    use neomind_cli_ops::output::format_output;
    use neomind_cli_ops::types::OutputFormat;

    let api_base = std::env::var("NEOMIND_API_BASE")
        .unwrap_or_else(|_| "http://localhost:9375/api".to_string());
    let client = neomind_cli_ops::ApiClient::with_base_url(&api_base);
    let output_format = OutputFormat::Human;

    let response = match cmd {
        LlmCommand::List { json: _ } => {
            list_backends(&client).await?
        }
        LlmCommand::Get { id } => {
            get_backend(&client, &id).await?
        }
        LlmCommand::Models { endpoint: _ } => {
            list_ollama_models(&client).await?
        }
    };

    format_output(&response, output_format);
    Ok(())
}

async fn run_api_key_cmd(cmd: ApiKeyCommand) -> Result<()> {
    match cmd {
        ApiKeyCommand::Create { name, data_dir } => {
            std::fs::create_dir_all(&data_dir)?;

            let auth = neomind_api::auth::AuthState::new_with_data_dir(&data_dir);
            let (key, info) = auth.create_key(name.clone(), vec!["*".to_string()]).await;

            println!("API Key created successfully!");
            println!();
            println!("  Name: {}", info.name);
            println!("  ID:   {}", info.id);
            println!("  Key:  {}", key);
            println!();
            println!("IMPORTANT: Save this key now. It will not be shown again.");
        }
        ApiKeyCommand::List { data_dir } => {
            let auth = neomind_api::auth::AuthState::new_with_data_dir(&data_dir);
            let keys = auth.list_keys().await;

            if keys.is_empty() {
                println!("No API keys found.");
                return Ok(());
            }

            println!("{:<36} {:<20} {:<12} {:<20}", "ID", "Name", "Active", "Created");
            println!("{}", "-".repeat(90));
            for (_hash, info) in keys {
                let created = {
                    let days = info.created_at / 86400;
                    let time = info.created_at % 86400;
                    let hours = time / 3600;
                    let minutes = (time % 3600) / 60;
                    format!("{}-{:02}:{:02}", days, hours, minutes)
                };
                println!("{:<36} {:<20} {:<12} {}", info.id, info.name, if info.active { "yes" } else { "no" }, created);
            }
        }
        ApiKeyCommand::Delete { name, data_dir } => {
            let auth = neomind_api::auth::AuthState::new_with_data_dir(&data_dir);
            let keys = auth.list_keys().await;
            let target = keys.iter().find(|(_, info)| info.name == name);

            match target {
                Some((hash, info)) => {
                    let removed = auth.delete_key_by_hash(hash).await;
                    if removed {
                        println!("Deleted API key '{}' (ID: {})", name, info.id);
                    } else {
                        anyhow::bail!("Failed to delete API key '{}'", name);
                    }
                }
                None => {
                    anyhow::bail!("API key '{}' not found", name);
                }
            }
        }
    }
    Ok(())
}

/// Run device management commands.
async fn run_device_cmd(cmd: DeviceCommand) -> Result<()> {
    use neomind_cli_ops::{ApiClient, device::*, output::format_output};
    use neomind_cli_ops::types::OutputFormat;

    // Get API base URL from environment or use default
    let api_base = std::env::var("NEOMIND_API_BASE")
        .unwrap_or_else(|_| "http://localhost:9375/api".to_string());

    // Create API client
    let client = ApiClient::with_base_url(&api_base);

    let (response, output_format) = match cmd {
        DeviceCommand::List { device_type, status, json } => {
            let output_format = if json { OutputFormat::Json } else { OutputFormat::Human };
            (list_devices(&client, device_type.as_deref(), status.as_deref()).await?, output_format)
        }
        DeviceCommand::Get { id, json } => {
            let output_format = if json { OutputFormat::Json } else { OutputFormat::Human };
            (get_device(&client, &id).await?, output_format)
        }
        DeviceCommand::Create { name, device_type, adapter_type, config, json } => {
            let output_format = if json { OutputFormat::Json } else { OutputFormat::Human };
            let connection_config = if let Some(config_str) = config {
                Some(serde_json::from_str(&config_str)?)
            } else {
                None
            };
            (create_device(&client, &name, &device_type, &adapter_type, connection_config).await?, output_format)
        }
        DeviceCommand::Update { id, name, config, json } => {
            let output_format = if json { OutputFormat::Json } else { OutputFormat::Human };
            let connection_config = if let Some(config_str) = config {
                Some(serde_json::from_str(&config_str)?)
            } else {
                None
            };
            (update_device(&client, &id, name.as_deref(), connection_config).await?, output_format)
        }
        DeviceCommand::Delete { id, json } => {
            let output_format = if json { OutputFormat::Json } else { OutputFormat::Human };
            (delete_device(&client, &id).await?, output_format)
        }
        DeviceCommand::Latest { id, json } => {
            let output_format = if json { OutputFormat::Json } else { OutputFormat::Human };
            (get_latest_metrics(&client, &id).await?, output_format)
        }
        DeviceCommand::History { id, metric, time_range, compress, json } => {
            let output_format = if json { OutputFormat::Json } else { OutputFormat::Human };
            (get_telemetry_history(&client, &id, metric.as_deref(), time_range.as_deref(), compress).await?, output_format)
        }
        DeviceCommand::Control { id, command, params, json } => {
            let output_format = if json { OutputFormat::Json } else { OutputFormat::Human };
            let params_json = if let Some(params_str) = params {
                serde_json::from_str(&params_str)?
            } else {
                serde_json::json!({})
            };
            (control_device(&client, &id, &command, params_json).await?, output_format)
        }
        DeviceCommand::Types { type_cmd } => {
            // For Types subcommands, keep using the old behavior for now
            // (they don't have --json flag in this change)
            let output_format = OutputFormat::Human;
            return run_device_type_cmd(client, type_cmd, output_format).await;
        }
        DeviceCommand::WriteMetric { id, metric, value, timestamp, json } => {
            let output_format = if json { OutputFormat::Json } else { OutputFormat::Human };
            // Try parsing value as number, bool, then fallback to string
            let value_json = if let Ok(n) = value.parse::<f64>() {
                serde_json::json!(n)
            } else if let Ok(b) = value.parse::<bool>() {
                serde_json::json!(b)
            } else {
                serde_json::json!(value)
            };
            (write_metric(&client, &id, &metric, value_json, timestamp).await?, output_format)
        }
    };

    // Format and print output
    format_output(&response, output_format);
    Ok(())
}

/// Run device type management commands.
async fn run_device_type_cmd(
    client: neomind_cli_ops::ApiClient,
    cmd: DeviceTypeCommand,
    output_format: neomind_cli_ops::types::OutputFormat,
) -> Result<()> {
    use neomind_cli_ops::device::*;
    use neomind_cli_ops::output::format_output;

    let response = match cmd {
        DeviceTypeCommand::List => {
            list_device_types(&client).await?
        }
        DeviceTypeCommand::Get { id } => {
            get_device_type(&client, &id).await?
        }
        DeviceTypeCommand::Create { id, name, metrics, commands } => {
            let metrics_json = serde_json::from_str(&metrics)?;
            let commands_json = if let Some(cmds_str) = commands {
                Some(serde_json::from_str(&cmds_str)?)
            } else {
                None
            };
            create_device_type(&client, id.as_deref(), &name, metrics_json, commands_json).await?
        }
        DeviceTypeCommand::Delete { id } => {
            delete_device_type(&client, &id).await?
        }
    };

    // Format and print output
    format_output(&response, output_format);
    Ok(())
}

/// Run dashboard management commands.
async fn run_dashboard_cmd(cmd: DashboardCommand) -> Result<()> {
    use neomind_cli_ops::{ApiClient, dashboard::*, output::format_output};
    use neomind_cli_ops::types::OutputFormat;

    // Get API base URL from environment or use default
    let api_base = std::env::var("NEOMIND_API_BASE")
        .unwrap_or_else(|_| "http://localhost:9375/api".to_string());

    // Create API client
    let client = ApiClient::with_base_url(&api_base);

    // Get output format (check for --json flag in global args or environment)
    let output_format = if std::env::var("NEOMIND_JSON").is_ok() {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    let response = match cmd {
        DashboardCommand::List { json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let resp = list_dashboards(&client).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
        DashboardCommand::Get { id, json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let resp = get_dashboard(&client, &id).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
        DashboardCommand::Create { name, description, layout, json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let layout_json = if let Some(layout_str) = layout {
                Some(serde_json::from_str(&layout_str)?)
            } else {
                None
            };
            let resp = create_dashboard(&client, &name, description.as_deref(), layout_json).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
        DashboardCommand::Update { id, name, description, layout, components, json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let layout_json = if let Some(layout_str) = layout {
                Some(serde_json::from_str(&layout_str)?)
            } else {
                None
            };
            let components_json = if let Some(components_str) = components {
                Some(serde_json::from_str(&components_str)?)
            } else {
                None
            };
            let resp = update_dashboard(&client, &id, name.as_deref(), description.as_deref(), layout_json, components_json).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
        DashboardCommand::Delete { id } => {
            delete_dashboard(&client, &id).await?
        }
        DashboardCommand::AddComponents { id, components, json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let comps = serde_json::from_str(&components).unwrap_or(serde_json::json!([]));
            let resp = add_components(&client, &id, comps).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
        DashboardCommand::RemoveComponents { id, ids, json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let ids_val = serde_json::from_str(&ids).unwrap_or(serde_json::json!([]));
            let resp = remove_components(&client, &id, ids_val).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
        DashboardCommand::Share { id, public, expires, json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let resp = share_dashboard(&client, &id, public, expires.as_deref()).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
    };

    // Format and print output
    format_output(&response, output_format);
    Ok(())
}

/// Run rule management commands.
async fn run_rule_cmd(cmd: RuleCommand) -> Result<()> {
    use neomind_cli_ops::{ApiClient, rule::*, output::format_output};
    use neomind_cli_ops::types::OutputFormat;

    // Get API base URL from environment or use default
    let api_base = std::env::var("NEOMIND_API_BASE")
        .unwrap_or_else(|_| "http://localhost:9375/api".to_string());

    // Create API client
    let client = ApiClient::with_base_url(&api_base);

    // Get output format (check for --json flag in global args or environment)
    let output_format = if std::env::var("NEOMIND_JSON").is_ok() {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    let response = match cmd {
        RuleCommand::List => {
            list_rules(&client).await?
        }
        RuleCommand::Get { id } => {
            get_rule(&client, &id).await?
        }
        RuleCommand::Create { name, dsl } => {
            create_rule(&client, name.as_deref(), &dsl).await?
        }
        RuleCommand::Update { id, name, dsl } => {
            update_rule(&client, &id, name.as_deref(), dsl.as_deref()).await?
        }
        RuleCommand::Delete { id } => {
            delete_rule(&client, &id).await?
        }
        RuleCommand::Enable { id } => {
            enable_rule(&client, &id).await?
        }
        RuleCommand::Disable { id } => {
            disable_rule(&client, &id).await?
        }
        RuleCommand::Test { id, input } => {
            let input_json = serde_json::from_str(&input)?;
            test_rule(&client, &id, input_json).await?
        }
        RuleCommand::History { id } => {
            get_rule_history(&client, &id).await?
        }
    };

    // Format and print output
    format_output(&response, output_format);
    Ok(())
}

/// Run transform management commands.
async fn run_transform_cmd(cmd: TransformCommand) -> Result<()> {
    use neomind_cli_ops::{ApiClient, transform::*, output::format_output};
    use neomind_cli_ops::types::OutputFormat;

    // Get API base URL from environment or use default
    let api_base = std::env::var("NEOMIND_API_BASE")
        .unwrap_or_else(|_| "http://localhost:9375/api".to_string());

    // Create API client
    let client = ApiClient::with_base_url(&api_base);

    // Get output format (check for --json flag in global args or environment)
    let output_format = if std::env::var("NEOMIND_JSON").is_ok() {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    let response = match cmd {
        TransformCommand::List => {
            list_transforms(&client).await?
        }
        TransformCommand::Get { id } => {
            get_transform(&client, &id).await?
        }
        TransformCommand::Create { name, scope, code, output_prefix, description, enabled } => {
            create_transform(
                &client, &name, &scope, &code,
                output_prefix.as_deref(), description.as_deref(), enabled,
            ).await?
        }
        TransformCommand::Update { id, name, description, code, scope, output_prefix, enabled } => {
            update_transform(
                &client, &id,
                name.as_deref(), description.as_deref(), code.as_deref(),
                scope.as_deref(), output_prefix.as_deref(), enabled,
            ).await?
        }
        TransformCommand::Delete { id } => {
            delete_transform(&client, &id).await?
        }
        TransformCommand::Metrics => {
            list_virtual_metrics(&client).await?
        }
        TransformCommand::TestCode { code, input } => {
            let input_json = serde_json::from_str(&input)?;
            test_transform_code(&client, &code, input_json).await?
        }
        TransformCommand::DataSources => {
            list_transform_data_sources(&client).await?
        }
    };

    // Format and print output
    format_output(&response, output_format);
    Ok(())
}

/// Run agent management commands.
async fn run_agent_cmd(cmd: AgentCommand) -> Result<()> {
    use neomind_cli_ops::{ApiClient, agent_cmd::*, output::format_output};
    use neomind_cli_ops::types::OutputFormat;

    // Get API base URL from environment or use default
    let api_base = std::env::var("NEOMIND_API_BASE")
        .unwrap_or_else(|_| "http://localhost:9375/api".to_string());

    // Create API client
    let client = ApiClient::with_base_url(&api_base);

    // Get output format (check for --json flag in global args or environment)
    let output_format = if std::env::var("NEOMIND_JSON").is_ok() {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    let response = match cmd {
        AgentCommand::List => {
            list_agents(&client).await?
        }
        AgentCommand::Get { id } => {
            get_agent(&client, &id).await?
        }
        AgentCommand::Create { name, prompt, description, schedule_type, schedule_config, every, event_filter, timezone, llm_backend, system_prompt, execution_mode, device_ids, resources, metrics, commands, enable_tool_chaining, max_chain_depth, priority, context_window_size } => {
            // Handle --every shortcut: parse duration to interval schedule
            let (resolved_st, resolved_sc) = if let Some(dur) = &every {
                let secs = parse_duration(dur);
                (Some("interval".to_string()), Some(secs.to_string()))
            } else {
                (schedule_type.clone(), schedule_config.clone())
            };
            create_agent(
                &client,
                &name,
                &prompt,
                description.as_deref(),
                resolved_st.as_deref(),
                resolved_sc.as_deref(),
                event_filter.as_deref(),
                timezone.as_deref(),
                llm_backend.as_deref(),
                system_prompt.as_deref(),
                execution_mode.as_deref(),
                device_ids.as_deref(),
                resources.as_deref(),
                metrics.as_deref(),
                commands.as_deref(),
                enable_tool_chaining,
                max_chain_depth,
                priority,
                context_window_size,
            ).await?
        }
        AgentCommand::Update { id, name, prompt, description, llm_backend, system_prompt, schedule_type, schedule_config, execution_mode, device_ids, resources, metrics, commands, enable_tool_chaining, max_chain_depth, priority, context_window_size } => {
            update_agent(
                &client,
                &id,
                name.as_deref(),
                description.as_deref(),
                llm_backend.as_deref(),
                system_prompt.as_deref(),
                prompt.as_deref(),
                schedule_type.as_deref(),
                schedule_config.as_deref(),
                execution_mode.as_deref(),
                device_ids.as_deref(),
                resources.as_deref(),
                metrics.as_deref(),
                commands.as_deref(),
                enable_tool_chaining,
                max_chain_depth,
                priority,
                context_window_size,
            ).await?
        }
        AgentCommand::Delete { id } => {
            delete_agent(&client, &id).await?
        }
        AgentCommand::Control { id, status } => {
            control_agent(&client, &id, &status).await?
        }
        AgentCommand::Invoke { id, input } => {
            invoke_agent(&client, &id, &input).await?
        }
        AgentCommand::Memory { id } => {
            get_agent_memory(&client, &id).await?
        }
        AgentCommand::Executions { id, limit, offset } => {
            get_agent_executions(&client, &id, limit, offset).await?
        }
        AgentCommand::LatestExecution { id } => {
            get_latest_execution(&client, &id).await?
        }
        AgentCommand::Conversation { id, limit } => {
            get_conversation(&client, &id, limit).await?
        }
        AgentCommand::SendMessage { id, message, message_type } => {
            send_message(&client, &id, &message, message_type.as_deref()).await?
        }
    };

    // Format and print output
    format_output(&response, output_format);
    Ok(())
}

/// Run message management commands.
async fn run_message_cmd(cmd: MessageCommand) -> Result<()> {
    use neomind_cli_ops::{ApiClient, message::*, output::format_output};
    use neomind_cli_ops::types::OutputFormat;

    // Get API base URL from environment or use default
    let api_base = std::env::var("NEOMIND_API_BASE")
        .unwrap_or_else(|_| "http://localhost:9375/api".to_string());

    // Create API client
    let client = ApiClient::with_base_url(&api_base);

    // Get output format (check for --json flag in global args or environment)
    let output_format = if std::env::var("NEOMIND_JSON").is_ok() {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    let response = match cmd {
        MessageCommand::List { limit, offset, severity, status } => {
            list_messages(&client, limit, offset, severity.as_deref(), status.as_deref()).await?
        }
        MessageCommand::Get { id } => {
            get_message(&client, &id).await?
        }
        MessageCommand::Send { title, message, severity, source } => {
            send_message(&client, &title, &message, &severity, source.as_deref()).await?
        }
        MessageCommand::Read { id } => {
            acknowledge_message(&client, &id).await?
        }
        MessageCommand::ChannelList => {
            list_channels(&client).await?
        }
        MessageCommand::ChannelGet { name } => {
            get_channel(&client, &name).await?
        }
        MessageCommand::ChannelTypes => {
            list_channel_types(&client).await?
        }
        MessageCommand::ChannelTypeSchema { channel_type } => {
            get_channel_type_schema(&client, &channel_type).await?
        }
        MessageCommand::ChannelCreate { name, channel_type, config } => {
            create_channel(&client, &name, &channel_type, &config).await?
        }
        MessageCommand::ChannelUpdate { name, config } => {
            update_channel(&client, &name, &config).await?
        }
        MessageCommand::ChannelDelete { name } => {
            delete_channel(&client, &name).await?
        }
        MessageCommand::ChannelTest { name } => {
            test_channel(&client, &name).await?
        }
    };

    // Format and print output
    format_output(&response, output_format);
    Ok(())
}

/// Run push management commands.
async fn run_push_cmd(cmd: PushCommand) -> Result<()> {
    use neomind_cli_ops::{ApiClient, data_push::*, output::format_output};
    use neomind_cli_ops::types::OutputFormat;

    let api_base = std::env::var("NEOMIND_API_BASE")
        .unwrap_or_else(|_| "http://localhost:9375/api".to_string());
    let client = ApiClient::with_base_url(&api_base);
    let output_format = if std::env::var("NEOMIND_JSON").is_ok() {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    let response = match cmd {
        PushCommand::List => list_targets(&client).await?,
        PushCommand::Get { id } => get_target(&client, &id).await?,
        PushCommand::Create { name, target_type, config, schedule, sources } => {
            let t_type = target_type.as_deref().unwrap_or("webhook");
            let cfg = config.as_deref().unwrap_or("{}");
            let sched = schedule.as_deref().unwrap_or("event");
            let src = sources.as_deref().unwrap_or("");
            create_target(&client, &name, t_type, cfg, sched, src).await?
        }
        PushCommand::Update { id, name, config, enabled } => {
            update_target(&client, &id, name.as_deref(), config.as_deref(), enabled).await?
        }
        PushCommand::Delete { id } => delete_target(&client, &id).await?,
        PushCommand::Start { id } => start_target(&client, &id).await?,
        PushCommand::Stop { id } => stop_target(&client, &id).await?,
        PushCommand::Test { id } => test_target(&client, &id).await?,
        PushCommand::Logs { id, limit } => list_logs(&client, &id, Some(limit)).await?,
        PushCommand::Stats => get_stats(&client).await?,
    };

    format_output(&response, output_format);
    Ok(())
}

/// Run widget management commands.
async fn run_widget_cmd(cmd: WidgetCommand) -> Result<()> {
    use neomind_cli_ops::{ApiClient, widget::*, output::format_output};
    use neomind_cli_ops::types::OutputFormat;

    // Get API base URL from environment or use default
    let api_base = std::env::var("NEOMIND_API_BASE")
        .unwrap_or_else(|_| "http://localhost:9375/api".to_string());

    // Create API client
    let client = ApiClient::with_base_url(&api_base);

    // Get output format (check for --json flag in global args or environment)
    let output_format = if std::env::var("NEOMIND_JSON").is_ok() {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    let response = match cmd {
        WidgetCommand::List { json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let resp = list_widgets(&client).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
        WidgetCommand::Get { id, json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let resp = get_widget(&client, &id).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
        WidgetCommand::Bundle { id, json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let resp = get_widget_bundle(&client, &id).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
        WidgetCommand::Create { name, widget_type, output, json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let resp = create_widget(&name, &widget_type, output.as_deref())?;
            format_output(&resp, fmt);
            return Ok(());
        }
        WidgetCommand::Install { file } => {
            install_widget_file(&client, &file).await?
        }
        WidgetCommand::Uninstall { id } => {
            uninstall_widget(&client, &id).await?
        }
        WidgetCommand::MarketList { json } => {
            let fmt = if json { OutputFormat::Json } else { output_format };
            let resp = list_marketplace_widgets(&client).await?;
            format_output(&resp, fmt);
            return Ok(());
        }
        WidgetCommand::MarketInstall { id, version } => {
            install_widget_market(&client, &id, version.as_deref()).await?
        }
    };

    // Format and print output (for commands without --json flag)
    format_output(&response, output_format);
    Ok(())
}

async fn run_system_cmd(cmd: SystemCommand) -> Result<()> {
    use neomind_cli_ops::output::format_output;
    use neomind_cli_ops::types::OutputFormat;
    let client = neomind_cli_ops::ApiClient::new();

    match cmd {
        SystemCommand::Info { json } => {
            let fmt = if json { OutputFormat::Json } else { OutputFormat::Human };
            let resp = neomind_cli_ops::system::system_info(&client).await?;
            format_output(&resp, fmt);
        }
    }
    Ok(())
}

async fn run_connector_cmd(cmd: ConnectorCommand) -> Result<()> {
    use neomind_cli_ops::output::format_output;
    use neomind_cli_ops::types::OutputFormat;
    let client = neomind_cli_ops::ApiClient::new();

    match cmd {
        ConnectorCommand::List { json } => {
            let fmt = if json { OutputFormat::Json } else { OutputFormat::Human };
            let resp = neomind_cli_ops::connector::list_connectors(&client).await?;
            format_output(&resp, fmt);
        }
        ConnectorCommand::Get { id, json } => {
            let fmt = if json { OutputFormat::Json } else { OutputFormat::Human };
            let resp = neomind_cli_ops::connector::get_connector(&client, &id).await?;
            format_output(&resp, fmt);
        }
        ConnectorCommand::Create { connector_type, name, host, port, tls, username, password, topics, json } => {
            let fmt = if json { OutputFormat::Json } else { OutputFormat::Human };
            let resp = neomind_cli_ops::connector::create_connector(
                &client, &name, Some(&connector_type), &host, port, tls,
                username.as_deref(), password.as_deref(), topics.as_deref(),
            ).await?;
            format_output(&resp, fmt);
        }
        ConnectorCommand::Update { id, name, host, port, tls, username, password, topics, disable, json } => {
            let fmt = if json { OutputFormat::Json } else { OutputFormat::Human };
            let enabled = if disable { Some(false) } else { None };
            let tls_val = if tls { Some(true) } else { None };
            let resp = neomind_cli_ops::connector::update_connector(
                &client, &id, name.as_deref(), host.as_deref(), port, tls_val,
                username.as_deref(), password.as_deref(), topics.as_deref(), enabled,
            ).await?;
            format_output(&resp, fmt);
        }
        ConnectorCommand::Delete { id } => {
            let resp = neomind_cli_ops::connector::delete_connector(&client, &id).await?;
            format_output(&resp, OutputFormat::Human);
        }
        ConnectorCommand::Test { id } => {
            let resp = neomind_cli_ops::connector::test_connector(&client, &id).await?;
            format_output(&resp, OutputFormat::Human);
        }
        ConnectorCommand::Subscriptions { json } => {
            let fmt = if json { OutputFormat::Json } else { OutputFormat::Human };
            let resp = neomind_cli_ops::connector::list_subscriptions(&client).await?;
            format_output(&resp, fmt);
        }
        ConnectorCommand::Subscribe { topic, qos } => {
            let resp = neomind_cli_ops::connector::subscribe_topic(&client, &topic, Some(qos)).await?;
            format_output(&resp, OutputFormat::Human);
        }
        ConnectorCommand::Unsubscribe { topic } => {
            let resp = neomind_cli_ops::connector::unsubscribe_topic(&client, &topic).await?;
            format_output(&resp, OutputFormat::Human);
        }
    }
    Ok(())
}
