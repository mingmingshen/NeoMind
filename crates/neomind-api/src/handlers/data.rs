//! Unified Data Source API handlers.
//!
//! Provides a unified endpoint to browse all data sources across the system:
//! - Device metrics
//! - Extension data sources
//! - Transform outputs
//! - System metrics (future)

use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::handlers::common::HandlerResult;
use crate::server::types::ServerState;
use neomind_core::datasource::DataSourceId;

/// Unified data source information.
/// Aggregates all data sources from devices, extensions, and transforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedDataSourceInfo {
    /// Unique identifier: "{type}:{source}:{field}"
    /// Examples: "device:sensor1:temperature", "extension:weather:temp", "transform:temp_converter:temp_f"
    pub id: String,
    /// Source type: "device" | "extension" | "transform" | "system"
    pub source_type: String,
    /// Source name (device_id, extension_id, transform_id, or "system")
    pub source_name: String,
    /// Display name of the source (device name, extension name, transform name)
    pub source_display_name: String,
    /// Field/metric name
    pub field: String,
    /// Display name of the field
    pub field_display_name: String,
    /// Data type: "float" | "integer" | "boolean" | "string"
    pub data_type: String,
    /// Unit of measurement
    pub unit: Option<String>,
    /// Human-readable description
    pub description: Option<String>,
    /// Current value (if available)
    pub current_value: Option<serde_json::Value>,
    /// Last update timestamp (Unix milliseconds)
    pub last_update: Option<i64>,
    /// Data quality score (0.0 - 1.0)
    pub quality: Option<f32>,
}

/// Query parameters for listing data sources
#[derive(Debug, Deserialize)]
pub struct ListDataSourcesQuery {
    /// Filter by source type
    pub source_type: Option<String>,
    /// Filter by source name
    pub source: Option<String>,
    /// Search term for filtering
    pub search: Option<String>,
}

/// GET /api/data/sources
///
/// List all data sources across devices, extensions, and transforms.
pub async fn list_all_data_sources_handler(
    State(state): State<ServerState>,
) -> HandlerResult<Vec<UnifiedDataSourceInfo>> {
    let mut sources = Vec::new();

    // 1. Collect device metrics
    collect_device_sources(&state, &mut sources).await;

    // 2. Collect extension data sources
    collect_extension_sources(&state, &mut sources).await;

    // 3. Collect transform data sources
    collect_transform_sources(&state, &mut sources).await;

    // 4. Collect AI agent metrics
    collect_ai_sources(&state, &mut sources).await;

    // 5. Populate latest telemetry values
    populate_latest_values(&state, &mut sources).await;

    // Sort by id for consistent ordering
    sources.sort_by(|a, b| a.id.cmp(&b.id));

    crate::handlers::common::ok(sources)
}

/// Fetch latest telemetry values for all collected data sources.
async fn populate_latest_values(state: &ServerState, sources: &mut Vec<UnifiedDataSourceInfo>) {
    let telemetry = &state.devices.telemetry;

    for source in sources.iter_mut() {
        let ds_id = match DataSourceId::parse(&source.id) {
            Some(id) => id,
            None => continue,
        };

        let device_part = ds_id.device_part();
        let metric_part = ds_id.metric_part();

        if let Ok(Some(data_point)) = telemetry.latest(&device_part, metric_part).await {
            source.last_update = Some(data_point.timestamp);
            source.quality = data_point.quality;

            let value = match &data_point.value {
                neomind_devices::mdl::MetricValue::Float(v) => serde_json::json!(v),
                neomind_devices::mdl::MetricValue::Integer(v) => serde_json::json!(v),
                neomind_devices::mdl::MetricValue::Boolean(v) => serde_json::json!(v),
                neomind_devices::mdl::MetricValue::String(v) => serde_json::json!(v),
                neomind_devices::mdl::MetricValue::Array(v) => serde_json::json!(v),
                neomind_devices::mdl::MetricValue::Binary(_) => serde_json::json!("<binary>"),
                neomind_devices::mdl::MetricValue::Null => serde_json::Value::Null,
            };
            source.current_value = Some(value);
        }
    }
}

