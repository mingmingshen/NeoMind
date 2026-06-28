use eval_runner::validate::validate_case;
use serde_json::json;

fn case_json(id: &str, scenario: &str, applies: &[&str], per_turn_n: usize) -> serde_json::Value {
    let per_turn: Vec<String> = (0..per_turn_n)
        .map(|i| format!("Turn {}: call device create", i))
        .collect();
    json!({
        "id": id, "lang": "zh", "category": "device", "workflow": "w",
        "scenario_type": scenario, "description": "x",
        "setup": {"fixture": "seed-empty"},
        "turns": (0..per_turn_n).map(|_| json!({"user": "x"})).collect::<Vec<_>>(),
        "applies": applies,
        "expectations": {"per_turn": per_turn, "overall": "x"}
    })
}

#[test]
fn rejects_lang_prefixed_id() {
    let v = case_json("zh-device-001", "single_turn", &["tool_accuracy"], 1);
    let res = validate_case(&v).unwrap();
    assert!(res.errors.iter().any(|e| e.contains("lang prefix")));
}

#[test]
fn rejects_context_retention_on_single_turn() {
    let v = case_json("device-001", "single_turn", &["context_retention"], 1);
    let res = validate_case(&v).unwrap();
    assert!(res.errors.iter().any(|e| e.contains("context_retention")));
}

#[test]
fn rejects_per_turn_length_mismatch() {
    let v = case_json("device-001", "multi_turn", &["tool_accuracy"], 3);
    let mut v = v;
    v["turns"] = json!([{"user":"a"},{"user":"b"}]);
    let res = validate_case(&v).unwrap();
    assert!(res.errors.iter().any(|e| e.contains("per_turn length")));
}

#[test]
fn accepts_valid_case() {
    let v = case_json(
        "device-001",
        "single_turn",
        &["tool_accuracy", "task_completion"],
        1,
    );
    let res = validate_case(&v).unwrap();
    assert!(res.errors.is_empty(), "{:?}", res.errors);
}

#[test]
fn rejects_unknown_state_query_type() {
    let mut v = case_json("device-001", "single_turn", &["tool_accuracy"], 1);
    v["state_queries"] = json!([{"type": "fake_query", "params": {}, "expected": true}]);
    let res = validate_case(&v).unwrap();
    assert!(res.errors.iter().any(|e| e.contains("not supported")));
}
