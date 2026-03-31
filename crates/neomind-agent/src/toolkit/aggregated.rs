//! Aggregated tools using action-based design pattern.
//!
//! This module consolidates 34+ individual tools into 5 aggregated tools,
//! reducing token usage in tool definitions by ~60%.
//!
//! ## Design Principles
//!
//! - **Action-based routing**: Single tool with `action` parameter to differentiate operations
//! - **Token efficiency**: Smaller schema, shared descriptions
//! - **Backward compatibility**: Output format unchanged from original tools
//!
//! ## Tools
//!
//! 1. `device` - Device operations (list, get, query, control)
//! 2. `agent` - Agent management (list, get, create, update, control, memory)
//! 3. `agent_history` - Execution history (executions, conversation)
//! 4. `rule` - Rule management (list, get, delete, history)
//! 5. `alert` - Alert management (list, create, acknowledge)

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{Result, ToolError};
use super::tool::{object_schema, string_property, Tool, ToolOutput};
use neomind_core::tools::ToolCategory;
use neomind_storage::agents::{AgentMemory, AgentStats};

// ============================================================================
// Device Tool - Aggregates: list_devices, get_device, query_data, control_device
// ============================================================================

/// Aggregated device tool with action-based routing.
pub struct DeviceTool {
    device_service: Arc<neomind_devices::DeviceService>,
    storage: Option<Arc<neomind_devices::TimeSeriesStorage>>,
}

impl DeviceTool {
    /// Create a new device tool.
    pub fn new(device_service: Arc<neomind_devices::DeviceService>) -> Self {
        Self {
            device_service,
            storage: None,
        }
    }

    /// Create with time series storage.
    pub fn with_storage(
        device_service: Arc<neomind_devices::DeviceService>,
        storage: Arc<neomind_devices::TimeSeriesStorage>,
    ) -> Self {
        Self {
            device_service,
            storage: Some(storage),
        }
    }
}

#[async_trait]
impl Tool for DeviceTool {
    fn name(&self) -> &str {
        "device"
    }

    fn description(&self) -> &str {
        "设备操作工具。action: list(列出设备), get(获取详情), query(查询数据), control(控制设备)"
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "query", "control"],
                    "description": "操作类型"
                },
                "device_id": string_property("设备ID (get/query/control必填)"),
                "metric": string_property("查询的指标名 (query可选)"),
                "command": string_property("控制命令 (control必填)"),
                "params": {
                    "type": "object",
                    "description": "控制参数 (control可选)"
                },
                "device_type": string_property("按类型过滤 (list可选)"),
                "start_time": {
                    "type": "number",
                    "description": "开始时间戳 (query可选)"
                },
                "end_time": {
                    "type": "number",
                    "description": "结束时间戳 (query可选)"
                },
                "limit": {
                    "type": "number",
                    "description": "返回数量限制 (可选)"
                }
            }),
            vec!["action".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Device
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "list" => self.execute_list(&args).await,
            "get" => self.execute_get(&args).await,
            "query" => self.execute_query(&args).await,
            "control" => self.execute_control(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: {}",
                action
            ))),
        }
    }
}

impl DeviceTool {
    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let devices = self.device_service.list_devices().await;

        let filtered: Vec<Value> = devices
            .iter()
            .filter(|d| {
                if let Some(dt) = args["device_type"].as_str() {
                    d.device_type == dt
                } else {
                    true
                }
            })
            .map(|d| {
                serde_json::json!({
                    "id": d.device_id,
                    "name": d.name,
                    "type": d.device_type,
                    "adapter_type": d.adapter_type
                })
            })
            .collect();

