//! Unified automations API handlers.
//!
//! This module provides a unified API for both rules and workflows,
//! allowing them to be managed through a single interface.

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use edge_ai_automation::{
    Automation, AutomationConverter, AutomationType, IntentResult,
};

use super::{
    ServerState,
    common::{HandlerResult, ok},
};
use crate::models::ErrorResponse;

/// Query parameters for filtering automations.
#[derive(Debug, Deserialize)]
pub struct AutomationFilter {
    /// Filter by automation type
    pub r#type: Option<AutomationTypeFilter>,
    /// Filter by enabled status
    pub enabled: Option<bool>,
    /// Search in name/description
    pub search: Option<String>,
}

/// Automation type filter.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutomationTypeFilter {
    Transform,
    Rule,
    Workflow,
    All,
}

impl From<AutomationTypeFilter> for Option<AutomationType> {
    fn from(filter: AutomationTypeFilter) -> Self {
        match filter {
            AutomationTypeFilter::Transform => Some(AutomationType::Transform),
            AutomationTypeFilter::Rule => Some(AutomationType::Rule),
            AutomationTypeFilter::Workflow => Some(AutomationType::Workflow),
            AutomationTypeFilter::All => None,
        }
    }
}

/// Automation list response.
#[derive(Debug, Serialize)]
pub struct AutomationListResponse {
    /// List of automations
    pub automations: Vec<AutomationDto>,
    /// Total count
    pub count: usize,
}

/// Automation DTO for API responses.
#[derive(Debug, Serialize)]
pub struct AutomationDto {
    /// Automation ID
    pub id: String,
    /// Automation name
    pub name: String,
    /// Description
    pub description: String,
    /// Automation type
    #[serde(rename = "type")]
    pub automation_type: AutomationType,
    /// Whether enabled
    pub enabled: bool,
    /// Execution count
    pub execution_count: u64,
    /// Last executed timestamp
    pub last_executed: Option<i64>,
    /// Complexity score (1-5)
    pub complexity: u8,
    /// Created at timestamp
    pub created_at: i64,
    /// Updated at timestamp
    pub updated_at: i64,
}

/// Request body for analyzing intent.
#[derive(Debug, Deserialize)]
pub struct AnalyzeIntentRequest {
    /// Natural language description
    pub description: String,
}

/// Request body for creating an automation.
#[derive(Debug, Deserialize)]
pub struct CreateAutomationRequest {
    /// Automation name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Automation type (if not specified, will be inferred from description)
    pub r#type: Option<AutomationType>,
    /// Whether to enable immediately
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Automation definition (rule or workflow)
    pub definition: Value,
}

fn default_enabled() -> bool {
    true
}

/// Request body for updating an automation.
#[derive(Debug, Deserialize)]
pub struct UpdateAutomationRequest {
    /// New name
    pub name: Option<String>,
    /// New description
    pub description: Option<String>,
    /// New definition
    pub definition: Option<Value>,
    /// Enabled status
    pub enabled: Option<bool>,
}

/// Request body for enabling/disabling an automation.
#[derive(Debug, Deserialize)]
pub struct SetAutomationStatusRequest {
    /// Whether to enable the automation
    pub enabled: bool,
}

/// Request body for converting automation type.
#[derive(Debug, Deserialize)]
pub struct ConvertAutomationRequest {
    /// Target type
    pub r#type: AutomationType,
}

/// Convert automation to DTO.
impl From<Automation> for AutomationDto {
    fn from(automation: Automation) -> Self {
        match automation {
            Automation::Transform(transform) => Self {
                id: transform.metadata.id.clone(),
                name: transform.metadata.name.clone(),
                description: transform.metadata.description.clone(),
                automation_type: AutomationType::Transform,
                enabled: transform.metadata.enabled,
                execution_count: transform.metadata.execution_count,
                last_executed: transform.metadata.last_executed,
                complexity: transform.complexity_score(),
                created_at: transform.metadata.created_at,
                updated_at: transform.metadata.updated_at,
            },
            Automation::Rule(rule) => Self {
                id: rule.metadata.id.clone(),
                name: rule.metadata.name.clone(),
                description: rule.metadata.description.clone(),
                automation_type: AutomationType::Rule,
                enabled: rule.metadata.enabled,
                execution_count: rule.metadata.execution_count,
                last_executed: rule.metadata.last_executed,
                complexity: rule.complexity_score(),
                created_at: rule.metadata.created_at,
                updated_at: rule.metadata.updated_at,
            },
            Automation::Workflow(workflow) => Self {
                id: workflow.metadata.id.clone(),
                name: workflow.metadata.name.clone(),
                description: workflow.metadata.description.clone(),
                automation_type: AutomationType::Workflow,
                enabled: workflow.metadata.enabled,
                execution_count: workflow.metadata.execution_count,
                last_executed: workflow.metadata.last_executed,
                complexity: workflow.complexity_score(),
                created_at: workflow.metadata.created_at,
                updated_at: workflow.metadata.updated_at,
            },
        }
    }
}

