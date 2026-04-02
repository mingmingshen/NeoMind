//! Edge AI Memory Crate
//!
//! This crate provides tiered memory management for the NeoMind platform.
//!
//! ## Features
//!
//! - **Short-term Memory**: Current conversation context with token limits
//! - **Mid-term Memory**: Recent conversation history with semantic search
//! - **Long-term Memory**: Device knowledge base and troubleshooting guides
//! - **Unified Interface**: Single interface to all memory layers
//! - **Markdown Memory**: Simple markdown-based memory with LLM extraction
//!
//! ## Example
//!
//! ```rust,no_run
//! use neomind_memory::{
//!     TieredMemory, KnowledgeEntry, KnowledgeCategory,
//!     short_term::MemoryMessage,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut memory = TieredMemory::new();
//!
//!     // Add to short-term memory
//!     memory.add_message("user", "What is the temperature?")?;
//!     memory.add_message("assistant", "The temperature is 25°C")?;
//!
//!     // Consolidate to mid-term
//!     memory.consolidate("session_1").await?;
//!
//!     // Add to long-term memory
//!     let entry = KnowledgeEntry::new(
//!         "Temperature Sensor Manual",
//!         "Instructions for using temperature sensors...",
//!         KnowledgeCategory::DeviceManual,
//!     );
//!     memory.add_knowledge(entry).await?;
//!
//!     // Query all layers
//!     let results = memory.query_all("temperature", 5).await;
//!     println!("Found {} results", results.short_term.len() + results.mid_term.len() + results.long_term.len());
//!
//!     Ok(())
//! }
//! ```

pub mod bm25;
pub mod embeddings;
pub mod error;
pub mod long_term;
pub mod manager;
pub mod mid_term;
pub mod short_term;
pub mod tiered;

// Re-export commonly used types
pub use bm25::{
    extract_text_for_bm25, BM25Index, BM25Result, DocumentStats, DEFAULT_B, DEFAULT_K1,
};
pub use embeddings::{
    cosine_similarity, create_embedding_model, dot_similarity, CachedEmbeddingModel,
    EmbeddingConfig, EmbeddingModel, EmbeddingProvider, LocalEmbedding, ModelInfo, OllamaEmbedding,
    OpenAIEmbedding, SimpleEmbedding,
};
pub use error::{MemoryError, NeoMindError, Result};

pub use long_term::{
    KnowledgeCategory, KnowledgeEntry, LongTermMemory, SolutionStep, TroubleshootingCase,
};
pub use mid_term::{ConversationEntry, MidTermMemory, SearchResult};
pub use short_term::{MemoryMessage, ShortTermMemory, DEFAULT_MAX_MESSAGES, DEFAULT_MAX_TOKENS};
pub use tiered::{MemoryQueryResult, MemoryStats, SearchMethod, TieredMemory, TieredMemoryConfig};

// Memory manager exports
pub use manager::MemoryManager;

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
        let mut memory = TieredMemory::new();

        // Short-term
        memory.add_message("user", "Test message").unwrap();

        // Mid-term
        memory
            .add_conversation("session1", "Question", "Answer")
            .await
            .unwrap();

        // Long-term
        let entry = KnowledgeEntry::new("Test", "Content", KnowledgeCategory::BestPractice);
        memory.add_knowledge(entry).await.unwrap();

        // Check stats
        let stats = memory.get_stats().await;
        assert!(stats.short_term_messages > 0);
        assert!(stats.mid_term_entries > 0);
        assert!(stats.long_term_entries > 0);
    }
}
