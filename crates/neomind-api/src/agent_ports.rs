//! Production host adapters for [`neomind_ai_agent`] ports.
//!
//! Wired from [`crate::ai_agent_runtime::HostedAiAgentRuntime::build`] so the
//! clean-slate kernel reads real telemetry/extensions, calls the instance
//! manager LLM, and delivers via MessageManager / DeviceService.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use neomind_ai_agent::error::{AgentError, Result as AgentResult};
use neomind_ai_agent::ports::{
    LlmBackendCapabilities, LlmPort, LlmRequest, LlmResponse, LocalBriefLlm, NotifyPort,
    SideEffectPort, SourceReadPort, SourceReadResult,
};
use neomind_ai_agent::service::{NotifyCounter, WriteCounter};
use neomind_core::extension::ExtensionRegistry;
use neomind_core::llm::{GenerationParams, LlmInput};
use neomind_core::Message;
use neomind_devices::{DeviceService, TimeSeriesStorage};
use neomind_messages::{Message as NotifyMessage, MessageManager, MessageSeverity};
use tokio_util::sync::CancellationToken;

use crate::server::state::ExtensionMetricsStorage;

/// Host services injected into the clean-slate runtime builder.
#[derive(Clone)]
pub struct HostAgentDeps {
    pub message_manager: Arc<MessageManager>,
    pub telemetry: Arc<TimeSeriesStorage>,
    pub device_service: Arc<DeviceService>,
    pub extension_registry: Arc<ExtensionRegistry>,
    pub extension_metrics: Arc<ExtensionMetricsStorage>,
}

impl HostAgentDeps {
    pub fn new(
        message_manager: Arc<MessageManager>,
        telemetry: Arc<TimeSeriesStorage>,
        device_service: Arc<DeviceService>,
        extension_registry: Arc<ExtensionRegistry>,
        extension_metrics: Arc<ExtensionMetricsStorage>,
    ) -> Self {
        Self {
            message_manager,
            telemetry,
            device_service,
            extension_registry,
            extension_metrics,
        }
    }
}

// ── LLM ─────────────────────────────────────────────────────────────────────

pub struct HostLlmPort {
    pub request_count: AtomicU64,
}

impl HostLlmPort {
    pub fn new() -> Self {
        Self {
            request_count: AtomicU64::new(0),
        }
    }

    pub fn requests(&self) -> u64 {
        self.request_count.load(Ordering::SeqCst)
    }
}

impl Default for HostLlmPort {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmPort for HostLlmPort {
    async fn resolve_backend(
        &self,
        backend_id: Option<&str>,
    ) -> AgentResult<LlmBackendCapabilities> {
        match self.try_resolve(backend_id) {
            Ok(caps) => Ok(caps),
            Err(e) => {
                tracing::debug!(error = %e, "host LLM resolve falling back to LocalBriefLlm");
                LocalBriefLlm.resolve_backend(backend_id).await
            }
        }
    }

    async fn complete(
        &self,
        request: LlmRequest,
        cancel: CancellationToken,
    ) -> AgentResult<LlmResponse> {
        self.request_count.fetch_add(1, Ordering::SeqCst);
        match self.try_complete(&request, cancel.child_token()).await {
            Ok(resp) => Ok(resp),
            Err(e) => {
                // Tests and edge hosts without an active backend still need
                // scheduled/event agents to settle (stats + notify). Fall back
                // to the deterministic local brief used by ToolPolicy::Disabled.
                tracing::warn!(
                    error = %e,
                    "host LLM unavailable — falling back to LocalBriefLlm"
                );
                LocalBriefLlm.complete(request, cancel).await
            }
        }
    }
}

impl HostLlmPort {
    fn try_resolve(&self, backend_id: Option<&str>) -> AgentResult<LlmBackendCapabilities> {
        let manager =
            neomind_agent::get_instance_manager().map_err(|e| AgentError::Llm(e.to_string()))?;
        let instance = match backend_id {
            Some(id) if id != "default" => manager
                .get_instance(id)
                .ok_or_else(|| AgentError::Llm(format!("backend '{id}' not found")))?,
            _ => manager
                .get_active_instance()
                .ok_or_else(|| AgentError::Llm("no active LLM backend".into()))?,
        };
        Ok(LlmBackendCapabilities {
            multimodal: instance.capabilities.supports_multimodal,
            tools: instance.capabilities.supports_tools,
            model: instance.model.clone(),
        })
    }