/// List all automations.
///
/// GET /api/automations?type=rule|workflow|all&enabled=true|false&search=query
pub async fn list_automations_handler(
    Query(filter): Query<AutomationFilter>,
    State(state): State<ServerState>,
) -> HandlerResult<AutomationListResponse> {
    let Some(store) = &state.automation_store else {
        return ok(AutomationListResponse {
            automations: Vec::new(),
            count: 0,
        });
    };

    let automations = store.list_automations().await.unwrap_or_default();

    let mut filtered: Vec<_> = automations
        .into_iter()
        .filter(|a| {
            // Filter by type
            if let Some(type_filter) = &filter.r#type {
                let filter_type = match type_filter {
                    AutomationTypeFilter::Transform => AutomationType::Transform,
                    AutomationTypeFilter::Rule => AutomationType::Rule,
                    AutomationTypeFilter::Workflow => AutomationType::Workflow,
                    AutomationTypeFilter::All => return true,
                };
                if a.automation_type() != filter_type {
                    return false;
                }
            }

            // Filter by enabled status
            if let Some(enabled) = filter.enabled
                && a.is_enabled() != enabled {
                    return false;
                }

            // Search in name/description
            if let Some(search) = &filter.search {
                let search_lower = search.to_lowercase();
                let name_matches = a.name().to_lowercase().contains(&search_lower);
                let desc_matches = match a {
                    Automation::Transform(t) => t.metadata.description.to_lowercase().contains(&search_lower),
                    Automation::Rule(r) => r.metadata.description.to_lowercase().contains(&search_lower),
                    Automation::Workflow(w) => w.metadata.description.to_lowercase().contains(&search_lower),
                };
                if !name_matches && !desc_matches {
                    return false;
                }
            }

            true
        })
        .map(AutomationDto::from)
        .collect();

    // Sort by updated_at descending
    filtered.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    ok(AutomationListResponse {
        count: filtered.len(),
        automations: filtered,
    })
}

/// Get a specific automation by ID.
///
/// GET /api/automations/:id
pub async fn get_automation_handler(
    Path(id): Path<String>,
    State(state): State<ServerState>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    match store.get_automation(&id).await {
        Ok(Some(automation)) => {
            let dto = AutomationDto::from(automation.clone());
            ok(json!({
                "automation": dto,
                "definition": automation,
            }))
        }
        Ok(None) => {
            Err(ErrorResponse::not_found("Automation not found"))
        }
        Err(e) => {
            tracing::error!("Error getting automation: {}", e);
            Err(ErrorResponse::internal(format!("Failed to get automation: {}", e)))
        }
    }
}

/// Create a new automation.
///
/// POST /api/automations
pub async fn create_automation_handler(
    State(state): State<ServerState>,
    Json(req): Json<CreateAutomationRequest>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    // Determine automation type
    let automation_type = req.r#type.unwrap_or(AutomationType::Rule);

    // Create the automation based on type
    let automation = match automation_type {
        AutomationType::Transform => {
            // Parse transform from definition
            match serde_json::from_value(req.definition) {
                Ok(transform) => Automation::Transform(transform),
                Err(e) => {
                    return Err(ErrorResponse::bad_request(format!("Invalid transform definition: {}", e)));
                }
            }
        }
        AutomationType::Rule => {
            // Parse rule from definition
            match serde_json::from_value(req.definition) {
                Ok(rule) => Automation::Rule(rule),
                Err(e) => {
                    return Err(ErrorResponse::bad_request(format!("Invalid rule definition: {}", e)));
                }
            }
        }
        AutomationType::Workflow => {
            // Parse workflow from definition
            match serde_json::from_value(req.definition) {
                Ok(workflow) => Automation::Workflow(workflow),
                Err(e) => {
                    return Err(ErrorResponse::bad_request(format!("Invalid workflow definition: {}", e)));
                }
            }
        }
    };

    // Save the automation
    match store.save_automation(&automation).await {
        Ok(_) => {
            let dto = AutomationDto::from(automation.clone());
            ok(json!({
                "automation": dto,
                "message": "Automation created successfully",
            }))
        }
        Err(e) => {
            tracing::error!("Error creating automation: {}", e);
            Err(ErrorResponse::internal(format!("Failed to create automation: {}", e)))
        }
    }
}

