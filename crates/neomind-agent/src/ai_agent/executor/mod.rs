//! AI Agent executor - runs agents and records decision processes.

#![allow(clippy::too_many_arguments)]

use crate::llm_backends::{OllamaConfig, OllamaRuntime};
use crate::memory::compat::persist_agent_memory;
use futures::future::join_all;
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
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Import DataSourceId for type-safe extension metric queries
use neomind_core::datasource::DataSourceId;

use crate::agent::semantic_mapper::SemanticToolMapper;
use crate::agent::types::LlmBackend;
use crate::error::{NeoMindError, Result as AgentResult};
use crate::prompts::LANGUAGE_POLICY;

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
    /// Whether final_text is a generic fallback (no LLM response) vs real output
    is_generic_fallback: bool,
    all_tool_results: Vec<crate::toolkit::ToolResult>,
    /// Text the LLM produced between tool calls (thinking/reasoning)
    all_reasoning_texts: Vec<String>,
    /// (thought, tool_calls) per round
    round_data_list_raw: Vec<(Option<String>, Vec<ToolCallRecord>)>,
}


// Sub-modules
mod response_parser;
mod context;
mod data_collector;
mod analyzer;
mod command_executor;
mod memory;
mod intent;

// Re-export public types
pub use context::{EventTriggerData, DataSourceRef, ChainState, ChainResult};
pub(crate) use analyzer::AnalysisResult;

// Re-export functions needed by sibling modules (via use super::*)
pub(crate) use response_parser::{
    extract_command_from_description, extract_device_from_description,
    json_value_to_string, extract_string_field, sanitize_json_string,
    extract_json_from_mixed_text, try_recover_truncated_json,
    parse_final_tool_response, summarize_tool_output, extract_json_from_codeblock,
};
pub(crate) use context::{
    clean_and_truncate_text, truncate_to, score_turn_relevance,
};
pub(crate) use data_collector::{
    get_time_context,
};
pub(crate) use intent::{
    extract_threshold,
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
    tool_registry: std::sync::RwLock<Option<Arc<crate::toolkit::ToolRegistry>>>,
    /// Memory store for extracting learned patterns
    memory_store: Option<Arc<MarkdownMemoryStore>>,
    /// Per-LLM-backend semaphores for concurrency limiting (shared with scheduler)
    backend_semaphores: Option<crate::ai_agent::scheduler::BackendSemaphores>,
}

