//! MQTT management handlers.
//! Settings are now managed via the plugin system.

pub mod brokers;
pub mod models;
pub mod status;
pub mod subscriptions;

// Re-export all handlers (excluding settings which are now plugin-managed)
pub use brokers::*;
pub use status::*;
pub use subscriptions::*;
