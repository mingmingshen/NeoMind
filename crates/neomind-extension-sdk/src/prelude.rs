//! NeoMind Extension SDK Prelude
//!
//! Common imports for extension development.
//!
//! # Usage
//!
//! ```rust,ignore
//! use neomind_extension_sdk::prelude::*;
//! ```

// Re-export async_trait
pub use async_trait::async_trait;

// Re-export serde_json helpers
pub use serde_json::{json, Value};

// Core types from host module (Native target)
#[cfg(not(target_arch = "wasm32"))]
pub use crate::host::{
    Extension, ExtensionCapability, CapabilityContext, CapabilityManifest,
    ExtensionContext, ExtensionContextConfig, AvailableCapabilities,
    StreamCapability, StreamSession, StreamMode, StreamDirection,
    DataChunk, StreamResult, StreamError, SessionStats,
    ClientInfo, FlowControl, StreamDataType,
};

// IPC types (stable ABI)
pub use crate::ipc_types::{
    ABI_VERSION, CExtensionMetadata, ExtensionError, ExtensionMetadata,
    ExtensionMetricValue, MetricDataType, MetricDescriptor, ParamMetricValue,
    ParameterDefinition, ExtensionDescriptor, ExtensionCommand,
    ExtensionRuntimeState, ExtensionStats, Result, PushOutputMessage,
    CommandDefinition, ValidationRule, ParameterGroup,
};

// SDK helper types (for backward compatibility)
pub use crate::ipc_types::{
    ExtensionMetadata as SdkExtensionMetadata,
    MetricDescriptor as SdkMetricDefinition,
    MetricDataType as SdkMetricDataType,
    ParamMetricValue as SdkMetricValue,
    ExtensionError as SdkExtensionError,
    Result as SdkResult,
    ExtensionCommand as SdkCommandDefinition,
    ParameterDefinition as SdkParameterDefinition,
};

// Frontend types
pub use crate::FrontendManifest;

// Helper types
pub use crate::{MetricBuilder, CommandBuilder};

// Macros
pub use crate::{
    neomind_export,
    static_metadata, static_metrics, static_commands,
    metric_value, metric_float, metric_int, metric_bool, metric_string,
    ext_log, ext_debug, ext_info, ext_warn, ext_error,
};

// SDK constants
pub use crate::{SDK_VERSION, SDK_ABI_VERSION, MIN_NEOMIND_VERSION};

// Semver for version handling
pub use semver::Version;

// Re-export chrono for timestamp handling
#[cfg(not(target_arch = "wasm32"))]
pub use chrono;

// Tokio sync for Push mode
#[cfg(not(target_arch = "wasm32"))]
pub use tokio::sync::mpsc;
