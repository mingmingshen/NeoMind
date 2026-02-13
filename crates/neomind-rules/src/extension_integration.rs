//! Extension integration for rule engine.
//!
//! This module provides integration between the rule engine and the extension system,
//! allowing rules to:
//! 1. Query extension data sources as conditions
//! 2. Execute extension commands as actions

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::dsl::RuleAction;

// Import DataSourceId for typed extension command handling
pub use neomind_core::datasource::DataSourceId;

/// Extension command action for rules.
/// This action executes a command on an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionCommandAction {
    /// Extension ID (e.g., "neomind.weather.live")
    pub extension_id: String,
    /// Command to execute (e.g., "get_current_weather")
    pub command: String,
    /// Command arguments
    #[serde(default)]
    pub args: Value,
    /// Optional timeout in milliseconds
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

impl ExtensionCommandAction {
    /// Create a new extension command action.
    pub fn new(extension_id: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            extension_id: extension_id.into(),
            command: command.into(),
            args: Value::Object(Default::default()),
            timeout_ms: None,
        }
    }

    /// Set the command arguments.
    pub fn with_args(mut self, args: Value) -> Self {
        self.args = args;
        self
    }

    /// Set a timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Add a single argument.
    pub fn add_arg(mut self, key: impl Into<String>, value: Value) -> Self {
        if let Some(obj) = self.args.as_object_mut() {
            obj.insert(key.into(), value);
        } else {
            let mut map = serde_json::Map::new();
            map.insert(key.into(), value);
            self.args = Value::Object(map);
        }
        self
    }
}

/// Extension data source condition for rules.
/// This condition queries an extension's output field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionCondition {
    /// Extension ID
    pub extension_id: String,
    /// Command that produced the data
    pub command: String,
    /// Output field name
    pub field: String,
    /// Comparison operator
    pub operator: ConditionOperator,
    /// Threshold value
    pub threshold: f64,
}

impl ExtensionCondition {
    /// Create a new extension condition.
    pub fn new(
        extension_id: impl Into<String>,
        command: impl Into<String>,
        field: impl Into<String>,
        operator: ConditionOperator,
        threshold: f64,
    ) -> Self {
        Self {
            extension_id: extension_id.into(),
            command: command.into(),
            field: field.into(),
            operator,
            threshold,
        }
    }

    /// Get the data source ID for this condition.
    ///
    /// Returns a DataSourceId with nested field path "command.field"
    pub fn data_source_id(&self) -> DataSourceId {
        DataSourceId::extension_command(&self.extension_id, &self.command, &self.field)
    }

    /// Evaluate the condition with the given value.
    pub fn evaluate(&self, value: Option<f64>) -> bool {
        match value {
            Some(v) => self.operator.evaluate(v, self.threshold),
            None => false,
        }
    }
}

/// Comparison operators for conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionOperator {
    /// Greater than
    GreaterThan,
    /// Less than
    LessThan,
    /// Greater than or equal
    GreaterThanOrEqual,
    /// Less than or equal
    LessThanOrEqual,
    /// Equal
    Equal,
    /// Not equal
    NotEqual,
}

impl ConditionOperator {
    /// Evaluate the operator.
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            Self::GreaterThan => value > threshold,
            Self::LessThan => value < threshold,
            Self::GreaterThanOrEqual => value >= threshold,
            Self::LessThanOrEqual => value <= threshold,
            Self::Equal => (value - threshold).abs() < 0.001,
            Self::NotEqual => (value - threshold).abs() >= 0.001,
        }
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            ">" | "gt" => Some(Self::GreaterThan),
            "<" | "lt" => Some(Self::LessThan),
            ">=" | "gte" => Some(Self::GreaterThanOrEqual),
            "<=" | "lte" => Some(Self::LessThanOrEqual),
            "==" | "eq" => Some(Self::Equal),
            "!=" | "ne" => Some(Self::NotEqual),
            _ => None,
        }
    }
}

/// Extension value provider for rule engine.
/// This provides values from extension command outputs.
pub struct ExtensionValueProvider {
    /// Extension registry for executing commands
    extension_registry: Arc<dyn ExtensionRegistry>,
    /// Cached values from extension commands
    cached_values: Arc<RwLock<HashMap<String, f64>>>,
    /// Cache TTL in seconds
    cache_ttl: u64,
}

