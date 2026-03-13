//! Extension Context - decoupled system access with capability-based approach
//!
//! This module provides a stable, decoupled extension context that:
//! - Is extensible without modifying core system code
//! - Supports multiple API versions共存
//! - Uses capability declarations for extensibility
//! - Maintains backward compatibility

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::event::NeoMindEvent;
use crate::EventBus;

// ============================================================================
// Capability Definition Macro
// ============================================================================

/// Macro to define extension capabilities in a single place.
/// This ensures consistency between Core and SDK WASM definitions.
macro_rules! define_capabilities {
    ($($variant:ident => $const_name:ident => $name:literal => $doc:literal),* $(,)?) => {
        /// Extension API capability declaration
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
        pub enum ExtensionCapability {
            $(
                #[doc = $doc]
                #[serde(rename = $name)]
                $variant,
            )*
            /// Custom capability (for future extensibility)
            #[serde(rename = "custom")]
            Custom(String),
        }

        impl ExtensionCapability {
            /// Check if this is a custom capability
            pub fn is_custom(&self) -> bool {
                matches!(self, ExtensionCapability::Custom(_))
            }

            /// Get the name of this capability
            pub fn name(&self) -> String {
                match self {
                    $(ExtensionCapability::$variant => $name.to_string(),)*
                    ExtensionCapability::Custom(name) => name.clone(),
                }
            }

            /// Get all standard capabilities
            pub fn all_capabilities() -> Vec<Self> {
                vec![
                    $(ExtensionCapability::$variant,)*
                ]
            }

            /// Parse a capability from its name
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(ExtensionCapability::$variant),)*
                    _ => Some(ExtensionCapability::Custom(name.to_string())),
                }
            }
        }

        /// Capability name constants for SDK WASM compatibility
        pub mod capabilities {
            $(pub const $const_name: &str = $name;)*
        }
    };
}

// Define all standard capabilities
define_capabilities! {
    DeviceMetricsRead => DEVICE_METRICS_READ => "device_metrics_read" => "Access to device metrics (read current state)",
    DeviceMetricsWrite => DEVICE_METRICS_WRITE => "device_metrics_write" => "Access to write device metrics (including virtual metrics)",
    DeviceControl => DEVICE_CONTROL => "device_control" => "Access to control devices (send commands)",
    StorageQuery => STORAGE_QUERY => "storage_query" => "Access to storage queries (read telemetry)",
    EventPublish => EVENT_PUBLISH => "event_publish" => "Access to publish events to EventBus",
    EventSubscribe => EVENT_SUBSCRIBE => "event_subscribe" => "Access to subscribe to events from EventBus",
    TelemetryHistory => TELEMETRY_HISTORY => "telemetry_history" => "Access to query device telemetry history",
    MetricsAggregate => METRICS_AGGREGATE => "metrics_aggregate" => "Access to aggregate device metrics",
    ExtensionCall => EXTENSION_CALL => "extension_call" => "Access to call other extensions",
    AgentTrigger => AGENT_TRIGGER => "agent_trigger" => "Access to trigger agents",
    RuleTrigger => RULE_TRIGGER => "rule_trigger" => "Access to trigger rules",
}

// ============================================================================
// Additional Capability Methods
// ============================================================================

impl ExtensionCapability {
    /// Get display name for this capability
    pub fn display_name(&self) -> String {
        match self {
            ExtensionCapability::DeviceMetricsRead => "Device Metrics Read".to_string(),
            ExtensionCapability::DeviceMetricsWrite => "Device Metrics Write".to_string(),
            ExtensionCapability::DeviceControl => "Device Control".to_string(),
            ExtensionCapability::StorageQuery => "Storage Query".to_string(),
            ExtensionCapability::EventPublish => "Event Publish".to_string(),
            ExtensionCapability::EventSubscribe => "Event Subscribe".to_string(),
            ExtensionCapability::TelemetryHistory => "Telemetry History".to_string(),
            ExtensionCapability::MetricsAggregate => "Metrics Aggregate".to_string(),
            ExtensionCapability::ExtensionCall => "Extension Call".to_string(),
            ExtensionCapability::AgentTrigger => "Agent Trigger".to_string(),
            ExtensionCapability::RuleTrigger => "Rule Trigger".to_string(),
            ExtensionCapability::Custom(name) => format!("Custom: {}", name),
        }
    }

