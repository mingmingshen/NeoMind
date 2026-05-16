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
    });
    if let Some(config) = connection_config {
        body["connection_config"] = config;
    }

    let data = client.post("/devices", &body).await?;
    let device_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
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
) -> Result<CliResponse> {
    let mut path = format!("/devices/{}/telemetry", id);
    let mut params = Vec::new();
    if let Some(m) = metric {
        params.push(format!("metric={}", m));
    }
    if let Some(tr) = time_range {
        params.push(format!("time_range={}", tr));
    }
    if !params.is_empty() {
        path.push('?');
        path.push_str(&params.join("&"));
    }

    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Telemetry history retrieved"))
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

/// Create a new device type
pub async fn create_device_type(
    client: &ApiClient,
    name: &str,
    metrics: serde_json::Value,
    commands: Option<serde_json::Value>,
) -> Result<CliResponse> {
    let mut body = json!({
        "name": name,
        "metrics": metrics,
    });
    if let Some(cmds) = commands {
        body["commands"] = cmds;
    }

    let data = client.post("/device-types", &body).await?;
    let type_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let meta = BuildMeta {
        r#type: "device_type".to_string(),
        action: "create".to_string(),
        entity_id: type_id.clone(),
        entity_name: Some(name.to_string()),
        undo_command: format!("neomind device types delete {}", type_id),
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
