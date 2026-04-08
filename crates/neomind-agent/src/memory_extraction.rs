//! Memory Extraction Module
//!
//! Provides functions to extract memories from chat conversations and agent executions
//! using LLM-based extraction and persist them to category-based markdown files.

use std::sync::Arc;
use tokio::sync::RwLock;

use neomind_core::llm::backend::{GenerationParams, LlmInput, LlmRuntime};
use neomind_storage::{MarkdownMemoryStore, MemoryCategory, SessionMessage};

use crate::error::Result;
use crate::memory::compressor::MemoryCompressor;
use crate::memory::dedup::DedupProcessor;
use crate::memory::extractor::{parse_category, AgentExtractor, ChatExtractor, ExtractResult, MemoryAction};

/// Memory extraction configuration
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Minimum messages required to trigger extraction
    pub min_messages: usize,
    /// Maximum messages to include in extraction prompt
    pub max_messages: usize,
    /// Minimum importance threshold for extracted memories
    pub min_importance: u8,
    /// Whether to deduplicate extracted memories
    pub dedup_enabled: bool,
    /// Similarity threshold for dedup (0.0-1.0)
    pub similarity_threshold: f32,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            min_messages: 3,
            max_messages: 50,
            min_importance: 30,
            dedup_enabled: true,
            similarity_threshold: 0.85,
        }
    }
}

/// Memory extractor that uses LLM to extract and persist memories
pub struct MemoryExtractor {
    store: Arc<RwLock<MarkdownMemoryStore>>,
    llm: Arc<dyn LlmRuntime>,
    config: ExtractionConfig,
    dedup: DedupProcessor,
}

