//! System Memory API handlers
//!
//! Provides endpoints for managing the Markdown-based system memory.
//! Supports both file-based API (legacy) and category-based API (new).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use neomind_storage::{CategoryStats, MarkdownMemoryStore, MemoryCategory, MemoryConfig, MemoryFileInfo};

use super::ServerState;

// ============================================================================
// Response Types
// ============================================================================

/// Response for memory file list
#[derive(Debug, Serialize)]
pub struct MemoryFileListResponse {
    pub files: Vec<MemoryFileInfo>,
    pub total: usize,
}

/// Response for single file content
#[derive(Debug, Serialize)]
pub struct MemoryFileContentResponse {
    pub id: String,
    pub source_type: String,
    pub content: String,
}

/// Request to update file content
#[derive(Debug, Deserialize)]
pub struct UpdateMemoryRequest {
    pub content: String,
}

// ============================================================================
// Category-based API Types (New)
// ============================================================================

/// Response for category content
#[derive(Debug, Serialize)]
pub struct CategoryContentResponse {
    pub category: String,
    pub content: String,
    pub stats: CategoryStats,
}

/// Response for all stats
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub categories: HashMap<String, CategoryStats>,
    pub config: Option<MemoryConfig>,
}

/// Request to update config
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub config: MemoryConfig,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the memory store
fn get_memory_store(_state: &ServerState) -> MarkdownMemoryStore {
    MarkdownMemoryStore::new("data/memory")
}

/// Create error response
fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": message.into()
        })),
    )
        .into_response()
}

// ============================================================================
// Category-based API Handlers (New)
// ============================================================================

/// GET /api/memory/category/:category - Get category content
pub async fn get_category(
    State(state): State<ServerState>,
    Path(category): Path<String>,
) -> Response {
    let cat = match MemoryCategory::from_str(&category) {
        Some(c) => c,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                format!("Invalid category: {}. Valid: user_profile, domain_knowledge, task_patterns, system_evolution", category),
            )
        }
    };

    let store = get_memory_store(&state);
    if let Err(e) = store.init() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to initialize store: {}", e),
        );
    }

    match store.read_category(&cat) {
        Ok(content) => {
            let stats = store.category_stats(&cat).unwrap_or_default();
            Json(CategoryContentResponse {
                category: cat.to_string(),
                content,
                stats,
            })
            .into_response()
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read category: {}", e),
        ),
    }
}

/// PUT /api/memory/category/:category - Update category content
pub async fn update_category(
    State(state): State<ServerState>,
    Path(category): Path<String>,
    Json(req): Json<UpdateMemoryRequest>,
) -> Response {
    let cat = match MemoryCategory::from_str(&category) {
        Some(c) => c,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                format!("Invalid category: {}", category),
            )
        }
    };

    let store = get_memory_store(&state);
    if let Err(e) = store.init() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to initialize store: {}", e),
        );
    }

    match store.write_category(&cat, &req.content) {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "message": format!("Category {} updated", cat.display_name())
        }))
        .into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write category: {}", e),
        ),
    }
}

/// GET /api/memory/stats - Get all category statistics
pub async fn get_stats(State(state): State<ServerState>) -> Response {
    let store = get_memory_store(&state);
    if let Err(e) = store.init() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to initialize store: {}", e),
        );
    }

    match store.all_stats() {
        Ok(categories) => Json(StatsResponse {
            categories,
            config: None,
        })
        .into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get stats: {}", e),
        ),
    }
}

/// GET /api/memory/config - Get memory configuration
pub async fn get_config(State(_state): State<ServerState>) -> Response {
    let config = MemoryConfig::load();
    Json(config).into_response()
}

/// PUT /api/memory/config - Update memory configuration
pub async fn update_config(
    State(_state): State<ServerState>,
    Json(req): Json<UpdateConfigRequest>,
) -> Response {
    match req.config.save() {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "config": req.config
        }))
        .into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save config: {}", e),
        ),
    }
}

/// POST /api/memory/extract - Trigger manual extraction
pub async fn trigger_extract(State(_state): State<ServerState>) -> Response {
    // TODO: Implement actual extraction
    Json(serde_json::json!({
        "success": true,
        "extracted": 0,
        "message": "Extraction triggered"
    }))
    .into_response()
}

/// POST /api/memory/compress - Trigger manual compression
pub async fn trigger_compress(State(_state): State<ServerState>) -> Response {
    // TODO: Implement actual compression
    Json(serde_json::json!({
        "success": true,
        "compressed": 0,
        "deleted": 0,
        "message": "Compression triggered"
    }))
    .into_response()
}

/// GET /api/memory/export - Export all categories as Markdown
pub async fn export_all(State(state): State<ServerState>) -> Response {
    let store = get_memory_store(&state);
    if let Err(e) = store.init() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to initialize store: {}", e),
        );
    }

    match store.export_all() {
        Ok(markdown) => (
            StatusCode::OK,
            [("Content-Type", "text/markdown; charset=utf-8")],
            markdown,
        )
            .into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to export: {}", e),
        ),
    }
}

// ============================================================================
// File-based API Handlers (Legacy - for backward compatibility)
// ============================================================================

/// GET /api/memory - List all memory files
pub async fn get_all_memory(State(state): State<ServerState>) -> Response {
    let store = get_memory_store(&state);

    // Ensure initialized
    if let Err(e) = store.init() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to initialize memory store: {}", e),
        );
    }

    match store.list_files() {
        Ok(files) => {
            let total = files.len();
            Json(MemoryFileListResponse { files, total }).into_response()
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list memory files: {}", e),
        ),
    }
}

/// GET /api/memory/:source_type/:id - Get raw markdown content
pub async fn get_memory_content(
    State(state): State<ServerState>,
    Path((source_type, id)): Path<(String, String)>,
) -> Response {
    let store = get_memory_store(&state);

    match store.read_raw_markdown(&source_type, &id) {
        Ok(content) => Json(MemoryFileContentResponse {
            id,
            source_type,
            content,
        })
        .into_response(),
        Err(e) => error_response(
            StatusCode::NOT_FOUND,
            format!("Memory file not found: {}", e),
        ),
    }
}

/// PUT /api/memory/:source_type/:id - Update markdown content
pub async fn update_memory_content(
    State(state): State<ServerState>,
    Path((source_type, id)): Path<(String, String)>,
    Json(request): Json<UpdateMemoryRequest>,
) -> Response {
    let store = get_memory_store(&state);

    match store.write_raw_markdown(&source_type, &id, &request.content) {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "message": "Memory updated"
        }))
        .into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update memory: {}", e),
        ),
    }
}

/// DELETE /api/memory/:source_type/:id - Delete a memory file
pub async fn delete_memory_file(
    State(state): State<ServerState>,
    Path((source_type, id)): Path<(String, String)>,
) -> Response {
    let store = get_memory_store(&state);

    match store.delete_file(&source_type, &id) {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "message": "Memory file deleted"
        }))
        .into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete memory: {}", e),
        ),
    }
}

/// GET /api/memory/export - Export all memory as a single Markdown (Legacy)
pub async fn export_memory(State(state): State<ServerState>) -> Response {
    // Use the new export_all internally
    export_all(State(state)).await
}
