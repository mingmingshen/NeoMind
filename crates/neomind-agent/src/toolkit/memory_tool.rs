//! Memory management tool for persistent and session-scoped storage.

use async_trait::async_trait;
use neomind_storage::{KnowledgeFileRef, MarkdownMemoryStore};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::memory::dedup::DedupProcessor;

use super::error::{Result, ToolError};
use super::tool::{Tool, ToolCategory};
use super::ToolOutput;

/// Tool for managing persistent memory across sessions.
///
/// Supports:
/// - Persistent: USER.md (user profile), KNOWLEDGE.md (domain knowledge)
/// - Agent-scoped custom files: `custom:{name}` → `agents/{agent_id}/custom/{name}.md`
/// - Session: `sessions/{id}/notes.md` (multi-step task tracking, 7-day TTL)
pub struct MemoryTool {
    store: Arc<RwLock<MarkdownMemoryStore>>,
    session_id: Arc<RwLock<Option<String>>>,
    /// Agent ID for agent-scoped custom file isolation — set per-execution
    agent_id: Arc<RwLock<Option<String>>>,
    /// Knowledge file index — updated when LLM creates custom files
    knowledge_files: Arc<RwLock<Vec<KnowledgeFileRef>>>,
}

impl MemoryTool {
    /// Create a new memory tool (no agent context — global scope).
    pub fn new(store: Arc<RwLock<MarkdownMemoryStore>>) -> Self {
        Self {
            store,
            session_id: Arc::new(RwLock::new(None)),
            agent_id: Arc::new(RwLock::new(None)),
            knowledge_files: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new memory tool with a shared session ID handle.
    pub fn with_session_handle(
        store: Arc<RwLock<MarkdownMemoryStore>>,
        session_handle: Arc<RwLock<Option<String>>>,
    ) -> Self {
        Self {
            store,
            session_id: session_handle,
            agent_id: Arc::new(RwLock::new(None)),
            knowledge_files: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new memory tool with shared handles for all dynamic state.
    /// The agent_id and knowledge_files handles are shared with the executor
    /// so it can inject agent_id per-execution and sync knowledge_files back.
    pub fn with_shared_handles(
        store: Arc<RwLock<MarkdownMemoryStore>>,
        session_handle: Arc<RwLock<Option<String>>>,
        agent_id_handle: Arc<RwLock<Option<String>>>,
        knowledge_files_handle: Arc<RwLock<Vec<KnowledgeFileRef>>>,
    ) -> Self {
        Self {
            store,
            session_id: session_handle,
            agent_id: agent_id_handle,
            knowledge_files: knowledge_files_handle,
        }
    }

    /// Get a handle to set the session ID (call after registration).
    pub fn session_id_handle(&self) -> Arc<RwLock<Option<String>>> {
        self.session_id.clone()
    }

    /// Get a handle to set the agent ID per-execution.
    pub fn agent_id_handle(&self) -> Arc<RwLock<Option<String>>> {
        self.agent_id.clone()
    }

    /// Get a handle to read/modify the knowledge file index.
    /// The executor reads this after tool loop to sync back to AgentMemory.
    pub fn knowledge_files_handle(&self) -> Arc<RwLock<Vec<KnowledgeFileRef>>> {
        self.knowledge_files.clone()
    }

    /// Get preview text (first 50 chars).
    fn get_preview(content: &str) -> String {
        content.chars().take(50).collect::<String>()
    }

    /// Append content to existing content.
    fn append_content(existing: &str, new_content: &str) -> String {
        if existing.is_empty() {
            new_content.to_string()
        } else if existing.ends_with('\n') {
            format!("{}{}", existing, new_content)
        } else {
            format!("{}\n{}", existing, new_content)
        }
    }

    /// Find and replace text in content (first occurrence only).
    fn replace_in_content(content: &str, old_text: &str, new_text: &str) -> Result<String> {
        if !content.contains(old_text) {
            return Err(ToolError::InvalidArguments(format!(
                "Text '{}' not found in content",
                old_text
            )));
        }
        Ok(content.replacen(old_text, new_text, 1))
    }

    /// Find and remove text from content (first occurrence only).
    fn remove_from_content(content: &str, old_text: &str) -> Result<String> {
        if !content.contains(old_text) {
            return Err(ToolError::InvalidArguments(format!(
                "Text '{}' not found in content",
                old_text
            )));
        }
        Ok(content.replacen(old_text, "", 1))
    }

    /// Validate that session_id is set for session-scoped operations.
    async fn require_session_id(&self) -> Result<String> {
        self.session_id
            .read()
            .await
            .clone()
            .ok_or_else(|| {
                ToolError::Execution(
                    "Session ID required for session-scoped operations.".into(),
                )
            })
    }

    /// Parse a target string. Returns Some(name) for custom:{name}, None for built-in targets.
    fn parse_custom_target(target: &str) -> Option<&str> {
        target.strip_prefix("custom:")
    }

    /// Get the current agent_id.
    async fn get_agent_id(&self) -> Option<String> {
        self.agent_id.read().await.clone()
    }

    /// Read a custom file, respecting agent scope.
    fn read_custom(&self, store: &MarkdownMemoryStore, agent_id: Option<&str>, name: &str) -> Result<String> {
        if let Some(aid) = agent_id {
            store
                .read_agent_custom_file(aid, name)
                .map_err(|e| ToolError::Execution(e.to_string()))
        } else {
            store
                .read_custom_file(name)
                .map_err(|e| ToolError::Execution(e.to_string()))
        }
    }

    /// Write a custom file, respecting agent scope.
    fn write_custom(&self, store: &MarkdownMemoryStore, agent_id: Option<&str>, name: &str, content: &str) -> Result<()> {
        if let Some(aid) = agent_id {
            store
                .write_agent_custom_file(aid, name, content)
                .map_err(|e| ToolError::Execution(e.to_string()))
        } else {
            store
                .write_custom_file(name, content)
                .map_err(|e| ToolError::Execution(e.to_string()))
        }
    }

    /// Update the knowledge file index when creating a new custom file.
    async fn register_knowledge_file(&self, name: &str, description: &str) {
        let now = chrono::Utc::now().timestamp();
        let mut files = self.knowledge_files.write().await;
        if let Some(existing) = files.iter_mut().find(|f| f.name == name) {
            existing.updated_at = now;
        } else {
            files.push(KnowledgeFileRef {
                name: name.to_string(),
                description: truncate_to(description, 100),
                created_at: now,
                updated_at: now,
            });
        }
    }
}

fn truncate_to(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars.saturating_sub(3)).collect();
        truncated + "..."
    }
}

#[async_trait]
impl Tool for MemoryTool {
    fn name(&self) -> &str {
        "memory"
    }

    fn description(&self) -> &str {
        r##"Manage persistent memory across sessions. Use this to store and retrieve information that should persist between conversations.

Actions:
- add: Append content to a memory target
- replace: Find and replace text in a memory target
- remove: Find and remove text from a memory target
- read: Read the full content of a memory target
- list: Show overview of all memory targets (chars used, preview)
- create: Create a new custom memory file (target must be custom:{name})

Targets:
- user: Persistent user profile and preferences (USER.md)
- knowledge: System knowledge and domain info (KNOWLEDGE.md)
- session: Session-scoped notes for multi-step task tracking (cleared after 7 days)
- custom:{name}: Domain-specific custom file (e.g., custom:device-patterns, custom:thresholds). Created with action='create'.

Examples:
- Add user preference: action='add', target='user', content='Prefers dark mode'
- Replace in knowledge: action='replace', target='knowledge', old_text='old info', content='new info'
- Read session notes: action='read', target='session'
- Create custom file: action='create', target='custom:device-patterns', content='- temp normal: 22-28°C\n- alert threshold: 40°C'
- List all targets: action='list'"##
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["add", "replace", "remove", "read", "list", "create"],
                    "description": "The memory action to perform"
                },
                "target": {
                    "type": "string",
                    "description": "Which memory target to operate on: 'user', 'knowledge', 'session', or 'custom:{name}'"
                },
                "content": {
                    "type": "string",
                    "description": "Content to add or replace with"
                },
                "old_text": {
                    "type": "string",
                    "description": "Text to find (for replace/remove)"
                }
            },
            "required": ["action"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        let target = args["target"].as_str().unwrap_or("");

        match action {
            "list" => {
                // target is optional for list
            }
            _ => {
                if target.is_empty() {
                    return Err(ToolError::InvalidArguments(
                        "target is required for this action".into(),
                    ));
                }
            }
        }

        // Read agent_id once for this execution
        let agent_id = self.get_agent_id().await;

        match action {
            "create" => {
                let content = args["content"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("content is required for create".into())
                })?;

                let custom_name = Self::parse_custom_target(target).ok_or_else(|| {
                    ToolError::InvalidArguments(format!(
                        "Create action requires target in format 'custom:{{name}}'. Got: '{}'",
                        target
                    ))
                })?;

                let store = self.store.write().await;
                self.write_custom(&store, agent_id.as_deref(), custom_name, content)?;

                // Extract description from first line of content
                let description = content
                    .lines()
                    .next()
                    .unwrap_or("Knowledge file")
                    .trim_start_matches("# ")
                    .to_string();

                drop(store);
                self.register_knowledge_file(custom_name, &description)
                    .await;

                Ok(ToolOutput::success(serde_json::json!({
                    "message": format!("Created custom file '{}' ({} chars)", custom_name, content.chars().count())
                })))
            }
            "add" => {
                let content = args["content"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("content is required for add".into())
                })?;

                let store = self.store.write().await;
                let dedup = DedupProcessor::with_defaults();

                let result = if let Some(custom_name) = Self::parse_custom_target(target) {
                    let existing = self.read_custom(&store, agent_id.as_deref(), custom_name)?;
                    let new_content = Self::append_content(&existing, content);
                    self.write_custom(&store, agent_id.as_deref(), custom_name, &new_content)?;
                    format!(
                        "Added to custom:{} ({} chars)",
                        custom_name,
                        new_content.chars().count()
                    )
                } else {
                    match target {
                        "user" | "knowledge" => {
                            let existing = store.read_file(target).await?;
                            let existing_lines: Vec<String> = existing
                                .lines()
                                .filter(|l| l.trim().starts_with("- ["))
                                .filter_map(|l| {
                                    let trimmed = l.trim();
                                    let after_date = trimmed.strip_prefix("- [")?;
                                    let close_bracket = after_date.find(']')?;
                                    let content_part = &after_date[close_bracket + 1..];
                                    let cleaned = if let Some(idx) = content_part.rfind(" [importance:") {
                                        &content_part[..idx]
                                    } else {
                                        content_part
                                    };
                                    Some(cleaned.trim().to_string())
                                })
                                .collect();
                            if let Some((_, sim)) = dedup.find_similar(content, &existing_lines) {
                                return Ok(ToolOutput::success(serde_json::json!({
                                    "message": format!("Skipped: similar content already exists (similarity: {:.0}%)", sim * 100.0)
                                })));
                            }
                            let new_content = Self::append_content(&existing, content);
                            store.write_file(target, &new_content).await?;
                            format!("Added to {} ({} chars)", target, new_content.chars().count())
                        }
                        "session" => {
                            let session_id = self.require_session_id().await?;
                            let existing = store.read_session_file(&session_id, "notes").await?;
                            let new_content = Self::append_content(&existing, content);
                            store
                                .write_session_file(&session_id, "notes", &new_content)
                                .await?;
                            format!(
                                "Added to session notes ({} chars)",
                                new_content.chars().count()
                            )
                        }
                        _ => {
                            return Err(ToolError::InvalidArguments(format!(
                                "Invalid target '{}'. Must be one of: user, knowledge, session, custom:{{name}}",
                                target
                            )))
                        }
                    }
                };

                Ok(ToolOutput::success(serde_json::json!({
                    "message": result
                })))
            }
            "replace" => {
                let content = args["content"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("content is required for replace".into())
                })?;
                let old_text = args["old_text"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("old_text is required for replace".into())
                })?;

