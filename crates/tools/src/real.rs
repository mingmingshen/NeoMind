//! Real tool implementations using actual storage and device managers.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use base64::{Engine as _, engine::general_purpose};

use super::error::Result;
use super::tool::{Tool, ToolDefinition, ToolOutput, number_property, object_schema, string_property};
use super::error::ToolError;
use edge_ai_core::tools::{ToolExample, UsageScenario};

pub type ToolResult<T> = std::result::Result<T, ToolError>;

use edge_ai_devices::{DeviceService, TimeSeriesStorage};
use edge_ai_rules::RuleEngine;


/// Tool for querying time series data using real storage.
pub struct QueryDataTool {
    storage: Arc<TimeSeriesStorage>,
}

impl QueryDataTool {
    /// Create a new query data tool with real storage.
    pub fn new(storage: Arc<TimeSeriesStorage>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl Tool for QueryDataTool {
    fn name(&self) -> &str {
        "query_data"
    }

    fn description(&self) -> &str {
        r#"查询设备的历史时间序列数据。

## 使用场景
- 查询传感器的历史数据（如温度、湿度、压力等）
- 获取设备的实时数据点
- 分析设备数据趋势
- 生成数据报告

## 注意事项
- device_id 必须是系统中已注册的设备ID
- metric 名称通常是：temperature（温度）、humidity（湿度）、pressure（压力）等
- 如果不指定时间范围，默认返回最近24小时的数据
- 返回数据按时间戳升序排列"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "device_id": string_property("设备ID，例如：sensor_1, temp_sensor_02"),
                "metric": string_property("指标名称，例如：temperature（温度）、humidity（湿度）、pressure（压力）"),
                "start_time": number_property("起始时间戳（Unix时间戳，秒）。可选，默认为当前时间往前推24小时"),
                "end_time": number_property("结束时间戳（Unix时间戳，秒）。可选，默认为当前时间"),
            }),
            vec!["device_id".to_string(), "metric".to_string()],
        )
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: serde_json::json!({
                    "device_id": "sensor_1",
                    "metric": "temperature",
                    "start_time": 1735718400,
                    "end_time": 1735804800
                }),
                result: serde_json::json!({
                    "device_id": "sensor_1",
                    "metric": "temperature",
                    "count": 24,
                    "data": [
                        {"timestamp": 1735718400, "value": 22.5},
                        {"timestamp": 1735722000, "value": 23.1}
                    ]
                }),
                description: "查询传感器最近24小时的温度数据".to_string(),
            }),
            category: edge_ai_core::tools::ToolCategory::Data,
            scenarios: vec![],
            relationships: edge_ai_core::tools::ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![ToolExample {
                arguments: serde_json::json!({
                    "device_id": "sensor_1",
                    "metric": "temperature"
                }),
                result: serde_json::json!({
                    "device_id": "sensor_1",
                    "metric": "temperature",
                    "count": 24,
                    "data": [{"timestamp": 1735718400, "value": 22.5}]
                }),
                description: "查询设备指标数据".to_string(),
            }],
            response_format: Some("concise".to_string()),
            namespace: Some("data".to_string()),
        }
    }

    fn namespace(&self) -> Option<&str> {
        Some("data")
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        self.validate_args(&args)?;

        let device_id = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id must be a string".to_string()))?;

        let metric = args["metric"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("metric must be a string".to_string()))?;

        let end_time = args["end_time"]
            .as_i64()
            .unwrap_or_else(|| chrono::Utc::now().timestamp());

        let start_time = args["start_time"].as_i64().unwrap_or(end_time - 86400); // Default 24 hours

        // Query the data from real storage
        let data_points = self
            .storage
            .query(device_id, metric, start_time, end_time)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to query data: {}", e)))?;

        // Convert data points to the expected format
        let data: Vec<Value> = data_points
            .iter()
            .map(|p| {
                serde_json::json!({
                    "timestamp": p.timestamp,
                    "value": p.value.as_f64().unwrap_or(0.0),
                })
            })
            .collect();

        Ok(ToolOutput::success_with_metadata(
            serde_json::json!({
                "device_id": device_id,
                "metric": metric,
                "start_time": start_time,
                "end_time": end_time,
                "count": data.len(),
                "data": data
            }),
            serde_json::json!({
                "query_type": "time_series_range"
            }),
        ))
    }
}

/// Tool for controlling devices using real device service.
pub struct ControlDeviceTool {
    service: Arc<DeviceService>,
}