        let limit = args["limit"].as_u64().unwrap_or(filtered.len() as u64) as usize;
        let result: Vec<_> = filtered.into_iter().take(limit).collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": result.len(),
            "devices": result
        })))
    }

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let device_id = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id is required".into()))?;

        let device = self
            .device_service
            .get_device(device_id)
            .await
            .ok_or_else(|| ToolError::Execution(format!("Device not found: {}", device_id)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": device.device_id,
            "name": device.name,
            "type": device.device_type,
            "adapter_type": device.adapter_type,
            "adapter_id": device.adapter_id
        })))
    }

    async fn execute_query(&self, args: &Value) -> Result<ToolOutput> {
        let device_id = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id is required".into()))?;

        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ToolError::Execution("Storage not configured".into()))?;

        let metric = args["metric"].as_str();
        let end_time = args["end_time"].as_i64().unwrap_or_else(|| chrono::Utc::now().timestamp());
        let start_time = args["start_time"].as_i64().unwrap_or(end_time - 3600);
        let limit = args["limit"].as_u64().unwrap_or(100) as usize;

        if let Some(m) = metric {
            let data = storage
                .query(device_id, m, start_time, end_time)
                .await
                .map_err(|e| ToolError::Execution(e.to_string()))?;

            Ok(ToolOutput::success(serde_json::json!({
                "device_id": device_id,
                "metric": m,
                "points": data.iter().take(limit).map(|p| serde_json::json!({
                    "timestamp": p.timestamp,
                    "value": p.value
                })).collect::<Vec<_>>()
            })))
        } else {
            Ok(ToolOutput::success(serde_json::json!({
                "device_id": device_id,
                "message": "Specify a metric name to query data"
            })))
        }
    }

    async fn execute_control(&self, args: &Value) -> Result<ToolOutput> {
        let device_id = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id is required".into()))?;

        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("command is required".into()))?;

        let params: HashMap<String, Value> = args
            .get("params")
            .and_then(|p| p.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        self.device_service
            .send_command(device_id, command, params)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "device_id": device_id,
            "command": command,
            "status": "executed"
        })))
    }
}

// ============================================================================
// Agent Tool - Aggregates: list_agents, get_agent, create_agent, update_agent,
//                          control_agent, agent_memory
// ============================================================================

/// Aggregated agent tool with action-based routing.
pub struct AgentTool {
    agent_store: Arc<neomind_storage::AgentStore>,
}

impl AgentTool {
    /// Create a new agent tool.
    pub fn new(agent_store: Arc<neomind_storage::AgentStore>) -> Self {
        Self { agent_store }
    }
}

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "agent"
    }

    fn description(&self) -> &str {
        "AI Agent管理工具。action: list(列出), get(详情), create(创建), update(更新), control(控制:pause/resume), memory(查询记忆)"
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "create", "update", "control", "memory"],
                    "description": "操作类型"
                },
                "agent_id": string_property("Agent ID (get/update/control/memory必填)"),
                "name": string_property("Agent名称 (create/update必填)"),
                "description": string_property("Agent描述 (create/update可选)"),
                "user_prompt": string_property("用户需求描述 (create/update必填)"),
                "schedule_type": string_property("调度类型: event/cron/interval (create必填)"),
                "schedule_config": string_property("调度配置 (create可选)"),
                "control_action": string_property("控制动作: pause/resume (control必填)"),
                "status": string_property("按状态过滤 (list可选): active/paused/stopped/error/executing"),
                "limit": {
                    "type": "number",
                    "description": "返回数量限制 (list/memory可选)"
                },
                "memory_type": string_property("记忆类型: patterns/intents (memory可选)")
            }),
            vec!["action".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "list" => self.execute_list(&args).await,
            "get" => self.execute_get(&args).await,
            "create" => self.execute_create(&args).await,
            "update" => self.execute_update(&args).await,
            "control" => self.execute_control(&args).await,
            "memory" => self.execute_memory(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: {}",
                action
            ))),
        }
    }
}

impl AgentTool {
    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        use neomind_storage::agents::{AgentFilter, AgentStatus};

        let mut filter = AgentFilter::default();

        if let Some(status) = args["status"].as_str() {
            filter.status = match status {
                "active" => Some(AgentStatus::Active),
                "paused" => Some(AgentStatus::Paused),
                "stopped" => Some(AgentStatus::Stopped),
                "error" => Some(AgentStatus::Error),
                "executing" => Some(AgentStatus::Executing),
                _ => None,
            };
        }

        if let Some(limit) = args["limit"].as_u64() {
            filter.limit = Some(limit as usize);
        }

