//! IPC Boundary Types for NeoMind Extension SDK
//!
//! This module defines the stable types used for IPC communication between
//! the main NeoMind process and extension processes. These types must remain
//! backward compatible to ensure extensions don't need recompilation when
//! the main project updates.
//!
//! # Design Principles
//!
//! 1. **Stability**: Types marked with `#[serde]` must maintain JSON format compatibility
//! 2. **Single Source**: All IPC boundary types defined here, eliminating duplication
//! 3. **Minimal Dependencies**: Only serde and chrono for serialization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// ABI Version
// ============================================================================

/// ABI version for dynamic loading.
/// This must be incremented when breaking changes are made to IPC types.
pub const ABI_VERSION: u32 = 3;

// ============================================================================
// Metric Types
// ============================================================================

/// Metric data type enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MetricDataType {
    Float,
    Integer,
    Boolean,
    #[default]
    String,
    Binary,
    Enum { options: Vec<String> },
}

/// Metric value for parameters and measurements.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MetricValue {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
    Binary(Vec<u8>),
    #[default]
    Null,
}

// Implement From traits for ergonomic construction
impl From<f64> for MetricValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<i64> for MetricValue {
    fn from(v: i64) -> Self {
        Self::Integer(v)
    }
}

impl From<bool> for MetricValue {
    fn from(v: bool) -> Self {
        Self::Boolean(v)
    }
}

impl From<String> for MetricValue {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for MetricValue {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

impl From<Vec<u8>> for MetricValue {
    fn from(v: Vec<u8>) -> Self {
        Self::Binary(v)
    }
}

// ============================================================================
// Compatibility Aliases (for backward compatibility)
// ============================================================================

/// Alias for backward compatibility with existing code.
pub type ParamMetricValue = MetricValue;

// ============================================================================
// Metric Definition
// ============================================================================

/// Metric definition/descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricDescriptor {
    /// Unique metric name
    pub name: String,
    /// Human-readable display name
    #[serde(default)]
    pub display_name: String,
    /// Data type
    #[serde(default)]
    pub data_type: MetricDataType,
    /// Unit of measurement
    #[serde(default)]
    pub unit: String,
    /// Minimum value (for numeric types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum value (for numeric types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Whether this metric is required
    #[serde(default)]
    pub required: bool,
}

impl MetricDescriptor {
    /// Create a new metric descriptor.
    pub fn new(
        name: impl Into<String>,
        display_name: impl Into<String>,
        data_type: MetricDataType,
    ) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            data_type,
            unit: String::new(),
            min: None,
            max: None,
            required: false,
        }
    }

    /// Add unit.
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Add min/max range.
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Set as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

// ============================================================================
// Parameter Definition
// ============================================================================

/// Parameter definition for commands.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParameterDefinition {
    /// Parameter name
    pub name: String,
    /// Human-readable display name
    #[serde(default)]
    pub display_name: String,
    /// Description for documentation
    #[serde(default)]
    pub description: String,
    /// Parameter data type
    #[serde(default)]
    pub param_type: MetricDataType,
    /// Whether this parameter is required
    #[serde(default)]
    pub required: bool,
    /// Default value if not provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<MetricValue>,
    /// Minimum value (for numeric types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum value (for numeric types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Options for enum types
    #[serde(default)]
    pub options: Vec<String>,
}

impl ParameterDefinition {
    /// Create a new required parameter.
    pub fn new(name: impl Into<String>, param_type: MetricDataType) -> Self {
        Self {
            name: name.into(),
            display_name: String::new(),
            description: String::new(),
            param_type,
            required: true,
            default_value: None,
            min: None,
            max: None,
            options: Vec::new(),
        }
    }

    /// Add display name.
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    /// Add description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Make optional with default value.
    pub fn with_default(mut self, default: MetricValue) -> Self {
        self.default_value = Some(default);
        self.required = false;
        self
    }
}

// ============================================================================
// Parameter Group
// ============================================================================

