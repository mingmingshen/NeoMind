//! Memory management tool for persistent and session-scoped storage.

use async_trait::async_trait;
use neomind_storage::MarkdownMemoryStore;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::error::{Result, ToolError};
use super::tool::{Tool, ToolCategory};
use super::ToolOutput;

/// Tool for managing persistent memory across sessions.
///
/// Supports:
/// - Persistent: USER.md (user profile), KNOWLEDGE.md (domain knowledge)
/// - Custom files: `custom/{name}.md` (domain-specific knowledge, LLM auto-created)
/// - Session: `sessions/{id}/notes.md` (multi-step task tracking, 7-day TTL)
pub struct MemoryTool {
    store: Arc<RwLock<MarkdownMemoryStore>>,
    session_id: Arc<RwLock<Option<String>>>,
}

impl MemoryTool {
    /// Create a new memory tool.
    pub fn new(store: Arc<RwLock<MarkdownMemoryStore>>) -> Self {
        Self {
            store,
            session_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new memory tool with a shared session ID handle.
    /// This allows the API layer to set the session ID per-request.
    pub fn with_session_handle(
        store: Arc<RwLock<MarkdownMemoryStore>>,
        session_handle: Arc<RwLock<Option<String>>>,
    ) -> Self {
        Self {
            store,
            session_id: session_handle,
        }
    }

    /// Get a handle to set the session ID (call after registration).
    pub fn session_id_handle(&self) -> Arc<RwLock<Option<String>>> {
        self.session_id.clone()
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

    /// Parse a target string. Returns Ok(Some(name)) for custom:{name}, Ok(None) for built-in targets.
    fn parse_custom_target(target: &str) -> Option<&str> {
        target.strip_prefix("custom:")
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
- custom:{name}: Domain-specific custom file (e.g., custom:mqtt-setup, custom:device-map). Created with action='create'.

Examples:
- Add user preference: action='add', target='user', content='Prefers dark mode'
- Replace in knowledge: action='replace', target='knowledge', old_text='old info', content='new info'
- Read session notes: action='read', target='session'
- Create custom file: action='create', target='custom:mqtt-setup', content='Broker at 192.168.1.1:1883...'
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
                store
                    .write_custom_file(custom_name, content)
                    .map_err(|e| ToolError::Execution(e.to_string()))?;

                Ok(ToolOutput::success(serde_json::json!({
                    "message": format!("Created custom file '{}' ({} chars)", custom_name, content.chars().count())
                })))
            }
            "add" => {
                let content = args["content"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("content is required for add".into())
                })?;

                let store = self.store.write().await;

                let result = if let Some(custom_name) = Self::parse_custom_target(target) {
                    let existing = store
                        .read_custom_file(custom_name)
                        .map_err(|e| ToolError::Execution(e.to_string()))?;
                    let new_content = Self::append_content(&existing, content);
                    store
                        .write_custom_file(custom_name, &new_content)
                        .map_err(|e| ToolError::Execution(e.to_string()))?;
                    format!(
                        "Added to custom:{} ({} chars)",
                        custom_name,
                        new_content.chars().count()
                    )
                } else {
                    match target {
                        "user" | "knowledge" => {
                            let existing = store.read_file(target).await?;
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
                    let existing = store
                        .read_custom_file(custom_name)
                        .map_err(|e| ToolError::Execution(e.to_string()))?;
                    let new_content = Self::replace_in_content(&existing, old_text, content)?;
                    store
                        .write_custom_file(custom_name, &new_content)
                        .map_err(|e| ToolError::Execution(e.to_string()))?;
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
                    let existing = store
                        .read_custom_file(custom_name)
                        .map_err(|e| ToolError::Execution(e.to_string()))?;
                    let new_content = Self::remove_from_content(&existing, old_text)?;
                    store
                        .write_custom_file(custom_name, &new_content)
                        .map_err(|e| ToolError::Execution(e.to_string()))?;
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
                    let content = store
                        .read_custom_file(custom_name)
                        .map_err(|e| ToolError::Execution(e.to_string()))?;
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

                // Read persistent files
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

                // Read session notes if session_id is set
                if let Some(session_id) = self.session_id.read().await.clone() {
                    let notes_content = store.read_session_file(&session_id, "notes").await?;
                    result["session"] = serde_json::json!({
                        "chars": notes_content.chars().count(),
                        "preview": Self::get_preview(&notes_content)
                    });
                }

                // Read custom files
                let custom_files = store
                    .list_custom_files()
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                if !custom_files.is_empty() {
                    let mut customs = serde_json::Map::new();
                    for (name, chars) in &custom_files {
                        let content = store
                            .read_custom_file(name)
                            .map_err(|e| ToolError::Execution(e.to_string()))?;
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
