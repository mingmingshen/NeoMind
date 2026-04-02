//! Memory compression module
//!
//! Compresses memory entries using LLM summarization and importance decay.

use neomind_storage::{CompressionConfig, MemoryCategory};

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

/// Memory compressor with importance decay
pub struct MemoryCompressor {
    config: CompressionConfig,
}

impl MemoryCompressor {
    /// Create a new compressor
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(CompressionConfig::default())
    }

    /// Build LLM prompt for compression
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

    /// Get max entries for a category
    pub fn max_entries(&self, category: &MemoryCategory) -> usize {
        self.config
            .max_entries
            .get(&category.to_string())
            .copied()
            .unwrap_or_else(|| category.max_entries())
    }

    /// Get configuration
    pub fn config(&self) -> &CompressionConfig {
        &self.config
    }
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

    #[test]
    fn test_decay_no_decay() {
        let compressor = MemoryCompressor::with_defaults();
        // No decay when days = 0
        assert_eq!(compressor.decay_importance(80, 0), 80);
    }

    #[test]
    fn test_decay_applied() {
        let compressor = MemoryCompressor::with_defaults();
        // Should decay after 30 days (default decay period)
        let decayed = compressor.decay_importance(100, 30);
        assert!(decayed < 100);
        assert!(decayed > 70); // Should be around 90
    }

    #[test]
    fn test_decay_multiple_periods() {
        let compressor = MemoryCompressor::with_defaults();
        // 60 days = 2 decay periods
        let decayed = compressor.decay_importance(100, 60);
        // 100 * 0.9 * 0.9 ≈ 81 (may vary due to floating point)
        assert!(decayed >= 79 && decayed <= 82);
    }

    #[test]
    fn test_should_delete() {
        let compressor = MemoryCompressor::with_defaults();
        // Default min_importance is 20
        assert!(!compressor.should_delete(50));
        assert!(!compressor.should_delete(20));
        assert!(compressor.should_delete(19));
        assert!(compressor.should_delete(0));
    }

    #[test]
    fn test_max_entries_default() {
        let compressor = MemoryCompressor::with_defaults();
        // Uses category defaults
        assert_eq!(compressor.max_entries(&MemoryCategory::UserProfile), 50);
        assert_eq!(
            compressor.max_entries(&MemoryCategory::DomainKnowledge),
            100
        );
        assert_eq!(compressor.max_entries(&MemoryCategory::TaskPatterns), 80);
        assert_eq!(compressor.max_entries(&MemoryCategory::SystemEvolution), 30);
    }
}
