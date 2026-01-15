//! Device type management.

use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::json;

use edge_ai_devices::{DeviceTypeDefinition, registry::DeviceTypeTemplate};

use super::models::DeviceTypeDto;
use crate::handlers::{
    ServerState,
    common::{HandlerResult, ok},
};
use crate::models::ErrorResponse;

/// List device types.
/// Uses new DeviceService
pub async fn list_device_types_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let templates = state.device_service.list_templates().await;
    let dtos: Vec<DeviceTypeDto> = templates
        .into_iter()
        .map(|t| {
            let mode_str = match t.mode {
                edge_ai_devices::registry::DeviceTypeMode::Simple => "simple",
                edge_ai_devices::registry::DeviceTypeMode::Full => "full",
            };
            DeviceTypeDto {
                device_type: t.device_type.clone(),
                name: t.name.clone(),
                description: t.description.clone(),
                categories: t.categories.clone(),
                mode: mode_str.to_string(),
                metric_count: t.metrics.len(),
                command_count: t.commands.len(),
            }
        })
        .collect();

    ok(json!({
        "device_types": dtos,
        "count": dtos.len(),
    }))
}

/// Get device type details.
/// Uses new DeviceService - returns simplified format (direct metrics/commands)
pub async fn get_device_type_handler(
    State(state): State<ServerState>,
    Path(device_type): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let template = state
        .device_service
        .get_template(&device_type)
        .await
        .ok_or_else(|| ErrorResponse::not_found("DeviceType"))?;

    let mode_str = match template.mode {
        edge_ai_devices::registry::DeviceTypeMode::Simple => "simple",
        edge_ai_devices::registry::DeviceTypeMode::Full => "full",
    };

    // Return in new simplified format (direct metrics/commands arrays)
    ok(json!({
        "device_type": template.device_type,
        "name": template.name,
        "description": template.description,
        "categories": template.categories,
        "mode": mode_str,
        "metrics": template.metrics,
        "uplink_samples": template.uplink_samples,
        "commands": template.commands,
        "metric_count": template.metrics.len(),
        "command_count": template.commands.len(),
    }))
}

/// Register a new device type.
/// Uses new DeviceService - accepts simplified format (direct metrics/commands)
pub async fn register_device_type_handler(
    State(state): State<ServerState>,
    Json(template): Json<DeviceTypeTemplate>,
) -> HandlerResult<serde_json::Value> {
    // Register the template directly (already in simplified format)
    state
        .device_service
        .register_template(template)
        .await
        .map_err(|e| {
            ErrorResponse::bad_request(&format!("Failed to register device type: {}", e))
        })?;

    ok(json!({
        "success": true,
        "registered": true,
    }))
}

/// Delete a device type.
/// Uses new DeviceService
pub async fn delete_device_type_handler(
    State(state): State<ServerState>,
    Path(device_type): Path<String>,
) -> HandlerResult<serde_json::Value> {
    state
        .device_service
        .unregister_template(&device_type)
        .await
        .map_err(|e| ErrorResponse::internal(&format!("Failed to delete device type: {}", e)))?;

    ok(json!({
        "success": true,
        "device_type": device_type,
        "deleted": true,
    }))
}

/// Validate a device type definition without registering it.
/// Accepts simplified format (direct metrics/commands)
pub async fn validate_device_type_handler(
    Json(template): Json<DeviceTypeTemplate>,
) -> HandlerResult<serde_json::Value> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Validate required fields
    if template.device_type.is_empty() {
        errors.push("device_type 不能为空".to_string());
    } else if !template
        .device_type
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        errors.push("device_type 只能包含字母、数字、下划线和连字符".to_string());
    }

    if template.name.is_empty() {
        errors.push("name 不能为空".to_string());
    }

    // Validate categories
    for category in &template.categories {
        if category.is_empty() {
            warnings.push("categories 中包含空字符串".to_string());
        }
    }

    // Validate metrics (simplified structure - direct array)
    for (idx, metric) in template.metrics.iter().enumerate() {
        if metric.name.is_empty() {
            errors.push(format!("metrics[{}]: name 不能为空", idx));
        }
        if metric.display_name.is_empty() {
            warnings.push(format!("metrics[{}]: display_name 为空", idx));
        }
        // Validate data type
        match metric.data_type {
            edge_ai_devices::MetricDataType::Integer
            | edge_ai_devices::MetricDataType::Float
            | edge_ai_devices::MetricDataType::String
            | edge_ai_devices::MetricDataType::Boolean
            | edge_ai_devices::MetricDataType::Binary
            | edge_ai_devices::MetricDataType::Enum { .. } => {}
        }
        // Validate min/max for numeric types
        if matches!(
            metric.data_type,
            edge_ai_devices::MetricDataType::Integer | edge_ai_devices::MetricDataType::Float
        ) {
            if let (Some(min), Some(max)) = (metric.min, metric.max) {
                if min > max {
                    errors.push(format!(
                        "metrics[{}]: min ({}) 不能大于 max ({})",
                        idx, min, max
                    ));
                }
            }
        }
    }

    // Validate commands (simplified structure - direct array)
    for (idx, command) in template.commands.iter().enumerate() {
        if command.name.is_empty() {
            errors.push(format!("commands[{}]: name 不能为空", idx));
        }
        if command.payload_template.is_empty() {
            warnings.push(format!("commands[{}]: payload_template 为空", idx));
        }
        // Validate parameters
        for (pidx, param) in command.parameters.iter().enumerate() {
            if param.name.is_empty() {
                errors.push(format!(
                    "commands[{}].parameters[{}]: name 不能为空",
                    idx, pidx
                ));
            }
        }
    }

    // Check for duplicate metric names
    let mut metric_names = std::collections::HashSet::new();
    for metric in &template.metrics {
        if !metric_names.insert(&metric.name) {
            errors.push(format!("metrics: 存在重复的指标名称 '{}'", metric.name));
        }
    }

    // Check for duplicate command names
    let mut command_names = std::collections::HashSet::new();
    for command in &template.commands {
        if !command_names.insert(&command.name) {
            errors.push(format!("commands: 存在重复的命令名称 '{}'", command.name));
        }
    }

    if errors.is_empty() {
        ok(json!({
            "valid": true,
            "warnings": warnings,
            "message": "设备类型定义有效"
        }))
    } else {
        ok(json!({
            "valid": false,
            "errors": errors,
            "warnings": warnings,
            "message": format!("发现 {} 个错误", errors.len())
        }))
    }
}
