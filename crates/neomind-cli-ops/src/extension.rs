use anyhow::Result;
use serde_json::json;
use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

/// List all extensions
pub async fn list_extensions(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/extensions").await?;
    Ok(CliResponse::success(data, "Extensions listed"))
}

/// Get extension by ID
pub async fn get_extension(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/extensions/{}", id)).await?;
    Ok(CliResponse::success(data, "Extension retrieved"))
}

/// Get extension health status
pub async fn get_extension_status(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/extensions/{}/health", id)).await?;
    Ok(CliResponse::success(data, "Extension status retrieved"))
}

/// Get extension logs
pub async fn get_extension_logs(
    client: &ApiClient,
    id: &str,
    lines: Option<usize>,
) -> Result<CliResponse> {
    let path = if let Some(n) = lines {
        format!("/extensions/{}/logs?lines={}", id, n)
    } else {
        format!("/extensions/{}/logs", id)
    };
    let data = client.get(&path).await?;
    Ok(CliResponse::success(data, "Extension logs retrieved"))
}

/// Install extension from file
pub async fn install_extension_file(
    client: &ApiClient,
    file_path: &str,
) -> Result<CliResponse> {
    // For file upload, we need to use multipart/form-data
    // This is a special case that will need custom handling in api_client
    let data = client.post_file("/extensions/upload/file", file_path).await?;
    let ext_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let meta = BuildMeta {
        r#type: "extension".to_string(),
        action: "install".to_string(),
        entity_id: ext_id.clone(),
        entity_name: ext_id.clone().into(),
        undo_command: format!("neomind extension uninstall {}", ext_id),
    };

    Ok(CliResponse::success_with_meta(data, "Extension installed", meta))
}

/// Install extension from marketplace
pub async fn install_extension_market(
    client: &ApiClient,
    extension_id: &str,
    version: Option<&str>,
) -> Result<CliResponse> {
    let mut body = json!({
        "extension_id": extension_id,
    });
    if let Some(v) = version {
        body["version"] = json!(v);
    }

    let data = client.post("/extensions/market/install", &body).await?;
    let ext_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| extension_id.to_string());

    let meta = BuildMeta {
        r#type: "extension".to_string(),
        action: "install".to_string(),
        entity_id: ext_id.clone(),
        entity_name: ext_id.clone().into(),
        undo_command: format!("neomind extension uninstall {}", ext_id),
    };

    Ok(CliResponse::success_with_meta(data, "Extension installed from marketplace", meta))
}

/// Uninstall extension
pub async fn uninstall_extension(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/extensions/{}/uninstall", id)).await?;
    Ok(CliResponse::success(
        json!({ "id": id }),
        "Extension uninstalled",
    ))
}

/// Reload an extension (restart from file)
pub async fn reload_extension(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.post_raw(&format!("/extensions/{}/reload", id)).await?;
    Ok(CliResponse::success(data, "Extension reloaded"))
}

/// List marketplace extensions
pub async fn list_marketplace(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/extensions/market/list").await?;
    Ok(CliResponse::success(data, "Marketplace extensions listed"))
}

/// Get extension configuration
pub async fn get_extension_config(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/extensions/{}/config", id)).await?;
    Ok(CliResponse::success(data, "Extension config retrieved"))
}

/// Update extension configuration
pub async fn update_extension_config(
    client: &ApiClient,
    id: &str,
    config: serde_json::Value,
) -> Result<CliResponse> {
    let data = client.put(&format!("/extensions/{}/config", id), &config).await?;
    Ok(CliResponse::success(data, "Extension config updated"))
}
