//! Intent analysis for determining automation type from natural language
//!
//! This module uses LLM analysis to determine whether a user's description
//! is better suited for a Rule or a Workflow, and provides reasoning.

use std::sync::Arc;

use crate::types::*;
use crate::error::{AutomationError, Result};
use crate::{RuleAutomation, WorkflowAutomation};

use edge_ai_core::{LlmRuntime, Message, GenerationParams};
use edge_ai_core::llm::backend::LlmInput;

/// Intent analyzer for determining automation type
pub struct IntentAnalyzer {
    llm: Arc<dyn LlmRuntime>,
}

impl IntentAnalyzer {
    /// Create a new intent analyzer
    pub fn new(llm: Arc<dyn LlmRuntime>) -> Self {
        Self { llm }
    }

    /// Analyze a natural language description to determine automation type
    pub async fn analyze(&self, description: &str) -> Result<IntentResult> {
        let description = description.trim();
        if description.is_empty() {
            return Ok(IntentResult {
                recommended_type: AutomationType::Rule,
                confidence: 0,
                reasoning: "Empty description".to_string(),
                suggested_automation: None,
                warnings: vec!["Description is empty".to_string()],
            });
        }

        // First, do heuristic analysis for quick response
        let heuristic = self.heuristic_analysis(description);

        // If confidence is high from heuristics, use that
        if heuristic.confidence >= 80 {
            return Ok(heuristic);
        }

        // Otherwise, use LLM for deeper analysis
        self.llm_analysis(description).await
    }

    /// Quick heuristic analysis without LLM
    fn heuristic_analysis(&self, description: &str) -> IntentResult {
        let desc_lower = description.to_lowercase();

        // Indicators for workflow (complex automation)
        let workflow_keywords = [
            "then", "after that", "next", "followed by", "sequence",
            "wait", "delay", "pause", "sleep",
            "if then else", "otherwise", "alternative",
            "loop", "repeat", "for each", "iterate",
            "parallel", "concurrent", "simultaneous",
            "check again", "verify", "validate", "confirm",
            "calculate", "compute", "transform", "convert",
            "analyze", "process", "aggregate",
            "step", "phase", "stage",
        ];

        // Indicators for rule (simple automation)
        let rule_keywords = [
            "when", "if", "whenever", "once",
            "exceeds", "above", "below", "greater", "less",
            "equals", "matches", "is",
            "send alert", "notify", "send message",
            "turn on", "turn off", "switch", "set",
            "trigger", "activate", "deactivate",
        ];

        let mut workflow_score = 0i32;
        let mut rule_score = 0i32;

        // Count workflow indicators
        for keyword in &workflow_keywords {
            if desc_lower.contains(keyword) {
                workflow_score += 10;
            }
        }

        // Count rule indicators
        for keyword in &rule_keywords {
            if desc_lower.contains(keyword) {
                rule_score += 5;
            }
        }

        // Check for sequential language
        if desc_lower.contains(" and then ")
            || desc_lower.contains(", then ")
            || desc_lower.contains(" after ")
        {
            workflow_score += 20;
        }

        // Check for condition complexity
        let condition_count = desc_lower.matches("when").count()
            + desc_lower.matches("if").count()
            + desc_lower.matches("whenever").count();

        if condition_count > 1 {
            workflow_score += 15;
        }

        // Check for action complexity
        let action_count = desc_lower.matches(',').count()
            + desc_lower.matches(" and ").count()
            + desc_lower.split_whitespace().count() / 5;

        if action_count > 5 {
            workflow_score += 10;
        }

        // Determine result
        let (recommended_type, confidence, reasoning) = if workflow_score > rule_score + 20 {
            (
                AutomationType::Workflow,
                (workflow_score - rule_score).min(100) as u8,
                format!(
                    "This appears to be a multi-step automation with {} workflow indicators",
                    workflow_score / 10
                ),
            )
        } else if rule_score > workflow_score + 20 {
            (
                AutomationType::Rule,
                (rule_score - workflow_score).min(100) as u8,
                "This appears to be a simple conditional automation".to_string(),
            )
        } else {
            // Uncertain - default to rule as it's simpler
            (
                AutomationType::Rule,
                50,
                "This could be either a rule or workflow. Starting with a rule is simpler.".to_string(),
            )
        };

        IntentResult {
            recommended_type,
            confidence,
            reasoning,
            suggested_automation: None,
            warnings: if confidence < 60 {
                vec!["Low confidence - consider reviewing the automation type".to_string()]
            } else {
                Vec::new()
            },
        }
    }

