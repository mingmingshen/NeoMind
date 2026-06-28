//! Fixture JSON schema (spec §4).
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub devices: Vec<serde_json::Value>,
    #[serde(default)]
    pub metrics: Vec<serde_json::Value>,
    #[serde(default)]
    pub rules: Vec<serde_json::Value>,
    #[serde(default)]
    pub agents: Vec<serde_json::Value>,
    #[serde(default)]
    pub transforms: Vec<serde_json::Value>,
    #[serde(default)]
    pub dashboards: Vec<serde_json::Value>,
    #[serde(default)]
    pub channels: Vec<serde_json::Value>,
    #[serde(default)]
    pub extensions: Vec<serde_json::Value>,
}

pub fn load_fixture(path: impl AsRef<Path>) -> anyhow::Result<Fixture> {
    let raw = std::fs::read_to_string(path)?;
    let fix: Fixture = serde_json::from_str(&raw)?;
    Ok(fix)
}
