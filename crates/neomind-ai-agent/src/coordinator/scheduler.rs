//! Pure Cron / Interval ticker — only submits when due.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use chrono::{TimeZone, Utc};
use cron::Schedule;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;

use super::{ExecutionCoordinator, SubmitSource};
use crate::model::{AgentStatus, TriggerSpec};
use crate::ports::ClockPort;
use crate::store::InMemoryAgentStore;

/// 1s tick loop that submits Active Cron/Interval agents via the coordinator.
pub struct AgentScheduler {
    coordinator: Arc<ExecutionCoordinator>,
    store: InMemoryAgentStore,
    clock: Arc<dyn ClockPort>,
    /// Last fire timestamp (ms) per agent — used for Interval spacing and Cron window.
    last_fired: Mutex<HashMap<String, i64>>,
    cancel: CancellationToken,
}

impl AgentScheduler {
    pub fn new(
        coordinator: Arc<ExecutionCoordinator>,
        store: InMemoryAgentStore,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            coordinator,
            store,
            clock,
            last_fired: Mutex::new(HashMap::new()),
            cancel: CancellationToken::new(),
        }
    }

    /// Spawn the 1s tick loop. Idempotent cancel via [`Self::stop`].
    pub fn start(self: Arc<Self>) {
        let cancel = self.cancel.clone();
        tokio::spawn(async move {
            tracing::info!("clean-slate agent scheduler started (1s tick)");
            let mut ticker = interval(Duration::from_secs(1));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            ticker.tick().await; // first tick completes immediately
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        tracing::info!("clean-slate agent scheduler stopped");
                        break;
                    }
                    _ = ticker.tick() => {
                        self.tick_once().await;
                    }
                }
            }
        });
    }

    pub fn stop(&self) {
        self.cancel.cancel();
    }

    async fn tick_once(&self) {
        let now = self.clock.now_ms();
        let specs = self.store.list_specs();
        for spec in specs {
            let Ok(runtime) = self.store.get_runtime(&spec.id) else {
                continue;
            };
            if !matches!(runtime.status, AgentStatus::Active) {
                continue;
            }
            let due = match &spec.trigger {
                TriggerSpec::Interval { every_secs } => {
                    self.interval_due(&spec.id, *every_secs, now, &runtime)
                }
                TriggerSpec::Cron { expr, timezone } => {
                    self.cron_due(&spec.id, expr, timezone, now)
                }
                TriggerSpec::Event { .. } | TriggerSpec::Manual => false,
            };
            if !due {
                continue;
            }
            {
                let mut g = self.last_fired.lock().expect("last_fired");
                g.insert(spec.id.clone(), now);
            }
            let id = spec.id.clone();
            let coordinator = self.coordinator.clone();
            tokio::spawn(async move {
                if let Err(e) = coordinator.submit(&id, SubmitSource::Schedule).await {
                    tracing::debug!(
                        agent_id = %id,
                        error = %e,
                        "clean-slate schedule submit skipped/failed"
                    );
                }
            });
        }
    }

    fn interval_due(
        &self,
        agent_id: &str,
        every_secs: u64,
        now: i64,
        runtime: &crate::model::AgentRuntimeRecord,
    ) -> bool {
        if every_secs == 0 {
            return false;
        }
        let period_ms = (every_secs as i64).saturating_mul(1000);
        let last = self
            .last_fired
            .lock()
            .expect("last_fired")
            .get(agent_id)
            .copied()
            .or(runtime.stats.last_execution_at)
            .unwrap_or(0);
        now.saturating_sub(last) >= period_ms
    }

    fn cron_due(&self, agent_id: &str, expr: &str, timezone: &str, now_ms: i64) -> bool {
        let normalized = neomind_core::cron::normalize(expr);
        let Ok(schedule) = Schedule::from_str(&normalized) else {
            tracing::warn!(agent_id, expr, "invalid cron expression");
            return false;
        };
        let last = self
            .last_fired
            .lock()
            .expect("last_fired")
            .get(agent_id)
            .copied()
            .unwrap_or(now_ms.saturating_sub(1000));

        let Some(from) = Utc.timestamp_millis_opt(last).single() else {
            return false;
        };
        let Some(until) = Utc.timestamp_millis_opt(now_ms).single() else {
            return false;
        };

        // Prefer the agent timezone when parseable; fall back to UTC.
        if let Ok(tz) = timezone.parse::<chrono_tz::Tz>() {
            let from_tz = from.with_timezone(&tz);
            for next in schedule.after(&from_tz).take(1) {
                let next_utc = next.with_timezone(&Utc);
                return next_utc <= until;
            }
            return false;
        }

        for next in schedule.after(&from).take(1) {
            return next <= until;
        }
        false
    }
}
