# Devices Module

**Package**: `neomind-devices`
**Version**: 0.5.8
**Completion**: 85%
**Purpose**: Device management and protocol adapters

## Overview

The Devices module is responsible for device registration, discovery, management, and communication with various device protocols.

## Module Structure

```
crates/devices/src/
├── lib.rs                      # Public interface
├── adapters/
│   ├── mod.rs                  # Adapter factory
│   ├── mqtt.rs                 # MQTT adapter
│   ├── http.rs                 # HTTP polling adapter
│   └── webhook.rs              # Webhook adapter
├── mdl_format/
│   ├── mod.rs                  # MDL format definitions
│   ├── types.rs                # MDL types
│   └── builder.rs              # MDL builder
├── discovery.rs                # Device discovery
├── crud.rs                     # Device CRUD operations
├── registry.rs                 # Device registry
└── service.rs                  # Device service
```

## Core Concepts

### 1. DeviceConfig - Device Configuration

```rust
pub struct DeviceConfig {
    /// Device ID (unique)
    pub device_id: String,

    /// Device name
    pub name: String,

    /// Device type (links to DeviceTypeTemplate)
    pub device_type: String,

    /// Adapter type (mqtt, http, webhook)
    pub adapter_type: String,

    /// Connection configuration
    pub connection_config: ConnectionConfig,

    /// ID of adapter managing this device
    pub adapter_id: Option<String>,
}

pub enum ConnectionConfig {
    /// MQTT configuration
    Mqtt {
        topic: String,
        qos: Option<u8>,
        retain: Option<bool>,
    },

    /// HTTP polling configuration
    Http {
        url: String,
        interval_secs: u64,
        headers: Option<HashMap<String, String>>,
    },

    /// Webhook configuration
    Webhook {
        webhook_path: String,
        secret: Option<String>,
    },
}
```

### 2. DeviceTypeTemplate - Device Type Template

```rust
pub struct DeviceTypeTemplate {
    /// Device type ID (e.g., dht22_sensor)
    pub device_type: String,

    /// Display name
    pub name: String,

    /// Description
    pub description: String,

    /// Category tags (can be multiple)
    pub categories: Vec<String>,

    /// Metric definitions
    pub metrics: Vec<MetricDefinition>,

    /// Parameter definitions
    pub parameters: Vec<ParameterDefinition>,

    /// Command definitions
    pub commands: Vec<CommandDefinition>,
}

pub struct MetricDefinition {
    pub name: String,
    pub display_name: String,
    pub data_type: MetricDataType,
    pub unit: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub required: bool,
}

pub struct CommandDefinition {
    pub name: String,
    pub display_name: String,
    pub payload_template: String,  // JSON template
    pub parameters: Vec<ParameterDefinition>,
    pub samples: Vec<CommandSample>,
    pub llm_hints: String,  // AI hints
}
```

### 3. DeviceAdapter - Device Adapter Interface

```rust
#[async_trait]
pub trait DeviceAdapter: Send + Sync {
    /// Get adapter ID
    fn id(&self) -> &str;

    /// Get adapter type
    fn adapter_type(&self) -> &str;

    /// Start adapter
    async fn start(&self) -> Result<AdapterError>;

    /// Stop adapter
    async fn stop(&self) -> Result<AdapterError>;

    /// Send command to device
    async fn send_command(
        &self,
        device_id: &str,
        command: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value>;

    /// Subscribe to device events
    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = DeviceEvent> + Send>>;
}
```

## Adapter Implementations

### MQTT Adapter

```rust
pub struct MqttDeviceAdapter {
    /// Adapter ID
    id: String,

    /// MQTT client
    client: Arc<AsyncClient>,

    /// Subscribed topics
    subscriptions: Vec<String>,

    /// Event sender
    event_tx: mpsc::Sender<DeviceEvent>,
}

impl MqttDeviceAdapter {
    /// Create new MQTT adapter
    pub async fn new(
        id: String,
        broker_url: &str,
        subscriptions: Vec<String>,
    ) -> Result<Self>;

    /// Publish to topic
    pub async fn publish(
        &self,
        topic: &str,
        payload: &[u8],
        qos: u8,
        retain: bool,
    ) -> Result<()>;
}
```

