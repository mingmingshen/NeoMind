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
use std::sync::atomic::{AtomicBool, Ordering};

/// Guard to prevent concurrent extraction tasks
static EXTRACTION_RUNNING: AtomicBool = AtomicBool::new(false);

use neomind_storage::{
    CategoryStats, MarkdownMemoryStore, MemoryCategory, MemoryConfig, MemoryFileInfo,
};

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

/// Request to trigger extraction
#[derive(Debug, Deserialize)]
pub struct ExtractionRequest {
    /// Session ID to extract from (optional - if not provided, extracts from all recent sessions)
    pub session_id: Option<String>,
    /// Force extraction even if minimum message count not met
    pub force: bool,
}

/// Response for extraction
#[derive(Debug, Serialize)]
pub struct ExtractionResponse {
    pub success: bool,
    pub extracted: usize,
    pub message: String,
}

/// Request to add a manual memory entry
#[derive(Debug, Deserialize)]
pub struct AddMemoryRequest {
    /// Category to add to
    pub category: String,
    /// Memory content
    pub content: String,
    /// Importance score (0-100)
    pub importance: u8,
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
///
/// This endpoint triggers memory extraction from chat sessions.
/// If session_id is provided, extracts from that specific session.
/// Otherwise, extracts from the most recent sessions.
pub async fn trigger_extract(
    State(state): State<ServerState>,
    Json(req): Json<ExtractionRequest>,
) -> Response {
    use neomind_agent::memory_extraction::{ExtractionConfig, MemoryExtractor};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    tracing::info!(
        session_id = ?req.session_id,
        force = req.force,
        "Starting memory extraction request"
    );

    // Prevent concurrent extraction tasks
    if EXTRACTION_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Json(ExtractionResponse {
            success: true,
            extracted: 0,
            message: "Extraction already in progress. Please wait for it to finish.".to_string(),
        })
        .into_response();
    }

    // Get the LLM backend
    let llm_manager = match neomind_agent::get_instance_manager() {
        Ok(manager) => {
            tracing::debug!("Got LLM instance manager");
            manager
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get LLM instance manager");
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("LLM backend not available: {}", e),
            );
        }
    };

    let llm_runtime = match llm_manager.get_active_runtime().await {
        Ok(runtime) => {
            tracing::info!(
                model = %runtime.model_name(),
                backend = ?runtime.backend_id(),
                "Got active LLM runtime"
            );
            runtime
        }
        Err(e) => {
            tracing::error!(error = %e, "No active LLM runtime configured");
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("No active LLM backend configured: {}", e),
            );
        }
    };

    // Get session store
    let session_store = state.agents.session_manager.session_store();

    // Determine which session(s) to extract from
    // Limit to recent sessions to avoid timeout issues
    const MAX_SESSIONS_PER_EXTRACTION: usize = 5;

    let session_ids: Vec<String> = match req.session_id {
        Some(id) => vec![id],
        None => {
            // Get all sessions, but only process the most recent ones
            match session_store.list_sessions() {
                Ok(ids) => ids.into_iter().take(MAX_SESSIONS_PER_EXTRACTION).collect(),
                Err(e) => {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to list sessions: {}", e),
                    )
                }
            }
        }
    };

    if session_ids.is_empty() {
        tracing::info!("No sessions available for extraction");
        return Json(ExtractionResponse {
            success: true,
            extracted: 0,
            message: "No sessions available to extract from".to_string(),
        })
        .into_response();
    }

    let session_count = session_ids.len();
    tracing::info!(
        session_count = session_count,
        sessions = ?session_ids,
        "Spawning background extraction task"
    );

    // Get the memory store (wrapped in Arc<RwLock<>>)
    let memory_store = {
        let store = (*state.agents.system_memory_store).clone();
        if let Err(e) = store.init() {
            tracing::warn!(error = %e, "Failed to init memory store for extraction");
        }
        Arc::new(RwLock::new(store))
    };

    // Create extractor using saved config
    let saved_config = neomind_storage::MemoryConfig::load();
    let config = if req.force {
        tracing::debug!("Using force extraction config (min_messages=1)");
        ExtractionConfig {
            min_messages: 1,
            max_messages: saved_config.extraction.max_messages,
            min_importance: saved_config.extraction.min_importance,
            dedup_enabled: saved_config.extraction.dedup_enabled,
            similarity_threshold: saved_config.extraction.similarity_threshold,
        }
    } else {
        ExtractionConfig {
            min_messages: saved_config.extraction.min_messages,
            max_messages: saved_config.extraction.max_messages,
            min_importance: saved_config.extraction.min_importance,
            dedup_enabled: saved_config.extraction.dedup_enabled,
            similarity_threshold: saved_config.extraction.similarity_threshold,
        }
    };

    let extractor = MemoryExtractor::with_config(memory_store, llm_runtime, config);

    // Spawn background task to avoid HTTP timeout
    tokio::spawn(async move {
        let mut total_extracted = 0;
        let mut processed_sessions = 0;

        for session_id in &session_ids {
            let messages = match session_store.load_history(session_id) {
                Ok(msgs) => msgs,
                Err(e) => {
                    tracing::warn!(session_id = %session_id, error = %e, "Failed to load session history, skipping");
                    continue;
                }
            };

            if messages.is_empty() {
                continue;
            }

            match extractor.extract_from_chat(&messages).await {
                Ok(count) => {
                    total_extracted += count;
                    processed_sessions += 1;
                    tracing::info!(session_id = %session_id, extracted = count, "Extraction completed for session");
                }
                Err(e) => {
                    tracing::error!(session_id = %session_id, error = %e, "Extraction failed for session");
                }
            }
        }

        // Note: compression is handled by the scheduler or manual trigger
        // We don't auto-compress here to prevent over-aggressive memory loss
        if total_extracted > 0 {
            tracing::info!(
                total_extracted = total_extracted,
                "Extraction complete. Compression will run on schedule or can be triggered manually."
            );
        }

        tracing::info!(
            total_extracted = total_extracted,
            processed_sessions = processed_sessions,
            "Background memory extraction complete"
        );

        // Release the lock
        EXTRACTION_RUNNING.store(false, Ordering::SeqCst);
    });

    // Return immediately - extraction runs in background
    Json(ExtractionResponse {
        success: true,
        extracted: 0,
        message: format!(
            "Extraction started for {} session(s). Processing in background.",
            session_count
        ),
    })
    .into_response()
}

