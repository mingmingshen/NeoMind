//! Test data generation handler.
//!
//! Provides endpoints to generate sample data for development/testing.

use axum::{extract::State};

use edge_ai_alerts::{Alert, AlertSeverity};
use edge_ai_core::event::{MetricValue, NeoTalkEvent, ProposedAction};

use super::{ServerState, common::{HandlerResult, ok}};

#[derive(Debug, serde::Serialize)]
pub struct TestDataSummary {
    pub alerts_created: usize,
    pub events_published: usize,
    pub message: String,
}

/// Generate test alerts.
pub async fn generate_test_alerts_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let alerts = generate_test_alerts();
    let mut created = 0;

    for alert in alerts {
        match state.alert_manager.create_alert(alert).await {
            Ok(_) => created += 1,
            Err(_) => {}, // Ignore duplicates
        }
    }

    ok(serde_json::json!({
        "alerts_created": created,
        "message": format!("Created {} test alerts", created)
    }))
}

/// Publish test events.
pub async fn generate_test_events_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let events = generate_test_events();
    let mut published = 0;

    if let Some(event_bus) = &state.event_bus {
        for event in events {
            event_bus.publish(event).await;
            published += 1;
        }
    }

    ok(serde_json::json!({
        "events_published": published,
        "message": format!("Published {} test events", published)
    }))
}

/// Generate all test data (alerts + events).
pub async fn generate_test_data_handler(
    State(state): State<ServerState>,
) -> HandlerResult<serde_json::Value> {
    let mut summary = TestDataSummary {
        alerts_created: 0,
        events_published: 0,
        message: String::new(),
    };

    // Create alerts
    let alerts = generate_test_alerts();
    for alert in alerts {
        if state.alert_manager.create_alert(alert).await.is_ok() {
            summary.alerts_created += 1;
        }
    }

    // Publish events
    if let Some(event_bus) = &state.event_bus {
        let events = generate_test_events();
        for event in events {
            event_bus.publish(event).await;
            summary.events_published += 1;
        }
    }

    summary.message = format!(
        "Generated {} alerts and {} events",
        summary.alerts_created,
        summary.events_published
    );

    ok(serde_json::to_value(&summary).unwrap())
}

fn generate_test_alerts() -> Vec<Alert> {
    let mut alerts = Vec::new();

    // Emergency alerts
    alerts.push(Alert::new(
        AlertSeverity::Emergency,
        "烟雾检测".to_string(),
        "厨房传感器检测到烟雾，请立即确认！".to_string(),
        "sensor/kitchen".to_string(),
    ));

    alerts.push(Alert::new(
        AlertSeverity::Emergency,
        "漏水警报".to_string(),
        "地下室检测到漏水，水泵已启动".to_string(),
        "sensor/basement".to_string(),
    ));

    // Critical alerts
    alerts.push(Alert::new(
        AlertSeverity::Critical,
        "冰箱温度过高".to_string(),
        "冰箱内部温度达到 8°C，食物可能变质风险".to_string(),
        "sensor/fridge".to_string(),
    ));

    alerts.push(Alert::new(
        AlertSeverity::Critical,
        "门锁异常".to_string(),
        "前门锁连续 3 次开锁失败，可能存在异常尝试".to_string(),
        "lock/front".to_string(),
    ));

    alerts.push(Alert::new(
        AlertSeverity::Critical,
        "电池电量低".to_string(),
        "门锁电池电量低于 10%，请及时更换".to_string(),
        "lock/front".to_string(),
    ));

    // Warning alerts
    alerts.push(Alert::new(
        AlertSeverity::Warning,
        "温度偏高".to_string(),
        "客厅温度达到 28°C，超过设定阈值 26°C".to_string(),
        "sensor/living".to_string(),
    ));

    alerts.push(Alert::new(
        AlertSeverity::Warning,
        "设备离线警告".to_string(),
        "传感器 sensor/garden-01 已超过 5 分钟未上报数据".to_string(),
        "device_monitor".to_string(),
    ));

    alerts.push(Alert::new(
        AlertSeverity::Warning,
        "存储空间不足".to_string(),
        "系统存储空间使用率超过 85%".to_string(),
        "system/monitor".to_string(),
    ));

    // Info alerts
    alerts.push(Alert::new(
        AlertSeverity::Info,
        "系统启动完成".to_string(),
        "NeoTalk 系统已成功启动，所有服务正常运行".to_string(),
        "system".to_string(),
    ));

    alerts.push(Alert::new(
        AlertSeverity::Info,
        "固件更新可用".to_string(),
        "网关设备有新固件版本 v2.1.0 可用".to_string(),
        "update_manager".to_string(),
    ));

    alerts.push(Alert::new(
        AlertSeverity::Info,
        "场景执行成功".to_string(),
        "「回家模式」场景已自动执行".to_string(),
        "automation".to_string(),
    ));

    alerts
}

