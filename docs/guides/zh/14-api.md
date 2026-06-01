# API 模块

**包名**: `neomind-api`
**版本**: 0.8.0
**完成度**: 95%
**用途**: REST/WebSocket API服务器

## 概述

API模块基于Axum框架，提供REST API、WebSocket和SSE端点，是前端与后端通信的桥梁。

## 模块结构

```
crates/neomind-api/src/
├── lib.rs                      # 公开接口
├── server/
│   ├── mod.rs                  # 服务器配置
│   ├── router.rs               # 路由定义
│   ├── types.rs                # 服务器状态
│   ├── assets.rs               # 静态资源
│   ├── state/                  # 状态管理
│   └── middleware/             # 中间件
├── handlers/                   # 请求处理器
│   ├── mod.rs
│   ├── sessions.rs             # 会话API
│   ├── devices/                # 设备API
│   │   ├── mod.rs
│   │   ├── crud.rs             # CRUD操作
│   │   ├── models.rs           # 数据模型
│   │   ├── discovery.rs        # 设备发现
│   │   ├── metrics.rs          # 指标查询
│   │   ├── telemetry.rs        # 遥测数据
│   │   ├── types.rs            # 设备类型
│   │   ├── webhook.rs          # Webhook
│   │   ├── auto_onboard.rs     # 自动入板
│   │   └── mdl.rs              # MDL格式
│   ├── agents.rs               # Agent API
│   ├── automations.rs          # 自动化API（规则 + 转换）
│   ├── rules.rs                # 规则API
│   ├── messages.rs             # 消息API
│   ├── tools.rs                # 工具API
│   ├── memory.rs               # 内存API（Markdown）
│   ├── llm_backends.rs         # LLM后端API
│   ├── settings.rs             # 设置API
│   ├── mqtt/                   # MQTT API
│   │   ├── mod.rs
│   │   ├── brokers.rs          # Broker管理
│   │   ├── models.rs           # 数据模型
│   │   ├── status.rs           # 状态查询
│   │   └── subscriptions.rs    # 订阅管理
│   ├── extensions.rs           # 扩展API
│   ├── extension_stream.rs     # 扩展流（WebSocket）
│   ├── frontend_components.rs  # 前端组件API
│   ├── message_channels.rs     # 消息通道API
│   ├── data_push.rs            # 数据推送API
│   ├── data.rs                 # 遥测与数据源API
│   ├── events.rs               # 事件API
│   ├── ws/                     # WebSocket处理
│   ├── auth.rs                 # 认证（API Key）
│   ├── auth_users.rs           # 用户管理（JWT）
│   ├── basic.rs                # 基础端点
│   ├── capabilities.rs         # 能力API
│   ├── config.rs               # 配置管理
│   ├── dashboards.rs           # 仪表板API（含分享）
│   ├── instances.rs            # 实例管理API
│   ├── skills.rs               # 技能API
│   ├── stats.rs                # 统计API
│   ├── suggestions.rs          # 建议API
│   ├── summarization.rs        # 摘要API
│   └── setup.rs                # 初始化设置
├── models/                     # 数据模型
├── automation/                 # 自动化引擎
├── auth.rs                     # 认证中间件
├── auth_users.rs               # 用户认证
├── config.rs                   # 服务器配置
├── rate_limit.rs               # 速率限制
├── cache.rs                    # 响应缓存
├── crypto.rs                   # 加密工具
├── validator.rs                # 输入验证
├── event_services.rs           # 事件服务
├── capability_providers.rs     # 能力提供者
├── shutdown.rs                 # 优雅关闭
└── startup.rs                  # 启动逻辑
```

## 重要变更 (v0.8.0)

### 新增模块
- `handlers/data_push.rs` - 数据推送API（定时数据投递到外部端点）
- `handlers/frontend_components.rs` - 前端组件市场和管理
- `handlers/instances.rs` - 远程实例管理API
- `handlers/skills.rs` - Agent技能匹配和管理API
- `handlers/capabilities.rs` - 能力发现API
- `handlers/summarization.rs` - 内容摘要API
- `handlers/data.rs` - 统一遥测和数据源API
- `automation/` - 自动化引擎模块

