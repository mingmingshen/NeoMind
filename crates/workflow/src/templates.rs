//! Workflow templates and generator
//!
//! Provides predefined workflow templates and LLM-based workflow generation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Workflow template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    /// Template ID
    pub id: String,
    /// Template name
    pub name: String,
    /// Template category
    pub category: String,
    /// Description
    pub description: String,
    /// DSL template with {parameter} placeholders
    pub dsl_template: String,
    /// Parameters for this template
    pub parameters: Vec<TemplateParameter>,
}

/// Template parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name
    pub name: String,
    /// Human-readable label
    pub label: String,
    /// Default value (if any)
    #[serde(default)]
    pub default: Option<String>,
    /// Whether this parameter is required
    #[serde(default)]
    pub required: bool,
    /// Parameter type
    #[serde(default)]
    pub param_type: TemplateParameterType,
    /// Options for enum type
    #[serde(default)]
    pub options: Vec<String>,
}

/// Parameter type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TemplateParameterType {
    #[default]
    String,
    Number,
    Boolean,
    Device,
    Metric,
    Enum,
}

/// Workflow filled from a template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatedWorkflow {
    /// The workflow DSL (JSON)
    pub dsl: String,
    /// Template ID used
    pub template_id: String,
    /// Parameter values used
    pub parameters: HashMap<String, String>,
}

impl WorkflowTemplate {
    /// Fill the template with given parameters
    pub fn fill(&self, params: &HashMap<String, String>) -> Result<TemplatedWorkflow, String> {
        let mut dsl = self.dsl_template.clone();

        // Check required parameters
        for param in &self.parameters {
            if param.required && !params.contains_key(&param.name) {
                return Err(format!("Missing required parameter: {}", param.label));
            }
        }

        // Fill parameters
        for param in &self.parameters {
            let value = params
                .get(&param.name)
                .or(param.default.as_ref())
                .ok_or_else(|| format!("Missing parameter: {}", param.name))?;

            dsl = dsl.replace(&format!("{{{}}}", param.name), value);
        }

        Ok(TemplatedWorkflow {
            dsl,
            template_id: self.id.clone(),
            parameters: params.clone(),
        })
    }

    /// Get parameters with defaults filled
    pub fn get_default_parameters(&self) -> HashMap<String, String> {
        self.parameters
            .iter()
            .filter_map(|p| {
                p.default.as_ref().map(|default| (p.name.clone(), default.clone()))
            })
            .collect()
    }
}

/// Predefined workflow templates for common scenarios
pub struct WorkflowTemplates;

