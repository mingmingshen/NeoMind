//! Memory compression module
//!
//! Compresses memory entries using LLM summarization and importance decay.

use std::sync::Arc;
use neomind_storage::{CompressionConfig, MemoryCategory, MarkdownMemoryStore};
use neomind_core::llm::backend::{GenerationParams, LlmInput, LlmRuntime};
use serde::{Deserialize, Serialize};
use crate::error::Result;

/// Default minimum entries before compression
const DEFAULT_MIN_ENTRIES: usize = 2;

/// Result of compression operation
#[derive(Debug, Clone, Default)]
pub struct CompressionResult {
    /// Total entries before compression
    pub total_before: usize,
    /// Entries kept as-is
    pub kept: usize,
    /// Entries merged/compressed
    pub compressed: usize,
    /// Entries deleted (below threshold)
    pub deleted: usize,
}

impl CompressionResult {
    /// Create a new compression result
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any changes were made
    pub fn has_changes(&self) -> bool {
        self.compressed > 0 || self.deleted > 0
    }

    /// Get total entries after compression
    pub fn total_after(&self) -> usize {
        self.kept + self.compressed
    }
}

/// Memory compressor with LLM support
pub struct MemoryCompressor {
    config: CompressionConfig,
    llm: Arc<dyn LlmRuntime>,
}

impl MemoryCompressor {
    /// Create with LLM runtime
    pub fn new(llm: Arc<dyn LlmRuntime>) -> Self {
        Self::with_config(CompressionConfig::default(), llm)
    }

    /// Create with custom config and LLM runtime
    pub fn with_config(config: CompressionConfig, llm: Arc<dyn LlmRuntime>) -> Self {
        Self { config, llm }
    }

    /// Get max entries for a category
    pub fn max_entries(&self, category: &MemoryCategory) -> usize {
        self.config
            .max_entries
            .get(category.filename())
            .copied()
            .unwrap_or_else(|| category.max_entries())
    }

    /// Get min importance threshold (global)
    pub fn min_importance(&self) -> u8 {
        self.config.min_importance
    }

    /// Get min entries before compression is needed
    pub fn min_entries(&self, _category: &MemoryCategory) -> usize {
        DEFAULT_MIN_ENTRIES
    }

    /// Apply importance decay based on age
    pub fn decay_importance(&self, current: u8, days_since_update: u64) -> u8 {
        if days_since_update == 0 || self.config.decay_period_days == 0 {
            return current;
        }

        let periods = days_since_update / self.config.decay_period_days as u64;
        let decayed = (current as f32) * 0.9_f32.powi(periods as i32);
        decayed as u8
    }

    /// Check if an entry should be deleted based on importance
    pub fn should_delete(&self, importance: u8) -> bool {
        importance < self.config.min_importance
    }

    /// Compress a category using LLM
    pub async fn compress(
        &self,
        store: &Arc<tokio::sync::RwLock<MarkdownMemoryStore>>,
        category: MemoryCategory,
    ) -> Result<CompressionResult> {
        let store_guard = store.read().await;

        // Read current content
        let content = store_guard
            .read_category(&category)
            .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

        // Parse entries
        let mut entries = self.parse_entries(&content);
        let original_count = entries.len();

        let min_entries = self.min_entries(&category);
        if entries.len() <= min_entries {
            tracing::debug!(
                category = ?category,
                count = entries.len(),
                min = min_entries,
                "Skipping compression: not enough entries"
            );
            return Ok(CompressionResult::default());
        }

        // === Apply importance decay ===
        let now = chrono::Utc::now();
        let mut decayed_count = 0;
        for entry in &mut entries {
            if let Ok(entry_date) = chrono::NaiveDate::parse_from_str(&entry.timestamp, "%Y-%m-%d") {
                let entry_datetime = entry_date.and_hms_opt(0, 0, 0).unwrap();
                let days_since = (now.date_naive() - entry_datetime.date()).num_days().max(0) as u64;
                let original_importance = entry.importance;
                entry.importance = self.decay_importance(entry.importance, days_since);
                if entry.importance != original_importance {
                    decayed_count += 1;
                }
            }
        }

        if decayed_count > 0 {
            tracing::info!(
                category = ?category,
                decayed = decayed_count,
                "Applied importance decay to entries"
            );
        }

        let min_importance = self.min_importance();

        // Filter out entries below threshold (deleted by decay)
        let before_filter = entries.len();
        let entries_to_compress: Vec<MemoryEntry> = entries
            .into_iter()
            .filter(|e| e.importance >= min_importance)
            .collect();
        let deleted_count = before_filter - entries_to_compress.len();

        if entries_to_compress.is_empty() {
            tracing::info!(
                category = ?category,
                deleted = deleted_count,
                "All entries below importance threshold after decay"
            );

            // Write empty content
            drop(store_guard);
            let store_guard = store.write().await;
            store_guard
                .write_category(&category, "")
                .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

            return Ok(CompressionResult {
                total_before: original_count,
                kept: 0,
                compressed: 0,
                deleted: deleted_count,
            });
        }

        // Build compression prompt
        let prompt = self.build_compression_prompt(&entries_to_compress, &category, min_importance);

        tracing::info!(
            category = ?category,
            model = %self.llm.model_name(),
            entry_count = entries_to_compress.len(),
            decayed = decayed_count,
            deleted_by_decay = deleted_count,
            "Calling LLM for memory compression"
        );

        // Drop the lock before LLM call
        drop(store_guard);

        // Call LLM
        let input = LlmInput::new(prompt).with_params(GenerationParams {
            temperature: Some(0.3),
            max_tokens: Some(1024),
            thinking_enabled: Some(false), // Disable thinking to avoid wasting tokens on reasoning
            ..Default::default()
        });

        let response = self
            .llm
            .generate(input)
            .await
            .map_err(|e| crate::error::NeoMindError::Llm(e.to_string()))?;

        tracing::info!(
            category = ?category,
            response_length = response.text.len(),
            "LLM compression response received"
        );

        // Parse response
        let summaries = self.parse_compression_response(&response.text);

        // Write back - try to preserve original timestamps for merged entries
        let store_guard = store.write().await;
        let mut new_content = String::new();
        let mut compressed_count = 0;

        for summary in &summaries {
            // Find the earliest timestamp from source entries that contributed to this summary
            let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let preserved_date = summary.source_dates.iter().min()
                .map(|d| d.as_str())
                .unwrap_or(today_str.as_str());

            let entry = format!(
                "- [{}] {} [importance: {}]\n",
                preserved_date,
                summary.content,
                summary.importance
            );
            new_content.push_str(&entry);
            compressed_count += 1;
        }

        store_guard
            .write_category(&category, &new_content)
            .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

        tracing::info!(
            category = ?category,
            original = original_count,
            kept = summaries.len(),
            compressed = compressed_count,
            deleted = deleted_count,
            "Compression completed"
        );

        Ok(CompressionResult {
            total_before: original_count,
            kept: summaries.len(),
            compressed: compressed_count,
            deleted: deleted_count,
        })
    }

