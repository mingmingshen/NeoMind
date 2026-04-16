//! Aggregated tools using action-based design pattern.
//!
//! This module consolidates 34+ individual tools into 5 aggregated tools,
//! reducing token usage in tool definitions by ~60%.
//!
//! ## Design Principles
//!
//! - **Action-based routing**: Single tool with `action` parameter to differentiate operations
//! - **Token efficiency**: Smaller schema, shared descriptions
//! - **Backward compatibility**: Output format unchanged from original tools
//!
//! ## Tools
//!
//! 1. `device` - Device operations (list, get, query, control)
//! 2. `agent` - Agent management (list, get, create, update, control, memory, executions, conversation, latest_execution)
//! 3. `rule` - Rule management (list, get, delete, history)
//! 4. `message` - Message and alert management (list, send, read/acknowledge)
//! 5. `extension` - Extension management (list, get, status)

use std::collections::HashMap;

/// Check if a metric should be skipped entirely in LLM tool output
/// because it contains raw/large payloads (e.g. base64 images, full MQTT messages).
fn is_raw_payload_metric(name: &str) -> bool {
    name == "_raw" || name.ends_with("_raw")
}
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{Result, ToolError};
use super::tool::{object_schema, Tool, ToolOutput};
use neomind_core::tools::ToolCategory;
use neomind_storage::agents::{AgentMemory, AgentStats};

// ============================================================================
// Device Tool - Aggregates: list_devices, get_device, query_data, control_device
// ============================================================================

/// Aggregated device tool with action-based routing.
pub struct DeviceTool {
    device_service: Arc<neomind_devices::DeviceService>,
    storage: Option<Arc<neomind_devices::TimeSeriesStorage>>,
}

impl DeviceTool {
    /// Create a new device tool.
    pub fn new(device_service: Arc<neomind_devices::DeviceService>) -> Self {
        Self {
            device_service,
            storage: None,
        }
    }

    /// Create with time series storage.
    pub fn with_storage(
        device_service: Arc<neomind_devices::DeviceService>,
        storage: Arc<neomind_devices::TimeSeriesStorage>,
    ) -> Self {
        Self {
            device_service,
            storage: Some(storage),
        }
    }
}

#[async_trait]
impl Tool for DeviceTool {
    fn name(&self) -> &str {
        "device"
    }

    fn description(&self) -> &str {
        r#"Device management tool for querying and controlling IoT devices.

Actions:
- list: List all devices with their available metrics and commands.
- latest: Device overview with ALL current (latest) metric values (name, value, unit, timestamp).
  Returns every metric's latest reading in one call. Use when user asks "latest data", "current status", "how is device now".
- history: Historical time-series data for a specific metric over a time range.
  Requires device_id and metric. Returns time-ordered data points. Defaults to last 24 hours.
  Use when user asks about trends, changes over time, or past data with a time range.
- control: Send control commands to devices. Requires confirm=true to execute.
- write_metric: Write a value to a device metric (virtual or existing). Use for calibration values, status flags,
  computed results, or any data the AI wants to persist on a device. Requires device_id, metric, value.
  Cannot overwrite physical device template metrics — use control action for that.

IMPORTANT - Batch Tool Calls:
When querying history for MULTIPLE devices, you MUST output ALL history calls in ONE
JSON array in a single response. Do NOT call one device at a time.

Important:
- latest = current snapshot (all metrics, latest values). Use for "how is device X?", "latest data", "current status" queries.
- history = historical trend (one metric, time range). Use for "show battery trend", "temperature over time" queries.
- Always confirm user intent before using control action
- Supports fuzzy matching on device names (partial name works)"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "latest", "history", "control", "write_metric"],
                    "description": "Operation type: 'list' (list devices), 'latest' (all current metric values), 'history' (historical time-series for one metric), 'control' (send command), 'write_metric' (write a value to a device metric)"
                },
                "device_id": {
                    "type": "string",
                    "description": "Device ID or name. Required for get/history/control. Supports fuzzy matching. Use the ID returned by list action."
                },
                "metric": {
                    "type": "string",
                    "description": "Metric name (history action). Use the metric names from list action output. Supports fuzzy matching (e.g., 'battery' matches 'values.battery')."
                },
                "command": {
                    "type": "string",
                    "description": "Control command to send (control action). Common: 'turn_on', 'turn_off', 'set_value', 'toggle'. Examples: 'turn_on', 'set_value'"
                },
                "params": {
                    "type": "object",
                    "description": "Control parameters as key-value pairs (control action, optional). Example: {\"value\": 26, \"unit\": \"celsius\"}"
                },
                "device_type": {
                    "type": "string",
                    "description": "Filter by device type (list action). Examples: 'sensor', 'switch', 'light', 'camera'"
                },
                "include_details": {
                    "type": "boolean",
                    "description": "Include metrics and commands info in list output (list action, default: true)"
                },
                "start_time": {
                    "type": "number",
                    "description": "Start timestamp in seconds for history time range (history action, default: 24 hours ago). Example: 1712000000"
                },
                "end_time": {
                    "type": "number",
                    "description": "End timestamp in seconds for history time range (history action, default: now)"
                },
                "limit": {
                    "type": "number",
                    "description": "Max number of data points to return (default: 10 for history, unlimited for list)"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': key info only (default). 'detailed': full data with IDs and metadata for follow-up chained calls"
                },
                "value": {
                    "type": ["string", "number", "boolean", "null"],
                    "description": "Value to write to the metric (write action). Can be number, string, boolean, or null."
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Set to true after user confirms. Required for control action. Without confirmation, the tool returns a preview instead of executing"
                }
            }),
            vec!["action".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Device
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "list" => self.execute_list(&args).await,
            "latest" | "get" => self.execute_get(&args).await,
            "history" | "query" => self.execute_query(&args).await,
            "control" => self.execute_control(&args).await,
            "write_metric" => self.execute_write(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: '{}'. Valid actions: list, latest, history, control, write_metric",
                action
            ))),
        }
    }
}

impl DeviceTool {
    /// Check if response_format is "detailed".
    fn is_detailed(args: &Value) -> bool {
        args["response_format"].as_str() == Some("detailed")
    }

    /// Resolve device_id with fuzzy matching support using the generic EntityResolver.
    async fn resolve_device_id(&self, device_id: &str) -> Result<String> {
        // Fast path: exact ID match without listing all devices
        if self.device_service.get_device(device_id).await.is_some() {
            return Ok(device_id.to_string());
        }

        // Slow path: fuzzy match via resolver
        let devices = self.device_service.list_devices().await;
        let candidates: Vec<(String, String)> = devices
            .iter()
            .map(|d| (d.device_id.clone(), d.name.clone()))
            .collect();

        super::resolver::EntityResolver::resolve(device_id, &candidates, "设备")
            .map_err(ToolError::Execution)
    }

    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let devices = self.device_service.list_devices().await;
        let device_type_filter = args["device_type"].as_str();
        let detailed = Self::is_detailed(args);

        let mut result = Vec::new();

        for d in devices.iter() {
            // Apply device_type filter
            if let Some(dt) = device_type_filter {
                if d.device_type != dt {
                    continue;
                }
            }

            // Concise mode: name/type only
            if !detailed {
                result.push(serde_json::json!({
                    "id": d.device_id,
                    "name": d.name,
                    "type": d.device_type,
                }));
                continue;
            }

            // Detailed mode: device info with inline metric/command names
            // so the LLM directly knows what it can query/control per device
            let mut device_json = serde_json::json!({
                "id": d.device_id,
                "name": d.name,
                "type": d.device_type,
            });

            if let Some(template) = self.device_service.get_template(&d.device_type).await {
                if !template.metrics.is_empty() {
                    let metric_names: Vec<&str> = template.metrics.iter().map(|m| m.name.as_str()).collect();
                    device_json["metrics"] = serde_json::json!(metric_names);
                }
                if !template.commands.is_empty() {
                    let commands_info: Vec<Value> = template
                        .commands
                        .iter()
                        .map(|c| {
                            if c.parameters.is_empty() {
                                serde_json::json!({"name": c.name})
                            } else {
                                let params: Vec<Value> = c.parameters.iter().map(|p| {
                                    let type_str = format!("{:?}", p.data_type).to_lowercase();
                                    if p.required {
                                        serde_json::json!(format!("{}:{} (required)", p.name, type_str))
                                    } else {
                                        serde_json::json!(format!("{}:{}", p.name, type_str))
                                    }
                                }).collect();
                                serde_json::json!({
                                    "name": c.name,
                                    "params": params
                                })
                            }
                        })
                        .collect();
                    device_json["commands"] = serde_json::json!(commands_info);
                }
            }

            // Discover virtual metrics from time-series storage
            if let Some(storage) = &self.storage {
                if let Ok(all_stored_metrics) = storage.list_metrics(&d.device_id).await {
                    let template_metric_set: std::collections::HashSet<&str> = device_json
                        .get("metrics")
                        .and_then(|m| m.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();

                    let virtual_metrics: Vec<&str> = all_stored_metrics
                        .iter()
                        .filter(|m| !template_metric_set.contains(m.as_str()))
                        .map(|m| m.as_str())
                        .collect();

                    if !virtual_metrics.is_empty() {
                        device_json["virtual_metrics"] = serde_json::json!(virtual_metrics);
                    }
                }
            }

            result.push(device_json);
        }

        let limit = args["limit"].as_u64().unwrap_or(result.len() as u64) as usize;
        let result: Vec<_> = result.into_iter().take(limit).collect();

        let output = serde_json::json!({
            "count": result.len(),
            "devices": result
        });

        Ok(ToolOutput::success(output))
    }

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let device_id_input = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id is required".into()))?;

        let device_id = self.resolve_device_id(device_id_input).await?;

        let device = self
            .device_service
            .get_device(&device_id)
            .await
            .ok_or_else(|| ToolError::Execution(format!("Device not found: {}", device_id)))?;

        let detailed = Self::is_detailed(args);

        let mut device_json = if detailed {
            serde_json::json!({
                "id": device.device_id,
                "name": device.name,
                "type": device.device_type,
                "adapter_type": device.adapter_type,
                "adapter_id": device.adapter_id
            })
        } else {
            serde_json::json!({
                "id": device.device_id,
                "name": device.name,
                "type": device.device_type
            })
        };

