//! Tool registry for managing available tools.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use super::error::{Result, ToolError};
use super::tool::{DynTool, ToolDefinition, ToolOutput};

/// Tool registry for managing available tools.
pub struct ToolRegistry {
    tools: HashMap<String, DynTool>,
}

impl ToolRegistry {
    /// Create a new tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: DynTool) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// Register multiple tools.
    pub fn register_all(&mut self, tools: Vec<DynTool>) {
        for tool in tools {
            self.register(tool);
        }
    }

    /// Unregister a tool by name.
    pub fn unregister(&mut self, name: &str) -> bool {
        self.tools.remove(name).is_some()
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&DynTool> {
        self.tools.get(name)
    }

    /// Check if a tool exists.
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// List all tool names.
    pub fn list(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get all tool definitions.
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Get tool definitions as JSON (for LLM).
    pub fn definitions_json(&self) -> Value {
        let defs: Vec<Value> = self
            .definitions()
            .into_iter()
            .map(|d| serde_json::to_value(d).unwrap())
            .collect();
        serde_json::json!({ "tools": defs })
    }

    /// Execute a tool by name.
    pub async fn execute(&self, name: &str, args: Value) -> Result<ToolOutput> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.execute(args).await
    }

    /// Execute multiple tools in parallel.
    pub async fn execute_parallel(&self, calls: Vec<ToolCall>) -> Vec<ToolResult> {
        let mut tasks = Vec::new();

        for call in calls {
            if let Some(tool) = self.get(&call.name) {
                let tool_clone = tool.clone();
                let args = call.args;
                let name = call.name.clone();

                tasks.push(tokio::spawn(async move {
                    ToolResult {
                        name,
                        result: tool_clone.execute(args).await,
                    }
                }));
            } else {
                let name = call.name.clone();
                let name_clone = name.clone();
                tasks.push(tokio::spawn(async move {
                    ToolResult {
                        name: name_clone,
                        result: Err(ToolError::NotFound(name)),
                    }
                }));
            }
        }

        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await.unwrap());
        }
        results
    }

    /// Get the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Search for tools by keyword.
    ///
    /// Searches tool names and descriptions for the given keyword.
    /// Returns a list of tool definitions that match.
    pub fn search(&self, keyword: &str) -> Vec<ToolDefinition> {
        let keyword_lower = keyword.to_lowercase();
        self.tools
            .values()
            .filter(|tool| {
                let name_matches = tool.name().to_lowercase().contains(&keyword_lower);
                let desc_matches = tool.description().to_lowercase().contains(&keyword_lower);
                name_matches || desc_matches
            })
            .map(|t| t.definition())
            .collect()
    }

    /// Search for tools by keyword with category filter.
    ///
    /// Allows filtering by tool category prefix (e.g., "device", "rule", "agent").
    pub fn search_with_category(
        &self,
        keyword: &str,
        category_prefix: Option<&str>,
    ) -> Vec<ToolDefinition> {
        let results = self.search(keyword);
        if let Some(prefix) = category_prefix {
            let prefix_lower = prefix.to_lowercase();
            results
                .into_iter()
                .filter(|def| def.name.to_lowercase().starts_with(&prefix_lower))
                .collect()
        } else {
            results
        }
    }

    /// Get tool categories (prefixes).
    ///
    /// Returns unique category prefixes from tool names.
    /// For example, "list_devices", "get_device" -> ["device", "list", "get"]
    pub fn categories(&self) -> Vec<String> {
        let mut categories = std::collections::HashSet::new();
        for tool_name in self.tools.keys() {
            // Extract common prefixes
            for (i, _) in tool_name.match_indices('_') {
                let prefix = &tool_name[..i];
                if prefix.len() >= 3 {
                    categories.insert(prefix.to_string());
                }
            }
        }
        let mut result: Vec<String> = categories.into_iter().collect();
        result.sort();
        result
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A tool call request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    /// Tool name
    pub name: String,
    /// Tool arguments
    pub args: Value,
    /// Optional call ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl ToolCall {
    /// Create a new tool call.
    pub fn new(name: impl Into<String>, args: Value) -> Self {
        Self {
            name: name.into(),
            args,
            id: None,
        }
    }

    /// Set the call ID.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

/// Result of a tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Tool name
    pub name: String,
    /// Execution result
    pub result: Result<ToolOutput>,
}

/// Builder for creating a tool registry with common tools.
///
/// # Example
///
/// ```
/// use neomind_agent::toolkit::ToolRegistryBuilder;
///
/// let registry = ToolRegistryBuilder::new().build();
/// assert_eq!(registry.len(), 0);
///```
pub struct ToolRegistryBuilder {
    registry: ToolRegistry,
    extension_registry: Option<Arc<neomind_core::extension::registry::ExtensionRegistry>>,
}

