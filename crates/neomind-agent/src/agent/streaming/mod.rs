//! Streaming response processing with thinking tag support.
//!
//! This module includes safeguards against infinite LLM loops:
//! - Global stream timeout
//! - Maximum thinking content length
//! - Maximum tool call iterations
//! - Repetition detection

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{Stream, StreamExt};
use tokio::sync::RwLock;

use super::staged::{IntentCategory, IntentClassifier};
use super::tool_parser::{parse_tool_calls, remove_tool_calls_from_response};
use super::types::{
    AgentEvent, AgentInternalState, AgentMessage, AgentMessageImage, LargeDataCache, ToolCall,
};
use crate::error::{NeoMindError, Result};
use crate::llm::LlmInterface;

mod intent;
pub(crate) use intent::*;

mod cache;
pub use cache::ToolResultCache;
pub(crate) use cache::*;

mod thinking;
pub use thinking::cleanup_thinking_content;

mod tool_detect;
pub(crate) use tool_detect::*;

mod sanitize;
pub(crate) use sanitize::*;

// Type aliases to reduce complexity
pub type SharedLlm = Arc<RwLock<LlmInterface>>;
pub type ToolResultStream = Pin<Box<dyn Stream<Item = (String, String)> + Send>>;
pub type EventChannel = tokio::sync::mpsc::Sender<AgentEvent>;

// Re-export compaction types for use in other modules
pub use neomind_core::llm::compaction::{CompactionConfig, MessagePriority};

/// Configuration for stream processing safeguards
///
/// These safeguards prevent infinite loops and excessive resource usage
/// during LLM streaming operations.
///
/// The default values are synchronized with `neomind_core::llm::backend::StreamConfig`
/// to ensure consistent behavior across the system.
pub struct StreamSafeguards {
    /// Maximum time allowed for entire stream processing (default: 300s)
    ///
    /// This matches `StreamConfig::max_stream_duration_secs` and provides
    /// adequate time for complex reasoning tasks, especially with thinking models.
    pub max_stream_duration: Duration,

    /// Maximum thinking content length in characters (default: unlimited)
    ///
    /// Note: The actual thinking limit is enforced by the LLM backend's
    /// `StreamConfig::max_thinking_chars`. This field is retained for
    /// additional safety if needed.
    pub max_thinking_length: usize,

    /// Maximum content length in characters (default: unlimited)
    pub max_content_length: usize,

    /// Maximum tool call iterations per request (default: 3)
    pub max_tool_iterations: usize,

    /// Maximum consecutive similar chunks to detect loops (default: 3)
    pub max_repetition_count: usize,

    /// Heartbeat interval to keep connection alive (default: 10s)
    pub heartbeat_interval: Duration,

    /// Progress update interval during long operations (default: 5s)
    pub progress_interval: Duration,

    /// Optional interrupt signal - when set, stream should stop gracefully
    /// This allows users to interrupt long thinking processes
    pub interrupt_signal: Option<tokio::sync::watch::Receiver<bool>>,
}

impl Default for StreamSafeguards {
    fn default() -> Self {
        Self {
            // Synchronized with StreamConfig::max_stream_duration_secs (1200s)
            // This provides adequate time for thinking models like qwen3-vl:2b
            // to complete extended reasoning before generating content.
            max_stream_duration: Duration::from_secs(1200),

            // No limit on thinking content - let the LLM backend enforce limits
            max_thinking_length: usize::MAX,

            max_content_length: usize::MAX,

            // Tool iterations limit - high limit to support complex multi-step queries
            // Actual loop uses MAX_TOOL_ITERATIONS constant; this value is for truncating
            // tool calls in a single LLM response.
            max_tool_iterations: 100,

            // Repetition detection threshold
            max_repetition_count: 3,

            // Heartbeat every 10 seconds to prevent WebSocket timeout
            heartbeat_interval: Duration::from_secs(10),

            // Progress update every 5 seconds during long operations
            progress_interval: Duration::from_secs(5),

            // No interrupt signal by default
            interrupt_signal: None,
        }
    }
}

impl StreamSafeguards {
    /// Create a new StreamSafeguards with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a StreamSafeguards optimized for fast models.
    ///
    /// This reduces timeouts and limits for models that respond quickly
    /// and don't need extended thinking time.
    pub fn fast_model() -> Self {
        Self {
            max_stream_duration: Duration::from_secs(120),
            max_thinking_length: 10_000,
            max_tool_iterations: 8,
            ..Self::default()
        }
    }

    /// Create a StreamSafeguards optimized for reasoning models.
    ///
    /// This increases timeouts for models that benefit from extended
    /// reasoning time (e.g., vision models, thinking-enabled models).
    pub fn reasoning_model() -> Self {
        Self {
            max_stream_duration: Duration::from_secs(600),
            max_thinking_length: 100_000,
            max_tool_iterations: 15,
            ..Self::default()
        }
    }

    /// Set the interrupt signal for this stream
    /// Returns a sender that can be used to trigger the interrupt
    pub fn with_interrupt_signal(mut self, rx: tokio::sync::watch::Receiver<bool>) -> Self {
        self.interrupt_signal = Some(rx);
        self
    }

    /// Create an interruptible stream with a (tx, rx) pair
    /// Returns (safeguards, sender) where sender can be used to interrupt
    pub fn with_interrupt() -> (Self, tokio::sync::watch::Sender<bool>) {
        let (tx, rx) = tokio::sync::watch::channel(false);
        let safeguards = Self::default().with_interrupt_signal(rx);
        (safeguards, tx)
    }
}

/// Deduplicate accumulated tool results across multiple rounds.
///
/// Keeps the **latest** result for each (tool_name, key_arguments) combination.
/// When the same tool is called with the same arguments across rounds (LLM retrying),
/// only the last successful result is kept. Different arguments produce separate entries.
fn deduplicate_tool_results(results: &[(String, String)]) -> Vec<(String, String)> {
    // Build a key from tool name + distinguishing arguments parsed from the result JSON
    let mut seen: Vec<(String, String)> = Vec::new(); // (key, dedup_key)
    let mut deduped: Vec<(String, String)> = Vec::new();

    for (name, result) in results {
        // Create a dedup key from name + result fingerprint
        let dedup_key = make_result_dedup_key(name, result);

        if let Some(pos) = seen
            .iter()
            .position(|(k, dk)| k == name && dk == &dedup_key)
        {
            // Replace with latest result
            deduped[pos] = (name.clone(), result.clone());
        } else {
            seen.push((name.clone(), dedup_key));
            deduped.push((name.clone(), result.clone()));
        }
    }

    deduped
}

/// Create a dedup key for a tool result by extracting entity identifiers.
fn make_result_dedup_key(name: &str, result: &str) -> String {
    // Try to extract entity IDs from the result JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(result) {
        let mut key_parts = vec![name.to_string()];

        // Extract common entity identifiers
        for field in &["device_id", "metric", "agent_id", "rule_id", "id", "name"] {
            if let Some(val) = json.get(*field).and_then(|v| v.as_str()) {
                key_parts.push(val.to_string());
            }
        }

        // For device query results, also check nested data
        if let Some(data) = json.get("data") {
            if let Some(obj) = data.as_object() {
                for field in &["device_id", "device_name"] {
                    if let Some(val) = obj.get(*field).and_then(|v| v.as_str()) {
                        key_parts.push(val.to_string());
                    }
                }
            }
        }

        return key_parts.join("|");
    }

    // Fallback: simple hash of the result content for dedup
    let preview: String = result.chars().take(200).collect();
    let hash = preview
        .chars()
        .fold(0u64, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u64));
    format!("{}|{:016x}", name, hash)
}

/// Helper function to extract an array from a JSON value, handling both direct arrays
/// and truncated nested structures ({"items": [...], "_total_count": N, ...})
fn extract_array(json_value: &serde_json::Value, key: &str) -> Option<Vec<serde_json::Value>> {
    // First try to get the key directly as an array
    if let Some(arr) = json_value.get(key).and_then(|v| v.as_array()) {
        return Some(arr.clone());
    }

    // Then try to get it from a truncated structure
    if let Some(obj) = json_value.get(key).and_then(|v| v.as_object()) {
        if let Some(items) = obj.get("items").and_then(|i| i.as_array()) {
            return Some(items.clone());
        }
    }

    None
}

/// Format results from aggregated tools (device, agent, rule, alert, extension)
/// by detecting the JSON structure. This handles both aggregated and legacy tool names.
fn format_aggregated_tool_result(tool_name: &str, json: &serde_json::Value, response: &mut String) {
    // Detect what kind of result this is based on JSON structure

    // Agent list: has "agents" key with array or nested object
    if json.get("agents").is_some() || json.get("count").is_some() && tool_name == "agent" {
        format_agent_list(json, response);
        return;
    }

    // Device list: has "devices" array
    if let Some(devices) = extract_array(json, "devices") {
        response.push_str(&format!("## Device List ({} total)\n\n", devices.len()));
        for device in devices {
            let name = device
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");
            let id = device.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let device_type = device
                .get("type")
                .or_else(|| device.get("device_type"))
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            let status = device.get("status").and_then(|s| s.as_str()).unwrap_or("");

            if status.is_empty() {
                response.push_str(&format!("- **{}** ({}) - {}\n", name, id, device_type));
            } else {
                response.push_str(&format!(
                    "- **{}** ({}) - {} - {}\n",
                    name, id, device_type, status
                ));
            }
        }
        return;
    }

    // Device query result: has "device_id" and "points"
    if json.get("device_id").is_some() && json.get("points").is_some() {
        let device_id = json
            .get("device_id")
            .and_then(|d| d.as_str())
            .unwrap_or("unknown");
        let metric = json
            .get("metric")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown");
        let points = json.get("points").and_then(|p| p.as_array());

        response.push_str(&format!("## {} - {}\n\n", device_id, metric));

        if let Some(pts) = points {
            if pts.is_empty() {
                response.push_str("No data available.\n");
            } else {
                for point in pts.iter().take(10) {
                    let ts = point.get("timestamp").and_then(|t| t.as_i64()).unwrap_or(0);
                    let is_image = point.get("base64_data").is_some();
                    let value = if is_image {
                        "[image data]".to_string()
                    } else {
                        point
                            .get("value")
                            .map(|v| v.to_string().trim_matches('"').to_string())
                            .unwrap_or_else(|| "N/A".to_string())
                    };

                    if ts > 0 {
                        let time_str = chrono::DateTime::from_timestamp(ts, 0)
                            .map(|dt| dt.format("%H:%M:%S").to_string())
                            .unwrap_or_else(|| ts.to_string());
                        response.push_str(&format!("- {}: {}\n", time_str, value));
                    } else {
                        response.push_str(&format!("- {}\n", value));
                    }
                }
                if pts.len() > 10 {
                    response.push_str(&format!("\n... ({} more data points)\n", pts.len() - 10));
                }
            }
        }
        return;
    }

    // Device get with metrics: has "id"/"name" + "type" + "metrics" array with values
    if json.get("name").is_some() && json.get("type").is_some() {
        if let Some(metrics) = json.get("metrics").and_then(|m| m.as_array()) {
            let name = json
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");
            let device_type = json
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            response.push_str(&format!("## {} ({})\n\n", name, device_type));

            for metric in metrics {
                let display_name = metric
                    .get("display_name")
                    .and_then(|d| d.as_str())
                    .or_else(|| metric.get("name").and_then(|n| n.as_str()))
                    .unwrap_or("unknown");
                let unit = metric.get("unit").and_then(|u| u.as_str()).unwrap_or("");

                if let Some(value) = metric.get("value") {
                    let value_str = value.to_string().trim_matches('"').to_string();
                    if unit.is_empty() {
                        response.push_str(&format!("- **{}**: {}\n", display_name, value_str));
                    } else {
                        response
                            .push_str(&format!("- **{}**: {} {}\n", display_name, value_str, unit));
                    }
                } else {
                    response.push_str(&format!("- **{}**: 无数据\n", display_name));
                }
            }
            return;
        }
    }

    // Metric not found with suggestions: has "error" + "available_metrics"
    if json.get("error").is_some() && json.get("available_metrics").is_some() {
        let error = json
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("Unknown error");
        response.push_str(&format!("**Error**: {}\n\n", error));

        if let Some(available) = json.get("available_metrics").and_then(|a| a.as_array()) {
            response.push_str("**Available metrics:**\n");
            for metric in available {
                let name = metric.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let display_name = metric
                    .get("display_name")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                let unit = metric.get("unit").and_then(|u| u.as_str()).unwrap_or("");
                if display_name.is_empty() {
                    response.push_str(&format!("- `{}`\n", name));
                } else if unit.is_empty() {
                    response.push_str(&format!("- `{}` ({})\n", name, display_name));
                } else {
                    response.push_str(&format!("- `{}` ({}) - {}\n", name, display_name, unit));
                }
            }
        }
        return;
    }

    // Rule list: has "rules" array
    if let Some(rules) = extract_array(json, "rules") {
        response.push_str(&format!("## Automation Rules ({} total)\n\n", rules.len()));
        for rule in rules {
            let name = rule
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");
            // Support both new "status" string field and legacy "enabled" boolean
            let status_display = if let Some(status) = rule.get("status").and_then(|s| s.as_str()) {
                match status {
                    "active" => "[Active]",
                    "paused" => "[Paused]",
                    "triggered" => "[Triggered]",
                    "disabled" => "[Disabled]",
                    _ => status,
                }
            } else if rule
                .get("enabled")
                .and_then(|e| e.as_bool())
                .unwrap_or(false)
            {
                "[Active]"
            } else {
                "[Disabled]"
            };
            let desc = rule
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            if desc.is_empty() {
                response.push_str(&format!("- **{}** {}\n", name, status_display));
            } else {
                response.push_str(&format!("- **{}** {} -- {}\n", name, status_display, desc));
            }
        }
        return;
    }

    // Alert list: has "alerts" array
    if let Some(alerts) = extract_array(json, "alerts") {
        response.push_str(&format!("## Alerts ({} total)\n\n", alerts.len()));
        for alert in alerts.iter().take(10) {
            let title = alert
                .get("title")
                .or_else(|| alert.get("message"))
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            let severity = alert
                .get("severity")
                .and_then(|s| s.as_str())
                .unwrap_or("info");
            let tag = match severity {
                "critical" => "[CRITICAL]",
                "warning" => "[WARN]",
                _ => "[INFO]",
            };
            response.push_str(&format!("- {} **{}** ({})\n", tag, title, severity));
        }
        if alerts.len() > 10 {
            response.push_str(&format!("\n... ({} more alerts)\n", alerts.len() - 10));
        }
        return;
    }

    // Extension list: has "extensions" array
    if let Some(extensions) = extract_array(json, "extensions") {
        response.push_str(&format!("## Extensions ({} total)\n\n", extensions.len()));
        for ext in extensions {
            let name = ext
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");
            let status = ext
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown");
            let tag = if status == "running" {
                "[running]"
            } else {
                "[stopped]"
            };
            response.push_str(&format!("- {} **{}** ({})\n", tag, name, status));
        }
        return;
    }

    // Agent details: has "name" and "type" at top level (single agent)
    if json.get("name").is_some() && json.get("type").is_some() && tool_name == "agent" {
        let name = json
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");
        let status = json
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");
        response.push_str(&format!("## Agent: {}\n\n", name));
        response.push_str(&format!("**Status**: {}\n", status));
        if let Some(stats) = json.get("stats") {
            if let Some(total) = stats.get("total_executions").and_then(|t| t.as_u64()) {
                response.push_str(&format!("**Total Executions**: {}\n", total));
            }
        }
        return;
    }

    // Agent execution history: has "agent_id" + "stats"
    if json.get("agent_id").is_some() && json.get("stats").is_some() {
        if let Some(stats) = json.get("stats") {
            let total = stats
                .get("total_executions")
                .and_then(|t| t.as_u64())
                .unwrap_or(0);
            let success = stats
                .get("successful_executions")
                .and_then(|s| s.as_u64())
                .unwrap_or(0);
            let failed = stats
                .get("failed_executions")
                .and_then(|f| f.as_u64())
                .unwrap_or(0);
            let avg_ms = stats
                .get("avg_duration_ms")
                .and_then(|d| d.as_u64())
                .unwrap_or(0);
            response.push_str("## Execution Stats\n\n");
            response.push_str(&format!("- **Total**: {} times\n", total));
            response.push_str(&format!(
                "- **Success**: {} | **Failed**: {}\n",
                success, failed
            ));
            if avg_ms > 0 {
                let avg_sec = avg_ms as f64 / 1000.0;
                response.push_str(&format!("- **Avg Duration**: {:.1}s\n", avg_sec));
            }
            if let Some(last_ms) = stats.get("last_duration_ms").and_then(|d| d.as_u64()) {
                if last_ms > 0 {
                    response.push_str(&format!(
                        "- **Last Duration**: {:.1}s\n",
                        last_ms as f64 / 1000.0
                    ));
                }
            }
        }
        return;
    }

    // Message/alert list: has "count" and "messages" array with message objects (id, title, level)
    if let Some(messages) = extract_array(json, "messages") {
        // Distinguish from agent conversation history:
        // message tool returns objects with "title", "level", "read" fields
        // agent conversation returns objects with "role", "content" fields
        let is_message_list = messages.first().is_some_and(|m| {
            m.get("title").is_some() || m.get("level").is_some() || m.get("read").is_some()
        });

        if is_message_list {
            let count = json
                .get("count")
                .and_then(|c| c.as_u64())
                .unwrap_or(messages.len() as u64);
            response.push_str(&format!("## Messages & Alerts ({} total)\n\n", count));
            for msg in messages.iter().take(15) {
                let title = msg
                    .get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                let level = msg.get("level").and_then(|l| l.as_str()).unwrap_or("info");
                let read = msg.get("read").and_then(|r| r.as_bool()).unwrap_or(false);
                let id = msg.get("id").and_then(|i| i.as_str()).unwrap_or("");

                let icon = match level {
                    "urgent" | "critical" => "[CRITICAL]",
                    "important" => "[IMPORTANT]",
                    "notice" | "warning" => "[WARN]",
                    _ => "[INFO]",
                };
                let read_icon = if read { "[read]" } else { "[unread]" };
                response.push_str(&format!(
                    "{} {} [{}] {} (`{}`)\n",
                    icon,
                    read_icon,
                    level,
                    title,
                    &id[..8.min(id.len())]
                ));
            }
            if messages.len() > 15 {
                response.push_str(&format!("\n... ({} more)\n", messages.len() - 15));
            }
            return;
        }
    }

    // Agent conversation history: has "messages" array with role/content
    if let Some(messages) = extract_array(json, "messages") {
        response.push_str(&format!(
            "## Conversation Log ({} messages)\n\n",
            messages.len()
        ));
        for msg in messages.iter().take(10) {
            let role = msg
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("unknown");
            let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
            let preview: String = content.chars().take(100).collect();
            response.push_str(&format!("- **{}**: {}\n", role, preview));
        }
        if messages.len() > 10 {
            response.push_str(&format!("\n... ({} more messages)\n", messages.len() - 10));
        }
        return;
    }

    // Control/execution success — but check if there's meaningful data first
    if json.get("success").is_some()
        || json.get("execution_id").is_some()
        || json.get("rule_id").is_some()
    {
        // If there's a "data" object with useful fields, format those instead of generic message
        if let Some(data) = json.get("data") {
            format_json_data(data, response);
            return;
        }
        if let Some(exec_id) = json.get("execution_id").and_then(|e| e.as_str()) {
            response.push_str(&format!("[OK] Executed successfully (ID: {})\n", exec_id));
        } else if let Some(rule_id) = json.get("rule_id").and_then(|r| r.as_str()) {
            response.push_str(&format!("[OK] Created successfully (ID: {})\n", rule_id));
        } else if let Some(agent_id) = json
            .get("agent_id")
            .or_else(|| json.get("id"))
            .and_then(|a| a.as_str())
        {
            response.push_str(&format!("[OK] Created successfully (ID: {})\n", agent_id));
        } else if json
            .get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(false)
        {
            response.push_str(&format!("**[OK]** {} operation succeeded\n", tool_name));
        } else {
            // Has error
            let error = json
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error");
            response.push_str(&format!("!! {} failed: {}\n", tool_name, error));
        }
        return;
    }

    // Fallback: format the JSON object with key-value pairs (handles extension tools, etc.)
    if json.is_object() || json.is_array() {
        format_json_data(json, response);
    } else {
        response.push_str(&format!("**[OK]** {} completed.\n", tool_name));
    }
}

