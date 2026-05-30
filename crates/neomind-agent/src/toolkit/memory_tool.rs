//! Memory management tool for persistent and session-scoped storage.

use async_trait::async_trait;
use serde_json::Value;
use neomind_storage::MarkdownMemoryStore;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::error::{Result, ToolError};
use super::tool::{Tool, ToolCategory};
use super::ToolOutput;

/// Tool for managing persistent memory across sessions.
///
/// Supports two types of memory:
/// - Persistent: USER.md (user profile), KNOWLEDGE.md (domain knowledge)
/// - Session-scoped: scratch, notes, todo (temporary files cleared after 7 days)
pub struct MemoryTool {
    store: Arc<RwLock<MarkdownMemoryStore>>,
    session_id: Option<String>,
}

impl MemoryTool {
    /// Create a new memory tool.
    pub fn new(store: Arc<RwLock<MarkdownMemoryStore>>) -> Self {
        Self {
            store,
            session_id: None,
        }
    }

    /// Set the session ID for session-scoped operations.
    pub fn set_session_id(&mut self, session_id: String) {
        self.session_id = Some(session_id);
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

    /// Find and replace text in content.
    fn replace_in_content(content: &str, old_text: &str, new_text: &str) -> Result<String> {
        if !content.contains(old_text) {
            return Err(ToolError::InvalidArguments(format!(
                "Text '{}' not found in content",
                old_text
            )));
        }
        Ok(content.replace(old_text, new_text))
    }

    /// Find and remove text from content.
    fn remove_from_content(content: &str, old_text: &str) -> Result<String> {
        if !content.contains(old_text) {
            return Err(ToolError::InvalidArguments(format!(
                "Text '{}' not found in content",
                old_text
            )));
        }
        Ok(content.replace(old_text, ""))
    }

    /// Validate that session_id is set for session-scoped operations.
    fn require_session_id(&self) -> Result<&str> {
        self.session_id.as_ref()
            .map(|s| s.as_str())
            .ok_or_else(|| ToolError::Execution("Session ID required for session-scoped operations. Use set_session_id() first.".into()))
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

Targets:
- user: Persistent user profile and preferences (USER.md)
- knowledge: System knowledge and domain info (KNOWLEDGE.md)
- scratch: Temporary session notes (cleared after 7 days)
- notes: Session-specific notes (cleared after 7 days)
- todo: Session task list (cleared after 7 days)

Examples:
- Add user preference: action='add', target='user', content='Prefers dark mode'
- Replace in knowledge: action='replace', target='knowledge', old_text='old info', content='new info'
- Read session notes: action='read', target='notes'
- List all targets: action='list'"##
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["add", "replace", "remove", "read", "list"],
                    "description": "The memory action to perform"
                },
                "target": {
                    "type": "string",
                    "enum": ["user", "knowledge", "scratch", "notes", "todo"],
                    "description": "Which memory target to operate on"
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
            "required": ["action", "target"]
        })
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("action is required".into()))?;

        let target = args["target"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("target is required".into()))?;

        match action {
            "add" => {
                let content = args["content"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArguments("content is required for add".into()))?;

                let store = self.store.read().await;

                let result = match target {
                    "user" | "knowledge" => {
                        let existing = store.read_file(target).await?;
                        let new_content = Self::append_content(&existing, content);
                        store.write_file(target, &new_content).await?;
                        format!("Added to {} ({} chars)", target, new_content.len())
                    }
                    "scratch" | "notes" | "todo" => {
                        let session_id = self.require_session_id()?;
                        let existing = store.read_session_file(session_id, target).await?;
                        let new_content = Self::append_content(&existing, content);
                        store.write_session_file(session_id, target, &new_content).await?;
                        format!("Added to session {} ({} chars)", target, new_content.len())
                    }
                    _ => return Err(ToolError::InvalidArguments(format!(
                        "Invalid target '{}'. Must be one of: user, knowledge, scratch, notes, todo",
                        target
                    )))
                };

                Ok(ToolOutput::success(serde_json::json!({
                    "message": result
                })))
            }
            "replace" => {
                let content = args["content"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArguments("content is required for replace".into()))?;
                let old_text = args["old_text"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArguments("old_text is required for replace".into()))?;

                let store = self.store.read().await;

                let result = match target {
                    "user" | "knowledge" => {
                        let existing = store.read_file(target).await?;
                        let new_content = Self::replace_in_content(&existing, old_text, content)?;
                        store.write_file(target, &new_content).await?;
                        format!("Replaced in {} ({} chars)", target, new_content.len())
                    }
                    "scratch" | "notes" | "todo" => {
                        let session_id = self.require_session_id()?;
                        let existing = store.read_session_file(session_id, target).await?;
                        let new_content = Self::replace_in_content(&existing, old_text, content)?;
                        store.write_session_file(session_id, target, &new_content).await?;
                        format!("Replaced in session {} ({} chars)", target, new_content.len())
                    }
                    _ => return Err(ToolError::InvalidArguments(format!(
                        "Invalid target '{}'. Must be one of: user, knowledge, scratch, notes, todo",
                        target
                    )))
                };

                Ok(ToolOutput::success(serde_json::json!({
                    "message": result
                })))
            }
            "remove" => {
                let old_text = args["old_text"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArguments("old_text is required for remove".into()))?;

                let store = self.store.read().await;

                let result = match target {
                    "user" | "knowledge" => {
                        let existing = store.read_file(target).await?;
                        let new_content = Self::remove_from_content(&existing, old_text)?;
                        store.write_file(target, &new_content).await?;
                        format!("Removed from {} ({} chars)", target, new_content.len())
                    }
                    "scratch" | "notes" | "todo" => {
                        let session_id = self.require_session_id()?;
                        let existing = store.read_session_file(session_id, target).await?;
                        let new_content = Self::remove_from_content(&existing, old_text)?;
                        store.write_session_file(session_id, target, &new_content).await?;
                        format!("Removed from session {} ({} chars)", target, new_content.len())
                    }
                    _ => return Err(ToolError::InvalidArguments(format!(
                        "Invalid target '{}'. Must be one of: user, knowledge, scratch, notes, todo",
                        target
                    )))
                };

                Ok(ToolOutput::success(serde_json::json!({
                    "message": result
                })))
            }
            "read" => {
                let store = self.store.read().await;

                let result = match target {
                    "user" | "knowledge" => {
                        let content = store.read_file(target).await?;
                        serde_json::json!({
                            "target": target,
                            "content": content,
                            "chars": content.len()
                        })
                    }
                    "scratch" | "notes" | "todo" => {
                        let session_id = self.require_session_id()?;
                        let content = store.read_session_file(session_id, target).await?;
                        serde_json::json!({
                            "target": target,
                            "content": content,
                            "chars": content.len()
                        })
                    }
                    _ => return Err(ToolError::InvalidArguments(format!(
                        "Invalid target '{}'. Must be one of: user, knowledge, scratch, notes, todo",
                        target
                    )))
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
                        "chars": user_content.len(),
                        "preview": Self::get_preview(&user_content)
                    },
                    "knowledge": {
                        "chars": knowledge_content.len(),
                        "preview": Self::get_preview(&knowledge_content)
                    }
                });

                // Read session files if session_id is set
                if let Some(ref session_id) = self.session_id {
                    let scratch_content = store.read_session_file(session_id, "scratch").await?;
                    let notes_content = store.read_session_file(session_id, "notes").await?;
                    let todo_content = store.read_session_file(session_id, "todo").await?;

                    result["scratch"] = serde_json::json!({
                        "chars": scratch_content.len(),
                        "preview": Self::get_preview(&scratch_content)
                    });
                    result["notes"] = serde_json::json!({
                        "chars": notes_content.len(),
                        "preview": Self::get_preview(&notes_content)
                    });
                    result["todo"] = serde_json::json!({
                        "chars": todo_content.len(),
                        "preview": Self::get_preview(&todo_content)
                    });
                }

                Ok(ToolOutput::success(result))
            }
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action '{}'. Available actions: add, replace, remove, read, list",
                action
            )))
        }
    }
}
