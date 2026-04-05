//! Planner module for generating execution plans.
//!
//! Two planners:
//! - `KeywordPlanner` — fast, rule-based, zero LLM cost
//! - `LLMPlanner` — deep, LLM-generated, for complex tasks
//!
//! Coordinator routes between them based on intent confidence and category.

pub mod types;
pub mod keyword;       // Task 2
pub mod llm_planner;   // Task 3
pub mod coordinator;   // Task 5

pub use types::{ExecutionPlan, PlanningConfig, PlanningMode, PlanStep, StepId};
pub use keyword::KeywordPlanner;
pub use llm_planner::LLMPlanner;
pub use coordinator::PlanningCoordinator;
