# Messages 模块扩展实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 扩展 Messages 模块支持双模式消息（Notification + DataPush）和通道路由过滤

**Architecture:** 在现有 `neomind-messages` crate 基础上扩展，新增 `ChannelFilter` 模型实现通道过滤，新增 `DeliveryLog` 追踪 DataPush 发送记录

**Tech Stack:** Rust (Axum, redb, serde), React 18 + TypeScript + Zustand

**Spec Document:** `docs/superpowers/specs/2026-03-25-messages-pipeline-extension-design.md`

---

## File Structure

```
crates/neomind-messages/src/
├── message.rs              # [修改] 新增 MessageType, source_id, payload
├── channels/
│   ├── mod.rs              # [修改] StoredChannelConfig 增加 filter
│   └── filter.rs           # [新增] ChannelFilter 模型
├── delivery_log.rs         # [新增] DeliveryLog 存储和模型
├── retry_queue.rs          # [新增] 重试队列和死信队列
├── manager.rs              # [修改] 增加过滤逻辑和 DataPush 处理
├── lib.rs                  # [修改] 导出新模块
└── error.rs                # [修改] 新增错误类型

crates/neomind-api/src/handlers/
├── messages.rs             # [修改] 扩展创建消息 API
└── message_channels.rs     # [修改] 新增过滤配置 API

web/src/
├── types/index.ts          # [修改] 新增类型定义
├── lib/api.ts              # [修改] 新增 API 调用
├── pages/messages.tsx      # [修改] 支持 DataPush 展示
├── components/alerts/UnifiedAlertChannelsTab.tsx  # [修改] 过滤配置 UI
└── i18n/locales/
    ├── en/common.json      # [修改] 英文文案
    └── zh/common.json      # [修改] 中文文案
```

---

## Phase 1: 后端 - ChannelFilter 模型与过滤逻辑

### Task 1.1: 新增 MessageType 枚举

**Files:**
- Modify: `crates/neomind-messages/src/message.rs:1-50`

- [ ] **Step 1: 编写 MessageType 枚举的单元测试**

在 `crates/neomind-messages/src/message.rs` 的 `#[cfg(test)] mod tests` 块中添加：

```rust
#[test]
fn test_message_type_from_string() {
    assert_eq!(MessageType::from_string("notification"), Some(MessageType::Notification));
    assert_eq!(MessageType::from_string("data_push"), Some(MessageType::DataPush));
    assert_eq!(MessageType::from_string("invalid"), None);
}

#[test]
fn test_message_type_as_str() {
    assert_eq!(MessageType::Notification.as_str(), "notification");
    assert_eq!(MessageType::DataPush.as_str(), "data_push");
}

#[test]
fn test_message_type_serialization() {
    let mt = MessageType::DataPush;
    let json = serde_json::to_string(&mt).unwrap();
    assert_eq!(json, "\"data_push\"");

    let parsed: MessageType = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, MessageType::DataPush);
}
```

- [ ] **Step 2: 运行测试验证失败**

Run: `cargo test -p neomind-messages test_message_type --no-run`
Expected: 编译失败，MessageType 未定义

- [ ] **Step 3: 实现 MessageType 枚举**

在 `crates/neomind-messages/src/message.rs` 中，在 `MessageSeverity` 定义之前添加：

```rust
/// Message type distinguishing notifications from data pushes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Human-readable notification (stored long-term)
    #[default]
    Notification,
    /// Structured data push (short-term delivery log)
    DataPush,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Notification => "notification",
            Self::DataPush => "data_push",
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "notification" => Some(Self::Notification),
            "data_push" | "datapush" => Some(Self::DataPush),
            _ => None,
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
```

- [ ] **Step 4: 扩展 Message 结构体**

在 `Message` 结构体中添加新字段：

```rust
pub struct Message {
    // ... 现有字段 ...

    /// Message type (notification or data_push)
    #[serde(default)]
    pub message_type: MessageType,

    /// Explicit source ID for filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,

    /// Structured payload for DataPush
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}
```

- [ ] **Step 5: 更新 Message::new 和 helper 方法**

修改 `Message::new` 方法初始化新字段：

```rust
pub fn new(
    category: impl Into<String>,
    severity: MessageSeverity,
    title: String,
    message: String,
    source: String,
) -> Self {
    let now = Utc::now();
    Self {
        id: MessageId::new(),
        category: category.into(),
        severity,
        title,
        message,
        source,
        source_type: "system".to_string(),
        timestamp: now,
        status: MessageStatus::Active,
        metadata: None,
        tags: Vec::new(),
        // 新增字段
        message_type: MessageType::Notification,
        source_id: None,
        payload: None,
    }
}
```

- [ ] **Step 6: 添加 DataPush helper 方法**