/// Update an automation.
///
/// PUT /api/automations/:id
pub async fn update_automation_handler(
    Path(id): Path<String>,
    State(state): State<ServerState>,
    Json(req): Json<UpdateAutomationRequest>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    // Get existing automation
    let existing = match store.get_automation(&id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return Err(ErrorResponse::not_found("Automation not found"));
        }
        Err(e) => {
            return Err(ErrorResponse::internal(format!("Failed to get automation: {}", e)));
        }
    };

    // Update the automation
    let updated = match existing {
        Automation::Transform(mut transform) => {
            if let Some(name) = req.name {
                transform.metadata.name = name;
            }
            if let Some(description) = req.description {
                transform.metadata.description = description;
            }
            if let Some(enabled) = req.enabled {
                transform.metadata.enabled = enabled;
            }
            Automation::Transform(transform)
        }
        Automation::Rule(mut rule) => {
            if let Some(name) = req.name {
                rule.metadata.name = name;
            }
            if let Some(description) = req.description {
                rule.metadata.description = description;
            }
            if let Some(enabled) = req.enabled {
                rule.metadata.enabled = enabled;
            }
            // Definition updates would require parsing the new definition
            Automation::Rule(rule)
        }
        Automation::Workflow(mut workflow) => {
            if let Some(name) = req.name {
                workflow.metadata.name = name;
            }
            if let Some(description) = req.description {
                workflow.metadata.description = description;
            }
            if let Some(enabled) = req.enabled {
                workflow.metadata.enabled = enabled;
            }
            Automation::Workflow(workflow)
        }
    };

    // Save the updated automation
    match store.save_automation(&updated).await {
        Ok(_) => {
            let dto = AutomationDto::from(updated.clone());
            ok(json!({
                "automation": dto,
                "message": "Automation updated successfully",
            }))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Failed to update automation: {}", e)))
        }
    }
}

/// Delete an automation.
///
/// DELETE /api/automations/:id
pub async fn delete_automation_handler(
    Path(id): Path<String>,
    State(state): State<ServerState>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    match store.delete_automation(&id).await {
        Ok(true) => {
            ok(json!({
                "message": "Automation deleted successfully",
            }))
        }
        Ok(false) => {
            Err(ErrorResponse::not_found("Automation not found"))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Failed to delete automation: {}", e)))
        }
    }
}

/// Set automation enabled status.
///
/// POST /api/automations/:id/enable
pub async fn set_automation_status_handler(
    Path(id): Path<String>,
    State(state): State<ServerState>,
    Json(req): Json<SetAutomationStatusRequest>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    // Get existing automation
    let mut existing = match store.get_automation(&id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return Err(ErrorResponse::not_found("Automation not found"));
        }
        Err(e) => {
            return Err(ErrorResponse::internal(format!("Failed to get automation: {}", e)));
        }
    };

    // Update enabled status
    match &mut existing {
        Automation::Transform(transform) => {
            transform.metadata.enabled = req.enabled;
        }
        Automation::Rule(rule) => {
            rule.metadata.enabled = req.enabled;
        }
        Automation::Workflow(workflow) => {
            workflow.metadata.enabled = req.enabled;
        }
    }

    // Save the updated automation
    match store.save_automation(&existing).await {
        Ok(_) => {
            ok(json!({
                "message": format!("Automation {}", if req.enabled { "enabled" } else { "disabled" }),
                "enabled": req.enabled,
            }))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Failed to update automation: {}", e)))
        }
    }
}

/// Analyze intent to recommend automation type.
///
/// POST /api/automations/analyze-intent
pub async fn analyze_intent_handler(
    State(state): State<ServerState>,
    Json(req): Json<AnalyzeIntentRequest>,
) -> HandlerResult<IntentResult> {
    // Use intent analyzer if available, otherwise use heuristic analysis
    let result = if let Some(analyzer) = &state.intent_analyzer {
        match analyzer.analyze(&req.description).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Error analyzing intent: {}", e);
                // Fallback to heuristic analysis
                heuristic_analysis(&req.description)
            }
        }
    } else {
        // Intent analyzer not initialized, use heuristic analysis
        heuristic_analysis(&req.description)
    };

    ok(result)
}

