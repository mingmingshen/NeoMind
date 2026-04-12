
pub(crate) fn extract_command_from_description(description: &str) -> Option<String> {
    // Use a helper to find patterns case-insensitively
    // and return the trimmed result as a String
    pub(crate) fn find_and_extract(text: &str, pattern: &str, pattern_len: usize) -> Option<String> {
        let text_lower = text.to_lowercase();
        if let Some(idx) = text_lower.find(pattern) {
            let after = &text[idx + pattern_len..];
            // Trim leading whitespace and extract first word
            let cmd = after.split_whitespace().next().unwrap_or(after);
            // Trim trailing non-alphanumeric characters (except underscore)
            let cmd = cmd.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if !cmd.is_empty() {
                return Some(cmd.to_string());
            }
        }
        None
    }

    // Try patterns in order of specificity
    find_and_extract(description, "command:", 8)
        .or_else(|| find_and_extract(description, "execute:", 8))
        .or_else(|| find_and_extract(description, "execute ", 8))
}

pub(crate) fn extract_device_from_description(description: &str) -> Option<String> {
    pub(crate) fn find_and_extract(text: &str, pattern: &str, pattern_len: usize) -> Option<String> {
        let text_lower = text.to_lowercase();
        if let Some(idx) = text_lower.find(pattern) {
            let after = &text[idx + pattern_len..];
            let device = after.split_whitespace().next().unwrap_or(after);
            let device = device.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if !device.is_empty() {
                return Some(device.to_string());
            }
        }
        None
    }

    find_and_extract(description, "device:", 7)
        .or_else(|| find_and_extract(description, "device ", 7))
        .or_else(|| find_and_extract(description, "on ", 3))
}

pub(crate) fn json_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(obj) => {
            // Serialize object to a readable format
            match serde_json::to_string_pretty(obj) {
                Ok(s) => s,
                Err(_) => value.to_string(),
            }
        }
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Array(arr) => {
            // For arrays, serialize to JSON string
            match serde_json::to_string(arr) {
                Ok(s) => s,
                Err(_) => value.to_string(),
            }
        }
    }
}

pub(crate) fn extract_string_field(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    obj.get(key).map(json_value_to_string).unwrap_or_default()
}

pub(crate) fn sanitize_json_string(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Handle escaped characters - pass them through
            '\\' => {
                result.push(ch);
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            }
            // Replace illegal control characters (0x00-0x1F) except for
            // tab (0x09), newline (0x0A), and carriage return (0x0D)
            '\u{0000}'..='\u{0008}' | '\u{000B}' | '\u{000C}' | '\u{000E}'..='\u{001F}' => {
                // Replace with space to preserve string structure
                result.push(' ');
            }
            // Pass through all other characters including valid whitespace
            _ => result.push(ch),
        }
    }

    result
}

pub(crate) fn extract_json_from_mixed_text(text: &str) -> Option<String> {
    // Find the first '{' that starts a JSON object
    let start_idx = text.find('{')?;
    let potential_json = &text[start_idx..];

    // Count braces to find the matching closing brace
    let mut open_braces = 0;
    let mut in_string = false;
    let mut escape_next = false;
    let mut end_char_idx = 0;

    for (i, ch) in potential_json.chars().enumerate() {
        match ch {
            '\\' if in_string => escape_next = true,
            '"' if !escape_next => in_string = !in_string,
            '{' if !in_string => open_braces += 1,
            '}' if !in_string => {
                open_braces -= 1;
                if open_braces == 0 {
                    end_char_idx = i + 1;
                    break;
                }
            }
            _ => {}
        }
        if escape_next && ch != '\\' {
            escape_next = false;
        }
    }

    if end_char_idx > 0 {
        // Use character index to safely extract substring (UTF-8 safe)
        let json_str: String = potential_json.chars().take(end_char_idx).collect();
        // Validate it's actually JSON
        if serde_json::from_str::<serde_json::Value>(&json_str).is_ok() {
            return Some(json_str);
        }
    }

    None
}

