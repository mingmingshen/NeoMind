# Messages Module

**Package**: `neomind-messages`
**Version**: 0.5.8
**Completion**: 70%
**Purpose**: Message notification system

## Overview

The Messages module is responsible for creating, distributing, and managing system messages, alerts, and notifications.

## Module Structure

```
crates/messages/src/
├── lib.rs                      # Public interface
├── channels/                   # Notification channels
│   ├── mod.rs
│   ├── webhook.rs              # Webhook channel
│   └── email.rs                # Email channel
├── store.rs                    # Message storage
├── notifier.rs                 # Notifier
└── types.rs                    # Type definitions
```

## Core Types

### 1. Message - Message Definition

```rust
pub struct Message {
    /// Message ID
    pub id: String,

    /// Message type
    pub message_type: MessageType,

    /// Title
    pub title: String,

    /// Content
    pub content: String,

    /// Severity level
    pub severity: MessageSeverity,

    /// Message status
    pub status: MessageStatus,

    /// Associated entity
    pub entity_id: Option<String>,

    /// Created at
    pub created_at: i64,

    /// Updated at
    pub updated_at: i64,

    /// Metadata
    pub metadata: serde_json::Value,
}

pub enum MessageType {
    /// Device event
    Device,

    /// Rule triggered
    Rule,

    /// System notification
    System,

    /// Alert
    Alert,

    /// Error
    Error,

    /// Info
    Info,
}

pub enum MessageSeverity {
    Critical,
    High,
    Warning,
    Info,
    Debug,
}

pub enum MessageStatus {
    /// Pending
    Pending,

    /// Acknowledged
    Acknowledged,

    /// Resolved
    Resolved,

    /// Archived
    Archived,

    /// Dismissed
    Dismissed,
}
```

### 2. AlertChannel - Notification Channel

```rust
#[async_trait]
pub trait AlertChannel: Send + Sync {
    /// Get channel ID
    fn id(&self) -> &str;

    /// Get channel type
    fn channel_type(&self) -> &str;

    /// Send message
    async fn send(&self, message: &Message) -> ChannelResult<()>;

    /// Validate configuration
    fn validate_config(&self, config: &serde_json::Value) -> ChannelResult<()>;

    /// Test connection
    async fn test(&self) -> ChannelResult<TestResult>;
}

pub struct TestResult {
    pub success: bool,
    pub message: String,
    pub response: Option<String>,
}
```

## Channel Implementations

### Webhook Channel

```rust
pub struct WebhookChannel {
    id: String,
    url: String,
    secret: Option<String>,
    client: reqwest::Client,
}

#[async_trait]
impl AlertChannel for WebhookChannel {
    async fn send(&self, message: &Message) -> ChannelResult<()> {
        let payload = serde_json::json!({
            "id": message.id,
            "type": message.message_type,
            "title": message.title,
            "content": message.content,
            "severity": message.severity,
            "timestamp": message.created_at,
        });

        let mut request = self.client.post(&self.url)
            .json(&payload);

        if let Some(secret) = &self.secret {
            let signature = hmac_sha256(&payload.to_string(), secret)?;
            request = request.header("X-Signature", signature);
        }

        request.send().await?;
        Ok(())
    }
}
```

### Email Channel

```rust
pub struct EmailChannel {
    id: String,
    smtp_server: String,
    smtp_port: u16,
    username: String,
    password: String,
    from: String,
    to: Vec<String>,
}

#[async_trait]
impl AlertChannel for EmailChannel {
    async fn send(&self, message: &Message) -> ChannelResult<()> {
        let email = MessageBuilder::new()
            .from(self.from.parse()?)
            .to(self.to.iter().map(|t| t.parse().unwrap()).collect())
            .subject(&message.title)
            .body(&message.content)
            .build();

        let mailer = SmtpTransport::builder_dangerous(&self.smtp_server)
            .port(self.smtp_port)
            .credentials(&Credentials::new(
                self.username.clone(),
                self.password.clone(),
            ))
            .build();

        mailer.send(&email).await?;
        Ok(())
    }
}
```

### Other Channel Types

| Channel Type | Status | Description |
|-------------|--------|-------------|
| Webhook | ✅ | HTTP POST notification |
| Email | ✅ | SMTP email |
| DingTalk | 🟡 | To be implemented |
| WeCom | 🟡 | To be implemented |
| Feishu | 🟡 | To be implemented |
| SMS | 🟡 | To be implemented |
| Telegram | 🟡 | To be implemented |

## Message Storage

