//! Planner module for generating execution plans.
//!
//! Two planners:
//! - `KeywordPlanner` — fast, rule-based, zero LLM cost
//! - `LLMPlanner` — deep, LLM-generated, for complex tasks

pub mod types;
// pub mod keyword;       // Task 2
// pub mod llm_planner;   // Task 3
// pub mod coordinator;   // Task 5

pub use types::{ExecutionPlan, PlanningConfig, PlanningMode, PlanStep, StepId};
