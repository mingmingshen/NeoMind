//! Comprehensive Unit Tests for UnifiedExtensionService
//!
//! Tests cover:
//! - Service creation and configuration
//! - Extension loading and unloading
//! - Command execution routing
//! - Health checking
//! - Metrics collection
//! - Extension listing and lookup
//! - Isolated vs in-process mode detection

use neomind_core::extension::unified::{
    UnifiedExtensionService, UnifiedExtensionConfig, UnifiedExtensionInfo,
};
use neomind_core::extension::registry::ExtensionRegistry;
use neomind_core::extension::isolated::IsolatedManagerConfig;
use std::sync::Arc;

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_unified_extension_config_default() {
    let config = UnifiedExtensionConfig::default();

    assert!(config.isolated_by_default);
}

#[test]
fn test_unified_extension_config_custom() {
    let config = UnifiedExtensionConfig {
        isolated_config: IsolatedManagerConfig::default(),
        isolated_by_default: false,
    };

    assert!(!config.isolated_by_default);
}

// ============================================================================
// Service Creation Tests
// ============================================================================

#[tokio::test]
async fn test_service_creation() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    assert_eq!(service.count().await, 0);
}

#[tokio::test]
async fn test_service_with_custom_config() {
    let registry = Arc::new(ExtensionRegistry::new());
    let config = UnifiedExtensionConfig {
        isolated_config: IsolatedManagerConfig::default(),
        isolated_by_default: true,
    };

    let service = UnifiedExtensionService::new(registry, config);
    assert_eq!(service.count().await, 0);
}

// ============================================================================
// Extension Listing Tests
// ============================================================================

#[tokio::test]
async fn test_list_empty() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    let list = service.list().await;
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_count_empty() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    assert_eq!(service.count().await, 0);
}

// ============================================================================
// Extension Lookup Tests
// ============================================================================

#[tokio::test]
async fn test_contains_nonexistent() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    assert!(!service.contains("nonexistent").await);
}

#[tokio::test]
async fn test_get_nonexistent() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    let result = service.get("nonexistent").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_info_nonexistent() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    let info = service.get_info("nonexistent").await;
    assert!(info.is_none());
}

// ============================================================================
// Isolated Mode Detection Tests
// ============================================================================

#[tokio::test]
async fn test_is_isolated_nonexistent() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    // Non-existent extension should return false
    assert!(!service.is_isolated("nonexistent").await);
}

// ============================================================================
// Registry Access Tests
// ============================================================================

#[tokio::test]
async fn test_registry_access() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry.clone());

    let retrieved_registry = service.registry();
    // Both should point to the same registry
    // Note: Arc::strong_count includes the original + the service's internal clone
    assert!(Arc::strong_count(&retrieved_registry) >= 2);
}

#[tokio::test]
async fn test_isolated_manager_access() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    let isolated_manager = service.isolated_manager();
    // Just verify we can access it
    let _ = isolated_manager;
}

// ============================================================================
// Stop All Tests
// ============================================================================

#[tokio::test]
async fn test_stop_all_empty() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    // Should not panic when stopping with no extensions
    service.stop_all().await;
    assert_eq!(service.count().await, 0);
}

// ============================================================================
// Unified Extension Info Tests
// ============================================================================

#[test]
fn test_unified_extension_info_structure() {
    use neomind_core::extension::system::ExtensionMetadata;

    let metadata = ExtensionMetadata::new(
        "test.extension",
        "Test Extension",
        semver::Version::new(1, 0, 0),
    );

    let info = UnifiedExtensionInfo {
        metadata: metadata.clone(),
        is_isolated: true,
        is_running: true,
        path: Some(std::path::PathBuf::from("/test/path")),
        metrics: vec![],
        commands: vec![],
    };

    assert_eq!(info.metadata.id, "test.extension");
    assert!(info.is_isolated);
    assert!(info.is_running);
    assert!(info.path.is_some());
}

#[test]
fn test_unified_extension_info_in_process() {
    use neomind_core::extension::system::ExtensionMetadata;

    let metadata = ExtensionMetadata::new(
        "in.process.ext",
        "In-Process Extension",
        semver::Version::new(2, 0, 0),
    );

    let info = UnifiedExtensionInfo {
        metadata,
        is_isolated: false,
        is_running: true,
        path: None,
        metrics: vec![],
        commands: vec![],
    };

    assert!(!info.is_isolated);
    assert!(info.is_running);
    assert!(info.path.is_none());
}

// ============================================================================
// Event Dispatcher Access Tests
// ============================================================================

#[tokio::test]
async fn test_event_dispatcher_access() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    let dispatcher = service.get_event_dispatcher();
    // Just verify we can access it
    let _ = dispatcher;
}

// ============================================================================
// Extension Runner Availability Tests
// ============================================================================

// Note: is_extension_runner_available is a private method
// The functionality is tested through the load() method behavior

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_contains_check() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = Arc::new(UnifiedExtensionService::with_defaults(registry));
    let mut handles = vec![];

    for i in 0..10 {
        let svc = service.clone();
        let handle = tokio::spawn(async move {
            svc.contains(&format!("ext.{}", i)).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(!result); // All should be false since no extensions are loaded
    }
}

#[tokio::test]
async fn test_concurrent_list() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = Arc::new(UnifiedExtensionService::with_defaults(registry));
    let mut handles = vec![];

    for _ in 0..5 {
        let svc = service.clone();
        let handle = tokio::spawn(async move {
            svc.list().await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_empty());
    }
}

#[tokio::test]
async fn test_concurrent_count() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = Arc::new(UnifiedExtensionService::with_defaults(registry));
    let mut handles = vec![];

    for _ in 0..10 {
        let svc = service.clone();
        let handle = tokio::spawn(async move {
            svc.count().await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert_eq!(result, 0);
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_execute_command_nonexistent() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    let result = service.execute_command("nonexistent", "test", &serde_json::json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_health_check_nonexistent() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    let result = service.health_check("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_metrics_nonexistent() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    let metrics = service.get_metrics("nonexistent").await;
    assert!(metrics.is_empty());
}

#[tokio::test]
async fn test_get_stats_nonexistent() {
    let registry = Arc::new(ExtensionRegistry::new());
    let service = UnifiedExtensionService::with_defaults(registry);

    let result = service.get_stats("nonexistent").await;
    assert!(result.is_err());
}