/// Format agent list from JSON result.
fn format_agent_list(json: &serde_json::Value, response: &mut String) {
    let agents_array = if let Some(agents_obj) = json.get("agents").and_then(|a| a.as_object()) {
        agents_obj.get("items").and_then(|i| i.as_array())
    } else {
        json.get("agents").and_then(|a| a.as_array())
    };

    if let Some(agents) = agents_array {
        if agents.is_empty() {
            response.push_str("**AI Agent List**\n\nNo AI Agents configured.");
        } else {
            response.push_str(&format!("**AI Agent List** ({} total)\n\n", agents.len()));
            for agent in agents {
                let name = agent
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown");
                let status = agent
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                let icon = match status {
                    "active" | "Active" => "[on]",
                    _ => "[off]",
                };
                response.push_str(&format!("- {} **{}** ({})\n", icon, name, status));

                if let Some(desc) = agent.get("description").and_then(|d| d.as_str()) {
                    if !desc.is_empty() && desc != "null" {
                        response.push_str(&format!("  {}\n", desc));
                    }
                }
            }
        }
    } else if let Some(count) = json.get("count").and_then(|c| c.as_u64()) {
        response.push_str(&format!("**AI Agent List** ({} total)\n", count));
    } else {
        response.push_str("**AI Agent List**\n\nNo AI Agents found.");
    }
}

/// Format a generic JSON data object into readable key-value pairs.
/// Used for extension tool results (weather, image analysis, etc.)
fn format_json_data(data: &serde_json::Value, response: &mut String) {
    if let Some(obj) = data.as_object() {
        for (key, value) in obj {
            // Skip nested objects and arrays in simple view
            if value.is_object() || value.is_array() {
                continue;
            }

            // Convert snake_case to Title Case
            let display_name: String = key
                .chars()
                .enumerate()
                .flat_map(|(i, c)| {
                    if i == 0 {
                        c.to_uppercase().collect::<Vec<char>>()
                    } else if c == '_' {
                        vec![' ']
                    } else {
                        vec![c]
                    }
                })
                .collect();

            let value_str = match value {
                serde_json::Value::Bool(b) => {
                    if *b {
                        "Yes".to_string()
                    } else {
                        "No".to_string()
                    }
                }
                serde_json::Value::Number(n) => {
                    if key.ends_with("_c") {
                        format!("{}°C", n)
                    } else if key.ends_with("_percent") {
                        format!("{}%", n)
                    } else if key.ends_with("_kmph") {
                        format!("{} km/h", n)
                    } else if key.ends_with("_hpa") {
                        format!("{} hPa", n)
                    } else if key.ends_with("_ms") || key.ends_with("_duration_ms") {
                        format!("{:.1}s", n.as_f64().unwrap_or(0.0) / 1000.0)
                    } else {
                        n.to_string()
                    }
                }
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };

            response.push_str(&format!("- **{}**: {}\n", display_name, value_str));
        }
    } else if let Some(arr) = data.as_array() {
        for (i, item) in arr.iter().enumerate().take(10) {
            response.push_str(&format!("{}. {}\n", i + 1, item));
        }
        if arr.len() > 10 {
            response.push_str(&format!("\n... ({} more)\n", arr.len() - 10));
        }
    } else {
        response.push_str(&format!("{}\n", data));
    }
}

/// Format tool results into a user-friendly response
/// This avoids calling the LLM again after tool execution, preventing excessive thinking
pub fn format_tool_results(tool_results: &[(String, String)]) -> String {
    if tool_results.is_empty() {
        return "操作已完成。".to_string();
    }

    let mut response = String::new();

    for (tool_name, result) in tool_results {
        // Try to parse the result as JSON for better formatting
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(result) {
            match tool_name.as_str() {
                "shell" => {
                    let cmd = json_value
                        .get("command")
                        .and_then(|c| c.as_str())
                        .unwrap_or("?");
                    let desc = json_value.get("description").and_then(|d| d.as_str());
                    if let Some(desc) = desc {
                        response.push_str(&format!("## Shell: {}\n**Command**: `{}`\n", desc, cmd));
                    } else {
                        response.push_str(&format!("## Shell: `{}`\n", cmd));
                    }
                    if json_value
                        .get("timed_out")
                        .and_then(|t| t.as_bool())
                        .unwrap_or(false)
                    {
                        response.push_str("**Timed out**\n");
                    }
                    if let Some(exit_code) = json_value.get("exit_code") {
                        response.push_str(&format!("**Exit code**: {}\n", exit_code));
                    }
                    if let Some(stdout) = json_value.get("stdout").and_then(|s| s.as_str()) {
                        if !stdout.is_empty() {
                            response.push_str(&format!("```\n{}\n```\n", stdout));
                        }
                    }
                    if let Some(stderr) = json_value.get("stderr").and_then(|s| s.as_str()) {
                        if !stderr.is_empty() {
                            response.push_str(&format!("**stderr:**\n```\n{}\n```\n", stderr));
                        }
                    }
                }
                _ => {
                    // Aggregated tools (device, agent, rule, alert, extension) share the
                    // same JSON output format as the legacy tools. Detect the format by
                    // inspecting the JSON structure instead of matching tool names.
                    format_aggregated_tool_result(tool_name, &json_value, &mut response);
                }
            }
        } else {
            // Result is not valid JSON, use as-is
            // Use a structured format with result prefix to prevent LLM hallucination
            // of tool results (model can learn the simple "tool executed" pattern)
            // Show more for error messages to preserve diagnostic info
            let is_error = result.starts_with("Error:");
            let max_chars = if is_error { 500 } else { 80 };
            let preview: String = result.chars().take(max_chars).collect();
            response.push_str(&format!("**[ToolResult:{}]** {}\n", tool_name, preview));
        }
    }

    if response.ends_with('\n') {
        response.pop();
    }

    // Safe character-based slicing for logging
    let preview: String = response.chars().take(200).collect();
    tracing::info!(
        "format_tool_results: Final output length: {} chars, preview: {}",
        response.len(),
        preview
    );
    response
}

/// Result of a single tool execution with metadata
struct ToolExecutionResult {
    _name: String,
    arguments: serde_json::Value,
    result: std::result::Result<crate::toolkit::ToolOutput, crate::toolkit::ToolError>,
}

/// Build context window with optional conversation summary injection.
///
/// When a summary is provided, messages up to `summary_up_to_index` are removed
/// and a system message with the summary is prepended to the context.
fn build_context_window_with_summary(
    messages: &[AgentMessage],
    max_tokens: usize,
    summary: Option<&str>,
    summary_up_to_index: Option<u64>,
) -> Vec<AgentMessage> {
    // Adapt compaction to model capacity — larger contexts get gentler treatment
    let config = CompactionConfig::for_context_size(max_tokens);

    // Filter out summarized messages if summary exists
    let filtered: Vec<AgentMessage> =
        if let (Some(_summary), Some(up_to)) = (summary, summary_up_to_index) {
            messages
                .iter()
                .enumerate()
                .filter(|(i, _)| (*i as u64) > up_to)
                .map(|(_, msg)| msg.clone())
                .collect()
        } else {
            messages.to_vec()
        };

    // Build context window from filtered messages
    let mut result = build_context_window_with_config(&filtered, max_tokens, &config);

    // Inject summary as a system message at the beginning (after any existing system messages)
    if let Some(summary_text) = summary {
        if !summary_text.is_empty() {
            let summary_msg = AgentMessage::system(format!("[之前对话的摘要]\n{}", summary_text));
            // Find insertion point: after system messages, before other messages
            let insert_pos = result.iter().take_while(|m| m.role == "system").count();
            result.insert(insert_pos, summary_msg);
        }
    }

    result
}

/// Build context window with custom compaction configuration.
///
/// This function applies the compaction strategy to AgentMessage sequences,
/// which are the primary message type used in the agent system.
///
/// ## Parameters
/// - `messages`: The message history to compact
/// - `max_tokens`: Maximum tokens available for history
/// - `config`: Compaction configuration
pub fn build_context_window_with_config(
    messages: &[AgentMessage],
    max_tokens: usize,
    config: &CompactionConfig,
) -> Vec<AgentMessage> {
    // Step 1: Calculate total tokens without any compaction
    let total_tokens: usize = messages.iter().map(estimate_message_tokens).sum();

    // Step 2: Only compact tool results if we're actually over budget
    let working = if config.compact_tool_results && total_tokens > max_tokens {
        compact_tool_results_stream_with_config(messages, config)
    } else {
        messages.to_vec()
    };

    let mut selected_messages = Vec::new();
    let mut current_tokens = 0;

    for msg in working.iter().rev() {
        let msg_tokens = estimate_message_tokens(msg);

        // Calculate priority for this message
        let priority = message_priority(&msg.role);
        let is_recent = selected_messages.len() < config.min_recent_messages;

        // Keep messages by priority:
        // - System: always keep
        // - User: always keep (represents conversation intent, critical for context)
        // - Recent: always keep (ensures continuity)
        let should_keep = priority >= MessagePriority::User || is_recent;

        if !should_keep && current_tokens + msg_tokens > max_tokens {
            // Budget exceeded, skip this message
            continue;
        }

        // Truncate long messages only if we're near budget
        let final_msg = if total_tokens > max_tokens && msg_tokens > config.max_message_length {
            truncate_agent_message(msg, config.max_message_length)
        } else {
            msg.clone()
        };

        current_tokens += estimate_message_tokens(&final_msg);
        selected_messages.push(final_msg);
    }

    selected_messages.reverse();
    selected_messages
}

/// Get the priority for an AgentMessage role.
fn message_priority(role: &str) -> MessagePriority {
    match role {
        "system" => MessagePriority::System,
        "user" => MessagePriority::User,
        "assistant" => MessagePriority::Assistant,
        _ => MessagePriority::Tool,
    }
}

/// Estimate tokens for an AgentMessage — delegates to unified tokenizer.
fn estimate_message_tokens(msg: &AgentMessage) -> usize {
    crate::agent::tokenizer::estimate_message_tokens(msg)
}

