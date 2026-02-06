//! Natural Language to Automation generator.
//!
//! This module extracts entities from natural language descriptions
//! and generates complete RuleAutomation or WorkflowAutomation instances.

use std::sync::Arc;

use crate::error::{AutomationError, Result};
use neomind_core::{LlmRuntime, Message, GenerationParams};
use neomind_core::llm::backend::LlmInput;
use serde_json::json;

/// Language for prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Chinese,
    English,
}

/// Context for entity extraction
#[derive(Debug, Clone, Default)]
pub struct ExtractionContext {
    /// Available devices
    pub available_devices: Vec<DeviceInfo>,
    /// Available metrics
    pub available_metrics: Vec<MetricInfo>,
    /// Known rules
    pub existing_rules: Vec<String>,
}

/// Device information for context
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub device_type: String,
    pub id: String,
}

/// Metric information for context
#[derive(Debug, Clone)]
pub struct MetricInfo {
    pub name: String,
    pub description: String,
    pub device_types: Vec<String>,
}

/// Natural language to automation converter
pub struct Nl2Automation {
    llm: Arc<dyn LlmRuntime>,
    /// Language for prompts
    language: Language,
    /// Available device/metric context
    context: ExtractionContext,
}

impl Nl2Automation {
    /// Create a new NL2Automation converter
    pub fn new(llm: Arc<dyn LlmRuntime>) -> Self {
        Self {
            llm,
            language: Language::Chinese,
            context: ExtractionContext::default(),
        }
    }

    /// Create with custom language
    pub fn with_language(llm: Arc<dyn LlmRuntime>, language: Language) -> Self {
        Self {
            llm,
            language,
            context: ExtractionContext::default(),
        }
    }

    /// Set the extraction context
    pub fn with_context(mut self, context: ExtractionContext) -> Self {
        self.context = context;
        self
    }

