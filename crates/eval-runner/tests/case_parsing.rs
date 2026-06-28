use eval_runner::case::{Case, ScenarioType};

#[test]
fn parse_minimal_single_turn_case() {
    let json = r#"{
        "id": "device-create-001",
        "lang": "zh",
        "category": "device",
        "workflow": "device-onboarding",
        "scenario_type": "single_turn",
        "description": "通过对话创建温度传感器",
        "setup": { "fixture": "seed-default" },
        "turns": [ { "user": "帮我添加 temp-001" } ],
        "applies": ["tool_accuracy","task_completion","response_quality","language_adherence"],
        "expectations": {
            "per_turn": ["Turn 1: Agent 应该调用 device create"],
            "overall": "无探索性调用"
        }
    }"#;
    let case: Case = serde_json::from_str(json).unwrap();
    assert_eq!(case.id, "device-create-001");
    assert_eq!(case.lang.as_str(), "zh");
    assert!(matches!(case.scenario_type, ScenarioType::SingleTurn));
    assert_eq!(case.turns.len(), 1);
    assert_eq!(case.expectations.per_turn.len(), 1);
    assert!(case.state_queries.is_none());
}

#[test]
fn lang_prefixed_id_parses_but_validator_catches() {
    // ID format rule lives in validate.rs, not parse-time.
    let json = r#"{
        "id": "zh-device-create-001", "lang": "zh", "category": "device",
        "workflow": "device-onboarding", "scenario_type": "single_turn",
        "description": "x", "setup": { "fixture": "seed-default" },
        "turns": [{"user":"hi"}], "applies": ["tool_accuracy"],
        "expectations": {"per_turn":["x"], "overall":"x"}
    }"#;
    let case: Result<Case, _> = serde_json::from_str(json);
    assert!(case.is_ok());
}

#[test]
fn per_turn_length_mismatch_parses_but_validator_catches() {
    let json = r#"{
        "id": "x", "lang": "en", "category": "device", "workflow": "w",
        "scenario_type": "multi_turn", "description": "x",
        "setup": {"fixture":"seed-empty"},
        "turns": [{"user":"a"},{"user":"b"}],
        "applies": ["tool_accuracy"],
        "expectations": {"per_turn":["only one"], "overall":"x"}
    }"#;
    let case: Case = serde_json::from_str(json).unwrap();
    assert_eq!(case.turns.len(), 2);
    assert_eq!(case.expectations.per_turn.len(), 1);
}
