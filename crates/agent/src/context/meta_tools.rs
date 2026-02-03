//! Meta-tools for context discovery and system understanding.
//!
//! These tools help the LLM understand the system before taking action.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Meta-tool for context discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaTool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool category
    pub category: MetaToolCategory,
    /// When to use this tool
    pub use_when: Vec<String>,
    /// Example queries that would trigger this tool
    pub examples: Vec<String>,
}

/// Category of meta-tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetaToolCategory {
    Discovery,
    Resolution,
    Validation,
}

impl MetaTool {
    /// Check if this tool should be used for a query.
    pub fn should_use(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.use_when.iter().any(|pattern| {
            query_lower.contains(pattern.to_lowercase().as_str())
        })
    }
}

/// Context for meta-tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    /// Original query
    pub query: String,
    /// Extracted keywords
    pub keywords: Vec<String>,
    /// Inferred intent
    pub intent: String,
    /// Confidence score
    pub confidence: f32,
}

/// Registry of meta-tools.
pub struct MetaToolRegistry {
    tools: HashMap<String, MetaTool>,
}

impl MetaToolRegistry {
    /// Create a new meta-tool registry.
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        registry.register_default_tools();
        registry
    }

    /// Register default meta-tools.
    fn register_default_tools(&mut self) {
        // search_devices tool
        self.register(MetaTool {
            name: "search_devices".to_string(),
            description: "搜索系统中的设备。支持按名称、位置、能力模糊搜索。".to_string(),
            category: MetaToolCategory::Discovery,
            use_when: vec![
                "哪些设备".to_string(),
                "有什么设备".to_string(),
                "找不到设备".to_string(),
                "设备在哪".to_string(),
            ],
            examples: vec![
                "有哪些设备".to_string(),
                "客厅有什么设备".to_string(),
                "哪些设备可以测温度".to_string(),
            ],
        });

        // get_system_state tool
        self.register(MetaTool {
            name: "get_system_state".to_string(),
            description: "获取系统当前状态概览，包括设备、规则、工作流和告警信息。".to_string(),
            category: MetaToolCategory::Discovery,
            use_when: vec![
                "系统状态".to_string(),
                "当前状态".to_string(),
                "怎么样".to_string(),
                "运行情况".to_string(),
            ],
            examples: vec![
                "系统状态如何".to_string(),
                "当前有什么告警".to_string(),
            ],
        });

        // resolve_device tool
        self.register(MetaTool {
            name: "resolve_device".to_string(),
            description: "根据模糊名称或位置解析设备ID。当用户使用自然语言指代设备时使用。".to_string(),
            category: MetaToolCategory::Resolution,
            use_when: vec![
                "打开灯".to_string(),
                "关闭设备".to_string(),
                "调节".to_string(),
                "控制".to_string(),
            ],
            examples: vec![
                "打开客厅的灯".to_string(),
                "把卧室灯调暗一点".to_string(),
            ],
        });

        // validate_device_capability tool
        self.register(MetaTool {
            name: "validate_device_capability".to_string(),
            description: "验证设备是否支持特定能力（如温度、湿度、亮度等）。".to_string(),
            category: MetaToolCategory::Validation,
            use_when: vec![
                "温度".to_string(),
                "湿度".to_string(),
                "亮度".to_string(),
                "能不能".to_string(),
            ],
            examples: vec![
                "这个传感器能测温度吗".to_string(),
                "客厅灯能调亮度吗".to_string(),
            ],
        });
    }

    /// Register a meta-tool.
    pub fn register(&mut self, tool: MetaTool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Get a meta-tool by name.
    pub fn get(&self, name: &str) -> Option<&MetaTool> {
        self.tools.get(name)
    }

    /// Get all meta-tools.
    pub fn list(&self) -> Vec<&MetaTool> {
        self.tools.values().collect()
    }

    /// Find relevant meta-tools for a query.
    pub fn find_relevant(&self, query: &str) -> Vec<&MetaTool> {
        self.tools.values()
            .filter(|tool| tool.should_use(query))
            .collect()
    }

    /// Get tool definitions for function calling.
    pub fn to_tool_definitions(&self) -> Vec<edge_ai_core::llm::backend::ToolDefinition> {
        self.tools.values()
            .map(|tool| edge_ai_core::llm::backend::ToolDefinition {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "用户原始查询"
                        }
                    },
                    "required": ["query"]
                }),
            })
            .collect()
    }
}

impl Default for MetaToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Vague query handler for processing ambiguous user requests.
pub struct VagueQueryHandler {
    /// Common vague patterns
    vague_patterns: Vec<VaguePattern>,
}

/// Pattern for detecting vague queries.
#[derive(Debug, Clone)]
struct VaguePattern {
    /// Pattern regex (simplified as keyword list)
    keywords: Vec<String>,
    /// Expected missing information
    _missing_info: String,
    /// Suggested follow-up question
    follow_up: String,
}