    /// Get description for this capability
    pub fn description(&self) -> String {
        match self {
            ExtensionCapability::DeviceMetricsRead => "Read current device metrics and state".to_string(),
            ExtensionCapability::DeviceMetricsWrite => "Write device metrics including virtual metrics".to_string(),
            ExtensionCapability::DeviceControl => "Send commands to control devices".to_string(),
            ExtensionCapability::StorageQuery => "Query stored telemetry data".to_string(),
            ExtensionCapability::EventPublish => "Publish events to the event bus".to_string(),
            ExtensionCapability::EventSubscribe => "Subscribe to events from the event bus".to_string(),
            ExtensionCapability::TelemetryHistory => "Query device telemetry history data".to_string(),
            ExtensionCapability::MetricsAggregate => "Aggregate and calculate device metrics".to_string(),
            ExtensionCapability::ExtensionCall => "Call other extensions".to_string(),
            ExtensionCapability::AgentTrigger => "Trigger AI agent execution".to_string(),
            ExtensionCapability::RuleTrigger => "Trigger rule engine execution".to_string(),
            ExtensionCapability::Custom(_) => "Custom capability".to_string(),
        }
    }

    /// Get category for this capability
    pub fn category(&self) -> String {
        match self {
            ExtensionCapability::DeviceMetricsRead
            | ExtensionCapability::DeviceMetricsWrite
            | ExtensionCapability::DeviceControl => "device".to_string(),
            ExtensionCapability::StorageQuery => "storage".to_string(),
            ExtensionCapability::EventPublish | ExtensionCapability::EventSubscribe => "event".to_string(),
            ExtensionCapability::TelemetryHistory | ExtensionCapability::MetricsAggregate => "telemetry".to_string(),
            ExtensionCapability::ExtensionCall => "extension".to_string(),
            ExtensionCapability::AgentTrigger => "agent".to_string(),
            ExtensionCapability::RuleTrigger => "rule".to_string(),
            ExtensionCapability::Custom(_) => "custom".to_string(),
        }
    }
}

/// Extension capability manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityManifest {
    /// List of capabilities this extension package provides
    pub capabilities: Vec<ExtensionCapability>,
    /// API version
    pub api_version: String,
    /// Minimum core version required
    pub min_core_version: String,
    /// Package name
    pub package_name: String,
}

/// Extension context configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionContextConfig {
    /// API base URL for extension calls
    #[serde(default)]
    pub api_base_url: String,
    /// API version to use
    pub api_version: String,
    /// Required capabilities
    #[serde(default)]
    pub required_capabilities: Vec<ExtensionCapability>,
    /// Extension ID
    pub extension_id: String,
    /// Rate limit
    #[serde(default)]
    pub rate_limit: Option<usize>,
}

impl Default for ExtensionContextConfig {
    fn default() -> Self {
        Self {
            api_base_url: String::new(),
            api_version: "v1".to_string(),
            required_capabilities: Vec::new(),
            extension_id: String::new(),
            rate_limit: None,
        }
    }
}

/// Available capabilities in the system
#[derive(Debug, Clone, Default)]
pub struct AvailableCapabilities {
    capabilities: HashMap<ExtensionCapability, (String, String)>,
}

impl AvailableCapabilities {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    pub fn register_capability(
        &mut self,
        capability: ExtensionCapability,
        package_name: String,
        api_version: String,
    ) {
        self.capabilities.insert(capability, (package_name, api_version));
    }

    pub fn has_capability(&self, capability: &ExtensionCapability) -> bool {
        self.capabilities.contains_key(capability)
    }

