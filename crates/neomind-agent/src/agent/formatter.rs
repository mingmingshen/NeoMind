//! Data formatting utilities for displaying tool results in user-friendly formats.
//!
//! This module provides functions to convert JSON data from tool results
//! into readable tables, lists, and other formatted representations.

use serde_json::Value;

/// Format tool result data into a human-readable string.
///
/// This function analyzes the JSON structure and formats it appropriately:
/// - Arrays of objects → Table format
/// - Simple arrays → List format
/// - Objects → Key-value pairs
pub fn format_tool_result(data: &Value) -> String {
    match data {
        // Array of objects - format as table
        Value::Array(arr) if arr.iter().all(|v| v.is_object()) && !arr.is_empty() => {
            format_as_table(arr)
        }
        // Simple array - format as list
        Value::Array(arr) => format_as_list(arr),
        // Object - format as key-value pairs
        Value::Object(obj) => format_as_key_value(obj),
        // Primitive values - just convert to string
        _ => data.to_string(),
    }
}

/// Format an array of objects as a Markdown table.
fn format_as_table(items: &[Value]) -> String {
    if items.is_empty() {
        return "（空列表）".to_string();
    }

    // Collect all unique keys from all objects
    let mut all_keys: Vec<String> = Vec::new();
    for item in items {
        if let Some(obj) = item.as_object() {
            for key in obj.keys() {
                if !all_keys.contains(key) {
                    all_keys.push(key.clone());
                }
            }
        }
    }

    if all_keys.is_empty() {
        return "（无数据）".to_string();
    }

    // Limit columns for readability
    let max_columns = 6;
    if all_keys.len() > max_columns {
        all_keys.truncate(max_columns);
    }

    // Build table header
    let mut table = String::from("| ");
    for key in &all_keys {
        table.push_str(&format!("{} | ", translate_key(key)));
    }
    table.push_str("\n| ");

    // Build separator row
    for _ in &all_keys {
        table.push_str("--- | ");
    }
    table.push('\n');

    // Build data rows
    for item in items {
        if let Some(obj) = item.as_object() {
            table.push_str("| ");
            for key in &all_keys {
                let value = obj
                    .get(key)
                    .map(format_value)
                    .unwrap_or_else(|| "-".to_string());
                table.push_str(&format!("{} | ", truncate(&value, 30)));
            }
            table.push('\n');
        }
    }

    table
}

/// Format a simple array as a bulleted list.
fn format_as_list(items: &[Value]) -> String {
    if items.is_empty() {
        return "（空列表）".to_string();
    }

    let mut list = String::new();
    for (i, item) in items.iter().enumerate() {
        list.push_str(&format!("{}. {}\n", i + 1, format_value(item)));
    }
    list
}

/// Format an object as key-value pairs.
fn format_as_key_value(obj: &serde_json::Map<String, Value>) -> String {
    if obj.is_empty() {
        return "（无数据）".to_string();
    }

    let mut result = String::new();
    for (key, value) in obj {
        // Skip nested objects at top level for cleaner output
        if value.is_object() || value.is_array() {
            result.push_str(&format!("**{}**: [复杂数据]\n", translate_key(key)));
        } else {
            result.push_str(&format!(
                "**{}**: {}\n",
                translate_key(key),
                format_value(value)
            ));
        }
    }
    result
}

/// Format a single value for display.
fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "-".to_string(),
        Value::Bool(b) => if *b { "是" } else { "否" }.to_string(),
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f == n.as_i64().unwrap_or(0) as f64 {
                    format!("{}", n.as_i64().unwrap_or(0))
                } else {
                    format!("{:.2}", f)
                }
            } else {
                n.to_string()
            }
        }
        Value::String(s) => {
            // Check if it's a timestamp
            if let Ok(ts) = s.parse::<i64>() {
                if ts > 1_000_000_000 && ts < 2_000_000_000 {
                    if let Some(datetime) = timestamp_to_datetime(ts) {
                        return datetime;
                    }
                }
            }
            s.clone()
        }
        Value::Array(arr) if arr.len() == 1 => format_value(&arr[0]),
        Value::Array(arr) => format!("[{}项数据]", arr.len()),
        Value::Object(_) => "[对象]".to_string(),
    }
}

