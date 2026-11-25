//! Host assembly for the clean-slate [`neomind_ai_agent`] runtime (M2).
//!
//! Opens the shared agents.redb via [`AgentStoreV2`], loads a
//! [`DurableAgentStore`], and builds [`AgentApplicationService`] +
//! [`ExecutionCoordinator`] with production host ports (LLM / sources /
//! notify / side-effects).

use std::sync::Arc;

use async_trait::async_trait;
use neomind_ai_agent::coordinator::{
    AgentScheduler, AgentSubmitPort, ExecutionCoordinator, SubmitSource,
};
use neomind_ai_agent::error::{AgentError, Result as AgentResult};
use neomind_ai_agent::input::InputPipeline;
use neomind_ai_agent::model::{
    AgentRuntimeRecord, AgentSpec, AgentSpecDraft, AgentSpecPatch, AgentStats, AgentStatus,
    EffectivePlan, RunResponse,
};
use neomind_ai_agent::persist::{persist_spec, DurableAgentStore};
use neomind_ai_agent::ports::{ClockPort, NotifyPort, SourceReadPort, SystemClock};
use neomind_ai_agent::runner::SinglePassRunner;
use neomind_ai_agent::service::AgentApplicationService;
use neomind_ai_agent::side_effects::{Notifier, SideEffectBoundary};
use neomind_ai_agent::storage_bridge::{record_to_spec, runtime_to_storage, status_from_storage};
use neomind_storage::agents_v2::{map_agent_v1_to_v2, AgentStoreV2};
use neomind_storage::{AgentStore, AiAgent};

use crate::agent_ports::{
    HostAgentDeps, HostLlmPort, HostNotifyPort, HostSideEffectPort, HostSourceReadPort,
};

/// Hosted clean-slate agent runtime (service + coordinator + durable store).
pub struct HostedAiAgentRuntime {
    pub service: Arc<AgentApplicationService>,
    pub coordinator: Arc<ExecutionCoordinator>,
    pub durable: DurableAgentStore,
    pub clock: Arc<dyn ClockPort>,
    /// Production ports retained for acceptance counters / diagnostics.
    pub sources: Arc<HostSourceReadPort>,
    pub notify: Arc<HostNotifyPort>,
    pub side_effects: Arc<HostSideEffectPort>,
    pub llm: Arc<HostLlmPort>,
    scheduler: Arc<std::sync::Mutex<Option<Arc<AgentScheduler>>>>,
    started: std::sync::atomic::AtomicBool,
}

/// Schedules ticks through [`HostedAiAgentRuntime::submit`] so Cron/Interval
/// share `mirror_after_run` with manual/event/rule paths.
struct HostedSubmit(Arc<HostedAiAgentRuntime>);

#[async_trait]
impl AgentSubmitPort for HostedSubmit {
    async fn submit(&self, agent_id: &str, source: SubmitSource) -> AgentResult<RunResponse> {
        self.0.submit(agent_id, source).await
    }
}

