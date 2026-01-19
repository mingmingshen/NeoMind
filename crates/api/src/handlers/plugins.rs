//! Unified plugin management API handlers.
//!
//! This module provides REST API endpoints for managing plugins,
//! including listing, loading, enabling/disabling, and monitoring plugins.

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use super::{
    ServerState,
    common::{HandlerResult, ok},
};
use crate::models::ErrorResponse;

use edge_ai_core::plugin::{
    PluginInfo, PluginLoadOptions, PluginStats, PluginType, UnifiedPluginRegistry,
};

/// Plugin list query parameters.
#[derive(Debug, Deserialize)]
pub struct PluginListQuery {
    /// Filter by plugin type
    pub r#type: Option<String>,
    /// Filter by state
    pub state: Option<String>,
    /// Show only enabled plugins
    pub enabled: Option<bool>,
    /// Exclude built-in plugins (LLM backends, device adapters, internal MQTT)
    /// When false, returns only dynamically loaded .so/.wasm extension plugins
    pub builtin: Option<bool>,
}

/// Plugin registration request.
#[derive(Debug, Deserialize)]
pub struct RegisterPluginRequest {
    /// Plugin ID
    pub id: String,
    /// Plugin type
    pub plugin_type: String,
    /// Path to the plugin file (for native plugins)
    pub path: Option<String>,
    /// Plugin configuration
    pub config: Option<serde_json::Value>,
    /// Whether to auto-start after loading
    pub auto_start: Option<bool>,
    /// Whether the plugin is enabled
    pub enabled: Option<bool>,
}

/// Plugin configuration update request.
#[derive(Debug, Deserialize)]
pub struct UpdatePluginConfigRequest {
    /// New configuration
    pub config: serde_json::Value,
    /// Whether to reload the plugin after config update
    pub reload: Option<bool>,
}

/// Plugin command request.
#[derive(Debug, Deserialize)]
pub struct PluginCommandRequest {
    /// Command name
    pub command: String,
    /// Command arguments
    pub args: Option<serde_json::Value>,
}

/// Plugin DTO for API responses.
#[derive(Debug, Serialize)]
pub struct PluginDto {
    /// Plugin ID
    pub id: String,
    /// Plugin name
    pub name: String,
    /// Plugin type
    pub plugin_type: String,
    /// Plugin category (user-facing: ai, devices, notify)
    pub category: String,
    /// Plugin state
    pub state: String,
    /// Whether the plugin is enabled
    pub enabled: bool,
    /// Version
    pub version: String,
    /// Description
    pub description: String,
    /// Author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Required NeoTalk version
    pub required_version: String,
    /// Statistics
    pub stats: PluginStatsDto,
    /// Load timestamp
    pub loaded_at: DateTime<Utc>,
    /// Plugin path (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Whether the plugin is currently running (derived from state)
    #[serde(default)]
    pub running: bool,
    /// Number of devices managed by this plugin (for device adapters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_count: Option<usize>,
}

/// Plugin statistics DTO.
#[derive(Debug, Serialize, Default)]
pub struct PluginStatsDto {
    /// Number of times plugin was started
    pub start_count: u64,
    /// Number of times plugin was stopped
    pub stop_count: u64,
    /// Number of errors encountered
    pub error_count: u64,
    /// Total execution time in milliseconds
    pub total_execution_ms: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Last start time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_start_time: Option<DateTime<Utc>>,
    /// Last stop time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_stop_time: Option<DateTime<Utc>>,
}

impl From<PluginStats> for PluginStatsDto {
    fn from(stats: PluginStats) -> Self {
        Self {
            start_count: stats.start_count,
            stop_count: stats.stop_count,
            error_count: stats.error_count,
            total_execution_ms: stats.total_execution_ms,
            avg_response_time_ms: stats.avg_response_time_ms,
            last_start_time: stats.last_start_time,
            last_stop_time: stats.last_stop_time,
        }
    }
}