**MQTT Topic Specification**:
```
sensors/{device_id}/data       # Device data
sensors/{device_id}/status     # Device status
actuators/{device_id}/command  # Device commands
actuators/{device_id}/result   # Command result
```

### HTTP Polling Adapter

```rust
pub struct HttpPollingAdapter {
    id: String,
    client: reqwest::Client,
    poll_tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

impl HttpPollingAdapter {
    /// Add polling task
    pub async fn add_poll_task(
        &self,
        device_id: String,
        url: String,
        interval_secs: u64,
    ) -> Result<()>;
}
```

### Webhook Adapter

```rust
pub struct WebhookAdapter {
    id: String,
    webhook_path: String,
    secret: Option<String>,
}

impl WebhookAdapter {
    /// Verify webhook signature
    pub fn verify_signature(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> bool;
}
```

## Device Discovery

```rust
pub struct DeviceDiscovery {
    /// Timeout duration (milliseconds)
    timeout_ms: u64,
}

impl DeviceDiscovery {
    /// Scan host ports
    pub async fn scan_ports(
        &self,
        host: &str,
        ports: Vec<u16>,
        timeout_ms: u64,
    ) -> Result<Vec<u16>>;

    /// Discover MQTT devices
    pub async fn discover_mqtt(
        &self,
        host: &str,
        port: u16,
    ) -> Result<Vec<DiscoveredDevice>>;

    /// Discover HTTP devices
    pub async fn discover_http(
        &self,
        host: &str,
        port: u16,
    ) -> Result<Vec<DiscoveredDevice>>;
}

pub struct DiscoveredDevice {
    pub device_type: Option<String>,
    pub host: String,
    pub port: u16,
    pub confidence: f32,
    pub info: HashMap<String, String>,
}
```

## Device Registry

```rust
pub struct DeviceRegistry {
    /// Storage backend
    store: Arc<DeviceRegistryStore>,

    /// Device type templates
    templates: RwLock<HashMap<String, DeviceTypeTemplate>>,
}

impl DeviceRegistry {
    /// Register device type template
    pub async fn register_template(
        &self,
        template: DeviceTypeTemplate,
    ) -> Result<()>;

    /// Register device
    pub async fn register_device(
        &self,
        config: DeviceConfig,
    ) -> Result<()>;

    /// Get device
    pub async fn get_device(&self, id: &str) -> Option<Device>;

    /// List devices
    pub async fn list_devices(&self) -> Vec<Device>;

    /// Update device
    pub async fn update_device(
        &self,
        id: &str,
        config: DeviceConfig,
    ) -> Result<()>;

    /// Delete device
    pub async fn delete_device(&self, id: &str) -> Result<()>;

    /// Get device status
    pub async fn get_device_status(&self, id: &str) -> DeviceStatus;
}
```

## Device Service

```rust
pub struct DeviceService {
    registry: Arc<DeviceRegistry>,
    adapters: Arc<RwLock<HashMap<String, Arc<dyn DeviceAdapter>>>>,
    event_bus: Arc<EventBus>,
}

impl DeviceService {
    /// Create device service
    pub fn new(
        registry: Arc<DeviceRegistry>,
        event_bus: Arc<EventBus>,
    ) -> Self;

    /// Register adapter
    pub async fn register_adapter(
        &self,
        id: String,
        adapter: Arc<dyn DeviceAdapter>,
    ) -> Result<()>;

    /// Send command
    pub async fn send_command(
        &self,
        device_id: &str,
        command: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value>;

    /// Get adapter stats
    pub async fn get_adapter_stats(&self) -> AdapterStats;
}
```

## MDL Format

MDL (Device Description Language) is used to describe device types:

```rust
pub struct MetricDataType;
impl MetricDataType {
    pub const BOOLEAN: &str = "boolean";
    pub const INTEGER: &str = "integer";
    pub const FLOAT: &str = "float";
    pub const STRING: &str = "string";
    pub const ARRAY: &str = "array";
    pub const OBJECT: &str = "object";
}
```

