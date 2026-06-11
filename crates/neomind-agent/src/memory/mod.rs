//! Memory module for the NeoMind agent.
//!
//! Provides markdown-based memory with LLM extraction, dedup, compression,
//! and a scheduler for background maintenance tasks.

pub mod compressor;
pub mod dedup;
pub mod error;
pub mod extractor;
pub mod scheduler;
pub mod security;
pub mod snapshot;

// Re-exports consumed via shortcut path (crate::memory::TypeName)
pub use scheduler::MemoryScheduler;
pub use snapshot::MemorySnapshot;
