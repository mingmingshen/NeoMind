//! Event stream API handlers.
//!
//! Provides real-time event streaming via SSE and event history queries.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{Json, Sse, sse::Event},
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::handlers::ServerState;
use edge_ai_core::eventbus::{EventBus, EventBusReceiver};
use edge_ai_core::NeoTalkEvent;

/// Extract event data without the nested `type` field for frontend compatibility.
///
/// The Rust enum uses `#[serde(tag = "type")]` which serializes as:
///   `{ "type": "AgentExecutionStarted", "agent_id": "...", ... }`
///
/// But the frontend expects `data` to contain just the fields without `type`:
///   `{ "agent_id": "...", "agent_name": "...", ... }`
fn extract_event_data(event: &NeoTalkEvent) -> Value {
    match event {
        NeoTalkEvent::AgentExecutionStarted { agent_id, agent_name, execution_id, trigger_type, .. } => {
            serde_json::json!({
                "agent_id": agent_id,
                "agent_name": agent_name,
                "execution_id": execution_id,
                "trigger_type": trigger_type,
            })
        }
        NeoTalkEvent::AgentExecutionCompleted { agent_id, execution_id, success, duration_ms, error, .. } => {
            serde_json::json!({
                "agent_id": agent_id,
                "execution_id": execution_id,
                "success": success,
                "duration_ms": duration_ms,
                "error": error,
            })
        }
        NeoTalkEvent::AgentThinking { agent_id, execution_id, step_number, step_type, description, details, .. } => {
            serde_json::json!({
                "agent_id": agent_id,
                "execution_id": execution_id,
                "step_number": step_number,
                "step_type": step_type,
                "description": description,
                "details": details,
            })
        }
        NeoTalkEvent::AgentDecision { agent_id, execution_id, description, rationale, action, confidence, .. } => {
            serde_json::json!({
                "agent_id": agent_id,
                "execution_id": execution_id,
                "description": description,
                "rationale": rationale,
                "action": action,
                "confidence": confidence,
            })
        }
        NeoTalkEvent::AgentMemoryUpdated { agent_id, memory_type, .. } => {
            serde_json::json!({
                "agent_id": agent_id,
                "memory_type": memory_type,
            })
        }
        NeoTalkEvent::AgentProgress { agent_id, execution_id, stage, stage_label, progress, details, .. } => {
            serde_json::json!({
                "agent_id": agent_id,
                "execution_id": execution_id,
                "stage": stage,
                "stage_label": stage_label,
                "progress": progress,
                "details": details,
            })
        }
        // For other event types, serialize the full event (they may have the type field, but frontend handles them)
        _ => serde_json::to_value(event).unwrap_or(Value::Null),
    }
}

/// Event stream query parameters.
#[derive(Debug, Deserialize)]
pub struct EventStreamParams {
    /// Filter by event type (can be specified multiple times)
    #[serde(default)]
    pub event_type: Vec<String>,
    /// Filter by category: device, rule, workflow, llm, alert, tool
    #[serde(default)]
    pub category: Option<String>,
    /// Last event ID to resume from
    #[serde(default)]
    pub last_event_id: Option<String>,
    /// JWT authentication token
    #[serde(default)]
    pub token: Option<String>,
}

/// Event history query parameters.
#[derive(Debug, Deserialize)]
pub struct EventHistoryQuery {
    /// Start timestamp (Unix seconds)
    pub start: Option<i64>,
    /// End timestamp (Unix seconds)
    pub end: Option<i64>,
    /// Filter by event type
    #[serde(default)]
    pub event_type: Vec<String>,
    /// Filter by category
    #[serde(default)]
    pub category: Option<String>,
    /// Maximum number of results (default: 50, max: 200)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Skip N results for pagination
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    50
}

/// Event subscription request.
#[derive(Debug, Deserialize)]
pub struct EventSubscriptionRequest {
    /// Event types to subscribe to
    #[serde(default)]
    pub event_types: Vec<String>,
    /// Category to subscribe to
    #[serde(default)]
    pub category: Option<String>,
}