        // Enrich with metrics and commands from device type template
        if let Some(template) = self.device_service.get_template(&device.device_type).await {
            if !template.metrics.is_empty() {
                let storage = self.storage.as_ref();
                let mut metrics_info: Vec<Value> = Vec::new();
                for m in &template.metrics {
                    // Concise mode: skip infrastructure/noise metrics (metadata.*, ai_result.*, encoding)
                    // These are available via device(action="query", metric=<name>) when needed.
                    if !detailed {
                        let name = m.name.as_str();
                        if name.starts_with("metadata.")
                            || name.starts_with("ai_result.")
                            || name == "encoding"
                            || name == "image_data"
                        {
                            continue;
                        }
                    }

                    let mut metric_json = serde_json::json!({
                        "name": m.name,
                        "display_name": m.display_name,
                    });
                    if detailed {
                        metric_json["unit"] = serde_json::json!(m.unit);
                        metric_json["data_type"] = serde_json::json!(format!("{:?}", m.data_type));
                    }
                    // Fetch latest value for this metric
                    if let Some(store) = storage {
                        if let Ok(Some(latest)) = store.latest(&device_id, &m.name).await {
                            // Binary metrics (images) are too large to include inline.
                            // Return metadata only; use device(action="query", metric=<name>)
                            // to fetch the actual binary data when needed.
                            if m.data_type == neomind_devices::mdl::MetricDataType::Binary {
                                if let Some(s) = latest.value.as_str() {
                                    let size_bytes = s.len();
                                    metric_json["value"] = serde_json::json!(format!(
                                        "[binary data, {}]",
                                        if size_bytes > 1024 * 1024 {
                                            format!("{:.1}MB", size_bytes as f64 / (1024.0 * 1024.0))
                                        } else {
                                            format!("{:.1}KB", size_bytes as f64 / 1024.0)
                                        }
                                    ));
                                }
                            } else if is_raw_payload_metric(&m.name) {
                                // Skip raw payload metrics - contain large base64/JSON blobs useless to LLM
                                let size = if let Some(s) = latest.value.as_str() {
                                    if s.len() > 1024 {
                                        format!("{:.1}KB", s.len() as f64 / 1024.0)
                                    } else {
                                        format!("{}B", s.len())
                                    }
                                } else {
                                    "N/A".to_string()
                                };
                                metric_json["value"] = serde_json::json!(format!("[raw payload, {}]", size));
                            } else {
                                metric_json["value"] = latest.value.to_json_value();
                            }
                            if detailed {
                                metric_json["timestamp"] = serde_json::json!(latest.timestamp);
                            }
                        }
                    }
                    metrics_info.push(metric_json);
                }
                device_json["metrics"] = serde_json::json!(metrics_info);
            }

            // Append virtual metrics (not in device template) with latest values
            if let Some(storage) = &self.storage {
                if let Ok(all_stored) = storage.list_metrics(&device_id).await {
                    let template_set: std::collections::HashSet<&str> = device_json
                        .get("metrics")
                        .and_then(|m| m.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.get("name")?.as_str())
                                .collect()
                        })
                        .unwrap_or_default();

                    let virtual_names: Vec<&String> = all_stored
                        .iter()
                        .filter(|m| !template_set.contains(m.as_str()))
                        .filter(|m| !is_raw_payload_metric(m.as_str()))
                        .collect();

                    let mut virtual_metrics: Vec<Value> = Vec::new();
                    for m in &virtual_names {
                        let mut mj = serde_json::json!({
                            "name": m,
                            "display_name": m,
                        });
                        if let Ok(Some(latest)) = storage.latest(&device_id, m).await {
                            mj["value"] = latest.value.to_json_value();
                            if detailed {
                                mj["timestamp"] = serde_json::json!(latest.timestamp);
                            }
                        }
                        virtual_metrics.push(mj);
                    }

                    if !virtual_metrics.is_empty() {
                        // Merge into existing metrics array or create it
                        if let Some(arr) = device_json.get_mut("metrics").and_then(|v| v.as_array_mut()) {
                            arr.extend(virtual_metrics);
                        } else {
                            device_json["metrics"] = serde_json::json!(virtual_metrics);
                        }
                    }
                }
            }

            // Commands: only include in detailed mode
            if detailed && !template.commands.is_empty() {
                let commands_info: Vec<Value> = template
                    .commands
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "name": c.name,
                            "display_name": c.display_name,
                            "parameters": c.parameters.iter().map(|p| {
                                serde_json::json!({
                                    "name": p.name,
                                    "display_name": p.display_name,
                                    "data_type": format!("{:?}", p.data_type),
                                    "required": p.required
                                })
                            }).collect::<Vec<_>>()
                        })
                    })
                    .collect();
                device_json["commands"] = serde_json::json!(commands_info);
            }
        }

        Ok(ToolOutput::success(device_json))
    }

    async fn execute_query(&self, args: &Value) -> Result<ToolOutput> {
        let device_id_input = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id is required".into()))?;

        let device_id = self.resolve_device_id(device_id_input).await?;

        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ToolError::Execution("Storage not configured".into()))?;

        let metric = args["metric"].as_str();
        let end_time = args["end_time"]
            .as_i64()
            .unwrap_or_else(|| chrono::Utc::now().timestamp());
        let metric_str = metric.unwrap_or("");
        // Image/snapshot metrics use a wider default time window (48h) because devices
        // may capture images infrequently. Regular metrics default to 24h to ensure
        // we capture data even for infrequently reporting devices.
        let default_window = if metric_str.contains("image")
            || metric_str.contains("snapshot")
            || metric_str.contains("picture")
            || metric_str.contains("frame")
        {
            48 * 3600 // 48 hours
        } else {
            24 * 3600 // 24 hours (was 1h - too short for infrequent metrics like battery)
        };
        let start_time = args["start_time"].as_i64().unwrap_or(end_time - default_window);
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;

        if let Some(m) = metric {
            let mut data = storage
                .query(&device_id, m, start_time, end_time)
                .await
                .map_err(|e| ToolError::Execution(e.to_string()))?;

            let mut resolved_metric = m.to_string();

            // If no data found, try to resolve metric name from device template
            // This handles cases where user passes "battery" but storage key is "values.battery"
            if data.is_empty() {
                if let Some(device) = self.device_service.get_device(&device_id).await {
                    if let Some(template) = self.device_service.get_template(&device.device_type).await {
                        // Try to find a metric whose name ends with the user input
                        // e.g., "battery" matches "values.battery"
                        let m_lower = m.to_lowercase();
                        if let Some(matched_def) = template.metrics.iter().find(|def| {
                            def.name == m || def.name.ends_with(&format!(".{}", m)) || def.name == m_lower
                        }) {
                            let resolved = matched_def.name.clone();
                            if let Ok(d) = storage
                                .query(&device_id, &resolved, start_time, end_time)
                                .await
                            {
                                data = d;
                                resolved_metric = resolved;
                            }
                        }
                    }
                }
            }

            // If still no data, fall back to latest() which returns the most recent
            // data point regardless of time range. This ensures we always return
            // something useful even if the time range didn't capture any data.
            if data.is_empty() {
                if let Ok(Some(latest_point)) = storage.latest(&device_id, &resolved_metric).await {
                    tracing::info!(
                        "Time range query returned empty, using latest point (timestamp={})",
                        latest_point.timestamp
                    );
                    data = vec![latest_point];
                }
            }

            // If no data found after all attempts, the metric key is likely incorrect.
            // Return available metrics from the device template so the LLM can retry.
            if data.is_empty() {
                if let Some(device) = self.device_service.get_device(&device_id).await {
                    if let Some(template) = self.device_service.get_template(&device.device_type).await {
                        let available: Vec<Value> = template.metrics.iter().map(|def| {
                            serde_json::json!({
                                "name": def.name,
                                "display_name": def.display_name,
                                "unit": def.unit,
                            })
                        }).collect();
                        tracing::warn!(
                            "Metric '{}' not found for device '{}'. Available: {:?}",
                            m, device_id,
                            available.iter().filter_map(|v| v.get("name").and_then(|n| n.as_str())).collect::<Vec<_>>()
                        );
                        return Ok(ToolOutput::success(serde_json::json!({
                            "device_id": device_id,
                            "metric": m,
                            "error": format!("Metric '{}' not found for device '{}'. Use one of the available metrics below.", m, device_id),
                            "available_metrics": available,
                            "hint": "Use the 'name' field from available_metrics as the metric parameter."
                        })));
                    }
                }
                // No template found either — return a generic error
                return Ok(ToolOutput::success(serde_json::json!({
                    "device_id": device_id,
                    "metric": m,
                    "error": format!("Metric '{}' not found. Use device(action=\"list\", response_format=\"detailed\") to see available metrics. Use device(action=\"latest\") to get all current values.", m),
                    "points": []
                })));
            }

            // Detect if this is an image metric and format accordingly.
            // Image metrics contain data URI values like "data:image/jpeg;base64,...".
            // For images, we return a structured format with separated base64 data and mime type,
            // making it easy to pass to extension tools for analysis.
            let is_image_metric = resolved_metric.contains("image")
                || resolved_metric.contains("snapshot")
                || resolved_metric.contains("picture")
                || resolved_metric.contains("frame")
                || data.iter().any(|p| {
                    p.value.as_str().map_or(false, |s| s.starts_with("data:image/"))
                });

            if is_image_metric && !data.is_empty() {
                // For image metrics, return structured data with base64 separated from the data URI prefix.
                // This makes it easy for the LLM to pass the base64_data field directly to
                // image analysis extensions, while also preserving the mime_type for content handling.
                let points: Vec<Value> = data.iter().take(limit).map(|p| {
                    if let Some(s) = p.value.as_str() {
                        if s.starts_with("data:image/") {
                            // Parse data URI: "data:image/jpeg;base64,<base64data>"
                            let mime_type = if let Some(semi) = s.find(';') {
                                s[..semi].trim_start_matches("data:").to_string()
                            } else {
                                "image/jpeg".to_string()
                            };
                            let base64_data = if let Some(comma) = s.find(";base64,") {
                                s[comma + 8..].to_string()
                            } else {
                                s.to_string()
                            };
                            let size_kb = base64_data.len() as f64 / 1024.0;
                            return serde_json::json!({
                                "timestamp": p.timestamp,
                                "mime_type": mime_type,
                                "base64_data": base64_data,
                                "size_kb": format!("{:.1}", size_kb),
                            });
                        }
                    }
                    serde_json::json!({
                        "timestamp": p.timestamp,
                        "value": p.value.to_json_value()
                    })
                }).collect();

                Ok(ToolOutput::success(serde_json::json!({
                    "device_id": device_id,
                    "metric": resolved_metric,
                    "data_type": "image",
                    "points": points
                })))
            } else {
                Ok(ToolOutput::success(serde_json::json!({
                    "device_id": device_id,
                    "metric": resolved_metric,
                    "points": data.iter().take(limit).map(|p| serde_json::json!({
                        "timestamp": p.timestamp,
                        "value": p.value.to_json_value()
                    })).collect::<Vec<_>>()
                })))
            }
        } else {
            Ok(ToolOutput::success(serde_json::json!({
                "device_id": device_id,
                "message": "Specify a metric name to query data"
            })))
        }
    }

    async fn execute_control(&self, args: &Value) -> Result<ToolOutput> {
        let device_id_input = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id is required".into()))?;

        let device_id = self.resolve_device_id(device_id_input).await?;

        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("command is required".into()))?;

        // Validate command against device type template
        if let Some(device) = self.device_service.get_device(&device_id).await {
            if let Some(template) = self.device_service.get_template(&device.device_type).await {
                if let Some(cmd_def) = template.commands.iter().find(|c| c.name == command) {
                    let params_obj = args.get("params").and_then(|p| p.as_object());
                    let mut missing: Vec<String> = Vec::new();
                    for p in &cmd_def.parameters {
                        if p.required {
                            let has_param = params_obj
                                .map(|obj| obj.contains_key(&p.name))
                                .unwrap_or(false);
                            if !has_param {
                                let type_str = format!("{:?}", p.data_type).to_lowercase();
                                missing.push(format!("{} ({})", p.name, type_str));
                            }
                        }
                    }
                    if !missing.is_empty() {
                        return Ok(ToolOutput::error(format!(
                            "Missing required parameters for '{}': {}. Available params: {}",
                            command,
                            missing.join(", "),
                            cmd_def.parameters.iter()
                                .map(|p| format!("{}:{}{}", p.name, format!("{:?}", p.data_type).to_lowercase(), if p.required { " (required)" } else { "" }))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )));
                    }
                } else {
                    let available: Vec<&str> = template.commands.iter().map(|c| c.name.as_str()).collect();
                    return Ok(ToolOutput::error(format!(
                        "Unknown command '{}' for device '{}'. Available commands: {}",
                        command, device_id, available.join(", ")
                    )));
                }
            }
        } else {
            return Ok(ToolOutput::error(format!("Device not found: {}", device_id)));
        }

        // Confirmation check
        let confirm = args["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Ok(ToolOutput::success(serde_json::json!({
                "preview": true,
                "device_id": device_id,
                "command": command,
                "params": args.get("params").cloned().unwrap_or(serde_json::json!({})),
                "message": "This will change device state. Set confirm=true to execute."
            })));
        }

        let params: HashMap<String, Value> = args
            .get("params")
            .and_then(|p| p.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        self.device_service
            .send_command(&device_id, command, params)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "device_id": device_id,
            "command": command,
            "status": "executed"
        })))
    }

    /// Write a value to a device metric (virtual or existing).
    async fn execute_write(&self, args: &Value) -> Result<ToolOutput> {
        let device_id_input = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id is required".into()))?;

        let device_id = self.resolve_device_id(device_id_input).await?;

        let metric = args["metric"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("metric is required".into()))?;

        // value is required — accept any JSON primitive
        let json_value = args.get("value");
        let metric_value = match json_value {
            None | Some(Value::Null) => neomind_devices::mdl::MetricValue::Null,
            Some(Value::Bool(b)) => neomind_devices::mdl::MetricValue::Boolean(*b),
            Some(Value::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    neomind_devices::mdl::MetricValue::Integer(i)
                } else {
                    neomind_devices::mdl::MetricValue::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            Some(Value::String(s)) => neomind_devices::mdl::MetricValue::String(s.clone()),
            Some(_) => {
                return Err(ToolError::InvalidArguments(
                    "value must be a string, number, boolean, or null".into(),
                ))
            }
        };

        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ToolError::Execution("Storage not configured".into()))?;

        let timestamp = args["timestamp"]
            .as_i64()
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        let point = neomind_devices::DataPoint {
            timestamp,
            value: metric_value,
            quality: Some(1.0),
        };

        storage
            .write(&device_id, metric, point)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to write metric: {}", e)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "device_id": device_id,
            "metric": metric,
            "status": "written"
        })))
    }
}

