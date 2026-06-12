//! Edge AI Rules Engine Crate
//!
//! This crate provides a rule engine with DSL support for the NeoMind platform.
//!
//! ## Features
//!
//! - **DSL Parser**: Human-readable rule definition language
//! - **Rule Engine**: Condition evaluation and action execution
//! - **Value Provider**: Integration with device metrics
//!
//! ## Example
//!
//! ```rust,no_run
//! use neomind_rules::{RuleEngine, RuleDslParser, InMemoryValueProvider};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let provider = std::sync::Arc::new(InMemoryValueProvider::new());
//!     let engine = RuleEngine::new(provider);
//!
//!     let dsl = r#"
//!         RULE "高温告警"
//!         WHEN sensor.temperature > 50
//!         DO
//!             NOTIFY "设备温度过高"
//!         END
//!     "#;
//!
//!     let rule_id = engine.add_rule_from_dsl(dsl).await?;
//!     println!("Added rule: {}", rule_id);
//!
//!     Ok(())
//! }
//! ```

pub mod dependencies;
pub mod device_integration;
pub mod dsl;
pub mod engine;
pub mod error;
pub mod extension_integration;
pub mod unified_provider;

pub mod history;
pub mod store;
pub mod validator;

// Re-exports (only types used externally via crate-root shortcut path)
pub use dsl::{ComparisonOperator, LogLevel, RuleAction, RuleCondition, RuleDslParser};
pub use engine::{
    CompiledRule, InMemoryValueProvider, RuleEngine, RuleId, RuleStatus, ValueProvider,
};
pub use error::RuleError;
pub use unified_provider::{ExtensionStorageLike, UnifiedValueProvider};
pub use validator::{
    AlertChannelInfo, CommandInfo, DeviceInfo, MetricDataType, MetricInfo, ParameterInfo,
    RuleValidator, ValidationContext,
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
