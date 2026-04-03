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
use super::tool::{object_schema, Tool, ToolOutput};
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
        r#"Device management tool for querying and controlling IoT devices.

Actions:
- list: List all devices with type/status filtering. Use when user asks what devices exist or their status.
- get: Get device details including capabilities, supported metrics and commands. Use when user asks about a specific device.
- query: Query device time-series data and sensor readings. Defaults to last 1 hour. Use when user asks for current or historical readings.
- control: Send control commands to devices (switch on/off, adjust parameters). WARNING: This changes real device state.

Important:
- Always confirm user intent before using control action
- If device_id is uncertain, call list first, then get/query
- Supports fuzzy matching on device names (partial name works)
- Use response_format="detailed" when you need IDs for follow-up chained calls"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "query", "control"],
                    "description": "Operation type: 'list' (list all devices), 'get' (device details), 'query' (time-series data), 'control' (send command)"
                },
                "device_id": {
                    "type": "string",
                    "description": "Device ID or name. Required for get/query/control. Supports fuzzy matching (e.g., 'living' matches 'Living Room Light'). Examples: 'ne101', 'sensor_1', 'living_room_light'"
                },
                "metric": {
                    "type": "string",
                    "description": "Metric name to query (query action). Format: 'field' or 'values.field'. Examples: 'values.battery', 'temperature', 'humidity'"
                },
                "command": {
                    "type": "string",
                    "description": "Control command to send (control action). Common: 'turn_on', 'turn_off', 'set_value', 'toggle'. Examples: 'turn_on', 'set_value'"
                },
                "params": {
                    "type": "object",
                    "description": "Control parameters as key-value pairs (control action, optional). Example: {\"value\": 26, \"unit\": \"celsius\"}"
                },
                "device_type": {
                    "type": "string",
                    "description": "Filter by device type (list action). Examples: 'sensor', 'switch', 'light', 'camera'"
                },
                "include_details": {
                    "type": "boolean",
                    "description": "Include metrics and commands info in list output (list action, default: true)"
                },
                "start_time": {
                    "type": "number",
                    "description": "Start timestamp in seconds for time range query (query action, default: 1 hour ago). Example: 1712000000"
                },
                "end_time": {
                    "type": "number",
                    "description": "End timestamp in seconds for time range query (query action, default: now)"
                },
                "limit": {
                    "type": "number",
                    "description": "Max number of results to return (default: 100 for query, unlimited for list)"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': key info only (default). 'detailed': full data with IDs and metadata for follow-up chained calls"
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Set to true after user confirms. Required for control action. Without confirmation, the tool returns a preview instead of executing"
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
    /// Check if response_format is "detailed".
    fn is_detailed(args: &Value) -> bool {
        args["response_format"].as_str() == Some("detailed")
    }

    /// Resolve device_id with fuzzy matching support.
    ///
    /// Tries exact match first, then falls back to case-insensitive
    /// substring matching on device_id or name.
    async fn resolve_device_id(&self, device_id: &str) -> Result<String> {
        // 1. Try exact match first
        if self.device_service.get_device(device_id).await.is_some() {
            return Ok(device_id.to_string());
        }

        // 2. Fuzzy match by id or name (case-insensitive substring)
        let devices = self.device_service.list_devices().await;
        let candidates: Vec<_> = devices
            .iter()
            .filter(|d| {
                d.device_id
                    .to_lowercase()
                    .contains(&device_id.to_lowercase())
                    || d.name.to_lowercase().contains(&device_id.to_lowercase())
            })
            .collect();

        match candidates.len() {
            0 => Err(ToolError::Execution(format!(
                "未找到设备 '{}'。请先调用 device(action: 'list') 查看可用设备。",
                device_id
            ))),
            1 => Ok(candidates[0].device_id.clone()),
            _ => {
                let device_list: Vec<String> = candidates
                    .iter()
                    .map(|d| format!("{} ({})", d.name, d.device_id))
                    .collect();
                Err(ToolError::Execution(format!(
                    "找到多个匹配 '{}' 的设备，请指定更明确的名称: {}",
                    device_id,
                    device_list.join(", ")
                )))
            }
        }
    }

    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let devices = self.device_service.list_devices().await;
        let device_type_filter = args["device_type"].as_str();
        let detailed = Self::is_detailed(args);

        let mut result = Vec::new();

        for d in devices.iter() {
            // Apply device_type filter
            if let Some(dt) = device_type_filter {
                if d.device_type != dt {
                    continue;
                }
            }

            // Concise mode: name/type only
            if !detailed {
                result.push(serde_json::json!({
                    "id": d.device_id,
                    "name": d.name,
                    "type": d.device_type,
                }));
                continue;
            }

            // Detailed mode: full info with metrics and commands
            let mut device_json = serde_json::json!({
                "id": d.device_id,
                "name": d.name,
                "type": d.device_type,
                "adapter_type": d.adapter_type
            });

            if let Some(template) = self.device_service.get_template(&d.device_type).await {
                if !template.metrics.is_empty() {
                    let metrics_info: Vec<Value> = template
                        .metrics
                        .iter()
                        .map(|m| {
                            serde_json::json!({
                                "name": m.name,
                                "display_name": m.display_name,
                                "unit": m.unit,
                                "data_type": format!("{:?}", m.data_type)
                            })
                        })
                        .collect();
                    device_json["metrics"] = serde_json::json!(metrics_info);
                }

                if !template.commands.is_empty() {
                    let commands_info: Vec<Value> = template
                        .commands
                        .iter()
                        .map(|c| {
                            serde_json::json!({
                                "name": c.name,
                                "display_name": c.display_name,
                                "parameters": c.parameters.iter().map(|p| {
                                    serde_json::json!({
                                        "name": p.name,
                                        "display_name": p.display_name,
                                        "data_type": format!("{:?}", p.data_type),
                                        "required": p.required
                                    })
                                }).collect::<Vec<_>>()
                            })
                        })
                        .collect();
                    device_json["commands"] = serde_json::json!(commands_info);
                }
            }

            result.push(device_json);
        }

        let limit = args["limit"].as_u64().unwrap_or(result.len() as u64) as usize;
        let result: Vec<_> = result.into_iter().take(limit).collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": result.len(),
            "devices": result
        })))
    }

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let device_id_input = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id is required".into()))?;

        let device_id = self.resolve_device_id(device_id_input).await?;

        let device = self
            .device_service
            .get_device(&device_id)
            .await
            .ok_or_else(|| ToolError::Execution(format!("Device not found: {}", device_id)))?;

        if Self::is_detailed(args) {
            Ok(ToolOutput::success(serde_json::json!({
                "id": device.device_id,
                "name": device.name,
                "type": device.device_type,
                "adapter_type": device.adapter_type,
                "adapter_id": device.adapter_id
            })))
        } else {
            Ok(ToolOutput::success(serde_json::json!({
                "name": device.name,
                "type": device.device_type
            })))
        }
    }

    async fn execute_query(&self, args: &Value) -> Result<ToolOutput> {
        let device_id_input = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id is required".into()))?;

        let device_id = self.resolve_device_id(device_id_input).await?;

        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ToolError::Execution("Storage not configured".into()))?;

        let metric = args["metric"].as_str();
        let end_time = args["end_time"]
            .as_i64()
            .unwrap_or_else(|| chrono::Utc::now().timestamp());
        let start_time = args["start_time"].as_i64().unwrap_or(end_time - 3600);
        let limit = args["limit"].as_u64().unwrap_or(100) as usize;

        if let Some(m) = metric {
            let data = storage
                .query(&device_id, m, start_time, end_time)
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

        // Confirmation check
        let confirm = args["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Ok(ToolOutput::success(serde_json::json!({
                "preview": true,
                "device_id": device_id,
                "command": command,
                "params": args.get("params").cloned().unwrap_or(serde_json::json!({})),
                "message": "This will change device state. Set confirm=true to execute."
            })));
        }

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
        r#"AI Agent management tool for creating and managing automated agents.

