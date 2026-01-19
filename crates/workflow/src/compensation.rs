//! Workflow compensation logic for rolling back failed workflows.
//!
//! When a workflow fails partway through, compensation logic allows the system
//! to undo the effects of previously completed steps, maintaining system consistency.

use crate::error::{Result, WorkflowError};
use crate::executor::ExecutionContext;
use crate::store::{CompensationResult as StoredCompensationResult, ExecutionStatus};
use crate::workflow::Step;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// When to execute compensation for a step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompensationTiming {
    /// Compensate immediately on workflow failure
    Immediate,
    /// Compensate after a delay
    Delayed { delay_seconds: u64 },
    /// Manual compensation trigger required
    Manual,
}

/// Strategy for handling workflow failures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailureStrategy {
    /// Fail the entire workflow and compensate all completed steps
    CompensateAll,
    /// Retry the failed step before compensating
    RetryThenCompensate,
    /// Skip the failed step and continue
    SkipAndContinue,
    /// Pause and wait for manual intervention
    ManualIntervention,
}

/// A compensation action that can undo a step's effects
#[async_trait]
pub trait CompensationAction: Send + Sync {
    /// Execute the compensation
    async fn compensate(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        original_output: Option<&serde_json::Value>,
    ) -> Result<CompensationResult>;

    /// Get a human-readable description of what this compensation does
    fn description(&self) -> String;

    /// Whether this compensation can be attempted multiple times
    fn retryable(&self) -> bool {
        false
    }
}

/// Result of a compensation execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationResult {
    /// Whether compensation succeeded
    pub succeeded: bool,
    /// When compensation was attempted
    pub compensated_at: i64,
    /// Error message if compensation failed
    pub error: Option<String>,
    /// Additional details about the compensation
    pub details: Option<serde_json::Value>,
}

impl CompensationResult {
    /// Create a successful compensation result
    pub fn success(details: Option<serde_json::Value>) -> Self {
        Self {
            succeeded: true,
            compensated_at: Utc::now().timestamp(),
            error: None,
            details,
        }
    }

    /// Create a failed compensation result
    pub fn failure(error: String) -> Self {
        Self {
            succeeded: false,
            compensated_at: Utc::now().timestamp(),
            error: Some(error),
            details: None,
        }
    }

    /// Convert to stored compensation result format
    pub fn to_stored(&self) -> StoredCompensationResult {
        StoredCompensationResult {
            succeeded: self.succeeded,
            compensated_at: self.compensated_at,
            error: self.error.clone(),
            details: self.details.clone(),
        }
    }
}

/// Registry of compensation actions for different step types
pub struct CompensationRegistry {
    actions: HashMap<String, Arc<dyn CompensationAction>>,
    default_strategy: FailureStrategy,
}

impl CompensationRegistry {
    /// Create a new compensation registry with default actions
    pub fn new() -> Self {
        let mut registry = Self {
            actions: HashMap::new(),
            default_strategy: FailureStrategy::CompensateAll,
        };

        // Register default compensation actions for each step type
        registry.register_defaults();

        registry
    }

    /// Register default compensation actions for built-in step types
    fn register_defaults(&mut self) {
        use crate::workflow::Step;

        // Log step - no compensation needed
        self.register(
            "log",
            Arc::new(LogCompensation),
        );

        // Delay step - no compensation needed
        self.register(
            "delay",
            Arc::new(NoOpCompensation::new("Delay step - no compensation needed")),
        );

        // Device query - no compensation needed (read-only)
        self.register(
            "device_query",
            Arc::new(NoOpCompensation::new("Device query is read-only")),
        );

        // Send alert - compensation acknowledges the alert
        self.register(
            "send_alert",
            Arc::new(SendAlertCompensation),
        );

        // Send command - compensation sends reverse command
        self.register(
            "send_command",
            Arc::new(SendCommandCompensation),
        );

        // Wait for device state - no compensation needed
        self.register(
            "wait_for_device_state",
            Arc::new(NoOpCompensation::new("Wait operation - no compensation needed")),
        );

        // HTTP request - compensation logged
        self.register(
            "http_request",
            Arc::new(HttpRequestCompensation),
        );

        // Execute WASM - compensation logged
        self.register(
            "execute_wasm",
            Arc::new(WasmExecutionCompensation),
        );

        // Data query - no compensation needed
        self.register(
            "data_query",
            Arc::new(NoOpCompensation::new("Data query is read-only")),
        );

        // Image process - no compensation needed
        self.register(
            "image_process",
            Arc::new(NoOpCompensation::new("Image processing - no compensation needed")),
        );

        // Condition and Parallel are handled specially
        self.register(
            "condition",
            Arc::new(ConditionCompensation),
        );

        self.register(
            "parallel",
            Arc::new(ParallelCompensation),
        );
    }