// ============================================================================
// Agent Tool - Aggregates: list_agents, get_agent, create_agent, update_agent,
//                          control_agent, agent_memory
// ============================================================================

/// Aggregated agent tool with action-based routing.
pub struct AgentTool {
    agent_store: Arc<neomind_storage::AgentStore>,
}

impl AgentTool {
    /// Create a new agent tool.
    pub fn new(agent_store: Arc<neomind_storage::AgentStore>) -> Self {
        Self { agent_store }
    }
}

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "agent"
    }

    fn description(&self) -> &str {
        r#"AI Agent management tool for creating and managing automated agents.

Actions:
- list: List all agents or filter by status (active/paused/stopped/error). Use when user asks about existing agents.
- get: Get agent details including config, schedule, and execution stats. Use when user asks about a specific agent.
- create: Create a new automated agent. Requires: name, user_prompt, schedule_type. Use when user wants to automate a monitoring or control task.
- update: Modify an existing agent's configuration (name, description, user_prompt). Use when user wants to change agent behavior.
- control: Pause or resume agent execution (control_action: pause/resume). WARNING: This affects running agents.
- memory: View agent's learned patterns and intent understanding. Use when debugging agent behavior.
- send_message: Send a message or instruction to the agent. The agent will see it in its next execution. Use when user wants to guide, correct, or update an agent's behavior through natural language.
- executions: View agent execution statistics (total runs, success rate, last execution time). Use when user asks about agent performance or reliability.
- conversation: View agent's conversation history (inputs and outputs from past runs). Use when debugging agent behavior or reviewing what an agent did.
- latest_execution: View the most recent execution with full details (analysis, reasoning, decisions, conclusion). Use when user asks about execution results or completion status.

When creating agents:
- schedule_type: 'event' (triggered by device events), 'cron' (cron schedule), 'interval' (periodic, e.g., every 5 minutes)
- user_prompt should be specific, e.g., 'Check temperature every 5 minutes, alert if above 30C'
- Use response_format="detailed" to get full agent config including IDs"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "create", "update", "control", "memory", "send_message", "executions", "conversation", "latest_execution"],
                    "description": "Operation type: 'list' (all agents), 'get' (agent details), 'create' (new agent), 'update' (modify agent), 'control' (pause/resume), 'memory' (view learned patterns), 'send_message' (send message to agent), 'executions' (execution stats), 'conversation' (conversation log), 'latest_execution' (most recent execution details)"
                },
                "agent_id": {
                    "type": "string",
                    "description": "Agent ID or name. Supports fuzzy matching (e.g., 'Temperature Monitor' matches by name). Use list action to discover available agents. Examples: '550e8400-...', 'Temperature Monitor'"
                },
                "message": {
                    "type": "string",
                    "description": "Message content to send to the agent (send_message action). The agent will see this in its next execution. Example: 'Focus on monitoring the front door area'"
                },
                "message_type": {
                    "type": "string",
                    "description": "Optional message type/tag for categorization (send_message action). Example: 'instruction', 'correction', 'update'"
                },
                "name": {
                    "type": "string",
                    "description": "Agent display name. Required for create, optional for update. Example: 'Temperature Monitor', 'Security Patrol'"
                },
                "description": {
                    "type": "string",
                    "description": "Agent description. Optional for create/update. Example: 'Monitors living room temperature and alerts on threshold breach'"
                },
                "user_prompt": {
                    "type": "string",
                    "description": "DETAILED requirements for the agent (create action). Write a structured, specific prompt describing: 1) What to monitor/check, 2) Thresholds/conditions, 3) Actions to take when conditions met, 4) Output format. Example: '检查所有温度传感器的最新读数。如果任何传感器温度超过30°C，立即发送紧急通知。同时每小时生成一份温度摘要报告。使用中文回复。' NOT just copying user input — expand into a proper agent instruction."
                },
                "schedule_type": {
                    "type": "string",
                    "description": "How the agent is triggered (create action): 'cron' (fixed time schedule, e.g., daily at 8am), 'interval' (periodic execution every N seconds), 'event' (triggered when device data changes in real-time). Choose based on user intent: daily/weekly = cron, every X minutes = interval, react to data changes = event."
                },
                "schedule_config": {
                    "type": "string",
                    "description": "Schedule configuration (create action). For cron: standard 5-field expression (e.g., '0 8 * * *' = daily 8am, '*/30 * * * *' = every 30min). For interval: number of seconds (e.g., '300' = every 5min, '3600' = hourly). For event: comma-separated DataSourceIds to watch, format '{type}:{id}:{field}' (e.g., 'device:sensor_001:temperature,extension:weather:humidity')."
                },
                "execution_mode": {
                    "type": "string",
                    "description": "Agent execution mode (create action): 'chat' = single-pass analysis (default, good for monitoring/reporting), 'react' = multi-round tool calling loop (good for complex automation needing device control or multiple tool calls). Use 'react' if agent needs to control devices, query multiple tools, or perform multi-step actions."
                },
                "resources": {
                    "type": "string",
                    "description": "Resources to bind to this agent (create action, multi-select). Format: JSON array of objects, each with 'type' and 'id'. Supported types: 'device' (full device, id=device_id), 'metric' (device metric, id='device_id:metric_name'), 'command' (device command, id='device_id:command_name'), 'extension_metric' (extension metric, id='extension:ext_id:metric_name'), 'extension_tool' (extension command, id='extension:ext_id:tool_name'). Prefer finest granularity: bind specific metrics/commands rather than whole devices. Example: [{\"type\":\"metric\",\"id\":\"sensor_001:temperature\"},{\"type\":\"extension_tool\",\"id\":\"extension:weather:forecast\"}]"
                },
                "enable_tool_chaining": {
                    "type": "boolean",
                    "description": "Enable tool chaining for react mode (create action). When true, agent can use output from one tool as input to another. Default: false. Set true for complex automation workflows."
                },
                "control_action": {
                    "type": "string",
                    "description": "Control operation (control action): 'pause' (stop execution temporarily), 'resume' (restart paused agent)"
                },
                "status": {
                    "type": "string",
                    "description": "Filter by agent status (list action): 'active', 'paused', 'stopped', 'error', 'executing'"
                },
                "limit": {
                    "type": "number",
                    "description": "Max results to return (list/memory actions). Omit to return all results."
                },
                "memory_type": {
                    "type": "string",
                    "description": "Type of memory to retrieve (memory action): 'patterns' (learned patterns, default), 'intents' (parsed intent). Default: 'patterns'"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': name/status/schedule only (default). 'detailed': full config with IDs and metadata"
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Set to true after user confirms. Required for control action. Without confirmation, returns a preview"
                }
            }),
            vec!["action".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "list" => self.execute_list(&args).await,
            "get" => self.execute_get(&args).await,
            "create" => self.execute_create(&args).await,
            "update" => self.execute_update(&args).await,
            "control" => self.execute_control(&args).await,
            "memory" => self.execute_memory(&args).await,
            "send_message" => self.execute_send_message(&args).await,
            "executions" => self.execute_executions(&args).await,
            "conversation" => self.execute_conversation(&args).await,
            "latest_execution" => self.execute_latest_execution(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: {}",
                action
            ))),
        }
    }
}

impl AgentTool {
    fn is_detailed(args: &Value) -> bool {
        args["response_format"].as_str() == Some("detailed")
    }

    /// Resolve agent_id with fuzzy matching using EntityResolver.
    async fn resolve_agent_id(&self, input: &str) -> Result<String> {
        // Fast path: exact ID match
        if let Ok(Some(_)) = self.agent_store.get_agent(input).await {
            return Ok(input.to_string());
        }

        // Slow path: fuzzy match by name
        let agents = self
            .agent_store
            .query_agents(neomind_storage::agents::AgentFilter::default())
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        let candidates: Vec<(String, String)> =
            agents.iter().map(|a| (a.id.clone(), a.name.clone())).collect();

        super::resolver::EntityResolver::resolve(input, &candidates, "agent")
            .map_err(ToolError::Execution)
    }

    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        use neomind_storage::agents::{AgentFilter, AgentStatus};