```rust
pub struct MessageStore {
    /// Storage backend
    backend: Arc<dyn MessageBackend>,

    /// Notification channels
    channels: HashMap<String, Arc<dyn AlertChannel>>,
}

impl MessageStore {
    /// Create message
    pub async fn create(&self, message: Message) -> Result<String>;

    /// List messages
    pub async fn list(&self, filter: MessageFilter) -> Result<Vec<Message>>;

    /// Get message
    pub async fn get(&self, id: &str) -> Result<Option<Message>>;

    /// Acknowledge message
    pub async fn acknowledge(&self, id: &str) -> Result<()>;

    /// Resolve message
    pub async fn resolve(&self, id: &str, resolution: &str) -> Result<()>;

    /// Archive message
    pub async fn archive(&self, id: &str) -> Result<()>;

    /// Bulk operations
    pub async fn bulk_acknowledge(&self, ids: Vec<String>) -> Result<usize>;
    pub async fn bulk_resolve(&self, ids: Vec<String>) -> Result<usize>;
    pub async fn bulk_delete(&self, ids: Vec<String>) -> Result<usize>;

    /// Cleanup old messages
    pub async fn cleanup(&self, older_than: i64) -> Result<usize>;

    /// Get statistics
    pub async fn get_stats(&self) -> Result<MessageStats>;
}

pub struct MessageFilter {
    pub message_type: Option<MessageType>,
    pub severity: Option<MessageSeverity>,
    pub status: Option<MessageStatus>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub limit: Option<usize>,
}
```

## Message Notifier

```rust
pub struct Notifier {
    /// Message storage
    store: Arc<MessageStore>,

    /// Channel registry
    channels: Arc<RwLock<HashMap<String, Arc<dyn AlertChannel>>>>,

    /// Event bus
    event_bus: Arc<EventBus>,
}

impl Notifier {
    /// Create notifier
    pub fn new(store: Arc<MessageStore>, event_bus: Arc<EventBus>) -> Self;

    /// Register channel
    pub async fn register_channel(&self, channel: Arc<dyn AlertChannel>) -> Result<()>;

    /// Send notification
    pub async fn notify(&self, message: Message) -> Result<()>;

    /// Send to specific channel
    pub async fn notify_to(
        &self,
        channel_id: &str,
        message: Message,
    ) -> Result<()>;

    /// Batch send
    pub async fn notify_batch(&self, messages: Vec<Message>) -> Result<Vec<NotificationResult>>;
}
```

## Message Statistics

```rust
pub struct MessageStats {
    /// Total messages
    pub total: usize,

    /// By status
    pub by_status: HashMap<MessageStatus, usize>,

    /// By severity
    pub by_severity: HashMap<MessageSeverity, usize>,

    /// By type
    pub by_type: HashMap<MessageType, usize>,

    /// Oldest message timestamp
    pub oldest_timestamp: Option<i64>,

    /// Newest message timestamp
    pub newest_timestamp: Option<i64>,
}
```

## API Endpoints

```
# Messages
GET    /api/messages                        # List messages
POST   /api/messages                        # Create message
GET    /api/messages/:id                    # Get message
DELETE /api/messages/:id                    # Delete message

# Message Actions
POST   /api/messages/:id/acknowledge         # Acknowledge message
POST   /api/messages/:id/resolve            # Resolve message
POST   /api/messages/:id/archive            # Archive message

# Bulk Actions
POST   /api/messages/acknowledge            # Bulk acknowledge
POST   /api/messages/resolve                # Bulk resolve
POST   /api/messages/delete                 # Bulk delete

# Maintenance
POST   /api/messages/cleanup                # Cleanup old messages
GET    /api/messages/stats                  # Message statistics

# Channels
GET    /api/messages/channels               # List channels
GET    /api/messages/channels/types         # Channel types
GET    /api/messages/channels/:type/schema  # Channel schema
GET    /api/messages/channels/:name         # Get channel
POST   /api/messages/channels               # Create channel
PUT    /api/messages/channels/:name         # Update channel
DELETE /api/messages/channels/:name         # Delete channel
POST   /api/messages/channels/:name/test    # Test channel
GET    /api/messages/channels/stats         # Channel statistics
```

## Feature Flags

```toml
[features]
default = ["webhook", "email"]
webhook = ["reqwest"]
email = ["lettre"]
```

## Usage Examples

### Create Message

```rust
use neomind_messages::{Message, MessageType, MessageSeverity, Notifier};

let message = Message {
    id: "msg_001".to_string(),
    message_type: MessageType::Alert,
    title: "Temperature Alert".to_string(),
    content: "Greenhouse temperature exceeds 30°C".to_string(),
    severity: MessageSeverity::Warning,
    status: MessageStatus::Pending,
    entity_id: Some("sensor_temp_1".to_string()),
    created_at: chrono::Utc::now().timestamp(),
    updated_at: chrono::Utc::now().timestamp(),
    metadata: serde_json::json!({
        "device": "sensor_temp_1",
        "value": 32.5
    }),
};

notifier.notify(message).await?;
```

### Register Channel

```rust
use neomind_messages::{WebhookChannel, AlertChannel};

let webhook = WebhookChannel::new(
    "alerts_webhook",
    "https://hooks.example.com/alerts",
    Some("secret_key"),
);

notifier.register_channel(Arc::new(webhook)).await?;
```

## Design Principles

1. **Channel Decoupling**: Message creation separated from sending
2. **Async Sending**: Non-blocking main flow
3. **Retry Mechanism**: Auto-retry on send failure
4. **Bulk Operations**: Support bulk acknowledge/delete