    /// LLM-powered analysis for complex cases
    async fn llm_analysis(&self, description: &str) -> Result<IntentResult> {
        let prompt = format!(
            r#"Analyze the following automation description and determine whether it's better implemented as a Rule or a Workflow.

Description: "{}"

Respond in JSON format:
{{
  "recommended_type": "rule" | "workflow",
  "confidence": 0-100,
  "reasoning": "Brief explanation",
  "complexity_indicators": ["list of factors that influenced the decision"],
  "suggested_name": "a clear name for this automation",
  "suggested_description": "a clear description"
}}

Guidelines:
- Recommend "rule" for: single condition checks, immediate actions, simple if-then logic
- Recommend "workflow" for: multiple steps, branching logic, delays/loops, data processing
- Confidence should reflect how clear-cut the choice is"#,
            description
        );

        let input = LlmInput {
            messages: vec![Message::user(prompt)],
            params: GenerationParams {
                temperature: Some(0.3),
                max_tokens: Some(500),
                ..Default::default()
            },
            model: None,
            stream: false,
            tools: None,
        };

        let response = self.llm.generate(input).await?;

        // Parse the JSON response
        let json_str = extract_json_from_response(&response.text)?;
        let analysis: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| AutomationError::IntentAnalysisFailed(format!("Invalid JSON: {}", e)))?;

        let recommended_type = match analysis["recommended_type"].as_str() {
            Some("workflow") => AutomationType::Workflow,
            _ => AutomationType::Rule,
        };

        let confidence = analysis["confidence"].as_u64().unwrap_or(50) as u8;
        let reasoning = analysis["reasoning"].as_str()
            .unwrap_or("AI analysis completed")
            .to_string();

        let suggested_name = analysis["suggested_name"].as_str()
            .unwrap_or("New Automation")
            .to_string();

        let suggested_description = analysis["suggested_description"].as_str()
            .unwrap_or(description)
            .to_string();

        Ok(IntentResult {
            recommended_type,
            confidence: confidence.min(100),
            reasoning,
            suggested_automation: Some(SuggestedAutomation {
                name: suggested_name,
                description: suggested_description,
                automation_type: recommended_type,
                rule: None,
                workflow: None,
                estimated_complexity: if recommended_type == AutomationType::Rule { 1 } else { 3 },
            }),
            warnings: Vec::new(),
        })
    }

    /// Determine if a workflow can be converted to a rule
    pub fn can_convert_to_rule(&self, workflow: &WorkflowAutomation) -> bool {
        // Can convert if:
        // - Single trigger
        // - Single or simple sequential steps (no branching)
        // - No delays longer than a few seconds
        // - No parallel execution
        // - No loops

        if workflow.triggers.len() != 1 {
            return false;
        }

        if workflow.steps.is_empty() || workflow.steps.len() > 3 {
            return false;
        }

        // Check for complex step types
        for step in &workflow.steps {
            match step {
                Step::Condition { .. } => return false,
                Step::Parallel { .. } => return false,
                Step::ForEach { .. } => return false,
                Step::LlmAnalysis { .. } => return false,
                Step::Delay { duration_seconds, .. } => {
                    // Only short delays allowed
                    if *duration_seconds > 10 {
                        return false;
                    }
                }
                _ => {}
            }
        }

        true
    }

    /// Convert a rule to a workflow
    pub fn rule_to_workflow(&self, rule: RuleAutomation) -> WorkflowAutomation {
        let mut workflow = WorkflowAutomation::new(
            format!("{}-as-workflow", rule.metadata.id),
            format!("{} (Workflow)", rule.metadata.name),
        )
        .with_description(rule.metadata.description.clone());

        // Convert trigger
        workflow.triggers.push(rule.trigger.clone());

        // Convert condition to a condition step
        let condition_str = format!(
            "{} {} {}",
            rule.condition.device_id,
            rule.condition.operator.as_str(),
            rule.condition.threshold
        );

        // Convert actions to steps
        let mut action_steps = Vec::new();
        for (i, action) in rule.actions.into_iter().enumerate() {
            let step_id = format!("action_{}", i);
            match action {
                Action::Notify { message } => {
                    action_steps.push(Step::SendAlert {
                        id: step_id,
                        severity: AlertSeverity::Info,
                        title: rule.metadata.name.clone(),
                        message,
                        channels: Vec::new(),
                    });
                }
                Action::ExecuteCommand { device_id, command, parameters } => {
                    action_steps.push(Step::ExecuteCommand {
                        id: step_id,
                        device_id,
                        command,
                        parameters,
                        wait_for_result: Some(true),
                    });
                }
                Action::Log { level, message, .. } => {
                    action_steps.push(Step::SetVariable {
                        id: step_id,
                        name: format!("log_{}", i),
                        value: serde_json::json!({
                            "level": format!("{:?}", level).to_lowercase(),
                            "message": message
                        }),
                    });
                }
                Action::CreateAlert { severity, title, message } => {
                    action_steps.push(Step::SendAlert {
                        id: step_id,
                        severity,
                        title,
                        message,
                        channels: Vec::new(),
                    });
                }
                Action::Delay { duration } => {
                    action_steps.push(Step::Delay {
                        id: step_id,
                        duration_seconds: duration,
                    });
                }
                Action::SetVariable { name, value } => {
                    action_steps.push(Step::SetVariable {
                        id: step_id,
                        name,
                        value,
                    });
                }
            }
        }

        // Create a condition step that wraps the actions
        workflow.steps.push(Step::Condition {
            id: "check_condition".to_string(),
            condition: condition_str,
            then_steps: action_steps,
            else_steps: Vec::new(),
            output_variable: None,
        });

        // Preserve enabled state as a variable
        workflow.variables.insert(
            "enabled".to_string(),
            serde_json::json!(rule.metadata.enabled),
        );

        workflow
    }

    /// Convert a workflow to a rule (if simple enough)
    pub fn workflow_to_rule(&self, workflow: &WorkflowAutomation) -> Option<RuleAutomation> {
        if !self.can_convert_to_rule(workflow) {
            return None;
        }

        // For now, return None if conversion is not straightforward
        // A full implementation would need to map each workflow step to rule actions
        None
    }
}

