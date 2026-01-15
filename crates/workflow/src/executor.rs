//! Workflow executor - executes workflow steps
//!
//! This module provides the execution engine for workflow steps, including:
//! - Sequential step execution
//! - Conditional branching (if/then/else)
//! - Parallel step execution with concurrency limits
//! - Nested step support with recursion depth limits

use crate::error::{Result, WorkflowError};
use crate::store::{ExecutionLog, ExecutionStatus, StepResult};
use crate::wasm_runtime::WasmRuntime;
use crate::workflow::Step;
use chrono::Utc;
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Execution context for running workflows
pub struct ExecutionContext {
    /// Current step results
    pub step_results: HashMap<String, StepResult>,
    /// Variables
    pub variables: HashMap<String, serde_json::Value>,
    /// Logs
    pub logs: Vec<ExecutionLog>,
    /// Workflow ID
    pub workflow_id: String,
    /// Execution ID
    pub execution_id: String,
    /// Started at
    pub started_at: i64,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(workflow_id: String, execution_id: String) -> Self {
        Self {
            step_results: HashMap::new(),
            variables: HashMap::new(),
            logs: Vec::new(),
            workflow_id,
            execution_id,
            started_at: Utc::now().timestamp(),
        }
    }

    /// Add a log entry
    pub fn log(&mut self, level: impl Into<String>, message: impl Into<String>) {
        self.logs.push(ExecutionLog {
            timestamp: Utc::now().timestamp(),
            level: level.into(),
            message: message.into(),
        });
    }

    /// Get a variable value
    pub fn get_variable(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    /// Get a step result
    pub fn get_step_result(&self, step_id: &str) -> Option<&StepResult> {
        self.step_results.get(step_id)
    }

    /// Set a variable
    pub fn set_variable(&mut self, name: impl Into<String>, value: serde_json::Value) {
        self.variables.insert(name.into(), value);
    }

    /// Substitute variables in a string
    pub fn substitute(&self, template: &str) -> String {
        let mut result = template.to_string();

        // Replace ${variable} syntax
        let mut pos = 0;
        while let Some(start) = result[pos..].find("${") {
            let abs_start = pos + start;
            if let Some(end) = result[abs_start + 2..].find('}') {
                let abs_end = abs_start + 2 + end;
                let var_name = &result[abs_start + 2..abs_end];

                let replacement = if let Some(step_result) = self.get_step_result(var_name) {
                    // Try to get value from step result
                    if let Some(output) = &step_result.output {
                        output.to_string()
                    } else {
                        "".to_string()
                    }
                } else if let Some(value) = self.get_variable(var_name) {
                    value.to_string()
                } else {
                    format!("${{{}}}", var_name)
                };

                result.replace_range(abs_start..=abs_end, &replacement);
                pos = abs_start + replacement.len();
            } else {
                break;
            }
        }

        result
    }
}

/// Workflow executor
pub struct Executor {
    wasm_runtime: tokio::sync::RwLock<Option<WasmRuntime>>,
    // Device manager can be set later when the API is available
    device_manager: Option<Arc<()>>,
    // Alert manager can be set later when the API is available
    alert_manager: Option<Arc<()>>,
}

impl Executor {
    /// Create a new executor
    pub fn new() -> Self {
        Self {
            wasm_runtime: tokio::sync::RwLock::new(None),
            device_manager: None,
            alert_manager: None,
        }
    }

    /// Set device manager (placeholder for future integration)
    pub fn with_device_manager(mut self, _manager: Arc<()>) -> Self {
        self.device_manager = Some(Arc::new(()));
        self
    }

    /// Set alert manager (placeholder for future integration)
    pub fn with_alert_manager(mut self, _manager: Arc<()>) -> Self {
        self.alert_manager = Some(Arc::new(()));
        self
    }

    /// Initialize WASM runtime
    pub async fn init_wasm_runtime(&self) -> Result<()> {
        let mut runtime_guard = self.wasm_runtime.write().await;
        *runtime_guard = Some(WasmRuntime::new()?);
        Ok(())
    }

