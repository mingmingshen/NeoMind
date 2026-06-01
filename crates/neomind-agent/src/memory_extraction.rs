//! Memory Extraction Module
//!
//! Provides functions to extract memories from chat conversations and agent executions
//! using LLM-based extraction and persist them to category-based markdown files.

use std::sync::Arc;
use tokio::sync::RwLock;

use neomind_core::llm::backend::{GenerationParams, LlmInput, LlmRuntime};
use neomind_storage::{MarkdownMemoryStore, SessionMessage};

use crate::error::Result;
use crate::memory::dedup::DedupProcessor;
use crate::memory::extractor::{AgentExtractor, ExtractResult, MemoryAction};

/// Memory extractor that uses LLM to extract and persist memories
pub struct MemoryExtractor {
    store: Arc<RwLock<MarkdownMemoryStore>>,
    llm: Arc<dyn LlmRuntime>,
}

impl MemoryExtractor {
    /// Create a new memory extractor
    pub fn new(store: Arc<RwLock<MarkdownMemoryStore>>, llm: Arc<dyn LlmRuntime>) -> Self {
        Self { store, llm }
    }

    /// Clone the LLM runtime reference
    pub fn llm_clone(&self) -> Arc<dyn LlmRuntime> {
        self.llm.clone()
    }

    /// Clone the store reference
    pub fn store_clone(&self) -> Arc<RwLock<MarkdownMemoryStore>> {
        self.store.clone()
    }

    /// Extract memories from chat messages
    ///
    /// Test-only. Production uses the memory tool for writes.
    /// The `/api/memory/extract` endpoint has been removed.
    ///
    /// This method:
    /// 1. Formats chat messages into a prompt
    /// 2. Calls LLM to extract memory candidates
    /// 3. Parses the response
    /// 4. Appends extracted memories to category files
    ///
    /// # Arguments
    /// * `messages` - Chat messages to extract from
    ///
    /// # Returns
    /// * Number of memories extracted
    pub async fn extract_from_chat(&self, messages: &[SessionMessage]) -> Result<usize> {
        if messages.is_empty() {
            tracing::debug!("Skipping extraction: no messages to extract from");
            return Ok(0);
        }

        // Read existing memories for context (to enable deduplication)
        let existing_memories = self.gather_existing_memories().await;

        // Build prompt with existing memories for deduplication
        let prompt = AgentExtractor::build_prompt(
            "chat",
            None,
            &self.format_chat_messages(messages),
            "",
            &existing_memories,
        );

        // Call LLM
        let response = self.call_llm(&prompt).await?;

        // Parse response
        let extract_result = AgentExtractor::parse_response(&response)
            .map_err(|e| crate::error::NeoMindError::Memory(format!("Parse error: {}", e)))?;

        // Filter and write memories (handles merge/append actions)
        let filtered = Self::filter_chat_candidates(extract_result);
        let count = self.persist_memories(filtered).await?;

        tracing::info!(
            extracted_count = count,
            message_count = messages.len(),
            "Chat memory extraction completed"
        );

        Ok(count)
    }

    /// Extract memories from agent execution.
    ///
    /// NOTE: This method is currently only used in integration tests.
    /// Production agent memory updates go through the executor's
    /// `update_memory()` path instead.
    ///
    /// # Arguments
    /// * `agent_name` - Name of the agent
    /// * `user_prompt` - Original user request (if any)
    /// * `reasoning_steps` - Agent reasoning process
    /// * `conclusion` - Final result/conclusion
    ///
    /// # Returns
    /// * Number of memories extracted
    pub async fn extract_from_agent(
        &self,
        agent_name: &str,
        user_prompt: Option<&str>,
        reasoning_steps: &str,
        conclusion: &str,
    ) -> Result<usize> {
        // Read existing memories for context (to enable deduplication)
        let existing_memories = self.gather_existing_memories().await;

        // Build prompt with existing memories
        let prompt = AgentExtractor::build_prompt(
            agent_name,
            user_prompt,
            reasoning_steps,
            conclusion,
            &existing_memories,
        );

        // Call LLM
        let response = self.call_llm(&prompt).await?;

        // Parse response
        let extract_result = AgentExtractor::parse_response(&response)
            .map_err(|e| crate::error::NeoMindError::Memory(format!("Parse error: {}", e)))?;

        // Filter and write memories (handles merge/append actions)
        let count = self.persist_memories(extract_result).await?;

        tracing::info!(
            extracted_count = count,
            agent_name = %agent_name,
            "Agent memory extraction completed"
        );

        Ok(count)
    }

