//! Memory deduplication module
//!
//! Detects and merges similar memory entries using character n-gram Jaccard similarity.
//! Uses bigrams (2-character sequences) to properly handle Chinese and other
//! languages that don't use spaces between words.

use std::collections::HashSet;

/// Default n-gram size (bigrams)
const DEFAULT_NGRAM_SIZE: usize = 2;

/// Deduplication processor using character n-gram Jaccard similarity
#[derive(Debug, Clone)]
pub struct DedupProcessor {
    similarity_threshold: f32,
    ngram_size: usize,
}

impl DedupProcessor {
    /// Create a new dedup processor
    pub fn new(similarity_threshold: f32) -> Self {
        Self {
            similarity_threshold,
            ngram_size: DEFAULT_NGRAM_SIZE,
        }
    }

    /// Create with default threshold (0.85)
    pub fn with_defaults() -> Self {
        Self::new(0.85)
    }

    /// Create with custom n-gram size
    pub fn with_ngram_size(similarity_threshold: f32, ngram_size: usize) -> Self {
        Self {
            similarity_threshold,
            ngram_size: ngram_size.max(2),
        }
    }

    /// Generate character n-grams from text
    fn char_ngrams(s: &str, n: usize) -> HashSet<String> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() < n {
            // For very short strings, use the whole string as one n-gram
            return HashSet::from_iter([s.to_string()]);
        }
        (0..=chars.len() - n)
            .map(|i| chars[i..i + n].iter().collect())
            .collect()
    }

    /// Calculate Jaccard similarity between two texts using character n-grams
    pub fn jaccard_similarity(a: &str, b: &str) -> f32 {
        Self::jaccard_similarity_with_ngram(a, b, DEFAULT_NGRAM_SIZE)
    }

    /// Calculate Jaccard similarity with configurable n-gram size
    pub fn jaccard_similarity_with_ngram(a: &str, b: &str, n: usize) -> f32 {
        let set_a = Self::char_ngrams(a.trim(), n);
        let set_b = Self::char_ngrams(b.trim(), n);

        if set_a.is_empty() && set_b.is_empty() {
            return 1.0;
        }

        if set_a.is_empty() || set_b.is_empty() {
            return 0.0;
        }

        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();

        intersection as f32 / union as f32
    }

    /// Alias for backward compatibility
    pub fn similarity(a: &str, b: &str) -> f32 {
        Self::jaccard_similarity(a, b)
    }

    /// Find similar entry in existing list
    pub fn find_similar<'a>(&self, content: &str, existing: &'a [String]) -> Option<(usize, f32)> {
        for (i, entry) in existing.iter().enumerate() {
            let similarity = Self::jaccard_similarity_with_ngram(content, entry, self.ngram_size);
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
    fn test_jaccard_chinese_no_spaces() {
        // Previously this was broken - Chinese without spaces was treated as single token
        let sim = DedupProcessor::jaccard_similarity("用户偏好中文交互", "用户偏好中文");
        // With bigrams: 用户/户偏/偏好/好中/中文 vs 用户/户偏/偏好/好中/中文/交互
        // Intersection: 用户/户偏/偏好/好中/中文 = 5, Union = 7, sim = 5/7 ≈ 0.71
        assert!(sim > 0.5 && sim < 0.9, "sim was {}", sim);
    }

    #[test]
    fn test_jaccard_english() {
        let sim = DedupProcessor::jaccard_similarity("user prefers chinese", "user prefers english");
        // Bigrams overlap on "us"/"se"/"er"/"r "/" p"/"pr"/"re"/"ef"/"fe"/"er"/"rs" etc.
        assert!(sim > 0.3 && sim < 0.9);
    }

    #[test]
    fn test_jaccard_empty() {
        assert_eq!(DedupProcessor::jaccard_similarity("", ""), 1.0);
        assert_eq!(DedupProcessor::jaccard_similarity("内容", ""), 0.0);
    }

    #[test]
    fn test_find_similar_chinese() {
        let dedup = DedupProcessor::new(0.6);
        let existing = vec!["温度传感器读数为25度".to_string()];

        let result = dedup.find_similar("温度传感器读数为26度", &existing);
        assert!(result.is_some(), "Should detect similar Chinese entries");
        let (_, sim) = result.unwrap();
        assert!(sim >= 0.6, "sim was {}", sim);
    }

    #[test]
    fn test_find_similar_not_found() {
        let dedup = DedupProcessor::new(0.85);
        let existing = vec!["温度监控正常".to_string()];

        let result = dedup.find_similar("用户偏好中文", &existing);
        assert!(result.is_none());
    }

    #[test]
    fn test_dedup_chinese() {
        let dedup = DedupProcessor::new(0.85);
        let entries = vec![
            "用户偏好中文交互".to_string(),
            "用户偏好中文".to_string(), // Similar bigrams, but 0.71 < 0.85, so kept
            "温度正常".to_string(),
        ];

        let result = dedup.dedup(&entries);
        assert_eq!(result.total, 3);
        // All 3 kept since similarity < 0.85
        assert!(result.kept >= 2);
    }

    #[test]
    fn test_dedup_near_duplicates() {
        let dedup = DedupProcessor::new(0.7);
        let entries = vec![
            "Living room temperature is 25 degrees".to_string(),
            "Living room temperature is 26 degrees".to_string(), // Very similar
        ];

        let result = dedup.dedup(&entries);
        assert!(result.has_duplicates(), "Should detect near-duplicates");
    }

    #[test]
    fn test_merge() {
        let dedup = DedupProcessor::new(0.85);
        let existing = vec!["用户偏好中文交互".to_string()];
        let new = vec![
            "温度正常".to_string(),
            "用户偏好中文".to_string(), // Similar but below threshold
        ];

        let merged = dedup.merge(&existing, &new);
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn test_default_threshold() {
        let dedup = DedupProcessor::with_defaults();
        assert!((dedup.threshold() - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_short_strings() {
        let sim = DedupProcessor::jaccard_similarity("a", "b");
        assert!(sim < 0.5);

        let sim2 = DedupProcessor::jaccard_similarity("ab", "ab");
        assert!((sim2 - 1.0).abs() < 0.001);
    }
}