        let agents = self
            .agent_store
            .query_agents(filter)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        let list: Vec<Value> = agents
            .iter()
            .map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "name": a.name,
                    "description": a.description,
                    "status": format!("{:?}", a.status).to_lowercase(),
                    "schedule_type": format!("{:?}", a.schedule.schedule_type).to_lowercase()
                })
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": list.len(),
            "agents": list
        })))
    }

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent = self
            .agent_store
            .get_agent(agent_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", agent_id)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": agent.id,
            "name": agent.name,
            "description": agent.description,
            "user_prompt": agent.user_prompt,
            "status": format!("{:?}", agent.status).to_lowercase(),
            "schedule": agent.schedule,
            "stats": agent.stats
        })))
    }

    async fn execute_create(&self, args: &Value) -> Result<ToolOutput> {
        use neomind_storage::agents::{AgentSchedule, AgentStatus, AiAgent, ScheduleType};

        let name = args["name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("name is required".into()))?;

        let user_prompt = args["user_prompt"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("user_prompt is required".into()))?;

        let schedule_type_str = args["schedule_type"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("schedule_type is required".into()))?;

        let schedule_type = match schedule_type_str {
            "event" => ScheduleType::Event,
            "cron" => ScheduleType::Cron,
            "interval" => ScheduleType::Interval,
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Invalid schedule_type: {}",
                    schedule_type_str
                )))
            }
        };

        let now = chrono::Utc::now().timestamp();
        let agent = AiAgent {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: args["description"].as_str().map(|s| s.to_string()),
            user_prompt: user_prompt.to_string(),
            llm_backend_id: None,
            parsed_intent: None,
            resources: Vec::new(),
            schedule: AgentSchedule {
                schedule_type,
                cron_expression: args["schedule_config"].as_str().map(|s| s.to_string()),
                interval_seconds: args["schedule_config"].as_u64(),
                event_filter: None,
                timezone: None,
            },
            status: AgentStatus::Active,
            priority: 128,
            created_at: now,
            updated_at: now,
            last_execution_at: None,
            stats: AgentStats::default(),
            memory: AgentMemory::default(),
            conversation_history: Vec::new(),
            user_messages: Vec::new(),
            conversation_summary: None,
            context_window_size: 10,
            enable_tool_chaining: false,
            max_chain_depth: 3,
            tool_config: None,
            error_message: None,
        };

        let id = agent.id.clone();
        self.agent_store
            .save_agent(&agent)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": id,
            "name": agent.name,
            "status": "created"
        })))
    }

    async fn execute_update(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let mut agent = self
            .agent_store
            .get_agent(agent_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", agent_id)))?;

        if let Some(name) = args["name"].as_str() {
            agent.name = name.to_string();
        }
        if let Some(desc) = args["description"].as_str() {
            agent.description = Some(desc.to_string());
        }
        if let Some(prompt) = args["user_prompt"].as_str() {
            agent.user_prompt = prompt.to_string();
        }

        self.agent_store
            .save_agent(&agent)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": agent_id,
            "status": "updated"
        })))
    }

    async fn execute_control(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let control_action = args["control_action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("control_action is required".into()))?;

        use neomind_storage::agents::AgentStatus;

        let (status, error_msg) = match control_action {
            "pause" => (AgentStatus::Paused, None),
            "resume" => (AgentStatus::Active, None),
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown control_action: {}",
                    control_action
                )))
            }
        };

        self.agent_store
            .update_agent_status(agent_id, status, error_msg)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": agent_id,
            "action": control_action,
            "status": "success"
        })))
    }

    async fn execute_memory(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent = self
            .agent_store
            .get_agent(agent_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", agent_id)))?;

        let memory_type = args["memory_type"].as_str().unwrap_or("patterns");
        let limit = args["limit"].as_u64().unwrap_or(20) as usize;

        match memory_type {
            "patterns" => Ok(ToolOutput::success(serde_json::json!({
                "agent_id": agent_id,
                "patterns": agent.memory.learned_patterns.iter().take(limit).cloned().collect::<Vec<_>>()
            }))),
            "intents" => Ok(ToolOutput::success(serde_json::json!({
                "agent_id": agent_id,
                "intent": agent.parsed_intent
            }))),
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown memory_type: {}",
                memory_type
            ))),
        }
    }
}

// ============================================================================
// Agent History Tool - Aggregates: executions, conversation
// ============================================================================

/// Aggregated agent history tool.
pub struct AgentHistoryTool {
    agent_store: Arc<neomind_storage::AgentStore>,
}

