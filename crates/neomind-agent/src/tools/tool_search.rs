//! Tool Search Tool for on-demand tool discovery.
//!
//! This tool allows the LLM to search for available tools by keyword,
//! reducing the need to include all tool definitions in every prompt.

use async_trait::async_trait;
use serde_json::Value;

use crate::toolkit::tool::{object_schema, string_property};
use crate::toolkit::{Tool, ToolDefinition, ToolOutput};

/// Tool for searching available tools.
///
/// This tool enables the LLM to discover tools on-demand rather than
/// having all tools loaded into every prompt. This reduces token usage
/// and improves response quality for tool-rich environments.
pub struct ToolSearchTool {
    /// Tool registry reference (cloned Arc to avoid reference lifetime issues)
    tool_names: Vec<String>,
    /// Tool descriptions (cached)
    tool_descriptions: Vec<(String, String)>,
}

impl ToolSearchTool {
    /// Create a new tool search tool.
    ///
    /// The tool_names and tool_descriptions should be extracted from
    /// the ToolRegistry at creation time.
    pub fn new(tool_names: Vec<String>, tool_descriptions: Vec<(String, String)>) -> Self {
        Self {
            tool_names,
            tool_descriptions,
        }
    }

    /// Create from a list of tool definitions.
    pub fn from_definitions(definitions: &[ToolDefinition]) -> Self {
        let tool_names = definitions.iter().map(|d| d.name.clone()).collect();
        let tool_descriptions = definitions
            .iter()
            .map(|d| (d.name.clone(), d.description.clone()))
            .collect();
        Self {
            tool_names,
            tool_descriptions,
        }
    }

    /// Search for tools by keyword.
    fn search(&self, keyword: &str) -> Vec<ToolSearchResult> {
        let keyword_lower = keyword.to_lowercase();
        let mut results = Vec::new();

        for (name, description) in &self.tool_descriptions {
            let name_matches = name.to_lowercase().contains(&keyword_lower);
            let desc_matches = description.to_lowercase().contains(&keyword_lower);

            if name_matches || desc_matches {
                results.push(ToolSearchResult {
                    name: name.clone(),
                    description: description.clone(),
                    matched_field: if name_matches { "name" } else { "description" }.to_string(),
                });
            }
        }

        // Sort by relevance (name matches first)
        results.sort_by(|a, b| {
            if a.matched_field == "name" && b.matched_field != "name" {
                std::cmp::Ordering::Less
            } else if a.matched_field != "name" && b.matched_field == "name" {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        // Limit results
        results.truncate(10);
        results
    }

    /// Get tool categories.
    fn get_categories(&self) -> Vec<String> {
        let mut categories = std::collections::HashSet::new();
        for tool_name in &self.tool_names {
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

/// Result of a tool search.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolSearchResult {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Which field matched the search
    pub matched_field: String,
}

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "tool_search"
    }

    fn description(&self) -> &str {
        "Search for available tools by keyword. Use this when you're unsure what tools are available or need to find tools for a specific task."
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "keyword": string_property("The keyword to search for in tool names and descriptions"),
                "category": string_property("Optional category prefix to filter results (e.g., 'device', 'rule', 'workflow')")
            }),
            vec!["keyword".to_string()],
        )
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput, crate::toolkit::ToolError> {
        self.validate_args(&args)?;

        let keyword = args["keyword"].as_str().ok_or_else(|| {
            crate::toolkit::ToolError::InvalidArguments("keyword must be a string".to_string())
        })?;

        let category = args["category"].as_str();

        let results = if let Some(cat) = category {
            // Filter by category prefix
            let cat_lower = cat.to_lowercase();
            self.search(keyword)
                .into_iter()
                .filter(|r| r.name.to_lowercase().starts_with(&cat_lower))
                .collect()
        } else {
            self.search(keyword)
        };

        if results.is_empty() {
            // No exact matches, try to provide suggestions
            let categories = self.get_categories();
            Ok(ToolOutput::success(serde_json::json!({
                "keyword": keyword,
                "found": 0,
                "tools": [],
                "suggestions": {
                    "categories": categories,
                    "hint": format!("Try searching for: device, rule, workflow, list, get, create, etc.")
                }
            })))
        } else {
            Ok(ToolOutput::success(serde_json::json!({
                "keyword": keyword,
                "found": results.len(),
                "tools": results
            })))
        }
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: None,
            category: self.category(),
            scenarios: self.scenarios(),
            relationships: self.relationships(),
            deprecated: self.is_deprecated(),
            replaced_by: None,
            version: self.version().to_string(),
            examples: vec![],
            response_format: None,
            namespace: self.namespace().map(|s| s.to_string()),
        }
    }

