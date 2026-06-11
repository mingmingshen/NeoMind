//! Resource registry for semantic mapping.
//!
//! Provides resource indexing for natural-language → technical ID resolution
//! (e.g., "客厅灯" → device_id). Used by `SemanticMapper` in the agent crate.

mod device_registry;
mod resource_index;

pub use device_registry::{DeviceCapability, DeviceRegistry};
pub use resource_index::{
    AccessType, AlertChannelResourceData, Capability, CapabilityType, DeviceResourceData,
    DeviceTypeResourceData, Resource, ResourceData, ResourceDataHelper, ResourceId, ResourceIndex,
    ResourceIndexStats, SearchQuery, SearchResult,
};

use std::sync::Arc;

// Type aliases to reduce complexity
pub type SharedDeviceRegistry = Arc<DeviceRegistry>;
pub type SharedResourceIndex = Arc<RwLock<ResourceIndex>>;

use tokio::sync::RwLock;
