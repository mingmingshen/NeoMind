//! Integration tests for extension descriptors.

use neomind_extension_sdk::descriptor::{PluginDescriptor, PluginType, CPluginDescriptor};
use serde_json::json;

#[test]
fn test_descriptor_creation() {
    let desc = PluginDescriptor::new("test-plugin", "1.0.0");

    assert_eq!(desc.id, "test-plugin");
    assert_eq!(desc.version, "1.0.0");
    assert_eq!(desc.plugin_type, PluginType::Tool);
}

#[test]
fn test_descriptor_builder() {
    let desc = PluginDescriptor::new("my-plugin", "2.0.0")
        .with_name("My Plugin")
        .with_description("A test plugin")
        .with_author("Test Author")
        .with_required_neomind(">=1.0.0");

    assert_eq!(desc.id, "my-plugin");
    assert_eq!(desc.name, "My Plugin");
    assert_eq!(desc.description, "A test plugin");
    assert_eq!(desc.author, Some("Test Author".to_string()));
    assert_eq!(desc.required_neomind, ">=1.0.0");
}

#[test]
fn test_plugin_type_as_str() {
    assert_eq!(PluginType::Tool.as_str(), "tool");
    assert_eq!(PluginType::LlmBackend.as_str(), "llm_backend");
    assert_eq!(PluginType::DeviceAdapter.as_str(), "device_adapter");
    assert_eq!(PluginType::StorageBackend.as_str(), "storage_backend");
    assert_eq!(PluginType::Integration.as_str(), "integration");
    assert_eq!(PluginType::AlertChannel.as_str(), "alert_channel");
}

#[test]
fn test_descriptor_with_capabilities() {
    let mut desc = PluginDescriptor::new("capable-plugin", "1.0.0");

    desc = desc.with_capability(1);
    desc = desc.with_capability(2);
    desc = desc.with_capability(4);

    assert_eq!(desc.capabilities, 7); // 1 | 2 | 4
}

#[test]
fn test_c_descriptor_export() {
    let desc = PluginDescriptor::new("test-plugin", "1.0.0")
        .with_name("Test Plugin")
        .with_description("Test Description");

    let c_desc = unsafe { desc.export() };

    assert_eq!(c_desc.abi_version, 1);
    assert!(!c_desc.id.is_null());
    assert!(!c_desc.name.is_null());
    assert!(!c_desc.version.is_null());
}