/// Calculate relevance score for a conversation turn based on current context.
///
/// Scoring factors (inspired by MemoryOS heat-based approach):
/// - Time decay (30%): exp(-0.03 * age_hours) - recent turns score higher
/// - Success reference (20%): successful turns are more valuable
/// - Device overlap (30%): turns involving same devices are more relevant
/// - Trigger similarity (20%): same trigger type suggests similar context
///
/// Returns a score between 0.0 (irrelevant) and 1.0 (highly relevant).

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
            tool_registry: std::sync::RwLock::new(config.tool_registry.clone()),
            memory_store: config.memory_store.clone(),
            backend_semaphores: config.backend_semaphores.clone(),
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
    /// Only Free mode agents use tool calling; Focused mode always uses
    /// structured LLM analysis.
    fn should_use_tools(
        &self,
        agent: &AiAgent,
        llm_runtime: &Arc<dyn LlmRuntime + Send + Sync>,
    ) -> bool {
        // Focused mode never uses tools
        if agent.execution_mode != neomind_storage::agents::ExecutionMode::Free {
            tracing::debug!(
                agent_id = %agent.id,
                execution_mode = ?agent.execution_mode,
                "Tool mode skipped — agent is not in Free execution mode"
            );
            return false;
        }

        let llm_supports_tools = llm_runtime.capabilities().function_calling;
        let registry_available = self.tool_registry.read().unwrap().is_some();
        let result = llm_supports_tools && registry_available;
        if !result {
            tracing::warn!(
                agent_id = %agent.id,
                llm_supports_tools,
                registry_available,
                "Tool mode NOT activated - one or more conditions not met"
            );
        }
        result
    }

    /// Execute agent using tool/function-calling mode.
    ///
    /// In this mode, the LLM receives tool definitions and can make tool calls
    /// that are parsed from its text response, executed, and the results fed back
    /// for further reasoning.
    /// Filter tool definitions based on agent's allowed_tools config.
    fn filter_tools(
        registry: &crate::toolkit::registry::ToolRegistry,
        tool_config: &Option<AgentToolConfig>,
    ) -> Vec<neomind_core::llm::backend::ToolDefinition> {
        let tool_defs_json = registry.definitions_json();
        let tools_list = tool_defs_json
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();

        let to_tool_def = |t: &serde_json::Value| -> Option<neomind_core::llm::backend::ToolDefinition> {
            Some(neomind_core::llm::backend::ToolDefinition {
                name: t.get("name")?.as_str()?.to_string(),
                description: t.get("description")?.as_str()?.to_string(),
                parameters: t.get("parameters")?.clone(),
            })
        };

        match tool_config {
            Some(config) if !config.allowed_tools.is_empty() => tools_list
                .iter()
                .filter(|t| {
                    t.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| config.allowed_tools.contains(&n.to_string()))
                        .unwrap_or(true)
                })
                .filter_map(|t| to_tool_def(t))
                .collect(),
            _ => tools_list.iter().filter_map(|t| to_tool_def(t)).collect(),
        }
    }

    /// Build the system prompt for tool-calling (Free) mode.
    ///
    /// Unlike the Focused analysis path which filters out memory data for small
    /// models, the Free prompt intentionally **includes** historical context
    /// (learned patterns, baselines, recent conclusions, user messages) so the
    /// agent can leverage accumulated experience and make progressively better
    /// decisions.
    fn build_tool_system_prompt(agent: &AiAgent, data_collected: &[DataCollected], invocation_input: Option<&super::AgentInput>) -> String {
        let time_ctx = get_time_context();

        // Collect non-image, non-placeholder data.
        // Keep memory / baselines / patterns data — they are valuable for Free mode.
        let data_text: Vec<String> = data_collected
            .iter()
            .filter(|d| {
                // Exclude images
                if d.values.get("_is_image").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return false;
                }
                // Exclude placeholder data from collect_data
                if d.source == "system"
                    && d.values.get("message").and_then(|v| v.as_str())
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

        let resource_info = if agent.resources.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = agent.resources.iter()
                .map(|r| format!("- {} ({})", r.name, r.resource_id))
                .collect();
            format!("\nRecommended resources to focus on:\n{}\n", items.join("\n"))
        };

        let current_data_section = if data_text.is_empty() {
            "\n## Current Data\nNo pre-collected data available.\n\n\
             **IMPORTANT**: You MUST use the available tools to query the data you need!\n\
             - Use `query_metric` or `get_latest_metrics` to fetch device metrics\n\
             - Use `list_devices` to discover available devices\n\
             - Do NOT conclude \"no data\" without first attempting to query using tools.\n"
        } else {
            &format!("\n## Current Data\n{}\n", data_text.join("\n\n"))
        };

        // ── Historical Context (Free mode exclusive) ──
        // Build a rich context from agent memory so the LLM can learn from
        // past executions and user feedback.
        let mut history_parts: Vec<String> = Vec::new();

        // 1. Recent conclusions from short-term memory
        if !agent.memory.short_term.summaries.is_empty() {
            let recent: Vec<String> = agent.memory.short_term.summaries
                .iter()
                .rev()
                .take(3)
                .map(|s| {
                    let status = if s.success { "OK" } else { "FAIL" };
                    format!("- [{}] {}", status, truncate_to(&s.conclusion, 80))
                })
                .collect();
            history_parts.push(format!("### Recent Executions\n{}", recent.join("\n")));
        }

        // 2. Learned patterns (high confidence only)
        if !agent.memory.learned_patterns.is_empty() {
            let patterns: Vec<String> = agent.memory.learned_patterns
                .iter()
                .filter(|p| p.confidence >= 0.6)
                .take(5)
                .map(|p| format!("- [{}] {} ({:.0}%)", p.pattern_type, p.description, p.confidence * 100.0))
                .collect();
            if !patterns.is_empty() {
                history_parts.push(format!("### Learned Patterns\n{}", patterns.join("\n")));
            }
        }

        // 3. Baseline values
        if !agent.memory.baselines.is_empty() {
            let bl: Vec<String> = agent.memory.baselines
                .iter()
                .take(5)
                .map(|(k, v)| format!("- {}: {:.2}", k, v))
                .collect();
            history_parts.push(format!("### Known Baselines\n{}", bl.join("\n")));
        }

        // 4. User messages (highest priority instructions)
        if !agent.user_messages.is_empty() {
            let msgs: Vec<String> = agent.user_messages
                .iter()
                .rev()
                .take(5)
                .map(|m| {
                    let ts = chrono::DateTime::from_timestamp(m.timestamp, 0)
                        .map(|dt| dt.format("%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "??".to_string());
                    format!("- [{}] {}", ts, m.content)
                })
                .collect();
            history_parts.push(format!(
                "### User Instructions (HIGHEST PRIORITY)\n\
                 These override any conflicting rules from initial config:\n{}",
                msgs.join("\n")
            ));
        }

        let history_section = if history_parts.is_empty() {
            String::new()
        } else {
            format!("\n## Historical Context (learn from past experience)\n{}\n", history_parts.join("\n\n"))
        };

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
                    format!("\n## Caller Input (invoked by external request)\n{}\n", parts.join("\n"))
                }
            }
            None => String::new(),
        };

        format!(
            "You are an intelligent IoT agent named '{}' monitoring edge devices.\n\
             Current time: {}\n\
             Your task: {}\n{}{}{}{}\
             You have access to tools for querying metrics, executing commands, and sending notifications.\n\
             Always use tools to fetch the latest data before making conclusions.\n\n\
             Guidelines:\n\
             - Do NOT call the same tool with the same parameters if it already returned results.\n\
             - If a metric query returns empty data, try a different metric or move on.\n\
             - Max 3 rounds of tool calls. Be efficient.\n\
             - For complex operations (rules, device control, messaging), use the `skill` tool to search for relevant guides before executing.\n\n\
             When you have enough information, respond with your analysis in natural language. \
             Do NOT wrap your response in JSON or code blocks — just write your analysis directly.\n\
             Keep your conclusion concise and structured (2-5 sentences). \
             State key findings first, then anomalies or recommendations. Avoid filler words.\n\
             Reply in the SAME language as the task description.",
            agent.name, time_ctx, agent.user_prompt, resource_info, current_data_section, history_section, invocation_section,
        )
    }

    /// Build initial messages (system + user) with multimodal image support.
    fn build_tool_messages(
        system_prompt: &str,
        data_collected: &[DataCollected],
    ) -> Vec<Message> {
        // Collect image parts
        let image_parts: Vec<ContentPart> = data_collected
            .iter()
            .filter(|d| d.values.get("_is_image").and_then(|v| v.as_bool()).unwrap_or(false))
            .filter_map(|d| {
                if let Some(url) = d.values.get("image_url").and_then(|v| v.as_str()) {
                    if !url.is_empty() {
                        return Some(ContentPart::image_url(url.to_string()));
                    }
                }
                if let Some(base64) = d.values.get("image_base64").and_then(|v| v.as_str()) {
                    if !base64.is_empty() {
                        let mime = d.values.get("image_mime_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("image/jpeg");
                        return Some(ContentPart::image_base64(base64.to_string(), mime.to_string()));
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
    ) -> ToolLoopOutput {
        use crate::agent::tool_parser::parse_tool_calls;
        use neomind_core::llm::backend::{GenerationParams, LlmInput};

        // Focused mode or tool chaining disabled: single round only.
        // Free mode with chaining: use max_chain_depth.
        let max_rounds = if agent.enable_tool_chaining {
            agent.max_chain_depth.max(1)
        } else {
            1
        };
        let mut all_tool_results: Vec<crate::toolkit::ToolResult> = Vec::new();
        let mut round_data_list: Vec<RoundData> = Vec::new();
        let mut all_reasoning_texts: Vec<String> = Vec::new();
        let mut final_text = String::new();
        let mut step_num = 1u32;
        // Accumulate skill tool results separately — inject as concise prompt, not full history
        let mut skill_reference = String::new();
        let mut skill_injected = false;

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
                &agent.id, execution_id, step_num,
                &format!("Tool execution round {} - calling LLM", round + 1),
            ).await;
            step_num += 1;

            let output = match llm_runtime.generate(input).await {
                Ok(o) => o,
                Err(e) => {
                    tracing::warn!(agent_id = %agent.id, error = %e, "LLM generation failed in tool loop");
                    final_text = "LLM generation failed during tool execution.".to_string();
                    break;
                }
            };

            let (remaining_text, tool_calls) = match parse_tool_calls(&output.text) {
                Ok(parsed) => parsed,
                Err(e) => {
                    tracing::warn!(agent_id = %agent.id, error = %e, "Failed to parse tool calls");
                    final_text = output.text;
                    break;
                }
            };

            if tool_calls.is_empty() {
                final_text = remaining_text;
                break;
            }

            // Collect LLM's reasoning text (what it said before/during tool calls)
            if !remaining_text.is_empty() {
                all_reasoning_texts.push(remaining_text.clone());
            }

            tracing::debug!(
                agent_id = %agent.id, round = round + 1, tool_count = tool_calls.len(),
                "Tool calls received"
            );

            self.send_thinking(
                &agent.id, execution_id, step_num,
                &format!(
                    "Round {}: Executing {} tool(s): {}",
                    round + 1, tool_calls.len(),
                    tool_calls.iter().map(|tc| tc.name.as_str()).collect::<Vec<_>>().join(", ")
                ),
            ).await;
            step_num += 1;

            messages.push(Message::new(MessageRole::Assistant, Content::text(&output.text)));

            let registry_calls: Vec<crate::toolkit::registry::ToolCall> = tool_calls
                .iter()
                .map(|tc| crate::toolkit::registry::ToolCall {
                    name: tc.name.clone(),
                    args: tc.arguments.clone(),
                    id: Some(tc.id.clone()),
                })
                .collect();
            let results = registry.execute_parallel(registry_calls).await;

            let mut round_tool_calls: Vec<ToolCallRecord> = Vec::new();
            for (i, tc) in tool_calls.iter().enumerate() {
                let result = results.get(i).cloned().unwrap_or_else(|| crate::toolkit::ToolResult {
                    name: tc.name.clone(),
                    result: Err(crate::toolkit::error::ToolError::Execution("No result".to_string())),
                });
                round_tool_calls.push(ToolCallRecord {
                    name: tc.name.clone(),
                    input: tc.arguments.clone(),
                    result,
                });
            }

            round_data_list.push(RoundData {
                thought: if remaining_text.is_empty() { None } else { Some(remaining_text) },
                tool_calls: round_tool_calls,
            });

            for result in &results {
                all_tool_results.push(result.clone());
                let result_text = match &result.result {
                    Ok(output) => {
                        let raw = serde_json::to_string_pretty(&output.data)
                            .unwrap_or_else(|_| "Success".to_string());
                        // Sanitize base64/image data to prevent context bloat
                        let sanitized = crate::agent::streaming::sanitize_tool_result_for_prompt(&raw);
                        // UTF-8 safe truncation
                        const MAX_TOOL_RESULT_IN_MSG: usize = 4000;
                        if sanitized.chars().count() > MAX_TOOL_RESULT_IN_MSG {
                            crate::agent::streaming::truncate_result_utf8(&sanitized, MAX_TOOL_RESULT_IN_MSG)
                        } else {
                            sanitized
                        }
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
                    messages.push(Message::new(
                        MessageRole::User,
                        Content::text(&format!("Tool '{}' result:\n{}", result.name, result_text)),
                    ));
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
                    &agent.id, execution_id, step_num,
                    &format!("tool '{}' → {}", result.name, result_preview),
                ).await;
                step_num += 1;
            }
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

            // Build Phase 2 prompt — natural language, NOT JSON template.
            // Pattern mirrors Focused's build_phase2_prompt_with_tool_results.
            let task = &agent.user_prompt;
            let mut phase2_user = format!(
                "{}\n\n[Completed {} rounds of tool execution, {} tool results collected]\n\
                 IMPORTANT: You MUST analyze ALL tool results below and provide a COMPLETE response. \
                 Do NOT just say \"execution completed\" — present the data naturally.\n\n",
                task,
                round_data_list.len().max(1),
                all_tool_results.len(),
            );

            const TOOL_RESULT_MAX_LEN: usize = 8000;
            for r in &all_tool_results {
                let result_text = match &r.result {
                    Ok(output) => {
                        let raw = serde_json::to_string_pretty(&output.data)
                            .unwrap_or_else(|_| "Success".to_string());
                        // Sanitize base64/image data to prevent context bloat
                        let sanitized = crate::agent::streaming::sanitize_tool_result_for_prompt(&raw);
                        if sanitized.chars().count() > TOOL_RESULT_MAX_LEN {
                            crate::agent::streaming::truncate_result_utf8(&sanitized, TOOL_RESULT_MAX_LEN)
                        } else {
                            sanitized
                        }
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

        // is_generic_fallback: true when the post-loop summary was needed (and may or
        // may not have succeeded). This tells build_tool_result to prefer tool-derived
        // fallbacks over the raw final_text when the latter is unhelpful.
        let is_generic_fallback = needs_summary;

        if final_text.is_empty() {
            final_text = "Completed tool execution rounds.".to_string();
        }

        ToolLoopOutput {
            final_text,
            is_generic_fallback,
            all_tool_results,
            all_reasoning_texts,
            round_data_list_raw: round_data_list
                .into_iter()
                .map(|rd| (rd.thought, rd.tool_calls))
                .collect(),
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
            is_generic_fallback,
            all_tool_results,
            all_reasoning_texts,
            round_data_list_raw,
        } = loop_output;

        let (mut situation_analysis, mut conclusion, confidence) =
            parse_final_tool_response(&final_text);

        // --- situation_analysis ---
        // When the LLM didn't produce structured JSON:
        // - If Phase 2 generated natural language, use it as situation_analysis
        // - Otherwise fall back to reasoning texts or generic summary
        if is_generic_fallback || situation_analysis.is_empty() || situation_analysis == "Completed tool execution rounds." {
            // Phase 2 natural language response — use as situation_analysis
            if is_generic_fallback && !final_text.is_empty()
                && final_text != "Completed tool execution rounds."
                && final_text != "LLM generation failed during tool execution."
            {
                situation_analysis = if final_text.len() > 500 {
                    let cut = final_text.floor_char_boundary(500);
                    let end = final_text[..cut]
                        .rfind(|c: char| c == '.' || c == '!' || c == '?' || c == '。')
                        .map(|i| i + 1)
                        .unwrap_or(cut);
                    format!("{}...", &final_text[..end])
                } else {
                    final_text.clone()
                };
            } else if !all_reasoning_texts.is_empty() {
                let combined = all_reasoning_texts.join(" ");
                situation_analysis = if combined.len() > 500 {
                    let cut = combined.floor_char_boundary(500);
                    let end = combined[..cut]
                        .rfind(|c: char| c == '.' || c == '!' || c == '?' || c == '。')
                        .map(|i| i + 1)
                        .unwrap_or(cut);
                    format!("{}...", &combined[..end])
                } else {
                    combined
                };
            } else {
                situation_analysis = format!(
                    "Agent executed {} tool operations across completed rounds.",
                    all_tool_results.len()
                );
            }
        }

        // --- conclusion ---
        // Priority: JSON-parsed > Phase 2 natural language > tool summary
        // Treat generic/error strings as empty so they get replaced.
        let is_generic_conclusion = conclusion == "Completed tool execution rounds."
            || conclusion == "LLM generation failed during tool execution.";
        if conclusion.is_empty() || is_generic_conclusion {
            if !final_text.is_empty()
                && final_text != "Completed tool execution rounds."
                && final_text != "LLM generation failed during tool execution."
            {
                // Use the LLM's Phase 2 natural language response
                conclusion = final_text.clone();
            } else if !all_tool_results.is_empty() {
                let tool_summary: Vec<String> = all_tool_results
                    .iter()
                    .filter_map(|r| match &r.result {
                        Ok(output) => Some(summarize_tool_output(&output.data, &r.name)),
                        Err(e) => Some(format!("{} failed: {}", r.name, e)),
                    })
                    .collect();
                conclusion = tool_summary.join("; ") + ".";
            } else {
                conclusion = "No tools were executed during this agent run.".to_string();
            }
        }

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
                Decision {
                    decision_type: "tool_execution".to_string(),
                    description: desc,
                    action,
                    rationale: String::new(),
                    expected_outcome: String::new(),
                }
            })
            .collect();

        // Derive confidence: LLM may provide it via JSON (explicit), natural language (0.7),
        // or not at all (0.5). Use tool success rate as a floor.
        let tool_success_rate = if all_tool_results.is_empty() {
            None
        } else {
            let ok = all_tool_results.iter().filter(|r| r.result.is_ok()).count() as f32;
            Some((ok / all_tool_results.len() as f32).max(0.5))
        };
        let final_confidence = if confidence > 0.7 {
            // LLM provided explicit confidence via JSON
            confidence
        } else if let Some(rate) = tool_success_rate {
            // Fall back to tool success rate when confidence is default or steps empty
            rate
        } else {
            0.5
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

        // summary: the actual LLM response text (Phase 2 or natural).
        // Skip generic/error strings — the frontend already shows conclusion separately.
        let summary_text = if is_generic_fallback || final_text == "LLM generation failed during tool execution." {
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
            .unwrap()
            .clone()
            .ok_or_else(|| NeoMindError::Tool("Tool registry not available".to_string()))?;

        let filtered_tools = Self::filter_tools(&registry, &agent.tool_config);
        let system_prompt = Self::build_tool_system_prompt(agent, data_collected, invocation_input);
        let mut messages = Self::build_tool_messages(&system_prompt, data_collected);

        let loop_output = self
            .run_tool_loop(agent, &registry, &llm_runtime, &filtered_tools, &mut messages, execution_id)
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
        *self.tool_registry.write().unwrap() = Some(registry);
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

                    let runtime: Result<Arc<dyn LlmRuntime + Send + Sync>, _> =
                        match backend.backend_type {
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
                                let endpoint = backend.endpoint.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                                let timeout = Self::env_timeout_secs("OPENAI_TIMEOUT_SECS", 60);
                                Self::create_cloud_runtime(
                                    CloudConfig::custom(&api_key, &endpoint).with_model(&backend.model).with_timeout_secs(timeout),
                                    &backend.capabilities,
                                )
                            }
                            LlmBackendType::Anthropic => {
                                let api_key = backend.api_key.clone().unwrap_or_default();
                                let timeout = Self::env_timeout_secs("ANTHROPIC_TIMEOUT_SECS", 60);
                                Self::create_cloud_runtime(
                                    CloudConfig::anthropic(&api_key).with_model(&backend.model).with_timeout_secs(timeout),
                                    &backend.capabilities,
                                )
                            }
                            LlmBackendType::Google => {
                                let api_key = backend.api_key.clone().unwrap_or_default();
                                let timeout = Self::env_timeout_secs("GOOGLE_TIMEOUT_SECS", 60);
                                Self::create_cloud_runtime(
                                    CloudConfig::google(&api_key).with_model(&backend.model).with_timeout_secs(timeout),
                                    &backend.capabilities,
                                )
                            }
                            LlmBackendType::XAi => {
                                let api_key = backend.api_key.clone().unwrap_or_default();
                                let timeout = Self::env_timeout_secs("XAI_TIMEOUT_SECS", 60);
                                Self::create_cloud_runtime(
                                    CloudConfig::grok(&api_key).with_model(&backend.model).with_timeout_secs(timeout),
                                    &backend.capabilities,
                                )
                            }
                            LlmBackendType::Qwen => {
                                let api_key = backend.api_key.clone().unwrap_or_default();
                                let endpoint = backend.endpoint.clone().unwrap_or_else(|| "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string());
                                let timeout = Self::env_timeout_secs("QWEN_TIMEOUT_SECS", 60);
                                Self::create_cloud_runtime(
                                    CloudConfig::custom(&api_key, &endpoint).with_model(&backend.model).with_timeout_secs(timeout),
                                    &backend.capabilities,
                                )
                            }
                            LlmBackendType::DeepSeek => {
                                let api_key = backend.api_key.clone().unwrap_or_default();
                                let endpoint = backend.endpoint.clone().unwrap_or_else(|| "https://api.deepseek.com".to_string());
                                let timeout = Self::env_timeout_secs("DEEPSEEK_TIMEOUT_SECS", 60);
                                Self::create_cloud_runtime(
                                    CloudConfig::custom(&api_key, &endpoint).with_model(&backend.model).with_timeout_secs(timeout),
                                    &backend.capabilities,
                                )
                            }
                            LlmBackendType::GLM => {
                                let api_key = backend.api_key.clone().unwrap_or_default();
                                let endpoint = backend.endpoint.clone().unwrap_or_else(|| "https://open.bigmodel.cn/api/paas/v4".to_string());
                                let timeout = Self::env_timeout_secs("GLM_TIMEOUT_SECS", 60);
                                Self::create_cloud_runtime(
                                    CloudConfig::custom(&api_key, &endpoint).with_model(&backend.model).with_timeout_secs(timeout),
                                    &backend.capabilities,
                                )
                            }
                            LlmBackendType::MiniMax => {
                                let api_key = backend.api_key.clone().unwrap_or_default();
                                let endpoint = backend.endpoint.clone().unwrap_or_else(|| "https://api.minimax.chat/v1".to_string());
                                let timeout = Self::env_timeout_secs("MINIMAX_TIMEOUT_SECS", 60);
                                Self::create_cloud_runtime(
                                    CloudConfig::custom(&api_key, &endpoint).with_model(&backend.model).with_timeout_secs(timeout),
                                    &backend.capabilities,
                                )
                            }
                            #[cfg(feature = "llamacpp")]
                            LlmBackendType::LlamaCpp => {
                                let endpoint = backend.endpoint.clone().unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
                                let timeout = Self::env_timeout_secs("LLAMACPP_TIMEOUT_SECS", 180);
                                let config = crate::llm_backends::backends::llamacpp::LlamaCppConfig::new(&backend.model)
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
                                        std::sync::Arc::new(rt) as std::sync::Arc<dyn neomind_core::llm::backend::LlmRuntime + Send + Sync>
                                    })
                            }
                            #[cfg(not(feature = "llamacpp"))]
                            LlmBackendType::LlamaCpp => {
                                Err(neomind_core::llm::backend::LlmError::BackendUnavailable("llama.cpp backend is not available (feature not enabled)".to_string()))
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
                    // Cooldown: one execution per agent per 60s window
                    const COOLDOWN_SECS: i64 = 60;
                    let dedup_key = agent.id.clone();
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
                    let executor_tool_registry = self.tool_registry.read().unwrap().clone();
                    let executor_extension_registry = self.extension_registry.clone();
                    let executor_memory_store = self.memory_store.clone();
                    let backend_id = agent.llm_backend_id
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
                            let _backend_permit = backend_sem.acquire().await.unwrap();
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

            // Cooldown: one execution per agent per 60s window
            const COOLDOWN_SECS: i64 = 60;
            let dedup_key = agent.id.clone();
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
            let executor_tool_registry = self.tool_registry.read().unwrap().clone();
            let executor_extension_registry = self.extension_registry.clone();
            let executor_memory_store = self.memory_store.clone();
            let backend_id = agent.llm_backend_id
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
                    let _backend_permit = backend_sem.acquire().await.unwrap();
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
                        if let Some(filter_device) = filter.get("device_id").and_then(|v| v.as_str()) {
                            if filter_device == "all" || filter_device == source_id {
                                if source_type == "device" {
                                    return true;
                                }
                            }
                        }
                    } else if event_type == "extension.output" {
                        if let Some(filter_ext) = filter.get("extension_id").and_then(|v| v.as_str()) {
                            if filter_ext == "all" || filter_ext == source_id {
                                if source_type == "extension" {
                                    return true;
                                }
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
                ResourceType::Device => {
                    source_type == "device" && r.resource_id == source_id
                }
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
                                res_device == source_id && (field == res_field || field.ends_with(&format!(".{}", res_field)))
                            } else {
                                false
                            }
                        } else {
                            r.resource_id == field || field.ends_with(&format!(".{}", r.resource_id))
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

        // Execute with error handling for stability
        // Use execute_with_chaining to support multi-round tool chaining
        let execution_result = self.execute_with_chaining(context, event_data.clone()).await;

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

    /// Check if an action result is chainable (contains data useful for next round)
    fn is_chainable_action(action: &neomind_storage::ActionExecuted) -> bool {
        // Extension commands that return data are chainable
        if action.action_type == "extension_command" {
            return true;
        }

        // Actions with meaningful results (not just success messages)
        if let Some(ref result) = action.result {
            // Filter out generic success messages
            if !result.is_empty()
                && result != "Command sent successfully"
                && result != "Success"
                && !result.starts_with("Failed:")
            {
                return true;
            }
        }

        false
    }

    /// Generate a conclusion summary using LLM when the original conclusion is empty or meaningless.
    async fn generate_conclusion_summary(
        &self,
        agent: &AiAgent,
        actions: &[neomind_storage::ActionExecuted],
        chain_depth: usize,
        original_prompt: &str,
    ) -> AgentResult<String> {
        // Get LLM runtime
        let llm_runtime = match self.get_llm_runtime_for_agent(agent).await? {
            Some(runtime) => runtime,
            None => {
                // Fallback to simple summary
                let success_count = actions.iter().filter(|a| a.success).count();
                return Ok(format!(
                    "Execution complete: {} rounds, {} / {} actions succeeded",
                    chain_depth,
                    success_count,
                    actions.len()
                ));
            }
        };

        // Build action summary
        let action_details: Vec<String> = actions.iter()
            .take(5)
            .map(|a| {
                format!(
                    "- {} -> {}: {} ({})",
                    a.action_type,
                    a.target,
                    if a.success { "success" } else { "failed" },
                    a.result.as_deref().unwrap_or("no result").chars().take(100).collect::<String>()
                )
            })
            .collect();

        let success_rate = if actions.is_empty() {
            1.0
        } else {
            actions.iter().filter(|a| a.success).count() as f32 / actions.len() as f32
        };

        let prompt = format!(
            r#"基于以下工具执行结果，生成一个简洁的总结（1-2句话）：

用户原始请求：{}
执行轮数：{}
成功率：{:.0}%

执行的操作：
{}

请直接输出总结，不要包含任何其他内容。"#,
            original_prompt,
            chain_depth,
            success_rate * 100.0,
            action_details.join("\n")
        );

        use neomind_core::llm::backend::{GenerationParams, LlmInput};
        use neomind_core::message::{Message, MessageRole, Content};

        let input = LlmInput {
            messages: vec![
                Message::new(MessageRole::System, Content::text("你是一个简洁的总结助手。用1-2句话总结执行结果。")),
                Message::new(MessageRole::User, Content::text(&prompt)),
            ],
            params: GenerationParams {
                max_tokens: Some(200),
                temperature: Some(0.3),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        match llm_runtime.generate(input).await {
            Ok(output) => {
                let conclusion = output.text.trim().to_string();
                if conclusion.is_empty() {
                    Ok(format!("Execution complete: {} rounds, success rate {:.0}%", chain_depth, success_rate * 100.0))
                } else {
                    Ok(conclusion)
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to generate conclusion summary");
                let success_count = actions.iter().filter(|a| a.success).count();
                Ok(format!(
                    "Execution complete: {} rounds, {} / {} actions succeeded",
                    chain_depth,
                    success_count,
                    actions.len()
                ))
            }
        }
    }

    /// Execute with tool chaining support
    async fn execute_with_chaining(
        &self,
        mut context: ExecutionContext,
        event_data: Option<EventTriggerData>,
    ) -> AgentResult<(DecisionProcess, StorageExecutionResult)> {
        let agent = context.agent.clone();
        let max_depth = if agent.enable_tool_chaining {
            agent.max_chain_depth
        } else {
            1 // No chaining, just single execution
        };

        let mut chain_state = ChainState::new(max_depth);
        #[allow(unused_assignments)]
        let mut final_decision_process: Option<DecisionProcess> = None;
        let mut all_actions_executed: Vec<neomind_storage::ActionExecuted> = Vec::new();
        let mut all_notifications_sent: Vec<neomind_storage::NotificationSent> = Vec::new();

        if let Some(ref ed) = event_data {
            tracing::debug!(
                agent_id = %agent.id,
                enable_chaining = agent.enable_tool_chaining,
                max_depth = max_depth,
                event_source = %ed.source.source_id,
                event_field = %ed.source.field,
                "Starting event-triggered agent execution"
            );
        } else {
            tracing::debug!(
                agent_id = %agent.id,
                enable_chaining = agent.enable_tool_chaining,
                max_depth = max_depth,
                "Starting agent execution"
            );
        }

        // Execute rounds until we reach max depth or no more chainable results
        loop {
            tracing::debug!(
                agent_id = %agent.id,
                current_depth = chain_state.depth,
                max_depth = chain_state.max_depth,
                "Execution round"
            );

            // Update context with chain results if we have any
            if chain_state.depth > 0 {
                context.agent.user_prompt = format!(
                    "{}{}",
                    context.agent.user_prompt,
                    chain_state.format_as_context()
                );
            }

            // Execute one round with retry
            let (decision_process, execution_result) =
                self.execute_with_retry(context.clone(), event_data.clone()).await?;

            // Collect results from this round
            all_actions_executed.extend(execution_result.actions_executed.clone());
            all_notifications_sent.extend(execution_result.notifications_sent.clone());

            // Store the final decision process (last round takes precedence)
            final_decision_process = Some(decision_process.clone());

            // Check if we should continue chaining
            if !agent.enable_tool_chaining || !chain_state.can_continue() {
                tracing::debug!(
                    agent_id = %agent.id,
                    depth = chain_state.depth,
                    "Chaining disabled or max depth reached, stopping"
                );
                break;
            }

            // Check if we have chainable results
            let has_chainable = execution_result
                .actions_executed
                .iter()
                .any(Self::is_chainable_action);

            if !has_chainable {
                tracing::debug!(
                    agent_id = %agent.id,
                    "No chainable results, stopping"
                );
                break;
            }

            // Check if decisions indicate more work needed
            let needs_more_work = decision_process.decisions.iter().any(|d| {
                d.decision_type == "needs_more_data"
                    || d.action.to_lowercase().contains("continue")
                    || d.action.to_lowercase().contains("further")
                    || d.action.to_lowercase().contains("next step")
                    || d.action.to_lowercase().contains("continue")
            });

            if !needs_more_work {
                tracing::debug!(
                    agent_id = %agent.id,
                    "Decisions indicate no more work needed, stopping"
                );
                break;
            }

            // Advance to next round
            chain_state.advance(&execution_result.actions_executed);

            // Send progress event for chaining
            self.send_progress(
                &context.agent.id,
                &context.execution_id,
                "chaining",
                &format!("Tool chaining round {}", chain_state.depth + 1),
                Some(&format!(
                    "Continuing analysis with results from {} previous action(s)...",
                    chain_state.previous_results.len()
                )),
            )
            .await;
        }

        // Merge decision processes from all rounds
        let merged_decision_process = if let Some(mut final_dp) = final_decision_process {
            // Add chain info to situation analysis
            if chain_state.depth > 1 {
                final_dp.situation_analysis = format!(
                    "{}\n\n[Tool chaining: {} rounds executed]",
                    final_dp.situation_analysis, chain_state.depth
                );
            }

            // If conclusion is empty or meaningless, generate via LLM
            // Only do this for multi-round chaining where we have accumulated actions to summarize
            if final_dp.conclusion.is_empty()
                || final_dp.conclusion == "No conclusion"
                || final_dp.conclusion == "Completed tool execution rounds."
            {
                if !all_actions_executed.is_empty() {
                    final_dp.conclusion = self
                        .generate_conclusion_summary(
                            &agent,
                            &all_actions_executed,
                            chain_state.depth,
                            &agent.user_prompt,
                        )
                        .await?;
                } else if final_dp.conclusion.is_empty() || final_dp.conclusion == "No conclusion" {
                    // No actions and no conclusion — keep reasoning text as fallback
                    final_dp.conclusion = "No additional actions required.".to_string();
                }
            }

            final_dp
        } else {
            // Fallback (shouldn't happen) - build from actions
            let conclusion = if !all_actions_executed.is_empty() {
                let success_count = all_actions_executed.iter().filter(|a| a.success).count();
                let total_count = all_actions_executed.len();
                format!(
                    "Execution complete: {} rounds, {} / {} actions succeeded",
                    chain_state.depth,
                    success_count,
                    total_count
                )
            } else {
                format!("Execution complete: {} tool call rounds", chain_state.depth)
            };

            DecisionProcess {
                situation_analysis: format!("Agent executed {} rounds via tool chaining", chain_state.depth),
                data_collected: vec![],
                reasoning_steps: vec![],
                decisions: vec![],
                conclusion,
                confidence: 0.5,
            }
        };

        // Extract conclusion for summary before moving
        let summary_conclusion = merged_decision_process.conclusion.clone();

        let success_rate = if all_actions_executed.is_empty() {
            1.0
        } else {
            all_actions_executed.iter().filter(|a| a.success).count() as f32
                / all_actions_executed.len() as f32
        };

        let total_actions = all_actions_executed.len();

        let merged_execution_result = StorageExecutionResult {
            actions_executed: all_actions_executed,
            report: None, // Reports are generated per-round, not in chaining
            notifications_sent: all_notifications_sent,
            summary: if chain_state.depth > 1 {
                format!(
                    "Completed {} execution rounds via tool chaining",
                    chain_state.depth
                )
            } else {
                summary_conclusion
            },
            success_rate,
        };

        tracing::debug!(
            agent_id = %agent.id,
            total_rounds = chain_state.depth,
            total_actions = total_actions,
            "Tool chaining execution completed"
        );

        Ok((merged_decision_process, merged_execution_result))
    }

    /// Execute with retry for stability.
    async fn execute_with_retry(
        &self,
        context: ExecutionContext,
        event_data: Option<EventTriggerData>,
    ) -> AgentResult<(DecisionProcess, StorageExecutionResult)> {
        let max_retries = 3u32;
        let mut last_error = None;

        for attempt in 0..=max_retries {
            let result = self
                .execute_internal(context.clone(), event_data.clone())
                .await;
            match result {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        agent_id = %context.agent.id,
                        attempt = attempt + 1,
                        max_retries = max_retries + 1,
                        error = %e,
                        "Agent execution failed, retrying"
                    );
                    last_error = Some(e);

                    if attempt < max_retries {
                        let delay_ms = 100 * (2_u64.pow(attempt));
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| NeoMindError::Llm("Max retries exceeded".to_string())))
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
            // ── Free path ──────────────────────────────────────────────
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
                        "Free mode analysis completed: {} tool call(s), confidence {:.0}%",
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
                    .map_err(|e| NeoMindError::Storage(format!("Failed to update memory: {}", e)))?;

                // Extract learned patterns into system memory
                if !decision_process.decisions.is_empty()
                    || !decision_process.situation_analysis.is_empty()
                {
                    self.extract_to_system_memory(
                        &agent,
                        &decision_process.situation_analysis,
                        &decision_process.conclusion,
                        &decision_process.decisions,
                    )
                    .await;
                }

                tracing::debug!(
                    agent_id = %agent_id,
                    "[FREE] Returning direct results — skipped Focused post-processing"
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
                    .map_err(|e| NeoMindError::Storage(format!("Failed to update memory: {}", e)))?;

                // Bridge: extract learned patterns into system memory
                if !decisions.is_empty() || !situation_analysis.is_empty() {
                    self.extract_to_system_memory(
                        &agent,
                        &situation_analysis,
                        &conclusion,
                        &decisions,
                    ).await;
                }

                // Calculate confidence from reasoning
                let confidence = if reasoning_steps.is_empty() {
                    0.5
                } else {
                    reasoning_steps.iter().map(|s| s.confidence).sum::<f32>() / reasoning_steps.len() as f32
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
                    let success_count = actions_executed.iter().filter(|a| a.success).count() as f32;
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