### 安全模型
路由按四个层级组织：
- **公开路由**: 健康检查、认证、初始化、静态元数据、扩展只读、分享代理、Webhook
- **JWT路由**: 用户会话管理（me、logout、change-password）
- **保护路由**（混合认证 - API Key 或 JWT）: 所有数据操作、CRUD、写入端点
- **管理员路由**（JWT + Admin 角色）: 用户管理（列表、创建、删除）

### 数据推送API
定时数据投递到外部端点：

```rust
GET    /api/data-push           # 列出推送目标
POST   /api/data-push           # 创建推送目标
GET    /api/data-push/stats     # 推送统计
GET    /api/data-push/:id       # 获取推送目标
PUT    /api/data-push/:id       # 更新推送目标
DELETE /api/data-push/:id       # 删除推送目标
POST   /api/data-push/:id/test  # 测试推送目标
POST   /api/data-push/:id/start # 启动推送目标
POST   /api/data-push/:id/stop  # 停止推送目标
GET    /api/data-push/:id/logs  # 列出投递日志
```

## 路由概览

### 公开路由（无需认证）

```rust
// 健康检查
GET /api/health
GET /api/health/status
GET /api/health/live
GET /api/health/ready
GET /api/system/network-info

// 认证
GET  /api/auth/status
GET  /api/auth/verify
POST /api/auth/login
POST /api/auth/register

// 初始化设置
GET  /api/setup/status
POST /api/setup/initialize
POST /api/setup/complete
POST /api/setup/llm-config

// 只读元数据（公开 - 仅静态模式）
GET /api/llm-backends/types
GET /api/llm-backends/types/:type/schema
GET /api/messages/channels/types
GET /api/messages/channels/types/:type/schema
GET /api/extensions
GET /api/extensions/types
GET /api/extensions/dashboard-components
GET /api/extensions/capabilities
GET /api/extensions/:id                # 获取扩展信息
GET /api/extensions/:id/health         # 健康检查
GET /api/extensions/:id/commands       # 列出命令
GET /api/extensions/:id/components     # 仪表板组件
GET /api/extensions/:id/assets/*       # 静态资源
GET /api/extensions/:id/event-subscriptions
GET /api/extensions/:id/stream/capability
GET /api/extensions/:id/stream/sessions

// 能力与工具（公开 - 静态元数据）
GET /api/capabilities
GET /api/capabilities/:name
GET /api/tools
GET /api/tools/:name

// 市场（公开 - 只读）
GET /api/extensions/market/list
GET /api/extensions/market/:id
GET /api/extensions/market/updates
GET /api/frontend-components/market/list
GET /api/frontend-components/:id/bundle
GET /api/device-types/cloud/list

// 建议（公开 - 输入提示）
GET /api/suggestions
GET /api/suggestions/categories

// 分享API（公开 - 无需认证即可访问分享的仪表板）
GET /api/share/:token
ANY /api/share/:token/proxy/*path

// Webhook（公开 - 外部设备无法携带JWT）
POST /api/devices/:id/webhook
POST /api/devices/webhook
GET  /api/devices/:id/webhook-url
```

### JWT保护路由

```rust
// 用户信息和会话管理
GET  /api/auth/me
POST /api/auth/logout
POST /api/auth/change-password
```

### WebSocket路由（认证在处理器中处理）

```rust
// 事件流 WebSocket/SSE
GET /api/events/ws
GET /api/events/stream

// 聊天WebSocket（通过 ?token= 参数传递JWT）
GET /api/chat

// 扩展流WebSocket
GET /api/extensions/:id/stream
```

### 保护路由（API Key 或 JWT）

