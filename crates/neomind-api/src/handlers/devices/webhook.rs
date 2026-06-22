//! Webhook receiver for device data.
//!
//! Devices can POST data to this endpoint instead of being polled.
//! This is useful for devices that actively push data.
//!
//! All processing is delegated to `WebhookAdapter` which handles:
//! - Per-device token verification (`Authorization: Bearer` or `?token=`)
//! - Optional adapter-level API key (`X-API-Key` header)
//! - Optional IP allowlist/blocklist
//! - Per-device rate limiting
//! - Per-IP discovery-event throttling (prevents auto-onboard amplification)
//! - Auto-discovery for unknown devices
//! - Data extraction via UnifiedExtractor

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    Json,
};
use std::net::SocketAddr;
use tracing::info;

use crate::handlers::{
    common::{ok, HandlerResult},
    ServerState,
};
use crate::models::ErrorResponse;

use neomind_devices::adapters::webhook::WebhookPayload;

/// Extract webhook token from request headers or query params.
///
/// Checks `Authorization: Bearer <token>` header first, then `?token=xxx` query param.
fn extract_token(
    headers: &HeaderMap,
    params: &std::collections::HashMap<String, String>,
) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .or_else(|| params.get("token").cloned())
}

/// Extract adapter-level API key from the `X-API-Key` header (case-insensitive).
///
/// Distinct from `extract_token` — that reads the per-device `Authorization: Bearer`
/// secret. The adapter-level key is a global pre-shared secret for the whole
/// adapter, useful when the platform is exposed without per-device provisioning.
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Get the internal webhook adapter, downcast from DeviceAdapter.
async fn get_webhook_adapter(
    state: &ServerState,
) -> Result<neomind_devices::adapters::webhook::WebhookAdapter, ErrorResponse> {
    let adapter = state
        .devices
        .service
        .get_adapter("internal-webhook")
        .await
        .ok_or_else(|| ErrorResponse::internal("Webhook adapter not initialized"))?;

    adapter
        .as_any()
        .downcast_ref::<neomind_devices::adapters::webhook::WebhookAdapter>()
        .cloned()
        .ok_or_else(|| ErrorResponse::internal("Failed to downcast webhook adapter"))
}

/// Handle webhook POST from device.
///
/// Endpoint: `POST /api/devices/:id/webhook`
///
/// Devices POST JSON data which is processed by the WebhookAdapter.
/// Auth options (checked by the adapter, all optional individually):
/// - `Authorization: Bearer <token>` or `?token=xxx` — per-device webhook token
/// - `X-API-Key: <key>` — adapter-level pre-shared key (only enforced if configured)
pub async fn webhook_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(mut payload): Json<WebhookPayload>,
) -> HandlerResult<serde_json::Value> {
    let adapter = get_webhook_adapter(&state).await?;
    let token = extract_token(&headers, &params);
    let api_key = extract_api_key(&headers);
    let remote_ip = connect_info.map(|ci| ci.0.ip());

    payload.device_id = Some(device_id.clone());

    let metrics_count = adapter
        .process_webhook(
            device_id.clone(),
            payload,
            token.as_deref(),
            api_key.as_deref(),
            remote_ip.as_ref(),
        )
        .await
        .map_err(|e| {
            tracing::warn!(device_id = %device_id, error = %e, "Webhook processing failed");
            match e {
                neomind_devices::adapter::AdapterError::Connection(msg) => {
                    ErrorResponse::unauthorized(msg)
                }
                neomind_devices::adapter::AdapterError::Configuration(msg) => {
                    ErrorResponse::bad_request(msg)
                }
                _ => ErrorResponse::internal(e.to_string()),
            }
        })?;

    info!(
        device_id = %device_id,
        metrics_count,
        "Webhook data processed"
    );

    ok(serde_json::json!({
        "success": true,
        "device_id": device_id,
        "metrics_received": metrics_count,
    }))
}

/// Handle webhook POST from device (alternative endpoint without device_id in URL).
///
/// Endpoint: `POST /api/devices/webhook`
///
/// The device_id must be provided in the request body.
/// NOTE: The body-supplied `device_id` is attacker-controllable. Deployments that
/// need strong device identity should configure either per-device `webhook_token`s
/// or an adapter-level `X-API-Key`. On closed LAN deployments, leave both unset —
/// the route's rate limit plus per-IP discovery throttle is the only defense.
pub async fn webhook_generic_handler(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(mut payload): Json<WebhookPayload>,
) -> HandlerResult<serde_json::Value> {
    let device_id = payload
        .device_id
        .take()
        .ok_or_else(|| ErrorResponse::bad_request("device_id is required in request body"))?;

    let adapter = get_webhook_adapter(&state).await?;
    let token = extract_token(&headers, &params);
    let api_key = extract_api_key(&headers);
    let remote_ip = connect_info.map(|ci| ci.0.ip());

    payload.device_id = Some(device_id.clone());

    let metrics_count = adapter
        .process_webhook(
            device_id.clone(),
            payload,
            token.as_deref(),
            api_key.as_deref(),
            remote_ip.as_ref(),
        )
        .await
        .map_err(|e| {
            tracing::warn!(device_id = %device_id, error = %e, "Webhook processing failed");
            match e {
                neomind_devices::adapter::AdapterError::Connection(msg) => {
                    ErrorResponse::unauthorized(msg)
                }
                neomind_devices::adapter::AdapterError::Configuration(msg) => {
                    ErrorResponse::bad_request(msg)
                }
                _ => ErrorResponse::internal(e.to_string()),
            }
        })?;

    info!(
        device_id = %device_id,
        metrics_count,
        "Webhook data processed (generic endpoint)"
    );

    ok(serde_json::json!({
        "success": true,
        "device_id": device_id,
        "metrics_processed": metrics_count,
    }))
}

/// Get webhook URL for a device.
///
/// Returns the URL that devices should POST to. Lives behind the hybrid auth
/// middleware (moved out of `public_routes`) — it's an admin/UI lookup, not
/// something devices call, and the previous public placement leaked device
/// existence (404 vs 200) plus the server's configured `NEOMIND_SERVER_URL`.
pub async fn get_webhook_url_handler(
    State(state): State<ServerState>,
    Path(device_id): Path<String>,
    headers: HeaderMap,
) -> HandlerResult<serde_json::Value> {
    // Verify device exists
    if state.devices.service.get_device(&device_id).is_none() {
        return Err(ErrorResponse::not_found(format!(
            "Device {} not found",
            device_id
        )));
    }

    let (server_url, url_source) = crate::handlers::common::resolve_server_url(Some(&headers));

    let mut payload = serde_json::json!({
        "device_id": device_id,
        "webhook_url": format!("{}/api/devices/{}/webhook", server_url, device_id),
        "alternative_url": format!("{}/api/devices/webhook", server_url),
        "method": "POST",
        "content_type": "application/json",
        "payload_example": {
            "timestamp": 1234567890,
            "quality": 1.0,
            "data": {
                "temperature": 23.5,
                "humidity": 65
            }
        },
        "url_source": url_source.as_str(),
    });

    if url_source == crate::handlers::common::ServerUrlSource::Fallback {
        payload["hint"] = serde_json::json!(
            "Set NEOMIND_SERVER_URL env var (or run behind a proxy that sends \
             X-Forwarded-Proto + Host) — the returned URL is a placeholder."
        );
    }

    ok(payload)
}
