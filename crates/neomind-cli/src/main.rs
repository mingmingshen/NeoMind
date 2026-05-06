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
}

/// API key subcommands.
#[derive(Subcommand, Debug)]
enum ApiKeyCommand {
    /// Create a new API key.
    Create {
        /// Name for the key.
        #[arg(short, long, default_value = "default")]
        name: String,
        /// Data directory path.
        #[arg(long, default_value = "data")]
        data_dir: String,
    },
    /// List all API keys.
    List {
        /// Data directory path.
        #[arg(long, default_value = "data")]
        data_dir: String,
    },
    /// Delete an API key by name.
    Delete {
        /// Key name to delete.
        name: String,
        /// Data directory path.
        #[arg(long, default_value = "data")]
        data_dir: String,
    },
}

/// Extension subcommands.
#[derive(Subcommand, Debug)]
enum ExtensionCommand {
    /// Validate a .nep extension package.
    Validate {
        /// Path to the .nep file.
        #[arg(required = true)]
        path: std::path::PathBuf,
        /// Show detailed output.
        #[arg(short, long)]
        verbose: bool,
    },
    /// List installed extensions.
    List {
        /// Show detailed information.
        #[arg(short, long)]
        verbose: bool,
    },
    /// Show extension information.
    Info {
        /// Extension ID or .nep file path.
        #[arg(required = true)]
        id_or_path: String,
    },
    /// Install a .nep extension package.
    Install {
        /// Path to the .nep file or URL.
        #[arg(required = true)]
        package: String,
    },
    /// Uninstall an extension.
    Uninstall {
        /// Extension ID.
        #[arg(required = true)]
        id: String,
    },
    /// Create a new extension scaffold.
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
    let check_url = "http://localhost:9375/api/health";
    match reqwest::get(check_url).await {
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
async fn list_extensions(verbose: bool) -> Result<()> {
    use std::fs;

    let search_dirs = [
        std::path::PathBuf::from("./data/extensions"),
        std::path::PathBuf::from("./extensions"),
    ];

    println!("Installed Extensions");
    println!("====================\\n");

    let mut found_count = 0;

    for search_dir in &search_dirs {
        if !search_dir.exists() {
            continue;
        }

        let entries = fs::read_dir(search_dir)?;

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            // Check for .nep files
            if path.extension().is_some_and(|e| e == "nep") {
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");

                println!("📦 {}", name);
                println!("   Path: {}", path.display());

                if verbose {
                    // Try to read manifest
                    if let Ok(manifest) = read_nep_manifest(&path) {
                        if let Some(version) = manifest.get("version").and_then(|v| v.as_str()) {
                            println!("   Version: {}", version);
                        }
                        if let Some(desc) = manifest.get("description").and_then(|v| v.as_str()) {
                            println!("   Description: {}", desc);
                        }
                    }
                    println!("   Size: {} bytes", path.metadata()?.len());
                }

                println!();
                found_count += 1;
            }
        }
    }

    if found_count == 0 {
        println!("No extensions found.");
        println!();
        println!("Searched in:");
        for dir in &search_dirs {
            println!("  - {}", dir.display());
        }
        println!();
        println!("Install extensions using:");
        println!("  neomind extension install <package.nep>");
    } else {
        println!("Total: {} extension(s)", found_count);
    }

    Ok(())
}

/// Show extension information.
async fn show_extension_info(id_or_path: &str) -> Result<()> {
    let path = std::path::PathBuf::from(id_or_path);

    if path.exists() {
        // It's a file path, validate it
        validate_nep_package(&path, true).await?;
    } else {
        // It's an extension ID, search for it
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
        } else {
            anyhow::bail!("Extension not found: {}", id_or_path);
        }
    }

    Ok(())
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

/// Read manifest from .nep package.
fn read_nep_manifest(path: &std::path::PathBuf) -> Result<serde_json::Value> {
    use std::fs::File;
    use zip::ZipArchive;

    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    // Collect file names first to avoid borrowing issues
    let manifest_names: Vec<String> = archive
        .file_names()
        .filter(|n| n.ends_with("manifest.json"))
        .map(|s| s.to_string())
        .collect();

    if let Some(name) = manifest_names.into_iter().next() {
        let manifest_file = archive.by_name(&name)?;
        let mut content = String::new();
        let mut reader = std::io::BufReader::new(manifest_file);
        reader.read_to_string(&mut content)?;
        let manifest: serde_json::Value = serde_json::from_str(&content)?;
        return Ok(manifest);
    }

    anyhow::bail!("No manifest.json found in package")
}

/// Create a new extension scaffold.
fn create_extension_scaffold(
    name: &str,
    extension_type: &str,
    _output: Option<std::path::PathBuf>,
) -> Result<()> {
    let valid_types = [
        "tool",
        "llm_backend",
        "storage_backend",
        "device_adapter",
        "integration",
        "alert_channel",
        "rule_engine",
        "workflow_engine",
    ];

    if !valid_types.contains(&extension_type) {
        anyhow::bail!(
            "Invalid extension type '{}'. Valid types: {}",
            extension_type,
            valid_types.join(", ")
        );
    }

    println!("Creating extension: {} (type: {})", name, extension_type);
    println!();
    println!("Please use the extension SDK for full scaffold generation:");
    println!("  See: https://github.com/camthink-ai/NeoMind-Extensions");
    println!();
    println!("Or manually create the extension structure:");
    println!("  mkdir -p extensions/{}", name);
    println!("  cd extensions/{}", name);
    println!("  cargo init --lib");
    println!("  # Add neomind-extension-sdk dependency and implement Extension trait");

    Ok(())
}

/// Run API key management commands.
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