        let mut filter = AgentFilter::default();

        if let Some(status) = args["status"].as_str() {
            filter.status = match status {
                "active" => Some(AgentStatus::Active),
                "paused" => Some(AgentStatus::Paused),
                "stopped" => Some(AgentStatus::Stopped),
                "error" => Some(AgentStatus::Error),
                "executing" => Some(AgentStatus::Executing),
                _ => None,
            };
        }

        // Only apply limit if explicitly requested and reasonable (>5).
        // LLMs sometimes pass arbitrary small limits; ignore those to show complete data.
        if let Some(limit) = args["limit"].as_u64() {
            if limit > 5 {
                filter.limit = Some(limit as usize);
            }
        }

        let agents = self
            .agent_store
            .query_agents(filter)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        let list: Vec<Value> = agents
            .iter()
            .map(|a| {
                if Self::is_detailed(args) {
                    serde_json::json!({
                        "id": a.id,
                        "name": a.name,
                        "description": a.description,
                        "status": format!("{:?}", a.status).to_lowercase(),
                        "schedule_type": format!("{:?}", a.schedule.schedule_type).to_lowercase()
                    })
                } else {
                    serde_json::json!({
                        "id": a.id,
                        "name": a.name,
                        "status": format!("{:?}", a.status).to_lowercase(),
                        "schedule_type": format!("{:?}", a.schedule.schedule_type).to_lowercase()
                    })
                }
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": list.len(),
            "agents": list
        })))
    }

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let resolved_id = self.resolve_agent_id(agent_id_input).await?;

        let agent = self
            .agent_store
            .get_agent(&resolved_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", resolved_id)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": agent.id,
            "name": agent.name,
            "description": agent.description,
            "user_prompt": agent.user_prompt,
            "status": format!("{:?}", agent.status).to_lowercase(),
            "schedule": agent.schedule,
            "stats": agent.stats
        })))
    }

    async fn execute_create(&self, args: &Value) -> Result<ToolOutput> {
        use neomind_storage::agents::{AgentSchedule, AgentStatus, AiAgent, ScheduleType};

        let name = args["name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("name is required".into()))?;

        let user_prompt = args["user_prompt"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("user_prompt is required".into()))?;

        let schedule_type_str = args["schedule_type"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("schedule_type is required".into()))?;

        let schedule_type = match schedule_type_str {
            "event" => ScheduleType::Event,
            "cron" => ScheduleType::Cron,
            "interval" => ScheduleType::Interval,
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Invalid schedule_type: {}",
                    schedule_type_str
                )))
            }
        };

        let now = chrono::Utc::now().timestamp();

        // Parse execution_mode
        let execution_mode = match args["execution_mode"].as_str() {
            Some("react") => neomind_storage::agents::ExecutionMode::React,
            _ => neomind_storage::agents::ExecutionMode::Chat, // default
        };

        // Parse resources — supports JSON array of {type, id} objects
        let agent_resources: Vec<neomind_storage::agents::AgentResource> = if let Some(res_val) = args.get("resources") {
            if let Some(arr) = res_val.as_array() {
                // JSON array format: [{"type":"metric","id":"sensor_001:temperature"}, ...]
                arr.iter().filter_map(|item| {
                    let type_str = item.get("type")?.as_str()?;
                    let id = item.get("id")?.as_str()?.to_string();
                    let resource_type = match type_str {
                        "device" => neomind_storage::agents::ResourceType::Device,
                        "metric" => neomind_storage::agents::ResourceType::Metric,
                        "command" => neomind_storage::agents::ResourceType::Command,
                        "extension_metric" => neomind_storage::agents::ResourceType::ExtensionMetric,
                        "extension_tool" => neomind_storage::agents::ResourceType::ExtensionTool,
                        "data_stream" => neomind_storage::agents::ResourceType::DataStream,
                        _ => return None,
                    };
                    Some(neomind_storage::agents::AgentResource {
                        resource_type,
                        resource_id: id.clone(),
                        name: id,
                        config: serde_json::json!({}),
                    })
                }).collect()
            } else if let Some(s) = res_val.as_str() {
                // Fallback: comma-separated device IDs (backward compat)
                s.split(',')
                    .filter(|id| !id.trim().is_empty())
                    .map(|id| neomind_storage::agents::AgentResource {
                        resource_type: neomind_storage::agents::ResourceType::Device,
                        resource_id: id.trim().to_string(),
                        name: id.trim().to_string(),
                        config: serde_json::json!({}),
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Parse event_filter from schedule_config when schedule_type is event
        let is_event = matches!(schedule_type, ScheduleType::Event);
        let is_cron = matches!(schedule_type, ScheduleType::Cron);
        let is_interval = matches!(schedule_type, ScheduleType::Interval);
        let event_filter = if is_event {
            args["schedule_config"].as_str().map(|s| s.to_string())
        } else {
            None
        };

        // Parse enable_tool_chaining
        let enable_tool_chaining = args["enable_tool_chaining"].as_bool().unwrap_or(false);

        let agent = AiAgent {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: args["description"].as_str().map(|s| s.to_string()),
            user_prompt: user_prompt.to_string(),
            llm_backend_id: None,
            parsed_intent: None,
            resources: agent_resources,
            schedule: AgentSchedule {
                schedule_type,
                cron_expression: if is_cron {
                    args["schedule_config"].as_str().map(|s| s.to_string())
                } else {
                    None
                },
                interval_seconds: if is_interval {
                    args["schedule_config"].as_u64()
                } else {
                    None
                },
                event_filter,
                timezone: None,
            },
            status: AgentStatus::Active,
            priority: 128,
            created_at: now,
            updated_at: now,
            last_execution_at: None,
            stats: AgentStats::default(),
            memory: AgentMemory::default(),
            conversation_history: Vec::new(),
            user_messages: Vec::new(),
            conversation_summary: None,
            context_window_size: 10,
            enable_tool_chaining,
            max_chain_depth: 3,
            tool_config: None,
            execution_mode,
            error_message: None,
            max_retries: 0,
            consecutive_failures: 0,
        };

        let id = agent.id.clone();
        self.agent_store
            .save_agent(&agent)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": id,
            "name": agent.name,
            "status": "created"
        })))
    }

    async fn execute_update(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let resolved_id = self.resolve_agent_id(agent_id_input).await?;

        let mut agent = self
            .agent_store
            .get_agent(&resolved_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", resolved_id)))?;

        if let Some(name) = args["name"].as_str() {
            agent.name = name.to_string();
        }
        if let Some(desc) = args["description"].as_str() {
            agent.description = Some(desc.to_string());
        }
        if let Some(prompt) = args["user_prompt"].as_str() {
            agent.user_prompt = prompt.to_string();
        }

        self.agent_store
            .save_agent(&agent)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": resolved_id,
            "status": "updated"
        })))
    }

    async fn execute_control(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent_id = self.resolve_agent_id(agent_id_input).await?;

        let control_action = args["control_action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("control_action is required".into()))?;

        // Confirmation check
        let confirm = args["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Ok(ToolOutput::success(serde_json::json!({
                "preview": true,
                "agent_id": agent_id,
                "control_action": control_action,
                "message": "This will change agent execution state. Set confirm=true to execute."
            })));
        }

        use neomind_storage::agents::AgentStatus;

        let (status, error_msg) = match control_action {
            "pause" => (AgentStatus::Paused, None),
            "resume" => (AgentStatus::Active, None),
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown control_action: {}",
                    control_action
                )))
            }
        };

        self.agent_store
            .update_agent_status(&agent_id, status, error_msg)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": agent_id,
            "action": control_action,
            "status": "success"
        })))
    }

    async fn execute_memory(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent_id = self.resolve_agent_id(agent_id_input).await?;

        let agent = self
            .agent_store
            .get_agent(&agent_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", agent_id)))?;

        let memory_type = args["memory_type"].as_str().unwrap_or("patterns");
        let limit = args["limit"].as_u64().unwrap_or(20) as usize;

        match memory_type {
            "patterns" => Ok(ToolOutput::success(serde_json::json!({
                "agent_id": agent_id,
                "patterns": agent.memory.learned_patterns.iter().take(limit).cloned().collect::<Vec<_>>()
            }))),
            "intents" => Ok(ToolOutput::success(serde_json::json!({
                "agent_id": agent_id,
                "intent": agent.parsed_intent
            }))),
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown memory_type: {}",
                memory_type
            ))),
        }
    }

    async fn execute_send_message(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent_id = self.resolve_agent_id(agent_id_input).await?;

        let content = args["message"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("message is required".into()))?;

        let message_type = args["message_type"].as_str().map(|s| s.to_string());

        let user_msg = self
            .agent_store
            .add_user_message(&agent_id, content.to_string(), message_type)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "agent_id": agent_id,
            "message_id": user_msg.id,
            "status": "delivered",
            "note": "Message will be included in the agent's next execution context"
        })))
    }

    async fn execute_executions(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent = self
            .agent_store
            .get_agent(agent_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", agent_id)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "agent_id": agent_id,
            "stats": agent.stats
        })))
    }

    /// Compact a JSON value by replacing large strings (images, base64) with size summaries.
    fn compact_value(val: &serde_json::Value, max_str_len: usize) -> serde_json::Value {
        match val {
            serde_json::Value::String(s) => {
                if s.len() > max_str_len {
                    let size_mb = s.len() as f64 / (1024.0 * 1024.0);
                    if s.starts_with("data:image/") || s.starts_with("/9j/") || s.starts_with("iVBOR") {
                        serde_json::json!(format!("[图像数据: {:.1}MB, 已省略]", size_mb))
                    } else if size_mb > 0.1 {
                        serde_json::json!(format!("[大数据: {:.1}MB, 已省略]", size_mb))
                    } else {
                        serde_json::json!(format!("[{}...已省略{}字符]", &s[..max_str_len.min(s.len())], s.len() - max_str_len.min(s.len())))
                    }
                } else {
                    val.clone()
                }
            }
            serde_json::Value::Object(map) => {
                let compacted: serde_json::Map<String, serde_json::Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), Self::compact_value(v, max_str_len)))
                    .collect();
                serde_json::Value::Object(compacted)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| Self::compact_value(v, max_str_len)).collect())
            }
            _ => val.clone(),
        }
    }

    /// Build a compact summary of TurnInput, filtering out large binary/image data.
    fn compact_input(input: &neomind_storage::TurnInput) -> serde_json::Value {
        let data_summaries: Vec<serde_json::Value> = input
            .data_collected
            .iter()
            .map(|dc| {
                let compacted_values = Self::compact_value(&dc.values, 200);
                serde_json::json!({
                    "source": dc.source,
                    "data_type": dc.data_type,
                    "values": compacted_values,
                })
            })
            .collect();

        let mut result = serde_json::Map::new();
        result.insert("data_collected".into(), serde_json::Value::Array(data_summaries));
        if let Some(ref event) = input.event_data {
            result.insert("event_data".into(), Self::compact_value(event, 200));
        }
        serde_json::Value::Object(result)
    }

    /// Build a focused output summary from TurnOutput for conversation history.
    fn compact_output(output: &neomind_storage::TurnOutput) -> serde_json::Value {
        let decisions: Vec<serde_json::Value> = output
            .decisions
            .iter()
            .map(|d| {
                serde_json::json!({
                    "type": d.decision_type,
                    "description": d.description,
                    "action": d.action,
                })
            })
            .collect();

        let reasoning: Vec<serde_json::Value> = output
            .reasoning_steps
            .iter()
            .map(|rs| {
                serde_json::json!({
                    "step": rs.step_number,
                    "type": rs.step_type,
                    "description": rs.description,
                    "output": rs.output,
                })
            })
            .collect();

        serde_json::json!({
            "situation_analysis": output.situation_analysis,
            "reasoning_steps": reasoning,
            "decisions": decisions,
            "conclusion": output.conclusion,
        })
    }

    async fn execute_conversation(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let limit = args["limit"].as_u64().unwrap_or(50) as usize;

        let agent = self
            .agent_store
            .get_agent(agent_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", agent_id)))?;

        let conversation: Vec<Value> = agent
            .conversation_history
            .iter()
            .take(limit)
            .map(|turn| {
                serde_json::json!({
                    "execution_id": turn.execution_id,
                    "timestamp": turn.timestamp,
                    "trigger_type": turn.trigger_type,
                    "success": turn.success,
                    "duration_ms": turn.duration_ms,
                    "input": Self::compact_input(&turn.input),
                    "output": Self::compact_output(&turn.output),
                })
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "agent_id": agent_id,
            "total_turns": agent.conversation_history.len(),
            "messages": conversation
        })))
    }

    async fn execute_latest_execution(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent = self
            .agent_store
            .get_agent(agent_id_input)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| {
                ToolError::Execution(format!("Agent not found: {}", agent_id_input))
            })?;

        let last_turn = agent.conversation_history.first();

        let stats_summary = serde_json::json!({
            "total_executions": agent.stats.total_executions,
            "successful_executions": agent.stats.successful_executions,
            "failed_executions": agent.stats.failed_executions,
            "success_rate": if agent.stats.total_executions > 0 {
                format!("{:.0}%", (agent.stats.successful_executions as f64 / agent.stats.total_executions as f64) * 100.0)
            } else {
                "N/A".to_string()
            },
            "last_duration_ms": last_turn.map(|t| t.duration_ms),
        });

        let last_execution = last_turn.map(|turn| {
            serde_json::json!({
                "execution_id": turn.execution_id,
                "timestamp": turn.timestamp,
                "trigger_type": turn.trigger_type,
                "success": turn.success,
                "duration_ms": turn.duration_ms,
                "input": Self::compact_input(&turn.input),
                "output": Self::compact_output(&turn.output),
            })
        });

        // Build memory summary for user insight
        let recent_summaries: Vec<serde_json::Value> = agent.memory.short_term.summaries
            .iter()
            .take(3)
            .map(|s| serde_json::json!({
                "situation": s.situation,
                "conclusion": s.conclusion,
                "decisions": s.decisions,
                "timestamp": s.timestamp,
                "success": s.success,
            }))
            .collect();

        let memory_summary = serde_json::json!({
            "working_memory": {
                "current_analysis": agent.memory.working.current_analysis,
                "current_conclusion": agent.memory.working.current_conclusion,
            },
            "recent_summaries": recent_summaries,
            "learned_patterns_count": agent.memory.learned_patterns.len(),
            "baselines_count": agent.memory.baselines.len(),
            "recent_patterns": agent.memory.learned_patterns.iter().take(5).map(|p| {
                serde_json::json!({
                    "pattern_type": p.pattern_type,
                    "description": p.description,
                    "confidence": p.confidence,
                })
            }).collect::<Vec<_>>(),
        });

        Ok(ToolOutput::success(serde_json::json!({
            "agent_id": agent_id_input,
            "agent_name": agent.name,
            "agent_status": format!("{:?}", agent.status).to_lowercase(),
            "last_execution": last_execution,
            "memory": memory_summary,
            "pending_user_messages": agent.user_messages.len(),
            "stats_summary": stats_summary
        })))
    }
}

