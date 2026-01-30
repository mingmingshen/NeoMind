//! LLM generation handler for one-shot LLM requests.
//! Used for features like AI-assisted MDL generation.

use axum::{extract::State, Json};
use serde_json::json;

use super::{
    ServerState,
    common::{HandlerResult, ok},
};
use crate::models::ErrorResponse;

/// Request body for LLM generation.
#[derive(serde::Deserialize)]
pub struct LlmGenerateRequest {
    pub prompt: String,
}

/// Generate LLM response (one-shot, no session required).
/// This bypasses the agent's tool calling pipeline and calls LLM directly.
/// Useful for features like AI-assisted MDL generation.
pub async fn llm_generate_handler(
    State(_state): State<ServerState>,
    Json(req): Json<LlmGenerateRequest>,
) -> HandlerResult<serde_json::Value> {
    use edge_ai_agent::LlmBackend;
    use edge_ai_core::{
        Message,
        llm::backend::{GenerationParams, LlmInput, LlmRuntime},
    };

    // Load current LLM backend configuration
    let backend_config = crate::config::load_llm_config().ok_or_else(|| {
        ErrorResponse::bad_request("LLM not configured. Please configure LLM settings first.")
    })?;

    // Convert LlmBackend to a Box<dyn LlmRuntime>
    let (llm_runtime, model_name): (Box<dyn LlmRuntime>, String) = match backend_config {
        LlmBackend::Ollama { endpoint, model } => {
            use edge_ai_llm::{OllamaConfig, OllamaRuntime};
            let config = OllamaConfig::new(&model).with_endpoint(&endpoint);
            let runtime = OllamaRuntime::new(config).map_err(|e| {
                ErrorResponse::internal(format!("Failed to create Ollama runtime: {}", e))
            })?;
            (Box::new(runtime) as Box<dyn LlmRuntime>, model)
        }
        LlmBackend::OpenAi {
            api_key,
            endpoint,
            model,
        } => {
            use edge_ai_llm::{CloudConfig, CloudRuntime};
            let config = if endpoint.is_empty() || endpoint.contains("api.openai.com") {
                CloudConfig::openai(&api_key).with_model(&model)
            } else {
                CloudConfig::custom(&api_key, &endpoint).with_model(&model)
            };
            let runtime = CloudRuntime::new(config).map_err(|e| {
                ErrorResponse::internal(format!("Failed to create Cloud runtime: {}", e))
            })?;
            (Box::new(runtime) as Box<dyn LlmRuntime>, model)
        }
    };

    // Build the input with system prompt
    let system_prompt = "You are a helpful assistant.";
    let input = LlmInput {
        messages: vec![Message::system(system_prompt), Message::user(&req.prompt)],
        params: GenerationParams {
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: None,
            max_tokens: Some(usize::MAX),
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
            thinking_enabled: None,
            max_context: None,
        },
        model: Some(model_name),
        stream: false,
        tools: None,
    };

    let start = std::time::Instant::now();

    // Call LLM directly (bypassing agent's tool calling)
    let output = llm_runtime
        .generate(input)
        .await
        .map_err(|e| ErrorResponse::internal(format!("LLM generation failed: {}", e)))?;

    let latency_ms = start.elapsed().as_millis();

    ok(json!({
        "response": output.text,
        "thinking": null,
        "tools_used": [],
        "processing_time_ms": latency_ms,
    }))
}
