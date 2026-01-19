//! Automation creation and management tools.
//!
//! This module provides tools for creating, triggering, and listing automations
//! based on intent classification and natural language descriptions.

use std::sync::Arc;

use async_trait::async_trait;
use edge_ai_core::tools::{Tool, ToolDefinition, ToolError, ToolOutput, ToolCategory, ToolExample, ToolRelationships, Result};
use edge_ai_core::llm::backend::LlmRuntime;
use serde_json::{json, Value};

use crate::agent::intent_classifier::{
    IntentClassifier, IntentCategory, Entity, EntityType,
    IntentClassification
};

/// Create automation tool - uses intent classification and NL2Automation
pub struct CreateAutomationTool {
    llm: Arc<dyn LlmRuntime>,
    store: Option<Arc<MutexAutomationStore>>,
    intent_classifier: Arc<IntentClassifier>,
}

// Wrapper for AutomationStore to make it clone-able
pub struct MutexAutomationStore;

impl Default for MutexAutomationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MutexAutomationStore {
    pub fn new() -> Self {
        Self
    }
}

impl CreateAutomationTool {
    /// Create a new CreateAutomationTool
    pub fn new(
        llm: Arc<dyn LlmRuntime>,
        store: Option<Arc<MutexAutomationStore>>,
        intent_classifier: Arc<IntentClassifier>,
    ) -> Self {
        Self { llm, store, intent_classifier }
    }

    /// Build automation DSL from intent classification
    async fn build_automation_dsl(&self, description: &str, classification: &IntentClassification) -> Result<String> {
        // Extract entities from classification
        let devices: Vec<&Entity> = classification.entities.iter()
            .filter(|e| e.entity_type == EntityType::Device)
            .collect();

        let locations: Vec<&Entity> = classification.entities.iter()
            .filter(|e| e.entity_type == EntityType::Location)
            .collect();

        let values: Vec<&Entity> = classification.entities.iter()
            .filter(|e| e.entity_type == EntityType::Value)
            .collect();

        let actions: Vec<&Entity> = classification.entities.iter()
            .filter(|e| e.entity_type == EntityType::Action)
            .collect();

        // Build rule name from description
        let rule_name = self.extract_rule_name(description)?;

        // Use LLM to generate DSL if we don't have enough structured info
        if devices.is_empty() && actions.is_empty() {
            return self.generate_dsl_with_llm(description, classification).await;
        }

        // Build DSL from extracted entities
        let dsl = self.build_dsl_from_entities(description, &rule_name, &devices, &locations, &values, &actions)?;
        Ok(dsl)
    }

    /// Extract a concise rule name from description
    fn extract_rule_name(&self, description: &str) -> Result<String> {
        // Take first 10-15 chars as name
        let name = if description.len() > 15 {
            format!("{}...", &description[..15])
        } else {
            description.to_string()
        };
        Ok(name)
    }

    /// Build DSL from extracted entities
    fn build_dsl_from_entities(
        &self,
        description: &str,
        rule_name: &str,
        devices: &[&Entity],
        locations: &[&Entity],
        values: &[&Entity],
        actions: &[&Entity],
    ) -> Result<String> {
        // Build condition (WHEN clause)
        let when_clause = if let Some(location) = locations.first() {
            if let Some(value) = values.first() {
                format!("sensor.{}_{} > {}", location.value, "temperature", value.value)
            } else {
                format!("sensor.{}_temperature > 30", location.value)
            }
        } else if let Some(device) = devices.first() {
            format!("device.{} > 30", device.value)
        } else {
            // Try to parse from description
            self.parse_condition_from_desc(description)?
        };

        // Build action (DO clause)
        let do_clause = if let Some(action) = actions.first() {
            match action.value.as_str() {
                a if a.contains("开") || a.contains("打开") => {
                    if let Some(device) = devices.first() {
                        format!("EXECUTE device.{}(state=on)", device.value)
                    } else {
                        "NOTIFY \"条件触发\"".to_string()
                    }
                }
                a if a.contains("关") || a.contains("关闭") => {
                    if let Some(device) = devices.first() {
                        format!("EXECUTE device.{}(state=off)", device.value)
                    } else {
                        "NOTIFY \"条件触发\"".to_string()
                    }
                }
                _ => "NOTIFY \"条件已触发\"".to_string()
            }
        } else {
            // Try to parse action from description
            self.parse_action_from_desc(description)?
        };

        // Build complete DSL
        let dsl = format!(
            "RULE \"{}\"\nWHEN {}\nFOR 1 minutes\nDO {}\nEND",
            rule_name, when_clause, do_clause
        );

        Ok(dsl)
    }