impl ControlDeviceTool {
    /// Create a new control device tool with real device service.
    pub fn new(service: Arc<DeviceService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for ControlDeviceTool {
    fn name(&self) -> &str {
        "control_device"
    }

    fn description(&self) -> &str {
        r#"向设备发送控制命令。

## 使用场景
- 开关设备控制（打开/关闭）
- 设置设备参数值
- 触发设备动作
- 修改设备工作模式

## 常用命令
- turn_on: 打开设备
- turn_off: 关闭设备
- set_value: 设置数值参数（需通过parameters传递value）
- toggle: 切换设备状态

## 注意事项
- 执行控制命令前应先确认设备在线状态
- 部分命令需要额外的参数（如set_value需要value参数）
- 控制命令执行是异步的，实际生效可能有延迟"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "device_id": string_property("要控制的设备ID，例如：actuator_1, switch_living_room"),
                "command": string_property("控制命令，例如：turn_on（打开）、turn_off（关闭）、set_value（设置值）"),
                "value": string_property("命令参数值（可选），对于set_value命令需要传递此参数，例如：25、true、auto")
            }),
            vec!["device_id".to_string(), "command".to_string()],
        )
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: serde_json::json!({
                    "device_id": "actuator_1",
                    "command": "turn_on"
                }),
                result: serde_json::json!({
                    "success": true,
                    "device_id": "actuator_1",
                    "command": "turn_on"
                }),
                description: "打开执行器设备".to_string(),
            }),
            category: edge_ai_core::tools::ToolCategory::Device,
            scenarios: vec![],
            relationships: edge_ai_core::tools::ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![
                ToolExample {
                    arguments: serde_json::json!({
                        "device_id": "actuator_1",
                        "command": "turn_on"
                    }),
                    result: serde_json::json!({
                        "success": true,
                        "device_id": "actuator_1",
                        "command": "turn_on"
                    }),
                    description: "打开设备".to_string(),
                },
                ToolExample {
                    arguments: serde_json::json!({
                        "device_id": "switch_living",
                        "command": "turn_off"
                    }),
                    result: serde_json::json!({
                        "success": true,
                        "device_id": "switch_living",
                        "command": "turn_off"
                    }),
                    description: "关闭设备".to_string(),
                },
                ToolExample {
                    arguments: serde_json::json!({
                        "device_id": "thermostat_1",
                        "command": "set_value",
                        "value": "22"
                    }),
                    result: serde_json::json!({
                        "success": true,
                        "device_id": "thermostat_1",
                        "command": "set_value",
                        "value": "22"
                    }),
                    description: "设置设备参数值".to_string(),
                },
            ],
            response_format: Some("concise".to_string()),
            namespace: Some("device".to_string()),
        }
    }

    fn namespace(&self) -> Option<&str> {
        Some("device")
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        self.validate_args(&args)?;

        let device_id = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id must be a string".to_string()))?;

        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("command must be a string".to_string()))?;

        // Extract parameters - DeviceService accepts HashMap<String, serde_json::Value>
        let mut params = std::collections::HashMap::new();

        // Check if "value" parameter exists (for set_value commands)
        if let Some(value) = args.get("value") {
            params.insert("value".to_string(), value.clone());
        }

        // Also check for "parameters" object
        if let Some(obj) = args.get("parameters").and_then(|v| v.as_object()) {
            for (key, val) in obj {
                params.insert(key.clone(), val.clone());
            }
        }

        // Send command to device using DeviceService
        self.service
            .send_command(device_id, command, params)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to send command: {}", e)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "status": "success",
            "device_id": device_id,
            "command": command,
            "message": "Command sent successfully"
        })))
    }
}

/// Tool for listing devices using real device service.
pub struct ListDevicesTool {
    service: Arc<DeviceService>,
}

impl ListDevicesTool {
    /// Create a new list devices tool with real device service.
    pub fn new(service: Arc<DeviceService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl Tool for ListDevicesTool {
    fn name(&self) -> &str {
        "list_devices"
    }

    fn description(&self) -> &str {
        r#"列出系统中所有已注册的设备。

## 使用场景
- 查看所有可用设备列表
- 按设备类型筛选设备
- 获取设备基本信息（ID、名称、类型）
- 检查设备在线状态

## 返回信息
- 设备ID：唯一标识符
- 设备名称：人类可读的名称
- 设备类型：sensor（传感器）、actuator（执行器）等
- 设备状态：online（在线）、offline（离线）

## 设备类型
- sensor: 传感器设备（温度、湿度、压力等）
- actuator: 执行器设备（开关、阀门、电机等）
- controller: 控制器设备
- gateway: 网关设备"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "filter_type": string_property("可选，按设备类型过滤。例如：sensor（传感器）、actuator（执行器）"),
            }),
            vec![],
        )
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: serde_json::json!({}),
                result: serde_json::json!({
                    "count": 3,
                    "devices": [
                        {"id": "sensor_1", "name": "温度传感器1", "type": "sensor", "status": "online"},
                        {"id": "actuator_1", "name": "开关1", "type": "actuator", "status": "online"},
                        {"id": "sensor_2", "name": "湿度传感器1", "type": "sensor", "status": "offline"}
                    ]
                }),
                description: "列出所有设备".to_string(),
            }),
            category: edge_ai_core::tools::ToolCategory::Device,
            scenarios: vec![],
            relationships: edge_ai_core::tools::ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![
                ToolExample {
                    arguments: serde_json::json!({}),
                    result: serde_json::json!({
                        "count": 3,
                        "devices": [
                            {"id": "sensor_1", "name": "温度传感器1", "type": "sensor", "status": "online"},
                            {"id": "actuator_1", "name": "开关1", "type": "actuator", "status": "online"}
                        ]
                    }),
                    description: "获取所有设备列表".to_string(),
                },
                ToolExample {
                    arguments: serde_json::json!({"filter_type": "sensor"}),
                    result: serde_json::json!({
                        "count": 2,
                        "devices": [
                            {"id": "sensor_1", "name": "温度传感器1", "type": "sensor", "status": "online"},
                            {"id": "sensor_2", "name": "湿度传感器1", "type": "sensor", "status": "online"}
                        ]
                    }),
                    description: "仅列出传感器设备".to_string(),
                },
            ],
            response_format: Some("concise".to_string()),
            namespace: Some("device".to_string()),
        }
    }

    fn namespace(&self) -> Option<&str> {
        Some("device")
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let configs = self.service.list_devices().await;

        // Apply filter if specified
        let filtered: Vec<_> = if let Some(filter_type) = args["filter_type"].as_str() {
            configs
                .into_iter()
                .filter(|d| d.device_type == filter_type)
                .collect()
        } else {
            configs
        };

        // Convert to simpler format
        let device_list: Vec<Value> = filtered
            .iter()
            .map(|d| {
                serde_json::json!({
                    "id": d.device_id,
                    "name": d.name,
                    "type": d.device_type,
                    "status": "unknown" // DeviceService doesn't track status yet
                })
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": device_list.len(),
            "devices": device_list
        })))
    }
}

