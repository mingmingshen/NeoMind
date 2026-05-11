//! Frontend Component API handlers.
//!
//! Provides endpoints for:
//! - Browsing the community component marketplace (GitHub-hosted index)
//! - Installing components from the marketplace
//! - Manual component installation (multipart upload)
//! - Listing, serving, and deleting installed components

use axum::extract::{Multipart, Path, State};
use axum::response::Response;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::io::Read;

use crate::handlers::common::{ok, HandlerResult};
use crate::models::error::ErrorResponse;
use crate::server::ServerState;

use neomind_storage::frontend_components::{ComponentManifest, MarketIndex};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Base URL for marketplace content.
/// Override via `NEOMIND_MARKET_URL` env var to use a mirror (e.g. GitHub proxy).
/// Examples:
///   - Default: https://raw.githubusercontent.com/camthink-ai/NeoMind-Dashboard-Components
///   - Mirror:  https://ghfast.top/https://raw.githubusercontent.com/camthink-ai/NeoMind-Dashboard-Components
fn market_base_url() -> String {
    std::env::var("NEOMIND_MARKET_URL").unwrap_or_else(|_| {
        "https://raw.githubusercontent.com/camthink-ai/NeoMind-Dashboard-Components".to_string()
    })
}
const MARKET_BRANCH: &str = "main";

/// Component IDs reserved for built-in components — cannot be overwritten.
const RESERVED_IDS: &[&str] = &[
    "value-card",
    "led-indicator",
    "sparkline",
    "progress-bar",
    "line-chart",
    "area-chart",
    "bar-chart",
    "pie-chart",
    "toggle-switch",
    "image-display",
    "image-history",
    "web-display",
    "markdown-display",
    "map-display",
    "video-display",
    "custom-layer",
    "agent-monitor-widget",
    "ai-analyst",
];

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Validate a component ID:
/// - Must not be empty
/// - Must not be a reserved built-in ID
/// - Must not contain path traversal characters
fn validate_component_id(id: &str) -> Result<(), ErrorResponse> {
    if id.is_empty() {
        return Err(ErrorResponse::bad_request("Component ID cannot be empty"));
    }
    if RESERVED_IDS.contains(&id) {
        return Err(ErrorResponse::bad_request(format!(
            "Component ID '{}' is reserved for built-in components",
            id
        )));
    }
    if id.contains('.')
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
    {
        return Err(ErrorResponse::bad_request(
            "Component ID contains invalid characters",
        ));
    }
    Ok(())
}

/// Publish a lifecycle event for a frontend component.
async fn publish_lifecycle_event(
    state: &ServerState,
    component_id: &str,
    lifecycle_state: &str,
) {
    if let Some(bus) = &state.core.event_bus {
        let _ = bus
            .publish(neomind_core::event::NeoMindEvent::Custom {
                event_type: "FrontendComponentLifecycle".to_string(),
                data: serde_json::json!({
                    "component_id": component_id,
                    "state": lifecycle_state,
                }),
            })
            .await;
    }
}

/// Build a reqwest client with sensible defaults.
fn http_client() -> Result<reqwest::Client, ErrorResponse> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| ErrorResponse::internal(format!("Failed to build HTTP client: {}", e)))
}

/// Fetch the marketplace index from GitHub.
async fn fetch_market_index(
    client: &reqwest::Client,
) -> Result<MarketIndex, ErrorResponse> {
    let cache_buster = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let index_url = format!(
        "{}/{}/index.json?t={}",
        market_base_url(), MARKET_BRANCH, cache_buster
    );

    let response = client
        .get(&index_url)
        .header("User-Agent", "NeoMind-Component-Marketplace")
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch marketplace index: {}", e);
            ErrorResponse::internal(format!("Network error: {}", e))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        tracing::error!("Marketplace index returned status {}", status);
        return Err(ErrorResponse::internal(format!(
            "Marketplace returned HTTP {}",
            status
        )));
    }

    response
        .json::<MarketIndex>()
        .await
        .map_err(|e| {
            tracing::error!("Failed to parse marketplace index: {}", e);
            ErrorResponse::internal(format!("Invalid marketplace index: {}", e))
        })
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct MarketInstallRequest {
    pub component_id: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET `/api/frontend-components/market/list`
///
/// Fetch the community component index from GitHub.
/// Public endpoint — no authentication required.
pub async fn market_list_handler(
    State(_state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let client = match http_client() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to create HTTP client: {:?}", e);
            return ok(json!({
                "components": [],
                "total": 0,
                "error": "network_error",
                "message": "Unable to create HTTP client"
            }));
        }
    };

    match fetch_market_index(&client).await {
        Ok(index) => ok(json!({
            "components": index.components,
            "total": index.components.len(),
            "market_version": index.version,
        })),
        Err(_) => ok(json!({
            "components": [],
            "total": 0,
            "error": "network_error",
            "message": "Unable to connect to component marketplace. Please check your internet connection."
        })),
    }
}

