//! Backward compatibility layer for memory extraction
//!
//! This module provides stubs for the old memory extraction API
//! during the transition to the new category-based system.

use neomind_storage::AgentExecutionRecord;
use std::sync::Arc;
use tracing::debug;

use crate::error::Result;

/// Persist agent memory (stub for backward compatibility)
///
/// This is a temporary stub that logs but does nothing.
/// Memory extraction is now handled by the new category-based system
/// via MemoryManager and AgentExtractor.
pub async fn persist_agent_memory(
    _memory_store: &Arc<neomind_storage::MarkdownMemoryStore>,
    record: &AgentExecutionRecord,
    agent_name: &str,
) -> Result<()> {
    debug!(
        agent_id = %record.agent_id,
        agent_name = %agent_name,
        "persist_agent_memory called - extraction now handled by MemoryManager"
    );
    // Stub - do nothing for now
    // The new category-based extraction will be triggered separately
    // through the MemoryScheduler or manual API calls
    Ok(())
}

/// Persist chat memory (stub for backward compatibility)
///
/// This is a temporary stub that logs but does nothing.
/// Memory extraction is now handled by the new category-based system.
pub async fn persist_chat_memory(
    _memory_store: &Arc<neomind_storage::MarkdownMemoryStore>,
    session_id: &str,
    _summary: Option<&str>,
) -> Result<()> {
    debug!(
        session_id = %session_id,
        "persist_chat_memory called - extraction now handled by MemoryManager"
    );
    // Stub - do nothing for now
    Ok(())
}