impl AgentHistoryTool {
    /// Create a new agent history tool.
    pub fn new(agent_store: Arc<neomind_storage::AgentStore>) -> Self {
        Self { agent_store }
    }
}

#[async_trait]
impl Tool for AgentHistoryTool {
    fn name(&self) -> &str {
        "agent_history"
    }

    fn description(&self) -> &str {
        "Agent执行历史工具。action: executions(执行统计), conversation(对话记录)"
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["executions", "conversation"],
                    "description": "操作类型"
                },
                "agent_id": string_property("Agent ID (必填)"),
                "limit": {
                    "type": "number",
                    "description": "返回数量限制 (可选)"
                }
            }),
            vec!["action".to_string(), "agent_id".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Agent
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "executions" => self.execute_executions(&args).await,
            "conversation" => self.execute_conversation(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: {}",
                action
            ))),
        }
    }
}

impl AgentHistoryTool {
    async fn execute_executions(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent = self
            .agent_store
            .get_agent(agent_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", agent_id)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "agent_id": agent_id,
            "stats": agent.stats
        })))
    }

    async fn execute_conversation(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let limit = args["limit"].as_u64().unwrap_or(50) as usize;

        let agent = self
            .agent_store
            .get_agent(agent_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", agent_id)))?;

        let conversation: Vec<Value> = agent
            .conversation_history
            .iter()
            .take(limit)
            .map(|turn| {
                serde_json::json!({
                    "execution_id": turn.execution_id,
                    "timestamp": turn.timestamp,
                    "trigger_type": turn.trigger_type,
                    "input": turn.input,
                    "output": turn.output
                })
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "agent_id": agent_id,
            "messages": conversation
        })))
    }
}

// ============================================================================
// Rule Tool - Aggregates: list_rules, get_rule, delete_rule, history
// ============================================================================

/// Aggregated rule tool.
pub struct RuleTool {
    rule_engine: Arc<neomind_rules::RuleEngine>,
    history_storage: Option<Arc<neomind_rules::RuleHistoryStorage>>,
}

impl RuleTool {
    /// Create a new rule tool.
    pub fn new(rule_engine: Arc<neomind_rules::RuleEngine>) -> Self {
        Self {
            rule_engine,
            history_storage: None,
        }
    }

    /// Create with history storage.
    pub fn with_history(
        rule_engine: Arc<neomind_rules::RuleEngine>,
        history_storage: Arc<neomind_rules::RuleHistoryStorage>,
    ) -> Self {
        Self {
            rule_engine,
            history_storage: Some(history_storage),
        }
    }
}

#[async_trait]
impl Tool for RuleTool {
    fn name(&self) -> &str {
        "rule"
    }

    fn description(&self) -> &str {
        "规则管理工具。action: list(列出规则), get(详情), create(创建规则), update(更新规则), delete(删除), history(执行历史)"
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "create", "update", "delete", "history"],
                    "description": "操作类型"
                },
                "rule_id": string_property("规则ID (get/update/delete必填)"),
                "dsl": string_property("规则DSL定义 (create/update必填)。格式: RULE \"名称\" WHEN 条件 DO 动作 END"),
                "name_filter": string_property("按名称过滤 (list可选)"),
                "limit": {
                    "type": "number",
                    "description": "返回数量限制 (可选)"
                },
                "start_time": {
                    "type": "number",
                    "description": "开始时间戳 (history可选)"
                },
                "end_time": {
                    "type": "number",
                    "description": "结束时间戳 (history可选)"
                }
            }),
            vec!["action".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Rule
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "list" => self.execute_list(&args).await,
            "get" => self.execute_get(&args).await,
            "create" => self.execute_create(&args).await,
            "update" => self.execute_update(&args).await,
            "delete" => self.execute_delete(&args).await,
            "history" => self.execute_history(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: {}",
                action
            ))),
        }
    }
}

impl RuleTool {
    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let rules = self.rule_engine.list_rules().await;

        let name_filter = args["name_filter"].as_str();
        let limit = args["limit"].as_u64().unwrap_or(100) as usize;

