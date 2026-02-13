//! Resource-aware intent resolution.
//!
//! This module combines user queries with the resource index to provide
//! accurate intent resolution with dynamic system context.

use std::sync::Arc;
use tokio::sync::RwLock;

use serde::{Deserialize, Serialize};

use super::resource_index::{ResourceIndex, SearchResult};

/// Intent resolution result with resource context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedIntent {
    /// Original user query
    pub query: String,
    /// Intent category
    pub intent: IntentCategory,
    /// Confidence score (0-1)
    pub confidence: f32,
    /// Matched resources
    pub resources: Vec<ResourceMatch>,
    /// Suggested actions
    pub actions: Vec<SuggestedAction>,
    /// Clarification needed if query is ambiguous
    pub clarification: Option<String>,
}

/// Intent category for resolved queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentCategory {
    /// List/query devices
    ListDevices,
    /// Query device data
    QueryData,
    /// Control device
    ControlDevice,
    /// Create/modify automation (rule/workflow)
    Automation,
    /// Alert/notification related
    Alert,
    /// System status
    SystemStatus,
    /// General/ambiguous
    General,
}

/// Matched resource with relevance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMatch {
    /// Resource ID
    pub resource_id: String,
    /// Resource name
    pub name: String,
    /// Match type
    pub match_type: MatchType,
    /// Relevance score (0-1)
    pub relevance: f32,
    /// Matched capabilities
    pub capabilities: Vec<String>,
}

/// How the resource matched the query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchType {
    /// Direct name match
    Direct,
    /// Alias match
    Alias,
    /// Capability match
    Capability,
    /// Location match
    Location,
    /// Partial match
    Partial,
}

/// Suggested action based on intent resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    /// Action type
    pub action_type: ActionType,
    /// Target resource (if any)
    pub target: Option<String>,
    /// Parameters for the action
    pub parameters: serde_json::Value,
    /// Human-readable description
    pub description: String,
}

/// Action type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    /// Call list_devices tool
    ListDevices,
    /// Call query_data tool
    QueryData,
    /// Call control_device tool
    ControlDevice,
    /// Call search_resources tool
    SearchResources,
    /// Ask user for clarification
    Clarify,
    /// Generic response
    Respond,
}

/// Resource-aware intent resolver.
pub struct ResourceResolver {
    /// Resource index
    index: Arc<RwLock<ResourceIndex>>,
    /// Minimum confidence for direct action
    min_confidence: f32,
}

impl ResourceResolver {
    /// Create a new resource resolver.
    pub fn new(index: Arc<RwLock<ResourceIndex>>) -> Self {
        Self {
            index,
            min_confidence: 0.6,
        }
    }