    async fn try_complete(
        &self,
        request: &LlmRequest,
        cancel: CancellationToken,
    ) -> AgentResult<LlmResponse> {
        let manager =
            neomind_agent::get_instance_manager().map_err(|e| AgentError::Llm(e.to_string()))?;
        let runtime = match request.backend_id.as_deref() {
            Some(id) if id != "default" => manager
                .get_runtime(id)
                .await
                .map_err(|e| AgentError::Llm(e.to_string()))?,
            _ => manager
                .get_active_runtime()
                .await
                .map_err(|e| AgentError::Llm(e.to_string()))?,
        };

        let mut params = GenerationParams::default();
        // Non-chat managed runs: disable thinking to avoid wasted tokens.
        params.thinking_enabled = Some(false);
        params.max_tokens = Some(2048);
        let input = LlmInput {
            messages: vec![
                Message::system(request.system.clone()),
                Message::user(request.user.clone()),
            ],
            params,
            model: None,
            stream: false,
            tools: None,
        };

        let output = tokio::select! {
            _ = cancel.cancelled() => return Err(AgentError::Cancelled),
            result = runtime.generate(input) => {
                result.map_err(|e| AgentError::Llm(e.to_string()))?
            }
        };

        let alert = output.text.contains("Verdict: criteria met")
            || output.text.to_ascii_uppercase().contains("OVERLOAD");
        Ok(LlmResponse {
            text: output.text,
            alert_criteria_met: alert,
        })
    }
}

// ── Source reads ────────────────────────────────────────────────────────────

pub struct HostSourceReadPort {
    deps: HostAgentDeps,
    pub read_count: AtomicU64,
}

impl HostSourceReadPort {
    pub fn new(deps: HostAgentDeps) -> Self {
        Self {
            deps,
            read_count: AtomicU64::new(0),
        }
    }