/// Tool for creating rules using real rule engine.
pub struct CreateRuleTool {
    engine: Arc<RuleEngine>,
}

impl CreateRuleTool {
    /// Create a new create rule tool with real engine.
    pub fn new(engine: Arc<RuleEngine>) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl Tool for CreateRuleTool {
    fn name(&self) -> &str {
        "create_rule"
    }

    fn description(&self) -> &str {
        r#"创建一个新的自动化规则。

## DSL 语法格式（多行格式，每部分单独一行）
RULE "规则名称"
WHEN sensor.temperature > 50
FOR 5 minutes
DO NOTIFY "温度过高"
END

## 重要：DSL必须多行格式！
- RULE "名称" （第一行）
- WHEN 条件 （第二行）
- FOR 持续时间 （可选，第三行）
- DO 动作 （第四行）
- END （最后一行）

## 条件示例
- sensor.temperature > 50: 温度大于50
- device.humidity < 30: 湿度小于30
- sensor.value == 1: 值等于1

## 动作类型（每个动作一行）
- NOTIFY "消息": 发送通知
- EXECUTE device.command(param=value): 执行设备命令
- LOG info: 记录日志

## 完整示例
RULE "高温告警"
WHEN sensor.temperature > 35
FOR 5 minutes
DO NOTIFY "温度过高"
END"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "name": string_property("规则名称，简洁描述规则的功能"),
                "dsl": string_property("规则DSL定义，格式：RULE \"名称\" WHEN 条件 DO 动作 END")
            }),
            vec!["name".to_string(), "dsl".to_string()],
        )
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: serde_json::json!({
                    "name": "高温告警",
                    "dsl": "RULE \"高温告警\"\nWHEN sensor.temperature > 35\nFOR 5 minutes\nDO NOTIFY \"温度过高，请注意\"\nEND"
                }),
                result: serde_json::json!({
                    "rule_id": "rule_123",
                    "status": "created"
                }),
                description: "创建一个温度超过35度时触发告警的规则".to_string(),
            }),
            category: edge_ai_core::tools::ToolCategory::Rule,
            scenarios: vec![],
            relationships: edge_ai_core::tools::ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![ToolExample {
                arguments: serde_json::json!({
                    "name": "高温告警",
                    "dsl": "RULE \"高温告警\"\nWHEN sensor.temperature > 35\nFOR 5 minutes\nDO NOTIFY \"温度过高，请注意\"\nEND"
                }),
                result: serde_json::json!({
                    "rule_id": "rule_123",
                    "status": "created"
                }),
                description: "创建温度告警规则".to_string(),
            }],
            response_format: Some("concise".to_string()),
            namespace: Some("rule".to_string()),
        }
    }

    fn namespace(&self) -> Option<&str> {
        Some("rule")
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        self.validate_args(&args)?;

        let _name = args["name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("name must be a string".to_string()))?;

        let dsl = args["dsl"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("dsl must be a string".to_string()))?;

        let rule_id = self
            .engine
            .add_rule_from_dsl(dsl)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to create rule: {}", e)))?;

        Ok(ToolOutput::success(serde_json::json!({
            "rule_id": rule_id.to_string(),
            "status": "created"
        })))
    }
}

/// Tool for listing rules using real rule engine.
pub struct ListRulesTool {
    engine: Arc<RuleEngine>,
}

impl ListRulesTool {
    /// Create a new list rules tool with real engine.
    pub fn new(engine: Arc<RuleEngine>) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl Tool for ListRulesTool {
    fn name(&self) -> &str {
        "list_rules"
    }

    fn description(&self) -> &str {
        r#"列出系统中所有自动化规则。

## 使用场景
- 查看所有已创建的规则
- 检查规则的启用状态
- 查看规则的触发次数统计
- 管理和监控自动化规则

## 返回信息
- 规则ID：唯一标识符
- 规则名称：人类可读的名称
- 启用状态：是否正在运行
- 触发次数：规则被执行的次数统计"#
    }

    fn parameters(&self) -> Value {
        object_schema(serde_json::json!({}), vec![])
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: serde_json::json!({}),
                result: serde_json::json!({
                    "count": 2,
                    "rules": [
                        {"id": "rule_1", "name": "高温告警", "enabled": true, "trigger_count": 5},
                        {"id": "rule_2", "name": "低湿提醒", "enabled": true, "trigger_count": 2}
                    ]
                }),
                description: "列出所有自动化规则".to_string(),
            }),
            category: edge_ai_core::tools::ToolCategory::Rule,
            scenarios: vec![],
            relationships: edge_ai_core::tools::ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![ToolExample {
                arguments: serde_json::json!({}),
                result: serde_json::json!({
                    "count": 2,
                    "rules": [
                        {"id": "rule_1", "name": "高温告警", "enabled": true, "trigger_count": 5}
                    ]
                }),
                description: "获取所有规则列表".to_string(),
            }],
            response_format: Some("concise".to_string()),
            namespace: Some("rule".to_string()),
        }
    }

    fn namespace(&self) -> Option<&str> {
        Some("rule")
    }

    async fn execute(&self, _args: Value) -> Result<ToolOutput> {
        use edge_ai_rules::RuleStatus;

        let rules = self.engine.list_rules().await;

        let rule_list: Vec<Value> = rules
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id.to_string(),
                    "name": r.name,
                    "enabled": matches!(r.status, RuleStatus::Active),
                    "trigger_count": r.state.trigger_count,
                })
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": rule_list.len(),
            "rules": rule_list
        })))
    }
}