    /// Parse entries from markdown content
    fn parse_entries(&self, content: &str) -> Vec<MemoryEntry> {
        let mut entries = Vec::new();

        // Regex to parse: - [2026-04-02] Content [importance: 80]
        let re = regex::Regex::new(r"- \[([^\]]+)\]\s*(.+?)\s*\[importance:\s*(\d+)\]").unwrap();

        for line in content.lines() {
            let line = line.trim();
            if !line.starts_with("- [") {
                continue;
            }

            if let Some(caps) = re.captures(line) {
                let timestamp = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                let content = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                let importance: u8 = caps.get(3)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(50);

                entries.push(MemoryEntry {
                    timestamp,
                    content,
                    importance,
                });
            }
        }

        entries
    }

    /// Build compression prompt
    fn build_compression_prompt(
        &self,
        entries: &[MemoryEntry],
        category: &MemoryCategory,
        min_importance: u8,
    ) -> String {
        let entries_text = entries
            .iter()
            .map(|e| format!("- [{}] {} [importance: {}]", e.timestamp, e.content, e.importance))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"Compress these memory entries for "{}". Merge redundant entries, keep unique facts. Max 120 chars per entry.

## Current Entries
{}

## Rules
1. **Merge** entries about the same topic into ONE concise fact
2. **Drop** entries with importance below {}
3. **Keep unique entries as-is** if they don't overlap with others
4. Each output entry must be max 120 characters — split if needed
5. **Preserve earliest date** — when merging entries, use the earliest date from the source entries
6. For merged entries, list ALL source entry dates in source_dates array

## Output Format (JSON only, no extra text)
{{"summaries":[{{"content":"<one fact, max 120 chars>","importance":<number>,"source_dates":["<date1>","<date2>"]}}]}}

## Good Example
Input:
- [2026-04-01] User prefers Chinese [importance: 80]
- [2026-04-02] User speaks Chinese [importance: 60]
- [2026-04-03] User likes concise responses [importance: 70]

Output:
{{"summaries":[{{"content":"User prefers Chinese language and concise responses","importance":80,"source_dates":["2026-04-01","2026-04-02","2026-04-03"]}}]}}

Now generate the JSON response:"#,
            category.display_name(),
            entries_text,
            min_importance,
        )
    }

    /// Parse compression response from LLM
    fn parse_compression_response(&self, response: &str) -> Vec<CompressionSummary> {
        // Strip markdown code fences
        let cleaned = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        // Find JSON object
        let start = match cleaned.find('{') {
            Some(s) => s,
            None => {
                tracing::warn!("No JSON object found in compression response");
                return Vec::new();
            }
        };
        let end = match cleaned.rfind('}') {
            Some(e) => e,
            None => {
                tracing::warn!("No closing brace in compression response");
                return Vec::new();
            }
        };

        let json = &cleaned[start..=end];

        match serde_json::from_str::<CompressionResponse>(json) {
            Ok(parsed) => parsed.summaries,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse compression response, trying fallback extraction");
                self.extract_summaries_fallback(response)
            }
        }
    }

    /// Fallback extraction when JSON parsing fails
    fn extract_summaries_fallback(&self, response: &str) -> Vec<CompressionSummary> {
        let mut summaries = Vec::new();
        let content_re = regex::Regex::new(r#""content"\s*:\s*"([^"]+)""#).unwrap();
        let importance_re = regex::Regex::new(r#""importance"\s*:\s*(\d+)"#).unwrap();

        let contents: Vec<&str> = content_re
            .captures_iter(response)
            .filter_map(|c| c.get(1).map(|m| m.as_str()))
            .collect();

        let importances: Vec<u8> = importance_re
            .captures_iter(response)
            .filter_map(|c| c.get(1).and_then(|m| m.as_str().parse().ok()))
            .collect();

        for (i, content) in contents.iter().enumerate() {
            let importance = importances.get(i).copied().unwrap_or(50);
            summaries.push(CompressionSummary {
                content: content.to_string(),
                importance,
                source_dates: Vec::new(),
            });
        }

        if summaries.is_empty() {
            tracing::warn!("Fallback extraction found no summaries");
        } else {
            tracing::info!(count = summaries.len(), "Fallback extraction succeeded");
        }

        summaries
    }

    /// Get configuration
    pub fn config(&self) -> &CompressionConfig {
        &self.config
    }
}

/// Parsed memory entry
#[derive(Debug, Clone)]
struct MemoryEntry {
    timestamp: String,
    content: String,
    importance: u8,
}

/// Compression summary from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompressionSummary {
    content: String,
    importance: u8,
    /// Source entry dates preserved for merged entries
    #[serde(default)]
    source_dates: Vec<String>,
}

