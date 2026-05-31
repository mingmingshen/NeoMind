//! AI Agent executor - runs agents and records decision processes.

#![allow(clippy::too_many_arguments)]

use crate::llm_backends::{OllamaConfig, OllamaRuntime};
use crate::memory::compat::persist_agent_memory;
use futures::future::join_all;
use futures::FutureExt;
use neomind_core::llm::backend::LlmRuntime;
use neomind_core::{
    message::{Content, ContentPart, Message, MessageRole},
    EventBus, MetricValue, NeoMindEvent,
};
use neomind_devices::DeviceService;

#[cfg(feature = "cloud")]
use crate::llm_backends::{CloudConfig, CloudRuntime};
use neomind_messages::MessageManager;
use neomind_storage::{
    AgentExecutionRecord,
    AgentMemory,
    AgentResource,
    AgentStore,
    AgentToolConfig,
    AiAgent,
    // New conversation types
    ConversationTurn,
    DataCollected,
    Decision,
    DecisionProcess,
    ExecutionResult as StorageExecutionResult,
    ExecutionStatus,
    GeneratedReport,
    LearnedPattern,
    LlmBackendStore,
    MarkdownMemoryStore,
    ReasoningStep,
    ResourceType,
    TrendPoint,
    TurnInput,
    TurnOutput,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};

// Import DataSourceId for type-safe extension metric queries
use neomind_core::datasource::DataSourceId;

use crate::agent::semantic_mapper::SemanticToolMapper;
use crate::agent::types::LlmBackend;
use crate::error::{NeoMindError, Result as AgentResult};

/// Internal representation of image content for multimodal LLM messages.
#[allow(dead_code)]
enum ImageContent {
    Url(String),
    Base64(String, String), // (_data, _mime_type)
}

/// Intermediate data from the tool execution loop, passed to result construction.
struct ToolCallRecord {
    name: String,
    input: serde_json::Value,
    result: crate::toolkit::ToolResult,
}

struct RoundData {
    thought: Option<String>,
    tool_calls: Vec<ToolCallRecord>,
}

struct ToolLoopOutput {
    final_text: String,
    all_tool_results: Vec<crate::toolkit::ToolResult>,
    /// (thought, tool_calls) per round
    round_data_list_raw: Vec<(Option<String>, Vec<ToolCallRecord>)>,
}

/// Configuration for the tool loop, varying by execution mode.
struct ToolLoopConfig {
    /// Maximum LLM call rounds
    max_rounds: usize,
    /// Recommended tool names (None = no recommendation, all tools available).
    /// Not a whitelist — just prompt guidance for the LLM.
    recommended_tools: Option<Vec<String>>,
    /// Whether this is Focused+ mode (Focused with tool chaining enabled)
    is_focused_plus: bool,
}

impl ToolLoopConfig {
    fn free() -> Self {
        Self {
            max_rounds: 30,
            recommended_tools: None,
            is_focused_plus: false,
        }
    }

    fn focused_plus(agent: &AiAgent) -> Self {
        Self {
            max_rounds: agent.max_chain_depth.clamp(1, 30),
            recommended_tools: Some(Self::build_focused_recommended_tools(agent)),
            is_focused_plus: true,
        }
    }

    fn build_focused_recommended_tools(agent: &AiAgent) -> Vec<String> {
        let mut tools = vec![
            "device".to_string(),
            "skill".to_string(),
            "message".to_string(),
        ];
        for r in &agent.resources {
            if matches!(r.resource_type, ResourceType::Command) {
                tools.push(format!("device:{}", r.resource_id));
            }
            if matches!(r.resource_type, ResourceType::ExtensionTool) {
                tools.push(r.resource_id.clone());
            }
        }
        tools
    }
}

// Sub-modules
mod analyzer;
mod command_executor;
mod context;
mod data_collector;
mod intent;
mod memory;
mod response_parser;

// Re-export public types
pub(crate) use analyzer::AnalysisResult;
pub use context::{DataSourceRef, EventTriggerData};

// Re-export functions needed by sibling modules (via use super::*)
pub(crate) use context::{
    build_history_context, clean_and_truncate_text, conclusion_fingerprint, truncate_to,
    HistoryConfig,
};
pub(crate) use data_collector::get_time_context;
pub(crate) use intent::extract_threshold;
pub(crate) use response_parser::{
    extract_command_from_description, extract_device_from_description, extract_json_from_codeblock,
    extract_json_from_mixed_text, extract_string_field, json_value_to_string, sanitize_json_string,
    summarize_tool_output, try_recover_truncated_json,
};

