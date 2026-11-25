//! Host assembly for the clean-slate [`neomind_ai_agent`] runtime (M2).
//!
//! Opens the shared agents.redb via [`AgentStoreV2`], loads a
//! [`DurableAgentStore`], and builds [`AgentApplicationService`] +
//! [`ExecutionCoordinator`] with host-safe stub ports (LocalBriefLlm,
//! CountingSourcePort / HostSourcePort, logging FakeNotifyPort).

use std::sync::Arc;

use async_trait::async_trait;
use neomind_ai_agent::coordinator::{AgentScheduler, ExecutionCoordinator, SubmitSource};
use neomind_ai_agent::error::{AgentError, Result as AgentResult};
use neomind_ai_agent::input::{CountingSourcePort, InputPipeline};
use neomind_ai_agent::model::{
    AgentSpec, AgentSpecDraft, AgentSpecPatch, EffectivePlan, RunResponse,
};
use neomind_ai_agent::persist::{persist_spec, DurableAgentStore};
use neomind_ai_agent::ports::LocalBriefLlm;
use neomind_ai_agent::ports::{
    ClockPort, NotifyPort, SourceReadPort, SourceReadResult, SystemClock,
};
use neomind_ai_agent::runner::SinglePassRunner;
use neomind_ai_agent::service::{AgentApplicationService, NotifyCounter, WriteCounter};
use neomind_ai_agent::side_effects::{
    FakeNotifyPort, FakeSideEffectPort, Notifier, SideEffectBoundary,
};
use neomind_storage::agents_v2::AgentStoreV2;
use neomind_storage::AgentStore;

/// Source port that returns [`AgentError::Other`] when no fixture is registered.
/// Used as a host placeholder until real extension/device reads are wired.
pub struct HostSourcePort {
    inner: CountingSourcePort,
}

impl HostSourcePort {
    pub fn new() -> Self {
        Self {
            inner: CountingSourcePort::new(),
        }
    }
}

impl Default for HostSourcePort {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceReadPort for HostSourcePort {
    async fn read_extension(
        &self,
        extension_id: &str,
        operation: &str,
        params: &serde_json::Value,
        timeout_secs: u32,
    ) -> AgentResult<SourceReadResult> {
        match self
            .inner
            .read_extension(extension_id, operation, params, timeout_secs)
            .await
        {
            Ok(v) => Ok(v),
            Err(AgentError::Source { key, reason }) => Err(AgentError::Other(format!(
                "host source missing fixture ({key}): {reason}"
            ))),
            Err(e) => Err(e),
        }
    }