/// Tool for querying rule execution history.
pub struct QueryRuleHistoryTool {
    history: Arc<edge_ai_rules::RuleHistoryStorage>,
}

impl QueryRuleHistoryTool {
    /// Create a new query rule history tool.
    pub fn new(history: Arc<edge_ai_rules::RuleHistoryStorage>) -> Self {
        Self { history }
    }
}

#[async_trait]
impl Tool for QueryRuleHistoryTool {
    fn name(&self) -> &str {
        "query_rule_history"
    }

    fn description(&self) -> &str {
        r#"查询自动化规则的执行历史记录。

## 使用场景
- 查看规则的触发历史
- 分析规则执行成功率
- 排查规则执行失败原因
- 统计规则执行频率

## 返回信息
- 规则ID和名称
- 执行时间戳
- 执行是否成功
- 执行的动作数量
- 错误信息（如果失败）
- 执行耗时（毫秒）

## 筛选选项
- rule_id: 指定规则ID筛选
- limit: 限制返回条数，默认10条"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "rule_id": string_property("可选，按规则ID筛选历史记录"),
                "limit": number_property("可选，返回的最大条数，默认10条")
            }),
            vec![],
        )
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: serde_json::json!({
                    "rule_id": "rule_1",
                    "limit": 5
                }),
                result: serde_json::json!({
                    "count": 5,
                    "history": [
                        {"id": "h1", "rule_id": "rule_1", "rule_name": "高温告警", "success": true, "actions_executed": 1, "timestamp": 1735804800}
                    ]
                }),
                description: "查询指定规则的执行历史".to_string(),
            }),
            category: edge_ai_core::tools::ToolCategory::Data,
            scenarios: vec![],
            relationships: edge_ai_core::tools::ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![ToolExample {
                arguments: serde_json::json!({
                    "rule_id": "rule_1",
                    "limit": 5
                }),
                result: serde_json::json!({
                    "count": 5,
                    "history": [
                        {"id": "h1", "rule_id": "rule_1", "rule_name": "高温告警", "success": true, "actions_executed": 1, "timestamp": 1735804800}
                    ]
                }),
                description: "查询规则执行历史".to_string(),
            }],
            response_format: Some("concise".to_string()),
            namespace: Some("rule".to_string()),
        }
    }

    fn namespace(&self) -> Option<&str> {
        Some("rule")
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        use edge_ai_rules::HistoryFilter;

        let mut filter = HistoryFilter::new();

        if let Some(rule_id) = args["rule_id"].as_str() {
            filter = filter.with_rule_id(rule_id);
        }

        let limit = args["limit"].as_u64().unwrap_or(10);
        filter = filter.with_limit(limit as usize);

        let entries = self
            .history
            .query(&filter)
            .await
            .map_err(|e| ToolError::Execution(format!("Failed to query history: {}", e)))?;

        let history_list: Vec<Value> = entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "id": entry.id,
                    "rule_id": entry.rule_id,
                    "rule_name": entry.rule_name,
                    "success": entry.success,
                    "actions_executed": entry.actions_executed,
                    "error": entry.error,
                    "duration_ms": entry.duration_ms,
                    "timestamp": entry.timestamp.timestamp(),
                })
            })
            .collect();

        Ok(ToolOutput::success(serde_json::json!({
            "count": history_list.len(),
            "history": history_list
        })))
    }
}

/// Tool for getting all current device data (simplified interface).
///
/// This tool provides a simpler interface than query_data - it doesn't require
/// specifying a metric name and returns all available device data at once.
pub struct GetDeviceDataTool {
    service: Arc<DeviceService>,
    storage: Arc<TimeSeriesStorage>,
}

impl GetDeviceDataTool {
    /// Create a new get device data tool.
    pub fn new(service: Arc<DeviceService>, storage: Arc<TimeSeriesStorage>) -> Self {
        Self { service, storage }
    }
}

#[async_trait]
impl Tool for GetDeviceDataTool {
    fn name(&self) -> &str {
        "get_device_data"
    }

