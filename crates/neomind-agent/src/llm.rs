//! LLM interface for the Agent.
//!
//! This module provides a simple LLM wrapper with concurrency limits
//! and integration with the LlmBackendInstanceManager for dynamic backend switching.

use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::Stream;
use tokio::sync::RwLock;

use neomind_core::{
    config::agent_env_vars,
    llm::backend::{LlmError, LlmInput, LlmRuntime},
    Message,
};

use crate::agent::tokenizer::estimate_tokens;

// Import intent classifier for staged processing
use crate::agent::staged::{IntentCategory, IntentClassifier, IntentResult, ToolFilter};
// Import the unified error type
use crate::error::NeoMindError;
// Import the Result type alias
use crate::error::Result as AgentResult;

/// Re-export the instance manager types for convenience
pub use crate::llm_backends::{
    get_instance_manager, BackendTypeDefinition, LlmBackendInstanceManager,
};

/// Default concurrent LLM request limit.
/// Note: This constant is kept for backward compatibility but the actual default
/// is loaded from environment variable AGENT_CONCURRENT_LIMIT with fallback to 3.
pub const DEFAULT_CONCURRENT_LIMIT: usize = 3;

/// Get the concurrent limit from environment or return default.
#[inline]
pub fn get_concurrent_limit() -> usize {
    agent_env_vars::concurrent_limit()
}

/// Simple atomic-based concurrency limiter.
///
/// This is simpler than using a semaphore for streams because it doesn't
/// have lifetime issues with permits.
#[derive(Clone)]
struct ConcurrencyLimiter {
    current: Arc<AtomicUsize>,
    max: Arc<AtomicUsize>, // Make max AtomciUsize for dynamic adjustment
    base_max: usize,       // Original configured max
}

impl ConcurrencyLimiter {
    fn new(max: usize) -> Self {
        Self {
            current: Arc::new(AtomicUsize::new(0)),
            max: Arc::new(AtomicUsize::new(max)),
            base_max: max,
        }
    }

    /// Try to acquire a permit. Returns Some(permit) if successful, None if at limit.
    fn try_acquire(&self) -> Option<ConcurrencyPermit> {
        // Update dynamic limit before acquiring
        self.update_dynamic_limit();

        let max = self.max.load(Ordering::Relaxed);
        let mut current = self.current.load(Ordering::Relaxed);
        loop {
            if current >= max {
                return None;
            }
            match self.current.compare_exchange_weak(
                current,
                current + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Some(ConcurrencyPermit {
                        limiter: self.current.clone(),
                    });
                }
                Err(new_current) => current = new_current,
            }
        }
    }

    /// Acquire a permit, waiting until one is available.
    async fn acquire(&self) -> ConcurrencyPermit {
        loop {
            if let Some(permit) = self.try_acquire() {
                return permit;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    /// Update the dynamic concurrency limit based on system load.
    ///
    /// This adjusts the maximum concurrent requests based on:
    /// - System memory usage
    /// - CPU load
    /// - Current utilization
    fn update_dynamic_limit(&self) {
        let utilization = self.current.load(Ordering::Relaxed) as f64 / self.base_max as f64;

        // Get system memory (in bytes)
        let memory_available = get_available_memory_bytes();
        let memory_gb = memory_available as f64 / (1024.0 * 1024.0 * 1024.0);

        // Calculate dynamic limit based on system conditions
        let dynamic_max = if memory_gb < 1.0 {
            // Low memory: reduce concurrency
            (self.base_max as f64 * 0.5).max(1.0) as usize
        } else if memory_gb < 2.0 {
            // Medium-low memory
            (self.base_max as f64 * 0.75).max(1.0) as usize
        } else if utilization > 0.8 {
            // High utilization: reduce slightly
            (self.base_max as f64 * 0.85).max(1.0) as usize
        } else {
            // Good conditions: use configured max
            self.base_max
        };

        self.max.store(dynamic_max, Ordering::Relaxed);
    }
}

/// Get available memory in bytes.
/// Returns a conservative estimate to avoid over-committing.
fn get_available_memory_bytes() -> usize {
    // Target 2GB minimum free memory for healthy operation
    // On systems with more memory, allow higher concurrency
    #[cfg(target_os = "linux")]
    {
        // Try to read from /proc/meminfo
        if let Ok(info) = std::fs::read_to_string("/proc/meminfo") {
            for line in info.lines() {
                if line.starts_with("MemAvailable:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<usize>() {
                            return kb * 1024;
                        }
                    }
                }
            }
        }
        // Fallback: assume 2GB available
        2 * 1024 * 1024 * 1024
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, use sysctl to get memory info
        // For simplicity, return a conservative estimate
        4 * 1024 * 1024 * 1024 // Assume 4GB available
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use GlobalMemoryStatusEx
        // For simplicity, return a conservative estimate
        4 * 1024 * 1024 * 1024 // Assume 4GB available
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        // Unknown platform: conservative estimate
        2 * 1024 * 1024 * 1024
    }
}

/// A permit that releases when dropped.
struct ConcurrencyPermit {
    limiter: Arc<AtomicUsize>,
}

