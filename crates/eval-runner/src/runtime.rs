//! Build a CloudRuntime for the agent from AGENT_LLM_* env vars.
//! Strict — panic if required env missing (avoid silent keyword-fallback).
use anyhow::Result;
use neomind_agent::llm_backends::{CloudConfig, CloudRuntime};
use std::sync::Arc;

pub fn build_agent_runtime_from_env() -> Result<Arc<CloudRuntime>> {
    let api_key = std::env::var("AGENT_LLM_API_KEY").expect(
        "AGENT_LLM_API_KEY required — refusing to run eval (would silently fall back to keyword matching)",
    );
    let endpoint = std::env::var("AGENT_LLM_ENDPOINT").expect("AGENT_LLM_ENDPOINT required");
    let model = std::env::var("AGENT_LLM_MODEL").expect("AGENT_LLM_MODEL required");
    let timeout = std::env::var("AGENT_LLM_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180);

    let cfg = CloudConfig::custom(api_key, endpoint)
        .with_model(model)
        .with_timeout_secs(timeout);
    let runtime = CloudRuntime::new(cfg)?
        .with_capabilities_override(
            false,  // multimodal: Tier 1 全是文本
            false,  // thinking: 强制关 (commit c6385169)
            true,   // tools
            32768,  // max_context
        );
    Ok(Arc::new(runtime))
}
