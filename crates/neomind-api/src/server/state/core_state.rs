//! Core system services state.
//!
//! Contains fundamental services used across the application:
//! - EventBus for event-driven communication
//! - CommandManager for command history and retry
//! - MessageManager for unified messaging
//!
//! Note: ExtensionRegistry has been moved to ExtensionState for proper decoupling.

use std::sync::Arc;

use neomind_commands::CommandManager;
use neomind_core::EventBus;
use neomind_messages::MessageManager;

/// Core system services state.
///
/// Provides access to fundamental cross-cutting services.
#[derive(Clone)]
pub struct CoreState {
    /// Event bus for system-wide event distribution.
    pub event_bus: Option<Arc<EventBus>>,

    /// Command manager for command history and retry.
    pub command_manager: Option<Arc<CommandManager>>,

    /// Message manager for unified messages/notifications system.
    pub message_manager: Arc<MessageManager>,
}

impl CoreState {
    /// Create a new core state.
    pub fn new(
        event_bus: Option<Arc<EventBus>>,
        command_manager: Option<Arc<CommandManager>>,
        message_manager: Arc<MessageManager>,
    ) -> Self {
        Self {
            event_bus,
            command_manager,
            message_manager,
        }
    }

    /// Create a minimal core state (for testing).
    #[cfg(test)]
    pub fn minimal() -> Self {
        use neomind_core::EventBus;
        Self {
            event_bus: Some(Arc::new(EventBus::new())),
            command_manager: None,
            message_manager: Arc::new(MessageManager::new()),
        }
    }
}
