//! Extension registry for managing dynamically loaded extensions.
//!
//! The registry provides:
//! - Extension registration and lifecycle management
//! - Extension discovery from filesystem
//! - Health monitoring
//! - Safety management (circuit breaker, panic isolation)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::extension::loader::{NativeExtensionLoader, WasmExtensionLoader};
use crate::extension::safety::ExtensionSafetyManager;
use crate::extension::system::{
    DynExtension, ExtensionError, ExtensionMetadata, ExtensionState, ExtensionStats,
};
use crate::extension::types::Result;

/// Information about a registered extension.
#[derive(Debug, Clone)]
pub struct ExtensionInfo {
    /// Extension metadata
    pub metadata: ExtensionMetadata,
    /// Current state
    pub state: ExtensionState,
    /// Runtime statistics
    pub stats: ExtensionStats,
    /// When the extension was loaded
    pub loaded_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Metrics provided by this extension
    pub metrics: Vec<super::system::MetricDescriptor>,
    /// Commands provided by this extension
    pub commands: Vec<super::system::ExtensionCommand>,
}

/// Registry for managing extensions.
pub struct ExtensionRegistry {
    /// Registered extensions
    extensions: RwLock<HashMap<String, DynExtension>>,
    /// Extension information cache
    info_cache: RwLock<HashMap<String, ExtensionInfo>>,
    /// Native extension loader
    native_loader: NativeExtensionLoader,
    /// WASM extension loader
    wasm_loader: WasmExtensionLoader,
    /// Extension directories to scan
    extension_dirs: Vec<PathBuf>,
    /// Loaded libraries (kept alive to prevent unloading)
    _loaded_libraries: Vec<libloading::Library>,
    /// Safety manager for circuit breaking and panic isolation
    safety_manager: Arc<ExtensionSafetyManager>,
}

impl ExtensionRegistry {
    /// Create a new extension registry.
    pub fn new() -> Self {
        Self {
            extensions: RwLock::new(HashMap::new()),
            info_cache: RwLock::new(HashMap::new()),
            native_loader: NativeExtensionLoader::new(),
            wasm_loader: WasmExtensionLoader::new().expect("Failed to create WASM loader"),
            extension_dirs: vec![],
            _loaded_libraries: vec![],
            safety_manager: Arc::new(ExtensionSafetyManager::new()),
        }
    }

    /// Add an extension directory to scan.
    pub fn add_extension_dir(&mut self, path: PathBuf) {
        self.extension_dirs.push(path);
    }

    /// Register an extension instance.
    pub async fn register(&self, id: String, extension: DynExtension) -> Result<()> {
        let ext = extension.read().await;
        let metadata = ext.metadata().clone();
        let metrics = ext.metrics().to_vec();
        let commands = ext.commands().to_vec();
        drop(ext);

        // Check if already registered
        if self.extensions.read().await.contains_key(&id) {
            return Err(ExtensionError::AlreadyRegistered(id));
        }

        // Store extension
        self.extensions
            .write()
            .await
            .insert(id.clone(), extension.clone());

        // Store info
        self.info_cache.write().await.insert(
            id.clone(),
            ExtensionInfo {
                metadata,
                state: ExtensionState::Running,
                stats: ExtensionStats::default(),
                loaded_at: Some(chrono::Utc::now()),
                metrics,
                commands,
            },
        );

        tracing::info!("Extension registered: {}", id);
        Ok(())
    }

    /// Unregister an extension.
    pub async fn unregister(&self, id: &str) -> Result<()> {
        self.extensions.write().await.remove(id);
        self.info_cache.write().await.remove(id);
        tracing::info!("Extension unregistered: {}", id);
        Ok(())
    }

    /// Register an extension instance (blocking version).
    pub fn blocking_register(&self, id: String, extension: DynExtension) -> Result<()> {
        let ext = extension.blocking_read();
        let metadata = ext.metadata().clone();
        let metrics = ext.metrics().to_vec();
        let commands = ext.commands().to_vec();
        drop(ext);

        // Check if already registered
        if self.extensions.blocking_read().contains_key(&id) {
            return Err(ExtensionError::AlreadyRegistered(id));
        }

        // Store extension
        self.extensions
            .blocking_write()
            .insert(id.clone(), extension.clone());

        // Store info
        self.info_cache.blocking_write().insert(
            id.clone(),
            ExtensionInfo {
                metadata,
                state: ExtensionState::Running,
                stats: ExtensionStats::default(),
                loaded_at: Some(chrono::Utc::now()),
                metrics,
                commands,
            },
        );

        tracing::info!("Extension registered: {}", id);
        Ok(())
    }

