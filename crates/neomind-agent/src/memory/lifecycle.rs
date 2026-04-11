//! Memory lifecycle hooks for managing memory operations across session stages.
//!
//! Provides hooks for session start (load snapshot), turn complete (sync to mid-term),
//! and session end (trigger extraction). Only active when memory is enabled.

use neomind_storage::MarkdownMemoryStore;

use super::snapshot::MemorySnapshot;

/// Trait for memory lifecycle management.
#[allow(async_fn_in_trait)]
pub trait MemoryLifecycle: Send + Sync {
    /// Called when a session starts with memory enabled.
    /// Loads the frozen snapshot and preheats mid-term memory.
    fn on_session_start(&self, session_id: &str) -> Option<MemorySnapshot>;

    /// Called when a turn completes with memory enabled.
    /// Syncs the turn to mid-term memory for future retrieval.
    async fn on_turn_complete(
        &self,
        session_id: &str,
        user_msg: &str,
        assistant_msg: &str,
    );

    /// Called when a session ends with memory enabled.
    /// Triggers memory extraction for the session.
    async fn on_session_end(&self, session_id: &str);
}

/// Default implementation using MarkdownMemoryStore.
pub struct DefaultMemoryLifecycle {
    store: MarkdownMemoryStore,
}

impl DefaultMemoryLifecycle {
    /// Create a new lifecycle manager.
    pub fn new(store: MarkdownMemoryStore) -> Self {
        Self { store }
    }
}

impl MemoryLifecycle for DefaultMemoryLifecycle {
    fn on_session_start(&self, _session_id: &str) -> Option<MemorySnapshot> {
        MemorySnapshot::load_opt(&self.store)
    }

    async fn on_turn_complete(
        &self,
        _session_id: &str,
        _user_msg: &str,
        _assistant_msg: &str,
    ) {
        // Mid-term memory sync is handled by the existing consolidation flow
        // in process_stream_to_channel. This hook is for future expansion.
    }

    async fn on_session_end(&self, _session_id: &str) {
        // Memory extraction on session end is handled by the existing
        // MemoryScheduler periodic extraction. This hook is for future expansion.
    }
}