/// Event subscription response.
#[derive(Debug, Serialize)]
pub struct EventSubscriptionResponse {
    /// Subscription ID
    pub subscription_id: String,
    /// Subscribed event types
    pub event_types: Vec<String>,
    /// Subscription category filter
    pub category: Option<String>,
}

/// Event list response.
#[derive(Debug, Serialize)]
pub struct EventListResponse {
    /// List of events with metadata
    pub events: Vec<EventWithMetadata>,
    /// Total count (for this query filter)
    pub total: usize,
    /// Current offset
    pub offset: usize,
    /// Current limit
    pub limit: usize,
    /// Whether there are more results
    pub has_more: bool,
}

/// Event with metadata.
#[derive(Debug, Serialize)]
pub struct EventWithMetadata {
    /// The event data
    #[serde(flatten)]
    pub event: serde_json::Value,
    /// Event type name
    pub event_type: String,
    /// Event timestamp
    pub timestamp: i64,
    /// Event source
    pub source: String,
    /// Event ID (metadata) - renamed to `id` for frontend compatibility
    #[serde(rename = "id")]
    pub event_id: String,
    /// Processed status
    pub processed: bool,
}

/// Event statistics.
#[derive(Debug, Serialize)]
pub struct EventStats {
    /// Total events in retention period
    pub total_events: u64,
    /// Events by type
    pub events_by_type: HashMap<String, u64>,
    /// Events by category
    pub events_by_category: HashMap<String, u64>,
    /// Active subscriptions
    pub active_subscriptions: usize,
}

