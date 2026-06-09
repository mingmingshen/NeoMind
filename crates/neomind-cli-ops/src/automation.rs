use anyhow::Result;
use serde_json::json;
use crate::types::CliResponse;
use crate::ApiClient;

/// List all automations (rules, transforms, agents) with compact summary.
///
/// Returns id, name, type, and enabled per automation.
/// Full config is available via `neomind automation get <id>`.
pub async fn list_automations(
    client: &ApiClient,
    type_filter: Option<&str>,
) -> Result<CliResponse> {
    let path = match type_filter {
        Some(t) => format!("/automations?type={}", t),
        None => "/automations".to_string(),
    };
    let data = client.get(&path).await?;

    let automations = extract_list_array(&data, "automations");

    let Some(automations) = automations else {
        return Ok(CliResponse::success(data, "Automations listed"));
    };

    let total = automations.len();
    let summary: Vec<serde_json::Value> = automations
        .iter()
        .map(|a| {
            json!({
                "id": a.get("id").and_then(|v| v.as_str()).unwrap_or("?"),
                "name": a.get("name").and_then(|v| v.as_str()).unwrap_or("(unnamed)"),
                "type": a.get("type").or_else(|| a.get("automation_type")).and_then(|v| v.as_str()).unwrap_or("unknown"),
                "enabled": a.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
            })
        })
        .collect();

    Ok(CliResponse::success(
        json!({ "total": total, "automations": summary }),
        format!("{} automation(s) listed", total),
    ))
}

/// Helper: extract an array from API response, trying common nesting patterns.
fn extract_list_array(data: &serde_json::Value, key: &str) -> Option<Vec<serde_json::Value>> {
    data.as_array().cloned()
        .or_else(|| data.get(key).and_then(|v| v.as_array()).cloned())
        .or_else(|| data.get("data").and_then(|d| d.as_array()).cloned())
        .or_else(|| data.get("data").and_then(|d| d.get(key)).and_then(|v| v.as_array()).cloned())
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