    pub fn get_provider(&self, capability: &ExtensionCapability) -> Option<(String, String)> {
        self.capabilities.get(capability).cloned()
    }
}

/// Extension capability provider trait
#[async_trait::async_trait]
pub trait ExtensionCapabilityProvider: Send + Sync {
    fn capability_manifest(&self) -> CapabilityManifest;
    async fn invoke_capability(
        &self,
        capability: ExtensionCapability,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, CapabilityError>;
}

/// Error type for capability invocations
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("Capability not available: {0:?}")]
    NotAvailable(ExtensionCapability),
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Provider not found for capability: {0:?}")]
    ProviderNotFound(ExtensionCapability),
}

/// Extension context
#[derive(Clone)]
pub struct ExtensionContext {
    config: ExtensionContextConfig,
    event_bus: Option<Arc<EventBus>>,
    available_capabilities: Arc<RwLock<AvailableCapabilities>>,
    providers: Arc<RwLock<HashMap<String, Arc<dyn ExtensionCapabilityProvider>>>>,
}

impl ExtensionContext {
    pub fn new(
        config: ExtensionContextConfig,
        event_bus: Option<Arc<EventBus>>,
        providers: Arc<RwLock<HashMap<String, Arc<dyn ExtensionCapabilityProvider>>>>,
    ) -> Self {
        Self {
            config,
            event_bus,
            available_capabilities: Arc::new(RwLock::new(AvailableCapabilities::new())),
            providers,
        }
    }

    pub fn with_defaults(
        extension_id: String,
        api_base_url: String,
        event_bus: Option<Arc<EventBus>>,
        providers: Arc<RwLock<HashMap<String, Arc<dyn ExtensionCapabilityProvider>>>>,
    ) -> Self {
        Self::new(
            ExtensionContextConfig {
                extension_id,
                api_base_url,
                ..Default::default()
            },
            event_bus,
            providers,
        )
    }

    pub fn extension_id(&self) -> &str {
        &self.config.extension_id
    }

    pub async fn register_provider(
        &self,
        package_name: String,
        provider: Arc<dyn ExtensionCapabilityProvider>,
    ) {
        let manifest = provider.capability_manifest();
        let mut available = self.available_capabilities.write().await;
        for capability in &manifest.capabilities {
            available.register_capability(
                capability.clone(),
                package_name.clone(),
                manifest.api_version.clone(),
            );
        }
        let mut providers = self.providers.write().await;
        providers.insert(package_name, provider);
    }