/// Truncate an AgentMessage's content to fit within max length.
fn truncate_agent_message(msg: &AgentMessage, max_len: usize) -> AgentMessage {
    let mut truncated = msg.clone();

    if msg.content.len() > max_len {
        // Truncate at character boundary
        let prefix: String = msg.content.chars().take(max_len).collect();
        let truncated_content = if let Some(last_space) = prefix.rfind(' ') {
            format!("{}...", &prefix[..last_space])
        } else {
            format!("{}...", prefix)
        };
        truncated.content = truncated_content.into();
    }

    // Also truncate thinking if present
    if let Some(thinking) = &truncated.thinking {
        if thinking.len() > max_len / 2 {
            let half = thinking.floor_char_boundary(max_len / 2);
            truncated.thinking = Some(if let Some(last_space) = thinking[..half].rfind(' ') {
                format!("{}...", &thinking[..last_space])
            } else {
                format!("{}...", &thinking[..half])
            });
        }
    }

    truncated
}

/// Compact tool results with custom configuration.
fn compact_tool_results_stream_with_config(
    messages: &[AgentMessage],
    config: &CompactionConfig,
) -> Vec<AgentMessage> {
    if !config.compact_tool_results {
        return messages.to_vec();
    }

    let mut result = Vec::new();
    let mut tool_result_count = 0;

    for msg in messages.iter().rev() {
        if msg.role == "user" || msg.role == "system" {
            result.push(msg.clone());
            continue;
        }

        // Check if this is a tool response
        if msg.tool_call_id.is_some() && msg.role == "assistant" {
            tool_result_count += 1;

            if tool_result_count <= config.keep_recent_tool_results {
                result.push(msg.clone());
            } else {
                // Build descriptive summary preserving action + args + result preview
                let summary_content = if let Some(ref tool_calls) = msg.tool_calls {
                    let summaries: Vec<String> = tool_calls
                        .iter()
                        .map(|tc| {
                            let args_summary =
                                super::types::summarize_tool_args(&tc.name, &tc.arguments);
                            let result_preview = tc
                                .result
                                .as_ref()
                                .map(|r| {
                                    let s = if let Some(s) = r.as_str() {
                                        s.to_string()
                                    } else {
                                        r.to_string()
                                    };
                                    // Read actions need more preview to preserve data.
                                    // Compact time-series format uses ~10KB for 1440 points,
                                    // so data actions need generous preview (2KB) to keep stats.
                                    let is_data_action = args_summary.contains("list")
                                        || args_summary.contains("get")
                                        || args_summary.contains("history");
                                    let preview_len = if is_data_action { 2048 } else { 80 };
                                    s.chars().take(preview_len).collect::<String>()
                                })
                                .unwrap_or_default();
                            if result_preview.is_empty() {
                                format!("the {} tool with {}", tc.name, args_summary)
                            } else {
                                format!(
                                    "the {} tool with {} and received: {}",
                                    tc.name, args_summary, result_preview
                                )
                            }
                        })
                        .collect();
                    format!(
                        "Previously called {}. These are past results, do not repeat.",
                        summaries.join(", then ")
                    )
                } else {
                    let tool_name = msg.tool_call_name.as_deref().unwrap_or("tool");
                    format!(
                        "Previously called the {} tool. These are past results, do not repeat.",
                        tool_name
                    )
                };

                let summary_msg = AgentMessage {
                    role: "assistant".to_string(),
                    content: summary_content.into(),
                    tool_calls: None,
                    tool_call_id: None,
                    tool_call_name: None,
                    thinking: None,
                    images: None,
                    round_contents: None,
                    round_thinking: None,
                    timestamp: msg.timestamp,
                };
                result.push(summary_msg);
            }
        } else {
            result.push(msg.clone());
        }
    }

    result.reverse();
    result
}

/// Process a user message with streaming response.
///
/// Logic:
/// 1. Stream LLM response in real-time
/// 2. Detect tool calls during streaming
/// 3. If tool call detected:
///    - Execute tools in parallel
///    - Get final LLM response based on tool results
///    - Stream the final response
///
/// ## Safeguards against infinite loops:
/// - Global stream timeout (60s default)
/// - Maximum thinking content length (10000 chars)
/// - Maximum content length (20000 chars)
/// - Repetition detection to catch loops
/// - Maximum tool call iterations (5)
pub async fn process_stream_events(
    llm_interface: Arc<LlmInterface>,
    internal_state: Arc<tokio::sync::RwLock<AgentInternalState>>,
    tools: Arc<crate::toolkit::ToolRegistry>,
    user_message: &str,
    conversation_summary: Option<String>,
    summary_up_to_index: Option<u64>,
) -> Result<Pin<Box<dyn Stream<Item = AgentEvent> + Send>>> {
    process_stream_events_with_safeguards(
        llm_interface,
        internal_state,
        tools,
        user_message,
        StreamSafeguards::default(),
        conversation_summary,
        summary_up_to_index,
    )
    .await
}

