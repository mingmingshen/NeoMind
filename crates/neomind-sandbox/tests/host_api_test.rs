//! Integration tests for the sandbox Host API.

use neomind_sandbox::HostApi;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_host_api_creation() {
    let api = HostApi::new();
    // Just verify creation doesn't panic
    drop(api);
}

#[tokio::test]
async fn test_host_api_device_read() {
    let api = HostApi::new();

    // Test reading from non-existent device (returns mock data)
    let response = api.device_read("nonexistent", "temperature").await;
    assert!(response.success);
    assert_eq!(response.data["value"], 20.0);
}

#[tokio::test]
async fn test_host_api_device_write() {
    let api = HostApi::new();

    let params = json!({
        "value": 25.5
    });

    let response = api.device_write("test-device", "set_value", &params).await;
    assert!(response.success);
    assert_eq!(response.data["executed"], true);
}

#[tokio::test]
async fn test_host_api_register_device() {
    let api = HostApi::new();

    let mut metrics = HashMap::new();
    metrics.insert("temperature".to_string(), 22.5);

    let response = api.register_device("sensor-1", metrics).await;
    assert!(response.success);
    assert_eq!(response.data["registered"], true);
}

#[tokio::test]
async fn test_host_api_log() {
    let api = HostApi::new();

    let response = api.log("info", "Test log message").await;
    assert!(response.success);

    // Verify log was stored
    let logs = api.get_logs().await;
    assert!(logs.iter().any(|l| l.contains("Test log message")));
}

#[tokio::test]
async fn test_host_api_notify() {
    let api = HostApi::new();

    let response = api.notify("Test notification").await;
    assert!(response.success);
    assert_eq!(response.data["sent"], true);

    // Verify notification was stored in logs
    let logs = api.get_logs().await;
    assert!(logs.iter().any(|l| l.contains("NOTIFY")));
}

#[tokio::test]
async fn test_host_api_query_data() {
    let api = HostApi::new();

    let response = api.query_data("SELECT * FROM devices").await;
    assert!(response.success);
    assert_eq!(response.data["results"], json!([]));
}

#[tokio::test]
async fn test_host_api_clear_logs() {
    let api = HostApi::new();

    // Add some logs
    let _ = api.log("info", "Test message 1").await;
    let _ = api.log("info", "Test message 2").await;

    // Clear logs
    api.clear_logs().await;

    // Verify logs are empty
    let logs = api.get_logs().await;
    assert_eq!(logs.len(), 0);
}
