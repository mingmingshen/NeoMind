use anyhow::Result;
use serde_json::json;
use crate::types::CliResponse;
use crate::ApiClient;

/// Get current timezone setting
pub async fn get_timezone(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/settings/timezone").await?;
    Ok(CliResponse::success(data, "Timezone retrieved"))
}

/// Update timezone setting
pub async fn update_timezone(client: &ApiClient, timezone: &str) -> Result<CliResponse> {
    let body = json!({ "timezone": timezone });
    let data = client.put("/settings/timezone", &body).await?;
    Ok(CliResponse::success(data, "Timezone updated"))
}

/// List available timezones
pub async fn list_timezones(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/settings/timezones").await?;
    Ok(CliResponse::success(data, "Available timezones listed"))
}

/// Get data retention settings
pub async fn get_retention(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/settings/retention").await?;
    Ok(CliResponse::success(data, "Retention settings retrieved"))
}

/// Update data retention settings
pub async fn update_retention(
    client: &ApiClient,
    enabled: Option<bool>,
    default_retention: Option<u64>,
    topic_retention: Option<serde_json::Value>,
) -> Result<CliResponse> {
    let mut body = json!({});
    if let Some(e) = enabled {
        body["enabled"] = json!(e);
    }
    if let Some(r) = default_retention {
        body["default_retention_hours"] = json!(r);
    }
    if let Some(tr) = topic_retention {
        body["topic_retention"] = tr;
    }
    let data = client.put("/settings/retention", &body).await?;
    Ok(CliResponse::success(data, "Retention settings updated"))
}

/// Trigger manual data cleanup
pub async fn trigger_cleanup(client: &ApiClient) -> Result<CliResponse> {
    let data = client.post_raw("/settings/retention/cleanup").await?;
    Ok(CliResponse::success(data, "Cleanup triggered"))
}
