//! Streaming response processing with thinking tag support.
//!
//! This module includes safeguards against infinite LLM loops:
//! - Global stream timeout
//! - Maximum thinking content length
//! - Maximum tool call iterations
//! - Repetition detection

use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{Stream, StreamExt};
use serde_json::Value;
use tokio::sync::RwLock;

use super::planner::types::ExecutionPlan;
use super::staged::{IntentCategory, IntentClassifier};
use super::tool_parser::{parse_tool_calls, remove_tool_calls_from_response};
use super::types::{
    AgentEvent, AgentInternalState, AgentMessage, AgentMessageImage, LargeDataCache, ToolCall,
};
use crate::error::{NeoMindError, Result};
use crate::llm::LlmInterface;

// Type aliases to reduce complexity
pub type SharedLlm = Arc<RwLock<LlmInterface>>;
pub type ToolResultStream = Pin<Box<dyn Stream<Item = (String, String)> + Send>>;
pub type EventChannel = tokio::sync::mpsc::UnboundedSender<AgentEvent>;

// Re-export compaction types for use in other modules
pub use neomind_core::llm::compaction::{
    CompactionConfig,
    MessagePriority,
    // Note: estimate_tokens is defined locally below to use the tokenizer module
};

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

            // Tool iterations limit - increased to support complex multi-step queries
            max_tool_iterations: 10,

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

/// Clean up repetitive thinking content by removing excessive repeated phrases
/// This preserves the core thinking while removing the repetitive noise
pub fn cleanup_thinking_content(thinking: &str) -> String {
    if thinking.len() < 200 {
        return thinking.to_string();
    }

    let mut result = thinking.to_string();
    let mut reduced = true;

    // Pass 1: Remove immediate repetitions of the same phrase
    // This handles cases like "可能可能可能可能" -> "可能"
    while reduced {
        reduced = false;
        let original = result.clone();

        // Common repetitive patterns in qwen3-vl:2b thinking
        let patterns = [
            ("可能可能", "可能"),
            ("或者或者", "或者"),
            ("也许也许", "也许"),
            ("温度温度", "温度"),
            ("。。", "。"),
            ("，，", "，"),
            ("??", "?"),
            ("  ", " "),
        ];

        for (pattern, replacement) in patterns {
            result = result.replace(pattern, replacement);
        }

        if result != original {
            reduced = true;
        }
    }

    // Pass 2: Limit consecutive occurrences of common filler words
    // Using character-based iteration to avoid UTF-8 issues
    let filler_words = [
        ("可能", 3), // Max 3 consecutive "可能"
        ("或者", 2), // Max 2 consecutive "或者"
        ("也许", 2),
    ];

    for (word, max_consecutive) in filler_words {
        let chars: Vec<char> = result.chars().collect();
        let mut new_result = String::new();
        let mut consecutive = 0;
        let mut last_was_word = false;
        let mut char_idx = 0;

        while char_idx < chars.len() {
            // Check if the word starts at this position
            let word_chars: Vec<char> = word.chars().collect();
            let matches = if char_idx + word_chars.len() <= chars.len() {
                chars[char_idx..char_idx + word_chars.len()] == word_chars[..]
            } else {
                false
            };

            if matches {
                if last_was_word {
                    consecutive += 1;
                    if consecutive <= max_consecutive {
                        for &ch in &word_chars {
                            new_result.push(ch);
                        }
                    }
                } else {
                    consecutive = 1;
                    last_was_word = true;
                    for &ch in &word_chars {
                        new_result.push(ch);
                    }
                }
                char_idx += word_chars.len();
            } else {
                consecutive = 0;
                last_was_word = false;
                new_result.push(chars[char_idx]);
                char_idx += 1;
            }
        }
        result = new_result;
    }

    // Pass 3: If still too long, truncate with ellipsis at char boundary
    if result.chars().count() > 500 {
        let _char_count = result.chars().count();
        // Take first 500 chars and add ellipsis
        result = result.chars().take(500).collect::<String>();
        result.push_str("...");
    }

    result
}

/// Detect JSON tool calls in buffer.
///
/// Looks for JSON array format: [{"name": "tool", "arguments": {...}}, ...]
/// Returns Some((start_pos, json_text, remaining_buffer)) if found, None otherwise.
fn detect_json_tool_calls(buffer: &str) -> Option<(usize, String, String)> {
    // Find the first '[' that might start a JSON array
    let start = buffer.find('[')?;

    // Find the matching closing ']' by counting brackets
    let mut bracket_count = 0;
    let mut end = None;

    for (i, c) in buffer[start..].char_indices() {
        if c == '[' {
            bracket_count += 1;
        } else if c == ']' {
            bracket_count -= 1;
            if bracket_count == 0 {
                end = Some(start + i + 1);
                break;
            }
        }
    }

    let end = end?;

    // Extract the JSON array
    let json_str = buffer[start..end].to_string();

    // Check if it looks like a tool call (has "name", "tool", or "function" key)
    if !json_str.contains("\"name\"")
        && !json_str.contains("\"tool\"")
        && !json_str.contains("\"function\"")
    {
        return None;
    }

    // Verify it's valid JSON
    let json_value = serde_json::from_str::<serde_json::Value>(&json_str).ok()?;

    // Validate that at least one element has a valid string "name" field
    // This prevents false positives from malformed JSON like [{"name":"[...]"}]
    if let Some(arr) = json_value.as_array() {
        let has_valid_tool_call = arr.iter().any(|item| {
            if let Some(obj) = item.as_object() {
                // Check if "name", "tool", or "function" field exists and is a valid string
                let name_value = obj
                    .get("name")
                    .or_else(|| obj.get("tool"))
                    .or_else(|| obj.get("function"));

                if let Some(name) = name_value {
                    if let Some(name_str) = name.as_str() {
                        // Ensure the name is a simple string (not a JSON string containing nested JSON)
                        // A valid tool name should not start with '[' or '{'
                        let trimmed = name_str.trim();
                        return !trimmed.starts_with('[') && !trimmed.starts_with('{');
                    }
                }
            }
            false
        });

        if !has_valid_tool_call {
            return None;
        }
    } else {
        return None;
    }

    // Return start position, the JSON, and remaining buffer
    let remaining = buffer[end..].to_string();
    Some((start, json_str, remaining))
}

/// Simple in-memory cache for tool results with TTL and size limit
#[derive(Debug)]
pub struct ToolResultCache {
    entries: HashMap<String, (crate::toolkit::ToolOutput, Instant)>,
    ttl: Duration,
    max_entries: usize,
}

impl ToolResultCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
            max_entries: 1000, // Prevent unbounded memory growth
        }
    }

    fn get(&self, key: &str) -> Option<crate::toolkit::ToolOutput> {
        self.entries.get(key).and_then(|(result, timestamp)| {
            if timestamp.elapsed() < self.ttl {
                Some(result.clone())
            } else {
                None
            }
        })
    }

    fn insert(&mut self, key: String, value: crate::toolkit::ToolOutput) {
        // Enforce size limit - remove oldest entry if at capacity
        if self.entries.len() >= self.max_entries {
            // Remove the oldest entry (first key in iteration)
            if let Some(oldest_key) = self.entries.keys().next().cloned() {
                self.entries.remove(&oldest_key);
            }
        }
        self.entries.insert(key, (value, Instant::now()));
    }

    fn cleanup_expired(&mut self) {
        self.entries
            .retain(|_, (_, timestamp)| timestamp.elapsed() < self.ttl);

        // Also enforce size limit after cleanup
        while self.entries.len() > self.max_entries {
            if let Some(oldest_key) = self.entries.keys().next().cloned() {
                self.entries.remove(&oldest_key);
            }
        }
    }

    /// Get current cache size (for monitoring)
    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.entries.len()
    }

    /// Generate cache key from tool name and arguments.
    /// Sorts object keys to ensure consistent keys regardless of parameter order.
    fn make_key(name: &str, arguments: &Value) -> String {
        // For objects, sort keys to ensure consistent cache keys
        if let Some(obj) = arguments.as_object() {
            let mut sorted_pairs: Vec<_> = obj.iter().collect();
            sorted_pairs.sort_by(|a, b| a.0.cmp(b.0));

            let sorted_obj: serde_json::Map<String, Value> = sorted_pairs
                .into_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            format!(
                "{}:{}",
                name,
                serde_json::to_string(&sorted_obj).unwrap_or_default()
            )
        } else {
            // For non-objects (arrays, strings, numbers, etc.), use as-is
            format!("{}:{}", name, arguments)
        }
    }
}

/// Tools that should NOT be cached (e.g., commands that change state)
const NON_CACHEABLE_TOOLS: &[&str] = &[
    "send_command",
    "execute_command",
    "set_device_state",
    "toggle_device",
    "delete_device",
];

/// Check if tool results should bypass LLM and return directly.
/// Always returns false — all results go through LLM Phase 2.
fn should_return_directly(_tool_results: &[(String, String)]) -> bool {
    false
}

fn is_tool_cacheable(name: &str) -> bool {
    !NON_CACHEABLE_TOOLS.contains(&name)
}

/// Max length of tool result text to inject into Phase 2 prompt (avoid context overflow).
const PHASE2_TOOL_RESULT_MAX_LEN: usize = 8000;

/// Minimum size (bytes) for a result to be considered large enough to strip base64 from.
const BASE64_STRIP_THRESHOLD: usize = 4096;