    /// Set minimum confidence threshold.
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = confidence;
        self
    }

    /// Resolve user intent with resource context.
    pub async fn resolve(&self, query: &str) -> ResolvedIntent {
        let _query_lower = query.to_lowercase();

        // Search for matching resources
        let index = self.index.read().await;
        let search_results = index.search_string(query).await;

        // Classify intent
        let (intent, confidence) = self.classify_intent(query, &search_results);

        // Extract resource matches
        let resources = self.extract_matches(&search_results);

        // Generate suggested actions
        let actions = self.generate_actions(query, &intent, &resources, &search_results);

        // Determine if clarification is needed
        let clarification = if confidence < self.min_confidence && resources.is_empty() {
            self.suggest_clarification(query, &intent)
        } else {
            None
        };

        ResolvedIntent {
            query: query.to_string(),
            intent,
            confidence,
            resources,
            actions,
            clarification,
        }
    }

    /// Classify intent considering resource context.
    fn classify_intent(&self, query: &str, results: &[SearchResult]) -> (IntentCategory, f32) {
        let query_lower = query.to_lowercase();

        // Check for specific intent patterns
        let mut intent = IntentCategory::General;
        let mut confidence = 0.5f32;
        let mut highest_specific = 0.5f32;

        // List/query devices
        if query_lower.contains("有哪些")
            || query_lower.contains("列出")
            || query_lower.contains("所有设备")
            || query_lower.contains("设备列表")
            || (query_lower.contains("什么设备") && query_lower.contains("有"))
        {
            intent = IntentCategory::ListDevices;
            confidence = 0.9;
        }
        // Data query
        else if query_lower.contains("温度")
            || query_lower.contains("湿度")
            || query_lower.contains("多少")
            || query_lower.contains("当前")
            || query_lower.contains("状态")
            || query_lower.contains("查询")
            || query_lower.contains("temperature")
            || query_lower.contains("humidity")
        {
            intent = IntentCategory::QueryData;
            confidence = if !results.is_empty() && results[0].score > 0.7 {
                0.9
            } else {
                0.5 // Low confidence when no specific device found - triggers clarification
            };
        }
        // Control
        else if query_lower.contains("打开")
            || query_lower.contains("关闭")
            || query_lower.contains("关掉")
            || query_lower.contains("打开")
            || query_lower.contains("控制")
            || query_lower.contains("调节")
            || query_lower.contains("开灯")
            || query_lower.contains("关灯")
            || query_lower.contains("open")
            || query_lower.contains("close")
            || query_lower.contains("turn on")
            || query_lower.contains("turn off")
        {
            intent = IntentCategory::ControlDevice;
            confidence = if !results.is_empty() && results[0].score > 0.7 {
                0.9
            } else {
                0.5 // Low confidence when no specific device found - triggers clarification
            };
        }
        // System status
        else if query_lower.contains("系统状态")
            || query_lower.contains("运行情况")
            || query_lower.contains("在线")
            || query_lower.contains("离线")
        {
            intent = IntentCategory::SystemStatus;
            confidence = 0.95;
        }
        // Alert related
        else if query_lower.contains("告警")
            || query_lower.contains("通知")
            || query_lower.contains("alert")
            || query_lower.contains("notification")
        {
            intent = IntentCategory::Alert;
            confidence = 0.8;
        }

        // Adjust confidence based on search results
        if !results.is_empty() {
            let best_score = results[0].score;
            confidence = confidence.max(best_score);
            highest_specific = best_score;
        }

        // If we have high-confidence resource matches, boost confidence
        if highest_specific > 0.8 {
            confidence = 0.95;
        }

        (intent, confidence)
    }

    /// Extract resource matches from search results.
    fn extract_matches(&self, results: &[SearchResult]) -> Vec<ResourceMatch> {
        results
            .iter()
            .filter(|r| r.score > 0.3) // Only include relevant matches
            .map(|r| {
                let match_type = if r.score > 0.8 {
                    MatchType::Direct
                } else if r.matched_fields.iter().any(|f| f == "alias") {
                    MatchType::Alias
                } else if r.matched_fields.iter().any(|f| f == "capability") {
                    MatchType::Capability
                } else if r.matched_fields.iter().any(|f| f == "location") {
                    MatchType::Location
                } else {
                    MatchType::Partial
                };

                let capabilities = r
                    .resource
                    .as_device()
                    .map(|d| d.capabilities.iter().map(|c| c.name.clone()).collect())
                    .unwrap_or_default();

                ResourceMatch {
                    resource_id: r.resource.id.to_string(),
                    name: r.resource.name.clone(),
                    match_type,
                    relevance: r.score,
                    capabilities,
                }
            })
            .collect()
    }

    /// Generate suggested actions based on intent and resources.
    fn generate_actions(
        &self,
        query: &str,
        intent: &IntentCategory,
        resources: &[ResourceMatch],
        _search_results: &[SearchResult],
    ) -> Vec<SuggestedAction> {
        let mut actions = Vec::new();

        match intent {
            IntentCategory::ListDevices => {
                actions.push(SuggestedAction {
                    action_type: ActionType::ListDevices,
                    target: None,
                    parameters: serde_json::json!({}),
                    description: "列出系统中的所有设备".to_string(),
                });
            }

            IntentCategory::QueryData => {
                if let Some(best) = resources.first() {
                    if best.relevance > 0.7 {
                        actions.push(SuggestedAction {
                            action_type: ActionType::QueryData,
                            target: Some(best.resource_id.clone()),
                            parameters: serde_json::json!({
                                "device": best.name.clone(),
                                "metric": self.infer_metric(query)
                            }),
                            description: format!("查询{}的数据", best.name),
                        });
                    }
                } else {
                    // No specific device found, need to search first
                    actions.push(SuggestedAction {
                        action_type: ActionType::SearchResources,
                        target: None,
                        parameters: serde_json::json!({"query": query}),
                        description: "搜索相关设备".to_string(),
                    });
                }
            }

            IntentCategory::ControlDevice => {
                if let Some(best) = resources.first() {
                    if best.relevance > 0.7 {
                        let action = self.infer_control_action(query);
                        actions.push(SuggestedAction {
                            action_type: ActionType::ControlDevice,
                            target: Some(best.resource_id.clone()),
                            parameters: serde_json::json!({
                                "device": best.name.clone(),
                                "action": action
                            }),
                            description: format!("{}{}", action, best.name),
                        });
                    }
                } else {
                    // No specific device found
                    actions.push(SuggestedAction {
                        action_type: ActionType::Clarify,
                        target: None,
                        parameters: serde_json::json!(null),
                        description: "请指定要控制哪个设备".to_string(),
                    });
                }
            }

            IntentCategory::SystemStatus => {
                actions.push(SuggestedAction {
                    action_type: ActionType::Respond,
                    target: None,
                    parameters: serde_json::json!(null),
                    description: "获取系统状态".to_string(),
                });
            }

            IntentCategory::General => {
                if resources.is_empty() {
                    actions.push(SuggestedAction {
                        action_type: ActionType::SearchResources,
                        target: None,
                        parameters: serde_json::json!({"query": query}),
                        description: "搜索相关资源".to_string(),
                    });
                }
            }

            _ => {}
        }

        actions
    }

    /// Infer which metric to query from user input.
    fn infer_metric(&self, query: &str) -> String {
        let query_lower = query.to_lowercase();

        if query_lower.contains("温度") || query_lower.contains("temperature") {
            return "temperature".to_string();
        }
        if query_lower.contains("湿度") || query_lower.contains("humidity") {
            return "humidity".to_string();
        }
        if query_lower.contains("亮度") || query_lower.contains("brightness") {
            return "brightness".to_string();
        }
        if query_lower.contains("压力") || query_lower.contains("pressure") {
            return "pressure".to_string();
        }

        // Default
        "temperature".to_string()
    }

    /// Infer control action from user input.
    fn infer_control_action(&self, query: &str) -> String {
        let query_lower = query.to_lowercase();

        if query_lower.contains("打开")
            || query_lower.contains("开灯")
            || query_lower.contains("open")
        {
            return "on".to_string();
        }
        if query_lower.contains("关闭")
            || query_lower.contains("关灯")
            || query_lower.contains("close")
        {
            return "off".to_string();
        }
        if query_lower.contains("切换") || query_lower.contains("toggle") {
            return "toggle".to_string();
        }

        // Default to toggle for vague control
        "toggle".to_string()
    }

    /// Suggest clarification for ambiguous queries.
    fn suggest_clarification(&self, query: &str, intent: &IntentCategory) -> Option<String> {
        match intent {
            IntentCategory::QueryData => {
                if query.contains("温度") {
                    return Some("您想查询哪个设备的温度？".to_string());
                }
                if query.contains("湿度") {
                    return Some("您想查询哪个设备的湿度？".to_string());
                }
                Some("您想查询哪个设备的数据？".to_string())
            }
            IntentCategory::ControlDevice => {
                if query.contains("灯") {
                    return Some("您想控制哪个位置的灯？".to_string());
                }
                Some("您想控制哪个设备？".to_string())
            }
            _ => None,
        }
    }

    /// Get tool suggestions for the resolved intent.
    pub fn get_tool_suggestions(&self, resolved: &ResolvedIntent) -> Vec<String> {
        let mut suggestions = Vec::new();

        match resolved.intent {
            IntentCategory::ListDevices => {
                suggestions.push("list_devices()".to_string());
            }
            IntentCategory::QueryData => {
                if let Some(action) = resolved.actions.first()
                    && let Some(target) = &action.target
                {
                    let metric = action
                        .parameters
                        .get("metric")
                        .and_then(|v| v.as_str())
                        .unwrap_or("temperature");
                    suggestions.push(format!(
                        "query_data(device='{}', metric='{}')",
                        target, metric
                    ));
                }
                suggestions.push("search_resources(query='相关设备类型')".to_string());
            }
            IntentCategory::ControlDevice => {
                if let Some(action) = resolved.actions.first()
                    && let Some(target) = &action.target
                {
                    let action_type = action
                        .parameters
                        .get("action")
                        .and_then(|v| v.as_str())
                        .unwrap_or("toggle");
                    suggestions.push(format!(
                        "control_device(device='{}', action='{}')",
                        target, action_type
                    ));
                }
            }
            _ => {
                suggestions.push("search_resources(query='查询内容')".to_string());
            }
        }

        suggestions
    }

    /// Format resolved intent for LLM context.
    pub fn format_for_prompt(&self, resolved: &ResolvedIntent) -> String {
        let mut text = String::new();

        text.push_str(&format!("**意图**: {:?}\n", resolved.intent));
        text.push_str(&format!(
            "**置信度**: {:.0}%\n",
            resolved.confidence * 100.0
        ));

        if !resolved.resources.is_empty() {
            text.push_str("\n**匹配的资源**:\n");
            for resource in &resolved.resources {
                text.push_str(&format!(
                    "- {} ({}, 相关度: {:.0}%, 能力: {:?})\n",
                    resource.name,
                    format!("{:?}", resource.match_type),
                    resource.relevance * 100.0,
                    resource.capabilities
                ));
            }
        }

        if !resolved.actions.is_empty() {
            text.push_str("\n**建议操作**:\n");
            for action in &resolved.actions {
                text.push_str(&format!("- {}\n", action.description));
            }
        }

        if let Some(clarification) = &resolved.clarification {
            text.push_str(&format!("\n**需要澄清**: {}\n", clarification));
        }

        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{AccessType, Capability, CapabilityType, Resource};

    #[tokio::test]
    async fn test_resolve_specific_device() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let device = Resource::device("sensor_1", "客厅温度传感器", "dht22")
            .with_location("客厅")
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("°C".to_string()),
                access: AccessType::Read,
            });

        index.write().await.register(device).await.unwrap();

        let resolver = ResourceResolver::new(index);

        let resolved = resolver.resolve("客厅温度").await;
        assert_eq!(resolved.intent, IntentCategory::QueryData);
        assert!(!resolved.resources.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_ambiguous_query() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let resolver = ResourceResolver::new(index);

        let resolved = resolver.resolve("温度是多少").await;
        // Should have clarification for ambiguous query
        assert!(resolved.clarification.is_some() || resolved.actions.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_control_intent() {
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

        let resolver = ResourceResolver::new(index);

        let resolved = resolver.resolve("打开客厅灯").await;
        assert_eq!(resolved.intent, IntentCategory::ControlDevice);
        assert!(!resolved.actions.is_empty());
    }
}