```rust
// 遥测与数据
GET /api/telemetry
GET /api/telemetry/stats
GET /api/data/sources
GET /api/stats/system

// LLM后端（读取 - 从公开移至保护）
GET /api/llm-backends
GET /api/llm-backends/:id
GET /api/llm-backends/stats
GET /api/llm-backends/ollama/models
GET /api/llm-backends/llamacpp/server-info

// LLM后端（写入）
POST   /api/llm-backends
PUT    /api/llm-backends/:id
DELETE /api/llm-backends/:id
POST   /api/llm-backends/:id/test
POST   /api/llm-backends/:id/activate
GET    /api/llm-backends/:id/models
POST   /api/llm/generate

// 技能
GET    /api/skills
POST   /api/skills
POST   /api/skills/reload
GET    /api/skills/match
GET    /api/skills/:id
PUT    /api/skills/:id
DELETE /api/skills/:id

// 实例（远程后端管理）
GET    /api/instances
POST   /api/instances
GET    /api/instances/:id
PUT    /api/instances/:id
DELETE /api/instances/:id
POST   /api/instances/:id/test

// 会话管理
GET    /api/sessions
POST   /api/sessions
GET    /api/sessions/:id
PUT    /api/sessions/:id
DELETE /api/sessions/:id
POST   /api/sessions/:id/chat
GET    /api/sessions/:id/history
PUT    /api/sessions/:id/memory-toggle
GET    /api/sessions/:id/pending
DELETE /api/sessions/:id/pending
POST   /api/sessions/cleanup

// 设备管理
GET    /api/devices
POST   /api/devices
POST   /api/devices/ble-provision
GET    /api/devices/:id
PUT    /api/devices/:id
DELETE /api/devices/:id
GET    /api/devices/:id/current
POST   /api/devices/current-batch
POST   /api/devices/:id/command/:command
GET    /api/devices/:id/telemetry
POST   /api/devices/:id/metrics
GET    /api/devices/:id/telemetry/summary
GET    /api/devices/:id/commands

// 设备类型
GET    /api/device-types
POST   /api/device-types
GET    /api/device-types/:id
PUT    /api/device-types          # 验证
DELETE /api/device-types/:id
POST   /api/device-types/generate-from-samples
POST   /api/device-types/cloud/import
POST   /api/devices/generate-mdl

// 草稿设备（自动入板）
GET    /api/devices/drafts
GET    /api/devices/drafts/:device_id
PUT    /api/devices/drafts/:device_id
POST   /api/devices/drafts/:device_id/approve
POST   /api/devices/drafts/:device_id/reject
POST   /api/devices/drafts/:device_id/analyze
POST   /api/devices/drafts/:device_id/enhance
GET    /api/devices/drafts/:device_id/suggest-types
POST   /api/devices/drafts/cleanup
GET    /api/devices/drafts/type-signatures
GET    /api/devices/drafts/config
PUT    /api/devices/drafts/config
POST   /api/devices/drafts/upload

// 规则管理
GET    /api/rules
POST   /api/rules
GET    /api/rules/export
POST   /api/rules/import
GET    /api/rules/resources
POST   /api/rules/validate
GET    /api/rules/:id
PUT    /api/rules/:id
DELETE /api/rules/:id
POST   /api/rules/:id/enable
POST   /api/rules/:id/test
GET    /api/rules/:id/history

// 自动化（统一规则 + 转换）
GET    /api/automations
POST   /api/automations
GET    /api/automations/export
POST   /api/automations/import
POST   /api/automations/analyze-intent
GET    /api/automations/templates
GET    /api/automations/:id
PUT    /api/automations/:id
DELETE /api/automations/:id
POST   /api/automations/:id/enable
GET    /api/automations/:id/executions

// 转换（数据处理）
GET    /api/automations/transforms
POST   /api/automations/transforms/process
POST   /api/automations/transforms/:id/test
POST   /api/automations/transforms/test-code
GET    /api/automations/transforms/metrics
GET    /api/automations/transforms/data-sources
GET    /api/automations/transforms/:id/data-sources
GET    /api/automations/transforms/data-sources/:data_source_id

// 消息
GET    /api/messages
POST   /api/messages
GET    /api/messages/stats
POST   /api/messages/cleanup
POST   /api/messages/acknowledge     # 批量
POST   /api/messages/resolve         # 批量
POST   /api/messages/delete          # 批量
GET    /api/messages/:id
DELETE /api/messages/:id
POST   /api/messages/:id/acknowledge
POST   /api/messages/:id/resolve
POST   /api/messages/:id/archive

// 消息通道
GET    /api/messages/channels
POST   /api/messages/channels
GET    /api/messages/channels/stats
GET    /api/messages/channels/:name
PUT    /api/messages/channels/:name
DELETE /api/messages/channels/:name
POST   /api/messages/channels/:name/test
GET    /api/messages/channels/:name/recipients
POST   /api/messages/channels/:name/recipients
DELETE /api/messages/channels/:name/recipients/:email
GET    /api/messages/channels/:name/filter
PUT    /api/messages/channels/:name/filter
PUT    /api/messages/channels/:name/enabled

// 数据推送
GET    /api/data-push
POST   /api/data-push
GET    /api/data-push/stats
GET    /api/data-push/:id
PUT    /api/data-push/:id
DELETE /api/data-push/:id
POST   /api/data-push/:id/test
POST   /api/data-push/:id/start
POST   /api/data-push/:id/stop
GET    /api/data-push/:id/logs

// AI Agent
GET    /api/agents
POST   /api/agents
GET    /api/agents/:id
PUT    /api/agents/:id
DELETE /api/agents/:id
POST   /api/agents/:id/execute
POST   /api/agents/:id/invoke
POST   /api/agents/:id/status
GET    /api/agents/:id/executions
GET    /api/agents/:id/executions/:execution_id
POST   /api/agents/:id/executions/details  # 批量获取
GET    /api/agents/:id/memory
DELETE /api/agents/:id/memory
GET    /api/agents/:id/stats
GET    /api/agents/:id/available-resources
POST   /api/agents/validate-cron
POST   /api/agents/validate-llm
GET    /api/agents/:id/messages
POST   /api/agents/:id/messages
DELETE /api/agents/:id/messages
DELETE /api/agents/:id/messages/:message_id

// 内存（Markdown）
GET    /api/memory
GET    /api/memory/export
GET    /api/memory/stats
GET    /api/memory/config
PUT    /api/memory/config
POST   /api/memory/compress
GET    /api/memory/category/:category
PUT    /api/memory/category/:category
GET    /api/memory/:source_type/:id
PUT    /api/memory/:source_type/:id
DELETE /api/memory/:source_type/:id

// MQTT管理
GET  /api/mqtt/status
GET  /api/mqtt/subscriptions
POST /api/mqtt/subscribe
POST /api/mqtt/unsubscribe
POST /api/mqtt/subscribe/:device_id
POST /api/mqtt/unsubscribe/:device_id

// 外部Broker
GET    /api/brokers
POST   /api/brokers
GET    /api/brokers/:id
PUT    /api/brokers/:id
DELETE /api/brokers/:id
POST   /api/brokers/:id/test

// 设置
GET  /api/settings/timezone
PUT  /api/settings/timezone
GET  /api/settings/timezones
GET  /api/settings/retention
PUT  /api/settings/retention
POST /api/settings/retention/cleanup

// 仪表板
GET    /api/dashboards
POST   /api/dashboards
GET    /api/dashboards/:id
PUT    /api/dashboards/:id
DELETE /api/dashboards/:id
POST   /api/dashboards/:id/components      # 添加
DELETE /api/dashboards/:id/components       # 移除
POST   /api/dashboards/:id/default
GET    /api/dashboards/templates
GET    /api/dashboards/templates/:id
POST   /api/dashboards/:id/share            # 创建分享
GET    /api/dashboards/:id/share            # 列出分享
DELETE /api/dashboards/:id/share/:token     # 撤销分享

// 扩展（写入操作）
POST   /api/extensions
POST   /api/extensions/sync
GET    /api/extensions/sync-status
DELETE /api/extensions/:id                  # 注销
DELETE /api/extensions/:id/uninstall
POST   /api/extensions/:id/start
POST   /api/extensions/:id/stop
POST   /api/extensions/:id/reload
POST   /api/extensions/:id/command
POST   /api/extensions/:id/invoke
GET    /api/extensions/:id/config
PUT    /api/extensions/:id/config
GET    /api/extensions/:id/logs
DELETE /api/extensions/:id/logs
GET    /api/extensions/:id/descriptor
GET    /api/extensions/:id/data-sources
GET    /api/extensions/:id/metrics/:metric/data
POST   /api/extensions/:id/push-metrics
POST   /api/extensions/market/install
POST   /api/extensions/upload/file          # 100MB限制

// 前端组件
GET    /api/frontend-components
POST   /api/frontend-components             # 安装（5MB限制）
GET    /api/frontend-components/:id
DELETE /api/frontend-components/:id
POST   /api/frontend-components/market/install

// 事件发布
POST /api/events

// 统计
GET /api/stats/devices
GET /api/stats/rules

// API Key管理
GET    /api/auth/keys
POST   /api/auth/keys
DELETE /api/auth/keys/:id

// 配置导入/导出
GET  /api/config/export
POST /api/config/import
POST /api/config/validate
```

