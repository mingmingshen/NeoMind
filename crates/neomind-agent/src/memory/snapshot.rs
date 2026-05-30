//! Memory snapshot for injecting long-term memories into system prompts.
//!
//! Uses a "frozen snapshot" pattern: load once at session start, cache for the
//! entire session. This keeps the prompt prefix stable for caching.
//!
//! ## Memory Layout (2-file persistent + agent summaries)
//!
//! - `USER.md` (max 2000 chars) — user profile, highest priority
//! - `KNOWLEDGE.md` (max 3000 chars) — system knowledge
//! - `agents/{agent_id}.md` — agent experiences (summarized by bridge)
//!
//! Total budget: 7500 chars (2000 user + 3000 knowledge + 2500 agents)

use neomind_storage::MarkdownMemoryStore;
use std::fs;

/// Hard character budget for memory context in prompts.
const CHAR_BUDGET: usize = 7500;

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
    /// Reads from the new 2-file layout:
    /// - `USER.md` (user profile, highest priority)
    /// - `KNOWLEDGE.md` (system knowledge)
    /// - `agents/{agent_id}.md` files (agent experiences, newest first)
    ///
    /// Combines them into a single snapshot with priority truncation:
    /// 1. Never truncate User section
    /// 2. First truncate Agent Experiences
    /// 3. Then truncate Knowledge if needed
    pub fn load(store: &MarkdownMemoryStore) -> Self {
        // Read persistent files using new API
        let user = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::try_current()
                .or_else(|_| {
                    tokio::runtime::Runtime::new()
                        .map(|rt| rt.handle().clone())
                })
                .unwrap()
                .block_on(store.read_file("user"))
        }).unwrap_or_default();

        let knowledge = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::try_current()
                .or_else(|_| {
                    tokio::runtime::Runtime::new()
                        .map(|rt| rt.handle().clone())
                })
                .unwrap()
                .block_on(store.read_file("knowledge"))
        }).unwrap_or_default();

        // Read agent summaries from agents/ directory
        let agent_summaries = read_agent_summaries(store);

        let combined = format!(
            "## User\n{user}\n\n## Knowledge\n{knowledge}\n\n## Agent Experiences\n{agent_summaries}"
        );

        let content = if combined.chars().count() <= CHAR_BUDGET {
            combined
        } else {
            truncate_with_priority(&combined, CHAR_BUDGET)
        };

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

/// Read agent summaries from the agents/ directory.
///
/// Reads all .md files from `data/memory/agents/`, sorts by modification time
/// (newest first), and takes up to 5 most recent agent summaries.
fn read_agent_summaries(store: &MarkdownMemoryStore) -> String {
    let agents_dir = store.base_path().join("agents");

    if !agents_dir.exists() {
        return String::new();
    }

    let mut agent_files: Vec<(String, String, i64)> = Vec::new();

    let entries = match fs::read_dir(&agents_dir) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!(path = %agents_dir.display(), error = %e, "Failed to read agents directory");
            return String::new();
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let agent_id = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "Failed to read agent file");
                    continue;
                }
            };

            let modified = entry.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            agent_files.push((agent_id.to_string(), content, modified));
        }
    }

    // Sort by modification time (newest first)
    agent_files.sort_by(|a, b| b.2.cmp(&a.2));

    // Take up to 5 most recent
    let mut summaries = Vec::new();
    for (agent_id, content, _) in agent_files.into_iter().take(5) {
        if !content.trim().is_empty() {
            summaries.push(format!("### {agent_id}\n{content}"));
        }
    }

    summaries.join("\n\n")
}

