//! Alert escalation management.
//!
//! This module provides escalation rules and management for alerts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Escalation rule configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRule {
    /// Rule name
    pub name: String,
    /// Escalation delay in seconds
    pub delay_seconds: u64,
    /// Target channels for escalation
    pub target_channels: Vec<String>,
    /// Whether the rule is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Escalation configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EscalationConfig {
    /// Escalation rules
    #[serde(default)]
    pub rules: Vec<EscalationRule>,
    /// Default escalation delay in seconds
    #[serde(default = "default_delay")]
    pub default_delay_seconds: u64,
}

fn default_delay() -> u64 {
    300 // 5 minutes
}

/// Record of an escalation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRecord {
    /// Alert ID that was escalated
    pub alert_id: String,
    /// Escalation level
    pub level: u32,
    /// Timestamp of escalation
    pub escalated_at: i64,
    /// Target channels notified
    pub channels: Vec<String>,
}

/// Manages alert escalation.
pub struct EscalationManager {
    /// Configuration
    config: EscalationConfig,
    /// Escalation records
    records: RwLock<HashMap<String, Vec<EscalationRecord>>>,
}

impl EscalationManager {
    /// Create a new escalation manager.
    pub fn new(config: EscalationConfig) -> Self {
        Self {
            config,
            records: RwLock::new(HashMap::new()),
        }
    }

    /// Create with default configuration.
    pub fn default() -> Self {
        Self::new(EscalationConfig::default())
    }

    /// Get the configuration.
    pub fn config(&self) -> &EscalationConfig {
        &self.config
    }

    /// Record an escalation event.
    pub async fn record_escalation(&self, alert_id: &str, level: u32, channels: Vec<String>) {
        let record = EscalationRecord {
            alert_id: alert_id.to_string(),
            level,
            escalated_at: chrono::Utc::now().timestamp(),
            channels,
        };

        let mut records = self.records.write().await;
        records
            .entry(alert_id.to_string())
            .or_insert_with(Vec::new)
            .push(record);
    }

    /// Get escalation history for an alert.
    pub async fn get_history(&self, alert_id: &str) -> Vec<EscalationRecord> {
        self.records
            .read()
            .await
            .get(alert_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Clear escalation history for an alert.
    pub async fn clear_history(&self, alert_id: &str) {
        self.records.write().await.remove(alert_id);
    }
}

impl Default for EscalationRule {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            delay_seconds: 300,
            target_channels: vec![],
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_escalation_manager() {
        let manager = EscalationManager::default();
        
        manager
            .record_escalation("alert-1", 1, vec!["email".to_string()])
            .await;

        let history = manager.get_history("alert-1").await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].level, 1);
    }
}