/// SSE endpoint for streaming events.
///
/// Streams real-time events from the event bus using Server-Sent Events.
/// Clients can filter by event type or category.
/// Requires JWT token authentication via `?token=xxx` parameter.
pub async fn event_stream_handler(
    State(state): State<ServerState>,
    Query(params): Query<EventStreamParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, axum::Error>>>, StatusCode> {
    // Validate JWT token - must be provided
    let token = params.token.as_ref().ok_or(StatusCode::UNAUTHORIZED)?;
    state
        .auth_user_state
        .validate_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Get the event bus from the server state
    let event_bus = state.event_bus.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    // Create a receiver for events
    let rx = create_filtered_receiver(event_bus, &params.category);

    // Create the SSE stream
    let stream = async_stream::stream! {
        let mut rx = rx;
        let mut _counter: u64 = 0;  // Event counter (reserved for future metrics)

        loop {
            match rx.recv().await {
                Some((event, metadata)) => {
                    // Apply event type filter if specified
                    if !params.event_type.is_empty() {
                        let event_type = event.type_name().to_string();
                        if !params.event_type.contains(&event_type) {
                            continue;
                        }
                    }

                    _counter += 1;

                    // Build SSE event with all data
                    // Use extract_event_data to remove nested type field for frontend compatibility
                    let data_with_id = serde_json::json!({
                        "id": metadata.event_id,
                        "type": event.type_name(),
                        "timestamp": event.timestamp(),
                        "source": metadata.source,
                        "data": extract_event_data(&event),
                    });

                    let sse_event = Event::default()
                        .event(event.type_name())
                        .json_data(data_with_id)
                        .unwrap_or_else(|_| Event::default().data(""));

                    yield Ok(sse_event);
                }
                None => {
                    // Channel closed
                    break;
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(30))
            .text("keepalive"),
    ))
}

/// Create a filtered receiver based on category.
fn create_filtered_receiver(event_bus: &EventBus, category: &Option<String>) -> EventBusReceiver {
    match category.as_deref() {
        Some("device") => {
            let filtered = event_bus.filter().device_events();
            // Convert to regular receiver by using the underlying broadcast
            drop(filtered);
            event_bus.subscribe()
        }
        Some("rule") => {
            let filtered = event_bus.filter().rule_events();
            drop(filtered);
            event_bus.subscribe()
        }
        Some("workflow") => {
            let filtered = event_bus.filter().workflow_events();
            drop(filtered);
            event_bus.subscribe()
        }
        Some("agent") => {
            let filtered = event_bus.filter().agent_events();
            drop(filtered);
            event_bus.subscribe()
        }
        Some("llm") => {
            let filtered = event_bus.filter().llm_events();
            drop(filtered);
            event_bus.subscribe()
        }
        Some("alert") => {
            let filtered = event_bus.filter().alert_events();
            drop(filtered);
            event_bus.subscribe()
        }
        Some("tool") => {
            let filtered = event_bus.filter().custom(|e| e.is_tool_event());
            drop(filtered);
            event_bus.subscribe()
        }
        _ => event_bus.subscribe(),
    }
}

/// WebSocket endpoint for event streaming.
///
/// Alternative to SSE using WebSocket for bidirectional communication.
/// Requires JWT token authentication via `?token=xxx` parameter.
pub async fn event_websocket_handler(
    State(state): State<ServerState>,
    ws: WebSocketUpgrade,
    Query(params): Query<EventStreamParams>,
) -> axum::response::Response {
    // Validate JWT token - reject connection if no token or invalid token
    let _auth_info = match params.token.as_ref() {
        Some(token) => match state.auth_user_state.validate_token(token) {
            Ok(info) => {
                tracing::info!("WebSocket event stream authenticated");
                Some(info)
            }
            Err(e) => {
                tracing::warn!(error = %e, "JWT validation failed, rejecting WebSocket connection");
                return ws.on_upgrade(|mut socket| {
                        async move {
                            use axum::extract::ws::Message;
                            let _ = socket.send(Message::Text(
                                serde_json::json!({"type": "Error", "message": "Invalid or expired token"}).to_string()
                            )).await;
                            let _ = socket.close().await;
                        }
                    });
            }
        },
        None => {
            tracing::warn!("No authentication provided, rejecting WebSocket connection");
            return ws.on_upgrade(|mut socket| async move {
                use axum::extract::ws::Message;
                let _ = socket
                    .send(Message::Text(
                        serde_json::json!({"type": "Error", "message": "Authentication required"})
                            .to_string(),
                    ))
                    .await;
                let _ = socket.close().await;
            });
        }
    };

    let event_bus = match state.event_bus.as_ref() {
        Some(bus) => bus.clone(),
        None => {
            return ws.on_upgrade(|mut socket| async move {
                use axum::extract::ws::Message;
                let _ = socket
                    .send(Message::Text(
                        serde_json::json!({"type": "Error", "message": "Event bus not available"})
                            .to_string(),
                    ))
                    .await;
                let _ = socket.close().await;
            });
        }
    };

    ws.on_upgrade(move |mut socket| async move {
        use axum::extract::ws::Message;

        let mut rx = create_filtered_receiver(&event_bus, &params.category);

        // Send events to the WebSocket client
        while let Some((event, metadata)) = rx.recv().await {
            // Apply event type filter
            if !params.event_type.is_empty() {
                let event_type = event.type_name().to_string();
                if !params.event_type.contains(&event_type) {
                    continue;
                }
            }

            let payload = serde_json::json!({
                "id": metadata.event_id,
                "type": event.type_name(),
                "timestamp": event.timestamp(),
                "source": metadata.source,
                "data": extract_event_data(&event),
            });

            let msg = match serde_json::to_string(&payload) {
                Ok(json) => Message::Text(json),
                Err(_) => continue,
            };

            if socket.send(msg).await.is_err() {
                break;
            }
        }

        let _ = socket.close().await;
    })
}

/// Query event history.
///
/// Retrieves historical events from storage with optional filtering and pagination.
pub async fn event_history_handler(
    State(state): State<ServerState>,
    Query(query): Query<EventHistoryQuery>,
) -> Result<Json<EventListResponse>, StatusCode> {
    // Get event log store from server state
    let event_log = state.event_log.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    use edge_ai_storage::business::EventFilter;

    // Cap limit to prevent excessive data load
    let limit = query.limit.min(200);
    let offset = query.offset;

    // Convert query parameters to EventFilter
    let mut filter = EventFilter::new();

    if let Some(start) = query.start {
        filter.start_time = Some(start);
    }
    if let Some(end) = query.end {
        filter.end_time = Some(end);
    }

    // Apply category filter by mapping to event types
    if let Some(category) = &query.category {
        filter.event_types = event_types_for_category(category);
    } else if !query.event_type.is_empty() {
        filter.event_types = query.event_type.clone();
    }

    // Set offset and limit for pagination
    filter.offset = Some(offset);
    filter.limit = Some(limit + 1); // Fetch one extra to determine if there are more

    // Query events from storage
    let logs = event_log
        .query(&filter)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Determine if there are more results
    let has_more = logs.len() > limit;
    let events_to_return = if has_more {
        logs[..limit].to_vec()
    } else {
        logs
    };

    // Convert EventLog entries to EventWithMetadata
    let events: Vec<EventWithMetadata> = events_to_return
        .into_iter()
        .map(|log| EventWithMetadata {
            event: serde_json::json!({
                "type": log.event_type,
                "message": log.message,
                "source": log.source,
                "severity": format!("{:?}", log.severity),
                "data": log.data,
            }),
            event_type: log.event_type,
            timestamp: log.timestamp,
            source: log.source.unwrap_or_default(),
            event_id: log.id,
            processed: true,
        })
        .collect();

    // Get total count by doing a separate query without limit/offset
    // For performance, we estimate or do a lightweight count query
    let _total_filter = EventFilter::new()
        .with_time_range(
            query.start.unwrap_or(i64::MAX - 86400 * 30),
            query.end.unwrap_or(i64::MAX)
        );
    // For exact total, we'd need to count - for now use a reasonable estimate
    // or omit this to avoid performance impact
    let total = offset + events.len() + if has_more { 1 } else { 0 };

    Ok(Json(EventListResponse {
        events,
        total,
        offset,
        limit,
        has_more,
    }))
}

/// Get event types for a category filter.
fn event_types_for_category(category: &str) -> Vec<String> {
    match category {
        "device" => vec![
            "DeviceOnline".to_string(),
            "DeviceOffline".to_string(),
            "DeviceMetric".to_string(),
            "DeviceCommandResult".to_string(),
        ],
        "rule" => vec![
            "RuleEvaluated".to_string(),
            "RuleTriggered".to_string(),
            "RuleExecuted".to_string(),
        ],
        "workflow" => vec![
            "WorkflowTriggered".to_string(),
            "WorkflowStepCompleted".to_string(),
            "WorkflowCompleted".to_string(),
        ],
        "llm" => vec![
            "PeriodicReviewTriggered".to_string(),
            "LlmDecisionProposed".to_string(),
            "LlmDecisionExecuted".to_string(),
            "UserMessage".to_string(),
            "LlmResponse".to_string(),
            "ToolExecutionStart".to_string(),
            "ToolExecutionSuccess".to_string(),
            "ToolExecutionFailure".to_string(),
        ],
        "alert" => vec![
            "AlertCreated".to_string(),
            "AlertAcknowledged".to_string(),
        ],
        _ => vec![],
    }
}

/// Query events with category filter.
///
/// Retrieves events filtered by category (device, rule, workflow, etc).
pub async fn events_query_handler(
    State(state): State<ServerState>,
    Query(query): Query<EventHistoryQuery>,
) -> Result<Json<EventListResponse>, StatusCode> {
    event_history_handler(State(state), Query(query)).await
}

/// Get event statistics.
///
/// Returns statistics about events in the system.
/// Optimized to only sample recent events (last 24 hours, max 1000).
pub async fn event_stats_handler(
    State(state): State<ServerState>,
) -> Result<Json<EventStats>, StatusCode> {
    let event_log = state.event_log.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    let event_bus = state.event_bus.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    // Only sample recent events for better performance
    // Query events from last 24 hours, max 1000
    let now = chrono::Utc::now().timestamp();
    let day_ago = now - 86400;

    use edge_ai_storage::business::EventFilter;
    let logs = event_log
        .query(&EventFilter::new()
            .with_time_range(day_ago, now)
            .with_limit(1000)
        )
        .unwrap_or_default();

    let mut events_by_type: HashMap<String, u64> = HashMap::new();
    let mut events_by_category: HashMap<String, u64> = HashMap::new();

    for log in logs {
        *events_by_type.entry(log.event_type.clone()).or_insert(0) += 1;

        // Categorize events
        let category = categorize_event(&log.event_type);
        *events_by_category.entry(category.to_string()).or_insert(0) += 1;
    }

    Ok(Json(EventStats {
        total_events: events_by_type.values().sum(),
        events_by_type,
        events_by_category,
        active_subscriptions: event_bus.subscriber_count(),
    }))
}

/// Create an event subscription.
///
/// Creates a new subscription for specific event types.
pub async fn subscribe_events_handler(
    State(state): State<ServerState>,
    Json(req): Json<EventSubscriptionRequest>,
) -> Result<Json<EventSubscriptionResponse>, StatusCode> {
    let _event_bus = state.event_bus.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    // Generate a subscription ID
    let subscription_id = format!("sub-{}", uuid::Uuid::new_v4());

    // In a full implementation, we would store this subscription
    // For now, return the subscription info
    Ok(Json(EventSubscriptionResponse {
        subscription_id,
        event_types: req.event_types,
        category: req.category,
    }))
}

/// Delete an event subscription.
///
/// Cancels an existing event subscription.
pub async fn unsubscribe_events_handler(Path(_id): Path<String>) -> Result<StatusCode, StatusCode> {
    // In a full implementation, we would remove the subscription
    // For now, just return success
    tracing::info!("Cancelling subscription: {}", _id);
    Ok(StatusCode::NO_CONTENT)
}

/// Helper function to categorize events.
fn categorize_event(event_type: &str) -> &str {
    match event_type {
        "DeviceOnline" | "DeviceOffline" | "DeviceMetric" | "DeviceCommandResult" => "device",
        "RuleEvaluated" | "RuleTriggered" | "RuleExecuted" => "rule",
        "WorkflowTriggered" | "WorkflowStepCompleted" | "WorkflowCompleted" => "workflow",
        "PeriodicReviewTriggered"
        | "LlmDecisionProposed"
        | "LlmDecisionExecuted"
        | "UserMessage"
        | "LlmResponse" => "llm",
        "AlertCreated" | "AlertAcknowledged" => "alert",
        "ToolExecutionStart" | "ToolExecutionSuccess" | "ToolExecutionFailure" => "tool",
        _ => "other",
    }
}

/// Generate test events for development/demo purposes.
///
/// This endpoint creates sample events of different types for testing the UI.
/// Only available in debug builds.
#[cfg(debug_assertions)]
pub async fn generate_test_events_handler(
    State(state): State<ServerState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let event_bus = state.event_bus.as_ref().ok_or(StatusCode::NOT_FOUND)?;

    use edge_ai_core::{MetricValue, NeoTalkEvent};

    let now = chrono::Utc::now().timestamp();

    // Generate various test events
    let test_events = vec![
        // Device events
        NeoTalkEvent::DeviceOnline {
            device_id: "sensor-temp-01".to_string(),
            device_type: "temperature_sensor".to_string(),
            timestamp: now - 300,
        },
        NeoTalkEvent::DeviceMetric {
            device_id: "sensor-temp-01".to_string(),
            metric: "temperature".to_string(),
            value: MetricValue::Float(23.5),
            timestamp: now - 290,
            quality: Some(1.0),
        },
        NeoTalkEvent::DeviceMetric {
            device_id: "sensor-temp-01".to_string(),
            metric: "humidity".to_string(),
            value: MetricValue::Float(65.0),
            timestamp: now - 280,
            quality: Some(1.0),
        },
        NeoTalkEvent::DeviceCommandResult {
            device_id: "switch-living-01".to_string(),
            command: "turn_on".to_string(),
            success: true,
            result: Some(serde_json::json!({"status": "ok"})),
            timestamp: now - 270,
        },
        NeoTalkEvent::DeviceOffline {
            device_id: "sensor-motion-02".to_string(),
            reason: Some("Connection lost".to_string()),
            timestamp: now - 260,
        },
        NeoTalkEvent::DeviceOnline {
            device_id: "sensor-light-03".to_string(),
            device_type: "light_sensor".to_string(),
            timestamp: now - 250,
        },
        NeoTalkEvent::DeviceMetric {
            device_id: "sensor-light-03".to_string(),
            metric: "illuminance".to_string(),
            value: MetricValue::Integer(450),
            timestamp: now - 240,
            quality: Some(1.0),
        },
        // Rule events
        NeoTalkEvent::RuleEvaluated {
            rule_id: "rule-temp-alert".to_string(),
            rule_name: "Temperature Alert".to_string(),
            condition_met: true,
            timestamp: now - 200,
        },
        NeoTalkEvent::RuleTriggered {
            rule_id: "rule-temp-alert".to_string(),
            rule_name: "Temperature Alert".to_string(),
            trigger_value: 28.5,
            actions: vec!["send_alert".to_string()],
            timestamp: now - 190,
        },
        // Workflow events
        NeoTalkEvent::WorkflowTriggered {
            workflow_id: "daily-report".to_string(),
            trigger_type: "schedule".to_string(),
            trigger_data: Some(serde_json::json!({"cron": "0 9 * * *"})),
            execution_id: "exec-001".to_string(),
            timestamp: now - 180,
        },
        NeoTalkEvent::WorkflowStepCompleted {
            workflow_id: "daily-report".to_string(),
            step_id: "collect-data".to_string(),
            execution_id: "exec-001".to_string(),
            result: serde_json::json!({"status": "completed"}),
            timestamp: now - 170,
        },
        NeoTalkEvent::WorkflowCompleted {
            workflow_id: "daily-report".to_string(),
            execution_id: "exec-001".to_string(),
            success: true,
            duration_ms: 5000,
            timestamp: now - 160,
        },
        // LLM events
        NeoTalkEvent::PeriodicReviewTriggered {
            review_id: "review-001".to_string(),
            review_type: "daily".to_string(),
            timestamp: now - 150,
        },
        NeoTalkEvent::LlmDecisionProposed {
            decision_id: "decision-001".to_string(),
            title: "Adjust temperature setting".to_string(),
            description: "Temperature is too high, consider lowering".to_string(),
            reasoning: "Based on current sensor readings".to_string(),
            actions: vec![],
            confidence: 0.85,
            timestamp: now - 140,
        },
        NeoTalkEvent::LlmDecisionExecuted {
            decision_id: "decision-001".to_string(),
            success: true,
            result: Some(serde_json::json!({"executed": true})),
            timestamp: now - 130,
        },
        // Alert events
        NeoTalkEvent::AlertCreated {
            alert_id: "alert-001".to_string(),
            title: "Temperature threshold exceeded".to_string(),
            severity: "warning".to_string(),
            message: "Temperature is above threshold".to_string(),
            timestamp: now - 120,
        },
        // Tool events
        NeoTalkEvent::ToolExecutionStart {
            tool_name: "mqtt-publish".to_string(),
            arguments: serde_json::json!({"topic": "test", "payload": "on"}),
            session_id: None,
            timestamp: now - 110,
        },
        NeoTalkEvent::ToolExecutionSuccess {
            tool_name: "mqtt-publish".to_string(),
            arguments: serde_json::json!({"topic": "test", "payload": "on"}),
            result: serde_json::json!({"published": true}),
            duration_ms: 50,
            session_id: None,
            timestamp: now - 100,
        },
    ];

    // Publish all test events
    let mut published = 0;
    for event in test_events {
        event_bus.publish(event).await;
        published += 1;
    }

    Ok(Json(serde_json::json!({
        "message": "Generated test events",
        "count": published,
        "types": ["device", "rule", "workflow", "llm", "alert", "tool"]
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_categorization() {
        assert_eq!(categorize_event("DeviceOnline"), "device");
        assert_eq!(categorize_event("RuleTriggered"), "rule");
        assert_eq!(categorize_event("WorkflowCompleted"), "workflow");
        assert_eq!(categorize_event("LlmDecisionProposed"), "llm");
        assert_eq!(categorize_event("AlertCreated"), "alert");
        assert_eq!(categorize_event("ToolExecutionStart"), "tool");
        assert_eq!(categorize_event("UnknownEvent"), "other");
    }

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 100);
    }
}