/// Extract JSON from an LLM response that might have extra text
fn extract_json_from_response(response: &str) -> Result<String> {
    // Find the first { and last }
    let start = response.find('{').ok_or_else(|| {
        AutomationError::IntentAnalysisFailed("No JSON object found in response".to_string())
    })?;

    let end = response.rfind('}').ok_or_else(|| {
        AutomationError::IntentAnalysisFailed("Incomplete JSON object".to_string())
    })?;

    Ok(response[start..=end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edge_ai_core::*;
    use edge_ai_core::llm::backend::{LlmInput, LlmError, StreamChunk};
    use std::pin::Pin;
    use std::result::Result;
    use futures::Stream;

    // Mock LLM for testing - simplified version
    struct MockLlm;

    #[async_trait::async_trait]
    impl LlmRuntime for MockLlm {
        fn backend_id(&self) -> BackendId {
            BackendId::new("mock")
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }

        fn max_context_length(&self) -> usize {
            4096
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }

        async fn generate(&self, _input: LlmInput) -> Result<LlmOutput, LlmError> {
            Ok(LlmOutput {
                text: r#"{"recommended_type": "rule", "confidence": 90, "reasoning": "Simple condition"}"#.to_string(),
                finish_reason: FinishReason::Stop,
                usage: None,
                thinking: None,
            })
        }

        async fn generate_stream(&self, _input: LlmInput) -> Result<Pin<Box<dyn Stream<Item = StreamChunk> + Send>>, LlmError> {
            // Return an empty stream for tests that don't use streaming
            use futures::stream;
            Ok(Box::pin(stream::empty()))
        }
    }

    #[test]
    fn test_heuristic_rule() {
        let analyzer = IntentAnalyzer {
            llm: Arc::new(MockLlm),
        };

        let result = analyzer.heuristic_analysis("When temperature exceeds 30, send an alert");
        assert_eq!(result.recommended_type, AutomationType::Rule);
    }

    #[test]
    fn test_heuristic_workflow() {
        let analyzer = IntentAnalyzer {
            llm: Arc::new(MockLlm),
        };

        let result = analyzer.heuristic_analysis(
            "Check temperature, then wait 5 minutes, if still high send alert, otherwise turn off device"
        );
        assert_eq!(result.recommended_type, AutomationType::Workflow);
    }

    #[test]
    fn test_extract_json() {
        let response = r#"Here's my analysis:
        {
          "recommended_type": "rule",
          "confidence": 90
        }
        Thanks!"#;

        let json = extract_json_from_response(response).unwrap();
        assert!(json.contains("recommended_type"));
    }
}
