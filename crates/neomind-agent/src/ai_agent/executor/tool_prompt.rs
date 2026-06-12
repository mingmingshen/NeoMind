//! Tool prompt construction for the agent tool-calling loop.
//!
//! Builds the system prompt, resource/data sections, and initial messages
//! (including multimodal image support) for the tool-calling execution mode.

use base64::Engine;
use neomind_core::message::{Content, ContentPart, Message, MessageRole};
use neomind_storage::{AiAgent, DataCollected, ResourceType};
use std::collections::HashMap;

use super::{
    build_history_context, format_timestamp, get_time_context, resolve_role, truncate_to,
    HistoryConfig, ToolLoopConfig,
};

/// Build the system prompt for tool-calling (Free) mode.
///
/// Unlike the Focused analysis path which filters out memory data for small
/// models, the Free prompt intentionally **includes** historical context
/// (knowledge files, execution journal, user messages) so the
/// agent can leverage accumulated experience and make progressively better
/// decisions.
///
/// `knowledge_content`: pre-fetched knowledge file contents for inline
/// injection, avoiding the need to waste a tool-call round reading them.
pub(crate) fn build_tool_system_prompt(
    agent: &AiAgent,
    data_collected: &[DataCollected],
    invocation_input: Option<&super::super::AgentInput>,
    config: &ToolLoopConfig,
    knowledge_content: Option<&HashMap<String, String>>,
) -> String {
    let time_ctx = get_time_context();

    // ── Event trigger callout (if triggered by data event) ──
    let event_callout = data_collected
        .iter()
        .find(|d| {
            d.values
                .get("_is_event_data")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .map(|d| {
            let value_str = if let Some(v) = d.values.get("value") {
                match v {
                    serde_json::Value::String(s) => truncate_to(s, 100),
                    other => truncate_to(&other.to_string(), 100),
                }
            } else {
                truncate_to(&serde_json::to_string(&d.values).unwrap_or_default(), 100)
            };
            let ts = format_timestamp(d.timestamp);
            format!(
                "\n## TRIGGERING EVENT\n\
                 Source: **{}**\n\
                 Time: {}\n\
                 Value: **{}**\n\
                 → This event triggered your execution. Prioritize analyzing this data.\n",
                d.source, ts, value_str
            )
        })
        .unwrap_or_default();

    // ── Merged resource + data section (eliminates redundancy) ──
    let resource_data_section = if config.is_focused_plus {
        build_focused_resource_section(agent, data_collected)
    } else {
        build_free_resource_section(agent, data_collected)
    };

    // ── Context: User Messages → Knowledge Files → Journal ──
    let history_section = if config.is_focused_plus {
        build_history_context(
            agent,
            &HistoryConfig::focused(agent.context_window_size),
            knowledge_content,
        )
    } else {
        build_history_context(
            agent,
            &HistoryConfig::free(agent.context_window_size),
            knowledge_content,
        )
    };

    // Build invocation input section
    let invocation_section = match invocation_input {
        Some(input) => {
            let mut parts = Vec::new();
            if let Some(ref source) = input.source {
                parts.push(format!("来源/Source: {}", source));
            }
            if let Some(ref content) = input.content {
                parts.push(format!("内容/Content: {}", content));
            }
            if let Some(ref data) = input.data {
                let data_str = serde_json::to_string_pretty(data).unwrap_or_default();
                parts.push(format!("附加数据/Data:\n{}", data_str));
            }
            if parts.is_empty() {
                String::new()
            } else {
                format!(
                    "\n## Caller Input (invoked by external request)\n{}\n",
                    parts.join("\n")
                )
            }
        }
        None => String::new(),
    };

    // Mode constraints
    let mut mode_constraints = String::new();
    if let Some(ref recommended) = config.recommended_tools {
        mode_constraints.push_str(&format!(
            "\nRecommended tools for this task (prioritize these): {}",
            recommended.join(", ")
        ));
    }
    if config.is_focused_plus {
        mode_constraints.push_str(&format!(
            "\nYou have at most {} round(s). Be efficient — \
             use tools to query history or details when the snapshot is insufficient.",
            config.max_rounds
        ));
    }

    // Combined guidelines + exit guidance (one section, no redundancy)
    let memory_guidance = "\
         - Use the `memory` tool to persist important discoveries. Create a knowledge file when you:\n\
           * Discover stable thresholds or normal ranges (e.g., 'temp normal: 22-28°C')\n\
           * Identify recurring patterns across executions\n\
           * Learn device quirks or environment-specific behaviors\n\
           * Derive alert rules from accumulated observations\n\
         - Knowledge file format: one topic per file, bullet points, concise. Example:\n\
           `memory(action='create', target='custom:thresholds', content='# Thresholds\\n- CPU alert: >85%\\n- Temp normal: 22-28°C\\n- Temp alert: >40°C')`\n\
         - Do NOT store temporary data, raw metrics, or information that changes every execution.\n\
         - When appending (`add`) to an existing file, append ONLY the new data point — never re-list previous entries or resend the full section. For time-series notes, add just the new timestamp line.\n\
         - Update existing files with `add`/`replace` rather than creating duplicates.";

    let action_guidance = "\
         - Send notifications/alerts via the `shell` tool with: `neomind message send --title \"<title>\" --body \"<body>\" --severity <info|warning|error|critical>`. There is NO separate `message` tool — always use `shell`.";

    let combined_guidance = if config.is_focused_plus {
        format!(
            "## Guidelines & Exit\n\
             - The snapshot above shows current values. Use `shell` with command `neomind device history <device_id>` for trends.\n\
             - You can use `shell` with command `neomind device control <device_id> <command>` to execute bound commands.\n\
             - Do NOT call the same tool with the same parameters if it already returned results.\n\
             - Max {} rounds. Be efficient.\n\
             - For complex operations, use the `skill` tool to search for guides.\n\
             - Stop when you have enough data or a tool call failed. Write your analysis directly — plain text only.\n\
             {action_guidance}\n\
             {memory_guidance}",
            config.max_rounds,
        )
    } else {
        format!(
            "## Guidelines & Exit\n\
             - Do NOT call the same tool with the same parameters if it already returned results.\n\
             - If a metric query returns empty data, try a different metric or move on.\n\
             - Batch similar queries: use `neomind device list` once, then query each device in ONE round. Do NOT re-query devices you already have data for.\n\
             - Before querying, review the tool results in your conversation — if a device's data was already returned, do not query it again.\n\
             - Track what you have: mentally note which devices/queries are already done and only issue new queries.\n\
             - Max {} rounds. Be efficient.\n\
             - For complex operations, use the `skill` tool to search for guides.\n\
             - Stop when you have enough data, already sent notifications, got the same result, or a tool failed.\n\
             - After your last tool call, write your analysis directly — plain text only, key findings first.\n\
             {action_guidance}\n\
             {memory_guidance}",
            config.max_rounds,
        )
    };

    let default_identity = format!(
        "You are an intelligent IoT agent named '{}' monitoring edge devices.",
        agent.name
    );
    let identity = resolve_role(agent, &default_identity);

    format!(
        "{}\nTime: {}\nTask: {}\n{}{}\n{}\n{}\n{}\n\n{}\n",
        identity,
        time_ctx,
        agent.user_prompt,
        event_callout,
        history_section,
        resource_data_section,
        invocation_section,
        mode_constraints,
        combined_guidance,
    )
}

/// Build merged resource + data section for Focused+ mode.
/// Single table: | Resource | Type | Current |
pub(crate) fn build_focused_resource_section(
    agent: &AiAgent,
    data_collected: &[DataCollected],
) -> String {
    let now_ts = chrono::Utc::now().timestamp();

    // Separate device_info from metric data
    let mut device_info_map: HashMap<&str, &serde_json::Value> = HashMap::new();
    let mut latest_values: HashMap<&str, (String, i64)> = HashMap::new();
    let mut image_sources: Vec<&str> = Vec::new();

    for d in data_collected {
        if d.source == "system" {
            continue;
        }
        // Collect device_info entries separately
        if d.data_type == "device_info" {
            device_info_map.insert(&d.source, &d.values);
            continue;
        }
        if d.values
            .get("_is_image")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            image_sources.push(&d.source);
            continue;
        }
        let val_str = if let Some(v) = d.values.get("value") {
            format!("{}", v)
        } else if d.values != serde_json::Value::Null {
            truncate_to(&serde_json::to_string(&d.values).unwrap_or_default(), 80)
        } else {
            continue;
        };
        let age = (now_ts - d.timestamp).max(0);
        latest_values.insert(&d.source, (val_str, age));
    }

    if agent.resources.is_empty() && latest_values.is_empty() && device_info_map.is_empty() {
        return "\n## Resources & Data\nNo bound resources. Use tools to query.\n".to_string();
    }

    let mut section = String::from("\n## Resources & Data\n");

    // Render device summary block
    if !device_info_map.is_empty() {
        section.push_str("**Devices:**\n");
        // Sort for deterministic output
        let mut devices: Vec<_> = device_info_map.iter().collect();
        devices.sort_by_key(|(id, _)| *id);
        for (device_id, info) in devices {
            let name = info
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(device_id);
            let dev_type = info
                .get("device_type")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let display = if name != *device_id {
                format!("{} ({})", device_id, name)
            } else {
                device_id.to_string()
            };
            section.push_str(&format!("- {} {}\n", display, dev_type));
        }
        section.push('\n');
    }

    section.push_str("| Resource | Type | Current | Age |\n|----------|------|---------|-----|\n");

    for r in &agent.resources {
        let type_str = match r.resource_type {
            ResourceType::Metric | ResourceType::ExtensionMetric => "metric",
            ResourceType::Command | ResourceType::ExtensionTool => "command",
            ResourceType::Device => "device",
            ResourceType::DataStream => "stream",
        };
        let display_name = if r.name != r.resource_id {
            format!("{} ({})", r.resource_id, r.name)
        } else {
            r.resource_id.clone()
        };
        let (current, age_str) = latest_values
            .get(r.resource_id.as_str())
            .map(|(v, age)| {
                let age_fmt = if *age == 0 {
                    "now".to_string()
                } else {
                    format!("{}s", age)
                };
                (v.clone(), age_fmt)
            })
            .unwrap_or_else(|| ("-".to_string(), "-".to_string()));
        section.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            display_name, type_str, current, age_str
        ));
    }

    // Add any data sources not in resources
    for (source, (value, age)) in &latest_values {
        let source_id = source.to_string();
        if !agent.resources.iter().any(|r| r.resource_id == source_id) {
            let age_fmt = if *age == 0 {
                "now".to_string()
            } else {
                format!("{}s", age)
            };
            section.push_str(&format!("| {} | - | {} | {} |\n", source, value, age_fmt));
        }
    }

    // Note image sources for context (images are already embedded in user message)
    if !image_sources.is_empty() {
        section.push_str(&format!(
            "\n**Images**: {} (included in message)\n",
            image_sources.join(", ")
        ));
    }

    section.push('\n');
    section
}

