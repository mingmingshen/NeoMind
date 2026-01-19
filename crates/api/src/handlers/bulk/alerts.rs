//! Bulk alert operations.

use axum::{Json, extract::State};
use serde_json::json;

use edge_ai_alerts::{Alert, AlertId, AlertSeverity};
use edge_ai_core::{eventbus::EventBus, event::NeoTalkEvent};

use super::models::{
    BulkAcknowledgeAlertsRequest, BulkCreateAlertsRequest, BulkDeleteAlertsRequest,
    BulkOperationResult, BulkResolveAlertsRequest,
};
use crate::handlers::{
    ServerState,
    common::{HandlerResult, ok},
};

/// Bulk create alerts.
///
/// POST /api/bulk/alerts
pub async fn bulk_create_alerts_handler(
    State(state): State<ServerState>,
    Json(req): Json<BulkCreateAlertsRequest>,
) -> HandlerResult<serde_json::Value> {
    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for (index, item) in req.alerts.into_iter().enumerate() {
        let severity = match item.severity.to_lowercase().as_str() {
            "info" => AlertSeverity::Info,
            "warning" => AlertSeverity::Warning,
            "critical" => AlertSeverity::Critical,
            "emergency" => AlertSeverity::Emergency,
            _ => AlertSeverity::Info,
        };

        let source = if item.source.is_empty() {
            "bulk_api".to_string()
        } else {
            item.source
        };

        let alert = Alert::new(severity, item.title, item.message, source);

        match state.alert_manager.create_alert(alert).await {
            Ok(alert) => {
                results.push(BulkOperationResult {
                    index,
                    success: true,
                    id: Some(alert.id.to_string()),
                    error: None,
                });
                succeeded += 1;
            }
            Err(e) => {
                results.push(BulkOperationResult {
                    index,
                    success: false,
                    id: None,
                    error: Some(e.to_string()),
                });
                failed += 1;
            }
        }
    }

    ok(json!({
        "total": results.len(),
        "succeeded": succeeded,
        "failed": failed,
        "results": results,
    }))
}

/// Bulk resolve alerts.
///
/// POST /api/bulk/alerts/resolve
pub async fn bulk_resolve_alerts_handler(
    State(state): State<ServerState>,
    Json(req): Json<BulkResolveAlertsRequest>,
) -> HandlerResult<serde_json::Value> {
    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for (index, id_str) in req.alert_ids.into_iter().enumerate() {
        match AlertId::from_string(&id_str) {
            Ok(alert_id) => match state.alert_manager.resolve(&alert_id).await {
                Ok(_) => {
                    results.push(BulkOperationResult {
                        index,
                        success: true,
                        id: Some(id_str.clone()),
                        error: None,
                    });
                    succeeded += 1;
                }
                Err(e) => {
                    results.push(BulkOperationResult {
                        index,
                        success: false,
                        id: Some(id_str.clone()),
                        error: Some(e.to_string()),
                    });
                    failed += 1;
                }
            },
            Err(_) => {
                results.push(BulkOperationResult {
                    index,
                    success: false,
                    id: Some(id_str.clone()),
                    error: Some("Invalid alert ID".to_string()),
                });
                failed += 1;
            }
        }
    }

    ok(json!({
        "total": results.len(),
        "succeeded": succeeded,
        "failed": failed,
        "results": results,
    }))
}

/// Bulk acknowledge alerts.
///
/// POST /api/bulk/alerts/acknowledge
pub async fn bulk_acknowledge_alerts_handler(
    State(state): State<ServerState>,
    Json(req): Json<BulkAcknowledgeAlertsRequest>,
) -> HandlerResult<serde_json::Value> {
    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for (index, id_str) in req.alert_ids.into_iter().enumerate() {
        match AlertId::from_string(&id_str) {
            Ok(alert_id) => match state.alert_manager.acknowledge(&alert_id).await {
                Ok(_) => {
                    // Publish AlertAcknowledged event
                    if let Some(event_bus) = &state.event_bus {
                        let _ = event_bus
                            .publish_with_source(
                                NeoTalkEvent::AlertAcknowledged {
                                    alert_id: id_str.clone(),
                                    acknowledged_by: "api:bulk".to_string(),
                                    timestamp: chrono::Utc::now().timestamp(),
                                },
                                "api:bulk_alert",
                            )
                            .await;
                    }

                    results.push(BulkOperationResult {
                        index,
                        success: true,
                        id: Some(id_str.clone()),
                        error: None,
                    });
                    succeeded += 1;
                }
                Err(e) => {
                    results.push(BulkOperationResult {
                        index,
                        success: false,
                        id: Some(id_str.clone()),
                        error: Some(e.to_string()),
                    });
                    failed += 1;
                }
            },
            Err(_) => {
                results.push(BulkOperationResult {
                    index,
                    success: false,
                    id: Some(id_str.clone()),
                    error: Some("Invalid alert ID".to_string()),
                });
                failed += 1;
            }
        }
    }

    ok(json!({
        "total": results.len(),
        "succeeded": succeeded,
        "failed": failed,
        "results": results,
    }))
}

/// Bulk delete alerts.
///
/// POST /api/bulk/alerts/delete
pub async fn bulk_delete_alerts_handler(
    State(state): State<ServerState>,
    Json(req): Json<BulkDeleteAlertsRequest>,
) -> HandlerResult<serde_json::Value> {
    let mut results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;

    for (index, id_str) in req.alert_ids.into_iter().enumerate() {
        match AlertId::from_string(&id_str) {
            Ok(alert_id) => match state.alert_manager.delete_alert(&alert_id).await {
                Ok(_) => {
                    results.push(BulkOperationResult {
                        index,
                        success: true,
                        id: Some(id_str.clone()),
                        error: None,
                    });
                    succeeded += 1;
                }
                Err(e) => {
                    results.push(BulkOperationResult {
                        index,
                        success: false,
                        id: Some(id_str.clone()),
                        error: Some(e.to_string()),
                    });
                    failed += 1;
                }
            },
            Err(_) => {
                results.push(BulkOperationResult {
                    index,
                    success: false,
                    id: Some(id_str.clone()),
                    error: Some("Invalid alert ID".to_string()),
                });
                failed += 1;
            }
        }
    }

    ok(json!({
        "total": results.len(),
        "succeeded": succeeded,
        "failed": failed,
        "results": results,
    }))
}