/// Collect data sources from all registered devices.
/// Includes both template-defined metrics and virtual metrics from telemetry storage.
async fn collect_device_sources(state: &ServerState, sources: &mut Vec<UnifiedDataSourceInfo>) {
    let devices = state.devices.service.list_devices().await;

    for device in devices {
        let mut known_metrics: std::collections::HashSet<String> = std::collections::HashSet::new();

        // 1. Add template-defined metrics
        let template = state
            .devices
            .registry
            .get_template(&device.device_type)
            .await;

        if let Some(template) = template {
            for metric in &template.metrics {
                known_metrics.insert(metric.name.clone());

                let source_id = DataSourceId::device(&device.device_id, &metric.name);

                let unit = if metric.unit.is_empty() {
                    None
                } else {
                    Some(metric.unit.clone())
                };

                sources.push(UnifiedDataSourceInfo {
                    id: source_id.storage_key(),
                    source_type: "device".to_string(),
                    source_name: device.device_id.clone(),
                    source_display_name: device.name.clone(),
                    field: metric.name.clone(),
                    field_display_name: if metric.display_name.is_empty() {
                        metric.name.clone()
                    } else {
                        metric.display_name.clone()
                    },
                    data_type: format!("{:?}", metric.data_type).to_lowercase(),
                    unit,
                    description: None,
                    current_value: None,
                    last_update: None,
                    quality: None,
                });
            }
        }

        // 2. Add virtual metrics from telemetry storage (metrics not in template)
        if let Ok(telemetry_metrics) = state
            .devices
            .telemetry
            .list_metrics(&device.device_id)
            .await
        {
            for metric_name in telemetry_metrics {
                if known_metrics.contains(&metric_name) {
                    continue; // Already added from template
                }

                let source_id = DataSourceId::device(&device.device_id, &metric_name);

                sources.push(UnifiedDataSourceInfo {
                    id: source_id.storage_key(),
                    source_type: "device".to_string(),
                    source_name: device.device_id.clone(),
                    source_display_name: device.name.clone(),
                    field: metric_name.clone(),
                    field_display_name: metric_name.clone(),
                    data_type: "unknown".to_string(),
                    unit: None,
                    description: Some("Virtual metric".to_string()),
                    current_value: None,
                    last_update: None,
                    quality: None,
                });
            }
        }
    }
}

/// Collect data sources from all registered extensions.
async fn collect_extension_sources(state: &ServerState, sources: &mut Vec<UnifiedDataSourceInfo>) {
    // Get all registered extensions
    let extensions = state.extensions.runtime.list().await;

    for ext_info in extensions {
        for metric in &ext_info.metrics {
            let source_id = DataSourceId::extension(&ext_info.metadata.id, &metric.name);

            // Convert unit from String to Option<String>
            let unit = if metric.unit.is_empty() {
                None
            } else {
                Some(metric.unit.clone())
            };

            sources.push(UnifiedDataSourceInfo {
                id: source_id.storage_key(),
                source_type: "extension".to_string(),
                source_name: ext_info.metadata.id.clone(),
                source_display_name: ext_info.metadata.name.clone(),
                field: metric.name.clone(),
                field_display_name: if metric.display_name.is_empty() {
                    metric.name.clone()
                } else {
                    metric.display_name.clone()
                },
                data_type: format!("{:?}", metric.data_type).to_lowercase(),
                unit,
                description: None,
                current_value: None,
                last_update: None,
                quality: None,
            });
        }
    }
}

/// Collect data sources from all registered transforms.
async fn collect_transform_sources(state: &ServerState, sources: &mut Vec<UnifiedDataSourceInfo>) {
    let Some(transform_engine) = &state.automation.transform_engine else {
        return;
    };

    let registry = transform_engine.output_registry();
    let transform_sources = registry.list_as_data_sources().await;

    for ts in transform_sources {
        sources.push(UnifiedDataSourceInfo {
            id: ts.id,
            source_type: "transform".to_string(),
            source_name: ts.transform_id,
            source_display_name: ts.transform_name,
            field: ts.metric_name,
            field_display_name: ts.display_name,
            data_type: ts.data_type,
            unit: ts.unit,
            description: Some(ts.description),
            current_value: None,
            last_update: ts.last_update,
            quality: None,
        });
    }
}

/// Collect data sources from AI agent metrics.
///
/// AI metrics are registered by agents via the `ai_metric` tool.
/// They are stored in telemetry with device_id = `"ai:{group}"` and metric = field name.
async fn collect_ai_sources(state: &ServerState, sources: &mut Vec<UnifiedDataSourceInfo>) {
    let registry = &state.agents.ai_metrics_registry;
    let keys = registry.all_keys();

    for (group, field) in keys {
        let source_id = DataSourceId::ai(&group, &field);
        let meta = registry.get(&group, &field);

        sources.push(UnifiedDataSourceInfo {
            id: source_id.storage_key(),
            source_type: "ai".to_string(),
            source_name: group.clone(),
            source_display_name: format!("AI {}", title_case(&group)),
            field: field.clone(),
            field_display_name: field.clone(),
            data_type: "unknown".to_string(), // will be inferred by populate_latest_values
            unit: meta.as_ref().and_then(|m| m.unit.clone()),
            description: meta.as_ref().and_then(|m| m.description.clone()),
            current_value: None,
            last_update: None,
            quality: None,
        });
    }
}

/// Convert a snake_case or kebab-case string to Title Case.
fn title_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in s.chars() {
        if c == '-' || c == '_' {
            result.push(' ');
            capitalize = true;
        } else if capitalize {
            result.extend(c.to_uppercase());
            capitalize = false;
        } else {
            result.push(c);
        }
    }
    result
}