#[allow(deprecated)]
impl From<PluginInfo> for PluginDto {
    fn from(info: PluginInfo) -> Self {
        use edge_ai_core::plugin::types::PluginCategory;
        let is_running = matches!(info.state, edge_ai_core::plugin::PluginState::Running);
        Self {
            id: info.metadata.base.id.clone(),
            name: info.metadata.base.name.clone(),
            plugin_type: info.plugin_type.to_string(),
            category: PluginCategory::from_plugin_type(&info.plugin_type).to_string(),
            state: format!("{:?}", info.state),
            enabled: info.enabled,
            version: info.metadata.version.to_string(),
            description: info.metadata.base.description.clone(),
            author: info.metadata.base.author.clone(),
            required_version: info.metadata.required_neotalk_version.to_string(),
            stats: PluginStatsDto::from(info.stats.clone()),
            loaded_at: DateTime::from_timestamp(info.loaded_at, 0).unwrap_or_default(),
            path: info.path.as_ref().map(|p| p.to_string_lossy().to_string()),
            running: is_running,
            device_count: None,
        }
    }
}

/// Get the global plugin registry.
fn get_plugin_registry(state: &ServerState) -> Arc<UnifiedPluginRegistry> {
    state.plugin_registry.clone()
}

/// Convert device adapter to unified PluginDto.
fn adapter_to_plugin_dto(adapter: AdapterPluginDto) -> PluginDto {
    PluginDto {
        id: adapter.id.clone(),
        name: adapter.name.clone(),
        plugin_type: format!("device_adapter_{}", adapter.adapter_type),
        category: "devices".to_string(),
        state: if adapter.running {
            "Running".to_string()
        } else {
            "Stopped".to_string()
        },
        enabled: adapter.enabled,
        version: adapter.version,
        description: format!("{} device adapter", adapter.adapter_type.to_uppercase()),
        author: None,
        required_version: "1.0.0".to_string(),
        stats: PluginStatsDto::default(),
        loaded_at: DateTime::from_timestamp(adapter.last_activity, 0).unwrap_or_default(),
        path: None,
        running: adapter.running,
        device_count: Some(adapter.device_count),
    }
}

