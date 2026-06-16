//! Abstract LLM runtime backend.
//!
//! This module defines the core abstraction for LLM inference,
//! supporting multiple backends (Hailo, Candle, Cloud, etc.).

use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

use super::modality::{ImageContent, ModalityContent};
use crate::message::{Message, MessageRole};

/// LLM backend identifier.
///
/// This is a dynamic string identifier instead of an enum,
/// allowing backends to be registered at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BackendId(String);

impl BackendId {
    /// Create a new backend ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Common backend IDs.
    pub const OLLAMA: &'static str = "ollama";
    pub const OPENAI: &'static str = "openai";
    pub const QWEN: &'static str = "qwen";
    pub const CANDLE: &'static str = "candle";
    pub const HAILO: &'static str = "hailo";
    pub const MOCK: &'static str = "mock";
    pub const LLAMACPP: &'static str = "llamacpp";
}

impl AsRef<str> for BackendId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for BackendId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for BackendId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Generation parameters.
#[derive(Debug, Clone)]
pub struct GenerationParams {
    /// Temperature (0.0 - 2.0)
    pub temperature: Option<f32>,

    /// Top-p sampling (0.0 - 1.0)
    pub top_p: Option<f32>,

    /// Top-k sampling
    pub top_k: Option<u32>,

    /// Maximum tokens to generate
    pub max_tokens: Option<usize>,

    /// Stop sequences
    pub stop: Option<Vec<String>>,

    /// Frequency penalty (-2.0 - 2.0)
    pub frequency_penalty: Option<f32>,

    /// Presence penalty (-2.0 - 2.0)
    pub presence_penalty: Option<f32>,

    /// Enable thinking/reasoning mode (for models that support it like qwen3-vl)
    pub thinking_enabled: Option<bool>,

    /// Maximum context window size in tokens
    /// CRITICAL for Qwen3: must be >= 16384 to avoid infinite repetition loops
    pub max_context: Option<usize>,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: None,
            max_tokens: Some(usize::MAX),
            stop: None,
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            thinking_enabled: None, // Let backend decide based on model capabilities
            max_context: None,      // Let backend decide based on model capabilities
        }
    }
}

/// Tool definition for LLM function calling.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Parameters as JSON Schema
    pub parameters: serde_json::Value,
}

/// Sanitize a tool name for OpenAI-compatible API compatibility.
///
/// OpenAI-compatible APIs require `function.name` to match `^[a-zA-Z0-9_-]+$`.
/// Extension tools use `{extension_id}:{command_name}` format (e.g. `test.extension:test_command`)
/// which contains `.` and `:`. This function replaces any character that is not
/// alphanumeric, underscore, or hyphen with an underscore.
pub fn sanitize_tool_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// LLM input.
#[derive(Debug, Clone)]
pub struct LlmInput {
    /// Messages for the conversation
    pub messages: Vec<Message>,

    /// Generation parameters
    pub params: GenerationParams,

    /// Model identifier (backend-specific)
    pub model: Option<String>,

    /// Stream response
    pub stream: bool,

    /// Tool definitions for function calling (optional)
    pub tools: Option<Vec<ToolDefinition>>,
}

impl LlmInput {
    /// Create a new input with a single user message.
    pub fn new(content: impl Into<ModalityContent>) -> Self {
        Self {
            messages: vec![Message::user(content.into().as_text())],
            params: GenerationParams::default(),
            model: None,
            stream: false,
            tools: None,
        }
    }

    /// Add a message to the conversation.
    pub fn with_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add messages to the conversation.
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Set generation parameters.
    pub fn with_params(mut self, params: GenerationParams) -> Self {
        self.params = params;
        self
    }

    /// Set model identifier.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Enable streaming.
    pub fn with_streaming(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Set tool definitions for function calling.
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Add multimodal content to user message.
    pub fn with_image(mut self, _image: ImageContent) -> Self {
        if let Some(msg) = self.messages.last_mut() {
            if msg.role == MessageRole::User {
                // Convert to multimodal content
                let text = msg.text();
                msg.content = crate::message::Content::text(format!(
                    "{} <image>",
                    if text.is_empty() {
                        "Describe this image."
                    } else {
                        &text
                    }
                ));
            }
        }
        self
    }
}

/// LLM output.
#[derive(Debug, Clone)]
pub struct LlmOutput {
    /// Generated text content
    pub text: String,

    /// Finish reason (stop, length, error)
    pub finish_reason: FinishReason,

    /// Tokens used (prompt + completion)
    pub usage: Option<TokenUsage>,