    // === Private helper methods ===

    /// Filter out categories that shouldn't come from chat extraction.
    ///
    /// Chat conversations should not produce `system_evolution` entries —
    /// only Agent execution can generate those. If the LLM mistakenly
    /// returns `system_evolution` from chat, reclassify to `task_patterns`
    /// since it likely represents a discovered pattern.
    fn filter_chat_candidates(result: ExtractResult) -> ExtractResult {
        let memories = result
            .memories
            .into_iter()
            .map(|mut m| {
                if m.category.to_lowercase() == "system_evolution" {
                    tracing::debug!(
                        content = %m.content,
                        "Reclassifying system_evolution from chat to task_patterns"
                    );
                    m.category = "task_patterns".to_string();
                }
                m
            })
            .collect();

        ExtractResult { memories }
    }

    /// Gather existing memories from all categories for deduplication context
    async fn gather_existing_memories(&self) -> String {
        let store = self.store.read().await;
        let mut all_memories = String::new();

        if let Ok(content) = store.read_file("user").await {
            if !content.trim().is_empty() {
                all_memories.push_str(&format!("\n### User\n{}\n", content));
            }
        }
        if let Ok(content) = store.read_file("knowledge").await {
            if !content.trim().is_empty() {
                all_memories.push_str(&format!("\n### Knowledge\n{}\n", content));
            }
        }
        all_memories
    }

