//! Autonomous Agent framework for LLM-driven decision making.
//!
//! This module provides the autonomous agent that periodically reviews
//! system state and generates decision proposals using LLM.

pub mod agent;
pub mod config;
pub mod context;
pub mod decision;
pub mod review;

pub use agent::{AgentState, AutonomousAgent};
pub use config::{AutonomousConfig, ReviewType};
pub use context::{ContextCollector, EnergyData, MetricAggregation, SystemContext, TimeRange};
pub use decision::{
    Decision, DecisionAction, DecisionEngine, DecisionEngineConfig, DecisionError,
    DecisionPriority, DecisionStatus, DecisionType, ImpactAssessment,
};
pub use review::{ReviewContext, ReviewResult, SystemReview};
