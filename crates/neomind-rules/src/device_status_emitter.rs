//! Periodic emitter that refreshes the `__last_seen_age_secs` virtual metric
//! in `UnifiedValueProvider` for every device currently subscribed by a rule.
//!
//! Spawned once at server startup (see `crates/neomind-api/src/server/types.rs`).
//! Tick interval: 60s default, configurable via `with_tick_interval`.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use neomind_core::datasource::DataSourceId;
use neomind_devices::DeviceService;
use tokio::task::JoinHandle;

use crate::engine::RuleEngine;
use crate::models::RuleValue;
use crate::unified_provider::UnifiedValueProvider;

/// 60s default tick. Override via [`DeviceStatusEmitter::with_tick_interval`].
const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(60);

/// Virtual metric name emitted by this task.
pub const VIRTUAL_METRIC_NAME: &str = "__last_seen_age_secs";

pub struct DeviceStatusEmitter {
    rule_engine: Arc<RuleEngine>,
    provider: Arc<UnifiedValueProvider>,
    device_service: Arc<DeviceService>,
    tick_interval: Duration,
}

impl DeviceStatusEmitter {
    pub fn new(
        rule_engine: Arc<RuleEngine>,
        provider: Arc<UnifiedValueProvider>,
        device_service: Arc<DeviceService>,
    ) -> Self {
        Self {
            rule_engine,
            provider,
            device_service,
            tick_interval: DEFAULT_TICK_INTERVAL,
        }
    }

    pub fn with_tick_interval(mut self, d: Duration) -> Self {
        self.tick_interval = d;
        self
    }

    /// Spawn the background tick task. Holds an `Arc<Self>`.
    ///
    /// The returned `JoinHandle` should be held by `AppState` to keep the task alive.
    /// Dropping the handle does NOT cancel the task — it keeps running as long as the
    /// Tokio runtime is alive.
    pub fn start(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self.tick_interval);
            interval.tick().await; // skip the immediate first tick
            loop {
                interval.tick().await;
                self.emit_once().await;
            }
        })
    }

    /// One tick: refresh every subscribed device's `__last_seen_age_secs`.
    pub(crate) async fn emit_once(&self) {
        let device_ids = self
            .rule_engine
            .subscribed_virtual_metric_devices(VIRTUAL_METRIC_NAME);
        if device_ids.is_empty() {
            return;
        }
        let now = Utc::now().timestamp();
        for device_id in device_ids {
            // DeviceService::get_device_last_seen returns i64; 0 for unknown/never-connected.
            let last_seen = self.device_service.get_device_last_seen(&device_id).await;
            if last_seen <= 0 {
                continue;
            }
            let age = (now - last_seen).max(0) as f64;
            self.provider
                .update_device_value(&device_id, VIRTUAL_METRIC_NAME, age)
                .await;
            self.rule_engine
                .on_data_update(
                    &DataSourceId::device(&device_id, VIRTUAL_METRIC_NAME),
                    RuleValue::Number(age),
                )
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: full integration tests (mock DeviceService, verify on_data_update called,
    // verify never-connected skip) live in `tests/offline_rule_integration_test.rs`
    // because they need a real DeviceService instance. See Task 7.

    #[test]
    fn test_virtual_metric_name_constant() {
        assert_eq!(VIRTUAL_METRIC_NAME, "__last_seen_age_secs");
    }
}
