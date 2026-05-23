use anyhow::Result;
use serde_json::json;
use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

/// List devices with optional filters
pub async fn list_devices(
    client: &ApiClient,
    device_type: Option<&str>,
    status: Option<&str>,
) -> Result<CliResponse> {
    let mut path = "/devices?limit=100".to_string();
    if let Some(dt) = device_type {
        path.push_str(&format!("&device_type={}", dt));
    }
    if let Some(s) = status {
        path.push_str(&format!("&status={}", s));
    }
    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Devices listed"))
}

/// Get device by ID
pub async fn get_device(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/devices/{}", id)).await?;
    Ok(CliResponse::success(data, "Device retrieved"))
}

/// Create a new device
pub async fn create_device(
    client: &ApiClient,
    name: &str,
    device_type: &str,
    adapter_type: &str,
    connection_config: Option<serde_json::Value>,
) -> Result<CliResponse> {
    let mut body = json!({
        "name": name,
        "device_type": device_type,
        "adapter_type": adapter_type,
        "connection_config": {}
    });
    if let Some(config) = connection_config {
        body["connection_config"] = config;
    }

    let data = client.post("/devices", &body).await?;
    let device_id = data.get("data")
        .and_then(|d| d.get("device_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_str().map(|s| s.to_string()))
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let meta = BuildMeta {
        r#type: "device".to_string(),
        action: "create".to_string(),
        entity_id: device_id.clone(),
        entity_name: Some(name.to_string()),
        undo_command: format!("neomind device delete {}", device_id),
    };

    Ok(CliResponse::success_with_meta(data, "Device created", meta))
}

/// Update device
pub async fn update_device(
    client: &ApiClient,
    id: &str,
    name: Option<&str>,
    connection_config: Option<serde_json::Value>,
) -> Result<CliResponse> {
    let mut body = json!({});
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(config) = connection_config {
        body["connection_config"] = config;
    }

    let data = client.put(&format!("/devices/{}", id), &body).await?;
    Ok(CliResponse::success(data, "Device updated"))
}

/// Delete device
pub async fn delete_device(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/devices/{}", id)).await?;
    Ok(CliResponse::success(
        json!({ "id": id }),
        "Device deleted",
    ))
}

/// Get latest metrics for a device
pub async fn get_latest_metrics(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/devices/{}/current", id)).await?;
    Ok(CliResponse::success(data, "Latest metrics retrieved"))
}

/// Get historical telemetry data for a device
pub async fn get_telemetry_history(
    client: &ApiClient,
    id: &str,
    metric: Option<&str>,
    time_range: Option<&str>,
    compress: bool,
) -> Result<CliResponse> {
    let mut path = format!("/devices/{}/telemetry", id);
    let mut params = Vec::new();
    if let Some(m) = metric {
        params.push(format!("metric={}", m));
    }
    // Parse --time-range (e.g., "1h", "24h", "7d", "30d") to Unix seconds and set start
    if let Some(tr) = time_range {
        let end = chrono::Utc::now().timestamp();
        let start = parse_time_range_to_timestamp(tr, end).unwrap_or(end - 86400);
        params.push(format!("start={}", start));
        params.push(format!("end={}", end));
    }
    if compress {
        params.push("compress=true".to_string());
    }
    if !params.is_empty() {
        path.push('?');
        path.push_str(&params.join("&"));
    }

    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Telemetry history retrieved"))
}

/// Parse a human-readable time range string (e.g., "1h", "24h", "7d", "30d") to a start timestamp.
fn parse_time_range_to_timestamp(range: &str, now_ts: i64) -> Option<i64> {
    let range = range.trim();
    if range.is_empty() {
        return None;
    }
    // Extract number suffix: last char(s)
    let num_end = range.len()
        - range.chars().last().map_or(0, |c| {
            if c.is_ascii_alphabetic() { 1 } else { 0 }
        });
    let num: i64 = range[..num_end].parse().ok()?;
    let unit = &range[num_end..];
    let secs = match unit {
        "s" => num,
        "m" | "min" | "mins" => num * 60,
        "h" | "hr" | "hrs" => num * 3600,
        "d" | "day" | "days" => num * 86400,
        "w" | "wk" | "wks" => num * 7 * 86400,
        "mo" | "month" | "months" => num * 30 * 86400,
        _ => return None,
    };
    Some(now_ts - secs)
}

/// Send control command to a device
pub async fn control_device(
    client: &ApiClient,
    id: &str,
    command: &str,
    params: serde_json::Value,
) -> Result<CliResponse> {
    let body = json!({ "params": params });
    let data = client
        .post(&format!("/devices/{}/command/{}", id, command), &body)
        .await?;
    Ok(CliResponse::success(data, "Command sent"))
}

/// List device types
pub async fn list_device_types(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/device-types").await?;
    Ok(CliResponse::success(data, "Device types listed"))
}

/// Get device type by ID
pub async fn get_device_type(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/device-types/{}", id)).await?;
    Ok(CliResponse::success(data, "Device type retrieved"))
}

/// Convert a display name to a snake_case device type ID.
/// E.g., "温湿度传感器" -> "wen_shi_du_chuan_gan_qi", "TempSensor" -> "temp_sensor"
fn name_to_type_id(name: &str) -> String {
    // Use a simple approach: lowercase, replace non-alphanumeric with underscore, collapse
    let id: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    // Collapse multiple underscores
    let mut result = String::new();
    let mut prev_underscore = false;
    for ch in id.chars() {
        if ch == '_' {
            if !prev_underscore {
                result.push(ch);
            }
            prev_underscore = true;
        } else {
            result.push(ch);
            prev_underscore = false;
        }
    }
    // Trim leading/trailing underscores
    result.trim_matches('_').to_string()
}

/// Create a new device type
pub async fn create_device_type(
    client: &ApiClient,
    id: Option<&str>,
    name: &str,
    metrics: serde_json::Value,
    commands: Option<serde_json::Value>,
) -> Result<CliResponse> {
    let device_type = id
        .map(|s| s.to_string())
        .unwrap_or_else(|| name_to_type_id(name));

    let mut body = json!({
        "device_type": device_type,
        "name": name,
        "description": "",
        "categories": [],
        "mode": "simple",
        "metrics": metrics,
        "uplink_samples": [],
        "parameters": [],
        "commands": [],
    });
    if let Some(cmds) = commands {
        body["commands"] = cmds;
    }

    let data = client.post("/device-types", &body).await?;
    let meta = BuildMeta {
        r#type: "device_type".to_string(),
        action: "create".to_string(),
        entity_id: device_type.clone(),
        entity_name: Some(name.to_string()),
        undo_command: format!("neomind device types delete {}", device_type),
    };

    Ok(CliResponse::success_with_meta(data, "Device type created", meta))
}

/// Delete device type
pub async fn delete_device_type(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/device-types/{}", id)).await?;
    Ok(CliResponse::success(
        json!({ "id": id }),
        "Device type deleted",
    ))
}

/// Write metric data point for a device
pub async fn write_metric(
    client: &ApiClient,
    id: &str,
    metric: &str,
    value: serde_json::Value,
    timestamp: Option<i64>,
) -> Result<CliResponse> {
    let mut body = json!({
        "metric": metric,
        "value": value,
    });
    if let Some(ts) = timestamp {
        body["timestamp"] = json!(ts);
    }
    let data = client.post(&format!("/devices/{}/metrics", id), &body).await?;
    Ok(CliResponse::success(data, "Metric written"))
}