Actions:
- list: List all agents or filter by status (active/paused/stopped/error). Use when user asks about existing agents.
- get: Get agent details including config, schedule, and execution stats. Use when user asks about a specific agent.
- create: Create a new automated agent. Requires: name, user_prompt, schedule_type. Use when user wants to automate a monitoring or control task.
- update: Modify an existing agent's configuration (name, description, user_prompt). Use when user wants to change agent behavior.
- control: Pause or resume agent execution (control_action: pause/resume). WARNING: This affects running agents.
- memory: View agent's learned patterns and intent understanding. Use when debugging agent behavior.

When creating agents:
- schedule_type: 'event' (triggered by device events), 'cron' (cron schedule), 'interval' (periodic, e.g., every 5 minutes)
- user_prompt should be specific, e.g., 'Check temperature every 5 minutes, alert if above 30C'
- Use response_format="detailed" to get full agent config including IDs"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "create", "update", "control", "memory"],
                    "description": "Operation type: 'list' (all agents), 'get' (agent details), 'create' (new agent), 'update' (modify agent), 'control' (pause/resume), 'memory' (view learned patterns)"
                },
                "agent_id": {
                    "type": "string",
                    "description": "Agent ID. Required for get/update/control/memory actions. Example: 'agent_1', or use the ID returned from list action"
                },
                "name": {
                    "type": "string",
                    "description": "Agent display name. Required for create, optional for update. Example: 'Temperature Monitor', 'Security Patrol'"
                },
                "description": {
                    "type": "string",
                    "description": "Agent description. Optional for create/update. Example: 'Monitors living room temperature and alerts on threshold breach'"
                },
                "user_prompt": {
                    "type": "string",
                    "description": "Natural language description of what the agent should do. Required for create, optional for update. Be specific: 'Check ne101 temperature every 5 minutes, alert if above 30C'"
                },
                "schedule_type": {
                    "type": "string",
                    "description": "How the agent is triggered (create action): 'event' (device data changes), 'cron' (time schedule), 'interval' (periodic execution)"
                },
                "schedule_config": {
                    "type": "string",
                    "description": "Schedule configuration (create action). For cron: cron expression like '*/5 * * * *'. For interval: seconds like '300'"
                },
                "control_action": {
                    "type": "string",
                    "description": "Control operation (control action): 'pause' (stop execution temporarily), 'resume' (restart paused agent)"
                },
                "status": {
                    "type": "string",
                    "description": "Filter by agent status (list action): 'active', 'paused', 'stopped', 'error', 'executing'"
                },
                "limit": {
                    "type": "number",
                    "description": "Max results to return (list/memory actions). Default: 20"
                },
                "memory_type": {
                    "type": "string",
                    "description": "Type of memory to retrieve (memory action): 'patterns' (learned patterns, default), 'intents' (parsed intent). Default: 'patterns'"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': name/status/schedule only (default). 'detailed': full config with IDs and metadata"
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Set to true after user confirms. Required for control action. Without confirmation, returns a preview"
                }
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
    fn is_detailed(args: &Value) -> bool {
        args["response_format"].as_str() == Some("detailed")
    }

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
                if Self::is_detailed(args) {
                    serde_json::json!({
                        "id": a.id,
                        "name": a.name,
                        "description": a.description,
                        "status": format!("{:?}", a.status).to_lowercase(),
                        "schedule_type": format!("{:?}", a.schedule.schedule_type).to_lowercase()
                    })
                } else {
                    serde_json::json!({
                        "id": a.id,
                        "name": a.name,
                        "status": format!("{:?}", a.status).to_lowercase(),
                        "schedule_type": format!("{:?}", a.schedule.schedule_type).to_lowercase()
                    })
                }
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
            execution_mode: Default::default(),
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

        // Confirmation check
        let confirm = args["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Ok(ToolOutput::success(serde_json::json!({
                "preview": true,
                "agent_id": agent_id,
                "control_action": control_action,
                "message": "This will change agent execution state. Set confirm=true to execute."
            })));
        }

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
        r#"Agent execution history tool for reviewing agent performance and conversation logs.

