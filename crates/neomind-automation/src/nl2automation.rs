//! Natural Language to Automation generator.
//!
//! This module extracts entities from natural language descriptions
//! and generates complete RuleAutomation or WorkflowAutomation instances.

use std::sync::Arc;

use crate::error::{AutomationError, Result};
use neomind_core::{LlmRuntime, Message, GenerationParams};
use neomind_core::llm::backend::LlmInput;
use serde_json::json;

/// Natural language to automation converter
pub struct Nl2Automation {
    llm: Arc<dyn LlmRuntime>,
}

impl Nl2Automation {
    /// Create a new NL2Automation converter
    pub fn new(llm: Arc<dyn LlmRuntime>) -> Self {
        Self { llm }
    }

    /// Extract entities from a natural language description
    pub async fn extract_entities(&self, description: &str) -> Result<ExtractedEntities> {
        let prompt = format!(
            r#"Extract the automation entities from the following description.

Description: "{}"

Respond in JSON format:
{{
  "triggers": [
    {{
      "type": "device_state" | "schedule" | "manual",
      "device_id": "device identifier or null",
      "metric": "metric name or null",
      "condition": "condition description or null",
      "cron": "cron expression or null",
      "description": "human-readable trigger description"
    }}
  ],
  "conditions": [
    {{
      "device_id": "device identifier",
      "metric": "metric name",
      "operator": "gt" | "lt" | "eq" | "ne" | "gte" | "lte",
      "threshold": "threshold value as number",
      "description": "condition description"
    }}
  ],
  "actions": [
    {{
      "type": "notify" | "execute_command" | "set_value" | "create_alert",
      "target": "target device or recipient",
      "parameters": {{}},
      "description": "action description"
    }}
  ],
  "devices": ["list of all mentioned devices"],
  "time_constraints": {{
    "start_time": "HH:MM or null",
    "end_time": "HH:MM or null",
    "days": ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] or null
  }}
}}

Guidelines:
- For device_state triggers: extract device_id, metric, and condition
- For schedule triggers: extract cron expression or time constraints
- For conditions: extract device, metric, comparison operator, and threshold value
- For actions: extract the action type and target/parameters
- If information is missing or unclear, use null"#,
            description
        );

        let input = LlmInput {
            messages: vec![
                Message::system("You are an IoT automation expert. Extract structured entities from natural language descriptions. Respond ONLY with valid JSON."),
                Message::user(prompt),
            ],
            params: GenerationParams {
                temperature: Some(0.1),
                max_tokens: Some(1000),
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
}