fn generate_test_events() -> Vec<NeoTalkEvent> {
    let now = chrono::Utc::now().timestamp();
    let mut events = Vec::new();

    // Device online events
    events.push(NeoTalkEvent::DeviceOnline {
        device_id: "sensor/living".to_string(),
        device_type: "sensor".to_string(),
        timestamp: now,
    });

    events.push(NeoTalkEvent::DeviceOnline {
        device_id: "light/bedroom".to_string(),
        device_type: "light".to_string(),
        timestamp: now,
    });

    events.push(NeoTalkEvent::DeviceOnline {
        device_id: "sensor/temp".to_string(),
        device_type: "sensor".to_string(),
        timestamp: now,
    });

    // Device metric events
    events.push(NeoTalkEvent::DeviceMetric {
        device_id: "sensor/temp".to_string(),
        metric: "temperature".to_string(),
        value: MetricValue::Float(25.5),
        timestamp: now,
        quality: None,
    });

    events.push(NeoTalkEvent::DeviceMetric {
        device_id: "sensor/temp".to_string(),
        metric: "humidity".to_string(),
        value: MetricValue::Float(60.0),
        timestamp: now,
        quality: Some(100.0),
    });

    events.push(NeoTalkEvent::DeviceMetric {
        device_id: "sensor/living".to_string(),
        metric: "temperature".to_string(),
        value: MetricValue::Float(28.5),
        timestamp: now,
        quality: Some(95.0),
    });

    // Device command result events
    events.push(NeoTalkEvent::DeviceCommandResult {
        device_id: "light/living".to_string(),
        command: "on".to_string(),
        success: true,
        result: Some(serde_json::json!({"brightness": 80})),
        timestamp: now,
    });

    events.push(NeoTalkEvent::DeviceCommandResult {
        device_id: "curtain/window".to_string(),
        command: "open".to_string(),
        success: true,
        result: Some(serde_json::json!({"position": 80})),
        timestamp: now,
    });

    events.push(NeoTalkEvent::DeviceCommandResult {
        device_id: "switch/ac".to_string(),
        command: "set_temperature".to_string(),
        success: true,
        result: Some(serde_json::json!({"target_temp": 24})),
        timestamp: now,
    });

    // Rule evaluated events
    events.push(NeoTalkEvent::RuleEvaluated {
        rule_id: "temp-high-alert".to_string(),
        rule_name: "高温告警".to_string(),
        condition_met: true,
        timestamp: now,
    });

    events.push(NeoTalkEvent::RuleTriggered {
        rule_id: "temp-high-alert".to_string(),
        rule_name: "高温告警".to_string(),
        trigger_value: 28.5,
        actions: vec!["send_notification".to_string(), "log_event".to_string()],
        timestamp: now,
    });

    // Rule executed events
    events.push(NeoTalkEvent::RuleExecuted {
        rule_id: "temp-high-alert".to_string(),
        rule_name: "高温告警".to_string(),
        success: true,
        duration_ms: 15,
        timestamp: now,
    });

    // Workflow events
    events.push(NeoTalkEvent::WorkflowTriggered {
        workflow_id: "morning-routine".to_string(),
        trigger_type: "schedule".to_string(),
        trigger_data: Some(serde_json::json!({"time": "07:00"})),
        execution_id: uuid::Uuid::new_v4().to_string(),
        timestamp: now,
    });

    events.push(NeoTalkEvent::WorkflowStepCompleted {
        workflow_id: "morning-routine".to_string(),
        execution_id: uuid::Uuid::new_v4().to_string(),
        step_id: "open_curtains".to_string(),
        result: serde_json::json!({"status": "success"}),
        timestamp: now,
    });

    // Workflow completed
    events.push(NeoTalkEvent::WorkflowCompleted {
        workflow_id: "morning-routine".to_string(),
        execution_id: uuid::Uuid::new_v4().to_string(),
        success: true,
        duration_ms: 250,
        timestamp: now,
    });

    // LLM decision events
    let actions = vec![
        ProposedAction::control_device("switch/ac", "set_temperature", serde_json::json!({"temp": 24})),
        ProposedAction::notify_user("室内温度过高，已自动开启空调降温"),
    ];

    events.push(NeoTalkEvent::LlmDecisionProposed {
        decision_id: "decision-001".to_string(),
        title: "建议开启空调".to_string(),
        description: "客厅温度达到28.5°C，超过设定阈值26°C".to_string(),
        reasoning: "根据温度传感器数据，当前室温过高，建议启动空调设备进行降温。".to_string(),
        actions,
        confidence: 0.92,
        timestamp: now,
    });

    // Tool execution events
    events.push(NeoTalkEvent::ToolExecutionStart {
        tool_name: "control_device".to_string(),
        arguments: serde_json::json!({"device_id": "light/living", "command": "on"}),
        session_id: Some("session-123".to_string()),
        timestamp: now,
    });

    events.push(NeoTalkEvent::ToolExecutionSuccess {
        tool_name: "control_device".to_string(),
        arguments: serde_json::json!({"device_id": "light/living", "command": "on"}),
        result: serde_json::json!({"status": "ok"}),
        duration_ms: 45,
        session_id: Some("session-123".to_string()),
        timestamp: now,
    });

    events
}
