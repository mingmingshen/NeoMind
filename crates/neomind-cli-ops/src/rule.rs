use anyhow::Result;
use serde_json::json;
use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

/// List all rules
pub async fn list_rules(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/rules").await?;
    Ok(CliResponse::success(data, "Rules listed"))
}

/// Get rule by ID
pub async fn get_rule(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/rules/{}", id)).await?;
    Ok(CliResponse::success(data, "Rule retrieved"))
}

/// Create a new rule
pub async fn create_rule(
    client: &ApiClient,
    name: &str,
    trigger: &str,
    actions: serde_json::Value,
    condition: Option<serde_json::Value>,
) -> Result<CliResponse> {
    let mut body = json!({
        "name": name,
        "trigger": trigger,
        "actions": actions,
    });
    if let Some(cond) = condition {
        body["condition"] = cond;
    }

    let data = client.post("/rules", &body).await?;
    let rule_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let meta = BuildMeta {
        r#type: "rule".to_string(),
        action: "create".to_string(),
        entity_id: rule_id.clone(),
        entity_name: Some(name.to_string()),
        undo_command: format!("neomind rule delete {}", rule_id),
    };

    Ok(CliResponse::success_with_meta(data, "Rule created", meta))
}

/// Update rule
pub async fn update_rule(
    client: &ApiClient,
    id: &str,
    name: Option<&str>,
    trigger: Option<&str>,
    actions: Option<serde_json::Value>,
    condition: Option<serde_json::Value>,
) -> Result<CliResponse> {
    let mut body = json!({});
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(t) = trigger {
        body["trigger"] = json!(t);
    }
    if let Some(a) = actions {
        body["actions"] = a;
    }
    if let Some(c) = condition {
        body["condition"] = c;
    }

    let data = client.put(&format!("/rules/{}", id), &body).await?;
    Ok(CliResponse::success(data, "Rule updated"))
}

/// Delete rule
pub async fn delete_rule(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/rules/{}", id)).await?;
    Ok(CliResponse::success(
        json!({ "id": id }),
        "Rule deleted",
    ))
}

/// Enable rule
pub async fn enable_rule(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let body = json!({ "enabled": true });
    client.post(&format!("/rules/{}/enable", id), &body).await?;
    Ok(CliResponse::success(
        json!({ "id": id, "enabled": true }),
        "Rule enabled",
    ))
}

/// Disable rule
pub async fn disable_rule(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let body = json!({ "enabled": false });
    client.post(&format!("/rules/{}/enable", id), &body).await?;
    Ok(CliResponse::success(
        json!({ "id": id, "enabled": false }),
        "Rule disabled",
    ))
}

/// Test rule
pub async fn test_rule(
    client: &ApiClient,
    id: &str,
    input: serde_json::Value,
) -> Result<CliResponse> {
    let data = client.post(&format!("/rules/{}/test", id), &input).await?;
    Ok(CliResponse::success(data, "Rule tested"))
}

/// Get rule execution history
pub async fn get_rule_history(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/rules/{}/history", id)).await?;
    Ok(CliResponse::success(data, "Rule history retrieved"))
}
