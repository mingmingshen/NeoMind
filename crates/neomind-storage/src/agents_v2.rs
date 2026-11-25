//! Schema-v2 agent tables and v1→v2 migration (M1 prep).
//!
//! M1 lands the five-table layout and the fidelity mapping helpers, but does
//! **not** bump [`crate::schema::CURRENT_SCHEMA_VERSION`] or wire this into
//! [`crate::agents::AgentStore::open`]. M2 activates startup migration.
//!
//! Storage must not depend on `neomind-ai-agent` — DTO shapes here mirror that
//! crate's contracts so rows stay JSON-compatible across the boundary.

use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::agents::{
    AgentMemory, AgentNotify, AgentResource, AgentSchedule, AgentStats, AgentStatus, AgentToolConfig,
    AiAgent, ExecutionMode, NotifyOn, ResourceType, ScheduleType, UserMessage,
};

// ── Table names (v2 layout) ─────────────────────────────────────────────────

const AGENTS_V1_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("agents");
const AGENT_SPECS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("agent_specs");
const AGENT_RUNTIME_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("agent_runtime");
const AGENT_EXECUTIONS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("agent_executions");
const AGENT_MEMORY_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("agent_memory");
const AGENT_USER_MESSAGES_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("agent_user_messages");

// ── V2 DTO types (JSON-compatible with neomind-ai-agent) ────────────────────

/// Tool access policy for a managed agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPolicy {
    Disabled,
    Scoped {
        allowed_tools: Vec<String>,
        #[serde(default)]
        allowed_commands: Vec<String>,
    },
    Unrestricted,
}

/// Three-way notify policy (complete / alert / failure) + migration flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NotifyPolicy {
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub on_complete: bool,
    #[serde(default)]
    pub on_alert: bool,
    #[serde(default)]
    pub alert_depends_on: Vec<String>,
    #[serde(default)]
    pub on_failure: bool,
    #[serde(default)]
    pub pending_confirmation: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TriggerSpec {
    Cron {
        expr: String,
        timezone: String,
    },
    Interval {
        every_secs: u64,
    },
    Event {
        #[serde(default)]
        filter: serde_json::Value,
    },
    Manual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PacingPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burst_merge_secs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_interval_secs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_runs_per_day: Option<u32>,
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
}

