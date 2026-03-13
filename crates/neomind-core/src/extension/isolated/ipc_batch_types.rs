//! Batch execution types for IPC
//!
//! This module defines types for batch command execution,
//! allowing multiple commands to be sent in a single IPC message.

use serde::{Deserialize, Serialize};

/// Single command in a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCommand {
    /// Command name
    pub command: String,
    /// Command arguments
    pub args: serde_json::Value,
}

/// Result of a single command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Command that was executed
    pub command: String,
    /// Whether execution was successful
    pub success: bool,
    /// Result data (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub elapsed_ms: f64,
}

/// Container for batch execution results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResultsVec {
    /// Individual command results
    pub results: Vec<BatchResult>,
    /// Total execution time in milliseconds
    pub total_elapsed_ms: f64,
}