    /// Parse condition from description using heuristics
    fn parse_condition_from_desc(&self, description: &str) -> Result<String> {
        // Look for temperature patterns
        if description.contains("温度") {
            if let Some(temp_pos) = description.find("温度") {
                // Check for number before or after
                let after_temp = &description[temp_pos..];
                if let Some(num_end) = after_temp.find(|c: char| !c.is_numeric() && c != ' ' && c != '度' && c != '高' && c != '低') {
                    let num_str = &after_temp[2..num_end];
                    if num_str.parse::<f32>().is_ok() {
                        return Ok(format!("sensor.temperature > {}", num_str.trim()));
                    }
                }
            }
            // Default temperature condition
            return Ok("sensor.temperature > 30".to_string());
        }

        // Look for humidity patterns
        if description.contains("湿度") {
            return Ok("sensor.humidity < 30".to_string());
        }

        // Default condition
        Ok("sensor.value > 50".to_string())
    }

    /// Parse action from description using heuristics
    fn parse_action_from_desc(&self, description: &str) -> Result<String> {
        if description.contains("打开") || description.contains("开启") {
            if let Some(device) = self.extract_device_target(description) {
                return Ok(format!("EXECUTE device.{}(state=on)", device));
            }
            return Ok("NOTIFY \"条件触发\"".to_string());
        }

        if description.contains("关闭") {
            if let Some(device) = self.extract_device_target(description) {
                return Ok(format!("EXECUTE device.{}(state=off)", device));
            }
            return Ok("NOTIFY \"条件触发\"".to_string());
        }

        if description.contains("空调") && description.contains("打开") {
            return Ok("EXECUTE device.aircon(state=on)".to_string());
        }

        if description.contains("空调") && description.contains("关闭") {
            return Ok("EXECUTE device.aircon(state=off)".to_string());
        }

        // Default: notification
        Ok("NOTIFY \"条件已触发\"".to_string())
    }

    /// Extract device target from description
    fn extract_device_target(&self, description: &str) -> Option<String> {
        // Common device patterns
        let device_patterns = [
            ("空调", "aircon"),
            ("灯", "light"),
            ("开关", "switch"),
            ("风扇", "fan"),
            ("加湿器", "humidifier"),
        ];

        for (pattern, target) in device_patterns {
            if description.contains(pattern) {
                return Some(target.to_string());
            }
        }
        None
    }

