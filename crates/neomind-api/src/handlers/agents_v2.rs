//! M2 adapter surface for the clean-slate agent DTO.
//!
//! Routes under `/api/agents/v2/*` sit beside the legacy handlers so Web/CLI
//! can adopt EffectivePlan preview before full schema-v2 cutover. Schema
//! version stays 1 until AgentStore serves the five v2 tables.

use axum::Json;
use neomind_ai_agent::model::{AgentSpec, AgentSpecDraft, EffectivePlan, RunResponse};
use neomind_ai_agent::service::plan_from_spec;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::common::{ok, HandlerResult};
use crate::models::ErrorResponse;

/// Envelope returned by create/update/get once M2 cuts over.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentV2Response {
    pub agent: AgentSpec,
    pub effective_plan: EffectivePlan,
}

/// Preview request — static plan only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlanPreviewRequest {
    pub draft: AgentSpecDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlanPreviewResponse {
    pub effective_plan: EffectivePlan,
    pub dry_run: bool,
}

/// Build a transient AgentSpec (config_version=0) for static preview without save.
pub fn preview_draft(draft: AgentSpecDraft) -> Result<AgentPlanPreviewResponse, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let spec = AgentSpec {
        id: "preview".into(),
        name: draft.name,
        description: draft.description,
        user_prompt: draft.user_prompt,
        llm_backend_id: draft.llm_backend_id,
        preset: draft.preset,
        trigger: draft.trigger,
        pacing: draft.pacing,
        input: draft.input,
        tool_policy: draft.tool_policy,
        output: draft.output,
        notify: draft.notify,
        controlled_actions: draft.controlled_actions,
        config_version: 0,
        created_at: now,
        updated_at: now,
    };
    let plan = plan_from_spec(&spec);
    if plan.external_read_counts.total() != 0 {
        return Err("static preview must not perform external reads".into());
    }
    Ok(AgentPlanPreviewResponse {
        effective_plan: plan,
        dry_run: false,
    })
}

/// POST /api/agents/v2/plan-preview — static EffectivePlan, zero external reads.
pub async fn plan_preview(
    Json(req): Json<AgentPlanPreviewRequest>,
) -> HandlerResult<AgentPlanPreviewResponse> {
    match preview_draft(req.draft) {
        Ok(resp) => ok(resp),
        Err(e) => Err(ErrorResponse::bad_request(e)),
    }
}

/// Schema descriptor for the clean-slate DTO (editor / CLI discovery).
pub async fn agent_v2_schema() -> HandlerResult<Value> {
    ok(serde_json::json!({
        "schema": "agent-contract/v2",
        "schema_version_runtime": 1,
        "schema_version_target": 2,
        "samples": ["daily-brief", "partial-meter", "unknown-delivery", "legacy-history"],
        "operations": {
            "plan_preview": "POST /api/agents/v2/plan-preview",
            "note": "Formal create/dry-run/invoke cutover lands with schema v2 activation"
        }
    }))
}

/// Placeholder type export so OpenAPI / future handlers share RunResponse.
#[allow(dead_code)]
type _RunResponseExport = RunResponse;

#[cfg(test)]
mod tests {
    use super::*;
    use neomind_ai_agent::model::{daily_brief_draft, ToolPolicy};
    use std::path::PathBuf;

    #[test]
    fn daily_brief_fixture_roundtrip_keys() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/agent-contract/v2/daily-brief.json");
        let raw = std::fs::read_to_string(&path).expect("fixture");
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["sample"], "daily-brief");
        let draft = daily_brief_draft("weather_cn", "tariff_tou", "test-channel");
        let preview = preview_draft(draft).unwrap();
        assert_eq!(preview.effective_plan.external_read_counts.weather, 0);
        assert_eq!(preview.effective_plan.external_read_counts.tariff, 0);
        assert!(matches!(
            preview.effective_plan.tool_policy,
            ToolPolicy::Disabled
        ));
        assert_eq!(
            preview.effective_plan.timezone.as_deref(),
            Some("Asia/Shanghai")
        );
        let enc = serde_json::to_value(&preview).unwrap();
        assert_eq!(enc["dry_run"], false);
        assert!(
            enc["effective_plan"]["authorized_reads"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
    }
}
