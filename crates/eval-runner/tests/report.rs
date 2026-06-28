use eval_runner::report::aggregate;

#[test]
fn aggregates_three_valid_lines() {
    let jsonl = r#"
{"case_id":"a","lang":"zh","scores":{"tool_accuracy":10,"task_completion":8},"overall_reasoning":"x","judge":"claude"}
{"case_id":"b","lang":"zh","scores":{"tool_accuracy":6,"task_completion":5},"overall_reasoning":"y","judge":"claude"}
{"case_id":"c","lang":"en","scores":{"tool_accuracy":9},"overall_reasoning":"z","judge":"claude"}
"#;
    let agg = aggregate(jsonl).unwrap();
    assert_eq!(agg.total_cases, 3);
    assert_eq!(agg.malformed, 0);
    // tool_accuracy avg = (10+6+9)/3 = 8.333
    let dim_avg = agg
        .dimension_averages()
        .get("tool_accuracy")
        .copied()
        .unwrap_or(0.0);
    assert!((dim_avg - 8.333).abs() < 0.01);
}

#[test]
fn quarantines_malformed_lines() {
    let jsonl =
        "{\"case_id\":\"a\",BROKEN}\n{\"case_id\":\"b\",\"lang\":\"zh\",\"scores\":{\"tool_accuracy\":1},\"judge\":\"c\"}\n";
    let agg = aggregate(jsonl).unwrap();
    assert_eq!(agg.total_cases, 1);
    assert_eq!(agg.malformed, 1);
    assert!(agg.malformed_rate() > 0.0);
}

#[test]
fn detects_agent_error_status() {
    let jsonl = "{\"case_id\":\"e\",\"lang\":\"zh\",\"scores\":{},\"judge\":\"c\",\"status\":\"runtime_error\"}\n";
    let agg = aggregate(jsonl).unwrap();
    assert_eq!(agg.agent_errors, 1);
}
