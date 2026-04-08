//! Memory scheduler for background extraction and compression tasks
//!
//! Runs periodic tasks for memory extraction from Chat/Agent sources
//! and memory compression for importance decay and summarization.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use super::compressor::{CompressionResult, MemoryCompressor};
use super::manager::MemoryManager;
use crate::memory_extraction::MemoryExtractor;
use neomind_core::llm::backend::LlmRuntime;
use neomind_storage::{MarkdownMemoryStore, MemoryCategory, MemoryConfig, SessionStore};

/// Extraction state file for tracking which sessions have been processed
const EXTRACTION_STATE_FILE: &str = ".extraction_state.json";

/// State tracking for incremental extraction
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct ExtractionState {
    /// Set of session IDs that have been extracted
    extracted_sessions: HashSet<String>,
    /// Timestamp of last extraction run
    last_extraction: Option<String>,
}

/// Memory scheduler for background tasks
pub struct MemoryScheduler {
    manager: Arc<RwLock<MemoryManager>>,
    store: Arc<RwLock<MarkdownMemoryStore>>,
    config: MemoryConfig,
    llm: Arc<dyn LlmRuntime>,
    /// Session store for extracting memories from chat history
    session_store: Option<Arc<SessionStore>>,
    extraction_handle: Option<tokio::task::JoinHandle<()>>,
    compression_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MemoryScheduler {
    /// Create a new scheduler with LLM runtime
    pub fn new(
        manager: Arc<RwLock<MemoryManager>>,
        store: Arc<RwLock<MarkdownMemoryStore>>,
        llm: Arc<dyn LlmRuntime>,
    ) -> Self {
        let config = {
            tokio::task::block_in_place(|| {
                futures::executor::block_on(async { manager.read().await.config().clone() })
            })
        };

        Self {
            manager,
            store,
            config,
            llm,
            session_store: None,
            extraction_handle: None,
            compression_handle: None,
        }
    }

    /// Create scheduler with explicit config
    pub fn with_config(
        manager: Arc<RwLock<MemoryManager>>,
        store: Arc<RwLock<MarkdownMemoryStore>>,
        config: MemoryConfig,
        llm: Arc<dyn LlmRuntime>,
    ) -> Self {
        Self {
            manager,
            store,
            config,
            llm,
            session_store: None,
            extraction_handle: None,
            compression_handle: None,
        }
    }

    /// Set the session store for chat history extraction
    pub fn with_session_store(mut self, session_store: Arc<SessionStore>) -> Self {
        self.session_store = Some(session_store);
        self
    }

    /// Start background tasks
    pub fn start(&mut self) {
        if !self.config.enabled {
            info!("Memory system disabled, not starting scheduler");
            return;
        }

        // Start extraction task
        if self.config.schedule.extraction_enabled {
            let manager = self.manager.clone();
            let store = self.store.clone();
            let llm = self.llm.clone();
            let session_store = self.session_store.clone();
            let interval_secs = self.config.schedule.extraction_interval_secs;

            self.extraction_handle = Some(tokio::spawn(async move {
                let mut timer = interval(Duration::from_secs(interval_secs));

                info!(
                    interval_secs = interval_secs,
                    has_session_store = session_store.is_some(),
                    "Memory extraction scheduler started"
                );

                loop {
                    timer.tick().await;

                    info!("Scheduled memory extraction triggered");

                    match Self::run_extraction(&manager, &store, &llm, &session_store).await {
                        Ok(count) => {
                            info!(entries_extracted = count, "Extraction completed");
                        }
                        Err(e) => {
                            error!(error = %e, "Extraction failed");
                        }
                    }
                }
            }));
        }

        // Start compression task
        if self.config.schedule.compression_enabled {
            let store = self.store.clone();
            let llm = self.llm.clone();
            let interval_secs = self.config.schedule.compression_interval_secs;

            self.compression_handle = Some(tokio::spawn(async move {
                let mut timer = interval(Duration::from_secs(interval_secs));

                info!(
                    interval_secs = interval_secs,
                    model = %llm.model_name(),
                    "Memory compression scheduler started"
                );

                loop {
                    timer.tick().await;

                    info!("Scheduled memory compression triggered");

                    match Self::run_compression(&store, &llm).await {
                        Ok(result) => {
                            info!(
                                total_before = result.total_before,
                                kept = result.kept,
                                compressed = result.compressed,
                                deleted = result.deleted,
                                "Compression completed"
                            );
                        }
                        Err(e) => {
                            error!(error = %e, "Compression failed");
                        }
                    }
                }
            }));
        }
    }

    /// Run extraction on sessions (incremental — skips already processed sessions)
    async fn run_extraction(
        _manager: &Arc<RwLock<MemoryManager>>,
        store: &Arc<RwLock<MarkdownMemoryStore>>,
        llm: &Arc<dyn LlmRuntime>,
        session_store: &Option<Arc<SessionStore>>,
    ) -> Result<usize, String> {
        let Some(session_store) = session_store else {
            info!("No session store configured, skipping extraction");
            return Ok(0);
        };

        // Get all sessions
        let sessions = session_store
            .list_sessions()
            .map_err(|e| format!("Failed to list sessions: {}", e))?;

        if sessions.is_empty() {
            info!("No sessions found for extraction");
            return Ok(0);
        }

        // Load extraction state for incremental processing
        let state_path = {
            let store_guard = store.read().await;
            store_guard.base_path().join(EXTRACTION_STATE_FILE)
        };
        let mut state = Self::load_extraction_state(&state_path);

        // Filter to only new sessions
        let new_sessions: Vec<String> = sessions
            .into_iter()
            .filter(|s| !state.extracted_sessions.contains(s))
            .collect();

        if new_sessions.is_empty() {
            info!("No new sessions to extract (all already processed)");
            return Ok(0);
        }

        info!(
            total_sessions = new_sessions.len(),
            previously_extracted = state.extracted_sessions.len(),
            "Starting incremental memory extraction"
        );

        // Create extractor
        let extractor = MemoryExtractor::new(store.clone(), llm.clone());

        let mut total_extracted = 0;
        let mut processed = 0;
        let mut skipped_no_messages = 0;
        let mut errors = 0;

        for session_id in new_sessions {
            // Load session history
            match session_store.load_history(&session_id) {
                Ok(messages) => {
                    if messages.len() < 3 {
                        // Skip sessions with too few messages but still mark as processed
                        skipped_no_messages += 1;
                        state.extracted_sessions.insert(session_id);
                        continue;
                    }

                    processed += 1;

                    // Extract memories from chat
                    match extractor.extract_from_chat(&messages).await {
                        Ok(count) => {
                            total_extracted += count;
                            state.extracted_sessions.insert(session_id.clone());
                            info!(
                                session_id = %session_id,
                                extracted = count,
                                message_count = messages.len(),
                                "Extracted memories from session"
                            );
                        }
                        Err(e) => {
                            errors += 1;
                            warn!(
                                session_id = %session_id,
                                error = %e,
                                "Failed to extract from session"
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        session_id = %session_id,
                        error = %e,
                        "Failed to load session history"
                    );
                }
            }
        }

        // Update and save extraction state
        state.last_extraction = Some(chrono::Utc::now().to_rfc3339());
        if let Err(e) = Self::save_extraction_state(&state_path, &state) {
            warn!(error = %e, "Failed to save extraction state");
        }

        info!(
            total_extracted = total_extracted,
            sessions_processed = processed,
            sessions_skipped = skipped_no_messages,
            errors = errors,
            total_extracted_sessions = state.extracted_sessions.len(),
            "Memory extraction completed"
        );

        Ok(total_extracted)
    }

    /// Load extraction state from file
    fn load_extraction_state(path: &PathBuf) -> ExtractionState {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(state) => state,
                    Err(e) => {
                        warn!(error = %e, "Failed to parse extraction state, starting fresh");
                        ExtractionState::default()
                    }
                }
            }
            Err(_) => ExtractionState::default(),
        }
    }

