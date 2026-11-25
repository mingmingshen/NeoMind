//! Optional redb persistence for [`crate::store::InMemoryAgentStore`].
//!
//! M1 tests stay in-memory. M2 host assembly attaches an [`AgentStoreV2`] so
//! create/update/invoke survive process restarts while schema version is still 1
//! (soft-copy) or 2 (migrated).

use std::sync::Arc;

use neomind_storage::agents_v2::{AgentExecutionRecordV2, AgentStoreV2, ExecutionStatusV2};

use crate::error::{AgentError, Result};
use crate::model::{
    AgentExecutionRecord, AgentRuntimeRecord, AgentSpec, AgentStatus, ExecutionStatus,
};
use crate::storage_bridge::{
    record_to_spec, runtime_to_storage, spec_to_record, status_from_storage,
};
use crate::store::InMemoryAgentStore;

/// Load every v2 spec (+ runtime) into an in-memory working set.
pub fn load_from_v2(v2: &AgentStoreV2, mem: &InMemoryAgentStore, now_ms: i64) -> Result<usize> {
    let specs = v2
        .list_specs()
        .map_err(|e| AgentError::Other(e.to_string()))?;
    let mut n = 0;
    for rec in specs {
        let spec = record_to_spec(&rec).map_err(AgentError::Other)?;
        let id = spec.id.clone();
        mem.save_spec(spec, now_ms)?;
        if let Ok(Some(rt)) = v2.get_runtime(&id) {
            let mapped = AgentRuntimeRecord {
                agent_id: rt.agent_id,
                status: status_from_storage(rt.status),
                stats: crate::model::AgentStats {
                    total_runs: rt.stats.total_runs,
                    successful_runs: rt.stats.successful_runs,
                    failed_runs: rt.stats.failed_runs,
                    partial_runs: rt.stats.partial_runs,
                    skipped_daily_limit: rt.stats.skipped_daily_limit,
                    skipped_permit_timeout: rt.stats.skipped_permit_timeout,
                    consecutive_failures: rt.stats.consecutive_failures,
                    last_execution_at: rt.stats.last_execution_at,
                    last_execution_id: rt.stats.last_execution_id,
                },
                error_message: rt.error_message,
                next_probe_at: rt.next_probe_at,
                updated_at: rt.updated_at,
            };
            mem.set_runtime(mapped)?;
        }
        n += 1;
    }
    Ok(n)
}

/// Persist a spec + runtime snapshot to the five-table store.
pub fn persist_spec(
    v2: &AgentStoreV2,
    spec: &AgentSpec,
    runtime: &AgentRuntimeRecord,
) -> Result<()> {
    v2.save_spec(&spec_to_record(spec))
        .map_err(|e| AgentError::Other(e.to_string()))?;
    v2.set_runtime(&runtime_to_storage(runtime))
        .map_err(|e| AgentError::Other(e.to_string()))?;
    Ok(())
}

fn exec_status_to_v2(s: &ExecutionStatus) -> ExecutionStatusV2 {
    match s {
        ExecutionStatus::Completed => ExecutionStatusV2::Completed,
        ExecutionStatus::Partial => ExecutionStatusV2::Partial,
        ExecutionStatus::Skipped => ExecutionStatusV2::Skipped,
        ExecutionStatus::Failed => ExecutionStatusV2::Failed,
    }
}

/// Persist an execution row (formal or dry-run) into agent_executions.
pub fn persist_execution(v2: &AgentStoreV2, exec: &AgentExecutionRecord) -> Result<()> {
    let row = AgentExecutionRecordV2 {
        id: exec.id.clone(),
        agent_id: exec.agent_id.clone(),
        config_version: exec.config_version,
        status: exec_status_to_v2(&exec.status),
        started_at: exec.started_at,
        finished_at: exec.finished_at,
        is_probe: exec.is_probe,
        dry_run: exec.dry_run,
        stale_config: exec.stale_config,
        evidence: serde_json::to_value(&exec.evidence).unwrap_or_default(),
        deliveries: exec
            .deliveries
            .iter()
            .filter_map(|d| serde_json::to_value(d).ok())
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect(),
        brief_text: exec.brief_text.clone(),
        conclusion: exec.conclusion.clone(),
        error: exec.error.clone(),
        alert_verdict: exec.alert_verdict.as_ref().map(|v| format!("{v:?}")),
    };
    v2.save_execution(&row)
        .map_err(|e| AgentError::Other(e.to_string()))?;
    Ok(())
}

/// Host-facing handle: in-memory hot path + optional durable mirror.
#[derive(Clone)]
pub struct DurableAgentStore {
    pub mem: InMemoryAgentStore,
    pub v2: Option<Arc<AgentStoreV2>>,
}

impl DurableAgentStore {
    pub fn memory_only() -> Self {
        Self {
            mem: InMemoryAgentStore::new(),
            v2: None,
        }
    }

    pub fn with_v2(v2: Arc<AgentStoreV2>, now_ms: i64) -> Result<Self> {
        let mem = InMemoryAgentStore::new();
        load_from_v2(&v2, &mem, now_ms)?;
        Ok(Self { mem, v2: Some(v2) })
    }

    pub fn save_spec(&self, spec: AgentSpec, now_ms: i64) -> Result<AgentSpec> {
        let saved = self.mem.save_spec(spec, now_ms)?;
        if let Some(v2) = &self.v2 {
            let rt = self.mem.get_runtime(&saved.id)?;
            persist_spec(v2, &saved, &rt)?;
        }
        Ok(saved)
    }

    pub fn delete_spec(&self, id: &str) -> Result<()> {
        self.mem.delete_spec(id)?;
        if let Some(v2) = &self.v2 {
            let _ = v2.delete_spec(id);
        }
        Ok(())
    }

    pub fn mirror_runtime(&self, runtime: &AgentRuntimeRecord) -> Result<()> {
        self.mem.set_runtime(runtime.clone())?;
        if let Some(v2) = &self.v2 {
            v2.set_runtime(&runtime_to_storage(runtime))
                .map_err(|e| AgentError::Other(e.to_string()))?;
        }
        Ok(())
    }

    pub fn mirror_execution(&self, exec: &AgentExecutionRecord) -> Result<()> {
        self.mem.save_execution(exec.clone())?;
        if let Some(v2) = &self.v2 {
            persist_execution(v2, exec)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn status_active_ids(&self) -> Vec<String> {
        self.mem
            .list_specs()
            .into_iter()
            .filter(|s| {
                self.mem
                    .get_runtime(&s.id)
                    .map(|r| matches!(r.status, AgentStatus::Active))
                    .unwrap_or(false)
            })
            .map(|s| s.id)
            .collect()
    }
}
