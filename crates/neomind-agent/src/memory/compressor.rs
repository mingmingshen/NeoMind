//! Memory compressor — simplified to char-based eviction.
//!
//! Removes oldest entries (from the bottom of the file) when the file
//! exceeds the character limit. No LLM calls needed.

/// Result of an eviction operation.
#[derive(Debug, Clone)]
pub struct EvictionResult {
    /// Content after eviction (may be unchanged if under limit)
    pub content: String,
    /// Number of lines removed
    pub lines_removed: usize,
    /// Whether eviction was performed
    pub evicted: bool,
}

/// Evict content to fit within max_chars by removing lines from the bottom.
///
/// This is a simple, deterministic approach — no LLM needed.
/// Lines are removed from the bottom (oldest entries last in a markdown file).
pub fn evict_to_limit(content: &str, max_chars: usize) -> EvictionResult {
    let char_count = content.chars().count();
    if char_count <= max_chars {
        return EvictionResult {
            content: content.to_string(),
            lines_removed: 0,
            evicted: false,
        };
    }

    let mut lines: Vec<&str> = content.lines().collect();
    let mut removed = 0;

    while !lines.is_empty() {
        // Calculate total chars including newlines
        let total_with_newlines: usize = if lines.is_empty() {
            0
        } else {
            lines.iter().map(|l| l.len()).sum::<usize>() + (lines.len() - 1)
        };

        if total_with_newlines <= max_chars {
            break;
        }

        lines.pop();
        removed += 1;
    }

    EvictionResult {
        content: lines.join("\n"),
        lines_removed: removed,
        evicted: removed > 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_under_limit() {
        let content = "hello world";
        let result = evict_to_limit(content, 100);
        assert!(!result.evicted);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_over_limit_removes_lines() {
        let content = "line 1\nline 2\nline 3\nline 4\nline 5";
        let result = evict_to_limit(content, 20);
        assert!(result.evicted);
        assert!(result.lines_removed > 0);
        assert!(result.content.chars().count() <= 20);
    }

    #[test]
    fn test_empty_content() {
        let result = evict_to_limit("", 100);
        assert!(!result.evicted);
        assert_eq!(result.content, "");
    }

    #[test]
    fn test_exact_limit() {
        let content = "hello";
        let result = evict_to_limit(content, 5);
        assert!(!result.evicted);
        assert_eq!(result.content, "hello");
    }

    #[test]
    fn test_one_char_over_limit() {
        let content = "hello world";
        let result = evict_to_limit(content, 10);
        assert!(result.evicted);
        assert!(result.content.chars().count() <= 10);
    }

    #[test]
    fn test_multiline_with_newlines() {
        let content = "line 1\nline 2\nline 3";
        let total_with_newlines = content.chars().count() + 2; // 2 newlines
        let result = evict_to_limit(content, total_with_newlines);
        assert!(!result.evicted);
    }

    #[test]
    fn test_eviction_preserves_order() {
        let content = "line 1\nline 2\nline 3\nline 4\nline 5";
        let result = evict_to_limit(content, 15);
        assert!(result.evicted);
        // Should keep first lines, remove from bottom
        assert!(result.content.starts_with("line 1"));
        assert!(result.content.contains("line 2"));
    }
}
