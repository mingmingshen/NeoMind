//! Workflow engine handlers.

use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use edge_ai_workflow::{Step, Trigger, Workflow, WorkflowEngine, WorkflowStatus};

use super::{
    ServerState,
    common::{HandlerResult, ok},
};
use crate::models::ErrorResponse;

/// Detailed workflow info for API responses.
#[derive(Debug, Serialize)]
struct WorkflowDto {
    id: String,
    name: String,
    description: String,
    enabled: bool,
    status: String,
    step_count: usize,
    trigger_count: usize,
    created_at: String,
    updated_at: String,
}

/// Workflow execution info for API responses.
#[derive(Debug, Serialize)]
struct ExecutionDto {
    id: String,
    workflow_id: String,
    status: String,
    started_at: String,
    completed_at: Option<String>,
    error: Option<String>,
    step_count: usize,
}

/// Request body for creating a workflow.
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: String,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

/// Request body for updating a workflow.
#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

/// Request body for enabling/disabling a workflow.
#[derive(Debug, Deserialize)]
pub struct SetWorkflowStatusRequest {
    pub enabled: bool,
}

impl From<&Workflow> for WorkflowDto {
    fn from(w: &Workflow) -> Self {
        let status = match w.status {
            WorkflowStatus::Active => "active",
            WorkflowStatus::Paused => "paused",
            WorkflowStatus::Disabled => "disabled",
            WorkflowStatus::Failed => "failed",
        }
        .to_string();

        Self {
            id: w.id.clone(),
            name: w.name.clone(),
            description: w.description.clone(),
            enabled: w.enabled,
            status,
            step_count: w.steps.len(),
            trigger_count: w.triggers.len(),
            created_at: format_timestamp(w.created_at),
            updated_at: format_timestamp(w.updated_at),
        }
    }
}

fn format_timestamp(ts: i64) -> String {
    use chrono::DateTime;
    DateTime::from_timestamp(ts, 0)
        .unwrap_or_default()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// Helper to get the workflow engine from state.
async fn get_workflow_engine(
    state: &ServerState,
) -> Result<Arc<edge_ai_workflow::WorkflowEngine>, ErrorResponse> {
    let engine_opt = state.workflow_engine.read().await;
    engine_opt
        .as_ref()
        .map(Arc::clone)
        .ok_or_else(|| ErrorResponse::service_unavailable("Workflow engine not initialized"))
}

/// List all workflows.
///
/// GET /api/workflows
pub async fn list_workflows_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;
    let workflows = engine
        .list_workflows()
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to list workflows: {}", e)))?;

    let dtos: Vec<WorkflowDto> = workflows.iter().map(WorkflowDto::from).collect();

    ok(json!({
        "workflows": dtos,
        "count": dtos.len(),
    }))
}

/// Get a workflow by ID.
///
/// GET /api/workflows/:id
pub async fn get_workflow_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;
    let workflow = engine
        .get_workflow(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to get workflow: {}", e)))?
        .ok_or_else(|| ErrorResponse::not_found(format!("Workflow {}", id)))?;

    let dto = WorkflowDto::from(&workflow);

    ok(json!({
        "workflow": dto,
    }))
}

/// Create a new workflow.
///
/// POST /api/workflows
pub async fn create_workflow_handler(
    State(state): State<ServerState>,
    Json(req): Json<CreateWorkflowRequest>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;

    let id = uuid::Uuid::new_v4().to_string();
    let mut workflow = Workflow::new(&id, &req.name).with_description(req.description);

    // Add steps from request
    for step in req.steps {
        workflow.steps.push(step);
    }

    // Add triggers from request
    for trigger in req.triggers {
        workflow.triggers.push(trigger);
    }

    // Set timeout if provided
    if let Some(timeout) = req.timeout_seconds {
        workflow.timeout_seconds = timeout;
    }

    // Validate workflow before registering
    workflow
        .validate()
        .map_err(|e| ErrorResponse::bad_request(format!("Invalid workflow: {}", e)))?;

    engine
        .register_workflow(workflow)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to create workflow: {}", e)))?;

    ok(json!({
        "message": "Workflow created",
        "workflow_id": id,
    }))
}