/// Quick heuristic analysis for intent classification (used when LLM is not available)
fn heuristic_analysis(description: &str) -> IntentResult {
    use edge_ai_automation::AutomationType;

    let desc_lower = description.to_lowercase();

    // Workflow indicators (multi-step, complex logic)
    let workflow_keywords = [
        "then", "after that", "next", "followed by", "sequence",
        "wait", "delay", "pause", "sleep",
        "if then else", "otherwise", "alternative",
        "loop", "repeat", "for each", "iterate",
        "branch", "conditional path", "depending on",
        "step 1", "step 2", "first", "second", "finally",
        "workflow", "process", "pipeline",
    ];

    // Rule indicators (simple if-then)
    let rule_keywords = [
        "when", "if", "then", "trigger", "activates",
        "exceeds", "below", "above", "equals",
        "sensor", "detects", "monitors",
        "simple", "basic", "straightforward",
    ];

    let mut workflow_score = 0i32;
    let mut rule_score = 0i32;

    // Check for workflow indicators
    for keyword in &workflow_keywords {
        if desc_lower.contains(keyword) {
            workflow_score += 10;
        }
    }

    // Check for rule indicators
    for keyword in &rule_keywords {
        if desc_lower.contains(keyword) {
            rule_score += 5;
        }
    }

    // Check for sequential language
    if desc_lower.contains(" and then ")
        || desc_lower.contains(", then ")
        || desc_lower.contains(" after ")
    {
        workflow_score += 20;
    }

    // Check for multiple conditions
    let condition_count = desc_lower.matches("when").count()
        + desc_lower.matches("if").count()
        + desc_lower.matches("whenever").count();

    if condition_count > 1 {
        workflow_score += 15;
    }

    // Check for action complexity
    let action_count = desc_lower.matches(',').count()
        + desc_lower.matches(" and ").count();

    if action_count > 3 {
        workflow_score += 15;
    }

    // Determine result
    let (recommended_type, confidence, reasoning, warnings) =
        if workflow_score > rule_score + 20 {
            (
                AutomationType::Workflow,
                (workflow_score - rule_score).min(100) as u8,
                format!("This appears to be a multi-step automation (workflow score: {}, rule score: {})", workflow_score, rule_score),
                vec![]
            )
        } else if rule_score > workflow_score + 20 {
            (
                AutomationType::Rule,
                (rule_score - workflow_score).min(100) as u8,
                "This appears to be a simple conditional automation".to_string(),
                vec![]
            )
        } else {
            // Close call - use word count and complexity to decide
            let word_count = desc_lower.split_whitespace().count();
            if word_count > 15 || action_count > 2 {
                (
                    AutomationType::Workflow,
                    60,
                    "This description has multiple elements - consider using a workflow for better structure".to_string(),
                    vec!["Moderate confidence - description is somewhat complex".to_string()]
                )
            } else {
                (
                    AutomationType::Rule,
                    65,
                    "This appears to be a straightforward condition-action pattern".to_string(),
                    vec![]
                )
            }
        };

    IntentResult {
        recommended_type,
        confidence,
        reasoning,
        suggested_automation: None,
        warnings,
    }
}

/// Get conversion recommendation for an automation.
///
/// GET /api/automations/:id/conversion-info
pub async fn get_conversion_info_handler(
    Path(id): Path<String>,
    State(state): State<ServerState>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    match store.get_automation(&id).await {
        Ok(Some(automation)) => {
            let recommendation = AutomationConverter::get_conversion_recommendation(&automation);
            ok(json!({
                "automation_id": id,
                "current_type": automation.automation_type(),
                "can_convert": recommendation.can_convert,
                "target_type": recommendation.target_type,
                "reason": recommendation.reason,
                "estimated_complexity": recommendation.estimated_complexity,
            }))
        }
        Ok(None) => {
            Err(ErrorResponse::not_found("Automation not found"))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Failed to get automation: {}", e)))
        }
    }
}

/// Convert an automation between types.
///
/// POST /api/automations/:id/convert
pub async fn convert_automation_handler(
    Path(id): Path<String>,
    State(state): State<ServerState>,
    Json(req): Json<ConvertAutomationRequest>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    // Get existing automation
    let existing = match store.get_automation(&id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return Err(ErrorResponse::not_found("Automation not found"));
        }
        Err(e) => {
            return Err(ErrorResponse::internal(format!("Failed to get automation: {}", e)));
        }
    };

    // Perform conversion
    let converted = match (existing, req.r#type) {
        (Automation::Transform(_), AutomationType::Rule) => {
            return Err(ErrorResponse::bad_request("Transform to Rule conversion is not directly supported. Create a new Rule based on the Transform's output metrics."));
        }
        (Automation::Transform(_), AutomationType::Workflow) => {
            return Err(ErrorResponse::bad_request("Transform to Workflow conversion is not directly supported. Create a new Workflow that uses the Transform's output metrics."));
        }
        (Automation::Rule(_rule), AutomationType::Transform) => {
            return Err(ErrorResponse::bad_request("Rule to Transform conversion is not supported. Transforms are for data processing, not reactive automation."));
        }
        (Automation::Rule(rule), AutomationType::Workflow) => {
            Automation::Workflow(AutomationConverter::rule_to_workflow(rule))
        }
        (Automation::Workflow(_workflow), AutomationType::Transform) => {
            return Err(ErrorResponse::bad_request("Workflow to Transform conversion is not supported. Transforms are for data processing, not complex automation."));
        }
        (Automation::Workflow(workflow), AutomationType::Rule) => {
            match AutomationConverter::workflow_to_rule(&workflow) {
                Some(rule) => Automation::Rule(rule),
                None => {
                    return Err(ErrorResponse::bad_request("Workflow is too complex to convert to a Rule"));
                }
            }
        }
        (automation, _) => {
            return Err(ErrorResponse::bad_request(format!(
                "Cannot convert to the same type (current: {:?})",
                automation.automation_type()
            )));
        }
    };

    // Save the converted automation (with a new ID to preserve the original)
    let new_id = format!("{}-converted", id);
    let converted_with_new_id = match converted {
        Automation::Transform(mut transform) => {
            transform.metadata.id = new_id.clone();
            transform.metadata.name = format!("{} (converted)", transform.metadata.name);
            Automation::Transform(transform)
        }
        Automation::Rule(mut rule) => {
            rule.metadata.id = new_id.clone();
            rule.metadata.name = format!("{} (converted)", rule.metadata.name);
            Automation::Rule(rule)
        }
        Automation::Workflow(mut workflow) => {
            workflow.metadata.id = new_id.clone();
            workflow.metadata.name = format!("{} (converted)", workflow.metadata.name);
            Automation::Workflow(workflow)
        }
    };

    match store.save_automation(&converted_with_new_id).await {
        Ok(_) => {
            let dto = AutomationDto::from(converted_with_new_id.clone());
            ok(json!({
                "automation": dto,
                "message": "Automation converted successfully",
                "original_id": id,
                "new_id": new_id,
            }))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Failed to save converted automation: {}", e)))
        }
    }
}