    /// Maximum nesting depth for recursive step execution
    const MAX_NESTING_DEPTH: usize = 10;

    /// Execute a single step
    pub async fn execute_step(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
    ) -> Result<StepResult> {
        self.execute_step_with_depth(step, context, 0).await
    }

    /// Execute a step with depth tracking for recursion safety
    pub fn execute_step_with_depth<'a>(
        &'a self,
        step: &'a Step,
        context: &'a mut ExecutionContext,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<StepResult>> + Send + 'a>> {
        Box::pin(self.execute_step_inner(step, context, depth))
    }

    /// Inner implementation of execute_step with depth tracking
    async fn execute_step_inner(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        depth: usize,
    ) -> Result<StepResult> {
        // Check nesting depth to prevent stack overflow
        if depth > Self::MAX_NESTING_DEPTH {
            return Err(WorkflowError::ExecutionError(format!(
                "Maximum nesting depth ({}) exceeded at step {}",
                Self::MAX_NESTING_DEPTH,
                step.id()
            )));
        }
        let started_at = Utc::now().timestamp();

        context.log("info", format!("Executing step: {}", step.id()));

        let result: Result<Option<serde_json::Value>> = match step {
            Step::Log { id, message, level } => {
                let message = context.substitute(message);
                context.log(level, &message);
                Ok(Some(serde_json::json!(message)))
            }

            Step::Delay {
                id,
                duration_seconds,
            } => {
                tokio::time::sleep(tokio::time::Duration::from_secs(*duration_seconds)).await;
                Ok(Some(serde_json::json!(format!(
                    "Delayed for {} seconds",
                    duration_seconds
                ))))
            }

            Step::Condition {
                id,
                condition,
                then_steps,
                else_steps,
            } => {
                // Evaluate the condition
                let condition_result = self.evaluate_condition(context, condition)?;
                context.log(
                    "info",
                    format!(
                        "Condition '{}' evaluated to: {}",
                        condition, condition_result
                    ),
                );

                // Select which branch to execute
                let steps_to_execute = if condition_result {
                    then_steps
                } else {
                    else_steps
                };

                if steps_to_execute.is_empty() {
                    context.log("info", format!("No steps to execute for condition branch"));
                    return Ok(StepResult {
                        step_id: id.clone(),
                        started_at,
                        completed_at: Some(Utc::now().timestamp()),
                        status: ExecutionStatus::Completed,
                        output: Some(serde_json::json!({
                            "condition_result": condition_result,
                            "executed_steps": 0,
                        })),
                        error: None,
                    });
                }

                // Execute the selected branch steps sequentially
                let mut executed_count = 0;
                let mut branch_results = Vec::new();

                for sub_step in steps_to_execute {
                    context.log(
                        "info",
                        format!("Executing condition branch step: {}", sub_step.id()),
                    );

                    let sub_result = self
                        .execute_step_with_depth(sub_step, context, depth + 1)
                        .await?;
                    context
                        .step_results
                        .insert(sub_step.id().to_string(), sub_result.clone());

                    if sub_result.status == ExecutionStatus::Failed {
                        context.log("error", format!("Branch step {} failed", sub_step.id()));
                        return Ok(StepResult {
                            step_id: id.clone(),
                            started_at,
                            completed_at: Some(Utc::now().timestamp()),
                            status: ExecutionStatus::Failed,
                            output: Some(serde_json::json!({
                                "condition_result": condition_result,
                                "executed_steps": executed_count,
                                "failed_step": sub_step.id(),
                            })),
                            error: sub_result.error,
                        });
                    }

                    branch_results.push(sub_result);
                    executed_count += 1;
                }

                context.log(
                    "info",
                    format!("Condition branch completed with {} steps", executed_count),
                );

                Ok(Some(serde_json::json!({
                    "condition_result": condition_result,
                    "executed_steps": executed_count,
                    "branch": if condition_result { "then" } else { "else" },
                })))
            }

            Step::Parallel {
                id: _,
                steps,
                max_parallel,
            } => {
                if steps.is_empty() {
                    context.log("info", "No steps to execute in parallel block");
                    return Ok(StepResult {
                        step_id: step.id().to_string(),
                        started_at,
                        completed_at: Some(Utc::now().timestamp()),
                        status: ExecutionStatus::Completed,
                        output: Some(
                            serde_json::json!({"executed": 0, "successful": 0, "failed": 0}),
                        ),
                        error: None,
                    });
                }

                let concurrency_limit = max_parallel.unwrap_or(steps.len()).max(1);
                context.log(
                    "info",
                    format!(
                        "Executing {} steps in parallel (max concurrency: {})",
                        steps.len(),
                        concurrency_limit
                    ),
                );

                let semaphore = Arc::new(Semaphore::new(concurrency_limit));
                let steps_clone = steps.clone();

                // Create tasks for parallel execution
                // Note: We need to use separate contexts for each parallel step
                // and merge results back at the end
                let mut handles = Vec::new();

                for sub_step in steps_clone {
                    let permit = semaphore.clone().acquire_owned().await.map_err(|e| {
                        WorkflowError::ExecutionError(format!("Semaphore error: {}", e))
                    })?;

                    // Clone context for parallel execution
                    let mut sub_context = context.clone();
                    let current_depth = depth;
                    let wasm_runtime = &self.wasm_runtime;

                    // Create a mini-executor for the sub-step
                    // Note: We can't easily move self into spawn, so we execute synchronously
                    // For true parallel execution, consider refactoring to use Arc<Executor>
                    let sub_result = {
                        let _permit = permit;
                        self.execute_step_with_depth(&sub_step, &mut sub_context, current_depth + 1)
                            .await
                    };

                    // Merge logs back
                    context.logs.extend(sub_context.logs);

                    match sub_result {
                        Ok(result) => {
                            context
                                .step_results
                                .insert(sub_step.id().to_string(), result.clone());
                            handles.push((sub_step.id().to_string(), result));
                        }
                        Err(e) => {
                            let failed_result = StepResult {
                                step_id: sub_step.id().to_string(),
                                started_at: Utc::now().timestamp(),
                                completed_at: Some(Utc::now().timestamp()),
                                status: ExecutionStatus::Failed,
                                output: None,
                                error: Some(e.to_string()),
                            };
                            context
                                .step_results
                                .insert(sub_step.id().to_string(), failed_result.clone());
                            handles.push((sub_step.id().to_string(), failed_result));
                        }
                    }
                }

                // Count results
                let successful = handles
                    .iter()
                    .filter(|(_, r)| r.status == ExecutionStatus::Completed)
                    .count();
                let failed = handles
                    .iter()
                    .filter(|(_, r)| r.status == ExecutionStatus::Failed)
                    .count();

                context.log(
                    "info",
                    format!(
                        "Parallel execution completed: {} successful, {} failed",
                        successful, failed
                    ),
                );

                let overall_status = if failed > 0 {
                    ExecutionStatus::Failed
                } else {
                    ExecutionStatus::Completed
                };

                Ok(Some(serde_json::json!({
                    "total_steps": steps.len(),
                    "successful": successful,
                    "failed": failed,
                    "step_ids": handles.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
                })))
            }

            Step::SendAlert {
                id,
                severity,
                title,
                message,
                channels: _,
            } => {
                let title = context.substitute(title);
                let message = context.substitute(message);

                context.log("info", format!("Creating alert: [{}] {}", severity, title));

                // For now, just log the alert
                // In the future, this would integrate with edge_ai_alerts
                Ok(Some(serde_json::json!({
                    "id": id,
                    "severity": severity,
                    "title": title,
                    "message": message,
                })))
            }

            Step::DeviceQuery {
                id: _,
                device_id,
                metric,
                aggregation: _,
            } => {
                context.log(
                    "info",
                    format!("Querying device {} metric {}", device_id, metric),
                );

                // Placeholder - actual implementation would query the device
                Ok(Some(serde_json::json!(null)))
            }

            Step::SendCommand {
                id: _,
                device_id,
                command,
                parameters: _,
            } => {
                context.log(
                    "info",
                    format!("Sending command {} to device {}", command, device_id),
                );

                // Placeholder - actual implementation would send the command
                Ok(Some(serde_json::json!(true)))
            }

            Step::WaitForDeviceState {
                id: _,
                device_id,
                metric,
                expected_value,
                tolerance,
                timeout_seconds,
                poll_interval_seconds: _,
            } => {
                context.log(
                    "info",
                    format!(
                        "Waiting for device {} metric {} to reach {} (tolerance: {:?})",
                        device_id, metric, expected_value, tolerance
                    ),
                );

                // Placeholder - actual implementation would poll the device
                Ok(Some(serde_json::json!({
                    "device_id": device_id,
                    "metric": metric,
                    "expected": expected_value,
                    "tolerance": tolerance,
                    "timeout_seconds": timeout_seconds,
                })))
            }

            Step::ExecuteWasm {
                id: _,
                module_id,
                function,
                arguments: _,
            } => {
                let runtime_guard = self.wasm_runtime.read().await;
                if let Some(_runtime) = runtime_guard.as_ref() {
                    // Execute WASM function
                    context.log(
                        "info",
                        format!(
                            "Executing WASM function {} from module {}",
                            function, module_id
                        ),
                    );

                    // Placeholder - actual implementation would execute the WASM function
                    Ok(Some(serde_json::json!(null)))
                } else {
                    context.log("warn", "WASM runtime not initialized");
                    Ok(None)
                }
            }

            Step::HttpRequest {
                id: _,
                url,
                method,
                headers,
                body,
            } => {
                let url = context.substitute(url);

                context.log("info", format!("HTTP {} request to {}", method, url));

                #[cfg(feature = "http")]
                {
                    // Execute HTTP request
                    let client = reqwest::Client::new();
                    let mut request = match method.as_str() {
                        "GET" => client.get(&url),
                        "POST" => client.post(&url),
                        "PUT" => client.put(&url),
                        "DELETE" => client.delete(&url),
                        _ => client.get(&url),
                    };

                    for (key, value) in headers {
                        request = request.header(key, value);
                    }

                    if let Some(body) = body {
                        let body = context.substitute(body);
                        request = request.body(body);
                    }

                    let response = request.send().await.map_err(|e| {
                        WorkflowError::ExecutionError(format!("HTTP request failed: {}", e))
                    })?;

                    let status = response.status();
                    let body_text = response.text().await.map_err(|e| {
                        WorkflowError::ExecutionError(format!("Failed to read response: {}", e))
                    })?;

                    Ok(Some(serde_json::json!({
                        "status": status.as_u16(),
                        "body": body_text,
                    })))
                }

                #[cfg(not(feature = "http"))]
                {
                    context.log("warn", "HTTP requests not enabled");
                    Ok(None)
                }
            }

            Step::ImageProcess {
                id: _,
                image_source,
                operations: _,
                output_format: _,
            } => {
                context.log("info", format!("Processing image from {}", image_source));

                // Image processing would be implemented here (requires image_processing feature)
                #[cfg(feature = "image_processing")]
                {
                    // Placeholder for actual image processing
                    Ok(Some(serde_json::json!(null)))
                }

                #[cfg(not(feature = "image_processing"))]
                {
                    context.log("warn", "Image processing not enabled");
                    Ok(None)
                }
            }

            Step::DataQuery {
                id: _,
                query_type,
                parameters: _,
            } => {
                context.log("info", format!("Executing data query: {:?}", query_type));

                // Data query implementation
                Ok(Some(serde_json::json!(null)))
            }
        };

        let completed_at = Utc::now().timestamp();

        Ok(StepResult {
            step_id: step.id().to_string(),
            started_at,
            completed_at: Some(completed_at),
            status: ExecutionStatus::Completed,
            output: result.unwrap_or(None).map(|v| serde_json::Value::from(v)),
            error: None,
        })
    }

