//! validate-case schema checks (spec §13 step 7).
use serde::{Deserialize, Serialize};

const ALLOWED_DIMS: &[&str] = &[
    "tool_accuracy",
    "task_completion",
    "response_quality",
    "context_retention",
    "error_recovery",
    "language_adherence",
];

const KNOWN_QUERY_TYPES: &[&str] = &[
    "device_exists",
    "device_count",
    "rule_exists",
    "rule_enabled",
    "agent_exists",
    "agent_status",
    "transform_exists",
    "dashboard_exists",
    "dashboard_component_count",
    "channel_exists",
    "message_count",
    "push_enabled",
];

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn validate_case(case: &serde_json::Value) -> anyhow::Result<ValidationReport> {
    let mut errors = Vec::new();
    let warnings = Vec::new();

    // Rule 1: ID must not have zh-/en- prefix.
    let id = case.get("id").and_then(|v| v.as_str()).unwrap_or("");
    if id.starts_with("zh-") || id.starts_with("en-") {
        errors.push(format!(
            "id '{}' has lang prefix — IDs are language-agnostic",
            id
        ));
    }

    // Rule 2: applies[] dims must all be in ALLOWED_DIMS.
    let applies: Vec<String> = case
        .get("applies")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    for d in &applies {
        if !ALLOWED_DIMS.contains(&d.as_str()) {
            errors.push(format!("applies '{}' is not a known dimension", d));
        }
    }

    // Rule 3: single_turn cases must NOT include context_retention.
    let scenario = case
        .get("scenario_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if scenario == "single_turn" && applies.contains(&"context_retention".to_string()) {
        errors.push("context_retention not applicable to single_turn".into());
    }

    // Rule 4: per_turn length must equal turns length.
    let turns_n = case
        .get("turns")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let per_turn_n = case
        .get("expectations")
        .and_then(|v| v.get("per_turn"))
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    if turns_n != per_turn_n {
        errors.push(format!(
            "per_turn length {} != turns length {}",
            per_turn_n, turns_n
        ));
    }

    // Rule 5: state_queries types must be known.
    if let Some(sq) = case.get("state_queries").and_then(|v| v.as_array()) {
        for q in sq {
            let t = q.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if !KNOWN_QUERY_TYPES.contains(&t) {
                errors.push(format!("state_query type '{}' not supported", t));
            }
        }
    }

    Ok(ValidationReport { errors, warnings })
}
