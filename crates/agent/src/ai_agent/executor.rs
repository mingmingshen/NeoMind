//! AI Agent executor - runs agents and records decision processes.

use edge_ai_core::{EventBus, MetricValue, NeoTalkEvent};
use edge_ai_storage::{
    AgentMemory, AgentStats, AgentStore, AgentExecutionRecord, AiAgent, DataCollected,
    Decision, DecisionProcess, ExecutionResult as StorageExecutionResult, ExecutionStatus,
    GeneratedReport, ReasoningStep, TrendPoint,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{Agent, AgentConfig, LlmBackend};
use crate::error::AgentError;

/// Configuration for agent executor.
#[derive(Debug, Clone)]
pub struct AgentExecutorConfig {
    /// Data directory for storage
    pub data_dir: String,
    /// Maximum retries for failed executions
    pub max_retries: u32,
    /// Timeout for LLM calls (seconds)
    pub llm_timeout_secs: u64,
}

impl Default for AgentExecutorConfig {
    fn default() -> Self {
        Self {
            data_dir: "data".to_string(),
            max_retries: 3,
            llm_timeout_secs: 30,
        }
    }
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
    /// Event bus for data collection
    event_bus: Arc<EventBus>,
    /// Configuration
    config: AgentExecutorConfig,
    /// LLM agents for each agent (cached)
    llm_agents: Arc<RwLock<std::collections::HashMap<String, Arc<Agent>>>>,
}

impl AgentExecutor {
    /// Create a new agent executor.
    pub async fn new(config: AgentExecutorConfig) -> Result<Self, AgentError> {
        let store_path = format!("{}/agents.redb", config.data_dir);
        let store = AgentStore::open(store_path)
            .map_err(|e| AgentError::Storage(format!("Failed to open agent store: {}", e)))?;

        let event_bus = EventBus::new();

        Ok(Self {
            store,
            event_bus: Arc::new(event_bus),
            config,
            llm_agents: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Get the agent store.
    pub fn store(&self) -> Arc<AgentStore> {
        self.store.clone()
    }

    /// Get the event bus.
    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    /// Parse user intent from natural language.
    pub async fn parse_intent(&self, user_prompt: &str) -> Result<edge_ai_storage::ParsedIntent, AgentError> {
        // For now, use simple keyword-based parsing
        // In production, this would use LLM to parse intent

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

        // Extract metrics mentioned
        let target_metrics = extract_metrics(&prompt_lower);

        Ok(edge_ai_storage::ParsedIntent {
            intent_type,
            target_metrics,
            conditions: extract_conditions(&prompt_lower),
            actions: extract_actions(&prompt_lower),
            confidence,
        })
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
            llm_backend: None, // Will use default
        };

        // Execute with error handling for stability
        let execution_result = self.execute_with_retry(context).await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Build execution record
        let record = match execution_result {
            Ok((decision_process, result)) => {
                // Update stats
                self.store
                    .update_agent_stats(&agent_id, true, duration_ms)
                    .await
                    .map_err(|e| AgentError::Storage(format!("Failed to update stats: {}", e)))?;

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
                self.store
                    .update_agent_stats(&agent_id, false, duration_ms)
                    .await
                    .map_err(|err| AgentError::Storage(format!("Failed to update stats: {}", err)))?;

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

        // Reset agent status based on result
        let new_status = if record.status == ExecutionStatus::Completed {
            edge_ai_storage::AgentStatus::Active
        } else {
            edge_ai_storage::AgentStatus::Error
        };

        self.store
            .update_agent_status(&agent_id, new_status, record.error.clone())
            .await
            .map_err(|e| AgentError::Storage(format!("Failed to update status: {}", e)))?;

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

    /// Execute with retry for stability.
    async fn execute_with_retry(
        &self,
        context: ExecutionContext,
    ) -> Result<(DecisionProcess, StorageExecutionResult), AgentError> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self.execute_internal(context.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        agent_id = %context.agent.id,
                        attempt = attempt + 1,
                        max_retries = self.config.max_retries + 1,
                        error = %e,
                        "Agent execution failed, retrying"
                    );
                    last_error = Some(e);

                    // Wait before retry (exponential backoff)
                    if attempt < self.config.max_retries {
                        let delay_ms = 100 * (2_u64.pow(attempt as u32));
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
        let agent = context.agent;

        // Step 1: Collect data
        let data_collected = self.collect_data(&agent).await?;

        // Step 2: Analyze situation with LLM
        let (situation_analysis, reasoning_steps, decisions, conclusion) =
            self.analyze_situation(&agent, &data_collected).await?;

        // Step 3: Execute decisions
        let (actions_executed, notifications_sent) =
            self.execute_decisions(&agent, &decisions).await?;

        // Step 4: Generate report if needed
        let report = self.maybe_generate_report(&agent, &data_collected).await?;

        // Step 5: Update memory with learnings
        let updated_memory = self.update_memory(&agent, &data_collected, &decisions).await?;

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

    /// Collect data for agent execution.
    async fn collect_data(&self, agent: &AiAgent) -> Result<Vec<DataCollected>, AgentError> {
        let mut data = Vec::new();
        let timestamp = chrono::Utc::now().timestamp();

        // For each metric resource, collect data
        for resource in &agent.resources {
            if resource.resource_type == edge_ai_storage::ResourceType::Metric {
                // In production, this would query actual device data
                // For now, create simulated data
                data.push(DataCollected {
                    source: resource.resource_id.clone(),
                    data_type: "metric".to_string(),
                    values: serde_json::json!({
                        "value": 25.0 + (rand::random::<f64>() - 0.5) * 5.0,
                        "timestamp": timestamp,
                    }),
                    timestamp,
                });
            }
        }

        // Add memory context
        if !agent.memory.state_variables.is_empty() {
            data.push(DataCollected {
                source: "memory".to_string(),
                data_type: "state".to_string(),
                values: serde_json::to_value(&agent.memory.state_variables)
                    .map_err(|e| AgentError::Serialization(e.to_string()))?,
                timestamp,
            });
        }

        Ok(data)
    }

    /// Analyze situation using LLM or rule-based logic.
    async fn analyze_situation(
        &self,
        agent: &AiAgent,
        data: &[DataCollected],
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
        if let Some(ref intent) = agent.parsed_intent {
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
        if !decisions.is_empty() {
            for action in agent.parsed_intent.as_ref().map(|i| &i.actions).unwrap_or(&vec![]) {
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
        // Simple condition evaluation
        // In production, this would use more sophisticated logic

        let condition_lower = condition.to_lowercase();

        // Check if any data meets the condition
        for data_item in data {
            if let Some(value) = data_item.values.get("value") {
                if let Some(num) = value.as_f64() {
                    if condition_lower.contains("大于") || condition_lower.contains(">") {
                        // Extract threshold
                        if let Some(threshold) = extract_threshold(&condition_lower) {
                            return num > threshold;
                        }
                    } else if condition_lower.contains("小于") || condition_lower.contains("<") {
                        if let Some(threshold) = extract_threshold(&condition_lower) {
                            return num < threshold;
                        }
                    }
                }
            }
        }

        false
    }

    /// Execute decisions.
    async fn execute_decisions(
        &self,
        _agent: &AiAgent,
        decisions: &[Decision],
    ) -> Result<(Vec<edge_ai_storage::ActionExecuted>, Vec<edge_ai_storage::NotificationSent>), AgentError> {
        let mut actions_executed = Vec::new();
        let mut notifications_sent = Vec::new();

        for decision in decisions {
            // For now, just record the decision as an action
            actions_executed.push(edge_ai_storage::ActionExecuted {
                action_type: decision.decision_type.clone(),
                description: decision.description.clone(),
                target: "system".to_string(),
                parameters: serde_json::json!({}),
                success: true,
                result: Some(decision.rationale.clone()),
            });
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
        _decisions: &[Decision],
    ) -> Result<AgentMemory, AgentError> {
        let mut memory = agent.memory.clone();

        // Add trend data points
        for data_item in data {
            if data_item.data_type == "metric" {
                if let Some(value) = data_item.values.get("value") {
                    if let Some(num) = value.as_f64() {
                        memory.trend_data.push(TrendPoint {
                            timestamp: data_item.timestamp,
                            metric: data_item.source.clone(),
                            value: num,
                            context: None,
                        });

                        // Keep only last 1000 points
                        if memory.trend_data.len() > 1000 {
                            memory.trend_data = memory.trend_data.split_off(memory.trend_data.len() - 1000);
                        }
                    }
                }
            }
        }

        // Update baselines
        for data_item in data {
            if data_item.data_type == "metric" {
                if let Some(value) = data_item.values.get("value") {
                    if let Some(num) = value.as_f64() {
                        let baseline = memory.baselines.entry(data_item.source.clone()).or_insert(num);
                        *baseline = *baseline * 0.9 + num * 0.1; // Exponential moving average
                    }
                }
            }
        }

        memory.updated_at = chrono::Utc::now().timestamp();

        Ok(memory)
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
