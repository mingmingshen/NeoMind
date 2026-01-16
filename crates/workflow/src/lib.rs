//! Edge AI Workflow Engine Crate
//!
//! This crate provides workflow orchestration and automation capabilities for the NeoTalk platform.
//!
//! ## Features
//!
//! - **Workflow Definition**: Define complex multi-step workflows
//! - **Execution Engine**: Execute workflows with parallel and sequential steps
//! - **Triggers**: Time-based (cron), event-based, and manual triggers
//! - **WASM Runtime**: Execute user-defined code in a sandboxed environment
//! - **Persistence**: Store workflow definitions and execution history
//! - **Image Processing**: Process images from devices (with feature)
//!
//! ## Example
//!
//! ```rust,no_run
//! use edge_ai_workflow::{Workflow, Step, WorkflowEngine};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = WorkflowEngine::new("./data/workflows").await?;
//!
//!     // Create a simple workflow
//!     let workflow = Workflow::new(
//!         "temperature-alert",
//!         "Check temperature and send alert if high",
//!     )
//!     .with_step(Step::DeviceQuery {
//!         id: "read_temp".to_string(),
//!         device_id: "sensor_1".to_string(),
//!         metric: "temperature".to_string(),
//!         aggregation: None,
//!     })
//!     .with_step(Step::Condition {
//!         id: "check_threshold".to_string(),
//!         condition: "${read_temp} > 80".to_string(),
//!         then_steps: vec![
//!             Step::SendAlert {
//!                 id: "send_alert".to_string(),
//!                 severity: "critical".to_string(),
//!                 title: "High Temperature".to_string(),
//!                 message: "Temperature is ${read_temp}°C".to_string(),
//!                 channels: vec![],
//!             }
//!         ],
//!         else_steps: vec![],
//!     });
//!
//!     engine.register_workflow(workflow).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod compiler;
pub mod engine;
pub mod error;
pub mod execution_tracker;
pub mod executor;
pub mod llm_generator;
pub mod scheduler;
pub mod steps;
pub mod store;
pub mod templates;
pub mod trigger;
pub mod triggers;
pub mod wasm_runtime;
pub mod workflow;

pub use compiler::{CompilationResult, MultiLanguageCompiler, SourceLanguage};
pub use engine::{ExecutionResult, WorkflowEngine};
pub use error::{NeoTalkError, Result, WorkflowError};
pub use execution_tracker::{ExecutionPermit, ExecutionState, ExecutionTracker, RunningExecution};
pub use executor::{ExecutionContext, Executor};
pub use llm_generator::{GeneratedWasmCode, GeneratorConfig, WasmCodeGenerator};
pub use scheduler::{ScheduledTask, Scheduler};
pub use steps::{
    AggregationType, DeviceCommandResult, DeviceQueryResult, DeviceState, DeviceWorkflowError,
    DeviceWorkflowIntegration,
};
pub use store::{ExecutionRecord, ExecutionStatus, ExecutionStore, WorkflowStore};
pub use templates::{
    GeneratedWorkflow, SuggestedEdit, TemplateParameter, TemplateParameterType, TemplatedWorkflow,
    ValidationContext, WorkflowGenerator, WorkflowTemplate, WorkflowTemplates,
};
pub use trigger::TriggerManager;
pub use triggers::event::{EventFilters, EventTrigger, EventTriggerConfig, EventTriggerManager};
pub use wasm_runtime::{WasmConfig, WasmModule, WasmRuntime};
pub use workflow::{
    ImageOperation, QueryType, Step, Trigger, TriggerType, Workflow, WorkflowStatus,
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