    fn description(&self) -> &str {
        r#"获取设备的所有当前数据（简化版查询）。

## 使用场景
- 查看设备的实时数据
- 获取设备所有指标的当前值
- 不需要知道具体指标名称，一次获取所有数据
- 快速了解设备状态

## 返回信息
- 设备ID和名称
- 所有可用的指标及其当前值
- 每个指标的数据类型和单位
- 数据时间戳

## 注意事项
- 此工具返回所有指标的当前值，不需要指定具体指标名称
- 如果设备离线或没有数据，会返回相应提示
- 数据来自最新的遥测记录"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "device_id": string_property("设备ID，例如：sensor_1, temp_sensor_02")
            }),
            vec!["device_id".to_string()],
        )
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: serde_json::json!({
                    "device_id": "sensor_1"
                }),
                result: serde_json::json!({
                    "device_id": "sensor_1",
                    "device_name": "温度传感器1",
                    "device_type": "DHT22",
                    "metrics": {
                        "temperature": {
                            "value": 25.3,
                            "unit": "°C",
                            "display_name": "温度",
                            "timestamp": 1735804800
                        },
                        "humidity": {
                            "value": 65,
                            "unit": "%",
                            "display_name": "湿度",
                            "timestamp": 1735804800
                        }
                    }
                }),
                description: "获取设备的所有当前数据".to_string(),
            }),
            category: edge_ai_core::tools::ToolCategory::Data,
            scenarios: vec![],
            relationships: edge_ai_core::tools::ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![ToolExample {
                arguments: serde_json::json!({
                    "device_id": "sensor_1"
                }),
                result: serde_json::json!({
                    "device_id": "sensor_1",
                    "metrics": {
                        "temperature": {"value": 25.3, "unit": "°C"}
                    }
                }),
                description: "获取设备数据".to_string(),
            }],
            response_format: Some("concise".to_string()),
            namespace: Some("data".to_string()),
        }
    }

    fn namespace(&self) -> Option<&str> {
        Some("data")
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        self.validate_args(&args)?;

        let device_id = args["device_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("device_id must be a string".to_string()))?;

        // Try to get device info first
        let (device_config, device_template) = self
            .service
            .get_device_with_template(device_id)
            .await
            .map_err(|e| ToolError::Execution(format!("Device not found: {}", e)))?;

        // Get current metrics for all defined metrics in template
        let mut metrics_data = serde_json::Map::new();

        if !device_template.metrics.is_empty() {
            // Template has defined metrics - get current values for each
            for metric_def in &device_template.metrics {
                let metric_name = &metric_def.name;

                // Try to get the latest value from storage
                if let Ok(Some(point)) = self.storage.latest(device_id, metric_name).await {
                    let value_json = match point.value {
                        edge_ai_devices::MetricValue::Float(v) => serde_json::json!(v),
                        edge_ai_devices::MetricValue::Integer(v) => serde_json::json!(v),
                        edge_ai_devices::MetricValue::String(ref v) => {
                            // Try to parse as JSON first
                            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(v) {
                                json_val
                            } else {
                                serde_json::json!(v)
                            }
                        }
                        edge_ai_devices::MetricValue::Boolean(v) => serde_json::json!(v),
                        edge_ai_devices::MetricValue::Array(ref a) => {
                            // Convert array to JSON
                            let json_arr: Vec<serde_json::Value> = a.iter().map(|v| match v {
                                edge_ai_devices::MetricValue::String(s) => serde_json::json!(s),
                                edge_ai_devices::MetricValue::Integer(i) => serde_json::json!(i),
                                edge_ai_devices::MetricValue::Float(f) => serde_json::json!(f),
                                edge_ai_devices::MetricValue::Boolean(b) => serde_json::json!(b),
                                _ => serde_json::json!(null),
                            }).collect();
                            serde_json::json!(json_arr)
                        }
                        edge_ai_devices::MetricValue::Binary(ref v) => {
                            serde_json::json!(general_purpose::STANDARD.encode(v))
                        }
                        edge_ai_devices::MetricValue::Null => serde_json::json!(null),
                    };

                    metrics_data.insert(
                        metric_name.clone(),
                        serde_json::json!({
                            "value": value_json,
                            "unit": metric_def.unit,
                            "display_name": metric_def.display_name,
                            "timestamp": point.timestamp,
                        })
                    );
                } else {
                    // No data available for this metric
                    metrics_data.insert(
                        metric_name.clone(),
                        serde_json::json!({
                            "value": null,
                            "unit": metric_def.unit,
                            "display_name": metric_def.display_name,
                            "status": "no_data"
                        })
                    );
                }
            }
        } else {
            // Template has no defined metrics - try to list actual metrics from storage
            if let Ok(actual_metrics) = self.storage.list_metrics(device_id).await {
                if !actual_metrics.is_empty() {
                    for metric_name in actual_metrics {
                        if let Ok(Some(point)) = self.storage.latest(device_id, &metric_name).await {
                            let value_json = match point.value {
                                edge_ai_devices::MetricValue::Float(v) => serde_json::json!(v),
                                edge_ai_devices::MetricValue::Integer(v) => serde_json::json!(v),
                                edge_ai_devices::MetricValue::String(ref v) => {
                                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(v) {
                                        json_val
                                    } else {
                                        serde_json::json!(v)
                                    }
                                }
                                edge_ai_devices::MetricValue::Boolean(v) => serde_json::json!(v),
                                edge_ai_devices::MetricValue::Binary(ref v) => {
                                    serde_json::json!(general_purpose::STANDARD.encode(v))
                                }
                                edge_ai_devices::MetricValue::Array(ref arr) => {
                                    let json_arr: Vec<serde_json::Value> = arr.iter().map(|v| match v {
                                        edge_ai_devices::MetricValue::Float(f) => serde_json::json!(f),
                                        edge_ai_devices::MetricValue::Integer(i) => serde_json::json!(i),
                                        edge_ai_devices::MetricValue::String(s) => serde_json::json!(s),
                                        edge_ai_devices::MetricValue::Boolean(b) => serde_json::json!(b),
                                        edge_ai_devices::MetricValue::Null => serde_json::json!(null),
                                        edge_ai_devices::MetricValue::Array(_) | edge_ai_devices::MetricValue::Binary(_) => {
                                            serde_json::json!(null)
                                        }
                                    }).collect();
                                    serde_json::json!(json_arr)
                                }
                                edge_ai_devices::MetricValue::Null => serde_json::json!(null),
                            };

                            metrics_data.insert(
                                metric_name.clone(),
                                serde_json::json!({
                                    "value": value_json,
                                    "timestamp": point.timestamp,
                                })
                            );
                        }
                    }
                } else {
                    return Err(ToolError::Execution(format!(
                        "No data available for device '{}'. The device may not be reporting data.",
                        device_id
                    )));
                }
            } else {
                return Err(ToolError::Execution(format!(
                    "Cannot retrieve data for device '{}'. Device may be offline or not configured.",
                    device_id
                )));
            }
        }

        Ok(ToolOutput::success(serde_json::json!({
            "device_id": device_id,
            "device_name": device_config.name,
            "device_type": device_config.device_type,
            "metrics": metrics_data,
            "metric_count": metrics_data.len()
        })))
    }
}

