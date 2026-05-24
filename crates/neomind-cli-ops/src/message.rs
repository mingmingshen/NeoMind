use anyhow::Result;
use serde_json::json;
use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

/// List messages with optional filters
pub async fn list_messages(
    client: &ApiClient,
    limit: Option<usize>,
    offset: Option<usize>,
    severity: Option<&str>,
    status: Option<&str>,
) -> Result<CliResponse> {
    let mut path = "/messages".to_string();
    let mut params = Vec::new();

    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if let Some(o) = offset {
        params.push(format!("offset={}", o));
    }
    if let Some(s) = severity {
        params.push(format!("severity={}", s));
    }
    if let Some(st) = status {
        params.push(format!("status={}", st));
    }

    if !params.is_empty() {
        path.push('?');
        path.push_str(&params.join("&"));
    }

    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Messages listed"))
}

/// Get message by ID
pub async fn get_message(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/messages/{}", id)).await?;
    Ok(CliResponse::success(data, "Message retrieved"))
}

/// Send/create a new message
pub async fn send_message(
    client: &ApiClient,
    title: &str,
    message: &str,
    severity: &str,
    source: Option<&str>,
) -> Result<CliResponse> {
    let mut body = json!({
        "title": title,
        "message": message,
        "severity": severity,
        "category": "system",
    });
    if let Some(src) = source {
        body["source"] = json!(src);
    }

    let data = client.post("/messages", &body).await?;
    let msg_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let meta = BuildMeta {
        r#type: "message".to_string(),
        action: "send".to_string(),
        entity_id: msg_id.clone(),
        entity_name: Some(title.to_string()),
        undo_command: format!("neomind message delete {}", msg_id),
    };

    Ok(CliResponse::success_with_meta(data, "Message sent", meta))
}

/// Acknowledge/read a message
pub async fn acknowledge_message(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.post(&format!("/messages/{}/acknowledge", id), &json!({})).await?;
    Ok(CliResponse::success(
        json!({ "id": id }),
        "Message acknowledged",
    ))
}

// ---- Message Channel operations ----

/// List all message channels
pub async fn list_channels(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/messages/channels").await?;
    Ok(CliResponse::success(data, "Channels listed"))
}

/// Get channel by name
pub async fn get_channel(client: &ApiClient, name: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/messages/channels/{}", name)).await?;
    Ok(CliResponse::success(data, "Channel retrieved"))
}

/// List available channel types
pub async fn list_channel_types(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/messages/channels/types").await?;
    Ok(CliResponse::success(data, "Channel types listed"))
}

/// Get channel type schema
pub async fn get_channel_type_schema(client: &ApiClient, channel_type: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/messages/channels/types/{}/schema", channel_type)).await?;
    Ok(CliResponse::success(data, "Channel type schema retrieved"))
}

/// Create a message channel
pub async fn create_channel(
    client: &ApiClient,
    name: &str,
    channel_type: &str,
    config: &str,
) -> Result<CliResponse> {
    let config_value = serde_json::from_str(config).unwrap_or(json!(config));
    let body = json!({
        "name": name,
        "channel_type": channel_type,
    });
    // Merge config into body (flattened)
    let mut body = body;
    if let serde_json::Value::Object(mut map) = config_value {
        body.as_object_mut().unwrap().append(&mut map);
    }
    let data = client.post("/messages/channels", &body).await?;
    let meta = BuildMeta {
        r#type: "channel".to_string(),
        action: "create".to_string(),
        entity_id: name.to_string(),
        entity_name: Some(name.to_string()),
        undo_command: format!("neomind message channel-delete {}", name),
    };
    Ok(CliResponse::success_with_meta(data, "Channel created", meta))
}

/// Update a channel's configuration
pub async fn update_channel(
    client: &ApiClient,
    name: &str,
    config: &str,
) -> Result<CliResponse> {
    let config_value = serde_json::from_str(config).unwrap_or(json!(config));
    let body = json!({ "config": config_value });
    let data = client.put(&format!("/messages/channels/{}", name), &body).await?;
    Ok(CliResponse::success(data, "Channel updated"))
}

/// Delete a message channel
pub async fn delete_channel(client: &ApiClient, name: &str) -> Result<CliResponse> {
    let data = client.delete(&format!("/messages/channels/{}", name)).await?;
    Ok(CliResponse::success(data, "Channel deleted"))
}

/// Test a channel
pub async fn test_channel(client: &ApiClient, name: &str) -> Result<CliResponse> {
    let data = client.post(&format!("/messages/channels/{}/test", name), &json!({})).await?;
    Ok(CliResponse::success(data, "Channel test completed"))
}