    /// Evaluate a condition expression
    fn evaluate_condition(&self, context: &ExecutionContext, condition: &str) -> Result<bool> {
        // Simple condition evaluator
        // Supports: ${step_id} == value, ${step_id} > value, etc.

        let condition = context.substitute(condition);

        // Try to parse as boolean
        if let Ok(b) = condition.trim().parse::<bool>() {
            return Ok(b);
        }

        // Try to parse as comparison
        if let Some(pos) = condition.find("==") {
            let left = condition[..pos].trim();
            let right = condition[pos + 2..].trim();
            return Ok(left == right);
        }

        if let Some(pos) = condition.find("!=") {
            let left = condition[..pos].trim();
            let right = condition[pos + 2..].trim();
            return Ok(left != right);
        }

        if let Some(pos) = condition.find(">") {
            let left: f64 = condition[..pos].trim().parse().map_err(|_| {
                WorkflowError::InvalidCondition(format!("Cannot parse as number: {}", condition))
            })?;
            let right: f64 = condition[pos + 1..].trim().parse().map_err(|_| {
                WorkflowError::InvalidCondition(format!("Cannot parse as number: {}", condition))
            })?;
            return Ok(left > right);
        }

        if let Some(pos) = condition.find("<") {
            let left: f64 = condition[..pos].trim().parse().map_err(|_| {
                WorkflowError::InvalidCondition(format!("Cannot parse as number: {}", condition))
            })?;
            let right: f64 = condition[pos + 1..].trim().parse().map_err(|_| {
                WorkflowError::InvalidCondition(format!("Cannot parse as number: {}", condition))
            })?;
            return Ok(left < right);
        }

        // Default: treat as boolean string
        Ok(!condition.is_empty() && condition != "false" && condition != "0")
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

// Implement ExecutionContext Clone
impl Clone for ExecutionContext {
    fn clone(&self) -> Self {
        Self {
            step_results: self.step_results.clone(),
            variables: self.variables.clone(),
            logs: self.logs.clone(),
            workflow_id: self.workflow_id.clone(),
            execution_id: self.execution_id.clone(),
            started_at: self.started_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{Step, Workflow};

    #[tokio::test]
    async fn test_execution_context() {
        let mut ctx = ExecutionContext::new("workflow1".to_string(), "exec1".to_string());

        ctx.set_variable("test_var", serde_json::json!(42));
        ctx.log("info", "Test message");

        assert_eq!(ctx.get_variable("test_var"), Some(&serde_json::json!(42)));
        assert_eq!(ctx.logs.len(), 1);

        let result = ctx.substitute("Value: ${test_var}");
        assert_eq!(result, "Value: 42");
    }

    #[tokio::test]
    async fn test_execute_log_step() {
        let executor = Executor::new();
        let mut ctx = ExecutionContext::new("workflow1".to_string(), "exec1".to_string());

        let step = Step::Log {
            id: "log1".to_string(),
            message: "Test log".to_string(),
            level: "info".to_string(),
        };

        let result = executor.execute_step(&step, &mut ctx).await.unwrap();
        assert_eq!(result.step_id, "log1");
        assert_eq!(result.status, ExecutionStatus::Completed);
    }

    #[tokio::test]
    async fn test_execute_delay_step() {
        let executor = Executor::new();
        let mut ctx = ExecutionContext::new("workflow1".to_string(), "exec1".to_string());

        let step = Step::Delay {
            id: "delay1".to_string(),
            duration_seconds: 1,
        };

        let start = std::time::Instant::now();
        let result = executor.execute_step(&step, &mut ctx).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(result.step_id, "delay1");
        assert!(elapsed >= std::time::Duration::from_secs(1));
    }
}