Actions:
- executions: View agent execution statistics (total runs, success rate, last execution time). Use when user asks about agent performance or reliability.
- conversation: View agent's conversation history (inputs and outputs from past runs). Use when debugging agent behavior or reviewing what an agent did.

Use cases:
- Check if an agent is running correctly: executions action
- Debug why an agent made a decision: conversation action
- Review recent agent activity: conversation with limit"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["executions", "conversation"],
                    "description": "Operation type: 'executions' (execution statistics), 'conversation' (conversation log)"
                },
                "agent_id": {
                    "type": "string",
                    "description": "Agent ID to query. Required. Use agent(action='list') to find available agent IDs"
                },
                "limit": {
                    "type": "number",
                    "description": "Max number of conversation entries to return (conversation action). Default: 50"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': summary stats only (default). 'detailed': full execution details with timestamps"
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
        r#"Rule management tool for automation rules that trigger actions based on device conditions.

Actions:
- list: List all rules or filter by name. Use when user asks about existing automation rules.
- get: Get rule details including full DSL definition. Use when user asks about a specific rule's logic.
- create: Create a new automation rule from DSL. Requires: dsl. Use when user wants to automate a response to device conditions.
- update: Replace a rule's DSL definition. WARNING: Deletes old rule and creates new one. Requires: rule_id, dsl.
- delete: Permanently remove a rule. WARNING: This is irreversible. Requires confirmation.
- history: View rule execution history (when rules triggered and what they did).

Rule DSL format:
RULE "rule_name" WHEN device_id.metric OPERATOR value DO ACTION END
Example: RULE "Low Battery Alert" WHEN ne101.battery < 50 DO NOTIFY "Battery below 50%" END

Operators: <, >, <=, >=, ==, !=
Actions: NOTIFY (send alert), CONTROL (send device command)"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "create", "update", "delete", "history"],
                    "description": "Operation type: 'list' (all rules), 'get' (rule details), 'create' (new rule), 'update' (modify rule), 'delete' (remove rule), 'history' (execution log)"
                },
                "rule_id": {
                    "type": "string",
                    "description": "Rule ID. Required for get/update/delete. Use list action first to find the rule ID"
                },
                "dsl": {
                    "type": "string",
                    "description": "Rule DSL definition. Required for create/update. Format: RULE \"name\" WHEN device.metric OP value DO ACTION END. Example: RULE \"Low Battery\" WHEN ne101.battery < 50 DO NOTIFY \"Battery low\" END"
                },
                "name_filter": {
                    "type": "string",
                    "description": "Filter rules by name substring (list action). Example: 'battery', 'temperature'"
                },
                "limit": {
                    "type": "number",
                    "description": "Max results to return. Default: 100"
                },
                "start_time": {
                    "type": "number",
                    "description": "Start timestamp for history range (history action). Unix timestamp in seconds"
                },
                "end_time": {
                    "type": "number",
                    "description": "End timestamp for history range (history action). Unix timestamp in seconds"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': name/status only (default). 'detailed': full DSL and metadata"
                },
                "confirm": {
                    "type": "boolean",
                    "description": "Set to true after user confirms. Required for delete and update actions. Without confirmation, returns a preview instead of executing"
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
    fn is_detailed(args: &Value) -> bool {
        args["response_format"].as_str() == Some("detailed")
    }

    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let rules = self.rule_engine.list_rules().await;

        let name_filter = args["name_filter"].as_str();
        let limit = args["limit"].as_u64().unwrap_or(100) as usize;
        let detailed = Self::is_detailed(args);

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
                if detailed {
                    serde_json::json!({
                        "id": r.id.to_string(),
                        "name": r.name,
                        "description": r.description,
                        "dsl": r.dsl
                    })
                } else {
                    serde_json::json!({
                        "id": r.id.to_string(),
                        "name": r.name,
                        "description": r.description
                    })
                }
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

        // Confirmation check
        let confirm = args["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Ok(ToolOutput::success(serde_json::json!({
                "preview": true,
                "rule_id": rule_id_str,
                "message": "This will permanently delete the rule. Set confirm=true to execute."
            })));
        }

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

        // Confirmation check
        let confirm = args["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Ok(ToolOutput::success(serde_json::json!({
                "preview": true,
                "rule_id": rule_id_str,
                "new_dsl": dsl,
                "message": "This will replace the rule definition. Set confirm=true to execute."
            })));
        }

        let rule_id = neomind_rules::RuleId::from_string(rule_id_str)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid rule_id: {}", e)))?;

        // First remove the old rule to clean up dependencies
        self.rule_engine
            .remove_rule(&rule_id)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to remove old rule: {}", e)))?;

        // Parse and add the new rule
        let new_rule_id =
            self.rule_engine.add_rule_from_dsl(dsl).await.map_err(|e| {
                ToolError::Execution(format!("Failed to create updated rule: {}", e))
            })?;

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
        r#"Alert management tool for viewing and managing system alerts and notifications.

