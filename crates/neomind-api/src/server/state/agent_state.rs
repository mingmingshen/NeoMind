//! AI Agent state.
//!
//! Contains all AI agent-related services:
//! - SessionManager for chat sessions
//! - TieredMemory for conversation history and knowledge
//! - AgentStore for agent persistence
//! - AgentManager for executing user-defined agents
//! - MarkdownMemoryStore for system-level memory
//! - MemoryScheduler for background memory tasks

use std::sync::Arc;
use tokio::sync::RwLock;

use neomind_agent::memory::{MemoryScheduler, TieredMemory};
use neomind_agent::toolkit::ai_metric::AiMetricsRegistry;
use neomind_agent::SessionManager;
use neomind_core::llm::backend::LlmRuntime;
use neomind_storage::{AgentStore, MarkdownMemoryStore, MemoryConfig};

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

    /// Memory scheduler for background extraction/compression (lazy-initialized).
    pub memory_scheduler: Arc<RwLock<Option<MemoryScheduler>>>,

    /// AI metrics registry for the AI metric tool and data handler.
    pub ai_metrics_registry: Arc<AiMetricsRegistry>,
}

impl AgentState {
    /// Create a new agent state.
    pub fn new(
        session_manager: Arc<SessionManager>,
        memory: Arc<RwLock<TieredMemory>>,
        agent_store: Arc<AgentStore>,
        agent_manager: Arc<RwLock<Option<AgentManager>>>,
        system_memory_store: Arc<MarkdownMemoryStore>,
        ai_metrics_registry: Arc<AiMetricsRegistry>,
    ) -> Self {
        Self {
            session_manager,
            memory,
            agent_store,
            agent_manager,
            system_memory_store,
            memory_scheduler: Arc::new(RwLock::new(None)),
            ai_metrics_registry,
        }
    }

    /// Start the memory scheduler with LLM runtime.
    /// Idempotent: if a scheduler is already running, returns Ok without creating a duplicate.
    pub async fn start_memory_scheduler(
        &self,
        llm: Arc<dyn LlmRuntime>,
    ) -> Result<(), String> {
        // Idempotency check — avoid spawning duplicate background tasks
        {
            let guard = self.memory_scheduler.read().await;
            if guard.is_some() {
                tracing::debug!("Memory scheduler already running, skipping");
                return Ok(());
            }
        }

        let config = MemoryConfig::load();

        if !config.enabled {
            tracing::info!("Memory system disabled, not starting scheduler");
            return Ok(());
        }

        let store = Arc::new(RwLock::new((*self.system_memory_store).clone()));
        let manager = Arc::new(RwLock::new(
            neomind_agent::memory::MemoryManager::new(config.clone())
        ));

        // Get session store from session manager for extraction
        let session_store = self.session_manager.session_store();

        let mut scheduler = MemoryScheduler::with_config(
            manager,
            store,
            config,
            llm,
        )
        .with_session_store(session_store);

        scheduler.start();

        let mut scheduler_guard = self.memory_scheduler.write().await;
        *scheduler_guard = Some(scheduler);

        tracing::info!("Memory scheduler started successfully");
        Ok(())
    }

    /// Stop the memory scheduler
    pub async fn stop_memory_scheduler(&self) {
        let mut scheduler_guard = self.memory_scheduler.write().await;
        if let Some(mut scheduler) = scheduler_guard.take() {
            scheduler.stop();
            tracing::info!("Memory scheduler stopped");
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
            memory_scheduler: Arc::new(RwLock::new(None)),
            ai_metrics_registry: AiMetricsRegistry::new(),
        }
    }
}