/// Parameter group for organizing command parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParameterGroup {
    /// Group name
    pub name: String,
    /// Human-readable display name
    #[serde(default)]
    pub display_name: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Parameter names in this group
    #[serde(default)]
    pub parameters: Vec<String>,
}

// ============================================================================
// Command Definition
// ============================================================================

/// Command definition/descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandDescriptor {
    /// Command name (used as identifier)
    pub name: String,
    /// Human-readable display name
    #[serde(default)]
    pub display_name: String,
    /// Description for documentation and LLM hints
    #[serde(default)]
    pub description: String,
    /// Payload template (optional)
    #[serde(default)]
    pub payload_template: String,
    /// Command parameters
    #[serde(default)]
    pub parameters: Vec<ParameterDefinition>,
    /// Fixed values to inject
    #[serde(default)]
    pub fixed_values: HashMap<String, serde_json::Value>,
    /// Sample payloads for documentation
    #[serde(default)]
    pub samples: Vec<serde_json::Value>,
    /// Parameter groups
    #[serde(default)]
    pub parameter_groups: Vec<ParameterGroup>,
}

impl CommandDescriptor {
    /// Create a new command descriptor.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Add display name.
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    /// Add description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a parameter.
    pub fn param(mut self, param: ParameterDefinition) -> Self {
        self.parameters.push(param);
        self
    }

    /// Add a sample payload.
    pub fn sample(mut self, sample: serde_json::Value) -> Self {
        self.samples.push(sample);
        self
    }
}

// ============================================================================
// Compatibility Aliases for Command Types
// ============================================================================

/// Alias for backward compatibility.
pub type ExtensionCommand = CommandDescriptor;

/// Alias for backward compatibility.
pub type CommandDefinition = CommandDescriptor;

// ============================================================================
// Extension Metadata
// ============================================================================

/// Extension metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    /// Unique extension identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Version (using String for serde compatibility)
    pub version: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Optional homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// Optional license
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// File path (not serialized)
    #[serde(skip)]
    pub file_path: Option<std::path::PathBuf>,
    /// Configuration parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_parameters: Option<Vec<ParameterDefinition>>,
}

impl ExtensionMetadata {
    /// Create new metadata with version string.
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
            config_parameters: None,
        }
    }

    /// Create new metadata with semver::Version.
    pub fn new_with_semver(id: impl Into<String>, name: impl Into<String>, version: semver::Version) -> Self {
        Self::new(id, name, version.to_string())
    }

    /// Add description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add author.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Add homepage.
    pub fn with_homepage(mut self, homepage: impl Into<String>) -> Self {
        self.homepage = Some(homepage.into());
        self
    }

    /// Add license.
    pub fn with_license(mut self, license: impl Into<String>) -> Self {
        self.license = Some(license.into());
        self
    }

    /// Add config parameters.
    pub fn with_config_parameters(mut self, params: Vec<ParameterDefinition>) -> Self {
        self.config_parameters = Some(params);
        self
    }

    /// Parse version as semver.
    pub fn parse_version(&self) -> std::result::Result<semver::Version, semver::Error> {
        semver::Version::parse(&self.version)
    }
}

// ============================================================================
// Extension Descriptor
// ============================================================================

/// Complete extension descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionDescriptor {
    /// Extension metadata
    pub metadata: ExtensionMetadata,
    /// Commands provided by this extension
    #[serde(default)]
    pub commands: Vec<CommandDescriptor>,
    /// Metrics provided by this extension
    #[serde(default)]
    pub metrics: Vec<MetricDescriptor>,
}

impl ExtensionDescriptor {
    /// Create a new descriptor with metadata.
    pub fn new(metadata: ExtensionMetadata) -> Self {
        Self {
            metadata,
            commands: Vec::new(),
            metrics: Vec::new(),
        }
    }

