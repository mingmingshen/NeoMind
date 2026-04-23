//! Web server for Edge AI Agent.
//!
//! This provides a web interface with WebSocket support for chat
//! and REST API for devices, rules, alerts, and session management.

pub mod assets;
pub mod extension_metrics;
pub mod install_service;
pub mod middleware;
pub mod router;
pub mod state;
pub mod tools;
pub mod types;
pub mod uninstall_service;

// Re-export commonly used types
pub use install_service::ExtensionInstallService;
pub use uninstall_service::{ExtensionUninstallService, UninstallReport};

// Re-export tools
pub use tools::TransformTool;

// Re-export commonly used types
pub use middleware::rate_limit_middleware;
pub use router::{create_router, create_router_with_state};
pub use state::DeviceStatusUpdate;
pub use types::{ServerState, MAX_REQUEST_BODY_SIZE};

use std::net::SocketAddr;
use std::time::Duration;

/// Start the web server on a specific address.
/// This is the main entry point for running the server.
pub async fn run(bind: SocketAddr) -> anyhow::Result<()> {
    use crate::startup::{ServiceStatus, StartupLogger};

    // Note: V2 extension system doesn't require panic hook installation
    // The V2 system uses safer FFI boundaries directly

    let mut startup = StartupLogger::new();
    startup.banner();

    let state = ServerState::new().await;

    // Initialization phase
    startup.phase_init();

    // Initialize device type storage (must be before init_mqtt)
    state.init_device_storage().await;
    startup.service("Device storage", ServiceStatus::Started);

    // Initialize LLM
    state.init_llm().await;

    // Initialize transform event service
    state.init_transform_event_service().await;
    startup.service("Transform event service", ServiceStatus::Started);

    // Initialize tools
    state.init_tools().await;
    startup.service("AI tools", ServiceStatus::Started);

    // Initialize rule engine event service
    state.init_rule_engine_events().await;
    startup.service("Rule engine events", ServiceStatus::Started);

    // Initialize auto-onboarding event listener
    state.init_auto_onboarding_events().await;
    startup.service("Auto-onboarding events", ServiceStatus::Started);

    // Initialize extension metrics collector (decoupled from device system)
    let runtime = state.extensions.runtime.clone();
    let metrics_storage = state.extensions.metrics_storage.clone();
    let _metrics_task = tokio::spawn(async move {
        use crate::server::extension_metrics::ExtensionMetricsCollector;
        use std::time::Duration;

        let collector = ExtensionMetricsCollector::new(runtime, metrics_storage)
            .with_interval(Duration::from_secs(60));

        collector.run().await;
    });
    startup.service("Extension metrics collector", ServiceStatus::Started);

    // Kill orphaned extension runner processes from a previous session.
    // MUST run before init_extensions() to avoid killing newly spawned runners.
    // Orphaned runners hold dylib files open and cause dlopen() hangs.
    neomind_core::extension::isolated::IsolatedExtensionManager::cleanup_orphaned_runners();
    startup.service("Extension orphan cleanup", ServiceStatus::Started);

    // Initialize extensions from persistent storage
    // This loads all extensions marked with auto_start=true
    state.init_extensions().await;

    // Refresh tool registry now that extensions are loaded
    // (init_tools runs before extensions, so we rescan here)
    state.refresh_extension_tools().await;

    // Start extension death monitoring for auto-restart
    state.extensions.runtime.clone().start_death_monitoring();
    startup.service("Extension death monitoring", ServiceStatus::Started);

    // Initialize extension event subscription
    // This must be after init_extensions() so extensions can subscribe to events
    state.init_extension_event_subscription().await;
    startup.service("Extension event subscription", ServiceStatus::Started);

    // Initialize AI Agent manager
    let _ = state.start_agent_manager().await;
    startup.service("AI Agent manager", ServiceStatus::Started);

    // Initialize AI Agent event listener
    state.init_agent_events().await;
    startup.service("AI Agent events", ServiceStatus::Started);

    // Detect llama.cpp backend capabilities from /props endpoint
    {
        tokio::spawn(async move {
            // Wait for instance manager to be ready
            let mut retry_interval = tokio::time::interval(Duration::from_secs(5));
            for _ in 0..12 {
                retry_interval.tick().await;
                if let Ok(instance_manager) = neomind_agent::get_instance_manager() {
                    instance_manager.detect_llamacpp_capabilities().await;
                    break;
                }
            }
        });
    }

    // Start memory scheduler — spawns a background retry task that polls
    // for LLM runtime availability. This ensures the scheduler starts even
    // when the LLM backend becomes ready after the server has started.
    {
        let agents_state = state.agents.clone();
        tokio::spawn(async move {
            let mut retry_interval = tokio::time::interval(Duration::from_secs(30));
            let mut attempts = 0u32;

            loop {
                retry_interval.tick().await;

                let Ok(instance_manager) = neomind_agent::get_instance_manager() else {
                    attempts += 1;
                    if attempts % 10 == 1 {
                        tracing::info!(
                            attempts = attempts,
                            "Waiting for LLM instance manager to initialize memory scheduler"
                        );
                    }
                    continue;
                };

                let Ok(runtime) = instance_manager.get_active_runtime().await else {
                    attempts += 1;
                    if attempts % 10 == 1 {
                        tracing::info!(
                            attempts = attempts,
                            "Waiting for LLM runtime to start memory scheduler"
                        );
                    }
                    continue;
                };

                match agents_state.start_memory_scheduler(runtime).await {
                    Ok(()) => {
                        tracing::info!(
                            category = "memory",
                            "Memory scheduler started (attempt {})",
                            attempts + 1
                        );
                        break; // Scheduler is running, no need to retry
                    }
                    Err(e) => {
                        tracing::warn!(
                            category = "memory",
                            error = %e,
                            "Failed to start memory scheduler, will retry"
                        );
                        attempts += 1;
                    }
                }
            }
        });
    }

    // Configuration phase
    startup.phase_config();

    // Initialize MQTT and register device types
    state.init_mqtt().await;

    // Services phase
    startup.phase_services();

    // Clone state for cleanup (move into shutdown task)
    let state_for_cleanup = state.clone();

    // Spawn rate limit cleanup task (runs every 5 minutes)
    let rate_limiter = state.rate_limiter.clone();
    tokio::spawn(async move {
        crate::rate_limit::cleanup_task(rate_limiter, Duration::from_secs(300)).await;
    });

    // P0: Spawn pending stream cleanup task (runs every 5 minutes)
    // Cleans up stale pending stream states that weren't properly cleared
    let state_for_cleanup_task = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            let session_store = state_for_cleanup_task
                .agents
                .session_manager
                .session_store();
            match session_store.cleanup_stale_pending_streams() {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!("Cleaned up {} stale pending stream states", count);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to cleanup stale pending streams: {}", e);
                }
            }
        }
    });

    let app = create_router_with_state(state);

    let listener = tokio::net::TcpListener::bind(bind).await?;

    // Ready phase
    startup.phase_ready();
    startup.ready_info(&bind.to_string());

    // Run with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(crate::shutdown::shutdown_signal())
        .await?;

    // Clean up resources after server shuts down
    crate::shutdown::cleanup_resources(&state_for_cleanup).await;

    tracing::info!("Server shutdown complete");
    Ok(())
}

/// Start the server with default configuration.
/// This function is designed to be called from Tauri or other embedded contexts.
/// It starts the server in the background and returns immediately.
///
/// Uses port 9375 by default to avoid conflicts with common applications.
/// Binds to 0.0.0.0 to allow LAN access.
/// Port can be configured via config.toml [server] section or NEOMIND_PORT env var.
pub async fn start_server() -> anyhow::Result<()> {
    let (host, port) = crate::config::get_server_config();
    let bind: SocketAddr = format!("{}:{}", host, port).parse()?;
    run(bind).await
}