    async fn read_device_metric(
        &self,
        device_id: &str,
        field: &str,
        params: &serde_json::Value,
        timeout_secs: u32,
    ) -> AgentResult<SourceReadResult> {
        match self
            .inner
            .read_device_metric(device_id, field, params, timeout_secs)
            .await
        {
            Ok(v) => Ok(v),
            Err(AgentError::Source { key, reason }) => Err(AgentError::Other(format!(
                "host source missing fixture ({key}): {reason}"
            ))),
            Err(e) => Err(e),
        }
    }
}

/// Notify port that counts via FakeNotifyPort and logs each deliver.
struct LoggingNotifyPort {
    inner: Arc<FakeNotifyPort>,
}

#[async_trait]
impl NotifyPort for LoggingNotifyPort {
    async fn deliver(
        &self,
        channel: &str,
        subject: &str,
        body: &str,
        execution_id: &str,
    ) -> AgentResult<()> {
        tracing::info!(
            channel,
            subject,
            execution_id,
            body_len = body.len(),
            "clean-slate notify deliver"
        );
        self.inner
            .deliver(channel, subject, body, execution_id)
            .await
    }
}

/// Hosted clean-slate agent runtime (service + coordinator + durable store).
pub struct HostedAiAgentRuntime {
    pub service: Arc<AgentApplicationService>,
    pub coordinator: Arc<ExecutionCoordinator>,
    pub durable: DurableAgentStore,
    pub clock: Arc<dyn ClockPort>,
    scheduler: Arc<std::sync::Mutex<Option<Arc<AgentScheduler>>>>,
    started: std::sync::atomic::AtomicBool,
}

impl HostedAiAgentRuntime {
    /// Open AgentStoreV2 on the shared agents.redb and build service.
    ///
    /// While `CURRENT_SCHEMA_VERSION < 2`, do **not** hydrate soft-copied v1
    /// shadows into the hot set — legacy `AiAgentManager` still owns those IDs
    /// and would double-fire. Schema v2 open loads every v2 row and becomes the
    /// sole scheduler.
    pub fn build(agent_store: Arc<AgentStore>) -> AgentResult<Self> {
        let v2 = AgentStoreV2::from_arc(agent_store.db())
            .map_err(|e| AgentError::Storage(e.to_string()))?;
        let clock: Arc<dyn ClockPort> = Arc::new(SystemClock);
        let now = clock.now_ms();
        let durable = if neomind_storage::schema::CURRENT_SCHEMA_VERSION >= 2 {
            DurableAgentStore::with_v2(v2, now)?
        } else {
            // Soft-copy may have warmed v2 tables; keep them for migrate fidelity
            // but start the hot path empty so only POST /api/agents/v2 creates
            // enter the clean-slate kernel before schema cutover.
            DurableAgentStore {
                mem: neomind_ai_agent::store::InMemoryAgentStore::new(),
                v2: Some(v2),
            }
        };

        let sources: Arc<dyn SourceReadPort> = Arc::new(HostSourcePort::new());
        let fake_notify = Arc::new(FakeNotifyPort::new());
        let notify_port: Arc<dyn NotifyPort> = Arc::new(LoggingNotifyPort {
            inner: fake_notify.clone(),
        });
        let writes = Arc::new(FakeSideEffectPort::new());

        let input = InputPipeline::new(sources, clock.clone());
        let runner = Arc::new(SinglePassRunner::new(Arc::new(LocalBriefLlm)));
        let side_effects = SideEffectBoundary {
            store: durable.mem.clone(),
            notifier: Notifier::new(notify_port),
            side_effects: writes.clone(),
            clock: clock.clone(),
        };

        struct NotifyAdapter(Arc<FakeNotifyPort>);
        impl NotifyCounter for NotifyAdapter {
            fn count(&self) -> u64 {
                self.0.count()
            }
        }
        struct WriteAdapter(Arc<FakeSideEffectPort>);
        impl WriteCounter for WriteAdapter {
            fn count(&self) -> u64 {
                self.0.call_count()
            }
        }

        let service = Arc::new(AgentApplicationService {
            store: durable.mem.clone(),
            clock: clock.clone(),
            input,
            runner,
            side_effects,
            notify_counter: Arc::new(NotifyAdapter(fake_notify)),
            write_counter: Arc::new(WriteAdapter(writes)),
        });

        let coordinator = Arc::new(ExecutionCoordinator::new(
            service.clone(),
            durable.mem.clone(),
            clock.clone(),
        ));

        Ok(Self {
            service,
            coordinator,
            durable,
            clock,
            scheduler: Arc::new(std::sync::Mutex::new(None)),
            started: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// Start the Cron/Interval scheduler (1s tick). Idempotent.
    ///
    /// No-op while schema version stays at 1 — legacy AiAgentManager owns
    /// schedule/event ticks until M2 migrate activates.
    pub fn start(self: &Arc<Self>) {
        if neomind_storage::schema::CURRENT_SCHEMA_VERSION < 2 {
            tracing::info!(
                "Clean-slate scheduler deferred until schema v2 (CURRENT_SCHEMA_VERSION={})",
                neomind_storage::schema::CURRENT_SCHEMA_VERSION
            );
            return;
        }
        if self.started.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let scheduler = Arc::new(AgentScheduler::new(
            self.coordinator.clone(),
            self.durable.mem.clone(),
            self.clock.clone(),
        ));
        *self.scheduler.lock().expect("scheduler") = Some(scheduler.clone());
        scheduler.start();
        tracing::info!("Clean-slate AgentScheduler started (schema v2)");
    }

    pub fn has_agent(&self, id: &str) -> bool {
        self.durable.mem.get_spec(id).is_ok()
    }

    pub fn create(&self, draft: AgentSpecDraft) -> AgentResult<(AgentSpec, EffectivePlan)> {
        let (spec, plan) = self.service.create(draft)?;
        self.mirror_spec(&spec)?;
        Ok((spec, plan))
    }

    pub fn list(&self) -> Vec<(AgentSpec, EffectivePlan)> {
        self.service.list()
    }

    pub fn get(&self, id: &str) -> AgentResult<(AgentSpec, EffectivePlan)> {
        let spec = self.service.get(id)?;
        let plan = self.service.effective_plan(id)?;
        Ok((spec, plan))
    }

    pub fn update(
        &self,
        id: &str,
        patch: AgentSpecPatch,
    ) -> AgentResult<(AgentSpec, EffectivePlan)> {
        let (spec, plan) = self.service.update(id, patch)?;
        self.mirror_spec(&spec)?;
        Ok((spec, plan))
    }

    pub fn delete(&self, id: &str) -> AgentResult<()> {
        // DurableAgentStore deletes mem + optional v2 mirror.
        self.durable.delete_spec(id)
    }

    pub async fn invoke(&self, id: &str) -> AgentResult<RunResponse> {
        let resp = self.coordinator.submit(id, SubmitSource::Manual).await?;
        self.mirror_after_run(id, &resp)?;
        Ok(resp)
    }

    pub async fn dry_run(&self, id: &str) -> AgentResult<RunResponse> {
        let resp = self.service.dry_run(id).await?;
        // Dry-run stays out of formal history; optional v2 mirror of dry_run row omitted.
        Ok(resp)
    }

    pub async fn submit(&self, id: &str, source: SubmitSource) -> AgentResult<RunResponse> {
        let resp = self.coordinator.submit(id, source).await?;
        self.mirror_after_run(id, &resp)?;
        Ok(resp)
    }

    pub fn explain_last(&self, id: &str) -> AgentResult<String> {
        self.service.explain_last(id)
    }

    fn mirror_spec(&self, spec: &AgentSpec) -> AgentResult<()> {
        if let Some(v2) = &self.durable.v2 {
            let rt = self.durable.mem.get_runtime(&spec.id)?;
            persist_spec(v2, spec, &rt)?;
        }
        Ok(())
    }

    fn mirror_after_run(&self, id: &str, resp: &RunResponse) -> AgentResult<()> {
        if let Some(exec) = &resp.execution {
            if !exec.dry_run {
                self.durable.mirror_execution(exec)?;
                if let Ok(rt) = self.durable.mem.get_runtime(id) {
                    self.durable.mirror_runtime(&rt)?;
                }
            }
        }
        Ok(())
    }
}

/// Convenience wrapper used by ServerState construction.
pub fn build_hosted_runtime(
    agent_store: Arc<AgentStore>,
) -> AgentResult<Arc<HostedAiAgentRuntime>> {
    let runtime = Arc::new(HostedAiAgentRuntime::build(agent_store)?);
    Ok(runtime)
}