impl ToolRegistryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
            extension_registry: None,
        }
    }

    /// Add a custom tool.
    pub fn with_tool(mut self, tool: DynTool) -> Self {
        self.registry.register(tool);
        self
    }

    /// Set the extension registry for scanning extension tools.
    pub fn with_extension_registry(
        mut self,
        registry: Arc<neomind_core::extension::registry::ExtensionRegistry>,
    ) -> Self {
        self.extension_registry = Some(registry);
        self
    }

    /// Scan extensions and add their tools to the registry.
    ///
    /// Returns the builder on success (with extension tools added), or the original builder on error.
    /// Call `.build()` after this method to get the final registry.
    pub async fn with_extensions_scanned(mut self) -> Self {
        if let Some(ext_registry) = &self.extension_registry {
            use super::extension_tools::ExtensionToolExecutor;

            let executor = ExtensionToolExecutor::new(ext_registry.clone());
            let tools = executor.generate_tools().await;

            for tool in tools {
                self.registry.register(Arc::new(tool));
            }
        }
        self
    }

    // ============================================================================
    // Aggregated Tools (Action-based design for token efficiency)
    // ============================================================================

    /// Add aggregated tools with optional message manager for alert tool.
    ///
    /// This consolidates 34+ individual tools into 5 action-based tools,
    /// reducing tool definition token usage by ~60%.
    pub fn with_aggregated_tools_full(
        mut self,
        device_service: Arc<neomind_devices::DeviceService>,
        storage: Arc<neomind_devices::TimeSeriesStorage>,
        agent_store: Arc<neomind_storage::AgentStore>,
        rule_engine: Arc<neomind_rules::RuleEngine>,
        rule_history: Option<Arc<neomind_rules::RuleHistoryStorage>>,
        message_manager: Option<Arc<neomind_messages::MessageManager>>,
    ) -> Self {
        use super::aggregated::AggregatedToolsBuilder;

        let mut builder = AggregatedToolsBuilder::new()
            .with_device_service(device_service)
            .with_time_series_storage(storage)
            .with_agent_store(agent_store)
            .with_rule_engine(rule_engine);

        if let Some(history) = rule_history {
            builder = builder.with_rule_history(history);
        }

        if let Some(manager) = message_manager {
            builder = builder.with_message_manager(manager);
        }

        if let Some(ext_reg) = self.extension_registry.clone() {
            builder = builder.with_extension_registry(ext_reg);
        }

        let tools = builder.build();

        for tool in tools {
            self.registry.register(tool);
        }

        self
    }

    /// Build the registry.
    pub fn build(self) -> ToolRegistry {
        self.registry
    }
}

