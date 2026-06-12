//! Edge AI Storage Crate
//!
//! This crate provides storage capabilities for the NeoMind platform.
//!
//! ## Features
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `redb` | ✅ | Persistent storage using redb |
//! | `hnsw` | ❌ | Vector search with HNSW |
//! | `all` | ❌ | All features |
//!
//! ## Example
//!
//! ```rust,no_run
//! use neomind_storage::{
//!     TimeSeriesStore, DataPoint,
//!     VectorStore, VectorDocument,
//!     SessionStore, SessionMessage,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Time series storage
//!     let ts_store = TimeSeriesStore::memory()?;
//!     let point = DataPoint::new(1234567890, 23.5);
//!     ts_store.write("sensor1", "temperature", point).await?;
//!
//!     // Vector storage
//!     let vec_store = VectorStore::new();
//!     let doc = VectorDocument::new("doc1", vec![0.1, 0.2, 0.3, 0.4]);
//!     vec_store.insert(doc).await?;
//!
//!     // Session storage
//!     let session_store = SessionStore::open(":memory:")?;
//!     session_store.save_session_id("chat-1")?;
//!     let messages = vec![
//!         SessionMessage::user("Hello"),
//!         SessionMessage::assistant("Hi there!"),
//!     ];
//!     session_store.save_history("chat-1", &messages)?;
//!
//!     Ok(())
//! }
//! ```

pub mod agents;
pub mod business;
pub mod dashboards;
pub mod device_registry;
pub mod error;
pub mod extensions;
pub mod frontend_components;
pub mod instances;
pub mod llm_backends;
pub mod memory_config;
pub mod messages;
pub mod session;
pub mod settings;
pub mod system_memory;
pub mod timeseries;
pub mod vector;

// Re-exports
pub use error::{Error, Result};

pub use timeseries::{
    compress_series_adaptive, BatchWriteRequest, DataPoint, PerformanceStats, RetentionPolicy,
    RetentionPolicyCleanupResult, TimeSeriesBucket, TimeSeriesConfig, TimeSeriesResult,
    TimeSeriesStore,
};

pub use vector::{
    Embedding, PersistentVectorStore, SearchResult, SimilarityMetric, VectorDocument, VectorStore,
};

pub use session::{
    PendingStreamState, SessionMessage, SessionMessageImage, SessionMetadata, SessionStore,
    StreamStage,
};

pub use messages::{MessageStats, MessageStore, StoredMessage};

pub use settings::{
    ConfigChangeEntry,
    ExternalBroker,
    LlmBackendType,
    LlmSettings,
    MqttSettings,
    SecurityLevel,
    SecurityWarning,
    SettingsStore,
    // Timezone settings
    DEFAULT_GLOBAL_TIMEZONE,
    KEY_GLOBAL_TIMEZONE,
    KEY_LLM_CONFIG,
    KEY_MQTT_CONFIG,
};

pub use llm_backends::{
    BackendCapabilities, ConnectionTestResult, LlmBackendInstance, LlmBackendStats, LlmBackendStore,
};

pub use instances::{InstanceRecord, InstanceStore};

pub use extensions::{ExtensionRecord, ExtensionStats, ExtensionStore};

pub use frontend_components::{
    ComponentManifest, FrontendComponentStore, MarketComponentEntry, MarketIndex, SizeConstraints,
};

pub use agents::{
    ActionExecuted,
    AgentExecutionRecord,
    AgentFilter,
    AgentMemory,
    AgentResource,
    AgentSchedule,
    AgentStats,
    AgentStatus,
    AgentStore,
    AgentToolConfig,
    AiAgent,
    // Conversation types
    DataCollected,
    DataSummary,
    Decision,
    DecisionProcess,
    ExecutionFilter,
    ExecutionJournal,
    ExecutionMode,
    ExecutionRecord,
    ExecutionResult,
    ExecutionStatus,
    GeneratedReport,
    IntentType,
    KnowledgeFileRef,
    NotificationSent,
    ParsedIntent,
    ReasoningStep,
    ResourceType,
    ScheduleType,
    UserMessage,
};

pub use business::{
    Alert, AlertFilter, AlertStatus, AlertStore, EventSeverity, RuleExecution, RuleExecutionResult,
    RuleExecutionStats, RuleHistoryStore,
};

pub use device_registry::{
    CommandDefinition, CommandHistoryRecord, CommandStatus, ConnectionConfig, DeviceConfig,
    DeviceRegistryStore, DeviceTypeTemplate, MetricDataType, MetricDefinition, ParamMetricValue,
    ParameterDefinition,
};

pub use dashboards::{
    default_templates, ComponentPosition, Dashboard, DashboardLayout, DashboardStore,
    DashboardTemplate, LayoutBreakpoints, RequiredResources, RowsValue,
};

// System memory exports (Markdown-based)
pub use system_memory::{
    AggregatedMemory, CategoryStats, MarkdownMemoryStore, MemoryCategory,
    MemoryEntry as SystemMemoryEntry, MemoryFileInfo, MemorySource, DEFAULT_MIN_IMPORTANCE,
    MAX_MEMORY_ENTRIES,
};

// Memory configuration exports
pub use memory_config::MemoryConfig;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// # Changelog
///
/// ## v0.2.0 (2026-01) - Storage Migration
///
/// ### Breaking Changes
/// - **Migrated from sled to redb 2.1** - All database files have a new format
/// - `VectorStore` serialization changed from bincode to JSON (for `serde_json::Value` compatibility)
///
/// ### New Features
/// - **Session storage** - New `SessionStore` for chat history persistence
/// - **Multimodal storage** - New `MultimodalStore` for image and document storage
/// - **Composite key queries** - TimeSeriesStore now supports efficient range queries
///
/// ### Migration Notes
/// - Old sled databases are **not compatible** with redb
/// - Use `SessionStore::open(":memory:")` for in-memory storage
/// - Vector document metadata is now serialized as JSON
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
