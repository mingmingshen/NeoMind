use crate::types::CliResponse;
use crate::ApiClient;
use anyhow::Result;
use serde_json::json;

/// Get system infrastructure info: MQTT broker, network, webhook URLs
pub async fn system_info(client: &ApiClient) -> Result<CliResponse> {
    // Fetch MQTT status, embedded broker config, and network info in parallel
    let mqtt_fut = client.get("/mqtt/status");
    let broker_config_fut = client.get("/mqtt/broker-config");
    let net_fut = client.get("/system/network-info");

    let (mqtt_result, broker_config_result, net_result) =
        tokio::join!(mqtt_fut, broker_config_fut, net_fut);

    // Extract MQTT info
    let mqtt_data = mqtt_result.ok();
    let mqtt_connected = mqtt_data
        .as_ref()
        .and_then(|d| d.get("status"))
        .and_then(|s| s.get("connected"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mqtt_ip = mqtt_data
        .as_ref()
        .and_then(|d| d.get("status"))
        .and_then(|s| s.get("server_ip"))
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0.0");
    let mqtt_port = mqtt_data
        .as_ref()
        .and_then(|d| d.get("status"))
        .and_then(|s| s.get("listen_port"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1883);
    let devices_count = mqtt_data
        .as_ref()
        .and_then(|d| d.get("status"))
        .and_then(|s| s.get("devices_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Extract embedded broker config (TLS, auth, credentials)
    let broker_config = broker_config_result.ok();
    let tls_enabled = broker_config
        .as_ref()
        .and_then(|c| c.get("config"))
        .and_then(|c| c.get("tls_enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let auth_enabled = broker_config
        .as_ref()
        .and_then(|c| c.get("config"))
        .and_then(|c| c.get("auth_enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let credentials: Vec<serde_json::Value> = broker_config
        .as_ref()
        .and_then(|c| c.get("config"))
        .and_then(|c| c.get("credentials"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|cred| {
                    json!({
                        "username": cred.get("username").and_then(|v| v.as_str()).unwrap_or(""),
                        "password": cred.get("password").and_then(|v| v.as_str()).unwrap_or(""),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let tls_ca_path = broker_config
        .as_ref()
        .and_then(|c| c.get("config"))
        .and_then(|c| c.get("tls_ca_path"))
        .and_then(|v| v.as_str());

    // Determine protocol scheme
    let protocol_scheme = if tls_enabled { "mqtts" } else { "mqtt" };
    let broker_url = format!("{}://{}:{}", protocol_scheme, mqtt_ip, mqtt_port);

    // Extract network info
    let net_data = net_result.ok();
    let server_ip = net_data
        .as_ref()
        .and_then(|d| d.get("ip"))
        .and_then(|v| v.as_str())
        .unwrap_or(mqtt_ip);
    let wifi_ssid = net_data
        .as_ref()
        .and_then(|d| d.get("ssid"))
        .and_then(|v| v.as_str());

    // Build the info response
    let api_base = client.base_url();
    let server_base = api_base.trim_end_matches("/api");
    let webhook_url = format!("{}/api/devices/{{device_id}}/webhook", server_base);
    let api_url = api_base.to_string();

    let info = json!({
        "mqtt": {
            "broker_address": format!("{}:{}", server_ip, mqtt_port),
            "broker_url": broker_url,
            "connected": mqtt_connected,
            "port": mqtt_port,
            "protocol": "MQTT 3.1.1",
            "tls_enabled": tls_enabled,
            "auth_enabled": auth_enabled,
            "credentials": credentials,
            "tls_ca_available": tls_ca_path.is_some(),
            "devices_connected": devices_count,
            "discovery_topic": "neomind/discovery/#",
        },
        "network": {
            "server_ip": server_ip,
            "wifi_ssid": wifi_ssid,
            "api_url": api_url,
        },
        "device_connection": {
            "mqtt": {
                "broker": broker_url,
                "topic_format": "any/topic/{metric_name}",
                "payload_format": "JSON {\"value\": <number>}",
                "auto_discovery": true,
                "discovery_prefix": "neomind/discovery",
                "tls": tls_enabled,
                "auth_required": auth_enabled,
            },
            "webhook": {
                "url": webhook_url,
                "method": "POST",
                "content_type": "application/json",
                "payload_example": {
                    "timestamp": 1234567890,
                    "quality": 1.0,
                    "data": {"temperature": 23.5, "humidity": 65}
                },
            },
        },
    });

    Ok(CliResponse::success(info, "System info retrieved"))
}