// ============================================================================
// Rule Tool - Aggregates: list_rules, get_rule, delete_rule, history
// ============================================================================

/// Aggregated rule tool.
pub struct RuleTool {
    rule_engine: Arc<neomind_rules::RuleEngine>,
    history_storage: Option<Arc<neomind_rules::RuleHistoryStorage>>,
}

impl RuleTool {
    /// Create a new rule tool.
    pub fn new(rule_engine: Arc<neomind_rules::RuleEngine>) -> Self {
        Self {
            rule_engine,
            history_storage: None,
        }
    }

    /// Create with history storage.
    pub fn with_history(
        rule_engine: Arc<neomind_rules::RuleEngine>,
        history_storage: Arc<neomind_rules::RuleHistoryStorage>,
    ) -> Self {
        Self {
            rule_engine,
            history_storage: Some(history_storage),
        }
    }
}

#[async_trait]
impl Tool for RuleTool {
    fn name(&self) -> &str {
        "rule"
    }

    fn description(&self) -> &str {
        r#"Rule management tool for automation rules that trigger actions based on conditions.

Actions:
- list: List all rules (with status). Filter by name. Use when user asks about existing rules.
- get: Get rule details including DSL, status, trigger stats. Use when user asks about a specific rule.
- create: Create a new rule from DSL. Requires: dsl.
- update: Replace a rule's DSL. WARNING: Deletes old rule, creates new one. Requires: rule_id, dsl.
- delete: Permanently remove a rule. Requires confirmation.
- history: View rule execution history.
- enable: Enable (resume) or disable (pause) a rule. Requires: rule_id, enabled.

Rule DSL Syntax:
RULE "rule_name" [DESCRIPTION "desc"] WHEN condition [FOR duration] DO actions END

Conditions:
- Device: device_id.metric OPERATOR value
  Example: sensor_01.temperature > 30
- Extension: EXTENSION ext_id.metric OPERATOR value
  Example: EXTENSION weather.temperature > 35
- Range: device_id.metric BETWEEN min AND max
  Example: sensor_01.humidity BETWEEN 40 AND 60
- AND: cond1 AND cond2  (higher precedence than OR)
  Example: sensor.temp > 30 AND sensor.humidity < 20
- OR: cond1 OR cond2
  Example: device.status == "on" OR device.status == "standby"
- NOT: NOT condition
  Example: NOT device.power == 0
- Parentheses for grouping: (cond1) AND (cond2)

Operators: <, >, <=, >=, ==, !=
FOR duration: FOR 5 seconds / FOR 10 minutes / FOR 1 hour

Actions (one or more, each on its own line):
- NOTIFY "message" — send alert notification
  NOTIFY "Temperature is {temperature}"
- EXECUTE device_id.command(param=value, ...) — send device command
  EXECUTE fan_01.set_speed(speed=100, mode=auto)
- SET device_id.property = value — set device property
  SET thermostat_01.target_temp = 25.5
- LOG level, "message" — log entry (level: info/warning/error)
  LOG warning, "Device overheating"
- ALERT "title" "message" [severity=LEVEL] — create alert (severity: INFO/WARNING/CRITICAL)
  ALERT "High Temp" "Device overheating" severity=CRITICAL
- HTTP METHOD url — make HTTP request (GET/POST/PUT/DELETE)
  HTTP POST https://api.example.com/webhook
- DELAY duration — wait before next action (5 seconds, 10 minutes, 1 hour)
  DELAY 5 seconds

Full examples:
RULE "Low Battery Alert" WHEN sensor_01.battery < 20 DO NOTIFY "Battery critical" END
RULE "Temp Control" WHEN sensor_01.temperature > 30 FOR 5 minutes DO SET ac_01.power = "on" END
RULE "Weather Alert" WHEN EXTENSION weather.temperature > 35 DO ALERT "Heat Wave" "External temp above 35C" severity=CRITICAL END
RULE "Safety Check" WHEN (smoke_01.level > 50) AND (temp_01.temperature > 60) DO EXECUTE alarm_01.trigger(mode=emergency) END"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "create", "update", "delete", "history", "enable"],
                    "description": "Operation type: 'list' (all rules with status), 'get' (rule details), 'create' (new rule), 'update' (modify rule), 'delete' (remove rule), 'history' (execution log), 'enable' (pause/resume rule)"
                },
                "rule_id": {
                    "type": "string",
                    "description": "Rule ID or name. Supports fuzzy matching (e.g., 'Low Battery Alert' matches by name). Use list action to discover available rules"
                },
                "dsl": {
                    "type": "string",
                    "description": "Rule DSL definition. Required for create/update.\nSyntax: RULE \"name\" WHEN condition [FOR duration] DO actions END\nConditions: device.metric OP value, EXTENSION ext.metric OP value, BETWEEN min AND max, AND/OR/NOT, parentheses.\nActions: NOTIFY \"msg\", EXECUTE dev.cmd(k=v), SET dev.prop = v, LOG level \"msg\", ALERT \"title\" \"msg\", HTTP METHOD url, DELAY duration.\nExamples:\nRULE \"Low Battery\" WHEN sensor_01.battery < 20 DO NOTIFY \"Battery critical\" END\nRULE \"Temp Control\" WHEN sensor_01.temperature > 30 FOR 5 minutes DO SET ac_01.power = \"on\" END\nRULE \"Weather\" WHEN EXTENSION weather.temp > 35 DO ALERT \"Heat\" \"Hot\" severity=CRITICAL END"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "For 'enable' action: true to resume (activate) the rule, false to pause it. Requires rule_id"
                },
                "name_filter": {
                    "type": "string",
                    "description": "Filter rules by name substring (list action). Example: 'battery', 'temperature'"
                },
                "limit": {
                    "type": "number",
                    "description": "Max results to return. Default: 100"
                },
                "start_time": {
                    "type": "number",
                    "description": "Start timestamp for history range (history action). Unix timestamp in seconds"
                },
                "end_time": {
                    "type": "number",
                    "description": "End timestamp for history range (history action). Unix timestamp in seconds"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': name/status only (default). 'detailed': full DSL and metadata"
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Set to true after user confirms. Required for delete and update actions. Without confirmation, returns a preview instead of executing"
                }
            }),
            vec!["action".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Rule
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "list" => self.execute_list(&args).await,
            "get" => self.execute_get(&args).await,
            "create" => self.execute_create(&args).await,
            "update" => self.execute_update(&args).await,
            "delete" => self.execute_delete(&args).await,
            "history" => self.execute_history(&args).await,
            "enable" => self.execute_enable(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: {}",
                action
            ))),
        }
    }
}

impl RuleTool {
    fn is_detailed(args: &Value) -> bool {
        args["response_format"].as_str() == Some("detailed")
    }