/// Translate common English keys to Chinese.
fn translate_key(key: &str) -> String {
    match key {
        "id" => "ID".to_string(),
        "name" => "名称".to_string(),
        "type" => "类型".to_string(),
        "status" => "状态".to_string(),
        "enabled" => "启用".to_string(),
        "count" => "数量".to_string(),
        "value" => "值".to_string(),
        "timestamp" | "time" => "时间".to_string(),
        "device_id" => "设备ID".to_string(),
        "device_type" => "设备类型".to_string(),
        "rule_id" => "规则ID".to_string(),
        "rule_name" => "规则名称".to_string(),
        "trigger_count" => "触发次数".to_string(),
        "actions_executed" => "执行动作".to_string(),
        "duration_ms" => "耗时(ms)".to_string(),
        "workflow_id" => "工作流ID".to_string(),
        "execution_id" => "执行ID".to_string(),
        "started_at" | "start_time" => "开始时间".to_string(),
        "completed_at" | "end_time" => "完成时间".to_string(),
        "current_step" => "当前步骤".to_string(),
        "success" => "成功".to_string(),
        "error" => "错误".to_string(),
        "data" => "数据".to_string(),
        "metric" => "指标".to_string(),
        "limit" => "限制".to_string(),
        "message" => "消息".to_string(),
        "command" => "命令".to_string(),
        "parameters" => "参数".to_string(),
        "description" => "描述".to_string(),
        "result" => "结果".to_string(),
        _ => key.to_string(),
    }
}

/// Convert Unix timestamp to readable datetime.
fn timestamp_to_datetime(ts: i64) -> Option<String> {
    use chrono::{DateTime, Local, Utc};

    let dt = DateTime::<Utc>::from_timestamp(ts, 0)?;
    Some(
        dt.with_timezone(&Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    )
}

/// Truncate string to max length (UTF-8 safe).
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let end = s.floor_char_boundary(max_len.saturating_sub(3));
        format!("{}...", &s[..end])
    }
}

/// Format a summary line for a tool result.
pub fn format_summary(tool_name: &str, result: &serde_json::Map<String, Value>) -> String {
    match tool_name {
        "list_devices" => {
            let count = result.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
            format!("📱 找到 {} 个设备", count)
        }
        "list_rules" => {
            let count = result.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
            format!("📜 找到 {} 条规则", count)
        }
        "create_rule" => "➕ 自动化规则已创建".to_string(),
        "trigger_workflow" => "⚡ 工作流已触发".to_string(),
        "query_rule_history" => {
            let count = result.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
            format!("📜 找到 {} 条执行历史", count)
        }
        "query_workflow_status" => {
            let count = result.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
            format!("🔄 找到 {} 条执行记录", count)
        }
        _ => format!("✅ {} 执行完成", tool_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_table() {
        let data = serde_json::json!([
            {"id": "1", "name": "Device 1", "type": "sensor"},
            {"id": "2", "name": "Device 2", "type": "actuator"}
        ]);

        let result = format_tool_result(&data);
        assert!(result.contains("|"));
        assert!(result.contains("类型")); // "type" translates to "类型"
    }

    #[test]
    fn test_format_list() {
        let data = serde_json::json!(["item1", "item2", "item3"]);
        let result = format_tool_result(&data);
        assert!(result.contains("1."));
        assert!(result.contains("2."));
    }

    #[test]
    fn test_format_key_value() {
        let data = serde_json::json!({"name": "test", "count": 5});
        let result = format_tool_result(&data);
        assert!(result.contains("名称"));
        assert!(result.contains("数量"));
    }

    #[test]
    fn test_summary_devices() {
        let result = serde_json::json!({"count": 3, "devices": []});
        let summary = format_summary("list_devices", result.as_object().unwrap());
        assert_eq!(summary, "📱 找到 3 个设备");
    }
}
