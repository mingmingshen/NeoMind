# API Module

**Package**: `neomind-api`
**Version**: 0.5.8
**Completion**: 90%
**Purpose**: REST/WebSocket API server

## Overview

The API module is based on Axum framework, providing REST API, WebSocket, and SSE endpoints. It serves as the bridge between the frontend and backend.

## Module Structure

```
crates/neomind-api/src/
├── lib.rs                      # Public interface
├── main.rs                     # Program entry
├── server/
│   ├── mod.rs                  # Server configuration
│   ├── router.rs               # Route definitions
│   ├── types.rs                # Server state
│   ├── assets.rs               # Static assets
│   ├── extension_metrics.rs    # Extension metrics storage service
│   ├── state/
│   │   ├── mod.rs              # State management
│   │   ├── agent_state.rs      # Agent state
│   │   ├── core_state.rs       # Core state
│   │   └── extension_state.rs  # Extension state
│   └── middleware/             # Middleware
├── handlers/                   # Request handlers
│   ├── mod.rs
│   ├── sessions.rs             # Session API
│   ├── devices/                # Device API
│   │   ├── mod.rs
│   │   ├── crud.rs             # CRUD operations
│   │   ├── models.rs           # Data models
│   │   ├── discovery.rs        # Device discovery
│   │   ├── metrics.rs          # Metric queries
│   │   ├── telemetry.rs        # Telemetry data
│   │   ├── types.rs            # Device types
│   │   ├── webhook.rs          # Webhook
│   │   ├── auto_onboard.rs     # Auto-onboarding
│   │   └── mdl.rs              # MDL format
│   ├── agents.rs               # Agent API
│   ├── automations.rs          # Automation API
│   ├── rules.rs                # Rules API
│   ├── messages.rs             # Messages API
│   ├── commands.rs             # Commands API
│   ├── decisions.rs            # Decisions API
│   ├── tools.rs                # Tools API
│   ├── memory.rs               # Memory API
│   ├── llm_backends.rs         # LLM backend API
│   ├── settings.rs             # Settings API
│   ├── mqtt/                   # MQTT API
│   │   ├── mod.rs
│   │   ├── brokers.rs          # Broker management
│   │   ├── models.rs           # Data models
│   │   ├── status.rs           # Status queries
│   │   └── subscriptions.rs    # Subscription management
│   ├── extensions.rs           # Extensions API
│   ├── plugins.rs              # Plugins API (deprecated)
│   ├── message_channels.rs     # Message channels API
│   ├── events.rs               # Events API
│   ├── ws/                     # WebSocket handlers
│   ├── bulk/                   # Bulk operations API
│   ├── auth.rs                 # Auth related
│   ├── auth_users.rs           # User management
│   ├── basic.rs                # Basic endpoints
│   ├── config.rs               # Config management
│   ├── dashboards.rs           # Dashboard API
│   ├── search.rs               # Search API
│   ├── stats.rs                # Statistics API
│   ├── suggestions.rs          # Suggestions API
│   ├── setup.rs                # Initial setup
│   └── test_data.rs            # Test data
├── models/                     # Data models
│   ├── error.rs                # Error response
│   └── openapi.rs              # OpenAPI documentation
└── utils/                      # Utility functions
```

## Important Changes (v0.5.x)

### New Modules
- `server/extension_metrics.rs` - Extension metrics storage service, unified management of extension time-series data
- `server/state/extension_state.rs` - Extension state management

### Extension Metrics Storage
Extension metrics are now stored in `data/timeseries.redb` via ExtensionMetricsStorage:

```rust
pub struct ExtensionMetricsStorage {
    metrics_storage: Arc<TimeSeriesStore>,
}
```

Storage format uses DataSourceId:
- `device_part`: `extension:{extension_id}`
- `metric_part`: `{metric_name}`

### Agent Status Update Fix
Fixed issue where Agent status stuck in "Executing" after completion:
- Removed `loadItems()` call after event handling
- WebSocket events now serve as single source of truth for status

## Route Overview

### Public Routes (No Authentication)

```rust
// Health checks
GET /api/health
GET /api/health/status
GET /api/health/live
GET /api/health/ready

// Auth status
GET /api/auth/status

// User registration/login
POST /api/auth/login
POST /api/auth/register

// Initial setup
GET  /api/setup/status
POST /api/setup/initialize
POST /api/setup/complete
POST /api/setup/llm-config

// Read-only metadata
GET /api/llm-backends/types
GET /api/llm-backends
GET /api/llm-backends/:id
GET /api/llm-backends/stats
GET /api/llm-backends/ollama/models
GET /api/device-adapters/types
GET /api/messages/channels/types
GET /api/messages/channels
GET /api/extensions
GET /api/extensions/types
GET /api/plugins/*  // Deprecated, kept for compatibility

// System info
GET /api/stats/system
GET /api/suggestions
GET /api/test-data/*
```

