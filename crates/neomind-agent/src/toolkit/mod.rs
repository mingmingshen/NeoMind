//! Edge AI Tools Crate
//!
//! This crate provides function calling capabilities for the NeoMind platform.

use std::sync::Arc;

pub mod error;
pub mod extension_tools;
pub mod file_edit;
pub mod file_write;
pub mod memory_tool;
pub mod path_validator;
pub mod registry;
pub mod resolver;
pub mod shell;
pub mod skill_tool;
pub mod time_utils;
pub mod tool;
pub mod vision;
pub mod web_fetch;

// Re-exports commonly used types
pub use error::{NeoMindError, Result, ToolError};
pub use registry::{ToolCall, ToolRegistry, ToolRegistryBuilder, ToolResult};
pub use tool::{DynTool, Parameter, Tool, ToolDefinition, ToolExample, ToolOutput};

// Type aliases to reduce complexity
pub type SharedToolRegistry = Arc<ToolRegistry>;
pub type ToolResultList = Vec<Result<ToolOutput>>;
pub type ToolCallList = Vec<ToolCall>;

// Re-exports from core (backward compatibility)
pub use neomind_core::tools::{
    array_property, boolean_property, number_property, object_schema, property, string_property,
    ToolCategory, ToolRelationships, UsageScenario,
};

// Extension Tools
pub use extension_tools::{
    ExtensionFilter, ExtensionTool, ExtensionToolExecutor, ExtensionToolGenerator,
};

pub use shell::{ShellConfig, ShellTool};

pub use skill_tool::SkillTool;

pub use memory_tool::MemoryTool;

pub use file_edit::FileEditTool;
pub use file_write::FileWriteTool;
pub use vision::{VisionConfig, VisionTool};

pub use web_fetch::WebFetchTool;

pub use time_utils::{parse_time_range, TransformStore};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[tokio::test]
    async fn test_registry_empty() {
        let registry = ToolRegistryBuilder::new().build();
        assert_eq!(registry.len(), 0);
    }
}
