//! System management tools for NeoMind platform.
//!
//! Provides tools for:
//! - System status and resource monitoring
//! - Help and onboarding information

use async_trait::async_trait;
use serde_json::Value;

use super::error::Result;
use super::tool::ToolExample;
use super::tool::{object_schema, string_property, Tool, ToolDefinition, ToolOutput};
use neomind_core::tools::{ToolCategory, ToolRelationships, UsageScenario};

// ============================================================================
// System Information Tools
// ============================================================================

/// Tool for getting system information and status.
pub struct SystemInfoTool {
    /// Optional system name for identification
    system_name: Option<String>,
}

impl SystemInfoTool {
    /// Create a new system info tool.
    pub fn new() -> Self {
        Self { system_name: None }
    }

    /// Create with a system name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            system_name: Some(name.into()),
        }
    }
}

impl Default for SystemInfoTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SystemInfoTool {
    fn name(&self) -> &str {
        "system_info"
    }

    fn description(&self) -> &str {
        "获取系统信息和状态，包括系统名称、运行时间、CPU和内存使用情况"
    }

    fn parameters(&self) -> Value {
        object_schema(serde_json::json!({}), vec![])
    }

    fn category(&self) -> neomind_core::tools::ToolCategory {
        ToolCategory::System
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: serde_json::json!({}),
                result: serde_json::json!({
                    "system_name": "NeoMind",
                    "uptime": "2 hours",
                    "cpu_usage": "15%",
                    "memory_usage": "256MB"
                }),
                description: "获取系统信息".to_string(),
            }),
            category: ToolCategory::System,
            scenarios: vec![UsageScenario {
                description: "查看系统状态".to_string(),
                example_query: "系统运行状态如何？".to_string(),
                suggested_call: Some(r#"{"tool": "system_info", "arguments": {}}"#.to_string()),
            }],
            relationships: ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![],
            response_format: Some("simple".to_string()),
            namespace: Some("system".to_string()),
        }
    }

    fn namespace(&self) -> Option<&str> {
        Some("system")
    }

    async fn execute(&self, _args: Value) -> Result<ToolOutput> {
        // Basic system info without external dependencies
        let result = serde_json::json!({
            "system_name": self.system_name.as_deref().unwrap_or("NeoMind"),
            "version": env!("CARGO_PKG_VERSION"),
            "status": "running",
            "message": "System is operational"
        });

        Ok(ToolOutput::success(result))
    }
}

// ============================================================================
// System Help / Onboarding Tool
// ============================================================================

/// Tool for providing system help and feature information to users.
///
/// This tool is designed to help new users understand what the system can do
/// and guide them through available features.
pub struct SystemHelpTool {
    /// Optional system name for identification
    system_name: Option<String>,
}

impl SystemHelpTool {
    /// Create a new system help tool.
    pub fn new() -> Self {
        Self { system_name: None }
    }

    /// Create with a system name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            system_name: Some(name.into()),
        }
    }
}

impl Default for SystemHelpTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SystemHelpTool {
    fn name(&self) -> &str {
        "system_help"
    }

    fn description(&self) -> &str {
        "获取系统帮助和功能介绍。支持主题：overview（概览）、devices（设备）、automation（自动化）、agents（智能体）、alerts（告警）、getting_started（入门）"
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "topic": string_property("帮助主题：overview（概览）、devices（设备）、automation（自动化）、agents（智能体）、alerts（告警）、getting_started（入门）、examples（示例）"),
                "detail": string_property("可选，要详细了解的具体功能")
            }),
            vec![],
        )
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: serde_json::json!({
                    "topic": "overview"
                }),
                result: serde_json::json!({
                    "topic": "overview",
                    "system_name": "NeoMind",
                    "features": ["设备管理", "自动化规则", "AI Agent", "告警系统"]
                }),
                description: "获取系统概览".to_string(),
            }),
            category: ToolCategory::System,
            scenarios: vec![UsageScenario {
                description: "新用户了解系统".to_string(),
                example_query: "这个系统能做什么？".to_string(),
                suggested_call: Some(
                    r#"{"tool": "system_help", "arguments": {"topic": "overview"}}"#.to_string(),
                ),
            }],
            relationships: ToolRelationships {
                call_after: vec![],
                output_to: vec![
                    "device".to_string(),
                    "agent".to_string(),
                    "rule".to_string(),
                ],
                exclusive_with: vec![],
            },
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![],
            response_format: Some("detailed".to_string()),
            namespace: Some("system".to_string()),
        }
    }

    fn namespace(&self) -> Option<&str> {
        Some("system")
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let topic = args["topic"].as_str().unwrap_or("overview");
        let detail = args["detail"].as_str();

        let system_name = self.system_name.as_deref().unwrap_or("NeoMind");

        let result = match topic {
            "overview" => self.get_overview(system_name),
            "devices" => self.get_devices_help(detail),
            "automation" | "rules" => self.get_automation_help(detail),
            "agents" => self.get_agents_help(detail),
            "alerts" => self.get_alerts_help(detail),
            "getting_started" => self.get_getting_started(system_name),
            "examples" => self.get_examples(),
            _ => self.get_overview(system_name),
        };

        Ok(ToolOutput::success(result))
    }
}

