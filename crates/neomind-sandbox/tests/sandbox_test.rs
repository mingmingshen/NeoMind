//! Integration tests for the WASM sandbox.

use neomind_sandbox::{Sandbox, SandboxConfig};
use serde_json::json;

#[tokio::test]
async fn test_sandbox_default_config() {
    let config = SandboxConfig::default();
    assert_eq!(config.max_memory_mb, 256);
    assert_eq!(config.max_execution_time_secs, 30);
    assert!(config.allow_wasi);
}

#[tokio::test]
async fn test_sandbox_creation() {
    let sandbox = Sandbox::default();
    assert_eq!(sandbox.list_modules().await.len(), 0);
}

#[tokio::test]
async fn test_sandbox_custom_config() {
    let config = SandboxConfig {
        max_memory_mb: 512,
        max_execution_time_secs: 60,
        allow_wasi: false,
    };

    let sandbox = Sandbox::new(config).expect("Failed to create sandbox");
    assert_eq!(sandbox.list_modules().await.len(), 0);
}

#[tokio::test]
async fn test_sandbox_unload_nonexistent_module() {
    let sandbox = Sandbox::default();

    let result = sandbox.unload_module("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_sandbox_execute_nonexistent_module() {
    let sandbox = Sandbox::default();

    let result = sandbox.execute("nonexistent", "test", json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_sandbox_modules_list_empty_initially() {
    let sandbox = Sandbox::default();
    let modules = sandbox.list_modules().await;
    assert_eq!(modules.len(), 0);
    assert!(modules.is_empty());
}

#[tokio::test]
async fn test_sandbox_host_api_reference() {
    let sandbox = Sandbox::default();
    // Just verify we can get a reference to host API
    let _api = sandbox.host_api();
}
