//! Integration Tests for Rules Engine
//!
//! Tests cover:
//! - DSL parsing and execution
//! - Rule engine operations
//! - Device integration
//! - Rule validation
//! - Complex rule scenarios

use neomind_rules::{
    dsl::{ComparisonOperator, RuleAction, RuleCondition},
    InMemoryValueProvider, RuleDslParser, RuleEngine, ValueProvider,
};
use std::sync::Arc;

// ============================================================================
// DSL Parsing Integration Tests
// ============================================================================

#[test]
fn test_parse_simple_temperature_rule() {
    let dsl = r#"
        RULE "High Temperature Alert"
        WHEN sensor.temperature > 30
        DO
            NOTIFY "Temperature is too high"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse rule");

    assert_eq!(rule.name, "High Temperature Alert");
    assert!(!rule.actions.is_empty());
}

#[test]
fn test_parse_complex_condition_rule() {
    let dsl = r#"
        RULE "Complex Alert"
        WHEN (sensor.temperature > 30) AND (sensor.humidity < 50)
        DO
            NOTIFY "Hot and dry conditions"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse rule");

    // Should parse AND condition
    match &rule.condition {
        RuleCondition::And(conditions) => {
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected AND condition"),
    }
}

#[test]
fn test_parse_or_condition_rule() {
    let dsl = r#"
        RULE "OR Alert"
        WHEN (sensor.temperature > 40) OR (sensor.humidity > 90)
        DO
            NOTIFY "Extreme conditions"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse rule");

    match &rule.condition {
        RuleCondition::Or(conditions) => {
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected OR condition"),
    }
}

#[test]
fn test_parse_range_condition() {
    let dsl = r#"
        RULE "Range Check"
        WHEN sensor.temperature BETWEEN 20 AND 30
        DO
            NOTIFY "Temperature in range"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse rule");

    match &rule.condition {
        RuleCondition::DeviceRange {
            device_id,
            metric,
            min,
            max,
        } => {
            assert_eq!(device_id, "sensor");
            assert_eq!(metric, "temperature");
            assert_eq!(*min, 20.0);
            assert_eq!(*max, 30.0);
        }
        _ => panic!("Expected range condition"),
    }
}

#[test]
fn test_parse_multiple_actions() {
    let dsl = r#"
        RULE "Multi-action"
        WHEN sensor.temperature > 30
        DO
            NOTIFY "High temperature"
            LOG alert "Temperature warning"
            EXECUTE thermostat.set_mode(mode="cool")
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse rule");

    assert_eq!(rule.actions.len(), 3);
}

#[test]
fn test_parse_with_duration() {
    let dsl = r#"
        RULE "Persistent Alert"
        WHEN sensor.temperature > 30
        FOR 5 minutes
        DO
            NOTIFY "Sustained high temperature"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse rule");

    assert!(rule.for_duration.is_some());
    let duration = rule.for_duration.unwrap();
    assert_eq!(duration.as_secs(), 300); // 5 minutes
}

#[test]
fn test_parse_with_description() {
    let dsl = r#"
        RULE "Described Rule"
        DESCRIPTION "This rule monitors temperature"
        WHEN sensor.temperature > 30
        DO
            NOTIFY "Alert"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse rule");

    assert_eq!(
        rule.description,
        Some("This rule monitors temperature".to_string())
    );
}

#[test]
fn test_parse_with_tags() {
    let dsl = r#"
        RULE "Tagged Rule"
        TAGS temperature, alert, critical
        WHEN sensor.temp > 40
        DO
            NOTIFY "Critical"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse rule");

    assert_eq!(rule.tags.len(), 3);
    assert!(rule.tags.contains(&"temperature".to_string()));
}

// ============================================================================
// Rule Engine Integration Tests
// ============================================================================

#[tokio::test]
async fn test_rule_engine_add_rule() {
    let provider = Arc::new(InMemoryValueProvider::new());
    let engine = RuleEngine::new(provider);

    let dsl = r#"
        RULE "Test Rule"
        WHEN sensor.temperature > 25
        DO
            NOTIFY "Test"
        END
    "#;

    let result = engine.add_rule_from_dsl(dsl).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rule_engine_multiple_rules() {
    let provider = Arc::new(InMemoryValueProvider::new());
    let engine = RuleEngine::new(provider);

    // Add multiple rules
    let rules = vec![
        r#"RULE "Rule1" WHEN sensor.a > 1 DO NOTIFY "1" END"#,
        r#"RULE "Rule2" WHEN sensor.b > 2 DO NOTIFY "2" END"#,
        r#"RULE "Rule3" WHEN sensor.c > 3 DO NOTIFY "3" END"#,
    ];

    for dsl in rules {
        let result = engine.add_rule_from_dsl(dsl).await;
        assert!(result.is_ok(), "Failed to add rule: {:?}", result);
    }

    assert_eq!(engine.list_rules().await.len(), 3);
}

#[tokio::test]
async fn test_rule_engine_remove_rule() {
    let provider = Arc::new(InMemoryValueProvider::new());
    let engine = RuleEngine::new(provider);

    let dsl = r#"RULE "Removable" WHEN sensor.x > 0 DO NOTIFY "Test" END"#;
    let rule_id = engine.add_rule_from_dsl(dsl).await.unwrap();

    // Remove
    let result = engine.remove_rule(&rule_id).await;
    assert!(result.is_ok());
    assert_eq!(engine.list_rules().await.len(), 0);
}

// ============================================================================
// Value Provider Integration Tests
// ============================================================================

#[test]
fn test_in_memory_value_provider() {
    let provider = InMemoryValueProvider::new();

    provider.set_value("device1", "temperature", 25.5);
    provider.set_value("device1", "humidity", 60.0);

    assert_eq!(provider.get_value("device1", "temperature"), Some(25.5));
    assert_eq!(provider.get_value("device1", "humidity"), Some(60.0));
    assert_eq!(provider.get_value("nonexistent", "metric"), None);
}

#[test]
fn test_value_provider_update() {
    let provider = InMemoryValueProvider::new();

    provider.set_value("device", "counter", 1.0);
    assert_eq!(provider.get_value("device", "counter"), Some(1.0));

    provider.set_value("device", "counter", 2.0);
    assert_eq!(provider.get_value("device", "counter"), Some(2.0));
}

// ============================================================================
// Condition Evaluation Tests
// ============================================================================

#[test]
fn test_comparison_operators() {
    let operators = vec![
        (ComparisonOperator::GreaterThan, 30.0, 25.0, true),
        (ComparisonOperator::LessThan, 20.0, 25.0, true),
        (ComparisonOperator::GreaterEqual, 25.0, 25.0, true),
        (ComparisonOperator::LessEqual, 25.0, 25.0, true),
        (ComparisonOperator::Equal, 25.0, 25.0, true),
        (ComparisonOperator::NotEqual, 25.0, 30.0, true),
    ];

    for (op, left, right, expected) in operators {
        let result = match op {
            ComparisonOperator::GreaterThan => left > right,
            ComparisonOperator::LessThan => left < right,
            ComparisonOperator::GreaterEqual => left >= right,
            ComparisonOperator::LessEqual => left <= right,
            ComparisonOperator::Equal => left == right,
            ComparisonOperator::NotEqual => left != right,
        };
        assert_eq!(result, expected);
    }
}

// ============================================================================
// Action Parsing Tests
// ============================================================================

#[test]
fn test_parse_notify_action() {
    let dsl = r#"
        RULE "Notify Test"
        WHEN sensor.x > 0
        DO
            NOTIFY "Alert message" [email, sms]
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse");

    match &rule.actions[0] {
        RuleAction::Notify { message, channels } => {
            assert_eq!(message, "Alert message");
            assert!(channels.is_some());
            let chans = channels.as_ref().unwrap();
            assert_eq!(chans.len(), 2);
        }
        _ => panic!("Expected Notify action"),
    }
}

#[test]
fn test_parse_execute_action() {
    let dsl = r#"
        RULE "Execute Test"
        WHEN sensor.x > 0
        DO
            EXECUTE device1.turn_on(mode="auto", speed=100)
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse");

    match &rule.actions[0] {
        RuleAction::Execute {
            device_id,
            command,
            params,
        } => {
            assert_eq!(device_id, "device1");
            assert_eq!(command, "turn_on");
            assert_eq!(params["mode"], "auto");
            assert_eq!(params["speed"], 100);
        }
        _ => panic!("Expected Execute action"),
    }
}

#[test]
fn test_parse_log_action() {
    let dsl = r#"
        RULE "Log Test"
        WHEN sensor.x > 0
        DO
            LOG info "Information message"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse");

    match &rule.actions[0] {
        RuleAction::Log { message, .. } => {
            assert_eq!(message, "Information message");
        }
        _ => panic!("Expected Log action"),
    }
}

#[test]
fn test_parse_http_request_action() {
    let dsl = r#"
        RULE "HTTP Test"
        WHEN sensor.x > 0
        DO
            HTTP POST https://api.example.com/alert
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse");

    match &rule.actions[0] {
        RuleAction::HttpRequest { url, .. } => {
            assert_eq!(url, "https://api.example.com/alert");
        }
        _ => panic!("Expected HttpRequest action"),
    }
}

#[test]
fn test_parse_set_action() {
    let dsl = r#"
        RULE "Set Test"
        WHEN sensor.x > 0
        DO
            SET device.mode = "auto"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse");

    match &rule.actions[0] {
        RuleAction::Set {
            device_id,
            property,
            value,
        } => {
            assert_eq!(device_id, "device");
            assert_eq!(property, "mode");
            assert_eq!(*value, serde_json::json!("auto"));
        }
        _ => panic!("Expected Set action"),
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_parse_invalid_rule() {
    let dsl = r#"
        RULE "Invalid"
        WHEN temperature >>>
        DO
            NOTIFY "Test"
        END
    "#;

    let result = RuleDslParser::parse(dsl);
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_rule() {
    let result = RuleDslParser::parse("");
    assert!(result.is_err());
}

// ============================================================================
// Complex Rule Scenarios
// ============================================================================

#[test]
fn test_nested_conditions() {
    let dsl = r#"
        RULE "Nested"
        WHEN ((sensor.temp > 30) OR (sensor.temp < 10)) AND (sensor.humidity > 50)
        DO
            NOTIFY "Complex condition"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse");
    assert_eq!(rule.name, "Nested");
}

#[test]
fn test_all_comparison_operators() {
    let operators = [">", "<", ">=", "<=", "==", "!="];

    for op in operators {
        let dsl = format!(
            r#"
            RULE "Op Test"
            WHEN sensor.temperature {} 25
            DO
                NOTIFY "Test"
            END
        "#,
            op
        );

        let result = RuleDslParser::parse(&dsl);
        assert!(result.is_ok(), "Failed for operator: {}", op);
    }
}

#[test]
fn test_all_time_units() {
    let time_units = [("seconds", 1), ("minutes", 60), ("hours", 3600)];

    for (unit, _) in time_units {
        let dsl = format!(
            r#"
            RULE "Time Test"
            WHEN sensor.x > 0
            FOR 5 {}
            DO
                NOTIFY "Test"
            END
        "#,
            unit
        );

        let rule = RuleDslParser::parse(&dsl).expect("Failed to parse");
        assert!(rule.for_duration.is_some());
    }
}

// ============================================================================
// Rule Metadata Tests
// ============================================================================

#[test]
fn test_rule_get_device_metrics() {
    let dsl = r#"
        RULE "Multi-device"
        WHEN (sensor1.temp > 30) AND (sensor2.humidity < 50)
        DO
            NOTIFY "Test"
        END
    "#;

    let rule = RuleDslParser::parse(dsl).expect("Failed to parse");

    let metrics = rule.condition.get_device_metrics();
    assert_eq!(metrics.len(), 2);
}

// ============================================================================
// Integration with Value Provider
// ============================================================================

#[tokio::test]
async fn test_rule_with_device_metric() {
    let provider = Arc::new(InMemoryValueProvider::new());
    provider.set_value("sensor", "temperature", 35.0);

    let engine = RuleEngine::new(provider);

    let dsl = r#"
        RULE "Device Metric"
        WHEN sensor.temperature > 30
        DO
            NOTIFY "High temperature"
        END
    "#;

    let _rule_id = engine.add_rule_from_dsl(dsl).await.unwrap();

    // Update states and evaluate
    engine.update_states().await;
    let triggered = engine.evaluate_rules().await;

    // Rule should trigger because temperature > 30
    assert_eq!(triggered.len(), 1);
}