### 管理员路由（JWT + Admin角色）

```rust
// 用户管理
GET    /api/users
POST   /api/users
DELETE /api/users/:username
```

## 服务器状态

```rust
pub struct ServerState {
    /// 事件总线
    pub event_bus: Arc<EventBus>,

    /// 设备服务
    pub device_service: Arc<DeviceService>,

    /// 会话管理器
    pub session_manager: Arc<SessionManager>,

    /// Agent服务
    pub agent_service: Arc<AgentService>,

    /// 规则引擎
    pub rule_engine: Arc<RuleEngine>,

    /// 消息服务
    pub message_service: Arc<MessageService>,

    /// 扩展注册表
    pub extension_registry: Arc<RwLock<ExtensionRegistry>>,

    /// 设置存储
    pub settings: Arc<SettingsStore>,

    /// 请求体大小限制
    pub max_body_size: usize,
}

impl ServerState {
    pub async fn new() -> Self {
        // 初始化所有服务
        // ...
    }
}
```

## 中间件

```rust
/// 速率限制中间件
pub async fn rate_limit_middleware(
    State(state): State<ServerState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>;

/// JWT认证中间件
pub async fn jwt_auth_middleware(
    State(state): State<ServerState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>;

/// 混合认证中间件（支持API Key或JWT）
pub async fn hybrid_auth_middleware(
    State(state): State<ServerState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>;
```