Actions:
- list: List alerts with optional severity and acknowledgment filters. Use when user asks about current or recent alerts.
- create: Create a new alert manually. Use when user wants to flag something or when an agent needs to notify the user.
- acknowledge: Mark an alert as acknowledged/resolved. Use when user confirms they've seen an alert.

Severity levels:
- info: Informational, no action needed (e.g., 'Device came online')
- warning: Attention recommended (e.g., 'Battery below 30%')
- error: Action needed (e.g., 'Device communication failed')
- critical: Immediate action required (e.g., 'Temperature exceeds safety limit')

Tips:
- Use unacknowledged_only=true to see only active alerts
- Filter by severity to prioritize critical issues"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "create", "acknowledge"],
                    "description": "Operation type: 'list' (view alerts), 'create' (new alert), 'acknowledge' (mark as resolved)"
                },
                "alert_id": {
                    "type": "string",
                    "description": "Alert ID to acknowledge (acknowledge action). Use list action first to find the alert ID"
                },
                "title": {
                    "type": "string",
                    "description": "Alert title (create action). Short summary. Example: 'High Temperature Alert', 'Device Offline'"
                },
                "message": {
                    "type": "string",
                    "description": "Alert message body (create action). Detailed description. Example: 'Living room sensor reports 35.2C, threshold is 30C'"
                },
                "severity": {
                    "type": "string",
                    "description": "Alert severity level (create action): 'info', 'warning', 'error', 'critical'. Default: 'warning'"
                },
                "source": {
                    "type": "string",
                    "description": "Alert source identifier (create action). Default: 'system'. Example: 'temperature_monitor', 'security_agent'"
                },
                "unacknowledged_only": {
                    "type": "boolean",
                    "description": "Only return unacknowledged alerts (list action). Default: false"
                },
                "severity_filter": {
                    "type": "string",
                    "description": "Filter by severity (list action): 'info', 'warning', 'error', 'critical'"
                },
                "limit": {
                    "type": "number",
                    "description": "Max alerts to return (list action). Default: 50"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': title/severity/status only (default). 'detailed': full alert info with timestamps"
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
    fn is_detailed(args: &Value) -> bool {
        args["response_format"].as_str() == Some("detailed")
    }

    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let unacknowledged_only = args["unacknowledged_only"].as_bool().unwrap_or(false);
        let severity_filter = args["severity_filter"].as_str();
        let limit = args["limit"].as_u64().unwrap_or(50) as usize;
        let detailed = Self::is_detailed(args);

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
                    if detailed {
                        serde_json::json!({
                            "id": m.id.to_string(),
                            "title": m.title,
                            "message": m.message,
                            "severity": severity_str,
                            "source": m.source_type,
                            "acknowledged": !m.is_active(),
                            "created_at": m.timestamp.timestamp()
                        })
                    } else {
                        serde_json::json!({
                            "id": m.id.to_string(),
                            "title": m.title,
                            "severity": severity_str,
                            "acknowledged": !m.is_active()
                        })
                    }
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
                .map(|a| {
                    if detailed {
                        serde_json::to_value(a).unwrap()
                    } else {
                        serde_json::json!({
                            "id": a.id,
                            "title": a.title,
                            "severity": format!("{:?}", a.severity).to_lowercase(),
                            "acknowledged": a.acknowledged
                        })
                    }
                })
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

            let msg = manager
                .create_message(msg)
                .await
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

            manager
                .acknowledge(&alert_id)
                .await
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