        let filtered: Vec<Value> = rules
            .iter()
            .filter(|r| {
                if let Some(name) = name_filter {
                    r.name.contains(name)
                } else {
                    true
                }
            })
            .take(limit)
            .map(|r| {
                serde_json::json!({
                    "id": r.id.to_string(),
                    "name": r.name,
                    "description": r.description
                })
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": filtered.len(),
            "rules": filtered
        })))
    }

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let rule_id_str = args["rule_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("rule_id is required".into()))?;

        let rule_id = neomind_rules::RuleId::from_string(rule_id_str)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid rule_id: {}", e)))?;

        let rule = self
            .rule_engine
            .get_rule(&rule_id)
            .await
            .ok_or_else(|| ToolError::Execution(format!("Rule not found: {}", rule_id_str)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": rule.id.to_string(),
            "name": rule.name,
            "description": rule.description,
            "dsl": rule.dsl
        })))
    }

    async fn execute_delete(&self, args: &Value) -> Result<ToolOutput> {
        let rule_id_str = args["rule_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("rule_id is required".into()))?;

        let rule_id = neomind_rules::RuleId::from_string(rule_id_str)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid rule_id: {}", e)))?;

        self.rule_engine
            .remove_rule(&rule_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": rule_id_str,
            "status": "deleted"
        })))
    }

    async fn execute_create(&self, args: &Value) -> Result<ToolOutput> {
        let dsl = args["dsl"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("dsl is required".into()))?;

        let rule_id = self
            .rule_engine
            .add_rule_from_dsl(dsl)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to create rule: {}", e)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": rule_id.to_string(),
            "status": "created",
            "message": "规则创建成功"
        })))
    }

    async fn execute_update(&self, args: &Value) -> Result<ToolOutput> {
        let rule_id_str = args["rule_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("rule_id is required".into()))?;

        let dsl = args["dsl"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("dsl is required".into()))?;

        let rule_id = neomind_rules::RuleId::from_string(rule_id_str)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid rule_id: {}", e)))?;

        // First remove the old rule to clean up dependencies
        self.rule_engine
            .remove_rule(&rule_id)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to remove old rule: {}", e)))?;

        // Parse and add the new rule
        let new_rule_id = self
            .rule_engine
            .add_rule_from_dsl(dsl)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to create updated rule: {}", e)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": new_rule_id.to_string(),
            "old_id": rule_id_str,
            "status": "updated",
            "message": "规则更新成功"
        })))
    }

    async fn execute_history(&self, args: &Value) -> Result<ToolOutput> {
        let storage = self
            .history_storage
            .as_ref()
            .ok_or_else(|| ToolError::Execution("History storage not configured".into()))?;

        use neomind_rules::HistoryFilter;

        let mut filter = HistoryFilter::default();

        if let Some(rule_id) = args["rule_id"].as_str() {
            filter.rule_id = Some(rule_id.to_string());
        }
        if let Some(start) = args["start_time"].as_i64() {
            filter.start = Some(chrono::DateTime::from_timestamp(start, 0).unwrap_or_default());
        }
        if let Some(end) = args["end_time"].as_i64() {
            filter.end = Some(chrono::DateTime::from_timestamp(end, 0).unwrap_or_default());
        }
        if let Some(limit) = args["limit"].as_u64() {
            filter.limit = Some(limit as usize);
        }

        let history = storage
            .query(&filter)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "history": history
        })))
    }
}

// ============================================================================
// Alert Tool - Aggregates: list_alerts, create_alert, acknowledge_alert
// ============================================================================

/// Alert information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedAlertInfo {
    pub id: String,
    pub title: String,
    pub message: String,
    pub severity: AggregatedAlertSeverity,
    pub source: String,
    pub acknowledged: bool,
    pub created_at: i64,
}

/// Alert severity levels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregatedAlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Aggregated alert tool.
pub struct AlertTool {
    message_manager: Option<Arc<neomind_messages::MessageManager>>,
    alerts: Arc<tokio::sync::RwLock<Vec<AggregatedAlertInfo>>>,
}

