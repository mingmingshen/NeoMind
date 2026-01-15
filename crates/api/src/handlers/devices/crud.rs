//! Device CRUD operations.

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::json;
use uuid::Uuid;

use super::compat::{config_to_device_instance, format_status_to_str};
use super::models::{
    AddDeviceRequest, DeviceDto, PaginationMeta, PaginationQuery, UpdateDeviceRequest,
};
use crate::handlers::{
    ServerState,
    common::{HandlerResult, ok},
};
use crate::models::ErrorResponse;
use edge_ai_devices::{
    adapter::ConnectionStatus as AdapterConnectionStatus,
    mdl::ConnectionStatus as MdlConnectionStatus, service::DeviceStatus,
};

/// Convert AdapterConnectionStatus to MdlConnectionStatus
fn convert_status(status: AdapterConnectionStatus) -> MdlConnectionStatus {
    match status {
        AdapterConnectionStatus::Connected => MdlConnectionStatus::Connected,
        AdapterConnectionStatus::Connecting => MdlConnectionStatus::Connecting,
        AdapterConnectionStatus::Disconnected => MdlConnectionStatus::Disconnected,
        AdapterConnectionStatus::Reconnecting => MdlConnectionStatus::Reconnecting,
        AdapterConnectionStatus::Error => MdlConnectionStatus::Error,
    }
}

/// Map adapter_id to plugin display name
fn get_plugin_info(adapter_id: &Option<String>) -> (Option<String>, Option<String>) {
    match adapter_id {
        None => (
            Some("internal-mqtt".to_string()),
            Some("内置MQTT".to_string()),
        ),
        Some(id) if id.starts_with("hass") => {
            (Some(id.clone()), Some("Home Assistant".to_string()))
        }
        Some(id) if id.starts_with("modbus") => (Some(id.clone()), Some(format!("Modbus: {}", id))),
        Some(id) if id.starts_with("external-mqtt") => {
            (Some(id.clone()), Some(format!("外部MQTT: {}", id)))
        }
        Some(id) => (Some(id.clone()), Some(id.clone())),
    }
}

