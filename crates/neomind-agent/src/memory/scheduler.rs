//! Memory scheduler for background extraction and compression tasks
//!
//! Runs periodic tasks for memory extraction from Chat/Agent sources
//! and memory compression for importance decay and summarization.

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use super::manager::MemoryManager;
use neomind_storage::MemoryConfig;

/// Memory scheduler for background tasks
pub struct MemoryScheduler {
    manager: Arc<RwLock<MemoryManager>>,
    config: MemoryConfig,
    extraction_handle: Option<tokio::task::JoinHandle<()>>,
    compression_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MemoryScheduler {
    /// Create a new scheduler
    pub fn new(manager: Arc<RwLock<MemoryManager>>) -> Self {
        let config = {
            tokio::task::block_in_place(|| {
                futures::executor::block_on(async { manager.read().await.config().clone() })
            })
        };

        Self {
            manager,
            config,
            extraction_handle: None,
            compression_handle: None,
        }
    }

    /// Create scheduler with explicit config
    pub fn with_config(manager: Arc<RwLock<MemoryManager>>, config: MemoryConfig) -> Self {
        Self {
            manager,
            config,
            extraction_handle: None,
            compression_handle: None,
        }
    }

    /// Start background tasks
    pub fn start(&mut self) {
        if !self.config.enabled {
            info!("Memory system disabled, not starting scheduler");
            return;
        }

        // Start extraction task
        if self.config.schedule.extraction_enabled {
            let manager = self.manager.clone();
            let interval_secs = self.config.schedule.extraction_interval_secs;

            self.extraction_handle = Some(tokio::spawn(async move {
                let mut timer = interval(Duration::from_secs(interval_secs));

                info!(
                    interval_secs = interval_secs,
                    "Memory extraction scheduler started"
                );

                loop {
                    timer.tick().await;

                    info!("Scheduled memory extraction triggered");

                    match Self::run_extraction(&manager).await {
                        Ok(count) => {
                            info!(entries_extracted = count, "Extraction completed");
                        }
                        Err(e) => {
                            error!(error = %e, "Extraction failed");
                        }
                    }
                }
            }));
        }

        // Start compression task
        if self.config.schedule.compression_enabled {
            let manager = self.manager.clone();
            let interval_secs = self.config.schedule.compression_interval_secs;

            self.compression_handle = Some(tokio::spawn(async move {
                let mut timer = interval(Duration::from_secs(interval_secs));

                info!(
                    interval_secs = interval_secs,
                    "Memory compression scheduler started"
                );

                loop {
                    timer.tick().await;

                    info!("Scheduled memory compression triggered");

                    match Self::run_compression(&manager).await {
                        Ok(result) => {
                            info!(
                                total_before = result.total_before,
                                kept = result.kept,
                                compressed = result.compressed,
                                deleted = result.deleted,
                                "Compression completed"
                            );
                        }
                        Err(e) => {
                            error!(error = %e, "Compression failed");
                        }
                    }
                }
            }));
        }
    }

    /// Run extraction on all categories
    async fn run_extraction(manager: &Arc<RwLock<MemoryManager>>) -> Result<usize, String> {
        // TODO: Implement actual extraction from Chat/Agent logs
        // For now, just return 0
        let _ = manager;
        Ok(0)
    }

    /// Run compression on all categories
    async fn run_compression(
        manager: &Arc<RwLock<MemoryManager>>,
    ) -> Result<super::compressor::CompressionResult, String> {
        use super::compressor::MemoryCompressor;
        use neomind_storage::MemoryCategory;

        let compressor = MemoryCompressor::with_defaults();
        let mut total_result = super::compressor::CompressionResult::default();

        let mgr = manager.read().await;

        for category in MemoryCategory::all() {
            let stats = mgr
                .stats(category)
                .await
                .map_err(|e| format!("Failed to get stats: {}", e))?;

            total_result.total_before += stats.entry_count;

            // Check if compression is needed
            let max = compressor.max_entries(category);
            if stats.entry_count > max {
                // TODO: Implement actual compression
                warn!(
                    category = ?category,
                    current = stats.entry_count,
                    max = max,
                    "Category exceeds max entries, compression needed"
                );
            }

            total_result.kept += stats.entry_count;
        }

        Ok(total_result)
    }

    /// Stop background tasks
    pub fn stop(&mut self) {
        if let Some(handle) = self.extraction_handle.take() {
            handle.abort();
            info!("Extraction scheduler stopped");
        }

        if let Some(handle) = self.compression_handle.take() {
            handle.abort();
            info!("Compression scheduler stopped");
        }
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.extraction_handle.is_some() || self.compression_handle.is_some()
    }
}

impl Drop for MemoryScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_scheduler_creation() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();
        config.schedule.extraction_interval_secs = 1;
        config.schedule.compression_interval_secs = 1;

        let manager = Arc::new(RwLock::new(MemoryManager::new(config.clone())));
        let scheduler = MemoryScheduler::with_config(manager, config);

        assert!(!scheduler.is_running());
    }

    #[tokio::test]
    async fn test_scheduler_start_stop() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();
        config.schedule.extraction_interval_secs = 60; // Long enough not to trigger
        config.schedule.compression_interval_secs = 60;

        let manager = Arc::new(RwLock::new(MemoryManager::new(config.clone())));
        let mut scheduler = MemoryScheduler::with_config(manager, config);

        scheduler.start();
        assert!(scheduler.is_running());

        scheduler.stop();
        assert!(!scheduler.is_running());
    }

    #[test]
    fn test_disabled_scheduler() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut config = MemoryConfig::default();
        config.storage_path = temp.path().to_string_lossy().to_string();
        config.enabled = false;

        let manager = Arc::new(RwLock::new(MemoryManager::new(config.clone())));
        let mut scheduler = MemoryScheduler::with_config(manager, config);

        scheduler.start();
        assert!(!scheduler.is_running()); // Should not start when disabled
    }
}