impl AlertTool {
    /// Create a new alert tool.
    pub fn new() -> Self {
        Self {
            message_manager: None,
            alerts: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Create with message manager for persistent storage.
    pub fn with_message_manager(message_manager: Arc<neomind_messages::MessageManager>) -> Self {
        Self {
            message_manager: Some(message_manager),
            alerts: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

impl Default for AlertTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AlertTool {
    fn name(&self) -> &str {
        "alert"
    }

    fn description(&self) -> &str {
        "告警管理工具。action: list(列出告警), create(创建告警), acknowledge(确认告警)"
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "create", "acknowledge"],
                    "description": "操作类型"
                },
                "alert_id": string_property("告警ID (acknowledge必填)"),
                "title": string_property("告警标题 (create必填)"),
                "message": string_property("告警消息 (create必填)"),
                "severity": string_property("严重程度: info/warning/error/critical (create可选)"),
                "source": string_property("告警来源 (create可选)"),
                "unacknowledged_only": {
                    "type": "boolean",
                    "description": "仅返回未确认的告警 (list可选)"
                },
                "severity_filter": string_property("按严重程度过滤 (list可选)"),
                "limit": {
                    "type": "number",
                    "description": "返回数量限制 (list可选)"
                }
            }),
            vec!["action".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        match action {
            "list" => self.execute_list(&args).await,
            "create" => self.execute_create(&args).await,
            "acknowledge" => self.execute_acknowledge(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: {}",
                action
            ))),
        }
    }
}

impl AlertTool {
    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let unacknowledged_only = args["unacknowledged_only"].as_bool().unwrap_or(false);
        let severity_filter = args["severity_filter"].as_str();
        let limit = args["limit"].as_u64().unwrap_or(50) as usize;

        // Use message_manager if available, otherwise use in-memory storage
        if let Some(manager) = &self.message_manager {
            use neomind_messages::{MessageSeverity, MessageType};

            let messages = manager.list_active_messages().await;

            let filtered: Vec<Value> = messages
                .into_iter()
                .filter(|m| m.message_type == MessageType::Notification)
                .filter(|m| {
                    if unacknowledged_only {
                        m.is_active()
                    } else {
                        true
                    }
                })
                .filter(|m| {
                    if let Some(sev) = severity_filter {
                        let m_sev = match m.severity {
                            MessageSeverity::Info => "info",
                            MessageSeverity::Warning => "warning",
                            MessageSeverity::Critical => "critical",
                            MessageSeverity::Emergency => "emergency",
                        };
                        m_sev == sev
                    } else {
                        true
                    }
                })
                .take(limit)
                .map(|m| {
                    let severity_str = match m.severity {
                        MessageSeverity::Info => "info",
                        MessageSeverity::Warning => "warning",
                        MessageSeverity::Critical => "critical",
                        MessageSeverity::Emergency => "emergency",
                    };
                    serde_json::json!({
                        "id": m.id.to_string(),
                        "title": m.title,
                        "message": m.message,
                        "severity": severity_str,
                        "source": m.source_type,
                        "acknowledged": !m.is_active(),
                        "created_at": m.timestamp.timestamp()
                    })
                })
                .collect();

            Ok(ToolOutput::success(serde_json::json!({
                "count": filtered.len(),
                "alerts": filtered
            })))
        } else {
            let alerts = self.alerts.read().await;

            let filtered: Vec<Value> = alerts
                .iter()
                .filter(|a| {
                    if unacknowledged_only {
                        !a.acknowledged
                    } else {
                        true
                    }
                })
                .filter(|a| {
                    if let Some(sev) = severity_filter {
                        format!("{:?}", a.severity).to_lowercase() == sev.to_lowercase()
                    } else {
                        true
                    }
                })
                .take(limit)
                .map(|a| serde_json::to_value(a).unwrap())
                .collect();

            Ok(ToolOutput::success(serde_json::json!({
                "count": filtered.len(),
                "alerts": filtered
            })))
        }
    }

    async fn execute_create(&self, args: &Value) -> Result<ToolOutput> {
        let title = args["title"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("title is required".into()))?;

        let message = args["message"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("message is required".into()))?;

        let severity_str = args["severity"].as_str().unwrap_or("warning");
        let source = args["source"].as_str().unwrap_or("system");

        // Use message_manager if available
        if let Some(manager) = &self.message_manager {
            use neomind_messages::{Message, MessageSeverity};

            let severity = match severity_str {
                "info" => MessageSeverity::Info,
                "warning" => MessageSeverity::Warning,
                "critical" => MessageSeverity::Critical,
                "emergency" => MessageSeverity::Emergency,
                _ => MessageSeverity::Warning,
            };

            let msg = Message::new(
                "alert",
                severity,
                title.to_string(),
                message.to_string(),
                source.to_string(),
            );

            let msg = manager.create_message(msg).await
                .map_err(|e| ToolError::Execution(e.to_string()))?;

            Ok(ToolOutput::success(serde_json::json!({
                "id": msg.id.to_string(),
                "status": "created"
            })))
        } else {
            let severity = match severity_str {
                "info" => AggregatedAlertSeverity::Info,
                "warning" => AggregatedAlertSeverity::Warning,
                "error" => AggregatedAlertSeverity::Error,
                "critical" => AggregatedAlertSeverity::Critical,
                _ => AggregatedAlertSeverity::Warning,
            };

            let alert = AggregatedAlertInfo {
                id: uuid::Uuid::new_v4().to_string(),
                title: title.to_string(),
                message: message.to_string(),
                severity,
                source: source.to_string(),
                acknowledged: false,
                created_at: chrono::Utc::now().timestamp(),
            };

            let id = alert.id.clone();
            self.alerts.write().await.push(alert);

            Ok(ToolOutput::success(serde_json::json!({
                "id": id,
                "status": "created"
            })))
        }
    }

    async fn execute_acknowledge(&self, args: &Value) -> Result<ToolOutput> {
        let alert_id_str = args["alert_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("alert_id is required".into()))?;

        // Use message_manager if available
        if let Some(manager) = &self.message_manager {
            use neomind_messages::MessageId;

            let alert_id = MessageId::from_string(alert_id_str)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid alert_id: {}", e)))?;

            manager.acknowledge(&alert_id).await
                .map_err(|e| ToolError::Execution(e.to_string()))?;

            Ok(ToolOutput::success(serde_json::json!({
                "id": alert_id_str,
                "status": "acknowledged"
            })))
        } else {
            let mut alerts = self.alerts.write().await;
            let alert = alerts
                .iter_mut()
                .find(|a| a.id == alert_id_str)
                .ok_or_else(|| ToolError::Execution("Alert not found".into()))?;

            alert.acknowledged = true;

            Ok(ToolOutput::success(serde_json::json!({
                "id": alert_id_str,
                "status": "acknowledged"
            })))
        }
    }
}

