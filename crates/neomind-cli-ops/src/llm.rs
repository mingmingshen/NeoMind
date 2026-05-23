use anyhow::Result;
use crate::types::CliResponse;
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