    /// Resolve rule_id with fuzzy matching using EntityResolver.
    async fn resolve_rule_id(&self, input: &str) -> Result<String> {
        // Try exact parse first
        if let Ok(rule_id) = neomind_rules::RuleId::from_string(input) {
            if self.rule_engine.get_rule(&rule_id).await.is_some() {
                return Ok(input.to_string());
            }
        }

        // Fuzzy match by name
        let rules = self.rule_engine.list_rules().await;
        let candidates: Vec<(String, String)> = rules
            .iter()
            .map(|r| (r.id.to_string(), r.name.clone()))
            .collect();

        super::resolver::EntityResolver::resolve(input, &candidates, "规则")
            .map_err(ToolError::Execution)
    }

    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let rules = self.rule_engine.list_rules().await;

        let name_filter = args["name_filter"].as_str();
        let limit = args["limit"].as_u64().unwrap_or(100) as usize;
        let detailed = Self::is_detailed(args);

        let filtered: Vec<Value> = rules
            .iter()
            .filter(|r| {
                if let Some(name) = name_filter {
                    r.name.contains(name)
                } else {
                    true
                }
            })
            .take(limit)
            .map(|r| {
                let status_str = match r.status {
                    neomind_rules::RuleStatus::Active => "active",
                    neomind_rules::RuleStatus::Paused => "paused",
                    neomind_rules::RuleStatus::Triggered => "triggered",
                    neomind_rules::RuleStatus::Disabled => "disabled",
                };
                if detailed {
                    serde_json::json!({
                        "id": r.id.to_string(),
                        "name": r.name,
                        "description": r.description,
                        "status": status_str,
                        "dsl": r.dsl,
                        "trigger_count": r.state.trigger_count,
                        "last_triggered": r.state.last_triggered.map(|t| t.to_rfc3339()),
                    })
                } else {
                    serde_json::json!({
                        "id": r.id.to_string(),
                        "name": r.name,
                        "status": status_str,
                        "description": r.description
                    })
                }
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": filtered.len(),
            "rules": filtered
        })))
    }

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let rule_id_input = args["rule_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("rule_id is required".into()))?;

        let resolved_id = self.resolve_rule_id(rule_id_input).await?;

        let rule_id = neomind_rules::RuleId::from_string(&resolved_id)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid rule_id: {}", e)))?;

        let rule = self
            .rule_engine
            .get_rule(&rule_id)
            .await
            .ok_or_else(|| ToolError::Execution(format!("Rule not found: {}", resolved_id)))?;

        let status_str = match rule.status {
            neomind_rules::RuleStatus::Active => "active",
            neomind_rules::RuleStatus::Paused => "paused",
            neomind_rules::RuleStatus::Triggered => "triggered",
            neomind_rules::RuleStatus::Disabled => "disabled",
        };

        Ok(ToolOutput::success(serde_json::json!({
            "id": rule.id.to_string(),
            "name": rule.name,
            "description": rule.description,
            "status": status_str,
            "dsl": rule.dsl,
            "trigger_count": rule.state.trigger_count,
            "last_triggered": rule.state.last_triggered.map(|t| t.to_rfc3339()),
            "created_at": rule.created_at.to_rfc3339(),
        })))
    }

    async fn execute_delete(&self, args: &Value) -> Result<ToolOutput> {
        let rule_id_input = args["rule_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("rule_id is required".into()))?;

        let resolved_id = self.resolve_rule_id(rule_id_input).await?;

        // Confirmation check
        let confirm = args["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Ok(ToolOutput::success(serde_json::json!({
                "preview": true,
                "rule_id": resolved_id,
                "message": "This will permanently delete the rule. Set confirm=true to execute."
            })));
        }

        let rule_id = neomind_rules::RuleId::from_string(&resolved_id)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid rule_id: {}", e)))?;

        self.rule_engine
            .remove_rule(&rule_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": resolved_id,
            "status": "deleted"
        })))
    }

    async fn execute_create(&self, args: &Value) -> Result<ToolOutput> {
        let dsl = args["dsl"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("dsl is required".into()))?;

        let rule_id = self
            .rule_engine
            .add_rule_from_dsl(dsl)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to create rule: {}", e)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": rule_id.to_string(),
            "status": "created",
            "message": "规则创建成功"
        })))
    }

    async fn execute_update(&self, args: &Value) -> Result<ToolOutput> {
        let rule_id_input = args["rule_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("rule_id is required".into()))?;

        let dsl = args["dsl"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("dsl is required".into()))?;

        let resolved_id = self.resolve_rule_id(rule_id_input).await?;

        // Confirmation check
        let confirm = args["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Ok(ToolOutput::success(serde_json::json!({
                "preview": true,
                "rule_id": resolved_id,
                "new_dsl": dsl,
                "message": "This will replace the rule definition. Set confirm=true to execute."
            })));
        }

        let rule_id = neomind_rules::RuleId::from_string(&resolved_id)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid rule_id: {}", e)))?;

        // First remove the old rule to clean up dependencies
        self.rule_engine
            .remove_rule(&rule_id)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to remove old rule: {}", e)))?;

        // Parse and add the new rule
        let new_rule_id =
            self.rule_engine.add_rule_from_dsl(dsl).await.map_err(|e| {
                ToolError::Execution(format!("Failed to create updated rule: {}", e))
            })?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": new_rule_id.to_string(),
            "old_id": resolved_id,
            "status": "updated",
            "message": "规则更新成功"
        })))
    }

    async fn execute_history(&self, args: &Value) -> Result<ToolOutput> {
        let storage = self
            .history_storage
            .as_ref()
            .ok_or_else(|| ToolError::Execution("History storage not configured".into()))?;

        use neomind_rules::HistoryFilter;

        let mut filter = HistoryFilter::default();

        if let Some(rule_id) = args["rule_id"].as_str() {
            filter.rule_id = Some(rule_id.to_string());
        }
        if let Some(start) = args["start_time"].as_i64() {
            filter.start = Some(chrono::DateTime::from_timestamp(start, 0).unwrap_or_default());
        }
        if let Some(end) = args["end_time"].as_i64() {
            filter.end = Some(chrono::DateTime::from_timestamp(end, 0).unwrap_or_default());
        }
        if let Some(limit) = args["limit"].as_u64() {
            filter.limit = Some(limit as usize);
        }

        let history = storage
            .query(&filter)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "history": history
        })))
    }

    async fn execute_enable(&self, args: &Value) -> Result<ToolOutput> {
        let rule_id_input = args["rule_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("rule_id is required".into()))?;

        let enabled = args["enabled"].as_bool().unwrap_or(true);

        let resolved_id = self.resolve_rule_id(rule_id_input).await?;

        let rule_id = neomind_rules::RuleId::from_string(&resolved_id)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid rule_id: {}", e)))?;

        if enabled {
            self.rule_engine
                .resume_rule(&rule_id)
                .await
                .map_err(|e| ToolError::Execution(format!("Failed to resume rule: {}", e)))?;

            Ok(ToolOutput::success(serde_json::json!({
                "id": resolved_id,
                "status": "resumed",
                "enabled": true,
                "message": "Rule resumed successfully"
            })))
        } else {
            self.rule_engine
                .pause_rule(&rule_id)
                .await
                .map_err(|e| ToolError::Execution(format!("Failed to pause rule: {}", e)))?;

            Ok(ToolOutput::success(serde_json::json!({
                "id": resolved_id,
                "status": "paused",
                "enabled": false,
                "message": "Rule paused successfully"
            })))
        }
    }
}

// ============================================================================
// Message Tool - Aggregates: list_messages, send_message, read_message
// ============================================================================

/// Message information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMessageInfo {
    pub id: String,
    pub title: String,
    pub message: String,
    pub level: AggregatedMessageLevel,
    pub source: String,
    pub read: bool,
    pub created_at: i64,
}

/// Message priority levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregatedMessageLevel {
    Info,
    Notice,
    Important,
    Urgent,
}

/// Aggregated message tool.
pub struct MessageTool {
    message_manager: Option<Arc<neomind_messages::MessageManager>>,
    messages: Arc<tokio::sync::RwLock<Vec<AggregatedMessageInfo>>>,
}

impl MessageTool {
    /// Create a new message tool.
    pub fn new() -> Self {
        Self {
            message_manager: None,
            messages: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Create with message manager for persistent storage.
    pub fn with_message_manager(message_manager: Arc<neomind_messages::MessageManager>) -> Self {
        Self {
            message_manager: Some(message_manager),
            messages: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

impl Default for MessageTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MessageTool {
    fn name(&self) -> &str {
        "message"
    }

    fn description(&self) -> &str {
        r#"Message tool for viewing and managing system messages and notifications.

Actions:
- list: List messages with optional level and read/unread filters. Use when user asks about messages or notifications.
- get: Get a single message by ID. Use when you need full details of a specific message.
- send: Send a new message/notification. Use when user wants to notify someone or when an agent needs to report something.
- read: Mark a message as read. Use when user confirms they've seen a message.

Priority levels:
- info: General information, no action needed (e.g., 'Device came online')
- notice: Worth noting (e.g., 'Battery below 30%')
- important: Needs attention (e.g., 'Device communication failed')
- urgent: Immediate action required (e.g., 'Temperature exceeds safety limit')

Tips:
- Use unread_only=true to see only unread messages
- Filter by level to prioritize urgent messages"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "send", "read"],
                    "description": "Operation type: 'list' (view messages), 'get' (single message by ID), 'send' (new message), 'read' (mark as read)"
                },
                "message_id": {
                    "type": "string",
                    "description": "Message ID or title. Supports fuzzy matching on title. Use list action to discover messages"
                },
                "title": {
                    "type": "string",
                    "description": "Message title (send action). Short summary. Example: 'Device Offline', 'Battery Low'"
                },
                "message": {
                    "type": "string",
                    "description": "Message body (send action). Detailed description. Example: 'Living room sensor reports 35.2C, threshold is 30C'"
                },
                "level": {
                    "type": "string",
                    "description": "Priority level (send action): 'info', 'notice', 'important', 'urgent'. Default: 'notice'"
                },
                "source": {
                    "type": "string",
                    "description": "Message source identifier (send action). Default: 'system'. Example: 'temperature_monitor', 'security_agent'"
                },
                "unread_only": {
                    "type": "boolean",
                    "description": "Only return unread messages (list action). Default: false"
                },
                "level_filter": {
                    "type": "string",
                    "description": "Filter by priority level (list action): 'info', 'notice', 'important', 'urgent'"
                },
                "limit": {
                    "type": "number",
                    "description": "Max messages to return (list action). Default: 50"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': title/level/status only (default). 'detailed': full message info with timestamps"
                }
            }),
            vec!["action".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "list" => self.execute_list(&args).await,
            "get" => self.execute_get(&args).await,
            "send" | "create" => self.execute_send(&args).await,
            "read" | "acknowledge" => self.execute_read(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: '{}'. Valid actions: list, get, send, read",
                action
            ))),
        }
    }
}

impl MessageTool {
    fn is_detailed(args: &Value) -> bool {
        args["response_format"].as_str() == Some("detailed")
    }