impl Default for ToolRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to format tool definitions for LLM function calling.
///
/// This function creates a comprehensive, structured prompt that guides the LLM
/// on how to use tools effectively, including:
/// - Tool descriptions
/// - Usage scenarios with example queries
/// - Tool relationships (call order, data flow)
/// - Tool categories for organization
pub fn format_for_llm(definitions: &[ToolDefinition]) -> String {
    let mut result = String::from("Available Tools\n");
    result.push_str(&"═".repeat(40));
    result.push_str("\n\n");

    // Group tools by category
    let mut grouped: std::collections::HashMap<String, Vec<&ToolDefinition>> =
        std::collections::HashMap::new();
    for def in definitions {
        if def.deprecated {
            continue; // Skip deprecated tools
        }
        let category = def.category.as_str();
        grouped.entry(category.to_string()).or_default().push(def);
    }

    // Define category order
    let category_order = vec![
        "device", "data", "analysis", "rule", "alert", "agent", "config", "system",
    ];

    // Output tools by category
    for category in category_order {
        if let Some(tools) = grouped.get(category) {
            let category_name = match category {
                "device" => "Device Management",
                "data" => "Data Query",
                "analysis" => "Data Analysis",
                "rule" => "Rule Management",
                "alert" => "Message Management",
                "message" => "Message Management",
                "agent" => "Agent Management",
                "config" => "Configuration",
                "system" => "System Tools",
                _ => category,
            };
            result.push_str(&format!("### {}\n\n", category_name));

            for def in tools {
                // Tool name and description
                result.push_str(&format!("**Tool**: `{}`\n", def.name));
                result.push_str(&format!("**Description**: {}\n", def.description));

                // Usage scenarios
                if !def.scenarios.is_empty() {
                    result.push_str("**Usage Scenarios**:\n");
                    for (i, scenario) in def.scenarios.iter().enumerate() {
                        result.push_str(&format!(
                            "  {}. {} - 示例: \"{}\"\n",
                            i + 1,
                            scenario.description,
                            scenario.example_query
                        ));
                    }
                }

                // Tool relationships
                if !def.relationships.call_after.is_empty()
                    || !def.relationships.output_to.is_empty()
                {
                    result.push_str("**Relationships**:\n");
                    if !def.relationships.call_after.is_empty() {
                        result.push_str(&format!(
                            "  → Call after: {}\n",
                            def.relationships.call_after.join(", ")
                        ));
                    }
                    if !def.relationships.output_to.is_empty() {
                        result.push_str(&format!(
                            "  → Output feeds into: {}\n",
                            def.relationships.output_to.join(", ")
                        ));
                    }
                }

                // Parameters
                result.push_str("**Parameters**:\n");
                if let Some(props) = def.parameters.get("properties") {
                    if let Some(obj) = props.as_object() {
                        for (name, prop) in obj {
                            let desc = prop
                                .get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("no description");
                            let type_name = prop
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("unknown");
                            result.push_str(&format!("  - `{}`: {} ({})", name, desc, type_name));

                            // Check if required
                            if let Some(required) = def.parameters.get("required") {
                                if let Some(arr) = required.as_array() {
                                    if arr.iter().any(|v| v.as_str() == Some(name)) {
                                        result.push_str(" **[required]**");
                                    }
                                }
                            }
                            result.push('\n');
                        }
                    }
                }

                if let Some(required) = def.parameters.get("required") {
                    if let Some(arr) = required.as_array() {
                        if !arr.is_empty() {
                            let required_names: Vec<&str> =
                                arr.iter().filter_map(|v| v.as_str()).collect();
                            result.push_str(&format!(
                                "**Required**: {}\n",
                                required_names.join(", ")
                            ));
                        }
                    }
                }

                result.push('\n');
            }
        }
    }

    // Add guidance section
    result.push_str(&"-".repeat(40));
    result.push_str("\n**Tool Call Guide**\n\n");
    result.push_str("1. **List devices** → device(action=\"list\")\n");
    result.push_str("2. **Query data** → device(action=\"query\", device_id=\"id\", metric=\"name\")\n");
    result.push_str("3. **Control device** → device(action=\"control\", device_id=\"id\", command=\"cmd\", confirm=true)\n\n");
    result.push_str("**Common Flows**:\n");
    result.push_str("- Query device data: device(action=\"list\") → device(action=\"query\")\n");
    result.push_str("- Create rule: device(action=\"list\") → rule(action=\"create\")\n");
    result.push_str("- Device control: device(action=\"list\") → device(action=\"control\")\n");

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolkit::ToolDefinition;
    use crate::toolkit::{Tool, ToolOutput};
    use async_trait::async_trait;
    use neomind_core::tools::{ToolCategory, ToolRelationships};
    use serde_json::Value;

    // Simple test tool for registry testing
    struct TestTool {
        name: String,
    }

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
            })
        }

        fn category(&self) -> ToolCategory {
            ToolCategory::System
        }

        async fn execute(&self, _args: Value) -> super::Result<ToolOutput> {
            Ok(ToolOutput {
                success: true,
                data: serde_json::json!({"result": "test"}),
                error: None,
                metadata: None,
            })
        }
    }

    #[tokio::test]
    async fn test_registry_register() {
        let mut registry = ToolRegistry::new();
        assert_eq!(registry.len(), 0);

        let tool = Arc::new(TestTool {
            name: "test_tool".to_string(),
        });
        registry.register(tool);

        assert_eq!(registry.len(), 1);
        assert!(registry.has("test_tool"));
    }

    #[tokio::test]
    async fn test_registry_get() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(TestTool {
            name: "test_tool".to_string(),
        });
        registry.register(tool.clone());

        let retrieved = registry.get("test_tool");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test_tool");
    }

    #[tokio::test]
    async fn test_registry_execute() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(TestTool {
            name: "test_tool".to_string(),
        });
        registry.register(tool);

        let result = registry
            .execute("test_tool", serde_json::json!({}))
            .await
            .unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_registry_execute_not_found() {
        let registry = ToolRegistry::new();

        let result = registry
            .execute("unknown_tool", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_registry_execute_parallel() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestTool {
            name: "tool1".to_string(),
        }));
        registry.register(Arc::new(TestTool {
            name: "tool2".to_string(),
        }));

        let calls = vec![
            ToolCall::new("tool1", serde_json::json!({})),
            ToolCall::new("tool2", serde_json::json!({})),
        ];

        let results = registry.execute_parallel(calls).await;
        assert_eq!(results.len(), 2);
        assert!(results[0].result.as_ref().unwrap().success);
        assert!(results[1].result.as_ref().unwrap().success);
    }

    #[test]
    fn test_format_for_llm() {
        let definitions = vec![ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "arg1": {
                        "type": "string",
                        "description": "First argument"
                    }
                },
                "required": ["arg1"]
            }),
            example: None,
            examples: vec![],
            response_format: Some("concise".to_string()),
            namespace: Some("test".to_string()),
            category: ToolCategory::System,
            scenarios: vec![],
            relationships: ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
        }];

        let formatted = format_for_llm(&definitions);
        assert!(formatted.contains("test_tool"));
        assert!(formatted.contains("A test tool"));
        assert!(formatted.contains("arg1"));
    }

    #[test]
    fn test_tool_call() {
        let call =
            ToolCall::new("test_tool", serde_json::json!({"key": "value"})).with_id("call_123");

        assert_eq!(call.name, "test_tool");
        assert_eq!(call.id, Some("call_123".to_string()));
    }
}