/// POST `/api/frontend-components/market/install`
///
/// Download and install a component from the community marketplace.
/// Protected endpoint — requires authentication.
pub async fn market_install_handler(
    State(state): State<ServerState>,
    Json(req): Json<MarketInstallRequest>,
) -> HandlerResult<serde_json::Value> {
    let component_id = req.component_id.trim().to_string();
    validate_component_id(&component_id)?;

    let client = match http_client() {
        Ok(c) => c,
        Err(e) => {
            return ok(json!({
                "success": false,
                "error": format!("Failed to create HTTP client: {}", e)
            }));
        }
    };

    // Fetch marketplace index
    let index = match fetch_market_index(&client).await {
        Ok(idx) => idx,
        Err(e) => {
            tracing::error!("Failed to fetch marketplace index: {}", e);
            return ok(json!({
                "success": false,
                "error": format!("Network error: Unable to connect to component marketplace. {}", e)
            }));
        }
    };

    // Find the component entry
    let entry = match index.components.iter().find(|c| c.id == component_id) {
        Some(e) => e,
        None => {
            return ok(json!({
                "success": false,
                "error": format!("Component '{}' not found in marketplace", component_id)
            }));
        }
    };

    // Download manifest and bundle in parallel
    let manifest_url = entry.manifest_url.clone();
    let bundle_url = entry.bundle_url.clone();

    let (manifest_result, bundle_result) = tokio::join!(
        client
            .get(&manifest_url)
            .header("User-Agent", "NeoMind-Component-Marketplace")
            .send(),
        client
            .get(&bundle_url)
            .header("User-Agent", "NeoMind-Component-Marketplace")
            .send()
    );

    let manifest_resp = match manifest_result {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            return ok(json!({
                "success": false,
                "error": format!("Failed to download manifest: HTTP {}", r.status())
            }));
        }
        Err(e) => {
            tracing::error!("Failed to download manifest: {}", e);
            return ok(json!({
                "success": false,
                "error": "Network error: Unable to download component manifest. Please check your internet connection."
            }));
        }
    };

    let bundle_resp = match bundle_result {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            return ok(json!({
                "success": false,
                "error": format!("Failed to download bundle: HTTP {}", r.status())
            }));
        }
        Err(e) => {
            tracing::error!("Failed to download bundle: {}", e);
            return ok(json!({
                "success": false,
                "error": "Network error: Unable to download component bundle. Please check your internet connection."
            }));
        }
    };

    let manifest_text = match manifest_resp.text().await {
        Ok(t) => t,
        Err(e) => {
            return ok(json!({
                "success": false,
                "error": format!("Failed to read manifest: {}", e)
            }));
        }
    };

    let bundle_bytes = match bundle_resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return ok(json!({
                "success": false,
                "error": format!("Failed to read bundle: {}", e)
            }));
        }
    };

    // Parse manifest
    let mut manifest: ComponentManifest = match serde_json::from_str(&manifest_text) {
        Ok(m) => m,
        Err(e) => {
            return ok(json!({
                "success": false,
                "error": format!("Invalid manifest JSON: {}", e)
            }));
        }
    };

    // Validate manifest ID matches requested ID
    if manifest.id != component_id {
        return ok(json!({
            "success": false,
            "error": format!("Manifest ID '{}' does not match requested component ID '{}'", manifest.id, component_id)
        }));
    }

    // Set install timestamp
    manifest.installed_at = chrono::Utc::now().timestamp();

    // Install via store (blocking filesystem operation)
    let store = state.frontend_component_store.clone();
    let id_for_event = manifest.id.clone();
    let manifest_for_response = manifest.clone();
    let install_result = tokio::task::spawn_blocking(move || store.install(&manifest, &bundle_bytes))
        .await;

    match install_result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            return ok(json!({
                "success": false,
                "error": format!("Failed to install component: {}", e)
            }));
        }
        Err(e) => {
            return ok(json!({
                "success": false,
                "error": format!("Install task failed: {}", e)
            }));
        }
    }

    // Publish lifecycle event
    publish_lifecycle_event(&state, &id_for_event, "installed").await;

    tracing::info!(
        component_id = %id_for_event,
        "Community component installed from marketplace"
    );

    ok(json!({
        "component": manifest_for_response
    }))
}

