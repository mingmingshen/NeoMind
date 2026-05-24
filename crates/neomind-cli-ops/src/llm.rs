use anyhow::Result;
use serde_json::json;
use crate::types::{BuildMeta, CliResponse};
use crate::ApiClient;

/// List configured LLM backends
pub async fn list_backends(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/llm-backends").await?;
    Ok(CliResponse::success(data, "LLM backends listed"))
}

/// Get LLM backend by ID
pub async fn get_backend(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let data = client.get(&format!("/llm-backends/{}", id)).await?;
    Ok(CliResponse::success(data, "LLM backend retrieved"))
}

/// List available models from Ollama
pub async fn list_ollama_models(client: &ApiClient) -> Result<CliResponse> {
    let data = client.get("/llm-backends/ollama/models").await?;
    Ok(CliResponse::success(data, "Ollama models listed"))
}

/// Create a new LLM backend
pub async fn create_backend(
    client: &ApiClient,
    name: &str,
    backend_type: &str,
    endpoint: &str,
    model: &str,
    api_key: Option<&str>,
    temperature: Option<f64>,
) -> Result<CliResponse> {
    let mut body = json!({
        "name": name,
        "backend_type": backend_type,
        "endpoint": endpoint,
        "model": model,
    });
    if let Some(k) = api_key {
        body["api_key"] = json!(k);
    }
    if let Some(t) = temperature {
        body["temperature"] = json!(t);
    }

    let data = client.post("/llm-backends", &body).await?;
    let backend_id = data["id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| data["id"].as_i64().map(|i| i.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let meta = BuildMeta {
        r#type: "llm_backend".to_string(),
        action: "create".to_string(),
        entity_id: backend_id.clone(),
        entity_name: Some(name.to_string()),
        undo_command: format!("neomind llm delete {}", backend_id),
    };

    Ok(CliResponse::success_with_meta(data, "LLM backend created", meta))
}

/// Update an existing LLM backend
pub async fn update_backend(
    client: &ApiClient,
    id: &str,
    name: Option<&str>,
    model: Option<&str>,
    endpoint: Option<&str>,
    api_key: Option<&str>,
    temperature: Option<f64>,
) -> Result<CliResponse> {
    let mut body = json!({});
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(m) = model {
        body["model"] = json!(m);
    }
    if let Some(e) = endpoint {
        body["endpoint"] = json!(e);
    }
    if let Some(k) = api_key {
        body["api_key"] = json!(k);
    }
    if let Some(t) = temperature {
        body["temperature"] = json!(t);
    }

    let data = client.put(&format!("/llm-backends/{}", id), &body).await?;
    Ok(CliResponse::success(data, "LLM backend updated"))
}

/// Delete an LLM backend
pub async fn delete_backend(client: &ApiClient, id: &str) -> Result<CliResponse> {
    client.delete(&format!("/llm-backends/{}", id)).await?;
    Ok(CliResponse::success(
        json!({ "id": id }),
        "LLM backend deleted",
    ))
}

/// Activate an LLM backend (set as default)
pub async fn activate_backend(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let body = json!({});
    let data = client.post(&format!("/llm-backends/{}/activate", id), &body).await?;
    Ok(CliResponse::success(data, "LLM backend activated"))
}

/// Test connection to an LLM backend
pub async fn test_backend(client: &ApiClient, id: &str) -> Result<CliResponse> {
    let body = json!({});
    let data = client.post(&format!("/llm-backends/{}/test", id), &body).await?;
    Ok(CliResponse::success(data, "LLM backend test completed"))
}
