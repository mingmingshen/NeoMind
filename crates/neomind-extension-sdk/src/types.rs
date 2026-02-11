//! Common types for extensions.
//!
//! V2 Extension System:
//! - Extensions use device-standard types (MetricDefinition, ExtensionCommand)
//! - Metrics and commands are separate
//! - ABI version 2

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current Extension ABI version - V2
pub const NEO_EXT_ABI_VERSION: u32 = 2;

/// Plugin context provides runtime information to plugins
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Plugin ID
    pub plugin_id: String,

    /// Plugin configuration
    pub config: Value,

    /// Base directory for plugin data
    pub data_dir: Option<String>,

    /// Temporary directory
    pub temp_dir: Option<String>,
}

impl PluginContext {
    /// Create a new plugin context
    pub fn new(plugin_id: impl Into<String>, config: Value) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            config,
            data_dir: None,
            temp_dir: None,
        }
    }

    /// Get a configuration value by key
    pub fn get_config(&self, key: &str) -> Option<&Value> {
        self.config.get(key)
    }

    /// Get a configuration value as string
    pub fn get_config_str(&self, key: &str) -> Option<&str> {
        self.config.get(key)?.as_str()
    }

    /// Get a configuration value as number
    pub fn get_config_number(&self, key: &str) -> Option<f64> {
        self.config.get(key)?.as_f64()
    }

    /// Get a configuration value as bool
    pub fn get_config_bool(&self, key: &str) -> Option<bool> {
        self.config.get(key)?.as_bool()
    }
}

/// A request from the host to the plugin
#[derive(Debug, Clone)]
pub struct PluginRequest {
    /// Request type/command
    pub command: String,

    /// Request arguments
    pub args: Value,

    /// Request ID for tracking
    pub request_id: Option<String>,
}

impl PluginRequest {
    /// Create a new request
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Value::Object(Default::default()),
            request_id: None,
        }
    }

    /// Set the request arguments
    pub fn with_args(mut self, args: Value) -> Self {
        self.args = args;
        self
    }

    /// Set the request ID
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Get an argument by key
    pub fn get_arg(&self, key: &str) -> Option<&Value> {
        self.args.get(key)
    }
}

/// A response from the plugin to the host
#[derive(Debug, Clone)]
pub struct PluginResponse {
    /// Response data
    pub data: Value,

    /// Whether the request was successful
    pub success: bool,

    /// Error message if not successful
    pub error: Option<String>,

    /// Additional metadata
    pub metadata: Value,
}

impl PluginResponse {
    /// Create a successful response
    pub fn success(data: Value) -> Self {
        Self {
            data,
            success: true,
            error: None,
            metadata: Value::Object(Default::default()),
        }
    }

    /// Create an error response
    pub fn error(error: impl Into<String>) -> Self {
        Self {
            data: Value::Null,
            success: false,
            error: Some(error.into()),
            metadata: Value::Object(Default::default()),
        }
    }

    /// Add metadata to the response
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        if let Value::Object(ref mut map) = self.metadata {
            map.insert(key.into(), value);
        }
        self
    }
}

impl From<Value> for PluginResponse {
    fn from(data: Value) -> Self {
        Self::success(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_config_access() {
        let config = serde_json::json!({
            "api_key": "secret",
            "timeout": 30,
            "enabled": true
        });

        let ctx = PluginContext::new("test-plugin", config);

        assert_eq!(ctx.get_config_str("api_key"), Some("secret"));
        assert_eq!(ctx.get_config_number("timeout"), Some(30.0));
        assert_eq!(ctx.get_config_bool("enabled"), Some(true));
    }

    #[test]
    fn test_response_creation() {
        let success = PluginResponse::success(serde_json::json!({"result": "ok"}));
        assert!(success.success);
        assert!(success.error.is_none());

        let error = PluginResponse::error("something went wrong");
        assert!(!error.success);
        assert_eq!(error.error, Some("something went wrong".to_string()));
    }
}

// ============================================================================
// Extension Core Types - V2 (Device-Standard Compatible)
// ============================================================================

/// Metric data type (matches device standard)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricDataType {
    /// Floating point number
    Float,
    /// Integer number
    Integer,
    /// Boolean value
    Boolean,
    /// String value
    String,
    /// Binary data
    Binary,
    /// Enum with specific options
    #[serde(rename = "enum")]
    Enum { options: Vec<String> },
}

/// Parameter definition for commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    /// Parameter name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Parameter description
    pub description: String,
    /// Parameter data type
    #[serde(rename = "type")]
    pub param_type: MetricDataType,
    /// Whether the parameter is required
    pub required: bool,
    /// Default value
    pub default_value: Option<Value>,
    /// Minimum value (for numeric types)
    pub min: Option<f64>,
    /// Maximum value (for numeric types)
    pub max: Option<f64>,
    /// Enum options (for Enum types)
    #[serde(default)]
    pub options: Vec<String>,
}

/// Metric definition (V2 - matches device standard)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    /// Metric name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Data type
    #[serde(rename = "type")]
    pub data_type: MetricDataType,
    /// Unit of measurement
    pub unit: String,
    /// Minimum value
    pub min: Option<f64>,
    /// Maximum value
    pub max: Option<f64>,
    /// Whether this metric is required
    pub required: bool,
}