// ---------------------------------------------------------------------------
// Base64 / image stripping for Phase 2 prompts
// ---------------------------------------------------------------------------

/// Strip base64/image data from a tool result for safe inclusion in Phase 2 LLM prompts.
///
/// Base64 image data wastes LLM context tokens and causes the model to reproduce
/// raw data in its response. This function:
/// - For JSON results: walks the tree and replaces base64/image strings with `[image data, {size}]`
/// - For text with `data:image/...` URLs: replaces URLs with size markers
/// - Preserves all non-binary data (numbers, text, metadata)
pub(crate) fn sanitize_tool_result_for_prompt(result: &str) -> String {
    // Fast path: small results without base64 indicators pass through
    if result.len() < BASE64_STRIP_THRESHOLD
        && !result.contains("base64")
        && !result.contains("data:image/")
    {
        return result.to_string();
    }

    // Try JSON path: parse, strip, re-serialize
    if result.starts_with('{') || result.starts_with('[') {
        if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(result) {
            if strip_base64_from_json_value(&mut value) {
                if let Ok(stripped) = serde_json::to_string(&value) {
                    return stripped;
                }
            }
        }
        // JSON parse succeeded but no base64 found, or serialization failed — fall through
    }

    // Text containing data:image URLs
    if result.contains("data:image/") {
        return replace_data_image_urls(result);
    }

    result.to_string()
}

/// Recursively strip base64/image data from a JSON value tree.
/// Returns `true` if any value was modified.
fn strip_base64_from_json_value(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            let mut modified = false;

            // Collect keys whose values are base64/image data
            let replacements: Vec<(String, serde_json::Value)> = map
                .iter()
                .filter_map(|(k, v)| {
                    let s = v.as_str()?;
                    if s.starts_with("data:image/") {
                        return Some((
                            k.clone(),
                            serde_json::json!(format!("[image data, {}]", humanize_bytes(s.len()))),
                        ));
                    }
                    if is_large_base64_string(s) {
                        return Some((
                            k.clone(),
                            serde_json::json!(format!(
                                "[base64 data, {}]",
                                humanize_bytes(s.len())
                            )),
                        ));
                    }
                    None
                })
                .collect();

            for (key, replacement) in replacements {
                map.insert(key, replacement);
                modified = true;
            }

            // Recurse into child values
            for v in map.values_mut() {
                if strip_base64_from_json_value(v) {
                    modified = true;
                }
            }
            modified
        }
        serde_json::Value::Array(arr) => {
            let mut modified = false;
            for v in arr.iter_mut() {
                if strip_base64_from_json_value(v) {
                    modified = true;
                }
            }
            modified
        }
        _ => false,
    }
}

