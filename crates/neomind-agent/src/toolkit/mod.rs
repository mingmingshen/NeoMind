//! Edge AI Tools Crate
//!
//! This crate provides function calling capabilities for the NeoMind platform.
//!
//! ## Tool Architecture
//!
//! The toolkit uses an **action-based aggregated design** for token efficiency:
//!
//! - **6 Aggregated Tools**: device, agent, agent_history, rule, alert, extension
//! - Each tool supports multiple actions (list, get, create, control, etc.)
//! - Reduces tool definition token usage by ~60% vs individual tools
//!
//! ## Example
//!
//! ```rust,no_run
//! use neomind_agent::toolkit::{ToolRegistryBuilder, ToolRegistry};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a registry with aggregated tools
//!     let registry = ToolRegistryBuilder::new()
//!         .with_system_help_tool_named("NeoMind")
//!         .build();
//!
//!     // List available tools
//!     println!("Available tools: {:?}", registry.list());
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

pub mod aggregated;
pub mod error;
pub mod extension_tools;
pub mod registry;
pub mod resolver;
pub mod session_search;
pub mod simplified;
pub mod system_tools;
pub mod tool;

// Re-exports commonly used types
pub use error::{NeoMindError, Result, ToolError};
pub use registry::{format_for_llm, ToolCall, ToolRegistry, ToolRegistryBuilder, ToolResult};
pub use simplified::{
    format_tools_as_json, format_tools_for_llm, get_simplified_tools, ErrorMessages, Example,
    FriendlyError, LlmToolDefinition, SimplifiedConfig,
};
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

// ============================================================================
// System Tools (Help/Onboarding)
// ============================================================================

pub use system_tools::{SystemHelpTool, SystemInfoTool};

// ============================================================================
// Extension Tools
// ============================================================================

pub use extension_tools::{
    ExtensionFilter, ExtensionTool, ExtensionToolExecutor, ExtensionToolGenerator,
};

pub use session_search::SessionSearchTool;

// ============================================================================
// Aggregated Tools (Action-based design for token efficiency)
// ============================================================================

pub use aggregated::{
    AgentHistoryTool, AgentTool, AggregatedMessageInfo, AggregatedMessageLevel,
    AggregatedToolsBuilder, DeviceTool, ExtensionAggregatedTool, MessageTool, RuleTool,
};

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

    #[tokio::test]
    async fn test_registry_with_system_help() {
        let registry = ToolRegistryBuilder::new().with_system_help_tool().build();

        assert!(!registry.is_empty());
        assert!(registry.has("system_help"));
    }
}
