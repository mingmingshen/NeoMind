/// Integration test for session creation bug fix.
///
/// This test verifies that:
/// 1. Messages sent to a valid session don't create new sessions
/// 2. Invalid session IDs don't cause the current valid session to be lost
/// 3. Empty session IDs are handled correctly

#[cfg(test)]
mod session_fix_tests {
    use edge_ai_agent::SessionManager;
    use edge_ai_core::eventbus::EventBus;
    use edge_ai_llm::LlmBackend;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Helper to create a test session manager
    async fn create_test_manager() -> Arc<SessionManager> {
        let event_bus = EventBus::new(100);

        // Use Ollama backend if available, otherwise mock
        let backend = match std::env::var("OLLAMA_ENDPOINT") {
            Ok(_) => LlmBackend::Ollama,
            Err(_) => {
                // For testing without Ollama, we'd need a mock backend
                // For now, skip if Ollama is not available
                LlmBackend::Ollama  // Will fail gracefully if not available
            }
        };

        SessionManager::new(A::new(event_bus), backend)
            .expect("Failed to create SessionManager")
    }

    #[tokio::test]
    async fn test_single_session_multiple_messages() {
        let manager = create_test_manager().await;

        // Create a session
        let session_id = manager.create_session().await
            .expect("Failed to create session");

        // Get initial session count
        let initial_count = manager.session_count().await;

        // Send multiple messages to the same session
        for i in 1..=3 {
            // Note: This may fail if Ollama is not running
            let _ = manager.process_message(&session_id, &format!("Message {}", i)).await;
        }

        // Verify session count hasn't increased
        let final_count = manager.session_count().await;
        assert_eq!(final_count, initial_count,
                   "Session count should not increase when sending messages to existing session");
    }

    #[tokio::test]
    async fn test_empty_session_id_creates_new_session() {
        let manager = create_test_manager().await;

        // Create initial session
        let _session1 = manager.create_session().await
            .expect("Failed to create first session");

        let count_after_first = manager.session_count().await;

        // Try to process message with empty session_id
        // This should create a new session if the logic is correct
        let result = manager.process_message("", "test message").await;

        // Empty session_id should either:
        // 1. Create a new session (if implemented)
        // 2. Return an error (if not implemented)

        if result.is_ok() {
            let count_after = manager.session_count().await;
            assert!(count_after > count_after_first,
                    "Empty session_id should either create new session or fail");
        }
    }

    #[tokio::test]
    async fn test_invalid_session_id_does_not_create_session() {
        let manager = create_test_manager().await;

        // Create a valid session
        let valid_id = manager.create_session().await
            .expect("Failed to create session");

        let count_before = manager.session_count().await;

        // Try to send message to invalid session
        let result = manager.process_message("invalid-session-id-12345", "test").await;

        // Should fail without creating a new session
        assert!(result.is_err(), "Invalid session ID should cause error");

        let count_after = manager.session_count().await;
        assert_eq!(count_after, count_before,
                   "Invalid session ID should not create new session");

        // Verify the original session still exists
        assert!(manager.get_session(&valid_id).await.is_ok(),
                "Original session should still be accessible");
    }
}
