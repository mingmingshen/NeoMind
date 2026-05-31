//! Tool execution with event integration.
//!
//! This module provides tool execution wrappers that integrate with
//! the NeoMind event bus for tracking tool calls, recording history,
//! and handling errors.

pub mod event_integration;
pub mod interaction;
pub mod mapper;

pub use event_integration::{
    EventIntegratedToolRegistry, ToolExecutionHistory, ToolExecutionRecord, ToolExecutionStats,
    ToolRetryConfig,
};

pub use interaction::{AskUserTool, ClarifyIntentTool, ConfirmActionTool};

pub use mapper::{get_mapper, map_tool_parameters, resolve_tool_name, ToolNameMapper};
