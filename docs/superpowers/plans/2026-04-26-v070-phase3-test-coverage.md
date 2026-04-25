# v0.7.0 Phase 3: Test Coverage

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add comprehensive unit tests to 6 core crates, targeting 30-60% coverage depending on module complexity.

**Architecture:** Three parallel tracks — (A) agent + storage tests, (B) rules + messages tests, (C) extension-runner + API tests. Each track uses the crate's existing error types and test patterns.

**Tech Stack:** Rust, tokio::test, redb (in-memory), serde_json, mock patterns

**Spec:** `docs/superpowers/specs/2026-04-26-v0.7.0-release-plan-design.md` Part 3 (Section 3.3)

**Depends on:** Phase 1 (error handling changes affect test expectations)

---

## Testing Patterns Reference

### In-Memory Store (storage tests)
```rust
let store = TimeSeriesStore::memory().expect("Failed to create memory store");
```

### Temporary Session Store
```rust
fn create_temp_store() -> Arc<SessionStore> {
    let temp_dir = std::env::temp_dir().join(format!("test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.redb");
    Arc::new(SessionStore::open(&db_path).unwrap())
}
```

### DSL Parser Test
```rust
fn parse_rule(dsl: &str) -> ParsedRule {
    RuleDslParser::parse(dsl).expect("Failed to parse rule")
}
```

---

## Track A: neomind-agent + neomind-storage

### Task A1: Storage — TimeseriesStore Tests

**Files:**
- Modify: `crates/neomind-storage/src/timeseries.rs` (append test module)

**Target:** 60%+ coverage

- [ ] **Step 1: Test basic write and query_latest**

```rust
#[test]
fn test_write_and_query_latest() {
    let store = TimeSeriesStore::memory().unwrap();
    let dp = DataPoint::new_f64(chrono::Utc::now().timestamp_millis(), 23.5);

    store.write("source1", "temperature", dp.clone()).unwrap();
    let result = store.query_latest("source1", "temperature").unwrap();

    assert!(result.is_some());
    assert_eq!(result.unwrap().value, serde_json::json!(23.5));
}
```

- [ ] **Step 2: Test batch write**

Write 100 data points, verify all are queryable.

- [ ] **Step 3: Test query_range with time bounds**

Insert data points at known timestamps, query a subrange, verify only matching points returned.

- [ ] **Step 4: Test aggregated queries (avg/min/max/sum/count)**

```rust
#[test]
fn test_query_aggregated() {
    let store = TimeSeriesStore::memory().unwrap();
    // Insert 10 data points: values 1.0 through 10.0
    for i in 1..=10 {
        let dp = DataPoint::new_f64(i * 1000, i as f64);
        store.write("src", "metric", dp).unwrap();
    }

    let result = store.query_aggregated("src", "metric", 0, 11000, 10000).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].avg, Some(5.5)); // avg of 1-10
}
```

- [ ] **Step 5: Test delete operations**

Write data, delete by range, verify deleted data gone, remaining data intact.

- [ ] **Step 6: Test list_metrics**

Write to multiple metrics, list all, verify complete set returned.

- [ ] **Step 7: Test concurrent access**

Spawn 10 tasks writing to different sources simultaneously, verify no data corruption.

- [ ] **Step 8: Test edge cases**

- Empty source/metric queries return None
- Very large values (f64::MAX)
- Negative timestamps
- Unicode metric names
- Null values in DataPoint

- [ ] **Step 9: Run tests**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo test -p neomind-storage --lib -- timeseries 2>&1 | tail -20`

- [ ] **Step 10: Commit**

```bash
git add crates/neomind-storage/src/timeseries.rs
git commit -m "test(storage): add comprehensive timeseries unit tests"
```

---

### Task A2: Storage — SessionStore Tests

**Files:**
- Modify: `crates/neomind-storage/src/session.rs` (append test module)

**Target:** 60%+ coverage

- [ ] **Step 1: Test session lifecycle (create, exists, list, delete)**

- [ ] **Step 2: Test message history (save, load, append, clear, count)**

- [ ] **Step 3: Test metadata operations**

- [ ] **Step 4: Test concurrent session access**

- [ ] **Step 5: Run tests and commit**

```bash
git commit -m "test(storage): add session store unit tests"
```

---

### Task A3: Agent — Tool Parameter Mapping Tests

**Files:**
- Create or modify: `crates/neomind-agent/src/tools/mapper.rs` tests

**Target:** 50%+ for tool layer

- [ ] **Step 1: Test parameter extraction from JSON args**

```rust
#[test]
fn test_extract_string_param() {
    let args = serde_json::json!({"name": "test-device", "action": "list"});
    let name = extract_string_param(&args, "name").unwrap();
    assert_eq!(name, "test-device");
}

