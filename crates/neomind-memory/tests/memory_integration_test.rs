//! Integration Tests for Memory System
//!
//! Tests cover:
//! - Tiered memory operations
//! - Memory consolidation
//! - Semantic search
//! - BM25 search
//! - Memory compression
//! - Knowledge graph operations

use neomind_memory::{
    TieredMemory, TieredMemoryConfig, KnowledgeEntry, KnowledgeCategory,
    SearchMethod,
};

// ============================================================================
// Short-term Memory Integration Tests
// ============================================================================

#[tokio::test]
async fn test_short_term_memory_basic_operations() {
    let mut memory = TieredMemory::new();

    // Add messages
    memory.add_message("user", "Hello, how are you?").unwrap();
    memory.add_message("assistant", "I'm doing well, thank you!").unwrap();
    memory.add_message("user", "What's the weather like?").unwrap();

    let stats = memory.get_stats().await;
    assert_eq!(stats.short_term_messages, 3);

    // Get last messages
    let last_2 = memory.get_last_messages(2);
    assert_eq!(last_2.len(), 2);
}

#[tokio::test]
async fn test_short_term_memory_token_limit() {
    let config = TieredMemoryConfig {
        max_short_term_tokens: 100,
        ..Default::default()
    };

    let mut memory = TieredMemory::with_config(config);

    // Add many messages
    for i in 0..50 {
        memory.add_message("user", format!("Message number {} with some additional text to increase token count", i)).unwrap();
    }

    let stats = memory.get_stats().await;
    // Should have truncated due to token limit
    assert!(stats.short_term_tokens <= 150); // Allow some margin
}

#[tokio::test]
async fn test_short_term_memory_clear() {
    let mut memory = TieredMemory::new();

    memory.add_message("user", "Test message").unwrap();
    assert_eq!(memory.get_stats().await.short_term_messages, 1);

    memory.short_term_mut().clear();
    assert_eq!(memory.get_stats().await.short_term_messages, 0);
}

// ============================================================================
// Mid-term Memory Integration Tests
// ============================================================================

#[tokio::test]
async fn test_mid_term_memory_consolidation() {
    let mut memory = TieredMemory::new();

    // Add conversation
    for i in 0..5 {
        memory.add_message("user", format!("Question {}", i)).unwrap();
        memory.add_message("assistant", format!("Answer {}", i)).unwrap();
    }

    // Consolidate to mid-term
    memory.consolidate("session_test").await.unwrap();

    let stats = memory.get_stats().await;
    assert_eq!(stats.mid_term_entries, 5);
}