fn default_failure_threshold() -> u32 {
    3
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InputContract {
    #[serde(default)]
    pub requirements: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OutputContract {
    #[serde(default)]
    pub fields: Vec<serde_json::Value>,
    #[serde(default)]
    pub publish: bool,
}

/// Frozen static configuration written to `agent_specs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSpecRecord {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub user_prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_backend_id: Option<String>,
    pub trigger: TriggerSpec,
    #[serde(default)]
    pub pacing: PacingPolicy,
    #[serde(default)]
    pub input: InputContract,
    pub tool_policy: ToolPolicy,
    #[serde(default)]
    pub output: OutputContract,
    #[serde(default)]
    pub notify: NotifyPolicy,
    #[serde(default)]
    pub controlled_actions: Vec<serde_json::Value>,
    pub config_version: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgentStatsV2 {
    pub total_runs: u64,
    pub successful_runs: u64,
    pub failed_runs: u64,
    #[serde(default)]
    pub partial_runs: u64,
    #[serde(default)]
    pub skipped_daily_limit: u64,
    #[serde(default)]
    pub skipped_permit_timeout: u64,
    #[serde(default)]
    pub consecutive_failures: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_execution_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_execution_id: Option<String>,
}

/// High-frequency runtime row written to `agent_runtime`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRuntimeRecord {
    pub agent_id: String,
    pub status: AgentStatus,
    pub stats: AgentStatsV2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_probe_at: Option<i64>,
    pub updated_at: i64,
}

// ── Fidelity mapping (pure; unit-tested) ────────────────────────────────────

/// Map v1 tool settings → v2 [`ToolPolicy`].
///
/// Priority: `Structured` always → `Disabled` (no privilege expansion). Else
/// `enabled == false` → `Disabled`; non-empty allowlist → `Scoped`; otherwise
/// `Unrestricted`. `ResourceType::Command` bindings become `allowed_commands`.
pub fn map_tool_policy(
    execution_mode: ExecutionMode,
    tool_config: &Option<AgentToolConfig>,
    resources: &[AgentResource],
) -> ToolPolicy {
    if matches!(execution_mode, ExecutionMode::Structured) {
        return ToolPolicy::Disabled;
    }

    let allowed_commands: Vec<String> = resources
        .iter()
        .filter(|r| matches!(r.resource_type, ResourceType::Command))
        .map(|r| r.resource_id.clone())
        .collect();

    match tool_config {
        Some(cfg) if !cfg.enabled => ToolPolicy::Disabled,
        Some(cfg) if !cfg.allowed_tools.is_empty() => ToolPolicy::Scoped {
            allowed_tools: cfg.allowed_tools.clone(),
            allowed_commands,
        },
        // enabled=true with empty allowlist, or tool_config absent → unrestricted
        _ => ToolPolicy::Unrestricted,
    }
}

/// Map v1 [`AgentNotify`] → v2 [`NotifyPolicy`].
///
/// - Channels are preserved verbatim (empty stays empty — never invent `IM`).
/// - `Always` → `on_complete`; `Failure` → `on_failure`.
/// - `Judgment` without a clear destination → `pending_confirmation`.
/// - Absent notify (legacy unrouted) keeps `on_failure` as the failure floor.
pub fn map_notify_policy(notify: &Option<AgentNotify>) -> NotifyPolicy {
    match notify {
        None => NotifyPolicy {
            channels: Vec::new(),
            on_failure: true,
            ..NotifyPolicy::default()
        },
        Some(n) => {
            let channels = n.channels.clone();
            let mut policy = NotifyPolicy {
                channels: channels.clone(),
                ..NotifyPolicy::default()
            };
            match n.on {
                NotifyOn::Always => {
                    policy.on_complete = true;
                }
                NotifyOn::Failure => {
                    policy.on_failure = true;
                }
                NotifyOn::Judgment => {
                    // No deterministic destination → park for operator confirm.
                    // Non-empty channels count as a clear dest; empty does not.
                    if channels.is_empty() {
                        policy.pending_confirmation = true;
                    }
                }
            }
            policy
        }
    }
}

fn map_trigger(schedule: &AgentSchedule) -> TriggerSpec {
    match schedule.schedule_type {
        ScheduleType::Cron => TriggerSpec::Cron {
            expr: schedule
                .cron_expression
                .clone()
                .unwrap_or_else(|| "0 * * * *".into()),
            timezone: schedule
                .timezone
                .clone()
                .unwrap_or_else(|| "UTC".into()),
        },
        ScheduleType::Interval => TriggerSpec::Interval {
            every_secs: schedule.interval_seconds.unwrap_or(300),
        },
        ScheduleType::Event => {
            let filter = schedule
                .event_filter
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::Value::Null);
            TriggerSpec::Event { filter }
        }
        ScheduleType::Manual => TriggerSpec::Manual,
    }
}

fn map_pacing(agent: &AiAgent) -> PacingPolicy {
    match &agent.operator_config {
        Some(cfg) => PacingPolicy {
            burst_merge_secs: Some(cfg.debounce_secs),
            min_interval_secs: cfg.event_cooldown_secs,
            max_runs_per_day: cfg.max_calls_per_day,
            failure_threshold: u32::from(cfg.consecutive_failure_threshold),
        },
        None => PacingPolicy::default(),
    }
}