/// Get execution history for an automation.
///
/// GET /api/automations/:id/executions?limit=10
pub async fn get_automations_executions_handler(
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<ServerState>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(10);

    match store.get_executions(&id, limit).await {
        Ok(executions) => {
            ok(json!({
                "automation_id": id,
                "executions": executions,
                "count": executions.len(),
            }))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Failed to get executions: {}", e)))
        }
    }
}

/// List all automation templates.
///
/// GET /api/automations/templates
pub async fn list_templates_handler(
    State(state): State<ServerState>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return ok(json!({
            "templates": [],
            "count": 0,
        }));
    };

    match store.list_templates() {
        Ok(templates) => {
            ok(json!({
                "templates": templates,
                "count": templates.len(),
            }))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Failed to list templates: {}", e)))
        }
    }
}

/// Export all automations.
///
/// GET /api/automations/export
pub async fn export_automations_handler(
    State(state): State<ServerState>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return ok(json!({
            "automations": [],
            "count": 0,
        }));
    };

    match store.list_automations().await {
        Ok(automations) => {
            ok(json!({
                "automations": automations,
                "count": automations.len(),
                "exported_at": chrono::Utc::now().to_rfc3339(),
            }))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Failed to export automations: {}", e)))
        }
    }
}

/// Import automations.
///
/// POST /api/automations/import
pub async fn import_automations_handler(
    State(state): State<ServerState>,
    Json(data): Json<Value>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    let automations: Vec<Automation> = match serde_json::from_value(data["automations"].clone()) {
        Ok(a) => a,
        Err(e) => {
            return Err(ErrorResponse::bad_request(format!("Invalid automations data: {}", e)));
        }
    };

    let mut imported = 0;
    let mut failed = 0;

    for automation in automations {
        match store.save_automation(&automation).await {
            Ok(_) => imported += 1,
            Err(_) => failed += 1,
        }
    }

    ok(json!({
        "message": "Import completed",
        "imported": imported,
        "failed": failed,
    }))
}

// ========== Transform-Specific Handlers ==========

/// Request body for processing data through transforms.
#[derive(Debug, Deserialize)]
pub struct ProcessDataRequest {
    /// Device ID that produced the data
    pub device_id: String,
    /// Device type (optional, for better transform matching)
    pub device_type: Option<String>,
    /// Raw data from the device (JSON)
    pub data: Value,
    /// Timestamp of the data (defaults to now)
    #[serde(default = "default_timestamp")]
    pub timestamp: i64,
}