// ============================================================================
// DeviceAnalyzeTool - Real implementation using TimeSeriesStorage
// ============================================================================

/// Device analyze tool - provides statistical analysis on device data using real storage.
pub struct DeviceAnalyzeTool {
    service: Arc<DeviceService>,
    storage: Arc<TimeSeriesStorage>,
}

impl DeviceAnalyzeTool {
    /// Create a new device analyze tool with real services.
    pub fn new(service: Arc<DeviceService>, storage: Arc<TimeSeriesStorage>) -> Self {
        Self { service, storage }
    }
}

#[async_trait]
impl Tool for DeviceAnalyzeTool {
    fn name(&self) -> &str {
        "device.analyze"
    }

    fn description(&self) -> &str {
        r#"使用LLM分析设备数据，发现趋势、异常、模式和预测。支持多种分析类型：
- trend: 趋势分析 - 识别数据上升/下降/稳定趋势
- anomaly: 异常检测 - 发现数据中的异常点
- summary: 数据摘要 - 生成统计信息和洞察

用法示例:
- '分析温度趋势' → 分析温度变化趋势
- '检测异常数据' → 检测数据中的异常点
- '数据摘要' → 生成统计摘要和洞察"#
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "device_id": string_property("设备ID，支持模糊匹配。例如: 'sensor_temp_living' 或 'temp'"),
                "metric": string_property("要分析的指标名称，如'temperature'。不指定则分析所有可用指标"),
                "analysis_type": string_property("分析类型：'trend'趋势分析、'anomaly'异常检测、'summary'数据摘要。默认'summary'"),
                "limit": number_property("要分析的数据点数量，默认24个点")
            }),
            vec!["device_id".to_string()],
        )
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: serde_json::json!({
                    "device_id": "sensor_temp_living",
                    "metric": "temperature",
                    "analysis_type": "trend"
                }),
                result: serde_json::json!({
                    "analysis_type": "trend",
                    "findings": ["温度从 22°C 上升到 28°C", "变化幅度: +6°C"],
                    "insights": ["趋势: 📈 明显上升"],
                    "recommendations": ["温度持续上升，建议检查空调设置"]
                }),
                description: "分析温度变化趋势".to_string(),
            }),
            category: edge_ai_core::tools::ToolCategory::Device,
            scenarios: vec![
                UsageScenario {
                    description: "趋势分析".to_string(),
                    example_query: "分析温度趋势".to_string(),
                    suggested_call: Some(r#"{"device_id": "sensor_temp_living", "metric": "temperature", "analysis_type": "trend"}"#.to_string()),
                },
                UsageScenario {
                    description: "异常检测".to_string(),
                    example_query: "检测异常数据".to_string(),
                    suggested_call: Some(r#"{"device_id": "sensor_temp_living", "metric": "temperature", "analysis_type": "anomaly"}"#.to_string()),
                },
            ],
            relationships: edge_ai_core::tools::ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![],
            response_format: None,
            namespace: None,
        }
    }

    async fn execute(&self, args: Value) -> ToolResult<ToolOutput> {
        let device_id = args
            .get("device_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("device_id is required".to_string()))?;

        // Find device(s) with fuzzy matching
        let devices = self.service.list_devices().await;
        let matched_devices: Vec<_> = devices
            .iter()
            .filter(|d| d.device_id.contains(device_id) || d.name.contains(device_id))
            .collect();

        if matched_devices.is_empty() {
            return Ok(ToolOutput::error_with_metadata(
                format!("未找到设备: {}", device_id),
                serde_json::json!({"device_id": device_id, "hint": "使用 device.discovery() 查看可用设备"}),
            ));
        }

        let device = &matched_devices[0];

        // Get analysis type
        let analysis_type = args
            .get("analysis_type")
            .and_then(|v| v.as_str())
            .unwrap_or("summary");

        // Get metric to analyze
        let metric_param = args.get("metric").and_then(|v| v.as_str());

        // Get limit
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(24) as usize;

        // Determine which metrics to analyze
        let metrics_to_analyze: Vec<String> = if let Some(m) = metric_param {
            vec![m.to_string()]
        } else {
            // Get available metrics from device template (async call)
            // For now, just list metrics from storage
            match self.storage.list_metrics(&device.device_id).await {
                Ok(metrics) => metrics,
                Err(_) => vec![],
            }
        };

        if metrics_to_analyze.is_empty() {
            return Ok(ToolOutput::error(
                "设备没有可分析的指标".to_string()
            ));
        }

        // Perform analysis for each metric
        let mut all_findings = vec![];
        let mut all_insights = vec![];
        let mut all_recommendations = vec![];

        for metric_name in metrics_to_analyze {
            // Fetch historical data from storage using query_telemetry
            let end_time = chrono::Utc::now().timestamp();
            let start_time = end_time - (24 * 3600); // 24 hours ago

            let history = self.service.query_telemetry(
                &device.device_id,
                &metric_name,
                Some(start_time),
                Some(end_time),
            ).await.map_err(|e| {
                ToolError::Execution(format!("Failed to query telemetry: {}", e))
            })?;

            if history.is_empty() {
                all_findings.push(format!("指标 {} 暂无数据", metric_name));
                continue;
            }

            // Convert to DataPoint format
            let data_points: Vec<edge_ai_devices::DataPoint> = history
                .into_iter()
                .map(|(ts, value)| edge_ai_devices::DataPoint {
                    timestamp: ts,
                    value,
                    quality: None,
                })
                .collect();

            match analysis_type {
                "trend" => {
                    let analysis = self.analyze_trend(&data_points, &metric_name);
                    all_findings.extend(analysis.findings);
                    all_insights.extend(analysis.insights);
                    all_recommendations.extend(analysis.recommendations);
                }
                "anomaly" => {
                    let analysis = self.analyze_anomaly(&data_points, &metric_name);
                    all_findings.extend(analysis.findings);
                    all_insights.extend(analysis.insights);
                    all_recommendations.extend(analysis.recommendations);
                }
                _ => {  // summary
                    let analysis = self.analyze_summary(&data_points, &metric_name);
                    all_findings.extend(analysis.findings);
                    all_insights.extend(analysis.insights);
                    all_recommendations.extend(analysis.recommendations);
                }
            }
        }

        Ok(ToolOutput::success(serde_json::json!({
            "device_id": device.device_id,
            "device_name": device.name,
            "analysis_type": analysis_type,
            "data_points_analyzed": limit,
            "findings": all_findings,
            "insights": all_insights,
            "recommendations": all_recommendations
        })))
    }
}

