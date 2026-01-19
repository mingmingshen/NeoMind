//! Unified Context Manager for AI Agent.
//!
//! This module provides a centralized context management system that:
//! 1. Integrates intent-based context selection (ContextSelector)
//! 2. Manages conversation history with token-efficient windowing
//! 3. Handles memory consolidation for long-term retention
//! 4. Provides context relevance scoring and prioritization

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use edge_ai_core::message::Message;

use crate::agent::AgentMessage;
use crate::agent::tokenizer::estimate_message_tokens;
use crate::context_selector::{ContextBundle, ContextSelector, IntentAnalysis, IntentType};

/// Maximum tokens for conversation history
const MAX_HISTORY_TOKENS: usize = 8000;
/// Minimum recent messages to keep
const MIN_RECENT_MESSAGES: usize = 4;
/// Tokens reserved for system prompt and tools
const RESERVED_TOKENS: usize = 2000;

/// Unified context containing all relevant information for LLM processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedContext {
    /// Intent analysis result
    pub intent: IntentAnalysis,
    /// Selected device/rule context bundle
    pub context_bundle: ContextBundle,
    /// Conversation history (token-compacted)
    pub history: Vec<HistoricalMessage>,
    /// Total estimated token count
    pub total_tokens: usize,
    /// Memory consolidation suggestions
    pub consolidation_hints: Vec<ConsolidationHint>,
}

/// Historical message with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMessage {
    /// The message content
    pub message: AgentMessage,
    /// Relevance score (0-1)
    pub relevance: f32,
    /// Estimated token count
    pub tokens: usize,
    /// Whether this message contains important tool results
    pub has_tool_results: bool,
}

/// Hint for memory consolidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationHint {
    /// Type of consolidation suggested
    pub hint_type: ConsolidationType,
    /// Message or topic to consolidate
    pub content: String,
    /// Priority (0-1)
    pub priority: f32,
    /// Reason for consolidation
    pub reason: String,
}

/// Type of consolidation operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsolidationType {
    /// Extract device discovery patterns
    DeviceDiscovery,
    /// Extract rule creation patterns
    RuleCreation,
    /// Extract recurring queries
    RecurringQuery,
    /// Summarize long conversation segment
    Summary,
}

/// Configuration for the unified context manager.
#[derive(Debug, Clone)]
pub struct ContextManagerConfig {
    /// Maximum tokens for history
    pub max_history_tokens: usize,
    /// Minimum recent messages to keep
    pub min_recent_messages: usize,
    /// Reserved tokens for system/tools
    pub reserved_tokens: usize,
    /// Enable automatic consolidation hints
    pub enable_consolidation: bool,
}

impl Default for ContextManagerConfig {
    fn default() -> Self {
        Self {
            max_history_tokens: MAX_HISTORY_TOKENS,
            min_recent_messages: MIN_RECENT_MESSAGES,
            reserved_tokens: RESERVED_TOKENS,
            enable_consolidation: true,
        }
    }
}

/// Unified context manager - central entry point for all context operations.
pub struct UnifiedContextManager {
    /// Context selector for device/rule context
    context_selector: Arc<RwLock<ContextSelector>>,
    /// Configuration
    config: ContextManagerConfig,
    /// Topic tracking for consolidation
    topic_history: Arc<RwLock<Vec<TopicEntry>>>,
}

/// Entry in topic history.
#[derive(Debug, Clone)]
struct TopicEntry {
    /// Topic/intent
    topic: String,
    /// Timestamp (seconds)
    timestamp: i64,
    /// Message count for this topic
    count: usize,
}