    /// Create a descriptor with all capabilities.
    pub fn with_capabilities(
        metadata: ExtensionMetadata,
        commands: Vec<CommandDescriptor>,
        metrics: Vec<MetricDescriptor>,
    ) -> Self {
        Self {
            metadata,
            commands,
            metrics,
        }
    }

    /// Get extension ID.
    pub fn id(&self) -> &str {
        &self.metadata.id
    }

    /// Get extension name.
    pub fn name(&self) -> &str {
        &self.metadata.name
    }
}

// ============================================================================
// Extension Metric Value
// ============================================================================

/// Extension metric value with timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetricValue {
    /// Metric name
    pub name: String,
    /// Metric value
    pub value: MetricValue,
    /// Timestamp in milliseconds since Unix epoch
    pub timestamp: i64,
}

impl ExtensionMetricValue {
    /// Create a new metric value with current timestamp.
    pub fn new(name: impl Into<String>, value: MetricValue) -> Self {
        Self {
            name: name.into(),
            value,
            timestamp: current_timestamp_ms(),
        }
    }

    /// Create with explicit timestamp.
    pub fn with_timestamp(name: impl Into<String>, value: MetricValue, timestamp: i64) -> Self {
        Self {
            name: name.into(),
            value,
            timestamp,
        }
    }
}

// ============================================================================
// Extension Error
// ============================================================================

/// Extension error types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtensionError {
    /// Command not found
    CommandNotFound(String),
    /// Metric not found
    MetricNotFound(String),
    /// Invalid arguments
    InvalidArguments(String),
    /// Execution failed
    ExecutionFailed(String),
    /// Timeout
    Timeout(String),
    /// Not found
    NotFound(String),
    /// Invalid format
    InvalidFormat(String),
    /// Load failed
    LoadFailed(String),
    /// Security error
    SecurityError(String),
    /// Symbol not found (FFI)
    SymbolNotFound(String),
    /// Incompatible version
    IncompatibleVersion { expected: u32, got: u32 },
    /// Null pointer (FFI)
    NullPointer,
    /// Already registered
    AlreadyRegistered(String),
    /// Not supported
    NotSupported(String),
    /// Invalid stream data
    InvalidStreamData(String),
    /// Session not found
    SessionNotFound(String),
    /// Session already exists
    SessionAlreadyExists(String),
    /// Inference failed
    InferenceFailed(String),
    /// IO error
    Io(String),
    /// JSON error
    Json(String),
    /// Configuration error
    ConfigurationError(String),
    /// Internal error
    InternalError(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for ExtensionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandNotFound(cmd) => write!(f, "Command not found: {}", cmd),
            Self::MetricNotFound(metric) => write!(f, "Metric not found: {}", metric),
            Self::InvalidArguments(msg) => write!(f, "Invalid arguments: {}", msg),
            Self::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            Self::Timeout(msg) => write!(f, "Timeout: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            Self::LoadFailed(msg) => write!(f, "Load failed: {}", msg),
            Self::SecurityError(msg) => write!(f, "Security error: {}", msg),
            Self::SymbolNotFound(msg) => write!(f, "Symbol not found: {}", msg),
            Self::IncompatibleVersion { expected, got } => {
                write!(f, "Incompatible version: expected {}, got {}", expected, got)
            }
            Self::NullPointer => write!(f, "Null pointer"),
            Self::AlreadyRegistered(msg) => write!(f, "Already registered: {}", msg),
            Self::NotSupported(msg) => write!(f, "Not supported: {}", msg),
            Self::InvalidStreamData(msg) => write!(f, "Invalid stream data: {}", msg),
            Self::SessionNotFound(msg) => write!(f, "Session not found: {}", msg),
            Self::SessionAlreadyExists(msg) => write!(f, "Session already exists: {}", msg),
            Self::InferenceFailed(msg) => write!(f, "Inference failed: {}", msg),
            Self::Io(msg) => write!(f, "IO error: {}", msg),
            Self::Json(msg) => write!(f, "JSON error: {}", msg),
            Self::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            Self::InternalError(msg) => write!(f, "Internal error: {}", msg),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ExtensionError {}

impl From<serde_json::Error> for ExtensionError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e.to_string())
    }
}