    /// Extract entities from a natural language description
    pub async fn extract_entities(&self, description: &str) -> Result<ExtractedEntities> {
        let user_prompt = self.build_prompt(description);

        let input = LlmInput {
            messages: vec![
                Message::system(match self.language {
                    Language::Chinese => "你是一个物联网自动化专家。从自然语言描述中提取结构化实体。只返回有效的JSON格式。",
                    Language::English => "You are an IoT automation expert. Extract structured entities from natural language descriptions. Respond ONLY with valid JSON.",
                }),
                Message::user(user_prompt),
            ],
            params: GenerationParams {
                temperature: Some(0.1),
                max_tokens: Some(1500),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        let response = self.llm.generate(input).await?;

        // Parse the JSON response
        let json_str = extract_json_from_response(&response.text)?;
        let entities: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| AutomationError::IntentAnalysisFailed(format!("Invalid JSON: {}", e)))?;

        Ok(self.parse_entities(entities))
    }

    /// Build the extraction prompt
    fn build_prompt(&self, description: &str) -> String {
        let (system_role, output_format, examples) = match self.language {
            Language::Chinese => (
                ZH_SYSTEM_ROLE,
                ZH_OUTPUT_FORMAT,
                ZH_EXAMPLES,
            ),
            Language::English => (
                EN_SYSTEM_ROLE,
                EN_OUTPUT_FORMAT,
                EN_EXAMPLES,
            ),
        };

        let mut prompt = String::new();
        prompt.push_str(system_role);
        prompt.push_str("\n\n");
        prompt.push_str(output_format);
        prompt.push_str("\n\n");
        prompt.push_str(examples);
        prompt.push_str("\n\n");

        // Add context if available
        if !self.context.available_devices.is_empty() {
            prompt.push_str("## 可用设备\n\n");
            for device in &self.context.available_devices {
                prompt.push_str(&format!("- {} ({})\n", device.name, device.device_type));
            }
            prompt.push('\n');
        }

        if !self.context.available_metrics.is_empty() {
            prompt.push_str("## 可用指标\n\n");
            for metric in &self.context.available_metrics {
                prompt.push_str(&format!("- {}: {}\n", metric.name, metric.description));
            }
            prompt.push('\n');
        }

        prompt.push_str("## 当前任务\n\n");
        prompt.push_str(&format!("描述: \"{}\"\n\n", description));
        prompt.push_str("请提取结构化实体，只返回JSON格式：");

        prompt
    }

    /// Parse entities from LLM response
    fn parse_entities(&self, value: serde_json::Value) -> ExtractedEntities {
        let triggers = value.get("triggers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| self.parse_trigger(v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let conditions = value.get("conditions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| self.parse_condition(v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let actions = value.get("actions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| self.parse_action(v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let devices = value.get("devices")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let time_constraints = value.get("time_constraints")
            .and_then(|v| self.parse_time_constraints(v.clone()));

        ExtractedEntities {
            triggers,
            conditions,
            actions,
            devices,
            time_constraints,
            confidence: 0.8,
        }
    }

    fn parse_trigger(&self, value: serde_json::Value) -> Option<TriggerEntity> {
        let trigger_type = value.get("type")?.as_str()?;
        let description = value.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Some(TriggerEntity {
            trigger_type: match trigger_type {
                "device_state" => TriggerTypeEntity::DeviceState,
                "schedule" => TriggerTypeEntity::Schedule,
                "manual" => TriggerTypeEntity::Manual,
                _ => TriggerTypeEntity::Manual,
            },
            device_id: value.get("device_id").and_then(|v| v.as_str()).map(String::from),
            metric: value.get("metric").and_then(|v| v.as_str()).map(String::from),
            condition: value.get("condition").and_then(|v| v.as_str()).map(String::from),
            cron: value.get("cron").and_then(|v| v.as_str()).map(String::from),
            description,
        })
    }

    fn parse_condition(&self, value: serde_json::Value) -> Option<ConditionEntity> {
        Some(ConditionEntity {
            device_id: value.get("device_id")?.as_str()?.to_string(),
            metric: value.get("metric")?.as_str()?.to_string(),
            operator: value.get("operator")?.as_str()?.to_string(),
            threshold: value.get("threshold").and_then(|v| v.as_f64()),
            description: value.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    fn parse_action(&self, value: serde_json::Value) -> Option<ActionEntity> {
        let action_type = value.get("type")?.as_str()?;
        let description = value.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Some(ActionEntity {
            action_type: match action_type {
                "notify" => ActionTypeEntity::Notify,
                "execute_command" => ActionTypeEntity::ExecuteCommand,
                "set_value" => ActionTypeEntity::SetValue,
                "create_alert" => ActionTypeEntity::CreateAlert,
                _ => ActionTypeEntity::Notify,
            },
            target: value.get("target").and_then(|v| v.as_str()).map(String::from),
            parameters: value.get("parameters").cloned().unwrap_or(json!({})),
            description,
        })
    }

    fn parse_time_constraints(&self, value: serde_json::Value) -> Option<TimeConstraints> {
        Some(TimeConstraints {
            start_time: value.get("start_time").and_then(|v| v.as_str()).map(String::from),
            end_time: value.get("end_time").and_then(|v| v.as_str()).map(String::from),
            days: value.get("days")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

/// Extracted entities from natural language
#[derive(Debug, Clone)]
pub struct ExtractedEntities {
    /// Identified triggers
    pub triggers: Vec<TriggerEntity>,
    /// Identified conditions
    pub conditions: Vec<ConditionEntity>,
    /// Identified actions
    pub actions: Vec<ActionEntity>,
    /// Referenced devices
    pub devices: Vec<String>,
    /// Time constraints
    pub time_constraints: Option<TimeConstraints>,
    /// Extraction confidence
    pub confidence: f32,
}

/// Trigger entity
#[derive(Debug, Clone)]
pub struct TriggerEntity {
    pub trigger_type: TriggerTypeEntity,
    pub device_id: Option<String>,
    pub metric: Option<String>,
    pub condition: Option<String>,
    pub cron: Option<String>,
    pub description: String,
}

/// Trigger type entity
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerTypeEntity {
    DeviceState,
    Schedule,
    Manual,
}

/// Condition entity
#[derive(Debug, Clone)]
pub struct ConditionEntity {
    pub device_id: String,
    pub metric: String,
    pub operator: String,
    pub threshold: Option<f64>,
    pub description: String,
}

/// Action entity
#[derive(Debug, Clone)]
pub struct ActionEntity {
    pub action_type: ActionTypeEntity,
    pub target: Option<String>,
    pub parameters: serde_json::Value,
    pub description: String,
}

/// Action type entity
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionTypeEntity {
    Notify,
    ExecuteCommand,
    SetValue,
    CreateAlert,
}

/// Time constraints
#[derive(Debug, Clone)]
pub struct TimeConstraints {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub days: Vec<String>,
}

/// Extract JSON from an LLM response
fn extract_json_from_response(response: &str) -> Result<String> {
    let start = response.find('{')
        .ok_or_else(|| AutomationError::IntentAnalysisFailed("No JSON object found".into()))?;

    let end = response.rfind('}')
        .ok_or_else(|| AutomationError::IntentAnalysisFailed("Incomplete JSON object".into()))?;

    Ok(response[start..=end].to_string())
}

// ============================================================================
// Prompt Templates
// ============================================================================

const ZH_SYSTEM_ROLE: &str = r#"你是一个物联网自动化专家，擅长从自然语言描述中提取结构化的自动化实体。

你的任务是分析用户描述，提取以下信息：
- **触发器 (triggers)**: 什么条件下触发自动化
- **条件 (conditions)**: 需要满足的判断条件
- **动作 (actions)**: 执行什么操作
- **涉及设备**: 提到的所有设备
- **时间约束**: 时间限制"#;

const EN_SYSTEM_ROLE: &str = r#"You are an IoT automation expert, skilled at extracting structured automation entities from natural language descriptions.

Your task is to analyze user descriptions and extract:
- **triggers**: What conditions trigger the automation
- **conditions**: Judgment conditions that must be met
- **actions**: What operations to execute
- **devices**: All mentioned devices
- **time_constraints**: Time restrictions"#;

const ZH_OUTPUT_FORMAT: &str = r#"## 输出格式

请严格按照以下JSON格式返回：

```json
{
  "triggers": [
    {
      "type": "device_state | schedule | manual",
      "device_id": "设备ID或null",
      "metric": "指标名称或null",
      "condition": "条件描述或null",
      "cron": "cron表达式或null",
      "description": "触发器的人类可读描述"
    }
  ],
  "conditions": [
    {
      "device_id": "设备ID",
      "metric": "指标名称",
      "operator": "gt | lt | eq | ne | gte | lte",
      "threshold": 阈值数字,
      "description": "条件描述"
    }
  ],
  "actions": [
    {
      "type": "notify | execute_command | set_value | create_alert",
      "target": "目标设备或接收者",
      "parameters": {},
      "description": "动作描述"
    }
  ],
  "devices": ["所有提到的设备列表"],
  "time_constraints": {
    "start_time": "HH:MM或null",
    "end_time": "HH:MM或null",
    "days": ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]或null
  }
}
```"#;

const EN_OUTPUT_FORMAT: &str = r#"## Output Format

Return in strict JSON format:

```json
{
  "triggers": [
    {
      "type": "device_state | schedule | manual",
      "device_id": "device ID or null",
      "metric": "metric name or null",
      "condition": "condition description or null",
      "cron": "cron expression or null",
      "description": "human-readable trigger description"
    }
  ],
  "conditions": [
    {
      "device_id": "device ID",
      "metric": "metric name",
      "operator": "gt | lt | eq | ne | gte | lte",
      "threshold": threshold number,
      "description": "condition description"
    }
  ],
  "actions": [
    {
      "type": "notify | execute_command | set_value | create_alert",
      "target": "target device or recipient",
      "parameters": {},
      "description": "action description"
    }
  ],
  "devices": ["list of all mentioned devices"],
  "time_constraints": {
    "start_time": "HH:MM or null",
    "end_time": "HH:MM or null",
    "days": ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] or null
  }
}
```"#;

const ZH_EXAMPLES: &str = r#"## 示例

**输入**: 当温度传感器1的温度超过30度时，发送通知
**输出**: {
  "triggers": [{"type": "device_state", "device_id": "温度传感器1", "metric": "temperature", "condition": "> 30", "description": "温度超过30度"}],
  "actions": [{"type": "notify", "description": "发送通知"}],
  "devices": ["温度传感器1"]
}

**输入**: 每天早上8点打开客厅灯
**输出**: {
  "triggers": [{"type": "schedule", "cron": "0 8 * * *", "description": "每天早上8点"}],
  "actions": [{"type": "set_value", "target": "客厅灯", "parameters": {"state": "on"}, "description": "打开客厅灯"}],
  "devices": ["客厅灯"]
}"#;

const EN_EXAMPLES: &str = r#"## Examples

**Input**: When temperature sensor 1 exceeds 30 degrees, send a notification
**Output**: {
  "triggers": [{"type": "device_state", "device_id": "temp_sensor_1", "metric": "temperature", "condition": "> 30", "description": "Temperature exceeds 30 degrees"}],
  "actions": [{"type": "notify", "description": "Send notification"}],
  "devices": ["temp_sensor_1"]
}

**Input**: Turn on living room light at 8am every day
**Output**: {
  "triggers": [{"type": "schedule", "cron": "0 8 * * *", "description": "8am daily"}],
  "actions": [{"type": "set_value", "target": "living_room_light", "parameters": {"state": "on"}, "description": "Turn on living room light"}],
  "devices": ["living_room_light"]
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_type_entity() {
        let types = vec![
            TriggerTypeEntity::DeviceState,
            TriggerTypeEntity::Schedule,
            TriggerTypeEntity::Manual,
        ];

        assert_eq!(types.len(), 3);
        assert_eq!(types[0], TriggerTypeEntity::DeviceState);
    }

    #[test]
    fn test_action_type_entity() {
        let types = vec![
            ActionTypeEntity::Notify,
            ActionTypeEntity::ExecuteCommand,
            ActionTypeEntity::SetValue,
            ActionTypeEntity::CreateAlert,
        ];

        assert_eq!(types.len(), 4);
    }

    #[test]
    fn test_extracted_entities_default() {
        let entities = ExtractedEntities {
            triggers: Vec::new(),
            conditions: Vec::new(),
            actions: Vec::new(),
            devices: Vec::new(),
            time_constraints: None,
            confidence: 0.0,
        };

        assert_eq!(entities.triggers.len(), 0);
        assert_eq!(entities.conditions.len(), 0);
        assert_eq!(entities.actions.len(), 0);
    }

    #[test]
    fn test_time_constraints() {
        let tc = TimeConstraints {
            start_time: Some("09:00".to_string()),
            end_time: Some("17:00".to_string()),
            days: vec!["mon".to_string(), "tue".to_string()],
        };

        assert_eq!(tc.start_time, Some("09:00".to_string()));
        assert_eq!(tc.days.len(), 2);
    }

    #[test]
    fn test_build_prompt_zh() {
        // Test prompt building with minimal setup
        let (system_role, output_format, examples) = (
            ZH_SYSTEM_ROLE,
            ZH_OUTPUT_FORMAT,
            ZH_EXAMPLES,
        );

        let mut prompt = String::new();
        prompt.push_str(system_role);
        prompt.push_str("\n\n");
        prompt.push_str(output_format);
        prompt.push_str("\n\n");
        prompt.push_str(examples);

        assert!(prompt.contains("物联网自动化专家"));
        assert!(prompt.contains("输出格式"));
    }

    #[test]
    fn test_build_context() {
        let ctx = ExtractionContext {
            available_devices: vec![
                DeviceInfo {
                    name: "温度传感器1".to_string(),
                    device_type: "sensor".to_string(),
                    id: "temp1".to_string(),
                }
            ],
            ..Default::default()
        };

        assert_eq!(ctx.available_devices.len(), 1);
        assert_eq!(ctx.available_devices[0].name, "温度传感器1");
    }
}
