//! Dynamic tool generation from resource index.
//!
//! This module generates tool definitions dynamically based on available resources
//! instead of using hardcoded tool definitions.

use std::sync::Arc;
use tokio::sync::RwLock;

use serde_json::json;

use super::resource_index::{ResourceIndex, Resource, CapabilityType};

/// Dynamic tool generator that creates tools from available resources.
pub struct DynamicToolGenerator {
    /// Resource index
    index: Arc<RwLock<ResourceIndex>>,
    /// Cached tool definitions
    cached_tools: Arc<RwLock<Vec<ToolDefinition>>>,
    /// Cache validity timestamp
    cache_time: Arc<RwLock<i64>>,
    /// Cache duration in seconds
    cache_duration: i64,
}

impl DynamicToolGenerator {
    /// Create a new dynamic tool generator.
    pub fn new(index: Arc<RwLock<ResourceIndex>>) -> Self {
        Self {
            index,
            cached_tools: Arc::new(RwLock::new(Vec::new())),
            cache_time: Arc::new(RwLock::new(0)),
            cache_duration: 5, // 5 seconds cache
        }
    }

    /// Set cache duration.
    pub fn with_cache_duration(mut self, seconds: i64) -> Self {
        self.cache_duration = seconds;
        self
    }

    /// Generate tool definitions from available resources.
    pub async fn generate_tools(&self) -> Vec<ToolDefinition> {
        // Check cache
        let now = chrono::Utc::now().timestamp();
        let cache_time = *self.cache_time.read().await;

        if now - cache_time < self.cache_duration {
            return self.cached_tools.read().await.clone();
        }

        // Generate fresh tools
        let index = self.index.read().await;
        let mut tools = Vec::new();

        // Always add discovery tools
        tools.extend(self.discovery_tools());

        // Generate device-specific tools
        let devices = index.list_devices().await;
        if !devices.is_empty() {
            tools.push(self.list_devices_tool(&devices));
            tools.push(self.query_data_tool(&devices));
            tools.push(self.control_device_tool(&devices));
        }

        // Generate channel tools
        let channels = index.list_channels().await;
        if !channels.is_empty() {
            tools.push(self.list_channels_tool(&channels));
            tools.push(self.send_notification_tool(&channels));
        }

        // Update cache
        *self.cached_tools.write().await = tools.clone();
        *self.cache_time.write().await = now;

        tools
    }

    /// Generate tools for a specific query context.
    pub async fn generate_tools_for_query(&self, query: &str) -> Vec<ToolDefinition> {
        let index = self.index.read().await;
        let search_results = index.search_string(query).await;

        let mut tools = Vec::new();

        // Always add discovery tools first
        tools.extend(self.discovery_tools());

        // Analyze query intent
        let query_lower = query.to_lowercase();

        // Device listing intent
        if query_lower.contains("有哪些") || query_lower.contains("列出") || query_lower.contains("所有设备") {
            let devices = index.list_devices().await;
            tools.push(self.list_devices_tool(&devices));
            return tools;
        }

        // Data query intent
        if query_lower.contains("温度") || query_lower.contains("湿度") || query_lower.contains("多少")
            || query_lower.contains("temperature") || query_lower.contains("humidity") {
            let devices = index.list_devices().await;
            tools.push(self.query_data_tool(&devices));
        }

        // Control intent
        if query_lower.contains("打开") || query_lower.contains("关闭") || query_lower.contains("控制")
            || query_lower.contains("调节") || query_lower.contains("open") || query_lower.contains("close") {
            let devices = index.list_devices().await;
            tools.push(self.control_device_tool(&devices));
        }

        // If search results found, add targeted tools
        if !search_results.is_empty() {
            for result in &search_results {
                if result.score > 0.5 {
                    tools.push(self.resource_specific_tool(&result.resource));
                }
            }
        }

        tools
    }