```rust
/// Create a data push message with structured payload
pub fn data_push(
    category: String,
    title: String,
    payload: serde_json::Value,
    source_type: String,
    source_id: String,
) -> Self {
    let mut msg = Self::new(category, MessageSeverity::Info, title, String::new(), source_id.clone());
    msg.message_type = MessageType::DataPush;
    msg.source_type = source_type;
    msg.source_id = Some(source_id);
    msg.payload = Some(payload);
    // DataPush 默认不需要 severity 和 status 追踪
    msg
}
```

- [ ] **Step 7: 运行测试验证通过**

Run: `cargo test -p neomind-messages test_message_type`
Expected: 所有测试通过

- [ ] **Step 8: 运行完整测试套件**

Run: `cargo test -p neomind-messages`
Expected: 所有测试通过

- [ ] **Step 9: Commit**

```bash
git add crates/neomind-messages/src/message.rs
git commit -m "feat(messages): add MessageType enum and extend Message model

- Add MessageType enum (Notification, DataPush)
- Add source_id and payload fields to Message
- Add data_push() helper constructor
- Default to Notification for backward compatibility"
```

---

### Task 1.2: 新增 ChannelFilter 模型

**Files:**
- Create: `crates/neomind-messages/src/channels/filter.rs`
- Modify: `crates/neomind-messages/src/channels/mod.rs`

- [ ] **Step 1: 编写 ChannelFilter 单元测试**

创建 `crates/neomind-messages/src/channels/filter.rs` 并添加测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Message, MessageSeverity, MessageType};

    fn make_test_message(message_type: MessageType, source_type: &str, severity: MessageSeverity) -> Message {
        let mut msg = Message::system("Test".to_string(), "Test".to_string());
        msg.message_type = message_type;
        msg.source_type = source_type.to_string();
        msg.severity = severity;
        msg
    }

    #[test]
    fn test_default_filter_matches_all() {
        let filter = ChannelFilter::default();
        let msg = make_test_message(MessageType::DataPush, "device", MessageSeverity::Critical);
        assert!(filter.matches(&msg));
    }

    #[test]
    fn test_filter_by_message_type() {
        let mut filter = ChannelFilter::default();
        filter.message_types = vec![MessageType::Notification];

        let notification = make_test_message(MessageType::Notification, "system", MessageSeverity::Info);
        let data_push = make_test_message(MessageType::DataPush, "system", MessageSeverity::Info);

        assert!(filter.matches(&notification));
        assert!(!filter.matches(&data_push));
    }

    #[test]
    fn test_filter_by_source_type() {
        let mut filter = ChannelFilter::default();
        filter.source_types = vec!["device".to_string(), "rule".to_string()];

        let device_msg = make_test_message(MessageType::Notification, "device", MessageSeverity::Info);
        let rule_msg = make_test_message(MessageType::Notification, "rule", MessageSeverity::Info);
        let system_msg = make_test_message(MessageType::Notification, "system", MessageSeverity::Info);

        assert!(filter.matches(&device_msg));
        assert!(filter.matches(&rule_msg));
        assert!(!filter.matches(&system_msg));
    }

    #[test]
    fn test_filter_by_min_severity() {
        let mut filter = ChannelFilter::default();
        filter.min_severity = Some(MessageSeverity::Warning);

        let info = make_test_message(MessageType::Notification, "system", MessageSeverity::Info);
        let warning = make_test_message(MessageType::Notification, "system", MessageSeverity::Warning);
        let critical = make_test_message(MessageType::Notification, "system", MessageSeverity::Critical);

        assert!(!filter.matches(&info));
        assert!(filter.matches(&warning));
        assert!(filter.matches(&critical));
    }

    #[test]
    fn test_filter_combined() {
        let mut filter = ChannelFilter::default();
        filter.message_types = vec![MessageType::DataPush];
        filter.source_types = vec!["device".to_string()];
        filter.min_severity = Some(MessageSeverity::Warning);

        // 匹配所有条件
        let matching = make_test_message(MessageType::DataPush, "device", MessageSeverity::Critical);
        assert!(filter.matches(&matching));

        // 消息类型不匹配
        let wrong_type = make_test_message(MessageType::Notification, "device", MessageSeverity::Critical);
        assert!(!filter.matches(&wrong_type));

        // 来源类型不匹配
        let wrong_source = make_test_message(MessageType::DataPush, "rule", MessageSeverity::Critical);
        assert!(!filter.matches(&wrong_source));

        // 严重级别不匹配
        let wrong_severity = make_test_message(MessageType::DataPush, "device", MessageSeverity::Info);
        assert!(!filter.matches(&wrong_severity));
    }

    #[test]
    fn test_filter_by_source_id() {
        let mut filter = ChannelFilter::default();
        filter.source_ids = vec!["sensor_001".to_string(), "sensor_002".to_string()];

        let mut msg1 = make_test_message(MessageType::Notification, "device", MessageSeverity::Info);
        msg1.source_id = Some("sensor_001".to_string());

        let mut msg2 = make_test_message(MessageType::Notification, "device", MessageSeverity::Info);
        msg2.source_id = Some("sensor_003".to_string());

        let msg_no_id = make_test_message(MessageType::Notification, "device", MessageSeverity::Info);

        assert!(filter.matches(&msg1));
        assert!(!filter.matches(&msg2));
        assert!(!filter.matches(&msg_no_id));
    }
}
```

- [ ] **Step 2: 运行测试验证失败**

Run: `cargo test -p neomind-messages channels::filter::tests`
Expected: 编译失败，ChannelFilter 未定义

- [ ] **Step 3: 实现 ChannelFilter 结构体**

在测试之前添加实现：

```rust
//! Channel filter for message routing.

