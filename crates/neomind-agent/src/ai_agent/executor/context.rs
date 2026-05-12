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

/// State for tracking tool chaining progress
#[derive(Debug, Clone)]
pub struct ChainState {
    /// Current depth in the chain
    pub(crate) depth: usize,
    /// Results from previous rounds that can be used as input
    pub(crate) previous_results: Vec<ChainResult>,
    /// Maximum depth allowed
    pub(crate) max_depth: usize,
}

/// A result from one step in the chain that can be used as input
#[derive(Debug, Clone)]
pub struct ChainResult {
    /// Action that produced this result
    action_type: String,
    /// Target of the action
    target: String,
    /// Result data (if any)
    result: Option<String>,
    /// Whether the action succeeded
    success: bool,
}

impl ChainState {
    pub(crate) fn new(max_depth: usize) -> Self {
        Self {
            depth: 0,
            previous_results: Vec::new(),
            max_depth,
        }
    }

    pub(crate) fn can_continue(&self) -> bool {
        self.depth < self.max_depth
    }

    pub(crate) fn advance(&mut self, results: &[neomind_storage::ActionExecuted]) {
        self.depth += 1;
        for action in results {
            self.previous_results.push(ChainResult {
                action_type: action.action_type.clone(),
                target: action.target.clone(),
                result: action.result.clone(),
                success: action.success,
            });
        }
    }

    /// Format previous results as context for the next LLM round
    pub(crate) fn format_as_context(&self) -> String {
        if self.previous_results.is_empty() {
            return String::new();
        }

        let mut context =
            String::from("\n\n## Previous Tool Execution Results (Tool Chaining)\n\n");
        context.push_str(&format!("Currently on round {}.\n\n", self.depth));

        for (i, result) in self.previous_results.iter().enumerate() {
            context.push_str(&format!(
                "### Execution Step {} - {}\n",
                i + 1,
                result.action_type
            ));
            context.push_str(&format!("- **Target**: {}\n", result.target));
            context.push_str(&format!(
                "- **Status**: {}\n",
                if result.success { "Success" } else { "Failed" }
            ));
            if let Some(ref result_str) = result.result {
                // Only include non-trivial results
                if !result_str.is_empty() && result_str != "Command sent successfully" {
                    // Sanitize base64/image data to prevent context bloat
                    let sanitized =
                        crate::agent::streaming::sanitize_tool_result_for_prompt(result_str);
                    let display = if sanitized.chars().count() > 2000 {
                        crate::agent::streaming::truncate_result_utf8(&sanitized, 2000)
                    } else {
                        sanitized
                    };
                    context.push_str(&format!("- **Result**: {}\n", display));
                }
            }
            context.push('\n');
        }

        context.push_str(
            "Based on the above execution results, determine if further operations are needed. ",
        );
        context.push_str("If previous operations have completed the goal, or there are no more meaningful operations, please explain and end. ");
        context.push_str("If you need to continue, please clearly state what to do next.\n");

        context
    }
}

#[allow(dead_code)]
pub(crate) fn build_medium_term_summary(
    memory: &AgentMemory,
    _current_analysis: &str,
    current_conclusion: &str,
) -> String {
    let mut parts = Vec::new();

    // Key metrics tracked
    if !memory.baselines.is_empty() {
        parts.push(format!(
            "Baseline metrics: {}",
            memory
                .baselines
                .iter()
                .map(|(k, v)| format!("{}={:.1}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Pattern summary - optimized to avoid intermediate Vec allocation
    if !memory.learned_patterns.is_empty() {
        let pattern_types: std::collections::HashSet<_> = memory
            .learned_patterns
            .iter()
            .map(|p| p.pattern_type.as_str())
            .collect();
        parts.push(format!(
            "Identified patterns: {}",
            pattern_types.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    // Current status
    if !current_conclusion.is_empty() {
        parts.push(format!("Current status: {}", current_conclusion));
    }

    parts.join("; ")
}

#[allow(dead_code)]
pub(crate) fn should_compact_context(history_context: &str, threshold_chars: usize) -> bool {
    // Rough estimation: 1 token ≈ 3 characters for Chinese/English mixed
    let estimated_tokens = history_context.chars().count() / 3;
    estimated_tokens > threshold_chars
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

/// Compact history context while preserving key information.
///
/// COMPRESSION STRATEGY:
/// 1. Ultra-compact format - no section titles, minimal punctuation
/// 2. Merge similar information - don't repeat the same conclusion
/// 3. Use codes instead of descriptions - "T30" instead of "温度超过30度"
/// 4. Selective retention - only most relevant info
///
/// Target: < 200 characters for small models (qwen3:1.7b)
#[allow(dead_code)]
pub(crate) fn compact_history_context(_history_context: &str, memory: &AgentMemory) -> String {
    let mut parts = Vec::new();

    // === STRATEGY 1: Recent trend ===
    // Instead of listing each execution, show the pattern
    if !memory.short_term.summaries.is_empty() {
        let last_3: Vec<_> = memory.short_term.summaries.iter().rev().take(3).collect();

        // Count patterns
        let success_count = last_3.iter().filter(|s| s.success).count();
        let total = last_3.len();

        // Get most recent conclusion (most relevant)
        if let Some(latest) = last_3.first() {
            // Ultra-compact: "Recent: 3 runs, 2 success, latest: ..."
            parts.push(format!(
                "Last{}: {}ok, {}",
                total,
                success_count,
                truncate_to(&latest.conclusion, 30)
            ));
        }
    }

    // === STRATEGY 2: Most important pattern ===
    // Instead of all patterns, just show the highest confidence one
    let patterns = if !memory.long_term.patterns.is_empty() {
        &memory.long_term.patterns
    } else {
        &memory.learned_patterns
    };

    if !patterns.is_empty() {
        if let Some(best) = patterns.iter().max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            // Ultra-compact: "Pattern: temp>30 alert (80%)"
            parts.push(format!(
                "Pattern: {} ({}%)",
                truncate_to(&best.description, 25),
                (best.confidence * 100.0) as u32
            ));
        }
    }

    // === STRATEGY 3: Key baseline ===
    // Only show baselines that are relevant to common metrics
    if !memory.baselines.is_empty() {
        // Show at most 2 most relevant baselines
        let baseline_items: Vec<_> = memory.baselines.iter().take(2).collect();

        if !baseline_items.is_empty() {
            let baseline_str = baseline_items
                .iter()
                .map(|(k, v)| format!("{}={}", k, **v as i32))
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("Baseline: {}", baseline_str));
        }
    }

    // Join with minimal separator
    if parts.is_empty() {
        String::new()
    } else {
        parts.join(" | ")
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
            let older_fp = conclusion_fingerprint(
                &history[scan].output.conclusion,
                history[scan].success,
            );
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
                first_ts, ts, status, conclusion, similar_count + 1
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