    /// Discovery tools - always available.
    fn discovery_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "search_resources".to_string(),
                description: "搜索系统中的资源。支持按名称、别名、位置、能力模糊搜索。返回匹配的资源列表。".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索关键词，如'温度'、'客厅'、'传感器'等"
                        }
                    },
                    "required": ["query"]
                }),
                examples: vec![
                    Example {
                        user_query: "有哪些温度传感器".to_string(),
                        tool_call: "search_resources(query='温度')".to_string(),
                    },
                    Example {
                        user_query: "客厅有什么设备".to_string(),
                        tool_call: "search_resources(query='客厅')".to_string(),
                    },
                ],
            },
            ToolDefinition {
                name: "get_system_status".to_string(),
                description: "获取系统状态概览，包括设备数量、在线状态、告警数量等。".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
                examples: vec![
                    Example {
                        user_query: "系统状态如何".to_string(),
                        tool_call: "get_system_status()".to_string(),
                    },
                ],
            },
        ]
    }

    /// Generate list_devices tool with current device context.
    fn list_devices_tool(&self, devices: &[Resource]) -> ToolDefinition {
        // Group devices by type and location for context
        let mut by_type: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut by_location: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for device in devices {
            if let Some(dev_data) = device.as_device() {
                *by_type.entry(dev_data.device_type.clone()).or_insert(0) += 1;
                if let Some(loc) = &dev_data.location {
                    *by_location.entry(loc.clone()).or_insert(0) += 1;
                }
            }
        }

        let mut type_summary = Vec::new();
        for (ty, count) in &by_type {
            type_summary.push(format!("{}: {}个", ty, count));
        }

        let mut location_summary = Vec::new();
        for (loc, count) in &by_location {
            location_summary.push(format!("{}: {}个", loc, count));
        }

        ToolDefinition {
            name: "list_devices".to_string(),
            description: format!(
                "列出系统中的所有设备。当前有{}个设备，包括{}。",
                devices.len(),
                if type_summary.is_empty() {
                    "各种类型".to_string()
                } else {
                    type_summary.join("、")
                }
            ),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "可选，按位置筛选设备"
                    },
                    "type": {
                        "type": "string",
                        "description": "可选，按类型筛选设备"
                    }
                },
                "required": []
            }),
            examples: vec![
                Example {
                    user_query: "列出所有设备".to_string(),
                    tool_call: "list_devices()".to_string(),
                },
                Example {
                    user_query: "客厅有哪些设备".to_string(),
                    tool_call: "list_devices(location='客厅')".to_string(),
                },
            ],
        }
    }

    /// Generate query_data tool with data-producing devices.
    fn query_data_tool(&self, devices: &[Resource]) -> ToolDefinition {
        // Find devices with readable metrics
        let mut metric_devices = Vec::new();
        let mut all_metrics = Vec::new();

        for device in devices {
            if let Some(dev_data) = device.as_device() {
                let metrics: Vec<_> = dev_data.capabilities.iter()
                    .filter(|c| c.cap_type == CapabilityType::Metric || c.access == crate::context::AccessType::Read || c.access == crate::context::AccessType::ReadWrite)
                    .collect();

                if !metrics.is_empty() {
                    metric_devices.push(device.name.clone());
                    for cap in &metrics {
                        if !all_metrics.contains(&cap.name) {
                            all_metrics.push(cap.name.clone());
                        }
                    }
                }
            }
        }

        let metrics_desc = if all_metrics.is_empty() {
            "温度、湿度等指标".to_string()
        } else {
            all_metrics.join("、")
        };

        let devices_desc = if metric_devices.is_empty() {
            "传感器".to_string()
        } else {
            metric_devices.iter().take(5).cloned().collect::<Vec<_>>().join("、")
        };

        ToolDefinition {
            name: "query_data".to_string(),
            description: format!(
                "查询设备数据。可查询的指标包括：{}。支持按设备或时间范围筛选。示例设备：{}。",
                metrics_desc, devices_desc
            ),
            parameters: json!({
                "type": "object",
                "properties": {
                    "device": {
                        "type": "string",
                        "description": "设备名称或ID，如'sensor_1'或'客厅温度传感器'"
                    },
                    "metric": {
                        "type": "string",
                        "description": "指标名称，如'temperature'、'humidity'等"
                    },
                    "hours": {
                        "type": "number",
                        "description": "查询过去多少小时的数据，默认24小时"
                    }
                },
                "required": ["device"]
            }),
            examples: vec![
                Example {
                    user_query: "客厅温度是多少".to_string(),
                    tool_call: "query_data(device='客厅温度传感器', metric='temperature')".to_string(),
                },
                Example {
                    user_query: "查询sensor_1的湿度".to_string(),
                    tool_call: "query_data(device='sensor_1', metric='humidity')".to_string(),
                },
            ],
        }
    }

    /// Generate control_device tool with controllable devices.
    fn control_device_tool(&self, devices: &[Resource]) -> ToolDefinition {
        // Find devices with writable commands
        let mut controllable = Vec::new();
        let mut all_commands = Vec::new();

        for device in devices {
            if let Some(dev_data) = device.as_device() {
                let commands: Vec<_> = dev_data.capabilities.iter()
                    .filter(|c| c.cap_type == CapabilityType::Command || c.access == crate::context::AccessType::Write || c.access == crate::context::AccessType::ReadWrite)
                    .collect();

                if !commands.is_empty() {
                    controllable.push(device.name.clone());
                    for cap in &commands {
                        let cmd_name = if cap.name == "power" {
                            format!("{}(开关)", device.name)
                        } else {
                            format!("{}({})", device.name, cap.name)
                        };
                        if !all_commands.contains(&cmd_name) {
                            all_commands.push(cmd_name);
                        }
                    }
                }
            }
        }

        let commands_desc = if all_commands.is_empty() {
            "打开、关闭、调节等".to_string()
        } else {
            all_commands.iter().take(5).cloned().collect::<Vec<_>>().join("、")
        };

        ToolDefinition {
            name: "control_device".to_string(),
            description: format!(
                "控制设备。支持的命令包括：{}。可以打开、关闭或调节设备状态。",
                commands_desc
            ),
            parameters: json!({
                "type": "object",
                "properties": {
                    "device": {
                        "type": "string",
                        "description": "设备名称或ID"
                    },
                    "action": {
                        "type": "string",
                        "description": "控制动作，如'on'、'off'、'toggle'、'set'等"
                    },
                    "value": {
                        "type": "string",
                        "description": "设置值，用于'set'动作，如亮度值、温度设定点等"
                    }
                },
                "required": ["device", "action"]
            }),
            examples: vec![
                Example {
                    user_query: "打开客厅灯".to_string(),
                    tool_call: "control_device(device='客厅灯', action='on')".to_string(),
                },
                Example {
                    user_query: "关闭卧室空调".to_string(),
                    tool_call: "control_device(device='卧室空调', action='off')".to_string(),
                },
            ],
        }
    }

    /// Generate list_channels tool.
    fn list_channels_tool(&self, channels: &[Resource]) -> ToolDefinition {
        ToolDefinition {
            name: "list_channels".to_string(),
            description: format!("列出所有告警通道。当前有{}个告警通道。", channels.len()),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            examples: vec![
                Example {
                    user_query: "有哪些告警通道".to_string(),
                    tool_call: "list_channels()".to_string(),
                },
            ],
        }
    }

    /// Generate send_notification tool.
    fn send_notification_tool(&self, channels: &[Resource]) -> ToolDefinition {
        let channel_names: Vec<_> = channels.iter().map(|c| c.name.clone()).collect();

        ToolDefinition {
            name: "send_notification".to_string(),
            description: format!(
                "发送通知告警。可用通道：{}。",
                channel_names.join("、")
            ),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel": {
                        "type": "string",
                        "description": "告警通道名称"
                    },
                    "message": {
                        "type": "string",
                        "description": "通知消息内容"
                    },
                    "severity": {
                        "type": "string",
                        "description": "严重级别：info、warning、error、critical"
                    }
                },
                "required": ["channel", "message"]
            }),
            examples: vec![],
        }
    }

    /// Generate a resource-specific tool.
    fn resource_specific_tool(&self, resource: &Resource) -> ToolDefinition {
        match &resource.data {
            super::resource_index::ResourceData::Device(d) => {
                let metrics: Vec<_> = d.capabilities.iter()
                    .filter(|c| c.cap_type == CapabilityType::Metric)
                    .map(|c| c.name.clone())
                    .collect();

                let commands: Vec<_> = d.capabilities.iter()
                    .filter(|c| c.cap_type == CapabilityType::Command)
                    .map(|c| c.name.clone())
                    .collect();

                ToolDefinition {
                    name: format!("device_{}", resource.id.id.replace('-', "_")),
                    description: format!(
                        "操作设备'{}'。可用指标：{}。可用命令：{}。",
                        resource.name,
                        metrics.join("、").if_empty("无".to_string()),
                        commands.join("、").if_empty("无".to_string())
                    ),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "action": {
                                "type": "string",
                                "description": "操作类型：query、control"
                            },
                            "metric": {
                                "type": "string",
                                "description": "查询的指标名称"
                            },
                            "command": {
                                "type": "string",
                                "description": "执行的命令名称"
                            },
                            "value": {
                                "type": "string",
                                "description": "命令参数值"
                            }
                        },
                        "required": ["action"]
                    }),
                    examples: vec![],
                }
            },
            _ => ToolDefinition {
                name: format!("resource_{}", resource.id.id),
                description: resource.name.clone(),
                parameters: json!({"type": "object", "properties": {}, "required": []}),
                examples: vec![],
            },
        }
    }

    /// Get formatted device summary for prompts.
    pub async fn device_summary(&self) -> String {
        let index = self.index.read().await;
        let devices = index.list_devices().await;

        if devices.is_empty() {
            return "系统当前没有设备。".to_string();
        }

        let mut summary = String::from("## 系统设备\n\n");

        // Group by location
        let mut by_location: std::collections::HashMap<String, Vec<&Resource>> =
            std::collections::HashMap::new();

        for device in &devices {
            let location = device.as_device()
                .and_then(|d| d.location.as_ref())
                .cloned()
                .unwrap_or_else(|| "未分类".to_string());

            by_location.entry(location).or_default().push(device);
        }

        // Format by location
        for (location, devices) in &by_location {
            summary.push_str(&format!("**{}**: ", location));

            let device_names: Vec<_> = devices.iter()
                .map(|d| {
                    // Add capability indicators
                    let mut indicators = Vec::new();
                    if let Some(dev_data) = d.as_device() {
                        for cap in &dev_data.capabilities {
                            if cap.cap_type == CapabilityType::Metric {
                                indicators.push("📊");
                            } else if cap.cap_type == CapabilityType::Command {
                                indicators.push("🎛️");
                            }
                        }
                    }

                    format!("{}{}", d.name, indicators.join(""))
                })
                .collect();

            summary.push_str(&device_names.join("、"));
            summary.push('\n');
        }

        summary
    }

    /// Invalidate the cache.
    pub async fn invalidate_cache(&self) {
        *self.cache_time.write().await = 0;
    }
}