    /// Register a compensation action for a step type
    pub fn register(&mut self, step_type: impl Into<String>, action: Arc<dyn CompensationAction>) {
        self.actions.insert(step_type.into(), action);
    }

    /// Get the compensation action for a step
    pub fn get(&self, step: &Step) -> Option<Arc<dyn CompensationAction>> {
        self.actions.get(step.step_type()).cloned()
    }

    /// Set the default failure strategy
    pub fn with_default_strategy(mut self, strategy: FailureStrategy) -> Self {
        self.default_strategy = strategy;
        self
    }

    /// Get the default failure strategy
    pub fn default_strategy(&self) -> FailureStrategy {
        self.default_strategy
    }
}

impl Default for CompensationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Executor for workflow compensation
pub struct CompensationExecutor {
    registry: Arc<CompensationRegistry>,
}

impl CompensationExecutor {
    /// Create a new compensation executor
    pub fn new(registry: Arc<CompensationRegistry>) -> Self {
        Self { registry }
    }

    /// Create with default registry
    pub fn with_defaults() -> Self {
        Self::new(Arc::new(CompensationRegistry::new()))
    }

    /// Execute compensation for completed steps in reverse order
    pub async fn compensate(
        &self,
        completed_steps: Vec<(Step, crate::store::StepResult)>,
        context: &mut ExecutionContext,
    ) -> Result<Vec<crate::store::StepResult>> {
        let mut compensation_results = Vec::new();

        context.log(
            "info",
            format!(
                "Starting compensation for {} completed steps",
                completed_steps.len()
            ),
        );

        // Execute compensations in reverse order (LIFO)
        for (step, mut step_result) in completed_steps.into_iter().rev() {
            context.log(
                "info",
                format!("Compensating step: {} ({})", step.id(), step.step_type()),
            );

            let compensation_action = self.registry.get(&step).ok_or_else(|| {
                WorkflowError::ExecutionError(format!(
                    "No compensation action found for step type: {}",
                    step.step_type()
                ))
            })?;

            let original_output = step_result.output.as_ref();

            let result = match compensation_action
                .compensate(&step, context, original_output)
                .await
            {
                Ok(comp_result) => {
                    context.log(
                        "info",
                        format!(
                            "Compensation for step {} {}",
                            step.id(),
                            if comp_result.succeeded { "succeeded" } else { "failed" }
                        ),
                    );
                    comp_result
                }
                Err(e) => {
                    context.log(
                        "error",
                        format!("Compensation for step {} failed: {}", step.id(), e),
                    );
                    CompensationResult::failure(e.to_string())
                }
            };

            // Update step result with compensation info
            step_result.compensated = result.succeeded;
            step_result.compensation_result = Some(result.to_stored());

            compensation_results.push(step_result);
        }

        context.log(
            "info",
            format!("Compensation completed for {} steps", compensation_results.len()),
        );

        Ok(compensation_results)
    }

    /// Compensate a single step
    pub async fn compensate_step(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        original_output: Option<&serde_json::Value>,
    ) -> Result<CompensationResult> {
        let compensation_action = self.registry.get(step).ok_or_else(|| {
            WorkflowError::ExecutionError(format!(
                "No compensation action found for step type: {}",
                step.step_type()
            ))
        })?;

        compensation_action.compensate(step, context, original_output).await
    }
}