    /// Save extraction state to file
    fn save_extraction_state(path: &PathBuf, state: &ExtractionState) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(state)?;
        std::fs::write(path, content)
    }

    /// Run compression on all categories
    async fn run_compression(
        store: &Arc<RwLock<MarkdownMemoryStore>>,
        llm: &Arc<dyn LlmRuntime>,
    ) -> Result<CompressionResult, String> {
        let compressor = MemoryCompressor::new(llm.clone());
        let mut total_result = CompressionResult::default();

        for category in MemoryCategory::all() {
            match compressor.compress(store, category.clone()).await {
                Ok(result) => {
                    total_result.total_before += result.total_before;
                    total_result.kept += result.kept;
                    total_result.compressed += result.compressed;
                    total_result.deleted += result.deleted;
                }
                Err(e) => {
                    warn!(
                        category = ?category,
                        error = %e,
                        "Compression failed for category"
                    );
                }
            }
        }

        Ok(total_result)
    }

    /// Stop background tasks
    pub fn stop(&mut self) {
        if let Some(handle) = self.extraction_handle.take() {
            handle.abort();
            info!("Extraction scheduler stopped");
        }

        if let Some(handle) = self.compression_handle.take() {
            handle.abort();
            info!("Compression scheduler stopped");
        }
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.extraction_handle.is_some() || self.compression_handle.is_some()
    }
}

