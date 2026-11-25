//! Zero-tool single-pass runner (daily brief, partial diagnosis, structured).

use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::{build_evidence_prompt, AgentRunner, RunOutcome};
use crate::error::Result;
use crate::model::{AgentSpec, EvidenceDetail, EvidenceQuality, EvidenceReport, ToolPolicy};
use crate::ports::{LlmPort, LlmRequest};

pub struct SinglePassRunner {
    llm: Arc<dyn LlmPort>,
}

impl SinglePassRunner {
    pub fn new(llm: Arc<dyn LlmPort>) -> Self {
        Self { llm }
    }
}

#[async_trait]
impl AgentRunner for SinglePassRunner {
    async fn run(
        &self,
        spec: &AgentSpec,
        evidence: &EvidenceReport,
        _execution_id: &str,
        cancel: CancellationToken,
    ) -> Result<RunOutcome> {
        // Tools must stay off for Disabled policy.
        let tools_enabled = !matches!(spec.tool_policy, ToolPolicy::Disabled);

        // Deterministic brief when tools disabled: compose from fresh evidence.
        if matches!(spec.tool_policy, ToolPolicy::Disabled) {
            let brief = compose_local_brief(spec, evidence);
            let alert = evidence.items.iter().any(|i| {
                matches!(i.quality, EvidenceQuality::Fresh)
                    && i.detail.as_ref().is_some_and(|d| match d {
                        EvidenceDetail::Structured { value } => value_shows_overload(value),
                        EvidenceDetail::TimeSeries { value, .. } => {
                            value.as_ref().is_some_and(value_shows_overload)
                        }
                        EvidenceDetail::Text { text } => {
                            text.to_ascii_uppercase().contains("OVERLOAD")
                        }
                        _ => false,
                    })
            });
            return Ok(RunOutcome {
                brief_text: Some(brief.clone()),
                conclusion: Some(brief),
                alert_criteria_met: alert,
                status_hint: None,
            });
        }

        let user = build_evidence_prompt(&spec.user_prompt, evidence);
        let resp = self
            .llm
            .complete(
                LlmRequest {
                    backend_id: spec.llm_backend_id.clone(),
                    system:
                        "You are a managed NeoMind agent. Cite evidence keys. Mark missing deps."
                            .into(),
                    user,
                    tools_enabled,
                },
                cancel,
            )
            .await?;
        Ok(RunOutcome {
            brief_text: Some(resp.text.clone()),
            conclusion: Some(resp.text),
            alert_criteria_met: resp.alert_criteria_met,
            status_hint: None,
        })
    }
}

fn value_shows_overload(value: &serde_json::Value) -> bool {
    value
        .get("overload")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || value.to_string().to_ascii_uppercase().contains("OVERLOAD")
}

fn compose_local_brief(spec: &AgentSpec, evidence: &EvidenceReport) -> String {
    let mut lines = vec![format!("# {}", spec.name)];
    for item in &evidence.items {
        let label = &item.key;
        match (&item.quality, &item.detail) {
            (EvidenceQuality::Fresh, Some(EvidenceDetail::Structured { value })) => {
                lines.push(format!("{label}: {value} (source={})", item.source_id));
                if let Some((a, b)) = item.valid_window {
                    lines.push(format!("  valid_window=[{a},{b}]"));
                }
            }
            (EvidenceQuality::Fresh, Some(EvidenceDetail::Text { text })) => {
                lines.push(format!("{label}: {text} (source={})", item.source_id));
            }
            (EvidenceQuality::Fresh, Some(EvidenceDetail::TimeSeries { points, value })) => {
                let extra = value.as_ref().map(|v| format!(" {v}")).unwrap_or_default();
                lines.push(format!(
                    "{label}: {points} samples{extra} (source={})",
                    item.source_id
                ));
            }
            (q, _) => {
                lines.push(format!("{label}: {:?} — 依据不足", q));
            }
        }
    }
    lines.join("\n")
}
