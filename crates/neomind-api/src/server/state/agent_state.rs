//! AI Agent state.
//!
//! Contains all AI agent-related services:
//! - SessionManager for chat sessions
//! - TieredMemory for conversation history and knowledge
//! - AgentStore for agent persistence
//! - AgentManager for executing user-defined agents
//! - MarkdownMemoryStore for system-level memory

use std::sync::Arc;
use tokio::sync::RwLock;

use neomind_agent::memory::TieredMemory;
use neomind_agent::SessionManager;
use neomind_storage::{AgentStore, MarkdownMemoryStore};

/// AI Agent manager type alias.
pub type AgentManager = Arc<neomind_agent::ai_agent::AiAgentManager>;

/// AI Agent state.
///
/// Provides access to session management, memory, and agent execution.
#[derive(Clone)]
pub struct AgentState {
    /// Session manager for chat sessions.
    pub session_manager: Arc<SessionManager>,

    /// Tiered memory system for conversation history and knowledge.
    pub memory: Arc<RwLock<TieredMemory>>,

    /// AI Agent store for user-defined automation agents.
    pub agent_store: Arc<AgentStore>,

    /// AI Agent manager for executing user-defined agents (lazy-initialized).
    pub agent_manager: Arc<RwLock<Option<AgentManager>>>,

    /// System memory store for Markdown-based persistent memory.
    pub system_memory_store: Arc<MarkdownMemoryStore>,
}

impl AgentState {
    /// Create a new agent state.
    pub fn new(
        session_manager: Arc<SessionManager>,
        memory: Arc<RwLock<TieredMemory>>,
        agent_store: Arc<AgentStore>,
        agent_manager: Arc<RwLock<Option<AgentManager>>>,
        system_memory_store: Arc<MarkdownMemoryStore>,
    ) -> Self {
        Self {
            session_manager,
            memory,
            agent_store,
            agent_manager,
            system_memory_store,
        }
    }

    /// Create a minimal agent state (for testing).
    #[cfg(test)]
    pub fn minimal() -> Self {
        Self {
            session_manager: Arc::new(SessionManager::memory()),
            memory: Arc::new(RwLock::new(TieredMemory::default())),
            agent_store: AgentStore::memory().unwrap(),
            agent_manager: Arc::new(RwLock::new(None)),
            system_memory_store: Arc::new(MarkdownMemoryStore::new(
                std::env::temp_dir().join("test-memory"),
            )),
        }
    }
}