pub(crate) fn try_recover_truncated_json(json_str: &str) -> Option<(String, bool)> {
    let trimmed = json_str.trim();

    // First, try to close any open objects/arrays
    let mut recovered = trimmed.to_string();
    let mut open_braces: usize = 0;
    let mut open_brackets: usize = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in trimmed.chars() {
        match ch {
            '\\' if in_string => escape_next = true,
            '"' if !escape_next => in_string = !in_string,
            '{' if !in_string => open_braces += 1,
            '}' if !in_string => open_braces = open_braces.saturating_sub(1),
            '[' if !in_string => open_brackets += 1,
            ']' if !in_string => open_brackets = open_brackets.saturating_sub(1),
            _ => {}
        }
        if escape_next && ch != '\\' {
            escape_next = false;
        }
    }

    // If no unclosed braces, JSON might be complete
    if open_braces == 0 && open_brackets == 0 {
        // Still might be truncated mid-string, try parsing
        if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
            return Some((trimmed.to_string(), false));
        }
    }

    // Try to close the objects
    for _ in 0..open_brackets {
        recovered.push(']');
    }
    for _ in 0..open_braces {
        recovered.push('}');
    }

    // Check if recovered JSON is valid
    if serde_json::from_str::<serde_json::Value>(&recovered).is_ok() {
        return Some((recovered, true));
    }

    // Try more aggressive recovery: find the last complete "step" object
    // This handles cases where the JSON is truncated in the middle of reasoning_steps
    if let Some(last_complete_idx) = trimmed.rfind(r#"  }"#) {
        let truncated = &trimmed[..last_complete_idx + 4];
        // Try to close the arrays and objects
        let mut closed = truncated.to_string();
        if trimmed.contains("reasoning_steps") {
            closed.push_str("\n  ]");
        }
        if trimmed.contains("decisions") {
            closed.push_str(",\n  \"decisions\": []");
        }
        closed.push_str("\n}");
        if serde_json::from_str::<serde_json::Value>(&closed).is_ok() {
            return Some((closed, true));
        }
    }

    // Last resort: return None to signal using raw text fallback
    None
}

fn extract_analysis_fields(
    parsed: &serde_json::Value,
    fallback_text: &str,
) -> (String, String, f32) {
    let situation_analysis = parsed
        .get("situation_analysis")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let conclusion = parsed
        .get("conclusion")
        .and_then(|v| v.as_str())
        .unwrap_or(fallback_text)
        .to_string();
    let confidence = parsed
        .get("confidence")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or(0.7);
    (situation_analysis, conclusion, confidence)
}

pub(crate) fn parse_final_tool_response(text: &str) -> (String, String, f32) {
    // Try to extract a JSON code block from the text
    if let Some(json_str) = extract_json_from_codeblock(text) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            return extract_analysis_fields(&parsed, text);
        }
    }

    // Try to parse the entire text as JSON (in case no code block wrapping)
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text.trim()) {
        return extract_analysis_fields(&parsed, text);
    }

    // Natural language response — the entire text is the conclusion.
    // Try to split into situation_analysis (first paragraph) and conclusion (rest).
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new(), 0.5);
    }

    // Split on double-newline to separate analysis from conclusion
    if let Some(pos) = trimmed.find("\n\n") {
        let analysis = trimmed[..pos].trim().to_string();
        let conclusion = trimmed[pos + 2..].trim().to_string();
        if !conclusion.is_empty() {
            return (analysis, conclusion, 0.7);
        }
    }

    // Single paragraph — use as both analysis and conclusion
    (String::new(), trimmed.to_string(), 0.7)
}

/// Extract JSON string from text that may be wrapped in markdown code blocks.
/// Handles `\`\`\`json ... \`\`\`` and plain `\`\`\` ... \`\`\`` wrappers.
pub(crate) fn extract_json_from_codeblock(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.contains("```json") {
        trimmed
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .map(|s| s.trim())
    } else if trimmed.contains("```") {
        trimmed.split("```").nth(1).map(|s| s.trim())
    } else {
        None
    }
}

pub(crate) fn summarize_tool_output(data: &serde_json::Value, tool_name: &str) -> String {
    if let Some(obj) = data.as_object() {
        if let Some(count) = obj.get("count") {
            format!("{} returned {} items", tool_name, count)
        } else if let Some(msg) = obj.get("message").and_then(|m| m.as_str()) {
            format!("{}: {}", tool_name, msg)
        } else if let Some(points) = obj.get("points").and_then(|p| p.as_array()) {
            format!("{} retrieved {} data points", tool_name, points.len())
        } else if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
            let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or(id);
            format!("Retrieved: {} ({})", name, id)
        } else {
            let keys: Vec<&str> = obj.keys().take(3).map(|s| s.as_str()).collect();
            if keys.is_empty() {
                format!("{} completed", tool_name)
            } else {
                format!("{} Fields: {}", tool_name, keys.join(", "))
            }
        }
    } else {
        format!("{} completed", tool_name)
    }
}