                let store = self.store.write().await;

                let result = if let Some(custom_name) = Self::parse_custom_target(target) {
                    let existing = self.read_custom(&store, agent_id.as_deref(), custom_name)?;
                    let new_content = Self::replace_in_content(&existing, old_text, content)?;
                    self.write_custom(&store, agent_id.as_deref(), custom_name, &new_content)?;
                    format!(
                        "Replaced in custom:{} ({} chars)",
                        custom_name,
                        new_content.chars().count()
                    )
                } else {
                    match target {
                        "user" | "knowledge" => {
                            let existing = store.read_file(target).await?;
                            let new_content = Self::replace_in_content(&existing, old_text, content)?;
                            store.write_file(target, &new_content).await?;
                            format!("Replaced in {} ({} chars)", target, new_content.chars().count())
                        }
                        "session" => {
                            let session_id = self.require_session_id().await?;
                            let existing = store.read_session_file(&session_id, "notes").await?;
                            let new_content = Self::replace_in_content(&existing, old_text, content)?;
                            store
                                .write_session_file(&session_id, "notes", &new_content)
                                .await?;
                            format!(
                                "Replaced in session notes ({} chars)",
                                new_content.chars().count()
                            )
                        }
                        _ => {
                            return Err(ToolError::InvalidArguments(format!(
                                "Invalid target '{}'. Must be one of: user, knowledge, session, custom:{{name}}",
                                target
                            )))
                        }
                    }
                };

