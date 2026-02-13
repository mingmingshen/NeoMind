//! Business context structures for LLM prompt injection.

use super::meta_tools::VagueQueryHandler;
use crate::context::{
    device_registry::DeviceAlias,
    state_provider::{DeviceState, SystemSnapshot},
};
use serde::{Deserialize, Serialize};

/// Business context containing all relevant information for LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessContext {
    /// Original user query
    pub query: String,
    /// Context scope
    pub scope: ContextScope,
    /// Relevant devices (resolved from aliases)
    pub devices: Vec<DeviceAlias>,
    /// Current system state snapshot
    pub system_state: SystemSnapshot,
    /// Context timestamp
    pub timestamp: i64,
}

impl BusinessContext {
    /// Format context for inclusion in LLM prompt.
    pub fn format_for_prompt(&self) -> String {
        let mut prompt = String::new();

        // Add context header
        prompt.push_str("## 系统当前状态\n\n");

        // Add device summary
        if !self.system_state.devices.is_empty() {
            prompt.push_str(&format!(
                "**在线设备**: {} 个\n",
                self.system_state.devices.len()
            ));
            prompt.push_str("\n### 设备列表\n\n");

            // Group devices by location
            let mut by_location: std::collections::HashMap<String, Vec<&DeviceState>> =
                std::collections::HashMap::new();

            for device in &self.system_state.devices {
                by_location
                    .entry(
                        device
                            .location
                            .clone()
                            .unwrap_or_else(|| "未分类".to_string()),
                    )
                    .or_default()
                    .push(device);
            }

            for (location, devices) in &by_location {
                prompt.push_str(&format!("**{}**: ", location));
                let device_names: Vec<&str> = devices.iter().map(|d| d.name.as_str()).collect();
                prompt.push_str(&device_names.join("、"));
                prompt.push('\n');
            }
            prompt.push('\n');
        }

        // Add relevant devices from query analysis
        if !self.devices.is_empty() {
            prompt.push_str("### 相关设备\n\n");
            for device in &self.devices {
                prompt.push_str(&format!("- **{}** (`{}`)", device.name, device.device_id));
                if let Some(loc) = &device.location {
                    prompt.push_str(&format!(" - 位置: {}", loc));
                }
                if !device.capabilities.is_empty() {
                    let caps: Vec<&str> = device.capabilities.iter().map(|c| c.as_str()).collect();
                    prompt.push_str(&format!(" - 能力: {}", caps.join("、")));
                }
                prompt.push('\n');
            }
            prompt.push('\n');
        }

        // Add rules summary
        if !self.system_state.rules.is_empty() {
            prompt.push_str(&format!(
                "**活跃规则**: {} 个\n",
                self.system_state.rules.len()
            ));
        }

        // Add workflows summary
        if !self.system_state.workflows.is_empty() {
            prompt.push_str(&format!(
                "**活跃工作流**: {} 个\n",
                self.system_state.workflows.len()
            ));
        }

        // Add alerts summary
        if !self.system_state.alerts.is_empty() {
            prompt.push_str(&format!(
                "**当前告警**: {} 个\n",
                self.system_state.alerts.len()
            ));
        }

        // Add context-specific guidance
        prompt.push_str("\n## 查询理解指南\n\n");
        prompt.push_str(self.scope.guidance());

        prompt
    }

    /// Get context relevance score for a tool.
    pub fn relevance_for_tool(&self, tool_name: &str) -> ContextRelevance {
        match tool_name {
            "list_devices" | "query_data" if self.scope == ContextScope::Discovery => {
                ContextRelevance::High
            }
            "control_device" if self.scope == ContextScope::Discovery => {
                // Need to discover devices first
                ContextRelevance::Medium
            }
            "query_data" if !self.devices.is_empty() => ContextRelevance::High,
            "control_device" if !self.devices.is_empty() => ContextRelevance::High,
            _ => ContextRelevance::Normal,
        }
    }

    /// Check if the query is vague and needs clarification.
    pub fn needs_clarification(&self) -> bool {
        let handler = VagueQueryHandler::new();
        handler.is_vague(&self.query) && self.devices.is_empty()
    }

    /// Get suggested clarification question.
    pub fn get_clarification(&self) -> Option<String> {
        let handler = VagueQueryHandler::new();
        handler.get_clarification(&self.query)
    }

