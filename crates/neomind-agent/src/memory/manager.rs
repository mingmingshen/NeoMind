//! Memory Manager - Unified entry point for memory operations
//!
//! This module provides a unified interface for memory operations,
//! wrapping the MarkdownMemoryStore and coordinating extraction,
//! compression, and deduplication.

use neomind_storage::{
    CategoryStats, MarkdownMemoryStore, MemoryCategory, MemoryConfig,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Memory manager - unified entry point for memory operations
#[derive(Debug)]
pub struct MemoryManager {
    config: MemoryConfig,
    store: Arc<RwLock<MarkdownMemoryStore>>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(config: MemoryConfig) -> Self {
        let store = MarkdownMemoryStore::new(&config.storage_path);
        Self {
            config,
            store: Arc::new(RwLock::new(store)),
        }
    }

    /// Initialize the memory system
    pub async fn init(&self) -> neomind_storage::error::Result<()> {
        let store = self.store.read().await;
        store.init()
    }

    /// Get the current configuration
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }

    /// Update the configuration
    pub async fn update_config(&mut self, config: MemoryConfig) {
        self.config = config.clone();
        let store = MarkdownMemoryStore::new(&config.storage_path);
        self.store = Arc::new(RwLock::new(store));
    }

    /// Read memory content for a category
    pub async fn read(&self, category: &MemoryCategory) -> neomind_storage::error::Result<String> {
        let store = self.store.read().await;
        store.read_category(category)
    }

    /// Write memory content for a category
    pub async fn write(&self, category: &MemoryCategory, content: &str) -> neomind_storage::error::Result<()> {
        let store = self.store.read().await;
        store.write_category(category, content)
    }

    /// Get statistics for a category
    pub async fn stats(&self, category: &MemoryCategory) -> neomind_storage::error::Result<CategoryStats> {
        let store = self.store.read().await;
        store.category_stats(category)
    }

    /// Get statistics for all categories
    pub async fn all_stats(&self) -> neomind_storage::error::Result<HashMap<String, CategoryStats>> {
        let store = self.store.read().await;
        store.all_stats()
    }

    /// Export all memory as a single markdown string
    pub async fn export(&self) -> neomind_storage::error::Result<String> {
        let store = self.store.read().await;
        store.export_all()
    }

    /// Check if memory system is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the store (for internal operations)
    pub fn store(&self) -> Arc<RwLock<MarkdownMemoryStore>> {
        self.store.clone()
    }
}

impl Clone for MemoryManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            store: self.store.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_manager_init() {
        let temp = TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();

        let manager = MemoryManager::new(config);
        assert!(manager.init().await.is_ok());
        assert!(manager.is_enabled());
    }

    #[tokio::test]
    async fn test_manager_read_write() {
        let temp = TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();

        let manager = MemoryManager::new(config);
        manager.init().await.unwrap();

        let content = "# Test\n\n- item1\n";
        manager.write(&MemoryCategory::UserProfile, content).await.unwrap();

        let read = manager.read(&MemoryCategory::UserProfile).await.unwrap();
        assert!(read.contains("item1"));
    }

    #[tokio::test]
    async fn test_manager_stats() {
        let temp = TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();

        let manager = MemoryManager::new(config);
        manager.init().await.unwrap();

        let content = "# 用户画像\n\n## 偏好\n\n- 偏好1\n- 偏好2\n";
        manager.write(&MemoryCategory::UserProfile, content).await.unwrap();

        let stats = manager.stats(&MemoryCategory::UserProfile).await.unwrap();
        assert!(stats.file_size > 0);

        let all_stats = manager.all_stats().await.unwrap();
        assert!(all_stats.contains_key("user_profile"));
    }

    #[tokio::test]
    async fn test_manager_export() {
        let temp = TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();

        let manager = MemoryManager::new(config);
        manager.init().await.unwrap();

        manager
            .write(&MemoryCategory::UserProfile, "# 用户画像\n\n- 偏好1\n")
            .await
            .unwrap();

        let export = manager.export().await.unwrap();
        assert!(export.contains("NeoMind Memory Export"));
        assert!(export.contains("用户画像"));
    }
}
