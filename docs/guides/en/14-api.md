# API Module

**Package**: `neomind-api`
**Version**: 0.8.0
**Completion**: 95%
**Purpose**: REST/WebSocket API server

## Overview

The API module is based on Axum framework, providing REST API, WebSocket, and SSE endpoints. It serves as the bridge between the frontend and backend.

## Module Structure

```
crates/neomind-api/src/
├── lib.rs                      # Public interface
├── server/
│   ├── mod.rs                  # Server configuration
│   ├── router.rs               # Route definitions
│   ├── types.rs                # Server state
│   ├── assets.rs               # Static assets
│   ├── state/                  # State management
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
│   ├── automations.rs          # Automation API (rules + transforms)
│   ├── rules.rs                # Rules API
│   ├── messages.rs             # Messages API
│   ├── tools.rs                # Tools API
│   ├── memory.rs               # Memory API (Markdown-based)
│   ├── llm_backends.rs         # LLM backend API
│   ├── settings.rs             # Settings API
│   ├── mqtt/                   # MQTT API
│   │   ├── mod.rs
│   │   ├── brokers.rs          # Broker management
│   │   ├── models.rs           # Data models
│   │   ├── status.rs           # Status queries
│   │   └── subscriptions.rs    # Subscription management
│   ├── extensions.rs           # Extensions API
│   ├── extension_stream.rs     # Extension streaming (WebSocket)
│   ├── frontend_components.rs  # Frontend component API
│   ├── message_channels.rs     # Message channels API
│   ├── data_push.rs            # Data push API
│   ├── data.rs                 # Telemetry & data source API
│   ├── events.rs               # Events API
│   ├── ws/                     # WebSocket handlers
│   ├── auth.rs                 # Auth (API keys)
│   ├── auth_users.rs           # User management (JWT)
│   ├── basic.rs                # Basic endpoints
│   ├── capabilities.rs         # Capability API
│   ├── config.rs               # Config management
│   ├── dashboards.rs           # Dashboard API (with sharing)
│   ├── instances.rs            # Instance management API
│   ├── skills.rs               # Skills API
│   ├── stats.rs                # Statistics API
│   ├── suggestions.rs          # Suggestions API
│   ├── summarization.rs        # Summarization API
│   └── setup.rs                # Initial setup
├── models/                     # Data models
├── automation/                 # Automation engine
├── auth.rs                     # Auth middleware
├── auth_users.rs               # User auth
├── config.rs                   # Server config
├── rate_limit.rs               # Rate limiting
├── cache.rs                    # Response caching
├── crypto.rs                   # Crypto utilities
├── validator.rs                # Input validation
├── event_services.rs           # Event services
├── capability_providers.rs     # Capability providers
├── shutdown.rs                 # Graceful shutdown
└── startup.rs                  # Startup logic
```

## Important Changes (v0.8.0)

### New Modules
- `handlers/data_push.rs` - Data push API (scheduled data delivery to external endpoints)
- `handlers/frontend_components.rs` - Frontend component marketplace and management
- `handlers/instances.rs` - Remote instance management API
- `handlers/skills.rs` - Agent skill matching and management API
- `handlers/capabilities.rs` - Capability discovery API
- `handlers/summarization.rs` - Content summarization API
- `handlers/data.rs` - Unified telemetry and data source API
- `automation/` - Automation engine module

### Security Model
Routes are organized into four tiers:
- **Public routes**: Health checks, auth, setup, static metadata, extension read-only, share proxy, webhook
- **JWT routes**: User session management (me, logout, change-password)
- **Protected routes** (hybrid auth - API Key or JWT): All data operations, CRUD, write endpoints
- **Admin routes** (JWT + Admin role): User management (list, create, delete)

### Data Push API
Scheduled data delivery to external endpoints:

