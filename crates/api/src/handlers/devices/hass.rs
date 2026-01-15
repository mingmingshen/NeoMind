//! Home Assistant (HASS) integration handlers.

use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::json;
use std::collections::HashMap;

use edge_ai_devices::{
    discovery::DeviceDiscovery,
    mdl_format::{
        CommandDefinition, DeviceTypeDefinition, DownlinkConfig, MetricDefinition, UplinkConfig,
    },
};

use super::models::{
    HassDiscoveredDeviceDto, HassDiscoveryMessageRequest, HassDiscoveryRequest,
    RegisterAggregatedHassDeviceRequest,
};
use crate::handlers::{
    ServerState,
    common::{HandlerResult, ok},
};
use crate::models::ErrorResponse;

/// Discover HASS ecosystem devices via MQTT Discovery.
///
/// POST /api/devices/hass/discover
pub async fn discover_hass_devices_handler(
    State(_state): State<ServerState>,
    Json(_req): Json<HassDiscoveryRequest>,
) -> HandlerResult<serde_json::Value> {
    // TODO: Migrate HASS discovery to work with new MqttAdapter
    // The old MqttDeviceManager-based HASS discovery is being refactored
    Err(ErrorResponse::internal(
        "HASS discovery is temporarily unavailable during architecture migration. \
         This feature will be restored in an upcoming release.",
    ))
}

/// Stop HASS MQTT Discovery.
///
/// POST /api/devices/hass/stop
pub async fn stop_hass_discovery_handler(
    State(_state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    // TODO: Migrate HASS discovery to work with new MqttAdapter
    Err(ErrorResponse::internal(
        "HASS discovery is temporarily unavailable during architecture migration. \
         This feature will be restored in an upcoming release.",
    ))
}

/// Process a HASS discovery message and optionally register the device.
///
/// POST /api/devices/hass/process
pub async fn process_hass_discovery_handler(
    State(state): State<ServerState>,
    Json(req): Json<HassDiscoveryMessageRequest>,
) -> HandlerResult<serde_json::Value> {
    // Parse the discovery message
    let payload_bytes = serde_json::to_vec(&req.payload).unwrap_or_default();

    let discovery = DeviceDiscovery::new();
    let msg = discovery
        .parse_hass_discovery(&req.topic, &payload_bytes)
        .map_err(|e| {
            ErrorResponse::bad_request(format!("Failed to parse discovery message: {:?}", e))
        })?;

    // Convert to MDL
    let mdl_def = discovery
        .hass_to_mdl(&msg)
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to convert to MDL: {:?}", e)))?;

    // Build device info map
    let mut device_info = HashMap::new();
    device_info.insert("discovery_topic".to_string(), req.topic.clone());
    device_info.insert("entity_id".to_string(), msg.topic_parts.entity_id());
    device_info.insert("component".to_string(), msg.topic_parts.component.clone());

    // Count metrics and commands
    let metric_count = mdl_def.uplink.metrics.len();
    let command_count = mdl_def.downlink.commands.len();

    // Check if already registered using DeviceService
    let already_registered = state
        .device_service
        .get_template(&mdl_def.device_type)
        .await
        .is_some();

    ok(json!({
        "device_type": mdl_def.device_type,
        "name": mdl_def.name,
        "description": mdl_def.description,
        "component": msg.topic_parts.component,
        "entity_id": msg.topic_parts.entity_id(),
        "discovery_topic": req.topic,
        "device_info": device_info,
        "metric_count": metric_count,
        "command_count": command_count,
        "already_registered": already_registered,
    }))
}

/// Register an aggregated HASS device (all entities of a physical device).
///
/// This creates a SINGLE device with ALL metrics and commands from all entities.
///
/// POST /api/devices/hass/register-aggregated
pub async fn register_aggregated_hass_device_handler(
    State(_state): State<ServerState>,
    Json(_req): Json<RegisterAggregatedHassDeviceRequest>,
) -> HandlerResult<serde_json::Value> {
    // TODO: Migrate HASS discovery to work with new MqttAdapter
    Err(ErrorResponse::internal(
        "HASS discovery is temporarily unavailable during architecture migration. \
         This feature will be restored in an upcoming release.",
    ))
}

/// Get HASS discovery status and supported components.
///
/// GET /api/devices/hass/status
pub async fn hass_discovery_status_handler(
    State(_state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let component_types: Vec<serde_json::Value> = DeviceDiscovery::hass_supported_components()
        .into_iter()
        .map(|c| {
            json!({
                "component": c,
                "device_type": DeviceDiscovery::hass_component_to_device_type(c),
            })
        })
        .collect();

    ok(json!({
        "hass_discovery": {
            "enabled": false,
            "subscription_topic": "homeassistant/+/config",
            "description": "Auto-discovery of HASS ecosystem devices (Tasmota, Shelly, ESPHome, etc.) - Temporarily unavailable during migration",
            "discovered_count": 0,
            "migration_note": "HASS discovery is being refactored to work with the new MqttAdapter architecture",
        },
        "supported_components": component_types,
        "component_count": component_types.len(),
    }))
}

/// Get discovered HASS devices (aggregated by physical device).
///
/// GET /api/devices/hass/discovered
pub async fn get_hass_discovered_devices_handler(
    State(_state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    ok(json!({
        "devices": [],
        "count": 0,
        "migration_note": "HASS discovery is being refactored to work with the new MqttAdapter architecture"
    }))
}

/// Clear discovered HASS devices.
///
/// DELETE /api/devices/hass/discovered
pub async fn clear_hass_discovered_devices_handler(
    State(_state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    ok(json!({
        "cleared": true,
        "migration_note": "HASS discovery is being refactored to work with the new MqttAdapter architecture"
    }))
}

/// Unregister a HASS device.
///
/// DELETE /api/devices/hass/unregister/:device_id
pub async fn unregister_hass_device_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    // Remove device using DeviceService
    state
        .device_service
        .unregister_device(&device_id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to unregister device: {}", e)))?;

    ok(json!({
        "device_id": device_id,
        "unregistered": true,
    }))
}