/// Tool definition generated from resources.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description (includes dynamic context)
    pub description: String,
    /// Tool parameters schema
    pub parameters: serde_json::Value,
    /// Example usage
    pub examples: Vec<Example>,
}

/// Example of tool usage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Example {
    /// User query that would use this tool
    pub user_query: String,
    /// Example tool call
    pub tool_call: String,
}

/// Helper to extend empty collections.
trait IfEmpty {
    type Item;
    fn if_empty(self, default: Self::Item) -> Self::Item;
}

impl IfEmpty for String {
    type Item = String;
    fn if_empty(self, default: Self::Item) -> Self::Item {
        if self.is_empty() { default } else { self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{Capability, CapabilityType, AccessType, Resource, ResourceId};

    #[tokio::test]
    async fn test_dynamic_tool_generation() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        // Register some devices
        let device1 = Resource::device("sensor_1", "客厅温度传感器", "dht22")
            .with_location("客厅")
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("°C".to_string()),
                access: AccessType::Read,
            });

        index.write().await.register(device1).await.unwrap();

        let generator = DynamicToolGenerator::new(index);

        let tools = generator.generate_tools().await;
        assert!(!tools.is_empty());

        // Should have discovery tools + device tools
        assert!(tools.iter().any(|t| t.name == "list_devices"));
        assert!(tools.iter().any(|t| t.name == "query_data"));
    }

    #[tokio::test]
    async fn test_contextual_tool_generation() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let device = Resource::device("light_living", "客厅灯", "switch")
            .with_location("客厅")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Write,
            });

        index.write().await.register(device).await.unwrap();

        let generator = DynamicToolGenerator::new(index);

        // Query for temperature should return query_data tool
        let tools = generator.generate_tools_for_query("客厅温度是多少").await;
        assert!(tools.iter().any(|t| t.name == "query_data"));

        // Query for controlling light should return control_device tool
        let tools = generator.generate_tools_for_query("打开客厅灯").await;
        assert!(tools.iter().any(|t| t.name == "control_device"));
    }
}
