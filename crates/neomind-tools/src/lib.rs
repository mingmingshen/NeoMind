//! Edge AI Tools Crate
//!
//! This crate provides function calling capabilities for the NeoMind platform.
//!
//! ## Tool Capabilities
//!
//! - **Tool Trait**: Unified interface for tool implementation
//! - **Device Tools**: Query, control, and manage IoT devices
//! - **Rule Tools**: Create and manage automation rules
//! - **Agent Tools**: AI agent management and execution
//! - **System Tools**: System info, alerts, and data export
//! - **Tool Registry**: Manage and execute tools
//! - **Parallel Execution**: Execute multiple tools concurrently
//! - **LLM Integration**: Format tool definitions for function calling
//!
//! ## Example
//!
//! ```rust,no_run
//! use neomind_tools::{ToolRegistry, ToolRegistryBuilder};
//! use neomind_devices::{DeviceService, TimeSeriesStorage};
//! use neomind_rules::RuleEngine;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let device_service = Arc::new(DeviceService::new());
//!     let storage = Arc::new(TimeSeriesStorage::memory()?);
//!     let rule_engine = Arc::new(RuleEngine::new());
//!
//!     // Create a registry with core_tools (preferred)
//!     let registry = ToolRegistryBuilder::new()
//!         .with_device_discover_tool_with_storage(device_service.clone(), storage.clone())
//!         .with_device_query_tool_with_storage(device_service.clone(), storage.clone())
//!         .with_core_device_control_tool_with_storage(device_service.clone(), storage.clone())
//!         .with_create_rule_tool(rule_engine)
//!         .build();
//!
//!     // List available tools
//!     println!("Available tools: {:?}", registry.list());
//!
//!     // Execute a tool
//!     let result = registry.execute(
//!         "device.query",
//!         serde_json::json!({"device_id": "sensor_1", "metric": "temperature"})
//!     ).await?;
//!
//!     println!("Result: {:?}", result);
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

pub mod agent_tools;
pub mod core_tools;
pub mod error;
pub mod extension_tools;
pub mod real;
pub mod registry;
pub mod system_tools;
pub mod tool;
pub mod simplified;

// Re-exports commonly used types
pub use error::{NeoMindError, Result, ToolError};
pub use registry::{ToolCall, ToolRegistry, ToolRegistryBuilder, ToolResult, format_for_llm};
pub use tool::{DynTool, Parameter, Tool, ToolDefinition, ToolExample, ToolOutput};
pub use simplified::{ErrorMessages, Example, FriendlyError, LlmToolDefinition, SimplifiedConfig,
                   format_tools_as_json, format_tools_for_llm, get_simplified_tools};

// Type aliases to reduce complexity
pub type SharedToolRegistry = Arc<ToolRegistry>;
pub type ToolResultList = Vec<Result<ToolOutput>>;
pub type ToolCallList = Vec<ToolCall>;

// Re-exports from core (backward compatibility)
pub use neomind_core::tools::{
    Parameter as CoreParameter, Tool as CoreTool, ToolDefinition as CoreToolDefinition,
    ToolError as CoreToolError, ToolFactory, ToolOutput as CoreToolOutput, array_property,
    boolean_property, number_property, object_schema, property, string_property,
};

// ============================================================================
// Real Tools (Primary Exports)
// ============================================================================
/// Device tools (QueryDataTool and GetDeviceDataTool remain - core_tools may not fully replace them yet)
pub use real::{
    QueryDataTool, GetDeviceDataTool, QueryRuleHistoryTool,
};

/// Rule tools (CreateRuleTool, ListRulesTool, DeleteRuleTool - no core_tools equivalent yet)
pub use real::{
    CreateRuleTool, ListRulesTool, DeleteRuleTool,
};

// ============================================================================
// Core Business-Scenario Tools
// ============================================================================

/// Core business-scenario tools with device registry abstraction
pub use core_tools::{
    // Device tools
    DeviceDiscoverTool, DeviceQueryTool, DeviceControlTool,
    DeviceAnalyzeTool as CoreDeviceAnalyzeTool,
    // Rule tools
    RuleFromContextTool,
    // Types
    DeviceInfo as CoreDeviceInfo, DeviceCapabilities, DeviceFilter,
    DeviceGroup, DiscoverySummary,
    MetricInfo as CoreMetricInfo, CommandInfo as CoreCommandInfo, ParameterInfo,
    // Query types
    TimeRange, DataPoint as CoreDataPoint, MetricQueryResult, MetricStatistics,
    // Control types
    ControlCommand, ControlResult, BatchControlResult,
    // Analysis types
    AnalysisType, AnalysisResult,
    // Rule types
    ExtractedRuleDefinition, RuleActionDef,
    // Registry trait and adapters
    DeviceRegistryTrait, RealDeviceRegistryAdapter, DeviceRegistryAdapter,
};

// ============================================================================
// System Management Tools
// ============================================================================

pub use system_tools::{
    // System tools
    SystemInfoTool, SystemConfigTool, ServiceRestartTool, SystemHelpTool,
    // Alert tools
    CreateAlertTool, ListAlertsTool, AcknowledgeAlertTool, AlertInfo, AlertSeverity,
    // Export tools
    ExportToCsvTool, ExportToJsonTool, GenerateReportTool,
};

// ============================================================================
// AI Agent Tools
// ============================================================================

pub use agent_tools::{
    ListAgentsTool, GetAgentTool, ExecuteAgentTool, ControlAgentTool,
    CreateAgentTool, AgentMemoryTool,
    GetAgentExecutionsTool, GetAgentExecutionDetailTool, GetAgentConversationTool,
};

// ============================================================================
// Extension Tools
// ============================================================================

pub use extension_tools::{
    ExtensionTool, ExtensionToolGenerator, ExtensionToolExecutor,
    ExtensionFilter,
};

// Note: ExtensionRegistry is now defined in neomind_core::extension
// and is re-exported from neomind_core directly

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
        let registry = ToolRegistryBuilder::new()
            .with_system_help_tool()
            .build();

        assert!(registry.len() >= 1);
        assert!(registry.has("system_help"));
    }
}