/// POST /api/memory/add - Manually add a memory entry
pub async fn add_memory_entry(
    State(state): State<ServerState>,
    Json(req): Json<AddMemoryRequest>,
) -> Response {
    // Parse category
    let category = match MemoryCategory::from_str(&req.category) {
        Some(c) => c,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                format!(
                    "Invalid category: {}. Valid: user_profile, domain_knowledge, task_patterns, system_evolution",
                    req.category
                ),
            )
        }
    };

    let store = state.agents.system_memory_store.clone();

    // Read existing content
    let mut content = match store.read_category(&category) {
        Ok(c) => c,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read category: {}", e),
            )
        }
    };

    // Format the entry
    let timestamp = chrono::Utc::now().format("%Y-%m-%d");
    let entry = format!(
        "- [{}] {} [importance: {}]\n",
        timestamp, req.content, req.importance
    );

    // Append to content
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&entry);

    // Write back
    match store.write_category(&category, &content) {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "message": format!("Memory added to {}", category.display_name())
        }))
        .into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write memory: {}", e),
        ),
    }
}

/// POST /api/memory/compress - Trigger manual compression
pub async fn trigger_compress(State(state): State<ServerState>) -> Response {
    use neomind_agent::memory::compressor::MemoryCompressor;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    tracing::info!("Starting memory compression request");

    // Get the LLM backend
    let llm_manager = match neomind_agent::get_instance_manager() {
        Ok(manager) => {
            tracing::debug!("Got LLM instance manager");
            manager
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get LLM instance manager");
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("LLM backend not available: {}", e),
            );
        }
    };

    let llm_runtime = match llm_manager.get_active_runtime().await {
        Ok(runtime) => {
            tracing::info!(
                model = %runtime.model_name(),
                backend = ?runtime.backend_id(),
                "Got active LLM runtime for compression"
            );
            runtime
        }
        Err(e) => {
            tracing::error!(error = %e, "No active LLM runtime configured");
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                format!("No active LLM backend configured: {}", e),
            );
        }
    };

    // Get the memory store (wrapped in Arc<RwLock<>>)
    let memory_store = {
        let store = (*state.agents.system_memory_store).clone();
        if let Err(e) = store.init() {
            tracing::warn!(error = %e, "Failed to init memory store for compression");
        }
        Arc::new(RwLock::new(store))
    };

    // Create compressor
    let compressor = MemoryCompressor::new(llm_runtime);

    // Compress all categories
    let mut total_result = CompressionResultSummary::default();

    for category in MemoryCategory::all() {
        match compressor.compress(&memory_store, *category).await {
            Ok(result) => {
                tracing::info!(
                    category = ?category,
                    total_before = result.total_before,
                    kept = result.kept,
                    compressed = result.compressed,
                    deleted = result.deleted,
                    "Compression completed for category"
                );
                total_result.total_before += result.total_before;
                total_result.kept += result.kept;
                total_result.compressed += result.compressed;
                total_result.deleted += result.deleted;
            }
            Err(e) => {
                tracing::warn!(
                    category = ?category,
                    error = %e,
                    "Compression failed for category"
                );
            }
        }
    }

    let message = if total_result.total_before == 0 {
        "No memory entries to compress. Extract memories first.".to_string()
    } else {
        format!(
            "Compression completed: {} entries processed, {} compressed, {} deleted",
            total_result.total_before, total_result.compressed, total_result.deleted
        )
    };

    Json(serde_json::json!({
        "success": true,
        "total_before": total_result.total_before,
        "kept": total_result.kept,
        "compressed": total_result.compressed,
        "deleted": total_result.deleted,
        "message": message
    }))
    .into_response()
}

/// Summary of compression results across all categories
#[derive(Debug, Default)]
struct CompressionResultSummary {
    total_before: usize,
    kept: usize,
    compressed: usize,
    deleted: usize,
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
