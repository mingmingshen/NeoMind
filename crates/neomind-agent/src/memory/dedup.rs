//! Memory deduplication module
//!
//! Detects and merges similar memory entries using Jaccard similarity.

use std::collections::HashSet;

/// Deduplication processor using Jaccard similarity
#[derive(Debug, Clone)]
pub struct DedupProcessor {
    similarity_threshold: f32,
}

impl DedupProcessor {
    /// Create a new dedup processor
    pub fn new(similarity_threshold: f32) -> Self {
        Self {
            similarity_threshold,
        }
    }

    /// Create with default threshold (0.85)
    pub fn with_defaults() -> Self {
        Self::new(0.85)
    }

    /// Calculate Jaccard similarity between two texts
    pub fn jaccard_similarity(a: &str, b: &str) -> f32 {
        let words_a: HashSet<&str> = a.split_whitespace().collect();
        let words_b: HashSet<&str> = b.split_whitespace().collect();

        if words_a.is_empty() && words_b.is_empty() {
            return 1.0;
        }

        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        intersection as f32 / union as f32
    }

    /// Alias for backward compatibility
    pub fn similarity(a: &str, b: &str) -> f32 {
        Self::jaccard_similarity(a, b)
    }

    /// Find similar entry in existing list
    pub fn find_similar<'a>(&self, content: &str, existing: &'a [String]) -> Option<(usize, f32)> {
        for (i, entry) in existing.iter().enumerate() {
            let similarity = Self::jaccard_similarity(content, entry);
            if similarity >= self.similarity_threshold {
                return Some((i, similarity));
            }
        }
        None
    }

    /// Filter out similar entries, keeping first occurrence
    pub fn dedup(&self, entries: &[String]) -> DedupResult {
        let mut kept = Vec::new();
        let mut duplicates = Vec::new();

        for entry in entries {
            if self.find_similar(entry, &kept).is_none() {
                kept.push(entry.clone());
            } else {
                duplicates.push(entry.clone());
            }
        }

        DedupResult {
            total: entries.len(),
            kept: kept.len(),
            duplicates: duplicates.len(),
            unique_entries: kept,
            duplicate_entries: duplicates,
        }
    }

    /// Merge new entries with existing, avoiding duplicates
    pub fn merge(&self, existing: &[String], new: &[String]) -> Vec<String> {
        let mut result = existing.to_vec();

        for entry in new {
            if self.find_similar(entry, &result).is_none() {
                result.push(entry.clone());
            }
        }

        result
    }

    /// Get threshold
    pub fn threshold(&self) -> f32 {
        self.similarity_threshold
    }
}

impl Default for DedupProcessor {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Result of deduplication
#[derive(Debug, Clone)]
pub struct DedupResult {
    /// Total entries before dedup
    pub total: usize,
    /// Unique entries kept
    pub kept: usize,
    /// Duplicate entries removed
    pub duplicates: usize,
    /// The unique entries
    pub unique_entries: Vec<String>,
    /// The removed duplicates
    pub duplicate_entries: Vec<String>,
}

impl DedupResult {
    /// Check if any duplicates were found
    pub fn has_duplicates(&self) -> bool {
        self.duplicates > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_identical() {
        let sim = DedupProcessor::jaccard_similarity("用户偏好中文", "用户偏好中文");
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_different() {
        let sim = DedupProcessor::jaccard_similarity("用户偏好中文", "完全不同的内容");
        assert!(sim < 0.5);
    }

    #[test]
    fn test_jaccard_partial() {
        // Note: split_whitespace() treats Chinese text without spaces as single words
        let sim = DedupProcessor::jaccard_similarity("用户 偏好 中文 交互", "用户 偏好 中文");
        // 3 common words out of 4 total unique = 0.75
        assert!(sim > 0.5 && sim < 0.9);
    }

    #[test]
    fn test_jaccard_empty() {
        assert_eq!(DedupProcessor::jaccard_similarity("", ""), 1.0);
        assert_eq!(DedupProcessor::jaccard_similarity("内容", ""), 0.0);
    }

    #[test]
    fn test_find_similar_found() {
        let dedup = DedupProcessor::new(0.5);
        let existing = vec!["用户 偏好 中文 交互".to_string()];

        let result = dedup.find_similar("用户 偏好 中文", &existing);
        // With spaces, we get 3/4 = 0.75 similarity
        assert!(result.is_some());
        let (idx, sim) = result.unwrap();
        assert_eq!(idx, 0);
        assert!(sim >= 0.5);
    }

    #[test]
    fn test_find_similar_not_found() {
        let dedup = DedupProcessor::new(0.85);
        let existing = vec!["温度监控正常".to_string()];

        let result = dedup.find_similar("用户偏好中文", &existing);
        assert!(result.is_none());
    }

    #[test]
    fn test_dedup() {
        let dedup = DedupProcessor::new(0.85);
        let entries = vec![
            "用户 偏好 中文".to_string(),
            "用户 偏好 中文 交互".to_string(), // Similar (3/4 = 0.75 < 0.85, so kept)
            "温度 正常".to_string(),
        ];

        let result = dedup.dedup(&entries);
        assert_eq!(result.total, 3);
        // All 3 are kept since 0.75 < 0.85 threshold
        assert_eq!(result.kept, 3);
        assert_eq!(result.duplicates, 0);
    }

    #[test]
    fn test_merge() {
        let dedup = DedupProcessor::new(0.85);
        let existing = vec!["用户 偏好 中文".to_string()];
        let new = vec![
            "温度 正常".to_string(),
            "用户 偏好 中文 交互".to_string(), // 0.75 similarity < 0.85 threshold
        ];

        let merged = dedup.merge(&existing, &new);
        // With 0.85 threshold, all entries are kept since 0.75 < 0.85
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn test_default_threshold() {
        let dedup = DedupProcessor::with_defaults();
        assert!((dedup.threshold() - 0.85).abs() < 0.001);
    }
}