/// Extension registry trait - abstracted for dependency injection.
#[async_trait]
pub trait ExtensionRegistry: Send + Sync {
    /// Execute a command on an extension.
    async fn execute_command(
        &self,
        extension_id: &str,
        command: &str,
        args: &Value,
    ) -> Result<Value, String>;

    /// Check if an extension exists.
    async fn has_extension(&self, extension_id: &str) -> bool;
}

impl ExtensionValueProvider {
    /// Create a new extension value provider.
    pub fn new(registry: Arc<dyn ExtensionRegistry>) -> Self {
        Self {
            extension_registry: registry,
            cached_values: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: 60, // Default 60 seconds
        }
    }

    /// Set the cache TTL.
    pub fn with_cache_ttl(mut self, ttl: u64) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Get a value from an extension data source.
    pub async fn get_extension_value(
        &self,
        extension_id: &str,
        command: &str,
        field: &str,
    ) -> Result<Option<f64>, String> {
        // Use DataSourceId for consistent cache key generation
        let ds_id = DataSourceId::extension_command(extension_id, command, field);
        let cache_key = ds_id.storage_key();

        // Check cache first
        {
            let cache = self.cached_values.read().await;
            if let Some(&value) = cache.get(&cache_key) {
                return Ok(Some(value));
            }
        }

        // Execute the command
        let result = self
            .extension_registry
            .execute_command(extension_id, command, &Value::Object(Default::default()))
            .await?;

        // Extract the field value
        let value = self.extract_field(&result, field)?;

        // Cache the result
        if let Some(v) = value {
            let mut cache = self.cached_values.write().await;
            cache.insert(cache_key, v);
        }

        Ok(value)
    }

    /// Extract a field value from the command result.
    fn extract_field(&self, result: &Value, field: &str) -> Result<Option<f64>, String> {
        // Try direct field access
        if let Some(v) = result.get(field) {
            if let Some(n) = v.as_f64() {
                return Ok(Some(n));
            }
        }

        // Try nested access (e.g., "summary.total_objects")
        for part in field.split('.') {
            let current = result;
            let mut found = None;
            if let Some(obj) = current.as_object() {
                for (key, val) in obj {
                    if key == part {
                        found = Some(val);
                        break;
                    }
                }
            }
            if let Some(f) = found {
                if let Some(n) = f.as_f64() {
                    return Ok(Some(n));
                }
            }
        }

        Ok(None)
    }

    /// Clear the cache.
    pub async fn clear_cache(&self) {
        self.cached_values.write().await.clear();
    }
}

/// Rule action executor for extension commands.
pub struct ExtensionActionExecutor {
    extension_registry: Arc<dyn ExtensionRegistry>,
}

impl ExtensionActionExecutor {
    /// Create a new extension action executor.
    pub fn new(registry: Arc<dyn ExtensionRegistry>) -> Self {
        Self {
            extension_registry: registry,
        }
    }

    /// Execute an extension command action.
    pub async fn execute(
        &self,
        action: &ExtensionCommandAction,
    ) -> Result<ExecutionResult, String> {
        // Check if extension exists
        if !self
            .extension_registry
            .has_extension(&action.extension_id)
            .await
        {
            return Err(format!("Extension not found: {}", action.extension_id));
        }

        let start = std::time::Instant::now();

        // Apply timeout if specified
        let result = if let Some(timeout_ms) = action.timeout_ms {
            let timeout = tokio::time::Duration::from_millis(timeout_ms);
            tokio::time::timeout(
                timeout,
                self.extension_registry.execute_command(
                    &action.extension_id,
                    &action.command,
                    &action.args,
                ),
            )
            .await
            .map_err(|_| format!("Command timed out after {}ms", timeout_ms))?
        } else {
            self.extension_registry
                .execute_command(&action.extension_id, &action.command, &action.args)
                .await
        }?;

        let duration = start.elapsed();

        Ok(ExecutionResult {
            success: true,
            extension_id: action.extension_id.clone(),
            command: action.command.clone(),
            result,
            duration_ms: duration.as_millis() as u64,
            error: None,
        })
    }
}

/// Result of executing an extension command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Extension ID
    pub extension_id: String,
    /// Command that was executed
    pub command: String,
    /// Result value
    pub result: Value,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Error message if execution failed
    pub error: Option<String>,
}

