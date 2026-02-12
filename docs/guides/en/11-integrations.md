# Integrations Module

**Package**: `neomind-integrations`
**Version**: 0.5.8
**Completion**: 65%
**Purpose**: External system integration framework

## Overview

The Integrations module provides a unified framework for connecting NeoMind with external systems (MQTT, HTTP, WebSocket, etc.).

## Module Structure

```
crates/integrations/src/
├── lib.rs                      # Public interface
├── connectors/
│   ├── mod.rs                  # Connectors
│   ├── base.rs                 # Base connector
│   ├── mqtt.rs                 # MQTT connector
│   └── mod.rs
├── protocols/
│   └── mod.rs                  # Protocol adapters
├── registry.rs                 # Integration registry
└── types.rs                    # Type definitions
```

## Core Concepts (from core)

### Integration Trait

```rust
#[async_trait]
pub trait Integration: Send + Sync {
    /// Get metadata
    fn metadata(&self) -> &IntegrationMetadata;

    /// Get state
    fn state(&self) -> IntegrationState;

    /// Start
    async fn start(&self) -> Result<()>;

    /// Stop
    async fn stop(&self) -> Result<()>;

    /// Subscribe to event stream
    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = IntegrationEvent> + Send + '_>>;

    /// Send command
    async fn send_command(&self, command: IntegrationCommand) -> Result<IntegrationResponse>;
}
```

### Integration Types

```rust
pub enum IntegrationType {
    Mqtt,
    Http,
    WebSocket,
    Tasmota,
    Zigbee,
    Custom(String),
}
```

### Integration States

```rust
pub enum IntegrationState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error(String),
}
```

### Integration Events

```rust
pub enum IntegrationEvent {
    /// State changed
    StateChanged {
        old_state: IntegrationState,
        new_state: IntegrationState,
        timestamp: i64,
    },

    /// Data received
    Data {
        source: String,
        data_type: String,
        payload: Vec<u8>,
        metadata: serde_json::Value,
        timestamp: i64,
    },

    /// Discovery event
    Discovery {
        discovered_id: String,
        discovery_type: String,
        info: DiscoveredInfo,
        timestamp: i64,
    },

    /// Error
    Error {
        message: String,
        details: Option<String>,
        timestamp: i64,
    },
}
```

## Connectors

### BaseConnector

```rust
pub struct BaseConnector {
    /// Connection config
    config: ConnectorConfig,

    /// Current state
    state: Arc<AtomicU8>,

    /// Event sender
    event_tx: mpsc::Sender<IntegrationEvent>,

    /// Connection metrics
    metrics: Arc<ConnectionMetrics>,
}

#[async_trait]
impl Connector for BaseConnector {
    async fn start(&self) -> ConnectorResult<()> {
        // Establish connection
    }

    async fn stop(&self) -> ConnectorResult<()> {
        // Disconnect
    }

    fn state(&self) -> ConnectorState {
        // Return state
    }

    fn metrics(&self) -> ConnectionMetrics {
        self.metrics.clone()
    }
}
```

### Connection Metrics

```rust
pub struct ConnectionMetrics {
    /// Connected at
    pub connected_at: Option<i64>,

    /// Reconnect count
    pub reconnect_count: u64,

    /// Messages received
    pub messages_received: u64,

    /// Messages sent
    pub messages_sent: u64,

    /// Error count
    pub error_count: u64,

    /// Last error
    pub last_error: Option<String>,
}
```

## Transformer

Data transformer for converting between external formats and NeoMind format.

```rust
pub trait Transformer: Send + Sync {
    /// External data to NeoMind event
    fn to_event(&self, data: &[u8], ctx: &TransformationContext) -> Result<serde_json::Value>;

    /// NeoMind command to external format
    fn to_external(&self, command: &serde_json::Value, target_format: &str) -> Result<Vec<u8>>;

    /// Validate data format
    fn validate(&self, data: &[u8], format_type: &str) -> Result<()>;

    /// Supported input formats
    fn supported_input_formats(&self) -> Vec<String>;

    /// Supported output formats
    fn supported_output_formats(&self) -> Vec<String>;
}
```

### Transformation Context

```rust
pub struct TransformationContext {
    pub source_system: String,
    pub source_type: String,
    pub timestamp: i64,
    pub metadata: serde_json::Value,
    pub entity_id: Option<String>,
    pub topic: Option<String>,
}
```

### Value Transform

```rust
pub enum TransformType {
    Direct,
    Scale { scale: f64, offset: f64 },
    Enum { mapping: HashMap<String, serde_json::Value> },
    Format { template: String },
    Expression { expr: String },
}

pub struct ValueTransform {
    pub source: String,
    pub target: String,
    pub transform_type: TransformType,
    pub params: serde_json::Value,
}
```

## Integration Registry

```rust
pub struct IntegrationRegistry {
    /// Registered integrations
    integrations: RwLock<HashMap<String, DynIntegration>>,

    /// Event bus
    event_bus: EventBus,
}

impl IntegrationRegistry {
    /// Create registry
    pub fn new(event_bus: EventBus) -> Self;

    /// Register integration
    pub async fn register(&self, integration: DynIntegration) -> RegistryResult<()>;

    /// Unregister
    pub async fn unregister(&self, id: &str) -> RegistryResult<()>;

    /// Start all
    pub async fn start_all(&self) -> RegistryResult<()>;

    /// Stop all
    pub async fn stop_all(&self) -> RegistryResult<()>;

    /// Get integration
    pub async fn get(&self, id: &str) -> Option<DynIntegration>;

    /// List integrations
    pub async fn list(&self) -> Vec<DynIntegration>;
}
```

## Supported Integrations

### MQTT Integration

```rust
pub struct MqttIntegration {
    config: MqttConfig,
    client: AsyncClient,
    event_tx: mpsc::Sender<IntegrationEvent>,
}

pub struct MqttConfig {
    pub broker_url: String,
    pub client_id: String,
    pub subscriptions: Vec<String>,
    pub qos: u8,
}
```

### HTTP Integration

```rust
pub struct HttpIntegration {
    config: HttpConfig,
    client: reqwest::Client,
}

pub struct HttpConfig {
    pub base_url: String,
    pub poll_interval_secs: u64,
    pub endpoints: Vec<HttpEndpoint>,
}
```

### WebSocket Integration

```rust
pub struct WebSocketIntegration {
    config: WebSocketConfig,
    ws_stream: Option<WebSocketStream>,
}

pub struct WebSocketConfig {
    pub url: String,
    pub reconnect_interval_secs: u64,
}
```

## Entity Mapping

```rust
pub struct EntityMapping {
    pub external_id: String,
    pub internal_id: String,
    pub entity_type: String,
    pub config: MappingConfig,
    pub attribute_map: HashMap<String, String>,
    pub extra: serde_json::Value,
}

pub struct MappingConfig {
    pub auto_map: bool,
    pub value_transforms: Vec<ValueTransform>,
    pub unit_conversions: HashMap<String, UnitConversion>,
}
```

## Design Principles

1. **Unified Interface**: All integrations implement same trait
2. **Event-Driven**: Data flow via event streams
3. **Data Transformation**: Transformer handles format conversion
4. **Observable**: Complete connection metrics
