//! Memory system configuration
//!
//! Configuration for the LLM-powered memory extraction and compression system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default = "default_storage_path")]
    pub storage_path: String,

    #[serde(default)]
    pub extraction: ExtractionConfig,

    #[serde(default)]
    pub compression: CompressionConfig,

    #[serde(default)]
    pub llm: MemoryLlmConfig,

    #[serde(default)]
    pub schedule: ScheduleConfig,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage_path: "data/memory".to_string(),
            extraction: ExtractionConfig::default(),
            compression: CompressionConfig::default(),
            llm: MemoryLlmConfig::default(),
            schedule: ScheduleConfig::default(),
        }
    }
}

fn default_enabled() -> bool {
    true
}
fn default_storage_path() -> String {
    "data/memory".to_string()
}

/// Extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.85,
        }
    }
}

fn default_similarity_threshold() -> f32 {
    0.85
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    #[serde(default = "default_decay_days")]
    pub decay_period_days: u8,

    #[serde(default = "default_min_importance")]
    pub min_importance: u8,

    #[serde(default = "default_max_entries")]
    pub max_entries: HashMap<String, usize>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        let mut max_entries = HashMap::new();
        max_entries.insert("user_profile".to_string(), 50);
        max_entries.insert("domain_knowledge".to_string(), 100);
        max_entries.insert("task_patterns".to_string(), 80);
        max_entries.insert("system_evolution".to_string(), 30);

        Self {
            decay_period_days: 30,
            min_importance: 20,
            max_entries,
        }
    }
}

fn default_decay_days() -> u8 {
    30
}
fn default_min_importance() -> u8 {
    20
}
fn default_max_entries() -> HashMap<String, usize> {
    CompressionConfig::default().max_entries
}

/// LLM backend configuration for memory operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryLlmConfig {
    /// Backend ID for extraction (lightweight model)
    pub extraction_backend_id: Option<String>,
    /// Backend ID for compression (powerful model)
    pub compression_backend_id: Option<String>,
}

/// Schedule configuration for background tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    #[serde(default = "default_true")]
    pub extraction_enabled: bool,

    #[serde(default = "default_extraction_interval")]
    pub extraction_interval_secs: u64,

    #[serde(default = "default_true")]
    pub compression_enabled: bool,

    #[serde(default = "default_compression_interval")]
    pub compression_interval_secs: u64,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            extraction_enabled: true,
            extraction_interval_secs: 3600,
            compression_enabled: true,
            compression_interval_secs: 86400,
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_extraction_interval() -> u64 {
    3600
}
fn default_compression_interval() -> u64 {
    86400
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
        assert_eq!(config.storage_path, "data/memory");
        assert_eq!(config.extraction.similarity_threshold, 0.85);
        assert_eq!(config.compression.decay_period_days, 30);
        assert_eq!(config.compression.min_importance, 20);
    }

    #[test]
    fn test_config_serialization() {
        let config = MemoryConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: MemoryConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.enabled, parsed.enabled);
        assert_eq!(
            config.extraction.similarity_threshold,
            parsed.extraction.similarity_threshold
        );
    }

    #[test]
    fn test_max_entries_defaults() {
        let config = CompressionConfig::default();
        assert_eq!(config.max_entries.get("user_profile"), Some(&50));
        assert_eq!(config.max_entries.get("domain_knowledge"), Some(&100));
        assert_eq!(config.max_entries.get("task_patterns"), Some(&80));
        assert_eq!(config.max_entries.get("system_evolution"), Some(&30));
    }

    #[test]
    fn test_schedule_defaults() {
        let schedule = ScheduleConfig::default();
        assert!(schedule.extraction_enabled);
        assert!(schedule.compression_enabled);
        assert_eq!(schedule.extraction_interval_secs, 3600);
        assert_eq!(schedule.compression_interval_secs, 86400);
    }
}