/// Check if a string looks like large base64 data (>10KB, valid base64 alphabet).
fn is_large_base64_string(s: &str) -> bool {
    if s.len() <= 10_000 {
        return false;
    }
    // Sample first 200 chars to check base64 alphabet
    let sample_end = s.len().min(200);
    let sample = &s[..sample_end];
    sample
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

/// Replace `data:image/...;base64,...` URLs in a text with size markers.
fn replace_data_image_urls(text: &str) -> String {
    let mut result = text.to_string();
    while let Some(start) = result.find("data:image/") {
        // Find end of the URL (whitespace, quote, or end of string)
        let end = result[start..]
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
            .map(|i| start + i)
            .unwrap_or(result.len());
        let data_len = end - start;
        let replacement = format!("[image data, {}]", humanize_bytes(data_len));
        result.replace_range(start..end, &replacement);
    }
    result
}

/// Format byte count as human-readable string (e.g., "2.3MB", "512B").
fn humanize_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;
    if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// UTF-8 safe truncation for tool result text.
/// Returns the truncated string with an ellipsis suffix if truncated.
pub(crate) fn truncate_result_utf8(result: &str, max_chars: usize) -> String {
    if result.chars().count() <= max_chars {
        return result.to_string();
    }
    let truncated: String = result.chars().take(max_chars).collect();
    format!(
        "{}... (truncated, total {} chars)",
        truncated,
        result.chars().count()
    )
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

/// Build Phase 2 user prompt with tool results explicitly included so the second LLM always sees them.
pub(crate) fn build_phase2_prompt_with_tool_results(
    original_question: Option<String>,
    tool_call_results: &[(String, String)],
) -> String {
    let question =
        original_question.unwrap_or_else(|| "请总结以上工具执行结果，给出完整的回复。".to_string());
    if tool_call_results.is_empty() {
        return question;
    }

    // Count tools to provide contextual guidance
    let tool_count = tool_call_results.len();

    // 检查是否有错误结果
    let has_errors = tool_call_results.iter().any(|(_, result)| {
        result.contains("failed")
            || result.contains("error")
            || result.contains("失败")
            || result.contains("错误")
    });

    let mut block = if tool_count > 1 {
        format!(
            "\n\n[Executed {} tools, please consolidate all results to answer the user's question]\n",
            tool_count
        )
    } else {
        String::from(
            "\n\n[Tool Execution Results - You MUST analyze and provide a complete response]\n",
        )
    };

    // Add mandatory response instruction
    block.push_str("IMPORTANT: You must analyze the tool results and provide a user-friendly complete response. Don't just say \"execution completed\".\n\n");

    // If there are errors, add special instructions
    if has_errors {
        block.push_str("Note: Tool execution failed.\n\n");
        block.push_str("Handling Strategy:\n");
        block.push_str(
            "1. First check if the user's original request already contains all necessary information (device, metric, condition, action)\n",
        );
        block.push_str("2. If the user has provided enough information, it's a DSL format issue. You should tell the user \"I understand, let me regenerate\" and then inform them the rule was created successfully\n");
        block.push_str("3. Only ask follow-up questions when the user is actually missing key information, and ask everything at once\n");
        block.push_str("4. Don't show technical error messages to the user\n");
        block.push_str(
            "5. Respond in a friendly manner, don't mechanically repeat the question list\n\n",
        );
    }

    for (name, result) in tool_call_results {
        // Sanitize base64/image data before including in LLM prompt
        let sanitized = sanitize_tool_result_for_prompt(result);
        let r = if sanitized.len() > PHASE2_TOOL_RESULT_MAX_LEN {
            truncate_result_utf8(&sanitized, PHASE2_TOOL_RESULT_MAX_LEN)
        } else {
            sanitized
        };
        block.push_str(&format!("[{}]\n{}\n\n", name, r));
    }

    // Add explicit instruction for multi-tool scenarios
    if tool_count > 1 {
        block.push_str("IMPORTANT: Please extract the most relevant information from the above tool results based on the user's original question. ");
        block.push_str("If the user asks for specific device data, prioritize showing that device's detailed data rather than a device list.");
    }

    question + &block
}

/// Build Phase 2 prompt with accumulated multi-round tool results.
///
/// This is used when the multi-round loop ends (iteration limit, consecutive
/// duplicates, or non-complex intent) to generate a comprehensive summary
/// covering ALL collected data, not just the last round.
fn build_phase2_summary_prompt(
    original_question: Option<String>,
    all_results: &[(String, String)],
    total_rounds: usize,
    end_reason: &str,
) -> String {
    let question =
        original_question.unwrap_or_else(|| "请总结以上工具执行结果，给出完整的回复。".to_string());

    if all_results.is_empty() {
        return question;
    }

    let mut block = format!(
        "\n\n[Completed {} rounds of tool execution (ended: {}), {} tool results collected]\n",
        total_rounds,
        end_reason,
        all_results.len()
    );

    block.push_str(
        "IMPORTANT: You MUST provide a COMPLETE summary of ALL tool results below. \
         Do NOT mention that tools were called or that execution ended - just present the data naturally.\n\n"
    );

    for (name, result) in all_results {
        // Sanitize base64/image data before including in LLM prompt
        let sanitized = sanitize_tool_result_for_prompt(result);
        let r = if sanitized.len() > PHASE2_TOOL_RESULT_MAX_LEN {
            truncate_result_utf8(&sanitized, PHASE2_TOOL_RESULT_MAX_LEN)
        } else {
            sanitized
        };
        block.push_str(&format!("[{}]\n{}\n\n", name, r));
    }

    block.push_str(&format!(
        "\nPlease organize the above data to answer: {}",
        question
    ));

    question + &block
}

/// Detect if Phase 2 LLM response is hallucinated (doesn't match actual tool results)
/// Returns true if hallucination is detected, indicating we should use fallback formatter
fn detect_hallucination(phase2_response: &str, tool_results: &[(String, String)]) -> bool {
    if tool_results.len() != 1 {
        return false; // Only detect for single-tool results
    }

    let (tool_name, tool_result) = &tool_results[0];

    match tool_name.as_str() {
        "list_agents" => {
            // Parse actual agent names from tool result
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(tool_result) {
                if let Some(agents) = json_value.get("agents").and_then(|a| a.as_array()) {
                    // Extract actual agent names
                    let actual_names: Vec<&str> = agents
                        .iter()
                        .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                        .collect();

                    // If response doesn't contain any actual agent names, it's hallucinated
                    if actual_names.is_empty() {
                        return false; // Can't determine
                    }

                    // Check if any actual agent name appears in the response
                    let has_match = actual_names.iter().any(|name| {
                        phase2_response.contains(name)
                            || phase2_response.contains(&format!("**{}**", name))
                    });

                    // Also check for common hallucination patterns
                    let has_hallucination_pattern = phase2_response.contains("agent_1")
                        || phase2_response.contains("agent_2")
                        || (phase2_response.contains("Agent ID") && !has_match);

                    !has_match || has_hallucination_pattern
                } else {
                    false
                }
            } else {
                false
            }
        }
        _ => false,
    }
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
        let is_message_list = messages.first().map_or(false, |m| {
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
    if json.is_object() {
        format_json_data(json, response);
    } else if json.is_array() {
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
                "device_discover" => {
                    // Format device_discover result with summary and device list
                    if let Some(summary) = json_value.get("summary") {
                        // Extract summary statistics
                        let total = summary.get("total").and_then(|t| t.as_u64()).unwrap_or(0);
                        let online = summary.get("online").and_then(|o| o.as_u64()).unwrap_or(0);
                        let offline = summary.get("offline").and_then(|o| o.as_u64()).unwrap_or(0);

                        response.push_str(&format!("Device Overview ({} total)\n\n", total));
                        response
                            .push_str(&format!("- Online: {} | Offline: {}\n\n", online, offline));

                        // Show device types
                        if let Some(by_type) = summary.get("by_type").and_then(|b| b.as_object()) {
                            response.push_str("**By Type**:\n");
                            for (device_type, count) in by_type.iter() {
                                if let Some(count) = count.as_u64() {
                                    response
                                        .push_str(&format!("- {}: {} units\n", device_type, count));
                                }
                            }
                            response.push('\n');
                        }
                    }

                    // List devices (handle both direct array and truncated nested structure)
                    if let Some(devices) = extract_array(&json_value, "devices") {
                        response.push_str("**Device List**:\n\n");
                        for device in devices {
                            let id = device
                                .get("id")
                                .and_then(|i| i.as_str())
                                .unwrap_or("unknown");
                            let name = device
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown");
                            let device_type = device
                                .get("device_type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("unknown");
                            let status = device
                                .get("status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("unknown");

                            response.push_str(&format!(
                                "- **{}** ({}) - {} - {}\n",
                                name, id, device_type, status
                            ));
                        }
                    }
                }
                "list_devices" => {
                    // Format device list as a table (legacy format)
                    // Handle both direct array and truncated nested structure
                    if let Some(devices) = extract_array(&json_value, "devices") {
                        response.push_str(&format!("## Device List ({} total)\n\n", devices.len()));
                        response.push_str("| Device Name | Status | Type |\n");
                        response.push_str("|-------------|--------|------|\n");
                        for device in devices {
                            let name = device
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown");
                            let status = device
                                .get("status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("unknown");
                            let device_type = device
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("unknown");
                            response.push_str(&format!(
                                "| {} | {} | {} |\n",
                                name, status, device_type
                            ));
                        }
                    } else {
                        response.push_str("No devices found.\n");
                    }
                }
                "list_rules" => {
                    // Format rule list (handle both direct array and truncated nested structure)
                    if let Some(rules) = extract_array(&json_value, "rules") {
                        response
                            .push_str(&format!("## Automation Rules ({} total)\n\n", rules.len()));
                        for rule in rules {
                            let name = rule
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown");
                            let enabled = rule
                                .get("enabled")
                                .and_then(|e| e.as_bool())
                                .unwrap_or(false);
                            let status = if enabled { "[Enabled]" } else { "!! Disabled" };
                            response.push_str(&format!("- **{}** {}\n", name, status));
                        }
                    } else if let Some(count) = json_value.get("count").and_then(|c| c.as_u64()) {
                        response.push_str(&format!("## Automation Rules ({} total)\n", count));
                    } else {
                        response.push_str("No automation rules found.\n");
                    }
                }
                "list_scenarios" => {
                    // Handle both direct array and truncated nested structure
                    if let Some(scenarios) = extract_array(&json_value, "scenarios") {
                        response
                            .push_str(&format!("## Scenario List ({} total)\n\n", scenarios.len()));
                        for scenario in scenarios {
                            let name = scenario
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown");
                            response.push_str(&format!("- {}\n", name));
                        }
                    } else {
                        response.push_str("No scenarios found.\n");
                    }
                }
                "list_workflows" => {
                    // Handle both direct array and truncated nested structure
                    if let Some(workflows) = extract_array(&json_value, "workflows") {
                        response
                            .push_str(&format!("## Workflow List ({} total)\n\n", workflows.len()));
                        for workflow in workflows {
                            let name = workflow
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown");
                            let status = workflow
                                .get("status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("unknown");
                            response.push_str(&format!("- **{}** ({})\n", name, status));
                        }
                    } else {
                        response.push_str("No workflows found.\n");
                    }
                }
                "query_rule_history" => {
                    // Handle both direct array and truncated nested structure
                    if let Some(history) = extract_array(&json_value, "history") {
                        response.push_str(&format!(
                            "## Rule Execution History ({} entries)\n\n",
                            history.len()
                        ));
                        for (i, entry) in history.iter().enumerate().take(10) {
                            // Limit to 10 entries
                            let name = entry
                                .get("rule_name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown");
                            let success = entry
                                .get("success")
                                .and_then(|s| s.as_bool())
                                .unwrap_or(false);
                            let status = if success { "[OK]" } else { "!! Failed" };
                            response.push_str(&format!("- **{}** {}\n", name, status));
                            if i == 9 {
                                response.push_str(&format!(
                                    "\n... ({} more entries)\n",
                                    history.len().saturating_sub(10)
                                ));
                                break;
                            }
                        }
                    } else {
                        response.push_str("No execution history found.\n");
                    }
                }
                "query_workflow_status" => {
                    // Handle both direct array and truncated nested structure
                    if let Some(executions) = extract_array(&json_value, "executions") {
                        response.push_str(&format!(
                            "## Workflow Execution Status ({} entries)\n\n",
                            executions.len()
                        ));
                        for (i, exec) in executions.iter().enumerate().take(10) {
                            let wf_id = exec
                                .get("workflow_id")
                                .and_then(|w| w.as_str())
                                .unwrap_or("unknown");
                            let status = exec
                                .get("status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("unknown");
                            response.push_str(&format!("- **{}** - {}\n", wf_id, status));
                            if i == 9 {
                                response.push_str(&format!(
                                    "\n... ({} more entries)\n",
                                    executions.len().saturating_sub(10)
                                ));
                                break;
                            }
                        }
                    } else {
                        response.push_str("No execution records found.\n");
                    }
                }
                "get_device_metrics" => {
                    // Handle both direct array and truncated nested structure
                    if let Some(metrics) = extract_array(&json_value, "metrics") {
                        response.push_str("## Device Metrics\n\n");
                        for metric in metrics {
                            let name = metric
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown");
                            let value = metric
                                .get("value")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            response.push_str(&format!("- **{}**: {}\n", name, value));
                        }
                    } else {
                        response.push_str("No device metrics found.\n");
                    }
                }
                "get_device_data" => {
                    // Format get_device_data result with device info and metrics
                    let device_name = json_value
                        .get("device_name")
                        .and_then(|n| n.as_str())
                        .or_else(|| json_value.get("device_id").and_then(|d| d.as_str()))
                        .unwrap_or("Unknown Device");

                    let device_type = json_value
                        .get("device_type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown");

                    response.push_str(&format!("## {} ({})\n\n", device_name, device_type));

                    if let Some(metrics) = json_value.get("metrics").and_then(|m| m.as_object()) {
                        for (metric_name, metric_data) in metrics {
                            let display_name = metric_data
                                .get("display_name")
                                .and_then(|n| n.as_str())
                                .unwrap_or(metric_name);

                            let value = metric_data
                                .get("value")
                                .map(|v| {
                                    if v.is_null() {
                                        "无数据".to_string()
                                    } else {
                                        v.to_string().replace("\"", "")
                                    }
                                })
                                .unwrap_or("未知".to_string());

                            let unit = metric_data
                                .get("unit")
                                .and_then(|u| u.as_str())
                                .unwrap_or("");

                            if !unit.is_empty() {
                                response.push_str(&format!(
                                    "- **{}**: {} {}\n",
                                    display_name, value, unit
                                ));
                            } else {
                                response.push_str(&format!("- **{}**: {}\n", display_name, value));
                            }

                            // Show timestamp if available
                            if let Some(ts) = metric_data.get("timestamp").and_then(|t| t.as_i64())
                            {
                                if ts > 0 {
                                    use chrono::{DateTime, Utc};
                                    if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                                        let time_ago = (Utc::now() - dt).num_seconds();
                                        if time_ago < 3600 {
                                            response.push_str(&format!(
                                                "  _{} seconds ago_\n",
                                                time_ago
                                            ));
                                        } else if time_ago < 86400 {
                                            response.push_str(&format!(
                                                "  _{} minutes ago_\n",
                                                time_ago / 60
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        response.push_str("No data available.\n");
                    }
                }
                "query_data" => {
                    // Format query result
                    if let Some(data) = json_value.get("data") {
                        response.push_str(&format!(
                            "## Query Result\n\n```\n{}\n```\n",
                            serde_json::to_string_pretty(data).unwrap_or_default()
                        ));
                    } else {
                        response.push_str("Query completed.\n");
                    }
                }
                "control_device" | "send_command" => {
                    response.push_str("**[OK]** command sent\n");
                }
                "list_agents" => {
                    // Format agent list with statistics
                    // Tool result structure: {"agents": {"items": [...], "_total_count": N}, "count": N}
                    let agents_array = if let Some(agents_obj) =
                        json_value.get("agents").and_then(|a| a.as_object())
                    {
                        // New structure: agents is an object with "items" array
                        agents_obj.get("items").and_then(|i| i.as_array())
                    } else {
                        // Old structure: agents is directly an array
                        json_value.get("agents").and_then(|a| a.as_array())
                    };

                    if let Some(agents) = agents_array {
                        if agents.is_empty() {
                            response.push_str("**AI Agent List**\n\n");
                            response.push_str("No AI Agents configured in the system.");
                        } else {
                            response.push_str(&format!(
                                "**AI Agent List** ({} total)\n\n",
                                agents.len()
                            ));
                            for agent in agents {
                                let name = agent
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown");
                                let id = agent.get("id").and_then(|i| i.as_str()).unwrap_or("");
                                let status = agent
                                    .get("status")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("unknown");

                                // Get execution stats - try multiple paths
                                let exec_count_str = agent
                                    .get("execution_count")
                                    .and_then(|e| e.as_u64())
                                    .or_else(|| {
                                        agent
                                            .get("stats")
                                            .and_then(|s| s.get("total_executions"))
                                            .and_then(|e| e.as_u64())
                                    })
                                    .map(|c| c.to_string())
                                    .or_else(|| {
                                        agent
                                            .get("stats")
                                            .and_then(|s| s.get("total_executions"))
                                            .and_then(|e| e.as_str())
                                            .map(String::from)
                                    })
                                    .unwrap_or_else(|| "0".to_string());

                                let last_exec = agent
                                    .get("last_execution_at")
                                    .and_then(|l| l.as_str())
                                    .unwrap_or("Not executed");

                                let status_icon = match status {
                                    "active" | "Active" => "[on]",
                                    _ => "[off]",
                                };

                                response.push_str(&format!(
                                    "- {} **{}** ({})\n",
                                    status_icon, name, status
                                ));

                                // Add ID for reference
                                if !id.is_empty() && id.len() < 30 {
                                    response.push_str(&format!("  ID: `{}`\n", id));
                                }

                                // Add execution info
                                if exec_count_str != "0" {
                                    response.push_str(&format!(
                                        "  Executions: {}, Last: {}\n",
                                        exec_count_str,
                                        if last_exec == "Not executed" || last_exec.contains("null")
                                        {
                                            "N/A"
                                        } else {
                                            last_exec
                                        }
                                    ));
                                }

                                // Add description if available
                                if let Some(desc) =
                                    agent.get("description").and_then(|d| d.as_str())
                                {
                                    if !desc.is_empty() && desc != "null" {
                                        response.push_str(&format!("  Description: {}\n", desc));
                                    }
                                }
                            }
                        }
                    } else if let Some(count) = json_value.get("count").and_then(|c| c.as_u64()) {
                        if count == 0 {
                            response.push_str("**AI Agent List**\n\n");
                            response.push_str("No AI Agents configured in the system.");
                        } else {
                            response.push_str(&format!("**AI Agent List** ({} total)\n", count));
                        }
                    } else {
                        response.push_str("**AI Agent List**\n\n");
                        response.push_str("No AI Agents configured in the system.");
                    }
                }
                "get_agent" => {
                    // Format single agent details
                    let name = json_value
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown");
                    let status = json_value
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown");
                    let agent_type = json_value
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown");

                    response.push_str(&format!("## Agent: {} ({})\n\n", name, agent_type));
                    response.push_str(&format!("**Status**: {}\n", status));

                    // Execution stats
                    if let Some(stats) = json_value.get("stats") {
                        if let Some(total) = stats.get("total_executions").and_then(|t| t.as_u64())
                        {
                            let success = stats
                                .get("successful_executions")
                                .and_then(|s| s.as_u64())
                                .unwrap_or(0);
                            let failed = stats
                                .get("failed_executions")
                                .and_then(|f| f.as_u64())
                                .unwrap_or(0);
                            response.push_str(&format!(
                                "**Execution Stats**: Total {} times, Success {} times, Failed {} times\n",
                                total, success, failed
                            ));
                        }
                    }

                    // Last execution
                    if let Some(last) = json_value.get("last_execution_at").and_then(|l| l.as_str())
                    {
                        if !last.is_empty() && last != "null" {
                            response.push_str(&format!("**Last Execution**: {}\n", last));
                        }
                    }

                    // Schedule
                    if let Some(schedule) = json_value.get("schedule_type").and_then(|s| s.as_str())
                    {
                        response.push_str(&format!("**Schedule Type**: {}\n", schedule));
                    }
                }
                "create_rule" => {
                    if let Some(rule_id) = json_value.get("rule_id").and_then(|r| r.as_str()) {
                        response.push_str(&format!(
                            "[OK] Rule created successfully (ID: {})\n",
                            rule_id
                        ));
                    } else {
                        response.push_str("[OK] Rule created successfully.\n");
                    }
                }
                "trigger_workflow" => {
                    if let Some(execution_id) =
                        json_value.get("execution_id").and_then(|e| e.as_str())
                    {
                        response.push_str(&format!(
                            "[OK] Workflow triggered (Execution ID: {})\n",
                            execution_id
                        ));
                    } else {
                        response.push_str("[OK] Workflow triggered.\n");
                    }
                }
                "create_agent" => {
                    if let Some(agent_id) = json_value.get("agent_id").and_then(|a| a.as_str()) {
                        response.push_str(&format!(
                            "[OK] Agent created successfully (ID: {})\n",
                            agent_id
                        ));
                    } else if let Some(id) = json_value.get("id").and_then(|i| i.as_str()) {
                        response
                            .push_str(&format!("[OK] Agent created successfully (ID: {})\n", id));
                    } else {
                        response.push_str("[OK] Agent created successfully.\n");
                    }
                }
                "execute_agent" => {
                    if let Some(execution_id) =
                        json_value.get("execution_id").and_then(|e| e.as_str())
                    {
                        response.push_str(&format!(
                            "[OK] Agent execution triggered (ID: {})\n",
                            execution_id
                        ));
                    } else if let Some(result) = json_value.get("result").and_then(|r| r.as_str()) {
                        response.push_str(&format!("[OK] Agent execution completed: {}\n", result));
                    } else {
                        response.push_str("[OK] Agent execution completed.\n");
                    }
                }
                "control_agent" => {
                    if let Some(new_status) = json_value.get("status").and_then(|s| s.as_str()) {
                        response.push_str(&format!("[OK] Agent status updated: {}\n", new_status));
                    } else {
                        response.push_str("[OK] Agent control command executed.\n");
                    }
                }
                "delete_rule" => {
                    response.push_str("[OK] Rule deleted.\n");
                }
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

/// Emit plan events from an ExecutionPlan through the event channel.
pub fn emit_plan_events(
    plan: &ExecutionPlan,
    tx: &tokio::sync::mpsc::UnboundedSender<super::types::AgentEvent>,
) {
    let _ = tx.send(super::types::AgentEvent::ExecutionPlanCreated {
        plan: plan.clone(),
        session_id: None,
    });
}

/// Result of a single tool execution with metadata
struct ToolExecutionResult {
    _name: String,
    arguments: serde_json::Value,
    result: std::result::Result<crate::toolkit::ToolOutput, crate::toolkit::ToolError>,
}

/// Estimate token count for a string.
///
/// Uses the accurate tokenizer implementation from the tokenizer module,
/// which properly handles:
/// - Chinese characters: ~1.8 tokens each
/// - English words: ~0.25 tokens per character
/// - Numbers: ~0.3 tokens per digit
/// - Special characters: ~0.5 tokens each
///
/// This is much more accurate than the simple char_count / 4 heuristic.
fn estimate_tokens(text: &str) -> usize {
    use crate::agent::tokenizer::estimate_tokens as accurate_estimate;
    accurate_estimate(text)
}

/// === ANTHROPIC-STYLE IMPROVEMENT: Tool Result Clearing for Streaming ===
///
/// Compacts old tool result messages into concise summaries.
/// This follows Anthropic's guidance for context engineering.
#[allow(dead_code)]
fn compact_tool_results_stream(messages: &[AgentMessage]) -> Vec<AgentMessage> {
    let mut result = Vec::new();
    let mut tool_result_count = 0;
    const KEEP_RECENT_TOOL_RESULTS: usize = 2;

    for msg in messages.iter().rev() {
        if msg.role == "user" || msg.role == "system" {
            result.push(msg.clone());
            continue;
        }

        if msg.tool_calls.is_some() && msg.tool_calls.as_ref().is_some_and(|t| !t.is_empty()) {
            tool_result_count += 1;

            if tool_result_count <= KEEP_RECENT_TOOL_RESULTS {
                result.push(msg.clone());
            } else {
                // Compress old tool results with descriptive summary
                let summaries: Vec<String> = msg
                    .tool_calls
                    .as_ref()
                    .iter()
                    .flat_map(|calls| calls.iter())
                    .map(|tc| {
                        let args_summary =
                            super::types::summarize_tool_args(&tc.name, &tc.arguments);
                        let result_preview = tc
                            .result
                            .as_ref()
                            .and_then(|r| {
                                let s = if let Some(s) = r.as_str() {
                                    s.to_string()
                                } else {
                                    r.to_string()
                                };
                                // Read actions need more preview to preserve data
                                let is_data_action = args_summary.contains("list")
                                    || args_summary.contains("get")
                                    || args_summary.contains("history");
                                let preview_len = if is_data_action { 300 } else { 80 };
                                Some(s.chars().take(preview_len).collect::<String>())
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

                let summary = format!(
                    "Previously called {}. These are past results, do not repeat.",
                    summaries.join(", then ")
                );

                result.push(AgentMessage {
                    role: msg.role.clone(),
                    content: summary,
                    tool_calls: None,
                    tool_call_id: None,
                    tool_call_name: None,
                    thinking: None,
                    images: None,
                    round_contents: None,
                    round_thinking: None,
                    timestamp: msg.timestamp,
                });
            }
        } else {
            result.push(msg.clone());
        }
    }

    result.reverse();
    result
}

/// === IMPROVED: Context Window with CompactionConfig ===
///
/// Builds conversation context using CompactionConfig for intelligent compaction:
/// 1. Reserve tokens floor for generation
/// 2. Tool result clearing for old messages
/// 3. Token-based windowing with priority
/// 4. Always keep recent messages for context continuity
///
/// The `max_tokens` parameter allows dynamic context sizing based on the model's actual capacity.
pub(crate) fn build_context_window(
    messages: &[AgentMessage],
    max_tokens: usize,
) -> Vec<AgentMessage> {
    build_context_window_with_summary(messages, max_tokens, None, None)
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
    // First, compact tool results
    let compacted = compact_tool_results_stream_with_config(messages, config);

    let mut selected_messages = Vec::new();
    let mut current_tokens = 0;

    for msg in compacted.iter().rev() {
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

        // Truncate long messages if needed
        let final_msg = if msg_tokens > config.max_message_length {
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

/// Estimate tokens for an AgentMessage for LLM context.
///
/// IMPORTANT: Thinking content is NOT counted because:
/// 1. to_core() does NOT include thinking when sending to LLM
/// 2. Thinking is only for frontend display, not for model context
/// 3. Counting thinking would incorrectly consume the context budget
fn estimate_message_tokens(msg: &AgentMessage) -> usize {
    let mut tokens = estimate_tokens(&msg.content);

    // NOTE: Thinking is intentionally NOT counted here
    // Even though it's stored in AgentMessage, it's not sent to LLM via to_core()
    // Only count content, tool_calls, and images

    // Add tokens for tool calls
    if let Some(tool_calls) = &msg.tool_calls {
        for tool_call in tool_calls {
            let args_str = tool_call.arguments.to_string();
            tokens += 10 + estimate_tokens(&args_str);
        }
    }

    // Add tokens for images (rough estimate)
    if let Some(images) = &msg.images {
        if !images.is_empty() {
            tokens += 85 * images.len();
        }
    }

    tokens
}

/// Truncate an AgentMessage's content to fit within max length.
fn truncate_agent_message(msg: &AgentMessage, max_len: usize) -> AgentMessage {
    let mut truncated = msg.clone();

    if msg.content.len() > max_len {
        // Truncate at word boundary
        let truncated_content = if let Some(last_space) = msg.content[..max_len].rfind(' ') {
            format!("{}...", &msg.content[..last_space])
        } else {
            format!("{}...", &msg.content[..max_len])
        };
        truncated.content = truncated_content;
    }

    // Also truncate thinking if present
    if let Some(thinking) = &truncated.thinking {
        if thinking.len() > max_len / 2 {
            truncated.thinking = Some(
                if let Some(last_space) = thinking[..max_len / 2].rfind(' ') {
                    format!("{}...", &thinking[..last_space])
                } else {
                    format!("{}...", &thinking[..max_len / 2])
                },
            );
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
                                .and_then(|r| {
                                    let s = if let Some(s) = r.as_str() {
                                        s.to_string()
                                    } else {
                                        r.to_string()
                                    };
                                    // Read actions need more preview to preserve data
                                    let is_data_action = args_summary.contains("list")
                                        || args_summary.contains("get")
                                        || args_summary.contains("history");
                                    let preview_len = if is_data_action { 300 } else { 80 };
                                    Some(s.chars().take(preview_len).collect::<String>())
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
                    content: summary_content,
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

    // === FAST PATH: Simple greetings and common patterns ===
    // Bypass LLM for simple, common interactions to improve speed and reliability
    let trimmed = user_message.trim();
    let lower = trimmed.to_lowercase();

    // Greeting patterns
    let greeting_patterns = [
        "你好",
        "您好",
        "hi",
        "hello",
        "嗨",
        "在吗",
        "早上好",
        "下午好",
        "晚上好",
    ];

    // Device list query patterns - fast path for common device queries
    let device_list_patterns = [
        "有哪些设备",
        "有什么设备",
        "设备列表",
        "查看设备",
        "所有设备",
        "列出设备",
        "系统设备",
        "显示设备",
        "devices",
        "list devices",
    ];

    // Temperature query patterns - fast path for temperature queries
    let temp_query_patterns = ["温度", "temperature"];

    let _is_greeting = greeting_patterns
        .iter()
        .any(|&pat| trimmed.eq_ignore_ascii_case(pat) || trimmed.starts_with(pat));

    // Check for device list query
    let _is_device_query = device_list_patterns
        .iter()
        .any(|&pat| lower.contains(&pat.to_lowercase()) && lower.len() < 30);

    // Check for temperature query (simple single-word queries)
    let _is_temp_query = temp_query_patterns.iter().any(|&pat| {
        lower == pat || lower.ends_with(pat) || lower.starts_with("当前") && lower.contains("温度")
    });

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

    // === COMPLEX INTENT DETECTION FOR MULTI-ROUND TOOL CALLING ===
    // Use keyword-based detection for fast response (removed LLM call to prevent blocking)
    // The fallback function provides reliable detection for common patterns
    let is_complex_intent = is_complex_multi_step_intent_fallback(&user_message);

    tracing::info!(
        "Complex intent detection (keyword-based): is_complex={}, message={}",
        is_complex_intent,
        user_message.chars().take(50).collect::<String>()
    );

    // === Get conversation history and pass to LLM ===
    // This prevents the LLM from repeating actions or calling tools again
    // Pure async - no block_in_place
    let state_guard = internal_state.read().await;
    let history_messages = state_guard.memory.clone();
    drop(state_guard); // Release lock before calling LLM

    // === DYNAMIC CONTEXT WINDOW: Get model's actual capacity ===
    let max_context = llm_interface.max_context_length().await;
    // Allocate history budget proportionally to model capacity:
    // - Small contexts (< 8k): 65% (system prompt + tools take a large share)
    // - Medium contexts (< 16k): 80%
    // - Large contexts (>= 16k): 95% (modern models have ample room)
    let effective_max = if max_context < 8192 {
        (max_context * 65) / 100
    } else if max_context < 16384 {
        (max_context * 80) / 100
    } else {
        (max_context * 95) / 100
    };

    tracing::debug!(
        "Context window: model_capacity={}, effective_max={} for history",
        max_context,
        effective_max
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
        const THINKING_TIMEOUT_SECS: u64 = 120;

        // === SAFEGUARD: Track recently executed tools for multi-round context ===
        let mut recently_executed_tools: VecDeque<String> = VecDeque::new();
        // Track tool signatures to detect consecutive duplicate rounds
        let mut prev_round_signatures: Vec<Vec<String>> = Vec::new();
        let mut consecutive_duplicate_rounds: usize = 0;

        // === SAFEGUARD: Track multi-round tool calling iterations ===
        let mut tool_iteration_count = 0usize;
        const MAX_TOOL_ITERATIONS: usize = 10;
        // Accumulate ALL tool results across rounds for final summary
        let mut all_round_tool_results: Vec<(String, String)> = Vec::new();
        // Track per-round thinking and content for persistence (round number → text)
        let mut round_thinking_map: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
        let mut round_contents_map: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
        // Accumulate ALL rounds' thinking for the message's thinking field
        let mut all_rounds_thinking = String::new();

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
                // Apply tool result compaction to prevent context bloat from accumulated
                // tool JSON results. compact_tool_results keeps the 2 most recent tool
                // rounds intact and compresses older ones to brief summaries.
                let state_guard = internal_state.read().await;

                let history_for_llm: Vec<neomind_core::Message> = {
                    // Compact tool results: keep 2 most recent rounds, summarize older ones
                    let compacted = super::compact_tool_results(&state_guard.memory, 2);
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
                    let executed_summary = recently_executed.iter()
                        .map(|s| format!("- {}", s))
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!(
                        "Round {} of processing.\n\n\
                        Previously executed tools:\n{}\n\n\
                        Rules:\n\
                        1. Do NOT repeat tools that were already executed — use their results from context.\n\
                        2. BATCH all new tool calls in ONE response using JSON array: [{{\"name\":\"tool\",\"arguments\":{{...}}}}, ...]\n\
                        3. Example: querying battery for 3 devices → output 3 device(query) calls in ONE array, NOT one per round.\n\
                        4. If you have enough data, give the final response now without calling any tools.",
                        tool_iteration_count + 1,
                        executed_summary
                    )
                };

                tracing::debug!("Multi-round context: {}", context_msg);

                // Use tools enabled for subsequent rounds (thinking follows instance setting)
                let round_stream_result = llm_interface.chat_stream_with_history(
                    &context_msg,
                    &history_for_llm
                ).await;

                let round_stream = match round_stream_result {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Round {} LLM call failed: {}", tool_iteration_count + 1, e);
                        yield AgentEvent::error(format!("Tool call failed: {}", e));
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
                    yield AgentEvent::content("\n\n[Interrupted]".to_string());
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
                                    yield AgentEvent::content(new_content.to_string());
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
                                        yield AgentEvent::content(new_content.to_string());
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
                                            yield AgentEvent::content(safe_content.to_string());
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
                if !remaining.is_empty() {
                    content_before_tools.push_str(remaining);
                    yield AgentEvent::content(remaining.to_string());
                }
                yielded_up_to = buffer.len();
            }

            // === PHASE 2: Handle tool calls if detected ===
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

                // === PHASE 3: Generate follow-up response ===
                // For complex intents, check if we need more tool calls
                if is_complex_intent && tool_iteration_count < MAX_TOOL_ITERATIONS - 1 {
                    tracing::debug!("Complex intent: Checking if more tool calls needed (iteration {}/{})",
                        tool_iteration_count + 1, MAX_TOOL_ITERATIONS);

                    // === DUPLICATE DETECTION ===
                    // Detect consecutive duplicate rounds (same tool calls repeated).
                    // Allow 1 retry but stop after 2+ consecutive duplicates to prevent loops.
                    // Different-entity calls (different device_id/metric) are never duplicates.
                    {
                        let mut new_tool_signatures: Vec<Vec<String>> = Vec::new();
                        for tc in &tool_calls_to_execute {
                            let action_key = tc.arguments.get("action")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let mut sig = vec![tc.name.clone(), action_key];
                            for param in &["device_id", "metric", "agent_id", "rule_id", "alert_id"] {
                                if let Some(val) = tc.arguments.get(*param).and_then(|v| v.as_str()) {
                                    sig.push(val.to_string());
                                }
                            }
                            new_tool_signatures.push(sig);
                        }

                        // Check if this round is identical to the PREVIOUS round (consecutive duplicate)
                        let is_consecutive_dup = !prev_round_signatures.is_empty()
                            && new_tool_signatures.len() == prev_round_signatures.len()
                            && new_tool_signatures.iter().all(|sig| {
                                prev_round_signatures.iter().any(|prev| {
                                    prev.len() == sig.len()
                                        && prev.iter().zip(sig.iter()).all(|(a, b)| a == b)
                                })
                            });

                        if is_consecutive_dup {
                            consecutive_duplicate_rounds += 1;
                            tracing::warn!(
                                "Consecutive duplicate round detected (count={}/2). Tools: {:?}",
                                consecutive_duplicate_rounds,
                                tool_call_results.iter().map(|(n, _)| n).collect::<Vec<_>>()
                            );
                        } else {
                            consecutive_duplicate_rounds = 0;
                        }

                        prev_round_signatures = new_tool_signatures;

                        // Stop after 2 consecutive identical rounds — the LLM is stuck
                        if consecutive_duplicate_rounds >= 2 {
                            tracing::warn!(
                                "LLM stuck in loop (2+ consecutive duplicate rounds). Stopping multi-round loop."
                            );
                            // Fall through to final response
                        } else {
                    // === Continue the loop (save state + loop back) ===
                        // === CRITICAL: Save assistant message with tool_calls BEFORE tool results ===
                        // Without this, tool_calls are lost when switching sessions because
                        // the assistant message is never persisted in the multi-round path.
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
                        internal_state.write().await.push_message(initial_msg);

                        // Save tool results to memory (large results go through cache → summary)
                        // Skill tool results go to transient skill_context instead of history
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

                        // Increment iteration count and loop back
                        tool_iteration_count += 1;

                        // Save per-round thinking and content for persistence
                        let round_num = tool_iteration_count as u32; // current round that just ended
                        if !thinking_content.is_empty() {
                            round_thinking_map.insert(round_num, thinking_content.clone());
                            all_rounds_thinking.push_str(&thinking_content);
                        }
                        if !content_before_tools.is_empty() {
                            // Clean any JSON/markdown artifacts from content before storing
                            let cleaned = remove_tool_calls_from_response(&content_before_tools);
                            // Also strip markdown code block prefixes that small models emit
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

                        // Signal end of current round before continuing
                        yield AgentEvent::IntermediateEnd;

                        // Continue the loop to make another LLM call with tools
                        continue 'multi_round_loop;
                        } // end else (not consecutive duplicate)
                    } // end duplicate detection block
                } // end if is_complex_intent

                // === MAX ITERATIONS REACHED, CONSECUTIVE DUPLICATES, or NON-COMPLEX INTENT: Final response ===
                // Save the initial message with thinking and tool calls
                let response_to_save = if content_before_tools.is_empty() {
                    // No content before tools - use empty string, don't show meaningless fallback
                    String::new()
                } else {
                    content_before_tools.clone()
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
                tracing::debug!("[streaming] Saving initial assistant message with {} tool_calls", initial_msg.tool_calls.as_ref().map_or(0, |c| c.len()));
                internal_state.write().await.push_message(initial_msg);

                // Add tool result messages to history (large results go through cache → summary)
                // Skill tool results go to transient skill_context instead of history
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

                // Trim history
                let state_guard = internal_state.read().await;
                let history_agent_messages = &state_guard.memory;

                // Extract the user question that triggered this round (most recent real user message).
                // Skip tool-result messages (they are converted to User with content "[Tool: ... returned]\n...").
                let original_user_question = history_agent_messages.iter()
                    .rev()
                    .find(|msg| {
                        if msg.role != "user" {
                            return false;
                        }
                        !msg.content.starts_with("[Tool:")
                    })
                    .map(|msg| msg.content.clone());

                // === DYNAMIC HISTORY TRIMMING: Use token-based windowing ===
                // Instead of hardcoded 6 messages, use the model's context capacity
                // Reserve 30% for Phase 2 response generation
                let max_context = llm_interface.max_context_length().await;
                let max_history_tokens = (max_context * 70) / 100;

                tracing::debug!(
                    "Phase 2 history: {} messages, max_tokens={} (70% of {})",
                    history_agent_messages.len(),
                    max_history_tokens,
                    max_context
                );

                // Use intelligent context window building with token limits
                let trimmed_agent_messages = build_context_window(history_agent_messages, max_history_tokens);

                // Convert to core Message format for LLM
                let history_messages: Vec<neomind_core::Message> = trimmed_agent_messages
                    .iter()
                    .map(|msg| msg.to_core())
                    .collect();

                drop(state_guard);

                // === SIMPLE QUERY FAST PATH: Skip Phase 2 for simple queries ===
                // This follows mainstream agent design patterns:
                // - AutoGen: reflect_on_tool_use=False returns tool results directly
                // - LangChain: return_direct=True stops agent loop after tool execution
                //
                // For simple list/query operations, the formatted tool result is the final answer.
                // No need for another LLM call to "interpret" the data.
                if should_return_directly(&tool_call_results) {
                    tracing::debug!(
                        "Simple query detected (tools: {:?}), skipping Phase 2 LLM call",
                        tool_call_results.iter().map(|(n, _)| n).collect::<Vec<_>>()
                    );

                    // Format tool results directly for user display
                    let formatted_response = format_tool_results(&tool_call_results);
                    tracing::debug!("Direct response length: {} chars", formatted_response.len());

                    // Stream the formatted response
                    for chunk in formatted_response.chars().collect::<Vec<_>>().chunks(30) {
                        let chunk_str: String = chunk.iter().collect();
                        if !chunk_str.is_empty() {
                            yield AgentEvent::content(chunk_str);
                        }
                    }

                    // Save the response to memory
                    let response_msg = AgentMessage::assistant(formatted_response.clone());
                    internal_state.write().await.push_message(response_msg);
                    internal_state.write().await.register_response(&formatted_response);

                    let pt = llm_interface.take_last_prompt_tokens().await;
                    match pt {
                        Some(t) => yield AgentEvent::end_with_tokens(t),
                        None => yield AgentEvent::end(),
                    }
                    return;
                }

                // === PHASE 2: Generate follow-up response ===
                // For complex queries that need LLM analysis/summarization
                tracing::debug!("Phase 2: Generating follow-up response (complex query)");

                // Deduplicate accumulated tool results across all rounds.
                // Keep the latest result for each (tool_name, key_params) combination.
                let deduped_results = deduplicate_tool_results(&all_round_tool_results);
                tracing::debug!(
                    "Phase 2: {} accumulated results → {} deduplicated",
                    all_round_tool_results.len(),
                    deduped_results.len()
                );

                // Build Phase 2 prompt with ALL accumulated tool results so the LLM
                // can produce a comprehensive summary even after multiple rounds.
                // Use the summary prompt builder for multi-round scenarios to ensure
                // the LLM summarizes everything, not just the last round.
                let phase2_prompt = if tool_iteration_count > 0 || all_round_tool_results.len() > tool_call_results.len() {
                    let end_reason = if consecutive_duplicate_rounds >= 2 {
                        "loop detected"
                    } else if tool_iteration_count >= MAX_TOOL_ITERATIONS - 1 {
                        "iteration limit reached"
                    } else {
                        "completed"
                    };
                    build_phase2_summary_prompt(
                        original_user_question.clone(),
                        &deduped_results,
                        tool_iteration_count + 1,
                        end_reason,
                    )
                } else {
                    build_phase2_prompt_with_tool_results(
                        original_user_question.clone(),
                        &deduped_results,
                    )
                };
                tracing::debug!("Phase 2 prompt length: {} chars (with tool results)", phase2_prompt.len());

                let followup_stream_result = llm_interface.chat_stream_no_tools_no_thinking_with_history(
                    &phase2_prompt, &history_messages
                ).await;

                let followup_stream = match followup_stream_result {
                    Ok(stream) => stream,
                    Err(e) => {
                        tracing::error!("Phase 2 LLM call failed: {}", e);
                        let fallback_text = format_tool_results(&deduped_results);
                        for chunk in fallback_text.chars().collect::<Vec<_>>().chunks(20) {
                            let chunk_str: String = chunk.iter().collect();
                            if !chunk_str.is_empty() {
                                yield AgentEvent::content(chunk_str);
                            }
                        }
                        let pt = llm_interface.take_last_prompt_tokens().await;
                        match pt {
                            Some(t) => yield AgentEvent::end_with_tokens(t),
                            None => yield AgentEvent::end(),
                        }
                        return;
                    }
                };

                let mut followup_stream = Box::pin(followup_stream);
                let mut final_response_content = String::new();
                let followup_start = Instant::now();

                let mut chunk_count = 0usize;
                while let Some(result) = StreamExt::next(&mut followup_stream).await {
                    if followup_start.elapsed() > Duration::from_secs(30) {
                        tracing::warn!("Phase 2 timeout (>30s), forcing completion");
                        break;
                    }

                    chunk_count += 1;
                    match result {
                        Ok((chunk, is_thinking)) => {
                            if chunk.is_empty() {
                                tracing::trace!("Phase 2: Received empty chunk #{}, skipping", chunk_count);
                                continue;
                            }
                            if !is_thinking {
                                // Skip duplicate chunks (model repetition: same error/text sent twice)
                                let ct = chunk.trim();
                                if !ct.is_empty() {
                                    if final_response_content.ends_with(ct) {
                                        tracing::trace!("Phase 2: Skipping duplicate chunk");
                                        continue;
                                    }
                                    if ct.len() > 30 && final_response_content.contains(ct) {
                                        tracing::trace!("Phase 2: Skipping contained chunk");
                                        continue;
                                    }
                                }
                                tracing::trace!("Phase 2: Yielding content chunk #{}: {} chars", chunk_count, chunk.len());
                                yield AgentEvent::content(chunk.clone());
                                final_response_content.push_str(&chunk);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Phase 2 stream error: {}", e);
                            break;
                        }
                    }
                }
                tracing::debug!("Phase 2 stream consumed: {} chunks, {} chars total", chunk_count, final_response_content.len());

                // Check for empty response OR hallucination detection
                let hallucination_detected = detect_hallucination(&final_response_content, &deduped_results);
                tracing::debug!("Phase 2 fallback check: empty={}, hallucination={}, tools={}",
                    final_response_content.is_empty(), hallucination_detected, tool_call_results.len());

                if final_response_content.is_empty() || hallucination_detected {
                    // Use rich formatter instead of simple fallback
                    let fallback = format_tool_results(&deduped_results);
                    tracing::debug!("Phase 2: Yielding fallback content: {} chars", fallback.len());
                    yield AgentEvent::content(fallback.clone());
                    final_response_content = fallback;
                }

                // === PHASE 2 TOOL CALL RECOVERY ===
                // Phase 2 calls LLM without tools (no_tools), but the LLM may still
                // output tool call JSON as text because the system prompt teaches this format.
                // Detect and extract these embedded tool calls so they get executed
                // instead of being shown as raw JSON to the user.
                if let Ok((cleaned_text, embedded_tool_calls)) = parse_tool_calls(&final_response_content) {
                    if !embedded_tool_calls.is_empty() {
                        if tool_iteration_count < MAX_TOOL_ITERATIONS - 1 {
                            tracing::debug!(
                                "Phase 2: Recovered {} embedded tool calls from follow-up response, continuing execution",
                                embedded_tool_calls.len()
                            );

                            // Replace final_response_content with cleaned text (JSON removed)
                            final_response_content = cleaned_text;

                            // IMPORTANT: Save the current state before continuing
                            {
                                let mut state = internal_state.write().await;
                                state.register_response(&final_response_content);
                                if let Some(last_msg) = state.memory.last_mut() {
                                    if last_msg.role == "assistant" && last_msg.tool_calls.is_some() {
                                        last_msg.content = final_response_content.clone();
                                    } else {
                                        let final_msg = AgentMessage::assistant(&final_response_content);
                                        state.push_message(final_msg);
                                    }
                                } else {
                                    let final_msg = AgentMessage::assistant(&final_response_content);
                                    state.push_message(final_msg);
                                }
                            }

                            // Set up for next round with the recovered tool calls
                            tool_calls = embedded_tool_calls;
                            tool_calls_detected = true;

                            // Save per-round thinking and content for persistence
                            let round_num = (tool_iteration_count + 1) as u32;
                            if !thinking_content.is_empty() {
                                round_thinking_map.insert(round_num, thinking_content.clone());
                                all_rounds_thinking.push_str(&thinking_content);
                            }

                            tool_iteration_count += 1;
                            content_before_tools.clear();
                            thinking_content.clear();

                            yield AgentEvent::IntermediateEnd;
                            continue 'multi_round_loop;
                        } else {
                            // Max iterations reached - can't execute more rounds
                            // But still clean the raw JSON from content to avoid showing it to user
                            tracing::debug!(
                                "Phase 2: Found {} embedded tool calls but max iterations reached, cleaning JSON from content",
                                embedded_tool_calls.len()
                            );
                            final_response_content = cleaned_text;
                        }
                    }
                }

                // IMPORTANT: Update the initial message with the follow-up content
                // instead of saving a separate message. This ensures the message
                // has both tool_calls and content in one place.

                // Save last round's thinking for persistence
                let last_round = (tool_iteration_count + 1) as u32;
                if !thinking_content.is_empty() {
                    let cleaned = cleanup_thinking_content(&thinking_content);
                    round_thinking_map.insert(last_round, cleaned.clone());
                    all_rounds_thinking.push_str(&cleaned);
                }
                // NOTE: Do NOT store final_response_content in round_contents_map for the last round.
                // It is already the message content (merged.content) — storing it here causes
                // the frontend to display it twice (once in tool round, once as final message).
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

                // Clean any embedded tool call JSON from the final response content
                // Some models echo tool call JSON in their text response, which should not
                // be stored in message.content as it's already tracked in tool_calls
                let cleaned_response_content = remove_tool_calls_from_response(&final_response_content);

                {
                    let mut state = internal_state.write().await;
                    // Register response for cross-turn repetition detection
                    state.register_response(&cleaned_response_content);
                    if let Some(last_msg) = state.memory.last_mut() {
                        if last_msg.role == "assistant" && last_msg.tool_calls.is_some() {
                            // Update the last assistant message (which has tool_calls) with the content
                            last_msg.content = cleaned_response_content.clone();
                            last_msg.thinking = if all_rounds_thinking.is_empty() { None } else { Some(all_rounds_thinking.clone()) };
                            last_msg.round_thinking = round_thinking_val.clone();
                            last_msg.round_contents = round_contents_val.clone();
                        } else {
                            // Fallback: push a new message if the last one isn't what we expect
                            let mut final_msg = AgentMessage::assistant(&cleaned_response_content);
                            final_msg.thinking = if all_rounds_thinking.is_empty() { None } else { Some(all_rounds_thinking.clone()) };
                            final_msg.round_thinking = round_thinking_val.clone();
                            final_msg.round_contents = round_contents_val.clone();
                            state.memory.push(final_msg);
                        }
                    } else {
                        let mut final_msg = AgentMessage::assistant(&cleaned_response_content);
                        final_msg.thinking = if all_rounds_thinking.is_empty() { None } else { Some(all_rounds_thinking.clone()) };
                        final_msg.round_thinking = round_thinking_val.clone();
                        final_msg.round_contents = round_contents_val.clone();
                        state.memory.push(final_msg);
                    }
                }

                tracing::debug!("Tool execution and Phase 2 response complete");
            } else {
                // No tool calls - save response directly
                // Use buffer if content_before_tools is empty (buffer contains all content chunks when no tools)
                let raw_response = if content_before_tools.is_empty() {
                    // When no tool calls were detected, buffer contains all the content
                    buffer.clone()
                } else {
                    content_before_tools.clone()
                };

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
        if image_data.starts_with("data:image/") {
            if let Some(base64_part) = image_data.split(',').nth(1) {
                // Extract mime type from data URL
                let mime_type = if image_data.contains("data:image/png") {
                    "image/png"
                } else if image_data.contains("data:image/jpeg") {
                    "image/jpeg"
                } else if image_data.contains("data:image/webp") {
                    "image/webp"
                } else if image_data.contains("data:image/gif") {
                    "image/gif"
                } else {
                    "image/png"
                };
                parts.push(ContentPart::image_base64(base64_part, mime_type));
            }
        }
    }

    // Get conversation history
    let state_guard = internal_state.read().await;
    let history_messages = state_guard.memory.clone();
    drop(state_guard);

    // Build context window
    let max_context = llm_interface.max_context_length().await;
    // Use conservative context limits
    let effective_max = if max_context < 8192 {
        (max_context * 60) / 100
    } else if max_context < 16384 {
        (max_context * 70) / 100
    } else {
        (max_context * 90) / 100
    };

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
            // Extract mime type from data URL if available
            let mime_type = if data_url.contains("data:image/jpeg") {
                Some("image/jpeg".to_string())
            } else if data_url.contains("data:image/png") {
                Some("image/png".to_string())
            } else if data_url.contains("data:image/webp") {
                Some("image/webp".to_string())
            } else if data_url.contains("data:image/gif") {
                Some("image/gif".to_string())
            } else {
                None
            };
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
                            yield AgentEvent::content(before_tool.to_string());
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
                                yield AgentEvent::content(before_tool.to_string());
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

            // Save initial message with tool calls
            let response_to_save = if content_before_tools.is_empty() {
                String::new()
            } else {
                content_before_tools.clone()
            };

            let initial_msg = AgentMessage::assistant_with_tools(&response_to_save, tool_calls_with_results.clone());
            internal_state.write().await.push_message(initial_msg);

            // Add tool result messages (large results go through cache → summary)
            // Skill tool results go to transient skill_context instead of history
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

            // Get updated history for Phase 2
            let state_guard = internal_state.read().await;
            let history_messages: Vec<neomind_core::Message> = state_guard.memory.iter()
                .map(|msg| msg.to_core())
                .collect::<Vec<_>>();
            drop(state_guard);

            // Extract the user question that triggered this round (most recent real user message).
            // Skip tool-result messages (they are converted to User with content "[Tool: ... returned]\n...").
            let original_user_question = history_messages.iter()
                .rev()
                .find(|msg| {
                    if msg.role != neomind_core::MessageRole::User {
                        return false;
                    }
                    let text = msg.content.as_text();
                    !text.starts_with("[Tool:")
                })
                .and_then(|msg| {
                    if let neomind_core::Content::Text(text) = &msg.content {
                        Some(text.clone())
                    } else if let neomind_core::Content::Parts(parts) = &msg.content {
                        // For multimodal messages, extract the text part
                        let text_parts: Vec<String> = parts.iter().filter_map(|p| {
                            if let neomind_core::ContentPart::Text { text: t } = p {
                                Some(t.clone())
                            } else {
                                None
                            }
                        }).collect();
                        if text_parts.is_empty() {
                            None
                        } else {
                            Some(text_parts.join(" "))
                        }
                    } else {
                        None
                    }
                });

            if history_messages.len() > 6 {
                let keep_count = 6;
                tracing::debug!("Trimming history from {} to {} messages", history_messages.len(), keep_count);
            }

            // Phase 2: Generate follow-up response (no tools, with thinking)
            tracing::debug!("Phase 2: Generating follow-up response (multimodal)");

            // Build Phase 2 prompt with tool results explicitly included so the second LLM
            // always receives them (history alone can be dropped or mishandled by backends).
            let phase2_prompt = build_phase2_prompt_with_tool_results(
                original_user_question.clone(),
                &tool_call_results,
            );
            tracing::debug!("Phase 2 prompt (multimodal) length: {} chars (with tool results)", phase2_prompt.len());

            let followup_stream_result = llm_interface.chat_stream_no_tools_no_thinking_with_history(
                &phase2_prompt, &history_messages
            ).await;

            let followup_stream = match followup_stream_result {
                Ok(stream) => stream,
                Err(e) => {
                    tracing::error!("Phase 2 LLM call failed: {}", e);
                    let fallback_text = format_tool_results(&tool_call_results);
                    for chunk in fallback_text.chars().collect::<Vec<_>>().chunks(20) {
                        let chunk_str: String = chunk.iter().collect();
                        if !chunk_str.is_empty() {
                            yield AgentEvent::content(chunk_str);
                        }
                    }
                    let pt = llm_interface.take_last_prompt_tokens().await;
                    match pt {
                        Some(t) => yield AgentEvent::end_with_tokens(t),
                        None => yield AgentEvent::end(),
                    }
                    return;
                }
            };

            let mut followup_stream = Box::pin(followup_stream);
            let mut final_response_content = String::new();
            let followup_start = Instant::now();

            while let Some(result) = StreamExt::next(&mut followup_stream).await {
                if followup_start.elapsed() > Duration::from_secs(30) {
                    tracing::warn!("Phase 2 timeout (>30s), forcing completion");
                    break;
                }

                match result {
                    Ok((chunk, is_thinking)) => {
                        if chunk.is_empty() {
                            continue;
                        }
                        if !is_thinking {
                            // Skip duplicate chunks (model repetition: same error/text sent twice)
                            let ct = chunk.trim();
                            if !ct.is_empty() {
                                if final_response_content.ends_with(ct) {
                                    continue;
                                }
                                if ct.len() > 30 && final_response_content.contains(ct) {
                                    continue;
                                }
                            }
                            yield AgentEvent::content(chunk.clone());
                            final_response_content.push_str(&chunk);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Phase 2 stream error: {}", e);
                        break;
                    }
                }
            }

            // Check for empty response OR hallucination detection
            if final_response_content.is_empty()
                || detect_hallucination(&final_response_content, &tool_call_results)
            {
                // Use rich formatter instead of simple fallback
                let fallback = format_tool_results(&tool_call_results);
                yield AgentEvent::content(fallback.clone());
                final_response_content = fallback;
            }

            // Clean any embedded tool call JSON from the final response content
            let cleaned_response_content = remove_tool_calls_from_response(&final_response_content);

            // Update the initial message with follow-up content
            {
                let mut state = internal_state.write().await;
                if let Some(last_msg) = state.memory.last_mut() {
                    if last_msg.role == "assistant" && last_msg.tool_calls.is_some() {
                        last_msg.content = cleaned_response_content.clone();
                    } else {
                        let final_msg = AgentMessage::assistant(&cleaned_response_content);
                        state.memory.push(final_msg);
                    }
                } else {
                    let final_msg = AgentMessage::assistant(&cleaned_response_content);
                    state.memory.push(final_msg);
                }
            }

            tracing::debug!("Multimodal tool execution and Phase 2 response complete");
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

/// Detect if the user's intent requires multi-step tool calling using LLM analysis.
///
/// NOTE: This function is currently unused as it adds latency.
/// Kept for potential future use with configuration option.
#[allow(dead_code)]
async fn detect_complex_intent_with_llm(llm_interface: &LlmInterface, user_message: &str) -> bool {
    let detection_prompt = format!(
        "分析以下用户请求是否需要**多步操作**才能完成。

用户请求: {}

判断标准（满足任一即返回 true）:
1. 条件判断: 如 \"如果A则B\"，\"当温度超过X时做Y\"
2. 链式操作: 如 \"先查询A，然后基于结果做B\"
3. 多个独立操作: 如 \"同时检查A和B\"，\"获取所有设备的数据\"
4. 需要分析后决定: 如 \"看看设备状态，如果有问题就告警\"
5. **数据分析**: 如 \"分析趋势\"，\"统计\"，\"对比\"，\"查看历史数据并分析\"
6. **多设备操作**: 如 \"所有\"，\"每个\"，\"全部设备\"

**关键**: 如果请求涉及\"分析\"、\"趋势\"、\"历史\"、\"所有\"等词，通常需要多步操作。

**只需要回答\"true\"或\"false\"，小写，不要其他内容。**",
        user_message
    );

    match llm_interface.chat_without_tools(&detection_prompt).await {
        Ok(response) => {
            let response_text = &response.text;
            let response_lower = response_text.to_lowercase();
            let is_complex = response_lower.contains("true")
                || response_lower.contains("是")
                || response_lower.contains("yes")
                || response_lower.contains("多步")
                || response_lower.contains("需要多次");
            let complexity_label = if is_complex { "complex" } else { "simple" };
            tracing::info!(
                "LLM intent detection: message='{}' => response='{}' => is_{}",
                user_message.chars().take(50).collect::<String>(),
                response_text.chars().take(50).collect::<String>(),
                complexity_label
            );
            is_complex
        }
        Err(e) => {
            tracing::warn!(
                "LLM complex intent detection failed: {}, falling back to keyword matching",
                e
            );
            // Fallback to keyword-based detection if LLM call fails
            is_complex_multi_step_intent_fallback(user_message)
        }
    }
}

/// Fallback keyword-based complex intent detection (used when LLM detection fails).
/// Detects patterns that indicate multi-step tool calling is needed.
fn is_complex_multi_step_intent_fallback(message: &str) -> bool {
    let complex_patterns = [
        // Conditional patterns
        ("如果", "就"),
        ("如果", "则"),
        ("当", "时"),
        ("超过", "就"),
        // Chained operation patterns
        ("查询", "然后"),
        ("检查", "之后"),
        ("根据", "然后"),
        // Multiple operation indicators
        ("并且", ""),
        ("同时", ""),
        // === NEW: Analysis and data patterns ===
        ("分析", ""),
        ("趋势", ""),
        ("统计", ""),
        ("历史", ""),
        ("对比", ""),
        ("比较", ""),
        ("所有", ""),
        ("每个", ""),
        ("全部", ""),
        // === NEW: Multi-step patterns ===
        ("查看", "并"),
        ("查询", "并"),
        ("获取", "后"),
        ("先", "后"),
    ];

    let lower = message.to_lowercase();

    for (first, second) in complex_patterns {
        if !second.is_empty() {
            if lower.contains(first) && lower.contains(second) {
                tracing::info!(
                    "Complex intent detected by keyword: '{}' + '{}'",
                    first,
                    second
                );
                return true;
            }
        } else if lower.contains(first) {
            tracing::info!("Complex intent detected by keyword: '{}'", first);
            return true;
        }
    }

    false
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

        let english_tokens = estimate_tokens(english);
        let chinese_tokens = estimate_tokens(chinese);

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
