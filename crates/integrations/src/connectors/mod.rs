//! Connector implementations for various protocols.
//!
//! This module provides concrete implementations of the `Connector` trait
//! for different communication protocols.

#[cfg(feature = "mqtt")]
pub mod mqtt;

pub mod base;

// Re-exports
pub use base::{StreamConnector, StreamConnectorConfig};
#[cfg(feature = "mqtt")]
pub use mqtt::{MqttConfig, MqttConnector};
