//! System state provider for dynamic context injection.
//!
//! This module provides real-time system state information
//! for LLM context building.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Snapshot of current system state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    /// All devices in the system
    pub devices: Vec<DeviceState>,
    /// All active rules
    pub rules: Vec<RuleSummary>,
    /// All active workflows
    pub workflows: Vec<WorkflowSummary>,
    /// Current alerts
    pub alerts: Vec<AlertSummary>,
    /// System metrics
    pub metrics: SystemMetrics,
    /// Snapshot timestamp
    pub timestamp: i64,
}

impl Default for SystemSnapshot {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            rules: Vec::new(),
            workflows: Vec::new(),
            alerts: Vec::new(),
            metrics: SystemMetrics::default(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

impl SystemSnapshot {
    /// Get a summary text for LLM context.
    pub fn summary(&self) -> String {
        let mut summary = String::new();

        if !self.devices.is_empty() {
            summary.push_str(&format!("设备: {} 个", self.devices.len()));
        }

        if !self.rules.is_empty() {
            if !summary.is_empty() {
                summary.push_str(", ");
            }
            summary.push_str(&format!("规则: {} 条", self.rules.len()));
        }

        if !self.workflows.is_empty() {
            if !summary.is_empty() {
                summary.push_str(", ");
            }
            summary.push_str(&format!("工作流: {} 个", self.workflows.len()));
        }

        if !self.alerts.is_empty() {
            if !summary.is_empty() {
                summary.push_str(", ");
            }
            summary.push_str(&format!("活跃告警: {} 个", self.alerts.len()));
        }

        summary
    }

    /// Find devices by capability.
    pub fn devices_with_capability(&self, capability: &str) -> Vec<&DeviceState> {
        self.devices
            .iter()
            .filter(|d| d.has_capability(capability))
            .collect()
    }

    /// Find devices by location.
    pub fn devices_at_location(&self, location: &str) -> Vec<&DeviceState> {
        self.devices
            .iter()
            .filter(|d| d.location.as_ref().is_some_and(|l| l == location))
            .collect()
    }
}

/// Device state information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    /// Device ID
    pub device_id: String,
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: String,
    /// Location (optional)
    pub location: Option<String>,
    /// Device capabilities
    pub capabilities: Vec<String>,
    /// Current state values
    pub values: Vec<DeviceValue>,
    /// Online status
    pub online: bool,
    /// Last update timestamp
    pub last_update: i64,
}

impl DeviceState {
    /// Check if device has a capability.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|c| c.eq_ignore_ascii_case(capability))
    }

    /// Get current value for a metric.
    pub fn get_value(&self, metric: &str) -> Option<f64> {
        self.values
            .iter()
            .find(|v| v.name.eq_ignore_ascii_case(metric))
            .and_then(|v| v.value)
    }
}

/// Device value/metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceValue {
    /// Value name/metric
    pub name: String,
    /// Current value
    pub value: Option<f64>,
    /// Unit
    pub unit: Option<String>,
    /// Timestamp
    pub timestamp: i64,
}

/// Rule summary for context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSummary {
    /// Rule ID
    pub rule_id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Related device
    pub device_id: String,
    /// Enabled status
    pub enabled: bool,
}

/// Workflow summary for context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummary {
    /// Workflow ID
    pub workflow_id: String,
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: String,
    /// Enabled status
    pub enabled: bool,
}

/// Alert summary for context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSummary {
    /// Alert ID
    pub alert_id: String,
    /// Alert title
    pub title: String,
    /// Severity level
    pub severity: AlertSeverity,
    /// Related device (optional)
    pub device_id: Option<String>,
    /// Acknowledged status
    pub acknowledged: bool,
}

/// Alert severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// System-wide metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemMetrics {
    /// Total device count
    pub total_devices: usize,
    /// Online device count
    pub online_devices: usize,
    /// Active rule count
    pub active_rules: usize,
    /// Total messages processed
    pub total_messages: u64,
    /// System uptime in seconds
    pub uptime_seconds: u64,
}

/// System resource types for discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemResource {
    Device,
    Rule,
    Workflow,
    Alert,
    Metric,
}

/// State provider for current system information.
pub struct StateProvider {
    /// Current cached snapshot
    snapshot: Arc<RwLock<SystemSnapshot>>,
    /// Update timestamp
    last_update: Arc<RwLock<i64>>,
    /// Cache validity in seconds
    cache_validity_secs: i64,
}

impl StateProvider {
    /// Create a new state provider.
    pub fn new() -> Self {
        Self {
            snapshot: Arc::new(RwLock::new(SystemSnapshot::default())),
            last_update: Arc::new(RwLock::new(0)),
            cache_validity_secs: 5, // Cache for 5 seconds
        }
    }

    /// Set cache validity duration.
    pub fn with_cache_validity(mut self, seconds: i64) -> Self {
        self.cache_validity_secs = seconds;
        self
    }

