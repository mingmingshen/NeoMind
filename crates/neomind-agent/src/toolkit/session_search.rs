//! Session search tool for searching conversation history.
//!
//! Allows the LLM to search past conversation messages in the current session
//! using simple keyword matching.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::error::{Result, ToolError};
use super::tool::{object_schema, string_property, Tool, ToolOutput};
use neomind_core::tools::ToolCategory;

/// Maximum characters per search result snippet.
const MAX_SNIPPET_LEN: usize = 200;

/// Tool for searching conversation history within the current session.
pub struct SessionSearchTool {
    session_store: Arc<neomind_storage::SessionStore>,
}

impl SessionSearchTool {
    /// Create a new session search tool.
    pub fn new(session_store: Arc<neomind_storage::SessionStore>) -> Self {
        Self { session_store }
    }
}

#[async_trait]
impl Tool for SessionSearchTool {
    fn name(&self) -> &str {
        "session_search"
    }

    fn description(&self) -> &str {
        "Search past conversation history in the current session. Use when you need to recall what was discussed earlier. Returns relevant message snippets."
    }

    fn parameters(&self) -> Value {
        object_schema(
            serde_json::json!({
                "query": string_property("Search query - keywords to find in past messages"),
                "session_id": string_property("Current session ID"),
            }),
            vec!["query".to_string(), "session_id".to_string()],
        )
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::System
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' parameter".to_string()))?;
        let session_id = args["session_id"].as_str().ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'session_id' parameter".to_string())
        })?;

        let limit = args["limit"].as_u64().unwrap_or(3) as usize;

        // Load session history and search
        let messages = self
            .session_store
            .load_history(session_id)
            .map_err(|e| ToolError::Execution(format!("Failed to load history: {}", e)))?;

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for msg in &messages {
            if msg.role != "user" && msg.role != "assistant" {
                continue;
            }
            let content_lower = msg.content.to_lowercase();
            if !content_lower.contains(&query_lower) {
                continue;
            }

            let snippet = truncate_str(&msg.content, MAX_SNIPPET_LEN);
            results.push(serde_json::json!({
                "role": msg.role,
                "snippet": snippet,
                "timestamp": msg.timestamp,
            }));

            if results.len() >= limit {
                break;
            }
        }

        if results.is_empty() {
            return Ok(ToolOutput::success(serde_json::json!({
                "found": false,
                "message": format!("No messages found matching '{}'", query),
            })));
        }

        Ok(ToolOutput::success(serde_json::json!({
            "found": true,
            "results": results,
        })))
    }
}

/// Truncate a string to a maximum character length, respecting Unicode boundaries.
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_chars).collect();
    format!("{}...", truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello world", 100), "hello world");
        assert_eq!(truncate_str("hello world", 5), "hello...");
    }
}
