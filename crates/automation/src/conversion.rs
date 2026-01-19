//! Conversion between rules and workflows
//!
//! This module provides utilities for converting between rule and workflow
//! automations when appropriate.

use crate::types::*;

/// Converter for transforming between automation types
pub struct AutomationConverter;

impl AutomationConverter {
    /// Convert a rule automation to a workflow automation
    pub fn rule_to_workflow(rule: RuleAutomation) -> WorkflowAutomation {
        let mut workflow = WorkflowAutomation::new(
            format!("{}-as-workflow", rule.metadata.id),
            rule.metadata.name.clone(),
        )
        .with_description(rule.metadata.description);

        // Copy enabled state and timeout
        workflow.metadata.enabled = rule.metadata.enabled;
        workflow.metadata.execution_count = rule.metadata.execution_count;
        workflow.metadata.last_executed = rule.metadata.last_executed;

        // Convert trigger
        workflow.triggers.push(rule.trigger);

        // Convert condition to a device query step
        let query_step = Step::DeviceQuery {
            id: "check_condition".to_string(),
            device_id: rule.condition.device_id.clone(),
            metric: rule.condition.metric.clone(),
            aggregation: None,
            output_variable: Some("condition_value".to_string()),
        };

        // Build condition expression
        let condition_expr = format!(
            "${{condition_value}} {} {}",
            rule.condition.operator.as_str(),
            rule.condition.threshold
        );

        // Convert actions to steps
        let action_steps: Vec<Step> = rule
            .actions
            .into_iter()
            .enumerate()
            .map(|(i, action)| -> Step {
                let step_id = format!("action_{}", i);
                match action {
                    Action::Notify { message } => Step::SendAlert {
                        id: step_id,
                        severity: AlertSeverity::Info,
                        title: "Notification".to_string(),
                        message,
                        channels: Vec::new(),
                    },
                    Action::ExecuteCommand {
                        device_id,
                        command,
                        parameters,
                    } => Step::ExecuteCommand {
                        id: step_id,
                        device_id,
                        command,
                        parameters,
                        wait_for_result: Some(true),
                    },
                    Action::Log { level, message, .. } => Step::SetVariable {
                        id: step_id,
                        name: format!("log_{}", i),
                        value: serde_json::json!({
                            "level": format!("{:?}", level).to_lowercase(),
                            "message": message
                        }),
                    },
                    Action::CreateAlert {
                        severity,
                        title,
                        message,
                    } => Step::SendAlert {
                        id: step_id,
                        severity,
                        title,
                        message,
                        channels: Vec::new(),
                    },
                    Action::Delay { duration } => Step::Delay {
                        id: step_id,
                        duration_seconds: duration,
                    },
                    Action::SetVariable { name, value } => Step::SetVariable {
                        id: step_id,
                        name,
                        value,
                    },
                }
            })
            .collect();

        // Create condition step
        let condition_step = Step::Condition {
            id: "evaluate_condition".to_string(),
            condition: condition_expr,
            then_steps: action_steps,
            else_steps: Vec::new(),
            output_variable: None,
        };

        workflow.steps.push(query_step);
        workflow.steps.push(condition_step);

        workflow
    }

    /// Attempt to convert a workflow to a rule
    ///
    /// Returns None if the workflow is too complex to be represented as a rule
    pub fn workflow_to_rule(workflow: &WorkflowAutomation) -> Option<RuleAutomation> {
        // Must have exactly one trigger
        if workflow.triggers.len() != 1 {
            return None;
        }

        // Must be simple (no complex steps)
        let trigger = &workflow.triggers[0];

        // Only device state triggers can be rules
        if trigger.r#type != TriggerType::DeviceState {
            return None;
        }

        // Check steps: must be a simple pattern
        // Pattern: DeviceQuery -> Condition -> Actions
        if workflow.steps.len() != 2 && workflow.steps.len() != 3 {
            return None;
        }

        // Extract device and metric from trigger
        let device_id = trigger.device_id.clone()?;
        let metric = trigger.metric.clone()?;

        // Find the condition step
        let condition_step = workflow.steps.iter().find(|s| matches!(s, Step::Condition { .. }))?;

        let (condition_str, then_steps) = match condition_step {
            Step::Condition {
                condition,
                then_steps,
                ..
            } => (condition, then_steps),
            _ => return None,
        };

        // Parse condition to get operator and threshold
        let (operator, threshold) = parse_condition_string(condition_str)?;

        // Convert actions
        let actions: Vec<Action> = then_steps
            .iter()
            .filter_map(|step| -> Option<Action> {
                match step {
                    Step::SendAlert {
                        severity,
                        title,
                        message,
                        ..
                    } => Some(Action::Notify {
                        message: format!("{}: {}", title, message),
                    }),
                    Step::ExecuteCommand {
                        device_id,
                        command,
                        parameters,
                        ..
                    } => Some(Action::ExecuteCommand {
                        device_id: device_id.clone(),
                        command: command.clone(),
                        parameters: parameters.clone(),
                    }),
                    Step::Delay { duration_seconds, .. } => Some(Action::Delay {
                        duration: *duration_seconds,
                    }),
                    Step::SetVariable { name, value, .. } => Some(Action::SetVariable {
                        name: name.clone(),
                        value: value.clone(),
                    }),
                    _ => None,
                }
            })
            .collect();

        // If we have no actions, this isn't a valid rule
        if actions.is_empty() {
            return None;
        }