### JWT Protected Routes

```rust
// User info
GET  /api/auth/me
POST /api/auth/logout
POST /api/auth/change-password
```

### WebSocket Routes (Token via ?token=)

```rust
// Events stream
GET /api/events/ws
GET /api/events/stream

// Chat WebSocket
GET /api/chat
```

### API Key Protected Routes

```rust
// Session management
GET    /api/sessions
POST   /api/sessions
GET    /api/sessions/:id
PUT    /api/sessions/:id
DELETE /api/sessions/:id
POST   /api/sessions/:id/chat
GET    /api/sessions/:id/history
POST   /api/sessions/cleanup

// Device management
GET    /api/devices
POST   /api/devices
GET    /api/devices/:id
PUT    /api/devices/:id
DELETE /api/devices/:id
GET    /api/devices/:id/current
POST   /api/devices/current-batch
GET    /api/devices/:id/state
GET    /api/devices/:id/health
POST   /api/devices/:id/command/:command

// Device types
GET    /api/device-types
POST   /api/device-types
GET    /api/device-types/:id
PUT    /api/device-types/:id
DELETE /api/device-types/:id
PUT    /api/device-types/:id/validate
POST   /api/device-types/generate-mdl
POST   /api/device-types/from-sample

// Device discovery
POST   /api/devices/discover
GET    /api/devices/pending
POST   /api/devices/pending/:id/confirm
DELETE /api/devices/pending/:id/dismiss

// Device metrics
GET    /api/devices/:id/metrics/:metric
GET    /api/devices/:id/metrics/:metric/data
GET    /api/devices/:id/metrics/:metric/aggregate

// Device telemetry
GET    /api/devices/:id/telemetry
GET    /api/devices/:id/telemetry/summary

// Rules management
GET    /api/rules
POST   /api/rules
GET    /api/rules/:id
PUT    /api/rules/:id
DELETE /api/rules/:id
POST   /api/rules/:id/enable
POST   /api/rules/:id/test
GET    /api/rules/:id/history
POST   /api/rules/validate
POST   /api/rules/from-nl

// Automations
GET    /api/transforms
POST   /api/transforms
GET    /api/transforms/:id
PUT    /api/transforms/:id
DELETE /api/transforms/:id
POST   /api/transforms/:id/enable
POST   /api/transforms/:id/test
GET    /api/transforms/:id/history

// Messages
GET    /api/messages
POST   /api/messages
GET    /api/messages/:id
DELETE /api/messages/:id
POST   /api/messages/:id/acknowledge
POST   /api/messages/:id/resolve
POST   /api/messages/:id/archive
POST   /api/messages/acknowledge
POST   /api/messages/resolve
POST   /api/messages/delete
POST   /api/messages/cleanup
GET    /api/messages/stats

// Message channels
GET    /api/messages/channels
POST   /api/messages/channels
GET    /api/messages/channels/:name
PUT    /api/messages/channels/:name
DELETE /api/messages/channels/:name
POST   /api/messages/channels/:name/test
GET    /api/messages/channels/stats

// Agents
GET    /api/agents
POST   /api/agents
GET    /api/agents/:id
PUT    /api/agents/:id
DELETE /api/agents/:id
POST   /api/agents/:id/execute
GET    /api/agents/:id/executions
GET    /api/agents/:id/conversation
GET    /api/agents/:id/memory
POST   /api/agents/:id/control

// Decisions
GET    /api/decisions
GET    /api/decisions/:id
POST /api/decisions/:id/execute
POST /api/decisions/:id/approve
POST /api/decisions/:id/reject
DELETE /api/decisions/:id
GET /api/decisions/stats

// Commands
GET    /api/commands
GET    /api/commands/:id
POST   /api/commands/:id/retry
POST   /api/commands/:id/cancel
GET    /api/commands/stats
POST   /api/commands/cleanup

// Tools
GET    /api/tools
GET    /api/tools/:name/schema
POST /api/tools/:name/execute
GET    /api/tools/format-for-llm
GET    /api/tools/metrics

// Memory
GET    /api/memory/stats
POST   /api/memory/query
GET    /api/memory/short-term
POST   /api/memory/short-term
DELETE /api/memory/short-term
GET    /api/memory/mid-term/:session_id
GET    /api/memory/long-term/search
GET    /api/memory/long-term/category/:category
POST   /api/memory/long-term
POST   /api/memory/consolidate/:session_id

// LLM backends
POST /api/llm-backends
PUT  /api/llm-backends/:id
DELETE /api/llm-backends/:id
POST /api/llm-backends/:id/test
POST /api/llm-backends/apply-settings
GET  /api/llm-backends/:id/models

// Settings
POST /api/settings/llm
GET  /api/settings/llm
POST /api/settings/llm/test
GET  /api/settings/mqtt
POST /api/settings/mqtt
POST /api/settings/timezone

// MQTT Brokers
GET  /api/mqtt/brokers
POST /api/mqtt/brokers
GET  /api/mqtt/brokers/:id
PUT    /api/mqtt/brokers/:id
DELETE /api/mqtt/brokers/:id
POST /api/mqtt/brokers/:id/start
POST /api/mqtt/brokers/:id/stop
GET  /api/mqtt/brokers/:id/status
GET  /api/mqtt/brokers/:id/subscriptions
PUT    /api/mqtt/brokers/:id/subscriptions

// Extensions
POST /api/extensions/discover
POST /api/extensions
DELETE /api/extensions/:id
POST /api/extensions/:id/start
POST /api/extensions/:id/stop
POST /api/extensions/:id/command

// Dashboards
GET    /api/dashboards
POST /api/dashboards
GET    /api/dashboards/:id
PUT    /api/dashboards/:id
DELETE /api/dashboards/:id
POST /api/dashboards/:id/execute
GET /api/dashboards/templates
GET /api/dashboards/widgets

// Search
GET    /api/search
GET    /api/search/suggestions

// Statistics
GET    /api/stats/devices
GET    /api/stats/rules
GET    /api/stats/automation
```