impl From<std::io::Error> for ExtensionError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// Result type for extension operations.
pub type Result<T> = std::result::Result<T, ExtensionError>;

// ============================================================================
// Extension Runtime State
// ============================================================================

/// Dynamic runtime state for a loaded extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRuntimeState {
    /// Is the extension running
    pub is_running: bool,
    /// Is the extension isolated
    pub is_isolated: bool,
    /// When the extension was loaded
    pub loaded_at: Option<i64>,
    /// Number of restarts
    pub restart_count: u64,
    /// Last restart time
    pub last_restart_at: Option<i64>,
    /// Number of starts
    pub start_count: u64,
    /// Number of stops
    pub stop_count: u64,
    /// Number of errors
    pub error_count: u64,
    /// Last error message
    pub last_error: Option<String>,
}

impl Default for ExtensionRuntimeState {
    fn default() -> Self {
        Self {
            is_running: false,
            is_isolated: false,
            loaded_at: None,
            restart_count: 0,
            last_restart_at: None,
            start_count: 0,
            stop_count: 0,
            error_count: 0,
            last_error: None,
        }
    }
}

impl ExtensionRuntimeState {
    /// Create new state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create isolated state.
    pub fn isolated() -> Self {
        Self {
            is_isolated: true,
            ..Self::default()
        }
    }

    /// Mark as running.
    pub fn mark_running(&mut self) {
        self.is_running = true;
        self.start_count += 1;
        if self.loaded_at.is_none() {
            self.loaded_at = Some(current_timestamp_secs());
        }
    }

    /// Mark as stopped.
    pub fn mark_stopped(&mut self) {
        self.is_running = false;
        self.stop_count += 1;
    }

    /// Record an error.
    pub fn record_error(&mut self, error: String) {
        self.error_count += 1;
        self.last_error = Some(error);
    }
}

// ============================================================================
// Extension Statistics
// ============================================================================

/// Extension runtime statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionStats {
    /// Number of metrics produced
    pub metrics_produced: u64,
    /// Number of commands executed
    pub commands_executed: u64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Last execution timestamp (milliseconds)
    pub last_execution_time_ms: Option<i64>,
    /// Number of starts
    pub start_count: u64,
    /// Number of stops
    pub stop_count: u64,
    /// Number of errors
    pub error_count: u64,
    /// Last error message
    pub last_error: Option<String>,
}

// ============================================================================
// Validation Rule
// ============================================================================

/// Validation rule for parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationRule {
    /// Rule type
    #[serde(default)]
    pub rule_type: String,
    /// Rule parameters
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
}

// ============================================================================
// Push Output Message
// ============================================================================

/// Message sent from extension to host for Push mode streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushOutputMessage {
    /// Session ID
    pub session_id: String,
    /// Sequence number
    pub sequence: u64,
    /// Data
    pub data: Vec<u8>,
    /// MIME type
    pub data_type: String,
    /// Timestamp
    pub timestamp: i64,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl PushOutputMessage {
    /// Create a new push output message.
    pub fn new(
        session_id: impl Into<String>,
        sequence: u64,
        data: Vec<u8>,
        data_type: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            sequence,
            data,
            data_type: data_type.into(),
            timestamp: current_timestamp_ms(),
            metadata: None,
        }
    }

    /// Create JSON output.
    pub fn json(
        session_id: impl Into<String>,
        sequence: u64,
        value: serde_json::Value,
    ) -> serde_json::Result<Self> {
        Ok(Self {
            session_id: session_id.into(),
            sequence,
            data: serde_json::to_vec(&value)?,
            data_type: "application/json".to_string(),
            timestamp: current_timestamp_ms(),
            metadata: None,
        })
    }

    /// Create JPEG image output.
    pub fn image_jpeg(session_id: impl Into<String>, sequence: u64, data: Vec<u8>) -> Self {
        Self {
            session_id: session_id.into(),
            sequence,
            data,
            data_type: "image/jpeg".to_string(),
            timestamp: current_timestamp_ms(),
            metadata: None,
        }
    }

    /// Add metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

