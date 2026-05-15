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
use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use neomind_storage::ExtensionStore;
use tower_http::timeout::{RequestBodyTimeoutLayer, TimeoutLayer};

/// Start the web server on a specific address.
/// This is the main entry point for running the server.
pub async fn run(bind: SocketAddr) -> anyhow::Result<()> {
    use crate::startup::{ServiceStatus, StartupLogger};

    // Note: V2 extension system doesn't require panic hook installation
    // The V2 system uses safer FFI boundaries directly

    let mut startup = StartupLogger::new();
    startup.banner();

    // ── Phase A: Core init + HTTP listener (fast path to serving) ──

    let t_start = std::time::Instant::now();
    let state = ServerState::new().await;
    tracing::info!(
        elapsed_ms = t_start.elapsed().as_millis() as u64,
        "ServerState::new() completed"
    );

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

    // Configuration phase
    startup.phase_config();

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

    let app = create_router_with_state(state.clone());

    let app = app
        .layer(RequestBodyTimeoutLayer::new(Duration::from_secs(20)))
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(30)));

    let listener = tokio::net::TcpListener::bind(bind).await?;

    // Ready phase — HTTP listener is bound
    startup.phase_ready();
    startup.ready_info(&bind.to_string());

    tracing::info!(
        elapsed_ms = t_start.elapsed().as_millis() as u64,
        "HTTP listener ready (time to serve)"
    );

    // ── Phase B: Deferred background services (after listener starts serving) ──
    // These run in the background and do not block HTTP serving.

    // Initialize extension metrics collector (decoupled from device system)
    let runtime = state.extensions.runtime.clone();
    let metrics_storage = state.extensions.metrics_storage.clone();
    let event_bus_for_metrics = state.core.event_bus.clone();
    tokio::spawn(async move {
        use crate::server::extension_metrics::ExtensionMetricsCollector;
        use std::time::Duration;

        let mut collector = ExtensionMetricsCollector::new(runtime, metrics_storage)
            .with_interval(Duration::from_secs(60));

        if let Some(bus) = event_bus_for_metrics {
            collector = collector.with_event_bus(bus);
        }

        collector.run().await;
    });

    // Start telemetry retention cleanup background task
    {
        tokio::spawn(async move {
            use neomind_storage::{SettingsStore, TimeSeriesStore};

            // Wait for server to initialize
            tokio::time::sleep(Duration::from_secs(10)).await;

            const SETTINGS_DB_PATH: &str = "data/settings.redb";
            const TELEMETRY_DB_PATH: &str = "data/telemetry.redb";

            loop {
                // Load config on each cycle so runtime changes take effect
                let config = SettingsStore::open(SETTINGS_DB_PATH)
                    .map(|s| s.get_retention_config())
                    .unwrap_or_default();

                let interval_secs = config.interval_hours * 3600;

                if config.enabled {
                    let policy = config.to_retention_policy();
                    if let Ok(ts_store) = TimeSeriesStore::open(TELEMETRY_DB_PATH) {
                        ts_store.set_retention_policy(policy).await;
                        match ts_store.apply_retention().await {
                            Ok(result) => {
                                if result.points_removed > 0 {
                                    tracing::info!(
                                        points_removed = result.points_removed,
                                        metrics_cleaned = result.metrics_cleaned.len(),
                                        "Retention cleanup completed"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Retention cleanup failed");
                            }
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            }
        });
    }

    // Heavy background services — extension loading, agent manager, MQTT
    {
        let bg_state = state.clone();
        tokio::spawn(async move {
            let t_bg = std::time::Instant::now();

            // Kill orphaned extension runner processes from a previous session.
            // MUST run before init_extensions() to avoid killing newly spawned runners.
            // Orphaned runners hold dylib files open and cause dlopen() hangs.
            neomind_core::extension::isolated::IsolatedExtensionManager::cleanup_orphaned_runners();
            tracing::info!(
                elapsed_ms = t_bg.elapsed().as_millis() as u64,
                "Extension orphan cleanup done"
            );

            // Initialize extensions from persistent storage
            bg_state.init_extensions().await;

            // Refresh tool registry now that extensions are loaded
            bg_state.refresh_extension_tools().await;

            // Start extension death monitoring for auto-restart
            {
                let runtime = bg_state.extensions.runtime.clone();
                bg_state.extensions.runtime.set_on_crash_recovery_restart(Arc::new(
                    move |extension_id: &str, _path: &std::path::Path| {
                        let ext_id = extension_id.to_string();
                        let rt = runtime.clone();
                        tokio::spawn(async move {
                            if let Ok(store) = ExtensionStore::open("data/extensions.redb") {
                                if let Ok(Some(record)) = store.load(&ext_id) {
                                    if let Some(ref config) = record.config {
                                        tracing::info!(
                                            extension_id = %ext_id,
                                            "Applying saved config to extension after crash recovery"
                                        );
                                        if let Err(e) = rt
                                            .execute_command(&ext_id, "configure", config)
                                            .await
                                        {
                                            tracing::warn!(
                                                extension_id = %ext_id,
                                                error = %e,
                                                "Failed to apply saved config after crash recovery"
                                            );
                                        }
                                    }
                                }
                            }
                        });
                    },
                ));
            }
            bg_state.extensions.runtime.clone().start_death_monitoring();

            // Initialize extension event subscription
            bg_state.init_extension_event_subscription().await;

            // Initialize AI Agent manager
            let _ = bg_state.start_agent_manager().await;

            // Initialize AI Agent event listener
            bg_state.init_agent_events().await;

            // Detect llama.cpp backend capabilities from /props endpoint
            {
                let mut retry_interval = tokio::time::interval(Duration::from_secs(5));
                for _ in 0..12 {
                    retry_interval.tick().await;
                    if let Ok(instance_manager) = neomind_agent::get_instance_manager() {
                        instance_manager.detect_llamacpp_capabilities().await;
                        break;
                    }
                }
            }

            // Start memory scheduler
            {
                let agents_state = bg_state.agents.clone();
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
                                break;
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

            // Initialize MQTT and register device types
            bg_state.init_mqtt().await;

            tracing::info!(
                elapsed_ms = t_bg.elapsed().as_millis() as u64,
                "Background services init complete"
            );
        });
    }

    // Services phase
    startup.phase_services();

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