/// List devices with pagination and filtering support.
/// Uses new DeviceService with real device status from event tracking
pub async fn list_devices_handler(
    State(state): State<ServerState>,
    Query(pagination): Query<PaginationQuery>,
) -> HandlerResult<serde_json::Value> {
    // Parse pagination parameters
    let page = pagination.page.unwrap_or(1).max(1);
    let limit = pagination.limit.unwrap_or(50).min(1000); // Cap at 1000 items per page
    let offset = (page - 1) * limit;

    // Get all devices
    let configs = state.device_service.list_devices().await;
    let total = configs.len();

    // Apply filters if provided
    let mut filtered_configs = Vec::new();
    for config in configs {
        // Filter by device_type
        if let Some(ref filter_type) = pagination.device_type {
            if &config.device_type != filter_type {
                continue;
            }
        }

        // Filter by status
        if let Some(ref filter_status) = pagination.status {
            let device_status = state
                .device_service
                .get_device_status(&config.device_id)
                .await;
            let status_str = match device_status.status {
                AdapterConnectionStatus::Connected => "connected",
                AdapterConnectionStatus::Disconnected => "disconnected",
                AdapterConnectionStatus::Connecting => "connecting",
                AdapterConnectionStatus::Reconnecting => "reconnecting",
                AdapterConnectionStatus::Error => "error",
            };
            if status_str != filter_status {
                continue;
            }
        }

        filtered_configs.push(config);
    }

    let filtered_total = filtered_configs.len();

    // Apply pagination
    let paginated_configs: Vec<_> = filtered_configs
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    // Convert to DTOs
    let mut dtos = Vec::new();
    for config in paginated_configs {
        let (plugin_id, plugin_name) = get_plugin_info(&config.adapter_id);

        // Get real device status from DeviceService
        let device_status = state
            .device_service
            .get_device_status(&config.device_id)
            .await;
        let status = convert_status(device_status.status);
        let last_seen = chrono::DateTime::from_timestamp(device_status.last_seen, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        let online = matches!(device_status.status, AdapterConnectionStatus::Connected);

        let instance = config_to_device_instance(&config, status, last_seen);

        // Get template info for metric/command counts
        let template = state.device_service.get_template(&config.device_type).await;
        let metric_count = template.as_ref().map(|t| t.metrics.len());
        let command_count = template.as_ref().map(|t| t.commands.len());

        dtos.push(DeviceDto {
            id: config.device_id.clone(),
            device_id: config.device_id.clone(),
            name: instance
                .name
                .clone()
                .unwrap_or_else(|| config.device_id.clone()),
            device_type: config.device_type.clone(),
            adapter_type: config.adapter_type.clone(),
            status: format_status_to_str(&instance.status).to_string(),
            last_seen: instance.last_seen.to_rfc3339(),
            online,
            plugin_id,
            plugin_name,
            adapter_id: config.adapter_id.clone(),
            metric_count,
            command_count,
            current_values: None, // Skip for list view to reduce payload
            config: Some(instance.config),
        });
    }

    // Build pagination metadata
    let pagination_meta = PaginationMeta::new(page, limit, filtered_total);

    ok(json!({
        "devices": dtos,
        "count": dtos.len(),
        "pagination": pagination_meta,
    }))
}

/// Get device details.
/// Uses new DeviceService with real device status from event tracking
pub async fn get_device_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    // Use new DeviceService
    let (config, template) = state
        .device_service
        .get_device_with_template(&device_id)
        .await
        .map_err(|_| ErrorResponse::not_found("Device"))?;

    let metric_count = template.metrics.len();
    let command_count = template.commands.len();

    // Get current metric values (if available)
    let current_values = state
        .device_service
        .get_current_metrics(&device_id)
        .await
        .unwrap_or_default();
    let current_values_json: std::collections::HashMap<String, serde_json::Value> = current_values
        .into_iter()
        .map(|(k, v)| (k, super::metrics::value_to_json(&v)))
        .collect();

    // Get real device status from DeviceService
    let device_status = state.device_service.get_device_status(&device_id).await;
    let status = convert_status(device_status.status);
    let last_seen = chrono::DateTime::from_timestamp(device_status.last_seen, 0)
        .unwrap_or_else(|| chrono::Utc::now());
    let instance = config_to_device_instance(&config, status, last_seen);

    // Get plugin info for display
    let (plugin_id, plugin_name) = get_plugin_info(&config.adapter_id);

    // Determine online status based on connection status
    let online = matches!(device_status.status, AdapterConnectionStatus::Connected);

    ok(json!({
        "id": config.device_id,
        "device_id": config.device_id,
        "name": config.name,
        "device_type": config.device_type,
        "adapter_type": config.adapter_type,
        "connection_config": config.connection_config,
        "status": format_status_to_str(&instance.status),
        "last_seen": instance.last_seen.to_rfc3339(),
        "online": online,
        "metric_count": metric_count,
        "command_count": command_count,
        "current_values": current_values_json,
        "config": instance.config,
        "plugin_id": plugin_id,
        "plugin_name": plugin_name,
        "adapter_id": config.adapter_id,
    }))
}

/// Delete a device.
/// Uses new DeviceService
pub async fn delete_device_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    state
        .device_service
        .unregister_device(&device_id)
        .await
        .map_err(|e| ErrorResponse::internal(&format!("Failed to delete device: {}", e)))?;
    ok(json!({
        "device_id": device_id,
        "deleted": true,
    }))
}

/// Add a new device manually.
/// Uses new DeviceService
pub async fn add_device_handler(
    State(state): State<ServerState>,
    Json(req): Json<AddDeviceRequest>,
) -> HandlerResult<serde_json::Value> {
    // Generate device ID if not provided: {device_type}_{random_8_chars}
    let device_id = if let Some(id) = req.device_id {
        id
    } else {
        // Generate random 8 character string
        let random_str: String = Uuid::new_v4()
            .to_string()
            .replace('-', "")
            .chars()
            .take(8)
            .collect();
        format!("{}_{}", req.device_type, random_str)
    };

    // Parse connection_config JSON into ConnectionConfig
    let connection_config: edge_ai_devices::ConnectionConfig =
        serde_json::from_value(req.connection_config).map_err(|e| {
            ErrorResponse::bad_request(&format!("Invalid connection_config: {}", e))
        })?;

    // Create DeviceConfig
    let config = edge_ai_devices::DeviceConfig {
        device_id: device_id.clone(),
        name: req.name,
        device_type: req.device_type,
        adapter_type: req.adapter_type,
        connection_config,
        adapter_id: None, // Will be set by adapter when registered
    };

    // Register device using new DeviceService
    state
        .device_service
        .register_device(config)
        .await
        .map_err(|e| ErrorResponse::internal(&format!("Failed to add device: {}", e)))?;

    ok(json!({
        "device_id": device_id,
        "added": true,
    }))
}

