//! Edge AI Agent Crate
//!
//! This crate provides the main AI Agent that orchestrates LLM, memory, and tools.
//!
//! ## Features
//!
//! - **Agent Coordination**: Integrates LLM, memory, and tools
//! - **Session Management**: Multi-session support with isolation
//! - **Tool Calling**: Function calling with built-in tools
//! - **Memory Integration**: Short-term conversation history
//!
//! ## Example
//!
//! ```rust,no_run
//! use neomind_agent::{SessionManager, AgentConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let manager = SessionManager::new()?;
//!
//!     // Create a new session
//!     let session_id = manager.create_session().await?;
//!
//!     // Process a message
//!     let response = manager.process_message(
//!         &session_id,
//!         "列出所有设备"
//!     ).await?;
//!
//!     println!("Response: {}", response.message.content);
//!     println!("Tools used: {:?}", response.tools_used);
//!
//!     Ok(())
//! }
//! ```

pub mod agent;
pub mod ai_agent;
pub mod config;
pub mod context;
pub mod context_selector;
pub mod error;
pub mod llm;
pub mod llm_backends; // Merged from neomind-llm crate
pub mod memory;
pub mod memory_extraction;
pub mod prompts;
pub mod session;
pub mod skills;
pub mod smart_conversation;
pub mod toolkit;
pub mod tools;
pub mod translation;

// Re-export commonly used types
pub use agent::{
    default_fallback_rules, process_fallback, Agent, AgentConfig, AgentEvent, AgentMessage,
    AgentResponse, FallbackRule, LlmBackend, SessionState, ToolCall,
};
// Re-export planner types
pub use agent::planner::{
    ExecutionPlan, KeywordPlanner, LLMPlanner, PlanningConfig, PlanningCoordinator, PlanningMode,
    PlanStep, StepId,
};
// Re-export staged types
pub use agent::staged::{IntentCategory, IntentResult};
// Re-export context selector types for planning tests
pub use context_selector::ContextBundle;
pub use ai_agent::IntentParser;
pub use ai_agent::AgentInput;
pub use config::{get_default_config, set_default_config, StreamingConfig};
pub use error::{NeoMindError, Result};
pub use session::SessionManager;
pub use tools::{
    resolve_tool_name as map_tool_name, EventIntegratedToolRegistry, ToolExecutionHistory,
    ToolExecutionRecord, ToolExecutionStats, ToolNameMapper,
};

// Re-export llm_backends types for backward compatibility (merged from neomind-llm crate)
pub use llm_backends::{
    get_instance_manager, BackendTypeDefinition, CloudConfig, CloudProvider, CloudRuntime,
    LlmBackendInstanceManager, OllamaConfig, OllamaRuntime,
};

// Re-export memory extraction types
pub use memory_extraction::{add_memory, ExtractionConfig, MemoryExtractor};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[tokio::test]
    async fn test_integration() {
        // Create a temporary directory for the test
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join(format!("neomind_test_{}", uuid::Uuid::new_v4()));

        let manager = SessionManager::with_path(test_path).unwrap();

        // Create a session
        let session_id = manager.create_session().await.unwrap();
        assert!(!session_id.is_empty());

        // Process messages
        let response1 = manager
            .process_message(&session_id, "列出设备")
            .await
            .unwrap();
        assert!(!response1.message.content.is_empty());

        let response2 = manager
            .process_message(&session_id, "列出规则")
            .await
            .unwrap();
        assert!(!response2.message.content.is_empty());

        // Check history
        let history = manager.get_history(&session_id).await.unwrap();
        assert!(history.len() >= 4); // 2 user + 2 assistant messages
    }
}
