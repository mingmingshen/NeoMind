use anyhow::Result;
use serde_json::json;
use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

/// List all installed widgets
pub async fn list_widgets(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/frontend-components").await?;
    Ok(CliResponse::success(data, "Widgets listed"))
}

/// Get widget by ID
pub async fn get_widget(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/frontend-components/{}", id)).await?;
    Ok(CliResponse::success(data, "Widget retrieved"))
}

/// Get widget bundle by ID
pub async fn get_widget_bundle(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/frontend-components/{}/bundle", id)).await?;
    Ok(CliResponse::success(data, "Widget bundle retrieved"))
}

/// Install widget from file
pub async fn install_widget_file(
    client: &ApiClient,
    file_path: &str,
) -> Result<CliResponse> {
    let data = client.post_file("/frontend-components", file_path).await?;
    let widget_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let widget_name = data["name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let meta = BuildMeta {
        r#type: "widget".to_string(),
        action: "install".to_string(),
        entity_id: widget_id.clone(),
        entity_name: Some(widget_name),
        undo_command: format!("neomind widget uninstall {}", widget_id),
    };

    Ok(CliResponse::success_with_meta(data, "Widget installed", meta))
}

/// Uninstall widget
pub async fn uninstall_widget(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/frontend-components/{}", id)).await?;
    Ok(CliResponse::success(
        json!({ "id": id }),
        "Widget uninstalled",
    ))
}

/// List marketplace widgets
pub async fn list_marketplace_widgets(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/frontend-components/market/list").await?;
    Ok(CliResponse::success(data, "Marketplace widgets listed"))
}

/// Install widget from marketplace
pub async fn install_widget_market(
    client: &ApiClient,
    widget_id: &str,
    version: Option<&str>,
) -> Result<CliResponse> {
    let mut body = json!({
        "id": widget_id,
    });
    if let Some(v) = version {
        body["version"] = json!(v);
    }

    let data = client.post("/frontend-components/market/install", &body).await?;
    let installed_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| widget_id.to_string());

    let widget_name = data["name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    let meta = BuildMeta {
        r#type: "widget".to_string(),
        action: "install".to_string(),
        entity_id: installed_id.clone(),
        entity_name: Some(widget_name),
        undo_command: format!("neomind widget uninstall {}", installed_id),
    };

    Ok(CliResponse::success_with_meta(data, "Widget installed from marketplace", meta))
}
