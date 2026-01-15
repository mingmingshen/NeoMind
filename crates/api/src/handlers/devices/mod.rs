//! Device management handlers.
//!
//! Provides REST API for device management with MDL support.

pub mod compat;
pub mod crud;
pub mod discovery;
pub mod hass;
pub mod mdl;
pub mod metrics;
pub mod models;
pub mod telemetry;
pub mod types;

// Re-export all handlers for use in routing
pub use crud::*;
pub use discovery::*;
pub use hass::*;
pub use mdl::*;
pub use metrics::*;
pub use telemetry::*;
pub use types::*;