#[test]
fn test_missing_required_param_returns_error() {
    let args = serde_json::json!({});
    let result = extract_string_param(&args, "name");
    assert!(result.is_err());
}
```

- [ ] **Step 2: Test parameter type coercion**

Test conversion of string params to int, float, bool.

- [ ] **Step 3: Test parameter validation**

Test min/max bounds, regex patterns, enum values.

- [ ] **Step 4: Run tests and commit**

```bash
git commit -m "test(agent): add tool parameter mapping tests"
```

---

### Task A4: Agent — Tool Search Tests

**Files:**
- Create or modify: `crates/neomind-agent/src/tools/tool_search.rs` tests

- [ ] **Step 1: Test tool discovery by keyword**

```rust
#[test]
fn test_search_by_keyword() {
    let tool = ToolSearchTool::new(vec![
        ("device_list".into(), "List all devices".into()),
        ("device_control".into(), "Control device".into()),
        ("metric_read".into(), "Read metrics".into()),
    ]);
    // Test searching for "device" returns both device tools
}
```

- [ ] **Step 2: Test fuzzy matching**

- [ ] **Step 3: Test empty results**

- [ ] **Step 4: Run tests and commit**

```bash
git commit -m "test(agent): add tool search unit tests"
```

---

## Track B: neomind-rules + neomind-messages

### Task B1: Rules — DSL Parser Tests

**Files:**
- Modify: `crates/neomind-rules/src/dsl.rs` (append test module)

**Target:** 50%+ coverage

- [ ] **Step 1: Test simple condition parsing**

```rust
#[test]
fn test_simple_comparison() {
    let rule = parse_rule(r#"
        rule "High Temp"
        when temperature > 30
        do
            notify "Hot"
        end
    "#);
    assert_eq!(rule.name, "High Temp");
    // Verify condition is Device("temperature", GreaterThan, 30.0)
}
```

- [ ] **Step 2: Test compound conditions (AND/OR/NOT)**

```rust
#[test]
fn test_and_condition() {
    let rule = parse_rule(r#"
        rule "AND Rule"
        when temperature > 30 and humidity < 50
        do notify "Hot and dry" end
    "#);
    // Verify condition is And(Box<temp>, Box<humidity>)
}

#[test]
fn test_or_condition() {
    // Test OR conditions
}

#[test]
fn test_not_condition() {
    // Test NOT conditions
}
```

- [ ] **Step 3: Test all action types**

Test parsing of: notify, execute, log, http, alert, set, delay.

- [ ] **Step 4: Test duration parsing**

`for 5 minutes`, `for 30 seconds`, `for 1 hour`.

- [ ] **Step 5: Test error cases**

```rust
#[test]
fn test_malformed_dsl_returns_error() {
    let result = RuleDslParser::parse("invalid rule syntax");
    assert!(result.is_err());
}

#[test]
fn test_missing_end_keyword() {
    let result = RuleDslParser::parse(r#"
        rule "Test"
        when temperature > 30
        do
            notify "Alert"
    "#);
    assert!(result.is_err());
}
```

- [ ] **Step 6: Test range conditions**

`when temperature between 20 and 25`.

- [ ] **Step 7: Test complex nested conditions**

`when (temp1 > 30 or temp2 < 20) and humidity > 50`.

- [ ] **Step 8: Run tests**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo test -p neomind-rules --lib -- dsl 2>&1 | tail -20`

- [ ] **Step 9: Commit**

```bash
git add crates/neomind-rules/src/dsl.rs
git commit -m "test(rules): add comprehensive DSL parser unit tests"
```

---

### Task B2: Rules — Engine Evaluation Tests

**Files:**
- Modify: `crates/neomind-rules/src/engine.rs` (append test module)

- [ ] **Step 1: Test condition evaluation with mock values**

- [ ] **Step 2: Test action execution**

- [ ] **Step 3: Test duration tracking**

- [ ] **Step 4: Run tests and commit**

```bash
git commit -m "test(rules): add engine condition evaluation tests"
```

---

### Task B3: Messages — CRUD and Delivery Tests

**Files:**
- Modify: `crates/neomind-messages/src/manager.rs` tests
- Modify: `crates/neomind-messages/src/delivery_log.rs` tests

**Target:** 40%+ coverage

- [ ] **Step 1: Test message create/read/update/delete**

- [ ] **Step 2: Test delivery log tracking**

- [ ] **Step 3: Test message filtering (by category, severity, time range)**

- [ ] **Step 4: Test notification channel configuration**

- [ ] **Step 5: Run tests and commit**

```bash
git commit -m "test(messages): add message CRUD and delivery unit tests"
```

---

## Track C: neomind-extension-runner + neomind-api

### Task C1: Extension Runner — IPC Message Tests

**Files:**
- Modify: `crates/neomind-extension-runner/src/ipc_routing.rs` tests

**Target:** 40%+ coverage

- [ ] **Step 1: Test IPC message serialization roundtrip**

```rust
#[test]
fn test_ipc_message_roundtrip() {
    let msg = IpcMessage::Init { config: json!({"key": "value"}) };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: IpcMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, IpcMessage::Init { .. }));
}
```

- [ ] **Step 2: Test all message types**

Init, ExecuteCommand, ProduceMetrics, StreamData, CapabilityResult.

- [ ] **Step 3: Test message routing logic**

Verify InFlightRequests completes/cancels correctly.

- [ ] **Step 4: Test error handling in malformed messages**

Send invalid JSON, verify graceful error response.

- [ ] **Step 5: Run tests and commit**

```bash
git commit -m "test(extension-runner): add IPC message serialization and routing tests"
```

---

### Task C2: Extension Runner — Process Management Tests

**Files:**
- Modify: `crates/neomind-extension-runner/src/main.rs` tests

- [ ] **Step 1: Test extension type detection**

Verify `.wasm`, `.dylib`, `.so`, `.dll` extension detection.

- [ ] **Step 2: Test resource limits**

Verify resource limit configuration and enforcement.

- [ ] **Step 3: Test dylib validation**

Test magic number validation for dynamic libraries.

- [ ] **Step 4: Run tests and commit**

```bash
git commit -m "test(extension-runner): add process management and validation tests"
```

---

### Task C3: API — Handler Parameter Validation Tests

**Files:**
- Create or modify: `crates/neomind-api/src/handlers/validation.rs` tests

**Target:** 30%+ coverage

- [ ] **Step 1: Test Validator helper functions**

```rust
#[test]
fn test_required_string_empty_returns_error() {
    let result = Validator::required_string("", "name");
    assert!(result.is_err());
}

#[test]
fn test_required_string_valid_passes() {
    let result = Validator::required_string("test", "name");
    assert!(result.is_ok());
}

#[test]
fn test_string_length_too_short_returns_error() {
    let result = Validator::string_length("ab", "name", 3, 100);
    assert!(result.is_err());
}

#[test]
fn test_numeric_range_out_of_bounds_returns_error() {
    let result = Validator::numeric_range(150.0, "qos", 0.0, 2.0);
    assert!(result.is_err());
}

#[test]
fn test_identifier_invalid_chars_returns_error() {
    let result = Validator::identifier("bad name!", "device_id");
    assert!(result.is_err());
}
```

- [ ] **Step 2: Test agent create validation**

Test: empty name, name too long, focused mode without resources, invalid mode.

- [ ] **Step 3: Test device registration validation**

Test: empty device_id, invalid characters, missing type.

- [ ] **Step 4: Test MQTT subscription validation**

Test: empty topic, invalid QoS, malformed topic pattern.

- [ ] **Step 5: Run tests**

Run: `cd /Users/shenmingming/CamThink\ Project/NeoMind && cargo test -p neomind-api --lib 2>&1 | tail -20`

- [ ] **Step 6: Commit**

```bash
git commit -m "test(api): add handler parameter validation unit tests"
```

---

## Completion Checklist

- [ ] `cargo test -p neomind-storage --lib` passes with new timeseries + session tests
- [ ] `cargo test -p neomind-agent --lib` passes with new tool tests
- [ ] `cargo test -p neomind-rules --lib` passes with new DSL + engine tests
- [ ] `cargo test -p neomind-messages --lib` passes with new CRUD tests
- [ ] `cargo test -p neomind-extension-runner --lib` passes with new IPC tests
- [ ] `cargo test -p neomind-api --lib` passes with new validation tests
- [ ] `cargo test --workspace --lib` passes clean
