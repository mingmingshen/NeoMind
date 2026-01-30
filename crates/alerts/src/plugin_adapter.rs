//! Alert Channel adapter and factory.
//!
//! This module provides type definitions and factory methods for alert channels,
//! allowing them to be created and managed through the alert system.

use edge_ai_core::alerts::{Alert, AlertChannel, ChannelFactory};
use serde_json::Value;
use std::sync::Arc;

use super::channels::{ConsoleChannelFactory, MemoryChannelFactory};
#[cfg(feature = "email")]
use super::channels::EmailChannelFactory;
#[cfg(feature = "webhook")]
use super::channels::WebhookChannelFactory;
use super::channel_schema;

/// Errors that can occur in alert channel operations.
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Channel not found: {0}")]
    NotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

/// Result type for channel operations.
pub type Result<T> = std::result::Result<T, ChannelError>;

/// Alert channel type definition.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AlertChannelTypeDefinition {
    /// Unique identifier for the channel type
    pub id: String,

    /// Display name
    pub name: String,

    /// Display name in Chinese
    pub name_zh: String,

    /// Description
    pub description: String,

    /// Description in Chinese
    pub description_zh: String,

    /// Icon name (for frontend)
    pub icon: String,

    /// Color theme (for frontend)
    pub color: String,

    /// Configuration schema
    pub config_schema: Value,

    /// Whether this channel type is always available
    pub always_available: bool,
}

/// Built-in alert channel type definitions.
/// Returns all channel types including debug/internal ones.
pub fn get_builtin_channel_types() -> Vec<AlertChannelTypeDefinition> {
    vec![
        AlertChannelTypeDefinition {
            id: "console".to_string(),
            name: "Console".to_string(),
            name_zh: "控制台".to_string(),
            description: "Print alerts to console output".to_string(),
            description_zh: "将告警打印到控制台输出".to_string(),
            icon: "Terminal".to_string(),
            color: "text-blue-500".to_string(),
            config_schema: channel_schema::get_channel_schema("console").unwrap_or_else(|| serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Channel name",
                        "description_zh": "通道名称"
                    },
                    "include_details": {
                        "type": "boolean",
                        "description": "Include detailed information",
                        "description_zh": "包含详细信息",
                        "default": true
                    }
                }
            })),
            always_available: true,
        },
        AlertChannelTypeDefinition {
            id: "memory".to_string(),
            name: "Memory".to_string(),
            name_zh: "内存".to_string(),
            description: "Store alerts in memory for testing".to_string(),
            description_zh: "将告警存储在内存中用于测试".to_string(),
            icon: "Database".to_string(),
            color: "text-purple-500".to_string(),
            config_schema: channel_schema::get_channel_schema("memory").unwrap_or_else(|| serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Channel name",
                        "description_zh": "通道名称"
                    }
                }
            })),
            always_available: true,
        },
        #[cfg(feature = "webhook")]
        AlertChannelTypeDefinition {
            id: "webhook".to_string(),
            name: "Webhook".to_string(),
            name_zh: "Webhook".to_string(),
            description: "Send alerts via HTTP POST webhook".to_string(),
            description_zh: "通过 HTTP POST Webhook 发送告警".to_string(),
            icon: "Webhook".to_string(),
            color: "text-green-500".to_string(),
            config_schema: channel_schema::get_channel_schema("webhook").unwrap_or_else(|| serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Channel name",
                        "description_zh": "通道名称"
                    },
                    "url": {
                        "type": "string",
                        "description": "Webhook URL",
                        "description_zh": "Webhook 地址"
                    },
                    "headers": {
                        "type": "object",
                        "description": "HTTP headers",
                        "description_zh": "HTTP 请求头"
                    }
                },
                "required": ["url"]
            })),
            always_available: true,
        },
        #[cfg(feature = "email")]
        AlertChannelTypeDefinition {
            id: "email".to_string(),
            name: "Email".to_string(),
            name_zh: "邮件".to_string(),
            description: "Send alerts via email SMTP".to_string(),
            description_zh: "通过 SMTP 邮件发送告警".to_string(),
            icon: "Mail".to_string(),
            color: "text-orange-500".to_string(),
            config_schema: channel_schema::get_channel_schema("email").unwrap_or_else(|| serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Channel name",
                        "description_zh": "通道名称"
                    },
                    "smtp_server": {
                        "type": "string",
                        "description": "SMTP server address",
                        "description_zh": "SMTP 服务器地址"
                    },
                    "smtp_port": {
                        "type": "integer",
                        "description": "SMTP port",
                        "description_zh": "SMTP 端口",
                        "default": 587
                    },
                    "username": {
                        "type": "string",
                        "description": "SMTP username",
                        "description_zh": "SMTP 用户名"
                    },
                    "password": {
                        "type": "string",
                        "description": "SMTP password",
                        "description_zh": "SMTP 密码",
                        "x_secret": true
                    },
                    "from_address": {
                        "type": "string",
                        "description": "From email address",
                        "description_zh": "发件人地址"
                    },
                    "recipients": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Recipient email addresses",
                        "description_zh": "收件人地址"
                    },
                    "use_tls": {
                        "type": "boolean",
                        "description": "Use TLS",
                        "description_zh": "使用 TLS",
                        "default": true
                    }
                },
                "required": ["smtp_server", "smtp_port", "username", "password", "from_address"]
            })),
            always_available: true,
        },
    ]
}