/// Update a device.
/// Only updates the fields provided in the request.
pub async fn update_device_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
    Json(req): Json<UpdateDeviceRequest>,
) -> HandlerResult<serde_json::Value> {
    // Get existing device config
    let existing = state
        .device_service
        .get_device(&device_id)
        .await
        .ok_or_else(|| ErrorResponse::not_found("Device"))?;

    // Parse connection_config if provided
    let connection_config = if let Some(config_json) = req.connection_config {
        serde_json::from_value(config_json)
            .map_err(|e| ErrorResponse::bad_request(&format!("Invalid connection_config: {}", e)))?
    } else {
        existing.connection_config
    };

    // Create updated config
    let config = edge_ai_devices::DeviceConfig {
        device_id: device_id.clone(),
        name: req.name.unwrap_or(existing.name),
        device_type: existing.device_type.clone(),
        adapter_type: req.adapter_type.unwrap_or(existing.adapter_type),
        connection_config,
        adapter_id: req.adapter_id.or(existing.adapter_id),
    };

    // Update device using new DeviceService
    state
        .device_service
        .update_device(&device_id, config)
        .await
        .map_err(|e| ErrorResponse::internal(&format!("Failed to update device: {}", e)))?;

    ok(json!({
        "device_id": device_id,
        "updated": true,
    }))
}

// ========== Device State Management APIs ==========

/// Get device state.
/// GET /api/devices/:id/state
pub async fn get_device_state_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    // Check device exists
    let _config = state
        .device_service
        .get_device(&device_id)
        .await
        .ok_or_else(|| ErrorResponse::not_found("Device"))?;

    // Get device status
    let device_status = state.device_service.get_device_status(&device_id).await;

    ok(json!({
        "device_id": device_id,
        "status": match device_status.status {
            AdapterConnectionStatus::Connected => "connected",
            AdapterConnectionStatus::Disconnected => "disconnected",
            AdapterConnectionStatus::Connecting => "connecting",
            AdapterConnectionStatus::Reconnecting => "reconnecting",
            AdapterConnectionStatus::Error => "error",
        },
        "last_seen": device_status.last_seen,
        "adapter_id": device_status.adapter_id,
        "is_connected": device_status.is_connected(),
    }))
}

/// Get device health status.
/// GET /api/devices/:id/health
pub async fn get_device_health_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    // Check device exists
    let _config = state
        .device_service
        .get_device(&device_id)
        .await
        .ok_or_else(|| ErrorResponse::not_found("Device"))?;

    let device_status = state.device_service.get_device_status(&device_id).await;
    let now = chrono::Utc::now().timestamp();
    let seconds_since_last_seen = now - device_status.last_seen;

    // Determine health status
    let (health, message) = if device_status.is_connected() {
        if seconds_since_last_seen < 60 {
            ("healthy", "Device is connected and actively reporting")
        } else if seconds_since_last_seen < 300 {
            ("stale", "Device is connected but hasn't reported recently")
        } else {
            (
                "warning",
                "Device is connected but hasn't reported for over 5 minutes",
            )
        }
    } else {
        ("unhealthy", "Device is not connected")
    };

    ok(json!({
        "device_id": device_id,
        "health": health,
        "message": message,
        "status": match device_status.status {
            AdapterConnectionStatus::Connected => "connected",
            AdapterConnectionStatus::Disconnected => "disconnected",
            AdapterConnectionStatus::Connecting => "connecting",
            AdapterConnectionStatus::Reconnecting => "reconnecting",
            AdapterConnectionStatus::Error => "error",
        },
        "last_seen": device_status.last_seen,
        "seconds_since_last_seen": seconds_since_last_seen,
    }))
}

/// Force device refresh (poll for current state).
/// POST /api/devices/:id/refresh
pub async fn refresh_device_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    // Check device exists
    let config = state
        .device_service
        .get_device(&device_id)
        .await
        .ok_or_else(|| ErrorResponse::not_found("Device"))?;

    // Try to get adapter and trigger refresh
    let adapter_id = config
        .adapter_id
        .as_ref()
        .unwrap_or(&"internal-mqtt".to_string())
        .clone();
    if let Some(adapter) = state.device_service.get_adapter(&adapter_id).await {
        use edge_ai_devices::DeviceAdapter;

        // List devices from adapter to refresh state
        let devices = adapter.list_devices();
        let is_online = devices.contains(&device_id);

        // Update device status based on adapter state
        let new_status = if is_online {
            AdapterConnectionStatus::Connected
        } else {
            AdapterConnectionStatus::Disconnected
        };
        state
            .device_service
            .update_device_status(&device_id, new_status)
            .await;

        ok(json!({
            "device_id": device_id,
            "refreshed": true,
            "online": is_online,
        }))
    } else {
        ok(json!({
            "device_id": device_id,
            "refreshed": false,
            "error": "Adapter not available",
        }))
    }
}