/// Build merged resource + data section for Free mode.
/// Resource list + JSON data dump in one section.
pub(crate) fn build_free_resource_section(
    agent: &AiAgent,
    data_collected: &[DataCollected],
) -> String {
    let mut section = String::from("\n## Resources & Data\n");

    if !agent.resources.is_empty() {
        let items: Vec<String> = agent
            .resources
            .iter()
            .map(|r| format!("- {} ({})", r.name, r.resource_id))
            .collect();
        section.push_str(&format!("Bound: {}\n", items.join(", ")));
    }

    let data_text: Vec<String> = data_collected
        .iter()
        .filter(|d| {
            if d.values
                .get("_is_image")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return false;
            }
            if d.source == "system"
                && d.values
                    .get("message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains("No pre-collected data"))
                    .unwrap_or(false)
            {
                return false;
            }
            true
        })
        .map(|d| {
            // Format as readable key=value pairs for small models
            let formatted = if let Some(obj) = d.values.as_object() {
                let pairs: Vec<String> = obj
                    .iter()
                    .filter(|(k, _)| !k.starts_with('_')) // skip internal fields
                    .map(|(k, v)| {
                        let val = match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            other => truncate_to(&other.to_string(), 100),
                        };
                        format!("{}={}", k, val)
                    })
                    .collect();
                if pairs.is_empty() {
                    serde_json::to_string_pretty(&d.values).unwrap_or_default()
                } else {
                    pairs.join(", ")
                }
            } else {
                serde_json::to_string_pretty(&d.values).unwrap_or_default()
            };
            format!("**{}**: {}", d.source, formatted)
        })
        .collect();

    if data_text.is_empty() {
        section.push_str(
            "No pre-collected data. **You MUST use tools to query the data you need!**\n",
        );
    } else {
        section.push_str(&format!("\nData:\n{}\n", data_text.join("\n")));
    }

    section
}

