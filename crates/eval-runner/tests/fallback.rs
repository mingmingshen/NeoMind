use eval_runner::fallback::detect_suspected_fallback;
use eval_runner::record::TurnRecord;
use serde_json::json;

fn empty_fast_turn() -> TurnRecord {
    TurnRecord {
        user: "thanks".into(),
        assistant_message: "you're welcome".into(),
        tool_calls: vec![],
        processing_time_ms: 100,
    }
}

#[test]
fn not_fallback_when_no_tool_word_in_expectations() {
    let turns = vec![empty_fast_turn()];
    let expectations = vec!["just say hi".to_string()];
    assert!(!detect_suspected_fallback(&turns, &expectations));
}

#[test]
fn not_fallback_when_any_turn_has_tools() {
    let mut t = empty_fast_turn();
    t.tool_calls.push(json!({"name":"shell","arguments":{}}));
    let expectations = vec!["Agent should call device create".to_string()];
    assert!(!detect_suspected_fallback(&[t], &expectations));
}

#[test]
fn is_fallback_when_all_turns_empty_fast_and_expectations_mention_tool() {
    let turns = vec![empty_fast_turn(), empty_fast_turn()];
    let expectations = vec![
        "Agent 应该调用 device create".to_string(),
        "invoke rule".to_string(),
    ];
    assert!(detect_suspected_fallback(&turns, &expectations));
}

#[test]
fn not_fallback_when_turn_is_slow() {
    let mut t = empty_fast_turn();
    t.processing_time_ms = 2000; // > 500ms
    let expectations = vec!["Agent should call device create".to_string()];
    assert!(!detect_suspected_fallback(&[t], &expectations));
}