fn default_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Process device data through all applicable transforms.
///
/// POST /api/automations/transforms/process
pub async fn process_data_handler(
    State(state): State<ServerState>,
    Json(req): Json<ProcessDataRequest>,
) -> HandlerResult<Value> {
    let Some(transform_engine) = &state.transform_engine else {
        return Err(ErrorResponse::service_unavailable("Transform engine not available"));
    };

    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    // Load all transforms
    let transforms = match store.list_automations().await {
        Ok(automations) => automations
            .into_iter()
            .filter_map(|a| match a {
                Automation::Transform(t) => Some(t),
                _ => None,
            })
            .collect::<Vec<_>>(),
        Err(e) => {
            tracing::error!("Failed to load transforms: {}", e);
            Vec::new()
        }
    };

    // Process data through transforms
    let result = transform_engine
        .process_device_data(
            &transforms,
            &req.device_id,
            req.device_type.as_deref(),
            &req.data,
        )
        .await;

    match result {
        Ok(transform_result) => {
            tracing::debug!(
                "Processed data for device {}: {} metrics produced",
                req.device_id,
                transform_result.metrics.len()
            );

            // Publish transformed metrics to event bus
            if let Some(event_bus) = &state.event_bus {
                for metric in &transform_result.metrics {
                    // Publish as a device metric event
                    use edge_ai_core::NeoTalkEvent;
                    if let Ok(_event_json) = serde_json::to_value(metric) {
                        let _ = event_bus.publish(NeoTalkEvent::DeviceMetric {
                            device_id: metric.device_id.clone(),
                            metric: metric.metric.clone(),
                            value: edge_ai_core::MetricValue::Float(metric.value),
                            timestamp: metric.timestamp,
                            quality: metric.quality,
                        });
                    }
                }
            }

            ok(json!({
                "success": true,
                "metrics": transform_result.metrics,
                "count": transform_result.metrics.len(),
                "warnings": transform_result.warnings,
            }))
        }
        Err(e) => {
            tracing::error!("Transform processing error: {}", e);
            Err(ErrorResponse::internal(format!("Transform processing failed: {}", e)))
        }
    }
}

/// Test a transform with sample data.
///
/// POST /api/automations/transforms/:id/test
pub async fn test_transform_handler(
    Path(id): Path<String>,
    State(state): State<ServerState>,
    Json(req): Json<ProcessDataRequest>,
) -> HandlerResult<Value> {
    let Some(transform_engine) = &state.transform_engine else {
        return Err(ErrorResponse::service_unavailable("Transform engine not available"));
    };

    let Some(store) = &state.automation_store else {
        return Err(ErrorResponse::service_unavailable("Automation store not available"));
    };

    // Load the specific transform
    let automation = match store.get_automation(&id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return Err(ErrorResponse::not_found("Transform not found"));
        }
        Err(e) => {
            return Err(ErrorResponse::internal(format!("Failed to load transform: {}", e)));
        }
    };

    let transform = match automation {
        Automation::Transform(t) => t,
        _ => {
            return Err(ErrorResponse::bad_request("Automation is not a Transform"));
        }
    };

    // Process data through this specific transform
    let result = transform_engine
        .process_device_data(
            &[transform],
            &req.device_id,
            req.device_type.as_deref(),
            &req.data,
        )
        .await;

    match result {
        Ok(transform_result) => {
            ok(json!({
                "transform_id": id,
                "metrics": transform_result.metrics,
                "count": transform_result.metrics.len(),
                "warnings": transform_result.warnings,
            }))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Transform test failed: {}", e)))
        }
    }
}

/// Get list of all transforms (filtering by type).
///
/// GET /api/automations/transforms
pub async fn list_transforms_handler(
    State(state): State<ServerState>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return ok(json!({
            "transforms": [],
            "count": 0,
        }));
    };

    match store.list_automations().await {
        Ok(automations) => {
            let transforms: Vec<_> = automations
                .into_iter()
                .filter_map(|a| match a {
                    Automation::Transform(t) => Some(t),
                    _ => None,
                })
                .collect();

            ok(json!({
                "transforms": transforms,
                "count": transforms.len(),
            }))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Failed to list transforms: {}", e)))
        }
    }
}

/// Get virtual metrics generated by transforms.
///
/// GET /api/automations/transforms/metrics
pub async fn list_virtual_metrics_handler(
    State(state): State<ServerState>,
) -> HandlerResult<Value> {
    let Some(store) = &state.automation_store else {
        return ok(json!({
            "metrics": [],
            "count": 0,
        }));
    };

    match store.list_automations().await {
        Ok(automations) => {
            use std::collections::HashMap;

            let mut metrics_map: HashMap<String, Vec<String>> = HashMap::new();

            for automation in automations {
                if let Automation::Transform(transform) = automation {
                    // Check for JS-based transforms
                    if transform.js_code.is_some() && !transform.output_prefix.is_empty() {
                        metrics_map
                            .entry(transform.output_prefix.clone())
                            .or_default()
                            .push(format!("{}:{}", transform.metadata.id, transform.metadata.name));
                    }
                    // Check for legacy operation-based transforms
                    if let Some(ref operations) = transform.operations {
                        for operation in operations {
                            let output_metrics = operation.output_metrics();
                            for metric in output_metrics {
                                metrics_map
                                    .entry(metric.clone())
                                    .or_default()
                                    .push(format!("{}:{}", transform.metadata.id, transform.metadata.name));
                            }
                        }
                    }
                }
            }

            ok(json!({
                "metrics": metrics_map,
                "count": metrics_map.len(),
            }))
        }
        Err(e) => {
            Err(ErrorResponse::internal(format!("Failed to list virtual metrics: {}", e)))
        }
    }
}