/// List all extension plugins (dynamically loaded .so/.wasm files).
///
/// This endpoint returns plugins based on the `builtin` query parameter:
/// - `builtin=true` (default): Returns all plugins including built-in ones
/// - `builtin=false`: Returns only dynamically loaded .so/.wasm extension plugins
///
/// Built-in system components (LLM backend, device connections, etc.) are managed
/// in their respective dedicated tabs in the UI.
///
/// GET /api/plugins
///
/// Query parameters:
/// - type: Filter by plugin type (llm_backend, storage_backend, etc.)
/// - state: Filter by state (Loaded, Initialized, Running, Stopped, etc.)
/// - enabled: Show only enabled plugins (true/false)
/// - builtin: Include built-in plugins (true=default, false=extension plugins only)
pub async fn list_plugins_handler(
    State(state): State<ServerState>,
    Query(query): Query<PluginListQuery>,
) -> HandlerResult<serde_json::Value> {
    
    

    let exclude_builtin = query.builtin == Some(false);

    let mut all_plugins: Vec<PluginDto> = Vec::new();

    // 1. Get plugins from UnifiedPluginRegistry
    let registry = get_plugin_registry(&state);
    let registry_plugins = registry.list().await;
    for plugin in registry_plugins {
        // Skip built-in LLM backend plugins when exclude_builtin is true
        if exclude_builtin && plugin.plugin_type == PluginType::LlmBackend {
            continue;
        }
        all_plugins.push(PluginDto::from(plugin));
    }

    // 2. Get device adapter information directly from DeviceService
    // Skip these entirely when exclude_builtin is true
    if !exclude_builtin {
        let adapter_stats = state.device_service.get_adapter_stats().await;
        for adapter in adapter_stats.adapters {
            all_plugins.push(adapter_to_plugin_dto(AdapterPluginDto {
                id: adapter.id,
                name: adapter.name,
                adapter_type: adapter.adapter_type,
                enabled: true,
                running: adapter.running,
                device_count: adapter.device_count,
                state: adapter.status,
                version: "1.0.0".to_string(),
                uptime_secs: None,
                last_activity: adapter.last_activity,
            }));
        }

        // 3. Add the internal MQTT device manager as a built-in plugin
        // The MqttAdapter is managed by DeviceService and registered as an adapter
        // Get connection status through the adapter
        use edge_ai_devices::adapter::ConnectionStatus;
        let is_connected =
            if let Some(adapter) = state.device_service.get_adapter("internal-mqtt").await {
                let status = adapter.connection_status();
                matches!(status, ConnectionStatus::Connected)
            } else {
                false
            };

        // Use DeviceService to get device count
        let configs = state.device_service.list_devices().await;
        let device_count = configs.len();

        all_plugins.push(PluginDto {
            id: "internal-mqtt".to_string(),
            name: "内置 MQTT".to_string(),
            plugin_type: "device_adapter_mqtt".to_string(),
            category: "devices".to_string(),
            state: if is_connected {
                "Running".to_string()
            } else {
                "Stopped".to_string()
            },
            enabled: true,
            version: "1.0.0".to_string(),
            description: "内置 MQTT 代理，用于连接和管理 MQTT 设备".to_string(),
            author: Some("NeoTalk".to_string()),
            required_version: "1.0.0".to_string(),
            stats: PluginStatsDto {
                start_count: 1,
                stop_count: 0,
                error_count: 0,
                total_execution_ms: 0,
                avg_response_time_ms: 0.0,
                last_start_time: None,
                last_stop_time: None,
            },
            loaded_at: chrono::Utc::now(),
            path: None,
            running: is_connected,
            device_count: Some(device_count),
        });
    }

    // 4. Apply filters
    if let Some(type_filter) = &query.r#type {
        all_plugins.retain(|p| {
            // Match exact plugin_type or match the base type (e.g., device_adapter_* matches device_adapter)
            p.plugin_type == *type_filter || p.plugin_type.starts_with(&format!("{}_", type_filter))
        });
    }

    if let Some(state_filter) = &query.state {
        all_plugins.retain(|p| p.state == *state_filter);
    }

    if let Some(enabled_only) = query.enabled {
        all_plugins.retain(|p| p.enabled == enabled_only);
    }

    ok(json!({
        "plugins": all_plugins,
        "count": all_plugins.len(),
    }))
}

/// Get plugin by ID.
///
/// GET /api/plugins/:id
pub async fn get_plugin_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    let info = registry
        .get_info(&id)
        .await
        .ok_or_else(|| ErrorResponse::not_found(format!("Plugin {}", id)))?;

    let stats = registry.get_stats(&id).await.unwrap_or_default();

    let mut dto = PluginDto::from(info);
    dto.stats = PluginStatsDto::from(stats);

    ok(json!({
        "plugin": dto,
    }))
}