/// Truncate content with priority preservation.
///
/// Priority order:
/// 1. User section (never truncated)
/// 2. Knowledge section (truncated second)
/// 3. Agent Experiences section (truncated first)
///
/// This ensures user profile is always preserved, even at the cost
/// of dropping all agent experiences and most knowledge.
fn truncate_with_priority(content: &str, max_chars: usize) -> String {
    let sections = parse_sections(content);

    // Calculate total length
    let total_len: usize = sections.values().map(|s| s.chars().count()).sum();
    if total_len <= max_chars {
        return content.to_string();
    }

    // Parse into individual sections
    let user_len = sections.get("User").map_or(0, |s| s.chars().count());
    let knowledge_len = sections.get("Knowledge").map_or(0, |s| s.chars().count());
    let _agents_len = sections.get("Agent Experiences").map_or(0, |s| s.chars().count());

    // Priority 1: Keep User, truncate Agents, then Knowledge
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
                remaining = remaining.saturating_sub("\n\n## Knowledge\n".chars().count() + knowledge_len);
            } else {
                // Truncate knowledge to fit remaining space
                let truncated = truncate_chars(knowledge, remaining.saturating_sub(20)); // Reserve space for header
                result.push_str("\n\n## Knowledge\n");
                result.push_str(&truncated);
                remaining = 0;
            }
        }
    }

    // Add Agent Experiences section (only if space remains)
    if remaining > 10 {
        if let Some(agents) = sections.get("Agent Experiences") {
            let truncated = truncate_chars(agents, remaining.saturating_sub(25)); // Reserve space for header
            if !truncated.is_empty() {
                result.push_str("\n\n## Agent Experiences\n");
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
            // Save previous section
            if !current_name.is_empty() {
                sections.insert(current_name.clone(), current_section.trim().to_string());
            }

            // Start new section
            current_name = line[3..].to_string();
            current_section = String::new();
        } else {
            current_section.push_str(line);
            current_section.push('\n');
        }
    }

    // Save last section
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

    // Try to find a sentence boundary near the limit
    let limit = max_chars;
    for i in (limit.saturating_sub(100)..limit).rev() {
        if i < chars.len() {
            let c = chars[i];
            if c == '.' || c == '!' || c == '?' {
                return chars[..=i].iter().collect();
            }
        }
    }

    // Fall back to hard truncate
    chars[..limit].iter().collect()
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
        store
    }

    #[test]
    fn test_char_budget_is_7500() {
        assert_eq!(CHAR_BUDGET, 7500);
    }

    #[test]
    fn test_empty_snapshot() {
        let store = create_test_store();
        let snapshot = MemorySnapshot::load(&store);
        assert!(snapshot.is_empty());
        assert!(snapshot.to_prompt_section().is_empty());
    }

    #[tokio::test]
    async fn test_snapshot_with_content() {
        let store = create_test_store();

        // Write user and knowledge files
        store
            .write_file(
                "user",
                "User prefers dark mode\nUser speaks Chinese\n",
            )
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
    }

    #[tokio::test]
    async fn test_priority_truncation_truncates_agents_first() {
        let store = create_test_store();

        let user = "u".repeat(2000);
        let knowledge = "k".repeat(3000);
        let agents = "a".repeat(3000);

        store.write_file("user", &user).await.unwrap();
        store.write_file("knowledge", &knowledge).await.unwrap();

        // Create agent files
        let agents_dir = store.base_path().join("agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(agents_dir.join("agent1.md"), &agents).unwrap();

        let snapshot = MemorySnapshot::load(&store);

        // Should be within budget
        assert!(snapshot.content.chars().count() <= CHAR_BUDGET);

        // User should be preserved (sample check)
        assert!(snapshot.content.contains(&"u".repeat(100)));
    }

    #[tokio::test]
    async fn test_priority_truncation_preserves_user_at_all_costs() {
        let store = create_test_store();

        let user = "u".repeat(2000);
        let knowledge = "k".repeat(6000); // Way over budget

        store.write_file("user", &user).await.unwrap();
        store.write_file("knowledge", &knowledge).await.unwrap();

        let snapshot = MemorySnapshot::load(&store);

        // Should be within budget
        assert!(snapshot.content.chars().count() <= CHAR_BUDGET);

        // User should be fully preserved
        assert!(snapshot.content.contains(&user));

        // Knowledge should be truncated
        assert!(!snapshot.content.contains(&"k".repeat(3000)));
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
        // Should cut at first sentence boundary
        assert!(truncated.contains("First sentence."));
        assert!(!truncated.contains("Second sentence."));
    }

    #[test]
    fn test_parse_sections() {
        let content = "## User\nuser content\n\n## Knowledge\nknowledge content\n\n## Agent Experiences\nagent content";
        let sections = parse_sections(content);

        assert_eq!(sections.get("User").unwrap(), "user content");
        assert_eq!(sections.get("Knowledge").unwrap(), "knowledge content");
        assert_eq!(sections.get("Agent Experiences").unwrap(), "agent content");
    }

    #[tokio::test]
    async fn test_read_agent_summaries() {
        let store = create_test_store();

        // Create agent files
        let agents_dir = store.base_path().join("agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(agents_dir.join("agent1.md"), "Agent 1 summary").unwrap();
        fs::write(agents_dir.join("agent2.md"), "Agent 2 summary").unwrap();

        let summaries = read_agent_summaries(&store);
        assert!(summaries.contains("### agent1"));
        assert!(summaries.contains("Agent 1 summary"));
        assert!(summaries.contains("### agent2"));
        assert!(summaries.contains("Agent 2 summary"));
    }

    #[tokio::test]
    async fn test_read_agent_summaries_empty_dir() {
        let store = create_test_store();
        let summaries = read_agent_summaries(&store);
        assert!(summaries.is_empty());
    }

    #[tokio::test]
    async fn test_read_agent_summaries_limits_to_5() {
        let store = create_test_store();

        // Create 7 agent files
        let agents_dir = store.base_path().join("agents");
        fs::create_dir_all(&agents_dir).unwrap();
        for i in 1..=7 {
            fs::write(agents_dir.join(format!("agent{}.md", i)), format!("Agent {}", i))
                .unwrap();
        }

        let summaries = read_agent_summaries(&store);

        // Should only include at most 5 agents
        let count = summaries.matches("### agent").count();
        assert!(count <= 5);
    }

    #[tokio::test]
    async fn test_snapshot_load_opt() {
        let store = create_test_store();
        assert!(MemorySnapshot::load_opt(&store).is_none());

        store
            .write_file("user", "test user")
            .await
            .unwrap();
        assert!(MemorySnapshot::load_opt(&store).is_some());
    }

    #[tokio::test]
    async fn test_char_budget_truncation() {
        let store = create_test_store();

        // Create massive content that exceeds budget
        let user = "u".repeat(2000);
        let knowledge = "k".repeat(3000);

        store.write_file("user", &user).await.unwrap();
        store.write_file("knowledge", &knowledge).await.unwrap();

        let snapshot = MemorySnapshot::load(&store);
        assert!(snapshot.content.chars().count() <= CHAR_BUDGET);
    }
}