/// Convert RuleAction to extension command if applicable.
///
/// This function detects if a RuleAction::Execute is targeting an extension
/// by parsing the device_id as a DataSourceId.
///
/// # Extension ID Formats Supported
/// - `extension:id:metric` - Extension metric data source
/// - `extension:id:command.field` - Extension command output (nested field)
/// - `extension:id` - Simple extension reference
///
/// Uses DataSourceId for type-safe parsing.
pub fn try_parse_extension_action(action: &RuleAction) -> Option<ExtensionCommandAction> {
    match action {
        RuleAction::Execute {
            device_id,
            command,
            params,
        } => {
            // Try parsing as standard DataSourceId (three-part: type:id:field)
            if let Some(ds_id) = DataSourceId::parse(device_id) {
                if ds_id.source_type == neomind_core::datasource::DataSourceType::Extension {
                    // Extract extension_id and optionally command from field_path
                    if let Some((_ext_id, cmd, _field)) = ds_id.as_extension_command_parts() {
                        // Has command.field format - use the command from field_path if provided command is empty
                        return Some(ExtensionCommandAction {
                            extension_id: ds_id.source_id.clone(),
                            command: if command.is_empty() {
                                cmd.to_string()
                            } else {
                                command.clone()
                            },
                            args: serde_json::to_value(params).unwrap_or_default(),
                            timeout_ms: None,
                        });
                    } else {
                        // Simple metric format - use the provided command
                        return Some(ExtensionCommandAction {
                            extension_id: ds_id.source_id.clone(),
                            command: command.clone(),
                            args: serde_json::to_value(params).unwrap_or_default(),
                            timeout_ms: None,
                        });
                    }
                }
            }

            // Try parsing as extension command format (four-part: extension:id:command:field)
            if let Some(ds_id) = DataSourceId::parse_extension_command(device_id) {
                if let Some((ext_id, cmd, _field)) = ds_id.as_extension_command_parts() {
                    return Some(ExtensionCommandAction {
                        extension_id: ext_id.to_string(),
                        command: if command.is_empty() {
                            cmd.to_string()
                        } else {
                            command.clone()
                        },
                        args: serde_json::to_value(params).unwrap_or_default(),
                        timeout_ms: None,
                    });
                }
            }

            // Not a valid extension format
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_operator_evaluate() {
        assert!(ConditionOperator::GreaterThan.evaluate(10.0, 5.0));
        assert!(!ConditionOperator::GreaterThan.evaluate(5.0, 10.0));

        assert!(ConditionOperator::LessThan.evaluate(5.0, 10.0));
        assert!(!ConditionOperator::LessThan.evaluate(10.0, 5.0));

        assert!(ConditionOperator::GreaterThanOrEqual.evaluate(10.0, 10.0));
        assert!(ConditionOperator::LessThanOrEqual.evaluate(5.0, 5.0));

        assert!(ConditionOperator::Equal.evaluate(5.0, 5.0));
        assert!(!ConditionOperator::Equal.evaluate(5.001, 5.0));
    }

    #[test]
    fn test_extension_condition_data_source_id() {
        let condition = ExtensionCondition::new(
            "neomind.weather.live",
            "get_current_weather",
            "temperature_c",
            ConditionOperator::GreaterThan,
            30.0,
        );

        // Test typed DataSourceId return
        let ds_id = condition.data_source_id();
        assert_eq!(
            ds_id.source_type,
            neomind_core::datasource::DataSourceType::Extension
        );
        assert_eq!(ds_id.source_id, "neomind.weather.live");
        assert_eq!(ds_id.field_path, "get_current_weather.temperature_c");
        assert_eq!(
            ds_id.storage_key(),
            "extension:neomind.weather.live:get_current_weather.temperature_c"
        );
    }

    #[test]
    fn test_extension_command_action_builder() {
        let action = ExtensionCommandAction::new("ext_id", "cmd_id")
            .add_arg("param1", Value::from(42))
            .add_arg("param2", Value::from("test"))
            .with_timeout(5000);

        assert_eq!(action.extension_id, "ext_id");
        assert_eq!(action.command, "cmd_id");
        assert_eq!(action.timeout_ms, Some(5000));
        assert_eq!(action.args["param1"], 42);
        assert_eq!(action.args["param2"], "test");
    }

    #[test]
    fn test_condition_operator_from_str() {
        assert_eq!(
            ConditionOperator::from_str(">"),
            Some(ConditionOperator::GreaterThan)
        );
        assert_eq!(
            ConditionOperator::from_str("lt"),
            Some(ConditionOperator::LessThan)
        );
        assert_eq!(ConditionOperator::from_str("invalid"), None);
    }
}