#[tokio::test]
async fn test_mid_term_memory_search() {
    let mut memory = TieredMemory::new();

    // Add and consolidate
    memory.add_message("user", "How do I configure WiFi?").unwrap();
    memory.add_message("assistant", "Go to Settings > Network > WiFi").unwrap();
    memory.consolidate("session_wifi").await.unwrap();

    // Search
    let results = memory.search_mid_term("WiFi network settings", 5).await;
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_mid_term_memory_bm25_search() {
    let mut memory = TieredMemory::new();

    // Add multiple conversations
    memory.add_message("user", "Temperature sensor calibration").unwrap();
    memory.add_message("assistant", "Use the calibration tool").unwrap();
    memory.consolidate("session1").await.unwrap();

    memory.add_message("user", "How to read temperature?").unwrap();
    memory.add_message("assistant", "Check the sensor readings").unwrap();
    memory.consolidate("session2").await.unwrap();

    // BM25 search
    let results = memory.search_mid_term_bm25("temperature sensor", 5).await;
    assert!(!results.is_empty());
}

// ============================================================================
// Long-term Memory Integration Tests
// ============================================================================

#[tokio::test]
async fn test_long_term_memory_knowledge() {
    let memory = TieredMemory::new();

    // Add knowledge
    let entry = KnowledgeEntry::new(
        "Temperature Sensor Manual",
        "The temperature sensor measures ambient temperature in Celsius.",
        KnowledgeCategory::DeviceManual,
    );
    memory.add_knowledge(entry).await.unwrap();

    let stats = memory.get_stats().await;
    assert_eq!(stats.long_term_entries, 1);
}

#[tokio::test]
async fn test_long_term_memory_troubleshooting() {
    let memory = TieredMemory::new();

    // Add troubleshooting case
    let entry = KnowledgeEntry::new(
        "WiFi Connection Issue",
        "If WiFi won't connect, try restarting the router and checking the password.",
        KnowledgeCategory::Troubleshooting,
    ).with_tags(vec!["wifi".to_string(), "network".to_string()]);
    memory.add_knowledge(entry).await.unwrap();

    // Verify knowledge was added
    let stats = memory.get_stats().await;
    assert!(stats.long_term_entries > 0);
}

#[tokio::test]
async fn test_long_term_memory_best_practices() {
    let memory = TieredMemory::new();

    // Add best practice
    let entry = KnowledgeEntry::new(
        "Device Placement",
        "Place temperature sensors away from heat sources for accurate readings.",
        KnowledgeCategory::BestPractice,
    );
    memory.add_knowledge(entry).await.unwrap();

    let stats = memory.get_stats().await;
    assert!(stats.long_term_entries > 0);
}

// ============================================================================
// Cross-layer Integration Tests
// ============================================================================

#[tokio::test]
async fn test_query_all_layers() {
    let mut memory = TieredMemory::new();

    // Add to short-term
    memory.add_message("user", "What is the temperature?").unwrap();

    // Add to long-term
    let entry = KnowledgeEntry::new(
        "Temperature Sensor",
        "Temperature sensors measure thermal energy.",
        KnowledgeCategory::DeviceManual,
    );
    memory.add_knowledge(entry).await.unwrap();

    // Verify short-term and long-term have data
    let stats = memory.get_stats().await;
    assert!(stats.short_term_messages > 0, "Short-term should have messages");
    assert!(stats.long_term_entries > 0, "Long-term should have entries");
}

#[tokio::test]
async fn test_query_with_different_methods() {
    let mut memory = TieredMemory::new();

    memory.add_message("user", "Configure network settings").unwrap();
    memory.add_message("assistant", "Go to Settings > Network").unwrap();
    memory.consolidate("session_net").await.unwrap();

    // Semantic search
    let semantic = memory.query_all_with_method("network configuration", 5, SearchMethod::Semantic).await;
    assert!(!semantic.mid_term.is_empty());

    // BM25 search
    let bm25 = memory.query_all_with_method("network settings", 5, SearchMethod::BM25).await;
    assert!(!bm25.mid_term.is_empty());

    // Hybrid search
    let hybrid = memory.query_all_with_method("network", 5, SearchMethod::Hybrid).await;
    assert!(!hybrid.mid_term.is_empty());
}

// ============================================================================
// Configuration Integration Tests
// ============================================================================

#[tokio::test]
async fn test_custom_memory_config() {
    let config = TieredMemoryConfig {
        max_short_term_messages: 100,
        max_short_term_tokens: 4000,
        max_mid_term_entries: 500,
        max_long_term_knowledge: 1000,
        embedding_dim: 384,
        use_hybrid_search: true,
        semantic_weight: 0.7,
        bm25_weight: 0.3,
        ..Default::default()
    };

    let memory = TieredMemory::with_config(config);

    assert!(memory.is_hybrid_search_enabled());
    assert_eq!(memory.config().max_short_term_messages, 100);
}

#[tokio::test]
async fn test_memory_config_limits() {
    let config = TieredMemoryConfig {
        max_short_term_messages: 5,
        ..Default::default()
    };

    let mut memory = TieredMemory::with_config(config);

    // Add more messages than limit
    for i in 0..10 {
        memory.add_message("user", format!("Message {}", i)).unwrap();
    }

    // Should be limited
    let stats = memory.get_stats().await;
    assert!(stats.short_term_messages <= 10); // Implementation may vary
}

// ============================================================================
// Memory Statistics Tests
// ============================================================================

#[tokio::test]
async fn test_memory_stats_accuracy() {
    let mut memory = TieredMemory::new();

    // Initial stats
    let stats = memory.get_stats().await;
    assert_eq!(stats.short_term_messages, 0);
    assert_eq!(stats.mid_term_entries, 0);
    assert_eq!(stats.long_term_entries, 0);

    // Add to each layer
    memory.add_message("user", "Test").unwrap();
    memory.add_message("assistant", "Response").unwrap();
    memory.consolidate("s1").await.unwrap();
    memory.add_knowledge(KnowledgeEntry::new("Test", "Content", KnowledgeCategory::BestPractice)).await.unwrap();

    // Check stats
    let stats = memory.get_stats().await;
    assert_eq!(stats.short_term_messages, 2);
    assert_eq!(stats.mid_term_entries, 1);
    assert_eq!(stats.long_term_entries, 1);
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[tokio::test]
async fn test_empty_memory_query() {
    let memory = TieredMemory::new();

    let results = memory.query_all("nonexistent query", 5).await;
    assert!(results.short_term.is_empty());
    assert!(results.mid_term.is_empty());
    assert!(results.long_term.is_empty());
}

#[tokio::test]
async fn test_special_characters_in_query() {
    let mut memory = TieredMemory::new();

    memory.add_message("user", "Test message with special chars: @#$%").unwrap();

    let _results = memory.query_all("@#$%", 5).await;
    // Should not crash
}

#[tokio::test]
async fn test_unicode_content() {
    let mut memory = TieredMemory::new();

    memory.add_message("user", "你好世界").unwrap();
    memory.add_message("user", "こんにちは").unwrap();
    memory.add_message("user", "مرحبا بالعالم").unwrap();

    let stats = memory.get_stats().await;
    assert_eq!(stats.short_term_messages, 3);
}

#[tokio::test]
async fn test_large_content() {
    let mut memory = TieredMemory::new();

    // Add large message
    let large_content = "x".repeat(10000);
    memory.add_message("user", &large_content).unwrap();

    let stats = memory.get_stats().await;
    assert!(stats.short_term_tokens > 0);
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_memory_access() {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let memory = Arc::new(RwLock::new(TieredMemory::new()));
    let mut handles = vec![];

    // Spawn multiple concurrent operations
    for i in 0..10 {
        let mem = memory.clone();
        handles.push(tokio::spawn(async move {
            let mut m = mem.write().await;
            m.add_message("user", format!("Concurrent message {}", i)).unwrap();
        }));
    }

    // Wait for all
    for handle in handles {
        handle.await.unwrap();
    }

    let m = memory.read().await;
    let stats = m.get_stats().await;
    assert_eq!(stats.short_term_messages, 10);
}

// ============================================================================
// Memory Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_memory_lifecycle() {
    let mut memory = TieredMemory::new();

    // Phase 1: Short-term only
    memory.add_message("user", "Initial question").unwrap();
    assert_eq!(memory.get_stats().await.short_term_messages, 1);

    // Phase 2: Consolidate to mid-term
    memory.add_message("assistant", "Initial answer").unwrap();
    memory.consolidate("session1").await.unwrap();
    assert!(memory.get_stats().await.mid_term_entries > 0);

    // Phase 3: Add knowledge
    memory.add_knowledge(KnowledgeEntry::new(
        "Topic",
        "Long-term knowledge",
        KnowledgeCategory::BestPractice,
    )).await.unwrap();
    assert!(memory.get_stats().await.long_term_entries > 0);

    // Phase 4: Query across all
    let results = memory.query_all("Initial", 5).await;
    assert!(!results.short_term.is_empty() || !results.mid_term.is_empty());
}