impl UnifiedContextManager {
    /// Create a new unified context manager.
    pub fn new(context_selector: Arc<RwLock<ContextSelector>>) -> Self {
        Self {
            context_selector,
            config: ContextManagerConfig::default(),
            topic_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        context_selector: Arc<RwLock<ContextSelector>>,
        config: ContextManagerConfig,
    ) -> Self {
        Self {
            context_selector,
            config,
            topic_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Build unified context for a query.
    ///
    /// This is the main entry point that combines:
    /// 1. Intent analysis and device/rule context selection
    /// 2. Conversation history compaction
    /// 3. Memory consolidation hints
    pub async fn build_context(
        &self,
        query: &str,
        history: &[AgentMessage],
    ) -> UnifiedContext {
        // Step 1: Analyze intent and select device/rule context
        let selector = self.context_selector.read().await;
        let (intent, context_bundle) = selector.select_context(query).await;
        drop(selector);

        // Step 2: Build compacted history with relevance scoring
        let history = self.build_relevant_history(query, &intent, history).await;

        // Step 3: Calculate total tokens
        let history_tokens: usize = history.iter().map(|m| m.tokens).sum();
        let total_tokens = history_tokens + context_bundle.estimated_tokens + self.config.reserved_tokens;

        // Step 4: Generate consolidation hints if enabled
        let consolidation_hints = if self.config.enable_consolidation {
            self.generate_consolidation_hints(&history, &intent).await
        } else {
            Vec::new()
        };

        UnifiedContext {
            intent,
            context_bundle,
            history,
            total_tokens,
            consolidation_hints,
        }
    }

    /// Build conversation history with relevance scoring and token management.
    async fn build_relevant_history(
        &self,
        query: &str,
        intent: &IntentAnalysis,
        history: &[AgentMessage],
    ) -> Vec<HistoricalMessage> {
        if history.is_empty() {
            return Vec::new();
        }

        let mut historical = Vec::new();
        let query_lower = query.to_lowercase();

        // Score each message for relevance
        for msg in history {
            let relevance = self.calculate_relevance(&query_lower, intent, msg);
            let tokens = estimate_message_tokens(msg);
            let has_tool_results = msg.tool_calls
                .as_ref()
                .map(|calls| calls.iter().any(|tc| tc.result.is_some()))
                .unwrap_or(false);

            historical.push(HistoricalMessage {
                message: msg.clone(),
                relevance,
                tokens,
                has_tool_results,
            });
        }

        // Token-aware selection: prioritize high relevance + recent messages
        self.select_within_token_limit(historical)
    }

    /// Calculate relevance score of a message to the current query.
    fn calculate_relevance(
        &self,
        query: &str,
        intent: &IntentAnalysis,
        msg: &AgentMessage,
    ) -> f32 {
        let mut score = 0.0;

        // Base score on message role
        match msg.role.as_str() {
            "user" | "assistant" => score += 0.3,
            "system" => score += 0.1,
            _ => {}
        }

        // Relevance based on content overlap
        let content_lower = msg.content.to_lowercase();
        let query_words: HashSet<&str> = query.split_whitespace().collect();
        let content_words: HashSet<&str> = content_lower.split_whitespace().collect();

        let overlap: usize = query_words.intersection(&content_words).count();
        if !query_words.is_empty() {
            score += (overlap as f32 / query_words.len() as f32) * 0.4;
        }

        // Boost if intent matches (device queries benefit from device-related history)
        match (&intent.intent_type, msg.content.as_str()) {
            (IntentType::DeviceQuery, content) if content.contains("设备") || content.contains("传感器") => {
                score += 0.2;
            }
            (IntentType::RuleQuery, content) if content.contains("规则") || content.contains("rule") => {
                score += 0.2;
            }
            (IntentType::RuleCreation, content) if content.contains("创建") || content.contains("规则") => {
                score += 0.2;
            }
            _ => {}
        }

        // Small boost for messages with tool results (useful context)
        if msg.tool_calls
            .as_ref()
            .map(|calls| calls.iter().any(|tc| tc.result.is_some()))
            .unwrap_or(false)
        {
            score += 0.1;
        }

        score.clamp(0.0, 1.0)
    }

    /// Select messages within token limit, prioritizing relevance and recency.
    fn select_within_token_limit(&self, mut historical: Vec<HistoricalMessage>) -> Vec<HistoricalMessage> {
        let mut selected = Vec::new();
        let mut total_tokens = 0;

        // Always keep the most recent N messages
        let recent_count = self.config.min_recent_messages;
        let recent_start = historical.len().saturating_sub(recent_count);

        // Sort by relevance score descending
        historical.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Select high-relevance messages within token limit
        for hist in &historical {
            // Skip if this is one of the recent messages (we'll add those separately)
            let is_recent = historical.iter().position(|h| std::ptr::eq(h, hist))
                .is_some_and(|pos| pos >= recent_start);

            if is_recent {
                continue;
            }

            if total_tokens + hist.tokens <= self.config.max_history_tokens {
                total_tokens += hist.tokens;
                selected.push(hist.clone());
            }
        }

        // Add recent messages (they weren't included yet)
        for hist in historical.iter().take(recent_count).rev() {
            if total_tokens + hist.tokens <= self.config.max_history_tokens * 2 {
                total_tokens += hist.tokens;
                selected.push(hist.clone());
            }
        }

        // Sort back to chronological order
        selected.sort_by(|a, b| {
            a.message
                .timestamp
                .cmp(&b.message.timestamp)
        });

        selected
    }

    /// Generate consolidation hints based on conversation patterns.
    async fn generate_consolidation_hints(
        &self,
        history: &[HistoricalMessage],
        _intent: &IntentAnalysis,
    ) -> Vec<ConsolidationHint> {
        let mut hints = Vec::new();

        // Detect recurring patterns
        let mut topic_counts: HashMap<String, usize> = HashMap::new();

        for hist in history {
            // Detect device discovery
            if hist.message.content.contains("设备") || hist.message.content.contains("device") {
                *topic_counts.entry("device_discovery".to_string()).or_insert(0) += 1;
            }

            // Detect rule creation
            if hist.message.content.contains("创建规则") || hist.message.content.contains("create rule") {
                *topic_counts.entry("rule_creation".to_string()).or_insert(0) += 1;
            }

            // Detect recurring queries
            if hist.message.role == "user" && hist.message.content.len() < 50 {
                *topic_counts.entry("short_query".to_string()).or_insert(0) += 1;
            }
        }

        // Generate hints from patterns
        for (topic, count) in topic_counts {
            if count >= 3 {
                match topic.as_str() {
                    "device_discovery" => {
                        hints.push(ConsolidationHint {
                            hint_type: ConsolidationType::DeviceDiscovery,
                            content: format!("{} device-related interactions", count),
                            priority: 0.6,
                            reason: "Frequent device queries detected".to_string(),
                        });
                    }
                    "rule_creation" => {
                        hints.push(ConsolidationHint {
                            hint_type: ConsolidationType::RuleCreation,
                            content: format!("{} rule creation operations", count),
                            priority: 0.7,
                            reason: "Multiple rule operations detected".to_string(),
                        });
                    }
                    "short_query" => {
                        hints.push(ConsolidationHint {
                            hint_type: ConsolidationType::RecurringQuery,
                            content: format!("{} short queries", count),
                            priority: 0.5,
                            reason: "Pattern of short queries detected".to_string(),
                        });
                    }
                    _ => {}
                }
            }
        }

        // Suggest summary for long conversations
        let total_messages = history.len();
        if total_messages > 20 {
            hints.push(ConsolidationHint {
                hint_type: ConsolidationType::Summary,
                content: format!("{} messages in history", total_messages),
                priority: 0.8,
                reason: "Long conversation - consider summarizing earlier messages".to_string(),
            });
        }

        hints
    }

    /// Convert unified context to LLM messages format.
    pub fn to_llm_messages(&self, context: &UnifiedContext, system_prompt: &str) -> Vec<Message> {
        let mut messages = Vec::new();

        // System prompt first
        messages.push(Message::system(system_prompt));

        // Add context bundle info as system message if present
        if !context.context_bundle.device_types.is_empty()
            || !context.context_bundle.rules.is_empty()
        {
            let context_info = self.format_context_bundle(&context.context_bundle);
            messages.push(Message::system(&context_info));
        }

        // Add conversation history
        for hist in &context.history {
            match hist.message.role.as_str() {
                "user" => {
                    messages.push(Message::user(&hist.message.content));
                }
                "assistant" => {
                    messages.push(Message::assistant(&hist.message.content));
                }
                "system" => {
                    // Skip system messages from history
                }
                _ => {}
            }
        }

        messages
    }

    /// Format context bundle for inclusion in prompt.
    fn format_context_bundle(&self, bundle: &ContextBundle) -> String {
        let mut info = String::from("=== Available Context ===\n");

        if !bundle.device_types.is_empty() {
            info.push_str("\n【Device Types】\n");
            for dt in &bundle.device_types {
                info.push_str(&format!("- {}: {}\n", dt.device_type, dt.name));
                if !dt.metrics.is_empty() {
                    info.push_str(&format!("  Metrics: {}\n", dt.metrics.join(", ")));
                }
                if !dt.commands.is_empty() {
                    info.push_str(&format!("  Commands: {}\n", dt.commands.join(", ")));
                }
            }
        }

        if !bundle.rules.is_empty() {
            info.push_str("\n【Active Rules】\n");
            for rule in &bundle.rules {
                info.push_str(&format!("- [{}] {} ({})\n", rule.rule_id, rule.name, rule.condition));
            }
        }

        if !bundle.commands.is_empty() {
            info.push_str(&format!(
                "\n【Available Commands】\n{} commands available\n",
                bundle.commands.len()
            ));
        }

        info.push_str(&format!("\nEstimated context tokens: {}\n", bundle.estimated_tokens));

        info
    }

    /// Update topic history with current intent.
    pub async fn update_topic_history(&self, intent: &IntentAnalysis) {
        let mut history = self.topic_history.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Check if we have a recent entry for this intent
        let intent_str = format!("{:?}", intent.intent_type);
        if let Some(entry) = history.iter_mut().find(|e| e.topic == intent_str) {
            entry.count += 1;
            entry.timestamp = now;
        } else {
            history.push(TopicEntry {
                topic: intent_str,
                timestamp: now,
                count: 1,
            });
        }

        // Keep only recent entries (last 100)
        while history.len() > 100 {
            history.remove(0);
        }
    }

    /// Get context selector reference.
    pub fn context_selector(&self) -> &Arc<RwLock<ContextSelector>> {
        &self.context_selector
    }

    /// Get current configuration.
    pub fn config(&self) -> &ContextManagerConfig {
        &self.config
    }

    /// Update configuration.
    pub async fn update_config(&self, _config: ContextManagerConfig) {
        // This would need interior mutability or a different approach
        // For now, just note this would be called during reconfiguration
    }
}

impl Default for UnifiedContextManager {
    fn default() -> Self {
        Self::new(Arc::new(RwLock::new(ContextSelector::new())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentMessage;

    #[tokio::test]
    async fn test_unified_context_build() {
        let selector = Arc::new(RwLock::new(ContextSelector::new()));
        let manager = UnifiedContextManager::new(selector);

        let history = vec![
            AgentMessage::user("查询温度"),
            AgentMessage::assistant("当前温度是25度"),
        ];

        let context = manager.build_context("当前温度多少", &history).await;

        assert!(!context.history.is_empty());
        assert_eq!(context.intent.intent_type, IntentType::DeviceQuery);
    }

    #[tokio::test]
    async fn test_consolidation_hints() {
        let selector = Arc::new(RwLock::new(ContextSelector::new()));
        let manager = UnifiedContextManager::new(selector);

        // Create long conversation
        let mut history = Vec::new();
        for i in 0..25 {
            history.push(AgentMessage::user(&format!("查询消息{}", i)));
            history.push(AgentMessage::assistant(&format!("回复{}", i)));
        }

        let context = manager.build_context("查询状态", &history).await;

        // Should suggest summary for long conversation
        assert!(context.consolidation_hints
            .iter()
            .any(|h| h.hint_type == ConsolidationType::Summary));
    }
}
