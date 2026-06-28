use eval_runner::state_query::sid;
use serde_json::json;

#[test]
fn sid_extracts_string_param() {
    let v = json!({"id": "x"});
    assert_eq!(sid(&v, "id").unwrap(), "x");
    assert!(sid(&v, "missing").is_err());
}

#[test]
fn sid_handles_non_object() {
    let v = json!(42);
    assert!(sid(&v, "id").is_err());
}