```rust
GET    /api/data-push           # List push targets
POST   /api/data-push           # Create push target
GET    /api/data-push/stats     # Push statistics
GET    /api/data-push/:id       # Get push target
PUT    /api/data-push/:id       # Update push target
DELETE /api/data-push/:id       # Delete push target
POST   /api/data-push/:id/test  # Test push target
POST   /api/data-push/:id/start # Start push target
POST   /api/data-push/:id/stop  # Stop push target
GET    /api/data-push/:id/logs  # List delivery logs
```

## Route Overview

### Public Routes (No Authentication)

```rust
// Health checks
GET /api/health
GET /api/health/status
GET /api/health/live
GET /api/health/ready
GET /api/system/network-info

// Auth
GET  /api/auth/status
GET  /api/auth/verify
POST /api/auth/login
POST /api/auth/register

// Initial setup
GET  /api/setup/status
POST /api/setup/initialize
POST /api/setup/complete
POST /api/setup/llm-config

// Read-only metadata (public - static schemas only)
GET /api/llm-backends/types
GET /api/llm-backends/types/:type/schema
GET /api/messages/channels/types
GET /api/messages/channels/types/:type/schema
GET /api/extensions
GET /api/extensions/types
GET /api/extensions/dashboard-components
GET /api/extensions/capabilities
GET /api/extensions/:id                # Get extension info
GET /api/extensions/:id/health         # Health check
GET /api/extensions/:id/commands       # List commands
GET /api/extensions/:id/components     # Dashboard components
GET /api/extensions/:id/assets/*       # Static assets
GET /api/extensions/:id/event-subscriptions
GET /api/extensions/:id/stream/capability
GET /api/extensions/:id/stream/sessions

// Capabilities & Tools (public - static metadata)
GET /api/capabilities
GET /api/capabilities/:name
GET /api/tools
GET /api/tools/:name

// Marketplace (public - read-only)
GET /api/extensions/market/list
GET /api/extensions/market/:id
GET /api/extensions/market/updates
GET /api/frontend-components/market/list
GET /api/frontend-components/:id/bundle
GET /api/device-types/cloud/list

// Suggestions (public - input hints)
GET /api/suggestions
GET /api/suggestions/categories

// Share API (public - access shared dashboards without auth)
GET /api/share/:token
ANY /api/share/:token/proxy/*path

// Webhook (public - external devices cannot carry JWT)
POST /api/devices/:id/webhook
POST /api/devices/webhook
GET  /api/devices/:id/webhook-url
```

### JWT Protected Routes

```rust
// User info and session management
GET  /api/auth/me
POST /api/auth/logout
POST /api/auth/change-password
```

### WebSocket Routes (Auth handled in handler)

```rust
// Event streaming WebSocket/SSE
GET /api/events/ws
GET /api/events/stream

// Chat WebSocket (JWT via ?token= parameter)
GET /api/chat

// Extension streaming WebSocket
GET /api/extensions/:id/stream
```

### Protected Routes (API Key or JWT)

