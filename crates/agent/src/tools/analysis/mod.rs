//! Analysis tools for trend analysis, anomaly detection, and decision making.
//!
//! This module provides tools that the LLM can use to analyze system data,
//! detect anomalies, propose decisions, and execute actions.

pub mod anomalies;
pub mod decisions;
pub mod trends;

pub use anomalies::DetectAnomaliesTool;
pub use decisions::{ExecuteDecisionTool, ProposeDecisionTool};
pub use trends::AnalyzeTrendsTool;
