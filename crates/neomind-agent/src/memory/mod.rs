//! Memory module for the NeoMind agent.
//!
//! Provides markdown-based memory with LLM extraction, dedup, compression,
//! and a scheduler for background maintenance tasks.

pub mod compressor;
pub mod dedup;
pub mod error;
pub mod extractor;
pub mod manager;
pub mod scheduler;
pub mod security;
pub mod snapshot;

// Re-export commonly used types
pub use error::{MemoryError, NeoMindError, Result};

// Memory manager exports
pub use manager::MemoryManager;

// Memory extractor/compressor/dedup exports
pub use compressor::{evict_to_limit, EvictionResult};
pub use dedup::{DedupProcessor, DedupResult};
pub use extractor::{parse_category, AgentExtractor, ExtractResult, MemoryCandidate};

// Memory scheduler export
pub use scheduler::MemoryScheduler;

// Memory snapshot export
pub use snapshot::MemorySnapshot;

// Memory security export
pub use security::{MemorySecurityScanner, SecurityScanResult};