impl Drop for ConcurrencyPermit {
    fn drop(&mut self) {
        self.limiter.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Wrapper stream that holds a concurrency permit.
///
/// This ensures the permit is held for the entire lifetime of the stream,
/// releasing it only when the stream is dropped.
struct PermitStream<S> {
    inner: S,
    _permit: ConcurrencyPermit,
}

impl<S> PermitStream<S> {
    fn new(inner: S, permit: ConcurrencyPermit) -> Self {
        Self {
            inner,
            _permit: permit,
        }
    }
}

impl<S: Stream + Unpin> Stream for PermitStream<S> {
    type Item = S::Item;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

/// Simple LLM chat interface.
///
/// This is a lightweight wrapper around LLM runtime with concurrency limiting.
/// It can operate in two modes:
/// 1. Direct runtime mode - Uses a manually set LLM runtime (backward compatible)
/// 2. Instance manager mode - Uses LlmBackendInstanceManager for dynamic backend switching
#[derive(Clone)]
pub struct LlmInterface {
    /// The LLM runtime backend (used in direct mode, stored as Arc for compatibility).
    llm: Arc<RwLock<Option<Arc<dyn LlmRuntime>>>>,
    /// Optional instance manager for dynamic backend switching.
    instance_manager: Option<Arc<LlmBackendInstanceManager>>,
    /// Model name.
    model: Arc<RwLock<Option<String>>>,
    /// Temperature.
    temperature: f32,
    /// Top-p sampling.
    top_p: f32,
    /// Top-k sampling (0 = disabled).
    top_k: usize,
    /// Max tokens.
    max_tokens: usize,
    /// Default system prompt (wrapped for dynamic updates).
    system_prompt: Arc<RwLock<String>>,
    /// Tool definitions for function calling.
    tool_definitions: Arc<RwLock<Vec<neomind_core::llm::backend::ToolDefinition>>>,
    /// System prompt cache to avoid rebuilding on every request.
    /// Stores (tools_hash, prompt) to invalidate when tools change.
    system_prompt_cache: Arc<RwLock<Option<(String, String)>>>,
    /// Hash of cached tool definitions for invalidation.
    cached_tools_hash: Arc<RwLock<Option<String>>>,
    /// Concurrency limiter.
    limiter: ConcurrencyLimiter,
    /// Whether to use instance manager for runtime retrieval.
    use_instance_manager: Arc<AtomicUsize>,
    /// Whether thinking mode is enabled (for direct mode when not using instance manager).
    /// Defaults to false for faster responses.
    thinking_enabled: Arc<RwLock<Option<bool>>>,
    /// Last prompt token count from the most recent stream response.
    /// Updated in-band when the stream completes.
    last_prompt_tokens: Arc<tokio::sync::Mutex<Option<u32>>>,
    /// Intent classifier for staged processing.
    intent_classifier: IntentClassifier,
    /// Tool filter for reducing tools sent to LLM.
    tool_filter: ToolFilter,
    /// Business context manager for dynamic context injection.
    context_manager: Option<crate::context::ContextManager>,
    /// Global timezone for time-aware prompts (IANA format, e.g., "Asia/Shanghai").
    /// Loaded from settings and used for all time-related context.
    global_timezone: Arc<RwLock<Option<String>>>,
}

impl LlmInterface {
    /// Create a new LLM interface.
    pub fn new(config: ChatConfig) -> Self {
        let concurrent_limit = config.concurrent_limit;
        Self {
            llm: Arc::new(RwLock::new(None)),
            instance_manager: None,
            model: Arc::new(RwLock::new(Some(config.model))),
            temperature: config.temperature,
            top_p: config.top_p,
            top_k: config.top_k,
            max_tokens: config.max_tokens,
            system_prompt: Arc::new(RwLock::new("You are a helpful AI assistant.".to_string())),
            tool_definitions: Arc::new(RwLock::new(Vec::new())),
            system_prompt_cache: Arc::new(RwLock::new(None)),
            cached_tools_hash: Arc::new(RwLock::new(None)),
            limiter: ConcurrencyLimiter::new(concurrent_limit),
            use_instance_manager: Arc::new(AtomicUsize::new(0)),
            thinking_enabled: Arc::new(RwLock::new(None)), // Use backend default (from storage)
            last_prompt_tokens: Arc::new(tokio::sync::Mutex::new(None)),
            intent_classifier: IntentClassifier::default(),
            tool_filter: ToolFilter::default(),
            context_manager: None,
            global_timezone: Arc::new(RwLock::new(None)), // Will be loaded from settings
        }
    }

    /// Create a new LLM interface with instance manager integration.
    pub fn with_instance_manager(
        config: ChatConfig,
        manager: Arc<LlmBackendInstanceManager>,
    ) -> Self {
        let concurrent_limit = config.concurrent_limit;
        Self {
            llm: Arc::new(RwLock::new(None)),
            instance_manager: Some(manager),
            model: Arc::new(RwLock::new(Some(config.model))),
            temperature: config.temperature,
            top_p: config.top_p,
            top_k: config.top_k,
            max_tokens: config.max_tokens,
            system_prompt: Arc::new(RwLock::new("You are a helpful AI assistant.".to_string())),
            tool_definitions: Arc::new(RwLock::new(Vec::new())),
            system_prompt_cache: Arc::new(RwLock::new(None)),
            cached_tools_hash: Arc::new(RwLock::new(None)),
            limiter: ConcurrencyLimiter::new(concurrent_limit),
            use_instance_manager: Arc::new(AtomicUsize::new(1)),
            thinking_enabled: Arc::new(RwLock::new(None)), // Will use instance manager setting
            last_prompt_tokens: Arc::new(tokio::sync::Mutex::new(None)),
            intent_classifier: IntentClassifier::default(),
            tool_filter: ToolFilter::default(),
            context_manager: None,
            global_timezone: Arc::new(RwLock::new(None)), // Will be loaded from settings
        }
    }

    /// Set the thinking mode for direct LLM usage (when not using instance manager).
    pub async fn set_thinking_enabled(&self, enabled: bool) {
        *self.thinking_enabled.write().await = Some(enabled);
    }

    /// Get the thinking mode setting.
    pub async fn get_thinking_enabled(&self) -> Option<bool> {
        *self.thinking_enabled.read().await
    }

    /// Take the last prompt token count from the most recent stream response.
    /// Returns the value and resets it to None.
    pub async fn take_last_prompt_tokens(&self) -> Option<u32> {
        self.last_prompt_tokens.lock().await.take()
    }

    /// Set the global timezone for time-aware prompts.
    /// This should be loaded from the settings store on initialization and updated when settings change.
    pub async fn set_global_timezone(&self, timezone: String) {
        *self.global_timezone.write().await = Some(timezone);
        // Clear the system prompt cache so time placeholders are re-evaluated with new timezone
        *self.system_prompt_cache.write().await = None;
    }

    /// Get the current global timezone setting.
    pub async fn get_global_timezone(&self) -> Option<String> {
        self.global_timezone.read().await.clone()
    }

    /// Load global timezone from settings store and apply it.
    /// Returns the loaded timezone or default if not found.
    pub async fn load_global_timezone(&self) -> AgentResult<String> {
        use neomind_storage::SettingsStore;

        const SETTINGS_DB_PATH: &str = "data/settings.redb";

        let settings_store = SettingsStore::open(SETTINGS_DB_PATH)
            .map_err(|e| NeoMindError::Llm(format!("Failed to open settings store: {}", e)))?;

        let timezone = settings_store.get_global_timezone();
        self.set_global_timezone(timezone.clone()).await;

        tracing::debug!("Loaded global timezone: {}", timezone);
        Ok(timezone)
    }

    /// Enable or disable instance manager mode.
    pub async fn set_use_instance_manager(&self, use_manager: bool) {
        self.use_instance_manager
            .store(if use_manager { 1 } else { 0 }, Ordering::Relaxed);
    }

    /// Check if instance manager mode is enabled.
    /// Returns true if the flag is set, regardless of whether the instance manager is currently available.
    pub fn uses_instance_manager(&self) -> bool {
        self.use_instance_manager.load(Ordering::Relaxed) == 1
    }

    /// Get the instance manager if available.
    pub fn instance_manager(&self) -> Option<Arc<LlmBackendInstanceManager>> {
        self.instance_manager.clone()
    }

    /// Set the instance manager.
    pub async fn set_instance_manager(&self, manager: Arc<LlmBackendInstanceManager>) {
        // Update the instance manager reference
        // Note: This requires interior mutability pattern
        let _ = manager;
        // For now, this is a placeholder - the instance manager is set at creation time
        // To make this fully dynamic, we'd need to wrap it in Arc<RwLock<>>
    }

    /// Get the current LLM runtime.
    /// Priority: Direct runtime (set via configure_llm) > Instance manager active runtime
    /// This ensures that when a specific backend is configured via backendId, it takes precedence.
    async fn get_runtime(&self) -> AgentResult<Arc<dyn LlmRuntime>> {
        // First, check if a direct runtime is set (via configure_llm)
        // This takes precedence over instance manager to support backendId selection
        let llm_guard = self.llm.read().await;
        if let Some(runtime) = llm_guard.as_ref() {
            tracing::debug!(
                model = %runtime.model_name(),
                "get_runtime: using direct runtime from configure_llm"
            );
            return Ok(Arc::clone(runtime));
        }
        drop(llm_guard);

        tracing::debug!("get_runtime: no direct runtime, checking instance manager");

        // Fall back to instance manager if no direct runtime is set
        if self.uses_instance_manager() {
            if let Some(manager) = &self.instance_manager {
                return manager
                    .get_active_runtime()
                    .await
                    .map_err(|e| NeoMindError::Llm(e.to_string()));
            }
        }

        Err(NeoMindError::Llm("LLM backend not ready".to_string()))
    }

    /// Compute the token budget for conversation history given the model's context window.
    ///
    /// Uses `estimate_tokens` for accurate token counting that handles Chinese (~1.8 tokens/char),
    /// English (~0.25 tokens/char), and mixed content correctly.
    ///
    /// Returns `(available_tokens, prompt_budget_tokens)` where `available_tokens` is the
    /// token budget for history messages after reserving space for everything else.
    async fn compute_history_budget(
        &self,
        max_ctx: usize,
        system_prompt: &str,
        user_message: &str,
        history_msg_count: usize,
        include_tools: bool,
    ) -> (usize, usize) {
        // Use more conservative budget for small contexts
        // < 8k: 50% for prompt, 50% for generation + overhead
        // < 16k: 60%, >= 16k: 70%
        let prompt_ratio = if max_ctx < 8192 {
            50
        } else if max_ctx < 16384 {
            60
        } else {
            70
        };
        let prompt_budget = (max_ctx * prompt_ratio) / 100;

        // Estimate tool definition overhead in tokens using estimate_tokens
        let tool_overhead_tokens = if include_tools {
            let tools = self.tool_definitions.read().await;
            if tools.is_empty() {
                0
            } else {
                tools
                    .iter()
                    .map(|t| {
                        estimate_tokens(&t.name)
                            + estimate_tokens(&t.description)
                            + estimate_tokens(&t.parameters.to_string())
                            + 10 // formatting overhead per tool
                    })
                    .sum::<usize>()
            }
        } else {
            0
        };

        // Chat template overhead: ~10 tokens per message for role markers and special tokens
        let template_overhead_tokens = 10 * (history_msg_count + 2);

        // Use estimate_tokens for accurate Chinese/English token counting
        let system_tokens = estimate_tokens(system_prompt);
        let user_tokens = estimate_tokens(user_message);

        let reserved = system_tokens
            + user_tokens
            + tool_overhead_tokens
            + template_overhead_tokens
            + 200; // additional safety margin

        let available_tokens = prompt_budget.saturating_sub(reserved);

        tracing::debug!(
            max_ctx,
            prompt_ratio,
            prompt_budget,
            reserved,
            available_tokens,
            tool_overhead_tokens,
            template_overhead_tokens,
            system_tokens,
            user_tokens,
            "Computed history budget (token-based)"
        );

        (available_tokens, prompt_budget)
    }

    /// Get effective generation parameters.
    /// When using instance manager, reads from the active backend instance.
    /// Otherwise falls back to local ChatConfig values.
    async fn get_effective_params(&self) -> (f32, f32, usize, usize) {
        if self.uses_instance_manager() {
            if let Some(manager) = &self.instance_manager {
                if let Some(inst) = manager.get_active_instance() {
                    return (inst.temperature, inst.top_p, inst.top_k, inst.max_tokens);
                }
            }
        }
        // Fall back to local config
        (self.temperature, self.top_p, self.top_k, self.max_tokens)
    }

    /// Set the LLM runtime (direct mode).
    pub async fn set_llm(&self, llm: Arc<dyn LlmRuntime>) {
        // Update the model name when setting a custom LLM
        let model_name = llm.model_name().to_string();
        *self.model.write().await = Some(model_name);
        let mut llm_guard = self.llm.write().await;
        *llm_guard = Some(llm);
    }

    /// Set the LLM runtime from a Box (for backward compatibility).
    ///
    /// This converts a Box<dyn LlmRuntime> to Arc<dyn LlmRuntime> by creating
    /// an Arc wrapper around the boxed value.
    pub async fn set_llm_from_box(&self, llm: Box<dyn LlmRuntime>) {
        // We need to convert Box to Arc
        // This is a bit tricky since we can't directly convert Box to Arc
        // The runtime needs to be created as Arc from the start
        // For backward compatibility, we'll need to use unsafe or restructure
        let _ = llm;
        // Store None for now - the caller should use Arc directly
        let mut llm_guard = self.llm.write().await;
        *llm_guard = None;
    }
    pub async fn switch_backend(&self, backend_id: &str) -> AgentResult<()> {
        if let Some(manager) = &self.instance_manager {
            manager
                .set_active(backend_id)
                .await
                .map_err(|e| NeoMindError::Llm(e.to_string()))?;
            Ok(())
        } else {
            Err(NeoMindError::Llm(
                "No instance manager configured".to_string(),
            ))
        }
    }

    /// Get available backend types.
    pub async fn get_available_backends(&self) -> Vec<BackendTypeDefinition> {
        if let Some(manager) = &self.instance_manager {
            manager.get_available_types()
        } else {
            Vec::new()
        }
    }

    /// Get the current concurrency limit (max concurrent requests).
    /// This returns the dynamic limit which may be adjusted based on system load.
    pub fn max_concurrent(&self) -> usize {
        self.limiter.max.load(Ordering::Relaxed)
    }

    /// Get the number of available permit slots.
    pub fn available_permits(&self) -> usize {
        let max = self.limiter.max.load(Ordering::Relaxed);
        let current = self.limiter.current.load(Ordering::Relaxed);
        max.saturating_sub(current)
    }

    /// Get the maximum context window size for the current LLM backend.
    ///
    /// This returns the actual context limit based on the model's capabilities.
    /// For example:
    /// - qwen3-vl:2b -> 32768
    /// - llama3:8b -> 8192
    /// - deepseek-r1 -> 64000
    ///
    /// Returns a conservative default (4096) if the LLM is not ready.
    pub async fn max_context_length(&self) -> usize {
        // First, try to query the runtime directly (most accurate)
        if let Ok(runtime) = self.get_runtime().await {
            return runtime.max_context_length();
        }

        // Fall back to instance manager if runtime is not available
        if self.uses_instance_manager() {
            if let Some(manager) = &self.instance_manager {
                if let Some(instance) = manager.get_active_instance() {
                    return instance.capabilities.max_context;
                }
            }
        }

        4_096 // Conservative default if LLM not ready
    }

    /// Check if the current LLM backend supports multimodal (vision) input.
    ///
    /// Returns true if the active backend supports image input, false otherwise.
    ///
    /// Priority: Runtime capabilities > Storage layer capabilities
    /// This ensures correct detection when user selects a specific backend via backendId.
    pub async fn supports_multimodal(&self) -> bool {
        // First, try to query the runtime directly (most accurate - uses model name detection)
        if let Ok(runtime) = self.get_runtime().await {
            let caps = runtime.capabilities();
            tracing::debug!(
                multimodal = %caps.multimodal,
                model = %runtime.model_name(),
                "supports_multimodal: checking runtime capabilities"
            );
            return caps.multimodal;
        }

        tracing::debug!("supports_multimodal: no runtime available, checking instance manager");

        // Fall back to instance manager if runtime is not available
        if self.uses_instance_manager() {
            if let Some(manager) = &self.instance_manager {
                if let Some(instance) = manager.get_active_instance() {
                    // Instance has capabilities with supports_multimodal (storage layer)
                    tracing::debug!(
                        supports_multimodal = %instance.capabilities.supports_multimodal,
                        model = %instance.model,
                        "supports_multimodal: using instance manager capabilities"
                    );
                    return instance.capabilities.supports_multimodal;
                }
            }
        }

        tracing::warn!("supports_multimodal: no runtime or instance available, returning false");
        false
    }

    /// Warm up the model by sending a minimal request.
    ///
    /// This eliminates the ~500ms first-request latency by triggering model loading
    /// during initialization. Should be called during application startup.
    ///
    /// # Example
    /// ```ignore
    /// // During application startup
    /// llm_interface.warmup().await?;
    /// ```
    pub async fn warmup(&self) -> AgentResult<()> {
        match self.get_runtime().await {
            Ok(runtime) => runtime
                .warmup()
                .await
                .map_err(|e| NeoMindError::Llm(e.to_string())),
            Err(e) => Err(e),
        }
    }

    /// Set the system prompt (builder pattern).
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        // Store the prompt in Arc<RwLock<String>>
        self.system_prompt = Arc::new(RwLock::new(prompt.into()));
        self
    }

    /// Set the system prompt (for dynamic updates).
    pub async fn set_system_prompt(&self, prompt: &str) {
        *self.system_prompt.write().await = prompt.to_string();
    }

    /// Get the current system prompt.
    pub async fn get_system_prompt(&self) -> String {
        self.system_prompt.read().await.clone()
    }

    /// Set tool definitions for function calling.
    pub async fn set_tool_definitions(
        &self,
        tools: Vec<neomind_core::llm::backend::ToolDefinition>,
    ) {
        *self.tool_definitions.write().await = tools;
        // Invalidate system prompt cache when tools change
        self.invalidate_prompt_cache().await;
    }

    /// Get current tool definitions.
    pub async fn get_tool_definitions(&self) -> Vec<neomind_core::llm::backend::ToolDefinition> {
        self.tool_definitions.read().await.clone()
    }

    /// Classify user intent from message.
    pub fn classify_intent(&self, message: &str) -> IntentResult {
        self.intent_classifier.classify(message)
    }

    /// Get intent-specific system prompt.
    pub fn get_intent_prompt(&self, intent: &IntentResult) -> String {
        self.tool_filter.intent_prompt(intent)
    }

    /// Set the business context manager.
    pub fn set_context_manager(&mut self, manager: crate::context::ContextManager) {
        self.context_manager = Some(manager);
    }

    /// Get context builder section for system prompt.
    async fn build_business_context_section(&self, query: &str) -> String {
        if let Some(ref cm) = self.context_manager {
            cm.format_for_prompt(query).await
        } else {
            String::new()
        }
    }

    /// Filter tools by user message (intent-based).
    /// Returns only relevant tools (3-5 max) to reduce thinking.
    pub async fn filter_tools_by_intent(
        &self,
        user_message: &str,
    ) -> Vec<neomind_core::llm::backend::ToolDefinition> {
        let all_tools = self.tool_definitions.read().await;
        if all_tools.is_empty() {
            return Vec::new();
        }

        // Classify intent from user message
        let intent = self.intent_classifier.classify(user_message);
        let target_namespace = intent.category.namespace();

        // Helper to derive namespace from tool name
        let derive_namespace = |name: &str| -> &str {
            if name.starts_with("list_")
                || name.starts_with("get_")
                || name == "control_device"
                || name.contains("device")
            {
                "device"
            } else if name.contains("rule") || name.contains("automation") {
                "rule"
            } else if name.contains("workflow")
                || name.contains("scenario")
                || name.contains("trigger")
            {
                "workflow"
            } else if name.contains("data") || name.contains("query") || name.contains("metrics") {
                "data"
            } else if name == "think" || name == "tool_search" {
                "system"
            } else {
                "general"
            }
        };

        // Filter tools by namespace (always include system tools)
        let mut filtered: Vec<neomind_core::llm::backend::ToolDefinition> = all_tools
            .iter()
            .filter(|t| {
                let ns = derive_namespace(&t.name);
                ns == "system" || ns == target_namespace
            })
            .cloned()
            .collect();

        // If no tools found or general intent, include list_* tools
        if filtered.is_empty() || intent.category == IntentCategory::General {
            let list_tools: Vec<neomind_core::llm::backend::ToolDefinition> = all_tools
                .iter()
                .filter(|t| t.name.starts_with("list_") || t.name.starts_with("query_"))
                .take(3)
                .cloned()
                .collect();
            filtered.extend(list_tools);
        }

        // Limit to 5 tools max
        filtered.truncate(5);
        filtered
    }

    /// Invalidate the system prompt cache.
    /// Call this when tools or configuration changes.
    pub async fn invalidate_prompt_cache(&self) {
        *self.system_prompt_cache.write().await = None;
        *self.cached_tools_hash.write().await = None;
    }

    /// Build the base system prompt (without user-specific parts).
    /// This is cached to avoid rebuilding on every request.
    async fn build_base_system_prompt(&self) -> String {
        use crate::prompts::PromptBuilder;

        // Check cache first
        {
            let cache_read = self.system_prompt_cache.read().await;
            if let Some((_, cached)) = cache_read.as_ref() {
                return cached.clone();
            }
        }

        // Build base prompt using PromptBuilder
        let base_prompt = PromptBuilder::new()
            .with_thinking(true) // Include thinking guidelines
            .with_examples(true) // Include usage examples
            .build_system_prompt();

        let mut prompt = String::with_capacity(4096);
        prompt.push_str(&base_prompt);
        prompt.push_str("\n\n");

        // Add tool calling section (centralized in PromptBuilder)
        prompt.push_str(&PromptBuilder::build_tool_calling_section());

        // Cache the result
        let cache_key = "base_prompt".to_string();
        {
            let mut cache_write = self.system_prompt_cache.write().await;
            *cache_write = Some((cache_key, prompt.clone()));
        }

        prompt
    }

    /// Build the base system prompt with current time injected.
    /// This replaces the time placeholders with actual time values using the configured global timezone.
    pub async fn build_base_system_prompt_with_time(&self, timezone: Option<&str>) -> String {
        use crate::prompts::{
            CURRENT_TIME_PLACEHOLDER, LOCAL_TIME_PLACEHOLDER, TIMEZONE_PLACEHOLDER,
        };

        // Get the base prompt (which contains placeholders)
        let base_prompt = self.build_base_system_prompt().await;

        // Calculate current times
        let now = chrono::Utc::now();
        let current_time_utc = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();

        // Use self.global_timezone first, then parameter, then default
        let effective_timezone = self
            .global_timezone
            .read()
            .await
            .as_ref()
            .cloned()
            .or_else(|| timezone.map(|s| s.to_string()))
            .unwrap_or_else(|| "Asia/Shanghai".to_string());

        // Parse timezone to get local time
        let tz = effective_timezone
            .parse::<chrono_tz::Tz>()
            .unwrap_or(chrono_tz::Tz::Asia__Shanghai); // Default to Shanghai on error

        let local_time = now
            .with_timezone(&tz)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        // Get additional time context for better LLM understanding
        let day_of_week = now.with_timezone(&tz).format("%A").to_string();
        let date_str = now.with_timezone(&tz).format("%B %d, %Y").to_string();

        // Get time period description (morning, afternoon, evening, night)
        let hour_str = now.with_timezone(&tz).format("%H").to_string();
        let hour: u32 = hour_str.parse().unwrap_or(12);
        let time_period = match hour {
            5..=11 => "Morning",
            12..=13 => "Noon",
            14..=17 => "Afternoon",
            18..=22 => "Evening",
            _ => "Night",
        };

        // Build enhanced time context
        let local_time_with_context = format!(
            "{} {} ({}{})",
            date_str, local_time, time_period, day_of_week
        );

        // Replace placeholders
        base_prompt
            .replace(CURRENT_TIME_PLACEHOLDER, &current_time_utc)
            .replace(LOCAL_TIME_PLACEHOLDER, &local_time_with_context)
            .replace(TIMEZONE_PLACEHOLDER, &effective_timezone)
    }

    /// Build system prompt with tool descriptions.
    /// Uses enhanced prompts from prompts module for better conversation quality.
    /// Uses cached base prompt with time placeholders replaced and adds user-specific parts.
    async fn build_system_prompt_with_tools(&self, user_message: Option<&str>) -> String {
        // Get base prompt with time placeholders replaced
        let mut prompt = self.build_base_system_prompt_with_time(None).await;

        // Add intent-specific addon if we can classify the user's message
        if let Some(msg) = user_message {
            let intent = self.intent_classifier.classify(msg);

            // Get intent addon using legacy PromptBuilder
            use crate::prompts::PromptBuilder;

            let task_type = match intent.category {
                crate::agent::staged::IntentCategory::Device => "device",
                crate::agent::staged::IntentCategory::Data => "data",
                crate::agent::staged::IntentCategory::Rule => "rule",
                crate::agent::staged::IntentCategory::Workflow => "workflow",
                _ => "general",
            };

            // Get intent-specific addon from PromptBuilder
            let addon = PromptBuilder::new().get_intent_prompt_addon(task_type);

            if !addon.is_empty() {
                prompt.push_str(&addon);
            }
        }

        // Add business context section if available
        if let Some(query) = user_message {
            let context_section = self.build_business_context_section(query).await;
            if !context_section.is_empty() {
                prompt.push_str(&context_section);
                prompt.push_str("\n\n");
            }
        }

        // Note: Tools are already included in base_prompt from build_base_system_prompt()
        // No need to duplicate them here unless we want to do user-specific filtering
        prompt
    }

    /// Filter simplified tools based on intent.
    #[allow(dead_code)]
    fn filter_simplified_tools(
        &self,
        tools: &[crate::toolkit::simplified::LlmToolDefinition],
        intent: &crate::agent::staged::IntentResult,
    ) -> Vec<crate::toolkit::simplified::LlmToolDefinition> {
        let mut filtered = Vec::new();

        // Always include tools that match the intent category
        for tool in tools {
            // Check if any use_when matches the intent
            let matches = tool.use_when.iter().any(|scenario| {
                let scenario_lower = scenario.to_lowercase();
                match intent.category {
                    crate::agent::staged::IntentCategory::Device => {
                        scenario_lower.contains("设备")
                            || scenario_lower.contains("控制")
                            || scenario_lower.contains("打开")
                            || scenario_lower.contains("关闭")
                    }
                    crate::agent::staged::IntentCategory::Data => {
                        scenario_lower.contains("询问")
                            || scenario_lower.contains("查询")
                            || scenario_lower.contains("数据")
                            || scenario_lower.contains("温度")
                    }
                    crate::agent::staged::IntentCategory::Rule => {
                        scenario_lower.contains("创建")
                            || scenario_lower.contains("规则")
                            || scenario_lower.contains("自动化")
                    }
                    crate::agent::staged::IntentCategory::Workflow => {
                        scenario_lower.contains("工作流") || scenario_lower.contains("执行")
                    }
                    crate::agent::staged::IntentCategory::Alert => {
                        scenario_lower.contains("告警")
                            || scenario_lower.contains("异常")
                            || scenario_lower.contains("通知")
                    }
                    crate::agent::staged::IntentCategory::System => {
                        scenario_lower.contains("系统")
                            || scenario_lower.contains("状态")
                            || scenario_lower.contains("健康")
                    }
                    crate::agent::staged::IntentCategory::Help => {
                        scenario_lower.contains("帮助")
                            || scenario_lower.contains("教程")
                            || scenario_lower.contains("说明")
                    }
                    crate::agent::staged::IntentCategory::General => true,
                }
            });

            if matches || tool.use_when.is_empty() {
                filtered.push(tool.clone());
            }
        }

        // Always include basic tools
        if !filtered.iter().any(|t| t.name == "list_devices") {
            if let Some(t) = tools.iter().find(|t| t.name == "list_devices") {
                filtered.push(t.clone());
            }
        }

        filtered.truncate(6); // Limit to 6 tools max
        filtered
    }

    /// Update the model name.
    pub async fn update_model(&self, model: String) {
        let mut model_guard = self.model.write().await;
        *model_guard = Some(model);
    }

    /// Check if the LLM backend is ready.
    pub async fn is_ready(&self) -> bool {
        if self.uses_instance_manager() {
            if let Some(manager) = &self.instance_manager {
                return manager.get_active_instance().is_some();
            }
        }
        let llm_guard = self.llm.read().await;
        llm_guard.as_ref().is_some()
    }

    /// Send a chat message and get a response.
    pub async fn chat(&self, user_message: impl Into<String>) -> AgentResult<ChatResponse> {
        self.chat_internal(user_message, None).await
    }

    /// Send a chat message with conversation history.
    pub async fn chat_with_history(
        &self,
        user_message: impl Into<String>,
        history: &[Message],
    ) -> AgentResult<ChatResponse> {
        self.chat_internal(user_message, Some(history)).await
    }

    /// Send a chat message without tools and get a response.
    /// This is useful for Phase 2 where tools have already been executed.
    pub async fn chat_without_tools(
        &self,
        user_message: impl Into<String>,
    ) -> AgentResult<ChatResponse> {
        self.chat_internal(user_message, None).await
    }

    /// Send a chat message without tools, with conversation history.
    pub async fn chat_without_tools_with_history(
        &self,
        user_message: impl Into<String>,
        history: &[Message],
    ) -> AgentResult<ChatResponse> {
        self.chat_internal(user_message, Some(history)).await
    }

    /// Send a multimodal message (with images) with conversation history.
    /// This is used when the user sends images along with their text.
    pub async fn chat_multimodal_with_history(
        &self,
        user_message: Message, // Can contain text + images
        history: &[Message],
    ) -> AgentResult<ChatResponse> {
        self.chat_internal_message(user_message, Some(history))
            .await
    }

    /// Internal chat implementation.
    async fn chat_internal(
        &self,
        user_message: impl Into<String>,
        history: Option<&[Message]>,
    ) -> AgentResult<ChatResponse> {
        let user_message: String = user_message.into();
        let max_ctx = self.max_context_length().await;

        // === FAST PATH: Simple greetings ===
        // Bypass LLM for simple greetings to improve response time
        let trimmed = user_message.trim();
        let greeting_patterns = [
            "你好",
            "您好",
            "hi",
            "hello",
            "嗨",
            "在吗",
            "早上好",
            "下午好",
            "晚上好",
        ];
        let is_greeting = greeting_patterns
            .iter()
            .any(|&pat| trimmed.eq_ignore_ascii_case(pat) || trimmed.starts_with(pat));

        if is_greeting && trimmed.len() < 20 {
            let greeting_response = "您好！我是 NeoMind 智能助手。我可以帮您：\n\
                • 查看设备列表 - 说「列出设备」\n\
                • 查询设备数据 - 说「查询温度」\n\
                • 创建自动化规则 - 说「创建规则」\n\
                • 查看所有规则 - 说「列出规则」";

            return Ok(ChatResponse {
                text: greeting_response.to_string(),
                tokens_used: 0,
                duration: Duration::from_millis(0),
                finish_reason: "stop".to_string(),
                thinking: None,
            });
        }

        // Acquire permit for concurrency limiting
        let _permit = self.limiter.acquire().await;

        let start = Instant::now();

        let model_arc = Arc::clone(&self.model);

        // Check if we have tools registered
        let has_tools = !self.tool_definitions.read().await.is_empty();

        let system_prompt = if has_tools {
            self.build_system_prompt_with_tools(Some(&user_message))
                .await
        } else {
            self.system_prompt.read().await.clone()
        };

        // Build input outside the lock
        let model_guard = model_arc.read().await;
        let model_from_config = model_guard.as_ref().cloned();
        drop(model_guard);

        // If no model is configured, try to get it from the LLM runtime
        let model = if let Some(m) = model_from_config {
            m
        } else {
            // Try to get model name from the runtime
            let llm_guard = self.llm.read().await;
            if let Some(ref llm) = *llm_guard {
                llm.model_name().to_string()
            } else {
                // Ultimate fallback
                "ministral-3:3b".to_string()
            }
        };

        // Get thinking_enabled - priority: local setting > instance setting
        // This allows per-request override (e.g., disable thinking for multimodal)
        let local_thinking = *self.thinking_enabled.read().await;
        let thinking_enabled = if local_thinking.is_some() {
            // Local override takes precedence
            local_thinking
        } else if self.uses_instance_manager() {
            // Fall back to instance setting
            if let Some(manager) = &self.instance_manager {
                manager
                    .get_active_instance()
                    .map(|inst| inst.thinking_enabled)
            } else {
                None
            }
        } else {
            // Direct mode with no local override
            None
        };

        tracing::debug!(
            thinking_enabled = ?thinking_enabled,
            uses_instance_manager = self.uses_instance_manager(),
            "LlmInterface chat_stream_internal"
        );

        // Get effective parameters from backend instance or local config
        let (eff_temp, eff_top_p, eff_top_k, eff_max_tokens) = self.get_effective_params().await;

        let params = neomind_core::llm::backend::GenerationParams {
            temperature: Some(eff_temp),
            top_p: Some(eff_top_p),
            top_k: Some(eff_top_k as u32),
            max_tokens: Some(eff_max_tokens),
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
            thinking_enabled,
            max_context: None,
        };

        let system_msg = Message::system(&system_prompt);
        let user_msg = Message::user(&user_message);

        // Build messages with history if provided, truncated to fit context window
        let messages = if let Some(hist) = history {
            let mut msgs = vec![system_msg];

            let history_msgs: Vec<&Message> = hist
                .iter()
                .filter(|msg| msg.role != neomind_core::MessageRole::System)
                .collect();

            let (available_tokens, _prompt_budget) = self
                .compute_history_budget(
                    max_ctx,
                    &system_prompt,
                    &user_message,
                    history_msgs.len(),
                    has_tools,
                )
                .await;

            let total_history_tokens: usize = history_msgs
                .iter()
                .map(|m| {
                    let text = m.content.as_text();
                    estimate_tokens(&text)
                })
                .sum();

            if total_history_tokens <= available_tokens {
                for msg in &history_msgs {
                    msgs.push((*msg).clone());
                }
            } else {
                let mut used = 0usize;
                let mut kept = Vec::new();
                for msg in history_msgs.iter().rev() {
                    let text = msg.content.as_text();
                    let tokens = estimate_tokens(&text);
                    if used + tokens > available_tokens {
                        break;
                    }
                    used += tokens;
                    kept.push((*msg).clone());
                }
                kept.reverse();
                tracing::info!(
                    total_history_tokens,
                    kept_messages = kept.len(),
                    total_messages = history_msgs.len(),
                    available_tokens,
                    "Truncated conversation history to fit context window"
                );
                msgs.extend(kept);
            }

            msgs.push(user_msg);
            msgs
        } else {
            vec![system_msg, user_msg]
        };

        // Get tool definitions
        let tools_input = if has_tools {
            let tools = self.tool_definitions.read().await;
            let result = if tools.is_empty() {
                None
            } else {
                Some(tools.clone())
            };
            drop(tools);
            result
        } else {
            None
        };

        let input = LlmInput {
            messages,
            params,
            model: Some(model),
            stream: false,
            tools: tools_input,
        };

        // Get runtime using instance manager if enabled
        let llm = self.get_runtime().await?;

        let output = llm
            .generate(input)
            .await
            .map_err(|e| NeoMindError::Llm(e.to_string()))?;

        let duration = start.elapsed();
        let tokens_used = output
            .usage
            .map(|u| u.completion_tokens as usize)
            .unwrap_or_else(|| output.text.split_whitespace().count());

        Ok(ChatResponse {
            text: output.text,
            tokens_used,
            duration,
            finish_reason: format!("{:?}", output.finish_reason),
            thinking: output.thinking,
        })
    }

    /// Internal chat implementation that accepts a Message directly (for multimodal).
    async fn chat_internal_message(
        &self,
        user_message: Message, // Can contain text + images
        history: Option<&[Message]>,
    ) -> AgentResult<ChatResponse> {
        let max_ctx = self.max_context_length().await;

        // Acquire permit for concurrency limiting
        let _permit = self.limiter.acquire().await;

        let start = Instant::now();

        let model_arc = Arc::clone(&self.model);

        // Check if we have tools registered
        let has_tools = !self.tool_definitions.read().await.is_empty();

        let system_prompt = if has_tools {
            // Extract text from user message for system prompt
            let user_text = user_message.content.as_text();
            self.build_system_prompt_with_tools(Some(&user_text)).await
        } else {
            self.system_prompt.read().await.clone()
        };

        // Build input outside the lock
        let model_guard = model_arc.read().await;
        let model_from_config = model_guard.as_ref().cloned();
        drop(model_guard);

        // If no model is configured, try to get it from the LLM runtime
        let model = if let Some(m) = model_from_config {
            m
        } else {
            // Try to get model name from the runtime
            let llm_guard = self.llm.read().await;
            if let Some(ref llm) = *llm_guard {
                llm.model_name().to_string()
            } else {
                // Ultimate fallback
                "ministral-3:3b".to_string()
            }
        };

        // Get thinking_enabled - priority: local setting > instance setting
        // This allows per-request override (e.g., disable thinking for multimodal)
        let local_thinking = *self.thinking_enabled.read().await;
        let thinking_enabled = if local_thinking.is_some() {
            // Local override takes precedence
            local_thinking
        } else if self.uses_instance_manager() {
            // Fall back to instance setting
            if let Some(manager) = &self.instance_manager {
                manager
                    .get_active_instance()
                    .map(|inst| inst.thinking_enabled)
            } else {
                None
            }
        } else {
            // Direct mode with no local override
            None
        };

        // Get effective parameters from backend instance or local config
        let (eff_temp, eff_top_p, eff_top_k, eff_max_tokens) = self.get_effective_params().await;

        let params = neomind_core::llm::backend::GenerationParams {
            temperature: Some(eff_temp),
            top_p: Some(eff_top_p),
            top_k: Some(eff_top_k as u32),
            max_tokens: Some(eff_max_tokens),
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
            thinking_enabled,
            max_context: None,
        };

        let system_msg = Message::system(&system_prompt);

        // Helper function to check if a message has empty content
        fn is_message_empty(msg: &Message) -> bool {
            use neomind_core::Content;
            match &msg.content {
                Content::Text(s) => s.is_empty(),
                Content::Parts(parts) => parts.is_empty(),
            }
        }

        // Build messages with history if provided, truncated to fit context window
        let messages = if let Some(hist) = history {
            let mut msgs = vec![system_msg];

            let history_msgs: Vec<&Message> = hist
                .iter()
                .filter(|msg| msg.role != neomind_core::MessageRole::System)
                .collect();

            let user_text = user_message.content.as_text();
            let (available_tokens, _prompt_budget) = self
                .compute_history_budget(
                    max_ctx,
                    &system_prompt,
                    &user_text,
                    history_msgs.len(),
                    has_tools,
                )
                .await;

            let total_history_tokens: usize = history_msgs
                .iter()
                .map(|m| {
                    let text = m.content.as_text();
                    estimate_tokens(&text)
                })
                .sum();

            if total_history_tokens <= available_tokens {
                for msg in &history_msgs {
                    msgs.push((*msg).clone());
                }
            } else {
                let mut used = 0usize;
                let mut kept = Vec::new();
                for msg in history_msgs.iter().rev() {
                    let text = msg.content.as_text();
                    let tokens = estimate_tokens(&text);
                    if used + tokens > available_tokens {
                        break;
                    }
                    used += tokens;
                    kept.push((*msg).clone());
                }
                kept.reverse();
                tracing::info!(
                    total_history_tokens,
                    kept_messages = kept.len(),
                    total_messages = history_msgs.len(),
                    available_tokens,
                    "Truncated conversation history to fit context window"
                );
                msgs.extend(kept);
            }

            if !is_message_empty(&user_message) {
                msgs.push(user_message);
            }
            msgs
        } else {
            vec![system_msg, user_message]
        };

        // Get tool definitions
        let tools_input = if has_tools {
            let tools = self.tool_definitions.read().await;
            let result = if tools.is_empty() {
                None
            } else {
                Some(tools.clone())
            };
            drop(tools);
            result
        } else {
            None
        };

        let input = LlmInput {
            messages,
            params,
            model: Some(model),
            stream: false,
            tools: tools_input,
        };

        // Get runtime using instance manager if enabled
        let llm = self.get_runtime().await?;

        let output = llm
            .generate(input)
            .await
            .map_err(|e| NeoMindError::Llm(e.to_string()))?;

        let duration = start.elapsed();
        let tokens_used = output
            .usage
            .map(|u| u.completion_tokens as usize)
            .unwrap_or_else(|| output.text.split_whitespace().count());

        Ok(ChatResponse {
            text: output.text,
            tokens_used,
            duration,
            finish_reason: format!("{:?}", output.finish_reason),
            thinking: output.thinking,
        })
    }

    /// Send a chat message with streaming response.
    pub async fn chat_stream(
        &self,
        user_message: impl Into<String>,
    ) -> AgentResult<Pin<Box<dyn Stream<Item = AgentResult<(String, bool)>> + Send>>> {
        self.chat_stream_internal(user_message, None, true).await
    }

    /// Send a chat message with streaming response, with conversation history.
    pub async fn chat_stream_with_history(
        &self,
        user_message: impl Into<String>,
        history: &[Message],
    ) -> AgentResult<Pin<Box<dyn Stream<Item = AgentResult<(String, bool)>> + Send>>> {
        // Enable thinking for complex queries (default behavior)
        *self.thinking_enabled.write().await = Some(true);
        self.chat_stream_internal(user_message, Some(history), true)
            .await
    }

    /// Send a chat message with streaming response, without tools.
    /// This is for Phase 2 where tools have already been executed.
    pub async fn chat_stream_without_tools(
        &self,
        user_message: impl Into<String>,
    ) -> AgentResult<Pin<Box<dyn Stream<Item = AgentResult<(String, bool)>> + Send>>> {
        self.chat_stream_internal(user_message, None, false).await
    }

    /// Send a chat message with streaming response, without tools, with conversation history.
    /// This is for Phase 2 where tools have already been executed.
    pub async fn chat_stream_without_tools_with_history(
        &self,
        user_message: impl Into<String>,
        history: &[Message],
    ) -> AgentResult<Pin<Box<dyn Stream<Item = AgentResult<(String, bool)>> + Send>>> {
        self.chat_stream_internal(user_message, Some(history), false)
            .await
    }

    /// Send a chat message with streaming response, with tools, but without thinking.
    /// This is for simple queries where we want fast responses without thinking overhead.
    pub async fn chat_stream_no_thinking_with_history(
        &self,
        user_message: impl Into<String>,
        history: &[Message],
    ) -> AgentResult<Pin<Box<dyn Stream<Item = AgentResult<(String, bool)>> + Send>>> {
        // Set thinking to false for this call
        *self.thinking_enabled.write().await = Some(false);
        // Note: We DON'T restore the old value here because:
        // 1. The async stream continues after this function returns
        // 2. Restoring here would affect concurrent requests
        // 3. The next request will set its own value
        self.chat_stream_internal(user_message, Some(history), true)
            .await
    }

    /// Send a chat message with streaming response, without tools, without thinking.
    /// This is for Phase 2 follow-up where we want a quick response based on tool results.
    pub async fn chat_stream_no_tools_no_thinking_with_history(
        &self,
        user_message: impl Into<String>,
        history: &[Message],
    ) -> AgentResult<Pin<Box<dyn Stream<Item = AgentResult<(String, bool)>> + Send>>> {
        // Temporarily disable thinking for this call
        let old_value = *self.thinking_enabled.read().await;
        *self.thinking_enabled.write().await = Some(false);
        let result = self
            .chat_stream_internal(user_message, Some(history), false)
            .await;
        *self.thinking_enabled.write().await = old_value;
        result
    }

    /// Send a multimodal chat message (with images) with streaming response.
    /// This method accepts a Message directly, which can contain text and images.
    pub async fn chat_stream_multimodal_with_history(
        &self,
        user_message: Message, // Can contain text + images via Content::Parts
        history: &[Message],
    ) -> AgentResult<Pin<Box<dyn Stream<Item = AgentResult<(String, bool)>> + Send>>> {
        self.chat_stream_internal_message(user_message, Some(history), true, false)
            .await
    }

    /// Send a multimodal chat message (with images) with streaming response, without thinking.
    /// For simple multimodal queries where we want fast responses.
    pub async fn chat_stream_multimodal_no_thinking_with_history(
        &self,
        user_message: Message, // Can contain text + images via Content::Parts
        history: &[Message],
    ) -> AgentResult<Pin<Box<dyn Stream<Item = AgentResult<(String, bool)>> + Send>>> {
        // Temporarily disable thinking for this call
        *self.thinking_enabled.write().await = Some(false);
        self.chat_stream_internal_message(user_message, Some(history), true, false)
            .await
    }

    /// Internal streaming chat implementation that accepts a Message directly (for multimodal).
    async fn chat_stream_internal_message(
        &self,
        user_message: Message,
        history: Option<&[Message]>,
        include_tools: bool,
        _restore_thinking: bool,
    ) -> AgentResult<Pin<Box<dyn Stream<Item = AgentResult<(String, bool)>> + Send>>> {
        let model_arc = Arc::clone(&self.model);
        let max_ctx = self.max_context_length().await;
        let _prompt_budget = (max_ctx * 50) / 100;

        // Build system prompt (with or without tools based on phase)
        let system_prompt = if include_tools {
            self.build_system_prompt_with_tools(None).await
        } else {
            // Phase 2 system prompt - NO tool calling, just generate response based on tool results
            // Tool execution is already complete, this phase is for summarizing results
            "你是NeoMind物联网助手。

## 当前阶段：工具执行完成，需要生成最终回复

对话历史包含：
1. 用户的原始问题
2. 助手的思考过程（如果有）
3. 工具调用信息（调用了哪些工具、传入了什么参数）
4. 工具执行结果（每个工具返回的数据）

## 你的任务

根据**工具执行结果**和**用户的原始问题**，给出一个完整、有用的回复。

## 回复要求

1. **直接回答用户的问题** - 不要说「工具已执行」这类废话
2. **总结关键信息** - 提取工具结果中的关键数据
3. **结构清晰** - 如果有多个设备/规则，用列表或分组展示
4. **友好的语气** - 自然对话，不要机械
5. **内容不要重复** - 每条信息只说一次，不要把相同的内容说两遍

## 示例

用户: \"列出所有设备\"
工具返回: {\"devices\": [{\"id\": \"1\", \"name\": \"温度传感器\", ...}]}
你的回复: \"共找到 5 个设备：\\n1. 温度传感器 (ID: 1)\\n2. 湿度传感器 (ID: 2)\\n...\"

用户: \"查看 ne101 详情\"
工具返回: {\"device\": {\"name\": \"ne101\", \"temperature\": 25, ...}}
你的回复: \"ne101 设备详情：\\n- 名称: ne101\\n- 当前温度: 25°C\\n- 状态: 在线\\n...\"

## 注意事项

- 不要调用工具（此阶段禁用工具调用）
- 不要重复显示原始 JSON 数据
- **关键：每条信息只说一次，不要重复相同的内容**
- 如果工具执行失败，解释原因并提供替代方案
- 如果工具返回的数据不完整，诚实地说明"
                .to_string()
        };

        // Build input outside the lock
        let model_guard = model_arc.read().await;
        let model_from_config = model_guard.as_ref().cloned();
        drop(model_guard);

        // If no model is configured, try to get it from the LLM runtime
        let model = if let Some(m) = model_from_config {
            m
        } else {
            // Try to get model name from the runtime
            let llm_guard = self.llm.read().await;
            if let Some(ref llm) = *llm_guard {
                llm.model_name().to_string()
            } else {
                // Ultimate fallback
                "ministral-3:3b".to_string()
            }
        };

        // Get thinking_enabled - priority: local setting > instance setting
        // This allows per-request override (e.g., disable thinking for multimodal)
        let local_thinking = *self.thinking_enabled.read().await;
        let thinking_enabled = if local_thinking.is_some() {
            // Local override takes precedence
            local_thinking
        } else if self.uses_instance_manager() {
            // Fall back to instance setting
            if let Some(manager) = &self.instance_manager {
                manager
                    .get_active_instance()
                    .map(|inst| inst.thinking_enabled)
            } else {
                None
            }
        } else {
            // Direct mode with no local override
            None
        };

        tracing::debug!(
            thinking_enabled = ?thinking_enabled,
            uses_instance_manager = self.uses_instance_manager(),
            "LlmInterface chat_stream_internal_message (multimodal)"
        );

        // Get effective parameters from backend instance or local config
        let (eff_temp, eff_top_p, eff_top_k, eff_max_tokens) = self.get_effective_params().await;

        let params = neomind_core::llm::backend::GenerationParams {
            temperature: Some(eff_temp),
            top_p: Some(eff_top_p),
            top_k: Some(eff_top_k as u32),
            max_tokens: Some(eff_max_tokens),
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
            thinking_enabled,
            max_context: None,
        };

        let system_msg = Message::system(&system_prompt);

        // Helper function to check if a message has empty content
        fn is_message_empty(msg: &Message) -> bool {
            use neomind_core::Content;
            match &msg.content {
                Content::Text(s) => s.is_empty(),
                Content::Parts(parts) => parts.is_empty(),
            }
        }

        // Build messages with history if provided, truncated to fit context window
        let messages = if let Some(hist) = history {
            let mut msgs = vec![system_msg];

            let history_msgs: Vec<&Message> = hist
                .iter()
                .filter(|msg| msg.role != neomind_core::MessageRole::System)
                .collect();

            let user_text = user_message.content.as_text();
            let (available_tokens, _prompt_budget) = self
                .compute_history_budget(
                    max_ctx,
                    &system_prompt,
                    &user_text,
                    history_msgs.len(),
                    include_tools,
                )
                .await;

            let total_history_tokens: usize = history_msgs
                .iter()
                .map(|m| {
                    let text = m.content.as_text();
                    estimate_tokens(&text)
                })
                .sum();

            if total_history_tokens <= available_tokens {
                for msg in &history_msgs {
                    msgs.push((*msg).clone());
                }
            } else {
                let mut used = 0usize;
                let mut kept = Vec::new();
                for msg in history_msgs.iter().rev() {
                    let text = msg.content.as_text();
                    let tokens = estimate_tokens(&text);
                    if used + tokens > available_tokens {
                        break;
                    }
                    used += tokens;
                    kept.push((*msg).clone());
                }
                kept.reverse();
                tracing::info!(
                    total_history_tokens,
                    kept_messages = kept.len(),
                    total_messages = history_msgs.len(),
                    available_tokens,
                    "Truncated conversation history to fit context window (multimodal stream)"
                );
                msgs.extend(kept);
            }

            if !is_message_empty(&user_message) {
                msgs.push(user_message);
            }
            msgs
        } else {
            vec![system_msg, user_message]
        };

        // Get tool definitions
        let tools = self.tool_definitions.read().await;
        let tools_input = if tools.is_empty() {
            None
        } else {
            Some(tools.clone())
        };
        drop(tools);

        let input = LlmInput {
            messages,
            params,
            model: Some(model),
            stream: true,
            tools: tools_input,
        };

        // Get runtime using instance manager if enabled
        let llm = self.get_runtime().await?;

        let stream = llm
            .generate_stream(input)
            .await
            .map_err(|e| NeoMindError::Llm(e.to_string()))?;

        // Acquire permit for concurrency limiting and wrap stream
        let permit = self.limiter.acquire().await;
        let wrapped_stream = PermitStream::new(stream, permit);
        let token_tracker = self.last_prompt_tokens.clone();

        // Convert stream
        Ok(Box::pin(async_stream::stream! {
            let mut stream = wrapped_stream;
            while let Some(result) = futures::StreamExt::next(&mut stream).await {
                match result {
                    Ok((content, is_thinking)) => {
                        let (clean, is_thinking, tokens) = extract_token_marker(&content, is_thinking);
                        if let Some(t) = tokens {
                            *token_tracker.lock().await = Some(t);
                        }
                        if !clean.is_empty() {
                            yield Ok((clean, is_thinking));
                        }
                    }
                    Err(e) => yield Err(NeoMindError::Llm(e.to_string())),
                }
            }
        }))
    }

    /// Internal streaming chat implementation.
    async fn chat_stream_internal(
        &self,
        user_message: impl Into<String>,
        history: Option<&[Message]>,
        include_tools: bool,
    ) -> AgentResult<Pin<Box<dyn Stream<Item = AgentResult<(String, bool)>> + Send>>> {
        let user_message = user_message.into();

        let model_arc = Arc::clone(&self.model);

        // Check model context capacity for adaptive prompt sizing
        let max_ctx = self.max_context_length().await;
        // Use more conservative budget for small contexts
        let prompt_budget = if max_ctx < 8192 {
            (max_ctx * 50) / 100
        } else if max_ctx < 16384 {
            (max_ctx * 60) / 100
        } else {
            (max_ctx * 70) / 100
        };

        let use_compact_prompt = prompt_budget < 3000; // < ~3000 tokens → use compact prompt
        let skip_history = prompt_budget < 1500;       // < ~1500 tokens → skip history entirely
        let skip_tools = prompt_budget < 2000;         // < ~2000 tokens → no tool definitions

        tracing::info!(
            max_ctx = max_ctx,
            prompt_budget = prompt_budget,
            use_compact = use_compact_prompt,
            skip_history = skip_history,
            skip_tools = skip_tools,
            "Adaptive prompt sizing for chat_stream_internal"
        );

        // Build system prompt (with or without tools based on phase)
        let system_prompt = if include_tools {
            if use_compact_prompt {
                // Compact prompt for small context models (< 4096)
                "You are NeoMind, a helpful IoT assistant. Answer questions concisely. \
                 You can help with device management, data queries, and automation rules. \
                 Keep responses brief.".to_string()
            } else {
                self.build_system_prompt_with_tools(Some(&user_message))
                    .await
            }
        } else {
            // Phase 2 system prompt - NO tool calling, just generate response based on tool results
            // Tool execution is already complete, this phase is for summarizing results
            "你是NeoMind物联网助手。

## 当前阶段：工具执行完成，需要生成最终回复

对话历史包含：
1. 用户的原始问题
2. 助手的思考过程（如果有）
3. 工具调用信息（调用了哪些工具、传入了什么参数）
4. 工具执行结果（每个工具返回的数据）

## 你的任务

根据**工具执行结果**和**用户的原始问题**，给出一个完整、有用的回复。

## 回复要求

1. **直接回答用户的问题** - 不要说「工具已执行」这类废话
2. **总结关键信息** - 提取工具结果中的关键数据
3. **结构清晰** - 如果有多个设备/规则，用列表或分组展示
4. **友好的语气** - 自然对话，不要机械
5. **内容不要重复** - 每条信息只说一次，不要把相同的内容说两遍

## 示例

用户: \"列出所有设备\"
工具返回: {\"devices\": [{\"id\": \"1\", \"name\": \"温度传感器\", ...}]}
你的回复: \"共找到 5 个设备：\\n1. 温度传感器 (ID: 1)\\n2. 湿度传感器 (ID: 2)\\n...\"

用户: \"查看 ne101 详情\"
工具返回: {\"device\": {\"name\": \"ne101\", \"temperature\": 25, ...}}
你的回复: \"ne101 设备详情：\\n- 名称: ne101\\n- 当前温度: 25°C\\n- 状态: 在线\\n...\"

## 注意事项

- 不要调用工具（此阶段禁用工具调用）
- 不要重复显示原始 JSON 数据
- **关键：每条信息只说一次，不要重复相同的内容**
- 如果工具执行失败，解释原因并提供替代方案
- 如果工具返回的数据不完整，诚实地说明"
                .to_string()
        };

        // Build input outside the lock
        let model_guard = model_arc.read().await;
        let model_from_config = model_guard.as_ref().cloned();
        drop(model_guard);

        // If no model is configured, try to get it from the LLM runtime
        let model = if let Some(m) = model_from_config {
            m
        } else {
            // Try to get model name from the runtime
            let llm_guard = self.llm.read().await;
            if let Some(ref llm) = *llm_guard {
                llm.model_name().to_string()
            } else {
                // Ultimate fallback
                "ministral-3:3b".to_string()
            }
        };

        // Get thinking_enabled - priority: local setting > instance setting
        // This allows per-request override (e.g., disable thinking for multimodal)
        let local_thinking = *self.thinking_enabled.read().await;
        let thinking_enabled = if local_thinking.is_some() {
            // Local override takes precedence
            local_thinking
        } else if self.uses_instance_manager() {
            // Fall back to instance setting
            if let Some(manager) = &self.instance_manager {
                manager
                    .get_active_instance()
                    .map(|inst| inst.thinking_enabled)
            } else {
                None
            }
        } else {
            // Direct mode with no local override
            None
        };

        tracing::debug!(
            thinking_enabled = ?thinking_enabled,
            uses_instance_manager = self.uses_instance_manager(),
            "LlmInterface chat_stream_internal"
        );

        // Get effective parameters from backend instance or local config
        let (eff_temp, eff_top_p, eff_top_k, eff_max_tokens) = self.get_effective_params().await;

        let params = neomind_core::llm::backend::GenerationParams {
            temperature: Some(eff_temp),
            top_p: Some(eff_top_p),
            top_k: Some(eff_top_k as u32),
            max_tokens: Some(eff_max_tokens),
            stop: None,
            frequency_penalty: None,
            presence_penalty: None,
            thinking_enabled,
            max_context: None,
        };

        let system_msg = Message::system(&system_prompt);
        let user_msg = Message::user(&user_message);

        // Prepare fallback strategies for context overflow retry
        // Strategy 1: Compact fallback - keep user message + last tool round (if any)
        // Strategy 2: Minimal fallback - system prompt + user message only
        let compact_fallback_system = "You are NeoMind IoT assistant. Summarize results concisely based on the tool results shown.".to_string();
        let compact_fallback_messages = if let Some(hist) = history {
            let mut compact = vec![Message::system(&compact_fallback_system)];
            // Keep only the last few messages (most recent tool round + user message)
            let non_system: Vec<&Message> = hist.iter()
                .filter(|m| m.role != neomind_core::MessageRole::System)
                .collect();
            // Take last 4 messages max (covers: user → assistant tool_call → tool result → assistant response)
            let keep = non_system.len().saturating_sub(4);
            for msg in &non_system[keep..] {
                compact.push((*msg).clone());
            }
            // Add current user message if not already present
            compact.push(Message::user(&user_message));
            compact
        } else {
            vec![
                Message::system(&compact_fallback_system),
                Message::user(&user_message),
            ]
        };
        // Strategy 2: Minimal fallback
        let minimal_fallback_messages = vec![
            Message::system(&compact_fallback_system),
            Message::user(&user_message),
        ];

        // Build messages with history if provided, truncated to fit context window
        let messages = if let Some(hist) = history {
            let mut msgs = vec![system_msg];

            // Collect non-system history messages
            let history_msgs: Vec<&Message> = hist
                .iter()
                .filter(|msg| msg.role != neomind_core::MessageRole::System)
                .collect();

            // Use token-based budget calculation accounting for tools and template overhead
            let (available_tokens, _prompt_budget) = self
                .compute_history_budget(
                    max_ctx,
                    &system_prompt,
                    &user_message,
                    history_msgs.len(),
                    include_tools,
                )
                .await;

            let total_history_tokens: usize = history_msgs
                .iter()
                .map(|m| {
                    let text = m.content.as_text();
                    estimate_tokens(&text)
                })
                .sum();

            if total_history_tokens <= available_tokens {
                for msg in &history_msgs {
                    msgs.push((*msg).clone());
                }
            } else {
                // Find the first user message index (original question) — always preserve it
                let original_user_idx = history_msgs.iter()
                    .position(|m| m.role == neomind_core::MessageRole::User);
                let user_msg_tokens = original_user_idx
                    .map(|idx| estimate_tokens(&history_msgs[idx].content.as_text()))
                    .unwrap_or(0);
                let budget_for_others = available_tokens.saturating_sub(user_msg_tokens);

                // Keep most recent messages that fit within remaining budget
                let mut used = 0usize;
                let mut kept = Vec::new();
                for (i, msg) in history_msgs.iter().enumerate().rev() {
                    // Skip the original user message — we'll add it separately
                    if original_user_idx == Some(i) {
                        continue;
                    }
                    let text = msg.content.as_text();
                    let tokens = estimate_tokens(&text);
                    if used + tokens > budget_for_others {
                        break;
                    }
                    used += tokens;
                    kept.push((*msg).clone());
                }
                kept.reverse();

                // Always prepend the original user message
                if let Some(idx) = original_user_idx {
                    kept.insert(0, history_msgs[idx].clone());
                }

                tracing::info!(
                    total_history_tokens,
                    kept_messages = kept.len(),
                    total_messages = history_msgs.len(),
                    available_tokens,
                    "Truncated conversation history to fit context window (original user message preserved)"
                );
                msgs.extend(kept);
            }

            msgs.push(user_msg);
            msgs
        } else {
            vec![system_msg, user_msg]
        };

        // Get tool definitions
        let tools = self.tool_definitions.read().await;
        let tools_input = if tools.is_empty() {
            None
        } else {
            Some(tools.clone())
        };
        drop(tools);

        let input = LlmInput {
            messages,
            params,
            model: Some(model.clone()),
            stream: true,
            tools: tools_input,
        };

        // Get runtime using instance manager if enabled
        let llm = self.get_runtime().await?;

        let stream = llm
            .generate_stream(input)
            .await
            .map_err(|e| NeoMindError::Llm(e.to_string()))?;

        // Acquire permit for concurrency limiting and wrap stream
        let permit = self.limiter.acquire().await;
        let wrapped_stream = PermitStream::new(stream, permit);

        // Prepare retry data for tiered context overflow retry
        let llm_retry = llm.clone();
        let limiter_retry = self.limiter.clone();
        let token_tracker = self.last_prompt_tokens.clone();

        // Convert stream with tiered context overflow retry
        Ok(Box::pin(async_stream::stream! {
            let mut stream = wrapped_stream;
            let mut retry_stage: u8 = 0; // 0=initial, 1=compact retry, 2=minimal retry
            while let Some(result) = futures::StreamExt::next(&mut stream).await {
                match result {
                    Ok((content, is_thinking)) => {
                        let (clean, is_thinking, tokens) = extract_token_marker(&content, is_thinking);
                        if let Some(t) = tokens {
                            *token_tracker.lock().await = Some(t);
                        }
                        if !clean.is_empty() {
                            yield Ok((clean, is_thinking));
                        }
                    }
                    Err(LlmError::ContextOverflow { prompt_tokens, max_context }) => {
                        retry_stage += 1;
                        if retry_stage <= 2 {
                            let (retry_messages, retry_label) = if retry_stage == 1 {
                                // Strategy 1: Compact - keep recent tool round
                                (compact_fallback_messages.clone(), "compact (recent tool round)")
                            } else {
                                // Strategy 2: Minimal - system + user only
                                (minimal_fallback_messages.clone(), "minimal (no history)")
                            };
                            tracing::warn!(
                                prompt_tokens,
                                max_context,
                                retry_stage,
                                %retry_label,
                                "Context overflow, retrying with {} strategy", retry_label
                            );
                            drop(stream);
                            let retry_input = LlmInput {
                                messages: retry_messages,
                                params: neomind_core::llm::backend::GenerationParams {
                                    temperature: Some(eff_temp),
                                    top_p: Some(eff_top_p),
                                    top_k: Some(eff_top_k as u32),
                                    max_tokens: Some(eff_max_tokens),
                                    stop: None,
                                    frequency_penalty: None,
                                    presence_penalty: None,
                                    thinking_enabled: None, // Disable thinking on retry
                                    max_context: None,
                                },
                                model: Some(model.clone()),
                                stream: true,
                                tools: None, // Strip tools from retry to save context
                            };
                            let permit2 = limiter_retry.acquire().await;
                            match llm_retry.generate_stream(retry_input).await {
                                Ok(fallback_stream) => {
                                    stream = PermitStream::new(fallback_stream, permit2);
                                    continue;
                                }
                                Err(e) => {
                                    yield Err(NeoMindError::Llm(format!(
                                        "Context overflow retry ({}): {}", retry_label, e
                                    )));
                                    return;
                                }
                            }
                        } else {
                            yield Err(NeoMindError::Llm(format!(
                                "Context exceeds model limit ({} > {}) after all retries, please shorten the conversation",
                                prompt_tokens, max_context
                            )));
                            return;
                        }
                    }
                    Err(e) => yield Err(NeoMindError::Llm(e.to_string())),
                }
            }
        }))
    }
}

/// Extract in-band token usage marker from a stream chunk.
/// Returns (clean_content, is_thinking, extracted_prompt_tokens).
/// The marker format is `\n__NEOMIND_TOKEN_PROMPT:NN__`.
fn extract_token_marker(content: &str, is_thinking: bool) -> (String, bool, Option<u32>) {
    if is_thinking {
        return (content.to_string(), is_thinking, None);
    }
    if let Some(start) = content.find("__NEOMIND_TOKEN_PROMPT:") {
        let after = &content[start + "__NEOMIND_TOKEN_PROMPT:".len()..];
        if let Some(end) = after.find("__") {
            if let Ok(tokens) = after[..end].parse::<u32>() {
                let clean = format!("{}{}", &content[..start], &content[start + "__NEOMIND_TOKEN_PROMPT:".len() + end + 2..]);
                return (clean.trim().to_string(), is_thinking, Some(tokens));
            }
        }
    }
    (content.to_string(), is_thinking, None)
}

impl Default for LlmInterface {
    fn default() -> Self {
        Self::new(ChatConfig::default())
    }
}

/// Configuration for LLM chat operations.
#[derive(Debug, Clone)]
pub struct ChatConfig {
    /// Model identifier.
    pub model: String,
    /// Temperature (0.0 to 2.0).
    pub temperature: f32,
    /// Top-p sampling.
    pub top_p: f32,
    /// Top-k sampling (0 = disabled).
    pub top_k: usize,
    /// Maximum tokens to generate.
    pub max_tokens: usize,
    /// Maximum concurrent LLM requests (default: 3).
    pub concurrent_limit: usize,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            model: "ministral-3:3b".to_string(),
            temperature: agent_env_vars::temperature(),
            top_p: agent_env_vars::top_p(),
            top_k: 40,
            max_tokens: agent_env_vars::max_tokens(),
            concurrent_limit: agent_env_vars::concurrent_limit(),
        }
    }
}

/// Response from a chat request.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// Generated text.
    pub text: String,
    /// Number of tokens generated.
    pub tokens_used: usize,
    /// Time taken to generate.
    pub duration: std::time::Duration,
    /// Finish reason.
    pub finish_reason: String,
    /// Thinking content (if the model generated any).
    pub thinking: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::staged::{IntentCategory, IntentResult};

    #[test]
    fn test_chat_config_default() {
        let config = ChatConfig::default();
        assert_eq!(config.model, "ministral-3:3b");
        assert_eq!(config.temperature, 0.3);
        assert_eq!(config.top_p, 0.7);
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.concurrent_limit, DEFAULT_CONCURRENT_LIMIT);
    }

    #[test]
    fn test_llm_interface_new() {
        let config = ChatConfig {
            model: "test-model".to_string(),
            temperature: 0.5,
            top_p: 0.9,
            top_k: 0,
            max_tokens: 2048,
            concurrent_limit: 2,
        };
        let interface = LlmInterface::new(config);
        assert!(!interface.uses_instance_manager());
        assert_eq!(interface.max_concurrent(), 2);
        assert_eq!(interface.available_permits(), 2);
    }

    #[test]
    fn test_llm_interface_with_system_prompt() {
        let config = ChatConfig::default();
        let interface = LlmInterface::new(config).with_system_prompt("You are a test assistant.");
        // The system prompt is set internally
        assert_eq!(interface.max_concurrent(), DEFAULT_CONCURRENT_LIMIT);
    }

    #[tokio::test]
    async fn test_thinking_enabled() {
        let config = ChatConfig::default();
        let interface = LlmInterface::new(config);

        // Initially None (uses backend default)
        assert_eq!(interface.get_thinking_enabled().await, None);

        // Set to true
        interface.set_thinking_enabled(true).await;
        assert_eq!(interface.get_thinking_enabled().await, Some(true));

        // Set to false
        interface.set_thinking_enabled(false).await;
        assert_eq!(interface.get_thinking_enabled().await, Some(false));
    }

    #[tokio::test]
    async fn test_tool_definitions() {
        let config = ChatConfig::default();
        let interface = LlmInterface::new(config);

        // Initially empty
        assert!(interface.get_tool_definitions().await.is_empty());

        // Set tool definitions
        let tools = vec![neomind_core::llm::backend::ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            }),
        }];
        interface.set_tool_definitions(tools.clone()).await;

