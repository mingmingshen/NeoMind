//! Protocol Mapping Layer
//!
//! This module provides the abstraction layer for mapping device capabilities
//! to protocol-specific addresses and data formats. It decouples the device
//! type definitions (MDL) from protocol implementations (MQTT, Modbus, HASS, etc.).
//!
//! ## Architecture
//!
//! ```text
//! Device Type Definition (MDL)       Protocol Mapping
//! ├─ temperature capability  ──────→  ├─ MQTT: sensor/${id}/temperature
//! ├─ humidity capability     ──────→  ├─ Modbus: register 0x0102, holding
//! └─ relay_state capability  ──────→  └─ HASS: switch.relay123
//! ```

pub mod hass_mapping;
pub mod mapping;
pub mod modbus_mapping;
pub mod mqtt_mapping;

// Re-exports
pub use hass_mapping::{HassDomain, HassEntityId, HassMapping, HassMappingBuilder};
pub use mapping::{
    Address, Capability, CapabilityType, MappingConfig, MappingError, MetricParser,
    ModbusRegisterType, PayloadSerializer, ProtocolMapping, SharedMapping,
};
pub use modbus_mapping::{ModbusDataType, ModbusMapping, ModbusMappingBuilder};
pub use mqtt_mapping::{BinaryFormat, MqttMapping, MqttMappingBuilder, MqttValueParser};