    /// Get an extension by ID.
    pub async fn get(&self, id: &str) -> Option<DynExtension> {
        self.extensions.read().await.get(id).cloned()
    }

    /// Get extension info by ID.
    pub async fn get_info(&self, id: &str) -> Option<ExtensionInfo> {
        self.info_cache.read().await.get(id).cloned()
    }

    /// Get current metric values from an extension.
    /// This calls the extension's `produce_metrics()` method and returns the current values.
    pub async fn get_current_metrics(&self, id: &str) -> Vec<super::system::ExtensionMetricValue> {
        if let Some(ext) = self.get(id).await {
            let ext = ext.read().await;
            // Call produce_metrics with panic handling
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ext.produce_metrics())) {
                Ok(Ok(metrics)) => metrics,
                Ok(Err(e)) => {
                    tracing::warn!(
                        extension_id = %id,
                        error = %e,
                        "[ExtensionRegistry] Extension failed to produce metrics"
                    );
                    Vec::new()
                }
                Err(_) => {
                    tracing::error!(
                        extension_id = %id,
                        "[ExtensionRegistry] Extension panicked while producing metrics"
                    );
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        }
    }

    /// List all extensions.
    pub async fn list(&self) -> Vec<ExtensionInfo> {
        self.info_cache.read().await.values().cloned().collect()
    }

    /// Load an extension from a file path and register it.
    pub async fn load_from_path(&self, path: &Path) -> Result<ExtensionMetadata> {
        let extension = path.extension().and_then(|e| e.to_str());

        match extension {
            Some("so") | Some("dylib") | Some("dll") => {
                // Load the native extension
                let loaded = self.native_loader.load(path)?;

                // Get metadata and metrics/commands
                let ext = loaded.extension.read().await;
                let metadata = ext.metadata().clone();
                let _metrics = ext.metrics().to_vec();
                let _commands = ext.commands().to_vec();
                drop(ext);

                // Register the extension
                let id = metadata.id.clone();
                self.register(id, loaded.extension).await?;

                Ok(metadata)
            }
            Some("wasm") => {
                // Load the WASM extension
                let loaded = self.wasm_loader.load(path).await?;

                // Get metadata and metrics/commands
                let ext = loaded.extension.read().await;
                let metadata = ext.metadata().clone();
                let _metrics = ext.metrics().to_vec();
                let _commands = ext.commands().to_vec();
                drop(ext);

                // Register the extension
                let id = metadata.id.clone();
                self.register(id, loaded.extension).await?;

                Ok(metadata)
            }
            _ => Err(ExtensionError::InvalidFormat(format!(
                "Unsupported extension format: {:?}",
                path
            ))),
        }
    }

    /// Load an extension from a file path with a provided config (blocking version).
    ///
    /// This is a synchronous version that can be called from `spawn_blocking`.
    /// It handles both native and WASM extensions.
    pub fn blocking_load(&self, path: &Path, config: &serde_json::Value) -> Result<()> {
        let extension = path.extension().and_then(|e| e.to_str());

        match extension {
            Some("so") | Some("dylib") | Some("dll") => {
                // Load the native extension
                let loaded = self.native_loader.load_with_config(path, Some(config))?;

                // Get metadata
                let metadata = {
                    let ext = loaded.extension.blocking_read();
                    ext.metadata().clone()
                };

                // Register the extension (blocking)
                let id = metadata.id.clone();
                if let Err(e) = self.blocking_register(id, loaded.extension) {
                    return Err(e);
                }

                Ok(())
            }
            Some("wasm") => {
                // For WASM, we need async context for loading
                // This is a limitation - WASM loading requires async
                // Return an error indicating this
                return Err(ExtensionError::LoadFailed(
                    "WASM extensions cannot be loaded in blocking mode. Use load_from_path() instead.".to_string()
                ));
            }
            _ => Err(ExtensionError::InvalidFormat(format!(
                "Unsupported extension format: {:?}",
                path
            ))),
        }
    }