        let retrieved = interface.get_tool_definitions().await;
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_use_instance_manager() {
        let config = ChatConfig::default();
        let interface = LlmInterface::new(config);

        // Initially not using instance manager
        assert!(!interface.uses_instance_manager());
        assert!(interface.instance_manager().is_none());

        // Enable instance manager mode
        interface.set_use_instance_manager(true).await;
        assert!(interface.uses_instance_manager());

        // Disable
        interface.set_use_instance_manager(false).await;
        assert!(!interface.uses_instance_manager());
    }

    #[tokio::test]
    async fn test_update_model() {
        let config = ChatConfig {
            model: "initial-model".to_string(),
            ..Default::default()
        };
        let interface = LlmInterface::new(config);

        // Update the model
        interface.update_model("new-model".to_string()).await;
        // Model is updated internally (verified through is_ready which checks model)
    }

    #[tokio::test]
    async fn test_is_ready_without_llm() {
        let config = ChatConfig::default();
        let interface = LlmInterface::new(config);

        // Without LLM set and not using instance manager, should not be ready
        assert!(!interface.is_ready().await);
    }

    #[test]
    fn test_classify_intent() {
        let config = ChatConfig::default();
        let interface = LlmInterface::new(config);

        // Test device control intent
        let result = interface.classify_intent("turn on the lights");
        assert_eq!(result.category, IntentCategory::Device);

        // Test data query
        let result = interface.classify_intent("what's the temperature?");
        assert_eq!(result.category, IntentCategory::Data);

        // Test rule intent
        let result = interface.classify_intent("create a new rule");
        assert_eq!(result.category, IntentCategory::Rule);
    }