/// Register a new extension plugin from a plugin file.
///
/// POST /api/plugins
///
/// This endpoint loads external plugins from plugin files:
/// - Native plugins: .so (Linux), .dylib (macOS), .dll (Windows)
/// - WASM plugins: .wasm file (future support)
///
/// Note: Built-in system components (LLM backend, device connections) are
/// managed in their respective dedicated tabs, not through this endpoint.
pub async fn register_plugin_handler(
    State(state): State<ServerState>,
    Json(req): Json<RegisterPluginRequest>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    // Parse plugin type
    let _plugin_type = PluginType::from_str(&req.plugin_type);

    // Build load options
    let options = PluginLoadOptions {
        auto_start: req.auto_start.unwrap_or(false),
        config: req.config.clone(),
        enabled: req.enabled.unwrap_or(true),
        timeout_secs: None,
    };

    // If path is provided, load the plugin from file
    if let Some(path) = req.path {
        let path_buf = std::path::PathBuf::from(&path);

        if !path_buf.exists() {
            return Err(ErrorResponse::not_found(format!("Plugin file {}", path)));
        }

        // Load based on file extension
        let plugin_id = match path_buf.extension().and_then(|e| e.to_str()) {
            Some("wasm") => {
                return Err(ErrorResponse::bad_request(
                    "WASM plugin loading not yet implemented",
                ));
            }
            Some("so") | Some("dylib") | Some("dll") => {
                registry.load_native_plugin(&path_buf).await.map_err(|e| {
                    ErrorResponse::internal(format!("Failed to load native plugin: {}", e))
                })?
            }
            _ => {
                return Err(ErrorResponse::bad_request(
                    "Unknown plugin file type. Expected .wasm, .so, .dylib, or .dll",
                ));
            }
        };

        // Initialize if config provided
        if let Some(config) = options.config
            && let Err(e) = registry.initialize(&plugin_id, &config).await {
                tracing::warn!("Failed to initialize plugin {}: {}", plugin_id, e);
            }

        // Start if auto-start is enabled
        if options.auto_start
            && let Err(e) = registry.start(&plugin_id).await {
                tracing::warn!("Failed to start plugin {}: {}", plugin_id, e);
            }

        ok(json!({
            "message": "Plugin registered successfully",
            "plugin_id": plugin_id,
        }))
    } else {
        // Register a built-in plugin by ID
        Err(ErrorResponse::bad_request(
            "Built-in plugin registration not yet implemented. Please provide a plugin file path.",
        ))
    }
}

/// Unregister a plugin.
///
/// DELETE /api/plugins/:id
pub async fn unregister_plugin_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    registry
        .unregister(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to unregister plugin: {}", e)))?;

    ok(json!({
        "message": format!("Plugin {} unregistered", id),
    }))
}

/// Enable a plugin.
///
/// POST /api/plugins/:id/enable
pub async fn enable_plugin_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    registry
        .enable(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to enable plugin: {}", e)))?;

    ok(json!({
        "message": format!("Plugin {} enabled", id),
    }))
}

/// Disable a plugin.
///
/// POST /api/plugins/:id/disable
pub async fn disable_plugin_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    registry
        .disable(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to disable plugin: {}", e)))?;

    ok(json!({
        "message": format!("Plugin {} disabled", id),
    }))
}

/// Start a plugin.
///
/// POST /api/plugins/:id/start
pub async fn start_plugin_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    registry
        .start(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to start plugin: {}", e)))?;

    ok(json!({
        "message": format!("Plugin {} started", id),
    }))
}

/// Stop a plugin.
///
/// POST /api/plugins/:id/stop
pub async fn stop_plugin_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    registry
        .stop(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to stop plugin: {}", e)))?;

    ok(json!({
        "message": format!("Plugin {} stopped", id),
    }))
}

/// Health check for a plugin.
///
/// GET /api/plugins/:id/health
pub async fn plugin_health_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    registry.health_check(&id).await.map_err(|e| {
        ErrorResponse::service_unavailable(format!("Plugin {} unhealthy: {}", id, e))
    })?;

    // Get plugin info
    let info = registry
        .get_info(&id)
        .await
        .ok_or_else(|| ErrorResponse::not_found(format!("Plugin {}", id)))?;

    ok(json!({
        "status": "healthy",
        "plugin_id": id,
        "state": format!("{:?}", info.state),
    }))
}

/// Get plugin configuration.
///
/// GET /api/plugins/:id/config
pub async fn get_plugin_config_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    let info = registry
        .get_info(&id)
        .await
        .ok_or_else(|| ErrorResponse::not_found(format!("Plugin {}", id)))?;

    ok(json!({
        "plugin_id": id,
        "config": info.metadata.config_schema,
    }))
}

