//! Event subscription types for extensions
//!
//! This module defines types that allow extensions to subscribe to
//! and filter system events from the event bus.

use serde::{Deserialize, Serialize};
use std::default::Default;

/// Event subscription configuration for extensions
///
/// Extensions can declare which events they want to receive and
/// apply filters to reduce event volume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    /// List of event types to subscribe to
    ///
    /// If empty, subscribes to all event types.
    /// Examples: ["DeviceMetric", "ExtensionOutput", "AgentProgress"]
    pub event_types: Vec<String>,

    /// Optional filters to apply to events
    ///
    /// Filters allow extensions to receive only relevant events,
    /// reducing processing overhead and noise.
    pub filters: Option<EventFilter>,

    /// Maximum buffer size for events
    ///
    /// If the extension cannot process events fast enough,
    /// older events will be dropped after this limit is reached.
    ///
    /// Default: 1000
    pub max_buffer_size: usize,

    /// Whether event subscription is enabled
    ///
    /// Extensions can disable event subscription at runtime.
    pub enabled: bool,
}

impl EventSubscription {
    /// Create a new empty event subscription
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a subscription for specific event types
    pub fn with_types(event_types: Vec<String>) -> Self {
        Self {
            event_types,
            filters: None,
            max_buffer_size: 1000,
            enabled: true,
        }
    }

    /// Add filters to this subscription
    pub fn with_filters(mut self, filters: EventFilter) -> Self {
        self.filters = Some(filters);
        self
    }

    /// Set maximum buffer size
    pub fn with_max_buffer_size(mut self, size: usize) -> Self {
        self.max_buffer_size = size;
        self
    }

    /// Disable subscription
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Check if this subscription matches an event type
    pub fn is_subscribed(&self, event_type: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // If event_types is empty, subscribe to all events
        if self.event_types.is_empty() {
            return true;
        }

        self.event_types.iter().any(|et| et == event_type)
    }

    /// Check if this subscription matches an event (including filters)
    pub fn matches_event(&self, event_type: &str, event_value: &serde_json::Value) -> bool {
        if !self.is_subscribed(event_type) {
            return false;
        }

        // Check filters if present
        if let Some(ref filters) = self.filters {
            return filters.matches(event_type, event_value);
        }

        true
    }
}

/// Event filter for reducing event volume
///
/// Filters allow extensions to receive only events that match
/// specific criteria, reducing processing overhead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Filter by event source component
    ///
    /// Examples: "devices", "agents", "extensions", "rules"
    pub source: Option<String>,

    /// Filter by device ID
    ///
    /// Only applies to device-related events.
    pub device_id: Option<String>,

    /// Filter by extension ID
    ///
    /// Only applies to extension-related events.
    pub extension_id: Option<String>,

    /// Filter by agent ID
    ///
    /// Only applies to agent-related events.
    pub agent_id: Option<String>,

    /// Filter by rule ID
    ///
    /// Only applies to rule-related events.
    pub rule_id: Option<String>,

    /// Filter by workflow ID
    ///
    /// Only applies to workflow-related events.
    pub workflow_id: Option<String>,

    /// Custom expression filter
    ///
    /// A JSONPath or simple expression for advanced filtering.
    /// Example: "$.temperature > 30" or "$.state == 'online'"
    pub expression: Option<String>,
}


