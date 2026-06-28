//! I/O records between runner and Claude judge.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRecord {
    pub user: String,
    pub assistant_message: String,
    pub tool_calls: Vec<serde_json::Value>, // each: {name, arguments, result?}
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseRecord {
    pub case_id: String,
    pub lang: String,
    pub turn_records: Vec<TurnRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_queries: Vec<serde_json::Value>,
    pub suspected_fallback: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreLine {
    pub case_id: String,
    pub lang: String,
    pub scores: serde_json::Value, // {dim: int}
    #[serde(default)]
    pub overall_reasoning: String,
    pub judge: String,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub suspected_fallback: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
