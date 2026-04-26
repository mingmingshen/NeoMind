//! MQTT subscription management handlers.

use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;

use super::models::{MqttSubscribeRequest, MqttSubscriptionDto, MqttUnsubscribeRequest};
use crate::handlers::{
    common::{ok, HandlerResult},
    ServerState,
};
use crate::models::ErrorResponse;

/// List MQTT subscriptions.
///
/// GET /api/mqtt/subscriptions
pub async fn list_subscriptions_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    // Use new DeviceService to get devices
    let configs = state.devices.service.list_devices().await;

    // Build subscriptions list
    let mut subscriptions = vec![
        MqttSubscriptionDto {
            topic: "device/+/uplink".to_string(),
            qos: 1,
            device_id: None,
        },
        MqttSubscriptionDto {
            topic: "device/+/downlink".to_string(),
            qos: 1,
            device_id: None,
        },
    ];

    // Add per-device subscriptions
    for config in configs {
        subscriptions.push(MqttSubscriptionDto {
            topic: format!("device/{}/uplink", config.device_id),
            qos: 1,
            device_id: Some(config.device_id.clone()),
        });
        subscriptions.push(MqttSubscriptionDto {
            topic: format!("device/{}/downlink", config.device_id),
            qos: 1,
            device_id: Some(config.device_id),
        });
    }

    ok(json!({
        "subscriptions": subscriptions,
        "count": subscriptions.len(),
    }))
}

/// Subscribe to a topic.
///
/// POST /api/mqtt/subscribe
pub async fn subscribe_handler(
    State(state): State<ServerState>,
    Json(req): Json<MqttSubscribeRequest>,
) -> HandlerResult<serde_json::Value> {
    // Get the first MQTT adapter
    let adapters = state.devices.service.list_adapters().await;
    let mqtt_adapter = adapters
        .iter()
        .find(|a| a.adapter_type == "mqtt");

    let adapter = match mqtt_adapter {
        Some(a) => a,
        None => {
            return ok(json!({
                "success": false,
                "message": "No MQTT adapter available",
            }));
        }
    };

    // Get the actual adapter instance
    let adapter_instance = state
        .devices
        .service
        .get_adapter(&adapter.id)
        .await
        .ok_or_else(|| ErrorResponse::internal("MQTT adapter not found".to_string()))?;

    // Subscribe to the custom topic using the adapter's subscribe_topic method
    // Note: This requires the adapter to have a subscribe_topic method
    // For now, we'll return a message indicating this needs adapter support
    tracing::info!(
        category = "mqtt",
        topic = %req.topic,
        qos = req.qos,
        "Custom topic subscription requested"
    );

    // Since the DeviceAdapter trait doesn't have a generic subscribe_topic method,
    // we'll need to downcast to MqttAdapter to access it
    // For now, return success with a message
    ok(json!({
        "success": false,
        "message": "Custom topic subscription requires MQTT adapter-specific interface. Use subscribe_device for device-specific subscriptions.",
        "topic": req.topic,
        "qos": req.qos,
    }))
}

/// Unsubscribe from a topic.
///
/// POST /api/mqtt/unsubscribe
pub async fn unsubscribe_handler(
    State(state): State<ServerState>,
    Json(req): Json<MqttUnsubscribeRequest>,
) -> HandlerResult<serde_json::Value> {
    // Get the first MQTT adapter
    let adapters = state.devices.service.list_adapters().await;
    let mqtt_adapter = adapters
        .iter()
        .find(|a| a.adapter_type == "mqtt");

    let adapter = match mqtt_adapter {
        Some(a) => a,
        None => {
            return ok(json!({
                "success": false,
                "message": "No MQTT adapter available",
            }));
        }
    };

    // Get the actual adapter instance
    let adapter_instance = state
        .devices
        .service
        .get_adapter(&adapter.id)
        .await
        .ok_or_else(|| ErrorResponse::internal("MQTT adapter not found".to_string()))?;

    // Since the DeviceAdapter trait doesn't have a generic unsubscribe_topic method,
    // we'll need to downcast to MqttAdapter to access it
    // For now, we'll track this as a best-effort operation
    tracing::info!(
        category = "mqtt",
        topic = %req.topic,
        "Custom topic unsubscription requested"
    );

    // Try to unsubscribe using the adapter's unsubscribe_device method
    // This is a workaround since we don't have direct access to unsubscribe_topic
    // In a full implementation, we would:
    // 1. Downcast to MqttAdapter
    // 2. Call adapter.unsubscribe_topic(&req.topic).await
    // 3. Return success/failure

    // For now, return success with a message indicating the limitation
    ok(json!({
        "success": false,
        "message": "Custom topic unsubscription requires MQTT adapter-specific interface. Use unsubscribe_device for device-specific unsubscriptions.",
        "topic": req.topic,
    }))
}

/// Subscribe to a device's metrics.
///
/// POST /api/mqtt/subscribe/:device_id
pub async fn subscribe_device_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    // Validate the device exists using DeviceService
    let device_opt = state.devices.service.get_device(&device_id).await;
    let device = device_opt
        .ok_or_else(|| ErrorResponse::not_found(format!("Device not found: {}", device_id)))?;

    // Get the adapter for this device and subscribe
    if let Some(ref adapter_id) = device.adapter_id {
        if let Some(adapter) = state.devices.service.get_adapter(adapter_id).await {
            adapter
                .subscribe_device(&device_id)
                .await
                .map_err(|e| ErrorResponse::internal(format!("Failed to subscribe: {}", e)))?;
        }
    }

    ok(json!({
        "message": format!("Subscribed to device: {}", device_id),
        "device_id": device_id,
    }))
}

/// Unsubscribe from a device's metrics.
///
/// POST /api/mqtt/unsubscribe/:device_id
pub async fn unsubscribe_device_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    // Validate the device exists using DeviceService
    let configs = state.devices.service.list_devices().await;
    let device_exists = configs.iter().any(|d| d.device_id == device_id);

    if !device_exists {
        return Err(ErrorResponse::not_found(format!(
            "Device not found: {}",
            device_id
        )));
    }

    ok(json!({
        "message": format!("Device {} uses wildcard subscription - no individual subscription to remove", device_id),
        "device_id": device_id,
    }))
}