/// Update a workflow.
///
/// PUT /api/workflows/:id
pub async fn update_workflow_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorkflowRequest>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;

    let mut workflow = engine
        .get_workflow(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to get workflow: {}", e)))?
        .ok_or_else(|| ErrorResponse::not_found(format!("Workflow {}", id)))?;

    // Update fields
    if let Some(name) = req.name {
        workflow.name = name;
    }
    if let Some(description) = req.description {
        workflow.description = description;
    }
    if let Some(enabled) = req.enabled {
        workflow.enabled = enabled;
    }
    workflow.updated_at = chrono::Utc::now().timestamp();

    engine
        .register_workflow(workflow)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to update workflow: {}", e)))?;

    ok(json!({
        "message": "Workflow updated",
    }))
}

/// Delete a workflow.
///
/// DELETE /api/workflows/:id
pub async fn delete_workflow_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;
    engine
        .unregister_workflow(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to delete workflow: {}", e)))?;

    ok(json!({
        "message": "Workflow deleted",
    }))
}

/// Execute a workflow.
///
/// POST /api/workflows/:id/execute
pub async fn execute_workflow_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;
    let result = engine
        .execute_workflow(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to execute workflow: {}", e)))?;

    ok(json!({
        "execution_id": result.execution_id,
        "status": format!("{:?}", result.status),
    }))
}