    /// Map user-facing level string to internal MessageSeverity.
    fn level_to_severity(level: &str) -> neomind_messages::MessageSeverity {
        use neomind_messages::MessageSeverity;
        match level {
            "info" => MessageSeverity::Info,
            "notice" | "warning" => MessageSeverity::Warning,
            "important" | "error" => MessageSeverity::Critical,
            "urgent" | "critical" | "emergency" => MessageSeverity::Emergency,
            _ => MessageSeverity::Warning,
        }
    }

    /// Map internal MessageSeverity to user-facing level string.
    fn severity_to_level(severity: neomind_messages::MessageSeverity) -> &'static str {
        use neomind_messages::MessageSeverity;
        match severity {
            MessageSeverity::Info => "info",
            MessageSeverity::Warning => "notice",
            MessageSeverity::Critical => "important",
            MessageSeverity::Emergency => "urgent",
        }
    }

    /// Map user-facing level string to AggregatedMessageLevel (in-memory path).
    fn level_to_enum(level: &str) -> AggregatedMessageLevel {
        match level {
            "info" => AggregatedMessageLevel::Info,
            "notice" | "warning" => AggregatedMessageLevel::Notice,
            "important" | "error" => AggregatedMessageLevel::Important,
            "urgent" | "critical" => AggregatedMessageLevel::Urgent,
            _ => AggregatedMessageLevel::Notice,
        }
    }

    /// Map AggregatedMessageLevel to display string.
    fn enum_to_level(level: &AggregatedMessageLevel) -> &'static str {
        match level {
            AggregatedMessageLevel::Info => "info",
            AggregatedMessageLevel::Notice => "notice",
            AggregatedMessageLevel::Important => "important",
            AggregatedMessageLevel::Urgent => "urgent",
        }
    }

    /// Check if a user-facing level matches an internal MessageSeverity.
    fn level_matches(level: &str, severity: &neomind_messages::MessageSeverity) -> bool {
        Self::severity_to_level(*severity) == level
    }

    /// Check if a user-facing level matches an AggregatedMessageLevel.
    fn level_matches_enum(level: &str, msg_level: &AggregatedMessageLevel) -> bool {
        Self::enum_to_level(msg_level) == level
    }

    /// Resolve message_id with fuzzy matching using EntityResolver.
    async fn resolve_message_id(&self, input: &str) -> Result<String> {
        let messages = self.messages.read().await;

        // Fast path: exact ID match
        let exact_match = messages.iter().find(|m| m.id == input);
        if let Some(msg) = exact_match {
            return Ok(msg.id.clone());
        }

        // Fuzzy match by title
        let candidates: Vec<(String, String)> = messages
            .iter()
            .map(|m| (m.id.clone(), m.title.clone()))
            .collect();

        super::resolver::EntityResolver::resolve(input, &candidates, "消息")
            .map_err(ToolError::Execution)
    }

    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let unread_only = args["unread_only"].as_bool()
            .or_else(|| args["unacknowledged_only"].as_bool())
            .unwrap_or(false);
        let level_filter = args["level_filter"].as_str()
            .or_else(|| args["severity_filter"].as_str());
        let limit = args["limit"].as_u64().unwrap_or(50) as usize;
        let detailed = Self::is_detailed(args);

        // Use message_manager if available, otherwise use in-memory storage
        if let Some(manager) = &self.message_manager {
            use neomind_messages::MessageType;

            let msgs = manager.list_active_messages().await;

            let filtered: Vec<Value> = msgs
                .into_iter()
                .filter(|m| m.message_type == MessageType::Notification)
                .filter(|m| {
                    if unread_only {
                        m.is_active()
                    } else {
                        true
                    }
                })
                .filter(|m| {
                    if let Some(lvl) = level_filter {
                        Self::level_matches(lvl, &m.severity)
                    } else {
                        true
                    }
                })
                .take(limit)
                .map(|m| {
                    let level_str = Self::severity_to_level(m.severity);
                    if detailed {
                        serde_json::json!({
                            "id": m.id.to_string(),
                            "title": m.title,
                            "message": m.message,
                            "level": level_str,
                            "source": m.source_type,
                            "read": !m.is_active(),
                            "created_at": m.timestamp.timestamp()
                        })
                    } else {
                        serde_json::json!({
                            "id": m.id.to_string(),
                            "title": m.title,
                            "level": level_str,
                            "read": !m.is_active()
                        })
                    }
                })
                .collect();

            Ok(ToolOutput::success(serde_json::json!({
                "count": filtered.len(),
                "messages": filtered
            })))
        } else {
            let msgs = self.messages.read().await;

            let filtered: Vec<Value> = msgs
                .iter()
                .filter(|m| {
                    if unread_only {
                        !m.read
                    } else {
                        true
                    }
                })
                .filter(|m| {
                    if let Some(lvl) = level_filter {
                        Self::level_matches_enum(lvl, &m.level)
                    } else {
                        true
                    }
                })
                .take(limit)
                .map(|m| {
                    if detailed {
                        serde_json::to_value(m).unwrap()
                    } else {
                        serde_json::json!({
                            "id": m.id,
                            "title": m.title,
                            "level": Self::enum_to_level(&m.level),
                            "read": m.read
                        })
                    }
                })
                .collect();

            Ok(ToolOutput::success(serde_json::json!({
                "count": filtered.len(),
                "messages": filtered
            })))
        }
    }

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let id_str = args["message_id"]
            .as_str()
            .or_else(|| args["alert_id"].as_str())
            .ok_or_else(|| ToolError::InvalidArguments("message_id is required".into()))?;

        let resolved_id = self.resolve_message_id(id_str).await?;

        // Use message_manager if available
        if let Some(manager) = &self.message_manager {
            use neomind_messages::MessageId;

            let msg_id = MessageId::from_string(&resolved_id)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid message_id: {}", e)))?;

            let msg = manager
                .get_message(&msg_id)
                .await
                .ok_or_else(|| ToolError::Execution("Message not found".into()))?;

            let level_str = Self::severity_to_level(msg.severity);

            Ok(ToolOutput::success(serde_json::json!({
                "id": msg.id.to_string(),
                "title": msg.title,
                "message": msg.message,
                "level": level_str,
                "source": msg.source_type,
                "read": !msg.is_active(),
                "created_at": msg.timestamp.timestamp()
            })))
        } else {
            let msgs = self.messages.read().await;
            let msg = msgs
                .iter()
                .find(|m| m.id == resolved_id)
                .ok_or_else(|| ToolError::Execution("Message not found".into()))?;

            Ok(ToolOutput::success(serde_json::json!({
                "id": msg.id,
                "title": msg.title,
                "message": msg.message,
                "level": Self::enum_to_level(&msg.level),
                "source": msg.source,
                "read": msg.read,
                "created_at": msg.created_at
            })))
        }
    }

    async fn execute_send(&self, args: &Value) -> Result<ToolOutput> {
        let title = args["title"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("title is required".into()))?;

        let message = args["message"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("message is required".into()))?;

        let level_str = args["level"].as_str()
            .or_else(|| args["severity"].as_str())
            .unwrap_or("notice");
        let source = args["source"].as_str().unwrap_or("system");

        // Use message_manager if available
        if let Some(manager) = &self.message_manager {
            use neomind_messages::Message;

            let severity = Self::level_to_severity(level_str);

            let msg = Message::new(
                "message",
                severity,
                title.to_string(),
                message.to_string(),
                source.to_string(),
            );

            let msg = manager
                .create_message(msg)
                .await
                .map_err(|e| ToolError::Execution(e.to_string()))?;

            Ok(ToolOutput::success(serde_json::json!({
                "id": msg.id.to_string(),
                "status": "sent"
            })))
        } else {
            let level = Self::level_to_enum(level_str);

            let msg = AggregatedMessageInfo {
                id: uuid::Uuid::new_v4().to_string(),
                title: title.to_string(),
                message: message.to_string(),
                level,
                source: source.to_string(),
                read: false,
                created_at: chrono::Utc::now().timestamp(),
            };

            let id = msg.id.clone();
            self.messages.write().await.push(msg);

            Ok(ToolOutput::success(serde_json::json!({
                "id": id,
                "status": "sent"
            })))
        }
    }

    async fn execute_read(&self, args: &Value) -> Result<ToolOutput> {
        let id_str = args["message_id"]
            .as_str()
            .or_else(|| args["alert_id"].as_str())
            .ok_or_else(|| ToolError::InvalidArguments("message_id is required".into()))?;

        let resolved_id = self.resolve_message_id(id_str).await?;

        // Use message_manager if available
        if let Some(manager) = &self.message_manager {
            use neomind_messages::MessageId;

            let msg_id = MessageId::from_string(&resolved_id)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid message_id: {}", e)))?;

            manager
                .acknowledge(&msg_id)
                .await
                .map_err(|e| ToolError::Execution(e.to_string()))?;

            Ok(ToolOutput::success(serde_json::json!({
                "id": resolved_id,
                "status": "read"
            })))
        } else {
            let mut msgs = self.messages.write().await;
            let msg = msgs
                .iter_mut()
                .find(|m| m.id == resolved_id)
                .ok_or_else(|| ToolError::Execution("Message not found".into()))?;

            msg.read = true;

            Ok(ToolOutput::success(serde_json::json!({
                "id": resolved_id,
                "status": "read"
            })))
        }
    }
}

// ============================================================================
// Extension Tool - Aggregates: list, get, execute, status
// ============================================================================

/// Aggregated extension tool with action-based routing.
///
/// Provides a unified entry point for interacting with all installed extensions,
/// replacing per-command tool registration.
pub struct ExtensionAggregatedTool {
    registry: Arc<neomind_core::extension::registry::ExtensionRegistry>,
}

impl ExtensionAggregatedTool {
    /// Create a new extension aggregated tool.
    pub fn new(registry: Arc<neomind_core::extension::registry::ExtensionRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for ExtensionAggregatedTool {
    fn name(&self) -> &str {
        "extension"
    }

    fn description(&self) -> &str {
        r#"Extension management tool for interacting with installed extensions (plugins).

Actions:
- list: List all installed extensions with their status and command count. Use when user asks what extensions or plugins are available.
- get: Get detailed info about a specific extension, including its commands and metrics. Use before executing a command.
- status: Check the health and runtime status of an extension.

To execute extension commands, first use list/get to discover available extensions and commands, then call them directly using the format: extension-id:command

Tips:
- Always call list first if you're unsure which extensions are available
- Use get to discover available commands and their parameters, then call them directly with extension-id:command format"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "status"],
                    "description": "Operation type: 'list' (list extensions), 'get' (extension details), 'status' (health check)"
                },
                "extension_id": {
                    "type": "string",
                    "description": "Extension ID or name. Supports fuzzy matching. Use list action to discover available extensions"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': summary only (default). 'detailed': full info with all metadata"
                }
            }),
            vec!["action".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "list" => self.execute_list(&args).await,
            "get" => self.execute_get(&args).await,
            "status" => self.execute_status(&args).await,
            // Backward compat: execute still works but delegates to direct command execution
            "execute" => self.execute_command_compat(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: '{}'. Valid actions: list, get, status. To execute extension commands, call them directly: extension-id:command",
                action
            ))),
        }
    }
}

