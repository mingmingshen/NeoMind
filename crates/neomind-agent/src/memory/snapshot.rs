//! Memory snapshot for injecting long-term memories into system prompts.
//!
//! Uses a "frozen snapshot" pattern: load once at session start, cache for the
//! entire session. This keeps the prompt prefix stable for caching.
//!
//! ## Hard-loaded (always in system prompt)
//! - `USER.md` (max 2000 chars) — user profile
//! - `KNOWLEDGE.md` (max 3000 chars) — system knowledge
//!
//! ## On-demand (AI reads via memory tool when needed)
//! - `agents/{agent_id}.md` — agent experiences
//! - `custom/{name}.md` — domain-specific files
//!
//! Total budget: 5000 chars (user + knowledge)

use neomind_storage::MarkdownMemoryStore;

/// Hard character budget for memory context in prompts (user + knowledge only).
const CHAR_BUDGET: usize = 5000;

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
    /// Only hard-loads USER.md and KNOWLEDGE.md.
    /// Agent summaries and custom files are available on-demand via the memory tool.
    pub fn load(store: &MarkdownMemoryStore) -> Self {
        let handle = tokio::runtime::Handle::try_current()
            .or_else(|_| tokio::runtime::Runtime::new().map(|rt| rt.handle().clone()))
            .unwrap();

        // Read persistent files only
        let user = tokio::task::block_in_place(|| handle.block_on(store.read_file("user")))
            .unwrap_or_default();

        let knowledge =
            tokio::task::block_in_place(|| handle.block_on(store.read_file("knowledge")))
                .unwrap_or_default();

        let combined = format!("## User\n{user}\n\n## Knowledge\n{knowledge}");

        let content = if combined.chars().count() <= CHAR_BUDGET {
            combined
        } else {
            // Truncate knowledge to fit, but if even user alone exceeds budget,
            // truncate user as a last resort
            let truncated = truncate_with_priority(&combined, CHAR_BUDGET);
            if truncated.chars().count() > CHAR_BUDGET {
                // User section alone exceeds budget — hard truncate
                truncate_chars(&truncated, CHAR_BUDGET)
            } else {
                truncated
            }
        };

        let loaded_at = chrono::Utc::now().timestamp();

        Self { content, loaded_at }
    }

    /// Load a snapshot, returning None if there's no meaningful memory content.
    /// Only returns Some if at least one section has actual content (not just headers).
    pub fn load_opt(store: &MarkdownMemoryStore) -> Option<Self> {
        let snapshot = Self::load(store);
        if snapshot.content.is_empty() {
            return None;
        }
        // Check if there's actual content beyond section headers
        let has_content = snapshot.content.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("## ") && !trimmed.starts_with("### ")
        });
        if has_content {
            Some(snapshot)
        } else {
            None
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

/// Truncate content with priority preservation.
///
/// Priority order:
/// 1. User section (never truncated)
/// 2. Knowledge section (truncated only if User exceeds budget)
fn truncate_with_priority(content: &str, max_chars: usize) -> String {
    let sections = parse_sections(content);

    let user_len = sections.get("User").map_or(0, |s| s.chars().count());
    let knowledge_len = sections.get("Knowledge").map_or(0, |s| s.chars().count());

    let mut result = String::new();
    let mut remaining = max_chars;

    // Add User section (always preserve)
    if let Some(user) = sections.get("User") {
        result.push_str("## User\n");
        result.push_str(user);
        remaining = remaining.saturating_sub("## User\n".chars().count() + user_len);
    }

    // Add Knowledge section (truncate if needed)
    if remaining > 0 {
        if let Some(knowledge) = sections.get("Knowledge") {
            if knowledge_len <= remaining {
                result.push_str("\n\n## Knowledge\n");
                result.push_str(knowledge);
            } else {
                let truncated = truncate_chars(knowledge, remaining.saturating_sub(20));
                result.push_str("\n\n## Knowledge\n");
                result.push_str(&truncated);
            }
        }
    }

    result
}

/// Parse content into sections by "## " headings.
fn parse_sections(content: &str) -> std::collections::HashMap<String, String> {
    let mut sections: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut current_section = String::new();
    let mut current_name = String::new();

    for line in content.lines() {
        if line.starts_with("## ") {
            if !current_name.is_empty() {
                sections.insert(current_name.clone(), current_section.trim().to_string());
            }
            current_name = line[3..].to_string();
            current_section = String::new();
        } else {
            current_section.push_str(line);
            current_section.push('\n');
        }
    }

    if !current_name.is_empty() {
        sections.insert(current_name, current_section.trim().to_string());
    }

    sections
}

/// Truncate string to max characters, trying to cut at sentence boundaries.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        return s.to_string();
    }

    let limit = max_chars;
    for i in (limit.saturating_sub(100)..limit).rev() {
        if i < chars.len() {
            let c = chars[i];
            if c == '.' || c == '!' || c == '?' {
                return chars[..=i].iter().collect();
            }
        }
    }

    chars[..limit].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_store() -> (MarkdownMemoryStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = MarkdownMemoryStore::new(dir.path().to_path_buf());
        store.init().expect("Failed to init test store");
        (store, dir)
    }

    #[test]
    fn test_char_budget_is_5000() {
        assert_eq!(CHAR_BUDGET, 5000);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_empty_snapshot() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        let store = MarkdownMemoryStore::new(dir.path().to_path_buf());
        assert!(MemorySnapshot::load_opt(&store).is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_snapshot_with_content() {
        let (store, _dir) = create_test_store();

        store
            .write_file("user", "User prefers dark mode\nUser speaks Chinese\n")
            .await
            .unwrap();

        store
            .write_file(
                "knowledge",
                "## System Resources\n- Device: TestDevice\n\n## Domain Knowledge\n- Location: TestRoom\n",
            )
            .await
            .unwrap();

        let snapshot = MemorySnapshot::load(&store);
        assert!(!snapshot.is_empty());

        let section = snapshot.to_prompt_section();
        assert!(section.contains("<memory-context>"));
        assert!(section.contains("</memory-context>"));
        assert!(section.contains("User prefers dark mode"));
        // Agents and custom files should NOT be in snapshot
        assert!(!section.contains("Agent Experiences"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_snapshot_only_user_and_knowledge() {
        let (store, _dir) = create_test_store();

        store.write_file("user", "test user").await.unwrap();
        store.write_file("knowledge", "test knowledge").await.unwrap();

        // Create agent files — should NOT appear in snapshot
        let agents_dir = store.base_path().join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::write(agents_dir.join("agent1.md"), "Agent summary").unwrap();

        // Create custom files — should NOT appear in snapshot
        store.write_custom_file("test-file", "Custom content").unwrap();

        let snapshot = MemorySnapshot::load(&store);
        let content = &snapshot.content;

        assert!(content.contains("test user"));
        assert!(content.contains("test knowledge"));
        assert!(!content.contains("Agent summary"));
        assert!(!content.contains("Custom content"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_priority_truncation_preserves_user_at_all_costs() {
        let (store, _dir) = create_test_store();

        let user = "u".repeat(2000);
        let knowledge = "k".repeat(3000);

        store.write_file("user", &user).await.unwrap();
        store.write_file("knowledge", &knowledge).await.unwrap();

        let snapshot = MemorySnapshot::load(&store);

        // Should be within budget
        assert!(snapshot.content.chars().count() <= CHAR_BUDGET);

        // User should be preserved (first 2000+ chars)
        assert!(snapshot.content.contains(&"u".repeat(100)));
    }

    #[test]
    fn test_truncate_chars() {
        let long = "a".repeat(1000);
        let truncated = truncate_chars(&long, 100);
        assert!(truncated.chars().count() <= 100);
    }

    #[test]
    fn test_truncate_chars_at_sentence() {
        let text = "First sentence. Second sentence. Third sentence.";
        let truncated = truncate_chars(text, 30);
        assert!(truncated.contains("First sentence."));
        assert!(!truncated.contains("Second sentence."));
    }

    #[test]
    fn test_parse_sections() {
        let content = "## User\nuser content\n\n## Knowledge\nknowledge content";
        let sections = parse_sections(content);

        assert_eq!(sections.get("User").unwrap(), "user content");
        assert_eq!(sections.get("Knowledge").unwrap(), "knowledge content");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_snapshot_load_opt() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        let store = MarkdownMemoryStore::new(dir.path().to_path_buf());
        assert!(MemorySnapshot::load_opt(&store).is_none());

        let (store, _dir) = create_test_store();
        assert!(MemorySnapshot::load_opt(&store).is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_char_budget_truncation() {
        let (store, _dir) = create_test_store();

        let user = "u".repeat(2000);
        let knowledge = "k".repeat(3000);

        store.write_file("user", &user).await.unwrap();
        store.write_file("knowledge", &knowledge).await.unwrap();

        let snapshot = MemorySnapshot::load(&store);
        assert!(snapshot.content.chars().count() <= CHAR_BUDGET);
    }
}