```rust
// Telemetry & Data
GET /api/telemetry
GET /api/telemetry/stats
GET /api/data/sources
GET /api/stats/system

// LLM Backends (read - moved from public)
GET /api/llm-backends
GET /api/llm-backends/:id
GET /api/llm-backends/stats
GET /api/llm-backends/ollama/models
GET /api/llm-backends/llamacpp/server-info

// LLM Backends (write)
POST   /api/llm-backends
PUT    /api/llm-backends/:id
DELETE /api/llm-backends/:id
POST   /api/llm-backends/:id/test
POST   /api/llm-backends/:id/activate
GET    /api/llm-backends/:id/models
POST   /api/llm/generate

// Skills
GET    /api/skills
POST   /api/skills
POST   /api/skills/reload
GET    /api/skills/match
GET    /api/skills/:id
PUT    /api/skills/:id
DELETE /api/skills/:id

// Instances (remote backend management)
GET    /api/instances
POST   /api/instances
GET    /api/instances/:id
PUT    /api/instances/:id
DELETE /api/instances/:id
POST   /api/instances/:id/test

// Session management
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

// Device management
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

// Device Types
GET    /api/device-types
POST   /api/device-types
GET    /api/device-types/:id
PUT    /api/device-types          # validate
DELETE /api/device-types/:id
POST   /api/device-types/generate-from-samples
POST   /api/device-types/cloud/import
POST   /api/devices/generate-mdl

// Draft Devices (auto-onboarding)
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

// Rules management
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

// Automations (unified rules + transforms)
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

// Transforms (data processing)
GET    /api/automations/transforms
POST   /api/automations/transforms/process
POST   /api/automations/transforms/:id/test
POST   /api/automations/transforms/test-code
GET    /api/automations/transforms/metrics
GET    /api/automations/transforms/data-sources
GET    /api/automations/transforms/:id/data-sources
GET    /api/automations/transforms/data-sources/:data_source_id

// Messages
GET    /api/messages
POST   /api/messages
GET    /api/messages/stats
POST   /api/messages/cleanup
POST   /api/messages/acknowledge     # bulk
POST   /api/messages/resolve         # bulk
POST   /api/messages/delete          # bulk
GET    /api/messages/:id
DELETE /api/messages/:id
POST   /api/messages/:id/acknowledge
POST   /api/messages/:id/resolve
POST   /api/messages/:id/archive

// Message Channels
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

// Data Push
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

// AI Agents
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
POST   /api/agents/:id/executions/details  # batch get
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

// Memory (Markdown-based)
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

// MQTT Management
GET  /api/mqtt/status
GET  /api/mqtt/subscriptions
POST /api/mqtt/subscribe
POST /api/mqtt/unsubscribe
POST /api/mqtt/subscribe/:device_id
POST /api/mqtt/unsubscribe/:device_id

// External Brokers
GET    /api/brokers
POST   /api/brokers
GET    /api/brokers/:id
PUT    /api/brokers/:id
DELETE /api/brokers/:id
POST   /api/brokers/:id/test

// Settings
GET  /api/settings/timezone
PUT  /api/settings/timezone
GET  /api/settings/timezones
GET  /api/settings/retention
PUT  /api/settings/retention
POST /api/settings/retention/cleanup

// Dashboards
GET    /api/dashboards
POST   /api/dashboards
GET    /api/dashboards/:id
PUT    /api/dashboards/:id
DELETE /api/dashboards/:id
POST   /api/dashboards/:id/components      # add
DELETE /api/dashboards/:id/components       # remove
POST   /api/dashboards/:id/default
GET    /api/dashboards/templates
GET    /api/dashboards/templates/:id
POST   /api/dashboards/:id/share            # create share
GET    /api/dashboards/:id/share            # list shares
DELETE /api/dashboards/:id/share/:token     # revoke share

// Extensions (write operations)
POST   /api/extensions
POST   /api/extensions/sync
GET    /api/extensions/sync-status
DELETE /api/extensions/:id                  # unregister
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
POST   /api/extensions/upload/file          # 100MB limit

// Frontend Components
GET    /api/frontend-components
POST   /api/frontend-components             # install (5MB limit)
GET    /api/frontend-components/:id
DELETE /api/frontend-components/:id
POST   /api/frontend-components/market/install

// Event publishing
POST /api/events

// Statistics
GET /api/stats/devices
GET /api/stats/rules

// Auth keys management
GET    /api/auth/keys
POST   /api/auth/keys
DELETE /api/auth/keys/:id

// Config Import/Export
GET  /api/config/export
POST /api/config/import
POST /api/config/validate
```

### Admin Routes (JWT + Admin Role)

```rust
// User management
GET    /api/users
POST   /api/users
DELETE /api/users/:username
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

Visit `http://localhost:9375/api/docs` to view API documentation.

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
SERVER_PORT=9375
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
