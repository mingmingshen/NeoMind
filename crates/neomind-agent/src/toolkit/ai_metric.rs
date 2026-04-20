//! AI Metric tool — allows agents to write and read custom metrics.
//!
//! Agents can create their own metrics (e.g. analysis scores, derived indicators)
//! which are persisted to the time-series store and discoverable via the registry.
//!
//! ## Storage convention
//!
//! - `device_id` in telemetry: `"ai:{group}"`
//! - metric name: the field name
//!
//! ## Actions
//!
//! - `write`: persist a data point and register its metadata
//! - `read`:  list all AI metrics with latest values, or query time-series data

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{Result, ToolError};
use super::tool::{object_schema, Tool, ToolOutput};
use neomind_core::tools::ToolCategory;

// ============================================================================
// AiMetricsRegistry — persisted metadata store
// ============================================================================

/// Metadata for an AI metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMetricMeta {
    pub unit: Option<String>,
    pub description: Option<String>,
}

const METADATA_FILE: &str = "ai_metrics_metadata.json";

/// Persisted registry for AI metric metadata.
///
/// Metadata is stored both in-memory (DashMap) for fast reads and persisted
/// to a JSON file (`{data_dir}/ai_metrics_metadata.json`) for survival across restarts.
/// Shared between AiMetricTool (writes metadata) and data handler (reads metadata).
pub struct AiMetricsRegistry {
    metrics: DashMap<(String, String), AiMetricMeta>,
    path: PathBuf,
}

impl AiMetricsRegistry {
    /// Create a new registry backed by a JSON file in `data_dir`.
    /// Loads existing metadata from disk if the file exists.
    pub fn new(data_dir: &Path) -> Arc<Self> {
        let path = data_dir.join(METADATA_FILE);
        let registry = Self {
            metrics: DashMap::new(),
            path,
        };
        registry.load_from_disk();
        Arc::new(registry)
    }

    /// Register (or update) metadata for a metric and persist to disk.
    pub fn register(&self, group: &str, field: &str, meta: AiMetricMeta) {
        self.metrics
            .insert((group.to_string(), field.to_string()), meta);
        self.save_to_disk();
    }

    pub fn get(&self, group: &str, field: &str) -> Option<AiMetricMeta> {
        self.metrics
            .get(&(group.to_string(), field.to_string()))
            .map(|v| v.value().clone())
    }

    pub fn all_keys(&self) -> Vec<(String, String)> {
        self.metrics.iter().map(|e| e.key().clone()).collect()
    }

    fn load_from_disk(&self) {
        if !self.path.exists() {
            return;
        }
        match std::fs::read_to_string(&self.path) {
            Ok(content) => {
                // Keys are stored as "group\0field" strings for JSON compatibility.
                let map: std::collections::HashMap<String, AiMetricMeta> =
                    match serde_json::from_str(&content) {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!("Failed to parse AI metrics metadata: {}", e);
                            return;
                        }
                    };
                for (k, v) in map {
                    let mut parts = k.splitn(2, '\0');
                    let group = parts.next().unwrap_or_default().to_string();
                    let field = parts.next().unwrap_or_default().to_string();
                    if !group.is_empty() && !field.is_empty() {
                        self.metrics.insert((group, field), v);
                    }
                }
                let count = self.metrics.len();
                tracing::info!(count, "Loaded AI metrics metadata from disk");
            }
            Err(e) => {
                tracing::warn!("Failed to read AI metrics metadata file: {}", e);
            }
        }
    }

    fn save_to_disk(&self) {
        // Use "group\0field" as string key since JSON requires string keys.
        let map: std::collections::HashMap<String, AiMetricMeta> = self
            .metrics
            .iter()
            .map(|e| {
                let (g, f) = e.key();
                (format!("{}\0{}", g, f), e.value().clone())
            })
            .collect();

        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match serde_json::to_string_pretty(&map) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&self.path, content) {
                    tracing::warn!("Failed to write AI metrics metadata: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to serialize AI metrics metadata: {}", e);
            }
        }
    }
}

