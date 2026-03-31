//! Universal tools for AI Agent function calling mode.
//!
//! These tools provide the agent with capabilities to:
//! - Query metrics from any data source (devices, extensions)
//! - Execute commands on devices or extensions
//! - Send notifications to users

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing::{info, warn};

use super::error::{Result, ToolError};
use super::tool::{
    array_property, number_property, object_schema, string_property, Tool, ToolOutput,
};
use neomind_core::tools::ToolCategory;
use neomind_devices::{DeviceService, TimeSeriesStorage};
use neomind_messages::{MessageManager, MessageSeverity};

// ============================================================================
// QueryMetricsTool - Unified metric query
// ============================================================================

/// Tool for querying metrics from any data source.
///
/// Supports device metrics and extension metrics using DataSourceId format
/// (`{type}:{id}:{field}`, e.g., `device:sensor1:temperature`).
pub struct QueryMetricsTool {
    storage: Arc<TimeSeriesStorage>,
    device_service: Option<Arc<DeviceService>>,
}

impl QueryMetricsTool {
    pub fn new(storage: Arc<TimeSeriesStorage>) -> Self {
        Self {
            storage,
            device_service: None,
        }
    }

    pub fn with_device_service(mut self, service: Arc<DeviceService>) -> Self {
        self.device_service = Some(service);
        self
    }
}

#[async_trait]
impl Tool for QueryMetricsTool {
    fn name(&self) -> &str {
        "query_metrics"
    }

    fn description(&self) -> &str {
        "查询指标数据。支持批量查询多个数据源。数据源ID格式: {type}:{id}:{field}，例如 device:sensor1:temperature、extension:weather:temp。不指定时间范围时返回最近24小时数据。"
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "source_ids": array_property("string", "数据源ID列表，格式: {type}:{id}:{field}。例如: [\"device:sensor1:temperature\", \"extension:weather:temp\"]"),
                "start_time": number_property("起始时间戳（Unix秒）。可选，默认24小时前"),
                "end_time": number_property("结束时间戳（Unix秒）。可选，默认当前时间"),
                "limit": number_property("返回数据点数量限制。可选，默认100")
            }),
            vec!["source_ids".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Data
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        self.validate_args(&args)?;

        let source_ids = args["source_ids"]
            .as_array()
            .ok_or_else(|| ToolError::InvalidArguments("source_ids must be an array".into()))?;

        let now = chrono::Utc::now().timestamp();
        let start_time = args["start_time"].as_i64().unwrap_or(now - 86400);
        let end_time = args["end_time"].as_i64().unwrap_or(now);
        let limit = args["limit"].as_u64().unwrap_or(100) as usize;

        let mut results = serde_json::Map::new();

        for source_id_val in source_ids {
            let source_id_str = source_id_val
                .as_str()
                .ok_or_else(|| ToolError::InvalidArguments("source_id must be a string".into()))?;

            // Parse DataSourceId: {type}:{id}:{field}
            let parts: Vec<&str> = source_id_str.splitn(3, ':').collect();
            if parts.len() != 3 {
                results.insert(
                    source_id_str.to_string(),
                    serde_json::json!({
                        "error": format!("Invalid source_id format: '{}'. Expected {{type}}:{{id}}:{{field}}", source_id_str)
                    }),
                );
                continue;
            }

            let source_type = parts[0];
            let source_id = parts[1];
            let field = parts[2];

            match source_type {
                "device" => {
                    match self
                        .storage
                        .query(source_id, field, start_time, end_time)
                        .await
                    {
                        Ok(data_points) => {
                            let count = data_points.len();
                            let limited: Vec<_> = data_points
                                .into_iter()
                                .rev() // newest first
                                .take(limit)
                                .map(|dp| {
                                    serde_json::json!({
                                        "timestamp": dp.timestamp,
                                        "value": dp.value,
                                    })
                                })
                                .collect();
                            results.insert(
                                source_id_str.to_string(),
                                serde_json::json!({
                                    "data_points": limited,
                                    "total_count": count,
                                }),
                            );
                        }
                        Err(e) => {
                            results.insert(
                                source_id_str.to_string(),
                                serde_json::json!({
                                    "error": format!("Query failed: {}", e)
                                }),
                            );
                        }
                    }
                }
                "extension" => {
                    // Extension metrics are also stored in TimeSeriesStorage
                    // with the extension ID as the "device_id"
                    match self
                        .storage
                        .query(source_id, field, start_time, end_time)
                        .await
                    {
                        Ok(data_points) => {
                            let count = data_points.len();
                            let limited: Vec<_> = data_points
                                .into_iter()
                                .rev()
                                .take(limit)
                                .map(|dp| {
                                    serde_json::json!({
                                        "timestamp": dp.timestamp,
                                        "value": dp.value,
                                    })
                                })
                                .collect();
                            results.insert(
                                source_id_str.to_string(),
                                serde_json::json!({
                                    "data_points": limited,
                                    "total_count": count,
                                }),
                            );
                        }
                        Err(e) => {
                            results.insert(
                                source_id_str.to_string(),
                                serde_json::json!({
                                    "error": format!("Query failed: {}", e)
                                }),
                            );
                        }
                    }
                }
                other => {
                    results.insert(
                        source_id_str.to_string(),
                        serde_json::json!({
                            "error": format!("Unsupported source type: '{}'. Use 'device' or 'extension'", other)
                        }),
                    );
                }
            }
        }

        info!(
            "query_metrics: queried {} source(s), time range [{}, {}]",
            source_ids.len(),
            start_time,
            end_time
        );

        Ok(ToolOutput::success(serde_json::Value::Object(results)))
    }
}