use serde::{Deserialize, Serialize};
use crate::{Message, MessageSeverity, MessageType};

/// Filter configuration for a channel to select which messages to receive.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelFilter {
    /// Message types to receive (empty = all)
    #[serde(default)]
    pub message_types: Vec<MessageType>,

    /// Source types to receive (empty = all)
    #[serde(default)]
    pub source_types: Vec<String>,

    /// Categories to receive (empty = all)
    #[serde(default)]
    pub categories: Vec<String>,

    /// Minimum severity level (None = all)
    #[serde(default)]
    pub min_severity: Option<MessageSeverity>,

    /// Specific source IDs to receive (empty = all)
    #[serde(default)]
    pub source_ids: Vec<String>,
}

impl ChannelFilter {
    /// Check if a message matches this filter.
    pub fn matches(&self, message: &Message) -> bool {
        // Filter by message_types
        if !self.message_types.is_empty()
            && !self.message_types.contains(&message.message_type)
        {
            return false;
        }

        // Filter by source_types
        if !self.source_types.is_empty()
            && !self.source_types.contains(&message.source_type)
        {
            return false;
        }

        // Filter by categories
        if !self.categories.is_empty()
            && !self.categories.contains(&message.category)
        {
            return false;
        }

        // Filter by min_severity
        if let Some(min_sev) = self.min_severity {
            if message.severity < min_sev {
                return false;
            }
        }

        // Filter by source_ids
        if !self.source_ids.is_empty() {
            match &message.source_id {
                Some(sid) if self.source_ids.contains(sid) => {}
                _ => return false,
            }
        }

        true
    }

    /// Create a filter that accepts all messages.
    pub fn accept_all() -> Self {
        Self::default()
    }
}

