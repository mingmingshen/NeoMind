use anyhow::Result;
use serde_json::json;
use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

/// List all dashboards
pub async fn list_dashboards(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/dashboards").await?;
    Ok(CliResponse::success(data, "Dashboards listed"))
}

/// Get dashboard by ID
pub async fn get_dashboard(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/dashboards/{}", id)).await?;
    Ok(CliResponse::success(data, "Dashboard retrieved"))
}

/// Create a new dashboard
pub async fn create_dashboard(
    client: &ApiClient,
    name: &str,
    description: Option<&str>,
    layout: Option<serde_json::Value>,
) -> Result<CliResponse> {
    let mut body = json!({
        "name": name,
    });
    if let Some(desc) = description {
        body["description"] = json!(desc);
    }
    if let Some(layout_value) = layout {
        body["layout"] = layout_value;
    }

    let data = client.post("/dashboards", &body).await?;
    let dashboard_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let meta = BuildMeta {
        r#type: "dashboard".to_string(),
        action: "create".to_string(),
        entity_id: dashboard_id.clone(),
        entity_name: Some(name.to_string()),
        undo_command: format!("neomind dashboard delete {}", dashboard_id),
    };

    Ok(CliResponse::success_with_meta(data, "Dashboard created", meta))
}

/// Update dashboard
pub async fn update_dashboard(
    client: &ApiClient,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
    layout: Option<serde_json::Value>,
    components: Option<serde_json::Value>,
) -> Result<CliResponse> {
    let mut body = json!({});
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(desc) = description {
        body["description"] = json!(desc);
    }
    if let Some(layout_value) = layout {
        body["layout"] = layout_value;
    }
    if let Some(components_value) = components {
        body["components"] = components_value;
    }

    let data = client.put(&format!("/dashboards/{}", id), &body).await?;
    Ok(CliResponse::success(data, "Dashboard updated"))
}

/// Delete dashboard
pub async fn delete_dashboard(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/dashboards/{}", id)).await?;
    Ok(CliResponse::success(
        json!({ "id": id }),
        "Dashboard deleted",
    ))
}

/// Share dashboard
pub async fn share_dashboard(
    client: &ApiClient,
    id: &str,
    public: bool,
    expires: Option<&str>,
) -> Result<CliResponse> {
    let mut body = json!({
        "public": public,
    });
    if let Some(exp) = expires {
        body["expires"] = json!(exp);
    }

    let data = client.post(&format!("/dashboards/{}/share", id), &body).await?;
    Ok(CliResponse::success(data, "Dashboard shared"))
}
