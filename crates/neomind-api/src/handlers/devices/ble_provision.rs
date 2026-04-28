//! BLE provisioning handler for zero-touch device setup.
//!
//! This module provides the REST endpoint for provisioning devices via BLE,
//! generating MQTT configuration and registering the device in the system.

use std::collections::HashMap;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config;
use crate::handlers::common::{ok, HandlerResult};
use crate::models::ErrorResponse;
use crate::server::types::ServerState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// BLE provision request body.
#[derive(Debug, Deserialize)]
pub struct BleProvisionRequest {
    /// Device model identifier (e.g. "NE101").
    pub model: String,
    /// Device serial number (e.g. "NE101-A2F003").
    pub sn: String,
    /// Device type template identifier (e.g. "ne101_camera").
    pub device_type: String,
    /// Human-readable device name (e.g. "门口摄像头").
    pub device_name: String,
    /// Broker identifier – "embedded" for the built-in broker, or a UUID for an external broker.
    pub broker_id: String,
}

/// MQTT configuration returned to the BLE client so the device can connect.
#[derive(Debug, Serialize)]
pub struct MqttConfigResponse {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub topic_prefix: String,
}

// ---------------------------------------------------------------------------
// Helper: determine the server's LAN IP address
// ---------------------------------------------------------------------------

/// Get the actual local IP address of the server (reused logic from mqtt/status).
fn get_server_ip() -> String {
    use std::net::IpAddr;

    // Try to get local IP by creating a UDP socket
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                let ip = local_addr.ip();
                if let IpAddr::V4(ipv4) = ip {
                    let o = ipv4.octets();
                    if (o[0] == 192 && o[1] == 168)
                        || o[0] == 10
                        || (o[0] == 172 && o[1] >= 16 && o[1] <= 31)
                    {
                        return ip.to_string();
                    }
                }
            }
        }
    }

    // Fallback: try network interfaces
    if let Ok(interfaces) = get_if_addrs::get_if_addrs() {
        for iface in interfaces {
            if !iface.is_loopback() {
                if let get_if_addrs::IfAddr::V4(iface_addr) = iface.addr {
                    let o = iface_addr.ip.octets();
                    if (o[0] == 192 && o[1] == 168)
                        || o[0] == 10
                        || (o[0] == 172 && o[1] >= 16 && o[1] <= 31)
                    {
                        return iface_addr.ip.to_string();
                    }
                }
            }
        }
    }

    // Last fallback
    std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string())
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// BLE provision endpoint.
///
/// `POST /api/devices/ble-provision`
///
/// Validates the device type, generates a device ID from the serial number, resolves the
/// MQTT broker configuration, registers the device and returns the MQTT connection details
/// so that the BLE client can program the device over-the-air.
pub async fn ble_provision_handler(
    State(state): State<ServerState>,
    Json(req): Json<BleProvisionRequest>,
) -> HandlerResult<serde_json::Value> {
    // 1. Validate device_type exists in registry
    if state
        .devices
        .service
        .get_template(&req.device_type)
        .is_none()
    {
        return Err(ErrorResponse::bad_request(format!(
            "Unknown device_type: {}",
            req.device_type
        )));
    }

    // 2. Generate device_id from SN
    let device_id = req.sn.to_lowercase().replace('-', "_");

    // 3. Check for duplicate
    if state.devices.service.get_device(&device_id).is_some() {
        return Err(ErrorResponse::conflict(format!(
            "Device already exists: {}",
            device_id
        )));
    }

    // 4. Resolve broker config
    let (host, port, username, password) = if req.broker_id == "embedded" {
        let store = config::open_settings_store()
            .map_err(|e| ErrorResponse::internal(format!("Failed to open settings store: {}", e)))?;
        let settings = store.get_mqtt_settings();
        let host = get_server_ip();
        (host, settings.port, String::new(), String::new())
    } else {
        // External broker
        let store = config::open_settings_store()
            .map_err(|e| ErrorResponse::internal(format!("Failed to open settings store: {}", e)))?;
        let broker = store
            .load_external_broker(&req.broker_id)
            .map_err(|e| ErrorResponse::internal(format!("Failed to load broker: {}", e)))?
            .ok_or_else(|| {
                ErrorResponse::not_found(format!("Broker not found: {}", req.broker_id))
            })?;
        (
            broker.broker,
            broker.port,
            broker.username.unwrap_or_default(),
            broker.password.unwrap_or_default(),
        )
    };

    // 5. Build DeviceConfig
    let topic_prefix = format!("device/{}/{}", req.device_type, device_id);

    let mut extra = HashMap::new();
    extra.insert(
        "ble_provisioned".to_string(),
        serde_json::Value::Bool(true),
    );
    extra.insert(
        "provisioned_at".to_string(),
        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
    );

    let config = neomind_devices::DeviceConfig {
        device_id: device_id.clone(),
        name: req.device_name,
        device_type: req.device_type,
        adapter_type: "mqtt".to_string(),
        connection_config: neomind_devices::ConnectionConfig {
            telemetry_topic: Some(format!("{}/uplink", topic_prefix)),
            command_topic: Some(format!("{}/downlink", topic_prefix)),
            json_path: None,
            entity_id: None,
            extra,
        },
        adapter_id: None,
    };

    // 6. Register device
    state
        .devices
        .service
        .register_device(config)
        .await
        .map_err(|e| {
            ErrorResponse::internal(format!("Failed to register BLE provisioned device: {}", e))
        })?;

    tracing::info!(
        category = "ble",
        device_id = %device_id,
        broker_id = %req.broker_id,
        "BLE provisioned device registered"
    );

    // 7. Return MQTT configuration
    let mqtt_config = MqttConfigResponse {
        host,
        port,
        username,
        password,
        topic_prefix,
    };

    ok(json!({
        "device_id": device_id,
        "mqtt_config": mqtt_config,
    }))
}