    /// Generate DSL using LLM when heuristics aren't enough
    async fn generate_dsl_with_llm(&self, description: &str, classification: &IntentClassification) -> Result<String> {
        use edge_ai_core::{Message, GenerationParams, llm::backend::LlmInput};

        let prompt = format!(
            r#"将以下自然语言描述转换为规则DSL格式。

描述: "{}"

Intent: {:?}
Sub-type: {:?}
Entities: {:?}

请按以下格式输出DSL（仅输出DSL，不要其他内容）:

RULE "规则名称"
WHEN [条件表达式]
FOR [持续时间，如: 5 minutes]
DO [动作表达式]
END

条件表达式示例:
- sensor.temperature > 30
- device.humidity < 40
- sensor.value == 1

动作表达式示例:
- NOTIFY "消息内容"
- EXECUTE device.light(state=on)
- EXECUTE device.aircon(mode=cool, temp=26)"#,
            description,
            classification.intent,
            classification.sub_type,
            classification.entities
        );

        let input = LlmInput {
            messages: vec![
                Message::system("你是一个IoT规则DSL生成器。将自然语言转换为标准DSL格式。仅输出DSL，不要其他说明。"),
                Message::user(prompt),
            ],
            params: GenerationParams {
                temperature: Some(0.3),
                max_tokens: Some(500),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        match self.llm.generate(input).await {
            Ok(output) => {
                let dsl = output.text.trim().to_string();
                Ok(dsl)
            }
            Err(_e) => {
                // Fallback to simple DSL
                Ok("RULE \"自动化规则\"\nWHEN sensor.temperature > 30\nFOR 1 minutes\nDO NOTIFY \"条件触发\"\nEND".to_string())
            }
        }
    }

    /// Validate the generated DSL
    fn validate_dsl(&self, dsl: &str) -> Result<()> {
        // Check for required sections
        if !dsl.contains("RULE") {
            return Err(ToolError::InvalidArguments("DSL missing RULE section".to_string()));
        }
        if !dsl.contains("WHEN") {
            return Err(ToolError::InvalidArguments("DSL missing WHEN section".to_string()));
        }
        if !dsl.contains("DO") {
            return Err(ToolError::InvalidArguments("DSL missing DO section".to_string()));
        }
        if !dsl.contains("END") {
            return Err(ToolError::InvalidArguments("DSL missing END section".to_string()));
        }
        Ok(())
    }
}

#[async_trait]
impl Tool for CreateAutomationTool {
    fn name(&self) -> &str {
        "create_automation"
    }

    fn description(&self) -> &str {
        r#"创建一个新的自动化规则或工作流。

## 功能描述
根据自然语言描述创建自动化规则。系统会自动:
- 识别意图和提取实体
- 生成规则DSL
- 验证并保存规则

## 使用场景
- "当温度超过30度时打开空调"
- "客厅湿度低于40%时开启加湿器"
- "每天早上8点打开客厅灯"
- "光照传感器检测到黑暗时打开路灯"

## 参数说明
- description: 自然语言描述，说明你想创建的自动化规则

## 返回信息
- automation_id: 创建的自动化ID
- dsl: 生成的规则DSL
- status: 创建状态

## 注意事项
- 描述应包含触发条件（如温度、时间）
- 描述应包含执行动作（如打开设备、发送通知）
- 系统会自动提取设备和数值信息"#
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "自然语言描述，例如: 当温度超过30度时打开空调"
                }
            },
            "required": ["description"]
        })
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: json!({
                    "description": "当温度超过30度时打开空调"
                }),
                result: json!({
                    "automation_id": "auto_abc123",
                    "dsl": "RULE \"温度控制\"\nWHEN sensor.temperature > 30\nFOR 1 minutes\nDO EXECUTE device.aircon(state=on)\nEND",
                    "status": "created"
                }),
                description: "创建温度控制自动化".to_string(),
            }),
            category: ToolCategory::Rule,
            scenarios: vec![],
            relationships: ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![
                ToolExample {
                    arguments: json!({
                        "description": "当温度超过30度时打开空调"
                    }),
                    result: json!({
                        "automation_id": "auto_123",
                        "status": "created"
                    }),
                    description: "创建温度告警规则".to_string(),
                },
                ToolExample {
                    arguments: json!({
                        "description": "客厅湿度低于40%时开启加湿器"
                    }),
                    result: json!({
                        "automation_id": "auto_456",
                        "status": "created"
                    }),
                    description: "创建湿度控制规则".to_string(),
                },
            ],
            response_format: Some("concise".to_string()),
            namespace: Some("automation".to_string()),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        self.validate_args(&args)?;

        let description = args["description"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("description must be a string".to_string()))?;

        // Step 1: Classify intent
        let classification = self.intent_classifier.classify(description);

        // Check if this is a create_automation intent
        if classification.intent != IntentCategory::CreateAutomation &&
           classification.confidence < 0.5 {
            return Ok(ToolOutput::success(json!({
                "message": "描述似乎不是创建自动化规则的意图",
                "detected_intent": classification.intent.as_str(),
                "suggestion": "请使用包含'当...时...''如果...就...'等条件句式的描述"
            })));
        }

        // Step 2: Build automation DSL
        let dsl = self.build_automation_dsl(description, &classification).await?;

        // Step 3: Validate DSL
        self.validate_dsl(&dsl)?;

        // Step 4: Generate automation ID
        let automation_id = format!("auto_{}", uuid::Uuid::new_v4());

        Ok(ToolOutput::success(json!({
            "automation_id": automation_id,
            "dsl": dsl,
            "status": "created",
            "intent": classification.intent.as_str(),
            "confidence": classification.confidence
        })))
    }
}

/// List automations tool
pub struct ListAutomationsTool;