## Server State

```rust
pub struct ServerState {
    /// Event bus
    pub event_bus: Arc<EventBus>,

    /// Device service
    pub device_service: Arc<DeviceService>,

    /// Session manager
    pub session_manager: Arc<SessionManager>,

    /// Agent service
    pub agent_service: Arc<AgentService>,

    /// Rule engine
    pub rule_engine: Arc<RuleEngine>,

    /// Message service
    pub message_service: Arc<MessageService>,

    /// Extension registry
    pub extension_registry: Arc<RwLock<ExtensionRegistry>>,

    /// Settings storage
    pub settings: Arc<SettingsStore>,

    /// Request body size limit
    pub max_body_size: usize,
}

impl ServerState {
    pub async fn new() -> Self {
        // Initialize all services
        // ...
    }
}
```

## Middleware

```rust
/// Rate limit middleware
pub async fn rate_limit_middleware(
    State(state): State<ServerState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>;

/// JWT auth middleware
pub async fn jwt_auth_middleware(
    State(state): State<ServerState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>;

/// Hybrid auth middleware (supports API Key or JWT)
pub async fn hybrid_auth_middleware(
    State(state): State<ServerState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>;
```

## WebSocket Handlers

### Events WebSocket

```rust
pub async fn event_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> Response {
    ws.on_upgrade(|socket| async move {
        let mut event_rx = state.event_bus.subscribe();
        // Send events to WebSocket
        while let Ok(event) = event_rx.recv().await {
            let msg = serde_json::to_string(&event).unwrap();
            socket.send(Message::Text(msg)).await.unwrap();
        }
    })
}
```

### Chat WebSocket

```rust
pub async fn ws_chat_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
    Query(params): ChatQuery,
) -> Response {
    ws.on_upgrade(|socket| async move {
        // Handle chat messages
        // ...
    })
}
```

### SSE Event Stream

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

## Error Handling

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

## OpenAPI Documentation

```rust
/// Create Swagger UI route
pub fn swagger_ui() -> Router {
    Router::new()
        .route("/swagger-ui", get(swagger_ui_handler))
        .route("/openapi.json", get(openapi_json_handler))
}
```

Visit `http://localhost:3000/swagger-ui` to view API documentation.

## Usage Examples

### Start Server

```bash
# Default config
cargo run -p neomind-api

# Custom config
cargo run -p neomind-api -- --config config.toml

# Custom port
SERVER_PORT=8080 cargo run -p neomind-api
```

### Environment Variables

```bash
# Server
SERVER_PORT=3000
SERVER_HOST=0.0.0.0

# Database
DATABASE_PATH=./data

# Logging
RUST_LOG=info
```

## Design Principles

1. **RESTful**: Follow REST design principles
2. **Versioned**: API versioning via path
3. **Documentation-Driven**: Auto-generated OpenAPI docs
4. **Real-time**: WebSocket + SSE support
5. **Flexible Auth**: Support both JWT and API Key
