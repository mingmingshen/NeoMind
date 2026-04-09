//! Aggregated tools using action-based design pattern.
//!
//! This module consolidates 34+ individual tools into 6 aggregated tools,
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
//! 6. `extension` - Extension management (list, get, execute, status)

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

    /// Resolve device_id with fuzzy matching support using the generic EntityResolver.
    async fn resolve_device_id(&self, device_id: &str) -> Result<String> {
        // Fast path: exact ID match without listing all devices
        if self.device_service.get_device(device_id).await.is_some() {
            return Ok(device_id.to_string());
        }

        // Slow path: fuzzy match via resolver
        let devices = self.device_service.list_devices().await;
        let candidates: Vec<(String, String)> = devices
            .iter()
            .map(|d| (d.device_id.clone(), d.name.clone()))
            .collect();

        super::resolver::EntityResolver::resolve(device_id, &candidates, "设备")
            .map_err(ToolError::Execution)
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

        let detailed = Self::is_detailed(args);

        let mut device_json = if detailed {
            serde_json::json!({
                "id": device.device_id,
                "name": device.name,
                "type": device.device_type,
                "adapter_type": device.adapter_type,
                "adapter_id": device.adapter_id
            })
        } else {
            serde_json::json!({
                "id": device.device_id,
                "name": device.name,
                "type": device.device_type
            })
        };

        // Enrich with metrics and commands from device type template
        if let Some(template) = self.device_service.get_template(&device.device_type).await {
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

        Ok(ToolOutput::success(device_json))
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
            let mut data = storage
                .query(&device_id, m, start_time, end_time)
                .await
                .map_err(|e| ToolError::Execution(e.to_string()))?;

            let mut resolved_metric = m.to_string();

            // If no data found, try to resolve metric name from device template
            // This handles cases where user passes "battery" but storage key is "values.battery"
            if data.is_empty() {
                if let Some(device) = self.device_service.get_device(&device_id).await {
                    if let Some(template) = self.device_service.get_template(&device.device_type).await {
                        // Try to find a metric whose name ends with the user input
                        // e.g., "battery" matches "values.battery"
                        let m_lower = m.to_lowercase();
                        if let Some(matched_def) = template.metrics.iter().find(|def| {
                            def.name == m || def.name.ends_with(&format!(".{}", m)) || def.name == m_lower
                        }) {
                            let resolved = matched_def.name.clone();
                            if let Ok(d) = storage
                                .query(&device_id, &resolved, start_time, end_time)
                                .await
                            {
                                data = d;
                                resolved_metric = resolved;
                            }
                        }
                    }
                }
            }

            Ok(ToolOutput::success(serde_json::json!({
                "device_id": device_id,
                "metric": resolved_metric,
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
- send_message: Send a message or instruction to the agent. The agent will see it in its next execution. Use when user wants to guide, correct, or update an agent's behavior through natural language.

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
                    "enum": ["list", "get", "create", "update", "control", "memory", "send_message"],
                    "description": "Operation type: 'list' (all agents), 'get' (agent details), 'create' (new agent), 'update' (modify agent), 'control' (pause/resume), 'memory' (view learned patterns), 'send_message' (send message to agent)"
                },
                "agent_id": {
                    "type": "string",
                    "description": "Agent ID or name. Supports fuzzy matching (e.g., 'Temperature Monitor' matches by name). Use list action to discover available agents. Examples: '550e8400-...', 'Temperature Monitor'"
                },
                "message": {
                    "type": "string",
                    "description": "Message content to send to the agent (send_message action). The agent will see this in its next execution. Example: 'Focus on monitoring the front door area'"
                },
                "message_type": {
                    "type": "string",
                    "description": "Optional message type/tag for categorization (send_message action). Example: 'instruction', 'correction', 'update'"
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
            "send_message" => self.execute_send_message(&args).await,
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

    /// Resolve agent_id with fuzzy matching using EntityResolver.
    async fn resolve_agent_id(&self, input: &str) -> Result<String> {
        // Fast path: exact ID match
        if let Ok(Some(_)) = self.agent_store.get_agent(input).await {
            return Ok(input.to_string());
        }

        // Slow path: fuzzy match by name
        let agents = self
            .agent_store
            .query_agents(neomind_storage::agents::AgentFilter::default())
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        let candidates: Vec<(String, String)> =
            agents.iter().map(|a| (a.id.clone(), a.name.clone())).collect();

        super::resolver::EntityResolver::resolve(input, &candidates, "agent")
            .map_err(ToolError::Execution)
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
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let resolved_id = self.resolve_agent_id(agent_id_input).await?;

        let agent = self
            .agent_store
            .get_agent(&resolved_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", resolved_id)))?;

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
            max_retries: 0,
            consecutive_failures: 0,
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
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let resolved_id = self.resolve_agent_id(agent_id_input).await?;

        let mut agent = self
            .agent_store
            .get_agent(&resolved_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| ToolError::Execution(format!("Agent not found: {}", resolved_id)))?;

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
            "id": resolved_id,
            "status": "updated"
        })))
    }

    async fn execute_control(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent_id = self.resolve_agent_id(agent_id_input).await?;

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
            .update_agent_status(&agent_id, status, error_msg)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": agent_id,
            "action": control_action,
            "status": "success"
        })))
    }

    async fn execute_memory(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent_id = self.resolve_agent_id(agent_id_input).await?;

        let agent = self
            .agent_store
            .get_agent(&agent_id)
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

    async fn execute_send_message(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent_id = self.resolve_agent_id(agent_id_input).await?;

        let content = args["message"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("message is required".into()))?;

        let message_type = args["message_type"].as_str().map(|s| s.to_string());

        let user_msg = self
            .agent_store
            .add_user_message(&agent_id, content.to_string(), message_type)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "agent_id": agent_id,
            "message_id": user_msg.id,
            "status": "delivered",
            "note": "Message will be included in the agent's next execution context"
        })))
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
- latest_execution: View the most recent execution with full details (analysis, reasoning, decisions, conclusion). Use when user asks about execution results or completion status.