/// Build JSON Schema parameters from extension command parameters.
fn build_parameters_schema(
    parameters: &[neomind_core::extension::ParameterDefinition],
) -> serde_json::Value {
    use neomind_core::extension::MetricDataType;
    use std::collections::HashMap;

    let mut properties = HashMap::new();
    let mut required = Vec::new();

    for param in parameters {
        let param_type = match param.param_type {
            MetricDataType::Float => "number",
            MetricDataType::Integer => "integer",
            MetricDataType::Boolean => "boolean",
            MetricDataType::String | MetricDataType::Enum { .. } => "string",
            MetricDataType::Binary => "string",
        };

        let mut param_schema = serde_json::json!({
            "type": param_type,
            "description": param.description,
        });

        if let MetricDataType::Enum { options } = &param.param_type {
            param_schema["enum"] = serde_json::json!(options);
        }

        if let Some(default_val) = &param.default_value {
            param_schema["default"] = serde_json::json!(default_val);
        }

        properties.insert(param.name.clone(), param_schema);

        if param.required {
            required.push(param.name.clone());
        }
    }

    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

/// Compact executor message history to prevent context window overflow.
///
/// When the number of non-system messages exceeds `keep_recent * 2`, old tool result
/// messages are replaced with short summaries. The system prompt (first message) and
/// the most recent messages are always preserved.
///
/// This operates directly on `Vec<Message>` used by the executor tool loop.
fn compact_executor_messages(messages: &mut [Message], keep_recent: usize) {
    // Threshold: if we have more than keep_recent * 2 non-system messages, compact.
    let non_system_count = messages
        .iter()
        .filter(|m| m.role != MessageRole::System)
        .count();

    let threshold = keep_recent * 2;
    if non_system_count <= threshold {
        return;
    }

    let to_compact = non_system_count.saturating_sub(keep_recent);
    if to_compact == 0 {
        return;
    }

    tracing::debug!(
        total_messages = messages.len(),
        non_system = non_system_count,
        to_compact,
        "Compacting executor messages"
    );

    // Walk from oldest (index 1, skip system prompt) and compact tool result messages.
    // We need to track how many non-system messages we've compacted.
    let mut compacted_count = 0usize;
    let mut i = 1; // Skip system prompt at index 0
    while i < messages.len() && compacted_count < to_compact {
        if messages[i].role == MessageRole::User {
            let text = messages[i].content.as_text();
            // Skill guide acknowledgment messages start with "Skill guide retrieved"
            if text.starts_with("Skill guide retrieved") {
                let summary = if text.len() > 100 {
                    let preview = &text[..text.floor_char_boundary(80)];
                    format!("[Previous tool result: {}...]", preview)
                } else {
                    format!("[Previous tool result: {}]", text)
                };
                messages[i].content = Content::text(summary);
                compacted_count += 1;
            } else {
                compacted_count += 1;
            }
        } else if messages[i].role == MessageRole::Tool {
            // Native tool result messages (MessageRole::Tool with tool_name)
            let text = messages[i].content.as_text();
            let summary = if text.len() > 100 {
                let preview = &text[..text.floor_char_boundary(80)];
                format!("[Previous tool result: {}...]", preview)
            } else {
                format!("[Previous tool result: {}]", text)
            };
            messages[i].content = Content::text(summary);
            compacted_count += 1;
        } else if messages[i].role == MessageRole::Assistant {
            // Compact old assistant messages to just a brief note
            let text = messages[i].content.as_text();
            if text.len() > 200 {
                let preview = &text[..text.floor_char_boundary(100)];
                messages[i].content =
                    Content::text(format!("[Previous reasoning: {}...]", preview));
            }
            compacted_count += 1;
        }
        i += 1;
    }
}

/// Configuration for agent executor.
#[derive(Clone)]
pub struct AgentExecutorConfig {
    /// Agent store
    pub store: Arc<AgentStore>,
    /// Time series storage for data collection
    pub time_series_storage: Option<Arc<neomind_storage::TimeSeriesStore>>,
    /// Device service for command execution
    pub device_service: Option<Arc<DeviceService>>,
    /// Event bus for event subscription
    pub event_bus: Option<Arc<EventBus>>,
    /// Message manager for sending notifications (replaces AlertManager)
    pub message_manager: Option<Arc<MessageManager>>,
    /// LLM runtime for intent analysis (default)
    pub llm_runtime: Option<Arc<dyn neomind_core::llm::backend::LlmRuntime + Send + Sync>>,
    /// LLM backend store for per-agent backend lookup
    pub llm_backend_store: Option<Arc<LlmBackendStore>>,
    /// Phase 3.3: Extension registry for dynamic tool loading
    pub extension_registry: Option<Arc<neomind_core::extension::registry::ExtensionRegistry>>,
    /// Tool registry for function calling mode
    pub tool_registry: Option<Arc<crate::toolkit::ToolRegistry>>,
    /// Memory store for extracting learned patterns
    pub memory_store: Option<Arc<MarkdownMemoryStore>>,
    /// Per-LLM-backend semaphores concurrency limiting (shared with scheduler)
    pub backend_semaphores: Option<crate::ai_agent::scheduler::BackendSemaphores>,
    /// Skill registry for querying operation guides
    pub skill_registry: Option<crate::skills::SharedSkillRegistry>,
}

/// Context for agent execution.
#[derive(Clone)]
pub struct ExecutionContext {
    /// Agent being executed
    pub agent: AiAgent,
    /// Trigger type (schedule, event, manual)
    pub trigger_type: String,
    /// Current event data (if event-triggered)
    pub event_data: Option<serde_json::Value>,
    /// LLM backend for decision making
    pub llm_backend: Option<LlmBackend>,
    /// Execution ID for event tracking
    pub execution_id: String,
    /// Invocation input from API/Rule/Agent caller
    pub invocation_input: Option<super::AgentInput>,
}

/// Result of agent execution.
pub struct AgentExecutionResult {
    /// Execution record
    pub record: AgentExecutionRecord,
    /// Updated memory
    pub memory: AgentMemory,
    /// Success status
    pub success: bool,
}

/// AI Agent executor - handles execution of user-defined agents.
pub struct AgentExecutor {
    /// Agent store
    store: Arc<AgentStore>,
    /// Time series storage for data collection
    time_series_storage: Option<Arc<neomind_storage::TimeSeriesStore>>,
    /// Device service for command execution
    device_service: Option<Arc<DeviceService>>,
    /// Event bus for publishing events
    event_bus: Option<Arc<EventBus>>,
    /// Message manager for sending notifications (replaces AlertManager)
    message_manager: Option<Arc<MessageManager>>,
    /// Configuration
    _config: AgentExecutorConfig,
    /// LLM runtime (default)
    llm_runtime: Option<Arc<dyn neomind_core::llm::backend::LlmRuntime + Send + Sync>>,
    /// LLM backend store for per-agent backend lookup
    llm_backend_store: Option<Arc<LlmBackendStore>>,
    /// Event-triggered agents cache
    event_agents: Arc<RwLock<HashMap<String, AiAgent>>>,
    /// Track recent executions to prevent duplicates (agent_id, device_id -> timestamp)
    /// Deduplicates by device only, not by individual metrics
    recent_executions: Arc<RwLock<HashMap<String, i64>>>,
    /// LLM runtime cache: backend_id -> runtime
    /// Key format: "{backend_type}:{endpoint}:{model}" for cache invalidation
    llm_runtime_cache:
        Arc<RwLock<HashMap<String, Arc<dyn neomind_core::llm::backend::LlmRuntime + Send + Sync>>>>,
    /// Phase 3.3: Extension registry for dynamic tool loading
    extension_registry: Option<Arc<neomind_core::extension::registry::ExtensionRegistry>>,
    /// Tool registry for function calling mode (wrapped for late initialization)
    tool_registry: parking_lot::RwLock<Option<Arc<crate::toolkit::ToolRegistry>>>,
    /// Memory store for extracting learned patterns
    memory_store: Option<Arc<MarkdownMemoryStore>>,
    /// Per-LLM-backend semaphores for concurrency limiting (shared with scheduler)
    backend_semaphores: Option<crate::ai_agent::scheduler::BackendSemaphores>,
    /// Semaphore limiting concurrent tool executions (default: 6)
    tool_concurrency: Arc<Semaphore>,
}

/// Parse the LLM's final text response to extract situation_analysis, conclusion, and confidence.
///
/// Expects a JSON block like: ```json\n{"situation_analysis":"...","conclusion":"...","confidence":0.8}\n```
/// Falls back to sensible defaults if parsing fails.
impl AgentExecutor {
    /// Publish an event to the event bus (no-op if no bus is configured).
    async fn publish_event(&self, event: NeoMindEvent) {
        if let Some(ref bus) = self.event_bus {
            let _ = bus.publish(event).await;
        }
    }

    /// Create a new agent executor.
    pub async fn new(config: AgentExecutorConfig) -> AgentResult<Self> {
        let llm_runtime = config.llm_runtime.clone();
        let llm_backend_store = config.llm_backend_store.clone();
        let message_manager = config.message_manager.clone();
        let extension_registry = config.extension_registry.clone();
        Ok(Self {
            store: config.store.clone(),
            time_series_storage: config.time_series_storage.clone(),
            device_service: config.device_service.clone(),
            event_bus: config.event_bus.clone(),
            message_manager,
            _config: config.clone(),
            llm_runtime,
            llm_backend_store,
            event_agents: Arc::new(RwLock::new(HashMap::new())),
            recent_executions: Arc::new(RwLock::new(HashMap::new())),
            llm_runtime_cache: Arc::new(RwLock::new(HashMap::new())),
            extension_registry,
            tool_registry: parking_lot::RwLock::new(config.tool_registry.clone()),
            memory_store: config.memory_store.clone(),
            backend_semaphores: config.backend_semaphores.clone(),
            tool_concurrency: Arc::new(Semaphore::new(6)),
        })
    }

    /// Set the LLM runtime for intent parsing.
    pub async fn set_llm_runtime(
        &mut self,
        llm: Arc<dyn neomind_core::llm::backend::LlmRuntime + Send + Sync>,
    ) {
        self.llm_runtime = Some(llm);
    }

    /// Check whether tool mode should be used for this agent execution.
    ///
    /// - Free mode: always uses tool calling (if LLM + registry support it)
    /// - Focused mode: uses tool calling only when `enable_tool_chaining=true`
    /// - Otherwise falls back to structured JSON analysis
    fn should_use_tools(
        &self,
        agent: &AiAgent,
        llm_runtime: &Arc<dyn LlmRuntime + Send + Sync>,
    ) -> bool {
        use neomind_storage::agents::ExecutionMode;

        let llm_supports_tools = llm_runtime.capabilities().function_calling;
        let registry_available = self.tool_registry.read().is_some();

        if !(llm_supports_tools && registry_available) {
            tracing::info!(
                agent_id = %agent.id,
                llm_supports_tools,
                registry_available,
                "Tool mode NOT activated - falling back to structured analysis"
            );
            return false;
        }

        match agent.execution_mode {
            ExecutionMode::Free => true,
            ExecutionMode::Focused => {
                let enabled = agent.enable_tool_chaining;
                if !enabled {
                    tracing::debug!(
                        agent_id = %agent.id,
                        "Tool mode skipped — Focused agent has enable_tool_chaining=false"
                    );
                }
                enabled
            }
        }
    }

    /// Execute agent using tool/function-calling mode.
    ///
    /// In this mode, the LLM receives tool definitions and can make tool calls
    /// that are parsed from its text response, executed, and the results fed back
    /// for further reasoning.
    /// Filter tool definitions based on agent's allowed_tools config.
    ///
    /// Uses cached definitions directly — no JSON round-trip.
    /// Sanitizes tool names for OpenAI-compatible API compatibility
    /// (replacing `.` and `:` with `_`) and returns a reverse mapping
    /// so tool calls from the LLM can be routed back to the correct tool.
    fn filter_tools(
        registry: &crate::toolkit::registry::ToolRegistry,
        tool_config: &Option<AgentToolConfig>,
    ) -> (
        Vec<neomind_core::llm::backend::ToolDefinition>,
        std::collections::HashMap<String, String>,
    ) {
        use neomind_core::llm::backend::sanitize_tool_name;

        let defs = registry.definitions();
        let mut name_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        let to_tool_def = |d: &crate::toolkit::tool::ToolDefinition,
                           map: &mut std::collections::HashMap<String, String>|
         -> neomind_core::llm::backend::ToolDefinition {
            let sanitized = sanitize_tool_name(&d.name);
            if sanitized != d.name {
                map.entry(sanitized.clone())
                    .or_insert_with(|| d.name.clone());
            }
            neomind_core::llm::backend::ToolDefinition {
                name: sanitized,
                description: d.description.clone(),
                parameters: d.parameters.clone(),
            }
        };

        // Both Focused and Free get the same tool set.
        // Focused JSON path: execute_decisions whitelist enforces scope.
        // Free mode (tool calling): all tools available, agent decides what to use.
        let filtered = match tool_config {
            Some(config) if !config.allowed_tools.is_empty() => {
                let allowed: std::collections::HashSet<&str> =
                    config.allowed_tools.iter().map(|s| s.as_str()).collect();

                defs.iter()
                    .filter(|d| allowed.contains(d.name.as_str()))
                    .map(|d| to_tool_def(d, &mut name_map))
                    .collect()
            }
            _ => defs.iter().map(|d| to_tool_def(d, &mut name_map)).collect(),
        };

        (filtered, name_map)
    }

    /// Build the system prompt for tool-calling (Free) mode.
    ///
    /// Unlike the Focused analysis path which filters out memory data for small
    /// models, the Free prompt intentionally **includes** historical context
    /// (learned patterns, baselines, recent conclusions, user messages) so the
    /// agent can leverage accumulated experience and make progressively better
    /// decisions.
    fn build_tool_system_prompt(
        agent: &AiAgent,
        data_collected: &[DataCollected],
        invocation_input: Option<&super::AgentInput>,
        config: &ToolLoopConfig,
    ) -> String {
        let time_ctx = get_time_context();

        // ── Build resource_info and current_data_section based on mode ──
        let (resource_info, current_data_section) = if config.is_focused_plus {
            // Focused+: grouped resources with latest value snapshot,
            // LLM uses tools for historical queries.
            Self::build_focused_plus_sections(agent, data_collected)
        } else {
            // Free mode: full pre-collected data dump, flat resource list.
            Self::build_free_sections(agent, data_collected)
        };

        // ── Historical Context (shared with Focused via build_history_context) ──
        let history_section = build_history_context(agent, &HistoryConfig::free());

        // Build invocation input section
        let invocation_section = match invocation_input {
            Some(input) => {
                let mut parts = Vec::new();
                if let Some(ref source) = input.source {
                    parts.push(format!("来源/Source: {}", source));
                }
                if let Some(ref content) = input.content {
                    parts.push(format!("内容/Content: {}", content));
                }
                if let Some(ref data) = input.data {
                    let data_str = serde_json::to_string_pretty(data).unwrap_or_default();
                    parts.push(format!("附加数据/Data:\n{}", data_str));
                }
                if parts.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\n## Caller Input (invoked by external request)\n{}\n",
                        parts.join("\n")
                    )
                }
            }
            None => String::new(),
        };

        // Mode constraints
        let mut mode_constraints = String::new();
        if let Some(ref recommended) = config.recommended_tools {
            mode_constraints.push_str(&format!(
                "\nRecommended tools for this task (prioritize these): {}",
                recommended.join(", ")
            ));
        }
        if config.is_focused_plus {
            mode_constraints.push_str(&format!(
                "\nYou have at most {} round(s). Be efficient — \
                 use tools to query history or details when the snapshot is insufficient.",
                config.max_rounds
            ));
        }

        let guidelines = if config.is_focused_plus {
            format!(
                "Guidelines:\n\
                 - The snapshot above shows current values. Use `device(action=\"history\")` \
                 with `time_range` to query trends when the task requires historical analysis.\n\
                 - You can use `device(action=\"control\")` to execute bound commands.\n\
                 - Do NOT call the same tool with the same parameters if it already returned results.\n\
                 - Max {} rounds of tool calls. Be efficient.\n\
                 - For complex operations, use the `skill` tool to search for guides.",
                config.max_rounds,
            )
        } else {
            format!(
                "Guidelines:\n\
                 - Do NOT call the same tool with the same parameters if it already returned results.\n\
                 - If a metric query returns empty data, try a different metric or move on.\n\
                 - Max {} rounds of tool calls. Be efficient.\n\
                 - For complex operations (rules, device control, messaging), use the `skill` tool to search for relevant guides before executing.",
                config.max_rounds,
            )
        };

        let exit_guidance = if config.is_focused_plus {
            "\n## When to stop\n\
             Stop calling tools and write your final response when:\n\
             - You have collected enough data to answer the task.\n\
             - A tool call failed and retrying won't help.\n\
             Write your analysis directly — do NOT use JSON or code blocks.\n"
                .to_string()
        } else {
            format!(
                "\n## When to stop\n\
                 IMPORTANT — stop calling tools and write your final response as soon as ONE of these is true:\n\
                 1. You have the data needed to answer the task.\n\
                 2. You have already sent notifications or executed commands (no need to verify them).\n\
                 3. You called a tool and got the same or similar result as before.\n\
                 4. You have used {} rounds and still don't have enough data — summarize what you have.\n\
                 5. A tool failed — explain the failure, don't retry.\n\
                 After your last tool call, write your analysis directly. \
                 Do NOT use JSON or code blocks — plain text only.\n\
                 Keep it concise: key findings first, then anomalies, then recommendations.\n",
                config.max_rounds,
            )
        };

        format!(
            "You are an intelligent IoT agent named '{}' monitoring edge devices.\n\
             Current time: {}\n\
             Your task: {}\n{}{}{}{}{}\
             You have access to tools for querying metrics, executing commands, and sending notifications.\n\n\
             {}\n\
             {}\n\
             Reply in the SAME language as the task description.",
            agent.name,
            time_ctx,
            agent.user_prompt,
            resource_info,
            current_data_section,
            history_section,
            invocation_section,
            mode_constraints,
            guidelines,
            exit_guidance,
        )
    }

    /// Build sections for Focused+ mode:
    /// - resource_info: grouped by type (metrics, commands, extension tools)
    /// - current_data: only latest value snapshots, no history blobs
    fn build_focused_plus_sections(
        agent: &AiAgent,
        data_collected: &[DataCollected],
    ) -> (String, String) {
        // Build a lookup: source -> latest value for snapshot
        let mut latest_values: std::collections::HashMap<&str, String> =
            std::collections::HashMap::new();
        for d in data_collected {
            if d.source == "system" {
                continue;
            }
            if d.values
                .get("_is_image")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                continue;
            }
            if let Some(v) = d.values.get("value") {
                latest_values.insert(&d.source, format!("{}", v));
            }
        }

        // Group resources by type
        let mut metric_lines: Vec<String> = Vec::new();
        let mut command_lines: Vec<String> = Vec::new();

        for r in &agent.resources {
            match r.resource_type {
                ResourceType::Metric => {
                    let current = latest_values
                        .get(r.resource_id.as_str())
                        .map(|v| format!(" (current: {})", v))
                        .unwrap_or_default();
                    metric_lines.push(format!("- {} (`{}`){}", r.name, r.resource_id, current));
                }
                ResourceType::Command => {
                    command_lines.push(format!("- {} (`{}`)", r.name, r.resource_id));
                }
                ResourceType::ExtensionTool => {
                    command_lines.push(format!("- {} (`{}`)", r.name, r.resource_id));
                }
                ResourceType::ExtensionMetric => {
                    let current = latest_values
                        .get(r.resource_id.as_str())
                        .map(|v| format!(" (current: {})", v))
                        .unwrap_or_default();
                    metric_lines.push(format!("- {} (`{}`){}", r.name, r.resource_id, current));
                }
                ResourceType::Device | ResourceType::DataStream => {}
            }
        }

        let mut resource_info = String::from("\n## Bound Resources\n");
        if !metric_lines.is_empty() {
            resource_info.push_str("### Metrics\n");
            for line in &metric_lines {
                resource_info.push_str(line);
                resource_info.push('\n');
            }
        }
        if !command_lines.is_empty() {
            resource_info.push_str("### Commands\n");
            for line in &command_lines {
                resource_info.push_str(line);
                resource_info.push('\n');
            }
        }
        resource_info.push('\n');

        // current_data: snapshot only, no history blobs
        let current_data_section = if latest_values.is_empty() {
            "\n## Current Snapshot\nNo pre-collected data. Use tools to query.\n".to_string()
        } else {
            let mut table = String::from("\n## Current Snapshot (latest values)\n");
            table.push_str("| Resource | Value |\n|----------|-------|\n");
            let mut entries: Vec<_> = latest_values.iter().collect();
            entries.sort_by_key(|(k, _)| *k);
            for (source, value) in entries {
                table.push_str(&format!("| {} | {} |\n", source, value));
            }
            table.push('\n');
            table
        };

        (resource_info, current_data_section)
    }

    /// Build sections for Free mode:
    /// - resource_info: flat resource list
    /// - current_data: full pre-collected data dump
    fn build_free_sections(agent: &AiAgent, data_collected: &[DataCollected]) -> (String, String) {
        let resource_info = if agent.resources.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = agent
                .resources
                .iter()
                .map(|r| format!("- {} ({})", r.name, r.resource_id))
                .collect();
            format!(
                "\nRecommended resources to focus on:\n{}\n",
                items.join("\n")
            )
        };

        let data_text: Vec<String> = data_collected
            .iter()
            .filter(|d| {
                if d.values
                    .get("_is_image")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    return false;
                }
                if d.source == "system"
                    && d.values
                        .get("message")
                        .and_then(|v| v.as_str())
                        .map(|s| s.contains("No pre-collected data"))
                        .unwrap_or(false)
                {
                    return false;
                }
                true
            })
            .map(|d| {
                let json_str = serde_json::to_string_pretty(&d.values).unwrap_or_default();
                format!("**Source: {}**\n{}", d.source, json_str)
            })
            .collect();

        let current_data_section = if data_text.is_empty() {
            "\n## Current Data\nNo pre-collected data available.\n\n\
             **IMPORTANT**: You MUST use the available tools to query the data you need!\n\
             - Use `query_metric` or `get_latest_metrics` to fetch device metrics\n\
             - Use `list_devices` to discover available devices\n\
             - Do NOT conclude \"no data\" without first attempting to query using tools.\n"
                .to_string()
        } else {
            format!("\n## Current Data\n{}\n", data_text.join("\n\n"))
        };

        (resource_info, current_data_section)
    }

    /// Build initial messages (system + user) with multimodal image support.
    fn build_tool_messages(system_prompt: &str, data_collected: &[DataCollected]) -> Vec<Message> {
        // Collect image parts
        let image_parts: Vec<ContentPart> = data_collected
            .iter()
            .filter(|d| {
                d.values
                    .get("_is_image")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .filter_map(|d| {
                if let Some(url) = d.values.get("image_url").and_then(|v| v.as_str()) {
                    if !url.is_empty() {
                        return Some(ContentPart::image_url(url.to_string()));
                    }
                }
                if let Some(base64) = d.values.get("image_base64").and_then(|v| v.as_str()) {
                    if !base64.is_empty() {
                        let mime = d
                            .values
                            .get("image_mime_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("image/jpeg");
                        return Some(ContentPart::image_base64(
                            base64.to_string(),
                            mime.to_string(),
                        ));
                    }
                }
                None
            })
            .collect();

        let user_msg = if !image_parts.is_empty() {
            let mut parts = vec![ContentPart::text(
                "Analyze the current situation and take appropriate actions using the available tools.",
            )];
            parts.extend(image_parts);
            Message::from_parts(MessageRole::User, parts)
        } else {
            Message::new(
                MessageRole::User,
                Content::text("Analyze the current situation and take appropriate actions using the available tools."),
            )
        };

        vec![
            Message::new(MessageRole::System, Content::text(system_prompt)),
            user_msg,
        ]
    }

    /// Run the tool execution loop for up to `max_rounds` LLM calls.
    async fn run_tool_loop(
        &self,
        agent: &AiAgent,
        registry: &crate::toolkit::registry::ToolRegistry,
        llm_runtime: &Arc<dyn LlmRuntime + Send + Sync>,
        filtered_tools: &[neomind_core::llm::backend::ToolDefinition],
        messages: &mut Vec<Message>,
        execution_id: &str,
        max_rounds: usize,
        tool_name_map: &std::collections::HashMap<String, String>,
    ) -> ToolLoopOutput {
        use crate::agent::tool_parser::parse_tool_calls;
        use neomind_core::llm::backend::{GenerationParams, LlmInput};

        // Build reverse map: original_name → sanitized_name
        // Used to convert tool result names back to what the LLM expects
        let original_to_sanitized: std::collections::HashMap<String, String> = tool_name_map
            .iter()
            .map(|(sanitized, original)| (original.clone(), sanitized.clone()))
            .collect();

        let mut all_tool_results: Vec<crate::toolkit::ToolResult> = Vec::new();
        let mut round_data_list: Vec<RoundData> = Vec::new();
        let mut final_text = String::new();
        let mut step_num = 1u32;
        // Accumulate skill tool results separately — inject as concise prompt, not full history
        let mut skill_reference = String::new();
        let mut skill_injected = false;

        // Cross-round tool deduplication: track tool signatures to avoid re-executing
        // the same tool with the same arguments across rounds.
        let mut all_executed_signatures: HashSet<String> = HashSet::new();
        // Duplicate round detection: track tool signatures per round to detect loops.
        let mut prev_round_tool_names: String = String::new();
        let mut consecutive_duplicate_rounds: usize = 0;

        for round in 0..max_rounds {
            // Inject accumulated skill reference into system prompt once, after first tool round
            if round > 0 && !skill_reference.is_empty() && !skill_injected {
                if let Some(sys_msg) = messages.first_mut() {
                    sys_msg.content = Content::text(format!(
                        "{}\n\n## Skill Reference\n{}",
                        sys_msg.content.as_text(),
                        skill_reference
                    ));
                }
                skill_injected = true;
            }

            let input = LlmInput {
                messages: messages.clone(),
                params: GenerationParams {
                    temperature: Some(0.7),
                    max_tokens: Some(4000),
                    ..Default::default()
                },
                model: None,
                stream: false,
                tools: Some(filtered_tools.to_vec()),
            };

            self.send_thinking(
                &agent.id,
                execution_id,
                step_num,
                &format!("Tool execution round {} - calling LLM", round + 1),
            )
            .await;
            step_num += 1;

            let output = match llm_runtime.generate(input).await {
                Ok(o) => o,
                Err(e) => {
                    tracing::warn!(agent_id = %agent.id, error = %e, "LLM generation failed in tool loop");
                    final_text = "LLM generation failed during tool execution.".to_string();
                    break;
                }
            };

            let mut tool_calls = match parse_tool_calls(&output.text) {
                Ok((_, calls)) => calls,
                Err(e) => {
                    // Thinking model fallback: some models (qwen3, deepseek-r1) put tool calls
                    // in the thinking field instead of the main text output.
                    let text_empty = output.text.trim().is_empty() || output.text.trim().len() < 20;
                    if text_empty {
                        if let Some(ref thinking) = output.thinking {
                            if let Ok((_, thinking_calls)) = parse_tool_calls(thinking) {
                                if !thinking_calls.is_empty() {
                                    tracing::debug!(
                                        agent_id = %agent.id,
                                        "Found {} tool calls in thinking field (fallback)",
                                        thinking_calls.len()
                                    );
                                    thinking_calls
                                } else {
                                    tracing::warn!(agent_id = %agent.id, error = %e, "Failed to parse tool calls");
                                    final_text = output.text;
                                    break;
                                }
                            } else {
                                tracing::warn!(agent_id = %agent.id, error = %e, "Failed to parse tool calls");
                                final_text = output.text;
                                break;
                            }
                        } else {
                            tracing::warn!(agent_id = %agent.id, error = %e, "Failed to parse tool calls");
                            final_text = output.text;
                            break;
                        }
                    } else {
                        tracing::warn!(agent_id = %agent.id, error = %e, "Failed to parse tool calls");
                        final_text = output.text;
                        break;
                    }
                }
            };

            // Get remaining text for reasoning tracking
            let remaining_text = match parse_tool_calls(&output.text) {
                Ok((text, _)) => text,
                Err(_) => output.text.clone(),
            };

            if tool_calls.is_empty() {
                final_text = remaining_text;
                break;
            }

            // --- Intra-round deduplication ---
            // Remove duplicate tool calls within the same round (same name + similar args).
            let mut seen_this_round: HashSet<String> = HashSet::new();
            tool_calls.retain(|tc| {
                let args_preview = serde_json::to_string(&tc.arguments).unwrap_or_default();
                let bound = args_preview.len().min(100);
                let args_short = &args_preview[..args_preview.floor_char_boundary(bound)];
                let sig = format!("{}:{}", tc.name, args_short);
                seen_this_round.insert(sig)
            });

            // --- Cross-round deduplication ---
            // Filter out tool calls that were already executed in previous rounds with
            // the same arguments. This prevents small models from wasting tokens.
            let before_count = tool_calls.len();
            tool_calls.retain(|tc| {
                let args_preview = serde_json::to_string(&tc.arguments).unwrap_or_default();
                let bound = args_preview.len().min(100);
                let args_short = &args_preview[..args_preview.floor_char_boundary(bound)];
                let sig = format!("{}:{}", tc.name, args_short);
                all_executed_signatures.insert(sig)
            });
            let deduped_count = before_count - tool_calls.len();
            if deduped_count > 0 {
                tracing::debug!(
                    agent_id = %agent.id,
                    round = round + 1,
                    deduped = deduped_count,
                    "Skipped duplicate tool calls from previous rounds"
                );
            }

            // If all tool calls were duplicates, treat as no-op and ask LLM again
            if tool_calls.is_empty() {
                tracing::warn!(
                    agent_id = %agent.id,
                    round = round + 1,
                    "All tool calls were duplicates, asking LLM to proceed differently"
                );
                messages.push(Message::new(
                    MessageRole::Assistant,
                    Content::text(&output.text),
                ));
                messages.push(Message::new(
                    MessageRole::User,
                    Content::text(
                        "Those tool calls were already executed in previous rounds with the same \
                         arguments. Please use different tools or parameters, or provide your \
                         final answer based on the results you already have.",
                    ),
                ));
                continue;
            }

            // --- Duplicate round detection ---
            // Compare tool signatures (name + key arguments) to detect truly stuck loops.
            // Only counts as duplicate when the FULL round's tool set AND arguments match
            // the previous round — different arguments to the same tool are NOT duplicates.
            let current_round_sig = {
                let mut sigs: Vec<String> = tool_calls
                    .iter()
                    .map(|tc| {
                        let action = tc
                            .arguments
                            .get("action")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let mut sig = format!("{}|{}", tc.name, action);
                        for param in &["device_id", "metric", "agent_id", "rule_id", "extension_id"]
                        {
                            if let Some(val) = tc.arguments.get(*param).and_then(|v| v.as_str()) {
                                sig.push_str(&format!("|{}", val));
                            }
                        }
                        sig
                    })
                    .collect();
                sigs.sort();
                sigs.join(";;")
            };
            if current_round_sig == prev_round_tool_names {
                consecutive_duplicate_rounds += 1;
                tracing::info!(
                    agent_id = %agent.id,
                    round = round + 1,
                    consecutive_duplicates = consecutive_duplicate_rounds,
                    "Duplicate tool round detected (same tools + args) — continuing, cross-round dedup handles re-execution"
                );
            } else {
                consecutive_duplicate_rounds = 0;
            }
            prev_round_tool_names = current_round_sig;

            // Stop after 3+ consecutive identical rounds — the LLM is stuck.
            // Repeated tool calls in complex tasks are normal; cross-round dedup above
            // already prevents actual re-execution.
            if consecutive_duplicate_rounds >= 3 {
                tracing::warn!(
                    agent_id = %agent.id,
                    round = round + 1,
                    consecutive_duplicates = consecutive_duplicate_rounds,
                    "LLM stuck in loop (3+ consecutive duplicate rounds), forcing text response"
                );
                self.send_thinking(
                    &agent.id,
                    execution_id,
                    step_num,
                    "Stopping: detected repeated tool calling pattern, forcing text response",
                )
                .await;
                break;
            }

            tracing::debug!(
                agent_id = %agent.id, round = round + 1, tool_count = tool_calls.len(),
                "Tool calls received"
            );

            self.send_thinking(
                &agent.id,
                execution_id,
                step_num,
                &format!(
                    "Round {}: Executing {} tool(s): {}",
                    round + 1,
                    tool_calls.len(),
                    tool_calls
                        .iter()
                        .map(|tc| tc.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
            .await;
            step_num += 1;

            messages.push(Message::new(
                MessageRole::Assistant,
                Content::text(&output.text),
            ));

            // Execute tools with concurrency limiting via semaphore
            // Map sanitized tool names back to original names for registry lookup
            let calls: Vec<_> = tool_calls
                .iter()
                .map(|tc| {
                    let original_name = tool_name_map
                        .get(&tc.name)
                        .cloned()
                        .unwrap_or_else(|| tc.name.clone());
                    crate::toolkit::registry::ToolCall {
                        name: original_name,
                        args: tc.arguments.clone(),
                        id: Some(tc.id.clone()),
                    }
                })
                .collect();
            let results = if calls.is_empty() {
                Vec::new()
            } else {
                let _permit = self.tool_concurrency.acquire().await.unwrap();
                registry.execute_parallel(calls).await
            };

            let mut round_tool_calls: Vec<ToolCallRecord> = Vec::new();
            for (i, tc) in tool_calls.iter().enumerate() {
                let result =
                    results
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| crate::toolkit::ToolResult {
                            name: tool_name_map
                                .get(&tc.name)
                                .cloned()
                                .unwrap_or_else(|| tc.name.clone()),
                            result: Err(crate::toolkit::error::ToolError::Execution(
                                "No result".to_string(),
                            )),
                        });
                // Use original name for history display
                let display_name = tool_name_map
                    .get(&tc.name)
                    .cloned()
                    .unwrap_or_else(|| tc.name.clone());
                round_tool_calls.push(ToolCallRecord {
                    name: display_name,
                    input: tc.arguments.clone(),
                    result,
                });
            }

            round_data_list.push(RoundData {
                thought: if remaining_text.is_empty() {
                    None
                } else {
                    Some(remaining_text)
                },
                tool_calls: round_tool_calls,
            });

            for result in &results {
                all_tool_results.push(result.clone());
                let result_text = match &result.result {
                    Ok(output) => {
                        let raw = serde_json::to_string_pretty(&output.data)
                            .unwrap_or_else(|_| "Success".to_string());
                        // Sanitize base64/image data to prevent context bloat
                        let sanitized =
                            crate::agent::streaming::sanitize_tool_result_for_prompt(&raw);
                        // UTF-8 safe truncation (has fast-path for short strings)
                        // 128KB limit: large enough for compact time-series and multi-device
                        // queries. The compaction layer handles context window limits later.
                        const MAX_TOOL_RESULT_IN_MSG: usize = 131072;
                        crate::agent::streaming::truncate_result_utf8(
                            &sanitized,
                            MAX_TOOL_RESULT_IN_MSG,
                        )
                    }
                    Err(e) => format!("Error: {}", e),
                };

                // Skill tool results go to separate reference buffer, not messages history
                if result.name == "skill" {
                    if !skill_reference.is_empty() {
                        skill_reference.push_str("\n\n");
                    }
                    skill_reference.push_str(&result_text);
                    // Add a concise acknowledgment to messages so LLM knows the skill was retrieved
                    messages.push(Message::new(
                        MessageRole::User,
                        Content::text("Skill guide retrieved and will be used as reference."),
                    ));
                } else {
                    // Use sanitized name for LLM message so it matches what the LLM used
                    let msg_name = original_to_sanitized
                        .get(&result.name)
                        .cloned()
                        .unwrap_or_else(|| result.name.clone());
                    messages.push(Message::tool_result(&msg_name, &result_text));
                }

                // Send thinking event for each tool result
                let result_preview = match &result.result {
                    Ok(output) => {
                        let brief = summarize_tool_output(&output.data, &result.name);
                        truncate_to(&brief, 200).to_string()
                    }
                    Err(e) => format!("Error: {}", e),
                };
                self.send_thinking(
                    &agent.id,
                    execution_id,
                    step_num,
                    &format!("tool '{}' → {}", result.name, result_preview),
                )
                .await;
                step_num += 1;
            }

            // --- Messages compaction ---
            // When the message history grows too large, compact old tool results into
            // short summaries to prevent context window overflow in subsequent rounds.
            compact_executor_messages(messages, 10);
        }

        // If all rounds exhausted without LLM producing final text, OR if LLM failed
        // mid-loop (error message in final_text), use Focused's Phase 2 pattern to
        // generate a natural language conclusion.
        //
        // Unlike the old JSON-template approach, this sends full tool results (truncated
        // to 8KB each) in [tool_name]\nresult\n\n format — same as Focused Phase 2 — so
        // the LLM has enough data to produce a real analysis.
        let needs_summary = final_text.is_empty()
            || final_text == "LLM generation failed during tool execution."
            || final_text == "Completed tool execution rounds.";
        if needs_summary && !all_tool_results.is_empty() {
            // Clear error text so summary response replaces it
            final_text.clear();

            // Build follow-up prompt — natural language, NOT JSON template.
            // Includes full tool results so the LLM can produce a real analysis.
            let task = &agent.user_prompt;
            let mut phase2_user = format!(
                "{}\n\n[Completed {} rounds of tool execution, {} tool results collected]\n\
                 IMPORTANT: You MUST analyze ALL tool results below and provide a COMPLETE response. \
                 Do NOT just say \"execution completed\" — present the data naturally.\n\n",
                task,
                round_data_list.len().max(1),
                all_tool_results.len(),
            );

            const TOOL_RESULT_MAX_LEN: usize = 131072;
            for r in &all_tool_results {
                let result_text = match &r.result {
                    Ok(output) => {
                        let raw = serde_json::to_string_pretty(&output.data)
                            .unwrap_or_else(|_| "Success".to_string());
                        // Sanitize base64/image data to prevent context bloat
                        let sanitized =
                            crate::agent::streaming::sanitize_tool_result_for_prompt(&raw);
                        crate::agent::streaming::truncate_result_utf8(
                            &sanitized,
                            TOOL_RESULT_MAX_LEN,
                        )
                    }
                    Err(e) => format!("Error: {}", e),
                };
                phase2_user.push_str(&format!("[{}]\n{}\n\n", r.name, result_text));
            }
            phase2_user.push_str(&format!(
                "\nPlease organize the above data to answer: {}",
                task
            ));

            let summary_messages = vec![
                Message::new(
                    MessageRole::System,
                    Content::text(
                        "You are an intelligent IoT assistant. Analyze the tool execution results \
                         and provide a comprehensive, user-friendly response in the SAME language \
                         as the task. Focus on the actual data and insights, not on mentioning that \
                         tools were called."
                    ),
                ),
                Message::new(MessageRole::User, Content::text(&phase2_user)),
            ];

            let summary_input = LlmInput {
                messages: summary_messages,
                params: GenerationParams {
                    temperature: Some(0.7),
                    max_tokens: Some(2000),
                    ..Default::default()
                },
                model: None,
                stream: false,
                tools: None, // No tools — force LLM to answer, not call more tools
            };

            match llm_runtime.generate(summary_input).await {
                Ok(output) => {
                    let text = output.text.trim().to_string();
                    let response_len = text.len();
                    if !text.is_empty() {
                        final_text = text;
                    }
                    tracing::debug!(
                        agent_id = %agent.id,
                        response_len,
                        "Phase 2 analysis generated successfully"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        agent_id = %agent.id,
                        error = %e,
                        "Failed to generate Phase 2 analysis"
                    );
                    // Leave final_text empty — build_tool_result will generate fallback
                }
            }
        }

        if final_text.is_empty() {
            final_text = "Completed tool execution rounds.".to_string();
        }

        ToolLoopOutput {
            final_text,
            all_tool_results,
            round_data_list_raw: round_data_list
                .into_iter()
                .map(|rd| (rd.thought, rd.tool_calls))
                .collect(),
        }
    }

    /// Map tool name to semantic decision type for memory pattern extraction.
    fn tool_name_to_semantic_type(tool_name: &str) -> &'static str {
        match tool_name {
            // Query tools → info
            "query_metric" | "get_latest_metrics" | "list_devices" | "get_device_info" => "info",
            // Control tools → command
            "execute_command" | "execute_extension_command" | "control_device" => "command",
            // Notification tools → alert
            "send_message" | "send_notification" => "alert",
            _ => "info",
        }
    }

    /// Build the final DecisionProcess and ExecutionResult from tool loop output.
    fn build_tool_result(
        agent: &AiAgent,
        data_collected: &[DataCollected],
        loop_output: ToolLoopOutput,
    ) -> (DecisionProcess, neomind_storage::ExecutionResult) {
        let ToolLoopOutput {
            final_text,
            all_tool_results,
            round_data_list_raw,
        } = loop_output;

        // === Free mode: LLM natural language response is the primary output ===
        // Tool calls already executed all actions. The final_text is the LLM's
        // summary/analysis for the user — use it directly as conclusion.

        // --- situation_analysis: use the LLM's first-round thinking as context summary ---
        // This is more meaningful than "Executed N tool operations" because it tells
        // the user what the agent was thinking/trying to accomplish.
        let situation_analysis = round_data_list_raw
            .iter()
            .find_map(|(thought, _)| thought.as_ref().filter(|t| !t.is_empty()))
            .cloned()
            .unwrap_or_else(|| {
                if all_tool_results.is_empty() {
                    "No tools were executed.".to_string()
                } else {
                    format!("Executed {} tool operation(s).", all_tool_results.len())
                }
            });

        // --- conclusion: LLM's natural language response, directly ---
        let is_generic = final_text.is_empty()
            || final_text == "Completed tool execution rounds."
            || final_text == "LLM generation failed during tool execution.";

        let conclusion = if !is_generic {
            final_text.clone()
        } else if !all_tool_results.is_empty() {
            // Fallback: summarize tool results when LLM didn't produce text
            let tool_summary: Vec<String> = all_tool_results
                .iter()
                .map(|r| match &r.result {
                    Ok(output) => summarize_tool_output(&output.data, &r.name),
                    Err(e) => format!("{} failed: {}", r.name, e),
                })
                .collect();
            tool_summary.join("; ") + "."
        } else {
            "No tools were executed during this agent run.".to_string()
        };

        // --- reasoning steps ---
        let mut reasoning_steps: Vec<ReasoningStep> = Vec::new();
        let mut step_counter = 0u32;

        for (thought, tool_calls) in &round_data_list_raw {
            if let Some(thought) = thought {
                step_counter += 1;
                reasoning_steps.push(ReasoningStep {
                    step_number: step_counter,
                    description: thought.clone(),
                    step_type: "thought".to_string(),
                    input: None,
                    output: String::new(),
                    confidence: 0.8,
                });
            }

            for tc in tool_calls {
                step_counter += 1;
                let (desc, conf, step_type) = match &tc.result.result {
                    Ok(output) => (
                        format!("Executed tool '{}'", tc.name),
                        if output.success { 0.9 } else { 0.3 },
                        "tool_call",
                    ),
                    Err(e) => (format!("Tool '{}' failed: {}", tc.name, e), 0.2, "error"),
                };

                let input_str = serde_json::to_string(&tc.input).ok();
                let output_str = match &tc.result.result {
                    Ok(output) => serde_json::to_string(&output.data).unwrap_or_default(),
                    Err(e) => format!("Error: {}", e),
                };

                reasoning_steps.push(ReasoningStep {
                    step_number: step_counter,
                    description: desc,
                    step_type: step_type.to_string(),
                    input: input_str,
                    output: output_str,
                    confidence: conf,
                });
            }
        }

        let decisions: Vec<Decision> = all_tool_results
            .iter()
            .map(|r| {
                let (desc, action) = match &r.result {
                    Ok(output) => {
                        let action_summary = summarize_tool_output(&output.data, &r.name);
                        (format!("Executed tool '{}'", r.name), action_summary)
                    }
                    Err(e) => (format!("Tool '{}' failed", r.name), format!("Error: {}", e)),
                };
                let semantic_type = Self::tool_name_to_semantic_type(&r.name);
                Decision {
                    decision_type: semantic_type.to_string(),
                    description: desc,
                    action,
                    rationale: format!("Tool '{}' executed successfully", r.name),
                    expected_outcome: String::new(),
                }
            })
            .collect();

        // Confidence: based on tool success rate
        let final_confidence = if all_tool_results.is_empty() {
            0.5
        } else {
            let ok = all_tool_results.iter().filter(|r| r.result.is_ok()).count() as f32;
            (ok / all_tool_results.len() as f32).max(0.5)
        };

        let decision_process = DecisionProcess {
            situation_analysis,
            data_collected: data_collected.to_vec(),
            reasoning_steps,
            decisions,
            conclusion,
            confidence: final_confidence,
        };

        let actions_executed: Vec<neomind_storage::ActionExecuted> = all_tool_results
            .iter()
            .map(|r| {
                let success = r.result.is_ok();
                neomind_storage::ActionExecuted {
                    action_type: "tool_call".to_string(),
                    description: format!("Execute tool '{}'", r.name),
                    target: r.name.clone(),
                    parameters: serde_json::Value::Null,
                    success,
                    result: if success {
                        r.result.as_ref().ok().map(|o| o.data.to_string())
                    } else {
                        r.result.as_ref().err().map(|e| e.to_string())
                    },
                }
            })
            .collect();

        let success_rate = if actions_executed.is_empty() {
            1.0
        } else {
            actions_executed.iter().filter(|a| a.success).count() as f32
                / actions_executed.len() as f32
        };

        // summary: the actual LLM response text.
        // Skip generic/error strings — the frontend already shows conclusion separately.
        let summary_text = if final_text.is_empty()
            || final_text == "Completed tool execution rounds."
            || final_text == "LLM generation failed during tool execution."
        {
            String::new()
        } else {
            final_text.clone()
        };
        let execution_result = neomind_storage::ExecutionResult {
            actions_executed,
            report: None,
            notifications_sent: vec![],
            summary: summary_text,
            success_rate,
        };

        tracing::debug!(
            agent_id = %agent.id,
            tool_calls = all_tool_results.len(),
            success_rate,
            "Tool execution completed"
        );

        (decision_process, execution_result)
    }

    async fn execute_with_tools(
        &self,
        agent: &AiAgent,
        data_collected: &[DataCollected],
        llm_runtime: Arc<dyn LlmRuntime + Send + Sync>,
        execution_id: &str,
        invocation_input: Option<&super::AgentInput>,
    ) -> AgentResult<(DecisionProcess, neomind_storage::ExecutionResult)> {
        let registry = self
            .tool_registry
            .read()
            .clone()
            .ok_or_else(|| NeoMindError::Tool("Tool registry not available".to_string()))?;

        // Build mode-specific config
        let tool_config = match agent.execution_mode {
            neomind_storage::agents::ExecutionMode::Free => ToolLoopConfig::free(),
            neomind_storage::agents::ExecutionMode::Focused => ToolLoopConfig::focused_plus(agent),
        };

        let (filtered_tools, tool_name_map) = Self::filter_tools(&registry, &agent.tool_config);
        if !tool_name_map.is_empty() {
            tracing::debug!(
                agent_id = %agent.id,
                tools = ?tool_name_map,
                "Sanitized tool names for API compatibility"
            );
        }
        let system_prompt =
            Self::build_tool_system_prompt(agent, data_collected, invocation_input, &tool_config);
        let mut messages = Self::build_tool_messages(&system_prompt, data_collected);

        let loop_output = self
            .run_tool_loop(
                agent,
                &registry,
                &llm_runtime,
                &filtered_tools,
                &mut messages,
                execution_id,
                tool_config.max_rounds,
                &tool_name_map,
            )
            .await;

        let (decision_process, execution_result) =
            Self::build_tool_result(agent, data_collected, loop_output);

        Ok((decision_process, execution_result))
    }

    /// Get the agent store.
    pub fn store(&self) -> Arc<AgentStore> {
        self.store.clone()
    }

    /// Update the tool registry (e.g. after extensions are loaded).
    pub fn set_tool_registry(&self, registry: Arc<crate::toolkit::ToolRegistry>) {
        *self.tool_registry.write() = Some(registry);
    }

    // ========================================================================
    // Extension Tools Integration
    // ========================================================================

    /// Get the extension registry.
    pub fn extension_registry(
        &self,
    ) -> Option<Arc<neomind_core::extension::registry::ExtensionRegistry>> {
        self.extension_registry.clone()
    }

    /// Get available tools from extensions.
    ///
    /// Returns a list of tool definitions from all registered extensions.
    /// Each extension command becomes a tool that AI agents can call.
    pub async fn get_extension_tools(&self) -> Vec<serde_json::Value> {
        let mut result = Vec::new();

        if let Some(ref registry) = self.extension_registry {
            let extensions = registry.list().await;

            for info in extensions {
                let metadata_id = info.metadata.id.clone();
                let commands = info.commands;

                // Convert each command to a tool description
                for cmd in commands {
                    // Build parameters JSON schema from command parameters
                    let parameters = build_parameters_schema(&cmd.parameters);

                    result.push(serde_json::json!({
                        "name": format!("{}_{}", metadata_id, cmd.name),
                        "description": cmd.description,
                        "parameters": parameters,
                        "extension_id": metadata_id,
                        "command": cmd.name,
                    }));
                }
            }
        }

        result
    }

    /// Execute an extension command.
    ///
    /// This allows agents to call tools provided by extensions.
    pub async fn execute_extension_command(
        &self,
        extension_id: &str,
        command: &str,
        args: &serde_json::Value,
    ) -> AgentResult<serde_json::Value> {
        let registry = self
            .extension_registry
            .as_ref()
            .ok_or_else(|| NeoMindError::Config("Extension registry not configured".to_string()))?;

        registry
            .execute_command(extension_id, command, args)
            .await
            .map_err(|e| NeoMindError::Tool(e.to_string()))
    }

    /// Get all extension tools.
    ///
    /// This is a convenience method that returns all extension tools.
    pub async fn get_all_extension_tools(&self) -> Vec<serde_json::Value> {
        self.get_extension_tools().await
    }

    /// Send a progress event for an agent execution.
    async fn send_progress(
        &self,
        agent_id: &str,
        execution_id: &str,
        stage: &str,
        stage_label: &str,
        details: Option<&str>,
    ) {
        self.publish_event(neomind_core::NeoMindEvent::AgentProgress {
            agent_id: agent_id.to_string(),
            execution_id: execution_id.to_string(),
            stage: stage.to_string(),
            stage_label: stage_label.to_string(),
            progress: None,
            details: details.map(|d| d.to_string()),
            timestamp: chrono::Utc::now().timestamp(),
        })
        .await;
    }

    /// Send a thinking event for an agent execution.
    async fn send_thinking(
        &self,
        agent_id: &str,
        execution_id: &str,
        step_number: u32,
        description: &str,
    ) {
        self.publish_event(neomind_core::NeoMindEvent::AgentThinking {
            agent_id: agent_id.to_string(),
            execution_id: execution_id.to_string(),
            step_number,
            step_type: "progress".to_string(),
            description: description.to_string(),
            details: None,
            timestamp: chrono::Utc::now().timestamp(),
        })
        .await;
    }

    /// Build a cache key for LLM runtime based on backend configuration.
    fn build_runtime_cache_key(backend_type: &str, endpoint: &str, model: &str) -> String {
        format!("{}|{}|{}", backend_type, endpoint, model)
    }

    /// Read a timeout value from an environment variable, falling back to the default.
    fn env_timeout_secs(env_var: &str, default: u64) -> u64 {
        std::env::var(env_var)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default)
    }

    /// Create a cloud LLM runtime from a pre-built `CloudConfig`.
    ///
    /// This deduplicates the common pattern across all cloud backend types:
    /// create config -> build runtime -> override capabilities -> wrap in Arc.
    #[cfg(feature = "cloud")]
    fn create_cloud_runtime(
        config: CloudConfig,
        capabilities: &neomind_storage::BackendCapabilities,
    ) -> Result<Arc<dyn LlmRuntime + Send + Sync>, neomind_core::LlmError> {
        CloudRuntime::new(config).map(|runtime| {
            let runtime = runtime.with_capabilities_override(
                capabilities.supports_multimodal,
                capabilities.supports_thinking,
                capabilities.supports_tools,
                capabilities.max_context,
            );
            Arc::new(runtime) as Arc<dyn LlmRuntime + Send + Sync>
        })
    }

    /// Get the LLM runtime for a specific agent.
    /// If the agent has a specific backend ID configured, use that.
    /// Otherwise, fall back to the default runtime.
    ///
    /// Runtimes are cached by backend configuration to avoid repeated initialization.
    pub async fn get_llm_runtime_for_agent(
        &self,
        agent: &AiAgent,
    ) -> Result<Option<Arc<dyn LlmRuntime + Send + Sync>>, NeoMindError> {
        // Resolve the actual backend ID (handle "default" → active backend)
        let resolved_backend_id = match agent.llm_backend_id.as_deref() {
            Some("default") | None => {
                // Use active backend
                self.llm_backend_store
                    .as_ref()
                    .and_then(|s| s.get_active_backend_id().ok().flatten())
            }
            Some(id) => Some(id.to_string()),
        };

        // If agent has a specific backend ID, try to use it
        if let Some(ref backend_id) = resolved_backend_id {
            if let Some(ref store) = self.llm_backend_store {
                if let Ok(Some(backend)) = store.load_instance(backend_id) {
                    use neomind_storage::LlmBackendType;

                    // Build cache key
                    let endpoint = backend.endpoint.clone().unwrap_or_default();
                    let model = backend.model.clone();
                    let cache_key = Self::build_runtime_cache_key(
                        format!("{:?}", backend.backend_type).as_str(),
                        endpoint.as_str(),
                        model.as_str(),
                    );

                    // Check cache first
                    {
                        let cache = self.llm_runtime_cache.read().await;
                        if let Some(runtime) = cache.get(&cache_key) {
                            tracing::debug!(
                                agent_id = %agent.id,
                                backend = %backend_id,
                                "LLM runtime cache hit"
                            );
                            return Ok(Some(runtime.clone()));
                        }
                    }

                    // Cache miss - create new runtime
                    tracing::debug!(
                        agent_id = %agent.id,
                        backend = %backend_id,
                        "LLM runtime cache miss, creating new runtime"
                    );

                    let runtime: Result<Arc<dyn LlmRuntime + Send + Sync>, _> = match backend
                        .backend_type
                    {
                        LlmBackendType::Ollama => {
                            let endpoint = backend
                                .endpoint
                                .clone()
                                .unwrap_or_else(|| "http://localhost:11434".to_string());
                            let model = backend.model.clone();
                            let timeout = std::env::var("OLLAMA_TIMEOUT_SECS")
                                .ok()
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(120);

                            OllamaRuntime::new(
                                OllamaConfig::new(&model)
                                    .with_endpoint(&endpoint)
                                    .with_timeout_secs(timeout),
                            )
                            .map(|runtime| {
                                let runtime = runtime.with_capabilities_override(
                                    backend.capabilities.supports_multimodal,
                                    backend.capabilities.supports_thinking,
                                    backend.capabilities.supports_tools,
                                    backend.capabilities.max_context,
                                );
                                Arc::new(runtime) as Arc<dyn LlmRuntime + Send + Sync>
                            })
                        }
                        LlmBackendType::OpenAi => {
                            let api_key = backend.api_key.clone().unwrap_or_default();
                            let endpoint = backend
                                .endpoint
                                .clone()
                                .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                            let timeout = Self::env_timeout_secs("OPENAI_TIMEOUT_SECS", 60);
                            Self::create_cloud_runtime(
                                CloudConfig::custom(&api_key, &endpoint)
                                    .with_model(&backend.model)
                                    .with_timeout_secs(timeout),
                                &backend.capabilities,
                            )
                        }
                        LlmBackendType::Anthropic => {
                            let api_key = backend.api_key.clone().unwrap_or_default();
                            let timeout = Self::env_timeout_secs("ANTHROPIC_TIMEOUT_SECS", 60);
                            Self::create_cloud_runtime(
                                CloudConfig::anthropic(&api_key)
                                    .with_model(&backend.model)
                                    .with_timeout_secs(timeout),
                                &backend.capabilities,
                            )
                        }
                        LlmBackendType::Google => {
                            let api_key = backend.api_key.clone().unwrap_or_default();
                            let timeout = Self::env_timeout_secs("GOOGLE_TIMEOUT_SECS", 60);
                            Self::create_cloud_runtime(
                                CloudConfig::google(&api_key)
                                    .with_model(&backend.model)
                                    .with_timeout_secs(timeout),
                                &backend.capabilities,
                            )
                        }
                        LlmBackendType::XAi => {
                            let api_key = backend.api_key.clone().unwrap_or_default();
                            let timeout = Self::env_timeout_secs("XAI_TIMEOUT_SECS", 60);
                            Self::create_cloud_runtime(
                                CloudConfig::grok(&api_key)
                                    .with_model(&backend.model)
                                    .with_timeout_secs(timeout),
                                &backend.capabilities,
                            )
                        }
                        LlmBackendType::Qwen => {
                            let api_key = backend.api_key.clone().unwrap_or_default();
                            let endpoint = backend.endpoint.clone().unwrap_or_else(|| {
                                "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string()
                            });
                            let timeout = Self::env_timeout_secs("QWEN_TIMEOUT_SECS", 60);
                            Self::create_cloud_runtime(
                                CloudConfig::custom(&api_key, &endpoint)
                                    .with_model(&backend.model)
                                    .with_timeout_secs(timeout),
                                &backend.capabilities,
                            )
                        }
                        LlmBackendType::DeepSeek => {
                            let api_key = backend.api_key.clone().unwrap_or_default();
                            let endpoint = backend
                                .endpoint
                                .clone()
                                .unwrap_or_else(|| "https://api.deepseek.com".to_string());
                            let timeout = Self::env_timeout_secs("DEEPSEEK_TIMEOUT_SECS", 60);
                            Self::create_cloud_runtime(
                                CloudConfig::custom(&api_key, &endpoint)
                                    .with_model(&backend.model)
                                    .with_timeout_secs(timeout),
                                &backend.capabilities,
                            )
                        }
                        LlmBackendType::GLM => {
                            let api_key = backend.api_key.clone().unwrap_or_default();
                            let endpoint = backend.endpoint.clone().unwrap_or_else(|| {
                                "https://open.bigmodel.cn/api/paas/v4".to_string()
                            });
                            let timeout = Self::env_timeout_secs("GLM_TIMEOUT_SECS", 60);
                            Self::create_cloud_runtime(
                                CloudConfig::custom(&api_key, &endpoint)
                                    .with_model(&backend.model)
                                    .with_timeout_secs(timeout),
                                &backend.capabilities,
                            )
                        }
                        LlmBackendType::MiniMax => {
                            let api_key = backend.api_key.clone().unwrap_or_default();
                            let endpoint = backend
                                .endpoint
                                .clone()
                                .unwrap_or_else(|| "https://api.minimax.chat/v1".to_string());
                            let timeout = Self::env_timeout_secs("MINIMAX_TIMEOUT_SECS", 60);
                            Self::create_cloud_runtime(
                                CloudConfig::custom(&api_key, &endpoint)
                                    .with_model(&backend.model)
                                    .with_timeout_secs(timeout),
                                &backend.capabilities,
                            )
                        }
                        #[cfg(feature = "llamacpp")]
                        LlmBackendType::LlamaCpp => {
                            let endpoint = backend
                                .endpoint
                                .clone()
                                .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
                            let timeout = Self::env_timeout_secs("LLAMACPP_TIMEOUT_SECS", 180);
                            let config =
                                crate::llm_backends::backends::llamacpp::LlamaCppConfig::new(
                                    &backend.model,
                                )
                                .with_endpoint(&endpoint)
                                .with_timeout_secs(timeout);
                            crate::llm_backends::backends::llamacpp::LlamaCppRuntime::new(config)
                                .map(|rt| {
                                    let rt = rt.with_capabilities_override(
                                        backend.capabilities.supports_multimodal,
                                        backend.capabilities.supports_thinking,
                                        backend.capabilities.supports_tools,
                                        backend.capabilities.max_context,
                                    );
                                    std::sync::Arc::new(rt)
                                        as std::sync::Arc<
                                            dyn neomind_core::llm::backend::LlmRuntime
                                                + Send
                                                + Sync,
                                        >
                                })
                        }
                        #[cfg(not(feature = "llamacpp"))]
                        LlmBackendType::LlamaCpp => {
                            Err(neomind_core::llm::backend::LlmError::BackendUnavailable(
                                "llama.cpp backend is not available (feature not enabled)"
                                    .to_string(),
                            ))
                        }
                    };

                    match runtime {
                        Ok(rt) => {
                            // Store in cache
                            let mut cache = self.llm_runtime_cache.write().await;
                            cache.insert(cache_key, rt.clone());
                            tracing::debug!(
                                agent_id = %agent.id,
                                backend = %backend_id,
                                "LLM runtime created and cached"
                            );
                            return Ok(Some(rt));
                        }
                        Err(e) => {
                            tracing::warn!(
                                agent_id = %agent.id,
                                backend_type = ?backend.backend_type,
                                error = %e,
                                "Failed to create LLM runtime for agent '{}'", agent.name
                            );
                        }
                    }
                }
            }
        }

        // Fall back to default runtime
        Ok(self.llm_runtime.clone())
    }

    /// Parse user intent from natural language using LLM or keyword-based fallback.
    /// Check if an event should trigger any agent and execute it.
    pub async fn check_and_trigger_event(
        &self,
        device_id: String,
        metric: &str,
        value: &MetricValue,
    ) -> AgentResult<()> {
        // Refresh event-triggered agents cache
        self.refresh_event_agents().await;

        let event_agents = self.event_agents.read().await;

        tracing::debug!(
            device_id = %device_id,
            metric = %metric,
            event_agent_count = event_agents.len(),
            "[EVENT] Checking device event against {} event-triggered agents",
            event_agents.len()
        );

        // Clone device_id for use in spawned tasks
        let device_id_for_spawn = device_id.clone();

        // Clean up old entries from recent_executions (older than cooldown window)
        let now = chrono::Utc::now().timestamp();
        let mut recent = self.recent_executions.write().await;
        recent.retain(|_, &mut timestamp| now - timestamp < 360);
        drop(recent);

        for (_agent_id, agent) in event_agents.iter() {
            // Check if this agent has event-based schedule
            if matches!(
                agent.schedule.schedule_type,
                neomind_storage::ScheduleType::Event
            ) {
                // Check if agent's event filter matches this event
                if self
                    .matches_data_source_filter(agent, "device", &device_id, metric)
                    .await
                {
                    // Cooldown: one execution per (agent, source) per 60s window
                    const COOLDOWN_SECS: i64 = 60;
                    let dedup_key = format!("{}:device:{}", agent.id, device_id);
                    let recent = self.recent_executions.read().await;
                    let is_duplicate = recent
                        .get(&dedup_key)
                        .map(|&timestamp| now - timestamp < COOLDOWN_SECS)
                        .unwrap_or(false);
                    drop(recent);

                    if is_duplicate {
                        tracing::info!(
                            agent_name = %agent.name,
                            device_id = %device_id,
                            metric = %metric,
                            "Skipping event-triggered execution (cooldown: {}s)",
                            COOLDOWN_SECS
                        );
                        continue;
                    }

                    // Mark this execution as recent
                    {
                        let mut recent = self.recent_executions.write().await;
                        recent.insert(dedup_key, now);
                    }

                    tracing::debug!(
                        agent_name = %agent.name,
                        device_id = %device_id,
                        metric = %metric,
                        "Event-triggered agent execution"
                    );

                    // Clone the agent and event data for execution
                    let agent_clone = agent.clone();
                    let metric_clone = metric.to_string();
                    let value_clone = value.clone();
                    let device_id_for_task = device_id_for_spawn.clone();
                    let timestamp = chrono::Utc::now().timestamp();

                    // Spawn full agent execution in background
                    let executor_store = self.store.clone();
                    let executor_time_series = self.time_series_storage.clone();
                    let executor_device = self.device_service.clone();
                    let executor_event_bus = self.event_bus.clone();
                    let executor_message_manager = self.message_manager.clone();
                    let executor_llm = self.llm_runtime.clone();
                    let executor_llm_store = self.llm_backend_store.clone();
                    let agent_id_for_log = agent.id.clone();
                    let backend_sems = self.backend_semaphores.clone();
                    let executor_skill_registry = self._config.skill_registry.clone();
                    let executor_tool_registry = self.tool_registry.read().clone();
                    let executor_extension_registry = self.extension_registry.clone();
                    let executor_memory_store = self.memory_store.clone();
                    let backend_id = agent
                        .llm_backend_id
                        .clone()
                        .unwrap_or_else(|| "default".to_string());

                    tokio::spawn(async move {
                        // Acquire per-backend semaphore (WAIT, not fail)
                        if let Some(ref sems) = backend_sems {
                            let backend_sem = sems.get(&backend_id).await;
                            let available = backend_sem.available_permits();
                            if available == 0 {
                                tracing::debug!(
                                    agent_id = %agent_id_for_log,
                                    backend_id = %backend_id,
                                    "Event agent waiting for backend permit"
                                );
                            }
                            let _backend_permit = match backend_sem.acquire().await {
                                Ok(p) => p,
                                Err(_) => {
                                    tracing::warn!(agent_id = %agent_id_for_log, backend_id = %backend_id, "Backend semaphore closed, skipping event-triggered execution");
                                    return;
                                }
                            };
                            tracing::debug!(
                                agent_id = %agent_id_for_log,
                                backend_id = %backend_id,
                                "Event agent acquired backend permit"
                            );
                        }

                        // Create event trigger data
                        let event_trigger_data = EventTriggerData {
                            source: DataSourceRef {
                                source_type: "device".to_string(),
                                source_id: device_id_for_task,
                                field: metric_clone,
                            },
                            value: value_clone,
                            timestamp,
                        };

                        // Create a new executor for this event-triggered execution
                        let executor_config = AgentExecutorConfig {
                            store: executor_store.clone(),
                            time_series_storage: executor_time_series.clone(),
                            device_service: executor_device.clone(),
                            event_bus: executor_event_bus.clone(),
                            message_manager: executor_message_manager,
                            llm_runtime: executor_llm,
                            llm_backend_store: executor_llm_store,
                            extension_registry: executor_extension_registry,
                            tool_registry: executor_tool_registry,
                            memory_store: executor_memory_store,
                            backend_semaphores: backend_sems,
                            skill_registry: executor_skill_registry,
                        };

                        match AgentExecutor::new(executor_config).await {
                            Ok(executor) => {
                                tracing::debug!(
                                    agent_id = %agent_id_for_log,
                                    trigger_device = %event_trigger_data.source.source_id,
                                    trigger_metric = %event_trigger_data.source.field,
                                    "Executing event-triggered agent with event data"
                                );

                                // Execute the agent with event data (includes the triggering metric value directly)
                                match executor
                                    .execute_agent(agent_clone, Some(event_trigger_data), None)
                                    .await
                                {
                                    Ok(record) => {
                                        tracing::debug!(
                                            agent_id = %agent_id_for_log,
                                            execution_id = %record.id,
                                            status = ?record.status,
                                            "Event-triggered agent execution completed"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            agent_id = %agent_id_for_log,
                                            error = %e,
                                            "Event-triggered agent execution failed"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    agent_id = %agent_id_for_log,
                                    error = %e,
                                    "Failed to create executor for event-triggered agent"
                                );
                            }
                        }
                    });
                }
            }
        }

        Ok(())
    }

    /// Unified entry point for triggering agents on any data source update.
    /// Called from the EventBus listener when any data source produces new values.
    pub async fn check_and_trigger_data_event(
        &self,
        source_type: &str,
        source_id: String,
        field: String,
        value: &MetricValue,
    ) -> AgentResult<()> {
        // Refresh event-triggered agents cache
        self.refresh_event_agents().await;

        let event_agents = self.event_agents.read().await;

        tracing::debug!(
            source_type = %source_type,
            source_id = %source_id,
            field = %field,
            event_agent_count = event_agents.len(),
            "[DATA_EVENT] Checking data event against {} event-triggered agents",
            event_agents.len()
        );

        let source_id_for_spawn = source_id.clone();

        // Clean up old entries from recent_executions (older than cooldown window)
        let now = chrono::Utc::now().timestamp();
        let mut recent = self.recent_executions.write().await;
        recent.retain(|_, &mut timestamp| now - timestamp < 360);
        drop(recent);

        for (_agent_id, agent) in event_agents.iter() {
            // Check if this agent has event-based schedule
            if !matches!(
                agent.schedule.schedule_type,
                neomind_storage::ScheduleType::Event
            ) {
                continue;
            }

            // Check if agent's data source filter matches this event
            if !self
                .matches_data_source_filter(agent, source_type, &source_id, &field)
                .await
            {
                continue;
            }

            // Cooldown: one execution per (agent, source) per 60s window
            const COOLDOWN_SECS: i64 = 60;
            let dedup_key = format!("{}:{}:{}", agent.id, source_type, source_id);
            let recent = self.recent_executions.read().await;
            let is_duplicate = recent
                .get(&dedup_key)
                .map(|&timestamp| now - timestamp < COOLDOWN_SECS)
                .unwrap_or(false);
            drop(recent);

            if is_duplicate {
                tracing::info!(
                    agent_name = %agent.name,
                    source_type = %source_type,
                    source_id = %source_id,
                    field = %field,
                    "Skipping data event-triggered execution (cooldown: {}s)",
                    COOLDOWN_SECS
                );
                continue;
            }

            // Mark this execution as recent
            {
                let mut recent = self.recent_executions.write().await;
                recent.insert(dedup_key, now);
            }

            tracing::debug!(
                agent_name = %agent.name,
                source_type = %source_type,
                source_id = %source_id,
                field = %field,
                "Data event-triggered agent execution"
            );

            // Clone the agent and event data for execution
            let agent_clone = agent.clone();
            let field_clone = field.clone();
            let value_clone = value.clone();
            let source_id_for_task = source_id_for_spawn.clone();
            let source_type_for_task = source_type.to_string();
            let timestamp = chrono::Utc::now().timestamp();

            // Spawn full agent execution in background
            let executor_store = self.store.clone();
            let executor_time_series = self.time_series_storage.clone();
            let executor_device = self.device_service.clone();
            let executor_event_bus = self.event_bus.clone();
            let executor_message_manager = self.message_manager.clone();
            let executor_llm = self.llm_runtime.clone();
            let executor_llm_store = self.llm_backend_store.clone();
            let agent_id_for_log = agent.id.clone();
            let backend_sems = self.backend_semaphores.clone();
            let executor_skill_registry = self._config.skill_registry.clone();
            let executor_tool_registry = self.tool_registry.read().clone();
            let executor_extension_registry = self.extension_registry.clone();
            let executor_memory_store = self.memory_store.clone();
            let backend_id = agent
                .llm_backend_id
                .clone()
                .unwrap_or_else(|| "default".to_string());

            tokio::spawn(async move {
                // Acquire per-backend semaphore (WAIT, not fail)
                if let Some(ref sems) = backend_sems {
                    let backend_sem = sems.get(&backend_id).await;
                    let available = backend_sem.available_permits();
                    if available == 0 {
                        tracing::debug!(
                            agent_id = %agent_id_for_log,
                            backend_id = %backend_id,
                            "Data event agent waiting for backend permit"
                        );
                    }
                    // Use expect with clear message - semaphore acquisition should not fail
                    // unless the semaphore is closed, which indicates a serious bug
                    let _backend_permit = backend_sem.acquire().await.expect(
                        "Backend semaphore acquisition failed - semaphore was closed or is broken",
                    );
                    tracing::debug!(
                        agent_id = %agent_id_for_log,
                        backend_id = %backend_id,
                        "Data event agent acquired backend permit"
                    );
                }

                // Create event trigger data with unified DataSourceRef
                let event_trigger_data = EventTriggerData {
                    source: DataSourceRef {
                        source_type: source_type_for_task,
                        source_id: source_id_for_task,
                        field: field_clone,
                    },
                    value: value_clone,
                    timestamp,
                };

                // Create a new executor for this event-triggered execution
                let executor_config = AgentExecutorConfig {
                    store: executor_store.clone(),
                    time_series_storage: executor_time_series.clone(),
                    device_service: executor_device.clone(),
                    event_bus: executor_event_bus.clone(),
                    message_manager: executor_message_manager,
                    llm_runtime: executor_llm,
                    llm_backend_store: executor_llm_store,
                    extension_registry: executor_extension_registry,
                    tool_registry: executor_tool_registry,
                    memory_store: executor_memory_store,
                    backend_semaphores: backend_sems,
                    skill_registry: executor_skill_registry,
                };

                match AgentExecutor::new(executor_config).await {
                    Ok(executor) => {
                        tracing::debug!(
                            agent_id = %agent_id_for_log,
                            trigger_source_type = %event_trigger_data.source.source_type,
                            trigger_source_id = %event_trigger_data.source.source_id,
                            trigger_field = %event_trigger_data.source.field,
                            "Executing data event-triggered agent with event data"
                        );

                        // Execute the agent with event data
                        match executor
                            .execute_agent(agent_clone, Some(event_trigger_data), None)
                            .await
                        {
                            Ok(record) => {
                                tracing::debug!(
                                    agent_id = %agent_id_for_log,
                                    execution_id = %record.id,
                                    status = ?record.status,
                                    "Data event-triggered agent execution completed"
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    agent_id = %agent_id_for_log,
                                    error = %e,
                                    "Data event-triggered agent execution failed"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            agent_id = %agent_id_for_log,
                            error = %e,
                            "Failed to create executor for data event-triggered agent"
                        );
                    }
                }
            });
        }

        Ok(())
    }

    /// Check if a data source update matches an agent's trigger conditions.
    /// For event-type agents: prefers event_filter.sources, falls back to resource bindings.
    /// Agents without any trigger source will NOT be triggered by data events.
    async fn matches_data_source_filter(
        &self,
        agent: &AiAgent,
        source_type: &str,
        source_id: &str,
        field: &str,
    ) -> bool {
        // Build the expected compound resource ID
        let compound_id = format!("{}:{}", source_id, field);

        // 1. Check event_filter.sources — explicit trigger configuration
        // Format: {"sources": [{"type": "device", "id": "sensor-01"}, {"type": "extension", "id": "weather"}]}
        if let Some(ref filter_json) = agent.schedule.event_filter {
            if let Ok(filter) = serde_json::from_str::<serde_json::Value>(filter_json) {
                // New sources-based matching
                if let Some(sources) = filter.get("sources").and_then(|v| v.as_array()) {
                    if !sources.is_empty() {
                        let matches_source = sources.iter().any(|s| {
                            let s_type = s.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            let s_id = s.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let s_field = s.get("field").and_then(|v| v.as_str());

                            if s_type != source_type {
                                return false;
                            }
                            // If id is empty/"all", match any source of this type
                            if s_id.is_empty() || s_id == "all" {
                                return true;
                            }
                            if s_id != source_id {
                                return false;
                            }
                            // If field specified, must match exactly
                            if let Some(f) = s_field {
                                if !f.is_empty() && f != field {
                                    return false;
                                }
                            }
                            true
                        });

                        // When explicit sources are configured, ONLY use them —
                        // do NOT fall through to resource bindings.
                        return matches_source;
                    }
                }

                // Legacy event_type-based matching (backward compat)
                if let Some(event_type) = filter.get("event_type").and_then(|v| v.as_str()) {
                    if event_type == "device.metric" {
                        if let Some(filter_device) =
                            filter.get("device_id").and_then(|v| v.as_str())
                        {
                            if (filter_device == "all" || filter_device == source_id)
                                && source_type == "device"
                            {
                                return true;
                            }
                        }
                    } else if event_type == "extension.output" {
                        if let Some(filter_ext) =
                            filter.get("extension_id").and_then(|v| v.as_str())
                        {
                            if (filter_ext == "all" || filter_ext == source_id)
                                && source_type == "extension"
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // 2. Fallback: check resource bindings (backward compat for agents
        //    without explicit event_filter.sources)
        let has_matching_resource = agent.resources.iter().any(|r| {
            match r.resource_type {
                ResourceType::Device => source_type == "device" && r.resource_id == source_id,
                ResourceType::Metric => {
                    if source_type == "device" {
                        if r.resource_id.contains(':') {
                            // Exact match: "device_id:metric" == "device_id:field"
                            if r.resource_id == compound_id {
                                return true;
                            }
                            // Suffix match: resource "device_id:image" matches field "values.image"
                            // Split resource_id into (res_device, res_field) and compare
                            let parts: Vec<&str> = r.resource_id.splitn(2, ':').collect();
                            if parts.len() == 2 {
                                let res_device = parts[0];
                                let res_field = parts[1];
                                res_device == source_id
                                    && (field == res_field
                                        || field.ends_with(&format!(".{}", res_field)))
                            } else {
                                false
                            }
                        } else {
                            r.resource_id == field
                                || field.ends_with(&format!(".{}", r.resource_id))
                        }
                    } else {
                        false
                    }
                }
                ResourceType::ExtensionMetric => {
                    if source_type == "extension" {
                        let ext_metric_id = format!("{}:{}", source_id, field);
                        r.resource_id == source_id || r.resource_id == ext_metric_id
                    } else {
                        false
                    }
                }
                ResourceType::ExtensionTool => {
                    source_type == "extension" && r.resource_id == source_id
                }
                _ => false,
            }
        });

        if has_matching_resource {
            return true;
        }

        // No resources and no matching trigger sources
        tracing::debug!(
            agent_name = %agent.name,
            source_type = %source_type,
            source_id = %source_id,
            field = %field,
            resources = ?agent.resources.iter().map(|r| &r.resource_id).collect::<Vec<_>>(),
            "[EVENT] Agent {} no matching trigger source",
            agent.name
        );
        false
    }

    /// Refresh the cache of event-triggered agents.
    async fn refresh_event_agents(&self) {
        let filter = neomind_storage::AgentFilter {
            status: Some(neomind_storage::AgentStatus::Active),
            ..Default::default()
        };

        if let Ok(agents) = self.store.query_agents(filter).await {
            let total_active = agents.len();
            let event_agents: HashMap<String, AiAgent> = agents
                .into_iter()
                .filter(|a| {
                    matches!(
                        a.schedule.schedule_type,
                        neomind_storage::ScheduleType::Event
                    )
                })
                .map(|a| (a.id.clone(), a))
                .collect();

            let mut cache = self.event_agents.write().await;
            let previous_count = cache.len();
            *cache = event_agents;

            tracing::debug!(
                total_active_agents = total_active,
                event_triggered_agents = cache.len(),
                previous_count = previous_count,
                "[EVENT] Refreshed event-triggered agents cache"
            );

            // Log each event-triggered agent for debugging
            for (id, agent) in cache.iter() {
                tracing::debug!(
                    agent_id = %id,
                    agent_name = %agent.name,
                    resource_count = agent.resources.len(),
                    "[EVENT] Event-triggered agent: {} with {} resources",
                    agent.name,
                    agent.resources.len()
                );
            }
        }
    }

    /// Remove an agent from the event-triggered agents cache.
    ///
    /// This should be called when an agent is deleted to immediately remove it
    /// from the cache, preventing it from being triggered by events before the
    /// next scheduled refresh.
    pub async fn remove_event_agent(&self, agent_id: &str) {
        let mut cache = self.event_agents.write().await;
        if cache.remove(agent_id).is_some() {
            tracing::debug!(
                agent_id = %agent_id,
                "[EVENT] Removed agent from event-triggered cache"
            );
        }
    }

    /// Execute an agent and record the full decision process.
    pub async fn execute_agent(
        &self,
        agent: AiAgent,
        event_data: Option<EventTriggerData>,
        invocation_input: Option<super::AgentInput>,
    ) -> AgentResult<AgentExecutionRecord> {
        let agent_id = agent.id.clone();
        let agent_name = agent.name.clone();
        let execution_id = uuid::Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();
        let timestamp = chrono::Utc::now().timestamp();

        // Update agent status to executing
        self.store
            .update_agent_status(&agent_id, neomind_storage::AgentStatus::Executing, None)
            .await
            .map_err(|e| NeoMindError::Storage(format!("Failed to update status: {}", e)))?;

        // RAII guard: if execute_agent is cancelled/panics before reaching the
        // normal status-reset at the end, this guard will reset the status on drop.
        // Without this, an agent can be permanently stuck in Executing state.
        struct StatusGuard {
            store: Arc<AgentStore>,
            agent_id: String,
            armed: std::cell::Cell<bool>,
        }

        impl Drop for StatusGuard {
            fn drop(&mut self) {
                if self.armed.get() {
                    let store = self.store.clone();
                    let aid = self.agent_id.clone();
                    tokio::spawn(async move {
                        tracing::warn!(
                            agent_id = %aid,
                            "execute_agent dropped before status reset — force-resetting to Active"
                        );
                        let _ = store
                            .update_agent_status(
                                &aid,
                                neomind_storage::AgentStatus::Active,
                                Some("Execution interrupted - force reset".to_string()),
                            )
                            .await;
                    });
                }
            }
        }

        let status_guard = StatusGuard {
            store: self.store.clone(),
            agent_id: agent_id.clone(),
            armed: std::cell::Cell::new(true),
        };

        // Determine trigger type and context event_data based on whether we have event trigger
        let trigger_type = match &event_data {
            Some(ed) => format!("event:{}", ed.source.field),
            None => "manual".to_string(),
        };

        let context_event_data = event_data.as_ref().map(|ed| {
            serde_json::json!({
                "source_type": ed.source.source_type,
                "source_id": ed.source.source_id,
                "field": ed.source.field,
                "value": serde_json::to_value(&ed.value).unwrap_or_default(),
                "timestamp": ed.timestamp,
            })
        });

        // Create execution context
        let context = ExecutionContext {
            agent: agent.clone(),
            trigger_type: trigger_type.clone(),
            event_data: context_event_data,
            llm_backend: None,
            execution_id: execution_id.clone(),
            invocation_input: invocation_input.clone(),
        };

        // Emit agent execution started event
        tracing::debug!(
            agent_id = %agent_id,
            execution_id = %execution_id,
            trigger_type = %trigger_type,
            has_event_bus = self.event_bus.is_some(),
            "Emitting AgentExecutionStarted event"
        );
        self.publish_event(NeoMindEvent::AgentExecutionStarted {
            agent_id: agent_id.clone(),
            agent_name: agent_name.clone(),
            execution_id: execution_id.clone(),
            trigger_type: trigger_type.clone(),
            timestamp,
        })
        .await;

        // Execute with error handling for stability.
        // Wrap with catch_unwind so panics (e.g. UTF-8 slice bugs) are converted
        // to Err and a proper Failed execution record is created instead of
        // silently disappearing.
        let execution_result: AgentResult<(DecisionProcess, StorageExecutionResult)> =
            match std::panic::AssertUnwindSafe(self.execute_internal(context, event_data.clone()))
                .catch_unwind()
                .await
            {
                Ok(Ok(result)) => Ok(result),
                Ok(Err(e)) => Err(e),
                Err(panic_payload) => {
                    let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                        (*s).to_string()
                    } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                        (*s).clone()
                    } else {
                        "Agent execution panicked".to_string()
                    };
                    tracing::error!(
                        agent_id = %agent_id,
                        execution_id = %execution_id,
                        error = %msg,
                        "Agent execution panicked — converting to error"
                    );
                    Err(NeoMindError::Llm(format!("Execution panic: {}", msg)))
                }
            };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Build execution record
        let (decision_process_for_turn, success) = match &execution_result {
            Ok((dp, _)) => (Some(dp.clone()), true),
            Err(_) => (None, false),
        };

        let record = match execution_result {
            Ok((decision_process, result)) => {
                // Update stats
                if let Err(e) = self
                    .store
                    .update_agent_stats(&agent_id, true, duration_ms)
                    .await
                {
                    tracing::error!(
                        agent_id = %agent_id,
                        error = %e,
                        "Failed to update agent stats after successful execution"
                    );
                }

                AgentExecutionRecord {
                    id: execution_id.clone(),
                    agent_id: agent_id.clone(),
                    timestamp,
                    trigger_type: trigger_type.clone(),
                    status: ExecutionStatus::Completed,
                    decision_process,
                    result: Some(result),
                    duration_ms,
                    error: None,
                }
            }
            Err(e) => {
                // Update stats with failure
                if let Err(stats_err) = self
                    .store
                    .update_agent_stats(&agent_id, false, duration_ms)
                    .await
                {
                    tracing::error!(
                        agent_id = %agent_id,
                        error = %stats_err,
                        "Failed to update agent stats after failed execution"
                    );
                }

                AgentExecutionRecord {
                    id: execution_id.clone(),
                    agent_id: agent_id.clone(),
                    timestamp,
                    trigger_type: trigger_type.clone(),
                    status: ExecutionStatus::Failed,
                    decision_process: DecisionProcess {
                        situation_analysis: format!("Execution failed: {}", e),
                        data_collected: vec![],
                        reasoning_steps: vec![],
                        decisions: vec![],
                        conclusion: format!("Failed: {}", e),
                        confidence: 0.0,
                    },
                    result: None,
                    duration_ms,
                    error: Some(e.to_string()),
                }
            }
        };

        // Save execution record and conversation turn in a single transaction
        tracing::debug!(
            agent_id = %agent_id,
            execution_id = %execution_id,
            has_decision_process = decision_process_for_turn.is_some(),
            success = success,
            "Creating conversation turn"
        );

        let turn = decision_process_for_turn.as_ref().map(|dp| {
            tracing::debug!(
                agent_id = %agent_id,
                execution_id = %execution_id,
                data_collected_count = dp.data_collected.len(),
                reasoning_steps_count = dp.reasoning_steps.len(),
                decisions_count = dp.decisions.len(),
                "Creating conversation turn from decision process"
            );
            // Extract event info for conversation turn if available
            let turn_event_data = event_data.as_ref().map(|ed| {
                serde_json::json!({
                    "source_type": ed.source.source_type,
                    "source_id": ed.source.source_id,
                    "field": ed.source.field,
                    "value": serde_json::to_value(&ed.value).unwrap_or_default(),
                })
            });
            self.create_conversation_turn(
                execution_id.clone(),
                trigger_type.clone(),
                dp.data_collected.clone(),
                turn_event_data,
                dp,
                duration_ms,
                success,
            )
        });

        tracing::debug!(
            agent_id = %agent_id,
            execution_id = %execution_id,
            turn_created = turn.is_some(),
            "About to save execution with conversation"
        );

        self.store
            .save_execution_with_conversation(&record, Some(&agent_id), turn.as_ref())
            .await
            .map_err(|e| NeoMindError::Storage(format!("Failed to save execution: {}", e)))?;

        tracing::debug!(
            agent_id = %agent_id,
            execution_id = %execution_id,
            "Execution and conversation turn saved successfully"
        );

        // Extract and persist memory from successful agent execution
        if record.status == ExecutionStatus::Completed {
            if let Some(ref memory_store) = self.memory_store {
                if let Err(e) = persist_agent_memory(memory_store, &record, &agent_name).await {
                    tracing::warn!(
                        agent_id = %agent_id,
                        error = %e,
                        "Failed to persist agent memory (non-blocking)"
                    );
                }
            }
        }

        // Reset agent status based on result
        // Disarm the RAII guard — normal completion handles status reset
        status_guard.armed.set(false);

        let new_status = if record.status == ExecutionStatus::Completed {
            neomind_storage::AgentStatus::Active
        } else {
            neomind_storage::AgentStatus::Error
        };

        // Retry status reset once on failure to prevent agent getting stuck in Executing state
        match self
            .store
            .update_agent_status(&agent_id, new_status, record.error.clone())
            .await
        {
            Ok(()) => {}
            Err(e) => {
                tracing::error!(
                    agent_id = %agent_id,
                    new_status = ?new_status,
                    error = %e,
                    "Failed to reset agent status after execution, retrying once"
                );
                // Single retry after a short delay
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                if let Err(retry_err) = self
                    .store
                    .update_agent_status(&agent_id, new_status, record.error.clone())
                    .await
                {
                    tracing::error!(
                        agent_id = %agent_id,
                        error = %retry_err,
                        "Agent may be stuck in Executing status after retry failed"
                    );
                }
            }
        }

        // Emit agent execution completed event
        let completion_timestamp = chrono::Utc::now().timestamp();
        self.publish_event(NeoMindEvent::AgentExecutionCompleted {
            agent_id: agent_id.clone(),
            execution_id: execution_id.clone(),
            success: record.status == ExecutionStatus::Completed,
            duration_ms: record.duration_ms,
            error: record.error.clone(),
            timestamp: completion_timestamp,
        })
        .await;

        tracing::info!(
            agent_id = %agent_id,
            agent_name = %agent_name,
            execution_id = %execution_id,
            status = ?record.status,
            duration_ms = record.duration_ms,
            "Agent execution completed"
        );

        Ok(record)
    }

    /// Execute multiple agents in parallel for improved performance.
    ///
    /// This is especially useful for multi-agent scenarios where agents
    /// have independent tasks and can run concurrently.
    ///
    /// # Example
    /// ```text
    /// let agents = vec![monitor_agent, executor_agent, analyst_agent];
    /// let results = executor.execute_agents_parallel(agents).await?;
    /// // Results are returned in the same order as input agents
    /// ```
    pub async fn execute_agents_parallel(
        &self,
        agents: Vec<AiAgent>,
    ) -> AgentResult<Vec<AgentExecutionRecord>> {
        use futures::future::join_all;

        // Sort agents by priority (higher priority first)
        let mut sorted_agents = agents;
        sorted_agents.sort_by(|a, b| b.priority.cmp(&a.priority));

        let executor_ref = self;
        let futures: Vec<_> = sorted_agents
            .into_iter()
            .map(|agent| executor_ref.execute_agent(agent, None, None))
            .collect();

        let results = join_all(futures).await;

        // Collect results, converting any errors into a combined error
        let mut records = Vec::new();
        let mut errors = Vec::new();

        for result in results {
            match result {
                Ok(record) => records.push(record),
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            tracing::warn!(
                count = errors.len(),
                "Some agents failed during parallel execution"
            );
        }

        if records.is_empty() && !errors.is_empty() {
            return Err(NeoMindError::Storage(format!(
                "All {} agents failed. First error: {}",
                errors.len(),
                errors[0]
            )));
        }

        Ok(records)
    }

    /// Internal execution logic.
    async fn execute_internal(
        &self,
        context: ExecutionContext,
        event_data: Option<EventTriggerData>,
    ) -> AgentResult<(DecisionProcess, StorageExecutionResult)> {
        let mut agent = context.agent;
        let agent_id = agent.id.clone();
        let execution_id = context.execution_id.clone();
        let mut step_num = 1u32;

        // Progress: Collecting data
        self.send_progress(
            &agent_id,
            &execution_id,
            "collecting",
            "Collecting data",
            Some("Gathering sensor data..."),
        )
        .await;

        // Step 1: Collect data (with or without event data)
        let data_collected = if let Some(ref ed) = event_data {
            self.collect_data_with_event(&agent, ed).await?
        } else {
            self.collect_data(&agent).await?
        };

        // Send thinking events for each data source collected
        for data in &data_collected {
            let desc = if event_data.is_some() {
                format!("Collecting {}: {} data points", data.source, data.data_type)
            } else {
                format!("Collected data source: {}", data.source)
            };
            self.send_thinking(&agent_id, &execution_id, step_num, &desc)
                .await;
            step_num += 1;
            // Small delay for visual effect
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        // Progress: Analyzing
        self.send_progress(
            &agent_id,
            &execution_id,
            "analyzing",
            "Analyzing",
            Some(&format!(
                "Analyzing {} data points...",
                data_collected.len()
            )),
        )
        .await;

        // Step 1.5: Parse intent if not already done
        let parsed_intent = if agent.parsed_intent.is_none() {
            match self.parse_intent(&agent.user_prompt).await {
                Ok(intent) => {
                    // Update agent with parsed intent
                    if let Err(e) = self
                        .store
                        .update_agent_parsed_intent(&agent.id, Some(intent.clone()))
                        .await
                    {
                        tracing::warn!(agent_id = %agent.id, error = %e, "Failed to store parsed intent");
                    }
                    Some(intent)
                }
                Err(e) => {
                    tracing::warn!(agent_id = %agent.id, error = %e, "Failed to parse intent, using default");
                    None
                }
            }
        } else {
            agent.parsed_intent.clone()
        };

        // Update agent reference with parsed intent
        if let Some(ref intent) = parsed_intent {
            agent.parsed_intent = Some(intent.clone());
        }

        // Step 2: Analyze situation — returns AnalysisResult which branches
        // Free vs Focused.
        let analysis = self
            .analyze_situation_with_intent(
                &agent,
                &data_collected,
                parsed_intent.as_ref(),
                &context.execution_id,
                context.invocation_input.as_ref(),
            )
            .await?;

        match analysis {
            // ── Tool-calling path (Free + Focused+) ────────────────────────
            // Tool-calling mode already produced a full DecisionProcess and
            // ExecutionResult.  We only need to update memory and return.
            AnalysisResult::Free {
                decision_process,
                execution_result,
            } => {
                self.send_thinking(
                    &agent_id,
                    &execution_id,
                    step_num,
                    &format!(
                        "Tool-calling analysis completed: {} tool call(s), confidence {:.0}%",
                        decision_process.decisions.len(),
                        decision_process.confidence * 100.0
                    ),
                )
                .await;

                // Update memory with Free mode results
                let updated_memory = self
                    .update_memory(
                        &agent,
                        &decision_process.data_collected,
                        &decision_process.decisions,
                        &decision_process.situation_analysis,
                        &decision_process.conclusion,
                        &execution_id,
                        true,
                    )
                    .await?;

                self.store
                    .update_agent_memory(&agent.id, updated_memory.clone())
                    .await
                    .map_err(|e| {
                        NeoMindError::Storage(format!("Failed to update memory: {}", e))
                    })?;

                // Extract learned patterns into system memory
                // DISABLED: The memory scheduler already runs periodic extraction.
                // Per-execution extraction is redundant, wastes tokens, and uses the wrong model.
                // See: memory/scheduler.rs for the scheduled extraction path.

                tracing::debug!(
                    agent_id = %agent_id,
                    "[TOOL-CALLING] Returning direct results — skipped Focused JSON post-processing"
                );

                Ok((decision_process, execution_result))
            }

            // ── Focused path ───────────────────────────────────────────────
            // Standard single-pass LLM or rule-based analysis.  Follow the
            // original pipeline: execute_decisions → report → memory → store.
            AnalysisResult::Focused {
                situation_analysis,
                reasoning_steps,
                decisions,
                conclusion,
            } => {
                // Send thinking event for analysis completion
                self.send_thinking(
                    &agent_id,
                    &execution_id,
                    step_num,
                    &format!(
                        "Analysis completed: Generated {} decision(s)",
                        decisions.len()
                    ),
                )
                .await;
                step_num += 1;

                // Progress: Executing decisions
                self.send_progress(
                    &agent_id,
                    &execution_id,
                    "executing",
                    "Executing decisions",
                    Some(&format!("Executing {} decision(s)...", decisions.len())),
                )
                .await;

                // Send initial executing status
                self.send_thinking(
                    &agent_id,
                    &execution_id,
                    step_num,
                    &format!("Starting execution of {} decision(s)", decisions.len()),
                )
                .await;
                step_num += 1;

                // Step 3: Execute decisions
                let (actions_executed, notifications_sent) =
                    self.execute_decisions(&agent, &decisions).await?;

                // Send thinking events for each action executed
                for action in &actions_executed {
                    self.send_thinking(
                        &agent_id,
                        &execution_id,
                        step_num,
                        &format!("Executing: {} -> {}", action.action_type, action.target),
                    )
                    .await;
                    step_num += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }

                // Send thinking events for notifications
                for notification in &notifications_sent {
                    self.send_thinking(
                        &agent_id,
                        &execution_id,
                        step_num,
                        &format!("Sending notification: {}", notification.message),
                    )
                    .await;
                    step_num += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }

                // Send completion event for executing stage
                if actions_executed.is_empty() && notifications_sent.is_empty() {
                    self.send_thinking(
                        &agent_id,
                        &execution_id,
                        step_num,
                        "Execution completed: No additional actions required",
                    )
                    .await;
                } else {
                    self.send_thinking(
                        &agent_id,
                        &execution_id,
                        step_num,
                        &format!(
                            "Execution completed: {} action(s), {} notification(s)",
                            actions_executed.len(),
                            notifications_sent.len()
                        ),
                    )
                    .await;
                }

                // Note: refine_conclusion_with_query_results is intentionally NOT called
                // here. This Focused JSON path does not produce data_query actions
                // (those come from tool-calling mode). Calling it would always return
                // None and waste an await.

                // Step 4: Generate report if needed
                let report = self.maybe_generate_report(&agent, &data_collected).await?;

                // Step 5: Update memory with learnings
                let updated_memory = self
                    .update_memory(
                        &agent,
                        &data_collected,
                        &decisions,
                        &situation_analysis,
                        &conclusion,
                        &execution_id,
                        true,
                    )
                    .await?;

                // Save updated memory
                self.store
                    .update_agent_memory(&agent.id, updated_memory.clone())
                    .await
                    .map_err(|e| {
                        NeoMindError::Storage(format!("Failed to update memory: {}", e))
                    })?;

                // Bridge: extract learned patterns into system memory
                // DISABLED: The memory scheduler already runs periodic extraction.
                // Per-execution extraction is redundant, wastes tokens, and uses the wrong model.
                // See: memory/scheduler.rs for the scheduled extraction path.

                // Calculate confidence from reasoning
                let confidence = if reasoning_steps.is_empty() {
                    0.5
                } else {
                    reasoning_steps.iter().map(|s| s.confidence).sum::<f32>()
                        / reasoning_steps.len() as f32
                };

                // No truncation — preserve full LLM output for quality
                let summary_for_result = conclusion.clone();

                let decision_process = DecisionProcess {
                    situation_analysis,
                    data_collected,
                    reasoning_steps,
                    decisions,
                    conclusion,
                    confidence,
                };

                let success_rate = if actions_executed.is_empty() {
                    1.0
                } else {
                    let success_count =
                        actions_executed.iter().filter(|a| a.success).count() as f32;
                    success_count / actions_executed.len() as f32
                };

                let execution_result = StorageExecutionResult {
                    actions_executed,
                    report,
                    notifications_sent,
                    summary: summary_for_result,
                    success_rate,
                };

                Ok((decision_process, execution_result))
            }
        }
    }
}
