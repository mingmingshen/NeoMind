//! Memory compression module
//!
//! Compresses memory entries using LLM summarization and importance decay.

use std::sync::Arc;
use neomind_storage::{CompressionConfig, MemoryCategory, MarkdownMemoryStore};
use neomind_core::llm::backend::{GenerationParams, LlmInput, LlmRuntime};
use serde::{Deserialize, Serialize};
use crate::error::Result;

/// Default minimum entries before compression
const DEFAULT_MIN_ENTRIES: usize = 10;

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
        let entries = self.parse_entries(&content);
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

        let min_importance = self.min_importance();

        // Build compression prompt
        let prompt = self.build_compression_prompt(&entries, &category, min_importance);

        tracing::info!(
            category = ?category,
            model = %self.llm.model_name(),
            entry_count = entries.len(),
            "Calling LLM for memory compression"
        );

        // Drop the lock before LLM call
        drop(store_guard);

        // Call LLM
        let input = LlmInput::new(prompt).with_params(GenerationParams {
            temperature: Some(0.3),
            max_tokens: Some(1024),
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

        // Write back
        let store_guard = store.write().await;
        let mut new_content = String::new();
        let mut compressed_count = 0;

        for summary in &summaries {
            let entry = format!(
                "- [{}] {} [importance: {}]\n",
                chrono::Utc::now().format("%Y-%m-%d"),
                summary.content,
                summary.importance
            );
            new_content.push_str(&entry);
            compressed_count += 1;
        }

        // Calculate deleted count (entries removed due to low importance)
        let deleted_count = entries
            .iter()
            .filter(|e| e.importance < min_importance)
            .count();

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

        let today = chrono::Utc::now().format("%Y-%m-%d");

        format!(
            r#"Compress the following memory entries for the {} category.

## Current Entries
{}

## Instructions
1. Merge similar or redundant entries
2. Keep unique, important information
3. Remove entries with importance below {}
4. Preserve timestamps (use today's date: {})

## Output Format (JSON only)
{{"summaries":[{{"content":"merged content","importance":70}}]}}

## Example
If entries are:
- [2026-04-01] User prefers Chinese [importance: 80]
- [2026-04-01] User speaks Chinese [importance: 60]

Output:
{{"summaries":[{{"content":"User prefers Chinese language","importance":80}}]}}

Now generate the JSON response:"#,
            category.display_name(),
            entries_text,
            min_importance,
            today
        )
    }

    /// Parse compression response from LLM
    fn parse_compression_response(&self, response: &str) -> Vec<CompressionSummary> {
        // Find JSON object
        let start = match response.find('{') {
            Some(s) => s,
            None => {
                tracing::warn!("No JSON object found in compression response");
                return Vec::new();
            }
        };
        let end = match response.rfind('}') {
            Some(e) => e,
            None => {
                tracing::warn!("No closing brace in compression response");
                return Vec::new();
            }
        };

        let json = &response[start..=end];

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
            });
        }

        if summaries.is_empty() {
            tracing::warn!("Fallback extraction found no summaries");
        } else {
            tracing::info!(count = summaries.len(), "Fallback extraction succeeded");
        }

        summaries
    }

    /// Build LLM prompt for compression (static helper)
    pub fn build_prompt(entries: &str, category: &MemoryCategory) -> String {
        format!(
            r#"Compress the following memory entries.

## Category: {}

## Entries
{}

## Compression Rules
1. Merge similar content
2. Extract general patterns
3. Keep key values and thresholds
4. Remove redundancy
5. Write in English by default, adapt to detected language if needed

## Output
Output Markdown formatted summary containing:
- Brief title
- Key points list
"#,
            category.display_name(),
            entries
        )
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
    fn test_build_prompt() {
        let entries =
            "- [2026-04-01] User prefers Chinese\n- [2026-04-02] User likes concise responses";
        let prompt = MemoryCompressor::build_prompt(entries, &MemoryCategory::UserProfile);
        assert!(prompt.contains("User Profile"));
        assert!(prompt.contains(entries));
    }
}