/// Get workflow execution history.
///
/// GET /api/workflows/:id/executions
pub async fn get_workflow_executions_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;

    // First check if workflow exists
    let _workflow = engine
        .get_workflow(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to get workflow: {}", e)))?
        .ok_or_else(|| ErrorResponse::not_found(format!("Workflow {}", id)))?;

    let executions = engine
        .get_workflow_executions(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to get executions: {}", e)))?;

    let execution_dtos: Vec<ExecutionDto> = executions
        .iter()
        .map(|e| ExecutionDto {
            id: e.id.clone(),
            workflow_id: e.workflow_id.clone(),
            status: format!("{:?}", e.status),
            started_at: format_timestamp(e.started_at),
            completed_at: e.completed_at.map(format_timestamp),
            error: e.error.clone(),
            step_count: e.step_results.len(),
        })
        .collect();

    ok(json!({
        "workflow_id": id,
        "executions": execution_dtos,
        "count": execution_dtos.len(),
    }))
}

/// Get a specific execution.
///
/// GET /api/workflows/:id/executions/:exec_id
pub async fn get_execution_handler(
    State(state): State<ServerState>,
    Path((id, exec_id)): Path<(String, String)>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;

    let execution = engine
        .get_execution(&exec_id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to get execution: {}", e)))?
        .ok_or_else(|| ErrorResponse::not_found(format!("Execution {}", exec_id)))?;

    if execution.workflow_id != id {
        return Err(ErrorResponse::bad_request(
            "Execution does not belong to this workflow",
        ));
    }

    ok(json!({
        "execution": ExecutionDto {
            id: execution.id.clone(),
            workflow_id: execution.workflow_id.clone(),
            status: format!("{:?}", execution.status),
            started_at: format_timestamp(execution.started_at),
            completed_at: execution.completed_at.map(format_timestamp),
            error: execution.error.clone(),
            step_count: execution.step_results.len(),
        },
    }))
}

/// Enable or disable a workflow.
///
/// POST /api/workflows/:id/enable
pub async fn set_workflow_status_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(req): Json<SetWorkflowStatusRequest>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;

    let mut workflow = engine
        .get_workflow(&id)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to get workflow: {}", e)))?
        .ok_or_else(|| ErrorResponse::not_found(format!("Workflow {}", id)))?;

    workflow.enabled = req.enabled;
    workflow.updated_at = chrono::Utc::now().timestamp();

    engine
        .register_workflow(workflow)
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to update workflow: {}", e)))?;

    ok(json!({
        "message": if req.enabled { "Workflow enabled" } else { "Workflow disabled" },
        "enabled": req.enabled,
    }))
}

/// Get workflow templates.
///
/// GET /api/workflows/templates
pub async fn get_workflow_templates_handler(
) -> HandlerResult<serde_json::Value> {
    use edge_ai_workflow::WorkflowTemplates;

    let templates = WorkflowTemplates::all();

    let template_dtos: Vec<serde_json::Value> = templates
        .iter()
        .map(|t| {
            json!({
                "id": t.id,
                "name": t.name,
                "category": t.category,
                "description": t.description,
                "parameters": t.parameters.iter().map(|p| {
                    json!({
                        "name": p.name,
                        "label": p.label,
                        "default": p.default,
                        "required": p.required,
                        "type": match p.param_type {
                            edge_ai_workflow::TemplateParameterType::String => "string",
                            edge_ai_workflow::TemplateParameterType::Number => "number",
                            edge_ai_workflow::TemplateParameterType::Boolean => "boolean",
                            edge_ai_workflow::TemplateParameterType::Device => "device",
                            edge_ai_workflow::TemplateParameterType::Metric => "metric",
                            edge_ai_workflow::TemplateParameterType::Enum => "enum",
                        },
                        "options": p.options,
                    })
                }).collect::<Vec<_>>(),
            })
        })
        .collect();

    ok(json!({
        "templates": template_dtos,
        "categories": WorkflowTemplates::categories(),
        "count": template_dtos.len(),
    }))
}

/// Fill a workflow template with parameters.
///
/// POST /api/workflows/templates/fill
pub async fn fill_workflow_template_handler(
    Json(req): Json<serde_json::Value>,
) -> HandlerResult<serde_json::Value> {
    use edge_ai_workflow::WorkflowTemplates;

    let template_id = req
        .get("template_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorResponse::bad_request("Missing 'template_id'"))?;

    let template = WorkflowTemplates::get(template_id)
        .ok_or_else(|| ErrorResponse::not_found(format!("Template {}", template_id)))?;

    let params_map: std::collections::HashMap<String, String> = req
        .get("parameters")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let filled = template
        .fill(&params_map)
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to fill template: {}", e)))?;

    ok(json!({
        "template_id": filled.template_id,
        "workflow_json": filled.dsl,
        "parameters": filled.parameters,
    }))
}

/// Generate a workflow from natural language.
///
/// POST /api/workflows/generate
pub async fn generate_workflow_handler(
    Json(req): Json<serde_json::Value>,
) -> HandlerResult<serde_json::Value> {
    use edge_ai_workflow::WorkflowGenerator;

    let description = req
        .get("description")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ErrorResponse::bad_request("Missing 'description'"))?;

    let context = edge_ai_workflow::ValidationContext::default();

    let generated = WorkflowGenerator::generate(description, &context)
        .map_err(|e| ErrorResponse::bad_request(format!("Failed to generate workflow: {}", e)))?;

    ok(json!({
        "workflow_json": generated.workflow_json,
        "explanation": generated.explanation,
        "confidence": generated.confidence,
        "suggested_edits": generated.suggested_edits,
        "warnings": generated.warnings,
    }))
}

/// Export workflows.
///
/// GET /api/workflows/export
pub async fn export_workflows_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;
    let workflows = engine
        .list_workflows()
        .await
        .map_err(|e| ErrorResponse::internal(format!("Failed to list workflows: {}", e)))?;

    ok(json!({
        "workflows": workflows,
        "export_date": chrono::Utc::now().to_rfc3339(),
        "total_count": workflows.len(),
    }))
}