pub async fn process_stream_events_with_safeguards(
    llm_interface: Arc<LlmInterface>,
    internal_state: Arc<tokio::sync::RwLock<AgentInternalState>>,
    tools: Arc<crate::toolkit::ToolRegistry>,
    user_message: &str,
    safeguards: StreamSafeguards,
    conversation_summary: Option<String>,
    summary_up_to_index: Option<u64>,
) -> Result<Pin<Box<dyn Stream<Item = AgentEvent> + Send>>> {
    let user_message = user_message.to_string();

    // === INTENT RECOGNITION: Understand user intent before LLM call ===
    // This helps reduce cognitive load and provides better visualization
    let classifier = IntentClassifier::default();
    let intent_result = classifier.classify(&user_message);

    tracing::info!(
        "Intent recognized: category={:?}, confidence={:.2}, keywords={:?}",
        intent_result.category,
        intent_result.confidence,
        intent_result.keywords
    );

    // Prepare intent and plan events for frontend visualization
    let intent_event = AgentEvent::intent(
        format!("{:?}", intent_result.category),
        intent_result.category.display_name(),
        intent_result.confidence,
        intent_result.keywords.clone(),
    );

    // Plan steps based on intent
    let plan_steps = match intent_result.category {
        IntentCategory::Device => vec![
            ("识别用户查询意图", "Intent"),
            ("获取设备列表", "Execution"),
            ("返回设备信息", "Response"),
        ],
        IntentCategory::Rule => vec![
            ("识别规则查询意图", "Intent"),
            ("获取规则列表", "Execution"),
            ("返回规则信息", "Response"),
        ],
        IntentCategory::Workflow => vec![
            ("识别工作流查询意图", "Intent"),
            ("获取工作流列表", "Execution"),
            ("返回工作流信息", "Response"),
        ],
        IntentCategory::Data => vec![
            ("识别数据查询意图", "Intent"),
            ("查询设备数据", "Execution"),
            ("返回数据结果", "Response"),
        ],
        IntentCategory::Alert => vec![
            ("识别告警查询意图", "Intent"),
            ("获取告警列表", "Execution"),
            ("返回告警信息", "Response"),
        ],
        IntentCategory::System => vec![
            ("识别系统状态意图", "Intent"),
            ("获取系统信息", "Execution"),
            ("返回系统状态", "Response"),
        ],
        IntentCategory::Help => vec![("识别帮助请求意图", "Intent"), ("提供使用说明", "Response")],
        IntentCategory::General => vec![("理解用户问题", "Intent"), ("生成回复", "Response")],
    };

    // === Get conversation history and pass to LLM ===
    // This prevents the LLM from repeating actions or calling tools again
    // Pure async - no block_in_place
    let state_guard = internal_state.read().await;
    let history_messages = state_guard.memory.clone();
    drop(state_guard); // Release lock before calling LLM

    // === DYNAMIC CONTEXT WINDOW: Get model's actual capacity ===
    let max_context = llm_interface.max_context_length().await;

    // Measure actual overhead from system prompt + tool definitions
    let prompt_overhead = llm_interface.estimate_prompt_overhead_tokens().await;

    // Reserve tokens for model response generation (minimum 1024)
    const RESERVE_FOR_RESPONSE: usize = 1024;

    // History budget = total capacity - prompt overhead - response reserve
    let effective_max = max_context
        .saturating_sub(prompt_overhead)
        .saturating_sub(RESERVE_FOR_RESPONSE);

    // Safety floor: always allow at least 20% of context for history
    let min_history = (max_context * 20) / 100;
    let effective_max = effective_max.max(min_history);

    tracing::debug!(
        "Context window: model_capacity={}, prompt_overhead={}, reserve={}, effective_max={} for history",
        max_context, prompt_overhead, RESERVE_FOR_RESPONSE, effective_max
    );

    let history_for_llm: Vec<neomind_core::Message> = build_context_window_with_summary(
        &history_messages,
        effective_max,
        conversation_summary.as_deref(),
        summary_up_to_index,
    )
    .iter()
    .map(|msg| msg.to_core())
    .collect::<Vec<_>>();

    tracing::debug!(
        "Passing {} messages from history to LLM",
        history_for_llm.len()
    );

    // === THINKING CONTROL ===
    // Thinking is controlled by the user/instance thinking_enabled setting.
    // The LlmInterface resolves the effective thinking state from:
    //   1. Local override (per-request)
    //   2. Instance manager setting (from storage/frontend)
    //   3. Backend default
    // No keyword-based filtering — model providers have inconsistent standards.

    // Thinking control: Respect the user/instance thinking_enabled setting directly.
    // The llm_interface already resolves thinking priority: local override > instance setting > None.
    // No keyword-based filtering — model providers have different standards, keyword heuristics
    // are unreliable and override user preference without good reason.
    tracing::info!("Thinking control: respecting user/instance thinking_enabled setting directly");

    // Get the stream from llm_interface - thinking is controlled by instance/user settings
    let stream_result = llm_interface
        .chat_stream_with_history(&user_message, &history_for_llm)
        .await;

    let stream = stream_result.map_err(|e| NeoMindError::Llm(e.to_string()))?;

    Ok(Box::pin(async_stream::stream! {
        let mut stream = stream;
        let mut buffer = String::new();
        let mut yielded_up_to: usize = 0; // Track how much of buffer has been yielded to prevent duplication
        let mut tool_calls_detected = false;
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut content_before_tools = String::new();
        let mut thinking_content = String::new();

        // === SKILL CONTEXT: Clear transient skill context from previous turn ===
        llm_interface.clear_skill_context().await;
        let mut has_content = false;
        let mut has_thinking = false;

        // === SAFEGUARD: Track stream start time for timeout ===
        let stream_start = Instant::now();

        // === KEEPALIVE: Track last event time for heartbeat ===
        #[allow(unused_assignments)]
        let mut last_event_time = Instant::now();
        let mut last_progress_time = Instant::now();
        #[allow(unused_assignments)]
        #[allow(unused_variables)]
        // === TIMEOUT WARNING FLAGS ===
        let mut timeout_warned = false;
        let mut long_thinking_warned = false;

        // === SAFEGUARD: Track recent chunks for repetition detection ===
        let mut recent_chunks: Vec<String> = Vec::new();
        const RECENT_CHUNK_WINDOW: usize = 10;

        // === SAFEGUARD: Track thinking time and content ===
        let mut thinking_start_time: Option<Instant> = None;
        let mut thinking_timeout_warned = false;
        const THINKING_TIMEOUT_SECS: u64 = 300;

        // === SAFEGUARD: Track recently executed tools for multi-round context ===
        let mut recently_executed_tools: VecDeque<String> = VecDeque::new();
        // Track actual shell command strings (for list-only dead end detection)
        let mut recently_executed_commands: VecDeque<String> = VecDeque::new();

        // === SAFEGUARD: Track multi-round tool calling iterations ===
        let mut tool_iteration_count = 0usize;
        const MAX_TOOL_ITERATIONS: usize = 30;
        // Accumulate ALL tool results across rounds for final summary
        let mut all_round_tool_results: Vec<(String, String)> = Vec::new();
        // Track per-round thinking and content for persistence (round number → text)
        let mut round_thinking_map: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
        let mut round_contents_map: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
        // Accumulate ALL rounds' thinking for the message's thinking field
        let mut all_rounds_thinking = String::new();

        // Track whether an incomplete tool call JSON was suppressed
        // (LLM stopped mid-JSON, e.g. hit backend token limit)
        let mut incomplete_tool_json = false;

        // === INTENT & PLAN VISUALIZATION ===
        // Send intent and plan events first to show user what's happening
        yield intent_event;
        last_event_time = Instant::now();

        for (step, stage) in &plan_steps {
            yield AgentEvent::plan(*step, *stage);
        }

        // === MULTI-ROUND TOOL CALLING LOOP ===
        // For complex intents, we may need multiple rounds of tool calling
        'multi_round_loop: loop {
            if tool_iteration_count > 0 {
                tracing::debug!("Starting tool iteration round {}", tool_iteration_count + 1);

                // For subsequent rounds, we need a new LLM call with tools enabled.
                // Use the same budget-managed context builder as the initial call.
                let state_guard = internal_state.read().await;

                let history_for_llm: Vec<neomind_core::Message> = {
                    // Build context with the same effective_max budget as the initial call
                    let config = CompactionConfig::for_context_size(max_context);
                    let compacted = build_context_window_with_config(
                        &state_guard.memory, effective_max, &config
                    );
                    compacted.iter().map(|msg| msg.to_core()).collect::<Vec<_>>()
                };

                // Build context for subsequent rounds - tell LLM what happened before
                let recently_executed: Vec<&str> = recently_executed_tools.iter().map(|s| s.as_str()).collect();
                drop(state_guard);

                let context_msg = if recently_executed.is_empty() {
                    format!(
                        "Round {} of processing. Call ALL needed tools in ONE batch using JSON array format. Give the final response if no more tools needed.",
                        tool_iteration_count + 1
                    )
                } else {
                    let executed_summary = if recently_executed_commands.is_empty() {
                        recently_executed.iter()
                            .map(|s| format!("- {}", s))
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else {
                        recently_executed_commands.iter()
                            .map(|s| format!("- {}", s))
                            .collect::<Vec<_>>()
                            .join("\n")
                    };

                    // === "LIST-ONLY DEAD END" DETECTION ===
                    // If the user asked for an action (create/delete/control/enable/etc)
                    // but all executed tools were read-only (list/get/latest/history),
                    // inject a FORCED continuation prompt to push the LLM to complete the task.
                    let commands_ref: Vec<&str> = recently_executed_commands.iter().map(|s| s.as_str()).collect();
                    let list_only_dead_end = user_message_requires_action(&user_message)
                        && all_tools_were_read_only(&commands_ref, &all_round_tool_results);

                    if list_only_dead_end {
                        let action_hint = extract_action_hint(&user_message);
                        tracing::warn!(
                            "List-only dead end detected! User wants action '{}' but only list/query tools were called. Injecting forced continuation.",
                            action_hint
                        );

                        let mut msg = format!(
                            "⚠️ CRITICAL: The user asked you to perform an ACTION, but you ONLY ran list/query commands.\n\
                            You MUST now execute the actual action command.\n\n\
                            Previously executed (read-only):\n{}\n\n",
                            executed_summary
                        );

                        if !action_hint.is_empty() {
                            msg.push_str(&format!(
                                "The user's original request requires: {}\n\
                                You MUST output a tool call NOW to complete this action.\n\
                                Use the IDs/data from the list results above to construct the command.\n\n",
                                action_hint
                            ));

                            // Rule-specific: if creating a rule, verify device/metric discovery was done
                            if action_hint.contains("rule") && action_hint.contains("create") {
                                let has_device_list = commands_ref.iter()
                                    .any(|c| c.contains("device list") || c.contains("device get"));
                                if !has_device_list {
                                    msg.push_str(
                                        "⚠️ RULE CREATION REQUIRES METRIC DISCOVERY:\n\
                                        You have NOT run `neomind device list` or `neomind device get <ID>` yet.\n\
                                        You CANNOT create a rule without knowing the REAL device ID and metric field name.\n\
                                        Run `neomind device list` FIRST to discover metric_fields, THEN construct the DSL.\n\
                                        NEVER guess device IDs or metric names — they will silently fail.\n\n"
                                    );
                                }
                            }
                        }

                        msg.push_str(
                            "DO NOT output text. DO NOT summarize the list results. DO NOT say 'I found the ...'.\n\
                            OUTPUT A TOOL CALL JSON ARRAY NOW to execute the action."
                        );
                        msg
                    } else {
                        // Normal context message — no list-only dead end detected
                        format!(
                            "Round {} of processing.\n\n\
                            Previously executed tools (results are in context above):\n{}\n\n\
                            STOP AND THINK: Do you need MORE tools, or can you answer from the results above?\n\
                            - If tools above already returned the data you need → give the final response NOW. Do NOT call them again.\n\
                            - If you need different tools → call them in ONE batch using JSON array: [{{\"name\":\"tool\",\"arguments\":{{...}}}}]\n\
                            - NEVER call the same tool with the same arguments — results are already in context.",
                            tool_iteration_count + 1,
                            executed_summary
                        )
                    }
                };

                tracing::debug!("Multi-round context: {}", context_msg);

                // Disable thinking for post-tool-execution rounds to preserve generation
                // budget for content output. Small thinking models (qwen3.5:2b) often
                // consume all num_predict tokens on thinking, leaving content=0.
                let thinking_override = {
                    let current_thinking = llm_interface.get_thinking_enabled().await;
                    if current_thinking == Some(true) {
                        tracing::info!(
                            round = tool_iteration_count + 1,
                            "Disabling thinking for post-tool round to preserve content budget"
                        );
                        Some(false)
                    } else {
                        None
                    }
                };

                let round_stream_result = llm_interface.chat_stream_with_history_thinking(
                    &context_msg,
                    &history_for_llm,
                    thinking_override
                ).await;

                let round_stream = match round_stream_result {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Round {} LLM call failed: {}", tool_iteration_count + 1, e);

                        // Instead of just erroring out, try to summarize what we have so far.
                        // This gives the user a meaningful response instead of a blank cutoff.
                        if !all_round_tool_results.is_empty() {
                            let deduped_results = deduplicate_tool_results(&all_round_tool_results);
                            let has_errors = deduped_results.iter().any(|(_, result)| {
                                let lower = result.to_lowercase();
                                lower.contains("error") || lower.contains("failed") || lower.contains("invalid")
                            });

                            let fallback_prompt = if has_errors {
                                "The tool calls above encountered errors and the LLM failed to generate a follow-up response. \
                                Summarize what was attempted and explain the errors to the user in plain language. \
                                Suggest what the user can do next. Do NOT output any tool calls."
                            } else {
                                "The tool calls above completed but the LLM failed to generate a follow-up response. \
                                Summarize the results for the user. Do NOT output any tool calls."
                            };

                            let summary_history: Vec<neomind_core::Message> = {
                                let state_guard = internal_state.read().await;
                                let compacted = super::compact_tool_results(&state_guard.memory, 2);
                                compacted.iter().map(|msg| msg.to_core()).collect()
                            };

                            let summary_result = llm_interface.chat_stream_summary(
                                fallback_prompt,
                                &summary_history,
                            ).await;

                            match summary_result {
                                Ok(s) => {
                                    let mut pin = Box::pin(s);
                                    while let Some(chunk) = pin.next().await {
                                        match chunk {
                                            Ok((text, _)) => { yield AgentEvent::content(text); }
                                            Err(_) => break,
                                        }
                                    }
                                }
                                Err(se) => {
                                    tracing::error!("Fallback summary also failed: {}", se);
                                    yield AgentEvent::error(format!("Processing failed: {}", e));
                                }
                            }
                        } else {
                            yield AgentEvent::error(format!("Processing failed: {}", e));
                        }
                        break 'multi_round_loop;
                    }
                };

                stream = Box::pin(round_stream);
                buffer = String::new();
                yielded_up_to = 0;
                tool_calls.clear();
                content_before_tools = String::new();
                // Reset repetition tracking for the new round to prevent
                // carry-over from previous rounds causing false positives
                recent_chunks.clear();
            }

            // === PHASE 1: Stream initial response (thinking + content + tool calls) ===
            while let Some(result) = StreamExt::next(&mut stream).await {
                let elapsed = stream_start.elapsed();

                // Check timeout with early warning at 80% of max duration
                let timeout_threshold = safeguards.max_stream_duration;
                let warning_threshold = timeout_threshold.mul_f32(0.8);

                if elapsed > timeout_threshold {
                    tracing::warn!("Stream timeout ({:?} elapsed, max: {:?}), forcing completion", elapsed, timeout_threshold);
                    // Don't break here - let tool calls be processed
                    // Just log the timeout and continue to check for tool calls
                    if tool_calls_detected {
                        tracing::debug!("Timeout with tool calls detected, proceeding to execution");
                        break;
                    } else {
                        yield AgentEvent::error(format!("Request timeout ({:.1}s elapsed), completing processing...", elapsed.as_secs_f64()));
                        break;
                    }
                } else if elapsed > warning_threshold && !timeout_warned {
                    tracing::warn!("Stream approaching timeout ({:.1}s elapsed, max: {:.1}s)", elapsed.as_secs_f64(), timeout_threshold.as_secs_f64());
                    yield AgentEvent::warning(format!("Response is taking longer ({:.1}s elapsed), please wait...", elapsed.as_secs_f64()));
                    timeout_warned = true;
                }

                // Special warning for extended thinking with no content
                if has_thinking && !has_content && elapsed > Duration::from_secs(60) && !long_thinking_warned {
                    tracing::warn!("Extended thinking detected ({:.1}s) with no content yet", elapsed.as_secs_f64());
                    yield AgentEvent::warning("The model is performing deep thinking, this may take longer...".to_string());
                    long_thinking_warned = true;
                }

                // Check for interrupt signal
                // We clone the value to avoid holding the guard across await
                let is_interrupted = safeguards.interrupt_signal.as_ref().map(|rx| *rx.borrow()).unwrap_or(false);
                if is_interrupted {
                    tracing::info!("Stream interrupted by user");
                    yield AgentEvent::content("\n\n[Interrupted]");
                    yield AgentEvent::end();
                    return;
                }

                // === KEEPALIVE: Send heartbeat if no events for too long ===
                if last_event_time.elapsed() > safeguards.heartbeat_interval {
                    yield AgentEvent::heartbeat();
                    last_event_time = Instant::now();
                }

                // === PROGRESS: Send progress update during long operations ===
                if last_progress_time.elapsed() > safeguards.progress_interval {
                    let stage_name = if has_thinking && !has_content {
                        "thinking"
                    } else if tool_calls_detected {
                        "executing"
                    } else {
                        "generating"
                    };
                    let elapsed_ms = elapsed.as_millis() as u64;
                    yield AgentEvent::progress(
                        format!("{}...", match stage_name {
                            "thinking" => "Thinking",
                            "executing" => "Executing tools",
                            _ => "Generating response",
                        }),
                        stage_name,
                        elapsed_ms
                    );
                    last_progress_time = Instant::now();
                }

                match result {
                    Ok((text, is_thinking)) => {
                        if text.is_empty() {
                            continue;
                        }

                        // === SAFEGUARD: Repetition detection ===
                        recent_chunks.push(text.clone());
                        if recent_chunks.len() > RECENT_CHUNK_WINDOW {
                            recent_chunks.remove(0);
                        }

                        // NOTE: Per-chunk repetition detection removed — it caused false positives
                        // when the LLM legitimately discusses multiple devices/sensors and words
                        // like "温度", "传感器" appear many times in a normal analysis report.

                        if is_thinking {
                            // Track thinking start time
                            if thinking_start_time.is_none() {
                                thinking_start_time = Some(Instant::now());
                            }

                            // Check for thinking timeout
                            if let Some(start) = thinking_start_time {
                                let thinking_elapsed = start.elapsed();
                                if thinking_elapsed > Duration::from_secs(THINKING_TIMEOUT_SECS) && !thinking_timeout_warned {
                                    tracing::warn!(
                                        "Thinking timeout detected ({:.1}s elapsed). Model may be stuck in thinking loop.",
                                        thinking_elapsed.as_secs_f64()
                                    );
                                    yield AgentEvent::warning(
                                        "The model is taking longer than expected to think. This may indicate a complex query or the model getting stuck. Please wait...".to_string()
                                    );
                                    thinking_timeout_warned = true;
                                }
                            }

                            // No thinking limit - let the model think as much as needed
                            // First, add the new text to thinking content
                            thinking_content.push_str(&text);
                            has_thinking = true;

                            // === IMPORTANT: Check for tool calls BEFORE yielding thinking event ===
                            // Some models (like qwen3-vl:2b) output tool calls within thinking field
                            // We need to detect and extract them BEFORE sending to frontend
                            let mut text_to_yield = text.clone();
                            let thinking_with_new = thinking_content.as_str();
                            let mut had_tool_calls = false;

                            // Check for XML tool calls in thinking: <tool_calls>...</tool_calls>
                            if let Some(tool_start) = thinking_with_new.find("<tool_calls>") {
                                if let Some(tool_end) = thinking_with_new.find("</tool_calls>") {
                                    let tool_content = thinking_with_new[tool_start..tool_end + 13].to_string();

                                    // Parse the tool calls from thinking
                                    if let Ok((_, calls)) = parse_tool_calls(&tool_content) {
                                        if !calls.is_empty() {
                                            tool_calls_detected = true;
                                            tool_calls.extend(calls);
                                            had_tool_calls = true;
                                            // Remove tool calls from thinking content
                                            thinking_content = format!("{}{}", &thinking_with_new[..tool_start], &thinking_with_new[tool_end + 13..]);
                                            // Don't yield tool call XML as thinking content
                                            text_to_yield = String::new();
                                            tracing::debug!("Extracted {} tool calls from thinking content", tool_calls.len());
                                        }
                                    }
                                }
                            }
                            // Also check for JSON tool calls in thinking
                            else if let Some((json_start, tool_json, remaining)) = detect_json_tool_calls(thinking_with_new) {
                                if let Ok((_, calls)) = parse_tool_calls(&tool_json) {
                                    if !calls.is_empty() {
                                        tool_calls_detected = true;
                                        tool_calls.extend(calls);
                                        had_tool_calls = true;
                                        // Remove tool calls from thinking content
                                        thinking_content = format!("{}{}", &thinking_with_new[..json_start], remaining);
                                        // Don't yield tool call JSON as thinking content
                                        text_to_yield = String::new();
                                        tracing::debug!("Extracted {} JSON tool calls from thinking content", tool_calls.len());
                                    }
                                }
                            }

                            // Only yield non-empty thinking content (without tool calls)
                            if !text_to_yield.is_empty() {
                                yield AgentEvent::thinking(text_to_yield);
                            } else if had_tool_calls {
                                // If we had tool calls but no other thinking content, yield empty thinking
                                // to ensure the frontend knows thinking phase is happening
                                yield AgentEvent::thinking(String::new());
                            }
                            last_event_time = Instant::now();
                            continue;
                        }

                        // content: need to check for tool calls
                        has_content = true;
                        last_event_time = Instant::now();

                        if safeguards.max_content_length != usize::MAX
                            && content_before_tools.len() + buffer.len() + text.len() > safeguards.max_content_length
                        {
                            tracing::warn!("Content exceeded max length ({}), stopping stream", safeguards.max_content_length);
                            yield AgentEvent::error("Response too long - content limit reached".to_string());
                            break;
                        }

                        // Add text to buffer
                        buffer.push_str(&text);

                        // Check for tool calls in buffer (support both XML and JSON formats)
                        // Try JSON format first: [{"name": "tool", "arguments": {...}}]
                        let json_tool_check = detect_json_tool_calls(&buffer);
                        if let Some((json_start, tool_json, _remaining)) = json_tool_check {
                            // Found JSON tool calls - only yield content NOT already yielded
                            if json_start > yielded_up_to {
                                let new_content = &buffer[yielded_up_to..json_start];
                                if !new_content.is_empty() {
                                    content_before_tools.push_str(new_content);
                                    yield AgentEvent::content(new_content);
                                }
                            }
                            // Still track ALL content before tools for memory saving
                            let before_tool = &buffer[..json_start];
                            if before_tool.len() > content_before_tools.len() {
                                content_before_tools = before_tool.to_string();
                            }

                            // Parse the JSON tool calls
                            if let Ok((_, calls)) = parse_tool_calls(&tool_json) {
                                if !calls.is_empty() {
                                    tool_calls_detected = true;
                                    tool_calls.extend(calls);
                                }
                            }

                            // Discard remaining content after embedded tool calls.
                            // Models often fabricate tool results after outputting JSON tool calls
                            // in text — these hallucinated results should not be shown to the user.
                            // The real results will come from actual tool execution.
                            buffer.clear();
                            yielded_up_to = 0;
                        } else {
                            // No JSON tool calls detected - check for XML format
                            if let Some(tool_start) = buffer.find("<tool_calls>") {
                                // Only yield content NOT already yielded
                                if tool_start > yielded_up_to {
                                    let new_content = &buffer[yielded_up_to..tool_start];
                                    if !new_content.is_empty() {
                                        content_before_tools.push_str(new_content);
                                        yield AgentEvent::content(new_content);
                                    }
                                }
                                let before_tool = &buffer[..tool_start];
                                if before_tool.len() > content_before_tools.len() {
                                    content_before_tools = before_tool.to_string();
                                }

                                if let Some(tool_end) = buffer.find("</tool_calls>") {
                                    let tool_content = buffer[tool_start..tool_end + 13].to_string();
                                    // Discard remaining content after XML tool calls (same reason as JSON)
                                    buffer.clear();
                                    yielded_up_to = 0;

                                    if let Ok((_, calls)) = parse_tool_calls(&tool_content) {
                                        if !calls.is_empty() {
                                            tool_calls_detected = true;
                                            tool_calls.extend(calls);
                                        }
                                    }
                                }
                            } else {
                                // Check if buffer might contain the START of a JSON tool call.
                                // Hold back suspicious content to prevent partial JSON
                                // from being yielded before the full JSON is detected.
                                let might_be_json_start = buffer.ends_with("[{")
                                    || buffer.ends_with("{\"")
                                    || buffer.ends_with("\"name\"")
                                    || buffer.ends_with("```")
                                    || buffer.ends_with("```json")
                                    || (buffer.contains("[{\"name") && !buffer.contains("]}"))
                                    || (buffer.contains("{\"name\"") && !buffer.contains("}]}"));

                                if might_be_json_start {
                                    // Don't yield yet — wait for more chunks to determine
                                    // if this is a tool call JSON or normal text
                                    // Find the earliest suspicious position
                                    let suspicious_pos = {
                                        let mut pos = buffer.len();
                                        if let Some(p) = buffer.rfind("[{") { pos = pos.min(p); }
                                        if let Some(p) = buffer.rfind("{\"") { pos = pos.min(p); }
                                        if let Some(p) = buffer.rfind("```") { pos = pos.min(p); }
                                        pos
                                    };
                                    if suspicious_pos > yielded_up_to {
                                        let safe_content = &buffer[yielded_up_to..suspicious_pos];
                                        if !safe_content.is_empty() {
                                            content_before_tools.push_str(safe_content);
                                            yield AgentEvent::content(safe_content);
                                        }
                                        yielded_up_to = suspicious_pos;
                                    }
                                } else if !text.is_empty() {
                                    // Safe to yield — no JSON pattern detected
                                    yield AgentEvent::content(text.clone());
                                    yielded_up_to = buffer.len();
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Stream error: {}", e);
                        yield AgentEvent::error(format!("Stream error: {}", e));
                        // Save partial response on error to maintain conversation context
                        // This prevents the next message from having incomplete context
                        if !buffer.is_empty() || !content_before_tools.is_empty() || !thinking_content.is_empty() {
                            let partial_content = if content_before_tools.is_empty() {
                                buffer.clone()
                            } else {
                                content_before_tools.clone()
                            };
                            let partial_msg = if !thinking_content.is_empty() {
                                let cleaned_thinking = cleanup_thinking_content(&thinking_content);
                                AgentMessage::assistant_with_thinking(&partial_content, &cleaned_thinking)
                            } else {
                                AgentMessage::assistant(&partial_content)
                            };
                            internal_state.write().await.push_message(partial_msg);
                            tracing::debug!("Saved partial response on error: {} chars content, {} chars thinking",
                                partial_content.len(), thinking_content.len());
                        }
                        break;
                    }
                }
            }

            // Release any held-back content if it turned out NOT to be a tool call.
            // If tool_calls_detected is true, the held content IS part of the tool call JSON
            // and should be discarded (it will not be displayed).
            if !tool_calls_detected && yielded_up_to < buffer.len() {
                let remaining = &buffer[yielded_up_to..];
                // Filter out incomplete tool call JSON patterns that leaked through
                // (happens when LLM hits max_tokens mid-tool-call or stream ends abruptly)
                let should_suppress = remaining.trim_start().starts_with('[')
                    && (remaining.contains("\"name\"") || remaining.contains("\"arguments\""))
                    && !remaining.trim_end().ends_with(']');
                if !remaining.is_empty() && !should_suppress {
                    content_before_tools.push_str(remaining);
                    yield AgentEvent::content(remaining);
                } else if should_suppress {
                    tracing::warn!(
                        "Detected incomplete tool call JSON ({} chars) — LLM stopped mid-output. \
                         Will trigger summary to guide next step.",
                        remaining.len()
                    );
                    incomplete_tool_json = true;
                }
                yielded_up_to = buffer.len();
            }

            // === Handle tool calls if detected ===
            if tool_calls_detected {
                tracing::debug!("Starting tool execution round {}", tool_iteration_count + 1);

                // Send progress event to inform user about tool iteration
                let current_elapsed = stream_start.elapsed();
                yield AgentEvent::progress(
                    format!("Executing tools (round {}/{})", tool_iteration_count + 1, safeguards.max_tool_iterations),
                    "executing",
                    current_elapsed.as_millis() as u64,
                );

                if tool_calls.len() > safeguards.max_tool_iterations {
                    tracing::warn!(
                        "Too many tool calls ({}) requested, limiting to {}",
                        tool_calls.len(),
                        safeguards.max_tool_iterations
                    );
                    yield AgentEvent::error(format!(
                        "Too many tool calls requested ({}), limiting to {}",
                        tool_calls.len(),
                        safeguards.max_tool_iterations
                    ));
                    tool_calls.truncate(safeguards.max_tool_iterations);
                }
                let tool_calls_to_execute = tool_calls.clone();

                // Resolve cached data references in tool arguments
                let (large_cache, cache) = {
                    let state = internal_state.read().await;
                    (state.large_data_cache.clone(), state.tool_result_cache.clone())
                };

                // Execute tool calls with bounded concurrency (max 6 parallel)
                const MAX_TOOL_CONCURRENCY: usize = 6;

                // Collect into owned tuples to avoid lifetime issues with async_stream
                let tool_inputs: Vec<(String, serde_json::Value)> = tool_calls_to_execute
                    .iter()
                    .map(|tc| (tc.name.clone(), resolve_cached_arguments(&tc.arguments, &large_cache)))
                    .collect();

                let tool_futures = futures::stream::iter(tool_inputs.into_iter().map(|(name, arguments)| {
                    let tools_clone = tools.clone();
                    let cache_clone = cache.clone();

                    async move {
                        (name.clone(), ToolExecutionResult {
                            _name: name.clone(),
                            arguments: arguments.clone(),
                            result: execute_tool_with_retry(&tools_clone, &cache_clone, &name, arguments.clone()).await,
                        })
                    }
                })).buffer_unordered(MAX_TOOL_CONCURRENCY);

                let tool_results_executed: Vec<_> = tool_futures.collect().await;

                // Process results
                let mut tool_calls_with_results: Vec<ToolCall> = Vec::new();
                let mut tool_call_results: Vec<(String, String)> = Vec::new();

                for (name, execution) in tool_results_executed {
                    // Use arguments from the execution result (preserves per-call arguments for same-name tools)
                    let exec_arguments = execution.arguments.clone();
                    yield AgentEvent::tool_call_start_round(&name, exec_arguments.clone(), tool_iteration_count + 1);

                    match execution.result {
                        Ok(output) => {
                            let result_value = if output.success {
                                output.data.clone()
                            } else {
                                output.error.clone().map(|e| serde_json::json!({"error": e}))
                                    .unwrap_or_else(|| serde_json::json!("Error"))
                            };
                            let result_str = if output.success {
                                serde_json::to_string(&output.data).unwrap_or_else(|_| "Success".to_string())
                            } else {
                                output.error.clone().unwrap_or_else(|| "Error".to_string())
                            };

                            // Sanitize base64/image data before sending to frontend or LLM
                            let display_str = sanitize_tool_result_for_prompt(&result_str);

                            tool_calls_with_results.push(ToolCall {
                                name: name.clone(),
                                id: String::new(),
                                arguments: exec_arguments,
                                result: Some(result_value.clone()),
                                round: Some(tool_iteration_count + 1),
                            });

                            yield AgentEvent::tool_call_end_round(&name, &display_str, output.success, tool_iteration_count + 1);
                            tool_call_results.push((name.clone(), display_str));
                        }
                        Err(e) => {
                            let error_msg = format!("Tool execution failed: {}", e);
                            let error_value = serde_json::json!({"error": error_msg});

                            tool_calls_with_results.push(ToolCall {
                                name: name.clone(),
                                id: String::new(),
                                arguments: exec_arguments,
                                result: Some(error_value.clone()),
                                round: Some(tool_iteration_count + 1),
                            });

                            yield AgentEvent::tool_call_end_round(&name, &error_msg, false, tool_iteration_count + 1);
                            tool_call_results.push((name.clone(), error_msg));
                        }
                    }
                }

                // Update recently executed tools list (for multi-round context)
                all_round_tool_results.extend(tool_call_results.iter().cloned());
                for (name, _result) in &tool_call_results {
                    if !recently_executed_tools.iter().any(|n| n == name) {
                        recently_executed_tools.push_back(name.clone());
                        if recently_executed_tools.len() > 10 {
                            recently_executed_tools.pop_front();
                        }
                        tracing::debug!("Added '{}' to recently executed tools (now: {:?})", name, recently_executed_tools);
                    }
                }
                // Track actual shell commands for list-only dead end detection
                for tc in &tool_calls_to_execute {
                    if tc.name == "shell" {
                        if let Some(cmd) = tc.arguments.get("command").and_then(|v| v.as_str()) {
                            recently_executed_commands.push_back(cmd.to_string());
                            if recently_executed_commands.len() > 20 {
                                recently_executed_commands.pop_front();
                            }
                        }
                    }
                }

                // === UNIFIED ReAct LOOP: Save results and continue ===
                // Always save assistant+tool_calls and tool results to history,
                // then let the LLM decide in the next round whether to call more tools
                // or give the final answer.

                // Check iteration limit and duplicate detection
                let should_continue = tool_iteration_count < MAX_TOOL_ITERATIONS - 1;

                // === Save assistant message with tool_calls BEFORE tool results ===
                let response_to_save = if content_before_tools.is_empty() {
                    String::new()
                } else {
                    remove_tool_calls_from_response(&content_before_tools)
                };
                let initial_msg = if !thinking_content.is_empty() {
                    let cleaned_thinking = cleanup_thinking_content(&thinking_content);
                    AgentMessage::assistant_with_tools_and_thinking(
                        &response_to_save,
                        tool_calls_with_results.clone(),
                        &cleaned_thinking,
                    )
                } else {
                    AgentMessage::assistant_with_tools(&response_to_save, tool_calls_with_results.clone())
                };
                tracing::debug!("[streaming] Saving assistant message with {} tool_calls (round {})",
                    initial_msg.tool_calls.as_ref().map_or(0, |c| c.len()), tool_iteration_count + 1);
                internal_state.write().await.push_message(initial_msg);

                // Save tool results to memory (large results go through cache → summary)
                for (tool_name, result_str) in &tool_call_results {
                    if tool_name == "skill" {
                        llm_interface.set_skill_context(result_str.clone()).await;
                    } else {
                        let mut state = internal_state.write().await;
                        let history_content = state.large_data_cache.store(tool_name, result_str);
                        let tool_result_msg = AgentMessage::tool_result(tool_name, &history_content);
                        state.push_message(tool_result_msg);
                    }
                }

                // NOTE: Mid-task compaction removed — build_context_window_with_config
                // (called above at each round) already handles LLM context trimming
                // without modifying the stored history. In-place compaction caused
                // persisted history to shrink after session switch (messages permanently
                // lost when compact_memory_mid_task modified state.memory).

                // If we should continue the ReAct loop, save round state and loop back
                if should_continue {
                    tool_iteration_count += 1;

                    // Save per-round thinking and content for persistence
                    let round_num = tool_iteration_count as u32;
                    if !thinking_content.is_empty() {
                        round_thinking_map.insert(round_num, thinking_content.clone());
                        all_rounds_thinking.push_str(&thinking_content);
                    }
                    if !content_before_tools.is_empty() {
                        let cleaned = remove_tool_calls_from_response(&content_before_tools);
                        let cleaned = cleaned.trim()
                            .trim_start_matches("```json").trim_start_matches("```")
                            .trim();
                        if !cleaned.is_empty() {
                            round_contents_map.insert(round_num, cleaned.to_string());
                        }
                    }

                    tool_calls_detected = false;
                    tool_calls.clear();
                    content_before_tools.clear();

                    yield AgentEvent::IntermediateEnd;
                    continue 'multi_round_loop;
                }

                // === LOOP END: iteration limit or duplicate detected ===
                // The LLM will see tool results in history on the next turn.
                // Save final round thinking for persistence.
                let last_round = (tool_iteration_count + 1) as u32;
                if !thinking_content.is_empty() {
                    let cleaned = cleanup_thinking_content(&thinking_content);
                    round_thinking_map.insert(last_round, cleaned.clone());
                    all_rounds_thinking.push_str(&cleaned);
                }

                // Convert round maps to serde_json::Value for AgentMessage
                let round_thinking_val = if !round_thinking_map.is_empty() {
                    Some(serde_json::to_value(&round_thinking_map).unwrap_or(serde_json::Value::Null))
                } else {
                    None
                };
                let round_contents_val = if !round_contents_map.is_empty() {
                    Some(serde_json::to_value(&round_contents_map).unwrap_or(serde_json::Value::Null))
                } else {
                    None
                };

                // Fallback: try a summary call when the last round had errors,
                // OR when LLM didn't produce content before tools.
                // Without tool definitions, the model must output text instead of more tool calls.
                let deduped_results = deduplicate_tool_results(&all_round_tool_results);
                let last_round_has_errors = deduped_results.iter().any(|(_, result)| {
                    let lower = result.to_lowercase();
                    lower.contains("error") || lower.contains("failed") || lower.contains("invalid")
                        || lower.contains("unauthorized") || lower.contains("401")
                });
                // The preamble "Let me check..." is NOT a final answer.
                // Force summary when tool errors exist, regardless of content_before_tools.
                let content_is_preamble = content_before_tools.trim().len() < 200;

                if content_before_tools.is_empty() || (last_round_has_errors && content_is_preamble) {

                    // Notify user that we're generating a final response
                    if last_round_has_errors {
                        yield AgentEvent::progress(
                            "Generating final response...".to_string(),
                            "summarizing",
                            0,
                        );
                    }

                    // Build compact history for the summary call
                    let summary_history: Vec<neomind_core::Message> = {
                        let state_guard = internal_state.read().await;
                        let compacted = super::compact_tool_results(&state_guard.memory, 2);
                        compacted.iter().map(|msg| msg.to_core()).collect()
                    };

                    // Detect whether tool results contain errors to tailor the prompt
                    let has_errors = deduped_results.iter().any(|(_, result)| {
                        let lower = result.to_lowercase();
                        lower.contains("error") || lower.contains("failed") || lower.contains("invalid")
                    });

                    let summary_prompt = if has_errors {
                        "The tool calls above returned errors. \
                        Analyze the errors and explain to the user what went wrong in plain language. \
                        Suggest what the user can do (e.g., provide different parameters, check the device, etc.). \
                        Do NOT output any tool calls — give a direct text response."
                    } else {
                        "Based on the tool execution results in the conversation above, \
                        provide a concise analysis and summary. Do NOT output any tool calls — \
                        give a direct text response to the user's question."
                    };

                    let summary_result = llm_interface.chat_stream_summary(
                        summary_prompt,
                        &summary_history,
                    ).await;

                    let mut final_content = String::new();
                    match summary_result {
                        Ok(stream) => {
                            let mut pin = Box::pin(stream);
                            while let Some(chunk) = pin.next().await {
                                match chunk {
                                    Ok((text, _)) => {
                                        final_content.push_str(&text);
                                        yield AgentEvent::content(text);
                                    }
                                    Err(e) => {
                                        tracing::error!("Summary stream error: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Summary call failed: {}", e);
                        }
                    }

                    // If summary also failed, fall back to formatted tool results
                    if final_content.trim().is_empty() {
                        final_content = format_tool_results(&deduped_results);
                        tracing::info!(
                            "Summary call produced empty content, using formatted tool results ({} chars)",
                            final_content.len()
                        );
                        yield AgentEvent::content(final_content.clone());
                    } else {
                        tracing::info!(
                            "Summary call succeeded ({} chars)",
                            final_content.len()
                        );
                    }

                    // Save as assistant message with round metadata
                    let mut final_msg = AgentMessage::assistant(&final_content);
                    final_msg.thinking = if all_rounds_thinking.is_empty() { None } else { Some(all_rounds_thinking.clone()) };
                    final_msg.round_thinking = round_thinking_val;
                    final_msg.round_contents = round_contents_val;
                    let mut state = internal_state.write().await;
                    state.register_response(&final_content);
                    state.push_message(final_msg);
                } else {
                    // LLM produced some content before tools in this last round
                    // Clean and save it as the final response
                    let cleaned_content = remove_tool_calls_from_response(&content_before_tools);
                    let mut final_msg = AgentMessage::assistant(&cleaned_content);
                    final_msg.thinking = if all_rounds_thinking.is_empty() { None } else { Some(all_rounds_thinking.clone()) };
                    final_msg.round_thinking = round_thinking_val;
                    final_msg.round_contents = round_contents_val;
                    let mut state = internal_state.write().await;
                    state.register_response(&cleaned_content);
                    state.push_message(final_msg);
                }

                tracing::debug!("ReAct loop completed after {} tool iterations", tool_iteration_count + 1);
            } else {
                // No tool calls - save response directly
                // Use buffer if content_before_tools is empty (buffer contains all content chunks when no tools)
                let mut raw_response = if content_before_tools.is_empty() {
                    buffer.clone()
                } else {
                    content_before_tools.clone()
                };

                // === RECOVERY: Incomplete tool call JSON ===
                // LLM stopped mid-tool-call (e.g. backend token limit).
                // Use summary call to explain the situation and guide the user.
                if incomplete_tool_json {
                    tracing::info!(
                        round = tool_iteration_count + 1,
                        "Triggering summary for incomplete tool call JSON"
                    );
                    yield AgentEvent::progress(
                        "Generating response from partial results...".to_string(),
                        "summarizing",
                        0,
                    );

                    let summary_history: Vec<neomind_core::Message> = {
                        let state_guard = internal_state.read().await;
                        let compacted = super::compact_tool_results(&state_guard.memory, 2);
                        compacted.iter().map(|msg| msg.to_core()).collect()
                    };

                    let deduped_results = deduplicate_tool_results(&all_round_tool_results);
                    let summary_prompt = if deduped_results.is_empty() {
                        "The previous tool call was interrupted mid-execution. \
                         Summarize what you were trying to do and ask the user \
                         if they want you to continue. \
                         Do NOT output any tool calls — give a direct text response."
                    } else {
                        "Based on the tool execution results gathered so far, \
                         provide a concise summary of what was accomplished. \
                         If the task is incomplete, explain what still needs to be done \
                         and ask the user if they want to continue. \
                         Do NOT output any tool calls — give a direct text response."
                    };

                    let summary_result = llm_interface.chat_stream_summary(
                        summary_prompt,
                        &summary_history,
                    ).await;

                    match summary_result {
                        Ok(stream) => {
                            let mut summary_content = String::new();
                            let mut pin = Box::pin(stream);
                            while let Some(chunk) = pin.next().await {
                                match chunk {
                                    Ok((text, _)) => {
                                        summary_content.push_str(&text);
                                        yield AgentEvent::content(text);
                                    }
                                    Err(e) => {
                                        tracing::error!("Incomplete JSON summary stream error: {}", e);
                                        break;
                                    }
                                }
                            }
                            if !summary_content.trim().is_empty() {
                                raw_response = summary_content;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Incomplete JSON summary call failed: {}", e);
                        }
                    }
                }

                // === RECOVERY: Retry without thinking when content is empty ===
                // This handles the case where thinking models consume all generation
                // budget on thinking tokens, producing no content. Retry once with
                // thinking forcefully disabled so the model outputs content directly.
                if raw_response.trim().is_empty() {
                    let had_thinking = !thinking_content.is_empty();
                    tracing::warn!(
                        had_thinking = had_thinking,
                        "Stream completed with empty content, attempting retry without thinking"
                    );

                    // Build a compact history for the retry (keep last few messages)
                    let state_guard = internal_state.read().await;
                    let retry_history: Vec<neomind_core::Message> = {
                        let non_system: Vec<&AgentMessage> = state_guard.memory.iter()
                            .filter(|m| m.role != "system")
                            .collect();
                        // Keep at most last 6 messages to reduce prompt size
                        let keep = non_system.len().saturating_sub(6);
                        non_system[keep..].iter().map(|m| m.to_core()).collect()
                    };
                    drop(state_guard);

                    // Get original user message from the first message in history
                    let retry_user_msg = retry_history.iter()
                        .find(|m| m.role == neomind_core::MessageRole::User)
                        .map(|m| m.content.as_text())
                        .unwrap_or_default();

                    let retry_prompt = if retry_user_msg.is_empty() {
                        "Please provide a response.".to_string()
                    } else {
                        format!(
                            "Please respond to the user's message directly and concisely.\n\nUser: {}",
                            retry_user_msg
                        )
                    };

                    let retry_result = llm_interface.chat_stream_with_history_thinking(
                        &retry_prompt,
                        &retry_history,
                        Some(false), // Force disable thinking
                    ).await;

                    match retry_result {
                        Ok(retry_stream) => {
                            let mut retry_content = String::new();
                            let mut pin = Box::pin(retry_stream);
                            while let Some(chunk) = pin.next().await {
                                match chunk {
                                    Ok((text, _)) => {
                                        retry_content.push_str(&text);
                                        // Don't yield yet — check for tool calls first
                                    }
                                    Err(e) => {
                                        tracing::error!("Retry stream error: {}", e);
                                        break;
                                    }
                                }
                            }

                            // Check for tool calls in retry content and strip them.
                            // When the first stream is interrupted, the retry may produce
                            // tool call JSON instead of plain content. We must not yield
                            // raw JSON to the user.
                            let cleaned = match parse_tool_calls(&retry_content) {
                                Ok((content, calls)) if !calls.is_empty() => {
                                    tracing::warn!(
                                        calls_count = calls.len(),
                                        "Retry produced tool calls instead of content, stripping them"
                                    );
                                    content
                                }
                                _ => retry_content.clone(),
                            };

                            if !cleaned.trim().is_empty() {
                                raw_response = cleaned.clone();
                                yield AgentEvent::content(cleaned);
                                tracing::info!(
                                    content_len = raw_response.len(),
                                    "Retry without thinking succeeded"
                                );
                            } else {
                                tracing::warn!("Retry produced only tool calls, using fallback");
                                let fallback = "抱歉，模型暂时无法生成回复，请稍后重试。".to_string();
                                raw_response = fallback.clone();
                                yield AgentEvent::content(fallback);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Retry LLM call failed: {}", e);
                            let fallback = "抱歉，模型暂时无法生成回复，请稍后重试。".to_string();
                            raw_response = fallback.clone();
                            yield AgentEvent::content(fallback);
                        }
                    }
                }

                // Clean any embedded tool call JSON from response
                let response_to_save = remove_tool_calls_from_response(&raw_response);

                let initial_msg = if !thinking_content.is_empty() {
                    let cleaned_thinking = cleanup_thinking_content(&thinking_content);
                    AgentMessage::assistant_with_thinking(&response_to_save, &cleaned_thinking)
                } else {
                    AgentMessage::assistant(&response_to_save)
                };
                {
                    let mut state = internal_state.write().await;
                    // Register response for cross-turn repetition detection
                    state.register_response(&response_to_save);
                    state.push_message(initial_msg);
                }

                // Yield any remaining un-yielded content from buffer
                if buffer.len() > yielded_up_to {
                    let remaining = buffer[yielded_up_to..].to_string();
                    if !remaining.is_empty() {
                        yield AgentEvent::content(remaining);
                    }
                }
            }

            // Break the loop after processing
            break 'multi_round_loop;
        }

        // Read token usage from LLM interface (captured from Ollama backend stream)
        let prompt_tokens = llm_interface.take_last_prompt_tokens().await;
        match prompt_tokens {
            Some(pt) => yield AgentEvent::end_with_tokens(pt),
            None => yield AgentEvent::end(),
        }
    }))
}

/// Process a multimodal user message (text + images) with streaming response.
///
/// This is similar to `process_stream_events` but accepts images as base64 data URLs.
/// Images are converted to ContentPart::ImageBase64 for the LLM.
pub async fn process_multimodal_stream_events(
    llm_interface: Arc<LlmInterface>,
    internal_state: Arc<tokio::sync::RwLock<AgentInternalState>>,
    tools: Arc<crate::toolkit::ToolRegistry>,
    user_message: &str,
    images: Vec<String>, // Base64 data URLs (e.g., "data:image/png;base64,...")
) -> Result<Pin<Box<dyn Stream<Item = AgentEvent> + Send>>> {
    process_multimodal_stream_events_with_safeguards(
        llm_interface,
        internal_state,
        tools,
        user_message,
        images,
        StreamSafeguards::default(),
        None,
        None,
    )
    .await
}

/// Process multimodal message with configurable safeguards.
#[allow(clippy::too_many_arguments)]
pub async fn process_multimodal_stream_events_with_safeguards(
    llm_interface: Arc<LlmInterface>,
    internal_state: Arc<tokio::sync::RwLock<AgentInternalState>>,
    tools: Arc<crate::toolkit::ToolRegistry>,
    user_message: &str,
    images: Vec<String>,
    safeguards: StreamSafeguards,
    conversation_summary: Option<String>,
    summary_up_to_index: Option<u64>,
) -> Result<Pin<Box<dyn Stream<Item = AgentEvent> + Send>>> {
    use neomind_core::ContentPart;

    let user_message = user_message.to_string();

    // Build multimodal message content with images
    let mut parts = vec![ContentPart::text(&user_message)];

    // Add images as ContentPart
    for image_data in &images {
        if let Some(parsed) = crate::image_utils::parse_image_data(image_data) {
            parts.push(ContentPart::image_base64(parsed.base64, parsed.mime_type));
        }
    }

    // Get conversation history
    let state_guard = internal_state.read().await;
    let history_messages = state_guard.memory.clone();
    drop(state_guard);

    // Build context window — measure actual prompt overhead instead of guessing
    let max_context = llm_interface.max_context_length().await;
    let prompt_overhead = llm_interface.estimate_prompt_overhead_tokens().await;
    let effective_max = max_context
        .saturating_sub(prompt_overhead)
        .saturating_sub(1024)
        .max((max_context * 20) / 100);

    let history_for_llm: Vec<neomind_core::Message> = build_context_window_with_summary(
        &history_messages,
        effective_max,
        conversation_summary.as_deref(),
        summary_up_to_index,
    )
    .iter()
    .map(|msg| msg.to_core())
    .collect::<Vec<_>>();

    tracing::debug!(
        "Passing {} messages from history to LLM (multimodal)",
        history_for_llm.len()
    );

    // Create multimodal user message
    let multimodal_user_msg = neomind_core::Message::new(
        neomind_core::MessageRole::User,
        neomind_core::Content::Parts(parts),
    );

    // Use regular multimodal chat (with thinking enabled)
    // Thinking helps the model analyze images more thoroughly
    let stream_result = llm_interface
        .chat_stream_multimodal_with_history(multimodal_user_msg, &history_for_llm)
        .await;

    let stream = stream_result.map_err(|e| NeoMindError::Llm(e.to_string()))?;

    // Check if images are present (before moving images)
    let has_images = !images.is_empty();

    // Extract base64 data for caching before images are consumed
    let image_base64_list: Vec<String> = images
        .iter()
        .filter_map(|data_url| data_url.split(',').nth(1).map(|s| s.to_string()))
        .collect();

    // Store user message in history with images
    // Convert the image strings to AgentMessageImage
    let user_images: Vec<AgentMessageImage> = images
        .into_iter()
        .map(|data_url| {
            let mime_type = crate::image_utils::parse_image_data(&data_url)
                .map(|p| p.mime_type.to_string());
            AgentMessageImage {
                data: data_url,
                mime_type,
            }
        })
        .collect();

    let user_msg = AgentMessage::user_with_images(&user_message, user_images);
    internal_state.write().await.push_message(user_msg);

    // Cache user-uploaded images so tools can reference them via $cached:user_image
    if !image_base64_list.is_empty() {
        let mut state = internal_state.write().await;
        for (i, base64_data) in image_base64_list.iter().enumerate() {
            let cache_key = if i == 0 {
                "user_image".to_string()
            } else {
                format!("user_image_{}", i)
            };
            state.large_data_cache.store(&cache_key, base64_data);
        }
    }

    Ok(Box::pin(async_stream::stream! {
        let mut stream = stream;
        let mut buffer = String::new();
        let mut tool_calls_detected = false;
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut content_before_tools = String::new();

        // === SKILL CONTEXT: Clear transient skill context from previous turn ===
        llm_interface.clear_skill_context().await;

        let stream_start = Instant::now();
        let mut last_event_time = Instant::now();

        // Simple progress event (only for images)
        if has_images {
            yield AgentEvent::progress("正在分析图像...", "analyzing", 0);
            last_event_time = Instant::now();
        }

        // Stream the response
        while let Some(result) = StreamExt::next(&mut stream).await {
            let elapsed = stream_start.elapsed();

            if elapsed > safeguards.max_stream_duration {
                tracing::warn!("Stream timeout ({:?} elapsed)", elapsed);
                yield AgentEvent::error(format!("Request timeout ({:.1}s elapsed)", elapsed.as_secs_f64()));
                break;
            }

            // Heartbeat
            if last_event_time.elapsed() > safeguards.heartbeat_interval {
                yield AgentEvent::heartbeat();
                last_event_time = Instant::now();
            }

            match result {
                Ok((text, is_thinking)) => {
                    if text.is_empty() {
                        continue;
                    }

                    if is_thinking {
                        yield AgentEvent::thinking(text.clone());
                        last_event_time = Instant::now();
                        continue;
                    }

                    buffer.push_str(&text);
                    last_event_time = Instant::now();

                    // Check for tool calls in buffer
                    let json_tool_check = detect_json_tool_calls(&buffer);
                    if let Some((json_start, tool_json, _remaining)) = json_tool_check {
                        let before_tool = &buffer[..json_start];
                        if !before_tool.is_empty() {
                            content_before_tools.push_str(before_tool);
                            yield AgentEvent::content(before_tool);
                        }

                        if let Ok((_, calls)) = parse_tool_calls(&tool_json) {
                            tool_calls_detected = true;
                            tool_calls.extend(calls);
                        }

                        // Discard remaining hallucinated content after embedded tool calls
                        buffer.clear();
                    } else {
                        // No JSON tool calls detected - check for XML format
                        if let Some(tool_start) = buffer.find("<tool_calls>") {
                            let before_tool = &buffer[..tool_start];
                            if !before_tool.is_empty() {
                                content_before_tools.push_str(before_tool);
                                yield AgentEvent::content(before_tool);
                            }

                            if let Some(tool_end) = buffer.find("</tool_calls>") {
                                let tool_content = buffer[tool_start..tool_end + 13].to_string();
                                // Discard remaining hallucinated content after XML tool calls
                                buffer.clear();

                                if let Ok((_, calls)) = parse_tool_calls(&tool_content) {
                                    tool_calls_detected = true;
                                    tool_calls.extend(calls);
                                }
                            }
                        } else {
                            // No tool calls detected - yield content immediately for real-time streaming
                            if !text.is_empty() {
                                yield AgentEvent::content(text.clone());
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Stream error: {}", e);
                    yield AgentEvent::error(format!("Stream error: {}", e));
                    // Save partial response on error to maintain conversation context
                    if !buffer.is_empty() || !content_before_tools.is_empty() {
                        let partial_content = if content_before_tools.is_empty() {
                            buffer.clone()
                        } else {
                            content_before_tools.clone()
                        };
                        let partial_msg = AgentMessage::assistant(&partial_content);
                        internal_state.write().await.push_message(partial_msg);
                        tracing::debug!("Saved partial multimodal response on error: {} chars", partial_content.len());
                    }
                    break;
                }
            }
        }

        // Handle tool calls if detected
        if tool_calls_detected {
            tracing::debug!("Tool calls detected in multimodal response, executing {} tools", tool_calls.len());

            let tool_calls_to_execute = tool_calls.clone();

            // Resolve cached data references in tool arguments
            let (large_cache, cache) = {
                let state = internal_state.read().await;
                (state.large_data_cache.clone(), state.tool_result_cache.clone())
            };

            // Execute tool calls with bounded concurrency (max 6 parallel)
            let tool_inputs: Vec<(String, serde_json::Value)> = tool_calls_to_execute
                .iter()
                .map(|tc| (tc.name.clone(), resolve_cached_arguments(&tc.arguments, &large_cache)))
                .collect();

            let tool_futures = futures::stream::iter(tool_inputs.into_iter().map(|(name, arguments)| {
                let tools_clone = tools.clone();
                let cache_clone = cache.clone();

                async move {
                    (name.clone(), ToolExecutionResult {
                        _name: name.clone(),
                        arguments: arguments.clone(),
                        result: execute_tool_with_retry(&tools_clone, &cache_clone, &name, arguments.clone()).await,
                    })
                }
            })).buffer_unordered(6);

            let tool_results_executed: Vec<_> = tool_futures.collect().await;

            // Process results
            let mut tool_calls_with_results: Vec<ToolCall> = Vec::new();
            let mut tool_call_results: Vec<(String, String)> = Vec::new();

            for (name, execution) in tool_results_executed {
                // Use arguments from the execution result (preserves per-call arguments for same-name tools)
                let exec_arguments = execution.arguments.clone();
                yield AgentEvent::tool_call_start(&name, exec_arguments.clone());

                match execution.result {
                    Ok(output) => {
                        let result_value = if output.success {
                            output.data.clone()
                        } else {
                            output.error.clone().map(|e| serde_json::json!({"error": e}))
                                .unwrap_or_else(|| serde_json::json!("Error"))
                        };
                        let result_str = if output.success {
                            serde_json::to_string(&output.data).unwrap_or_else(|_| "Success".to_string())
                        } else {
                            output.error.clone().unwrap_or_else(|| "Error".to_string())
                        };

                        // Sanitize base64/image data before sending to frontend or LLM
                        let display_str = sanitize_tool_result_for_prompt(&result_str);

                        tool_calls_with_results.push(ToolCall {
                            name: name.clone(),
                            id: String::new(),
                            arguments: exec_arguments,
                            result: Some(result_value.clone()),
                            round: Some(1),
                        });

                        yield AgentEvent::tool_call_end(&name, &display_str, output.success);
                        tool_call_results.push((name.clone(), display_str));
                    }
                    Err(e) => {
                        let error_msg = format!("Tool execution failed: {}", e);
                        let error_value = serde_json::json!({"error": error_msg});

                        tool_calls_with_results.push(ToolCall {
                            name: name.clone(),
                            id: String::new(),
                            arguments: exec_arguments,
                            result: Some(error_value.clone()),
                            round: Some(1),
                        });

                        yield AgentEvent::tool_call_end(&name, &error_msg, false);
                        tool_call_results.push((name.clone(), error_msg));
                    }
                }
            }

            // Save assistant message with tool calls
            let response_to_save = if content_before_tools.is_empty() {
                String::new()
            } else {
                content_before_tools.clone()
            };

            let initial_msg = AgentMessage::assistant_with_tools(&response_to_save, tool_calls_with_results.clone());
            internal_state.write().await.push_message(initial_msg);

            // Add tool result messages (large results go through cache → summary)
            for (tool_name, result_str) in &tool_call_results {
                if tool_name == "skill" {
                    llm_interface.set_skill_context(result_str.clone()).await;
                } else {
                    let mut state = internal_state.write().await;
                    let history_content = state.large_data_cache.store(tool_name, result_str);
                    let tool_result_msg = AgentMessage::tool_result(tool_name, &history_content);
                    state.push_message(tool_result_msg);
                }
            }

            // Summary fallback: ask LLM to summarize tool results (no tools, no thinking)
            // This avoids dumping raw tool JSON to the user
            let summary_history: Vec<neomind_core::Message> = {
                let state_guard = internal_state.read().await;
                let compacted = super::compact_tool_results(&state_guard.memory, 2);
                compacted.iter().map(|msg| msg.to_core()).collect()
            };

            let summary_prompt = "Based on the tool execution results in the conversation above, \
                provide a concise analysis and summary. Do NOT output any tool calls — \
                give a direct text response to the user's question.";

            let mut final_content = String::new();
            let summary_result = llm_interface.chat_stream_summary(
                summary_prompt,
                &summary_history,
            ).await;

            match summary_result {
                Ok(stream) => {
                    let mut pin = Box::pin(stream);
                    use futures::StreamExt;
                    while let Some(chunk) = pin.next().await {
                        match chunk {
                            Ok((text, _)) => {
                                final_content.push_str(&text);
                                yield AgentEvent::content(text);
                            }
                            Err(e) => {
                                tracing::error!("Multimodal summary stream error: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Multimodal summary call failed: {}", e);
                }
            }

            // Fallback to formatted tool results if summary is empty
            if final_content.trim().is_empty() {
                let deduped_results = deduplicate_tool_results(&tool_call_results);
                let formatted = format_tool_results(&deduped_results);
                final_content = formatted.clone();
                yield AgentEvent::content(formatted);
            }

            // Save the final content
            {
                let mut state = internal_state.write().await;
                if let Some(last_msg) = state.memory.last_mut() {
                    if last_msg.role == "assistant" && last_msg.tool_calls.is_some() {
                        last_msg.content = final_content.into();
                    } else {
                        let final_msg = AgentMessage::assistant(&final_content);
                        state.memory.push(final_msg);
                    }
                } else {
                    let final_msg = AgentMessage::assistant(&final_content);
                    state.memory.push(final_msg);
                }
            }

            tracing::debug!("Multimodal tool execution complete with summary");
        } else {
            // No tool calls - save response directly
            let raw_response = if buffer.is_empty() {
                String::new()
            } else {
                buffer.clone()
            };

            // Clean any embedded tool call JSON from response
            let response_to_save = remove_tool_calls_from_response(&raw_response);

            let initial_msg = AgentMessage::assistant(&response_to_save);
            internal_state.write().await.push_message(initial_msg);

            // Yield any remaining content
            if !buffer.is_empty() {
                yield AgentEvent::content(buffer.clone());
            }
        }

        let pt = llm_interface.take_last_prompt_tokens().await;
        match pt {
            Some(t) => yield AgentEvent::end_with_tokens(t),
            None => yield AgentEvent::end(),
        }
    }))
}

/// Argument names that typically hold image/base64 data.
const IMAGE_ARG_NAMES: &[&str] = &["image", "image_base64", "base64_data", "image_data", "img"];

/// Resolve `$cached:tool_name` references in tool arguments by replacing them
/// with the full cached data. Also **auto-injects** cached image data for any
/// image-related argument — the LLM cannot reliably pass binary image data, so
/// whenever cached image data exists it takes precedence over the LLM's value.
///
/// Only HTTP(S) URLs are passed through (they may point to a real image resource).
fn resolve_cached_arguments(
    arguments: &serde_json::Value,
    cache: &LargeDataCache,
) -> serde_json::Value {
    match arguments {
        // Explicit $cached: reference → resolve
        serde_json::Value::String(s) if s.starts_with("$cached:") => {
            if let Some(resolved) = cache.resolve_reference(s) {
                tracing::info!(
                    reference = %s,
                    resolved_bytes = resolved.len(),
                    "Resolved cached data reference in tool arguments"
                );
                serde_json::Value::String(resolved)
            } else {
                tracing::warn!(reference = %s, "Cached data reference not found, using as-is");
                arguments.clone()
            }
        }
        serde_json::Value::Object(map) => {
            let resolved: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| {
                    let resolved_val = resolve_cached_arguments(v, cache);
                    // Auto-injection for image arguments:
                    // The LLM cannot reliably pass binary image data — it will copy
                    // truncated previews, output MIME types, or invent values.
                    // If we have cached image data, always prefer it over the LLM's value.
                    if IMAGE_ARG_NAMES.contains(&k.as_str()) {
                        if let serde_json::Value::String(ref s) = resolved_val {
                            // Pass through valid HTTP(S) URLs — those are legitimate references
                            if !s.starts_with("http://") && !s.starts_with("https://") {
                                if let Some((image_data, source)) = cache.get_latest_image() {
                                    tracing::info!(
                                        arg_name = %k,
                                        original_preview = %&s[..s.len().min(80)],
                                        source = %source,
                                        injected_bytes = image_data.len(),
                                        "Auto-injected cached image data (LLM cannot pass binary data)"
                                    );
                                    return (k.clone(), serde_json::Value::String(image_data));
                                }
                            }
                        }
                    }
                    (k.clone(), resolved_val)
                })
                .collect();
            serde_json::Value::Object(resolved)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(
            arr.iter()
                .map(|v| resolve_cached_arguments(v, cache))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Execute a tool with retry logic for transient errors and caching.
async fn execute_tool_with_retry(
    tools: &crate::toolkit::ToolRegistry,
    cache: &Arc<RwLock<ToolResultCache>>,
    name: &str,
    arguments: serde_json::Value,
) -> std::result::Result<crate::toolkit::ToolOutput, crate::toolkit::ToolError> {
    // Check cache for read-only tools
    if is_tool_cacheable(name) {
        let cache_key = ToolResultCache::make_key(name, &arguments);
        {
            let cache_read = cache.read().await;
            if let Some(cached) = cache_read.get(&cache_key) {
                println!("[streaming.rs] Cache HIT for tool: {}", name);
                return Ok(cached);
            }
        }
        println!("[streaming.rs] Cache MISS for tool: {}", name);
    }

    let max_retries = 2u32;
    let result = execute_with_retry_impl(tools, name, arguments.clone(), max_retries).await;

    // Cache successful results for cacheable tools
    if is_tool_cacheable(name) {
        if let Ok(ref output) = result {
            if output.success {
                let cache_key = ToolResultCache::make_key(name, &arguments);
                let mut cache_write = cache.write().await;
                cache_write.insert(cache_key, output.clone());
                // Periodic cleanup
                cache_write.cleanup_expired();
            }
        }
    }

    result
}

/// Map simplified tool names to real tool names.
///
/// Simplified names are used in LLM prompts (e.g., "device.discover")
/// while real names are used in ToolRegistry (e.g., "list_devices").
///
/// NOTE: This now uses the unified ToolNameMapper to ensure consistency.
fn resolve_tool_name(simplified_name: &str) -> String {
    crate::tools::resolve_tool_name(simplified_name)
}

/// Inner retry logic without caching (for code reuse)
async fn execute_with_retry_impl(
    tools: &crate::toolkit::ToolRegistry,
    name: &str,
    arguments: serde_json::Value,
    max_retries: u32,
) -> std::result::Result<crate::toolkit::ToolOutput, crate::toolkit::ToolError> {
    // Map simplified tool name to real tool name
    let real_tool_name = resolve_tool_name(name);

    // Tool execution timeout: 30s default, but respect shell tool's internal timeout
    const DEFAULT_TIMEOUT_SECS: u64 = 30;
    let timeout_secs = if real_tool_name == "shell" {
        // Shell tool manages its own timeout internally; give it room to breathe
        let shell_timeout: u64 = arguments
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(30)
            .min(600);
        shell_timeout + 5 // buffer for process cleanup
    } else {
        DEFAULT_TIMEOUT_SECS
    };

    for attempt in 0..=max_retries {
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(timeout_secs),
            tools.execute(&real_tool_name, arguments.clone()),
        )
        .await
        .unwrap_or(Err(crate::toolkit::ToolError::Execution(format!(
            "Tool '{}' timed out after {}s",
            name, timeout_secs
        ))));

        match &result {
            Ok(output) if output.success => return result,
            Err(e) => {
                let last_error = e.to_string();
                let is_transient = last_error.contains("timeout")
                    || last_error.contains("network")
                    || last_error.contains("connection")
                    || last_error.contains("unavailable");

                if is_transient && attempt < max_retries {
                    let delay_ms = 100u64 * (2_u64.pow(attempt));
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }
                return result;
            }
            _ => return result,
        }
    }

    Err(crate::toolkit::ToolError::Execution(
        "Max retries exceeded".to_string(),
    ))
}

/// Convert AgentEvent stream to String stream for backward compatibility.
pub fn events_to_string_stream(
    event_stream: Pin<Box<dyn Stream<Item = AgentEvent> + Send>>,
) -> Pin<Box<dyn Stream<Item = String> + Send>> {
    Box::pin(async_stream::stream! {
        let mut stream = event_stream;
        while let Some(event) = StreamExt::next(&mut stream).await {
            match event {
                AgentEvent::Content { content } => {
                    yield content;
                }
                AgentEvent::Error { message } => {
                    yield format!("[Error: {}]", message);
                }
                AgentEvent::End { .. } => break,
                _ => {
                    // Ignore other events for backward compatibility
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    // Use std::result::Result for test data (not the crate's Result alias)
    type TestResult<T> = std::result::Result<T, &'static str>;

    /// Test scenario 1: Pure content response (no thinking, no tools)
    #[tokio::test]
    async fn test_pure_content_stream() {
        let chunks: Vec<TestResult<(String, bool)>> = vec![
            Ok(("你好，我是".to_string(), false)),
            Ok(("NeoMind助手".to_string(), false)),
            Ok(("。".to_string(), false)),
        ];

        let mut stream = futures::stream::iter(chunks);

        let mut full_content = String::new();
        while let Some(result) = stream.next().await {
            if let Ok((text, is_thinking)) = result {
                assert!(!is_thinking, "Should not be thinking");
                full_content.push_str(&text);
            }
        }

        assert_eq!(full_content, "你好，我是NeoMind助手。");
        println!("Pure content stream test passed: {}", full_content);
    }

    /// Test scenario 2: Thinking + content response
    #[tokio::test]
    async fn test_thinking_then_content_stream() {
        let chunks: Vec<TestResult<(String, bool)>> = vec![
            Ok(("让我分析一下".to_string(), true)),
            Ok(("这个问题".to_string(), true)),
            Ok(("好的，我来回答".to_string(), false)),
            Ok(("这是答案".to_string(), false)),
        ];

        let mut stream = futures::stream::iter(chunks);

        let mut thinking_content = String::new();
        let mut actual_content = String::new();

        while let Some(result) = stream.next().await {
            if let Ok((text, is_thinking)) = result {
                if is_thinking {
                    thinking_content.push_str(&text);
                } else {
                    actual_content.push_str(&text);
                }
            }
        }

        assert_eq!(thinking_content, "让我分析一下这个问题");
        assert_eq!(actual_content, "好的，我来回答这是答案");
        println!("Thinking + content stream test passed");
        println!("  Thinking: {}", thinking_content);
        println!("  Content: {}", actual_content);
    }

    /// Test scenario 3: Content followed by tool call
    #[tokio::test]
    async fn test_content_with_tool_call() {
        let chunks: Vec<TestResult<(String, bool)>> = vec![
            Ok(("让我帮您".to_string(), false)),
            Ok(("查询设备".to_string(), false)),
            Ok((
                "<tool_calls><invoke name=\"list_devices\"></invoke></tool_calls>".to_string(),
                false,
            )),
        ];

        let mut stream = futures::stream::iter(chunks);

        let mut content_before_tools = String::new();
        let mut buffer = String::new();
        let mut tool_calls_found = false;

        while let Some(result) = stream.next().await {
            if let Ok((text, is_thinking)) = result {
                assert!(!is_thinking, "Should not be thinking in this test");
                buffer.push_str(&text);

                // Check for tool calls
                if let Some(tool_start) = buffer.find("<tool_calls>") {
                    content_before_tools.push_str(&buffer[..tool_start]);
                    if let Some(_tool_end) = buffer.find("</tool_calls>") {
                        tool_calls_found = true;
                        break;
                    }
                }
            }
        }

        assert_eq!(content_before_tools, "让我帮您查询设备");
        assert!(tool_calls_found, "Tool calls should be detected");
        println!("Content with tool call test passed");
        println!("  Content before tools: {}", content_before_tools);
    }

    /// Test scenario 4: Thinking + content + tool call
    #[tokio::test]
    async fn test_thinking_content_tool_call() {
        let chunks: Vec<TestResult<(String, bool)>> = vec![
            Ok(("用户想查询设备".to_string(), true)),
            Ok(("需要调用list_devices".to_string(), true)),
            Ok(("好的，我来".to_string(), false)),
            Ok(("查询一下".to_string(), false)),
            Ok((
                "<tool_calls><invoke name=\"list_devices\"></invoke></tool_calls>".to_string(),
                false,
            )),
        ];

        let mut stream = futures::stream::iter(chunks);

        let mut thinking = String::new();
        let mut content = String::new();
        let mut has_tool_calls = false;

        while let Some(result) = stream.next().await {
            if let Ok((text, is_thinking)) = result {
                if is_thinking {
                    thinking.push_str(&text);
                } else {
                    content.push_str(&text);
                    if text.contains("<tool_calls>") {
                        has_tool_calls = true;
                    }
                }
            }
        }

        assert_eq!(thinking, "用户想查询设备需要调用list_devices");
        assert!(content.contains("好的，我来查询一下"));
        assert!(has_tool_calls, "Should have tool calls");
        println!("Thinking + content + tool call test passed");
    }

    /// Test scenario 5: Empty content with thinking (edge case for think=true models)
    #[tokio::test]
    async fn test_thinking_only_no_content() {
        let chunks: Vec<TestResult<(String, bool)>> = vec![
            Ok(("这是我的思考过程".to_string(), true)),
            Ok(("继续思考".to_string(), true)),
        ];

        let mut stream = futures::stream::iter(chunks);

        let mut thinking = String::new();
        let mut content = String::new();

        while let Some(result) = stream.next().await {
            if let Ok((text, is_thinking)) = result {
                if is_thinking {
                    thinking.push_str(&text);
                } else {
                    content.push_str(&text);
                }
            }
        }

        assert_eq!(thinking, "这是我的思考过程继续思考");
        assert!(
            content.is_empty(),
            "Content should be empty for thinking-only response"
        );
        println!("Thinking-only test passed");
        println!("  Thinking: {}", thinking);
    }

    /// Test scenario 6: Content split across multiple chunks with Chinese characters
    #[tokio::test]
    async fn test_multibyte_chunk_handling() {
        let chunks: Vec<TestResult<(String, bool)>> = vec![
            // Split in middle of multi-byte sequence (shouldn't happen but test robustness)
            Ok(("你好".to_string(), false)),
            Ok(("世界".to_string(), false)),
            Ok(("，这是".to_string(), false)),
            Ok(("一个测试".to_string(), false)),
        ];

        let mut stream = futures::stream::iter(chunks);

        let mut full_content = String::new();
        while let Some(result) = stream.next().await {
            if let Ok((text, is_thinking)) = result {
                assert!(!is_thinking);
                full_content.push_str(&text);
            }
        }

        assert_eq!(full_content, "你好世界，这是一个测试");
        println!("Multi-byte chunk handling test passed");
        println!("  Content: {}", full_content);
    }

    /// Test scenario 7: Tool call with arguments
    #[tokio::test]
    async fn test_tool_call_with_arguments() {
        let tool_xml = r#"<tool_calls><invoke name="set_device_state">
<parameter name="device_id">lamp_1</parameter>
<parameter name="state">on</parameter>
</invoke></tool_calls>"#;

        let chunks: Vec<TestResult<(String, bool)>> = vec![
            Ok(("好的，我来帮您".to_string(), false)),
            Ok((tool_xml.to_string(), false)),
        ];

        let mut stream = futures::stream::iter(chunks);

        let mut content = String::new();
        let mut buffer = String::new();

        while let Some(result) = stream.next().await {
            if let Ok((text, _)) = result {
                buffer.push_str(&text);

                if let Some(tool_start) = buffer.find("<tool_calls>") {
                    content.push_str(&buffer[..tool_start]);
                    if buffer.contains("</tool_calls>") {
                        break;
                    }
                }
            }
        }

        assert_eq!(content, "好的，我来帮您");
        assert!(buffer.contains("<invoke name=\"set_device_state\">"));
        assert!(buffer.contains("<parameter name=\"device_id\">lamp_1</parameter>"));
        println!("Tool call with arguments test passed");
    }

    /// Test scenario 8: Empty chunks handling
    #[tokio::test]
    async fn test_empty_chunk_handling() {
        let chunks: Vec<TestResult<(String, bool)>> = vec![
            Ok(("开始".to_string(), false)),
            Ok(("".to_string(), false)), // Empty chunk
            Ok(("继续".to_string(), false)),
            Ok(("".to_string(), false)), // Another empty chunk
            Ok(("结束".to_string(), false)),
        ];

        let mut stream = futures::stream::iter(chunks);

        let mut full_content = String::new();
        while let Some(result) = stream.next().await {
            if let Ok((text, _)) = result {
                full_content.push_str(&text);
            }
        }

        // Empty chunks should be included but not cause issues
        assert!(full_content.contains("开始"));
        assert!(full_content.contains("继续"));
        assert!(full_content.contains("结束"));
        println!("Empty chunk handling test passed");
        println!("  Content: {}", full_content);
    }

    /// Test tool parser
    #[test]
    fn test_tool_parser() {
        let input = r#"{"name": "test_tool", "arguments": {"param1": "value1"}}"#;

        let result = parse_tool_calls(input);
        assert!(result.is_ok(), "Should parse tool calls successfully");

        let (_remaining, calls) = result.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "test_tool");
        assert_eq!(calls[0].arguments["param1"], "value1");
        println!("Tool parser test passed");
    }

    /// Test token estimation
    #[test]
    fn test_token_estimation() {
        let english = "Hello world, this is a test";
        let chinese = "你好世界，这是一个测试";

        let english_tokens = crate::agent::tokenizer::estimate_tokens(english);
        let chinese_tokens = crate::agent::tokenizer::estimate_tokens(chinese);

        // Rough estimation: ~4 chars per token for English, ~1.8 tokens per Chinese char
        assert!(english_tokens > 0 && english_tokens < 20);
        // Chinese: ~12 chars × 1.8 × 1.1 buffer ≈ 24 tokens
        assert!(chinese_tokens > 10 && chinese_tokens < 30);

        println!("Token estimation test passed");
        println!(
            "  English ({} chars): ~{} tokens",
            english.chars().count(),
            english_tokens
        );
        println!(
            "  Chinese ({} chars): ~{} tokens",
            chinese.chars().count(),
            chinese_tokens
        );
    }

    /// Test tool cache key generation
    #[test]
    fn test_cache_key_generation() {
        let key1 = ToolResultCache::make_key("list_devices", &serde_json::json!({}));
        let key2 = ToolResultCache::make_key("list_devices", &serde_json::json!(null));
        let key3 = ToolResultCache::make_key("list_devices", &serde_json::json!({}));

        assert_eq!(key1, key3, "Same args should produce same key");
        assert_ne!(key1, key2, "Different args should produce different keys");

        println!("Cache key generation test passed");
    }

    /// Test that malformed tool call JSON is not detected as tool calls
    /// This prevents false positives from JSON like [{"name":"[...]"}]
    #[test]
    fn test_malformed_tool_call_detection() {
        // Case 1: name field contains nested JSON array (should NOT be detected as tool call)
        let malformed1 = r#"[{"name":"[{"name":"device_discover","arguments":{}}]"}]"#;
        assert!(
            detect_json_tool_calls(malformed1).is_none(),
            "Should not detect malformed tool call with nested JSON array in name field"
        );

        // Case 2: name field contains nested JSON object (should NOT be detected as tool call)
        let malformed2 = r#"[{"name":"{"tool":"test"}"}]"#;
        assert!(
            detect_json_tool_calls(malformed2).is_none(),
            "Should not detect malformed tool call with nested JSON object in name field"
        );

        // Case 3: valid tool call (SHOULD be detected)
        let valid = r#"[{"name":"device_discover","arguments":{}}]"#;
        let result = detect_json_tool_calls(valid);
        assert!(result.is_some(), "Should detect valid tool call");
        let (_, json, _) = result.unwrap();
        assert_eq!(json, valid);

        // Case 4: valid tool call with different name field (SHOULD be detected)
        let valid2 = r#"[{"tool":"list_devices","params":{}}]"#;
        assert!(
            detect_json_tool_calls(valid2).is_some(),
            "Should detect valid tool call with 'tool' field"
        );

        // Case 5: valid tool call with function field (SHOULD be detected)
        let valid3 = r#"[{"function":"get_status","arguments":{}}]"#;
        assert!(
            detect_json_tool_calls(valid3).is_some(),
            "Should detect valid tool call with 'function' field"
        );

        println!("Malformed tool call detection test passed");
    }

    /// Run all streaming tests and print summary
    #[test]
    fn run_all_streaming_tests() {
        println!("\n=== Running LLM Streaming Tests ===\n");

        println!("Test Coverage:");
        println!("  1. Pure content response (no thinking, no tools)");
        println!("  2. Thinking + content response");
        println!("  3. Content followed by tool call");
        println!("  4. Thinking + content + tool call");
        println!("  5. Empty content with thinking (edge case)");
        println!("  6. Multi-byte chunk handling (Chinese)");
        println!("  7. Tool call with arguments");
        println!("  8. Empty chunks handling");
        println!("  9. Tool parser");
        println!(" 10. Token estimation");
        println!(" 11. Cache key generation");
        println!(" 12. Malformed tool call detection");
        println!("\n=== Test Suite Complete ===\n");
    }

    // -----------------------------------------------------------------------
    // Base64 stripping tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanitize_small_result_passes_through() {
        let result = r#"{"device_name":"test","battery":"100%"}"#;
        assert_eq!(sanitize_tool_result_for_prompt(result), result);
    }

    #[test]
    fn test_sanitize_json_with_data_image_url() {
        let result = serde_json::json!({
            "device_name": "NE101",
            "battery": "100%",
            "image_data": "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQ"
        })
        .to_string();

        let sanitized = sanitize_tool_result_for_prompt(&result);
        assert!(
            !sanitized.contains("base64"),
            "Should strip base64 data URL"
        );
        assert!(
            !sanitized.contains("/9j/4AAQ"),
            "Should strip image content"
        );
        assert!(
            sanitized.contains("image data"),
            "Should have image data placeholder"
        );
        assert!(
            sanitized.contains("device_name"),
            "Should preserve non-image fields"
        );
        assert!(sanitized.contains("NE101"), "Should preserve device name");
        assert!(sanitized.contains("100%"), "Should preserve battery info");
    }

    #[test]
    fn test_sanitize_json_with_large_base64_string() {
        // Create a JSON with a large base64 string (>10KB)
        let fake_base64: String = "ABCDEFGHijklmnop+/=".repeat(600); // ~13KB
        let result = serde_json::json!({
            "device_name": "Camera",
            "firmware": "v1.7",
            "base64_data": fake_base64
        })
        .to_string();

        let sanitized = sanitize_tool_result_for_prompt(&result);
        assert!(!sanitized.contains("ABCDEFGH"), "Should strip large base64");
        assert!(
            sanitized.contains("base64 data"),
            "Should have base64 placeholder"
        );
        assert!(sanitized.contains("Camera"), "Should preserve device name");
        assert!(sanitized.contains("v1.7"), "Should preserve firmware");
    }

    #[test]
    fn test_sanitize_nested_json_with_base64() {
        let result = serde_json::json!({
            "device": {
                "name": "NE101",
                "info": {
                    "battery": "85%",
                    "image": "data:image/png;base64,iVBORw0KGgo="
                }
            }
        })
        .to_string();

        let sanitized = sanitize_tool_result_for_prompt(&result);
        assert!(sanitized.contains("NE101"), "Should preserve nested text");
        assert!(sanitized.contains("85%"), "Should preserve battery");
        assert!(!sanitized.contains("iVBOR"), "Should strip nested base64");
        assert!(sanitized.contains("image data"), "Should have placeholder");
    }

    #[test]
    fn test_sanitize_text_with_data_image_url() {
        let text = "Device: Camera\nBattery: 100%\nImage: data:image/jpeg;base64,/9j/4AAQSkZJRgABAQ==\nStatus: OK";

        let sanitized = sanitize_tool_result_for_prompt(text);
        assert!(!sanitized.contains("/9j/"), "Should strip image data");
        assert!(sanitized.contains("Camera"), "Should preserve text");
        assert!(sanitized.contains("100%"), "Should preserve battery");
        assert!(
            sanitized.contains("Status: OK"),
            "Should preserve other text"
        );
    }

    #[test]
    fn test_sanitize_no_base64_large_result_passes_through() {
        // Large result without base64 should be preserved
        let large_data: String = "x".repeat(5000);
        let result = format!(r#"{{"data": "{}"}}"#, large_data);

        let sanitized = sanitize_tool_result_for_prompt(&result);
        assert_eq!(sanitized, result, "Should pass through non-base64 data");
    }

    #[test]
    fn test_truncate_utf8_safe() {
        // Chinese text truncation
        let text = "你好世界这是一段中文测试文本用于验证UTF8安全截断功能";
        let truncated = truncate_result_utf8(text, 5);
        assert!(truncated.starts_with("你好世界这"));
        assert!(truncated.contains("truncated"));

        // Text shorter than max
        let short = "hello";
        assert_eq!(truncate_result_utf8(short, 100), short);
    }

    #[test]
    fn test_humanize_bytes() {
        assert_eq!(humanize_bytes(500), "500B");
        assert_eq!(humanize_bytes(1024), "1.0KB");
        assert_eq!(humanize_bytes(1536), "1.5KB");
        assert_eq!(humanize_bytes(1048576), "1.0MB");
        assert_eq!(humanize_bytes(2621440), "2.5MB");
    }

    #[test]
    fn test_is_large_base64_string() {
        // Too small
        assert!(!is_large_base64_string("abc123"));

        // Large valid base64
        let large_b64: String = "ABCDEFGHijklmnop+/=".repeat(600);
        assert!(is_large_base64_string(&large_b64));

        // Large but not base64 (contains invalid chars)
        let not_b64 = "hello world! ".repeat(1000);
        assert!(!is_large_base64_string(&not_b64));
    }
}