        let mut rule = RuleAutomation::new(
            format!("{}-as-rule", workflow.metadata.id),
            workflow.metadata.name.clone(),
        )
        .with_description(workflow.metadata.description.clone())
        .with_trigger(trigger.clone())
        .with_condition(Condition::new(
            device_id, metric, operator, threshold,
        ));

        // Copy metadata
        rule.metadata.enabled = workflow.metadata.enabled;
        rule.metadata.execution_count = workflow.metadata.execution_count;
        rule.metadata.last_executed = workflow.metadata.last_executed;

        for action in actions {
            rule = rule.with_action(action);
        }

        Some(rule)
    }

    /// Check if a workflow can be simplified to a rule
    pub fn can_simplify_to_rule(workflow: &WorkflowAutomation) -> bool {
        Self::workflow_to_rule(workflow).is_some()
    }

    /// Get conversion recommendations for an automation
    pub fn get_conversion_recommendation(automation: &Automation) -> ConversionRecommendation {
        match automation {
            Automation::Transform(transform) => {
                // Transforms can be converted to rules/workflows if they have single output
                let output_count = transform.output_metrics().len();
                ConversionRecommendation {
                    can_convert: output_count <= 1,
                    target_type: AutomationType::Rule,
                    reason: if output_count <= 1 {
                        "Transform can be converted to a rule for reactive automation".to_string()
                    } else {
                        "Transform has multiple outputs, consider splitting into separate transforms".to_string()
                    },
                    estimated_complexity: transform.complexity_score(),
                }
            }
            Automation::Rule(rule) => {
                // Rules can always be converted to workflows
                ConversionRecommendation {
                    can_convert: true,
                    target_type: AutomationType::Workflow,
                    reason: "Rules can be converted to workflows for more complex logic".to_string(),
                    estimated_complexity: 2,
                }
            }
            Automation::Workflow(workflow) => {
                if Self::can_simplify_to_rule(workflow) {
                    ConversionRecommendation {
                        can_convert: true,
                        target_type: AutomationType::Rule,
                        reason: "This workflow is simple enough to be a rule for better performance".to_string(),
                        estimated_complexity: 1,
                    }
                } else {
                    ConversionRecommendation {
                        can_convert: false,
                        target_type: AutomationType::Rule,
                        reason: "This workflow is too complex to be represented as a rule".to_string(),
                        estimated_complexity: workflow.complexity_score(),
                    }
                }
            }
        }
    }
}

/// Recommendation for type conversion
#[derive(Debug, Clone)]
pub struct ConversionRecommendation {
    /// Whether conversion is possible
    pub can_convert: bool,
    /// The target type for conversion
    pub target_type: AutomationType,
    /// Reason for the recommendation
    pub reason: String,
    /// Estimated complexity after conversion
    pub estimated_complexity: u8,
}

/// Parse a condition string to extract operator and threshold
///
/// Examples: "condition_value > 50", "${value} >= 100"
fn parse_condition_string(condition: &str) -> Option<(ComparisonOperator, f64)> {
    let condition = condition.trim();

    // Try to find the operator
    let operators = [
        (">=", ComparisonOperator::GreaterThanOrEqual),
        ("<=", ComparisonOperator::LessThanOrEqual),
        ("==", ComparisonOperator::Equal),
    ];

    for (op_str, op) in &operators {
        if let Some(pos) = condition.find(op_str) {
            let parts: Vec<&str> = condition.splitn(2, op_str).collect();
            if parts.len() == 2 {
                let threshold = parts[1].trim().trim_start_matches('$').trim_matches('{').trim_matches('}').trim().parse().ok()?;
                return Some((*op, threshold));
            }
        }
    }

    // Try single character operators
    let single_ops = [">", "<"];
    for op in &single_ops {
        if let Some(pos) = condition.find(op) {
            let parts: Vec<&str> = condition.splitn(2, op).collect();
            if parts.len() == 2 {
                let threshold = parts[1].trim().trim_start_matches('$').trim_matches('{').trim_matches('}').trim().parse().ok()?;
                let comp_op = if *op == ">" {
                    ComparisonOperator::GreaterThan
                } else {
                    ComparisonOperator::LessThan
                };
                return Some((comp_op, threshold));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_condition() {
        let (op, val) = parse_condition_string("value > 50").unwrap();
        assert_eq!(op, ComparisonOperator::GreaterThan);
        assert_eq!(val, 50.0);

        let (op, val) = parse_condition_string("${temp} >= 30.5").unwrap();
        assert_eq!(op, ComparisonOperator::GreaterThanOrEqual);
        assert_eq!(val, 30.5);
    }

    #[test]
    fn test_rule_to_workflow() {
        let rule = RuleAutomation::new("test", "Test Rule")
            .with_trigger(Trigger::device_state("sensor-1", "temperature"))
            .with_condition(Condition::new(
                "sensor-1",
                "temperature",
                ComparisonOperator::GreaterThan,
                30.0,
            ))
            .with_action(Action::Notify {
                message: "High temp!".to_string(),
            });

        let workflow = AutomationConverter::rule_to_workflow(rule);
        assert_eq!(workflow.triggers.len(), 1);
        assert_eq!(workflow.steps.len(), 2); // DeviceQuery + Condition
    }

    #[test]
    fn test_workflow_to_rule_complex() {
        // Complex workflow should not convert
        let workflow = WorkflowAutomation::new("test", "Complex")
            .with_step(Step::Parallel {
                id: "parallel".to_string(),
                steps: vec![],
            });

        assert!(AutomationConverter::workflow_to_rule(&workflow).is_none());
    }
}
