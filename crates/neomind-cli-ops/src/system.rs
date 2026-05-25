use anyhow::Result;
use serde_json::json;
use crate::types::CliResponse;
use crate::ApiClient;

/// Get system infrastructure info: MQTT broker, network, webhook URLs
pub async fn system_info(client: &ApiClient) -> Result<CliResponse> {
    // Fetch MQTT status and network info in parallel
    let mqtt_fut = client.get("/mqtt/status");
    let net_fut = client.get("/system/network-info");

    let (mqtt_result, net_result) = tokio::join!(mqtt_fut, net_fut);

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
    // api_base includes /api suffix (e.g. http://host:9375/api), strip it for device-facing URLs
    let server_base = api_base.trim_end_matches("/api");
    let webhook_url = format!("{}/api/devices/{{device_id}}/webhook", server_base);
    let api_url = api_base.to_string();

    let info = json!({
        "mqtt": {
            "broker_address": format!("{}:{}", server_ip, mqtt_port),
            "connected": mqtt_connected,
            "port": mqtt_port,
            "protocol": "MQTT 3.1.1",
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
                "broker": format!("tcp://{}:{}", server_ip, mqtt_port),
                "topic_format": "any/topic/{metric_name}",
                "payload_format": "JSON {\"value\": <number>}",
                "auto_discovery": true,
                "discovery_prefix": "neomind/discovery",
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