// ============================================================================
// ExecuteCommandTool - Unified command execution
// ============================================================================

/// Tool for executing commands on devices or extensions.
///
/// Routes commands based on prefix:
/// - `device:{device_id}:{command}` -> DeviceService::send_command()
/// - `extension:{ext_id}:{command}` -> ExtensionRegistry::execute_command()
pub struct ExecuteCommandTool {
    device_service: Option<Arc<DeviceService>>,
    extension_registry: Option<Arc<neomind_core::extension::registry::ExtensionRegistry>>,
}

impl ExecuteCommandTool {
    pub fn new() -> Self {
        Self {
            device_service: None,
            extension_registry: None,
        }
    }

    pub fn with_device_service(mut self, service: Arc<DeviceService>) -> Self {
        self.device_service = Some(service);
        self
    }

    pub fn with_extension_registry(
        mut self,
        registry: Arc<neomind_core::extension::registry::ExtensionRegistry>,
    ) -> Self {
        self.extension_registry = Some(registry);
        self
    }
}

impl Default for ExecuteCommandTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExecuteCommandTool {
    fn name(&self) -> &str {
        "execute_command"
    }

    fn description(&self) -> &str {
        "执行命令。支持设备控制和扩展命令。命令ID格式: {type}:{id}:{command}，例如 device:sensor1:set_threshold、extension:camera:capture。参数通过params字段传递。"
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "command_id": string_property("命令ID，格式: {type}:{id}:{command}。例如: \"device:ac01:set_temperature\", \"extension:camera:capture\""),
                "params": object_schema(
                    serde_json::json!({}),
                    vec![],
                )
            }),
            vec!["command_id".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Device
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        self.validate_args(&args)?;

        let command_id = args["command_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("command_id must be a string".into()))?;

        let params = args.get("params").cloned().unwrap_or(serde_json::json!({}));
        let params_map: std::collections::HashMap<String, Value> =
            serde_json::from_value(params.clone()).unwrap_or_default();

        // Parse command_id: {type}:{id}:{command}
        let parts: Vec<&str> = command_id.splitn(3, ':').collect();
        if parts.len() != 3 {
            return Err(ToolError::InvalidArguments(format!(
                "Invalid command_id format: '{}'. Expected {{type}}:{{id}}:{{command}}",
                command_id
            )));
        }

        let source_type = parts[0];
        let source_id = parts[1];
        let command = parts[2];

        match source_type {
            "device" => {
                let service = self
                    .device_service
                    .as_ref()
                    .ok_or_else(|| ToolError::Execution("Device service not available".into()))?;

                match service.send_command(source_id, command, params_map).await {
                    Ok(result) => {
                        info!(
                            "execute_command: device {} command {} succeeded",
                            source_id, command
                        );
                        Ok(ToolOutput::success(serde_json::json!({
                            "command_id": command_id,
                            "status": "executed",
                            "result": result.map(|v| serde_json::to_value(v).unwrap_or_default()),
                        })))
                    }
                    Err(e) => {
                        warn!(
                            "execute_command: device {} command {} failed: {}",
                            source_id, command, e
                        );
                        Ok(ToolOutput::error_with_data(
                            format!("Command execution failed: {}", e),
                            serde_json::json!({
                                "command_id": command_id,
                                "status": "failed",
                            }),
                        ))
                    }
                }
            }
            "extension" => {
                let registry = self.extension_registry.as_ref().ok_or_else(|| {
                    ToolError::Execution("Extension registry not available".into())
                })?;

                match registry.execute_command(source_id, command, &params).await {
                    Ok(result) => {
                        info!(
                            "execute_command: extension {} command {} succeeded",
                            source_id, command
                        );
                        Ok(ToolOutput::success(serde_json::json!({
                            "command_id": command_id,
                            "status": "executed",
                            "result": result,
                        })))
                    }
                    Err(e) => {
                        warn!(
                            "execute_command: extension {} command {} failed: {}",
                            source_id, command, e
                        );
                        Ok(ToolOutput::error_with_data(
                            format!("Extension command failed: {}", e),
                            serde_json::json!({
                                "command_id": command_id,
                                "status": "failed",
                            }),
                        ))
                    }
                }
            }
            other => Err(ToolError::InvalidArguments(format!(
                "Unsupported command type: '{}'. Use 'device' or 'extension'",
                other
            ))),
        }
    }
}