/// Analysis result structure
struct AnalysisResult {
    analysis_type: String,
    device_id: String,
    metric: String,
    time_period: String,
    findings: Vec<String>,
    insights: Vec<String>,
    recommendations: Vec<String>,
    confidence: f64,
    supporting_data: Option<Value>,
}

impl DeviceAnalyzeTool {
    /// Perform trend analysis on metric data.
    fn analyze_trend(&self, data: &[edge_ai_devices::DataPoint], metric: &str) -> AnalysisResult {
        if data.len() < 2 {
            return AnalysisResult {
                analysis_type: "trend".to_string(),
                device_id: String::new(),
                metric: metric.to_string(),
                time_period: "数据不足".to_string(),
                findings: vec![format!("{} 暂无足够数据进行趋势分析", metric)],
                insights: vec![],
                recommendations: vec![],
                confidence: 0.0,
                supporting_data: None,
            };
        }

        let values: Vec<f64> = data.iter()
            .filter_map(|p| match p.value {
                edge_ai_devices::MetricValue::Float(v) => Some(v),
                edge_ai_devices::MetricValue::Integer(v) => Some(v as f64),
                _ => None,
            })
            .collect();

        if values.is_empty() {
            return AnalysisResult {
                analysis_type: "trend".to_string(),
                device_id: String::new(),
                metric: metric.to_string(),
                time_period: "无数据".to_string(),
                findings: vec![format!("{} 没有数值数据", metric)],
                insights: vec![],
                recommendations: vec![],
                confidence: 0.0,
                supporting_data: None,
            };
        }

        let first = values.first().unwrap_or(&0.0);
        let last = values.last().unwrap_or(&0.0);
        let change = last - first;
        let pct_change = if first.abs() > 0.001 {
            (change / first.abs()) * 100.0
        } else {
            0.0
        };

        let (trend_desc, icon) = if pct_change > 10.0 {
            ("明显上升", "📈")
        } else if pct_change > 3.0 {
            ("缓慢上升", "📈")
        } else if pct_change < -10.0 {
            ("明显下降", "📉")
        } else if pct_change < -3.0 {
            ("缓慢下降", "📉")
        } else {
            ("保持稳定", "➡️")
        };

        let findings = vec![
            format!("{} 数据点分析", data.len()),
            format!("初始值: {:.2}, 最终值: {:.2}", first, last),
            format!("变化: {:+.2} ({:+.1}%)", change, pct_change),
        ];

        let insights = vec![format!("趋势: {} {}", icon, trend_desc)];

        let mut recommendations = vec![];

        if metric.contains("temperature") || metric.contains("temp") {
            if pct_change > 5.0 {
                recommendations.push("温度持续上升，建议检查空调设置".to_string());
            } else if pct_change < -5.0 {
                recommendations.push("温度持续下降，注意保温".to_string());
            }
        }

        AnalysisResult {
            analysis_type: "trend".to_string(),
            device_id: String::new(),
            metric: metric.to_string(),
            time_period: format!("最近{}个数据点", data.len()),
            findings,
            insights,
            recommendations,
            confidence: if pct_change.abs() > 3.0 { 0.85 } else { 0.6 },
            supporting_data: Some(serde_json::json!({
                "first_value": first,
                "last_value": last,
                "change": change,
                "pct_change": pct_change
            })),
        }
    }