    pub async fn invoke_capability(
        &self,
        capability: ExtensionCapability,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, CapabilityError> {
        if !self.config.required_capabilities.contains(&capability) {
            return Err(CapabilityError::PermissionDenied(format!(
                "Extension '{}' does not have capability '{:?}'",
                self.config.extension_id, capability
            )));
        }

        let available = self.available_capabilities.read().await;
        let (package_name, _) = available
            .get_provider(&capability)
            .ok_or_else(|| CapabilityError::ProviderNotFound(capability.clone()))?;

        let providers = self.providers.read().await;
        let provider = providers
            .get(&package_name)
            .ok_or_else(|| CapabilityError::ProviderError(format!("Provider '{}' not found", package_name)))?;

        provider.invoke_capability(capability, params).await
    }

    pub async fn has_capability(&self, capability: &ExtensionCapability) -> bool {
        let available = self.available_capabilities.read().await;
        available.has_capability(capability)
    }

    /// List all available capabilities
    pub async fn list_capabilities(&self) -> Vec<(ExtensionCapability, String, String)> {
        let available = self.available_capabilities.read().await;
        available
            .capabilities
            .iter()
            .map(|(cap, (pkg, ver))| (cap.clone(), pkg.clone(), ver.clone()))
            .collect()
    }

    /// Get the event bus if available
    pub fn event_bus(&self) -> Option<Arc<EventBus>> {
        self.event_bus.clone()
    }

    /// Publish an event to the event bus
    ///
    /// Requires EventPublish capability
    pub fn publish_event_sync(
        &self,
        event: NeoMindEvent,
    ) -> Result<(), CapabilityError> {
        // Check permission
        if !self.config.required_capabilities.contains(&ExtensionCapability::EventPublish) {
            return Err(CapabilityError::PermissionDenied(format!(
                "Extension '{}' does not have EventPublish capability",
                self.config.extension_id
            )));
        }

        // Get event bus
        let event_bus = self.event_bus.as_ref()
            .ok_or(CapabilityError::NotAvailable(ExtensionCapability::EventPublish))?;

        // Publish event (sync version)
        event_bus.publish_sync(event);
        Ok(())
    }

    /// Subscribe to events from the event bus
    ///
    /// Requires EventSubscribe capability
    /// Returns a receiver that can be polled for events
    pub fn subscribe_events(
        &self,
        event_types: Vec<String>,
    ) -> Result<crate::eventbus::FilteredReceiver<impl Fn(&NeoMindEvent) -> bool + Send + Clone>, CapabilityError> {
        // Check permission
        if !self.config.required_capabilities.contains(&ExtensionCapability::EventSubscribe) {
            return Err(CapabilityError::PermissionDenied(format!(
                "Extension '{}' does not have EventSubscribe capability",
                self.config.extension_id
            )));
        }

        // Get event bus
        let event_bus = self.event_bus.as_ref()
            .ok_or(CapabilityError::NotAvailable(ExtensionCapability::EventSubscribe))?;

        // Create filter closure
        let event_types_clone = event_types.clone();
        let filter = move |event: &NeoMindEvent| {
            let type_name = event.type_name();
            event_types_clone.iter().any(|t| type_name == t || type_name.starts_with(&format!("{}::", t)))
        };

        // Subscribe with filter
        Ok(event_bus.subscribe_filtered(filter))
    }

    /// Get the configuration
    pub fn config(&self) -> &ExtensionContextConfig {
        &self.config
    }

    /// Check if a capability is granted to this extension
    pub fn has_capability_granted(&self, capability: &ExtensionCapability) -> bool {
        self.config.required_capabilities.contains(capability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that all capability names match between Core and SDK WASM
    /// This ensures synchronization between:
    /// - neomind-core/src/extension/context.rs (ExtensionCapability enum)
    /// - neomind-extension-sdk/src/wasm/context.rs (capabilities module)
    #[test]
    fn test_capability_names_sync() {
        // Standard capabilities - must match SDK WASM capabilities module
        let expected_names = [
            "device_metrics_read",
            "device_metrics_write",
            "device_control",
            "storage_query",
            "event_publish",
            "event_subscribe",
            "telemetry_history",
            "metrics_aggregate",
            "extension_call",
            "agent_trigger",
            "rule_trigger",
        ];

        let all_caps = ExtensionCapability::all_capabilities();
        assert_eq!(all_caps.len(), expected_names.len(), 
            "Capability count mismatch: Core has {}, expected {}", 
            all_caps.len(), expected_names.len());

        for (cap, expected_name) in all_caps.iter().zip(expected_names.iter()) {
            assert_eq!(cap.name(), *expected_name,
                "Capability name mismatch: {:?} should be {}", cap, expected_name);
        }
    }

    #[test]
    fn test_capability_from_name() {
        // Test standard capabilities
        assert!(matches!(
            ExtensionCapability::from_name("device_metrics_read"),
            Some(ExtensionCapability::DeviceMetricsRead)
        ));
        
        // Test custom capability
        assert!(matches!(
            ExtensionCapability::from_name("my_custom_capability"),
            Some(ExtensionCapability::Custom(_))
        ));
    }

    #[test]
    fn test_capability_serialization() {
        // Test that serialization uses snake_case
        let cap = ExtensionCapability::DeviceMetricsRead;
        let json = serde_json::to_string(&cap).unwrap();
        assert_eq!(json, r#""device_metrics_read""#);

        // Test deserialization
        let deserialized: ExtensionCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }
}