impl ExtensionAggregatedTool {
    fn is_detailed(args: &Value) -> bool {
        args["response_format"].as_str() == Some("detailed")
    }

    /// Resolve extension_id with fuzzy matching using EntityResolver.
    async fn resolve_extension_id(&self, input: &str) -> Result<String> {
        // Fast path: exact ID match
        if self.registry.get_info(input).await.is_some() {
            return Ok(input.to_string());
        }

        // Fuzzy match by name
        let extensions = self.registry.list().await;
        let candidates: Vec<(String, String)> = extensions
            .iter()
            .map(|e| (e.metadata.id.clone(), e.metadata.name.clone()))
            .collect();

        super::resolver::EntityResolver::resolve(input, &candidates, "扩展")
            .map_err(ToolError::Execution)
    }

    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let detailed = Self::is_detailed(args);
        let extensions = self.registry.list().await;

        let items: Vec<Value> = extensions
            .iter()
            .map(|info| {
                if detailed {
                    serde_json::json!({
                        "id": info.metadata.id,
                        "name": info.metadata.name,
                        "version": info.metadata.version,
                        "description": info.metadata.description,
                        "state": format!("{:?}", info.state),
                        "commands_count": info.commands.len(),
                        "metrics_count": info.metrics.len()
                    })
                } else {
                    serde_json::json!({
                        "id": info.metadata.id,
                        "name": info.metadata.name,
                        "state": format!("{:?}", info.state),
                        "commands": info.commands.len()
                    })
                }
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": items.len(),
            "extensions": items
        })))
    }

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let raw_id = args["extension_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("extension_id is required".into()))?;
        let extension_id = self.resolve_extension_id(raw_id).await?;

        let info = self
            .registry
            .get_info(&extension_id)
            .await
            .ok_or_else(|| ToolError::Execution(format!("Extension '{}' not found", extension_id)))?;

        let commands: Vec<Value> = info
            .commands
            .iter()
            .map(|cmd| {
                serde_json::json!({
                    "name": cmd.name,
                    "display_name": cmd.display_name,
                    "description": cmd.description,
                    "params": cmd.parameters
                })
            })
            .collect();

        let metrics: Vec<Value> = info
            .metrics
            .iter()
            .map(|m| {
                serde_json::json!({
                    "name": m.name,
                    "display_name": m.display_name,
                    "unit": m.unit
                })
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "id": info.metadata.id,
            "name": info.metadata.name,
            "version": info.metadata.version,
            "description": info.metadata.description,
            "state": format!("{:?}", info.state),
            "commands": commands,
            "metrics": metrics
        })))
    }

    async fn execute_status(&self, args: &Value) -> Result<ToolOutput> {
        let raw_id = args["extension_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("extension_id is required".into()))?;
        let extension_id = self.resolve_extension_id(raw_id).await?;

        let info = self
            .registry
            .get_info(&extension_id)
            .await
            .ok_or_else(|| ToolError::Execution(format!("Extension '{}' not found", extension_id)))?;

        let healthy = self
            .registry
            .health_check(&extension_id)
            .await
            .unwrap_or(false);

        Ok(ToolOutput::success(serde_json::json!({
            "id": info.metadata.id,
            "name": info.metadata.name,
            "state": format!("{:?}", info.state),
            "healthy": healthy,
            "commands_executed": info.stats.commands_executed,
            "error_count": info.stats.error_count
        })))
    }

    /// Backward-compatible execute action for legacy callers.
    /// Delegates to the registry's execute_command directly.
    async fn execute_command_compat(&self, args: &Value) -> Result<ToolOutput> {
        let raw_id = args["extension_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("extension_id is required".into()))?;
        let extension_id = self.resolve_extension_id(raw_id).await?;
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("command is required".into()))?;
        let params = args.get("params").cloned().unwrap_or(serde_json::json!({}));

        let result = self
            .registry
            .execute_command(&extension_id, command, &params)
            .await
            .map_err(|e| ToolError::Execution(format!("Extension command failed: {}", e)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "extension_id": extension_id,
            "command": command,
            "result": result
        })))
    }
}

// ============================================================================
// Builder for Aggregated Tools
// ============================================================================

/// Builder for creating all aggregated tools with dependencies.
pub struct AggregatedToolsBuilder {
    device_service: Option<Arc<neomind_devices::DeviceService>>,
    time_series_storage: Option<Arc<neomind_devices::TimeSeriesStorage>>,
    agent_store: Option<Arc<neomind_storage::AgentStore>>,
    rule_engine: Option<Arc<neomind_rules::RuleEngine>>,
    rule_history: Option<Arc<neomind_rules::RuleHistoryStorage>>,
    message_manager: Option<Arc<neomind_messages::MessageManager>>,
    extension_registry: Option<Arc<neomind_core::extension::registry::ExtensionRegistry>>,
    session_store: Option<Arc<neomind_storage::SessionStore>>,
}

impl AggregatedToolsBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            device_service: None,
            time_series_storage: None,
            agent_store: None,
            rule_engine: None,
            rule_history: None,
            message_manager: None,
            extension_registry: None,
            session_store: None,
        }
    }

    /// Set device service.
    pub fn with_device_service(mut self, service: Arc<neomind_devices::DeviceService>) -> Self {
        self.device_service = Some(service);
        self
    }

    /// Set time series storage.
    pub fn with_time_series_storage(
        mut self,
        storage: Arc<neomind_devices::TimeSeriesStorage>,
    ) -> Self {
        self.time_series_storage = Some(storage);
        self
    }

    /// Set agent store.
    pub fn with_agent_store(mut self, store: Arc<neomind_storage::AgentStore>) -> Self {
        self.agent_store = Some(store);
        self
    }

    /// Set rule engine.
    pub fn with_rule_engine(mut self, engine: Arc<neomind_rules::RuleEngine>) -> Self {
        self.rule_engine = Some(engine);
        self
    }

    /// Set rule history storage.
    pub fn with_rule_history(mut self, history: Arc<neomind_rules::RuleHistoryStorage>) -> Self {
        self.rule_history = Some(history);
        self
    }

    /// Set message manager for message tool persistence.
    pub fn with_message_manager(mut self, manager: Arc<neomind_messages::MessageManager>) -> Self {
        self.message_manager = Some(manager);
        self
    }

    /// Set extension registry for the extension aggregated tool.
    pub fn with_extension_registry(
        mut self,
        registry: Arc<neomind_core::extension::registry::ExtensionRegistry>,
    ) -> Self {
        self.extension_registry = Some(registry);
        self
    }

    /// Set session store for the session search tool.
    pub fn with_session_store(mut self, store: Arc<neomind_storage::SessionStore>) -> Self {
        self.session_store = Some(store);
        self
    }

    /// Build all aggregated tools as a vector of DynTool.
    pub fn build(self) -> Vec<Arc<dyn Tool>> {
        let mut tools: Vec<Arc<dyn Tool>> = Vec::new();

        // Device tool
        if let Some(ds) = self.device_service {
            let device_tool = if let Some(storage) = self.time_series_storage {
                DeviceTool::with_storage(ds, storage)
            } else {
                DeviceTool::new(ds)
            };
            tools.push(Arc::new(device_tool));
        }

        // Agent tool (includes history actions: executions, conversation, latest_execution)
        if let Some(store) = self.agent_store.clone() {
            tools.push(Arc::new(AgentTool::new(store)));
        }

        // Rule tool
        if let Some(engine) = self.rule_engine {
            let rule_tool = if let Some(history) = self.rule_history {
                RuleTool::with_history(engine, history)
            } else {
                RuleTool::new(engine)
            };
            tools.push(Arc::new(rule_tool));
        }

        // Message tool (always available)
        let message_tool = if let Some(manager) = self.message_manager {
            MessageTool::with_message_manager(manager)
        } else {
            MessageTool::new()
        };
        tools.push(Arc::new(message_tool));

        // Extension tool
        if let Some(ext_reg) = self.extension_registry {
            tools.push(Arc::new(ExtensionAggregatedTool::new(ext_reg)));
        }

        // Session search tool
        if let Some(session_store) = self.session_store {
            tools.push(Arc::new(super::SessionSearchTool::new(session_store)));
        }

        tools
    }
}

impl Default for AggregatedToolsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregated_tools_builder_creates_tools() {
        // Test that builder creates tools even without dependencies
        let tools = AggregatedToolsBuilder::new().build();
        // Without dependencies, only MessageTool is created
        assert_eq!(tools.len(), 1);
    }

    #[test]
    fn test_message_tool_name() {
        // Test MessageTool metadata
        let tool = MessageTool::new();
        assert_eq!(tool.name(), "message");
    }

    #[tokio::test]
    async fn test_message_tool_list_empty() {
        // Test listing messages when none exist
        let tool = MessageTool::new();
        let result = tool
            .execute(serde_json::json!({"action": "list"}))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.data["count"].as_u64().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_message_tool_send_and_list() {
        // Test sending and listing messages
        let tool = MessageTool::new();

        // Send a message
        let send_result = tool
            .execute(serde_json::json!({
                "action": "send",
                "title": "Test Message",
                "message": "This is a test",
                "level": "notice"
            }))
            .await
            .unwrap();

        assert!(send_result.success);
        let msg_id = send_result.data["id"].as_str().unwrap().to_string();

        // List messages
        let list_result = tool
            .execute(serde_json::json!({"action": "list"}))
            .await
            .unwrap();

        assert!(list_result.success);
        assert_eq!(list_result.data["count"].as_u64().unwrap(), 1);

        // Read the message
        let read_result = tool
            .execute(serde_json::json!({
                "action": "read",
                "message_id": msg_id
            }))
            .await
            .unwrap();

        assert!(read_result.success);
    }

    #[tokio::test]
    async fn test_message_tool_unknown_action() {
        let tool = MessageTool::new();

        let result = tool
            .execute(serde_json::json!({"action": "unknown_action"}))
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_aggregated_message_level_variants() {
        // Test that AggregatedMessageLevel has all expected variants
        let info = AggregatedMessageLevel::Info;
        let notice = AggregatedMessageLevel::Notice;
        let important = AggregatedMessageLevel::Important;
        let urgent = AggregatedMessageLevel::Urgent;

        // Verify Debug trait is implemented
        assert!(format!("{:?}", info).contains("Info"));
        assert!(format!("{:?}", notice).contains("Notice"));
        assert!(format!("{:?}", important).contains("Important"));
        assert!(format!("{:?}", urgent).contains("Urgent"));
    }

    #[test]
    fn test_aggregated_message_info_serialization() {
        // Test that AggregatedMessageInfo can be serialized
        let msg = AggregatedMessageInfo {
            id: "test-id".to_string(),
            title: "Test Message".to_string(),
            message: "Test body".to_string(),
            level: AggregatedMessageLevel::Notice,
            source: "test".to_string(),
            read: false,
            created_at: 1234567890,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("Test Message"));
    }
}