// ============================================================================
// SendNotificationTool - Unified notification
// ============================================================================

/// Tool for sending notifications to users.
pub struct SendNotificationTool {
    message_manager: Arc<MessageManager>,
}

impl SendNotificationTool {
    pub fn new(message_manager: Arc<MessageManager>) -> Self {
        Self { message_manager }
    }
}

fn parse_severity(s: &str) -> MessageSeverity {
    match s.to_lowercase().as_str() {
        "warning" | "warn" => MessageSeverity::Warning,
        "critical" | "error" => MessageSeverity::Critical,
        "emergency" => MessageSeverity::Emergency,
        _ => MessageSeverity::Info,
    }
}

#[async_trait]
impl Tool for SendNotificationTool {
    fn name(&self) -> &str {
        "send_notification"
    }

    fn description(&self) -> &str {
        "发送通知消息。支持不同严重级别: info(信息)、warning(警告)、critical(严重)、emergency(紧急)。用于向用户报告重要发现或异常情况。"
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "title": string_property("通知标题，简短描述通知内容"),
                "content": string_property("通知内容，详细描述发现的情况或需要关注的事项"),
                "severity": string_property("严重级别: info, warning, critical, emergency。默认info")
            }),
            vec!["title".to_string(), "content".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        self.validate_args(&args)?;

        let title = args["title"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("title must be a string".into()))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("content must be a string".into()))?;
        let severity_str = args["severity"].as_str().unwrap_or("info");
        let severity = parse_severity(severity_str);

        match self
            .message_manager
            .alert(
                severity,
                title.to_string(),
                content.to_string(),
                "ai_agent".to_string(),
            )
            .await
        {
            Ok(msg) => {
                info!("send_notification: sent '{}' ({})", title, severity_str);
                Ok(ToolOutput::success(serde_json::json!({
                    "notification_id": msg.id.to_string(),
                    "status": "sent",
                    "severity": severity_str,
                })))
            }
            Err(e) => {
                warn!("send_notification: failed to send '{}': {}", title, e);
                Err(ToolError::Execution(format!(
                    "Failed to send notification: {}",
                    e
                )))
            }
        }
    }
}
