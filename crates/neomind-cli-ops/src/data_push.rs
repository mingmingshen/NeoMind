//! Data Push CLI operations.

use anyhow::Result;
use serde_json::json;

use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

/// List push targets.
pub async fn list_targets(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/data-push").await?;
    Ok(CliResponse::success(data, "Push targets listed"))
}

/// Get a push target by ID.
pub async fn get_target(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/data-push/{}", id)).await?;
    Ok(CliResponse::success(data, "Push target retrieved"))
}

/// Create a push target.
pub async fn create_target(
    client: &ApiClient,
    name: &str,
    target_type: &str,
    config: &str,
    schedule_type: &str,
    source_patterns: &str,
) -> Result<CliResponse> {
    let config_val: serde_json::Value =
        serde_json::from_str(config).unwrap_or_else(|_| json!({"url": config}));

    let schedule = match schedule_type {
        "interval" => json!({
            "type": "interval",
            "interval_secs": 60
        }),
        _ => json!({
            "type": "event_driven",
            "event_types": ["device_metric", "extension_output"]
        }),
    };

    let body = json!({
        "name": name,
        "target_type": target_type,
        "config": config_val,
        "schedule": schedule,
        "data_filter": {
            "source_patterns": source_patterns.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>(),
            "only_changes": false
        }
    });

    let data = client.post("/data-push", &body).await?;
    let target_id = data["id"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let meta = BuildMeta {
        r#type: "push".to_string(),
        action: "create".to_string(),
        entity_id: target_id.clone(),
        entity_name: Some(name.to_string()),
        undo_command: format!("neomind push delete {}", target_id),
    };

    Ok(CliResponse::success_with_meta(data, "Push target created", meta))
}

/// Update a push target.
pub async fn update_target(
    client: &ApiClient,
    id: &str,
    name: Option<&str>,
    config: Option<&str>,
    enabled: Option<bool>,
) -> Result<CliResponse> {
    let mut body = json!({});
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(c) = config {
        body["config"] =
            serde_json::from_str(c).unwrap_or_else(|_| json!({"url": c}));
    }
    if let Some(e) = enabled {
        body["enabled"] = json!(e);
    }

    let data = client.put(&format!("/data-push/{}", id), &body).await?;
    Ok(CliResponse::success(data, "Push target updated"))
}

/// Delete a push target.
pub async fn delete_target(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.delete(&format!("/data-push/{}", id)).await?;
    Ok(CliResponse::success(data, "Push target deleted"))
}

/// Start a push target.
pub async fn start_target(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.post(&format!("/data-push/{}/start", id), &json!({})).await?;
    Ok(CliResponse::success(data, "Push target started"))
}

/// Stop a push target.
pub async fn stop_target(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.post(&format!("/data-push/{}/stop", id), &json!({})).await?;
    Ok(CliResponse::success(data, "Push target stopped"))
}

/// Test a push target.
pub async fn test_target(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.post(&format!("/data-push/{}/test", id), &json!({})).await?;
    Ok(CliResponse::success(data, "Push target test completed"))
}

/// List delivery logs for a push target.
pub async fn list_logs(client: &ApiClient, id: &str, limit: Option<usize>) -> Result<CliResponse> {
    let path = if let Some(l) = limit {
        format!("/data-push/{}/logs?limit={}", id, l)
    } else {
        format!("/data-push/{}/logs", id)
    };
    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Delivery logs listed"))
}

/// Get push statistics.
pub async fn get_stats(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/data-push/stats").await?;
    Ok(CliResponse::success(data, "Push stats retrieved"))
}