**MDL Example**:
```json
{
  "device_type": "dht22_sensor",
  "name": "DHT22 Temperature & Humidity Sensor",
  "description": "DHT22-based temperature and humidity sensor",
  "categories": ["sensor", "climate"],
  "metrics": [
    {
      "name": "temperature",
      "display_name": "Temperature",
      "data_type": "float",
      "unit": "°C",
      "min": -40.0,
      "max": 80.0
    },
    {
      "name": "humidity",
      "display_name": "Humidity",
      "data_type": "float",
      "unit": "%",
      "min": 0.0,
      "max": 100.0
    }
  ]
}
```

## API Endpoints

```
# Device Management
GET    /api/devices                           # List devices
POST   /api/devices                           # Add device
GET    /api/devices/:id                       # Get device details
PUT    /api/devices/:id                       # Update device
DELETE /api/devices/:id                       # Delete device
GET    /api/devices/:id/current               # Current value
POST   /api/devices/current-batch             # Batch current values
GET    /api/devices/:id/state                 # Device state
GET    /api/devices/:id/health                # Health check
POST   /api/devices/:id/refresh               # Refresh device

# Commands
POST   /api/devices/:id/command/:command     # Send command
GET    /api/devices/:id/commands              # Command history

# Metrics Data
GET    /api/devices/:id/metrics/:metric       # Read metric
GET    /api/devices/:id/metrics/:metric/data  # Query data
GET    /api/devices/:id/metrics/:metric/aggregate  # Aggregate data

# Telemetry
GET    /api/devices/:id/telemetry             # Telemetry data
GET    /api/devices/:id/telemetry/summary     # Telemetry summary

# Device Types
GET    /api/device-types                      # List device types
POST   /api/device-types                      # Create device type
GET    /api/device-types/:id                  # Get device type
PUT    /api/device-types/:id                  # Update device type
DELETE /api/device-types/:id                  # Delete device type
PUT    /api/device-types/:id/validate         # Validate device type
POST   /api/device-types/generate-mdl         # Generate MDL
POST   /api/device-types/from-sample          # Generate from samples

# Discovery
POST   /api/devices/discover                   # Device discovery
GET    /api/devices/pending                   # Pending devices
POST   /api/devices/pending/:id/confirm       # Confirm device
DELETE /api/devices/pending/:id/dismiss       # Dismiss device
```

## Usage Examples

### Register Device Type

```rust
use neomind_devices::{DeviceTypeTemplate, MetricDefinition, MetricDataType};

let template = DeviceTypeTemplate::new("dht22_sensor", "DHT22 Temperature & Humidity Sensor")
    .with_description("DHT22-based temperature and humidity sensor")
    .with_category("sensor")
    .with_category("climate")
    .with_metric(MetricDefinition {
        name: "temperature".to_string(),
        display_name: "Temperature".to_string(),
        data_type: MetricDataType::Float,
        unit: "°C".to_string(),
        min: Some(-40.0),
        max: Some(80.0),
        required: false,
    });

service.register_template(template).await?;
```

### Add Device

```rust
use neomind_devices::{DeviceConfig, ConnectionConfig};

let device = DeviceConfig {
    device_id: "greenhouse_temp_1".to_string(),
    name: "Greenhouse Temperature Sensor 1".to_string(),
    device_type: "dht22_sensor".to_string(),
    adapter_type: "mqtt".to_string(),
    connection_config: ConnectionConfig::mqtt(
        "sensors/greenhouse/temp1",
        None::<String>,
    ),
    adapter_id: Some("main-mqtt".to_string()),
};

service.register_device(device).await?;
```

### Send Command

```rust
let result = service.send_command(
    "greenhouse_fan_1",
    "turn_on",
    serde_json::json!({}),
).await?;
```

## Cleaned Up Features

The following features have been removed from the codebase:
- ✅ Modbus adapter (not implemented)
- ✅ Home Assistant discovery module (deprecated)
- ✅ Agent analysis tools (anomaly detection, trend analysis - unused)

## Design Principles

1. **Protocol Decoupling**: Support multiple protocols via adapter pattern
2. **Type-Driven**: Define device capabilities via DeviceTypeTemplate
3. **Event-Driven**: Device state changes notified via EventBus
4. **Extensible**: Easy to add new protocol adapters