impl HostedAiAgentRuntime {
    /// Open AgentStoreV2 on the shared agents.redb and build service with real host ports.
    pub fn build(agent_store: Arc<AgentStore>, deps: HostAgentDeps) -> AgentResult<Self> {
        let v2 = AgentStoreV2::from_arc(agent_store.db())
            .map_err(|e| AgentError::Storage(e.to_string()))?;
        let clock: Arc<dyn ClockPort> = Arc::new(SystemClock);
        let now = clock.now_ms();
        let durable = if neomind_storage::schema::CURRENT_SCHEMA_VERSION >= 2 {
            DurableAgentStore::with_v2(v2, now)?
        } else {
            DurableAgentStore {
                mem: neomind_ai_agent::store::InMemoryAgentStore::new(),
                v2: Some(v2),
                fail_next_v2_delete: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            }
        };

        let sources = Arc::new(HostSourceReadPort::new(deps.clone()));
        let notify = Arc::new(HostNotifyPort::new(deps.message_manager.clone()));
        let writes = Arc::new(HostSideEffectPort::new(
            deps.device_service.clone(),
            deps.extension_registry.clone(),
        ));
        let llm = Arc::new(HostLlmPort::new());

        let input = InputPipeline::new(sources.clone() as Arc<dyn SourceReadPort>, clock.clone());
        let runner = Arc::new(SinglePassRunner::new(llm.clone()));
        let side_effects = SideEffectBoundary {
            store: durable.mem.clone(),
            notifier: Notifier::new(notify.clone() as Arc<dyn NotifyPort>),
            side_effects: writes.clone(),
            clock: clock.clone(),
        };

        let service = Arc::new(AgentApplicationService {
            store: durable.mem.clone(),
            clock: clock.clone(),
            input,
            runner,
            side_effects,
            notify_counter: notify.clone() as Arc<dyn neomind_ai_agent::service::NotifyCounter>,
            write_counter: writes.clone() as Arc<dyn neomind_ai_agent::service::WriteCounter>,
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
            sources,
            notify,
            side_effects: writes,
            llm,
            scheduler: Arc::new(std::sync::Mutex::new(None)),
            started: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// Start the Cron/Interval scheduler (1s tick). Idempotent.
    ///
    /// No-op while schema version stays at 1 (migrate not yet activated).
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
        let submit: Arc<dyn AgentSubmitPort> = Arc::new(HostedSubmit(self.clone()));
        let scheduler = Arc::new(AgentScheduler::new(
            submit,
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

    /// Map a legacy v1 [`AiAgent`] into the clean-slate hot set (+ optional v2
    /// mirror). Used by `/api/agents` create/update/status so Cron/Interval/
    /// Event ticks and invoke share one runtime after schema v2.
    pub fn ingest_v1_agent(&self, agent: &AiAgent) -> AgentResult<()> {
        let (rec, rt_rec, _, _) = map_agent_v1_to_v2(agent);
        let spec = record_to_spec(&rec).map_err(AgentError::Other)?;
        let now = self.clock.now_ms();
        // Hot set only — do NOT go through DurableAgentStore::save_spec /
        // mirror_spec: those re-encode via `spec_to_record` and wipe
        // `v1_compat` (resources, output_schema, parsed_intent, …).
        self.durable.mem.save_spec(spec, now)?;
        let runtime = AgentRuntimeRecord {
            agent_id: rt_rec.agent_id,
            status: status_from_storage(rt_rec.status),
            stats: AgentStats {
                total_runs: rt_rec.stats.total_runs,
                successful_runs: rt_rec.stats.successful_runs,
                failed_runs: rt_rec.stats.failed_runs,
                partial_runs: rt_rec.stats.partial_runs,
                skipped_daily_limit: rt_rec.stats.skipped_daily_limit,
                skipped_permit_timeout: rt_rec.stats.skipped_permit_timeout,
                consecutive_failures: rt_rec.stats.consecutive_failures,
                last_execution_at: rt_rec.stats.last_execution_at,
                last_execution_id: rt_rec.stats.last_execution_id,
            },
            error_message: rt_rec.error_message,
            next_probe_at: rt_rec.next_probe_at,
            updated_at: rt_rec.updated_at,
        };
        self.durable.mem.set_runtime(runtime.clone())?;
        if let Some(v2) = &self.durable.v2 {
            v2.save_spec(&rec)
                .map_err(|e| AgentError::Other(e.to_string()))?;
            v2.set_runtime(&runtime_to_storage(&runtime))
                .map_err(|e| AgentError::Other(e.to_string()))?;
        }
        Ok(())
    }

    /// Ensure a legacy agent id is present in the hot set (ingest if missing).
    pub fn ensure_v1_agent(&self, agent: &AiAgent) -> AgentResult<()> {
        if self.has_agent(&agent.id) {
            return Ok(());
        }
        self.ingest_v1_agent(agent)
    }

    /// Update hot-set + durable runtime status (Active/Paused/…).
    pub fn set_agent_status(&self, id: &str, status: AgentStatus) -> AgentResult<()> {
        let mut rt = self.durable.mem.get_runtime(id)?;
        rt.status = status;
        rt.updated_at = self.clock.now_ms();
        self.durable.mirror_runtime(&rt)
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
        self.durable.delete_spec(id)
    }

    pub async fn invoke(&self, id: &str) -> AgentResult<RunResponse> {
        let resp = self.coordinator.submit(id, SubmitSource::Manual).await?;
        self.mirror_after_run(id, &resp)?;
        Ok(resp)
    }

    pub async fn dry_run(&self, id: &str) -> AgentResult<RunResponse> {
        let resp = self.service.dry_run(id).await?;
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
    deps: HostAgentDeps,
) -> AgentResult<Arc<HostedAiAgentRuntime>> {
    let runtime = Arc::new(HostedAiAgentRuntime::build(agent_store, deps)?);
    Ok(runtime)
}

#[cfg(test)]
mod tests {
    use super::*;
    use neomind_ai_agent::model::{
        FreshnessPolicy, InputContract, InputModality, InputRequirement, InputSourceRef,
        MissingPolicy, NotifyPolicy, OutputContract, PacingPolicy, ToolPolicy, TriggerSpec,
    };
    use neomind_ai_agent::ports::FixedClock;
    use neomind_core::extension::ExtensionRegistry;
    use neomind_core::EventBus;
    use neomind_devices::telemetry::DataPoint;
    use neomind_devices::{DeviceRegistry, DeviceService, MetricValue, TimeSeriesStorage};
    use neomind_messages::channels::MessageChannel;
    use neomind_messages::{MessageManager, Result as MsgResult};
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::server::state::ExtensionMetricsStorage;

    struct CountingChannel {
        name: String,
        sends: AtomicU64,
    }

    #[async_trait]
    impl MessageChannel for CountingChannel {
        fn name(&self) -> &str {
            &self.name
        }
        fn channel_type(&self) -> &str {
            "test"
        }
        fn is_enabled(&self) -> bool {
            true
        }
        async fn send(&self, _message: &neomind_messages::Message) -> MsgResult<()> {
            self.sends.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    async fn test_deps() -> (HostAgentDeps, Arc<CountingChannel>, Arc<TimeSeriesStorage>) {
        let mm = Arc::new(MessageManager::new());
        let ch = Arc::new(CountingChannel {
            name: "ops".into(),
            sends: AtomicU64::new(0),
        });
        mm.channels().await.read().await.register(ch.clone()).await;

        let telemetry = Arc::new(TimeSeriesStorage::memory().unwrap());
        let now = chrono::Utc::now().timestamp_millis();
        telemetry
            .write(
                "device:meter-1",
                "kw",
                DataPoint::new(now, MetricValue::Float(1.5)),
            )
            .await
            .unwrap();

        let bus = EventBus::new();
        let registry = Arc::new(DeviceRegistry::new());
        let device_service = Arc::new(DeviceService::new(registry, bus));
        device_service
            .set_telemetry_storage(telemetry.clone())
            .await;

        let ext_reg = Arc::new(ExtensionRegistry::new());
        let ext_metrics = Arc::new(ExtensionMetricsStorage::with_shared_storage(
            telemetry.clone(),
        ));

        (
            HostAgentDeps::new(mm, telemetry.clone(), device_service, ext_reg, ext_metrics),
            ch,
            telemetry,
        )
    }

    #[tokio::test]
    async fn prod_builder_isolated_sinks_count_real_deliveries_and_reads() {
        let (deps, channel, _telemetry) = test_deps().await;
        let store = AgentStore::memory().unwrap();
        let runtime = HostedAiAgentRuntime::build(store, deps).unwrap();

        let draft = AgentSpecDraft {
            name: "sink-check".into(),
            description: None,
            user_prompt: "report".into(),
            llm_backend_id: None,
            preset: None,
            trigger: TriggerSpec::Manual,
            pacing: PacingPolicy::default(),
            input: InputContract {
                requirements: vec![InputRequirement {
                    key: "kw".into(),
                    source: InputSourceRef::DeviceMetric {
                        device_id: "meter-1".into(),
                        field: "kw".into(),
                    },
                    operation: None,
                    params: serde_json::json!({}),
                    auth_scope: None,
                    timeout_secs: 5,
                    result_type: InputModality::Metric,
                    freshness: FreshnessPolicy::Any,
                    on_missing: MissingPolicy::ReportStale,
                }],
            },
            tool_policy: ToolPolicy::Disabled,
            output: OutputContract::default(),
            notify: NotifyPolicy {
                channels: vec!["ops".into()],
                on_complete: true,
                on_alert: false,
                alert_depends_on: vec![],
                on_failure: true,
                pending_confirmation: false,
            },
            controlled_actions: vec![],
        };

        let (spec, _) = runtime.create(draft).unwrap();
        let resp = runtime.invoke(&spec.id).await.unwrap();
        assert!(resp.execution.is_some());
        assert!(
            runtime.sources.reads() >= 1,
            "host source port must hit telemetry"
        );
        assert!(
            runtime.notify.deliveries() >= 1,
            "host notify port must deliver"
        );
        assert!(
            channel.sends.load(Ordering::SeqCst) >= 1,
            "MessageManager must fan out to the counting channel"
        );
        // ToolPolicy::Disabled uses local brief — LLM not required for this path.
        assert_eq!(runtime.llm.requests(), 0);
    }

    #[tokio::test]
    async fn scheduler_tick_persists_execution_via_shared_submit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents.redb");
        let store = AgentStore::open(&path).unwrap();
        let (deps, _ch, _) = test_deps().await;
        let runtime = Arc::new(HostedAiAgentRuntime::build(store.clone(), deps).unwrap());

        // Swap SystemClock for FixedClock so interval is immediately due.
        // Rebuild a thin scheduler against HostedSubmit + fixed clock.
        let clock = Arc::new(FixedClock::new(10_000));
        let draft = AgentSpecDraft {
            name: "sched-persist".into(),
            description: None,
            user_prompt: "tick".into(),
            llm_backend_id: None,
            preset: None,
            trigger: TriggerSpec::Interval { every_secs: 1 },
            pacing: PacingPolicy::default(),
            input: InputContract::default(),
            tool_policy: ToolPolicy::Disabled,
            output: OutputContract::default(),
            notify: NotifyPolicy::default(),
            controlled_actions: vec![],
        };
        let (spec, _) = runtime.create(draft).unwrap();
        // Force last_execution_at none and advance clock past period via FixedClock on scheduler.
        let submit: Arc<dyn AgentSubmitPort> = Arc::new(HostedSubmit(runtime.clone()));
        // Patch runtime mem clock is SystemClock; interval_due uses scheduler clock.
        let scheduler = AgentScheduler::new(submit, runtime.durable.mem.clone(), clock.clone());
        // last=0 → due immediately at 10_000 with every_secs=1
        scheduler.tick_once().await;
        // Wait for spawned submit to finish mirroring.
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            if runtime
                .durable
                .mem
                .formal_history_len(&spec.id)
                .saturating_mul(1)
                >= 1
            {
                break;
            }
        }
        assert!(
            runtime.durable.mem.formal_history_len(&spec.id) >= 1,
            "hot formal history must record the scheduled run"
        );
        let exec_id = runtime
            .durable
            .mem
            .formal_history(&spec.id)
            .last()
            .map(|e| e.id.clone())
            .expect("execution id");
        let rt = runtime.durable.mem.get_runtime(&spec.id).unwrap();
        assert!(rt.stats.total_runs >= 1);

        drop(scheduler);
        drop(runtime);
        drop(store);

        let reopened = AgentStore::open(&path).unwrap();
        let v2 = AgentStoreV2::from_arc(reopened.db()).unwrap();
        let formal = v2.list_formal_executions(&spec.id).unwrap();
        assert!(
            !formal.is_empty(),
            "reopened redb must contain the scheduled formal run"
        );
        assert_eq!(formal.last().unwrap().id, exec_id);
        let rt2 = v2.get_runtime(&spec.id).unwrap().expect("runtime row");
        assert!(rt2.stats.total_runs >= 1);
        assert!(rt2.stats.last_execution_id.is_some() || rt2.stats.last_execution_at.is_some());
    }

    #[tokio::test]
    async fn delete_surfaces_injected_fail_and_success_removes_on_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents-del.redb");
        let store = AgentStore::open(&path).unwrap();
        let (deps, _, _) = test_deps().await;
        let runtime = HostedAiAgentRuntime::build(store.clone(), deps).unwrap();
        let (spec, _) = runtime
            .create(AgentSpecDraft {
                name: "to-delete".into(),
                description: None,
                user_prompt: "x".into(),
                llm_backend_id: None,
                preset: None,
                trigger: TriggerSpec::Manual,
                pacing: PacingPolicy::default(),
                input: InputContract::default(),
                tool_policy: ToolPolicy::Disabled,
                output: OutputContract::default(),
                notify: NotifyPolicy::default(),
                controlled_actions: vec![],
            })
            .unwrap();

        runtime
            .durable
            .fail_next_v2_delete
            .store(true, Ordering::SeqCst);
        let err = runtime.delete(&spec.id).unwrap_err();
        assert!(matches!(err, AgentError::Storage(_)));
        assert!(runtime.has_agent(&spec.id), "mem reconciled after fail");

        runtime.delete(&spec.id).unwrap();
        assert!(!runtime.has_agent(&spec.id));
        drop(runtime);
        drop(store);

        let reopened = AgentStore::open(&path).unwrap();
        let v2 = AgentStoreV2::from_arc(reopened.db()).unwrap();
        assert!(v2.get_spec(&spec.id).unwrap().is_none());
    }
}