// ========== AI Code Generation ==========

/// Request body for generating transform code.
#[derive(Debug, Deserialize)]
pub struct GenerateTransformCodeRequest {
    /// Natural language description of what the transform should do
    pub intent: String,
    /// Sample input data for context (optional)
    pub sample_input: Option<Value>,
    /// Language preference for comments (zh, en, etc.)
    pub language: Option<String>,
}

/// Response from code generation.
#[derive(Debug, Serialize)]
pub struct GenerateTransformCodeResponse {
    /// Generated JavaScript code
    pub js_code: String,
    /// Suggested output prefix for metrics
    pub output_prefix: String,
    /// Suggested name for the transform
    pub suggested_name: String,
    /// Explanation of what the code does
    pub explanation: String,
}

/// Generate JavaScript code for a transform from natural language intent.
///
/// POST /api/automations/generate-code
pub async fn generate_transform_code_handler(
    State(_state): State<ServerState>,
    Json(req): Json<GenerateTransformCodeRequest>,
) -> HandlerResult<GenerateTransformCodeResponse> {
    let language = req.language.as_deref().unwrap_or("en");

    // Use heuristic-based code generation
    // (TODO: Can be upgraded to use LLM in the future)
    let result = heuristic_generate_transform_code(&req.intent, language);

    ok(result)
}

/// Generate transform code using heuristics.
fn heuristic_generate_transform_code(intent: &str, language: &str) -> GenerateTransformCodeResponse {
    let intent_lower = intent.to_lowercase();

    let (code, explanation) = if language == "zh" {
        generate_code_zh(&intent_lower)
    } else {
        generate_code_en(&intent_lower)
    };

    let output_prefix = generate_output_prefix_from_intent(intent);
    let suggested_name = intent.chars().take(50).collect::<String>();

    GenerateTransformCodeResponse {
        js_code: code,
        output_prefix,
        suggested_name,
        explanation,
    }
}

/// Generate output prefix from intent.
fn generate_output_prefix_from_intent(intent: &str) -> String {
    let intent_lower = intent.to_lowercase();

    // Common patterns for output prefix
    let patterns = [
        ("count", "count"),
        ("统计", "count"),
        ("平均", "average"),
        ("average", "average"),
        ("avg", "avg"),
        ("mean", "mean"),
        ("sum", "sum"),
        ("总和", "sum"),
        ("filter", "filtered"),
        ("过滤", "filtered"),
        ("最大", "max"),
        ("max", "max"),
        ("最小", "min"),
        ("min", "min"),
        ("检测", "detection"),
        ("detection", "detection"),
        ("转换", "converted"),
        ("convert", "converted"),
    ];

    for (keyword, prefix) in patterns {
        if intent_lower.contains(keyword) {
            return prefix.to_string();
        }
    }

    // Default: derive from intent
    let derived: String = intent_lower
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .trim_matches('_')
        .to_string()
        .replace("__", "_");
    derived.split('_').take(3).collect::<Vec<&str>>().join("_")
}

