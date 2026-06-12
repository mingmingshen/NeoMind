//! Core traits and types for NeoMind.
//!
//! This crate defines the foundational abstractions used across the project.

// alerts module removed - use neomind_messages instead
pub mod brand;
pub mod config;
pub mod datasource;
pub mod error;
pub mod event;
pub mod eventbus;
pub mod extension;
pub mod llm;
pub mod message;
pub mod storage;
pub mod tools;

pub use llm::LlmError;

// Exports
pub use llm::backend::{
    BackendCapabilities, BackendId, FinishReason, GenerationParams,
    LlmInput as LlmRuntimeInput, LlmOutput, LlmRuntime, StreamChunk, TokenUsage,
};
pub use llm::modality::{ImageContent, ImageFormat, ImageInput, ModalityContent};

pub use message::{Content, ContentPart, ImageDetail, Message, MessageRole};

// Event exports
pub use event::{EventMetadata, MetricValue, NeoMindEvent, ProposedAction};

// Event bus exports
pub use eventbus::{
    EventBus, EventBusReceiver, FilterBuilder, FilteredReceiver,
    SharedEventBus, DEFAULT_CHANNEL_CAPACITY,
};