    /// Thinking content (for models that support reasoning/thinking)
    pub thinking: Option<String>,

    /// Native structured tool calls from the API response.
    /// Populated by backends that support native tool calling (OpenAI, Ollama, llama.cpp).
    /// Each entry is a JSON object with `id`, `name`, and `arguments` fields.
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

/// Finish reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FinishReason {
    /// Model stopped naturally
    Stop,

    /// Max tokens reached
    Length,

    /// Model hit an error
    Error,

    /// Content filter triggered
    ContentFilter,

    /// Model wants to call tools
    ToolCalls,
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

/// Stream chunk.
/// Contains the text content and a boolean indicating if it's from a "thinking" field
/// (e.g., qwen3-vl's thinking field vs actual content).
pub type StreamChunk = Result<(String, bool), LlmError>;

/// Stream configuration for LLM backends.
///
/// This configuration controls timeouts, thinking limits, and progress reporting
/// for streaming responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Maximum thinking characters before cutoff.
    ///
    /// When the model generates more than this many characters in the "thinking" field,
    /// the remaining thinking content will be skipped and the system will wait for
    /// the actual content to begin. This prevents models from getting stuck in
    /// infinite thinking loops.
    ///
    /// Default: 50,000 characters
    #[serde(default = "StreamConfig::default_max_thinking_chars")]
    pub max_thinking_chars: usize,

    /// Maximum thinking time in seconds.
    ///
    /// If the model spends more than this time generating thinking content,
    /// the system will skip remaining thinking and wait for content.
    ///
    /// Default: 120 seconds
    #[serde(default = "StreamConfig::default_max_thinking_time_secs")]
    pub max_thinking_time_secs: u64,

    /// Total stream timeout in seconds.
    ///
    /// The entire streaming operation (thinking + content generation) must
    /// complete within this time limit.
    ///
    /// Default: 300 seconds (5 minutes)
    #[serde(default = "StreamConfig::default_max_stream_duration_secs")]
    pub max_stream_duration_secs: u64,

    /// Progressive warning thresholds in seconds.
    ///
    /// The system will send progress warnings at these elapsed times.
    /// This helps users understand long-running operations.
    ///
    /// Default: [60, 120, 180, 240] seconds
    #[serde(default = "StreamConfig::default_warning_thresholds")]
    pub warning_thresholds: Vec<u64>,

    /// Maximum consecutive identical thinking chunks before assuming loop.
    ///
    /// This detects when a model is stuck repeating the same thinking content.
    ///
    /// Default: 10
    #[serde(default = "StreamConfig::default_max_thinking_loop")]
    pub max_thinking_loop: usize,

    /// Enable progressive progress reporting.
    ///
    /// When enabled, the backend will send progress events at regular intervals
    /// and at warning thresholds.
    ///
    /// Default: true
    #[serde(default = "StreamConfig::default_progress_enabled")]
    pub progress_enabled: bool,

    /// Maximum total characters (thinking + content) before hard cutoff.
    ///
    /// This is a safety limit to prevent infinite loops where the model
    /// continuously generates content without completing. If the total
    /// output exceeds this limit, the stream will be terminated.
    ///
    /// Default: 200,000 characters
    #[serde(default = "StreamConfig::default_max_total_chars")]
    pub max_total_chars: usize,
}

impl StreamConfig {
    fn default_max_thinking_chars() -> usize {
        50_000
    }

    fn default_max_thinking_time_secs() -> u64 {
        300
    }

    fn default_max_stream_duration_secs() -> u64 {
        1200
    }

    fn default_warning_thresholds() -> Vec<u64> {
        vec![60, 300, 600, 900]
    }

    fn default_max_thinking_loop() -> usize {
        10
    }

    fn default_progress_enabled() -> bool {
        true
    }

    fn default_max_total_chars() -> usize {
        200_000
    }

    /// Get the max stream duration as a Duration.
    pub fn max_stream_duration(&self) -> Duration {
        Duration::from_secs(self.max_stream_duration_secs)
    }

    /// Get the max thinking time as a Duration.
    pub fn max_thinking_time(&self) -> Duration {
        Duration::from_secs(self.max_thinking_time_secs)
    }
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            max_thinking_chars: Self::default_max_thinking_chars(),
            max_thinking_time_secs: Self::default_max_thinking_time_secs(),
            max_stream_duration_secs: Self::default_max_stream_duration_secs(),
            warning_thresholds: Self::default_warning_thresholds(),
            max_thinking_loop: Self::default_max_thinking_loop(),
            progress_enabled: Self::default_progress_enabled(),
            max_total_chars: Self::default_max_total_chars(),
        }
    }
}

