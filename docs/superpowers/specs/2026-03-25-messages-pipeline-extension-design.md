# Messages 模块扩展设计 - 事件管道与通道路由

> **版本**: v1.0
> **日期**: 2026-03-25
> **状态**: 待审核

## 1. 背景与目标

### 1.1 背景

现有 Messages 模块主要承担消息/告警功能，但存在以下局限：

1. **消息类型单一** - 只支持人类可读的通知，不支持结构化业务数据推送
2. **通道广播模式** - 所有启用的通道都会收到所有消息，无法按需路由
3. **推送缺乏追溯** - 数据推送场景无法查看"发了什么出去"

### 1.2 目标

1. **双模式消息** - 支持 Notification（通知）和 DataPush（数据推送）两种类型
2. **通道过滤路由** - 每个通道可配置接收规则，按维度组合过滤
3. **推送可追溯** - DataPush 有发送记录，支持重试队列和死信队列
4. **渐进式实现** - 复用现有架构，最小化改动

## 2. 核心概念

### 2.1 消息类型 (MessageType)

| 类型 | 说明 | 存储 | 用途 |
|------|------|------|------|
| `Notification` | 人类可读通知 | 长期存储 | 告警、事件通知、审计追溯 |
| `DataPush` | 结构化数据推送 | 1天发送记录 | 业务数据同步、遥测推送、报表导出 |

### 2.2 通道过滤器 (ChannelFilter)

每个通道可配置的订阅规则，支持组合过滤：

- `message_types`: 接收的消息类型（空=全部）
- `source_types`: 接收的来源类型（空=全部）
- `categories`: 接收的分类（空=全部）
- `min_severity`: 最低严重级别（空=全部）
- `source_ids`: 指定来源ID（空=全部）

### 2.3 触发来源 (SourceType)

可扩展的来源类型：

- `device` - 设备事件
- `rule` - 规则引擎触发
- `telemetry` - 设备遥测数据
- `schedule` - 定时任务
- `llm` - LLM 生成内容
- `system` - 系统事件
- 未来可扩展更多...

## 3. 数据模型

### 3.1 Message 模型扩展

```rust
// crates/neomind-messages/src/message.rs

pub struct Message {
    // 现有字段（保持不变）
    pub id: MessageId,
    pub category: String,
    pub severity: MessageSeverity,
    pub title: String,
    pub message: String,
    pub source: String,
    pub source_type: String,
    pub timestamp: DateTime<Utc>,
    pub status: MessageStatus,
    pub metadata: Option<serde_json::Value>,
    pub tags: Vec<String>,

    // 新增字段
    pub message_type: MessageType,
    pub source_id: Option<String>,      // 明确的来源ID
    pub payload: Option<serde_json::Value>,  // DataPush 的结构化数据
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    #[default]
    Notification,
    DataPush,
}
```

### 3.2 ChannelFilter 模型（新增）

```rust
// crates/neomind-messages/src/channels/filter.rs

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelFilter {
    /// 接收的消息类型 (空=全部)
    #[serde(default)]
    pub message_types: Vec<MessageType>,

    /// 接收的来源类型 (空=全部)
    #[serde(default)]
    pub source_types: Vec<String>,

    /// 接收的分类 (空=全部)
    #[serde(default)]
    pub categories: Vec<String>,

    /// 最低严重级别 (None=全部)
    #[serde(default)]
    pub min_severity: Option<MessageSeverity>,

    /// 指定来源ID (空=全部)
    #[serde(default)]
    pub source_ids: Vec<String>,
}

impl ChannelFilter {
    /// 检查消息是否匹配此过滤器
    pub fn matches(&self, message: &Message) -> bool {
        // message_types 过滤
        if !self.message_types.is_empty()
            && !self.message_types.contains(&message.message_type) {
            return false;
        }

        // source_types 过滤
        if !self.source_types.is_empty()
            && !self.source_types.contains(&message.source_type) {
            return false;
        }

        // categories 过滤
        if !self.categories.is_empty()
            && !self.categories.contains(&message.category) {
            return false;
        }

        // min_severity 过滤
        if let Some(min_sev) = self.min_severity {
            if message.severity < min_sev {
                return false;
            }
        }

        // source_ids 过滤
        if !self.source_ids.is_empty() {
            if let Some(ref source_id) = message.source_id {
                if !self.source_ids.contains(source_id) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}
```