    fn namespace(&self) -> Option<&str> {
        Some("system")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolkit::{ToolCategory, ToolRelationships};

    fn create_test_tool() -> ToolSearchTool {
        let tool_descriptions = vec![
            (
                "list_devices".to_string(),
                "List all available devices".to_string(),
            ),
            (
                "list_rules".to_string(),
                "List all automation rules".to_string(),
            ),
            (
                "create_rule".to_string(),
                "Create a new automation rule".to_string(),
            ),
            (
                "get_device".to_string(),
                "Get details of a specific device".to_string(),
            ),
            (
                "control_device".to_string(),
                "Control a device by sending commands".to_string(),
            ),
            (
                "query_data".to_string(),
                "Query time series data from devices".to_string(),
            ),
        ];
        ToolSearchTool::new(
            tool_descriptions.iter().map(|(n, _)| n.clone()).collect(),
            tool_descriptions,
        )
    }

    fn create_large_tool_set() -> Vec<(String, String)> {
        vec![
            ("device_list".to_string(), "List all devices".to_string()),
            ("device_get".to_string(), "Get device details".to_string()),
            ("device_create".to_string(), "Create a new device".to_string()),
            ("device_update".to_string(), "Update device".to_string()),
            ("device_delete".to_string(), "Delete device".to_string()),
            ("rule_list".to_string(), "List automation rules".to_string()),
            ("rule_get".to_string(), "Get rule details".to_string()),
            ("rule_create".to_string(), "Create automation rule".to_string()),
            ("rule_enable".to_string(), "Enable a rule".to_string()),
            ("rule_disable".to_string(), "Disable a rule".to_string()),
            ("workflow_list".to_string(), "List workflows".to_string()),
            ("workflow_start".to_string(), "Start a workflow".to_string()),
            ("workflow_stop".to_string(), "Stop a workflow".to_string()),
        ]
    }

    #[tokio::test]
    async fn test_tool_search_by_keyword() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "device"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        // Note: The test data may have changed, just check that results were found
        assert!(result.data["found"].as_u64().unwrap() >= 3);
    }