    /// Try to resolve vague query using context.
    pub fn resolve_vague_query(&self) -> Option<String> {
        let handler = VagueQueryHandler::new();
        let device_ids: Vec<String> = self
            .system_state
            .devices
            .iter()
            .map(|d| d.device_id.clone())
            .collect();

        handler.resolve_with_context(&self.query, &device_ids)
    }

    /// Get recommended devices for the query.
    pub fn recommended_devices(&self) -> Vec<&DeviceAlias> {
        // For temperature/humidity queries, recommend devices with that capability
        let query_lower = self.query.to_lowercase();

        if query_lower.contains("温度") || query_lower.contains("temperature") {
            return self
                .devices
                .iter()
                .filter(|d| {
                    d.capabilities
                        .iter()
                        .any(|c| c.to_lowercase().contains("temperature") || c == "温度")
                })
                .collect();
        }

        if query_lower.contains("湿度") || query_lower.contains("humidity") {
            return self
                .devices
                .iter()
                .filter(|d| {
                    d.capabilities
                        .iter()
                        .any(|c| c.to_lowercase().contains("humidity") || c == "湿度")
                })
                .collect();
        }

        // For location-based queries, recommend devices at that location
        if query_lower.contains("客厅") || query_lower.contains("living") {
            return self
                .devices
                .iter()
                .filter(|d| {
                    d.location
                        .as_ref()
                        .is_some_and(|l| l.contains("客厅") || l.to_lowercase().contains("living"))
                })
                .collect();
        }

        if query_lower.contains("卧室") || query_lower.contains("bedroom") {
            return self
                .devices
                .iter()
                .filter(|d| {
                    d.location
                        .as_ref()
                        .is_some_and(|l| l.contains("卧室") || l.to_lowercase().contains("bedroom"))
                })
                .collect();
        }

        Vec::new()
    }
}

/// Context scope determines how much information to include.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextScope {
    /// Minimal context - only essential system info
    Minimal,
    /// Discovery context - user wants to explore
    Discovery,
    /// Standard context - typical query
    Standard,
    /// Focused context - specific device/entity mentioned
    Focused,
    /// Location context - query about a location
    Location,
    /// Full context - complete system overview
    Full,
}

impl ContextScope {
    /// Get guidance text for this scope.
    pub fn guidance(&self) -> &'static str {
        match self {
            ContextScope::Minimal => "用户查询简单，直接回答问题即可。",
            ContextScope::Discovery => {
                "用户想了解系统但未指定具体设备。先调用 list_devices 查看可用设备，\
                然后根据设备能力引导用户。"
            }
            ContextScope::Standard => {
                "用户询问数据或状态但未指定设备。分析上下文，如果只有一个相关设备则直接查询，\
                否则列出选项让用户选择。"
            }
            ContextScope::Focused => "用户指定了具体设备，直接执行相应操作即可。",
            ContextScope::Location => "用户指定了位置。先查询该位置的设备，然后执行相应操作。",
            ContextScope::Full => "用户请求系统概览。调用相关列表工具返回完整信息。",
        }
    }

    /// Estimated token count for this scope.
    pub fn estimated_tokens(&self) -> usize {
        match self {
            ContextScope::Minimal => 100,
            ContextScope::Discovery => 300,
            ContextScope::Standard => 400,
            ContextScope::Focused => 250,
            ContextScope::Location => 350,
            ContextScope::Full => 600,
        }
    }
}

/// Relevance score for context-tool matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextRelevance {
    /// High relevance - tool should be prioritized
    High,
    /// Medium relevance - tool may be useful
    Medium,
    /// Normal relevance - standard priority
    Normal,
    /// Low relevance - tool likely not needed
    Low,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_scope_guidance() {
        assert!(ContextScope::Discovery.guidance().contains("list_devices"));
        assert!(ContextScope::Full.guidance().contains("概览"));
    }

    #[test]
    fn test_relevance_for_tool() {
        let context = BusinessContext {
            query: "温度是多少".to_string(),
            scope: ContextScope::Standard,
            devices: vec![],
            system_state: SystemSnapshot::default(),
            timestamp: 0,
        };

        // Discovery scope should make list_devices high relevance
        assert_eq!(
            context.relevance_for_tool("list_devices"),
            ContextRelevance::Normal
        );
    }
}
