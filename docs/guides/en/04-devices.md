# Devices Module

**Package**: `neomind-devices`
**Version**: 0.8.0
**Completion**: 90%
**Purpose**: Device management and protocol adapters

## Overview

The Devices module is responsible for device registration, discovery, management, and communication with various device protocols.

## Module Structure

```
crates/neomind-devices/src/
├── lib.rs                      # Public interface and re-exports
├── adapter.rs                  # DeviceAdapter trait, DeviceEvent, ConnectionStatus
├── adapters/
│   ├── mod.rs                  # Adapter factory (create_adapter, available_adapters)
│   ├── mqtt.rs                 # MQTT adapter (full implementation)
│   └── webhook.rs              # Webhook adapter (passive data reception)
├── mdl.rs                      # MDL core types (MetricValue, DeviceState, etc.)
├── mdl_format.rs               # MDL format (DeviceTypeDefinition, CommandDefinition)
├── mqtt.rs                     # MQTT protocol utilities
├── protocol/
│   ├── mod.rs                  # Protocol mapping module
│   ├── mapping.rs              # Protocol mapping definitions
│   └── mqtt_mapping.rs         # MQTT topic/payload mapping
├── registry.rs                 # DeviceRegistry (DeviceConfig, DeviceTypeTemplate)
├── service.rs                  # DeviceService (unified API, commands, health)
├── telemetry.rs                # TimeSeriesStorage and MetricCache
├── unified_extractor.rs        # UnifiedExtractor for all adapters
└── embedded_broker.rs          # Embedded MQTT broker (feature-gated)
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

    /// Adapter type (mqtt, webhook, hass)
    pub adapter_type: String,

    /// Connection configuration (unified struct)
    pub connection_config: ConnectionConfig,

    /// ID of adapter managing this device
    pub adapter_id: Option<String>,

    /// Last seen timestamp (0 = never connected)
    pub last_seen: i64,
}

/// Unified connection configuration for different protocols
pub struct ConnectionConfig {
    // MQTT-specific
    pub telemetry_topic: Option<String>,
    pub command_topic: Option<String>,
    pub json_path: Option<String>,

    // HASS-specific
    pub entity_id: Option<String>,

    // Generic metadata
    pub extra: HashMap<String, serde_json::Value>,
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

    /// Definition mode: Simple (raw data + LLM) or Full (structured)
    pub mode: DeviceTypeMode,

    /// Metric definitions
    pub metrics: Vec<MetricDefinition>,

    /// Sample uplink data for Simple mode
    pub uplink_samples: Vec<serde_json::Value>,

    /// Command definitions
    pub commands: Vec<CommandDefinition>,
}

pub enum DeviceTypeMode {
    Simple,  // Raw data + LLM auto-discovery
    Full,    // Structured definitions
}
```

### 3. DeviceAdapter - Device Adapter Interface

```rust
#[async_trait]
pub trait DeviceAdapter: Send + Sync {
    /// Get adapter name
    fn name(&self) -> &str;

    /// Get adapter type identifier (e.g., "mqtt", "webhook")
    fn adapter_type(&self) -> &'static str;

    /// Check if adapter is running
    fn is_running(&self) -> bool;

    /// Start adapter
    async fn start(&self) -> AdapterResult<()>;

    /// Stop adapter
    async fn stop(&self) -> AdapterResult<()>;

    /// Subscribe to device events
    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = DeviceEvent> + Send + '_>>;

    /// Send command to device
    async fn send_command(
        &self,
        device_id: &str,
        command_name: &str,
        payload: String,
        topic: Option<String>,
    ) -> AdapterResult<()>;

    /// Get connection status
    fn connection_status(&self) -> ConnectionStatus;

    /// Get device count
    fn device_count(&self) -> usize;

    /// Subscribe to a device's data stream
    async fn subscribe_device(&self, device_id: &str) -> AdapterResult<()>;

    /// Unsubscribe from a device's data stream
    async fn unsubscribe_device(&self, device_id: &str) -> AdapterResult<()>;
}
```

## Adapter Implementations

### MQTT Adapter

The MQTT adapter provides full MQTT connectivity with:
- Auto-discovery of new devices
- Template-based data parsing
- Protocol mapping (topic/payload to metrics)
- Command sending via MQTT topics
- Integration with embedded broker (feature-gated)

**MQTT Topic Specification**:
```
{telemetry_topic}              # Device data (configurable per device)
{command_topic}                # Device commands (configurable per device)
```

### Webhook Adapter

The Webhook adapter receives device data via HTTP POST:

```rust
pub struct WebhookAdapterConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub allowed_ips: Vec<String>,
    pub blocked_ips: Vec<String>,
    pub rate_limit_per_minute: Option<u32>,
    pub storage_dir: Option<String>,
}
```

**Webhook URL Format**: `POST /api/devices/{device_id}/webhook`

**Features**:
- Passive data reception via webhook endpoint
- Device authentication support (API keys)
- IP whitelist/blacklist
- Request rate limiting
- Automatic device discovery
- Command support (via response body)

## Device Discovery

Device discovery is handled through the adapter system. Discovered devices generate `DeviceEvent::Discovery` events that trigger auto-onboarding:

```rust
pub struct DiscoveredDeviceInfo {
    pub device_id: String,
    pub device_type: String,
    pub name: Option<String>,
    pub endpoint: Option<String>,
    pub capabilities: Vec<String>,
    pub timestamp: i64,
    pub metadata: serde_json::Value,
}
```

Discovered devices can be:
1. Automatically analyzed with LLM
2. Enhanced with suggested device types
3. Approved or rejected by user
4. Converted to full device instances

## Device Registry

