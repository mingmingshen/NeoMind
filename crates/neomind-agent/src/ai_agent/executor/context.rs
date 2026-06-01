use super::*;

/// Identifies the data source that triggered an agent execution.
#[derive(Clone, Debug)]
pub struct DataSourceRef {
    /// Source type: "device", "extension", "transform", "ai"
    pub source_type: String,
    /// Source entity ID (device_id, extension_id, transform_id, ai group)
    pub source_id: String,
    /// Specific field/metric name within the source
    pub field: String,
}

/// Event data for triggering agent execution.
#[derive(Clone, Debug)]
pub struct EventTriggerData {
    /// What data source triggered this event
    pub source: DataSourceRef,
    /// The value that triggered the event
    pub value: MetricValue,
    /// When the event occurred
    pub timestamp: i64,
}

/// Configuration for `build_history_context` — controls how much history
/// to include and whether to include image analysis history.
pub(crate) struct HistoryConfig {
    pub max_history_entries: usize,
    pub max_short_term: usize,
    pub pattern_confidence: f32,
    pub max_patterns: usize,
    pub max_baselines: usize,
    pub max_user_messages: usize,
    pub include_image_history: bool,
}

impl HistoryConfig {
    /// Focused: lightweight, concise context for fast analysis on small models.
    pub(crate) fn focused() -> Self {
        Self {
            max_history_entries: 3,
            max_short_term: 3,
            pattern_confidence: 0.7,
            max_patterns: 5,
            max_baselines: 5,
            max_user_messages: 5,
            include_image_history: true,
        }
    }

    /// Free: full context for autonomous multi-round tool calling.
    pub(crate) fn free() -> Self {
        Self {
            max_history_entries: 5,
            max_short_term: 3,
            pattern_confidence: 0.6,
            max_patterns: 5,
            max_baselines: 5,
            max_user_messages: 5,
            include_image_history: true,
        }
    }
}

