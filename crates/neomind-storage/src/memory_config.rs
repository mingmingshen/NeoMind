//! Memory system configuration
//!
//! Simplified configuration using character limits for memory files.

use serde::{Deserialize, Serialize};

/// Simplified memory system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Whether the memory system is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Storage path for memory files
    #[serde(default = "default_storage_path")]
    pub storage_path: String,
    /// Max chars for USER.md
    #[serde(default = "default_user_limit")]
    pub user_char_limit: usize,
    /// Max chars for KNOWLEDGE.md
    #[serde(default = "default_knowledge_limit")]
    pub knowledge_char_limit: usize,
    /// Max chars per agent summary file
    #[serde(default = "default_agent_limit")]
    pub agent_char_limit: usize,
    /// Max number of agent summary files
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
    /// TTL in days for session temp files
    #[serde(default = "default_ttl")]
    pub temp_file_ttl_days: u64,
    /// Interval in seconds for scheduled jobs
    #[serde(default = "default_schedule_interval")]
    pub schedule_interval_secs: u64,
}

fn default_enabled() -> bool { true }
fn default_storage_path() -> String { "data/memory".to_string() }
fn default_user_limit() -> usize { 2000 }
fn default_knowledge_limit() -> usize { 3000 }
fn default_agent_limit() -> usize { 500 }
fn default_max_agents() -> usize { 5 }
fn default_ttl() -> u64 { 7 }
fn default_schedule_interval() -> u64 { 3600 }

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            storage_path: default_storage_path(),
            user_char_limit: default_user_limit(),
            knowledge_char_limit: default_knowledge_limit(),
            agent_char_limit: default_agent_limit(),
            max_agents: default_max_agents(),
            temp_file_ttl_days: default_ttl(),
            schedule_interval_secs: default_schedule_interval(),
        }
    }
}

impl MemoryConfig {
    /// Configuration file path
    pub const CONFIG_FILE: &'static str = "data/memory_config.json";

    /// Load configuration from file
    pub fn load() -> Self {
        let path = Self::CONFIG_FILE;
        if !std::path::Path::new(path).exists() {
            return Self::default();
        }
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save configuration to file
    pub fn save(&self) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(Self::CONFIG_FILE, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MemoryConfig::default();
        assert!(config.enabled);
        assert_eq!(config.user_char_limit, 2000);
        assert_eq!(config.knowledge_char_limit, 3000);
        assert_eq!(config.agent_char_limit, 500);
        assert_eq!(config.max_agents, 5);
        assert_eq!(config.temp_file_ttl_days, 7);
        assert_eq!(config.schedule_interval_secs, 3600);
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let config = MemoryConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: MemoryConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.user_char_limit, parsed.user_char_limit);
    }

    #[test]
    fn test_old_config_fields_ignored() {
        // Old config JSON with extraction/compression fields should still parse (serde ignores unknown)
        let old_json = r#"{
            "enabled": true,
            "storage_path": "data/memory",
            "user_char_limit": 2000,
            "knowledge_char_limit": 3000,
            "extraction": {"similarity_threshold": 0.85}
        }"#;
        let config: MemoryConfig = serde_json::from_str(old_json).unwrap();
        assert!(config.enabled);
    }
}