    /// Perform anomaly detection on metric data.
    fn analyze_anomaly(&self, data: &[edge_ai_devices::DataPoint], metric: &str) -> AnalysisResult {
        let values: Vec<f64> = data.iter()
            .filter_map(|p| match p.value {
                edge_ai_devices::MetricValue::Float(v) => Some(v),
                edge_ai_devices::MetricValue::Integer(v) => Some(v as f64),
                _ => None,
            })
            .collect();

        if values.len() < 3 {
            return AnalysisResult {
                analysis_type: "anomaly".to_string(),
                device_id: String::new(),
                metric: metric.to_string(),
                time_period: "数据不足".to_string(),
                findings: vec![format!("{} 需要至少3个数据点进行异常检测", metric)],
                insights: vec![],
                recommendations: vec![],
                confidence: 0.0,
                supporting_data: None,
            };
        }

        // Calculate mean and standard deviation
        let n = values.len() as f64;
        let mean: f64 = values.iter().sum();
        let mean = mean / n;

        let variance: f64 = values.iter()
            .map(|&v| (v - mean).powi(2))
            .sum();
        let variance = variance / n;
        let std_dev = variance.sqrt();

        // Find anomalies (values beyond 2 standard deviations)
        let threshold = 2.0 * std_dev;
        let anomalies: Vec<(usize, f64)> = values.iter()
            .enumerate()
            .filter(|&(_, &v)| (v - mean).abs() > threshold)
            .map(|(i, &v)| (i, v))
            .collect();

        let findings = vec![
            format!("分析{}个数据点", data.len()),
            format!("平均值: {:.2}, 标准差: {:.2}", mean, std_dev),
            format!("检测到{}个异常值", anomalies.len()),
        ];

        let mut insights = vec![];
        if anomalies.is_empty() {
            insights.push("✓ 未发现明显异常".to_string());
        } else {
            insights.push(format!("⚠️ 发现{}个异常值", anomalies.len()));
        }

        let mut recommendations = vec![];
        if !anomalies.is_empty() {
            recommendations.push("建议检查异常数据点对应时间的设备状态".to_string());
        }

        AnalysisResult {
            analysis_type: "anomaly".to_string(),
            device_id: String::new(),
            metric: metric.to_string(),
            time_period: format!("最近{}个数据点", data.len()),
            findings,
            insights,
            recommendations,
            confidence: 0.75,
            supporting_data: Some(serde_json::json!({
                "mean": mean,
                "std_dev": std_dev,
                "anomaly_count": anomalies.len()
            })),
        }
    }

    /// Perform summary analysis on metric data.
    fn analyze_summary(&self, data: &[edge_ai_devices::DataPoint], metric: &str) -> AnalysisResult {
        let values: Vec<f64> = data.iter()
            .filter_map(|p| match p.value {
                edge_ai_devices::MetricValue::Float(v) => Some(v),
                edge_ai_devices::MetricValue::Integer(v) => Some(v as f64),
                _ => None,
            })
            .collect();

        if values.is_empty() {
            return AnalysisResult {
                analysis_type: "summary".to_string(),
                device_id: String::new(),
                metric: metric.to_string(),
                time_period: "无数据".to_string(),
                findings: vec![format!("{} 没有数值数据", metric)],
                insights: vec![],
                recommendations: vec![],
                confidence: 0.0,
                supporting_data: None,
            };
        }

        let n = values.len();
        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let mean: f64 = values.iter().sum();
        let mean = mean / n as f64;

        let variance: f64 = values.iter()
            .map(|&v| (v - mean).powi(2))
            .sum();
        let variance = variance / n as f64;
        let std_dev = variance.sqrt();

        let findings = vec![
            format!("数据点数: {}", n),
            format!("最小值: {:.2}", min),
            format!("最大值: {:.2}", max),
            format!("平均值: {:.2}", mean),
            format!("标准差: {:.2}", std_dev),
        ];

        let insights = vec![
            format!("数据范围: {:.2} ~ {:.2}", min, max),
            format!("波动程度: {}", if std_dev < (max - min) * 0.1 { "稳定" } else { "波动较大" }),
        ];

        let recommendations = vec![];

        AnalysisResult {
            analysis_type: "summary".to_string(),
            device_id: String::new(),
            metric: metric.to_string(),
            time_period: format!("最近{}个数据点", data.len()),
            findings,
            insights,
            recommendations,
            confidence: 1.0,
            supporting_data: Some(serde_json::json!({
                "min": min,
                "max": max,
                "mean": mean,
                "std_dev": std_dev
            })),
        }
    }
}