impl Drop for MemoryScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock LLM for testing
    struct MockLlm;

    #[async_trait::async_trait]
    impl LlmRuntime for MockLlm {
        fn backend_id(&self) -> neomind_core::llm::backend::BackendId {
            neomind_core::llm::backend::BackendId::new("mock")
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }

        async fn generate(
            &self,
            _input: neomind_core::llm::backend::LlmInput,
        ) -> std::result::Result<neomind_core::llm::backend::LlmOutput, neomind_core::llm::backend::LlmError> {
            Ok(neomind_core::llm::backend::LlmOutput {
                text: r#"{"summaries":[{"content":"Test summary","importance":70}]}"#.to_string(),
                finish_reason: neomind_core::llm::backend::FinishReason::Stop,
                usage: None,
                thinking: None,
            })
        }

        async fn generate_stream(
            &self,
            _input: neomind_core::llm::backend::LlmInput,
        ) -> std::result::Result<std::pin::Pin<Box<dyn futures::Stream<Item = neomind_core::llm::backend::StreamChunk> + Send>>, neomind_core::llm::backend::LlmError> {
            unimplemented!()
        }

        fn max_context_length(&self) -> usize {
            4096
        }
    }

    #[test]
    fn test_scheduler_creation() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();
        config.schedule.extraction_interval_secs = 1;
        config.schedule.compression_interval_secs = 1;

        let manager = Arc::new(RwLock::new(MemoryManager::new(config.clone())));
        let store = Arc::new(RwLock::new(MarkdownMemoryStore::new(temp.path())));
        let llm = Arc::new(MockLlm);
        let scheduler = MemoryScheduler::with_config(manager, store, config, llm);

        assert!(!scheduler.is_running());
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();
        config.schedule.extraction_interval_secs = 60; // Long enough not to trigger
        config.schedule.compression_interval_secs = 60;

        let manager = Arc::new(RwLock::new(MemoryManager::new(config.clone())));
        let store = Arc::new(RwLock::new(MarkdownMemoryStore::new(temp.path())));
        let llm = Arc::new(MockLlm);
        let mut scheduler = MemoryScheduler::with_config(manager, store, config, llm);

        scheduler.start();
        assert!(scheduler.is_running());

        scheduler.stop();
        assert!(!scheduler.is_running());
    }

    #[test]
    fn test_disabled_scheduler() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();
        config.enabled = false;

        let manager = Arc::new(RwLock::new(MemoryManager::new(config.clone())));
        let store = Arc::new(RwLock::new(MarkdownMemoryStore::new(temp.path())));
        let llm = Arc::new(MockLlm);
        let mut scheduler = MemoryScheduler::with_config(manager, store, config, llm);

        scheduler.start();
        assert!(!scheduler.is_running()); // Should not start when disabled
    }
}
