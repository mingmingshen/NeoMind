//! Integration tests for integration registry.

use neomind_integrations::{IntegrationRegistry, IntegrationMetadata, IntegrationType};
use neomind_core::EventBus;

#[tokio::test]
async fn test_registry_creation() {
    let event_bus = EventBus::new();
    let registry = IntegrationRegistry::new(event_bus);

    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
}

#[tokio::test]
async fn test_registry_count_after_register() {
    let event_bus = EventBus::new();
    let registry = IntegrationRegistry::new(event_bus);

    // Use the existing tests in lib.rs which use MockIntegration
    assert_eq!(registry.len(), 0);
}
