//! Extension type definitions (V2 - Re-exports from system.rs)
//!
//! This module re-exports the V2 extension system types.

// ============================================================================
// Re-exports from V2 system.rs
// ============================================================================

pub use super::system::{
    // Core Extension trait (V2)
    Extension,
    // Metadata types (V2)
    ExtensionMetadata,
    // Error types (V2)
    ExtensionError,
    // Result type (V2)
    Result,
    // Extension state (V2)
    ExtensionState,
    // Dynamic extension type (V2)
    DynExtension,
    // Metrics and commands (V2)
    MetricDescriptor,
    ExtensionCommand,
    ExtensionMetricValue,
    ParamMetricValue,
    MetricDataType,
    // Extension stats (V2)
    ExtensionStats,
    // C-compatible metadata (V2)
    CExtensionMetadata,
    // Tool descriptor (V2)
    ToolDescriptor,
    // ABI version (V2)
    ABI_VERSION,
};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi_version() {
        // V2 uses ABI version 2
        assert_eq!(ABI_VERSION, 2);
    }
}