## WebSocket处理

### 事件WebSocket

```rust
pub async fn event_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> Response {
    ws.on_upgrade(|socket| async move {
        let mut event_rx = state.event_bus.subscribe();
        // 发送事件到WebSocket
        while let Ok(event) = event_rx.recv().await {
            let msg = serde_json::to_string(&event).unwrap();
            socket.send(Message::Text(msg)).await.unwrap();
        }
    })
}
```

### 聊天WebSocket

```rust
pub async fn ws_chat_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
    Query(params): ChatQuery,
) -> Response {
    ws.on_upgrade(|socket| async move {
        // 处理聊天消息
        // ...
    })
}
```

### SSE事件流

```rust
pub async fn event_stream_handler(
    State(state): State<ServerState>,
) -> SseResult {
    let mut event_rx = state.event_bus.subscribe();

    Sse::new(move || {
        while let Ok(event) = event_rx.recv().await {
            let json = serde_json::to_string(&event).unwrap();
            yield Event::default().json_data(json);
        }
    })
}
```

## 错误处理

```rust
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
    pub status: StatusCode,
}

impl ErrorResponse {
    pub fn bad_request(message: impl Into<String>) -> Self;
    pub fn unauthorized(message: impl Into<String>) -> Self;
    pub fn not_found(resource: impl Into<String>) -> Self;
    pub fn conflict(message: impl Into<String>) -> Self;
    pub fn internal(message: impl Into<String>) -> Self;
    pub fn gone(message: impl Into<String>) -> Self;
}
```

## OpenAPI文档

```rust
/// 创建Swagger UI路由
pub fn swagger_ui() -> Router {
    Router::new()
        .route("/swagger-ui", get(swagger_ui_handler))
        .route("/openapi.json", get(openapi_json_handler))
}
```

访问 `http://localhost:9375/api/docs` 查看API文档。

## 使用示例

### 启动服务器

```bash
# 默认配置
cargo run -p neomind-api

# 自定义配置
cargo run -p neomind-api -- --config config.toml

# 指定端口
SERVER_PORT=8080 cargo run -p neomind-api
```

### 环境变量

```bash
# 服务器
SERVER_PORT=9375
SERVER_HOST=0.0.0.0

# 数据库
DATABASE_PATH=./data

# 日志
RUST_LOG=info
```

## 设计原则

1. **RESTful**: 遵循REST设计原则
2. **版本化**: API版本通过路径管理
3. **文档驱动**: OpenAPI文档自动生成
4. **实时通信**: WebSocket + SSE支持
5. **认证灵活**: 支持JWT和API Key
