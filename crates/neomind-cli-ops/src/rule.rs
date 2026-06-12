use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;
use anyhow::Result;
use serde_json::json;

/// List all rules with compact summary.
///
/// Returns id, name, enabled, and trigger description per rule.
/// Full DSL is available via `neomind rule get <id>`.
pub async fn list_rules(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/rules").await?;

    let rules = data
        .as_array()
        .or_else(|| data.get("rules").and_then(|v| v.as_array()))
        .or_else(|| {
            data.get("data").and_then(|d| d.as_array()).or_else(|| {
                data.get("data")
                    .and_then(|d| d.get("rules"))
                    .and_then(|v| v.as_array())
            })
        });

    let Some(rules) = rules else {
        return Ok(CliResponse::success(data, "Rules listed"));
    };

    let total = rules.len();
    let summary: Vec<serde_json::Value> = rules
        .iter()
        .map(|r| {
            json!({
                "id": r.get("id").and_then(|v| v.as_str()).unwrap_or(r.get("rule_id").and_then(|v| v.as_str()).unwrap_or("?")),
                "name": r.get("name").and_then(|v| v.as_str()).unwrap_or("(unnamed)"),
                "enabled": r.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                "description": r.get("description").and_then(|v| v.as_str()).unwrap_or(""),
            })
        })
        .collect();

    Ok(CliResponse::success(
        json!({ "total": total, "rules": summary }),
        format!("{} rule(s) listed", total),
    ))
}

/// Get rule by ID
pub async fn get_rule(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/rules/{}", id)).await?;
    Ok(CliResponse::success(data, "Rule retrieved"))
}

/// Create a new rule via DSL
pub async fn create_rule(client: &ApiClient, name: Option<&str>, dsl: &str) -> Result<CliResponse> {
    let mut body = json!({
        "dsl": dsl,
    });
    if let Some(n) = name {
        body["name"] = json!(n);
    }

    let data = client.post("/rules", &body).await?;
    let rule_id = data["id"].as_str().unwrap_or("unknown").to_string();

    let meta = BuildMeta {
        r#type: "rule".to_string(),
        action: "create".to_string(),
        entity_id: rule_id.clone(),
        entity_name: name.map(|s| s.to_string()),
        undo_command: format!("neomind rule delete {}", rule_id),
    };

    Ok(CliResponse::success_with_meta(data, "Rule created", meta))
}

/// Update rule via DSL
pub async fn update_rule(
    client: &ApiClient,
    id: &str,
    name: Option<&str>,
    dsl: Option<&str>,
) -> Result<CliResponse> {
    let mut body = json!({});
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(d) = dsl {
        body["dsl"] = json!(d);
    }

    let data = client.put(&format!("/rules/{}", id), &body).await?;
    Ok(CliResponse::success(data, "Rule updated"))
}

/// Delete rule
pub async fn delete_rule(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/rules/{}", id)).await?;
    Ok(CliResponse::success(json!({ "id": id }), "Rule deleted"))
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
