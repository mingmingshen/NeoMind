//! M2 adapter surface for the clean-slate agent DTO.
//!
//! Wired behind feature/env later; M1 uses the isolated test service. This
//! module provides serialization helpers and fixture round-trip tests so Web
//! and CLI can share `tests/fixtures/agent-contract/v2/` before cutover.

use neomind_ai_agent::model::{AgentSpec, AgentSpecDraft, EffectivePlan};
use neomind_ai_agent::service::plan_from_spec;
use serde::{Deserialize, Serialize};

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
        // Serialize plan for Web consumers
        let enc = serde_json::to_value(&preview).unwrap();
        assert_eq!(enc["dry_run"], false);
        assert!(enc["effective_plan"]["authorized_reads"].as_array().unwrap().len() >= 2);
    }
}