                Ok(ToolOutput::success(serde_json::json!({
                    "message": result
                })))
            }
            "remove" => {
                let old_text = args["old_text"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("old_text is required for remove".into())
                })?;

                let store = self.store.write().await;

                let result = if let Some(custom_name) = Self::parse_custom_target(target) {
                    let existing = self.read_custom(&store, agent_id.as_deref(), custom_name)?;
                    let new_content = Self::remove_from_content(&existing, old_text)?;
                    self.write_custom(&store, agent_id.as_deref(), custom_name, &new_content)?;
                    format!(
                        "Removed from custom:{} ({} chars)",
                        custom_name,
                        new_content.chars().count()
                    )
                } else {
                    match target {
                        "user" | "knowledge" => {
                            let existing = store.read_file(target).await?;
                            let new_content = Self::remove_from_content(&existing, old_text)?;
                            store.write_file(target, &new_content).await?;
                            format!("Removed from {} ({} chars)", target, new_content.chars().count())
                        }
                        "session" => {
                            let session_id = self.require_session_id().await?;
                            let existing = store.read_session_file(&session_id, "notes").await?;
                            let new_content = Self::remove_from_content(&existing, old_text)?;
                            store
                                .write_session_file(&session_id, "notes", &new_content)
                                .await?;
                            format!(
                                "Removed from session notes ({} chars)",
                                new_content.chars().count()
                            )
                        }
                        _ => {
                            return Err(ToolError::InvalidArguments(format!(
                                "Invalid target '{}'. Must be one of: user, knowledge, session, custom:{{name}}",
                                target
                            )))
                        }
                    }
                };

                Ok(ToolOutput::success(serde_json::json!({
                    "message": result
                })))
            }
            "read" => {
                let store = self.store.read().await;

                let result = if let Some(custom_name) = Self::parse_custom_target(target) {
                    let content = self.read_custom(&store, agent_id.as_deref(), custom_name)?;
                    serde_json::json!({
                        "target": target,
                        "content": content,
                        "chars": content.chars().count()
                    })
                } else {
                    match target {
                        "user" | "knowledge" => {
                            let content = store.read_file(target).await?;
                            serde_json::json!({
                                "target": target,
                                "content": content,
                                "chars": content.chars().count()
                            })
                        }
                        "session" => {
                            let session_id = self.require_session_id().await?;
                            let content = store.read_session_file(&session_id, "notes").await?;
                            serde_json::json!({
                                "target": "session",
                                "content": content,
                                "chars": content.chars().count()
                            })
                        }
                        _ => {
                            return Err(ToolError::InvalidArguments(format!(
                                "Invalid target '{}'. Must be one of: user, knowledge, session, custom:{{name}}",
                                target
                            )))
                        }
                    }
                };

                Ok(ToolOutput::success(result))
            }
            "list" => {
                let store = self.store.read().await;

                let user_content = store.read_file("user").await?;
                let knowledge_content = store.read_file("knowledge").await?;

                let mut result = serde_json::json!({
                    "user": {
                        "chars": user_content.chars().count(),
                        "preview": Self::get_preview(&user_content)
                    },
                    "knowledge": {
                        "chars": knowledge_content.chars().count(),
                        "preview": Self::get_preview(&knowledge_content)
                    }
                });

                if let Some(session_id) = self.session_id.read().await.clone() {
                    let notes_content = store.read_session_file(&session_id, "notes").await?;
                    result["session"] = serde_json::json!({
                        "chars": notes_content.chars().count(),
                        "preview": Self::get_preview(&notes_content)
                    });
                }

                // Read custom files (agent-scoped or global)
                let custom_files = if let Some(ref aid) = agent_id {
                    store
                        .list_agent_custom_files(aid)
                        .map_err(|e| ToolError::Execution(e.to_string()))?
                } else {
                    store
                        .list_custom_files()
                        .map_err(|e| ToolError::Execution(e.to_string()))?
                };

                if !custom_files.is_empty() {
                    let mut customs = serde_json::Map::new();
                    for (name, chars) in &custom_files {
                        let content = self.read_custom(&store, agent_id.as_deref(), name)?;
                        customs.insert(
                            format!("custom:{}", name),
                            serde_json::json!({
                                "chars": chars,
                                "preview": Self::get_preview(&content)
                            }),
                        );
                    }
                    result["custom_files"] = Value::Object(customs);
                }

                Ok(ToolOutput::success(result))
            }
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action '{}'. Available actions: add, replace, remove, read, list, create",
                action
            ))),
        }
    }
}