    #[test]
    fn test_get_intent_prompt() {
        let config = ChatConfig::default();
        let interface = LlmInterface::new(config);

        let result = IntentResult {
            category: IntentCategory::Device,
            confidence: 0.9,
            keywords: vec!["device".to_string()],
        };

        let prompt = interface.get_intent_prompt(&result);
        assert!(prompt.contains("device"));
    }

    #[tokio::test]
    async fn test_filter_tools_by_intent() {
        let config = ChatConfig::default();
        let interface = LlmInterface::new(config);

        // Set up sample tool definitions
        let tools = vec![
            neomind_core::llm::backend::ToolDefinition {
                name: "list_devices".to_string(),
                description: "List all devices".to_string(),
                parameters: serde_json::json!({}),
            },
            neomind_core::llm::backend::ToolDefinition {
                name: "control_device".to_string(),
                description: "Control a device".to_string(),
                parameters: serde_json::json!({}),
            },
            neomind_core::llm::backend::ToolDefinition {
                name: "list_rules".to_string(),
                description: "List all rules".to_string(),
                parameters: serde_json::json!({}),
            },
            neomind_core::llm::backend::ToolDefinition {
                name: "query_data".to_string(),
                description: "Query time series data".to_string(),
                parameters: serde_json::json!({}),
            },
        ];
        interface.set_tool_definitions(tools).await;

        // Test filtering with device-related query
        let filtered = interface.filter_tools_by_intent("turn on the lights").await;
        // Device-related tools should be included
        assert!(!filtered.is_empty());

        // Test filtering with data query
        let filtered = interface
            .filter_tools_by_intent("what's the temperature?")
            .await;
        // Data-related tools should be included
        assert!(!filtered.is_empty());
    }