/// Generate code with Chinese comments.
fn generate_code_zh(intent: &str) -> (String, String) {
    let (code, desc) = if intent.contains("统计") || intent.contains("count") || intent.contains("数量") {
        if intent.contains("cls") || intent.contains("class") || intent.contains("类别") {
            (r#"// 统计每个类别的检测数量
const counts = {};
for (const item of input.detections || []) {
  const cls = item.cls || item.class || 'unknown';
  counts[cls] = (counts[cls] || 0) + 1;
}
return counts;"#, "按类别统计检测数量")
        } else {
            (r#"// 统计数组元素数量
return (input.items || input.detections || input.array || []).length;"#, "统计元素总数")
        }
    } else if intent.contains("平均") || intent.contains("average") || intent.contains("avg") {
        (r#"// 计算平均值
const arr = input.values || input.items || input.detections || [];
const sum = arr.reduce((acc, item) => acc + (item.value || item.count || item || 0), 0);
return sum / arr.length;"#, "计算平均值")
    } else if intent.contains("过滤") || intent.contains("filter") || intent.contains("筛选") {
        if intent.contains("0.5") || intent.contains("置信度") || intent.contains("confidence") {
            (r#"// 过滤置信度低于阈值的检测
return (input.detections || input.items || []).filter(
  item => (item.confidence || item.conf || item.score || 0) >= 0.5
);"#, "过滤低置信度检测")
        } else {
            (r#"// 过滤数据（根据条件）
return (input.items || input.detections || []).filter(
  item => item.value > 0  // 根据需要修改条件
);"#, "根据条件过滤数据")
        }
    } else if intent.contains("16") || intent.contains("hex") || intent.contains("进制") {
        (r#"// 解码 16 进制字符串为 JSON
const hex = input.hex_data || input.data || input.hex || '';
const str = hex.match(/.{1,2}/g)?.map(byte => String.fromCharCode(parseInt(byte, 16))).join('') || '';
try {
  return JSON.parse(str);
} catch {
  return str;
}", "16 进制字符串转 JSON")
    } else if intent.contains("提取") || intent.contains("extract") || intent.contains("嵌套") {
        (r#"// 提取嵌套字段到根级别
const result = {};
const data = input.data || input;
for (const [key, value] of Object.entries(data)) {
  if (typeof value !== 'object' || value === null) {
    result[key] = value;
  }
}
return result;"#, "提取嵌套字段")
    } else if intent.contains("求和") || intent.contains("sum") {
        (r#"// 计算总和
const arr = input.values || input.items || [];
return arr.reduce((acc, item) => acc + (item.value || item || 0), 0);"#, "计算总和")
    } else if intent.contains("最大") || intent.contains("max") {
        (r#"// 找出最大值
const arr = input.values || input.items || [];
return Math.max(...arr.map(item => item.value || item));"#, "找出最大值")
    } else if intent.contains("最小") || intent.contains("min") {
        (r#"// 找出最小值
const arr = input.values || input.items || [];
return Math.min(...arr.map(item => item.value || item));"#, "找出最小值")
    } else {
        (r#"// 通用数据转换
return {
  ...input,
  processed: true,
  timestamp: Date.now()
};"#, "通用数据处理")
    };

    (code.to_string(), format!("此转换代码: {}", desc))
}

/// Generate code with English comments.
fn generate_code_en(intent: &str) -> (String, String) {
    let (code, desc) = if intent.contains("count") || intent.contains("统计") {
        if intent.contains("cls") || intent.contains("class") {
            (r#"// Count detections by class
const counts = {};
for (const item of input.detections || []) {
  const cls = item.cls || item.class || 'unknown';
  counts[cls] = (counts[cls] || 0) + 1;
}
return counts;"#, "Count items by class")
        } else {
            (r#"// Count array elements
return (input.items || input.detections || input.array || []).length;"#, "Count total elements")
        }
    } else if intent.contains("average") || intent.contains("avg") || intent.contains("mean") || intent.contains("平均") {
        (r#"// Calculate average
const arr = input.values || input.items || input.detections || [];
const sum = arr.reduce((acc, item) => acc + (item.value || item.count || item || 0), 0);
return sum / arr.length;"#, "Calculate average")
    } else if intent.contains("filter") || intent.contains("过滤") {
        if intent.contains("0.5") || intent.contains("confidence") {
            (r#"// Filter by confidence threshold
return (input.detections || input.items || []).filter(
  item => (item.confidence || item.conf || item.score || 0) >= 0.5
);"#, "Filter by confidence")
        } else {
            (r#"// Filter items by condition
return (input.items || input.detections || []).filter(
  item => item.value > 0  // adjust condition as needed
);"#, "Filter by condition")
        }
    } else if intent.contains("hex") || intent.contains("16") || intent.contains("decode") {
        (r#"// Decode hex string to JSON
const hex = input.hex_data || input.data || input.hex || '';
const str = hex.match(/.{1,2}/g)?.map(byte => String.fromCharCode(parseInt(byte, 16))).join('') || '';
try {
  return JSON.parse(str);
} catch {
  return str;
}", "Decode hex to JSON")
    } else if intent.contains("extract") || intent.contains("提取") || intent.contains("flatten") {
        (r#"// Extract nested fields to root level
const result = {};
const data = input.data || input;
for (const [key, value] of Object.entries(data)) {
  if (typeof value !== 'object' || value === null) {
    result[key] = value;
  }
}
return result;"#, "Extract nested fields")
    } else if intent.contains("sum") || intent.contains("求和") {
        (r#"// Calculate sum
const arr = input.values || input.items || [];
return arr.reduce((acc, item) => acc + (item.value || item || 0), 0);"#, "Calculate sum")
    } else if intent.contains("max") || intent.contains("最大") {
        (r#"// Find maximum value
const arr = input.values || input.items || [];
return Math.max(...arr.map(item => item.value || item));"#, "Find maximum")
    } else if intent.contains("min") || intent.contains("最小") {
        (r#"// Find minimum value
const arr = input.values || input.items || [];
return Math.min(...arr.map(item => item.value || item));"#, "Find minimum")
    } else {
        (r#"// Generic data transformation
return {
  ...input,
  processed: true,
  timestamp: Date.now()
};"#, "Generic data transformation")
    };

    (code.to_string(), format!("This transform: {}", desc))
}