impl MemoryExtractor {
    /// Create a new memory extractor
    pub fn new(store: Arc<RwLock<MarkdownMemoryStore>>, llm: Arc<dyn LlmRuntime>) -> Self {
        let dedup_threshold = 0.85;
        Self {
            store,
            llm,
            config: ExtractionConfig::default(),
            dedup: DedupProcessor::new(dedup_threshold),
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        store: Arc<RwLock<MarkdownMemoryStore>>,
        llm: Arc<dyn LlmRuntime>,
        config: ExtractionConfig,
    ) -> Self {
        let dedup = DedupProcessor::new(config.similarity_threshold);
        Self { store, llm, config, dedup }
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
        if messages.len() < self.config.min_messages {
            tracing::debug!(
                message_count = messages.len(),
                min_required = self.config.min_messages,
                "Skipping extraction: not enough messages"
            );
            return Ok(0);
        }

        // Format messages for extraction
        let formatted = self.format_chat_messages(messages);

        // Read existing memories for context (to enable deduplication)
        let existing_memories = self.gather_existing_memories().await;

        // Build prompt with existing memories for deduplication
        let prompt = ChatExtractor::build_prompt(&formatted, &existing_memories);

        // Call LLM
        let response = self.call_llm(&prompt).await?;

        // Parse response
        let extract_result = ChatExtractor::parse_response(&response)
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

    /// Extract memories from agent execution
    ///
    /// This method:
    /// 1. Formats agent execution record into a prompt
    /// 2. Calls LLM to extract memory candidates
    /// 3. Parses the response
    /// 4. Appends extracted memories to category files
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

    /// Extract and compress memories from chat (full pipeline)
    ///
    /// This performs extraction followed by compression if needed
    pub async fn extract_and_compress_chat(
        &self,
        messages: &[SessionMessage],
        compressor: &MemoryCompressor,
    ) -> Result<(usize, bool)> {
        // Extract
        let extracted = self.extract_from_chat(messages).await?;

        if extracted == 0 {
            return Ok((0, false));
        }

        // Check if compression is needed for each category
        let mut compressed = false;
        let store = self.store.read().await;

        for category in MemoryCategory::all() {
            let stats = store
                .category_stats(&category)
                .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

            let max_entries = compressor.max_entries(&category);

            if stats.entry_count > max_entries {
                tracing::info!(
                    category = ?category,
                    current = stats.entry_count,
                    max = max_entries,
                    "Category exceeds max entries, compression needed"
                );
                // Note: Actual compression would be done by MemoryScheduler
                compressed = true;
            }
        }

        Ok((extracted, compressed))
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
                if m.category == "system_evolution" {
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

        for category in MemoryCategory::all() {
            if let Ok(content) = store.read_category(&category) {
                if !content.trim().is_empty() {
                    all_memories.push_str(&format!("\n### {}\n", category.display_name()));
                    // Limit to last 20 entries per category to avoid context overflow
                    let lines: Vec<&str> = content.lines().rev().take(20).collect();
                    for line in lines.into_iter().rev() {
                        all_memories.push_str(line);
                        all_memories.push('\n');
                    }
                }
            }
        }

        all_memories
    }

    /// Format chat messages for extraction prompt
    fn format_chat_messages(&self, messages: &[SessionMessage]) -> String {
        let limited: Vec<_> = messages
            .iter()
            .rev()
            .take(self.config.max_messages)
            .rev()
            .collect();

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
            ..Default::default()
        });

        let output = self
            .llm
            .generate(input)
            .await
            .map_err(|e| {
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

        for candidate in result.memories {
            // Filter by minimum importance
            if candidate.importance < self.config.min_importance {
                tracing::debug!(
                    content = %candidate.content,
                    importance = candidate.importance,
                    min_required = self.config.min_importance,
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

            // Parse category
            let category = parse_category(&candidate.category);

            // Read existing content
            let mut content = store
                .read_category(&category)
                .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

            // Handle action
            let should_add = match &candidate.action {
                MemoryAction::Append => {
                    // For append, check duplicates if enabled
                    if self.config.dedup_enabled && self.is_duplicate(&content, &candidate.content) {
                        tracing::debug!(
                            content = %candidate.content,
                            category = ?category,
                            "Skipping duplicate memory (append)"
                        );
                        false
                    } else {
                        true
                    }
                }
                MemoryAction::Merge { targets } => {
                    // For merge, find and replace matching lines
                    let merged = self.merge_with_targets(&mut content, &candidate.content, targets, candidate.importance);
                    if merged {
                        // Write back the modified content
                        store
                            .write_category(&category, &content)
                            .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

                        count += 1;
                        tracing::debug!(
                            content = %candidate.content,
                            category = ?category,
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
                    .write_category(&category, &content)
                    .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

                count += 1;
                tracing::debug!(
                    content = %candidate.content,
                    category = ?category,
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

            // Strategy 1: N-gram similarity (primary)
            if let Some(entry_content) = Self::extract_entry_content(line_trimmed) {
                let sim = DedupProcessor::jaccard_similarity(&entry_content, new_content);
                if sim >= 0.6 {
                    new_lines.push(format!(
                        "- [{}] {} [importance: {}]",
                        timestamp, new_content, importance
                    ));
                    merged = true;
                    tracing::debug!(
                        old_line = %line,
                        new_content = %new_content,
                        similarity = sim,
                        "Merged memory via n-gram similarity"
                    );
                    continue;
                }
            }

            // Strategy 2: Keyword target matching (fallback)
            let line_lower = line_trimmed.to_lowercase();
            let matches_target = targets.iter().any(|t| {
                line_lower.contains(&t.to_lowercase())
            });

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

    /// Check if content already exists using n-gram Jaccard similarity
    fn is_duplicate(&self, existing_content: &str, new_content: &str) -> bool {
        if !self.config.dedup_enabled {
            return false;
        }

        // Extract content parts from existing entries (strip markdown formatting)
        let existing_entries: Vec<String> = existing_content
            .lines()
            .filter(|l| l.trim().starts_with("- ["))
            .map(|l| {
                // Extract just the content part between date and importance
                if let Some(content) = Self::extract_entry_content(l) {
                    content
                } else {
                    l.to_string()
                }
            })
            .collect();

        self.dedup.find_similar(new_content, &existing_entries).is_some()
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
            let boundary = content.char_indices()
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

/// Convenience functions for manual memory operations

/// Manually add a memory entry to a category
pub async fn add_memory(
    store: &Arc<RwLock<MarkdownMemoryStore>>,
    category: &MemoryCategory,
    content: &str,
    importance: u8,
) -> Result<()> {
    let store_guard = store.read().await;

    let mut existing = store_guard
        .read_category(category)
        .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%d");
    let entry = format!(
        "- [{}] {} [importance: {}]\n",
        timestamp, content, importance
    );

    if !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(&entry);

    store_guard
        .write_category(category, &existing)
        .map_err(|e| crate::error::NeoMindError::Memory(e.to_string()))?;

    Ok(())
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
            unimplemented!()
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

        add_memory(&store, &MemoryCategory::UserProfile, "User likes pizza", 60)
            .await
            .unwrap();

        let content = store
            .read()
            .await
            .read_category(&MemoryCategory::UserProfile)
            .unwrap();
        assert!(content.contains("User likes pizza"));
        assert!(content.contains("[importance: 60]"));
    }
}
