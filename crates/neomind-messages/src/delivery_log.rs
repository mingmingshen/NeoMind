//! Delivery log for tracking DataPush messages.
//!
//! Records every data push attempt for debugging and auditing purposes.
//! Retention: 1 day (auto cleanup)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique delivery log ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeliveryLogId(pub String);

impl DeliveryLogId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Default for DeliveryLogId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DeliveryLogId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Delivery status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum DeliveryStatus {
    /// Waiting to be sent
    #[default]
    Pending,
    /// Successfully delivered
    Success,
    /// Final failure after retries
    Failed,
    /// Currently retrying
    Retrying,
}

impl DeliveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Retrying => "retrying",
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Some(Self::Pending),
            "success" => Some(Self::Success),
            "failed" => Some(Self::Failed),
            "retrying" => Some(Self::Retrying),
            _ => None,
        }
    }
}


/// Delivery log entry for tracking DataPush messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryLog {
    /// Unique log ID
    pub id: DeliveryLogId,
    /// Original message/event ID
    pub event_id: String,
    /// Target channel name
    pub channel_name: String,
    /// Delivery status
    pub status: DeliveryStatus,
    /// Payload summary (truncated for display)
    #[serde(default)]
    pub payload_summary: String,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Number of retry attempts
    #[serde(default)]
    pub retry_count: u32,
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// When the log was created
    pub created_at: DateTime<Utc>,
    /// When the log was last updated
    pub updated_at: DateTime<Utc>,
}

fn default_max_retries() -> u32 {
    3
}

impl DeliveryLog {
    /// Create a new delivery log entry.
    pub fn new(event_id: String, channel_name: String, payload_summary: String) -> Self {
        let now = Utc::now();
        Self {
            id: DeliveryLogId::new(),
            event_id,
            channel_name,
            status: DeliveryStatus::Pending,
            payload_summary,
            error_message: None,
            retry_count: 0,
            max_retries: 3,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the delivery status.
    pub fn with_status(mut self, status: DeliveryStatus) -> Self {
        self.status = status;
        self.updated_at = Utc::now();
        self
    }

    /// Set the error message.
    pub fn with_error(mut self, error: String) -> Self {
        self.error_message = Some(error);
        self.updated_at = Utc::now();
        self
    }

    /// Increment retry count.
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.updated_at = Utc::now();
    }

    /// Check if more retries are available.
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && self.status != DeliveryStatus::Success
    }

    /// Check if this log entry is expired (older than retention period).
    pub fn is_expired(&self, retention_days: i64) -> bool {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);
        self.created_at < cutoff
    }

    /// Create a summary for display.
    pub fn summary(&self) -> String {
        format!(
            "[{}] {} -> {} ({})",
            self.status.as_str(),
            self.event_id,
            self.channel_name,
            if self.error_message.is_some() {
                "failed"
            } else {
                "ok"
            }
        )
    }
}

/// Delivery log statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeliveryStats {
    pub total: usize,
    pub pending: usize,
    pub success: usize,
    pub failed: usize,
    pub retrying: usize,
}

/// Query parameters for delivery logs.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeliveryLogQuery {
    /// Filter by channel name
    pub channel: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by event ID
    pub event_id: Option<String>,
    /// Hours to look back (default: 24)
    pub hours: Option<i64>,
    /// Maximum results (default: 100)
    pub limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_log_id() {
        let id1 = DeliveryLogId::new();
        let id2 = DeliveryLogId::new();
        assert_ne!(id1, id2);

        let id_str = id1.to_string();
        let parsed = DeliveryLogId::from_string(&id_str);
        assert_eq!(id1, parsed);
    }

    #[test]
    fn test_delivery_status() {
        assert_eq!(
            DeliveryStatus::from_string("success"),
            Some(DeliveryStatus::Success)
        );
        assert_eq!(
            DeliveryStatus::from_string("failed"),
            Some(DeliveryStatus::Failed)
        );
        assert_eq!(DeliveryStatus::from_string("invalid"), None);
    }

    #[test]
    fn test_delivery_log_creation() {
        let log = DeliveryLog::new(
            "event_123".to_string(),
            "webhook_1".to_string(),
            r#"{"temp":85}"#.to_string(),
        );

        assert_eq!(log.event_id, "event_123");
        assert_eq!(log.channel_name, "webhook_1");
        assert_eq!(log.status, DeliveryStatus::Pending);
        assert_eq!(log.retry_count, 0);
        assert!(log.can_retry());
    }

    #[test]
    fn test_delivery_log_with_status() {
        let log = DeliveryLog::new(
            "event_123".to_string(),
            "webhook_1".to_string(),
            String::new(),
        )
        .with_status(DeliveryStatus::Success);

        assert_eq!(log.status, DeliveryStatus::Success);
        assert!(!log.can_retry()); // Success means no retry needed
    }

    #[test]
    fn test_delivery_log_with_error() {
        let log = DeliveryLog::new(
            "event_123".to_string(),
            "webhook_1".to_string(),
            String::new(),
        )
        .with_status(DeliveryStatus::Failed)
        .with_error("Connection timeout".to_string());

        assert_eq!(log.status, DeliveryStatus::Failed);
        assert_eq!(log.error_message, Some("Connection timeout".to_string()));
    }

    #[test]
    fn test_delivery_log_retry() {
        let mut log = DeliveryLog::new(
            "event_123".to_string(),
            "webhook_1".to_string(),
            String::new(),
        );

        assert!(log.can_retry());

        log.increment_retry();
        assert_eq!(log.retry_count, 1);
        assert!(log.can_retry());

        // Retry until max
        log.increment_retry();
        log.increment_retry();
        assert_eq!(log.retry_count, 3);
        assert!(!log.can_retry()); // Max retries reached
    }

    #[test]
    fn test_delivery_log_expiration() {
        let log = DeliveryLog::new(
            "event_123".to_string(),
            "webhook_1".to_string(),
            String::new(),
        );

        // Fresh log should not be expired
        assert!(!log.is_expired(1));
    }

    #[test]
    fn test_delivery_log_serialization() {
        let log = DeliveryLog::new(
            "event_123".to_string(),
            "webhook_1".to_string(),
            r#"{"data":42}"#.to_string(),
        )
        .with_status(DeliveryStatus::Success);

        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("event_123"));
        assert!(json.contains("webhook_1"));
        assert!(json.contains("success"));

        let parsed: DeliveryLog = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_id, log.event_id);
        assert_eq!(parsed.status, DeliveryStatus::Success);
    }

    #[test]
    fn test_delivery_stats() {
        let stats = DeliveryStats {
            total: 100,
            pending: 10,
            success: 80,
            failed: 5,
            retrying: 5,
        };

        assert_eq!(stats.total, 100);
    }

    #[test]
    fn test_delivery_log_query() {
        let query = DeliveryLogQuery {
            channel: Some("webhook_1".to_string()),
            status: Some("failed".to_string()),
            hours: Some(48),
            limit: Some(50),
            event_id: None,
        };

        assert_eq!(query.channel, Some("webhook_1".to_string()));
        assert_eq!(query.hours, Some(48));
    }
}
