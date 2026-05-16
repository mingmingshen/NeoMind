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