// ============================================================================
// Builder for Aggregated Tools
// ============================================================================

/// Builder for creating all aggregated tools with dependencies.
pub struct AggregatedToolsBuilder {
    device_service: Option<Arc<neomind_devices::DeviceService>>,
    time_series_storage: Option<Arc<neomind_devices::TimeSeriesStorage>>,
    agent_store: Option<Arc<neomind_storage::AgentStore>>,
    rule_engine: Option<Arc<neomind_rules::RuleEngine>>,
    rule_history: Option<Arc<neomind_rules::RuleHistoryStorage>>,
    message_manager: Option<Arc<neomind_messages::MessageManager>>,
}

impl AggregatedToolsBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            device_service: None,
            time_series_storage: None,
            agent_store: None,
            rule_engine: None,
            rule_history: None,
            message_manager: None,
        }
    }

    /// Set device service.
    pub fn with_device_service(mut self, service: Arc<neomind_devices::DeviceService>) -> Self {
        self.device_service = Some(service);
        self
    }

    /// Set time series storage.
    pub fn with_time_series_storage(
        mut self,
        storage: Arc<neomind_devices::TimeSeriesStorage>,
    ) -> Self {
        self.time_series_storage = Some(storage);
        self
    }

    /// Set agent store.
    pub fn with_agent_store(mut self, store: Arc<neomind_storage::AgentStore>) -> Self {
        self.agent_store = Some(store);
        self
    }

    /// Set rule engine.
    pub fn with_rule_engine(mut self, engine: Arc<neomind_rules::RuleEngine>) -> Self {
        self.rule_engine = Some(engine);
        self
    }

    /// Set rule history storage.
    pub fn with_rule_history(mut self, history: Arc<neomind_rules::RuleHistoryStorage>) -> Self {
        self.rule_history = Some(history);
        self
    }

    /// Set message manager for alert tool persistence.
    pub fn with_message_manager(mut self, manager: Arc<neomind_messages::MessageManager>) -> Self {
        self.message_manager = Some(manager);
        self
    }

    /// Build all aggregated tools as a vector of DynTool.
    pub fn build(self) -> Vec<Arc<dyn Tool>> {
        let mut tools: Vec<Arc<dyn Tool>> = Vec::new();

        // Device tool
        if let Some(ds) = self.device_service {
            let device_tool = if let Some(storage) = self.time_series_storage {
                DeviceTool::with_storage(ds, storage)
            } else {
                DeviceTool::new(ds)
            };
            tools.push(Arc::new(device_tool));
        }

        // Agent tools
        if let Some(store) = self.agent_store.clone() {
            tools.push(Arc::new(AgentTool::new(store.clone())));
            tools.push(Arc::new(AgentHistoryTool::new(store)));
        }

        // Rule tool
        if let Some(engine) = self.rule_engine {
            let rule_tool = if let Some(history) = self.rule_history {
                RuleTool::with_history(engine, history)
            } else {
                RuleTool::new(engine)
            };
            tools.push(Arc::new(rule_tool));
        }

        // Alert tool (always available)
        let alert_tool = if let Some(manager) = self.message_manager {
            AlertTool::with_message_manager(manager)
        } else {
            AlertTool::new()
        };
        tools.push(Arc::new(alert_tool));

        tools
    }
}