```rust
pub struct DeviceRegistry {
    /// Storage backend (in-memory or persistent via redb)
    store: Arc<DeviceRegistryStore>,

    /// Device type templates
    templates: DashMap<String, DeviceTypeTemplate>,
}

impl DeviceRegistry {
    /// Create in-memory registry
    pub fn new() -> Self;

    /// Create with persistence (redb)
    pub async fn with_persistence(path: impl AsRef<Path>) -> Result<Self>;

    /// Register device type template
    pub async fn register_template(&self, template: DeviceTypeTemplate) -> Result<()>;

    /// Register device
    pub async fn register_device(&self, config: DeviceConfig) -> Result<()>;

    /// Get device
    pub async fn get_device(&self, id: &str) -> Option<DeviceConfig>;

    /// List devices
    pub async fn list_devices(&self) -> Vec<DeviceConfig>;

    /// Update device
    pub async fn update_device(&self, id: &str, config: DeviceConfig) -> Result<()>;

    /// Delete device
    pub async fn delete_device(&self, id: &str) -> Result<()>;
}
```

## Device Service

```rust
pub struct DeviceService {
    registry: Arc<DeviceRegistry>,
    adapters: Arc<RwLock<HashMap<String, Arc<dyn DeviceAdapter>>>>,
    event_bus: Arc<EventBus>,
    telemetry: Arc<TimeSeriesStorage>,
    extension_command_router: Option<ExtensionCommandRouterFn>,
}

impl DeviceService {
    /// Create device service
    pub fn new(
        registry: Arc<DeviceRegistry>,
        event_bus: EventBus,
        telemetry: Arc<TimeSeriesStorage>,
    ) -> Self;

    /// Register adapter
    pub async fn register_adapter(
        &self,
        id: String,
        adapter: Arc<dyn DeviceAdapter>,
    ) -> Result<()>;

    /// Send command (auto-selects adapter and uses template for payload)
    pub async fn send_command(
        &self,
        device_id: &str,
        command_name: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<CommandHistoryRecord>;

    /// Get adapter stats
    pub async fn get_adapter_stats(&self) -> AdapterStats;

    /// Get device health
    pub async fn get_device_health(&self, device_id: &str) -> DeviceHealth;

    /// Set extension command router
    pub fn set_extension_command_router(&self, router: ExtensionCommandRouterFn);
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
GET    /api/devices/:id/current               # Current metric value
POST   /api/devices/current-batch             # Batch current values

# Commands
POST   /api/devices/:id/command/:command      # Send command
GET    /api/devices/:id/commands              # Command history

# Metrics Data
POST   /api/devices/:id/metrics               # Write metric data

# Telemetry
GET    /api/devices/:id/telemetry             # Telemetry data
GET    /api/devices/:id/telemetry/summary     # Telemetry summary

# BLE Provisioning
POST   /api/devices/ble-provision             # BLE device provisioning

# Device Types
GET    /api/device-types                      # List device types
POST   /api/device-types                      # Create device type
GET    /api/device-types/:id                  # Get device type
PUT    /api/device-types                      # Validate device type
DELETE /api/device-types/:id                  # Delete device type
POST   /api/device-types/generate-from-samples  # Generate from samples
POST   /api/device-types/cloud/import         # Import from cloud

# MDL Generation
POST   /api/devices/generate-mdl              # Generate MDL

# Device Discovery & Auto-Onboarding (Drafts)
GET    /api/devices/drafts                    # List draft devices
GET    /api/devices/drafts/:device_id         # Get draft device
PUT    /api/devices/drafts/:device_id         # Update draft
POST   /api/devices/drafts/:device_id/approve # Approve draft
POST   /api/devices/drafts/:device_id/reject  # Reject draft
POST   /api/devices/drafts/:device_id/analyze # Analyze with LLM
POST   /api/devices/drafts/:device_id/enhance # Enhance with LLM
GET    /api/devices/drafts/:device_id/suggest-types  # Suggest types
POST   /api/devices/drafts/cleanup            # Cleanup drafts
GET    /api/devices/drafts/type-signatures    # Get type signatures
GET    /api/devices/drafts/config             # Get onboard config
PUT    /api/devices/drafts/config             # Update onboard config
POST   /api/devices/drafts/upload             # Upload device data
```

## Usage Examples

### Register Device Type

```rust
use neomind_devices::{DeviceTypeTemplate, DeviceTypeMode};

let template = DeviceTypeTemplate::new("dht22_sensor", "DHT22 Temperature & Humidity Sensor")
    .with_description("DHT22-based temperature and humidity sensor")
    .with_category("sensor")
    .with_category("climate");

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
    connection_config: ConnectionConfig::new()
        .with_telemetry_topic("sensors/greenhouse/temp1"),
    adapter_id: Some("main-mqtt".to_string()),
    last_seen: 0,
};

service.register_device(device).await?;
```

### Send Command

```rust
let result = service.send_command(
    "greenhouse_fan_1",
    "turn_on",
    HashMap::new(),
).await?;
```

## Cleaned Up Features

The following features have been removed from the codebase:
- Modbus adapter (not implemented)
- Home Assistant discovery module (deprecated)
- Agent analysis tools (anomaly detection, trend analysis - unused)
- HTTP polling adapter (replaced by webhook adapter)
- Separate discovery module (now integrated into adapters)

## Design Principles

1. **Protocol Decoupling**: Support multiple protocols via adapter pattern
2. **Type-Driven**: Define device capabilities via DeviceTypeTemplate
3. **Event-Driven**: Device state changes notified via EventBus
4. **Template-Based Parsing**: Adapters use device templates to parse protocol data
5. **Extensible**: Easy to add new protocol adapters
6. **Unified Extraction**: UnifiedExtractor handles data extraction from all adapters
