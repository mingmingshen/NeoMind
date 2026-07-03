//! Chat Stream Capability
//!
//! Streaming chat session invocation backed by NeoMind's SessionManager.
//!
//! `invoke()` triggers a chat turn for a given (or new) session and returns
//! immediately with the `session_id`. The actual `AgentEvent` stream produced
//! by `SessionManager::process_message_events` is published onto the platform
//! EventBus as `NeoMindEvent::AgentStreamChunk` events, each tagged with the
//! same `session_id`. Extensions consume the stream by subscribing via the
//! existing `event::subscribe` capability and filtering by `session_id`.
//!
//! # Parameters
//!
//! - `message` (required): user message text
//! - `session_id` (optional): existing session to continue; omit to let the
//!   host create a new ad-hoc session.
//!
//! # Return
//!
//! ```json
//! { "session_id": "uuid", "created": true }
//! ```
//!
//! `created=true` means a new session was allocated for this call; `false`
//! means an existing session was reused.
//!
//! # Event payload
//!
//! Each `AgentStreamChunk` event carries the full AgentEvent JSON, e.g.:
//!
//! ```json
//! {
//!   "type": "agent_stream_chunk",
//!   "session_id": "...",
//!   "chunk": { "type": "Content", "content": "hello" },
//!   "timestamp": 0
//! }
//! ```

use serde_json::{json, Value};

pub type CapabilityError = String;

#[cfg(not(target_arch = "wasm32"))]
pub type Context = crate::host::ExtensionContext;

#[cfg(not(target_arch = "wasm32"))]
use crate::host::ExtensionCapability;

/// Invoke a streaming chat turn.
///
/// Returns the `session_id` to subscribe to. Callers should already have an
/// event handler registered (via `event::register_handler`) and pass a
/// subscription filter matching `"agent_stream_chunk"` for this `session_id`.
#[cfg(not(target_arch = "wasm32"))]
pub async fn invoke(
    context: &Context,
    message: &str,
    session_id: Option<&str>,
) -> Result<Value, CapabilityError> {
    let mut params = json!({ "message": message });
    if let Some(sid) = session_id {
        params["session_id"] = json!(sid);
    }
    context
        .invoke_capability(ExtensionCapability::ChatStream, &params)
        .await
        .map_err(|e| e.to_string())
}

/// Convenience: invoke and extract `session_id` from the response.
#[cfg(not(target_arch = "wasm32"))]
pub async fn invoke_for_session_id(
    context: &Context,
    message: &str,
    session_id: Option<&str>,
) -> Result<String, CapabilityError> {
    let resp = invoke(context, message, session_id).await?;
    resp.get("session_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "missing session_id in ChatStream response".to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoke_params_with_existing_session() {
        let params = json!({
            "message": "hello",
            "session_id": "sess-123",
        });
        assert_eq!(params["message"], "hello");
        assert_eq!(params["session_id"], "sess-123");
    }

    #[test]
    fn test_invoke_params_new_session() {
        // When session_id is omitted, only message is required.
        let params = json!({ "message": "hello" });
        assert_eq!(params["message"], "hello");
        assert!(params.get("session_id").is_none());
    }

    #[test]
    fn test_invoke_for_session_id_extracts_field() {
        let resp = json!({ "session_id": "abc-xyz", "created": true });
        let sid = resp
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        assert_eq!(sid.as_deref(), Some("abc-xyz"));
    }

    #[test]
    fn test_invoke_for_session_id_missing_field() {
        let resp = json!({ "created": true });
        let sid = resp.get("session_id").and_then(|v| v.as_str());
        assert!(sid.is_none());
    }
}
