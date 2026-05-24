use anyhow::Result;
use serde_json::json;
use crate::types::CliResponse;
use crate::ApiClient;

/// List all automations (rules, transforms, agents) with optional type filter
pub async fn list_automations(
    client: &ApiClient,
    type_filter: Option<&str>,
) -> Result<CliResponse> {
    let path = match type_filter {
        Some(t) => format!("/automations?type={}", t),
        None => "/automations".to_string(),
    };
    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Automations listed"))
}

/// Get automation details by ID
pub async fn get_automation(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/automations/{}", id)).await?;
    Ok(CliResponse::success(data, "Automation retrieved"))
}

/// Export all automations
pub async fn export_automations(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/automations/export").await?;
    Ok(CliResponse::success(data, "Automations exported"))
}

/// Import automations from JSON data
pub async fn import_automations(client: &ApiClient, data_json: &str) -> Result<CliResponse> {
    let import_data: serde_json::Value = serde_json::from_str(data_json)
        .map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;
    let data = client.post("/automations/import", &import_data).await?;
    Ok(CliResponse::success(data, "Automations imported"))
}

/// Enable or disable an automation
pub async fn enable_automation(
    client: &ApiClient,
    id: &str,
    enabled: bool,
) -> Result<CliResponse> {
    let body = json!({ "enabled": enabled });
    let data = client.post(&format!("/automations/{}/enable", id), &body).await?;
    let status = if enabled { "enabled" } else { "disabled" };
    Ok(CliResponse::success(data, format!("Automation {}", status)))
}

/// Get execution history for an automation
pub async fn get_automation_executions(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/automations/{}/executions", id)).await?;
    Ok(CliResponse::success(data, "Automation executions retrieved"))
}