/// Update plugin configuration.
///
/// PUT /api/plugins/:id/config
pub async fn update_plugin_config_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePluginConfigRequest>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    // Update config by re-initializing with new config
    registry
        .initialize(&id, &req.config)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to update config: {}", e)))?;

    // Reload if requested
    if req.reload.unwrap_or(false) {
        registry
            .reload(&id)
            .await
            .map_err(|e| ErrorResponse::internal(format!("Failed to reload plugin: {}", e)))?;
    }

    ok(json!({
        "message": format!("Plugin {} configuration updated", id),
    }))
}

/// Execute a plugin command.
///
/// POST /api/plugins/:id/command
pub async fn execute_plugin_command_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(req): Json<PluginCommandRequest>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    let args = req.args.unwrap_or_else(|| json!({}));
    let result = registry
        .execute_command(&id, &req.command, &args)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Command execution failed: {}", e)))?;

    ok(json!({
        "result": result,
    }))
}

/// Get plugin statistics.
///
/// GET /api/plugins/:id/stats
pub async fn get_plugin_stats_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    let stats = registry
        .get_stats(&id)
        .await
        .ok_or_else(|| ErrorResponse::not_found(format!("Plugin {}", id)))?;

    ok(json!({
        "plugin_id": id,
        "stats": stats,
    }))
}

/// Discover and load plugins from search paths.
///
/// POST /api/plugins/discover
pub async fn discover_plugins_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);

    let count = registry
        .discover_native_plugins()
        .await
        .map_err(|e| ErrorResponse::internal(format!("Plugin discovery failed: {}", e)))?;

    ok(json!({
        "message": format!("Discovered and loaded {} plugins", count),
        "count": count,
    }))
}

/// List plugins by type.
///
/// GET /api/plugins/type/:type
pub async fn list_plugins_by_type_handler(
    State(state): State<ServerState>,
    Path(plugin_type): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);
    let plugin_type_parsed = PluginType::from_str(&plugin_type);

    let plugins = registry.list_by_type(plugin_type_parsed).await;
    let dtos: Vec<PluginDto> = plugins.into_iter().map(PluginDto::from).collect();

    ok(json!({
        "plugin_type": plugin_type,
        "plugins": dtos,
        "count": dtos.len(),
    }))
}

/// Get plugin types summary.
///
/// GET /api/plugins/types
pub async fn get_plugin_types_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let registry = get_plugin_registry(&state);
    let plugins = registry.list().await;

    let mut summary: HashMap<String, usize> = HashMap::new();

    for plugin in &plugins {
        let key = plugin.plugin_type.to_string();
        *summary.entry(key).or_insert(0) += 1;
    }

    ok(json!({
        "types": summary,
        "total": plugins.len(),
    }))
}

// ============================================================================
// Device Adapter Plugin Endpoints
// ============================================================================

/// Response for device adapter plugins.
#[derive(Debug, Serialize)]
pub struct DeviceAdapterPluginsResponse {
    /// Total number of device adapter plugins
    pub total_adapters: usize,
    /// Number of running adapters
    pub running_adapters: usize,
    /// Total number of devices across all adapters
    pub total_devices: usize,
    /// Adapter plugin list
    pub adapters: Vec<AdapterPluginDto>,
}

/// Adapter plugin DTO with device information.
#[derive(Debug, Serialize)]
pub struct AdapterPluginDto {
    /// Plugin ID
    pub id: String,
    /// Plugin name
    pub name: String,
    /// Adapter type (mqtt, modbus, hass, etc.)
    pub adapter_type: String,
    /// Whether the plugin is enabled
    pub enabled: bool,
    /// Whether the plugin is running
    pub running: bool,
    /// Number of devices managed
    pub device_count: usize,
    /// Plugin state
    pub state: String,
    /// Version
    pub version: String,
    /// Uptime in seconds
    pub uptime_secs: Option<u64>,
    /// Last activity timestamp
    pub last_activity: i64,
}