// tests 模块放在这里...
```

- [ ] **Step 4: 在 channels/mod.rs 中导出 filter 模块**

在 `crates/neomind-messages/src/channels/mod.rs` 开头添加：

```rust
pub mod filter;
pub use filter::ChannelFilter;
```

- [ ] **Step 5: 扩展 StoredChannelConfig**

修改 `StoredChannelConfig` 结构体：

```rust
use super::filter::ChannelFilter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredChannelConfig {
    pub name: String,
    pub channel_type: String,
    pub config: serde_json::Value,
    pub enabled: bool,
    /// Filter for message routing
    #[serde(default)]
    pub filter: ChannelFilter,
}
```

- [ ] **Step 6: 更新 ChannelRegistry 以支持 filter**

在 `ChannelRegistry` 中添加方法：

```rust
impl ChannelRegistry {
    /// Get the filter for a channel.
    pub async fn get_filter(&self, name: &str) -> ChannelFilter {
        let configs = self.configs.read().await;
        // 从存储的配置中获取 filter，如果没有则返回默认（接受所有）
        configs.get(name)
            .and_then(|c| {
                if let Some(obj) = c.as_object() {
                    obj.get("filter").and_then(|f| {
                        serde_json::from_value(f.clone()).ok()
                    })
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    /// Set the filter for a channel.
    pub async fn set_filter(&self, name: &str, filter: ChannelFilter) -> Result<()> {
        // 1. 更新内存中的 configs
        {
            let mut configs = self.configs.write().await;
            if let Some(config) = configs.get_mut(name) {
                if let Some(obj) = config.as_object_mut() {
                    obj.insert("filter".to_string(), serde_json::to_value(&filter).map_err(|e| {
                        Error::InvalidConfiguration(format!("Failed to serialize filter: {}", e))
                    })?);
                }
            } else {
                return Err(Error::NotFound(format!("Channel not found: {}", name)));
            }
        }

        // 2. 持久化到 storage
        self.save_channel_filter(name, &filter).await?;

        Ok(())
    }

    /// Persist channel filter to storage
    async fn save_channel_filter(&self, name: &str, filter: &ChannelFilter) -> Result<()> {
        let storage = self.storage.read().await;
        if let Some(db) = storage.as_ref() {
            // 读取现有配置
            let existing: StoredChannelConfig = {
                let read_txn = db.begin_read()
                    .map_err(|e| Error::Storage(format!("Failed to begin read: {}", e)))?;
                let table = read_txn
                    .open_table(redb::TableDefinition::<&str, &str>::new("channels"))
                    .map_err(|e| Error::Storage(format!("Failed to open table: {}", e)))?;

                let value = table.get(name)
                    .map_err(|e| Error::Storage(format!("Failed to get channel: {}", e)))?
                    .ok_or_else(|| Error::NotFound(format!("Channel not found: {}", name)))?;

                serde_json::from_str(value.value())
                    .map_err(|e| Error::Storage(format!("Failed to parse config: {}", e)))?
            };

            // 更新 filter 并保存
            let updated = StoredChannelConfig {
                name: existing.name,
                channel_type: existing.channel_type,
                config: existing.config,
                enabled: existing.enabled,
                filter: filter.clone(),
            };

            let json = serde_json::to_string(&updated)
                .map_err(|e| Error::Storage(format!("Failed to serialize: {}", e)))?;

            let write_txn = db.begin_write()
                .map_err(|e| Error::Storage(format!("Failed to begin write: {}", e)))?;
            {
                let mut table = write_txn
                    .open_table(redb::TableDefinition::<&str, &str>::new("channels"))
                    .map_err(|e| Error::Storage(format!("Failed to open table: {}", e)))?;
                table.insert(name, json.as_str())
                    .map_err(|e| Error::Storage(format!("Failed to save: {}", e)))?;
            }
            write_txn.commit()
                .map_err(|e| Error::Storage(format!("Failed to commit: {}", e)))?;

            tracing::info!("Persisted filter for channel: {}", name);
        }
        Ok(())
    }
}
```

- [ ] **Step 7: 运行测试验证通过**

Run: `cargo test -p neomind-messages channels::filter::tests`
Expected: 所有测试通过

- [ ] **Step 8: Commit**

```bash
git add crates/neomind-messages/src/channels/filter.rs
git add crates/neomind-messages/src/channels/mod.rs
git commit -m "feat(messages): add ChannelFilter for message routing

- Add ChannelFilter struct with multi-dimension filtering
- Support filtering by message_type, source_type, category, severity, source_id
- Extend StoredChannelConfig with filter field
- Default filter accepts all messages (backward compatible)"
```

---

### Task 1.3: 修改消息分发逻辑应用过滤

**Files:**
- Modify: `crates/neomind-messages/src/manager.rs`

- [ ] **Step 1: 添加测试用例验证过滤行为**

在 `manager.rs` 的测试模块中添加：

```rust
#[cfg(feature = "webhook")]
#[tokio::test]
async fn test_message_filtering_by_source_type() {
    let manager = MessageManager::new();

    // 注册一个只接收 device 消息的 mock channel
    // (这里需要 mock，或者使用实际的 webhook channel 进行集成测试)
    // 简化：只测试 filter 逻辑

    use crate::channels::ChannelFilter;
    let mut filter = ChannelFilter::default();
    filter.source_types = vec!["device".to_string()];

    let device_msg = Message::device(
        MessageSeverity::Warning,
        "Device Alert".to_string(),
        "Test".to_string(),
        "sensor_1".to_string(),
    );

    let system_msg = Message::system("System".to_string(), "Test".to_string());

    assert!(filter.matches(&device_msg));
    assert!(!filter.matches(&system_msg));
}
```

- [ ] **Step 2: 修改 create_message 方法应用过滤**

修改 `manager.rs` 中的 `create_message` 方法：

```rust
pub async fn create_message(&self, message: Message) -> Result<Message> {
    let id = message.id.clone();
    let severity = message.severity;
    let message_type = message.message_type;

    // Store in memory
    self.messages
        .write()
        .await
        .insert(id.clone(), message.clone());

    // Persist to storage if available (only for Notification)
    if message_type == MessageType::Notification {
        if let Some(store) = self.storage.read().await.as_ref() {
            let stored = Self::message_to_stored(&message);
            store
                .insert(&stored)
                .map_err(|e| Error::Storage(format!("Failed to persist message: {}", e)))?;
        }
    }

    // Send through channels with filtering
    let channels = self.channels.read().await;
    let channel_names = channels.list_names().await;
    let mut send_results = Vec::new();

    for channel_name in &channel_names {
        if let Some(channel) = channels.get(channel_name).await {
            if !channel.is_enabled() {
                continue;
            }

            // 应用过滤器
            let filter = channels.get_filter(channel_name).await;
            if !filter.matches(&message) {
                tracing::debug!(
                    "Channel '{}' filter rejected message '{}'",
                    channel_name,
                    message.title
                );
                continue;
            }

            match channel.send(&message).await {
                Ok(()) => {
                    send_results.push((channel_name.clone(), Ok(())));

                    // DataPush 记录发送日志
                    if message_type == MessageType::DataPush {
                        if let Err(e) = self.log_delivery(&message, channel_name, DeliveryStatus::Success).await {
                            tracing::warn!("Failed to log delivery: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to send message through channel '{}': {}",
                        channel_name,
                        e
                    );
                    send_results.push((channel_name.clone(), Err(e.clone())));

                    // DataPush 处理失败
                    if message_type == MessageType::DataPush {
                        if let Err(log_err) = self.handle_push_failure(&message, channel_name, e).await {
                            tracing::warn!("Failed to handle push failure: {}", log_err);
                        }
                    }
                }
            }
        }
    }

    // ... 其余逻辑保持不变 ...

    Ok(message)
}
```

- [ ] **Step 3: 更新 message_to_stored 方法**

确保新字段被正确序列化：

```rust
fn message_to_stored(msg: &Message) -> neomind_storage::StoredMessage {
    neomind_storage::StoredMessage {
        id: msg.id.to_string(),
        category: msg.category.clone(),
        severity: msg.severity.as_str().to_string(),
        title: msg.title.clone(),
        message: msg.message.clone(),
        source: msg.source.clone(),
        source_type: Some(msg.source_type.clone()),
        status: msg.status.as_str().to_string(),
        tags: if msg.tags.is_empty() { None } else { Some(msg.tags.clone()) },
        metadata: if msg.payload.is_some() {
            // 将 payload 合并到 metadata 中
            let mut meta = msg.metadata.clone().unwrap_or(serde_json::json!({}));
            if let Some(obj) = meta.as_object_mut() {
                obj.insert("message_type".to_string(), serde_json::json!(msg.message_type.as_str()));
                if let Some(sid) = &msg.source_id {
                    obj.insert("source_id".to_string(), serde_json::json!(sid));
                }
                if let Some(p) = &msg.payload {
                    obj.insert("payload".to_string(), p.clone());
                }
            }
            Some(meta)
        } else {
            msg.metadata.clone()
        },
        timestamp: msg.timestamp.timestamp(),
        acknowledged_at: None,
        resolved_at: None,
        acknowledged_by: None,
    }
}
```

- [ ] **Step 4: 更新 stored_to_message 方法**

```rust
fn stored_to_message(stored: neomind_storage::StoredMessage) -> Message {
    let (message_type, source_id, payload) = if let Some(ref meta) = stored.metadata {
        let mt = meta.get("message_type")
            .and_then(|v| v.as_str())
            .and_then(|s| MessageType::from_string(s))
            .unwrap_or(MessageType::Notification);
        let sid = meta.get("source_id").and_then(|v| v.as_str()).map(String::from);
        let p = meta.get("payload").cloned();
        (mt, sid, p)
    } else {
        (MessageType::Notification, None, None)
    };

    Message {
        id: MessageId::from_string(&stored.id).unwrap_or_else(|_| MessageId::new()),
        category: stored.category,
        severity: MessageSeverity::from_string(&stored.severity).unwrap_or(MessageSeverity::Info),
        title: stored.title,
        message: stored.message,
        source: stored.source,
        source_type: stored.source_type.unwrap_or_else(|| "system".to_string()),
        timestamp: chrono::DateTime::from_timestamp(stored.timestamp, 0).unwrap_or_else(chrono::Utc::now),
        status: MessageStatus::from_string(&stored.status).unwrap_or(MessageStatus::Active),
        metadata: stored.metadata,
        tags: stored.tags.unwrap_or_default(),
        message_type,
        source_id,
        payload,
    }
}
```

- [ ] **Step 5: 运行测试**

Run: `cargo test -p neomind-messages`
Expected: 所有测试通过

- [ ] **Step 6: Commit**

```bash
git add crates/neomind-messages/src/manager.rs
git commit -m "feat(messages): apply channel filter in message routing

- Check ChannelFilter before sending to each channel
- Only persist Notification messages (DataPush uses delivery log)
- Handle DataPush success/failure with delivery logging"
```

---

### Task 1.4: 新增通道过滤 API

**Files:**
- Modify: `crates/neomind-api/src/handlers/message_channels.rs`

- [ ] **Step 1: 添加 API handler**

```rust
// 在 message_channels.rs 中添加

use neomind_messages::channels::ChannelFilter;

/// Get channel filter configuration
pub async fn get_channel_filter_handler(
    Path(name): Path<String>,
    State(state): State<Arc<ServerState>>,
) -> Result<Json<ChannelFilter>, NeoMindError> {
    let channels = state.message_manager.channels().await;
    let filter = channels.get_filter(&name).await;
    Ok(Json(filter))
}

/// Update channel filter configuration
pub async fn update_channel_filter_handler(
    Path(name): Path<String>,
    State(state): State<Arc<ServerState>>,
    Json(filter): Json<ChannelFilter>,
) -> Result<Json<serde_json::Value>, NeoMindError> {
    let channels = state.message_manager.channels().await;

    // 验证通道存在
    if channels.get(&name).await.is_none() {
        return Err(NeoMindError::not_found(format!("Channel not found: {}", name)));
    }

    channels.set_filter(&name, filter.clone()).await
        .map_err(|e| NeoMindError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "Filter updated successfully",
        "message_zh": "过滤器更新成功",
        "channel": name,
        "filter": filter
    })))
}
```

- [ ] **Step 2: 注册路由**

在 `message_channels_router()` 中添加：

```rust
pub fn message_channels_router() -> Router<Arc<ServerState>> {
    Router::new()
        // ... 现有路由 ...
        .route("/:name/filter", get(get_channel_filter_handler))
        .route("/:name/filter", put(update_channel_filter_handler))
}
```

- [ ] **Step 3: 测试 API**

Run: `cargo build -p neomind-api && cargo test -p neomind-api`
Expected: 编译通过，测试通过

- [ ] **Step 4: Commit**

```bash
git add crates/neomind-api/src/handlers/message_channels.rs
git commit -m "feat(api): add channel filter configuration endpoints

- GET /messages/channels/:name/filter
- PUT /messages/channels/:name/filter"
```

---

## Phase 2: 前端 - 通道过滤配置 UI

### Task 2.1: 新增前端类型定义

**Files:**
- Modify: `web/src/types/index.ts`

- [ ] **Step 1: 添加类型定义**

```typescript
// 在 web/src/types/index.ts 中添加

// Message Type
export type MessageType = 'notification' | 'data_push'

// Channel Filter
export interface ChannelFilter {
  message_types: MessageType[]
  source_types: string[]
  categories: string[]
  min_severity: MessageSeverity | null
  source_ids: string[]
}

// 扩展 CreateMessageRequest
export interface CreateMessageRequest {
  category: MessageCategory
  severity: MessageSeverity
  title: string
  message: string
  source?: string
  source_type?: string
  metadata?: Record<string, unknown>
  tags?: string[]
  // 新增
  message_type?: MessageType
  source_id?: string
  payload?: Record<string, unknown>
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/types/index.ts
git commit -m "feat(web): add MessageType and ChannelFilter types"
```

---

### Task 2.2: 新增 API 调用方法

**Files:**
- Modify: `web/src/lib/api.ts`

- [ ] **Step 1: 添加 API 方法**

```typescript
// 在 api.ts 中添加

// Channel Filter API
getChannelFilter: (name: string) =>
  fetchAPI<ChannelFilter>(`/messages/channels/${encodeURIComponent(name)}/filter`),

updateChannelFilter: (name: string, filter: ChannelFilter) =>
  fetchAPI<{ message: string; message_zh: string; channel: string; filter: ChannelFilter }>(
    `/messages/channels/${encodeURIComponent(name)}/filter`,
    {
      method: 'PUT',
      body: JSON.stringify(filter),
    }
  ),
```

- [ ] **Step 2: Commit**

```bash
git add web/src/lib/api.ts
git commit -m "feat(web): add channel filter API methods"
```

---

### Task 2.3: 实现通道过滤配置组件

**Files:**
- Modify: `web/src/components/alerts/UnifiedAlertChannelsTab.tsx`

- [ ] **Step 1: 添加过滤配置状态和对话框**

在组件中添加：

```typescript
// State
const [filterDialogChannel, setFilterDialogChannel] = useState<MessageChannel | null>(null)
const [filterConfig, setFilterConfig] = useState<ChannelFilter>({
  message_types: [],
  source_types: [],
  categories: [],
  min_severity: null,
  source_ids: [],
})
const [savingFilter, setSavingFilter] = useState(false)

// Handler
const handleOpenFilterDialog = async (channel: MessageChannel) => {
  setFilterDialogChannel(channel)
  try {
    const filter = await api.getChannelFilter(channel.name)
    setFilterConfig(filter)
  } catch (error) {
    // 默认配置
    setFilterConfig({
      message_types: [],
      source_types: [],
      categories: [],
      min_severity: null,
      source_ids: [],
    })
  }
}

const handleSaveFilter = async () => {
  if (!filterDialogChannel) return
  setSavingFilter(true)
  try {
    await api.updateChannelFilter(filterDialogChannel.name, filterConfig)
    toast({ title: t('common.success'), description: t('messages.channels.filterSaved') })
    setFilterDialogChannel(null)
  } catch (error) {
    handleError(error, { operation: 'Save filter' })
  } finally {
    setSavingFilter(false)
  }
}
```

- [ ] **Step 2: 添加过滤配置对话框 UI**

```tsx
{/* Filter Configuration Dialog */}
<Dialog open={!!filterDialogChannel} onOpenChange={() => setFilterDialogChannel(null)}>
  <DialogContent className="max-w-lg">
    <DialogHeader>
      <DialogTitle>{t('messages.channels.filterConfig', '消息过滤配置')}</DialogTitle>
      <DialogDescription>
        {t('messages.channels.filterConfigDesc', '配置此通道接收哪些消息')}
      </DialogDescription>
    </DialogHeader>

    <div className="space-y-4 py-4">
      {/* Message Types */}
      <div className="space-y-2">
        <Label>{t('messages.channels.messageTypes', '消息类型')}</Label>
        <div className="flex gap-4">
          <label className="flex items-center gap-2">
            <Checkbox
              checked={filterConfig.message_types.includes('notification') || filterConfig.message_types.length === 0}
              onCheckedChange={(checked) => {
                if (checked) {
                  setFilterConfig(prev => ({
                    ...prev,
                    message_types: [...new Set([...prev.message_types, 'notification'])]
                  }))
                } else {
                  setFilterConfig(prev => ({
                    ...prev,
                    message_types: prev.message_types.filter(t => t !== 'notification')
                  }))
                }
              }}
            />
            {t('messages.channels.notification', '通知')}
          </label>
          <label className="flex items-center gap-2">
            <Checkbox
              checked={filterConfig.message_types.includes('data_push') || filterConfig.message_types.length === 0}
              onCheckedChange={(checked) => {
                if (checked) {
                  setFilterConfig(prev => ({
                    ...prev,
                    message_types: [...new Set([...prev.message_types, 'data_push'])]
                  }))
                } else {
                  setFilterConfig(prev => ({
                    ...prev,
                    message_types: prev.message_types.filter(t => t !== 'data_push')
                  }))
                }
              }}
            />
            {t('messages.channels.dataPush', '数据推送')}
          </label>
        </div>
        <p className="text-xs text-muted-foreground">
          {t('messages.channels.messageTypesHint', '不选择则接收所有类型')}
        </p>
      </div>

      {/* Source Types */}
      <div className="space-y-2">
        <Label>{t('messages.channels.sourceTypes', '来源类型')}</Label>
        <div className="flex flex-wrap gap-2">
          {['device', 'rule', 'telemetry', 'schedule', 'llm', 'system'].map(st => (
            <label key={st} className="flex items-center gap-2">
              <Checkbox
                checked={filterConfig.source_types.includes(st)}
                onCheckedChange={(checked) => {
                  if (checked) {
                    setFilterConfig(prev => ({
                      ...prev,
                      source_types: [...prev.source_types, st]
                    }))
                  } else {
                    setFilterConfig(prev => ({
                      ...prev,
                      source_types: prev.source_types.filter(t => t !== st)
                    }))
                  }
                }}
              />
              {st}
            </label>
          ))}
        </div>
        <p className="text-xs text-muted-foreground">
          {t('messages.channels.sourceTypesHint', '不选择则接收所有来源')}
        </p>
      </div>

      {/* Categories */}
      <div className="space-y-2">
        <Label>{t('messages.channels.categories', '消息分类')}</Label>
        <div className="flex flex-wrap gap-2">
          {['alert', 'system', 'business', 'notification'].map(cat => (
            <label key={cat} className="flex items-center gap-2">
              <Checkbox
                checked={filterConfig.categories.includes(cat)}
                onCheckedChange={(checked) => {
                  if (checked) {
                    setFilterConfig(prev => ({
                      ...prev,
                      categories: [...prev.categories, cat]
                    }))
                  } else {
                    setFilterConfig(prev => ({
                      ...prev,
                      categories: prev.categories.filter(t => t !== cat)
                    }))
                  }
                }}
              />
              {cat}
            </label>
          ))}
        </div>
      </div>

      {/* Min Severity */}
      <div className="space-y-2">
        <Label>{t('messages.channels.minSeverity', '最低严重级别')}</Label>
        <Select
          value={filterConfig.min_severity || ''}
          onValueChange={(value) => {
            setFilterConfig(prev => ({
              ...prev,
              min_severity: value as MessageSeverity || null
            }))
          }}
        >
          <SelectTrigger>
            <SelectValue placeholder={t('messages.channels.allSeverities', '所有级别')} />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="">{t('messages.channels.allSeverities', '所有级别')}</SelectItem>
            <SelectItem value="info">Info</SelectItem>
            <SelectItem value="warning">Warning</SelectItem>
            <SelectItem value="critical">Critical</SelectItem>
            <SelectItem value="emergency">Emergency</SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>

    <DialogFooter>
      <Button variant="outline" onClick={() => setFilterDialogChannel(null)}>
        {t('common.cancel')}
      </Button>
      <Button onClick={handleSaveFilter} disabled={savingFilter}>
        {savingFilter ? t('common.saving') : t('common.save')}
      </Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
```

- [ ] **Step 3: 在通道列表中添加过滤配置按钮**

在通道操作按钮区域添加：

```tsx
<Button
  variant="ghost"
  size="sm"
  onClick={() => handleOpenFilterDialog(channel)}
  title={t('messages.channels.configureFilter', '配置过滤规则')}
>
  <Filter className="h-4 w-4" />
</Button>
```

- [ ] **Step 4: 添加 i18n 文案**

在 `web/src/i18n/locales/zh/common.json` 添加：

```json
{
  "messages": {
    "channels": {
      "filterConfig": "消息过滤配置",
      "filterConfigDesc": "配置此通道接收哪些消息",
      "messageTypes": "消息类型",
      "notification": "通知",
      "dataPush": "数据推送",
      "messageTypesHint": "不选择则接收所有类型",
      "sourceTypes": "来源类型",
      "sourceTypesHint": "不选择则接收所有来源",
      "categories": "消息分类",
      "minSeverity": "最低严重级别",
      "allSeverities": "所有级别",
      "configureFilter": "配置过滤规则",
      "filterSaved": "过滤配置已保存"
    }
  }
}
```

- [ ] **Step 5: 测试前端**

Run: `cd web && npm run dev`
验证：打开通道管理，点击过滤配置按钮，对话框正常显示和保存

- [ ] **Step 6: Commit**

```bash
git add web/src/components/alerts/UnifiedAlertChannelsTab.tsx
git add web/src/i18n/locales/zh/common.json
git add web/src/i18n/locales/en/common.json
git commit -m "feat(web): add channel filter configuration UI

- Add filter dialog with message type, source type, category, severity filters
- Add filter button to channel list
- Add i18n support for zh/en"
```

---

## Phase 3: 后端 - DeliveryLog 和重试机制

### Task 3.1: 新增 DeliveryLog 模块

**Files:**
- Create: `crates/neomind-messages/src/delivery_log.rs`
- Modify: `crates/neomind-messages/src/lib.rs`
- Modify: `crates/neomind-messages/src/manager.rs`

- [ ] **Step 1: 创建 delivery_log.rs**

```rust
//! Delivery log for tracking DataPush messages.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique delivery log ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeliveryLogId(pub String);

impl DeliveryLogId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for DeliveryLogId {
    fn default() -> Self {
        Self::new()
    }
}

/// Delivery status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Pending,
    Success,
    Failed,
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
}

/// Delivery log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryLog {
    pub id: DeliveryLogId,
    pub event_id: String,
    pub channel_name: String,
    pub status: DeliveryStatus,
    pub payload_summary: String,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DeliveryLog {
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
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_status(mut self, status: DeliveryStatus) -> Self {
        self.status = status;
        self.updated_at = Utc::now();
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error_message = Some(error);
        self.updated_at = Utc::now();
        self
    }
}
```

- [ ] **Step 2: 在 lib.rs 中导出**

```rust
pub mod delivery_log;
pub use delivery_log::{DeliveryLog, DeliveryLogId, DeliveryStatus};
```

- [ ] **Step 3: 在 manager.rs 中添加 log_delivery 方法**

```rust
use crate::delivery_log::{DeliveryLog, DeliveryStatus};

impl MessageManager {
    /// Log a delivery for DataPush messages.
    async fn log_delivery(
        &self,
        message: &Message,
        channel_name: &str,
        status: DeliveryStatus,
    ) -> Result<()> {
        let payload_summary = if let Some(ref payload) = message.payload {
            // 截断摘要，限制长度
            let summary = serde_json::to_string(payload).unwrap_or_default();
            if summary.len() > 200 {
                format!("{}...", &summary[..197])
            } else {
                summary
            }
        } else {
            String::new()
        };

        let log = DeliveryLog::new(
            message.id.to_string(),
            channel_name.to_string(),
            payload_summary,
        ).with_status(status);

        // TODO: 存储到 delivery_log.redb
        tracing::debug!("Logged delivery: {:?}", log);

        Ok(())
    }

    /// Handle push failure for retry logic.
    async fn handle_push_failure(
        &self,
        message: &Message,
        channel_name: &str,
        error: Error,
    ) -> Result<()> {
        let log = DeliveryLog::new(
            message.id.to_string(),
            channel_name.to_string(),
            String::new(),
        )
        .with_status(DeliveryStatus::Failed)
        .with_error(error.to_string());

        // TODO: 存储并加入重试队列
        tracing::warn!("Push failed: {:?}", log);

        Ok(())
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/neomind-messages/src/delivery_log.rs
git add crates/neomind-messages/src/lib.rs
git add crates/neomind-messages/src/manager.rs
git commit -m "feat(messages): add DeliveryLog for DataPush tracking

- Add DeliveryLog model with status tracking
- Add log_delivery and handle_push_failure methods
- Prepare for retry queue implementation"
```

---

## Summary & Next Steps

**Phase 1 & 2 完成后可交付：**
- ✅ 通道可配置过滤规则
- ✅ 消息按规则路由到通道
- ✅ 前端 UI 支持过滤配置

**Phase 3 (后续):**
- DeliveryLog 持久化存储
- 重试队列和死信队列
- 推送记录查询 API
- 前端推送记录展示

**Phase 4 (后续):**
- 定时清理 DeliveryLog (1天过期)
- 重试队列处理任务
- 死信队列管理 API

---

## Testing Checklist

- [ ] `cargo test -p neomind-messages` 全部通过
- [ ] `cargo test -p neomind-api` 全部通过
- [ ] `cargo clippy --all-targets` 无警告
- [ ] 前端 `npm run build` 编译通过
- [ ] 手动测试：创建通道 → 配置过滤 → 发送消息 → 验证路由