    /// Get current system snapshot.
    pub async fn get_snapshot(&self) -> SystemSnapshot {
        // Check if cache is still valid
        let now = chrono::Utc::now().timestamp();
        let last_update = *self.last_update.read().await;

        if now - last_update < self.cache_validity_secs {
            // Cache is valid, return cached snapshot
            return self.snapshot.read().await.clone();
        }

        // Cache expired, refresh snapshot
        self.refresh_snapshot().await
    }

    /// Refresh the system snapshot from actual sources.
    async fn refresh_snapshot(&self) -> SystemSnapshot {
        let snapshot = SystemSnapshot {
            devices: self.fetch_devices().await,
            rules: self.fetch_rules().await,
            workflows: self.fetch_workflows().await,
            alerts: self.fetch_alerts().await,
            metrics: self.fetch_metrics().await,
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Update cache
        *self.snapshot.write().await = snapshot.clone();
        *self.last_update.write().await = snapshot.timestamp;

        snapshot
    }

    /// Fetch current device states.
    async fn fetch_devices(&self) -> Vec<DeviceState> {
        // In production, this would query the actual device service
        // For now, return sample data
        vec![
            DeviceState {
                device_id: "sensor_1".to_string(),
                name: "客厅温度传感器".to_string(),
                device_type: "dht22_sensor".to_string(),
                location: Some("客厅".to_string()),
                capabilities: vec!["temperature".to_string(), "humidity".to_string()],
                values: vec![
                    DeviceValue {
                        name: "temperature".to_string(),
                        value: Some(23.5),
                        unit: Some("°C".to_string()),
                        timestamp: chrono::Utc::now().timestamp(),
                    },
                    DeviceValue {
                        name: "humidity".to_string(),
                        value: Some(65.0),
                        unit: Some("%".to_string()),
                        timestamp: chrono::Utc::now().timestamp(),
                    },
                ],
                online: true,
                last_update: chrono::Utc::now().timestamp(),
            },
            DeviceState {
                device_id: "sensor_2".to_string(),
                name: "卧室温度传感器".to_string(),
                device_type: "dht22_sensor".to_string(),
                location: Some("卧室".to_string()),
                capabilities: vec!["temperature".to_string(), "humidity".to_string()],
                values: vec![DeviceValue {
                    name: "temperature".to_string(),
                    value: Some(22.0),
                    unit: Some("°C".to_string()),
                    timestamp: chrono::Utc::now().timestamp(),
                }],
                online: true,
                last_update: chrono::Utc::now().timestamp(),
            },
            DeviceState {
                device_id: "light_living_1".to_string(),
                name: "客厅灯".to_string(),
                device_type: "switch".to_string(),
                location: Some("客厅".to_string()),
                capabilities: vec!["power".to_string(), "brightness".to_string()],
                values: vec![DeviceValue {
                    name: "power".to_string(),
                    value: Some(1.0),
                    unit: None,
                    timestamp: chrono::Utc::now().timestamp(),
                }],
                online: true,
                last_update: chrono::Utc::now().timestamp(),
            },
        ]
    }

    /// Fetch current rules.
    async fn fetch_rules(&self) -> Vec<RuleSummary> {
        // In production, this would query the actual rule engine
        vec![RuleSummary {
            rule_id: "rule_1".to_string(),
            name: "高温告警".to_string(),
            description: "当温度超过30度时触发".to_string(),
            device_id: "sensor_1".to_string(),
            enabled: true,
        }]
    }

    /// Fetch current workflows.
    async fn fetch_workflows(&self) -> Vec<WorkflowSummary> {
        // In production, this would query the actual workflow engine
        vec![]
    }

    /// Fetch current alerts.
    async fn fetch_alerts(&self) -> Vec<AlertSummary> {
        // In production, this would query the actual alert system
        vec![]
    }

    /// Fetch system metrics.
    async fn fetch_metrics(&self) -> SystemMetrics {
        SystemMetrics {
            total_devices: 3,
            online_devices: 3,
            active_rules: 1,
            total_messages: 0,
            uptime_seconds: 3600,
        }
    }

    /// Force refresh the cache.
    pub async fn refresh(&self) {
        self.refresh_snapshot().await;
    }
}

impl Default for StateProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_provider() {
        let provider = StateProvider::new();
        let snapshot = provider.get_snapshot().await;

        assert!(!snapshot.devices.is_empty());
        assert_eq!(snapshot.summary(), "设备: 3 个, 规则: 1 条");
    }

    #[tokio::test]
    async fn test_devices_with_capability() {
        let provider = StateProvider::new();
        let snapshot = provider.get_snapshot().await;

        let temp_devices = snapshot.devices_with_capability("temperature");
        assert!(!temp_devices.is_empty());

        let light_devices = snapshot.devices_at_location("客厅");
        assert!(!light_devices.is_empty());
    }

    #[tokio::test]
    async fn test_cache_validity() {
        let provider = StateProvider::new().with_cache_validity(10);

        let snapshot1 = provider.get_snapshot().await;
        let snapshot2 = provider.get_snapshot().await;

        // Should return cached version
        assert_eq!(snapshot1.timestamp, snapshot2.timestamp);
    }
}