/// Response for devices managed by an adapter plugin.
#[derive(Debug, Serialize)]
pub struct AdapterDevicesResponse {
    /// Plugin ID
    pub plugin_id: String,
    /// Plugin name
    pub plugin_name: String,
    /// Device list
    pub devices: Vec<AdapterDeviceDto>,
    /// Device count
    pub count: usize,
}

/// Device DTO managed by an adapter.
#[derive(Debug, Serialize)]
pub struct AdapterDeviceDto {
    /// Device ID
    pub id: String,
    /// Device name (if available)
    pub name: Option<String>,
    /// Device type
    pub device_type: String,
    /// Status
    pub status: String,
    /// Last seen timestamp
    pub last_seen: i64,
}

impl From<edge_ai_devices::AdapterPluginInfo> for AdapterPluginDto {
    fn from(info: edge_ai_devices::AdapterPluginInfo) -> Self {
        Self {
            id: info.id,
            name: info.name,
            adapter_type: info.adapter_type,
            enabled: info.enabled,
            running: info.running,
            device_count: info.device_count,
            state: info.state,
            version: info.version,
            uptime_secs: info.uptime_secs,
            last_activity: info.last_activity,
        }
    }
}

/// Get all device adapter plugins.
///
/// GET /api/plugins/device-adapters
pub async fn list_device_adapter_plugins_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let stats = state.device_service.get_adapter_stats().await;

    let adapters: Vec<AdapterPluginDto> = stats
        .adapters
        .into_iter()
        .map(|adapter| AdapterPluginDto {
            id: adapter.id,
            name: adapter.name,
            adapter_type: adapter.adapter_type,
            enabled: true,
            running: adapter.running,
            device_count: adapter.device_count,
            state: adapter.status,
            version: "1.0.0".to_string(),
            uptime_secs: None,
            last_activity: adapter.last_activity,
        })
        .collect();

    ok(json!({
        "total_adapters": stats.total_adapters,
        "running_adapters": stats.running_adapters,
        "total_devices": stats.total_devices,
        "adapters": adapters,
    }))
}

/// Register a new device adapter plugin.
///
/// POST /api/plugins/device-adapters
pub async fn register_device_adapter_handler(
    State(state): State<ServerState>,
    Json(req): Json<serde_json::Value>,
) -> HandlerResult<serde_json::Value> {
    use edge_ai_devices::adapters::create_adapter;

    // Parse request
    let id = req
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorResponse::bad_request("Missing 'id' field"))?;

    let adapter_type = req
        .get("adapter_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorResponse::bad_request("Missing 'adapter_type' field"))?;

    let config = req
        .get("config")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let auto_start = req
        .get("auto_start")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Get event bus for adapter creation
    let event_bus = state.event_bus.as_ref().ok_or_else(|| {
        ErrorResponse::internal("Event bus not initialized")
    })?;

    // Create the adapter using the factory
    let adapter = create_adapter(adapter_type, &config, event_bus)
        .map_err(|e| ErrorResponse::internal(format!("Failed to create adapter: {}", e)))?;

    // Register with DeviceService
    state
        .device_service
        .register_adapter(id.to_string(), adapter.clone())
        .await;

    // Auto-start if requested
    if auto_start {
        if let Err(e) = adapter.start().await {
            tracing::warn!("Failed to auto-start adapter {}: {}", id, e);
        }
    }

    ok(json!({
        "message": "Device adapter registered successfully",
        "adapter_id": id,
    }))
}