    /// Discover extensions in configured directories.
    ///
    /// Returns a list of (path, metadata) tuples for discovered extensions.
    pub async fn discover(&self) -> Vec<(PathBuf, ExtensionMetadata)> {
        let mut discovered = Vec::new();

        tracing::debug!(
            "Extension discover: starting, dirs: {:?}",
            self.extension_dirs
        );

        for dir in &self.extension_dirs {
            if !dir.exists() {
                tracing::debug!("Extension discover: directory does not exist: {:?}", dir);
                continue;
            }

            tracing::debug!("Extension discover: scanning directory: {:?}", dir);

            // Use the loader's discover method
            let native_found = self.native_loader.discover(dir).await;
            tracing::debug!(
                "Extension discover: found {} native extensions",
                native_found.len()
            );
            for (path, metadata) in native_found {
                discovered.push((path, metadata));
            }

            // Discover WASM extensions
            let wasm_found = self.wasm_loader.discover(dir).await;
            tracing::debug!(
                "Extension discover: found {} wasm extensions",
                wasm_found.len()
            );
            for (path, metadata) in wasm_found {
                discovered.push((path, metadata));
            }
        }

        tracing::debug!(
            "Extension discover: complete, total found: {}",
            discovered.len()
        );
        discovered
    }

    /// Execute a command on an extension.
    pub async fn execute_command(
        &self,
        id: &str,
        command: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let ext = self
            .get(id)
            .await
            .ok_or_else(|| ExtensionError::NotFound(id.to_string()))?;

        let ext = ext.read().await;
        ext.execute_command(command, args).await
    }

    /// Perform health check on an extension.
    pub async fn health_check(&self, id: &str) -> Result<bool> {
        let ext = self
            .get(id)
            .await
            .ok_or_else(|| ExtensionError::NotFound(id.to_string()))?;

        let ext = ext.read().await;
        ext.health_check().await
    }

    /// Check if an extension is registered.
    pub async fn contains(&self, id: &str) -> bool {
        self.extensions.read().await.contains_key(id)
    }

    /// Get the number of registered extensions.
    pub async fn count(&self) -> usize {
        self.extensions.read().await.len()
    }

    /// Get all registered extensions (alias for get_extensions() from trait).
    pub async fn get_all(&self) -> Vec<DynExtension> {
        self.extensions.read().await.values().cloned().collect()
    }

    /// Get the safety manager for this registry.
    pub fn safety_manager(&self) -> Arc<ExtensionSafetyManager> {
        Arc::clone(&self.safety_manager)
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for registries that manage extensions.
#[async_trait::async_trait]
pub trait ExtensionRegistryTrait: Send + Sync {
    /// Get all registered extensions.
    async fn get_extensions(&self) -> Vec<DynExtension>;

    /// Get a specific extension by ID.
    async fn get_extension(&self, id: &str) -> Option<DynExtension>;

    /// Execute a command on an extension.
    async fn execute_command(
        &self,
        extension_id: &str,
        command: &str,
        args: &serde_json::Value,
    ) -> std::result::Result<serde_json::Value, String>;

    /// Get metrics from an extension.
    async fn get_metrics(&self, extension_id: &str) -> Vec<super::system::MetricDescriptor>;
}

#[async_trait::async_trait]
impl ExtensionRegistryTrait for ExtensionRegistry {
    async fn get_extensions(&self) -> Vec<DynExtension> {
        self.extensions.read().await.values().cloned().collect()
    }

    async fn get_extension(&self, id: &str) -> Option<DynExtension> {
        self.get(id).await
    }

    async fn execute_command(
        &self,
        extension_id: &str,
        command: &str,
        args: &serde_json::Value,
    ) -> std::result::Result<serde_json::Value, String> {
        self.execute_command(extension_id, command, args)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_metrics(&self, extension_id: &str) -> Vec<super::system::MetricDescriptor> {
        if let Some(ext) = self.get(extension_id).await {
            let ext = ext.read().await;
            ext.metrics().to_vec()
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = ExtensionRegistry::new();
        assert_eq!(registry.count().await, 0);
    }
}