/// Parsed compression response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompressionResponse {
    summaries: Vec<CompressionSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_result() {
        let result = CompressionResult {
            total_before: 10,
            kept: 3,
            compressed: 2,
            deleted: 5,
        };
        assert_eq!(result.total_after(), 5);
        assert!(result.has_changes());
    }

    #[test]
    fn test_decay_importance() {
        let llm = MockLlm;
        let compressor = MemoryCompressor::new(Arc::new(llm));

        // No decay for recent entries
        assert_eq!(compressor.decay_importance(80, 0), 80);

        // Decay over time: 0.9^1 = 0.9, so 80 * 0.9 = 72
        let decayed = compressor.decay_importance(80, 30);
        assert_eq!(decayed, 72);

        // More decay over longer time: 0.9^2 = 0.81, so 80 * 0.81 = 64.8 -> 64
        let decayed2 = compressor.decay_importance(80, 60);
        assert_eq!(decayed2, 64);
    }

    #[test]
    fn test_should_delete() {
        let llm = MockLlm;
        let compressor = MemoryCompressor::new(Arc::new(llm));
        // Default min_importance is 20, so < 20 gets deleted
        assert!(!compressor.should_delete(30));
        assert!(!compressor.should_delete(20)); // Equal to threshold, not deleted
        assert!(compressor.should_delete(10));  // Below threshold, deleted
        assert!(compressor.should_delete(0));   // Zero importance, deleted
    }

    #[test]
    fn test_parse_entries() {
        let llm = MockLlm;
        let compressor = MemoryCompressor::new(Arc::new(llm));

        let content = "# Title\n\n- [2026-04-01] User prefers Chinese [importance: 80]\n- [2026-04-02] Temperature 25C [importance: 60]\n";
        let entries = compressor.parse_entries(content);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].timestamp, "2026-04-01");
        assert_eq!(entries[0].content, "User prefers Chinese");
        assert_eq!(entries[0].importance, 80);
    }

    #[test]
    fn test_compression_summary_source_dates() {
        let json = r#"{"summaries":[{"content":"User prefers Chinese","importance":80,"source_dates":["2026-04-01","2026-04-02"]}]}"#;
        let response: CompressionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.summaries.len(), 1);
        assert_eq!(response.summaries[0].source_dates, vec!["2026-04-01", "2026-04-02"]);
    }

    // Mock LLM for testing
    struct MockLlm;

    #[async_trait::async_trait]
    impl LlmRuntime for MockLlm {
        fn backend_id(&self) -> neomind_core::llm::backend::BackendId {
            neomind_core::llm::backend::BackendId::new("mock")
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }

        async fn generate(
            &self,
            _input: neomind_core::llm::backend::LlmInput,
        ) -> std::result::Result<neomind_core::llm::backend::LlmOutput, neomind_core::llm::backend::LlmError> {
            Ok(neomind_core::llm::backend::LlmOutput {
                text: r#"{"summaries":[{"content":"Test summary","importance":70,"source_dates":[]}]}"#.to_string(),
                finish_reason: neomind_core::llm::backend::FinishReason::Stop,
                usage: None,
                thinking: None,
            })
        }

        async fn generate_stream(
            &self,
            _input: neomind_core::llm::backend::LlmInput,
        ) -> std::result::Result<
            std::pin::Pin<Box<dyn futures::Stream<Item = neomind_core::llm::backend::StreamChunk> + Send>>,
            neomind_core::llm::backend::LlmError,
        > {
            unimplemented!()
        }

        fn max_context_length(&self) -> usize {
            4096
        }
    }
}