/// LLM runtime error.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// Backend not available
    #[error("Backend {0} not available")]
    BackendUnavailable(String),

    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Generation error
    #[error("Generation error: {0}")]
    Generation(String),

    /// HTTP API error with status code and response body.
    /// Used by cloud backends to preserve status code for classification.
    #[error("API error {status}: {body}")]
    Api {
        /// HTTP status code (e.g., 401, 403, 429, 500).
        status: u16,
        /// Raw response body (may be JSON, may be empty).
        body: String,
    },

    /// Context window exceeded - request too large for model's context
    #[error("Context overflow: {prompt_tokens} prompt tokens exceed {max_context} context limit")]
    ContextOverflow {
        /// Number of prompt tokens in the failed request
        prompt_tokens: usize,
        /// Maximum context size of the model
        max_context: usize,
    },

    /// Network error (for cloud/remote backends)
    #[error("Network error: {0}")]
    Network(String),

    /// Timeout
    #[error("Operation timed out after {0}s")]
    Timeout(u64),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Metrics for LLM backend operations.
#[derive(Debug, Clone, Default)]
pub struct BackendMetrics {
    /// Total number of requests
    pub total_requests: u64,

    /// Successful requests
    pub successful_requests: u64,

    /// Failed requests
    pub failed_requests: u64,

    /// Total tokens generated
    pub total_tokens: u64,

    /// Average latency in milliseconds
    pub avg_latency_ms: f64,

    /// Last request timestamp
    pub last_request: Option<std::time::SystemTime>,
}

impl BackendMetrics {
    /// Record a successful request.
    pub fn record_success(&mut self, tokens: u64, latency_ms: u64) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.total_tokens += tokens;
        self.avg_latency_ms = (self.avg_latency_ms * (self.total_requests - 1) as f64
            + latency_ms as f64)
            / self.total_requests as f64;
        self.last_request = Some(std::time::SystemTime::now());
    }

    /// Record a failed request.
    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.failed_requests += 1;
        self.last_request = Some(std::time::SystemTime::now());
    }

    /// Get success rate (0.0 to 1.0).
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        self.successful_requests as f64 / self.total_requests as f64
    }
}

/// Abstract LLM runtime backend.
#[async_trait::async_trait]
pub trait LlmRuntime: Send + Sync {
    /// Get the backend type identifier.
    fn backend_id(&self) -> BackendId;

    /// Get the current model name.
    fn model_name(&self) -> &str;

    /// Check if the backend is available.
    async fn is_available(&self) -> bool {
        true
    }

    /// Warm up the model by sending a minimal request.
    ///
    /// This eliminates first-request latency by triggering model loading
    /// during initialization. Implementations should use minimal tokens
    /// to reduce overhead. Default implementation does nothing.
    async fn warmup(&self) -> Result<(), LlmError> {
        // Default: no warmup
        Ok(())
    }

    /// Generate a response (non-streaming).
    async fn generate(&self, input: LlmInput) -> Result<LlmOutput, LlmError>;

    /// Generate a response (streaming).
    async fn generate_stream(
        &self,
        input: LlmInput,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamChunk> + Send>>, LlmError>;

    /// Get max context length.
    fn max_context_length(&self) -> usize;

    /// Estimate token count (approximate).
    fn estimate_tokens(&self, text: &str) -> usize {
        // Rough estimate: ~4 chars per token
        text.len() / 4
    }

    /// Check if multimodal (vision) is supported.
    fn supports_multimodal(&self) -> bool {
        false
    }

    /// Get backend capabilities.
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    /// Get backend metrics (if supported).
    ///
    /// This allows backends to optionally provide runtime metrics
    /// such as request counts, latencies, etc.
    fn metrics(&self) -> BackendMetrics {
        BackendMetrics::default()
    }
}

/// Backend capabilities.
#[derive(Debug, Clone, Default)]
pub struct BackendCapabilities {
    /// Supports streaming generation
    pub streaming: bool,

    /// Supports multimodal (vision)
    pub multimodal: bool,

    /// Supports function calling
    pub function_calling: bool,

    /// Supports multiple models
    pub multiple_models: bool,

    /// Maximum context length
    pub max_context: Option<usize>,

    /// Supported modalities
    pub modalities: Vec<String>,

    /// Supports thinking/reasoning display
    pub thinking_display: bool,

    /// Supports image input
    pub supports_images: bool,

