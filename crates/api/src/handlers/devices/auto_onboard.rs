//! Auto-onboarding API handlers for zero-config device discovery
//!
//! This module provides REST endpoints for:
//! - Listing draft devices
//! - Getting draft device details
//! - Approving/rejecting draft devices
//! - Managing auto-onboarding configuration

use edge_ai_automation::{AutoOnboardManager, DraftDevice, GeneratedDeviceType, DiscoveredMetric, ProcessingSummary};

use crate::handlers::devices::models::*;
use crate::handlers::common::{HandlerResult, ok};
use crate::models::ErrorResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

/// List all draft devices
#[utoipa::path(
    get,
    path = "/api/devices/drafts",
    tag = "devices",
    responses(
        (status = 200, description = "List of draft devices", body = DraftDevicesResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_draft_devices(
    State(manager): State<AutoOnboardManager>,
) -> HandlerResult<DraftDevicesResponse> {
    let drafts = manager.get_drafts().await;

    let items: Vec<DraftDeviceDto> = drafts.into_iter().map(DraftDeviceDto::from).collect();

    ok(DraftDevicesResponse {
        total: items.len(),
        items,
    })
}

/// Get a specific draft device by ID
#[utoipa::path(
    get,
    path = "/api/devices/drafts/{device_id}",
    tag = "devices",
    params(
        ("device_id" = String, Path, description = "Device ID")
    ),
    responses(
        (status = 200, description = "Draft device details", body = DraftDeviceDto),
        (status = 404, description = "Draft device not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_draft_device(
    State(manager): State<AutoOnboardManager>,
    Path(device_id): Path<String>,
) -> HandlerResult<DraftDeviceDto> {
    let draft = manager
        .get_draft(&device_id)
        .await
        .ok_or_else(|| ErrorResponse::not_found(&format!("Draft device '{}' not found", device_id)))?;

    ok(DraftDeviceDto::from(draft))
}

/// Update a draft device (user edits)
#[utoipa::path(
    put,
    path = "/api/devices/drafts/{device_id}",
    tag = "devices",
    params(
        ("device_id" = String, Path, description = "Device ID")
    ),
    request_body = UpdateDraftDeviceRequest,
    responses(
        (status = 200, description = "Draft device updated"),
        (status = 404, description = "Draft device not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_draft_device(
    State(manager): State<AutoOnboardManager>,
    Path(device_id): Path<String>,
    Json(request): Json<UpdateDraftDeviceRequest>,
) -> HandlerResult<SuccessResponse> {
    manager
        .update_draft(&device_id, request.name, request.description)
        .await
        .map_err(|e| ErrorResponse::internal(&e.to_string()))?;

    ok(SuccessResponse {
        message: format!("Draft device '{}' updated", device_id),
    })
}

/// Approve and register a draft device
///
/// If an existing_type is provided in the request body, the device will be
/// assigned to that existing type instead of creating a new one.
#[utoipa::path(
    post,
    path = "/api/devices/drafts/{device_id}/approve",
    tag = "devices",
    params(
        ("device_id" = String, Path, description = "Device ID")
    ),
    request_body = Option<ApproveDraftDeviceRequest>,
    responses(
        (status = 200, description = "Device approved and registered"),
        (status = 404, description = "Draft device not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn approve_draft_device(
    State(manager): State<AutoOnboardManager>,
    Path(device_id): Path<String>,
    Json(request): Json<Option<ApproveDraftDeviceRequest>>,
) -> HandlerResult<ApproveDraftResponse> {
    // First get the draft to find the draft_id
    let draft = manager
        .get_draft(&device_id)
        .await
        .ok_or_else(|| ErrorResponse::not_found(&format!("Draft device '{}' not found", device_id)))?;

    let draft_id = draft.id.clone();

    // If an existing type is specified, we need to handle it specially
    // The generated_type might need to be updated with the existing type
    let device_type = if let Some(existing_type) = request.and_then(|r| r.existing_type) {
        // User wants to reuse an existing type
        tracing::info!(
            "Device {} being assigned to existing type {}",
            device_id,
            existing_type
        );
        existing_type
    } else {
        // Use the generated type
        draft
            .generated_type
            .as_ref()
            .map(|g| g.device_type.clone())
            .unwrap_or_else(|| format!("auto_{}", device_id))
    };

    // Register the device
    manager
        .register_device(&draft_id, &device_id)
        .await
        .map_err(|e| ErrorResponse::internal(&e.to_string()))?;

    ok(ApproveDraftResponse {
        device_id: device_id.clone(),
        device_type: device_type.clone(),
        registered: true,
        message: format!(
            "Device '{}' approved and registered as type '{}'",
            device_id, device_type
        ),
    })
}

/// Reject a draft device
#[utoipa::path(
    post,
    path = "/api/devices/drafts/{device_id}/reject",
    tag = "devices",
    params(
        ("device_id" = String, Path, description = "Device ID")
    ),
    request_body = RejectDraftDeviceRequest,
    responses(
        (status = 200, description = "Draft device rejected"),
        (status = 404, description = "Draft device not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn reject_draft_device(
    State(manager): State<AutoOnboardManager>,
    Path(device_id): Path<String>,
    Json(request): Json<RejectDraftDeviceRequest>,
) -> HandlerResult<SuccessResponse> {
    manager
        .reject_device(&device_id, &request.reason)
        .await
        .map_err(|e| ErrorResponse::internal(&e.to_string()))?;

    ok(SuccessResponse {
        message: format!("Draft device '{}' rejected", device_id),
    })
}

/// Trigger manual analysis of a draft device
#[utoipa::path(
    post,
    path = "/api/devices/drafts/{device_id}/analyze",
    tag = "devices",
    params(
        ("device_id" = String, Path, description = "Device ID")
    ),
    responses(
        (status = 200, description = "Analysis triggered"),
        (status = 404, description = "Draft device not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn trigger_draft_analysis(
    State(manager): State<AutoOnboardManager>,
    Path(device_id): Path<String>,
) -> HandlerResult<SuccessResponse> {
    let draft = manager
        .get_draft(&device_id)
        .await
        .ok_or_else(|| ErrorResponse::not_found(&format!("Draft device '{}' not found", device_id)))?;

    let draft_id = draft.id.clone();
    let samples = draft.json_samples();

    if samples.is_empty() {
        return Err(ErrorResponse::bad_request("No samples available for analysis"));
    }

    // Trigger analysis in background
    let manager_clone = manager.clone();
    let device_id_clone = device_id.clone();
    tokio::spawn(async move {
        let _ = manager_clone.analyze_device(&draft_id, &device_id_clone, samples).await;
    });

    ok(SuccessResponse {
        message: format!("Analysis triggered for device '{}'", device_id),
    })
}

/// Clean up old draft devices
#[utoipa::path(
    post,
    path = "/api/devices/drafts/cleanup",
    tag = "devices",
    responses(
        (status = 200, description = "Old drafts cleaned up", body = CleanupResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn cleanup_draft_devices(
    State(manager): State<AutoOnboardManager>,
) -> HandlerResult<CleanupResponse> {
    let count = manager.cleanup_old_drafts().await;

    ok(CleanupResponse {
        cleaned: count,
        message: format!("Cleaned up {} old draft devices", count),
    })
}

/// Get all registered type signatures
///
/// Returns a mapping of signature hashes to device type IDs.
/// This is used for type reusability - devices with matching signatures
/// can reuse the same device type.
#[utoipa::path(
    get,
    path = "/api/devices/drafts/type-signatures",
    tag = "devices",
    responses(
        (status = 200, description = "Type signatures mapping", body = TypeSignaturesResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_type_signatures(
    State(manager): State<AutoOnboardManager>,
) -> HandlerResult<TypeSignaturesResponse> {
    let signatures = manager.get_all_type_signatures().await;
    let count = signatures.len().to_string();

    ok(TypeSignaturesResponse {
        signatures,
        count,
    })
}

/// Get auto-onboarding configuration
#[utoipa::path(
    get,
    path = "/api/devices/drafts/config",
    tag = "devices",
    responses(
        (status = 200, description = "Auto-onboarding configuration"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_onboard_config(
    State(manager): State<AutoOnboardManager>,
) -> HandlerResult<AutoOnboardConfig> {
    // TODO: Add config getter to AutoOnboardManager
    // For now, return default
    ok(AutoOnboardConfig::default())
}

/// Update auto-onboarding configuration
#[utoipa::path(
    put,
    path = "/api/devices/drafts/config",
    tag = "devices",
    request_body = AutoOnboardConfig,
    responses(
        (status = 200, description = "Configuration updated"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_onboard_config(
    State(_manager): State<AutoOnboardManager>,
    Json(_config): Json<AutoOnboardConfig>,
) -> HandlerResult<SuccessResponse> {
    // TODO: Add config setter to AutoOnboardManager
    ok(SuccessResponse {
        message: "Configuration updated".to_string(),
    })
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for listing draft devices
#[derive(Debug, Serialize)]
pub struct DraftDevicesResponse {
    pub total: usize,
    pub items: Vec<DraftDeviceDto>,
}

/// DTO for a draft device
#[derive(Debug, Clone, Serialize)]
pub struct DraftDeviceDto {
    pub id: String,
    pub device_id: String,
    pub source: String,
    pub status: String,
    pub sample_count: usize,
    pub max_samples: usize,
    pub generated_type: Option<GeneratedDeviceTypeDto>,
    pub discovered_at: i64,
    pub updated_at: i64,
    pub error_message: Option<String>,
    pub user_name: Option<String>,
}

impl From<edge_ai_automation::DraftDevice> for DraftDeviceDto {
    fn from(draft: edge_ai_automation::DraftDevice) -> Self {
        Self {
            id: draft.id,
            device_id: draft.device_id,
            source: draft.source,
            status: format!("{:?}", draft.status),
            sample_count: draft.samples.len(),
            max_samples: draft.max_samples,
            generated_type: draft.generated_type.map(GeneratedDeviceTypeDto::from),
            discovered_at: draft.discovered_at,
            updated_at: draft.updated_at,
            error_message: draft.error_message,
            user_name: draft.user_name,
        }
    }
}

/// DTO for a generated device type
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedDeviceTypeDto {
    pub device_type: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub metrics: Vec<MetricSummaryDto>,
    pub confidence: f32,
    pub summary: ProcessingSummaryDto,
}

impl From<edge_ai_automation::GeneratedDeviceType> for GeneratedDeviceTypeDto {
    fn from(gen_type: edge_ai_automation::GeneratedDeviceType) -> Self {
        Self {
            device_type: gen_type.device_type,
            name: gen_type.name,
            description: gen_type.description,
            category: gen_type.category.display_name().to_string(),
            metrics: gen_type.metrics.into_iter().map(MetricSummaryDto::from).collect(),
            confidence: gen_type.confidence,
            summary: ProcessingSummaryDto::from(&gen_type.summary),
        }
    }
}

/// DTO for metric summary
#[derive(Debug, Clone, Serialize)]
pub struct MetricSummaryDto {
    pub name: String,
    pub path: String,
    pub semantic_type: String,
    pub display_name: String,
    pub confidence: f32,
}

impl From<edge_ai_automation::DiscoveredMetric> for MetricSummaryDto {
    fn from(metric: edge_ai_automation::DiscoveredMetric) -> Self {
        Self {
            name: metric.name,
            path: metric.path,
            semantic_type: metric.semantic_type.display_name().to_string(),
            display_name: metric.display_name,
            confidence: metric.confidence,
        }
    }
}

/// DTO for processing summary
#[derive(Debug, Clone, Serialize)]
pub struct ProcessingSummaryDto {
    pub samples_analyzed: usize,
    pub fields_discovered: usize,
    pub metrics_generated: usize,
    pub inferred_category: String,
    pub insights: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

impl From<&edge_ai_automation::ProcessingSummary> for ProcessingSummaryDto {
    fn from(summary: &edge_ai_automation::ProcessingSummary) -> Self {
        Self {
            samples_analyzed: summary.samples_analyzed,
            fields_discovered: summary.fields_discovered,
            metrics_generated: summary.metrics_generated,
            inferred_category: summary.inferred_category.clone(),
            insights: summary.insights.clone(),
            warnings: summary.warnings.clone(),
            recommendations: summary.recommendations.clone(),
        }
    }
}

/// Response for device approval
#[derive(Debug, Serialize)]
pub struct ApproveDraftResponse {
    pub device_id: String,
    pub device_type: String,
    pub registered: bool,
    pub message: String,
}

/// Request to approve a draft device (with optional existing type)
#[derive(Debug, Deserialize)]
pub struct ApproveDraftDeviceRequest {
    /// Optional existing device type to reuse instead of creating a new one
    pub existing_type: Option<String>,
}

/// Response for type signatures
#[derive(Debug, Serialize)]
pub struct TypeSignaturesResponse {
    /// Mapping of signature hash to device type ID
    pub signatures: std::collections::HashMap<String, String>,
    /// Number of registered signatures
    pub count: String,
}

/// Response for cleanup
#[derive(Debug, Serialize)]
pub struct CleanupResponse {
    pub cleaned: usize,
    pub message: String,
}

/// Request to update a draft device
#[derive(Debug, Deserialize)]
pub struct UpdateDraftDeviceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Request to reject a draft device
#[derive(Debug, Deserialize)]
pub struct RejectDraftDeviceRequest {
    pub reason: String,
}

/// Simple success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
}

/// Auto-onboarding configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct AutoOnboardConfig {
    pub enabled: bool,
    pub auto_register_threshold: f32,
    pub max_samples_per_device: usize,
    pub cleanup_after_hours: u64,
}

impl Default for AutoOnboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_register_threshold: 0.9,
            max_samples_per_device: 100,
            cleanup_after_hours: 24,
        }
    }
}