// ============================================================================
// AiMetricTool — agent tool implementation
// ============================================================================

/// Tool that lets agents write and read custom AI-generated metrics.
pub struct AiMetricTool {
    storage: Arc<neomind_devices::TimeSeriesStorage>,
    registry: Arc<AiMetricsRegistry>,
}

impl AiMetricTool {
    pub fn new(
        storage: Arc<neomind_devices::TimeSeriesStorage>,
        registry: Arc<AiMetricsRegistry>,
    ) -> Self {
        Self { storage, registry }
    }

    /// Validate that a name contains only alphanumeric chars, hyphens, and underscores.
    fn validate_name(name: &str, label: &str) -> Result<()> {
        if name.is_empty() {
            return Err(ToolError::InvalidArguments(format!(
                "{} must not be empty",
                label
            )));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ToolError::InvalidArguments(format!(
                "{} must contain only alphanumeric characters, hyphens, and underscores",
                label
            )));
        }
        Ok(())
    }

    /// Convert a serde_json::Value to a MetricValue.
    fn json_to_metric_value(value: &Value) -> neomind_devices::mdl::MetricValue {
        match value {
            Value::Null => neomind_devices::mdl::MetricValue::Null,
            Value::Bool(b) => neomind_devices::mdl::MetricValue::Boolean(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    neomind_devices::mdl::MetricValue::Integer(i)
                } else {
                    neomind_devices::mdl::MetricValue::Float(n.as_f64().unwrap_or(0.0))
                }
            }
            Value::String(s) => neomind_devices::mdl::MetricValue::String(s.clone()),
            Value::Array(arr) => neomind_devices::mdl::MetricValue::Array(
                arr.iter()
                    .map(|v| Self::json_to_metric_value(v))
                    .collect(),
            ),
            other => neomind_devices::mdl::MetricValue::String(other.to_string()),
        }
    }

    // -- action handlers -------------------------------------------------------

    async fn execute_write(&self, args: &Value) -> Result<ToolOutput> {
        let group = args["group"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("group is required".into()))?;
        let field = args["field"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("field is required".into()))?;

        Self::validate_name(group, "group")?;
        Self::validate_name(field, "field")?;

        let json_value = args
            .get("value")
            .ok_or_else(|| ToolError::InvalidArguments("value is required".into()))?;
        let metric_value = Self::json_to_metric_value(json_value);

        // Use seconds-level timestamps to match the telemetry system convention.
        // Device telemetry uses `chrono::Utc::now().timestamp()` (seconds) everywhere,
        // and the API handler queries with second-based ranges.
        let timestamp = args["timestamp"]
            .as_i64()
            .map(|ts| if ts > 1e12 as i64 { ts / 1000 } else { ts }) // normalize ms → s
            .unwrap_or_else(|| chrono::Utc::now().timestamp());

        let point = neomind_devices::DataPoint {
            timestamp,
            value: metric_value,
            quality: Some(1.0),
        };

        let device_id = format!("ai:{}", group);
        self.storage
            .write(&device_id, field, point)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to write AI metric: {}", e)))?;

        // Always register metadata so the metric is discoverable via read_list.
        // Merge with existing metadata: keep unit/description if already set and not overridden.
        let existing = self.registry.get(group, field);
        let existing_unit = existing.as_ref().and_then(|m| m.unit.clone());
        let existing_desc = existing.and_then(|m| m.description);
        let meta = AiMetricMeta {
            unit: args["unit"]
                .as_str()
                .map(|s| s.to_string())
                .or(existing_unit),
            description: args["description"]
                .as_str()
                .map(|s| s.to_string())
                .or(existing_desc),
        };
        self.registry.register(group, field, meta);

        Ok(ToolOutput::success(serde_json::json!({
            "device_id": device_id,
            "metric": field,
            "status": "written"
        })))
    }

    async fn execute_read(&self, args: &Value) -> Result<ToolOutput> {
        let query = args["query"]
            .as_str()
            .unwrap_or("list")
            .to_lowercase();

        match query.as_str() {
            "list" => self.read_list().await,
            "data" => self.read_data(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown query type: '{}'. Valid: list, data",
                query
            ))),
        }
    }

    /// List all registered AI metrics with their latest values.
    async fn read_list(&self) -> Result<ToolOutput> {
        let keys = self.registry.all_keys();
        let mut metrics = Vec::new();

        for (group, field) in keys {
            let device_id = format!("ai:{}", group);
            let meta = self.registry.get(&group, &field);

            let mut entry = serde_json::json!({
                "group": group,
                "field": field,
                "source_id": format!("ai:{}:{}", group, field),
            });

            if let Some(meta) = &meta {
                if let Some(unit) = &meta.unit {
                    entry["unit"] = Value::String(unit.clone());
                }
                if let Some(desc) = &meta.description {
                    entry["description"] = Value::String(desc.clone());
                }
            }

            // Try to fetch latest value
            match self.storage.latest(&device_id, &field).await {
                Ok(Some(dp)) => {
                    entry["value"] = dp.value.to_json_value();
                    entry["timestamp"] = serde_json::json!(dp.timestamp);
                }
                Ok(None) => {
                    entry["value"] = Value::Null;
                }
                Err(e) => {
                    tracing::debug!("Failed to fetch latest for {}/{}: {}", device_id, field, e);
                    entry["value"] = Value::Null;
                }
            }

            metrics.push(entry);
        }

        Ok(ToolOutput::success(serde_json::json!({
            "metrics": metrics,
            "count": metrics.len()
        })))
    }

    /// Query time-series data for a specific AI metric.
    async fn read_data(&self, args: &Value) -> Result<ToolOutput> {
        let group = args["group"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("group is required for data query".into()))?;
        let field = args["field"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("field is required for data query".into()))?;

        // Use seconds-level timestamps to match telemetry system convention.
        let now = chrono::Utc::now().timestamp();
        let start = args["start_time"].as_i64().map(|ts| if ts > 1e12 as i64 { ts / 1000 } else { ts }).unwrap_or(now - 86400); // default: 24h ago
        let end = args["end_time"].as_i64().map(|ts| if ts > 1e12 as i64 { ts / 1000 } else { ts }).unwrap_or(now);

        let device_id = format!("ai:{}", group);
        let points = self
            .storage
            .query(&device_id, field, start, end)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to query AI metric: {}", e)))?;

        let data: Vec<Value> = points
            .iter()
            .map(|dp| {
                serde_json::json!({
                    "timestamp": dp.timestamp,
                    "value": dp.value.to_json_value(),
                })
            })
            .collect();

        let meta = self.registry.get(group, field);

        let mut result = serde_json::json!({
            "group": group,
            "field": field,
            "source_id": format!("ai:{}:{}", group, field),
            "data": data,
            "count": data.len(),
        });

        if let Some(meta) = meta {
            if let Some(unit) = meta.unit {
                result["unit"] = Value::String(unit);
            }
            if let Some(desc) = meta.description {
                result["description"] = Value::String(desc);
            }
        }

        Ok(ToolOutput::success(result))
    }
}

