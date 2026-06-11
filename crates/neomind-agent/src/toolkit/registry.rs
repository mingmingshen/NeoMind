//! Tool registry for managing available tools.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::Value;

use super::error::{Result, ToolError};
use super::tool::{DynTool, ToolDefinition, ToolOutput};

/// Tool registry for managing available tools.
///
/// Caches serialized tool definitions to avoid redundant JSON serialization
/// on every LLM call. The cache is invalidated when tools are registered or
/// unregistered.
pub struct ToolRegistry {
    tools: HashMap<String, DynTool>,
    /// Cached tool definitions (rebuilt on register/unregister).
    cached_definitions: RwLock<Option<Vec<ToolDefinition>>>,
}

impl ToolRegistry {
    /// Create a new tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            cached_definitions: RwLock::new(None),
        }
    }

    /// Invalidate cached definitions (call after any mutation).
    fn invalidate_cache(&self) {
        *self.cached_definitions.write() = None;
    }

    /// Register a tool.
    pub fn register(&mut self, tool: DynTool) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
        self.invalidate_cache();
    }

    /// Register multiple tools.
    pub fn register_all(&mut self, tools: Vec<DynTool>) {
        for tool in tools {
            let name = tool.name().to_string();
            self.tools.insert(name, tool);
        }
        self.invalidate_cache();
    }

    /// Unregister a tool by name.
    pub fn unregister(&mut self, name: &str) -> bool {
        let removed = self.tools.remove(name).is_some();
        if removed {
            self.invalidate_cache();
        }
        removed
    }

    /// Get a tool by name.
    ///
    /// Falls back to desanitized lookup for extension tools whose names were
    /// sanitized for API compatibility (e.g., `test_extension_cmd` → `test.extension:cmd`).
    pub fn get(&self, name: &str) -> Option<&DynTool> {
        if let Some(tool) = self.tools.get(name) {
            return Some(tool);
        }
        // Fallback: try to find a tool whose sanitized name matches the requested name.
        // This handles the case where the LLM returns a sanitized tool name.
        self.tools.values().find(|tool| {
            let sanitized = neomind_core::llm::backend::sanitize_tool_name(tool.name());
            sanitized == name
        })
    }

    /// Check if a tool exists (with sanitized name fallback).
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name) || self.get(name).is_some()
    }

    /// List all tool names.
    pub fn list(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get all tool definitions (cached).
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        if let Some(ref defs) = *self.cached_definitions.read() {
            return defs.clone();
        }
        let defs: Vec<ToolDefinition> = self.tools.values().map(|t| t.definition()).collect();
        *self.cached_definitions.write() = Some(defs.clone());
        defs
    }

    /// Execute a tool by name.
    pub async fn execute(&self, name: &str, args: Value) -> Result<ToolOutput> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.execute(args).await
    }

    /// Execute multiple tools in parallel using `JoinSet` for lower overhead
    /// than spawning individual tasks.
    pub async fn execute_parallel(&self, calls: Vec<ToolCall>) -> Vec<ToolResult> {
        use tokio::task::JoinSet;

        if calls.is_empty() {
            return Vec::new();
        }

        let mut join_set = JoinSet::new();

        for call in calls {
            if let Some(tool) = self.get(&call.name) {
                let tool_clone = tool.clone();
                let args = call.args;
                let name = call.name;

                join_set.spawn(async move {
                    ToolResult {
                        name,
                        result: tool_clone.execute(args).await,
                    }
                });
            } else {
                let name = call.name;
                join_set.spawn(async move {
                    ToolResult {
                        name: name.clone(),
                        result: Err(ToolError::NotFound(name)),
                    }
                });
            }
        }

        let mut results = Vec::with_capacity(join_set.len());
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(tool_result) => results.push(tool_result),
                Err(e) => {
                    tracing::error!("Tool task panicked: {}", e);
                    results.push(ToolResult {
                        name: String::new(),
                        result: Err(ToolError::Execution(format!("Tool task panicked: {}", e))),
                    });
                }
            }
        }
        results
    }

    /// Get the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.tools.len() == 0
    }

    /// Prepare the MemoryTool for a new agent execution.
    /// Creates fresh per-execution handles (concurrency-safe) and swaps them in.
    /// Returns (agent_id_handle, knowledge_files_handle) for the executor to use.
    pub fn prepare_memory_tool_execution(
        &self,
        agent_id: String,
        knowledge_files: Vec<neomind_storage::KnowledgeFileRef>,
    ) -> Option<(
        std::sync::Arc<tokio::sync::RwLock<Option<String>>>,
        std::sync::Arc<tokio::sync::RwLock<Vec<neomind_storage::KnowledgeFileRef>>>,
    )> {
        let tool = self.tools.get("memory")?;
        tool.swap_agent_context(agent_id, knowledge_files)
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
    // Domain Tools
    // ============================================================================

    /// Add shell tool for system command execution.
    ///
    /// Only registers the tool when config is `Some` and `enabled: true`.
    pub fn with_shell_tool(mut self, config: Option<super::shell::ShellConfig>) -> Self {
        if let Some(shell_config) = config {
            if shell_config.enabled {
                self.registry
                    .register(Arc::new(super::shell::ShellTool::new(shell_config)));
            }
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolkit::{Tool, ToolOutput};
    use async_trait::async_trait;
    use neomind_core::tools::ToolCategory;
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
    fn test_tool_call() {
        let call =
            ToolCall::new("test_tool", serde_json::json!({"key": "value"})).with_id("call_123");

        assert_eq!(call.name, "test_tool");
        assert_eq!(call.id, Some("call_123".to_string()));
    }
}