/// POST `/api/frontend-components` (multipart)
///
/// Manual component installation via multipart upload.
///
/// Supports two modes:
/// 1. **ZIP package**: single `package` field containing a `.zip` with `manifest.json` + `bundle.js`
/// 2. **Separate files**: `manifest` (JSON text) + `bundle` (JS bytes) fields
///
/// Protected endpoint with 5 MB body limit.
pub async fn install_component_handler(
    State(state): State<ServerState>,
    mut multipart: Multipart,
) -> HandlerResult<serde_json::Value> {
    let mut manifest_text: Option<String> = None;
    let mut bundle_bytes: Option<Vec<u8>> = None;
    let mut package_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ErrorResponse::bad_request(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "package" => {
                package_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            ErrorResponse::bad_request(format!(
                                "Failed to read package field: {}",
                                e
                            ))
                        })?
                        .to_vec(),
                );
            }
            "manifest" => {
                manifest_text = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| {
                            ErrorResponse::bad_request(format!(
                                "Failed to read manifest field: {}",
                                e
                            ))
                        })?,
                );
            }
            "bundle" => {
                bundle_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            ErrorResponse::bad_request(format!(
                                "Failed to read bundle field: {}",
                                e
                            ))
                        })?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    // Mode 1: ZIP package
    if let Some(zip_data) = package_bytes {
        let (m_text, b_bytes) = tokio::task::spawn_blocking(move || {
            extract_zip_contents(&zip_data)
        })
        .await
        .map_err(|e| ErrorResponse::internal(format!("ZIP extraction task failed: {}", e)))??;

        manifest_text = Some(m_text);
        bundle_bytes = Some(b_bytes);
    }

    let manifest_text =
        manifest_text.ok_or_else(|| ErrorResponse::bad_request("Missing 'manifest' or 'package' field"))?;
    let bundle_bytes =
        bundle_bytes.ok_or_else(|| ErrorResponse::bad_request("Missing 'bundle' or 'package' field"))?;

    // Parse manifest
    let mut manifest: ComponentManifest =
        serde_json::from_str(&manifest_text).map_err(|e| {
            ErrorResponse::bad_request(format!("Invalid manifest JSON: {}", e))
        })?;

    validate_component_id(&manifest.id)?;

    // Set install timestamp
    manifest.installed_at = chrono::Utc::now().timestamp();

    // Install via store (blocking filesystem operation)
    let store = state.frontend_component_store.clone();
    let id_for_event = manifest.id.clone();
    let manifest_for_response = manifest.clone();
    tokio::task::spawn_blocking(move || store.install(&manifest, &bundle_bytes))
        .await
        .map_err(|e| ErrorResponse::internal(format!("Install task failed: {}", e)))?
        .map_err(|e| ErrorResponse::internal(format!("Failed to install component: {}", e)))?;

    // Publish lifecycle event
    publish_lifecycle_event(&state, &id_for_event, "installed").await;

    tracing::info!(
        component_id = %id_for_event,
        "Component installed via manual upload"
    );

    ok(json!({
        "component": manifest_for_response
    }))
}