### 3.3 StoredChannelConfig 扩展

```rust
// crates/neomind-messages/src/channels/mod.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredChannelConfig {
    pub name: String,
    pub channel_type: String,
    pub config: serde_json::Value,
    pub enabled: bool,

    // 新增：通道过滤器
    #[serde(default)]
    pub filter: ChannelFilter,
}
```

### 3.4 DeliveryLog 模型（新增）

```rust
// crates/neomind-messages/src/delivery_log.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryLog {
    pub id: String,
    pub event_id: String,
    pub channel_name: String,
    pub status: DeliveryStatus,
    pub payload_summary: String,    // payload 摘要 (用于展示，限制长度)
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,    // 等待发送
    Success,    // 发送成功
    Failed,     // 最终失败
    Retrying,   // 重试中
}
```

### 3.5 RetryQueue 和 DeadLetter 模型（新增）

```rust
// crates/neomind-messages/src/retry_queue.rs

/// 重试队列项
pub struct RetryQueueItem {
    pub id: String,
    pub message: Message,           // 完整消息
    pub channel_name: String,
    pub retry_count: u32,
    pub next_retry_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 死信队列项
pub struct DeadLetterItem {
    pub id: String,
    pub message: Message,
    pub channel_name: String,
    pub retry_count: u32,
    pub final_error: String,
    pub failed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
```

## 4. 存储策略

### 4.1 存储对比

| 维度 | Notification | DataPush |
|------|-------------|----------|
| **主存储** | MessageStore | DeliveryLog |
| **文件** | `data/messages.redb` | `data/delivery_log.redb` |
| **保留周期** | 长期，按配置清理 | **1天自动过期** |
| **存储内容** | 完整消息 + 状态变更 | 事件ID + 通道 + 状态 + payload摘要 |
| **重试机制** | 无 | RetryQueue + DeadLetter |
| **用途** | 告警历史、审计 | 排查问题、追溯数据 |

### 4.2 存储文件规划

```
data/
├── messages.redb          # Notification 消息存储（现有）
├── channels.redb          # 通道配置（现有，扩展 filter 字段）
├── delivery_log.redb      # DataPush 发送记录（新增，1天清理）
├── retry_queue.redb       # 重试队列（新增）
└── dead_letter.redb       # 死信队列（新增）
```

## 5. API 设计

### 5.1 通道过滤配置 API（新增/扩展）

```http
# 获取通道过滤器
GET /api/messages/channels/:name/filter

# 设置通道过滤器
PUT /api/messages/channels/:name/filter
Content-Type: application/json

{
    "message_types": ["notification", "data_push"],
    "source_types": ["device", "rule"],
    "categories": ["alert", "business"],
    "min_severity": "warning",
    "source_ids": []
}
```

### 5.2 推送记录查询 API（新增）

```http
# 查询推送记录
GET /api/messages/delivery-logs?channel=xxx&status=failed&hours=24

# 响应
{
    "logs": [
        {
            "id": "dl_xxx",
            "event_id": "evt_xxx",
            "channel_name": "webhook-1",
            "status": "failed",
            "payload_summary": "{\"temperature\": 85, ...}",
            "error_message": "Connection timeout",
            "retry_count": 3,
            "created_at": "2026-03-25T10:00:00Z"
        }
    ],
    "count": 1
}
```

### 5.3 重试队列管理 API（新增）

```http
# 查看重试队列
GET /api/messages/retry-queue

# 查看死信队列
GET /api/messages/dead-letter

# 手动重试死信队列中的消息
POST /api/messages/dead-letter/:id/retry

# 删除死信队列中的消息
DELETE /api/messages/dead-letter/:id
```

### 5.4 消息创建 API 扩展

```http
POST /api/messages
Content-Type: application/json

{
    "title": "设备遥测数据",
    "category": "telemetry",
    "severity": "info",

    // 新增字段
    "message_type": "data_push",
    "source_type": "device",
    "source_id": "sensor_001",
    "payload": {
        "temperature": 85,
        "humidity": 60,
        "device_id": "sensor_001"
    }
}
```

## 6. 前端设计

### 6.1 页面结构（保持两 Tab）

```
/messages
├── Tab: 消息列表（展示 Notification + DataPush 记录）
└── Tab: 通道管理（增加过滤配置）
```

