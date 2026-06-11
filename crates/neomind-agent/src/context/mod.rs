//! Resource registry for semantic mapping.
//!
//! Provides resource indexing for natural-language → technical ID resolution
//! (e.g., "客厅灯" → device_id). Used by `SemanticMapper` in the agent crate.

mod resource_index;

pub use resource_index::{
    AccessType, AlertChannelResourceData, Capability, CapabilityType, DeviceResourceData,
    DeviceTypeResourceData, Resource, ResourceData, ResourceDataHelper, ResourceId, ResourceIndex,
    SearchResult,
};

use std::sync::Arc;

// Type aliases to reduce complexity
pub type SharedResourceIndex = Arc<RwLock<ResourceIndex>>;

use tokio::sync::RwLock;