#[async_trait]
impl Tool for AiMetricTool {
    fn name(&self) -> &str {
        "ai_metric"
    }

    fn description(&self) -> &str {
        r#"Create and query custom time-series metrics that appear in the Data Explorer.

WRITE a metric (action="write"):
  Required: group, field, value. The value must be a number, boolean, or string.
  Optional: unit, description, timestamp (defaults to current time).
  Each call appends a new data point. Use the same group+field to update the same metric.

  Example — write an anomaly score:
    {"action":"write", "group":"anomaly", "field":"score", "value": 0.85, "unit":"0-1", "description":"Anomaly score"}
  Example — write a temperature prediction:
    {"action":"write", "group":"prediction", "field":"temperature", "value": 23.5, "unit":"celsius"}

READ metrics (action="read"):
  query="list": Returns all AI metrics with their latest values.
    {"action":"read", "query":"list"}
  query="data": Returns time-series data for one metric (default: last 24 hours).
    {"action":"read", "query":"data", "group":"anomaly", "field":"score"}

Common use cases: analysis scores, predictions, derived indicators, computed statistics."#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["write", "read"],
                    "description": "'write' to record a metric value, 'read' to query existing metrics"
                },
                "group": {
                    "type": "string",
                    "description": "Logical grouping for the metric, e.g. 'anomaly', 'prediction', 'system'"
                },
                "field": {
                    "type": "string",
                    "description": "Metric name within the group, e.g. 'score', 'temperature', 'cpu_usage'"
                },
                "value": {
                    "description": "REQUIRED for write. The metric value — use a number (e.g. 0.85, 42), boolean, or string."
                },
                "unit": {
                    "type": "string",
                    "description": "Unit of the metric, e.g. 'celsius', 'percent', 'score' (write action, optional)"
                },
                "description": {
                    "type": "string",
                    "description": "Human-readable description of the metric (write action, optional)"
                },
                "timestamp": {
                    "type": "number",
                    "description": "Timestamp in milliseconds for the data point (write action, default: now)"
                },
                "query": {
                    "type": "string",
                    "enum": ["list", "data"],
                    "description": "Read query type: 'list' returns all AI metrics with latest values, 'data' returns time-series for a specific metric"
                },
                "start_time": {
                    "type": "number",
                    "description": "Start timestamp in milliseconds for data query (default: 24 hours ago)"
                },
                "end_time": {
                    "type": "number",
                    "description": "End timestamp in milliseconds for data query (default: now)"
                }
            }),
            vec!["action".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Device // reuse Device category since it's telemetry-related
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "write" => self.execute_write(&args).await,
            "read" => self.execute_read(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: '{}'. Valid actions: write, read",
                action
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_ok() {
        assert!(AiMetricTool::validate_name("temp-sensor_1", "group").is_ok());
        assert!(AiMetricTool::validate_name("abc123", "group").is_ok());
        assert!(AiMetricTool::validate_name("a_b-c", "group").is_ok());
    }

    #[test]
    fn test_validate_name_rejects_empty() {
        assert!(AiMetricTool::validate_name("", "group").is_err());
    }

    #[test]
    fn test_validate_name_rejects_special_chars() {
        assert!(AiMetricTool::validate_name("hello world", "group").is_err());
        assert!(AiMetricTool::validate_name("a.b", "group").is_err());
        assert!(AiMetricTool::validate_name("a/b", "group").is_err());
    }

    #[test]
    fn test_json_to_metric_value() {
        use neomind_devices::mdl::MetricValue;

        assert_eq!(
            AiMetricTool::json_to_metric_value(&serde_json::json!(42)),
            MetricValue::Integer(42)
        );
        assert_eq!(
            AiMetricTool::json_to_metric_value(&serde_json::json!(3.14)),
            MetricValue::Float(3.14)
        );
        assert_eq!(
            AiMetricTool::json_to_metric_value(&serde_json::json!("hello")),
            MetricValue::String("hello".into())
        );
        assert_eq!(
            AiMetricTool::json_to_metric_value(&serde_json::json!(true)),
            MetricValue::Boolean(true)
        );
        assert_eq!(
            AiMetricTool::json_to_metric_value(&serde_json::json!(null)),
            MetricValue::Null
        );
    }

    #[test]
    fn test_registry_basic() {
        let reg = AiMetricsRegistry::new(std::env::temp_dir().as_path());
        reg.register(
            "analysis",
            "score",
            AiMetricMeta {
                unit: Some("percent".into()),
                description: Some("Analysis score".into()),
            },
        );

        let meta = reg.get("analysis", "score").unwrap();
        assert_eq!(meta.unit.as_deref(), Some("percent"));
        assert_eq!(meta.description.as_deref(), Some("Analysis score"));

        let keys = reg.all_keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], ("analysis".into(), "score".into()));
    }

    #[test]
    fn test_registry_missing() {
        let reg = AiMetricsRegistry::new(std::env::temp_dir().as_path());
        assert!(reg.get("nope", "nope").is_none());
    }
}