/// User-facing alert channel type definitions.
/// Returns only channels that should be exposed to end users (excludes debug/internal channels).
pub fn get_user_facing_channel_types() -> Vec<AlertChannelTypeDefinition> {
    let all_types = get_builtin_channel_types();
    all_types
        .into_iter()
        .filter(|t| t.id != "console" && t.id != "memory")
        .collect()
}

/// Test result for channel testing.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TestResult {
    pub success: bool,
    pub message: String,
    pub message_zh: String,
    pub duration_ms: u64,
}

/// Test result re-export for plugin compatibility.
pub type PluginTestResult = TestResult;

/// Alert Channel plugin factory for creating channels from type definitions.
pub struct AlertChannelPluginFactory;

impl AlertChannelPluginFactory {
    /// List all available channel types.
    pub fn available_types() -> Vec<AlertChannelTypeDefinition> {
        get_builtin_channel_types()
    }

    /// Get a channel type by ID.
    pub fn get_type(type_id: &str) -> Option<AlertChannelTypeDefinition> {
        Self::available_types()
            .into_iter()
            .find(|t| t.id == type_id)
    }

    /// Create a channel from a channel type ID and configuration.
    pub fn create_channel(
        type_id: &str,
        config: &Value,
    ) -> Result<Arc<dyn AlertChannel + Send + Sync>> {
        let factory = Self::get_factory(type_id)
            .ok_or_else(|| ChannelError::NotFound(format!("Channel type: {}", type_id)))?;

        let channel = factory
            .create(config)
            .map_err(|e| ChannelError::InitializationFailed(format!("Failed to create channel: {}", e)))?;

        Ok(channel)
    }

    /// Get the channel factory for a type ID.
    fn get_factory(type_id: &str) -> Option<Box<dyn ChannelFactory>> {
        match type_id {
            "console" => Some(Box::new(ConsoleChannelFactory)),
            "memory" => Some(Box::new(MemoryChannelFactory)),
            #[cfg(feature = "webhook")]
            "webhook" => Some(Box::new(WebhookChannelFactory)),
            #[cfg(feature = "email")]
            "email" => Some(Box::new(EmailChannelFactory)),
            _ => None,
        }
    }

    /// Test a channel by sending a test alert.
    pub async fn test_channel(
        type_id: &str,
        config: &Value,
    ) -> Result<TestResult> {
        let channel = Self::create_channel(type_id, config)?;

        // Create a test alert
        let test_alert = Alert::new(
            "test-alert",  // id
            edge_ai_core::alerts::AlertSeverity::Info,  // severity
            "Test Alert",  // title
            "This is a test notification to verify the channel is working correctly.",  // message
            "channel_test",  // source
        );

        let start = std::time::Instant::now();
        match channel.send(&test_alert).await {
            Ok(()) => Ok(TestResult {
                success: true,
                message: "Test alert sent successfully".to_string(),
                message_zh: "测试告警发送成功".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
            }),
            Err(e) => Ok(TestResult {
                success: false,
                message: format!("Failed to send test alert: {}", e),
                message_zh: format!("发送测试告警失败: {}", e),
                duration_ms: start.elapsed().as_millis() as u64,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_factory() {
        let types = AlertChannelPluginFactory::available_types();
        assert!(!types.is_empty(), "Should have at least one channel type");

        // Check console and memory are always available
        assert!(types.iter().any(|t| t.id == "console"));
        assert!(types.iter().any(|t| t.id == "memory"));
    }

    #[test]
    fn test_get_channel_type() {
        let console_type = AlertChannelPluginFactory::get_type("console");
        assert!(console_type.is_some());
        assert_eq!(console_type.unwrap().id, "console");

        let invalid_type = AlertChannelPluginFactory::get_type("invalid");
        assert!(invalid_type.is_none());
    }

    #[tokio::test]
    async fn test_create_channel() {
        let config = serde_json::json!({
            "name": "test_console"
        });
        let channel = AlertChannelPluginFactory::create_channel("console", &config);
        assert!(channel.is_ok());
    }
}