### 6.2 通道过滤配置 UI

在通道编辑对话框中新增过滤配置区域：

- 消息类型：多选（通知 / 数据推送）
- 来源类型：多选（device / rule / telemetry / schedule / llm / system）
- 消息分类：多选（alert / business / system / notification）
- 最低级别：单选下拉（info / warning / critical / emergency）
- 指定来源ID：标签输入（可选）

### 6.3 消息列表扩展

- 过滤器增加"消息类型"选项
- DataPush 消息显示 payload 摘要
- 状态展示适配（Success / Failed / Retrying）

### 6.4 推送详情对话框（新增）

展示：
- 事件ID、通道名称、状态
- Payload 摘要
- 错误信息（失败时）
- 重试次数

## 7. 实现计划

### 7.1 阶段划分

| 阶段 | 内容 | 优先级 | 预估工时 |
|------|------|--------|----------|
| **Phase 1** | 后端：ChannelFilter 模型 + 过滤逻辑 + API | P0 | 4h |
| **Phase 2** | 前端：通道过滤配置 UI | P0 | 3h |
| **Phase 3** | 后端：DataPush 类型支持 + DeliveryLog | P1 | 4h |
| **Phase 4** | 前端：消息列表支持 DataPush 展示 | P1 | 3h |
| **Phase 5** | 后端：RetryQueue + DeadLetter | P2 | 3h |
| **Phase 6** | 前端：推送详情 + 重试队列管理 UI | P2 | 3h |

### 7.2 文件变更清单

#### 后端（Rust）

| 文件 | 操作 | 说明 |
|------|------|------|
| `crates/neomind-messages/src/message.rs` | 修改 | 新增 MessageType、source_id、payload 字段 |
| `crates/neomind-messages/src/channels/filter.rs` | 新增 | ChannelFilter 模型 |
| `crates/neomind-messages/src/channels/mod.rs` | 修改 | StoredChannelConfig 增加 filter 字段 |
| `crates/neomind-messages/src/delivery_log.rs` | 新增 | DeliveryLog 模型和存储 |
| `crates/neomind-messages/src/retry_queue.rs` | 新增 | RetryQueue 和 DeadLetter 模型 |
| `crates/neomind-messages/src/manager.rs` | 修改 | 增加过滤逻辑和 DataPush 处理 |
| `crates/neomind-messages/src/lib.rs` | 修改 | 导出新模块 |
| `crates/neomind-api/src/handlers/message_channels.rs` | 修改 | 新增过滤配置 API |
| `crates/neomind-api/src/handlers/messages.rs` | 修改 | 新增推送记录 API |

#### 前端（React/TypeScript）

| 文件 | 操作 | 说明 |
|------|------|------|
| `web/src/types/index.ts` | 修改 | 新增 MessageType、ChannelFilter 等类型 |
| `web/src/lib/api.ts` | 修改 | 新增 API 调用方法 |
| `web/src/pages/messages.tsx` | 修改 | 消息列表支持 DataPush |
| `web/src/components/alerts/UnifiedAlertChannelsTab.tsx` | 修改 | 通道过滤配置 UI |
| `web/src/components/messages/DeliveryLogDialog.tsx` | 新增 | 推送详情对话框 |
| `web/src/i18n/locales/en/common.json` | 修改 | 英文文案 |
| `web/src/i18n/locales/zh/common.json` | 修改 | 中文文案 |

## 8. 兼容性考虑

### 8.1 向后兼容

- 现有消息默认 `message_type = Notification`，行为不变
- 现有通道默认 `filter = ChannelFilter::default()`，接收所有消息
- 现有 API 响应保持兼容

### 8.2 数据迁移

无需迁移，新字段使用默认值。

## 9. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 过滤逻辑性能 | 高并发下可能影响吞吐 | 过滤器设计简单高效，可缓存编译结果 |
| DeliveryLog 存储 | 大量推送时存储增长 | 1天自动清理，限制 payload_summary 长度 |
| 重试队列堆积 | 内存/存储压力 | 设置最大重试次数，死信队列定期清理 |

## 10. 未来扩展

- 高级过滤：支持自定义表达式（如 `payload.temperature > 50`）
- 消息模板：支持变量替换的消息模板系统
- 通道优先级：多通道时的优先级和回退策略
- 批量推送：聚合多条消息批量发送
