//! AI Agent executor - runs agents and records decision processes.

use edge_ai_core::{EventBus, MetricValue, NeoTalkEvent, message::{Content, ContentPart, Message, MessageRole}};
use edge_ai_storage::{
    AgentMemory, AgentStats, AgentStore, AgentExecutionRecord, AiAgent, DataCollected,
    Decision, DecisionProcess, ExecutionResult as StorageExecutionResult, ExecutionStatus,
    GeneratedReport, ReasoningStep, TrendPoint, AgentResource, ResourceType,
    // New conversation types
    ConversationTurn, TurnInput, TurnOutput, AgentRole,
    LlmBackendStore, LlmBackendInstance,
};
use edge_ai_devices::DeviceService;
use edge_ai_alerts::AlertManager;
use edge_ai_llm::{OllamaConfig, OllamaRuntime, CloudConfig, CloudRuntime};
use edge_ai_core::llm::backend::LlmRuntime;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};

use crate::{Agent, AgentConfig, LlmBackend};
use crate::error::AgentError;
use crate::prompts::get_role_system_prompt;
use crate::translation::Language;

/// Internal representation of image content for multimodal LLM messages.
enum ImageContent {
    Url(String),
    Base64(String, String), // (data, mime_type)
}

/// Event data for triggering agent execution.
#[derive(Clone, Debug)]
pub struct EventTriggerData {
    pub device_id: String,
    pub metric: String,
    pub value: MetricValue,
    pub timestamp: i64,
}

/// Configuration for agent executor.
#[derive(Clone)]
pub struct AgentExecutorConfig {
    /// Agent store
    pub store: Arc<AgentStore>,
    /// Time series storage for data collection
    pub time_series_storage: Option<Arc<edge_ai_storage::TimeSeriesStore>>,
    /// Device service for command execution
    pub device_service: Option<Arc<DeviceService>>,
    /// Event bus for event subscription
    pub event_bus: Option<Arc<EventBus>>,
    /// Alert manager for sending notifications
    pub alert_manager: Option<Arc<AlertManager>>,
    /// LLM runtime for intent analysis (default)
    pub llm_runtime: Option<Arc<dyn edge_ai_core::llm::backend::LlmRuntime + Send + Sync>>,
    /// LLM backend store for per-agent backend lookup
    pub llm_backend_store: Option<Arc<LlmBackendStore>>,
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
    time_series_storage: Option<Arc<edge_ai_storage::TimeSeriesStore>>,
    /// Device service for command execution
    device_service: Option<Arc<DeviceService>>,
    /// Event bus for publishing events
    event_bus: Option<Arc<EventBus>>,
    /// Alert manager for sending notifications
    alert_manager: Option<Arc<AlertManager>>,
    /// Configuration
    config: AgentExecutorConfig,
    /// LLM runtime (default)
    llm_runtime: Option<Arc<dyn edge_ai_core::llm::backend::LlmRuntime + Send + Sync>>,
    /// LLM backend store for per-agent backend lookup
    llm_backend_store: Option<Arc<LlmBackendStore>>,
    /// Event-triggered agents cache
    event_agents: Arc<RwLock<HashMap<String, AiAgent>>>,
    /// Track recent executions to prevent duplicates (agent_id, device_id, metric -> timestamp)
    recent_executions: Arc<RwLock<HashMap<(String, String, String), i64>>>,
}