/// Extract `manifest.json` and `bundle.js` from a ZIP archive.
fn extract_zip_contents(zip_data: &[u8]) -> Result<(String, Vec<u8>), ErrorResponse> {
    let reader = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ErrorResponse::bad_request(format!("Invalid ZIP file: {}", e)))?;

    let mut manifest_text: Option<String> = None;
    let mut bundle_bytes: Option<Vec<u8>> = None;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            ErrorResponse::bad_request(format!("Failed to read ZIP entry: {}", e))
        })?;

        // Normalize path: strip directory prefixes
        let name = file.name().to_string();
        let filename = name
            .rsplit('/')
            .next()
            .unwrap_or(&name)
            .to_string();

        match filename.as_str() {
            "manifest.json" => {
                let mut text = String::new();
                file.read_to_string(&mut text).map_err(|e| {
                    ErrorResponse::bad_request(format!("Failed to read manifest.json: {}", e))
                })?;
                manifest_text = Some(text);
            }
            "bundle.js" => {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).map_err(|e| {
                    ErrorResponse::bad_request(format!("Failed to read bundle.js: {}", e))
                })?;
                bundle_bytes = Some(bytes);
            }
            _ => {}
        }
    }

    let manifest_text = manifest_text.ok_or_else(|| {
        ErrorResponse::bad_request("ZIP must contain manifest.json")
    })?;
    let bundle_bytes = bundle_bytes.ok_or_else(|| {
        ErrorResponse::bad_request("ZIP must contain bundle.js")
    })?;

    Ok((manifest_text, bundle_bytes))
}

/// GET `/api/frontend-components`
///
/// List all installed community components.
/// Protected endpoint — requires authentication.
pub async fn list_components_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let store = state.frontend_component_store.clone();
    let components = tokio::task::spawn_blocking(move || store.list_all())
        .await
        .map_err(|e| ErrorResponse::internal(format!("List task failed: {}", e)))?
        .map_err(|e| ErrorResponse::internal(format!("Failed to list components: {}", e)))?;

    ok(json!({
        "components": components,
        "total": components.len(),
    }))
}

/// GET `/api/frontend-components/:id/bundle`
///
/// Serve a component's bundle.js file.
/// Public endpoint — allows unauthenticated loading of component bundles.
pub async fn get_bundle_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Response, ErrorResponse> {
    validate_component_id(&id)?;

    let store = state.frontend_component_store.clone();
    let id_for_path = id.clone();
    let bundle_path = tokio::task::spawn_blocking(move || store.get_bundle_path(&id_for_path))
        .await
        .map_err(|e| ErrorResponse::internal(format!("Task failed: {}", e)))?;

    let bundle_path = bundle_path.ok_or_else(|| {
        ErrorResponse::not_found(format!("Bundle not found for component '{}'", id))
    })?;

    let bytes = tokio::task::spawn_blocking(move || std::fs::read(&bundle_path))
        .await
        .map_err(|e| ErrorResponse::internal(format!("Read task failed: {}", e)))?
        .map_err(|e| ErrorResponse::internal(format!("Failed to read bundle: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/javascript")
        .header("cache-control", "public, max-age=3600")
        .body(axum::body::Body::from(bytes))
        .unwrap())
}

/// DELETE `/api/frontend-components/:id`
///
/// Uninstall (delete) a community component.
/// Protected endpoint — requires authentication.
pub async fn uninstall_component_handler(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> HandlerResult<serde_json::Value> {
    validate_component_id(&id)?;

    // Check component exists
    let store = state.frontend_component_store.clone();
    let exists = tokio::task::spawn_blocking({
        let store = store.clone();
        let id = id.clone();
        move || store.exists(&id)
    })
    .await
    .map_err(|e| ErrorResponse::internal(format!("Task failed: {}", e)))?;

    if !exists {
        return Err(ErrorResponse::not_found(format!(
            "Component '{}' is not installed",
            id
        )));
    }

    // Delete via store
    let id_for_event = id.clone();
    tokio::task::spawn_blocking(move || store.delete(&id))
        .await
        .map_err(|e| ErrorResponse::internal(format!("Delete task failed: {}", e)))?
        .map_err(|e| ErrorResponse::internal(format!("Failed to delete component: {}", e)))?;

    // Publish lifecycle event
    publish_lifecycle_event(&state, &id_for_event, "uninstalled").await;

    tracing::info!(
        component_id = %id_for_event,
        "Component uninstalled"
    );

    ok(json!({
        "success": true,
        "component_id": id_for_event,
        "state": "uninstalled"
    }))
}