/// Get devices managed by a device adapter plugin.
///
/// GET /api/plugins/:id/devices
pub async fn get_adapter_devices_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let devices = state
        .device_service
        .get_adapter_device_ids(&id)
        .await
        .ok_or_else(|| ErrorResponse::not_found(format!("Adapter {}", id)))?;

    let device_dtos: Vec<AdapterDeviceDto> = devices
        .into_iter()
        .map(|device_id| AdapterDeviceDto {
            id: device_id.clone(),
            name: None,
            device_type: "unknown".to_string(),
            status: "online".to_string(),
            last_seen: chrono::Utc::now().timestamp(),
        })
        .collect();

    ok(json!({
        "adapter_id": id,
        "devices": device_dtos,
        "count": device_dtos.len(),
    }))
}

/// Get device adapter plugin statistics.
///
/// GET /api/plugins/device-adapters/stats
pub async fn get_device_adapter_stats_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let stats = state.device_service.get_adapter_stats().await;

    ok(json!({
        "total_adapters": stats.total_adapters,
        "running_adapters": stats.running_adapters,
        "total_devices": stats.total_devices,
        "adapters": stats.adapters,
    }))
}

/// Adapter type definition for frontend.
#[derive(Debug, Serialize)]
pub struct AdapterTypeDto {
    /// Type ID (e.g., "mqtt", "http", "webhook")
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Icon name (for frontend)
    pub icon: String,
    /// Icon background color class
    pub icon_bg: String,
    /// Connection mode (push/pull/hybrid)
    pub mode: String,
    /// Whether multiple instances can be created
    pub can_add_multiple: bool,
    /// Whether this is a built-in adapter
    pub builtin: bool,
}

/// Get available device adapter types.
///
/// Similar to /llm-backends/types, this returns the available adapter types
/// that can be used to create new connections.
///
/// GET /api/device-adapters/types
pub async fn list_adapter_types_handler(
    State(_state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    use edge_ai_devices::adapters::available_adapters;

    let available = available_adapters();

    // Define adapter type metadata (this could be moved to a config file in the future)
    let type_info: Vec<AdapterTypeDto> = available
        .into_iter()
        .map(|adapter_type| match adapter_type {
            "mqtt" => AdapterTypeDto {
                id: "mqtt".to_string(),
                name: "MQTT".to_string(),
                description: "MQTT broker connections (built-in + external)".to_string(),
                icon: "Server".to_string(),
                icon_bg: "bg-blue-100 text-blue-700 dark:bg-blue-900/20 dark:text-blue-400".to_string(),
                mode: "push".to_string(),
                can_add_multiple: true,
                builtin: true,
            },
            "http" => AdapterTypeDto {
                id: "http".to_string(),
                name: "HTTP (Polling)".to_string(),
                description: "Poll data from device REST APIs on a schedule".to_string(),
                icon: "Radio".to_string(),
                icon_bg: "bg-orange-100 text-orange-700 dark:bg-orange-900/20 dark:text-orange-400".to_string(),
                mode: "pull".to_string(),
                can_add_multiple: true,
                builtin: true,
            },
            "webhook" => AdapterTypeDto {
                id: "webhook".to_string(),
                name: "Webhook".to_string(),
                description: "Devices push data via HTTP POST to your server".to_string(),
                icon: "Webhook".to_string(),
                icon_bg: "bg-green-100 text-green-700 dark:bg-green-900/20 dark:text-green-400".to_string(),
                mode: "push".to_string(),
                can_add_multiple: false,
                builtin: true,
            },
            _ => AdapterTypeDto {
                id: adapter_type.to_string(),
                name: adapter_type.to_uppercase(),
                description: format!("{} device adapter", adapter_type),
                icon: "Server".to_string(),
                icon_bg: "bg-gray-100 text-gray-700 dark:bg-gray-900/20 dark:text-gray-400".to_string(),
                mode: "unknown".to_string(),
                can_add_multiple: true,
                builtin: true,
            },
        })
        .collect();

    ok(json!({
        "types": type_info,
        "count": type_info.len(),
    }))
}