/// Build a unified history context string from agent memory and conversation history.
/// Used by both Focused (analyze_with_llm) and Free (build_tool_system_prompt) paths.
pub(crate) fn build_history_context(agent: &AiAgent, config: &HistoryConfig) -> String {
    let mut parts: Vec<String> = Vec::new();

    // 1. Recent execution history (conversation turns) — change-detected, collapses duplicates
    if !agent.conversation_history.is_empty() {
        let entries =
            format_changed_history(&agent.conversation_history, config.max_history_entries);
        if !entries.is_empty() {
            parts.push(format!(
                "### Recent Execution History ({} events)\n{}\n\
                 Use this to track trends and avoid repeating the same analysis.",
                entries.len(),
                entries.join("\n")
            ));
        }
    }

    // 2. Short-term memory summaries (recent execution results)
    if !agent.memory.short_term.summaries.is_empty() {
        let recent: Vec<String> = agent
            .memory
            .short_term
            .summaries
            .iter()
            .rev()
            .take(config.max_short_term)
            .map(|s| {
                let ts = chrono::DateTime::from_timestamp(s.timestamp, 0)
                    .map(|dt| dt.format("%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "??".to_string());
                let status = if s.success { "OK" } else { "FAIL" };
                format!("- [{}][{}] {}", ts, status, truncate_to(&s.conclusion, 80))
            })
            .collect();
        parts.push(format!("### Short-term Memory\n{}", recent.join("\n")));
    }

    // 3. Learned patterns (high confidence only)
    if !agent.memory.learned_patterns.is_empty() {
        let patterns: Vec<String> = agent
            .memory
            .learned_patterns
            .iter()
            .filter(|p| p.confidence >= config.pattern_confidence)
            .take(config.max_patterns)
            .map(|p| {
                format!(
                    "- [{}] {} ({:.0}%)",
                    p.pattern_type,
                    p.description,
                    p.confidence * 100.0
                )
            })
            .collect();
        if !patterns.is_empty() {
            parts.push(format!("### Learned Patterns\n{}", patterns.join("\n")));
        }
    }

    // 4. Baseline values
    if !agent.memory.baselines.is_empty() {
        let bl: Vec<String> = agent
            .memory
            .baselines
            .iter()
            .take(config.max_baselines)
            .map(|(k, v)| format!("- {}: {:.2}", k, v))
            .collect();
        parts.push(format!("### Known Baselines\n{}", bl.join("\n")));
    }

    // 5. User messages (highest priority instructions)
    if !agent.user_messages.is_empty() {
        let msgs: Vec<String> = agent
            .user_messages
            .iter()
            .rev()
            .take(config.max_user_messages)
            .map(|m| {
                let ts = chrono::DateTime::from_timestamp(m.timestamp, 0)
                    .map(|dt| dt.format("%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "??".to_string());
                format!("- [{}] {}", ts, m.content)
            })
            .collect();
        parts.push(format!(
            "### User Instructions (HIGHEST PRIORITY)\n\
             These override any conflicting rules from initial config:\n{}",
            msgs.join("\n")
        ));
    }

    // 6. Conversation summary (compressed older history preserved across evictions)
    if let Some(ref summary) = agent.conversation_summary {
        if !summary.is_empty() {
            parts.push(format!("### Earlier History Summary\n{}", summary));
        }
    }

    // 7. Image analysis history (from short-term decisions with [image_analysis] prefix)
    if config.include_image_history {
        let image_entries: Vec<String> = agent
            .memory
            .short_term
            .summaries
            .iter()
            .rev()
            .flat_map(|s| {
                let time_str = chrono::DateTime::from_timestamp(s.timestamp, 0)
                    .map(|dt| dt.format("%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| s.timestamp.to_string());
                s.decisions
                    .iter()
                    .filter(|d| d.starts_with("[image_analysis]"))
                    .map(move |d| format!("- [{}] {}", time_str, truncate_to(d, 120)))
            })
            .take(3)
            .collect();

        if !image_entries.is_empty() {
            parts.push(format!(
                "### Recent Image Analysis\n{}",
                image_entries.join("\n")
            ));
        }
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!(
            "\n## Historical Context (learn from past experience)\n{}\n",
            parts.join("\n\n")
        )
    }
}

/// Strip LLM thinking/reasoning artifacts from text.
/// Some models output their internal reasoning as plain text (e.g., "Thinking Process: ...",
/// "Let me analyze...", "## Thinking") instead of the expected structured JSON.
/// This function detects and removes such artifacts, keeping only the substantive content.
fn strip_llm_thinking(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Common LLM thinking markers (ordered by specificity)
    let thinking_markers = [
        "Thinking Process:",
        "Thinking process:",
        "thinking process:",
        "Let me analyze",
        "Let me think",
        "## Thinking",
        "## Analysis",
        "## Reasoning",
        "Let me break this down",
    ];

    // Find the earliest thinking marker and truncate there
    let mut earliest_pos = None;
    for marker in &thinking_markers {
        if let Some(pos) = cleaned.find(marker) {
            match earliest_pos {
                None => earliest_pos = Some(pos),
                Some(current) if pos < current => earliest_pos = Some(pos),
                _ => {}
            }
        }
    }

    if let Some(pos) = earliest_pos {
        // Keep content before the thinking marker
        cleaned.truncate(pos);
    }

    // Also remove common LLM meta-commentary patterns from the end
    let trailing_patterns = ["\n\nNote:", "\n\nSummary:", "\n\nIn conclusion,"];
    for pattern in &trailing_patterns {
        if let Some(pos) = cleaned.rfind(pattern) {
            // Only remove if it appears near the end (last 30% of text)
            let threshold = (cleaned.len() as f64 * 0.7) as usize;
            if pos >= threshold {
                cleaned.truncate(pos);
            }
        }
    }

    cleaned.trim().to_string()
}

pub(crate) fn clean_and_truncate_text(text: &str, max_chars: usize) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Strip LLM thinking/reasoning artifacts before further processing.
    // Some models output their internal reasoning as plain text instead of the expected JSON.
    let text = strip_llm_thinking(text);

    // First, check for obvious repetition patterns
    // If a short phrase (10-50 chars) appears 3+ times, it's likely stuck in a loop
    let chars: Vec<char> = text.chars().collect();
    let char_count = chars.len();

    // Quick check for extreme repetition (same char repeated > 50 times)
    let mut streak = 1;
    for i in 1..char_count.min(1000) {
        if chars[i] == chars[i - 1] {
            streak += 1;
            if streak > 50 {
                // High repetition detected, truncate early
                let truncated: String = chars.iter().take(i.saturating_sub(20)).collect();
                return format!("{}...[truncated]", truncated);
            }
        } else {
            streak = 1;
        }
    }

    // Check for phrase-level repetition using sliding window
    let text_lower = text.to_lowercase();
    for window_size in [10, 15, 20, 30, 50].iter() {
        if char_count < *window_size * 3 {
            continue;
        }

        let mut phrase_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for i in 0..=(char_count.saturating_sub(*window_size)) {
            let phrase: String = chars
                .iter()
                .skip(i)
                .take(*window_size)
                .collect::<String>()
                .to_lowercase();

            if !phrase.chars().all(|c| c.is_whitespace()) {
                *phrase_counts.entry(phrase).or_insert(0) += 1;
            }
        }

        // If any phrase appears 3+ times, truncate at first occurrence
        for (phrase, count) in phrase_counts.iter() {
            if *count >= 3 && phrase.len() > 10 {
                // Find first occurrence and truncate
                if let Some(pos) = text_lower.find(phrase) {
                    let safe_pos = pos.saturating_sub(50);
                    let truncated: String = chars.iter().take(safe_pos).collect();
                    return if truncated.chars().count() > max_chars {
                        format!(
                            "{}...",
                            truncated.chars().take(max_chars).collect::<String>()
                        )
                    } else {
                        truncated
                    };
                }
            }
        }
    }

    // No repetition detected, just truncate if too long
    if char_count > max_chars {
        let truncated: String = chars.iter().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        text.to_string()
    }
}

/// Truncate text to max_chars, adding "..." if needed
pub(crate) fn truncate_to(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars.saturating_sub(3)).collect();
        truncated + "..."
    }
}

/// Produce a lightweight fingerprint for comparing conclusions.
/// Normalizes to lowercase alphanumeric + first 30 chars, combined with success flag.
pub(crate) fn conclusion_fingerprint(conclusion: &str, success: bool) -> String {
    let normalized: String = conclusion
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect();
    let truncated = if normalized.len() > 30 {
        normalized[..normalized.floor_char_boundary(30)].to_string()
    } else {
        normalized
    };
    format!("{}:{}", if success { "1" } else { "0" }, truncated.trim())
}

/// Format conversation history with change-detection: consecutive turns with
/// the same conclusion fingerprint are collapsed into a single entry showing
/// the count (e.g., "×3 similar"). Only turns representing a *change* get
/// individual entries, so the limited context window is spent on information
/// that actually differs.
///
/// Returns formatted lines (newest first), up to `max_entries` unique events.
pub(crate) fn format_changed_history(
    history: &[ConversationTurn],
    max_entries: usize,
) -> Vec<String> {
    if history.is_empty() {
        return Vec::new();
    }

    let mut entries: Vec<String> = Vec::new();
    // Walk newest → oldest
    let mut idx = history.len();

    while idx > 0 && entries.len() < max_entries {
        idx -= 1;
        let turn = &history[idx];
        let fp = conclusion_fingerprint(&turn.output.conclusion, turn.success);

        let ts = chrono::DateTime::from_timestamp(turn.timestamp, 0)
            .map(|dt| dt.format("%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "??".to_string());
        let status = if turn.success { "OK" } else { "FAIL" };
        let conclusion = truncate_to(&turn.output.conclusion, 80);

        // Count how many consecutive older turns share the same fingerprint
        let mut similar_count: usize = 0;
        let mut scan = idx;
        while scan > 0 {
            scan -= 1;
            let older_fp =
                conclusion_fingerprint(&history[scan].output.conclusion, history[scan].success);
            if older_fp == fp {
                similar_count += 1;
                idx = scan; // Skip past these
            } else {
                break;
            }
        }

        if similar_count > 0 {
            let first_ts = chrono::DateTime::from_timestamp(history[idx].timestamp, 0)
                .map(|dt| dt.format("%m-%d").to_string())
                .unwrap_or_else(|| "??".to_string());
            entries.push(format!(
                "- [{} ~ {}][{}] {} (×{} similar)",
                first_ts,
                ts,
                status,
                conclusion,
                similar_count + 1
            ));
        } else {
            entries.push(format!("- [{}][{}] {}", ts, status, conclusion));
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_llm_thinking_removes_thinking_process() {
        let input = "温度正常范围。Thinking Process: 1. Analyze the Request: ...";
        let result = strip_llm_thinking(input);
        assert_eq!(result, "温度正常范围。");
    }

    #[test]
    fn test_strip_llm_thinking_no_markers() {
        let input = "设备温度25度，正常范围。";
        let result = strip_llm_thinking(input);
        assert_eq!(result, "设备温度25度，正常范围。");
    }

    #[test]
    fn test_strip_llm_thinking_let_me_analyze() {
        let input = "结论文本。Let me analyze this further...";
        let result = strip_llm_thinking(input);
        assert_eq!(result, "结论文本。");
    }

    #[test]
    fn test_clean_and_truncate_strips_thinking() {
        let input = "所有设备正常。Thinking Process: 1. **Analyze the Request:** * Input: Execution results of tools...";
        let result = clean_and_truncate_text(input, 500);
        assert!(!result.contains("Thinking Process"));
        assert!(result.contains("所有设备正常"));
    }
}