/// Build initial messages (system + user) with multimodal image support.
pub(crate) fn build_tool_messages(
    system_prompt: &str,
    data_collected: &[DataCollected],
) -> Vec<Message> {
    // Collect image parts
    let image_parts: Vec<ContentPart> = data_collected
        .iter()
        .filter(|d| {
            d.values
                .get("_is_image")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .filter_map(|d| {
            if let Some(url) = d.values.get("image_url").and_then(|v| v.as_str()) {
                if !url.is_empty() {
                    return Some(ContentPart::image_url(url.to_string()));
                }
            }
            if let Some(base64) = d.values.get("image_base64").and_then(|v| v.as_str()) {
                if !base64.is_empty() {
                    // Prefer stored mime → fall back to magic-prefix
                    // inference → final jpeg fallback.
                    let mime = d
                        .values
                        .get("image_mime_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            crate::image_utils::infer_mime_from_base64_prefix(base64)
                                .map(|s| s.to_string())
                        })
                        .unwrap_or_else(|| "image/jpeg".to_string());
                    // Clean base64: handle URL-safe chars, strip whitespace, fix padding
                    let cleaned: String = base64
                        .chars()
                        .filter_map(|c| match c {
                            '-' => Some('+'),
                            '_' => Some('/'),
                            c if c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' => {
                                Some(c)
                            }
                            _ => None, // skip whitespace/newlines
                        })
                        .collect();
                    let padded_len = (cleaned.len() + 3) & !3;
                    let padded = if cleaned.len() < padded_len {
                        let mut s = cleaned;
                        while s.len() < padded_len {
                            s.push('=');
                        }
                        s
                    } else {
                        cleaned
                    };
                    // Decode + re-encode to guarantee clean standard base64
                    match base64::engine::general_purpose::STANDARD.decode(&padded) {
                        Ok(bytes) => {
                            let clean = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            return Some(ContentPart::image_base64(clean, mime));
                        }
                        Err(e) => {
                            tracing::warn!(
                                source = %d.source,
                                len = base64.len(),
                                error = %e,
                                "Invalid base64 image data in build_tool_messages, skipping"
                            );
                            return None;
                        }
                    }
                }
            }
            None
        })
        .collect();

    let user_msg = if !image_parts.is_empty() {
        let mut parts = vec![ContentPart::text(
            "Analyze the current situation and take appropriate actions using the available tools.",
        )];
        parts.extend(image_parts);
        Message::from_parts(MessageRole::User, parts)
    } else {
        Message::new(
            MessageRole::User,
            Content::text("Analyze the current situation and take appropriate actions using the available tools."),
        )
    };

    vec![
        Message::new(MessageRole::System, Content::text(system_prompt)),
        user_msg,
    ]
}
