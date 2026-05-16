use anyhow::Result;
use serde_json::json;
use crate::types::CliResponse;
use crate::ApiClient;

/// List all transforms
pub async fn list_transforms(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/automations/transforms").await?;
    Ok(CliResponse::success(data, "Transforms listed"))
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