    pub fn reads(&self) -> u64 {
        self.read_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl SourceReadPort for HostSourceReadPort {
    async fn read_extension(
        &self,
        extension_id: &str,
        operation: &str,
        params: &serde_json::Value,
        timeout_secs: u32,
    ) -> AgentResult<SourceReadResult> {
        self.read_count.fetch_add(1, Ordering::SeqCst);
        let source_id = format!("extension:{extension_id}");
        let timeout = Duration::from_secs(timeout_secs.max(1) as u64);

        // Prefer durable metric storage when the operation names a metric field.
        if let Ok(Some(point)) = self
            .deps
            .extension_metrics
            .query_latest(&source_id, operation)
            .await
        {
            return Ok(SourceReadResult {
                source_id: source_id.clone(),
                binding_proof: format!("{source_id}:{operation}"),
                observed_at: Some(point.timestamp),
                valid_window: None,
                value: serde_json::to_value(&point).unwrap_or_else(|_| {
                    serde_json::json!({
                        "timestamp": point.timestamp,
                    })
                }),
            });
        }

        // Fall back to live registry metrics, then to execute_command.
        let metrics = tokio::time::timeout(
            timeout,
            self.deps
                .extension_registry
                .get_current_metrics(extension_id),
        )
        .await
        .map_err(|_| AgentError::Source {
            key: extension_id.into(),
            reason: "extension metrics timed out".into(),
        })?;
        if let Some(m) = metrics.iter().find(|m| m.name == operation) {
            return Ok(SourceReadResult {
                source_id: source_id.clone(),
                binding_proof: format!("{source_id}:{operation}:live"),
                observed_at: Some(m.timestamp),
                valid_window: None,
                value: serde_json::to_value(m).unwrap_or(serde_json::Value::Null),
            });
        }

        let result = tokio::time::timeout(
            timeout,
            self.deps
                .extension_registry
                .execute_command(extension_id, operation, params),
        )
        .await
        .map_err(|_| AgentError::Source {
            key: extension_id.into(),
            reason: "extension command timed out".into(),
        })?
        .map_err(|e| AgentError::Source {
            key: extension_id.into(),
            reason: e.to_string(),
        })?;

        Ok(SourceReadResult {
            source_id,
            binding_proof: format!("extension:{extension_id}:{operation}:cmd"),
            observed_at: Some(chrono::Utc::now().timestamp_millis()),
            valid_window: None,
            value: result,
        })
    }

    async fn read_device_metric(
        &self,
        device_id: &str,
        field: &str,
        _params: &serde_json::Value,
        timeout_secs: u32,
    ) -> AgentResult<SourceReadResult> {
        self.read_count.fetch_add(1, Ordering::SeqCst);
        let source_key = format!("device:{device_id}");
        let timeout = Duration::from_secs(timeout_secs.max(1) as u64);

        if field == "*" {
            let metrics =
                tokio::time::timeout(timeout, self.deps.telemetry.list_metrics(&source_key))
                    .await
                    .map_err(|_| AgentError::Source {
                        key: device_id.into(),
                        reason: "list_metrics timed out".into(),
                    })?
                    .map_err(|e| AgentError::Source {
                        key: device_id.into(),
                        reason: e.to_string(),
                    })?;
            let metric_refs: Vec<&str> = metrics.iter().map(|s| s.as_str()).collect();
            let batch = tokio::time::timeout(
                timeout,
                self.deps.telemetry.latest_batch(&source_key, &metric_refs),
            )
            .await
            .map_err(|_| AgentError::Source {
                key: device_id.into(),
                reason: "latest_batch timed out".into(),
            })?
            .map_err(|e| AgentError::Source {
                key: device_id.into(),
                reason: e.to_string(),
            })?;
            let observed = batch.values().map(|p| p.timestamp).max();
            return Ok(SourceReadResult {
                source_id: source_key.clone(),
                binding_proof: format!("{source_key}:*"),
                observed_at: observed,
                valid_window: None,
                value: serde_json::to_value(&batch).unwrap_or(serde_json::json!({})),
            });
        }

        let point = tokio::time::timeout(timeout, self.deps.telemetry.latest(&source_key, field))
            .await
            .map_err(|_| AgentError::Source {
                key: device_id.into(),
                reason: "latest timed out".into(),
            })?
            .map_err(|e| AgentError::Source {
                key: device_id.into(),
                reason: e.to_string(),
            })?
            .ok_or_else(|| AgentError::Source {
                key: device_id.into(),
                reason: format!("no data for {source_key}:{field}"),
            })?;

        Ok(SourceReadResult {
            source_id: source_key.clone(),
            binding_proof: format!("{source_key}:{field}"),
            observed_at: Some(point.timestamp),
            valid_window: None,
            value: serde_json::to_value(&point).unwrap_or(serde_json::Value::Null),
        })
    }
}

// ── Notify ──────────────────────────────────────────────────────────────────

pub struct HostNotifyPort {
    message_manager: Arc<MessageManager>,
    pub deliver_count: AtomicU64,
}

impl HostNotifyPort {
    pub fn new(message_manager: Arc<MessageManager>) -> Self {
        Self {
            message_manager,
            deliver_count: AtomicU64::new(0),
        }
    }

    pub fn deliveries(&self) -> u64 {
        self.deliver_count.load(Ordering::SeqCst)
    }
}

impl NotifyCounter for HostNotifyPort {
    fn count(&self) -> u64 {
        self.deliveries()
    }
}

#[async_trait]
impl NotifyPort for HostNotifyPort {
    async fn deliver(
        &self,
        channel: &str,
        subject: &str,
        body: &str,
        execution_id: &str,
    ) -> AgentResult<()> {
        self.deliver_count.fetch_add(1, Ordering::SeqCst);
        let mut msg = NotifyMessage::new(
            "agent",
            MessageSeverity::Info,
            subject.to_string(),
            body.to_string(),
            format!("agent-exec:{execution_id}"),
        );
        msg.source_type = "agent".into();
        msg.target_channels = Some(vec![channel.to_string()]);
        msg.metadata = Some(serde_json::json!({ "execution_id": execution_id }));
        self.message_manager
            .create_message(msg)
            .await
            .map_err(|e| AgentError::Delivery(e.to_string()))?;
        Ok(())
    }
}

// ── Side effects ────────────────────────────────────────────────────────────

pub struct HostSideEffectPort {
    device_service: Arc<DeviceService>,
    extension_registry: Arc<ExtensionRegistry>,
    pub invoke_count: AtomicU64,
}

impl HostSideEffectPort {
    pub fn new(
        device_service: Arc<DeviceService>,
        extension_registry: Arc<ExtensionRegistry>,
    ) -> Self {
        Self {
            device_service,
            extension_registry,
            invoke_count: AtomicU64::new(0),
        }
    }

    pub fn invokes(&self) -> u64 {
        self.invoke_count.load(Ordering::SeqCst)
    }
}

impl WriteCounter for HostSideEffectPort {
    fn count(&self) -> u64 {
        self.invokes()
    }
}

#[async_trait]
impl SideEffectPort for HostSideEffectPort {
    async fn invoke(
        &self,
        destination: &str,
        action: &str,
        params: &serde_json::Value,
        _execution_id: &str,
    ) -> AgentResult<()> {
        self.invoke_count.fetch_add(1, Ordering::SeqCst);

        if let Some(ext_id) = destination.strip_prefix("extension:") {
            self.extension_registry
                .execute_command(ext_id, action, params)
                .await
                .map_err(|e| AgentError::Other(e.to_string()))?;
            return Ok(());
        }

        let device_id = destination.strip_prefix("device:").unwrap_or(destination);
        let map = json_to_string_map(params);

        // Prefer device when the id is registered; otherwise try extension.
        if self.device_service.get_device(device_id).is_some() {
            self.device_service
                .send_command_reported(device_id, action, map)
                .await
                .map_err(|e| AgentError::Other(e.to_string()))?;
            return Ok(());
        }

        if self
            .extension_registry
            .get_info(destination)
            .await
            .is_some()
        {
            self.extension_registry
                .execute_command(destination, action, params)
                .await
                .map_err(|e| AgentError::Other(e.to_string()))?;
            return Ok(());
        }

        self.device_service
            .send_command_reported(device_id, action, map)
            .await
            .map_err(|e| AgentError::Other(e.to_string()))?;
        Ok(())
    }
}

fn json_to_string_map(params: &serde_json::Value) -> HashMap<String, serde_json::Value> {
    match params.as_object() {
        Some(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        None => HashMap::new(),
    }
}
