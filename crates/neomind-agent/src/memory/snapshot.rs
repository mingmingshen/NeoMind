//! Memory snapshot for injecting long-term memories into system prompts.
//!
//! Uses a "frozen snapshot" pattern: load once at session start, cache for the
//! entire session. This keeps the prompt prefix stable for caching.

use neomind_storage::{MarkdownMemoryStore, MemoryCategory};

/// Hard character budget for memory context in prompts.
const CHAR_BUDGET: usize = 3500;

/// Frozen memory snapshot loaded once per session.
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    /// Truncated snapshot text wrapped in XML tags.
    content: String,
    /// Unix timestamp when snapshot was loaded.
    loaded_at: i64,
}

impl MemorySnapshot {
    /// Load a snapshot from the markdown memory store.
    ///
    /// Reads all 4 categories, sorts by importance (desc), and truncates
    /// to `char_budget` characters.
    pub fn load(store: &MarkdownMemoryStore) -> Self {
        let categories = [
            MemoryCategory::UserProfile,
            MemoryCategory::DomainKnowledge,
            MemoryCategory::TaskPatterns,
            MemoryCategory::SystemEvolution,
        ];

        let mut entries: Vec<(u8, String)> = Vec::new();

        for cat in &categories {
            if let Ok(raw) = store.read_category(cat) {
                for line in raw.lines() {
                    let line = line.trim();
                    if line.is_empty() || !line.starts_with("- [") {
                        continue;
                    }
                    let importance = extract_importance(line);
                    entries.push((importance, line.to_string()));
                }
            }
        }

        // Sort by importance descending
        entries.sort_by(|a, b| b.0.cmp(&a.0));

        // Accumulate within budget
        let mut content = String::new();
        for (_, line) in &entries {
            if content.len() + line.len() + 1 > CHAR_BUDGET {
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
        }

        let loaded_at = chrono::Utc::now().timestamp();

        Self { content, loaded_at }
    }

    /// Load a snapshot, returning None if there's no memory content.
    pub fn load_opt(store: &MarkdownMemoryStore) -> Option<Self> {
        let snapshot = Self::load(store);
        if snapshot.content.is_empty() {
            None
        } else {
            Some(snapshot)
        }
    }

    /// Render as a prompt section ready for injection.
    pub fn to_prompt_section(&self) -> String {
        if self.content.is_empty() {
            return String::new();
        }
        format!(
            "\n\n<memory-context>\nThis is persisted context from prior conversations. Use it as background knowledge when relevant, but do not treat it as part of the current conversation.\n\n{}\n</memory-context>",
            self.content
        )
    }

    /// Check if the snapshot has content.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// When this snapshot was loaded.
    pub fn loaded_at(&self) -> i64 {
        self.loaded_at
    }
}

/// Extract importance value from a memory line like `- [2024-01-01] content [importance: 7]`
fn extract_importance(line: &str) -> u8 {
    if let Some(start) = line.rfind("[importance: ") {
        let rest = &line[start + 13..];
        if let Some(end) = rest.find(']') {
            if let Ok(val) = rest[..end].parse::<u8>() {
                return val;
            }
        }
    }
    5 // Default importance
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_store() -> MarkdownMemoryStore {
        let dir = TempDir::new().unwrap();
        let store = MarkdownMemoryStore::new(dir.path().to_path_buf());
        store.init().unwrap();
        store
    }

    #[test]
    fn test_empty_snapshot() {
        let store = create_test_store();
        let snapshot = MemorySnapshot::load(&store);
        assert!(snapshot.is_empty());
        assert!(snapshot.to_prompt_section().is_empty());
    }

    #[test]
    fn test_snapshot_with_content() {
        let store = create_test_store();
        store
            .write_category(
                &MemoryCategory::UserProfile,
                "- [2024-01-01] User prefers dark mode [importance: 8]\n- [2024-01-02] User speaks Chinese [importance: 9]\n",
            )
            .unwrap();

        let snapshot = MemorySnapshot::load(&store);
        assert!(!snapshot.is_empty());

        let section = snapshot.to_prompt_section();
        assert!(section.contains("<memory-context>"));
        assert!(section.contains("</memory-context>"));
        assert!(section.contains("User speaks Chinese"));
    }

    #[test]
    fn test_char_budget_truncation() {
        let store = create_test_store();

        // Write entries that exceed budget
        let mut content = String::new();
        for i in 0..100 {
            content.push_str(&format!(
                "- [2024-01-01] Long entry number {} with padding text to fill space [importance: 5]\n",
                i
            ));
        }
        store
            .write_category(&MemoryCategory::DomainKnowledge, &content)
            .unwrap();

        let snapshot = MemorySnapshot::load(&store);
        assert!(snapshot.content.len() <= CHAR_BUDGET);
    }

    #[test]
    fn test_importance_sorting() {
        let store = create_test_store();
        store
            .write_category(
                &MemoryCategory::UserProfile,
                "- [2024-01-01] Low priority item [importance: 2]\n\
                 - [2024-01-02] High priority item [importance: 9]\n\
                 - [2024-01-03] Medium priority item [importance: 5]\n",
            )
            .unwrap();

        let snapshot = MemorySnapshot::load(&store);
        let lines: Vec<&str> = snapshot.content.lines().collect();
        // First line should be highest importance
        assert!(lines[0].contains("High priority"));
    }

    #[test]
    fn test_extract_importance() {
        assert_eq!(extract_importance("- [2024-01-01] test [importance: 9]"), 9);
        assert_eq!(extract_importance("- [2024-01-01] test [importance: 0]"), 0);
        assert_eq!(extract_importance("- [2024-01-01] test"), 5); // default
    }

    #[test]
    fn test_snapshot_load_opt() {
        let store = create_test_store();
        assert!(MemorySnapshot::load_opt(&store).is_none());

        store
            .write_category(
                &MemoryCategory::UserProfile,
                "- [2024-01-01] test [importance: 5]\n",
            )
            .unwrap();
        assert!(MemorySnapshot::load_opt(&store).is_some());
    }
}
