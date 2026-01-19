//! Multi-turn dialogue task orchestration.
//!
//! This module provides the TaskOrchestrator for managing complex automation
//! tasks that require multiple turns of conversation with the user.

use std::sync::Arc;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use edge_ai_core::{Message, GenerationParams, llm::backend::{LlmRuntime, LlmInput}};
use edge_ai_core::tools::{ToolOutput, Result};

use crate::agent::intent_classifier::{IntentClassifier, IntentCategory, IntentClassification};

/// Task orchestrator for managing multi-turn conversations
pub struct TaskOrchestrator {
    llm: Arc<dyn LlmRuntime>,
    intent_classifier: Arc<IntentClassifier>,
    tasks: Arc<RwLock<HashMap<String, TaskSession>>>,
}

impl TaskOrchestrator {
    /// Create a new task orchestrator
    pub fn new(llm: Arc<dyn LlmRuntime>, intent_classifier: Arc<IntentClassifier>) -> Self {
        Self {
            llm,
            intent_classifier,
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a new task session
    pub async fn start_task(&self, user_input: &str, session_id: &str) -> Result<TaskResponse> {
        // Classify the intent
        let classification = self.intent_classifier.classify(user_input);

        // Check if this is a complex task requiring multi-turn
        let strategy = classification.strategy;

        match strategy {
            ProcessingStrategy::MultiTurn => {
                // Create a new task session
                let task_id = format!("task_{}", uuid::Uuid::new_v4().to_string());
                let classification_clone = classification.clone();
                let steps = self.decompose_task(user_input, &classification).await?;
                let session = TaskSession {
                    task_id: task_id.clone(),
                    session_id: session_id.to_string(),
                    original_input: user_input.to_string(),
                    classification: classification_clone,
                    steps,
                    current_step: 0,
                    status: TaskStatus::InProgress,
                    context: TaskContext::default(),
                };

                // Save the session
                self.tasks.write().await.insert(task_id.clone(), session);

                // Get the first step
                let tasks = self.tasks.read().await;
                let task_session = tasks.get(&task_id).unwrap();

                Ok(TaskResponse {
                    task_id: task_id.clone(),
                    response_type: ResponseType::TaskStarted,
                    message: format!("我理解你想创建一个自动化任务。让我们一步步来完成。\n\n第 {} 步: {}",
                        task_session.current_step + 1,
                        task_session.steps.get(task_session.current_step).map(|s| s.description.as_str()).unwrap_or("开始")
                    ),
                    current_step: task_session.steps.get(task_session.current_step).cloned(),
                    total_steps: task_session.steps.len(),
                    needs_input: true,
                    completed: false,
                })
            }
            ProcessingStrategy::FastPath | ProcessingStrategy::Standard | ProcessingStrategy::Quality => {
                // Simple task - can be handled directly
                Ok(TaskResponse {
                    task_id: format!("direct_{}", uuid::Uuid::new_v4().to_string()),
                    response_type: ResponseType::Direct,
                    message: "这是一个简单任务，可以直接处理。".to_string(),
                    current_step: None,
                    total_steps: 0,
                    needs_input: false,
                    completed: true,
                })
            }
            ProcessingStrategy::Fallback => {
                Ok(TaskResponse {
                    task_id: format!("fallback_{}", uuid::Uuid::new_v4().to_string()),
                    response_type: ResponseType::Clarification,
                    message: "我需要更多信息来帮助你。请详细描述你想做什么？".to_string(),
                    current_step: None,
                    total_steps: 0,
                    needs_input: true,
                    completed: false,
                })
            }
        }
    }

    /// Continue an existing task
    pub async fn continue_task(&self, task_id: &str, user_input: &str) -> Result<TaskResponse> {
        let mut tasks = self.tasks.write().await;

        let session = tasks.get_mut(task_id)
            .ok_or_else(|| edge_ai_core::tools::ToolError::InvalidArguments(format!("Task {} not found", task_id)))?;

        if session.status != TaskStatus::InProgress {
            return Ok(TaskResponse {
                task_id: task_id.to_string(),
                response_type: ResponseType::Completed,
                message: "这个任务已经完成或结束了。".to_string(),
                current_step: None,
                total_steps: session.steps.len(),
                needs_input: false,
                completed: true,
            });
        }

        // Process the current step with user input
        let current_step_idx = session.current_step;
        let current_step = session.steps.get(current_step_idx)
            .ok_or_else(|| edge_ai_core::tools::ToolError::ExecutionFailed("No more steps".to_string()))?;

        // Update context based on user input
        self.update_context_from_input(&mut session.context, user_input, current_step);

        // Check if the step is complete
        let step_complete = self.is_step_complete(current_step, user_input);

        if step_complete {
            // Move to next step
            session.current_step += 1;

            if session.current_step >= session.steps.len() {
                // All steps complete
                session.status = TaskStatus::Completed;
                return Ok(TaskResponse {
                    task_id: task_id.to_string(),
                    response_type: ResponseType::Completed,
                    message: "太好了！所有步骤都完成了，自动化已经创建。".to_string(),
                    current_step: None,
                    total_steps: session.steps.len(),
                    needs_input: false,
                    completed: true,
                });
            }

            // Return next step
            let next_step = session.steps.get(session.current_step).unwrap();
            return Ok(TaskResponse {
                task_id: task_id.to_string(),
                response_type: ResponseType::NextStep,
                message: format!("好的，第 {} 步完成了。\n\n第 {} 步: {}",
                    current_step_idx + 1,
                    session.current_step + 1,
                    next_step.description
                ),
                current_step: Some(next_step.clone()),
                total_steps: session.steps.len(),
                needs_input: true,
                completed: false,
            });
        }

        // Need more input for current step
        Ok(TaskResponse {
            task_id: task_id.to_string(),
            response_type: ResponseType::NeedsInput,
            message: self.get_prompt_for_step(current_step, user_input),
            current_step: Some(current_step.clone()),
            total_steps: session.steps.len(),
            needs_input: true,
            completed: false,
        })
    }

    /// Get the current state of a task
    pub async fn get_task_state(&self, task_id: &str) -> Option<TaskSession> {
        self.tasks.read().await.get(task_id).cloned()
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;

        let session = tasks.get_mut(task_id)
            .ok_or_else(|| edge_ai_core::tools::ToolError::InvalidArguments(format!("Task {} not found", task_id)))?;

        session.status = TaskStatus::Cancelled;
        Ok(())
    }

    /// Decompose a complex task into steps
    async fn decompose_task(&self, input: &str, classification: &IntentClassification) -> Result<Vec<TaskStep>> {
        // Use LLM to decompose the task into steps
        let prompt = format!(
            r#"将以下自动化任务分解为具体的步骤。

用户输入: "{}"

识别的意图: {:?}
识别的实体: {:?}

请将这个任务分解为3-5个具体的步骤。每个步骤应该包括：
1. 步骤类型 (gather_info/confirm/execute)
2. 步骤描述（简洁明了）
3. 需要的信息（可选）

以JSON格式返回:
{{
  "steps": [
    {{"type": "gather_info", "description": "...", "prompt": "..."}},
    {{"type": "confirm", "description": "...", "details": "..."}},
    {{"type": "execute", "description": "...", "action": "..."}}
  ]
}}

步骤类型说明:
- gather_info: 收集用户信息（如设备名称、阈值等）
- confirm: 确认信息或选择
- execute: 执行最终操作"#,
            input,
            classification.intent,
            classification.entities
        );

        let llm_input = LlmInput {
            messages: vec![
                Message::system("你是一个任务分解专家，帮助将复杂的自动化任务分解为简单步骤。"),
                Message::user(prompt),
            ],
            params: GenerationParams {
                temperature: Some(0.3),
                max_tokens: Some(1000),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        match self.llm.generate(llm_input).await {
            Ok(output) => {
                // Parse JSON from LLM response
                if let Some(json_start) = output.text.find('{') {
                    if let Some(json_end) = output.text.rfind('}') {
                        let json_str = &output.text[json_start..=json_end];
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                            if let Some(steps) = parsed.get("steps").and_then(|s| s.as_array()) {
                                let task_steps: Vec<TaskStep> = steps.iter()
                                    .filter_map(|s| self.parse_step(s))
                                    .collect();

                                if !task_steps.is_empty() {
                                    return Ok(task_steps);
                                }
                            }
                        }
                    }
                }

                // Fallback to default steps
                Ok(self.default_steps_for_intent(classification))
            }
            Err(_) => {
                Ok(self.default_steps_for_intent(classification))
            }
        }
    }

    fn parse_step(&self, step: &serde_json::Value) -> Option<TaskStep> {
        let step_type = step.get("type")?.as_str()?;
        let description = step.get("description")?.as_str()?.to_string();

        let step_type = match step_type {
            "gather_info" => StepType::GatherInfo,
            "confirm" => StepType::Confirm,
            "execute" => StepType::Execute,
            _ => StepType::GatherInfo,
        };

        Some(TaskStep {
            step_type,
            description,
            prompt: step.get("prompt").and_then(|p| p.as_str()).map(String::from),
            details: step.get("details").and_then(|d| d.as_str()).map(String::from),
            action: step.get("action").and_then(|a| a.as_str()).map(String::from),
        })
    }

    fn default_steps_for_intent(&self, classification: &IntentClassification) -> Vec<TaskStep> {
        match classification.intent {
            IntentCategory::CreateAutomation => vec![
                TaskStep {
                    step_type: StepType::GatherInfo,
                    description: "确认触发条件".to_string(),
                    prompt: Some("请告诉我触发条件是什么？例如：温度超过30度、每天早上8点等。".to_string()),
                    details: None,
                    action: None,
                },
                TaskStep {
                    step_type: StepType::GatherInfo,
                    description: "确认执行动作".to_string(),
                    prompt: Some("请告诉我要执行什么动作？例如：打开空调、发送通知等。".to_string()),
                    details: None,
                    action: None,
                },
                TaskStep {
                    step_type: StepType::Confirm,
                    description: "确认规则设置".to_string(),
                    prompt: None,
                    details: Some("请确认以上设置是否正确？".to_string()),
                    action: None,
                },
                TaskStep {
                    step_type: StepType::Execute,
                    description: "创建自动化".to_string(),
                    prompt: None,
                    details: None,
                    action: Some("create_automation".to_string()),
                },
            ],
            _ => vec![
                TaskStep {
                    step_type: StepType::GatherInfo,
                    description: "了解更多信息".to_string(),
                    prompt: Some("请提供更多详细信息。".to_string()),
                    details: None,
                    action: None,
                },
            ],
        }
    }

    fn update_context_from_input(&self, context: &mut TaskContext, input: &str, step: &TaskStep) {
        match step.step_type {
            StepType::GatherInfo => {
                if input.contains("温度") || input.contains("湿度") || input.contains("度") {
                    context.condition = Some(input.to_string());
                } else if input.contains("打开") || input.contains("关闭") {
                    context.action = Some(input.to_string());
                } else if input.contains("确认") || input.contains("好的") || input.contains("对") {
                    context.confirmed = true;
                }
            }
            StepType::Confirm => {
                if input.contains("确认") || input.contains("好的") || input.contains("对") || input.contains("是") {
                    context.confirmed = true;
                } else {
                    context.confirmed = false;
                }
            }
            StepType::Execute => {
                // No context update needed for execute step
            }
        }
    }

    fn is_step_complete(&self, step: &TaskStep, input: &str) -> bool {
        match step.step_type {
            StepType::GatherInfo => {
                // Check if user provided meaningful input
                input.len() > 2 && !input.contains("?") && !input.contains("？")
            }
            StepType::Confirm => {
                input.contains("确认") || input.contains("好的") || input.contains("对") || input.contains("是") || input.contains("行")
            }
            StepType::Execute => {
                // Execute step is always complete
                true
            }
        }
    }

    fn get_prompt_for_step(&self, step: &TaskStep, input: &str) -> String {
        if let Some(prompt) = &step.prompt {
            return prompt.clone();
        }

        match step.step_type {
            StepType::GatherInfo => {
                if input.contains("?") || input.contains("？") {
                    "请提供具体信息。".to_string()
                } else {
                    format!("请提供关于 '{}' 的更多信息。", step.description)
                }
            }
            StepType::Confirm => {
                "请确认是否正确？回复 '确认' 继续，或提供修改意见。".to_string()
            }
            StepType::Execute => {
                "正在执行...".to_string()
            }
        }
    }
}

/// Task session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSession {
    /// Unique task identifier
    pub task_id: String,
    /// Session identifier
    pub session_id: String,
    /// Original user input
    pub original_input: String,
    /// Intent classification
    pub classification: IntentClassification,
    /// Task steps
    pub steps: Vec<TaskStep>,
    /// Current step index
    pub current_step: usize,
    /// Task status
    pub status: TaskStatus,
    /// Accumulated context
    pub context: TaskContext,
}

/// Task step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    /// Step type
    pub step_type: StepType,
    /// Step description
    pub description: String,
    /// Optional prompt for user input
    pub prompt: Option<String>,
    /// Additional details
    pub details: Option<String>,
    /// Action to execute (for execute steps)
    pub action: Option<String>,
}

/// Step type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepType {
    /// Gather information from user
    GatherInfo,
    /// Confirm with user
    Confirm,
    /// Execute action
    Execute,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is in progress
    InProgress,
    /// Task is completed
    Completed,
    /// Task is cancelled
    Cancelled,
    /// Task failed
    Failed,
}