impl AgentExecutor {
    /// Create a new agent executor.
    pub async fn new(config: AgentExecutorConfig) -> Result<Self, AgentError> {
        let llm_runtime = config.llm_runtime.clone();
        let llm_backend_store = config.llm_backend_store.clone();
        let alert_manager = config.alert_manager.clone();
        Ok(Self {
            store: config.store.clone(),
            time_series_storage: config.time_series_storage.clone(),
            device_service: config.device_service.clone(),
            event_bus: config.event_bus.clone(),
            alert_manager,
            config,
            llm_runtime,
            llm_backend_store,
            event_agents: Arc::new(RwLock::new(HashMap::new())),
            recent_executions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Set the LLM runtime for intent parsing.
    pub async fn set_llm_runtime(&mut self, llm: Arc<dyn edge_ai_core::llm::backend::LlmRuntime + Send + Sync>) {
        self.llm_runtime = Some(llm);
    }

    /// Get the agent store.
    pub fn store(&self) -> Arc<AgentStore> {
        self.store.clone()
    }

    /// Get the LLM runtime for a specific agent.
    /// If the agent has a specific backend ID configured, use that.
    /// Otherwise, fall back to the default runtime.
    pub async fn get_llm_runtime_for_agent(
        &self,
        agent: &AiAgent,
    ) -> Result<Option<Arc<dyn LlmRuntime + Send + Sync>>, AgentError> {
        // If agent has a specific backend ID, try to use it
        if let Some(ref backend_id) = agent.llm_backend_id {
            if let Some(ref store) = self.llm_backend_store {
                if let Ok(Some(backend)) = store.load_instance(backend_id) {
                    use edge_ai_storage::LlmBackendType;
                    match backend.backend_type {
                        LlmBackendType::Ollama => {
                            let endpoint = backend.endpoint.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
                            let model = backend.model.clone();
                            let timeout = std::env::var("OLLAMA_TIMEOUT_SECS")
                                .ok()
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(120);
                            match OllamaRuntime::new(
                                OllamaConfig::new(&model)
                                    .with_endpoint(&endpoint)
                                    .with_timeout_secs(timeout)
                            ) {
                                Ok(runtime) => return Ok(Some(Arc::new(runtime) as Arc<dyn LlmRuntime + Send + Sync>)),
                                Err(e) => {
                                    tracing::warn!(category = "ai", error = %e,
                                        "Failed to create Ollama runtime for agent '{}'", agent.name);
                                }
                            }
                        }
                        LlmBackendType::OpenAi => {
                            let api_key = backend.api_key.clone().unwrap_or_default();
                            let endpoint = backend.endpoint.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                            let model = backend.model.clone();
                            let timeout = std::env::var("OPENAI_TIMEOUT_SECS")
                                .ok()
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(60);
                            match CloudRuntime::new(
                                CloudConfig::custom(&api_key, &endpoint)
                                    .with_model(&model)
                                    .with_timeout_secs(timeout)
                            ) {
                                Ok(runtime) => return Ok(Some(Arc::new(runtime) as Arc<dyn LlmRuntime + Send + Sync>)),
                                Err(e) => {
                                    tracing::warn!(category = "ai", error = %e,
                                        "Failed to create OpenAI runtime for agent '{}'", agent.name);
                                }
                            }
                        }
                        _ => {
                            tracing::warn!(category = "ai", backend_type = ?backend.backend_type,
                                "Unsupported backend type for agent '{}'", agent.name);
                        }
                    }
                }
            }
        }

        // Fall back to default runtime
        Ok(self.llm_runtime.clone())
    }

    /// Parse user intent from natural language using LLM or keyword-based fallback.
    pub async fn parse_intent(&self, user_prompt: &str) -> Result<edge_ai_storage::ParsedIntent, AgentError> {
        // Try LLM-based parsing if available
        if let Some(ref llm) = self.llm_runtime {
            if let Ok(intent) = self.parse_intent_with_llm(llm, user_prompt).await {
                return Ok(intent);
            }
        }

        // Fall back to keyword-based parsing
        self.parse_intent_keywords(user_prompt).await
    }

    /// Parse intent using LLM.
    async fn parse_intent_with_llm(
        &self,
        llm: &Arc<dyn edge_ai_core::llm::backend::LlmRuntime + Send + Sync>,
        user_prompt: &str,
    ) -> Result<edge_ai_storage::ParsedIntent, AgentError> {
        use edge_ai_core::llm::backend::{LlmInput, GenerationParams};

        let system_prompt = r#"You are an intent parser for IoT automation. Analyze the user's request and extract:
1. Intent type: Monitoring, ReportGeneration, AnomalyDetection, Control, or Automation
2. Target metrics: temperature, humidity, power, etc.
3. Conditions: any thresholds or comparison operators
4. Actions: what actions to take when conditions are met

Respond in JSON format:
{
  "intent_type": "Monitoring|ReportGeneration|AnomalyDetection|Control|Automation",
  "target_metrics": ["metric1", "metric2"],
  "conditions": ["condition1", "condition2"],
  "actions": ["action1", "action2"],
  "confidence": 0.9
}"#;

        let messages = vec![
            Message::new(MessageRole::System, Content::text(system_prompt)),
            Message::new(MessageRole::User, Content::text(user_prompt)),
        ];

        let input = LlmInput {
            messages,
            params: GenerationParams {
                temperature: Some(0.3),
                max_tokens: Some(500),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: Some(Vec::new()),
        };

        match llm.generate(input).await {
            Ok(output) => {
                // Try to parse JSON from LLM output
                let json_str = output.text.trim();
                // Extract JSON if it's wrapped in markdown code blocks
                let json_str = if json_str.contains("```json") {
                    json_str.split("```json").nth(1)
                        .and_then(|s| s.split("```").next())
                        .unwrap_or(json_str)
                        .trim()
                } else if json_str.contains("```") {
                    json_str.split("```").nth(1)
                        .unwrap_or(json_str)
                        .trim()
                } else {
                    json_str
                };

                serde_json::from_str(json_str)
                    .map_err(|_| AgentError::Llm("Failed to parse LLM intent response".to_string()))
            }
            Err(_) => Err(AgentError::Llm("LLM call failed".to_string())),
        }
    }

    /// Parse intent using keyword-based fallback.
    async fn parse_intent_keywords(&self, user_prompt: &str) -> Result<edge_ai_storage::ParsedIntent, AgentError> {
        let prompt_lower = user_prompt.to_lowercase();

        let (intent_type, confidence) = if prompt_lower.contains("报告") || prompt_lower.contains("汇总") || prompt_lower.contains("每天") {
            (edge_ai_storage::IntentType::ReportGeneration, 0.8)
        } else if prompt_lower.contains("异常") || prompt_lower.contains("检测") {
            (edge_ai_storage::IntentType::AnomalyDetection, 0.8)
        } else if prompt_lower.contains("控制") || prompt_lower.contains("开关") {
            (edge_ai_storage::IntentType::Control, 0.7)
        } else {
            (edge_ai_storage::IntentType::Monitoring, 0.7)
        };

        let target_metrics = extract_metrics(&prompt_lower);
        let conditions = extract_conditions(&prompt_lower);
        let actions = extract_actions(&prompt_lower);

        Ok(edge_ai_storage::ParsedIntent {
            intent_type,
            target_metrics,
            conditions,
            actions,
            confidence,
        })
    }

    /// Check if an event should trigger any agent and execute it.
    pub async fn check_and_trigger_event(
        &self,
        device_id: String,
        metric: &str,
        value: &MetricValue,
    ) -> Result<(), AgentError> {
        // Refresh event-triggered agents cache
        self.refresh_event_agents().await;

        let event_agents = self.event_agents.read().await;

        // Clone device_id for use in spawned tasks
        let device_id_for_spawn = device_id.clone();

        // Clean up old entries from recent_executions (older than 60 seconds)
        let now = chrono::Utc::now().timestamp();
        let mut recent = self.recent_executions.write().await;
        recent.retain(|_, &mut timestamp| now - timestamp < 60);
        drop(recent);

        for (_agent_id, agent) in event_agents.iter() {
            // Check if this agent has event-based schedule
            if matches!(agent.schedule.schedule_type, edge_ai_storage::ScheduleType::Event) {
                // Check if agent's event filter matches this event
                if self.matches_event_filter(agent, &device_id, metric, value).await {
                    // Check for duplicate execution within the last 5 seconds
                    let key = (agent.id.clone(), device_id.clone(), metric.to_string());
                    let recent = self.recent_executions.read().await;
                    let is_duplicate = recent.get(&key)
                        .map(|&timestamp| now - timestamp < 5)
                        .unwrap_or(false);
                    drop(recent);

                    if is_duplicate {
                        tracing::debug!(
                            agent_name = %agent.name,
                            device_id = %device_id,
                            metric = %metric,
                            "Skipping duplicate event-triggered execution (within 5 seconds)"
                        );
                        continue;
                    }

                    // Mark this execution as recent
                    {
                        let mut recent = self.recent_executions.write().await;
                        recent.insert(key, now);
                    }

                    tracing::info!(
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
                    let executor_alert_manager = self.alert_manager.clone();
                    let executor_llm = self.llm_runtime.clone();
                    let executor_llm_store = self.llm_backend_store.clone();
                    let agent_id_for_log = agent.id.clone();

                    tokio::spawn(async move {
                        // Create event trigger data
                        let event_trigger_data = EventTriggerData {
                            device_id: device_id_for_task,
                            metric: metric_clone,
                            value: value_clone,
                            timestamp,
                        };

                        // Create a new executor for this event-triggered execution
                        let executor_config = AgentExecutorConfig {
                            store: executor_store.clone(),
                            time_series_storage: executor_time_series.clone(),
                            device_service: executor_device.clone(),
                            event_bus: executor_event_bus.clone(),
                            alert_manager: executor_alert_manager,
                            llm_runtime: executor_llm,
                            llm_backend_store: executor_llm_store,
                        };

                        match AgentExecutor::new(executor_config).await {
                            Ok(executor) => {
                                tracing::debug!(
                                    agent_id = %agent_id_for_log,
                                    trigger_device = %event_trigger_data.device_id,
                                    trigger_metric = %event_trigger_data.metric,
                                    "Executing event-triggered agent with event data"
                                );

                                // Execute the agent with event data (includes the triggering metric value directly)
                                match executor.execute_agent_with_event(agent_clone, event_trigger_data).await {
                                    Ok(record) => {
                                        tracing::info!(
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

    /// Check if an event matches the agent's event filter.
    async fn matches_event_filter(
        &self,
        agent: &AiAgent,
        device_id: &str,
        metric: &str,
        _value: &MetricValue,
    ) -> bool {
        // Check if agent has this device in its resources
        let has_device = agent.resources.iter().any(|r| {
            r.resource_type == ResourceType::Device && r.resource_id == device_id
        });

        if !has_device {
            return false;
        }

        // Check if agent has this metric in its resources
        let has_metric = agent.resources.iter().any(|r| {
            r.resource_type == ResourceType::Metric && r.resource_id.contains(metric)
        });

        has_metric || agent.resources.is_empty()
    }

    /// Refresh the cache of event-triggered agents.
    async fn refresh_event_agents(&self) {
        let filter = edge_ai_storage::AgentFilter {
            status: Some(edge_ai_storage::AgentStatus::Active),
            ..Default::default()
        };

        if let Ok(agents) = self.store.query_agents(filter).await {
            let event_agents: HashMap<String, AiAgent> = agents
                .into_iter()
                .filter(|a| matches!(a.schedule.schedule_type, edge_ai_storage::ScheduleType::Event))
                .map(|a| (a.id.clone(), a))
                .collect();

            let mut cache = self.event_agents.write().await;
            *cache = event_agents;
        }
    }

    /// Execute an agent and record the full decision process.
    pub async fn execute_agent(&self, agent: AiAgent) -> Result<AgentExecutionRecord, AgentError> {
        let agent_id = agent.id.clone();
        let agent_name = agent.name.clone();
        let execution_id = uuid::Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();
        let timestamp = chrono::Utc::now().timestamp();

        // Update agent status to executing
        self.store
            .update_agent_status(&agent_id, edge_ai_storage::AgentStatus::Executing, None)
            .await
            .map_err(|e| AgentError::Storage(format!("Failed to update status: {}", e)))?;

        // Create execution context
        let context = ExecutionContext {
            agent: agent.clone(),
            trigger_type: "manual".to_string(),
            event_data: None,
            llm_backend: None,
            execution_id: execution_id.clone(),
        };

        // Emit agent execution started event
        tracing::info!(
            agent_id = %agent_id,
            execution_id = %execution_id,
            has_event_bus = self.event_bus.is_some(),
            "Emitting AgentExecutionStarted event"
        );
        if let Some(ref bus) = self.event_bus {
            let _ = bus.publish(NeoTalkEvent::AgentExecutionStarted {
                agent_id: agent_id.clone(),
                agent_name: agent_name.clone(),
                execution_id: execution_id.clone(),
                trigger_type: "manual".to_string(),
                timestamp,
            }).await;
            tracing::info!("AgentExecutionStarted event published");
        } else {
            tracing::warn!("No event_bus available, agent events will not be published");
        }

        // Execute with error handling for stability
        let execution_result = self.execute_with_retry(context).await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Build execution record
        let (decision_process_for_turn, success) = match &execution_result {
            Ok((dp, _)) => (Some(dp.clone()), true),
            Err(_) => (None, false),
        };

        let record = match execution_result {
            Ok((decision_process, result)) => {
                // Update stats
                let _ = self.store
                    .update_agent_stats(&agent_id, true, duration_ms)
                    .await;

                AgentExecutionRecord {
                    id: execution_id.clone(),
                    agent_id: agent_id.clone(),
                    timestamp,
                    trigger_type: "manual".to_string(),
                    status: ExecutionStatus::Completed,
                    decision_process,
                    result: Some(result),
                    duration_ms,
                    error: None,
                }
            }
            Err(e) => {
                // Update stats with failure
                let _ = self.store
                    .update_agent_stats(&agent_id, false, duration_ms)
                    .await;

                AgentExecutionRecord {
                    id: execution_id.clone(),
                    agent_id: agent_id.clone(),
                    timestamp,
                    trigger_type: "manual".to_string(),
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

        // Save execution record
        self.store
            .save_execution(&record)
            .await
            .map_err(|e| AgentError::Storage(format!("Failed to save execution: {}", e)))?;

        // Save conversation turn for context continuity
        if let Some(decision_process) = decision_process_for_turn {
            let turn = self.create_conversation_turn(
                execution_id.clone(),
                "manual".to_string(),
                decision_process.data_collected.clone(),
                None, // event_data
                &decision_process,
                duration_ms,
                success,
            );

            if let Err(e) = self.store.append_conversation_turn(&agent_id, &turn).await {
                tracing::warn!(
                    agent_id = %agent_id,
                    execution_id = %execution_id,
                    error = %e,
                    "Failed to save conversation turn"
                );
            } else {
                tracing::debug!(
                    agent_id = %agent_id,
                    execution_id = %execution_id,
                    "Conversation turn saved successfully"
                );
            }
        }

        // Reset agent status based on result
        let new_status = if record.status == ExecutionStatus::Completed {
            edge_ai_storage::AgentStatus::Active
        } else {
            edge_ai_storage::AgentStatus::Error
        };

        let _ = self.store
            .update_agent_status(&agent_id, new_status, record.error.clone())
            .await;

        // Emit agent execution completed event
        let completion_timestamp = chrono::Utc::now().timestamp();
        if let Some(ref bus) = self.event_bus {
            let _ = bus.publish(NeoTalkEvent::AgentExecutionCompleted {
                agent_id: agent_id.clone(),
                execution_id: execution_id.clone(),
                success: record.status == ExecutionStatus::Completed,
                duration_ms: record.duration_ms,
                error: record.error.clone(),
                timestamp: completion_timestamp,
            }).await;
        }

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

    /// Execute an agent with event trigger data.
    /// This method passes the triggering event data directly to avoid storage delays.
    pub async fn execute_agent_with_event(
        &self,
        agent: AiAgent,
        event_data: EventTriggerData,
    ) -> Result<AgentExecutionRecord, AgentError> {
        let agent_id = agent.id.clone();
        let agent_name = agent.name.clone();
        let execution_id = uuid::Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();
        let timestamp = chrono::Utc::now().timestamp();

        // Update agent status to executing
        self.store
            .update_agent_status(&agent_id, edge_ai_storage::AgentStatus::Executing, None)
            .await
            .map_err(|e| AgentError::Storage(format!("Failed to update status: {}", e)))?;

        // Clone event data for later use (before moving)
        let event_device_id = event_data.device_id.clone();
        let event_metric_name = event_data.metric.clone();
        let event_value_json = serde_json::to_value(&event_data.value).unwrap_or_default();
        let event_timestamp = event_data.timestamp;

        // Create execution context with event data
        let context = ExecutionContext {
            agent: agent.clone(),
            trigger_type: format!("event:{}", event_metric_name),
            event_data: Some(serde_json::json!({
                "device_id": event_device_id,
                "metric": event_metric_name,
                "value": event_value_json,
                "timestamp": event_timestamp,
            })),
            llm_backend: None,
            execution_id: execution_id.clone(),
        };

        // Emit agent execution started event
        tracing::info!(
            agent_id = %agent_id,
            execution_id = %execution_id,
            trigger_device = %event_device_id,
            trigger_metric = %event_metric_name,
            has_event_bus = self.event_bus.is_some(),
            "Emitting AgentExecutionStarted event (event-triggered)"
        );
        if let Some(ref bus) = self.event_bus {
            let _ = bus.publish(NeoTalkEvent::AgentExecutionStarted {
                agent_id: agent_id.clone(),
                agent_name: agent_name.clone(),
                execution_id: execution_id.clone(),
                trigger_type: format!("event:{}", event_metric_name),
                timestamp,
            }).await;
        }

        // Execute with error handling for stability
        let execution_result = self.execute_with_retry_and_event(context, event_data).await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Build execution record
        let (decision_process_for_turn, success) = match &execution_result {
            Ok((dp, _)) => (Some(dp.clone()), true),
            Err(_) => (None, false),
        };

        let record = match execution_result {
            Ok((decision_process, result)) => {
                // Update stats
                let _ = self.store
                    .update_agent_stats(&agent_id, true, duration_ms)
                    .await;

                AgentExecutionRecord {
                    id: execution_id.clone(),
                    agent_id: agent_id.clone(),
                    timestamp,
                    trigger_type: format!("event:{}", event_metric_name),
                    status: ExecutionStatus::Completed,
                    decision_process,
                    result: Some(result),
                    duration_ms,
                    error: None,
                }
            }
            Err(e) => {
                // Update stats with failure
                let _ = self.store
                    .update_agent_stats(&agent_id, false, duration_ms)
                    .await;

                AgentExecutionRecord {
                    id: execution_id.clone(),
                    agent_id: agent_id.clone(),
                    timestamp,
                    trigger_type: format!("event:{}", event_metric_name),
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

        // Save execution record
        self.store
            .save_execution(&record)
            .await
            .map_err(|e| AgentError::Storage(format!("Failed to save execution: {}", e)))?;

        // Save conversation turn for context continuity
        if let Some(ref decision_process) = decision_process_for_turn {
            let turn = self.create_conversation_turn(
                execution_id.clone(),
                format!("event:{}", event_metric_name),
                decision_process.data_collected.clone(),
                Some(serde_json::json!({
                    "device_id": event_device_id,
                    "metric": event_metric_name,
                    "value": event_value_json,
                })),
                decision_process,
                duration_ms,
                success,
            );

            if let Err(e) = self.store.append_conversation_turn(&agent_id, &turn).await {
                tracing::warn!(
                    agent_id = %agent_id,
                    execution_id = %execution_id,
                    error = %e,
                    "Failed to save conversation turn"
                );
            }
        }

        // Reset agent status based on result
        let new_status = if record.status == ExecutionStatus::Completed {
            edge_ai_storage::AgentStatus::Active
        } else {
            edge_ai_storage::AgentStatus::Error
        };

        let _ = self.store
            .update_agent_status(&agent_id, new_status, record.error.clone())
            .await;

        // Emit agent execution completed event
        let completion_timestamp = chrono::Utc::now().timestamp();
        if let Some(ref bus) = self.event_bus {
            let _ = bus.publish(NeoTalkEvent::AgentExecutionCompleted {
                agent_id: agent_id.clone(),
                execution_id: execution_id.clone(),
                success: record.status == ExecutionStatus::Completed,
                duration_ms: record.duration_ms,
                error: record.error.clone(),
                timestamp: completion_timestamp,
            }).await;
        }

        tracing::info!(
            agent_id = %agent_id,
            agent_name = %agent_name,
            execution_id = %execution_id,
            trigger_device = %event_device_id,
            trigger_metric = %event_metric_name,
            status = ?record.status,
            duration_ms = record.duration_ms,
            "Event-triggered agent execution completed"
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
    ) -> Result<Vec<AgentExecutionRecord>, AgentError> {
        use futures::future::join_all;

        let executor_ref = self;
        let futures: Vec<_> = agents
            .into_iter()
            .map(|agent| executor_ref.execute_agent(agent))
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
            return Err(AgentError::Storage(format!(
                "All {} agents failed. First error: {}",
                errors.len(),
                errors[0]
            )));
        }

        Ok(records)
    }

    /// Execute with retry for stability.
    async fn execute_with_retry(
        &self,
        context: ExecutionContext,
    ) -> Result<(DecisionProcess, StorageExecutionResult), AgentError> {
        let max_retries = 3u32;
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match self.execute_internal(context.clone()).await {
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

        Err(last_error.unwrap_or_else(|| {
            AgentError::Llm("Max retries exceeded".to_string())
        }))
    }

    /// Execute with retry for stability (with event data).
    async fn execute_with_retry_and_event(
        &self,
        context: ExecutionContext,
        event_data: EventTriggerData,
    ) -> Result<(DecisionProcess, StorageExecutionResult), AgentError> {
        let max_retries = 3u32;
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match self.execute_internal_with_event(context.clone(), event_data.clone()).await {
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

        Err(last_error.unwrap_or_else(|| {
            AgentError::Llm("Max retries exceeded".to_string())
        }))
    }

    /// Internal execution logic.
    async fn execute_internal(
        &self,
        context: ExecutionContext,
    ) -> Result<(DecisionProcess, StorageExecutionResult), AgentError> {
        let mut agent = context.agent;

        // Step 1: Collect data
        let data_collected = self.collect_data(&agent).await?;

        // Step 1.5: Parse intent if not already done
        let parsed_intent = if agent.parsed_intent.is_none() {
            match self.parse_intent(&agent.user_prompt).await {
                Ok(intent) => {
                    // Update agent with parsed intent
                    let _ = self.store.update_agent_parsed_intent(&agent.id, Some(intent.clone())).await;
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

        // Step 2: Analyze situation with LLM
        let (situation_analysis, reasoning_steps, decisions, conclusion) =
            self.analyze_situation_with_intent(&agent, &data_collected, parsed_intent.as_ref(), &context.execution_id).await?;

        // Step 3: Execute decisions
        let (actions_executed, notifications_sent) =
            self.execute_decisions(&agent, &decisions).await?;

        // Step 4: Generate report if needed
        let report = self.maybe_generate_report(&agent, &data_collected).await?;

        // Step 5: Update memory with learnings
        let updated_memory = self.update_memory(
            &agent,
            &data_collected,
            &decisions,
            &situation_analysis,
            &conclusion,
        ).await?;

        // Save updated memory
        self.store
            .update_agent_memory(&agent.id, updated_memory.clone())
            .await
            .map_err(|e| AgentError::Storage(format!("Failed to update memory: {}", e)))?;

        // Calculate confidence from reasoning
        let confidence = if reasoning_steps.is_empty() {
            0.5
        } else {
            reasoning_steps.iter().map(|s| s.confidence).sum::<f32>() / reasoning_steps.len() as f32
        };

        let decision_process = DecisionProcess {
            situation_analysis: situation_analysis.clone(),
            data_collected,
            reasoning_steps,
            decisions,
            conclusion: conclusion.clone(),
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
            summary: conclusion,
            success_rate,
        };

        Ok((decision_process, execution_result))
    }

    /// Internal execution logic with event data.
    async fn execute_internal_with_event(
        &self,
        context: ExecutionContext,
        event_data: EventTriggerData,
    ) -> Result<(DecisionProcess, StorageExecutionResult), AgentError> {
        let mut agent = context.agent;

        // Step 1: Collect data including event data
        let data_collected = self.collect_data_with_event(&agent, &event_data).await?;

        // Step 1.5: Parse intent if not already done
        let parsed_intent = if agent.parsed_intent.is_none() {
            match self.parse_intent(&agent.user_prompt).await {
                Ok(intent) => {
                    // Update agent with parsed intent
                    let _ = self.store.update_agent_parsed_intent(&agent.id, Some(intent.clone())).await;
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

        // Step 2: Analyze situation with LLM
        let (situation_analysis, reasoning_steps, decisions, conclusion) =
            self.analyze_situation_with_intent(&agent, &data_collected, parsed_intent.as_ref(), &context.execution_id).await?;

        // Step 3: Execute decisions
        let (actions_executed, notifications_sent) =
            self.execute_decisions(&agent, &decisions).await?;

        // Step 4: Generate report if needed
        let report = self.maybe_generate_report(&agent, &data_collected).await?;

        // Step 5: Update memory with learnings
        let updated_memory = self.update_memory(
            &agent,
            &data_collected,
            &decisions,
            &situation_analysis,
            &conclusion,
        ).await?;

        // Save updated memory
        self.store
            .update_agent_memory(&agent.id, updated_memory.clone())
            .await
            .map_err(|e| AgentError::Storage(format!("Failed to update memory: {}", e)))?;

        // Calculate confidence from reasoning
        let confidence = if reasoning_steps.is_empty() {
            0.5
        } else {
            reasoning_steps.iter().map(|s| s.confidence).sum::<f32>() / reasoning_steps.len() as f32
        };

        let decision_process = DecisionProcess {
            situation_analysis: situation_analysis.clone(),
            data_collected,
            reasoning_steps,
            decisions,
            conclusion: conclusion.clone(),
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
            summary: conclusion,
            success_rate,
        };

        Ok((decision_process, execution_result))
    }

    /// Collect real data from time series storage.
    async fn collect_data(&self, agent: &AiAgent) -> Result<Vec<DataCollected>, AgentError> {
        let mut data = Vec::new();
        let timestamp = chrono::Utc::now().timestamp();

        // Collect real data from time series storage if available
        if let Some(ref storage) = self.time_series_storage {
            for resource in &agent.resources {
                if resource.resource_type == ResourceType::Metric {
                    // Parse device_id and metric from resource_id
                    // Format: "device_id:metric_name"
                    let parts: Vec<&str> = resource.resource_id.split(':').collect();
                    if parts.len() == 2 {
                        let device_id = parts[0];
                        let metric_name = parts[1];

                        // Read data collection config from resource.config
                        let time_range_minutes = resource.config
                            .get("data_collection")
                            .and_then(|dc| dc.get("time_range_minutes"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(60); // Default: 60 minutes

                        let include_history = resource.config
                            .get("data_collection")
                            .and_then(|dc| dc.get("include_history"))
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false); // Default: only latest value

                        let max_points = resource.config
                            .get("data_collection")
                            .and_then(|dc| dc.get("max_points"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(1000) as usize;

                        // Query time series data with configured range
                        let end_time = chrono::Utc::now().timestamp_millis();
                        let start_time = end_time - ((time_range_minutes * 60) as i64 * 1000);

                        if let Ok(result) = storage.query_range(
                            device_id,
                            metric_name,
                            start_time,
                            end_time,
                        ).await {
                            if !result.points.is_empty() {
                                // Get the latest value
                                let latest = &result.points[result.points.len() - 1];

                                // Check if this is an image metric
                                let is_image = is_image_metric(metric_name, &latest.value);
                                let (image_url, image_base64, image_mime) = if is_image {
                                    extract_image_data(&latest.value)
                                } else {
                                    (None, None, None)
                                };

                                let mut values_json = serde_json::json!({
                                    "value": latest.value,
                                    "timestamp": latest.timestamp,
                                    "points_count": result.points.len(),
                                    "time_range_minutes": time_range_minutes,
                                });

                                // Add image metadata if applicable
                                if is_image {
                                    values_json["_is_image"] = serde_json::json!(true);
                                    if let Some(url) = &image_url {
                                        values_json["image_url"] = serde_json::json!(url);
                                    }
                                    if let Some(base64) = &image_base64 {
                                        values_json["image_base64"] = serde_json::json!(base64);
                                    }
                                    if let Some(mime) = &image_mime {
                                        values_json["image_mime_type"] = serde_json::json!(mime);
                                    }

                                    tracing::debug!(
                                        metric = %metric_name,
                                        has_url = image_url.is_some(),
                                        has_base64 = image_base64.is_some(),
                                        mime = ?image_mime,
                                        "Detected image metric in collect_data"
                                    );
                                }

                                // Include history if configured and not an image
                                // (images are usually too large for full history)
                                if include_history && !is_image && result.points.len() > 1 {
                                    let history_limit = max_points.min(result.points.len());
                                    let start_idx = if result.points.len() > history_limit {
                                        result.points.len() - history_limit
                                    } else {
                                        0
                                    };

                                    let history_values: Vec<_> = result.points[start_idx..]
                                        .iter()
                                        .map(|p| {
                                            serde_json::json!({
                                                "value": p.value,
                                                "timestamp": p.timestamp
                                            })
                                        })
                                        .collect();

                                    values_json["history"] = serde_json::json!(history_values);
                                    values_json["history_count"] = serde_json::json!(history_values.len());

                                    // Calculate basic statistics for numeric values
                                    let nums: Vec<f64> = result.points[start_idx..]
                                        .iter()
                                        .filter_map(|p| p.value.as_f64())
                                        .collect();

                                    if !nums.is_empty() {
                                        let min_val = nums.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                                        let max_val = nums.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                                        let avg_val = nums.iter().sum::<f64>() / nums.len() as f64;

                                        values_json["stats"] = serde_json::json!({
                                            "min": min_val,
                                            "max": max_val,
                                            "avg": avg_val,
                                            "count": nums.len()
                                        });
                                    }

                                    tracing::debug!(
                                        metric = %metric_name,
                                        points_count = history_values.len(),
                                        "Included metric history in data collection"
                                    );
                                }

                                // Include trend data from memory if configured
                                if resource.config
                                    .get("data_collection")
                                    .and_then(|dc| dc.get("include_trend"))
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false)
                                {
                                    let trend_points: Vec<_> = agent.memory.trend_data
                                        .iter()
                                        .filter(|tp| tp.metric == resource.resource_id)
                                        .rev()
                                        .take(100)
                                        .map(|tp| serde_json::json!({
                                            "timestamp": tp.timestamp,
                                            "value": tp.value
                                        }))
                                        .collect();

                                    if !trend_points.is_empty() {
                                        values_json["trend_data"] = serde_json::json!(trend_points);
                                    }
                                }

                                // Include baseline comparison if configured
                                if resource.config
                                    .get("data_collection")
                                    .and_then(|dc| dc.get("include_baseline"))
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false)
                                {
                                    if let Some(baseline) = agent.memory.baselines.get(&resource.resource_id) {
                                        if let Some(current_val) = latest.value.as_f64() {
                                            let diff = current_val - baseline;
                                            let diff_pct = if *baseline != 0.0 {
                                                (diff / *baseline) * 100.0
                                            } else {
                                                0.0
                                            };

                                            values_json["baseline"] = serde_json::json!({
                                                "value": baseline,
                                                "diff": diff,
                                                "diff_pct": diff_pct
                                            });
                                        }
                                    }
                                }

                                data.push(DataCollected {
                                    source: resource.resource_id.clone(),
                                    data_type: metric_name.to_string(),
                                    values: values_json,
                                    timestamp,
                                });
                            }
                        }
                    }
                }
            }
        }

        // For Device type resources, also collect their metrics from time series
        if let Some(ref device_service) = self.device_service {
            for resource in &agent.resources {
                if resource.resource_type == ResourceType::Device {
                    let device_id = &resource.resource_id;

                    // First, try to get device info
                    if let Some(device) = device_service.get_device(device_id).await {
                        // Get device info from DeviceConfig
                        let device_values: serde_json::Value = serde_json::json!({
                            "device_id": device.device_id,
                            "device_type": device.device_type,
                            "name": device.name,
                            "adapter_type": device.adapter_type,
                        });

                        data.push(DataCollected {
                            source: device_id.clone(),
                            data_type: "device_info".to_string(),
                            values: device_values,
                            timestamp,
                        });

                        // Now, collect actual metrics from time series storage
                        if let Some(ref storage) = self.time_series_storage {
                            // Try to get available metrics for this device
                            let end_time = chrono::Utc::now().timestamp_millis();
                            let start_time = end_time - (300 * 1000); // Last 5 minutes

                            // Get metric names from device template or storage
                            // For now, try common image/snapshot metric names
                            let potential_metrics = vec![
                                "values.image", "image", "snapshot", "values.snapshot",
                                "camera.image", "camera.snapshot",
                                "picture", "values.picture",
                                "frame", "values.frame",
                            ];

                            for metric_name in potential_metrics {
                                if let Ok(result) = storage.query_range(
                                    device_id,
                                    metric_name,
                                    start_time,
                                    end_time,
                                ).await {
                                    if !result.points.is_empty() {
                                        let latest = &result.points[result.points.len() - 1];

                                        // Check if this is an image metric
                                        let is_image = is_image_metric(metric_name, &latest.value);
                                        let (image_url, image_base64, image_mime) = if is_image {
                                            extract_image_data(&latest.value)
                                        } else {
                                            (None, None, None)
                                        };

                                        let mut values_json = serde_json::json!({
                                            "value": latest.value,
                                            "timestamp": latest.timestamp,
                                            "points_count": result.points.len(),
                                        });

                                        // Add image metadata if applicable
                                        if is_image {
                                            values_json["_is_image"] = serde_json::json!(true);
                                            if let Some(url) = &image_url {
                                                values_json["image_url"] = serde_json::json!(url);
                                            }
                                            if let Some(base64) = &image_base64 {
                                                values_json["image_base64"] = serde_json::json!(base64);
                                            }
                                            if let Some(mime) = &image_mime {
                                                values_json["image_mime_type"] = serde_json::json!(mime);
                                            }

                                            tracing::info!(
                                                device_id = %device_id,
                                                metric = %metric_name,
                                                has_url = image_url.is_some(),
                                                has_base64 = image_base64.is_some(),
                                                "Collected image metric for device resource"
                                            );
                                        }

                                        data.push(DataCollected {
                                            source: format!("{}:{}", device_id, metric_name),
                                            data_type: metric_name.to_string(),
                                            values: values_json,
                                            timestamp,
                                        });

                                        // If we found an image, we can stop looking for other image metrics
                                        // (one image per device is usually enough for analysis)
                                        if is_image {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add memory context
        if !agent.memory.state_variables.is_empty() {
            data.push(DataCollected {
                source: "memory".to_string(),
                data_type: "state".to_string(),
                values: serde_json::to_value(&agent.memory.state_variables)
                    .unwrap_or_default(),
                timestamp,
            });
        }

        // If no data collected, add a placeholder
        if data.is_empty() {
            data.push(DataCollected {
                source: "system".to_string(),
                data_type: "info".to_string(),
                values: serde_json::json!({"message": "No data sources configured"}),
                timestamp,
            });
        }

        Ok(data)
    }

    /// Collect data including the triggering event data.
    /// This ensures that the event that triggered the agent is included in the analysis.
    async fn collect_data_with_event(
        &self,
        agent: &AiAgent,
        event_data: &EventTriggerData,
    ) -> Result<Vec<DataCollected>, AgentError> {
        let mut data = Vec::new();
        let timestamp = chrono::Utc::now().timestamp();

        // First, add the triggering event data directly
        let event_value_json = serde_json::to_value(&event_data.value).unwrap_or_default();

        // Check if the event value is an image
        let is_image = is_image_metric(&event_data.metric, &event_value_json);
        let (image_url, image_base64, image_mime) = if is_image {
            extract_image_data(&event_value_json)
        } else {
            (None, None, None)
        };

        let mut event_values = serde_json::json!({
            "value": event_data.value,
            "timestamp": event_data.timestamp,
            "_is_event_data": true,
        });

        // Add image metadata if applicable
        if is_image {
            event_values["_is_image"] = serde_json::json!(true);
            if let Some(ref url) = image_url {
                event_values["image_url"] = serde_json::json!(url);
            }
            if let Some(ref base64) = image_base64 {
                event_values["image_base64"] = serde_json::json!(base64);
            }
            if let Some(ref mime) = image_mime {
                event_values["image_mime_type"] = serde_json::json!(mime);
            }

            tracing::info!(
                device_id = %event_data.device_id,
                metric = %event_data.metric,
                has_url = image_url.is_some(),
                has_base64 = image_base64.is_some(),
                mime = ?image_mime,
                "Adding event-triggered image data to collection"
            );
        }

        data.push(DataCollected {
            source: format!("{}:{}", event_data.device_id, event_data.metric),
            data_type: event_data.metric.clone(),
            values: event_values,
            timestamp: event_data.timestamp,
        });

        // Then collect other data from regular sources
        let regular_data = self.collect_data(agent).await?;

        // Add regular data (excluding duplicates)
        for item in regular_data {
            // Skip if it's the "No data sources configured" placeholder
            if item.data_type == "info" && item.source == "system" {
                continue;
            }
            // Skip if it's the same event we already added
            if item.source == format!("{}:{}", event_data.device_id, event_data.metric) {
                continue;
            }
            data.push(item);
        }

        tracing::debug!(
            agent_id = %agent.id,
            event_device = %event_data.device_id,
            event_metric = %event_data.metric,
            total_data_count = data.len(),
            event_is_image = is_image,
            "Collected data including event trigger"
        );

        Ok(data)
    }

    /// Analyze situation using LLM or rule-based logic.
    async fn analyze_situation_with_intent(
        &self,
        agent: &AiAgent,
        data: &[DataCollected],
        parsed_intent: Option<&edge_ai_storage::ParsedIntent>,
        execution_id: &str,
    ) -> Result<(String, Vec<ReasoningStep>, Vec<Decision>, String), AgentError> {
        // Try LLM-based analysis first
        tracing::info!(
            agent_id = %agent.id,
            agent_name = %agent.name,
            "Starting situation analysis, checking LLM availability..."
        );

        match self.get_llm_runtime_for_agent(agent).await {
            Ok(Some(llm)) => {
                tracing::info!(
                    agent_id = %agent.id,
                    "LLM runtime available, performing LLM-based analysis"
                );
                match self.analyze_with_llm(llm, agent, data, parsed_intent, execution_id).await {
                    Ok(result) => {
                        tracing::info!(
                            agent_id = %agent.id,
                            "LLM-based analysis completed successfully"
                        );
                        return Ok(result);
                    }
                    Err(e) => {
                        tracing::warn!(
                            agent_id = %agent.id,
                            error = %e,
                            "LLM analysis failed, falling back to rule-based"
                        );
                    }
                }
            }
            Ok(None) => {
                tracing::warn!(
                    agent_id = %agent.id,
                    "No LLM runtime configured, falling back to rule-based analysis"
                );
            }
            Err(e) => {
                tracing::warn!(
                    agent_id = %agent.id,
                    error = %e,
                    "Failed to get LLM runtime, falling back to rule-based"
                );
            }
        }

        // Fall back to rule-based logic
        self.analyze_rule_based(agent, data, parsed_intent).await
    }

    /// Analyze situation using LLM for intelligent decision making.
    async fn analyze_with_llm(
        &self,
        llm: Arc<dyn edge_ai_core::llm::backend::LlmRuntime + Send + Sync>,
        agent: &AiAgent,
        data: &[DataCollected],
        parsed_intent: Option<&edge_ai_storage::ParsedIntent>,
        execution_id: &str,
    ) -> Result<(String, Vec<ReasoningStep>, Vec<Decision>, String), AgentError> {
        use edge_ai_core::llm::backend::{LlmInput, GenerationParams};

        tracing::info!(
            agent_id = %agent.id,
            data_count = data.len(),
            "Calling LLM for situation analysis..."
        );

        // Check if any data contains images
        let has_images = data.iter().any(|d| {
            d.values.get("_is_image").and_then(|v| v.as_bool()).unwrap_or(false)
        });

        // Build text data summary for non-image data
        let text_data_summary: Vec<_> = data.iter()
            .filter(|d| !d.values.get("_is_image").and_then(|v| v.as_bool()).unwrap_or(false))
            .map(|d| format!("- {}: {} = {}", d.source, d.data_type, d.values))
            .collect();

        // Collect image parts
        let image_parts: Vec<_> = data.iter()
            .filter_map(|d| {
                let is_image = d.values.get("_is_image").and_then(|v| v.as_bool()).unwrap_or(false);
                if !is_image {
                    return None;
                }

                // Try to get image URL first
                if let Some(url) = d.values.get("image_url").and_then(|v| v.as_str()) {
                    return Some((
                        d.source.clone(),
                        d.data_type.clone(),
                        ImageContent::Url(url.to_string())
                    ));
                }

                // Try to get base64 data
                if let Some(base64) = d.values.get("image_base64").and_then(|v| v.as_str()) {
                    let mime = d.values.get("image_mime_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("image/jpeg");
                    return Some((
                        d.source.clone(),
                        d.data_type.clone(),
                        ImageContent::Base64(base64.to_string(), mime.to_string())
                    ));
                }

                None
            })
            .collect();

        // Build intent context
        let intent_context = if let Some(ref intent) = parsed_intent.or(agent.parsed_intent.as_ref()) {
            format!(
                "\n意图类型: {:?}\n目标指标: {:?}\n条件: {:?}\n动作: {:?}",
                intent.intent_type, intent.target_metrics, intent.conditions, intent.actions
            )
        } else {
            "".to_string()
        };

        // Build history context from conversation turns and memory
        let mut history_parts = Vec::new();

        // Add memory summary if available
        if !agent.memory.state_variables.is_empty() {
            // Get recent analyses from memory
            if let Some(analyses) = agent.memory.state_variables.get("recent_analyses").and_then(|v| v.as_array()) {
                if !analyses.is_empty() {
                    let summary: Vec<_> = analyses.iter()
                        .take(3) // Only show last 3 to avoid too much context
                        .filter_map(|a| {
                            a.get("analysis").and_then(|an| an.as_str()).map(|txt| {
                                let conclusion = a.get("conclusion")
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("");
                                if !conclusion.is_empty() {
                                    format!("- 分析: {} | 结论: {}", txt, conclusion)
                                } else {
                                    format!("- 分析: {}", txt)
                                }
                            })
                        })
                        .collect();

                    if !summary.is_empty() {
                        history_parts.push(format!(
                            "\n## 历史分析 (最近{}次)\n{}",
                            summary.len(),
                            summary.join("\n")
                        ));
                    }
                }
            }

            // Add learned patterns
            if let Some(patterns) = agent.memory.state_variables.get("decision_patterns").and_then(|v| v.as_array()) {
                if !patterns.is_empty() {
                    let recent_patterns: Vec<_> = patterns.iter()
                        .rev()
                        .take(5)
                        .filter_map(|p| {
                            p.get("description").and_then(|d| d.as_str())
                                .map(|desc| format!("- {}", desc))
                        })
                        .collect();

                    if !recent_patterns.is_empty() {
                        history_parts.push(format!(
                            "\n## 学到的决策模式\n{}",
                            recent_patterns.join("\n")
                        ));
                    }
                }
            }

            // Add baselines if available
            if !agent.memory.baselines.is_empty() {
                let baseline_info: Vec<_> = agent.memory.baselines
                    .iter()
                    .take(5)
                    .map(|(metric, value)| format!("- {}: 基线值 {:.2}", metric, value))
                    .collect();
                history_parts.push(format!(
                    "\n## 指标基线\n{}",
                    baseline_info.join("\n")
                ));
            }
        }

        // Add conversation history if available
        if !agent.conversation_history.is_empty() {
            let recent: Vec<_> = agent.conversation_history
                .iter()
                .rev()
                .take(2) // Reduced since we now have memory context
                .collect();
            history_parts.push(format!(
                "\n## 历史执行记录 (最近{}次)\n{}",
                recent.len(),
                recent.iter().rev().enumerate()
                    .map(|(i, turn)| format!(
                        "{}. 触发: {}, 结论: {}",
                        i + 1,
                        turn.trigger_type,
                        turn.output.conclusion
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        let history_context = if !history_parts.is_empty() {
            history_parts.join("\n")
        } else {
            "".to_string()
        };

        // Build system prompt for the agent role
        let role_prompt = match agent.role {
            AgentRole::Monitor => "你是一个监控智能体，负责监控IoT设备和指标。分析当前数据，判断是否异常或需要告警。",
            AgentRole::Executor => "你是一个执行智能体，负责根据条件执行控制操作。分析当前数据，决定是否需要执行动作。",
            AgentRole::Analyst => "你是一个分析智能体，负责分析数据趋势和模式。深入分析数据，提供有价值的洞察。",
        };

        // Enhanced system prompt for image analysis
        let system_prompt = if has_images {
            format!(
                "{}\n\n## 你的任务\n\
                用户指令: {}\n\
                {}\n\
                {}\n\
                \n\
                ## 图像分析能力\n\
                你具备视觉分析能力。用户消息中可能包含图像数据。请仔细观察图像：\n\
                - 描述图像中的关键内容\n\
                - 识别异常、危险或需要注意的情况\n\
                - 结合图像和其他传感器数据做出判断\n\
                \n\
                ## 分析步骤\n\
                1. 理解当前数据和用户指令\n\
                2. 如果有图像，仔细分析图像内容\n\
                3. 检查是否有异常或需要响应的情况\n\
                4. 基于历史数据判断趋势\n\
                5. 决定需要采取的行动\n\
                \n\
                ## 响应格式\n\
                请以JSON格式回复:\n\
                {{\n\
                  \"situation_analysis\": \"情况分析描述（包括对图像的观察）\",\n\
                  \"reasoning_steps\": [\n\
                    {{\"step\": 1, \"description\": \"步骤描述\", \"confidence\": 0.9}}\n\
                  ],\n\
                  \"decisions\": [\n\
                    {{\n\
                      \"decision_type\": \"类型\",\n\
                      \"description\": \"决策描述\",\n\
                      \"action\": \"动作名称\",\n\
                      \"rationale\": \"决策理由\",\n\
                      \"confidence\": 0.85\n\
                    }}\n\
                  ],\n\
                  \"conclusion\": \"总结结论\"\n\
                }}",
                role_prompt,
                agent.user_prompt,
                intent_context,
                history_context
            )
        } else {
            format!(
                "{}\n\n## 你的任务\n\
                用户指令: {}\n\
                {}\n\
                {}\n\
                \n\
                ## 分析步骤\n\
                1. 理解当前数据和用户指令\n\
                2. 检查是否有异常或需要响应的情况\n\
                3. 基于历史数据判断趋势\n\
                4. 决定需要采取的行动\n\
                \n\
                ## 响应格式\n\
                请以JSON格式回复:\n\
                {{\n\
                  \"situation_analysis\": \"情况分析描述\",\n\
                  \"reasoning_steps\": [\n\
                    {{\"step\": 1, \"description\": \"步骤描述\", \"confidence\": 0.9}}\n\
                  ],\n\
                  \"decisions\": [\n\
                    {{\n\
                      \"decision_type\": \"类型\",\n\
                      \"description\": \"决策描述\",\n\
                      \"action\": \"动作名称\",\n\
                      \"rationale\": \"决策理由\",\n\
                      \"confidence\": 0.85\n\
                    }}\n\
                  ],\n\
                  \"conclusion\": \"总结结论\"\n\
                }}",
                role_prompt,
                agent.user_prompt,
                intent_context,
                history_context
            )
        };

        // Build messages - multimodal if images present
        let messages = if has_images {
            // Build multimodal message with text and images
            let mut parts = vec![ContentPart::text(format!(
                "## 当前数据\n\n{}\n\n请分析上述数据和图像，然后做出决策。",
                if text_data_summary.is_empty() {
                    "仅有图像数据".to_string()
                } else {
                    text_data_summary.join("\n")
                }
            ))];

            // Add image references with context
            for (source, data_type, image_content) in &image_parts {
                let context_text = format!("\n\n图像来源: {} ({})", source, data_type);
                parts.push(ContentPart::text(context_text));

                match image_content {
                    ImageContent::Url(url) => {
                        parts.push(ContentPart::image_url(url.clone()));
                        tracing::debug!(source = %source, url = %url, "Adding image URL to LLM message");
                    }
                    ImageContent::Base64(data, mime) => {
                        parts.push(ContentPart::image_base64(data.clone(), mime.clone()));
                        tracing::debug!(source = %source, mime = %mime, "Adding base64 image to LLM message");
                    }
                }
            }

            vec![
                Message::new(MessageRole::System, Content::text(system_prompt)),
                Message::from_parts(MessageRole::User, parts),
            ]
        } else {
            // Text-only message (original behavior)
            let data_summary = if text_data_summary.is_empty() {
                "No data available".to_string()
            } else {
                text_data_summary.join("\n")
            };

            vec![
                Message::new(MessageRole::System, Content::text(system_prompt)),
                Message::new(MessageRole::User, Content::text(format!(
                    "## 当前数据\n\n{}\n\n请分析上述数据并做出决策。",
                    data_summary
                ))),
            ]
        };

        let input = LlmInput {
            messages,
            params: GenerationParams {
                temperature: Some(0.7),
                max_tokens: Some(3000), // Increased to prevent JSON truncation for image analysis
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: Some(Vec::new()),
        };

        match llm.generate(input).await {
            Ok(output) => {
                let json_str = output.text.trim();
                // Extract JSON if wrapped in markdown
                let json_str = if json_str.contains("```json") {
                    json_str.split("```json").nth(1)
                        .and_then(|s| s.split("```").next())
                        .unwrap_or(json_str)
                        .trim()
                } else if json_str.contains("```") {
                    json_str.split("```").nth(1)
                        .unwrap_or(json_str)
                        .trim()
                } else {
                    json_str
                };

                // Parse the LLM response
                #[derive(serde::Deserialize)]
                struct LlmResponse {
                    situation_analysis: String,
                    reasoning_steps: Vec<ReasoningFromLlm>,
                    decisions: Vec<DecisionFromLlm>,
                    conclusion: String,
                }

                #[derive(serde::Deserialize)]
                struct ReasoningFromLlm {
                    step: u32,
                    description: String,
                    #[serde(default)]
                    confidence: f32,
                }

                #[derive(serde::Deserialize)]
                struct DecisionFromLlm {
                    decision_type: String,
                    description: String,
                    action: String,
                    rationale: String,
                    #[serde(default)]
                    confidence: f32,
                }

                match serde_json::from_str::<LlmResponse>(json_str) {
                    Ok(response) => {
                        let reasoning_steps: Vec<edge_ai_storage::ReasoningStep> = response.reasoning_steps
                            .into_iter()
                            .enumerate()
                            .map(|(i, step)| edge_ai_storage::ReasoningStep {
                                step_number: step.step as u32,
                                description: step.description,
                                step_type: "llm_analysis".to_string(),
                                input: Some(text_data_summary.join("\n")),
                                output: response.situation_analysis.clone(),
                                confidence: step.confidence,
                            })
                            .collect();

                        let decisions: Vec<edge_ai_storage::Decision> = response.decisions
                            .into_iter()
                            .map(|d| edge_ai_storage::Decision {
                                decision_type: d.decision_type,
                                description: d.description,
                                action: d.action,
                                rationale: d.rationale,
                                expected_outcome: response.conclusion.clone(),
                            })
                            .collect();

                        // Emit AgentThinking events for each reasoning step
                        if let Some(ref bus) = self.event_bus {
                            let event_timestamp = chrono::Utc::now().timestamp();
                            for step in &reasoning_steps {
                                let _ = bus.publish(NeoTalkEvent::AgentThinking {
                                    agent_id: agent.id.clone(),
                                    execution_id: execution_id.to_string(),
                                    step_number: step.step_number,
                                    step_type: step.step_type.clone(),
                                    description: step.description.clone(),
                                    details: None,
                                    timestamp: event_timestamp,
                                }).await;
                            }

                            // Emit AgentDecision events for each decision
                            for decision in &decisions {
                                let _ = bus.publish(NeoTalkEvent::AgentDecision {
                                    agent_id: agent.id.clone(),
                                    execution_id: execution_id.to_string(),
                                    description: decision.description.clone(),
                                    rationale: decision.rationale.clone(),
                                    action: decision.action.clone(),
                                    confidence: 0.8_f32,
                                    timestamp: event_timestamp,
                                }).await;
                            }
                        }

                        Ok((
                            response.situation_analysis,
                            reasoning_steps,
                            decisions,
                            response.conclusion,
                        ))
                    }
                    Err(parse_error) => {
                        tracing::warn!(
                            error = %parse_error,
                            response_preview = %json_str.chars().take(500).collect::<String>(),
                            "Failed to parse LLM JSON response, attempting to extract text content"
                        );

                        // Try to extract useful content from the non-JSON response
                        // Use the raw text as situation_analysis and create minimal reasoning steps
                        let raw_text = output.text.trim();
                        let situation_analysis = if raw_text.len() > 500 {
                            format!("{}...", &raw_text[..500])
                        } else {
                            raw_text.to_string()
                        };

                        let reasoning_steps = vec![
                            edge_ai_storage::ReasoningStep {
                                step_number: 1,
                                description: "LLM analysis completed (response format varied)".to_string(),
                                step_type: "llm_analysis".to_string(),
                                input: Some(format!("{} data sources", data.len())),
                                output: situation_analysis.clone(),
                                confidence: 0.7,
                            }
                        ];

                        let decisions = vec![
                            edge_ai_storage::Decision {
                                decision_type: "analysis".to_string(),
                                description: "See situation analysis for details".to_string(),
                                action: "review".to_string(),
                                rationale: "LLM provided text response instead of structured JSON".to_string(),
                                expected_outcome: "Manual review of analysis recommended".to_string(),
                            }
                        ];

                        let conclusion = format!("LLM analysis completed. Raw response: {}", situation_analysis);

                        tracing::info!(
                            agent_id = %agent.id,
                            raw_response_length = raw_text.len(),
                            "Using raw LLM response as fallback"
                        );

                        Ok((
                            situation_analysis,
                            reasoning_steps,
                            decisions,
                            conclusion,
                        ))
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    agent_id = %agent.id,
                    error = %e,
                    error_details = ?e,
                    "LLM generation failed - check LLM backend configuration and connectivity"
                );
                Err(AgentError::Llm(format!("LLM generation failed: {}", e)))
            }
        }
    }

    /// Rule-based analysis fallback.
    async fn analyze_rule_based(
        &self,
        agent: &AiAgent,
        data: &[DataCollected],
        parsed_intent: Option<&edge_ai_storage::ParsedIntent>,
    ) -> Result<(String, Vec<ReasoningStep>, Vec<Decision>, String), AgentError> {
        let mut reasoning_steps = Vec::new();
        let mut decisions = Vec::new();

        // Step 1: Understand the situation
        let situation_analysis = format!(
            "Analyzing {} data points for agent '{}'",
            data.len(),
            agent.name
        );

        reasoning_steps.push(ReasoningStep {
            step_number: 1,
            description: "Collect and analyze input data".to_string(),
            step_type: "data_collection".to_string(),
            input: Some(format!("{} data sources", data.len())),
            output: format!("Data collected from {} sources", data.len()),
            confidence: 1.0,
        });

        // Step 2: Evaluate conditions based on parsed intent
        let intent = parsed_intent.or(agent.parsed_intent.as_ref());
        if let Some(ref intent) = intent {
            for condition in &intent.conditions {
                let result = self.evaluate_condition(condition, data).await;

                reasoning_steps.push(ReasoningStep {
                    step_number: reasoning_steps.len() as u32 + 1,
                    description: format!("Evaluate condition: {}", condition),
                    step_type: "condition_eval".to_string(),
                    input: Some(condition.clone()),
                    output: format!("Condition result: {}", result),
                    confidence: 0.8,
                });

                if result {
                    decisions.push(Decision {
                        decision_type: "condition_met".to_string(),
                        description: format!("Condition '{}' is met", condition),
                        action: "trigger_actions".to_string(),
                        rationale: format!("The condition '{}' evaluated to true", condition),
                        expected_outcome: "Execute defined actions".to_string(),
                    });
                }
            }
        }

        // Step 3: Determine actions
        let empty_actions = vec![];
        let actions = intent.map(|i| &i.actions).unwrap_or(&empty_actions);
        if !decisions.is_empty() {
            for action in actions {
                reasoning_steps.push(ReasoningStep {
                    step_number: reasoning_steps.len() as u32 + 1,
                    description: format!("Plan action: {}", action),
                    step_type: "action_planning".to_string(),
                    input: Some(action.clone()),
                    output: format!("Action '{}' queued for execution", action),
                    confidence: 0.7,
                });
            }
        }

        let conclusion = if decisions.is_empty() {
            "No actions required - conditions not met".to_string()
        } else {
            format!("{} action(s) to be executed", decisions.len())
        };

        Ok((situation_analysis, reasoning_steps, decisions, conclusion))
    }

    /// Evaluate a condition against collected data.
    async fn evaluate_condition(&self, condition: &str, data: &[DataCollected]) -> bool {
        let condition_lower = condition.to_lowercase();

        // Check if any data meets the condition
        for data_item in data {
            if let Some(value) = data_item.values.get("value") {
                if let Some(num) = value.as_f64() {
                    if condition_lower.contains("大于") || condition_lower.contains(">") || condition_lower.contains("超过") {
                        if let Some(threshold) = extract_threshold(&condition_lower) {
                            return num > threshold;
                        }
                    } else if condition_lower.contains("小于") || condition_lower.contains("<") || condition_lower.contains("低于") {
                        if let Some(threshold) = extract_threshold(&condition_lower) {
                            return num < threshold;
                        }
                    }
                }
            }
        }

        false
    }

    /// Execute decisions - real command execution.
    async fn execute_decisions(
        &self,
        agent: &AiAgent,
        decisions: &[Decision],
    ) -> Result<(Vec<edge_ai_storage::ActionExecuted>, Vec<edge_ai_storage::NotificationSent>), AgentError> {
        let mut actions_executed = Vec::new();
        let mut notifications_sent = Vec::new();

        for decision in decisions {
            // Execute actions based on decision type
            if decision.decision_type == "condition_met" {
                // Execute commands defined in agent resources
                if let Some(ref device_service) = self.device_service {
                    for resource in &agent.resources {
                        if resource.resource_type == ResourceType::Command {
                            // Parse device_id and command from resource_id
                            // Format: "device_id:command_name"
                            let parts: Vec<&str> = resource.resource_id.split(':').collect();
                            if parts.len() == 2 {
                                let device_id = parts[0];
                                let command_name = parts[1];

                                // Get parameters from resource config
                                let parameters = resource.config.get("parameters")
                                    .and_then(|v| v.as_object())
                                    .cloned()
                                    .unwrap_or_default();

                                // Convert parameters to HashMap for DeviceService
                                let params_map: std::collections::HashMap<String, serde_json::Value> = parameters
                                    .into_iter()
                                    .map(|(k, v)| (k, v))
                                    .collect();

                                // Actually execute the command via DeviceService
                                let execution_result = device_service.send_command(
                                    device_id,
                                    command_name,
                                    params_map,
                                ).await;

                                let (success, result) = match execution_result {
                                    Ok(_) => (true, Some("Command sent successfully".to_string())),
                                    Err(e) => {
                                        tracing::warn!(
                                            agent_id = %agent.id,
                                            device_id = %device_id,
                                            command = %command_name,
                                            error = %e,
                                            "Failed to send command"
                                        );
                                        (false, Some(format!("Failed: {}", e)))
                                    }
                                };

                                // Re-create parameters for ActionExecuted record
                                let parameters_for_record = resource.config.get("parameters")
                                    .and_then(|v| v.as_object())
                                    .cloned()
                                    .unwrap_or_default();

                                actions_executed.push(edge_ai_storage::ActionExecuted {
                                    action_type: "device_command".to_string(),
                                    description: format!("Execute {} on {}", command_name, device_id),
                                    target: device_id.to_string(),
                                    parameters: serde_json::to_value(parameters_for_record).unwrap_or_default(),
                                    success,
                                    result,
                                });
                            }
                        }
                    }
                }

                // Send notifications for alert actions
                let should_send_alert = agent.parsed_intent.as_ref()
                    .map(|i| i.actions.iter().any(|a| {
                        a.contains("alert") || a.contains("notification") ||
                        a.contains("报警") || a.contains("通知")
                    }))
                    .unwrap_or(false);

                // Debug log for notification trigger
                tracing::debug!(
                    agent_id = %agent.id,
                    should_send_alert,
                    has_parsed_intent = agent.parsed_intent.is_some(),
                    actions = ?agent.parsed_intent.as_ref().map(|i| &i.actions),
                    has_alert_manager = self.alert_manager.is_some(),
                    "Checking if alert should be sent"
                );

                if should_send_alert {
                    let alert_message = format!("Agent '{}' triggered: {}", agent.name, decision.description);

                    // Send via AlertManager if available
                    if let Some(ref alert_manager) = self.alert_manager {
                        use edge_ai_alerts::{Alert as AlertsAlert, AlertSeverity as AlertsAlertSeverity};
                        let alert = AlertsAlert::new(
                            AlertsAlertSeverity::Info,
                            format!("Agent Alert: {}", agent.name),
                            alert_message.clone(),
                            agent.id.clone(),
                        );

                        tracing::info!(
                            agent_id = %agent.id,
                            alert_message = %alert_message,
                            "Sending alert via AlertManager"
                        );

                        match alert_manager.create_alert(alert).await {
                            Ok(alert) => {
                                notifications_sent.push(edge_ai_storage::NotificationSent {
                                    channel: "alert_manager".to_string(),
                                    recipient: "configured_channels".to_string(),
                                    message: alert_message,
                                    sent_at: chrono::Utc::now().timestamp(),
                                    success: true,
                                });
                                tracing::info!(
                                    agent_id = %agent.id,
                                    alert_id = %alert.id.to_string(),
                                    "Alert sent via AlertManager successfully"
                                );
                            }
                            Err(e) => {
                                notifications_sent.push(edge_ai_storage::NotificationSent {
                                    channel: "alert_manager".to_string(),
                                    recipient: "configured_channels".to_string(),
                                    message: alert_message.clone(),
                                    sent_at: chrono::Utc::now().timestamp(),
                                    success: false,
                                });
                                tracing::warn!(
                                    agent_id = %agent.id,
                                    error = %e,
                                    "Failed to send alert via AlertManager"
                                );
                            }
                        }
                    } else {
                        // Fallback: Publish event to EventBus if AlertManager not available
                        tracing::warn!(
                            agent_id = %agent.id,
                            "AlertManager not available, using EventBus fallback"
                        );
                        if let Some(ref bus) = self.event_bus {
                            let _ = bus.publish(NeoTalkEvent::AlertCreated {
                                alert_id: uuid::Uuid::new_v4().to_string(),
                                title: format!("Agent Alert: {}", agent.name),
                                severity: "info".to_string(),
                                message: decision.description.clone(),
                                timestamp: chrono::Utc::now().timestamp(),
                            }).await;

                            notifications_sent.push(edge_ai_storage::NotificationSent {
                                channel: "event_bus".to_string(),
                                recipient: "event_subscribers".to_string(),
                                message: alert_message,
                                sent_at: chrono::Utc::now().timestamp(),
                                success: true,
                            });
                        }
                    }
                }
            }
        }

        Ok((actions_executed, notifications_sent))
    }

    /// Generate report if needed.
    async fn maybe_generate_report(
        &self,
        agent: &AiAgent,
        data: &[DataCollected],
    ) -> Result<Option<GeneratedReport>, AgentError> {
        // Only generate reports for report generation agents
        if let Some(ref intent) = agent.parsed_intent {
            if matches!(
                intent.intent_type,
                edge_ai_storage::IntentType::ReportGeneration
            ) {
                let content = self.generate_report_content(agent, data).await?;

                return Ok(Some(GeneratedReport {
                    report_type: "summary".to_string(),
                    content,
                    data_summary: data
                        .iter()
                        .map(|d| edge_ai_storage::DataSummary {
                            source: d.source.clone(),
                            metric: d.data_type.clone(),
                            count: 1,
                            statistics: d.values.clone(),
                        })
                        .collect(),
                    generated_at: chrono::Utc::now().timestamp(),
                }));
            }
        }

        Ok(None)
    }

    /// Generate report content.
    async fn generate_report_content(&self, agent: &AiAgent, data: &[DataCollected]) -> Result<String, AgentError> {
        let mut report = format!("# {} - 报告\n\n", agent.name);
        report.push_str(&format!("生成时间: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")));

        report.push_str("## 数据摘要\n\n");
        for data_item in data {
            report.push_str(&format!("- **{}**: {}\n", data_item.source, data_item.values));
        }

        report.push_str("\n## 分析结果\n\n");
        if let Some(ref intent) = agent.parsed_intent {
            report.push_str(&format!("意图类型: {:?}\n", intent.intent_type));
            report.push_str(&format!("目标指标: {:?}\n", intent.target_metrics));
        }

        report.push_str("\n## 结论\n\n");
        report.push_str(&agent.user_prompt);

        Ok(report)
    }

    /// Update agent memory with learnings from this execution.
    async fn update_memory(
        &self,
        agent: &AiAgent,
        data: &[DataCollected],
        decisions: &[Decision],
        situation_analysis: &str,
        conclusion: &str,
    ) -> Result<AgentMemory, AgentError> {
        let mut memory = agent.memory.clone();

        // Add trend data points for numeric metrics
        // Note: data_type contains the metric name (e.g., "temperature", "values.image"), not "metric"
        // We filter out non-numeric data types like "device_info", "state", "info"
        let is_numeric_data = |data_type: &str| {
            !matches!(data_type, "device_info" | "state" | "info")
        };

        for data_item in data {
            // Skip non-numeric data types
            if !is_numeric_data(&data_item.data_type) {
                continue;
            }

            // Try to extract numeric value
            if let Some(value) = data_item.values.get("value") {
                if let Some(num) = value.as_f64() {
                    // Add to trend data
                    memory.trend_data.push(TrendPoint {
                        timestamp: data_item.timestamp,
                        metric: data_item.source.clone(),
                        value: num,
                        context: Some(serde_json::json!(data_item.data_type)),
                    });

                    // Keep only last 1000 points
                    if memory.trend_data.len() > 1000 {
                        memory.trend_data = memory.trend_data.split_off(memory.trend_data.len() - 1000);
                    }

                    // Update baseline using exponential moving average
                    let baseline = memory.baselines.entry(data_item.source.clone()).or_insert(num);
                    *baseline = *baseline * 0.9 + num * 0.1;
                }
            }
        }

        // Store LLM analysis results in state_variables
        if !situation_analysis.is_empty() {
            // Store recent analysis (keep last 5)
            // When exceeding 5, consolidate older analyses into a summary
            let analyses_key = "recent_analyses";
            let analyses = memory.state_variables
                .entry(analyses_key.to_string())
                .or_insert(serde_json::json!([]));

            if let Some(arr) = analyses.as_array_mut() {
                // Add new analysis at the beginning
                let new_analysis = serde_json::json!({
                    "timestamp": chrono::Utc::now().timestamp(),
                    "analysis": situation_analysis,
                    "conclusion": conclusion,
                });

                let mut new_vec = vec![new_analysis];

                // If we have more than 4 existing items (total would be 5+),
                // consolidate the oldest items into a summary
                if arr.len() >= 4 {
                    // Take the oldest 2-3 items and create a summary
                    let to_summarize: Vec<_> = arr.iter().skip(arr.len() - 2).cloned().collect();
                    if !to_summarize.is_empty() {
                        // Extract conclusions for summary
                        let conclusions: Vec<_> = to_summarize.iter()
                            .filter_map(|a| a.get("conclusion").and_then(|c| c.as_str()))
                            .collect();

                        let summary_text = if conclusions.len() > 1 {
                            format!("之前{}次分析的结论: {}", conclusions.len(), conclusions.join("; "))
                        } else if conclusions.len() == 1 {
                            format!("之前分析结论: {}", conclusions[0])
                        } else {
                            "之前的分析未见异常".to_string()
                        };

                        // Create consolidated summary entry
                        let consolidated = serde_json::json!({
                            "timestamp": to_summarize[0].get("timestamp").unwrap_or(&serde_json::json!(0)),
                            "analysis": format!("[历史总结] {}", summary_text),
                            "conclusion": "",
                            "is_summary": true,
                            "count": to_summarize.len(),
                        });
                        new_vec.push(consolidated);
                    }

                    // Keep the middle items (excluding newest and oldest)
                    let middle_start = if arr.len() > 3 { arr.len() - 4 } else { 0 };
                    let middle_end = if arr.len() > 2 { arr.len() - 2 } else { arr.len() };
                    if middle_end > middle_start {
                        new_vec.extend(arr.iter().skip(middle_start).take(middle_end - middle_start).cloned());
                    }
                } else {
                    // Just keep existing items (no consolidation needed)
                    new_vec.extend(arr.iter().cloned());
                }

                *analyses = serde_json::Value::Array(new_vec.into_iter().collect());
            }
        }

        // Store decision patterns from this execution
        if !decisions.is_empty() {
            let patterns_key = "decision_patterns";
            let patterns = memory.state_variables
                .entry(patterns_key.to_string())
                .or_insert(serde_json::json!([]));

            if let Some(arr) = patterns.as_array_mut() {
                for decision in decisions {
                    if !decision.decision_type.is_empty() {
                        let pattern = serde_json::json!({
                            "type": decision.decision_type,
                            "description": decision.description,
                            "timestamp": chrono::Utc::now().timestamp(),
                        });
                        arr.push(pattern);
                    }
                }
                // Keep only last 20 patterns
                if arr.len() > 20 {
                    let trimmed: Vec<serde_json::Value> = arr.iter().skip(arr.len() - 20).cloned().collect();
                    memory.state_variables.insert(patterns_key.to_string(), serde_json::json!(trimmed));
                }
            }
        }

        // Also track execution count in state_variables
        let execution_count = memory.state_variables
            .get("total_executions")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) + 1;
        memory.state_variables.insert(
            "total_executions".to_string(),
            serde_json::json!(execution_count),
        );
        memory.state_variables.insert(
            "last_execution".to_string(),
            serde_json::json!(chrono::Utc::now().timestamp()),
        );
        memory.state_variables.insert(
            "last_situation_analysis".to_string(),
            serde_json::json!(situation_analysis),
        );
        memory.state_variables.insert(
            "last_conclusion".to_string(),
            serde_json::json!(conclusion),
        );

        // Store metrics we've seen
        for data_item in data {
            if is_numeric_data(&data_item.data_type) {
                let metrics_seen = memory.state_variables
                    .entry("metrics_seen".to_string())
                    .or_insert(serde_json::json!([]));
                if let Some(arr) = metrics_seen.as_array_mut() {
                    let metric_ref = data_item.source.clone();
                    if !arr.iter().any(|v| v.as_str() == Some(&metric_ref)) {
                        arr.push(serde_json::json!(metric_ref));
                    }
                }
            }
        }

        memory.updated_at = chrono::Utc::now().timestamp();

        tracing::debug!(
            trend_data_count = memory.trend_data.len(),
            baselines_count = memory.baselines.len(),
            state_variables_count = memory.state_variables.len(),
            has_analysis = !situation_analysis.is_empty(),
            "Agent memory updated"
        );

        Ok(memory)
    }

    /// Build LLM messages with conversation history for context-aware execution.
    pub fn build_conversation_messages(
        &self,
        agent: &AiAgent,
        current_data: &[DataCollected],
        event_data: Option<serde_json::Value>,
    ) -> Vec<Message> {
        let mut messages = Vec::new();

        // 1. Role-specific system prompt with conversation context
        let role_str = format!("{:?}", agent.role);
        let system_prompt = get_role_system_prompt(
            &role_str,
            &agent.user_prompt,
            Language::Chinese,
        );
        messages.push(Message::system(system_prompt));

        // 2. Add conversation summary if available
        if let Some(ref summary) = agent.conversation_summary {
            messages.push(Message::system(format!(
                "## 历史对话摘要\n\n{}",
                summary
            )));
        }

        // 3. Add recent conversation turns as context
        let context_window = agent.context_window_size;
        let recent_turns: Vec<_> = agent.conversation_history
            .iter()
            .rev()
            .take(context_window)
            .collect();

        if !recent_turns.is_empty() {
            messages.push(Message::system(format!(
                "## 之前的执行历史 (最近 {} 次)\n\n请参考以下历史记录，避免重复告警，追踪趋势变化。",
                recent_turns.len()
            )));

            // Add each turn as context (in reverse order since we collected reversed)
            for (i, turn) in recent_turns.iter().rev().enumerate() {
                let timestamp_str = chrono::DateTime::from_timestamp(turn.timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let turn_context = format!(
                    "### 历史执行 #{} ({})\n触发方式: {}\n分析: {}\n结论: {}",
                    i + 1,
                    timestamp_str,
                    turn.trigger_type,
                    turn.output.situation_analysis,
                    turn.output.conclusion
                );

                messages.push(Message::system(turn_context));

                // Add decisions if any
                if !turn.output.decisions.is_empty() {
                    let decisions_summary: Vec<String> = turn.output.decisions
                        .iter()
                        .map(|d| format!("- {}", d.description))
                        .collect();
                    messages.push(Message::system(format!(
                        "历史决策:\n{}",
                        decisions_summary.join("\n")
                    )));
                }
            }

            messages.push(Message::system(
                "## 当前执行\n\n请参考上述历史，分析当前情况。特别注意：\n\
                - 与之前数据相比的变化趋势\n\
                - 之前报告的问题是否持续\n\
                - 避免重复相同的分析或决策".to_string()
            ));
        }

        // 4. Current execution data
        let data_text = if current_data.is_empty() {
            "无数据".to_string()
        } else {
            current_data
                .iter()
                .map(|d| format!("- {}: {}", d.source, d.data_type))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let current_input = format!(
            "## 当前数据\n\n数据来源:\n{}\n\n触发方式: {}\n\n请分析当前情况并做出决策。",
            data_text,
            if event_data.is_some() { "事件触发" } else { "定时/手动" }
        );

        messages.push(Message::user(current_input));

        messages
    }

    /// Create a conversation turn from execution results.
    pub fn create_conversation_turn(
        &self,
        execution_id: String,
        trigger_type: String,
        input_data: Vec<DataCollected>,
        event_data: Option<serde_json::Value>,
        decision_process: &DecisionProcess,
        duration_ms: u64,
        success: bool,
    ) -> ConversationTurn {
        ConversationTurn {
            execution_id,
            timestamp: chrono::Utc::now().timestamp(),
            trigger_type,
            input: TurnInput {
                data_collected: input_data,
                event_data,
            },
            output: TurnOutput {
                situation_analysis: decision_process.situation_analysis.clone(),
                reasoning_steps: decision_process.reasoning_steps.clone(),
                decisions: decision_process.decisions.clone(),
                conclusion: decision_process.conclusion.clone(),
            },
            duration_ms,
            success,
        }
    }
}

/// Check if a metric value contains image data.
fn is_image_metric(metric_name: &str, value: &serde_json::Value) -> bool {
    // Check metric name for image-related keywords
    let name_indicates_image = metric_name.to_lowercase().contains("image")
        || metric_name.to_lowercase().contains("snapshot")
        || metric_name.to_lowercase().contains("photo")
        || metric_name.to_lowercase().contains("picture")
        || metric_name.to_lowercase().contains("camera")
        || metric_name.to_lowercase().contains("video")
        || metric_name.to_lowercase().contains("frame");

    if name_indicates_image {
        return true;
    }

    // Check value for URL or base64 data
    if let Some(s) = value.as_str() {
        // Check for URL
        if s.starts_with("http://") || s.starts_with("https://") {
            return true;
        }
        // Check for base64 image data
        if s.starts_with("data:image/") {
            return true;
        }
        // Check for common base64 prefixes without data URL scheme
        if s.len() > 100 && (s.contains("/9j/") || s.contains("iVBORw0KGgo")) {
            // /9j/ is JPEG magic number in base64
            // iVBORw0KGgo is PNG magic number in base64
            return true;
        }
        false
    } else if let Some(obj) = value.as_object() {
        // Check for image_url, url, base64, or data fields
        obj.contains_key("image_url")
            || obj.contains_key("url")
            || obj.contains_key("base64")
            || obj.contains_key("data")
            || obj.contains_key("image_data")
    } else {
        false
    }
}

/// Extract image data from a metric value.
/// Returns (url, base64_data, mime_type) - at most one will be Some.
fn extract_image_data(value: &serde_json::Value) -> (Option<String>, Option<String>, Option<String>) {
    if let Some(s) = value.as_str() {
        if s.starts_with("http://") || s.starts_with("https://") {
            (Some(s.to_string()), None, None)
        } else if s.starts_with("data:image/") {
            // Parse data URL: data:image/<mime>;base64,<data>
            if let Some(rest) = s.strip_prefix("data:image/") {
                let parts: Vec<&str> = rest.splitn(2, ';').collect();
                if parts.len() == 2 {
                    let mime_type = parts[0].to_string();
                    if let Some(data) = parts[1].strip_prefix("base64,") {
                        (None, Some(data.to_string()), Some(mime_type))
                    } else {
                        (None, Some(parts[1].to_string()), Some(mime_type))
                    }
                } else {
                    (None, Some(rest.to_string()), Some("image/jpeg".to_string()))
                }
            } else {
                (None, Some(s.to_string()), Some("image/jpeg".to_string()))
            }
        } else if s.len() > 100 && (s.contains("/9j/") || s.contains("iVBORw0KGgo")) {
            // Raw base64 image data
            let mime_type = if s.contains("iVBORw0KGgo") {
                "image/png"
            } else {
                "image/jpeg"
            };
            (None, Some(s.to_string()), Some(mime_type.to_string()))
        } else {
            (None, None, None)
        }
    } else if let Some(obj) = value.as_object() {
        // Try various field names
        if let Some(url) = obj.get("image_url").or(obj.get("url")).and_then(|v| v.as_str()) {
            return (Some(url.to_string()), None, None);
        }
        if let Some(base64) = obj.get("base64").or(obj.get("data")).or(obj.get("image_data")).and_then(|v| v.as_str()) {
            let mime = obj.get("mime_type").or(obj.get("type")).and_then(|v| v.as_str())
                .unwrap_or("image/jpeg");
            return (None, Some(base64.to_string()), Some(mime.to_string()));
        }
        (None, None, None)
    } else {
        (None, None, None)
    }
}

/// Helper function to extract metrics from text.
fn extract_metrics(text: &str) -> Vec<String> {
    let mut metrics = Vec::new();

    if text.contains("温度") {
        metrics.push("temperature".to_string());
    }
    if text.contains("湿度") {
        metrics.push("humidity".to_string());
    }
    if text.contains("能耗") || text.contains("功率") || text.contains("电量") {
        metrics.push("power".to_string());
    }
    if text.contains("光照") {
        metrics.push("illuminance".to_string());
    }
    if text.contains("气压") {
        metrics.push("pressure".to_string());
    }

    metrics
}

/// Helper function to extract conditions from text.
fn extract_conditions(text: &str) -> Vec<String> {
    let mut conditions = Vec::new();

    // Look for patterns like "大于30", "小于50", "超过", "低于"
    if text.contains("大于") || text.contains("超过") {
        if let Some(start) = text.find("大于").or_else(|| text.find("超过")) {
            let end = start + 2;
            if end + 10 <= text.len() {
                conditions.push(text[start..end + 10].to_string());
            }
        }
    }

    if text.contains("小于") || text.contains("低于") {
        if let Some(start) = text.find("小于").or_else(|| text.find("低于")) {
            let end = start + 2;
            if end + 10 <= text.len() {
                conditions.push(text[start..end + 10].to_string());
            }
        }
    }

    conditions
}

/// Helper function to extract actions from text.
fn extract_actions(text: &str) -> Vec<String> {
    let mut actions = Vec::new();

    if text.contains("报警") || text.contains("通知") {
        actions.push("send_alert".to_string());
    }
    if text.contains("开关") || text.contains("控制") {
        actions.push("send_command".to_string());
    }
    if text.contains("生成报告") {
        actions.push("generate_report".to_string());
    }

    actions
}

/// Helper function to extract threshold value from condition text.
fn extract_threshold(text: &str) -> Option<f64> {
    // Find numbers in the text
    let nums: Vec<f64> = text
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    nums.first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_metrics() {
        let text = "监控温度和湿度，如果温度大于30度就报警";
        let metrics = extract_metrics(text);
        assert!(metrics.contains(&"temperature".to_string()));
        assert!(metrics.contains(&"humidity".to_string()));
    }

    #[test]
    fn test_extract_threshold() {
        assert_eq!(extract_threshold("大于30"), Some(30.0));
        assert_eq!(extract_threshold("温度超过35.5"), Some(35.5));
    }
}