fn map_output(agent: &AiAgent) -> OutputContract {
    let fields: Vec<serde_json::Value> = agent
        .output_schema
        .as_ref()
        .map(|schema| {
            schema
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "name": f.name,
                        "depends_on": [],
                        "description": f.description,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let publish = matches!(agent.execution_mode, ExecutionMode::Structured) && !fields.is_empty();
    OutputContract { fields, publish }
}

fn map_stats(stats: &AgentStats, consecutive_failures: u32, last_execution_at: Option<i64>) -> AgentStatsV2 {
    AgentStatsV2 {
        total_runs: stats.total_executions,
        successful_runs: stats.successful_executions,
        failed_runs: stats.failed_executions,
        partial_runs: 0,
        skipped_daily_limit: 0,
        skipped_permit_timeout: 0,
        consecutive_failures,
        last_execution_at,
        last_execution_id: None,
    }
}

/// Convert a v1 [`AiAgent`] row into the v2 spec + runtime + user-message slices.
pub fn map_agent_v1_to_v2(
    agent: &AiAgent,
) -> (AgentSpecRecord, AgentRuntimeRecord, Vec<UserMessage>, AgentMemory) {
    let spec = AgentSpecRecord {
        id: agent.id.clone(),
        name: agent.name.clone(),
        description: agent.description.clone(),
        user_prompt: agent.user_prompt.clone(),
        llm_backend_id: agent.llm_backend_id.clone(),
        trigger: map_trigger(&agent.schedule),
        pacing: map_pacing(agent),
        input: InputContract::default(),
        tool_policy: map_tool_policy(agent.execution_mode, &agent.tool_config, &agent.resources),
        output: map_output(agent),
        notify: map_notify_policy(&agent.notify),
        controlled_actions: Vec::new(),
        config_version: 1,
        created_at: agent.created_at,
        updated_at: agent.updated_at,
    };
    let runtime = AgentRuntimeRecord {
        agent_id: agent.id.clone(),
        status: agent.status,
        stats: map_stats(&agent.stats, agent.consecutive_failures, agent.last_execution_at),
        error_message: agent.error_message.clone(),
        next_probe_at: None,
        updated_at: agent.updated_at,
    };
    (
        spec,
        runtime,
        agent.user_messages.clone(),
        agent.memory.clone(),
    )
}

// ── Migration ───────────────────────────────────────────────────────────────

fn table_exists(db: &Database, name: TableDefinition<&str, &[u8]>) -> Result<bool, String> {
    let read = db.begin_read().map_err(|e| e.to_string())?;
    Ok(read.open_table(name).is_ok())
}

/// Create the five v2 tables and migrate rows out of the legacy `agents` table.
///
/// Idempotent: a second call when `agents` is already gone is a no-op beyond
/// ensuring the five tables exist. Does **not** stamp schema version 2.
pub fn migrate_agents_v1_to_v2(db: &Database) -> Result<(), String> {
    // Always ensure the five tables exist (even on a fresh / already-migrated DB).
    {
        let write = db.begin_write().map_err(|e| e.to_string())?;
        write
            .open_table(AGENT_SPECS_TABLE)
            .map_err(|e| e.to_string())?;
        write
            .open_table(AGENT_RUNTIME_TABLE)
            .map_err(|e| e.to_string())?;
        write
            .open_table(AGENT_EXECUTIONS_TABLE)
            .map_err(|e| e.to_string())?;
        write
            .open_table(AGENT_MEMORY_TABLE)
            .map_err(|e| e.to_string())?;
        write
            .open_table(AGENT_USER_MESSAGES_TABLE)
            .map_err(|e| e.to_string())?;
        write.commit().map_err(|e| e.to_string())?;
    }

    if !table_exists(db, AGENTS_V1_TABLE)? {
        return Ok(());
    }

    // Snapshot v1 rows under a read transaction.
    let v1_rows: Vec<(String, Vec<u8>)> = {
        let read = db.begin_read().map_err(|e| e.to_string())?;
        let table = read
            .open_table(AGENTS_V1_TABLE)
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        let iter = table.iter().map_err(|e| e.to_string())?;
        for entry in iter {
            let (k, v) = entry.map_err(|e| e.to_string())?;
            out.push((k.value().to_string(), v.value().to_vec()));
        }
        out
    };

    let write = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut specs = write
            .open_table(AGENT_SPECS_TABLE)
            .map_err(|e| e.to_string())?;
        let mut runtime = write
            .open_table(AGENT_RUNTIME_TABLE)
            .map_err(|e| e.to_string())?;
        let mut memory = write
            .open_table(AGENT_MEMORY_TABLE)
            .map_err(|e| e.to_string())?;
        let mut user_msgs = write
            .open_table(AGENT_USER_MESSAGES_TABLE)
            .map_err(|e| e.to_string())?;
        // Touch executions so the table is definitely present in this txn.
        let _executions = write
            .open_table(AGENT_EXECUTIONS_TABLE)
            .map_err(|e| e.to_string())?;

        for (id, bytes) in &v1_rows {
            let agent: AiAgent = serde_json::from_slice(bytes)
                .map_err(|e| format!("deserialize agent {id}: {e}"))?;
            let (spec, rt, messages, mem) = map_agent_v1_to_v2(&agent);

            let spec_bytes =
                serde_json::to_vec(&spec).map_err(|e| format!("serialize spec {id}: {e}"))?;
            specs
                .insert(id.as_str(), spec_bytes.as_slice())
                .map_err(|e| e.to_string())?;

            let rt_bytes =
                serde_json::to_vec(&rt).map_err(|e| format!("serialize runtime {id}: {e}"))?;
            runtime
                .insert(id.as_str(), rt_bytes.as_slice())
                .map_err(|e| e.to_string())?;

            let mem_bytes =
                serde_json::to_vec(&mem).map_err(|e| format!("serialize memory {id}: {e}"))?;
            memory
                .insert(id.as_str(), mem_bytes.as_slice())
                .map_err(|e| e.to_string())?;

            let msg_bytes = serde_json::to_vec(&messages)
                .map_err(|e| format!("serialize user_messages {id}: {e}"))?;
            user_msgs
                .insert(id.as_str(), msg_bytes.as_slice())
                .map_err(|e| e.to_string())?;
        }
    }

    // Drop the legacy combined table inside the same write transaction.
    write
        .delete_table(AGENTS_V1_TABLE)
        .map_err(|e| e.to_string())?;
    write.commit().map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{AgentSchedule, AgentStats, AgentStatus, ExecutionMode, ScheduleType};

    fn bare_resource(resource_type: ResourceType, resource_id: &str) -> AgentResource {
        AgentResource {
            resource_type,
            resource_id: resource_id.into(),
            name: resource_id.into(),
            config: serde_json::Value::Null,
        }
    }

    fn fixture_agent() -> AiAgent {
        AiAgent {
            id: "a1".into(),
            name: "fixture".into(),
            description: None,
            user_prompt: "p".into(),
            llm_backend_id: None,
            parsed_intent: None,
            resources: vec![],
            schedule: AgentSchedule {
                schedule_type: ScheduleType::Manual,
                cron_expression: None,
                interval_seconds: None,
                event_filter: None,
                timezone: None,
            },
            status: AgentStatus::Active,
            priority: 128,
            created_at: 1,
            updated_at: 2,
            last_execution_at: None,
            stats: AgentStats::default(),
            memory: AgentMemory::default(),
            conversation_history: vec![],
            user_messages: vec![],
            conversation_summary: None,
            context_window_size: 10,
            enable_tool_chaining: false,
            max_chain_depth: 10,
            tool_config: None,
            execution_mode: ExecutionMode::Free,
            error_message: None,
            system_prompt: None,
            max_retries: 0,
            consecutive_failures: 0,
            output_schema: None,
            operator_config: None,
            memory_mode: None,
            notify: None,
            max_tokens_per_run: None,
        }
    }

    #[test]
    fn structured_maps_to_disabled_no_privilege_expansion() {
        // Even with a broad allowlist, Structured must not gain tools.
        let cfg = Some(AgentToolConfig {
            enabled: true,
            allowed_tools: vec!["shell".into(), "memory".into()],
        });
        let resources = vec![bare_resource(ResourceType::Command, "relay.on")];
        let policy = map_tool_policy(ExecutionMode::Structured, &cfg, &resources);
        assert_eq!(
            policy,
            ToolPolicy::Disabled,
            "Structured must stay Disabled (no privilege expansion)"
        );
    }

    #[test]
    fn tool_config_disabled_maps_to_disabled() {
        let cfg = Some(AgentToolConfig {
            enabled: false,
            allowed_tools: vec!["shell".into()],
        });
        assert_eq!(
            map_tool_policy(ExecutionMode::Free, &cfg, &[]),
            ToolPolicy::Disabled
        );
    }

    #[test]
    fn non_empty_allowlist_maps_to_scoped_with_commands() {
        let cfg = Some(AgentToolConfig {
            enabled: true,
            allowed_tools: vec!["shell".into()],
        });
        let resources = vec![
            bare_resource(ResourceType::Command, "cam.snapshot"),
            bare_resource(ResourceType::Device, "dev-1"),
        ];
        match map_tool_policy(ExecutionMode::Focused, &cfg, &resources) {
            ToolPolicy::Scoped {
                allowed_tools,
                allowed_commands,
            } => {
                assert_eq!(allowed_tools, vec!["shell".to_string()]);
                assert_eq!(allowed_commands, vec!["cam.snapshot".to_string()]);
            }
            other => panic!("expected Scoped, got {other:?}"),
        }
    }

    #[test]
    fn empty_or_absent_allowlist_maps_to_unrestricted() {
        assert_eq!(
            map_tool_policy(ExecutionMode::Free, &None, &[]),
            ToolPolicy::Unrestricted
        );
        let cfg = Some(AgentToolConfig {
            enabled: true,
            allowed_tools: vec![],
        });
        assert_eq!(
            map_tool_policy(ExecutionMode::Focused, &cfg, &[]),
            ToolPolicy::Unrestricted
        );
    }

    #[test]
    fn empty_channels_stay_silent() {
        let notify = Some(AgentNotify {
            channels: vec![],
            on: NotifyOn::Always,
        });
        let policy = map_notify_policy(&notify);
        assert!(
            policy.channels.is_empty(),
            "empty channels must stay empty (never invent IM)"
        );
        assert!(policy.on_complete);
        assert!(!policy.on_failure);
        assert!(!policy.pending_confirmation);
    }

    #[test]
    fn failure_notify_maps_on_failure_and_keeps_channels() {
        let notify = Some(AgentNotify {
            channels: vec!["ops".into()],
            on: NotifyOn::Failure,
        });
        let policy = map_notify_policy(&notify);
        assert_eq!(policy.channels, vec!["ops".to_string()]);
        assert!(policy.on_failure);
        assert!(!policy.on_complete);
        assert!(!policy.pending_confirmation);
    }

    #[test]
    fn unrouted_agent_keeps_failure_floor_without_inventing_im() {
        let policy = map_notify_policy(&None);
        assert!(policy.channels.is_empty());
        assert!(policy.on_failure);
        assert!(!policy.on_complete);
        assert!(!policy.pending_confirmation);
    }

    #[test]
    fn judgment_without_clear_dest_sets_pending_confirmation() {
        let notify = Some(AgentNotify {
            channels: vec![],
            on: NotifyOn::Judgment,
        });
        let policy = map_notify_policy(&notify);
        assert!(
            policy.pending_confirmation,
            "Judgment with no channels must park for operator confirmation"
        );
        assert!(
            policy.channels.is_empty(),
            "must not auto-route Judgment to IM"
        );
        assert!(!policy.on_complete);
        assert!(!policy.on_alert);
        assert!(!policy.on_failure);
    }

    #[test]
    fn judgment_with_channels_is_not_pending() {
        let notify = Some(AgentNotify {
            channels: vec!["ops".into()],
            on: NotifyOn::Judgment,
        });
        let policy = map_notify_policy(&notify);
        assert!(!policy.pending_confirmation);
        assert_eq!(policy.channels, vec!["ops".to_string()]);
    }

    #[test]
    fn migrate_creates_five_tables_and_is_idempotent() {
        let db = Database::builder()
            .create_with_backend(redb::backends::InMemoryBackend::new())
            .unwrap();

        // Seed a v1 agents row (mimic AgentStore layout).
        {
            let write = db.begin_write().unwrap();
            {
                let mut agents = write.open_table(AGENTS_V1_TABLE).unwrap();
                let mut execs = write.open_table(AGENT_EXECUTIONS_TABLE).unwrap();
                let mut mem = write.open_table(AGENT_MEMORY_TABLE).unwrap();
                let mut agent = fixture_agent();
                agent.execution_mode = ExecutionMode::Structured;
                agent.tool_config = Some(AgentToolConfig {
                    enabled: true,
                    allowed_tools: vec!["shell".into()],
                });
                agent.notify = Some(AgentNotify {
                    channels: vec![],
                    on: NotifyOn::Judgment,
                });
                agent.user_messages = vec![UserMessage {
                    id: "m1".into(),
                    timestamp: 10,
                    content: "hi".into(),
                    message_type: None,
                }];
                let bytes = serde_json::to_vec(&agent).unwrap();
                agents.insert(agent.id.as_str(), bytes.as_slice()).unwrap();
                // Touch legacy sibling tables so they exist like a real store.
                let _ = &mut execs;
                let _ = &mut mem;
            }
            write.commit().unwrap();
        }

        migrate_agents_v1_to_v2(&db).unwrap();

        {
            let read = db.begin_read().unwrap();
            assert!(read.open_table(AGENT_SPECS_TABLE).is_ok());
            assert!(read.open_table(AGENT_RUNTIME_TABLE).is_ok());
            assert!(read.open_table(AGENT_EXECUTIONS_TABLE).is_ok());
            assert!(read.open_table(AGENT_MEMORY_TABLE).is_ok());
            assert!(read.open_table(AGENT_USER_MESSAGES_TABLE).is_ok());
            assert!(
                read.open_table(AGENTS_V1_TABLE).is_err(),
                "legacy agents table must be removed"
            );

            let specs = read.open_table(AGENT_SPECS_TABLE).unwrap();
            let bytes = specs.get("a1").unwrap().unwrap().value().to_vec();
            let spec: AgentSpecRecord = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(spec.tool_policy, ToolPolicy::Disabled);
            assert!(spec.notify.pending_confirmation);
            assert!(spec.notify.channels.is_empty());

            let msgs = read.open_table(AGENT_USER_MESSAGES_TABLE).unwrap();
            let msg_bytes = msgs.get("a1").unwrap().unwrap().value().to_vec();
            let messages: Vec<UserMessage> = serde_json::from_slice(&msg_bytes).unwrap();
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].content, "hi");
        }

        // Schema stamp must remain untouched by this helper (M1 keeps version 1).
        assert_eq!(crate::schema::CURRENT_SCHEMA_VERSION, 1);

        // Idempotent second pass.
        migrate_agents_v1_to_v2(&db).unwrap();
        let read = db.begin_read().unwrap();
        let specs = read.open_table(AGENT_SPECS_TABLE).unwrap();
        assert!(specs.get("a1").unwrap().is_some());
    }
}