impl Default for AggregatedToolsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregated_tools_builder_creates_tools() {
        // Test that builder creates tools even without dependencies
        let tools = AggregatedToolsBuilder::new().build();
        // Without dependencies, only AlertTool is created
        assert_eq!(tools.len(), 1);
    }

    #[test]
    fn test_alert_tool_name() {
        // Test AlertTool metadata
        let tool = AlertTool::new();
        assert_eq!(tool.name(), "alert");
    }

    #[tokio::test]
    async fn test_alert_tool_list_empty() {
        // Test listing alerts when none exist
        let tool = AlertTool::new();
        let result = tool
            .execute(serde_json::json!({"action": "list"}))
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.data["count"].as_u64().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_alert_tool_create_and_list() {
        // Test creating and listing alerts
        let tool = AlertTool::new();

        // Create an alert
        let create_result = tool
            .execute(serde_json::json!({
                "action": "create",
                "title": "Test Alert",
                "message": "This is a test",
                "severity": "warning"
            }))
            .await
            .unwrap();

        assert!(create_result.success);
        let alert_id = create_result.data["id"].as_str().unwrap().to_string();

        // List alerts
        let list_result = tool
            .execute(serde_json::json!({"action": "list"}))
            .await
            .unwrap();

        assert!(list_result.success);
        assert_eq!(list_result.data["count"].as_u64().unwrap(), 1);

        // Acknowledge the alert
        let ack_result = tool
            .execute(serde_json::json!({
                "action": "acknowledge",
                "alert_id": alert_id
            }))
            .await
            .unwrap();

        assert!(ack_result.success);
    }

    #[tokio::test]
    async fn test_alert_tool_unknown_action() {
        let tool = AlertTool::new();

        let result = tool
            .execute(serde_json::json!({"action": "unknown_action"}))
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_aggregated_alert_severity_variants() {
        // Test that AggregatedAlertSeverity has all expected variants
        let info = AggregatedAlertSeverity::Info;
        let warning = AggregatedAlertSeverity::Warning;
        let error = AggregatedAlertSeverity::Error;
        let critical = AggregatedAlertSeverity::Critical;

        // Verify Debug trait is implemented
        assert!(format!("{:?}", info).contains("Info"));
        assert!(format!("{:?}", warning).contains("Warning"));
        assert!(format!("{:?}", error).contains("Error"));
        assert!(format!("{:?}", critical).contains("Critical"));
    }

    #[test]
    fn test_aggregated_alert_info_serialization() {
        // Test that AggregatedAlertInfo can be serialized
        let alert = AggregatedAlertInfo {
            id: "test-id".to_string(),
            title: "Test Alert".to_string(),
            message: "Test message".to_string(),
            severity: AggregatedAlertSeverity::Warning,
            source: "test".to_string(),
            acknowledged: false,
            created_at: 1234567890,
        };

        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("Test Alert"));
    }
}