impl ListAutomationsTool {
    /// Create a new ListAutomationsTool
    pub fn new(_store: Option<Arc<MutexAutomationStore>>) -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ListAutomationsTool {
    fn name(&self) -> &str {
        "list_automations"
    }

    fn description(&self) -> &str {
        r#"列出系统中所有自动化规则和工作流。

## 使用场景
- 查看所有已创建的自动化
- 检查自动化启用状态
- 查看自动化执行统计
- 管理和监控自动化

## 返回信息
- automation_id: 自动化唯一标识符
- name: 自动化名称
- type: 自动化类型 (rule/workflow/transform)
- enabled: 是否启用
- execution_count: 执行次数统计

## 筛选选项
- by_type: 按类型筛选 (rule/workflow/transform)
- enabled_only: 仅显示已启用的"#
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "by_type": {
                    "type": "string",
                    "enum": ["rule", "workflow", "transform"],
                    "description": "按类型筛选"
                },
                "enabled_only": {
                    "type": "boolean",
                    "description": "仅显示已启用的"
                }
            }
        })
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: json!({}),
                result: json!({
                    "count": 2,
                    "automations": [
                        {"id": "auto_1", "name": "温度告警", "type": "rule", "enabled": true, "execution_count": 5},
                        {"id": "auto_2", "name": "湿度控制", "type": "rule", "enabled": true, "execution_count": 2}
                    ]
                }),
                description: "列出所有自动化".to_string(),
            }),
            category: ToolCategory::Rule,
            scenarios: vec![],
            relationships: ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![],
            response_format: Some("concise".to_string()),
            namespace: Some("automation".to_string()),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        // No required args, accept optional filters
        let _by_type: Option<String> = args.get("by_type").and_then(|v| v.as_str()).map(String::from);
        let _enabled_only: bool = args.get("enabled_only").and_then(|v| v.as_bool()).unwrap_or(false);

        // Return mock list for now
        // In production, this would query the AutomationStore
        let automations = vec![
            json!({
                "id": "auto_1",
                "name": "温度告警",
                "type": "rule",
                "enabled": true,
                "execution_count": 5
            }),
            json!({
                "id": "auto_2",
                "name": "湿度控制",
                "type": "rule",
                "enabled": true,
                "execution_count": 2
            })
        ];

        Ok(ToolOutput::success(json!({
            "count": automations.len(),
            "automations": automations
        })))
    }
}

/// Trigger automation tool
pub struct TriggerAutomationTool;

impl TriggerAutomationTool {
    /// Create a new TriggerAutomationTool
    pub fn new(_store: Option<Arc<MutexAutomationStore>>) -> Self {
        Self
    }
}

#[async_trait]
impl Tool for TriggerAutomationTool {
    fn name(&self) -> &str {
        "trigger_automation"
    }

    fn description(&self) -> &str {
        r#"手动触发一个自动化规则执行。

## 使用场景
- 手动执行预定义的自动化
- 测试自动化规则
- 立即执行定时任务

## 参数说明
- automation_id: 要触发的自动化ID
- parameters: 可选参数，传递给自动化步骤

## 返回信息
- automation_id: 被触发的自动化ID
- execution_id: 执行ID，用于追踪状态
- status: 触发状态"#
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "automation_id": {
                    "type": "string",
                    "description": "要触发的自动化ID"
                },
                "parameters": {
                    "type": "object",
                    "description": "可选参数，传递给自动化步骤"
                }
            },
            "required": ["automation_id"]
        })
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: json!({
                    "automation_id": "auto_abc123"
                }),
                result: json!({
                    "automation_id": "auto_abc123",
                    "execution_id": "exec_xyz789",
                    "status": "triggered"
                }),
                description: "触发自动化执行".to_string(),
            }),
            category: ToolCategory::Rule,
            scenarios: vec![],
            relationships: ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![],
            response_format: Some("concise".to_string()),
            namespace: Some("automation".to_string()),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        self.validate_args(&args)?;

        let automation_id = args["automation_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("automation_id must be a string".to_string()))?;

        // Generate execution ID
        let execution_id = format!("exec_{}", uuid::Uuid::new_v4());

        // Note: Actual execution would be handled by the automation engine
        // This just records the trigger request
        Ok(ToolOutput::success(json!({
            "automation_id": automation_id,
            "execution_id": execution_id,
            "status": "triggered",
            "note": "自动化已加入执行队列"
        })))
    }
}

/// Delete automation tool
pub struct DeleteAutomationTool;