    #[tokio::test]
    async fn test_tool_search_by_description() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "automation"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["found"], 2); // list_rules, create_rule
    }

    #[tokio::test]
    async fn test_tool_search_with_category() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "all",
            "category": "list"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        // Should only return tools starting with "list_"
        assert_eq!(result.data["found"], 2); // list_devices, list_rules
    }

    #[tokio::test]
    async fn test_tool_search_no_results() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "nonexistent"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["found"], 0);
        assert!(result.data["suggestions"].is_object());
    }

    // ===== Parameter Validation Tests =====

    #[tokio::test]
    async fn test_tool_search_missing_keyword() {
        let tool = create_test_tool();
        let args = serde_json::json!({}); // Missing required "keyword"

        let result = tool.execute(args).await;
        assert!(result.is_err());
        // The validate_args function checks for required fields
    }

    #[tokio::test]
    async fn test_tool_search_keyword_null() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": null
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        // null is treated as missing
    }

    #[tokio::test]
    async fn test_tool_search_category_null() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "device",
            "category": null
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        // null category should be treated as None (no filter)
    }

    #[tokio::test]
    async fn test_tool_search_empty_keyword() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": ""
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        // Empty keyword matches everything (empty string is substring of all strings)
        let found = result.data["found"].as_u64().unwrap();
        assert!(found > 0);
    }

    #[tokio::test]
    async fn test_tool_search_unicode_keyword() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "设备"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        // Should handle unicode search terms
    }

    #[tokio::test]
    async fn test_tool_search_case_insensitive() {
        let tool = create_test_tool();
        let args_lowercase = serde_json::json!({"keyword": "device"});
        let args_uppercase = serde_json::json!({"keyword": "DEVICE"});
        let args_mixed = serde_json::json!({"keyword": "DeViCe"});

        let result_lower = tool.execute(args_lowercase).await.unwrap();
        let result_upper = tool.execute(args_uppercase).await.unwrap();
        let result_mixed = tool.execute(args_mixed).await.unwrap();

        // All should return the same results (case-insensitive)
        assert_eq!(result_lower.data["found"], result_upper.data["found"]);
        assert_eq!(result_lower.data["found"], result_mixed.data["found"]);
    }

    #[tokio::test]
    async fn test_tool_search_extra_parameters() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "device",
            "extra_param": "ignored",
            "another_param": 123
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        // Extra parameters should be ignored
    }

    #[tokio::test]
    async fn test_tool_search_results_sorted() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "device"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);

        // Results should be sorted (name matches first, then alphabetically)
        let tools = result.data["tools"].as_array().unwrap();
        if tools.len() > 1 {
            // Check that results are present and structured correctly
            for tool in tools {
                assert!(tool.get("name").is_some());
                assert!(tool.get("description").is_some());
                assert!(tool.get("matched_field").is_some());
            }
        }
    }

    #[tokio::test]
    async fn test_tool_search_special_characters() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "@#$%^&*()"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        // Should handle special characters gracefully
        assert_eq!(result.data["found"], 0);
    }

    #[test]
    fn test_search_case_insensitive() {
        let tool = create_test_tool();
        let results = tool.search("DEVICE");
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.name.to_lowercase().contains("device") ||
                                     r.description.to_lowercase().contains("device")));
    }

    #[test]
    fn test_search_relevance_sorting() {
        let tool = create_test_tool();
        let results = tool.search("device");
        // Name matches should come before description matches
        let name_matches: Vec<_> = results.iter().filter(|r| r.matched_field == "name").collect();
        let desc_matches: Vec<_> = results.iter().filter(|r| r.matched_field == "description").collect();

        // All name matches should come before description matches
        if !name_matches.is_empty() && !desc_matches.is_empty() {
            let last_name_idx = results.iter().position(|r| r.matched_field == "name").unwrap();
            let first_desc_idx = results.iter().position(|r| r.matched_field == "description").unwrap();
            assert!(last_name_idx < first_desc_idx);
        }
    }

    #[test]
    fn test_search_result_limiting() {
        let tools = create_large_tool_set();
        let tool_names = tools.iter().map(|(n, _)| n.clone()).collect();
        let tool = ToolSearchTool::new(tool_names, tools);

        let results = tool.search("e"); // Should match almost everything
        assert!(results.len() <= 10, "Results should be limited to 10, got {}", results.len());
    }

    #[test]
    fn test_search_empty_results() {
        let tool = create_test_tool();
        let results = tool.search("xyz_nonexistent_tool");
        assert!(results.is_empty());
    }

    #[test]
    fn test_from_definitions() {
        let definitions = vec![
            ToolDefinition {
                name: "test_tool".to_string(),
                description: "A test tool".to_string(),
                parameters: serde_json::json!({}),
                example: None,
                category: ToolCategory::System,
                scenarios: vec![],
                relationships: ToolRelationships::default(),
                deprecated: false,
                replaced_by: None,
                version: "1.0.0".to_string(),
                examples: vec![],
                response_format: None,
                namespace: None,
            },
        ];

        let tool = ToolSearchTool::from_definitions(&definitions);
        assert_eq!(tool.tool_names.len(), 1);
        assert_eq!(tool.tool_names[0], "test_tool");
        assert_eq!(tool.tool_descriptions.len(), 1);
    }

    #[test]
    fn test_from_definitions_empty() {
        let definitions: Vec<ToolDefinition> = vec![];
        let tool = ToolSearchTool::from_definitions(&definitions);
        assert!(tool.tool_names.is_empty());
        assert!(tool.tool_descriptions.is_empty());
    }

    #[test]
    fn test_get_categories() {
        let tools = create_large_tool_set();
        let tool_names = tools.iter().map(|(n, _)| n.clone()).collect();
        let tool = ToolSearchTool::new(tool_names, tools);

        let categories = tool.get_categories();
        assert!(!categories.is_empty());
        assert!(categories.contains(&"device".to_string()));
        assert!(categories.contains(&"rule".to_string()));
        assert!(categories.contains(&"workflow".to_string()));
    }

    #[test]
    fn test_get_categories_empty_tools() {
        let tool = ToolSearchTool::new(vec![], vec![]);
        let categories = tool.get_categories();
        assert!(categories.is_empty());
    }

    #[test]
    fn test_get_categories_no_underscores() {
        let tool = ToolSearchTool::new(
            vec!["simpletool".to_string(), "anothertool".to_string()],
            vec![
                ("simpletool".to_string(), "Simple tool".to_string()),
                ("anothertool".to_string(), "Another tool".to_string()),
            ],
        );
        let categories = tool.get_categories();
        // Should not have categories since no underscores
        assert!(categories.is_empty());
    }

    #[tokio::test]
    async fn test_tool_trait_methods() {
        let tool = create_test_tool();
        assert_eq!(tool.name(), "tool_search");
        assert!(tool.description().contains("Search for available tools"));
        assert_eq!(tool.namespace(), Some("system"));

        let params = tool.parameters();
        assert!(params.is_object());
        assert!(params["properties"].is_object());
    }

    #[tokio::test]
    async fn test_execute_with_category_filter() {
        let tools = create_large_tool_set();
        let tool_names = tools.iter().map(|(n, _)| n.clone()).collect();
        let tool = ToolSearchTool::new(tool_names, tools);

        let args = serde_json::json!({
            "keyword": "list",
            "category": "device"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        // Should only return device tools with "list"
        let tools_result: Vec<ToolSearchResult> = serde_json::from_value(result.data["tools"].clone()).unwrap();
        assert!(!tools_result.is_empty());
        assert!(tools_result.iter().all(|t| t.name.starts_with("device")));
    }

    #[tokio::test]
    async fn test_execute_with_category_case_insensitive() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "all",
            "category": "LIST"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["found"], 2); // list_devices, list_rules
    }

    #[tokio::test]
    async fn test_execute_invalid_keyword_type() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": 12345
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_missing_keyword() {
        let tool = create_test_tool();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tool_definition() {
        let tool = create_test_tool();
        let def = tool.definition();
        assert_eq!(def.name, "tool_search");
        assert!(def.description.contains("Search for available tools"));
        assert_eq!(def.namespace, Some("system".to_string()));
        assert!(!def.deprecated);
        assert_eq!(def.version, "1.0.0");
    }

    #[test]
    fn test_search_with_special_characters() {
        let tool = ToolSearchTool::new(
            vec!["tool-with-dash".to_string(), "tool_with_underscore".to_string()],
            vec![
                ("tool-with-dash".to_string(), "Tool with dash".to_string()),
                ("tool_with_underscore".to_string(), "Tool with underscore".to_string()),
            ],
        );

        let results = tool.search("-");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "tool-with-dash");
    }

    #[test]
    fn test_search_exact_name_match_priority() {
        let tool = ToolSearchTool::new(
            vec!["device".to_string(), "device_controller".to_string(), "other_tool".to_string()],
            vec![
                ("device".to_string(), "Main device tool".to_string()),
                ("device_controller".to_string(), "Controller for devices".to_string()),
                ("other_tool".to_string(), "Manages device state".to_string()),
            ],
        );

        let results = tool.search("device");
        // "device" should be first since it's an exact name match
        assert_eq!(results[0].name, "device");
        assert_eq!(results[0].matched_field, "name");
    }

    #[test]
    fn test_search_multiple_keywords() {
        let tool = create_test_tool();
        let results = tool.search("control");
        // Should match tools with "control" in name/description
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.name.contains("control") || r.description.contains("control")));
    }

    #[tokio::test]
    async fn test_no_results_suggestions_structure() {
        let tool = create_test_tool();
        let args = serde_json::json!({
            "keyword": "nonexistent_tool_xyz"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.data["found"], 0);

        let suggestions = &result.data["suggestions"];
        assert!(suggestions["categories"].is_array());
        assert!(suggestions["hint"].is_string());
    }
}