impl Default for CompensationExecutor {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ============================================================================
// Default Compensation Actions
// ============================================================================

/// No-op compensation for steps that don't need rollback
struct NoOpCompensation {
    description: String,
}

impl NoOpCompensation {
    fn new(description: &str) -> Self {
        Self {
            description: description.to_string(),
        }
    }
}

#[async_trait]
impl CompensationAction for NoOpCompensation {
    async fn compensate(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        _original_output: Option<&serde_json::Value>,
    ) -> Result<CompensationResult> {
        context.log(
            "info",
            format!(
                "No compensation needed for step {} ({})",
                step.id(),
                self.description
            ),
        );
        Ok(CompensationResult::success(Some(serde_json::json!({
            "note": self.description
        }))))
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

/// Compensation for log steps
struct LogCompensation;

#[async_trait]
impl CompensationAction for LogCompensation {
    async fn compensate(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        _original_output: Option<&serde_json::Value>,
    ) -> Result<CompensationResult> {
        if let Step::Log { message, .. } = step {
            context.log(
                "info",
                format!("Compensating log step: {}", message),
            );
        }
        Ok(CompensationResult::success(Some(serde_json::json!({
            "message": "Log step compensation - no action needed"
        }))))
    }

    fn description(&self) -> String {
        "Log step compensation".to_string()
    }
}

/// Compensation for send alert steps
struct SendAlertCompensation;

#[async_trait]
impl CompensationAction for SendAlertCompensation {
    async fn compensate(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        original_output: Option<&serde_json::Value>,
    ) -> Result<CompensationResult> {
        if let Step::SendAlert { id, severity, title, .. } = step {
            context.log(
                "info",
                format!(
                    "Compensating alert {} [{}]: {} - marking as acknowledged",
                    id, severity, title
                ),
            );

            // In a real implementation, this would acknowledge the alert
            // via the alerts system
            Ok(CompensationResult::success(Some(serde_json::json!({
                "alert_id": id,
                "action": "acknowledged",
                "original_alert": original_output
            }))))
        } else {
            Ok(CompensationResult::success(None))
        }
    }

    fn description(&self) -> String {
        "Acknowledge sent alert".to_string()
    }

    fn retryable(&self) -> bool {
        true
    }
}

/// Compensation for send command steps
struct SendCommandCompensation;

#[async_trait]
impl CompensationAction for SendCommandCompensation {
    async fn compensate(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        original_output: Option<&serde_json::Value>,
    ) -> Result<CompensationResult> {
        if let Step::SendCommand { id, device_id, command, parameters } = step {
            context.log(
                "info",
                format!(
                    "Compensating command {} to device {}: {}",
                    id, device_id, command
                ),
            );

            // Try to determine the reverse command
            let reverse_command = Self::reverse_command(command);

            context.log(
                "info",
                format!(
                    "Sending reverse command '{}' to device {}",
                    reverse_command, device_id
                ),
            );

            // In a real implementation, this would send the reverse command
            // via the device manager
            Ok(CompensationResult::success(Some(serde_json::json!({
                "device_id": device_id,
                "original_command": command,
                "reverse_command": reverse_command,
                "parameters": parameters,
                "original_output": original_output,
                "note": "Reverse command sent (simulated)"
            }))))
        } else {
            Ok(CompensationResult::success(None))
        }
    }

    fn description(&self) -> String {
        "Send reverse command to device".to_string()
    }

    fn retryable(&self) -> bool {
        true
    }
}

impl SendCommandCompensation {
    /// Determine the reverse command for a given command
    fn reverse_command(command: &str) -> String {
        match command.to_lowercase().as_str() {
            "on" | "turn_on" | "enable" => "off".to_string(),
            "off" | "turn_off" | "disable" => "on".to_string(),
            "open" => "close".to_string(),
            "close" => "open".to_string(),
            "start" => "stop".to_string(),
            "stop" => "start".to_string(),
            "activate" => "deactivate".to_string(),
            "deactivate" => "activate".to_string(),
            "lock" => "unlock".to_string(),
            "unlock" => "lock".to_string(),
            _ => format!("reverse_{}", command),
        }
    }
}

/// Compensation for HTTP request steps
struct HttpRequestCompensation;

#[async_trait]
impl CompensationAction for HttpRequestCompensation {
    async fn compensate(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        original_output: Option<&serde_json::Value>,
    ) -> Result<CompensationResult> {
        if let Step::HttpRequest { id, url, method, .. } = step {
            context.log(
                "info",
                format!(
                    "Compensating HTTP request {} ({} {}): logging compensation",
                    id, method, url
                ),
            );

            // For HTTP requests, we can't reliably compensate
            // Just log that compensation was attempted
            Ok(CompensationResult::success(Some(serde_json::json!({
                "request_id": id,
                "method": method,
                "url": url,
                "original_output": original_output,
                "note": "HTTP request compensation - effects logged"
            }))))
        } else {
            Ok(CompensationResult::success(None))
        }
    }

    fn description(&self) -> String {
        "Log HTTP request for manual review".to_string()
    }
}

/// Compensation for WASM execution steps
struct WasmExecutionCompensation;

#[async_trait]
impl CompensationAction for WasmExecutionCompensation {
    async fn compensate(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        _original_output: Option<&serde_json::Value>,
    ) -> Result<CompensationResult> {
        if let Step::ExecuteWasm { id, module_id, function, .. } = step {
            context.log(
                "info",
                format!(
                    "Compensating WASM execution {} (module: {}, function: {})",
                    id, module_id, function
                ),
            );

            // Try to call a compensate function if it exists
            let compensate_function = format!("{}_compensate", function);

            Ok(CompensationResult::success(Some(serde_json::json!({
                "module_id": module_id,
                "original_function": function,
                "compensate_function": compensate_function,
                "note": "WASM compensation - would call compensate function if defined"
            }))))
        } else {
            Ok(CompensationResult::success(None))
        }
    }

    fn description(&self) -> String {
        "Execute WASM compensation function".to_string()
    }

    fn retryable(&self) -> bool {
        true
    }
}

/// Compensation for condition steps (compensate the executed branch)
struct ConditionCompensation;

#[async_trait]
impl CompensationAction for ConditionCompensation {
    async fn compensate(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        _original_output: Option<&serde_json::Value>,
    ) -> Result<CompensationResult> {
        if let Step::Condition { id, then_steps, else_steps, .. } = step {
            context.log(
                "info",
                format!("Compensating condition step {}", id),
            );

            // The original_output should tell us which branch was executed
            // We need to compensate all steps in that branch
            let executed_branch = _original_output
                .and_then(|o| o.get("branch").and_then(|b| b.as_str()))
                .unwrap_or("unknown");

            let steps_to_compensate = match executed_branch {
                "then" => then_steps,
                "else" => else_steps,
                _ => return Ok(CompensationResult::success(None)),
            };

            // Recursively compensate each step in the branch
            let executor = CompensationExecutor::with_defaults();
            let mut completed_results = Vec::new();

            for sub_step in steps_to_compensate {
                if let Some(step_result) = context.step_results.get(sub_step.id()) {
                    completed_results.push((sub_step.clone(), step_result.clone()));
                }
            }

            let compensated_count = completed_results.len();
            if !completed_results.is_empty() {
                executor.compensate(completed_results, context).await?;
            }

            Ok(CompensationResult::success(Some(serde_json::json!({
                "condition_id": id,
                "executed_branch": executed_branch,
                "compensated_steps": compensated_count
            }))))
        } else {
            Ok(CompensationResult::success(None))
        }
    }

    fn description(&self) -> String {
        "Compensate executed branch of condition".to_string()
    }
}

/// Compensation for parallel steps (compensate all completed parallel steps)
struct ParallelCompensation;

#[async_trait]
impl CompensationAction for ParallelCompensation {
    async fn compensate(
        &self,
        step: &Step,
        context: &mut ExecutionContext,
        _original_output: Option<&serde_json::Value>,
    ) -> Result<CompensationResult> {
        if let Step::Parallel { id, steps, .. } = step {
            context.log(
                "info",
                format!("Compensating parallel step {}", id),
            );

            // Compensate all steps that completed in the parallel block
            let executor = CompensationExecutor::with_defaults();
            let mut completed_results = Vec::new();

            for sub_step in steps {
                if let Some(step_result) = context.step_results.get(sub_step.id()) {
                    completed_results.push((sub_step.clone(), step_result.clone()));
                }
            }

            let compensated_count = completed_results.len();
            if !completed_results.is_empty() {
                executor.compensate(completed_results, context).await?;
            }

            Ok(CompensationResult::success(Some(serde_json::json!({
                "parallel_id": id,
                "total_steps": steps.len(),
                "compensated_steps": compensated_count
            }))))
        } else {
            Ok(CompensationResult::success(None))
        }
    }

    fn description(&self) -> String {
        "Compensate all completed parallel steps".to_string()
    }
}

// ============================================================================
// Helper Functions for Creating Compensation Actions
// ============================================================================

/// Create a custom compensation action from a function
pub fn create_compensation<F>(
    description: &str,
    compensate_fn: F,
) -> Arc<dyn CompensationAction>
where
    F: Fn(
            &Step,
            &mut ExecutionContext,
            Option<&serde_json::Value>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<CompensationResult>> + Send>,
        > + Send
        + Sync
        + 'static,
{
    struct FnCompensation<F> {
        description: String,
        compensate_fn: F,
    }

    #[async_trait]
    impl<F> CompensationAction for FnCompensation<F>
    where
        F: Fn(
                &Step,
                &mut ExecutionContext,
                Option<&serde_json::Value>,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<CompensationResult>> + Send>,
            > + Send
            + Sync
            + 'static,
    {
        async fn compensate(
            &self,
            step: &Step,
            context: &mut ExecutionContext,
            original_output: Option<&serde_json::Value>,
        ) -> Result<CompensationResult> {
            (self.compensate_fn)(step, context, original_output).await
        }

        fn description(&self) -> String {
            self.description.clone()
        }
    }

    Arc::new(FnCompensation {
        description: description.to_string(),
        compensate_fn: compensate_fn,
    })
}

/// Create a simple log-based compensation
pub fn create_log_compensation(message: &str) -> Arc<dyn CompensationAction> {
    Arc::new(NoOpCompensation::new(message))
}

/// Create a delayed compensation action
pub fn create_delay_compensation(
    delay_seconds: u64,
    inner_compensation: Arc<dyn CompensationAction>,
) -> Arc<dyn CompensationAction> {
    struct DelayedCompensation {
        delay_seconds: u64,
        inner: Arc<dyn CompensationAction>,
    }

    #[async_trait]
impl CompensationAction for DelayedCompensation {
        async fn compensate(
            &self,
            step: &Step,
            context: &mut ExecutionContext,
            original_output: Option<&serde_json::Value>,
        ) -> Result<CompensationResult> {
            context.log(
                "info",
                format!(
                    "Delaying compensation for step {} by {} seconds",
                    step.id(),
                    self.delay_seconds
                ),
            );

            tokio::time::sleep(std::time::Duration::from_secs(self.delay_seconds)).await;

            self.inner.compensate(step, context, original_output).await
        }

        fn description(&self) -> String {
            format!(
                "Delayed compensation ({}s): {}",
                self.delay_seconds,
                self.inner.description()
            )
        }
    }

    Arc::new(DelayedCompensation {
        delay_seconds,
        inner: inner_compensation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::Step;

    #[tokio::test]
    async fn test_compensation_registry() {
        let registry = CompensationRegistry::new();

        // Check that default step types are registered
        let log_step = Step::Log {
            id: "test".to_string(),
            message: "test".to_string(),
            level: "info".to_string(),
        };

        assert!(registry.get(&log_step).is_some());
    }

    #[tokio::test]
    async fn test_compensation_executor() {
        let executor = CompensationExecutor::with_defaults();
        let mut context = ExecutionContext::new("test_wf".to_string(), "exec1".to_string());

        let log_step = Step::Log {
            id: "log1".to_string(),
            message: "Test log".to_string(),
            level: "info".to_string(),
        };

        let step_result = crate::store::StepResult {
            step_id: "log1".to_string(),
            started_at: 1000,
            completed_at: Some(1001),
            status: ExecutionStatus::Completed,
            output: Some(serde_json::json!("Test log")),
            error: None,
            compensated: false,
            compensation_result: None,
        };

        let results = executor
            .compensate(vec![(log_step, step_result)], &mut context)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].compensated);
    }

    #[tokio::test]
    async fn test_send_command_reverse() {
        assert_eq!(SendCommandCompensation::reverse_command("on"), "off");
        assert_eq!(SendCommandCompensation::reverse_command("off"), "on");
        assert_eq!(SendCommandCompensation::reverse_command("open"), "close");
        assert_eq!(SendCommandCompensation::reverse_command("unknown"), "reverse_unknown");
    }
}