    /// Supports audio input
    pub supports_audio: bool,
}

impl BackendCapabilities {
    /// Create a new builder for capabilities.
    pub fn builder() -> BackendCapabilitiesBuilder {
        BackendCapabilitiesBuilder::new()
    }
}

/// Builder for BackendCapabilities.
pub struct BackendCapabilitiesBuilder {
    capabilities: BackendCapabilities,
}

impl Default for BackendCapabilitiesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendCapabilitiesBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            capabilities: BackendCapabilities::default(),
        }
    }

    /// Enable streaming.
    pub fn streaming(mut self) -> Self {
        self.capabilities.streaming = true;
        self
    }

    /// Enable multimodal.
    pub fn multimodal(mut self) -> Self {
        self.capabilities.multimodal = true;
        self.capabilities.supports_images = true; // Multimodal implies image support
        self
    }

    /// Enable function calling.
    pub fn function_calling(mut self) -> Self {
        self.capabilities.function_calling = true;
        self
    }

    /// Set max context.
    pub fn max_context(mut self, max: usize) -> Self {
        self.capabilities.max_context = Some(max);
        self
    }

    /// Enable thinking display.
    pub fn thinking_display(mut self) -> Self {
        self.capabilities.thinking_display = true;
        self
    }

    /// Build the capabilities.
    pub fn build(self) -> BackendCapabilities {
        self.capabilities
    }
}

impl LlmError {
    /// Classify whether this error requires user action (permanent) or may
    /// succeed on retry (transient).
    ///
    /// Used for log clarity only — both classes propagate as Failed execution
    /// per the agent error surfacing design (Rev 4, Option A).
    pub fn is_permanent(&self) -> bool {
        match self {
            Self::BackendUnavailable(_)
            | Self::ModelNotFound(_)
            | Self::InvalidInput(_)
            | Self::ContextOverflow { .. }
            | Self::Serialization(_) => true,
            Self::Api { status, .. } => *status >= 400 && *status < 500 && *status != 429,
            Self::Timeout(_)
            | Self::Network(_)
            | Self::Io(_)
            | Self::Generation(_)
            | Self::Unknown(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_input_builder() {
        let input = LlmInput::new("Hello")
            .with_model("qwen2")
            .with_streaming(true);

        assert_eq!(input.messages.len(), 1);
        assert_eq!(input.model.as_deref(), Some("qwen2"));
        assert!(input.stream);
    }
}

#[cfg(test)]
mod error_classification_tests {
    use super::LlmError;

    #[test]
    fn permanent_variants() {
        assert!(LlmError::BackendUnavailable("ollama".into()).is_permanent());
        assert!(LlmError::ModelNotFound("qwen3.5:4b".into()).is_permanent());
        assert!(LlmError::InvalidInput("bad request".into()).is_permanent());
        assert!(
            LlmError::ContextOverflow {
                prompt_tokens: 10000,
                max_context: 8000
            }
            .is_permanent()
        );
        assert!(
            LlmError::Serialization(
                serde_json::from_str::<i32>("not a number").unwrap_err()
            )
            .is_permanent()
        );
    }

    #[test]
    fn permanent_http_statuses() {
        assert!(LlmError::Api { status: 400, body: "".into() }.is_permanent());
        assert!(LlmError::Api { status: 401, body: "".into() }.is_permanent());
        assert!(LlmError::Api { status: 403, body: "quota exhausted".into() }.is_permanent());
        assert!(LlmError::Api { status: 404, body: "".into() }.is_permanent());
    }

    #[test]
    fn transient_http_statuses() {
        assert!(!LlmError::Api { status: 429, body: "rate limited".into() }.is_permanent());
        assert!(!LlmError::Api { status: 500, body: "".into() }.is_permanent());
        assert!(!LlmError::Api { status: 502, body: "".into() }.is_permanent());
        assert!(!LlmError::Api { status: 503, body: "".into() }.is_permanent());
    }

    #[test]
    fn transient_variants() {
        assert!(!LlmError::Timeout(60).is_permanent());
        assert!(!LlmError::Network("connection refused".into()).is_permanent());
        assert!(!LlmError::Generation("legacy fallback".into()).is_permanent());
        assert!(!LlmError::Unknown("something".into()).is_permanent());
    }

    #[test]
    fn api_variant_display_format() {
        let e = LlmError::Api {
            status: 403,
            body: "quota exhausted".into(),
        };
        let s = format!("{}", e);
        assert!(
            s.contains("403"),
            "Display should include status: got {}",
            s
        );
        assert!(
            s.contains("quota exhausted"),
            "Display should include body: got {}",
            s
        );
    }
}