/// Extension command (V2 - matches device standard)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionCommand {
    /// Command name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Payload template for generating the command payload
    pub payload_template: String,
    /// Command parameters
    pub parameters: Vec<ParameterDefinition>,
    /// Fixed values for this command
    #[serde(default)]
    pub fixed_values: serde_json::Map<String, Value>,
    /// Sample executions for AI hints
    #[serde(default)]
    pub samples: Vec<Value>,
    /// AI hints for using this command
    pub llm_hints: String,
    /// Parameter groups for UI organization
    #[serde(default)]
    pub parameter_groups: Vec<ParameterGroup>,
}

/// Parameter group for UI organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterGroup {
    /// Group name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Parameters in this group
    pub parameters: Vec<String>,
    /// Group order
    pub order: usize,
}

/// Extension metadata (V2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    /// Extension ID (e.g., "com.example.my_extension")
    pub id: String,
    /// Extension name
    pub name: String,
    /// Extension version
    pub version: String,
    /// Extension description
    pub description: Option<String>,
    /// Extension author
    pub author: Option<String>,
    /// Homepage URL
    pub homepage: Option<String>,
    /// License
    pub license: Option<String>,
    /// File path (for loaded extensions)
    #[serde(skip)]
    pub file_path: Option<std::path::PathBuf>,
}

impl ExtensionMetadata {
    /// Create a new extension metadata
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            description: None,
            author: None,
            homepage: None,
            license: None,
            file_path: None,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set the homepage
    pub fn with_homepage(mut self, homepage: impl Into<String>) -> Self {
        self.homepage = Some(homepage.into());
        self
    }

    /// Set the license
    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    /// Set the file path
    pub fn with_file_path(mut self, path: std::path::PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }
}

/// Extension trait that all extensions must implement (V2)
///
/// V2 changes:
/// - `metrics()` and `commands()` are separate methods
/// - `execute_command()` instead of `handle_command()`
/// - No lifecycle methods (initialize, start, stop, shutdown)
pub trait Extension: Send + Sync {
    /// Get the extension's metadata
    fn metadata(&self) -> &ExtensionMetadata;

    /// Get metrics provided by this extension
    fn metrics(&self) -> &[MetricDefinition] {
        &[]
    }

    /// Get commands provided by this extension
    fn commands(&self) -> &[ExtensionCommand] {
        &[]
    }

    /// Execute a command with given arguments
    fn execute_command(&self, command: &str, _args: &Value) -> Result<Value, ExtensionError> {
        Err(ExtensionError::UnsupportedCommand {
            command: command.to_string(),
        })
    }

    /// Health check for the extension
    fn health_check(&self) -> Result<bool, ExtensionError> {
        Ok(true)
    }
}

/// Extension error type (V2)
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    /// Command not found
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    /// Unsupported command
    #[error("Unsupported command: {command}")]
    UnsupportedCommand { command: String },

    /// Invalid arguments
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Operation timed out
    #[error("Operation timed out")]
    Timeout,

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic error with message
    #[error("{0}")]
    Other(String),
}

impl From<String> for ExtensionError {
    fn from(msg: String) -> Self {
        Self::Other(msg)
    }
}

impl From<&str> for ExtensionError {
    fn from(msg: &str) -> Self {
        Self::Other(msg.to_string())
    }
}

// ============================================================================
// Legacy Types (for backward compatibility)
// ============================================================================

/// Extension capability type (legacy)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionCapabilityType {
    /// Tool extensions provide AI function calling tools
    Tool,
    /// Provider extensions provide data sources/metrics
    Provider,
    /// Processor extensions provide data transformations
    Processor,
    /// Notifier extensions provide notification channels
    Notifier,
    /// Hybrid extensions have multiple capabilities
    Hybrid,
}

/// Tool descriptor for AI function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input parameters schema (JSON Schema)
    pub parameters: serde_json::Value,
    /// Whether the tool requires execution confirmation
    pub requires_confirmation: Option<bool>,
    /// Example usage
    pub examples: Option<Vec<serde_json::Value>>,
}

/// Metric descriptor for data sources (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDescriptorLegacy {
    /// Metric name
    pub name: String,
    /// Data type (string, number, boolean, etc.)
    pub data_type: String,
    /// Unit of measurement
    pub unit: Option<String>,
    /// Metric description
    pub description: Option<String>,
}

/// Channel descriptor for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDescriptor {
    /// Channel name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Channel description
    pub description: String,
    /// Configuration schema
    pub config_schema: serde_json::Value,
}

/// Command descriptor for processor extensions (legacy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDescriptor {
    /// Command name
    pub name: String,
    /// Command description
    pub description: String,
}

/// Extension capability descriptor (legacy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionCapabilityDescriptor {
    /// Extension ID
    pub id: String,
    /// Extension type (tool, provider, processor, notifier, hybrid)
    #[serde(rename = "type")]
    pub capability_type: ExtensionCapabilityType,
    /// Tools (for tool/hybrid extensions)
    pub tools: Option<Vec<ToolDescriptor>>,
    /// Metrics (for provider/hybrid extensions)
    pub metrics: Option<Vec<MetricDescriptorLegacy>>,
    /// Input schema (for processor/hybrid extensions)
    pub input_schema: Option<serde_json::Value>,
    /// Output schema (for processor/hybrid extensions)
    pub output_schema: Option<serde_json::Value>,
    /// Channels (for notifier/hybrid extensions)
    pub channels: Option<Vec<ChannelDescriptor>>,
    /// Commands (for processor extensions)
    pub commands: Option<Vec<CommandDescriptor>>,
    /// Configuration schema
    pub config_schema: Option<serde_json::Value>,
}

/// Type alias for backward compatibility
pub use MetricDescriptorLegacy as MetricDescriptor;