impl WorkflowTemplates {
    /// Get all available templates
    pub fn all() -> Vec<WorkflowTemplate> {
        vec![
            // Template 1: Device Alert with Notification
            WorkflowTemplate {
                id: "device_alert".to_string(),
                name: "设备告警通知".to_string(),
                category: "alert".to_string(),
                description: "当设备状态异常时发送告警通知".to_string(),
                dsl_template: r#"{
  "name": "{workflow_name}",
  "description": "监控{device_name}并在{condition}时发送通知",
  "steps": [
    {
      "type": "device_query",
      "id": "read_{device_id}",
      "device_id": "{device_id}",
      "metric": "{metric}"
    },
    {
      "type": "condition",
      "id": "check_condition",
      "condition": "${{device_id}_{metric}} {operator} {threshold}",
      "then_steps": [
        {
          "type": "send_alert",
          "id": "send_alert",
          "severity": "{severity}",
          "title": "{alert_title}",
          "message": "{device_name} {metric} 为 ${{{device_id}_{metric}}}，{condition_description}"
        }
      ]
    }
  ],
  "triggers": [
    {
      "type": "cron",
      "id": "trigger_1",
      "expression": "{cron_expression}"
    }
  ]
}"#.to_string(),
                parameters: vec![
                    TemplateParameter {
                        name: "workflow_name".to_string(),
                        label: "工作流名称".to_string(),
                        default: Some("设备告警".to_string()),
                        required: false,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "device_id".to_string(),
                        label: "设备ID".to_string(),
                        default: None,
                        required: true,
                        param_type: TemplateParameterType::Device,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "device_name".to_string(),
                        label: "设备名称".to_string(),
                        default: Some("设备".to_string()),
                        required: false,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "metric".to_string(),
                        label: "指标".to_string(),
                        default: Some("temperature".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Metric,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "operator".to_string(),
                        label: "操作符".to_string(),
                        default: Some(">".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Enum,
                        options: vec![">".to_string(), "<".to_string(), ">=".to_string(), "<=".to_string()],
                    },
                    TemplateParameter {
                        name: "threshold".to_string(),
                        label: "阈值".to_string(),
                        default: Some("80".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Number,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "severity".to_string(),
                        label: "严重程度".to_string(),
                        default: Some("warning".to_string()),
                        required: false,
                        param_type: TemplateParameterType::Enum,
                        options: vec!["info".to_string(), "warning".to_string(), "critical".to_string()],
                    },
                    TemplateParameter {
                        name: "alert_title".to_string(),
                        label: "告警标题".to_string(),
                        default: Some("设备异常告警".to_string()),
                        required: false,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "condition_description".to_string(),
                        label: "条件描述".to_string(),
                        default: Some("超过阈值".to_string()),
                        required: false,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "cron_expression".to_string(),
                        label: "Cron表达式".to_string(),
                        default: Some("*/5 * * * *".to_string()),
                        required: true,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                ],
            },
            // Template 2: Multi-Device Control
            WorkflowTemplate {
                id: "multi_device_control".to_string(),
                name: "多设备联动控制".to_string(),
                category: "automation".to_string(),
                description: "根据一个设备的状态控制多个其他设备".to_string(),
                dsl_template: r#"{
  "name": "{workflow_name}",
  "description": "当{trigger_device_name}的{trigger_metric}满足条件时，控制多个设备",
  "steps": [
    {
      "type": "device_query",
      "id": "read_trigger_device",
      "device_id": "{trigger_device_id}",
      "metric": "{trigger_metric}"
    },
    {
      "type": "condition",
      "id": "check_condition",
      "condition": "${trigger_device_id}_{trigger_metric} {operator} {threshold}",
      "then_steps": [
        {
          "type": "parallel",
          "id": "control_devices",
          "steps": {control_steps}
        }
      ]
    }
  ],
  "triggers": [
    {
      "type": "device",
      "id": "device_trigger",
      "device_id": "{trigger_device_id}",
      "metric": "{trigger_metric}",
      "condition": "{operator} {threshold}"
    }
  ]
}"#.to_string(),
                parameters: vec![
                    TemplateParameter {
                        name: "workflow_name".to_string(),
                        label: "工作流名称".to_string(),
                        default: Some("多设备联动".to_string()),
                        required: false,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "trigger_device_id".to_string(),
                        label: "触发设备ID".to_string(),
                        default: None,
                        required: true,
                        param_type: TemplateParameterType::Device,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "trigger_device_name".to_string(),
                        label: "触发设备名称".to_string(),
                        default: Some("触发设备".to_string()),
                        required: false,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "trigger_metric".to_string(),
                        label: "触发指标".to_string(),
                        default: Some("value".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Metric,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "operator".to_string(),
                        label: "操作符".to_string(),
                        default: Some(">".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Enum,
                        options: vec![">".to_string(), "<".to_string()],
                    },
                    TemplateParameter {
                        name: "threshold".to_string(),
                        label: "阈值".to_string(),
                        default: Some("50".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Number,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "control_steps".to_string(),
                        label: "控制步骤 (JSON)".to_string(),
                        default: Some(r#"[
  {"type": "send_command", "id": "cmd1", "device_id": "device1", "command": "on"},
  {"type": "send_command", "id": "cmd2", "device_id": "device2", "command": "off"}
]"#.to_string()),
                        required: true,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                ],
            },
            // Template 3: Scheduled Device Action
            WorkflowTemplate {
                id: "scheduled_action".to_string(),
                name: "定时设备操作".to_string(),
                category: "schedule".to_string(),
                description: "按计划自动执行设备操作".to_string(),
                dsl_template: r#"{
  "name": "{workflow_name}",
  "description": "每天{time}执行{action}",
  "steps": {action_steps},
  "triggers": [
    {
      "type": "cron",
      "id": "schedule_trigger",
      "expression": "{cron_expression}",
      "timezone": "{timezone}"
    }
  ]
}"#.to_string(),
                parameters: vec![
                    TemplateParameter {
                        name: "workflow_name".to_string(),
                        label: "工作流名称".to_string(),
                        default: Some("定时操作".to_string()),
                        required: false,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "cron_expression".to_string(),
                        label: "Cron表达式".to_string(),
                        default: Some("0 8 * * *".to_string()),
                        required: true,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "timezone".to_string(),
                        label: "时区".to_string(),
                        default: Some("Asia/Shanghai".to_string()),
                        required: false,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "action_steps".to_string(),
                        label: "操作步骤 (JSON)".to_string(),
                        default: Some(r#"[{"type": "send_command", "id": "cmd1", "device_id": "device1", "command": "on"}]"#.to_string()),
                        required: true,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                ],
            },
            // Template 4: Data Collection and Report
            WorkflowTemplate {
                id: "data_collection".to_string(),
                name: "数据采集与上报".to_string(),
                category: "data".to_string(),
                description: "定时采集多个设备数据并发送报告".to_string(),
                dsl_template: r#"{
  "name": "{workflow_name}",
  "description": "采集{devices}数据并发送到{endpoint}",
  "steps": [
    {
      "type": "parallel",
      "id": "collect_data",
      "steps": {query_steps}
    },
    {
      "type": "delay",
      "id": "wait_for_collection",
      "duration_seconds": 2
    },
    {
      "type": "http_request",
      "id": "send_report",
      "url": "{endpoint}",
      "method": "POST",
      "headers": {"Content-Type": "application/json"},
      "body": "{\"timestamp\": \"${timestamp}\", \"data\": ${collected_data}}"
    }
  ],
  "triggers": [
    {
      "type": "cron",
      "id": "schedule_trigger",
      "expression": "{cron_expression}"
    }
  ]
}"#.to_string(),
                parameters: vec![
                    TemplateParameter {
                        name: "workflow_name".to_string(),
                        label: "工作流名称".to_string(),
                        default: Some("数据采集".to_string()),
                        required: false,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "query_steps".to_string(),
                        label: "查询步骤 (JSON)".to_string(),
                        default: Some(r#"[
  {"type": "device_query", "id": "query1", "device_id": "sensor1", "metric": "temperature"},
  {"type": "device_query", "id": "query2", "device_id": "sensor2", "metric": "humidity"}
]"#.to_string()),
                        required: true,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "endpoint".to_string(),
                        label: "上报端点".to_string(),
                        default: Some("http://example.com/api/report".to_string()),
                        required: true,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "cron_expression".to_string(),
                        label: "Cron表达式".to_string(),
                        default: Some("0 * * * *".to_string()),
                        required: true,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                ],
            },
            // Template 5: Conditional Workflow with Delay
            WorkflowTemplate {
                id: "conditional_delay".to_string(),
                name: "条件判断与延迟执行".to_string(),
                category: "logic".to_string(),
                description: "检查条件，满足则延迟一段时间后执行操作".to_string(),
                dsl_template: r#"{
  "name": "{workflow_name}",
  "description": "当{condition}时，等待{delay}秒后执行{action}",
  "steps": [
    {
      "type": "device_query",
      "id": "check_condition",
      "device_id": "{device_id}",
      "metric": "{metric}"
    },
    {
      "type": "condition",
      "id": "evaluate_condition",
      "condition": "${device_id}_{metric} {operator} {threshold}",
      "then_steps": [
        {
          "type": "delay",
          "id": "wait_delay",
          "duration_seconds": {delay}
        },
        {
          "type": "send_command",
          "id": "execute_action",
          "device_id": "{target_device_id}",
          "command": "{command}"
        }
      ],
      "else_steps": [
        {
          "type": "log",
          "id": "log_not_met",
          "message": "条件未满足，跳过执行",
          "level": "info"
        }
      ]
    }
  ],
  "triggers": [
    {
      "type": "cron",
      "id": "cron_trigger",
      "expression": "{cron_expression}"
    }
  ]
}"#.to_string(),
                parameters: vec![
                    TemplateParameter {
                        name: "workflow_name".to_string(),
                        label: "工作流名称".to_string(),
                        default: Some("条件延迟执行".to_string()),
                        required: false,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "device_id".to_string(),
                        label: "检查设备ID".to_string(),
                        default: None,
                        required: true,
                        param_type: TemplateParameterType::Device,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "metric".to_string(),
                        label: "指标".to_string(),
                        default: Some("value".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Metric,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "operator".to_string(),
                        label: "操作符".to_string(),
                        default: Some(">".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Enum,
                        options: vec![">".to_string(), "<".to_string(), "==".to_string()],
                    },
                    TemplateParameter {
                        name: "threshold".to_string(),
                        label: "阈值".to_string(),
                        default: Some("50".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Number,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "delay".to_string(),
                        label: "延迟(秒)".to_string(),
                        default: Some("30".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Number,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "target_device_id".to_string(),
                        label: "目标设备ID".to_string(),
                        default: None,
                        required: true,
                        param_type: TemplateParameterType::Device,
                        options: vec![],
                    },
                    TemplateParameter {
                        name: "command".to_string(),
                        label: "命令".to_string(),
                        default: Some("on".to_string()),
                        required: true,
                        param_type: TemplateParameterType::Enum,
                        options: vec!["on".to_string(), "off".to_string(), "toggle".to_string()],
                    },
                    TemplateParameter {
                        name: "cron_expression".to_string(),
                        label: "Cron表达式".to_string(),
                        default: Some("*/10 * * * *".to_string()),
                        required: true,
                        param_type: TemplateParameterType::String,
                        options: vec![],
                    },
                ],
            },
        ]
    }

    /// Get template by ID
    pub fn get(id: &str) -> Option<WorkflowTemplate> {
        Self::all()
            .into_iter()
            .find(|t| t.id == id)
    }

    /// Get templates by category
    pub fn by_category(category: &str) -> Vec<WorkflowTemplate> {
        Self::all()
            .into_iter()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Get all categories
    pub fn categories() -> Vec<String> {
        let mut cats: Vec<String> = Self::all()
            .iter()
            .map(|t| t.category.clone())
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }
}

/// LLM-based workflow generator
pub struct WorkflowGenerator;

impl WorkflowGenerator {
    /// Generate a workflow from natural language description
    pub fn generate(
        description: &str,
        context: &ValidationContext,
    ) -> Result<GeneratedWorkflow, String> {
        // Extract information from description
        let extracted = Self::extract_workflow_info(description)?;

        // Build workflow JSON
        let workflow_json = Self::build_workflow_json(&extracted, context)?;

        Ok(GeneratedWorkflow {
            workflow_json,
            explanation: Self::generate_explanation(&extracted),
            confidence: Self::calculate_confidence(&extracted),
            suggested_edits: Self::generate_suggestions(&extracted, context),
            warnings: Self::validate_workflow(&extracted, context),
        })
    }

    /// Extract workflow information from description
    fn extract_workflow_info(desc: &str) -> Result<ExtractedWorkflowInfo, String> {
        let desc_lower = desc.to_lowercase();

        // Extract trigger type
        let trigger_type = if desc_lower.contains("定时") || desc_lower.contains("每天") || desc_lower.contains("cron") {
            WorkflowTriggerType::Cron
        } else if desc_lower.contains("当") || desc_lower.contains("时") || desc_lower.contains("如果") {
            WorkflowTriggerType::Device
        } else {
            WorkflowTriggerType::Manual
        };

        // Extract cron expression
        let cron_expression = if desc_lower.contains("每5分钟") || desc_lower.contains("5分钟") {
            Some("*/5 * * * *".to_string())
        } else if desc_lower.contains("每小时") || desc_lower.contains("1小时") {
            Some("0 * * * *".to_string())
        } else if desc_lower.contains("每天") {
            if desc_lower.contains("8点") || desc_lower.contains("早上") {
                Some("0 8 * * *".to_string())
            } else if desc_lower.contains("18点") || desc_lower.contains("晚上") {
                Some("0 18 * * *".to_string())
            } else {
                Some("0 0 * * *".to_string())
            }
        } else {
            None
        };

        // Extract devices (simple pattern matching)
        let devices = Self::extract_devices_from_text(desc)?;

        // Extract action
        let action = if desc_lower.contains("开灯") || desc_lower.contains("打开") {
            WorkflowAction::TurnOn
        } else if desc_lower.contains("关灯") || desc_lower.contains("关闭") {
            WorkflowAction::TurnOff
        } else if desc_lower.contains("告警") || desc_lower.contains("通知") {
            WorkflowAction::Alert
        } else if desc_lower.contains("上报") || desc_lower.contains("发送") {
            WorkflowAction::Report
        } else {
            WorkflowAction::Custom("custom_action".to_string())
        };

        // Extract condition
        let condition = if desc_lower.contains("温度") && (desc_lower.contains("超过") || desc_lower.contains("大于")) {
            Some(Condition {
                metric: "temperature".to_string(),
                operator: ">".to_string(),
                threshold: 50.0,
            })
        } else if desc_lower.contains("湿度") && (desc_lower.contains("超过") || desc_lower.contains("大于")) {
            Some(Condition {
                metric: "humidity".to_string(),
                operator: ">".to_string(),
                threshold: 80.0,
            })
        } else {
            None
        };

        Ok(ExtractedWorkflowInfo {
            name: Self::extract_name(desc),
            description: desc.to_string(),
            trigger_type,
            cron_expression,
            devices,
            action,
            condition,
        })
    }

    /// Extract name from description
    fn extract_name(desc: &str) -> String {
        // Simple heuristic: use first few words or generate a default
        let words: Vec<&str> = desc.split(['，', ',', '\n']).collect();
        if let Some(first) = words.first() {
            let trimmed = first.trim();
            // Use character count instead of byte count for UTF-8 safety
            let char_count = trimmed.chars().count();
            if char_count <= 20 {
                trimmed.to_string()
            } else {
                // Take first 17 characters safely
                let truncated: String = trimmed.chars().take(17).collect();
                format!("{}...", truncated)
            }
        } else {
            "自动生成工作流".to_string()
        }
    }

    /// Extract device references from text
    fn extract_devices_from_text(desc: &str) -> Result<Vec<String>, String> {
        let mut devices = Vec::new();

        // Look for device patterns
        if desc.contains("设备") {
            devices.push("device1".to_string());
        }
        if desc.contains("传感器") || desc.contains("sensor") {
            devices.push("sensor1".to_string());
        }

        Ok(devices)
    }

    /// Build workflow JSON from extracted info
    fn build_workflow_json(
        extracted: &ExtractedWorkflowInfo,
        _context: &ValidationContext,
    ) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();

        let mut steps = Vec::new();

        // Add condition check if present
        if let Some(ref condition) = extracted.condition {
            steps.push(serde_json::json!({
                "type": "device_query",
                "id": format!("check_{}", condition.metric),
                "device_id": extracted.devices.first().unwrap_or(&"device1".to_string()),
                "metric": condition.metric
            }));

            let then_steps = match extracted.action {
                WorkflowAction::TurnOn => vec![serde_json::json!({
                    "type": "send_command",
                    "id": "turn_on",
                    "device_id": extracted.devices.get(1).unwrap_or(&"device1".to_string()),
                    "command": "on"
                })],
                WorkflowAction::TurnOff => vec![serde_json::json!({
                    "type": "send_command",
                    "id": "turn_off",
                    "device_id": extracted.devices.get(1).unwrap_or(&"device1".to_string()),
                    "command": "off"
                })],
                WorkflowAction::Alert => vec![serde_json::json!({
                    "type": "send_alert",
                    "id": "send_alert",
                    "severity": "warning",
                    "title": "工作流告警",
                    "message": format!("{} 触发告警", extracted.name)
                })],
                WorkflowAction::Report => vec![serde_json::json!({
                    "type": "log",
                    "id": "log_report",
                    "message": "数据已上报",
                    "level": "info"
                })],
                WorkflowAction::Custom(ref cmd) => vec![serde_json::json!({
                    "type": "log",
                    "id": "custom_log",
                    "message": format!("执行: {}", cmd),
                    "level": "info"
                })],
            };

            steps.push(serde_json::json!({
                "type": "condition",
                "id": "evaluate_condition",
                "condition": format!("${{check_{}}}_{} {} {}", condition.metric, condition.metric, condition.operator, condition.threshold),
                "then_steps": then_steps,
                "else_steps": []
            }));
        } else {
            // No condition, just execute action
            match extracted.action {
                WorkflowAction::TurnOn => steps.push(serde_json::json!({
                    "type": "send_command",
                    "id": "turn_on",
                    "device_id": extracted.devices.first().unwrap_or(&"device1".to_string()),
                    "command": "on"
                })),
                WorkflowAction::TurnOff => steps.push(serde_json::json!({
                    "type": "send_command",
                    "id": "turn_off",
                    "device_id": extracted.devices.first().unwrap_or(&"device1".to_string()),
                    "command": "off"
                })),
                _ => steps.push(serde_json::json!({
                    "type": "log",
                    "id": "log",
                    "message": "执行工作流",
                    "level": "info"
                })),
            }
        }

        // Build triggers
        let triggers = match extracted.trigger_type {
            WorkflowTriggerType::Cron => vec![serde_json::json!({
                "type": "cron",
                "id": "cron_trigger",
                "expression": extracted.cron_expression.as_ref().unwrap_or(&"0 * * * *".to_string()),
                "timezone": "Asia/Shanghai"
            })],
            WorkflowTriggerType::Device => vec![serde_json::json!({
                "type": "device",
                "id": "device_trigger",
                "device_id": extracted.devices.first().unwrap_or(&"device1".to_string()),
                "metric": extracted.condition.as_ref().map(|c| c.metric.clone()).unwrap_or("value".to_string()),
                "condition": extracted.condition.as_ref().map(|c| format!("{} {}", c.operator, c.threshold)).unwrap_or("changed".to_string())
            })],
            WorkflowTriggerType::Manual => vec![serde_json::json!({
                "type": "manual",
                "id": "manual_trigger"
            })],
        };

        let workflow = serde_json::json!({
            "id": id,
            "name": extracted.name,
            "description": extracted.description,
            "steps": steps,
            "triggers": triggers,
            "enabled": true,
            "timeout_seconds": 300
        });

        serde_json::to_string_pretty(&workflow)
            .map_err(|e| format!("Failed to serialize workflow: {}", e))
    }

    /// Generate human-readable explanation
    fn generate_explanation(extracted: &ExtractedWorkflowInfo) -> String {
        format!(
            "工作流 \"{}\": {}触发器，{}动作",
            extracted.name,
            match extracted.trigger_type {
                WorkflowTriggerType::Cron => "定时",
                WorkflowTriggerType::Device => "设备事件",
                WorkflowTriggerType::Manual => "手动",
            },
            match extracted.action {
                WorkflowAction::TurnOn => "开启设备",
                WorkflowAction::TurnOff => "关闭设备",
                WorkflowAction::Alert => "发送告警",
                WorkflowAction::Report => "数据上报",
                WorkflowAction::Custom(_) => "自定义操作",
            }
        )
    }

    /// Calculate confidence score
    fn calculate_confidence(extracted: &ExtractedWorkflowInfo) -> f64 {
        let mut confidence = 0.5;

        if !extracted.devices.is_empty() {
            confidence += 0.2;
        }
        if extracted.condition.is_some() {
            confidence += 0.15;
        }
        if extracted.cron_expression.is_some() {
            confidence += 0.15;
        }

        f64::min(confidence, 1.0)
    }

    /// Generate suggested edits
    fn generate_suggestions(
        extracted: &ExtractedWorkflowInfo,
        context: &ValidationContext,
    ) -> Vec<SuggestedEdit> {
        let mut suggestions = Vec::new();

        if extracted.devices.is_empty()
            && let Some(first_device) = context.devices.keys().next() {
                suggestions.push(SuggestedEdit {
                    field: "devices".to_string(),
                    current_value: "[]".to_string(),
                    suggested_value: format!(r#"["{}"]"#, first_device),
                    reason: "需要至少指定一个设备".to_string(),
                });
            }

        if extracted.cron_expression.is_none() && extracted.trigger_type == WorkflowTriggerType::Cron {
            suggestions.push(SuggestedEdit {
                field: "cron_expression".to_string(),
                current_value: "null".to_string(),
                suggested_value: "0 * * * *".to_string(),
                reason: "定时触发器需要Cron表达式".to_string(),
            });
        }

        suggestions
    }

    /// Validate workflow and return warnings
    fn validate_workflow(
        extracted: &ExtractedWorkflowInfo,
        context: &ValidationContext,
    ) -> Vec<String> {
        let mut warnings = Vec::new();

        for device in &extracted.devices {
            if !context.devices.contains_key(device) && !device.starts_with("device") && !device.starts_with("sensor") {
                warnings.push(format!("设备 '{}' 可能不存在", device));
            }
        }

        warnings
    }
}

/// Validation context for workflow generation
#[derive(Debug, Clone, Default)]
pub struct ValidationContext {
    pub devices: HashMap<String, DeviceInfo>,
    pub metrics: Vec<String>,
    pub alert_channels: Vec<String>,
}

/// Device info for validation
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: String,
}

/// Generated workflow result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedWorkflow {
    /// Workflow JSON
    pub workflow_json: String,
    /// Human-readable explanation
    pub explanation: String,
    /// Confidence score (0-1)
    pub confidence: f64,
    /// Suggested edits
    pub suggested_edits: Vec<SuggestedEdit>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

/// Suggested edit for generated workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedEdit {
    pub field: String,
    pub current_value: String,
    pub suggested_value: String,
    pub reason: String,
}

/// Extracted workflow information from natural language
#[derive(Debug, Clone)]
struct ExtractedWorkflowInfo {
    name: String,
    description: String,
    trigger_type: WorkflowTriggerType,
    cron_expression: Option<String>,
    devices: Vec<String>,
    action: WorkflowAction,
    condition: Option<Condition>,
}

/// Workflow trigger type
#[derive(Debug, Clone, PartialEq)]
enum WorkflowTriggerType {
    Cron,
    Device,
    Manual,
}

/// Workflow action type
#[derive(Debug, Clone)]
enum WorkflowAction {
    TurnOn,
    TurnOff,
    Alert,
    Report,
    Custom(String),
}

/// Condition for workflow
#[derive(Debug, Clone)]
struct Condition {
    metric: String,
    operator: String,
    threshold: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_fill() {
        let templates = WorkflowTemplates::all();
        assert!(!templates.is_empty());

        let template = &templates[0];
        let mut params = template.get_default_parameters();
        params.insert("device_id".to_string(), "test_device".to_string());

        let result = template.fill(&params);
        assert!(result.is_ok());
        assert!(result.unwrap().dsl.contains("test_device"));
    }

    #[test]
    fn test_template_by_category() {
        let alert_templates = WorkflowTemplates::by_category("alert");
        assert!(!alert_templates.is_empty());

        let automation_templates = WorkflowTemplates::by_category("automation");
        assert!(!automation_templates.is_empty());
    }

    #[test]
    fn test_workflow_generator() {
        let context = ValidationContext::default();
        let result = WorkflowGenerator::generate("每天早上8点打开设备", &context);
        assert!(result.is_ok());
        let generated = result.unwrap();
        assert!(!generated.workflow_json.is_empty());
        assert!(generated.confidence > 0.0);
    }
}
