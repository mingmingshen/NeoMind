//! Case JSON schema (see spec §3).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub lang: Lang,
    pub category: String,
    pub workflow: String,
    pub scenario_type: ScenarioType,
    pub description: String,
    pub setup: Setup,
    pub turns: Vec<Turn>,
    pub applies: Vec<String>,
    pub expectations: Expectations,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_queries: Option<Vec<StateQuery>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn as_str(&self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioType {
    SingleTurn,
    MultiTurn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setup {
    pub fixture: String,
    #[serde(default)]
    pub extras: Extras,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extras {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transforms: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dashboards: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expectations {
    pub per_turn: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub overall: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateQuery {
    pub r#type: String,
    pub params: serde_json::Value,
    pub expected: serde_json::Value,
}
