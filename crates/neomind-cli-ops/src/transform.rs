use anyhow::Result;
use serde_json::json;
use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

/// List all transforms
pub async fn list_transforms(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/automations/transforms").await?;
    Ok(CliResponse::success(data, "Transforms listed"))
}

/// Get transform by ID
pub async fn get_transform(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/automations/{}", id)).await?;
    Ok(CliResponse::success(data, "Transform retrieved"))
}

/// Create a new transform
pub async fn create_transform(
    client: &ApiClient,
    name: &str,
    scope: &str,
    js_code: &str,
    output_prefix: Option<&str>,
    description: Option<&str>,
    enabled: Option<bool>,
) -> Result<CliResponse> {
    let definition = json!({
        "scope": scope,
        "js_code": js_code,
        "output_prefix": output_prefix.unwrap_or("transform"),
    });
    let body = json!({
        "name": name,
        "description": description.unwrap_or(""),
        "enabled": enabled.unwrap_or(true),
        "type": "transform",
        "definition": definition,
    });
    let data = client.post("/automations", &body).await?;
    let transform_id = data["automation"]["metadata"]["id"]
        .as_str()
        .unwrap_or("unknown");
    let meta = BuildMeta {
        r#type: "transform".to_string(),
        action: "create".to_string(),
        entity_id: transform_id.to_string(),
        entity_name: Some(name.to_string()),
        undo_command: format!("neomind transform delete {}", transform_id),
    };
    Ok(CliResponse::success_with_meta(data, "Transform created", meta))
}

/// Update transform
pub async fn update_transform(
    client: &ApiClient,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
    js_code: Option<&str>,
    scope: Option<&str>,
    output_prefix: Option<&str>,
    enabled: Option<bool>,
) -> Result<CliResponse> {
    let mut definition = json!({});
    if let Some(code) = js_code {
        definition["js_code"] = json!(code);
    }
    if let Some(s) = scope {
        definition["scope"] = json!(s);
    }
    if let Some(op) = output_prefix {
        definition["output_prefix"] = json!(op);
    }
    let mut body = json!({});
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(d) = description {
        body["description"] = json!(d);
    }
    if let Some(e) = enabled {
        body["enabled"] = json!(e);
    }
    if definition.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
        body["definition"] = definition;
    }
    let data = client.put(&format!("/automations/{}", id), &body).await?;
    Ok(CliResponse::success(data, "Transform updated"))
}

/// Delete transform
pub async fn delete_transform(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/automations/{}", id)).await?;
    Ok(CliResponse::success(json!({ "id": id }), "Transform deleted"))
}

/// List virtual metrics from transforms
pub async fn list_virtual_metrics(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/automations/transforms/metrics").await?;
    Ok(CliResponse::success(data, "Virtual metrics listed"))
}

/// Test transform code
pub async fn test_transform_code(
    client: &ApiClient,
    code: &str,
    input_data: serde_json::Value,
) -> Result<CliResponse> {
    let body = json!({
        "code": code,
        "input_data": input_data,
    });
    let data = client.post("/automations/transforms/test-code", &body).await?;
    Ok(CliResponse::success(data, "Transform code tested"))
}

/// List transform data sources
pub async fn list_transform_data_sources(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/automations/transforms/data-sources").await?;
    Ok(CliResponse::success(data, "Transform data sources listed"))
}