Use cases:
- Check if an agent is running correctly: executions action
- Debug why an agent made a decision: conversation action
- Review recent agent activity: conversation with limit
- Check last execution result or success/failure: latest_execution action"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["executions", "conversation", "latest_execution"],
                    "description": "Operation type: 'executions' (execution statistics), 'conversation' (conversation log), 'latest_execution' (most recent execution with full details)"
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
            "latest_execution" => self.execute_latest_execution(&args).await,
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

    async fn execute_latest_execution(&self, args: &Value) -> Result<ToolOutput> {
        let agent_id_input = args["agent_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("agent_id is required".into()))?;

        let agent = self
            .agent_store
            .get_agent(agent_id_input)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .ok_or_else(|| {
                ToolError::Execution(format!("Agent not found: {}", agent_id_input))
            })?;

        let last_turn = agent.conversation_history.first();

        let stats_summary = serde_json::json!({
            "total_executions": agent.stats.total_executions,
            "successful_executions": agent.stats.successful_executions,
            "failed_executions": agent.stats.failed_executions,
            "success_rate": if agent.stats.total_executions > 0 {
                format!("{:.0}%", (agent.stats.successful_executions as f64 / agent.stats.total_executions as f64) * 100.0)
            } else {
                "N/A".to_string()
            },
            "last_duration_ms": last_turn.map(|t| t.duration_ms),
        });

        let last_execution = last_turn.map(|turn| {
            serde_json::json!({
                "execution_id": turn.execution_id,
                "timestamp": turn.timestamp,
                "trigger_type": turn.trigger_type,
                "success": turn.success,
                "duration_ms": turn.duration_ms,
                "input": turn.input,
                "output": turn.output
            })
        });

        Ok(ToolOutput::success(serde_json::json!({
            "agent_id": agent_id_input,
            "agent_name": agent.name,
            "agent_status": format!("{:?}", agent.status).to_lowercase(),
            "last_execution": last_execution,
            "pending_user_messages": agent.user_messages.len(),
            "stats_summary": stats_summary
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
                    "description": "Rule ID or name. Supports fuzzy matching (e.g., 'Low Battery Alert' matches by name). Use list action to discover available rules"
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

    /// Resolve rule_id with fuzzy matching using EntityResolver.
    async fn resolve_rule_id(&self, input: &str) -> Result<String> {
        // Try exact parse first
        if let Ok(rule_id) = neomind_rules::RuleId::from_string(input) {
            if self.rule_engine.get_rule(&rule_id).await.is_some() {
                return Ok(input.to_string());
            }
        }

        // Fuzzy match by name
        let rules = self.rule_engine.list_rules().await;
        let candidates: Vec<(String, String)> = rules
            .iter()
            .map(|r| (r.id.to_string(), r.name.clone()))
            .collect();

        super::resolver::EntityResolver::resolve(input, &candidates, "规则")
            .map_err(ToolError::Execution)
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
        let rule_id_input = args["rule_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("rule_id is required".into()))?;

        let resolved_id = self.resolve_rule_id(rule_id_input).await?;

        let rule_id = neomind_rules::RuleId::from_string(&resolved_id)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid rule_id: {}", e)))?;

        let rule = self
            .rule_engine
            .get_rule(&rule_id)
            .await
            .ok_or_else(|| ToolError::Execution(format!("Rule not found: {}", resolved_id)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": rule.id.to_string(),
            "name": rule.name,
            "description": rule.description,
            "dsl": rule.dsl
        })))
    }

    async fn execute_delete(&self, args: &Value) -> Result<ToolOutput> {
        let rule_id_input = args["rule_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("rule_id is required".into()))?;

        let resolved_id = self.resolve_rule_id(rule_id_input).await?;

        // Confirmation check
        let confirm = args["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Ok(ToolOutput::success(serde_json::json!({
                "preview": true,
                "rule_id": resolved_id,
                "message": "This will permanently delete the rule. Set confirm=true to execute."
            })));
        }

        let rule_id = neomind_rules::RuleId::from_string(&resolved_id)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid rule_id: {}", e)))?;

        self.rule_engine
            .remove_rule(&rule_id)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolOutput::success(serde_json::json!({
            "id": resolved_id,
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
        let rule_id_input = args["rule_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("rule_id is required".into()))?;

        let dsl = args["dsl"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("dsl is required".into()))?;

        let resolved_id = self.resolve_rule_id(rule_id_input).await?;

        // Confirmation check
        let confirm = args["confirm"].as_bool().unwrap_or(false);
        if !confirm {
            return Ok(ToolOutput::success(serde_json::json!({
                "preview": true,
                "rule_id": resolved_id,
                "new_dsl": dsl,
                "message": "This will replace the rule definition. Set confirm=true to execute."
            })));
        }

        let rule_id = neomind_rules::RuleId::from_string(&resolved_id)
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
            "old_id": resolved_id,
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
- get: Get a single alert by ID. Use when you need full details of a specific alert.
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
                    "enum": ["list", "get", "create", "acknowledge"],
                    "description": "Operation type: 'list' (view alerts), 'get' (single alert by ID), 'create' (new alert), 'acknowledge' (mark as resolved)"
                },
                "alert_id": {
                    "type": "string",
                    "description": "Alert ID or title. Supports fuzzy matching on title (e.g., 'High Temperature' matches). Use list action to discover alerts"
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
            "get" => self.execute_get(&args).await,
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

    /// Resolve alert_id with fuzzy matching using EntityResolver.
    async fn resolve_alert_id(&self, input: &str) -> Result<String> {
        let alerts = self.alerts.read().await;

        // Fast path: exact ID match
        let exact_match = alerts.iter().find(|a| a.id == input);
        if let Some(alert) = exact_match {
            return Ok(alert.id.clone());
        }

        // Fuzzy match by title
        let candidates: Vec<(String, String)> = alerts
            .iter()
            .map(|a| (a.id.clone(), a.title.clone()))
            .collect();

        super::resolver::EntityResolver::resolve(input, &candidates, "告警")
            .map_err(ToolError::Execution)
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

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let alert_id_str = args["alert_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("alert_id is required".into()))?;

        // Resolve alert_id (supports fuzzy matching by title)
        let resolved_id = self.resolve_alert_id(alert_id_str).await?;

        // Use message_manager if available
        if let Some(manager) = &self.message_manager {
            use neomind_messages::MessageId;

            let alert_id = MessageId::from_string(&resolved_id)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid alert_id: {}", e)))?;

            let message = manager
                .get_message(&alert_id)
                .await
                .ok_or_else(|| ToolError::Execution("Alert not found".into()))?;

            let severity_str = match message.severity {
                neomind_messages::MessageSeverity::Info => "info",
                neomind_messages::MessageSeverity::Warning => "warning",
                neomind_messages::MessageSeverity::Critical => "critical",
                neomind_messages::MessageSeverity::Emergency => "emergency",
            };

            Ok(ToolOutput::success(serde_json::json!({
                "id": message.id.to_string(),
                "title": message.title,
                "message": message.message,
                "severity": severity_str,
                "source": message.source_type,
                "acknowledged": !message.is_active(),
                "created_at": message.timestamp.timestamp()
            })))
        } else {
            let alerts = self.alerts.read().await;
            let alert = alerts
                .iter()
                .find(|a| a.id == resolved_id)
                .ok_or_else(|| ToolError::Execution("Alert not found".into()))?;

            Ok(ToolOutput::success(serde_json::json!({
                "id": alert.id,
                "title": alert.title,
                "message": alert.message,
                "severity": format!("{:?}", alert.severity).to_lowercase(),
                "source": alert.source,
                "acknowledged": alert.acknowledged,
                "created_at": alert.created_at
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

        // Resolve alert_id (supports fuzzy matching by title)
        let resolved_id = self.resolve_alert_id(alert_id_str).await?;

        // Use message_manager if available
        if let Some(manager) = &self.message_manager {
            use neomind_messages::MessageId;

            let alert_id = MessageId::from_string(&resolved_id)
                .map_err(|e| ToolError::InvalidArguments(format!("Invalid alert_id: {}", e)))?;

            manager
                .acknowledge(&alert_id)
                .await
                .map_err(|e| ToolError::Execution(e.to_string()))?;

            Ok(ToolOutput::success(serde_json::json!({
                "id": resolved_id,
                "status": "acknowledged"
            })))
        } else {
            let mut alerts = self.alerts.write().await;
            let alert = alerts
                .iter_mut()
                .find(|a| a.id == resolved_id)
                .ok_or_else(|| ToolError::Execution("Alert not found".into()))?;

            alert.acknowledged = true;

            Ok(ToolOutput::success(serde_json::json!({
                "id": resolved_id,
                "status": "acknowledged"
            })))
        }
    }
}

// ============================================================================
// Extension Tool - Aggregates: list, get, execute, status
// ============================================================================

/// Aggregated extension tool with action-based routing.
///
/// Provides a unified entry point for interacting with all installed extensions,
/// replacing per-command tool registration (e.g., `weather-forecast-v2:get_weather`).
pub struct ExtensionAggregatedTool {
    registry: Arc<neomind_core::extension::registry::ExtensionRegistry>,
}

impl ExtensionAggregatedTool {
    /// Create a new extension aggregated tool.
    pub fn new(registry: Arc<neomind_core::extension::registry::ExtensionRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for ExtensionAggregatedTool {
    fn name(&self) -> &str {
        "extension"
    }

    fn description(&self) -> &str {
        r#"Extension management tool for interacting with installed extensions (plugins).

Actions:
- list: List all installed extensions with their status and command count. Use when user asks what extensions or plugins are available.
- get: Get detailed info about a specific extension, including its commands and metrics. Use before executing a command.
- execute: Execute a command on an extension. Requires extension_id, command name, and optional params.
- status: Check the health and runtime status of an extension.

Tips:
- Always call list first if you're unsure which extensions are available
- Use get to discover available commands before calling execute
- The params field should be a JSON object matching the command's expected parameters"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "action": {
                    "type": "string",
                    "enum": ["list", "get", "execute", "status"],
                    "description": "Operation type: 'list' (list extensions), 'get' (extension details), 'execute' (run command), 'status' (health check)"
                },
                "extension_id": {
                    "type": "string",
                    "description": "Extension ID or name. Supports fuzzy matching (e.g., 'weather' matches 'Weather Forecast'). Use list action to discover available extensions"
                },
                "command": {
                    "type": "string",
                    "description": "Command name to execute (execute action). Use get action to discover available commands. Example: 'get_weather'"
                },
                "params": {
                    "type": "object",
                    "description": "Command parameters as JSON object (execute action). Example: {\"city\": \"Beijing\"}"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["concise", "detailed"],
                    "description": "Output format. 'concise': summary only (default). 'detailed': full info with all metadata"
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
            "get" => self.execute_get(&args).await,
            "execute" => self.execute_command(&args).await,
            "status" => self.execute_status(&args).await,
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: {}",
                action
            ))),
        }
    }
}

impl ExtensionAggregatedTool {
    fn is_detailed(args: &Value) -> bool {
        args["response_format"].as_str() == Some("detailed")
    }

    /// Resolve extension_id with fuzzy matching using EntityResolver.
    async fn resolve_extension_id(&self, input: &str) -> Result<String> {
        // Fast path: exact ID match
        if self.registry.get_info(input).await.is_some() {
            return Ok(input.to_string());
        }

        // Fuzzy match by name
        let extensions = self.registry.list().await;
        let candidates: Vec<(String, String)> = extensions
            .iter()
            .map(|e| (e.metadata.id.clone(), e.metadata.name.clone()))
            .collect();

        super::resolver::EntityResolver::resolve(input, &candidates, "扩展")
            .map_err(ToolError::Execution)
    }

    async fn execute_list(&self, args: &Value) -> Result<ToolOutput> {
        let detailed = Self::is_detailed(args);
        let extensions = self.registry.list().await;

        let items: Vec<Value> = extensions
            .iter()
            .map(|info| {
                if detailed {
                    serde_json::json!({
                        "id": info.metadata.id,
                        "name": info.metadata.name,
                        "version": info.metadata.version,
                        "description": info.metadata.description,
                        "state": format!("{:?}", info.state),
                        "commands_count": info.commands.len(),
                        "metrics_count": info.metrics.len()
                    })
                } else {
                    serde_json::json!({
                        "id": info.metadata.id,
                        "name": info.metadata.name,
                        "state": format!("{:?}", info.state),
                        "commands": info.commands.len()
                    })
                }
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": items.len(),
            "extensions": items
        })))
    }

    async fn execute_get(&self, args: &Value) -> Result<ToolOutput> {
        let raw_id = args["extension_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("extension_id is required".into()))?;
        let extension_id = self.resolve_extension_id(raw_id).await?;

        let info = self
            .registry
            .get_info(&extension_id)
            .await
            .ok_or_else(|| ToolError::Execution(format!("Extension '{}' not found", extension_id)))?;

        let commands: Vec<Value> = info
            .commands
            .iter()
            .map(|cmd| {
                serde_json::json!({
                    "name": cmd.name,
                    "display_name": cmd.display_name,
                    "description": cmd.description,
                    "params": cmd.parameters
                })
            })
            .collect();

        let metrics: Vec<Value> = info
            .metrics
            .iter()
            .map(|m| {
                serde_json::json!({
                    "name": m.name,
                    "display_name": m.display_name,
                    "unit": m.unit
                })
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "id": info.metadata.id,
            "name": info.metadata.name,
            "version": info.metadata.version,
            "description": info.metadata.description,
            "state": format!("{:?}", info.state),
            "commands": commands,
            "metrics": metrics
        })))
    }

    async fn execute_command(&self, args: &Value) -> Result<ToolOutput> {
        let raw_id = args["extension_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("extension_id is required".into()))?;
        let extension_id = self.resolve_extension_id(raw_id).await?;
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("command is required".into()))?;
        let params = args.get("params").cloned().unwrap_or(serde_json::json!({}));

        let result = self
            .registry
            .execute_command(&extension_id, command, &params)
            .await
            .map_err(|e| ToolError::Execution(format!("Extension command failed: {}", e)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "extension_id": extension_id,
            "command": command,
            "result": result
        })))
    }

    async fn execute_status(&self, args: &Value) -> Result<ToolOutput> {
        let raw_id = args["extension_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("extension_id is required".into()))?;
        let extension_id = self.resolve_extension_id(raw_id).await?;

        let info = self
            .registry
            .get_info(&extension_id)
            .await
            .ok_or_else(|| ToolError::Execution(format!("Extension '{}' not found", extension_id)))?;

        let healthy = self
            .registry
            .health_check(&extension_id)
            .await
            .unwrap_or(false);

        Ok(ToolOutput::success(serde_json::json!({
            "id": info.metadata.id,
            "name": info.metadata.name,
            "state": format!("{:?}", info.state),
            "healthy": healthy,
            "commands_executed": info.stats.commands_executed,
            "error_count": info.stats.error_count
        })))
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
    extension_registry: Option<Arc<neomind_core::extension::registry::ExtensionRegistry>>,
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
            extension_registry: None,
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

    /// Set extension registry for the extension aggregated tool.
    pub fn with_extension_registry(
        mut self,
        registry: Arc<neomind_core::extension::registry::ExtensionRegistry>,
    ) -> Self {
        self.extension_registry = Some(registry);
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

        // Extension tool
        if let Some(ext_reg) = self.extension_registry {
            tools.push(Arc::new(ExtensionAggregatedTool::new(ext_reg)));
        }

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