    /// Format chat messages for extraction prompt
    fn format_chat_messages(&self, messages: &[SessionMessage]) -> String {
        let limited: Vec<_> = messages.iter().rev().take(50).rev().collect();

        limited
            .iter()
            .map(|m| {
                let role = match m.role.as_str() {
                    "user" => "User",
                    "assistant" => "Assistant",
                    "system" => "System",
                    "tool" => "Tool",
                    _ => &m.role,
                };

                // Include thinking if present
                let content = if let Some(ref thinking) = m.thinking {
                    format!("[Thinking: {}]\n{}", thinking, m.content)
                } else {
                    m.content.clone()
                };

                format!("**{}**: {}", role, content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Call LLM with prompt and return response
    async fn call_llm(&self, prompt: &str) -> Result<String> {
        tracing::info!(
            prompt_length = prompt.len(),
            model = %self.llm.model_name(),
            "Calling LLM for memory extraction"
        );

        // Log prompt preview (first 500 chars)
        tracing::debug!(
            prompt_preview = %prompt.chars().take(500).collect::<String>(),
            "Memory extraction prompt"
        );

        let input = LlmInput::new(prompt).with_params(GenerationParams {
            temperature: Some(0.3), // Lower temperature for more consistent extraction
            max_tokens: Some(1024), // Limit response size
            thinking_enabled: Some(false), // Disable thinking to avoid wasting tokens on reasoning
            ..Default::default()
        });

        let output = self.llm.generate(input).await.map_err(|e| {
            tracing::error!(error = %e, "LLM generation failed");
            crate::error::NeoMindError::Llm(e.to_string())
        })?;

        // Log the LLM response
        tracing::info!(
            response_length = output.text.len(),
            finish_reason = ?output.finish_reason,
            "LLM response received"
        );

        // Log response preview (first 500 chars)
        tracing::debug!(
            response_preview = %output.text.chars().take(500).collect::<String>(),
            "Memory extraction LLM response"
        );

        Ok(output.text)
    }

    /// Persist extracted memories to category files
    ///
    /// Handles both Append and Merge actions:
    /// - Append: Add as new entry
    /// - Merge: Find matching entries and replace with merged content
    async fn persist_memories(&self, result: ExtractResult) -> Result<usize> {
        let mut count = 0;
        let store = self.store.read().await;
        let dedup = DedupProcessor::with_defaults();

        for candidate in result.memories {
            // Security scan - block prompt injection and data exfiltration
            match crate::memory::MemorySecurityScanner::scan(&candidate.content) {
                crate::memory::SecurityScanResult::Clean => {}
                crate::memory::SecurityScanResult::Blocked { reason } => {
                    tracing::warn!(
                        content = %&candidate.content[..candidate.content.floor_char_boundary(candidate.content.len().min(100))],
                        reason = %reason,
                        "Memory blocked by security scanner"
                    );
                    continue;
                }
            }

            // Filter by minimum importance
            if candidate.importance < 30 {
                tracing::debug!(
                    content = %candidate.content,
                    importance = candidate.importance,
                    "Skipping memory: below importance threshold"
                );
                continue;
            }

            // Truncate overly long entries (max 200 chars)
            if candidate.content.len() > 200 {
                tracing::warn!(
                    content_len = candidate.content.len(),
                    "Memory entry too long, truncating to 200 chars"
                );
                // Intentionally don't mutate — truncate when formatting
            }

            // Map category to target file
            let target = match candidate.category.as_str() {
                "user_profile" => "user",
                _ => "knowledge",
            };

            // Read existing content
            let mut content = store
                .read_file(target)
                .await
                .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

            // Handle action
            let should_add = match &candidate.action {
                MemoryAction::Append => {
                    // Use n-gram Jaccard similarity for dedup
                    let existing_entries: Vec<String> = content
                        .lines()
                        .filter(|l| l.trim().starts_with("- ["))
                        .filter_map(Self::extract_entry_content)
                        .collect();
                    if let Some((idx, sim)) = dedup.find_similar(&candidate.content, &existing_entries) {
                        tracing::debug!(
                            content = %candidate.content,
                            target = %target,
                            similar_to = %existing_entries.get(idx).unwrap_or(&"".to_string()),
                            similarity = sim,
                            "Skipping similar memory (append)"
                        );
                        false
                    } else {
                        true
                    }
                }
                MemoryAction::Merge { targets } => {
                    // For merge, find and replace matching lines
                    let merged = self.merge_with_targets(
                        &mut content,
                        &candidate.content,
                        targets,
                        candidate.importance,
                    );
                    if merged {
                        // Write back the modified content
                        store
                            .write_file(target, &content)
                            .await
                            .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

                        count += 1;
                        tracing::debug!(
                            content = %candidate.content,
                            target = %target,
                            targets = ?targets,
                            "Merged memory entry"
                        );
                    }
                    false // Don't add as new entry
                }
            };

            if should_add {
                // Format the memory entry
                let entry = self.format_memory_entry(&candidate.content, candidate.importance);

                // Append to content
                if !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(&entry);

                // Write back
                store
                    .write_file(target, &content)
                    .await
                    .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

                count += 1;
                tracing::debug!(
                    content = %candidate.content,
                    target = %target,
                    importance = candidate.importance,
                    "Appended memory entry"
                );
            }
        }

        Ok(count)
    }

    /// Merge new content with existing entries using similarity matching
    ///
    /// Falls back to keyword target matching if no similar entries found via n-gram.
    /// Returns true if merge was performed, false otherwise
    fn merge_with_targets(
        &self,
        content: &mut String,
        new_content: &str,
        targets: &[String],
        importance: u8,
    ) -> bool {
        let lines: Vec<&str> = content.lines().collect();
        let mut merged = false;
        let mut new_lines = Vec::new();
        let timestamp = chrono::Utc::now().format("%Y-%m-%d");

        for line in lines {
            if merged {
                new_lines.push(line.to_string());
                continue;
            }

            let line_trimmed = line.trim();
            if !line_trimmed.starts_with("- [") {
                new_lines.push(line.to_string());
                continue;
            }

            // Strategy 1: Keyword target matching (primary)
            let line_lower = line_trimmed.to_lowercase();
            let matches_target = targets
                .iter()
                .any(|t| line_lower.contains(&t.to_lowercase()));

            if matches_target {
                new_lines.push(format!(
                    "- [{}] {} [importance: {}]",
                    timestamp, new_content, importance
                ));
                merged = true;
                tracing::debug!(
                    old_line = %line,
                    new_content = %new_content,
                    targets = ?targets,
                    "Merged memory via keyword target"
                );
                continue;
            }

            new_lines.push(line.to_string());
        }

        if merged {
            *content = new_lines.join("\n");
            if !content.ends_with('\n') {
                content.push('\n');
            }
        }

        merged
    }

    /// Extract the text content from a markdown memory entry line
    /// Format: "- [2026-04-08] Some content here [importance: 80]"
    fn extract_entry_content(line: &str) -> Option<String> {
        let line = line.trim();
        // Skip the "- [date] " prefix
        let after_date = line.strip_prefix("- [")?;
        let close_bracket = after_date.find(']')?;
        let content_with_importance = &after_date[close_bracket + 1..];
        // Remove the " [importance: NN]" suffix
        let content = if let Some(idx) = content_with_importance.rfind(" [importance:") {
            &content_with_importance[..idx]
        } else {
            content_with_importance
        };
        Some(content.trim().to_string())
    }

    /// Format a memory entry for markdown.
    /// Enforces a max content length of 200 chars.
    fn format_memory_entry(&self, content: &str, importance: u8) -> String {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d");
        let truncated = if content.len() > 200 {
            // Find a word boundary near 200 chars
            let boundary = content
                .char_indices()
                .take_while(|(i, _)| *i < 197)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(200);
            &content[..boundary]
        } else {
            content
        };
        format!(
            "- [{}] {} [importance: {}]\n",
            timestamp, truncated, importance
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::Stream;
    use neomind_core::llm::backend::{FinishReason, LlmOutput, StreamChunk};
    use tempfile::TempDir;

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
            _input: LlmInput,
        ) -> std::result::Result<LlmOutput, neomind_core::llm::backend::LlmError> {
            Ok(LlmOutput {
                text: r#"{"memories":[{"content":"User prefers Chinese language","category":"user_profile","importance":80}]}"#.to_string(),
                finish_reason: FinishReason::Stop,
                usage: None,
                thinking: None,
            })
        }

        async fn generate_stream(
            &self,
            _input: LlmInput,
        ) -> std::result::Result<
            std::pin::Pin<Box<dyn Stream<Item = StreamChunk> + Send>>,
            neomind_core::llm::backend::LlmError,
        > {
            Err(neomind_core::llm::backend::LlmError::InvalidInput(
                "streaming not supported by mock".into(),
            ))
        }

        fn max_context_length(&self) -> usize {
            4096
        }
    }

    #[tokio::test]
    async fn test_format_chat_messages() {
        let temp_dir = TempDir::new().unwrap();
        let store = Arc::new(RwLock::new(MarkdownMemoryStore::new(temp_dir.path())));
        let llm = Arc::new(MockLlm);

        let extractor = MemoryExtractor::new(store, llm);

        let messages = vec![
            SessionMessage::user("Hello"),
            SessionMessage::assistant("Hi there!"),
            SessionMessage::user("I prefer Chinese"),
        ];

        let formatted = extractor.format_chat_messages(&messages);
        assert!(formatted.contains("**User**: Hello"));
        assert!(formatted.contains("**Assistant**: Hi there"));
        assert!(formatted.contains("**User**: I prefer Chinese"));
    }

    #[test]
    fn test_format_memory_entry() {
        let temp_dir = TempDir::new().unwrap();
        let store = Arc::new(RwLock::new(MarkdownMemoryStore::new(temp_dir.path())));
        let llm = Arc::new(MockLlm);

        let extractor = MemoryExtractor::new(store, llm);
        let entry = extractor.format_memory_entry("Test memory", 75);

        assert!(entry.starts_with("- ["));
        assert!(entry.contains("Test memory"));
        assert!(entry.contains("[importance: 75]"));
    }

    #[tokio::test]
    async fn test_add_memory() {
        let temp_dir = TempDir::new().unwrap();
        let store = MarkdownMemoryStore::new(temp_dir.path());
        store.init().unwrap();
        let store = Arc::new(RwLock::new(store));

        // Write user file directly
        let timestamp = chrono::Utc::now().format("%Y-%m-%d");
        let entry = format!("- [{}] User likes pizza [importance: 60]\n", timestamp);
        let mut content = String::from("# User Profile\n\n");
        content.push_str(&entry);

        store
            .write()
            .await
            .write_file("user", &content)
            .await
            .unwrap();

        let read_content = store.read().await.read_file("user").await.unwrap();
        assert!(read_content.contains("User likes pizza"));
        assert!(read_content.contains("[importance: 60]"));
    }
}