impl Default for EventFilter {
    fn default() -> Self {
        Self {
            source: None,
            device_id: None,
            extension_id: None,
            agent_id: None,
            rule_id: None,
            workflow_id: None,
            expression: None,
        }
    }
}
impl EventFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by source
    pub fn by_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Filter by device ID
    pub fn by_device_id(mut self, device_id: impl Into<String>) -> Self {
        self.device_id = Some(device_id.into());
        self
    }

    /// Filter by extension ID
    pub fn by_extension_id(mut self, extension_id: impl Into<String>) -> Self {
        self.extension_id = Some(extension_id.into());
        self
    }

    /// Filter by agent ID
    pub fn by_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Filter by rule ID
    pub fn by_rule_id(mut self, rule_id: impl Into<String>) -> Self {
        self.rule_id = Some(rule_id.into());
        self
    }

    /// Filter by workflow ID
    pub fn by_workflow_id(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    /// Filter by expression
    pub fn by_expression(mut self, expression: impl Into<String>) -> Self {
        self.expression = Some(expression.into());
        self
    }

    /// Check if an event matches this filter
    pub fn matches(&self, event_type: &str, event_value: &serde_json::Value) -> bool {
        // Check source filter
        if let Some(ref source) = self.source {
            // Try to extract source from event
            if let Some(event_source) = event_value.get("source").and_then(|v| v.as_str()) {
                if event_source != source {
                    return false;
                }
            } else {
                // If source field not present, try to infer from event type
                let inferred_source = infer_source_from_event_type(event_type);
                if inferred_source != source.as_str() {
                    return false;
                }
            }
        }

        // Check device_id filter
        if let Some(ref device_id) = self.device_id {
            if let Some(event_device_id) = event_value.get("device_id").and_then(|v| v.as_str()) {
                if event_device_id != device_id {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check extension_id filter
        if let Some(ref extension_id) = self.extension_id {
            if let Some(event_ext_id) = event_value.get("extension_id").and_then(|v| v.as_str()) {
                if event_ext_id != extension_id {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check agent_id filter
        if let Some(ref agent_id) = self.agent_id {
            if let Some(event_agent_id) = event_value.get("agent_id").and_then(|v| v.as_str()) {
                if event_agent_id != agent_id {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check rule_id filter
        if let Some(ref rule_id) = self.rule_id {
            if let Some(event_rule_id) = event_value.get("rule_id").and_then(|v| v.as_str()) {
                if event_rule_id != rule_id {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check workflow_id filter
        if let Some(ref workflow_id) = self.workflow_id {
            if let Some(event_wf_id) = event_value.get("workflow_id").and_then(|v| v.as_str()) {
                if event_wf_id != workflow_id {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check expression filter (basic implementation)
        if let Some(ref expression) = self.expression {
            // Basic expression evaluation
            // Supports simple comparisons like: "$.temperature > 30" or "$.state == 'online'"
            if !Self::evaluate_expression(expression, event_value) {
                return false;
            }
        }

        true
    }

    /// Evaluate a simple expression against an event value.
    ///
    /// Supported formats:
    /// - `$.field > value` (numeric comparison)
    /// - `$.field < value` (numeric comparison)
    /// - `$.field >= value` (numeric comparison)
    /// - `$.field <= value` (numeric comparison)
    /// - `$.field == "value"` (string equality)
    /// - `$.field != "value"` (string inequality)
    fn evaluate_expression(expression: &str, event_value: &serde_json::Value) -> bool {
        let expr = expression.trim();

        // Remove $. prefix if present
        let expr = if let Some(stripped) = expr.strip_prefix("$.") {
            stripped
        } else {
            expr
        };

        // Try to parse as a comparison expression
        // Pattern: field operator value

        // Find operator
        let operators = [">=", "<=", "==", "!=", ">", "<"];
        let mut found_op: Option<&str> = None;
        let mut op_pos: usize = 0;

        for op in &operators {
            if let Some(pos) = expr.find(op) {
                if found_op.is_none() || pos < op_pos {
                    found_op = Some(op);
                    op_pos = pos;
                }
            }
        }

        let Some(op) = found_op else {
            tracing::warn!(expression = %expression, "Invalid expression format: no operator found");
            return true; // Pass through if expression is malformed
        };

        let field = expr[..op_pos].trim();
        let value_str = expr[op_pos + op.len()..].trim();

        // Get the field value from the event
        let field_value = if field.contains('.') {
            // Nested field access
            Self::get_nested_value(event_value, field)
        } else {
            event_value.get(field).cloned()
        };

        let Some(field_value) = field_value else {
            tracing::debug!(field = %field, "Field not found in event for expression filter");
            return false;
        };

        // Perform comparison based on operator
        match op {
            "==" | "!=" => {
                // String or value equality
                let expected = value_str.trim_matches('"').trim_matches('\'');
                let matches = if field_value.is_string() {
                    field_value.as_str() == Some(expected)
                } else {
                    // Try to parse the expected value as JSON and compare
                    match serde_json::from_str::<serde_json::Value>(value_str) {
                        Ok(expected_json) => field_value == expected_json,
                        Err(_) => field_value.as_str() == Some(expected),
                    }
                };
                if op == "!=" { !matches } else { matches }
            }
            ">" | "<" | ">=" | "<=" => {
                // Numeric comparison
                let field_num = if field_value.is_number() {
                    field_value.as_f64()
                } else if field_value.is_string() {
                    field_value.as_str().and_then(|s| s.parse::<f64>().ok())
                } else {
                    None
                };

                let expected_num = value_str.parse::<f64>().ok();

                match (field_num, expected_num) {
                    (Some(f), Some(e)) => match op {
                        ">" => f > e,
                        "<" => f < e,
                        ">=" => f >= e,
                        "<=" => f <= e,
                        _ => true,
                    },
                    _ => {
                        tracing::debug!(
                            field = %field,
                            field_value = ?field_value,
                            expected = %value_str,
                            "Could not compare values as numbers"
                        );
                        true // Pass through if comparison fails
                    }
                }
            }
            _ => true,
        }
    }

    /// Get a nested value from a JSON object using dot notation.
    fn get_nested_value(value: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in &parts {
            current = current.get(part)?;
        }

        Some(current.clone())
    }
}

/// Infer source component from event type
fn infer_source_from_event_type(event_type: &str) -> &'static str {
    match event_type {
        "DeviceOnline" | "DeviceOffline" | "DeviceMetric" | "DeviceCommandResult" => "devices",
        "RuleEvaluated" | "RuleTriggered" | "RuleExecuted" => "rules",
        "WorkflowTriggered" | "WorkflowStepCompleted" | "WorkflowCompleted" => "workflows",
        "AlertCreated" | "AlertAcknowledged" => "alerts",
        "MessageCreated" | "MessageAcknowledged" | "MessageResolved" => "messages",
        "AgentExecutionStarted" | "AgentThinking" | "AgentDecision" | "AgentProgress"
        | "AgentExecutionCompleted" | "AgentMemoryUpdated" => "agents",
        "PeriodicReviewTriggered" | "LlmDecisionProposed" | "LlmDecisionExecuted"
        | "UserMessage" | "LlmResponse" => "llm",
        "ToolExecutionStart" | "ToolExecutionSuccess" | "ToolExecutionFailure" => "tools",
        "ExtensionOutput" | "ExtensionLifecycle" | "ExtensionCommandStarted"
        | "ExtensionCommandCompleted" | "ExtensionCommandFailed" => "extensions",
        _ => "unknown",
    }
}

impl Default for EventSubscription {
    fn default() -> Self {
        Self {
            event_types: Vec::new(),
            filters: None,
            max_buffer_size: 1000,
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_subscription_new() {
        let sub = EventSubscription::new();
        assert!(sub.event_types.is_empty());
        assert!(sub.enabled);
        assert_eq!(sub.max_buffer_size, 1000);
    }

    #[test]
    fn test_event_subscription_with_types() {
        let sub = EventSubscription::with_types(vec!["DeviceMetric".to_string()]);
        assert_eq!(sub.event_types.len(), 1);
        assert!(sub.enabled);
    }

    #[test]
    fn test_event_subscription_disabled() {
        let sub = EventSubscription::with_types(vec!["DeviceMetric".to_string()]).disabled();
        assert!(!sub.enabled);
    }

    #[test]
    fn test_event_subscription_is_subscribed() {
        let sub = EventSubscription::with_types(vec!["DeviceMetric".to_string()]);
        assert!(sub.is_subscribed("DeviceMetric"));
        assert!(!sub.is_subscribed("AgentExecutionStarted"));
    }

    #[test]
    fn test_event_subscription_all_types() {
        let sub = EventSubscription::with_types(vec![]);
        assert!(sub.is_subscribed("DeviceMetric"));
        assert!(sub.is_subscribed("AgentExecutionStarted"));
    }

    #[test]
    fn test_event_subscription_disabled_no_events() {
        let sub = EventSubscription::with_types(vec!["DeviceMetric".to_string()]).disabled();
        assert!(!sub.is_subscribed("DeviceMetric"));
    }

    #[test]
    fn test_event_filter_new() {
        let filter = EventFilter::new();
        assert!(filter.source.is_none());
        assert!(filter.device_id.is_none());
    }

    #[test]
    fn test_event_filter_by_source() {
        let filter = EventFilter::new().by_source("devices");
        assert_eq!(filter.source, Some("devices".to_string()));
    }

    #[test]
    fn test_event_filter_by_device_id() {
        let filter = EventFilter::new().by_device_id("sensor-1");
        assert_eq!(filter.device_id, Some("sensor-1".to_string()));
    }

    #[test]
    fn test_event_filter_matches_device() {
        let filter = EventFilter::new().by_device_id("sensor-1");

        let event = serde_json::json!({
            "device_id": "sensor-1",
            "metric": "temperature"
        });

        assert!(filter.matches("DeviceMetric", &event));

        let event2 = serde_json::json!({
            "device_id": "sensor-2",
            "metric": "temperature"
        });

        assert!(!filter.matches("DeviceMetric", &event2));
    }

    #[test]
    fn test_infer_source_from_event_type() {
        assert_eq!(infer_source_from_event_type("DeviceMetric"), "devices");
        assert_eq!(infer_source_from_event_type("AgentExecutionStarted"), "agents");
        assert_eq!(infer_source_from_event_type("ExtensionOutput"), "extensions");
        assert_eq!(infer_source_from_event_type("RuleTriggered"), "rules");
    }
}