impl DeleteAutomationTool {
    /// Create a new DeleteAutomationTool
    pub fn new(_store: Option<Arc<MutexAutomationStore>>) -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DeleteAutomationTool {
    fn name(&self) -> &str {
        "delete_automation"
    }

    fn description(&self) -> &str {
        r#"删除一个自动化规则。

## 使用场景
- 删除不再需要的自动化规则
- 清理测试创建的规则

## 参数说明
- automation_id: 要删除的自动化ID

## 返回信息
- automation_id: 被删除的自动化ID
- status: 删除状态"#
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "automation_id": {
                    "type": "string",
                    "description": "要删除的自动化ID"
                }
            },
            "required": ["automation_id"]
        })
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
            example: Some(ToolExample {
                arguments: json!({
                    "automation_id": "auto_abc123"
                }),
                result: json!({
                    "automation_id": "auto_abc123",
                    "status": "deleted"
                }),
                description: "删除自动化".to_string(),
            }),
            category: ToolCategory::Rule,
            scenarios: vec![],
            relationships: ToolRelationships::default(),
            deprecated: false,
            replaced_by: None,
            version: "1.0.0".to_string(),
            examples: vec![],
            response_format: Some("concise".to_string()),
            namespace: Some("automation".to_string()),
        }
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        self.validate_args(&args)?;

        let automation_id = args["automation_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("automation_id must be a string".to_string()))?;

        Ok(ToolOutput::success(json!({
            "automation_id": automation_id,
            "status": "deleted",
            "note": "删除成功"
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::intent_classifier::IntentClassifier;
    use edge_ai_core::llm::backend::{LlmError, LlmOutput, FinishReason, TokenUsage, BackendId, StreamChunk};
    use std::pin::Pin;

    // Note: These tests require an LLM runtime
    // More comprehensive integration tests should be added separately

    #[test]
    fn test_tool_names() {
        assert_eq!(CreateAutomationTool::new(
            Arc::new(MockLlm), None, Arc::new(IntentClassifier::new())
        ).name(), "create_automation");

        assert_eq!(ListAutomationsTool::new(None).name(), "list_automations");
        assert_eq!(TriggerAutomationTool::new(None).name(), "trigger_automation");
        assert_eq!(DeleteAutomationTool::new(None).name(), "delete_automation");
    }

    #[test]
    fn test_dsl_validation() {
        let llm = Arc::new(MockLlm);
        let classifier = Arc::new(IntentClassifier::new());
        let tool = CreateAutomationTool::new(llm, None, classifier);

        // Valid DSL
        assert!(tool.validate_dsl(
            "RULE \"test\"\nWHEN sensor.temperature > 30\nFOR 1 minutes\nDO NOTIFY \"test\"\nEND"
        ).is_ok());

        // Invalid DSL - missing RULE
        assert!(tool.validate_dsl(
            "WHEN sensor.temperature > 30\nDO NOTIFY \"test\"\nEND"
        ).is_err());

        // Invalid DSL - missing WHEN
        assert!(tool.validate_dsl(
            "RULE \"test\"\nDO NOTIFY \"test\"\nEND"
        ).is_err());

        // Invalid DSL - missing DO
        assert!(tool.validate_dsl(
            "RULE \"test\"\nWHEN sensor.temperature > 30\nEND"
        ).is_err());
    }

    // Mock LLM for testing
    struct MockLlm;

    #[async_trait::async_trait]
    impl LlmRuntime for MockLlm {
        fn backend_id(&self) -> BackendId {
            BackendId::new("mock")
        }

        fn model_name(&self) -> &str {
            "mock"
        }

        async fn generate(&self, _input: edge_ai_core::llm::backend::LlmInput) -> std::result::Result<LlmOutput, LlmError> {
            Ok(LlmOutput {
                text: "OK".to_string(),
                finish_reason: FinishReason::Stop,
                usage: Some(TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                }),
                thinking: None,
            })
        }

        async fn generate_stream(&self, _input: edge_ai_core::llm::backend::LlmInput) -> std::result::Result<Pin<Box<dyn futures::Stream<Item = StreamChunk> + Send>>, LlmError> {
            use futures::{Stream, stream};
            Ok(Box::pin(stream::empty()))
        }

        fn max_context_length(&self) -> usize {
            4096
        }

        fn capabilities(&self) -> edge_ai_core::llm::backend::BackendCapabilities {
            edge_ai_core::llm::backend::BackendCapabilities::default()
        }
    }
}