// ============================================================================
// C Extension Metadata (FFI)
// ============================================================================

/// C-compatible extension metadata for FFI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CExtensionMetadata {
    /// ABI version
    pub abi_version: u32,
    /// Extension ID
    pub id: *const std::os::raw::c_char,
    /// Display name
    pub name: *const std::os::raw::c_char,
    /// Version string
    pub version: *const std::os::raw::c_char,
    /// Description
    pub description: *const std::os::raw::c_char,
    /// Author
    pub author: *const std::os::raw::c_char,
    /// Number of metrics
    pub metric_count: usize,
    /// Number of commands
    pub command_count: usize,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get current timestamp in milliseconds.
#[cfg(not(target_arch = "wasm32"))]
pub fn current_timestamp_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Get current timestamp in milliseconds (WASM stub).
#[cfg(target_arch = "wasm32")]
pub fn current_timestamp_ms() -> i64 {
    0
}

/// Get current timestamp in seconds.
#[cfg(not(target_arch = "wasm32"))]
pub fn current_timestamp_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Get current timestamp in seconds (WASM stub).
#[cfg(target_arch = "wasm32")]
pub fn current_timestamp_secs() -> i64 {
    0
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_data_type_serialization() {
        let types = vec![
            (MetricDataType::Float, r#""float""#),
            (MetricDataType::Integer, r#""integer""#),
            (MetricDataType::Boolean, r#""boolean""#),
            (MetricDataType::String, r#""string""#),
            (MetricDataType::Binary, r#""binary""#),
        ];

        for (dtype, expected) in types {
            let json = serde_json::to_string(&dtype).unwrap();
            assert_eq!(json, expected);

            let deserialized: MetricDataType = serde_json::from_str(expected).unwrap();
            assert_eq!(dtype, deserialized);
        }
    }

    #[test]
    fn test_metric_value_from() {
        let f: MetricValue = 42.0.into();
        assert!(matches!(f, MetricValue::Float(42.0)));

        let i: MetricValue = 42i64.into();
        assert!(matches!(i, MetricValue::Integer(42)));

        let b: MetricValue = true.into();
        assert!(matches!(b, MetricValue::Boolean(true)));

        let s: MetricValue = "test".into();
        assert!(matches!(s, MetricValue::String(_)));
    }

    #[test]
    fn test_extension_metadata() {
        let meta = ExtensionMetadata::new("test-ext", "Test Extension", "1.0.0")
            .with_description("A test extension")
            .with_author("Test Author");

        assert_eq!(meta.id, "test-ext");
        assert_eq!(meta.name, "Test Extension");
        assert_eq!(meta.description, Some("A test extension".to_string()));
        assert_eq!(meta.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_extension_descriptor_serialization() {
        let descriptor = ExtensionDescriptor::with_capabilities(
            ExtensionMetadata::new("test", "Test", "1.0.0"),
            vec![CommandDescriptor::new("cmd1")],
            vec![MetricDescriptor::new("m1", "M1", MetricDataType::Float)],
        );

        let json = serde_json::to_string(&descriptor).unwrap();
        let deserialized: ExtensionDescriptor = serde_json::from_str(&json).unwrap();

        assert_eq!(descriptor.metadata.id, deserialized.metadata.id);
        assert_eq!(descriptor.commands.len(), deserialized.commands.len());
        assert_eq!(descriptor.metrics.len(), deserialized.metrics.len());
    }

    #[test]
    fn test_extension_error_display() {
        let err = ExtensionError::CommandNotFound("test".to_string());
        assert!(err.to_string().contains("Command not found"));
    }

    #[test]
    fn test_abi_version() {
        assert_eq!(ABI_VERSION, 3);
    }
}