impl VagueQueryHandler {
    /// Create a new vague query handler.
    pub fn new() -> Self {
        Self {
            vague_patterns: vec![
                VaguePattern {
                    keywords: vec!["温度".to_string(), "是多少".to_string()],
                    _missing_info: "device".to_string(),
                    follow_up: "您想查询哪个设备的温度？".to_string(),
                },
                VaguePattern {
                    keywords: vec!["湿度".to_string(), "是多少".to_string()],
                    _missing_info: "device".to_string(),
                    follow_up: "您想查询哪个设备的湿度？".to_string(),
                },
                VaguePattern {
                    keywords: vec!["打开".to_string(), "灯".to_string()],
                    _missing_info: "location".to_string(),
                    follow_up: "您想打开哪个位置的灯？".to_string(),
                },
                VaguePattern {
                    keywords: vec!["关闭".to_string(), "灯".to_string()],
                    _missing_info: "location".to_string(),
                    follow_up: "您想关闭哪个位置的灯？".to_string(),
                },
            ],
        }
    }

    /// Check if a query is vague.
    /// A query is NOT vague if it contains a device identifier (like sensor_1, light_living, etc.)
    /// or a location keyword (like 客厅, 卧室, etc.)
    pub fn is_vague(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        // Check if query contains a device identifier (not vague if it does)
        // Device identifiers typically are like: sensor_1, light_living, temp_sensor, etc.
        let has_device_id = query_lower.contains("sensor_")
            || query_lower.contains("device_")
            || query_lower.contains("light_")
            || query_lower.contains("temp_")
            || query_lower.contains("humid_")
            || query_lower.contains("switch_")
            || query_lower.contains("_")
                && (query_lower.contains("sensor") || query_lower.contains("device"));

        // Check if query contains a location (not vague if it does)
        let has_location = query_lower.contains("客厅")
            || query_lower.contains("卧室")
            || query_lower.contains("厨房")
            || query_lower.contains("卫生间")
            || query_lower.contains("living")
            || query_lower.contains("bedroom")
            || query_lower.contains("kitchen")
            || query_lower.contains("bathroom");

        if has_device_id || has_location {
            return false;
        }

        // Check against patterns
        for pattern in &self.vague_patterns {
            let matches = pattern.keywords.iter()
                .all(|kw| query_lower.contains(&kw.to_lowercase()));

            if matches {
                return true;
            }
        }

        false
    }

    /// Get clarification suggestions for a vague query.
    pub fn get_clarification(&self, query: &str) -> Option<String> {
        let query_lower = query.to_lowercase();

        for pattern in &self.vague_patterns {
            let matches = pattern.keywords.iter()
                .all(|kw| query_lower.contains(&kw.to_lowercase()));

            if matches {
                return Some(pattern.follow_up.clone());
            }
        }

        None
    }

    /// Try to resolve a vague query using context.
    pub fn resolve_with_context(
        &self,
        query: &str,
        available_devices: &[String],
    ) -> Option<String> {
        let query_lower = query.to_lowercase();

        // Temperature query - look for temperature sensors
        if query_lower.contains("温度") {
            for device in available_devices {
                if device.to_lowercase().contains("sensor")
                    || device.to_lowercase().contains("温度")
                {
                    return Some(format!("{}的温度", device));
                }
            }
        }

        // Light control with location
        if (query_lower.contains("打开") || query_lower.contains("关闭"))
            && query_lower.contains("灯")
        {
            for device in available_devices {
                if device.to_lowercase().contains("light") || device.to_lowercase().contains("灯") {
                    return Some(format!("{}的灯", device));
                }
            }
        }

        None
    }
}

impl Default for VagueQueryHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_tool_should_use() {
        let registry = MetaToolRegistry::new();
        let tool = registry.get("search_devices").unwrap();

        assert!(tool.should_use("有哪些设备"));
        assert!(tool.should_use("客厅有什么设备"));
        assert!(!tool.should_use("温度是多少"));
    }

    #[test]
    fn test_vague_query_detection() {
        let handler = VagueQueryHandler::new();

        assert!(handler.is_vague("温度是多少"));
        assert!(handler.is_vague("打开灯"));

        assert!(!handler.is_vague("sensor_1的温度是多少"));
        assert!(!handler.is_vague("打开客厅灯"));
    }

    #[test]
    fn test_vague_query_clarification() {
        let handler = VagueQueryHandler::new();

        assert_eq!(
            handler.get_clarification("温度是多少"),
            Some("您想查询哪个设备的温度？".to_string())
        );

        assert_eq!(
            handler.get_clarification("打开灯"),
            Some("您想打开哪个位置的灯？".to_string())
        );
    }

    #[test]
    fn test_vague_query_resolution() {
        let handler = VagueQueryHandler::new();
        let devices = vec!["sensor_1".to_string(), "light_living".to_string()];

        let resolved = handler.resolve_with_context("温度是多少", &devices);
        assert!(resolved.is_some());
        assert!(resolved.unwrap().contains("sensor"));
    }
}