/// Task context accumulated during conversation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskContext {
    /// Identified condition (if any)
    pub condition: Option<String>,
    /// Identified action (if any)
    pub action: Option<String>,
    /// Whether user confirmed
    pub confirmed: bool,
    /// Additional collected data
    pub extra_data: HashMap<String, String>,
}

/// Response from task orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    /// Task ID
    pub task_id: String,
    /// Response type
    pub response_type: ResponseType,
    /// Message to user
    pub message: String,
    /// Current step (if any)
    pub current_step: Option<TaskStep>,
    /// Total number of steps
    pub total_steps: usize,
    /// Whether input is needed
    pub needs_input: bool,
    /// Whether task is completed
    pub completed: bool,
}

/// Response type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseType {
    /// Task started
    TaskStarted,
    /// Next step
    NextStep,
    /// Need more input
    NeedsInput,
    /// Task completed
    Completed,
    /// Direct (single-turn) response
    Direct,
    /// Clarification needed
    Clarification,
}

/// Processing strategy (re-export from intent_classifier)
pub use crate::agent::intent_classifier::ProcessingStrategy;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_session_creation() {
        use crate::agent::intent_classifier::{IntentCategory, IntentSubType, ProcessingStrategy};

        let task = TaskSession {
            task_id: "task_1".to_string(),
            session_id: "session_1".to_string(),
            original_input: "创建一个自动化".to_string(),
            classification: IntentClassification {
                intent: IntentCategory::CreateAutomation,
                sub_type: IntentSubType::SimpleRule,
                confidence: 0.8,
                entities: vec![],
                strategy: ProcessingStrategy::MultiTurn,
                needs_followup: false,
                followup_prompt: None,
                capability_statement: None,
            },
            steps: vec![],
            current_step: 0,
            status: TaskStatus::InProgress,
            context: TaskContext::default(),
        };

        assert_eq!(task.task_id, "task_1");
        assert_eq!(task.current_step, 0);
        assert_eq!(task.status, TaskStatus::InProgress);
    }

    #[test]
    fn test_task_context_default() {
        let context = TaskContext::default();
        assert!(context.condition.is_none());
        assert!(context.action.is_none());
        assert!(!context.confirmed);
        assert!(context.extra_data.is_empty());
    }

    #[test]
    fn test_step_type() {
        let gather = StepType::GatherInfo;
        let confirm = StepType::Confirm;
        let execute = StepType::Execute;

        assert_eq!(gather as i32, 0);
        assert_eq!(confirm as i32, 1);
        assert_eq!(execute as i32, 2);
    }
}