/// Import workflows.
///
/// POST /api/workflows/import
pub async fn import_workflows_handler(
    State(state): State<ServerState>,
    Json(req): Json<serde_json::Value>,
) -> HandlerResult<serde_json::Value> {
    let engine: Arc<WorkflowEngine> = get_workflow_engine(&state).await?;

    let workflows_to_import: Vec<Workflow> = serde_json::from_value(
        req.get("workflows")
            .or_else(|| req.get("rules"))
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )
    .map_err(|e| ErrorResponse::bad_request(format!("Invalid workflows data: {}", e)))?;

    let mut imported = 0usize;
    let skipped = 0usize;
    let mut errors = Vec::new();

    for mut workflow in workflows_to_import {
        // Generate new ID to avoid conflicts
        let workflow_name = workflow.name.clone();
        workflow.id = uuid::Uuid::new_v4().to_string();

        match workflow.validate() {
            Ok(_) => {
                if let Err(e) = engine.register_workflow(workflow).await {
                    errors.push(json!({
                        "workflow": {"name": workflow_name},
                        "error": format!("{}", e)
                    }));
                } else {
                    imported += 1;
                }
            }
            Err(e) => {
                errors.push(json!({
                    "workflow": {"name": workflow_name},
                    "error": format!("Invalid workflow: {}", e)
                }));
            }
        }
    }

    ok(json!({
        "imported": imported,
        "skipped": skipped,
        "errors": errors,
    }))
}

/// Get available resources for workflow building.
/// Now uses DeviceTypeTemplate for actual device capabilities instead of generic metrics.
///
/// GET /api/workflows/resources
pub async fn get_workflow_resources_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    

    // Get devices with their templates
    let all_devices = state.device_service.list_devices().await;

    // Build detailed device list with actual metrics and commands
    let devices: Vec<serde_json::Value> = {
        let mut result = Vec::new();
        for device in all_devices {
            // Get the template for this device type
            let template = state.device_service.get_template(&device.device_type).await;

            let device_json = if let Some(tpl) = template {
                // Use actual template metrics and commands
                let metrics: Vec<serde_json::Value> = tpl.metrics.iter().map(|m| {
                    json!({
                        "name": m.name,
                        "display_name": m.display_name,
                        "data_type": format!("{:?}", m.data_type),
                        "unit": m.unit,
                        "min": m.min,
                        "max": m.max,
                    })
                }).collect();

                let commands: Vec<serde_json::Value> = tpl.commands.iter().map(|c| {
                    json!({
                        "name": c.name,
                        "display_name": c.display_name,
                        "description": c.llm_hints,
                        "parameters": c.parameters.iter().map(|p| {
                            json!({
                                "name": p.name,
                                "display_name": p.display_name,
                                "data_type": format!("{:?}", p.data_type),
                                "default": p.default_value,
                                "min": p.min,
                                "max": p.max,
                                "unit": p.unit,
                            })
                        }).collect::<Vec<_>>(),
                    })
                }).collect();

                json!({
                    "id": device.device_id,
                    "name": device.name,
                    "type": device.device_type,
                    "description": tpl.description,
                    "categories": tpl.categories,
                    "mode": format!("{:?}", tpl.mode),
                    "metrics": metrics,
                    "commands": commands,
                })
            } else {
                // Fallback for devices without template
                json!({
                    "id": device.device_id,
                    "name": device.name,
                    "type": device.device_type,
                    "metrics": [{"name": "value", "display_name": "Value", "data_type": "Number"}],
                    "commands": [
                        {"name": "on", "display_name": "On"},
                        {"name": "off", "display_name": "Off"}
                    ],
                })
            };

            result.push(device_json);
        }
        result
    };

    // Collect all unique metric names across all device types
    let mut metric_names = std::collections::HashSet::new();
    let templates = state.device_service.list_templates().await;
    for tpl in &templates {
        for metric in &tpl.metrics {
            metric_names.insert(metric.name.clone());
        }
    }
    // Add some common metrics as fallback
    if metric_names.is_empty() {
        for name in &["temperature", "humidity", "pressure", "value", "state", "status", "battery", "signal"] {
            metric_names.insert(name.to_string());
        }
    }
    let metrics: Vec<String> = metric_names.into_iter().collect();

    // Alert channels
    let channels = state.alert_manager.list_channels().await;

    ok(json!({
        "devices": devices,
        "metrics": metrics,
        "alert_channels": channels,
        "device_types": templates.iter().map(|t| json!({
            "device_type": t.device_type,
            "name": t.name,
            "description": t.description,
            "categories": t.categories,
            "metric_count": t.metrics.len(),
            "command_count": t.commands.len(),
        })).collect::<Vec<_>>(),
    }))
}