impl SystemHelpTool {
    fn get_overview(&self, system_name: &str) -> Value {
        serde_json::json!({
            "topic": "overview",
            "system_name": system_name,
            "title": format!("{} - 智能物联网边缘平台", system_name),
            "description": format!("{} 是一个部署在边缘的智能物联网平台，支持设备管理、自动化规则、AI Agent和告警系统。", system_name),
            "main_features": [
                {
                    "name": "设备管理",
                    "description": "连接和管理各类IoT设备",
                    "tool": "device",
                    "actions": ["list", "get", "query", "control"]
                },
                {
                    "name": "自动化规则",
                    "description": "创建自动化规则，实现设备智能联动",
                    "tool": "rule",
                    "actions": ["list", "get", "delete", "history"]
                },
                {
                    "name": "AI Agent",
                    "description": "创建自主运行的AI智能体",
                    "tool": "agent",
                    "actions": ["list", "get", "create", "update", "control", "memory"]
                },
                {
                    "name": "消息通知",
                    "description": "查看和发送消息通知",
                    "tool": "message",
                    "actions": ["list", "send", "read"]
                }
            ],
            "quick_start_questions": [
                "有哪些设备在线？",
                "如何创建自动化规则？",
                "有哪些Agent正在运行？",
                "最近有什么告警？"
            ]
        })
    }

    fn get_devices_help(&self, _detail: Option<&str>) -> Value {
        serde_json::json!({
            "topic": "devices",
            "title": "设备管理功能",
            "description": "使用 device 工具管理连接到系统的各类IoT设备",
            "tool": "device",
            "actions": {
                "list": "列出所有设备，支持按类型、状态过滤",
                "get": "获取单个设备详情",
                "query": "查询设备时序数据",
                "control": "控制设备（开关、设置参数等）"
            },
            "examples": [
                {"query": "查看所有设备", "tool_call": {"action": "list"}},
                {"query": "温度传感器数据", "tool_call": {"action": "query", "device_id": "sensor_1", "metric": "temperature"}},
                {"query": "打开客厅灯", "tool_call": {"action": "control", "device_id": "lamp_1", "command": "on"}}
            ]
        })
    }

    fn get_automation_help(&self, _detail: Option<&str>) -> Value {
        serde_json::json!({
            "topic": "automation",
            "title": "自动化规则功能",
            "description": "使用 rule 工具管理自动化规则",
            "tool": "rule",
            "actions": {
                "list": "列出所有自动化规则",
                "get": "获取规则详情",
                "delete": "删除规则",
                "history": "查看规则执行历史"
            },
            "examples": [
                {"query": "有哪些自动化规则？", "tool_call": {"action": "list"}},
                {"query": "查看规则详情", "tool_call": {"action": "get", "rule_id": "rule_1"}}
            ]
        })
    }

    fn get_agents_help(&self, _detail: Option<&str>) -> Value {
        serde_json::json!({
            "topic": "agents",
            "title": "AI Agent功能",
            "description": "使用 agent 工具管理AI智能体",
            "tool": "agent",
            "actions": {
                "list": "列出所有Agent",
                "get": "获取Agent详情",
                "create": "创建新Agent",
                "update": "更新Agent配置",
                "control": "启动/停止/暂停Agent",
                "memory": "管理Agent记忆"
            },
            "examples": [
                {"query": "有哪些Agent？", "tool_call": {"action": "list"}},
                {"query": "创建温度监控Agent", "tool_call": {"action": "create", "name": "温度监控", "schedule_type": "interval"}}
            ]
        })
    }

    fn get_alerts_help(&self, _detail: Option<&str>) -> Value {
        serde_json::json!({
            "topic": "messages",
            "title": "消息通知功能",
            "description": "使用 message 工具管理消息通知",
            "tool": "message",
            "actions": {
                "list": "列出消息",
                "send": "发送消息",
                "read": "标记已读"
            },
            "examples": [
                {"query": "最近的告警", "tool_call": {"action": "list"}},
                {"query": "确认告警", "tool_call": {"action": "acknowledge", "alert_id": "alert_1"}}
            ]
        })
    }

    fn get_getting_started(&self, system_name: &str) -> Value {
        serde_json::json!({
            "topic": "getting_started",
            "system_name": system_name,
            "title": format!("{} 快速入门", system_name),
            "steps": [
                {"step": 1, "action": "查看系统概览", "query": "这个系统能做什么？"},
                {"step": 2, "action": "查看已连接设备", "query": "有哪些设备？"},
                {"step": 3, "action": "尝试查询设备数据", "query": "温度传感器现在的温度是多少？"},
                {"step": 4, "action": "控制一个设备", "query": "打开客厅的灯"}
            ],
            "tips": [
                "使用自然语言提问，系统会自动理解你的意图",
                "可以随时问\"帮助\"获取更多信息",
                "系统支持中英文交互"
            ]
        })
    }

    fn get_examples(&self) -> Value {
        serde_json::json!({
            "topic": "examples",
            "title": "常用示例",
            "categories": [
                {
                    "name": "设备管理",
                    "examples": [
                        "查看所有设备",
                        "温度传感器现在的温度是多少",
                        "打开客厅灯",
                        "分析温度变化趋势"
                    ]
                },
                {
                    "name": "自动化规则",
                    "examples": [
                        "有哪些自动化规则",
                        "创建一个温度监控规则",
                        "禁用告警规则"
                    ]
                },
                {
                    "name": "AI Agent",
                    "examples": [
                        "列出所有Agent",
                        "创建一个定时监控Agent",
                        "查看Agent学到了什么"
                    ]
                }
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_info_tool() {
        let tool = SystemInfoTool::new();
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert!(result.success);
        assert!(result.data.is_object());
    }

    #[tokio::test]
    async fn test_system_help_tool_overview() {
        let tool = SystemHelpTool::new();
        let result = tool
            .execute(serde_json::json!({"topic": "overview"}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["topic"], "overview");
    }

    #[tokio::test]
    async fn test_system_help_tool_with_name() {
        let tool = SystemHelpTool::with_name("TestSystem");
        let result = tool
            .execute(serde_json::json!({"topic": "overview"}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.data["system_name"], "TestSystem");
    }
}