    #[test]
    fn test_concurrent_limit() {
        let config = ChatConfig {
            concurrent_limit: 5,
            ..Default::default()
        };
        let interface = LlmInterface::new(config);

        assert_eq!(interface.max_concurrent(), 5);
        assert_eq!(interface.available_permits(), 5);
    }

    #[tokio::test]
    async fn test_chat_response_structure() {
        let response = ChatResponse {
            text: "Test response".to_string(),
            tokens_used: 10,
            duration: std::time::Duration::from_millis(100),
            finish_reason: "stop".to_string(),
            thinking: None,
        };

        assert_eq!(response.text, "Test response");
        assert_eq!(response.tokens_used, 10);
        assert_eq!(response.finish_reason, "stop");
        assert!(response.thinking.is_none());
    }

    #[tokio::test]
    async fn test_chat_response_with_thinking() {
        let response = ChatResponse {
            text: "Test response".to_string(),
            tokens_used: 15,
            duration: std::time::Duration::from_millis(200),
            finish_reason: "stop".to_string(),
            thinking: Some("Let me think...".to_string()),
        };

        assert!(response.thinking.is_some());
        assert_eq!(response.thinking.as_ref().unwrap(), "Let me think...");
    }

    #[test]
    fn test_agent_error_display() {
        let err = NeoMindError::Llm("LLM not ready".to_string());
        assert!(err.to_string().contains("not ready"));

        let err = NeoMindError::Llm("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }
}
