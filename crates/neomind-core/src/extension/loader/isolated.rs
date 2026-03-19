//! Isolated extension loader
//!
//! This loader wraps native extensions in a process-isolated environment.
//! Extensions loaded through this loader run in a separate process and
//! communicate via IPC, ensuring that extension crashes cannot affect
//! the main NeoMind process.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::NativeExtensionLoader;
use crate::extension::isolated::{IsolatedExtension, IsolatedExtensionConfig};
use crate::extension::system::{ExtensionMetadata, ExtensionMetricValue};
use crate::extension::types::{ExtensionError, Result};
use serde::Deserialize;

/// Manifest.json structure for loading metadata
#[derive(Debug, Clone, Deserialize)]
struct ExtensionManifest {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    author: Option<String>,
}

/// Configuration for the isolated loader
#[derive(Debug, Clone)]
pub struct IsolatedLoaderConfig {
    /// Base configuration for isolated extensions
    pub isolated_config: IsolatedExtensionConfig,
    /// Whether to use isolated mode by default
    pub use_isolated_by_default: bool,
    /// Extensions that should always run in isolated mode
    pub force_isolated: Vec<String>,
    /// Extensions that should always run in-process
    pub force_in_process: Vec<String>,
}

impl Default for IsolatedLoaderConfig {
    fn default() -> Self {
        Self {
            isolated_config: IsolatedExtensionConfig::default(),
            // Default to isolated mode for safety - extension crashes won't affect main process
            use_isolated_by_default: true,
            force_isolated: Vec::new(),
            force_in_process: Vec::new(),
        }
    }
}

/// Loader for isolated extensions
pub struct IsolatedExtensionLoader {
    /// Native loader for metadata extraction
    native_loader: NativeExtensionLoader,
    /// Configuration
    config: IsolatedLoaderConfig,
}

impl IsolatedExtensionLoader {
    /// Create a new isolated extension loader
    pub fn new(config: IsolatedLoaderConfig) -> Self {
        Self {
            native_loader: NativeExtensionLoader::new(),
            config,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(IsolatedLoaderConfig::default())
    }

    /// Check if an extension should run in isolated mode
    pub fn should_use_isolated(&self, extension_id: &str) -> bool {
        // Check force lists first
        if self.config.force_isolated.iter().any(|s| s == extension_id) {
            return true;
        }
        if self.config.force_in_process.iter().any(|s| s == extension_id) {
            return false;
        }

        // Use default
        self.config.use_isolated_by_default
    }

    /// Load metadata from manifest.json in the extension directory
    fn load_metadata_from_manifest(path: &Path) -> Option<ExtensionMetadata> {
        // Try to find manifest.json in the extension directory
        let ext_dir = path.parent()?;
        let manifest_path = ext_dir.join("manifest.json");

        if !manifest_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&manifest_path).ok()?;
        let manifest: ExtensionManifest = serde_json::from_str(&content).ok()?;

        let version = semver::Version::parse(&manifest.version)
            .unwrap_or_else(|_| semver::Version::new(0, 1, 0));

        tracing::debug!(
            manifest_path = %manifest_path.display(),
            extension_id = %manifest.id,
            "Loaded metadata from manifest.json"
        );

        Some(ExtensionMetadata {
            id: manifest.id,
            name: manifest.name,
            version,
            description: manifest.description,
            author: manifest.author,
            homepage: None,
            license: None,
            file_path: Some(path.to_path_buf()),
            config_parameters: None,
        })
    }

    /// Load an extension in isolated mode
    pub async fn load_isolated(&self, path: &Path) -> Result<Arc<IsolatedExtension>> {
        // Try to load metadata from manifest.json first (more reliable)
        // Fall back to FFI metadata if manifest not found
        let metadata = if let Some(manifest_meta) = Self::load_metadata_from_manifest(path) {
            tracing::debug!("Using manifest.json metadata for extension ID");
            manifest_meta
        } else {
            tracing::debug!("manifest.json not found, using FFI metadata");
            self.native_loader.load_metadata(path).await?
        };

        tracing::debug!(
            extension_id = %metadata.id,
            path = %path.display(),
            "Loading extension in isolated mode"
        );

        // Create isolated extension wrapper
        let isolated = IsolatedExtension::new(
            &metadata.id,
            path,
            self.config.isolated_config.clone(),
        );

        // Start the extension process
        isolated.start().await.map_err(|e| {
            ExtensionError::LoadFailed(format!("Failed to start isolated extension: {}", e))
        })?;

        Ok(Arc::new(isolated))
    }

    /// Load an extension (decides mode based on configuration)
    pub async fn load(&self, path: &Path) -> Result<LoadedExtension> {
        // Extract metadata first to determine mode
        let metadata = self.native_loader.load_metadata(path).await?;

        if self.should_use_isolated(&metadata.id) {
            let isolated = self.load_isolated(path).await?;
            Ok(LoadedExtension::Isolated(isolated))
        } else {
            // Load in-process using native loader
            let native = self.native_loader.load(path)?;
            Ok(LoadedExtension::Native(native.extension))
        }
    }

    /// Discover extensions in a directory
    pub async fn discover(&self, dir: &Path) -> Vec<(PathBuf, ExtensionMetadata)> {
        // Use native loader for discovery
        self.native_loader.discover(dir).await
    }

    /// Get the configuration
    pub fn config(&self) -> &IsolatedLoaderConfig {
        &self.config
    }
}

/// Result of loading an extension
#[derive(Clone)]
pub enum LoadedExtension {
    /// Extension loaded in-process (native)
    Native(
        /// The extension instance wrapped in Arc<RwLock>
        Arc<tokio::sync::RwLock<Box<dyn crate::extension::system::Extension>>>
    ),
    /// Extension loaded in isolated process
    Isolated(Arc<IsolatedExtension>),
}

impl LoadedExtension {
    /// Get the extension ID
    pub async fn extension_id(&self) -> String {
        match self {
            Self::Native(ext) => {
                let guard = ext.read().await;
                guard.metadata().id.clone()
            }
            Self::Isolated(isolated) => isolated.extension_id(),
        }
    }

    /// Check if this is an isolated extension
    pub fn is_isolated(&self) -> bool {
        matches!(self, Self::Isolated(_))
    }

    /// Get metadata
    pub async fn metadata(&self) -> Option<ExtensionMetadata> {
        match self {
            Self::Native(ext) => {
                let guard = ext.read().await;
                Some(guard.metadata().clone())
            }
            Self::Isolated(isolated) => isolated.metadata().await,
        }
    }

    /// Execute a command
    pub async fn execute_command(
        &self,
        command: &str,
        args: &serde_json::Value,
    ) -> std::result::Result<serde_json::Value, ExtensionError> {
        match self {
            Self::Native(ext) => {
                let guard = ext.read().await;
                guard.execute_command(command, args).await
            }
            Self::Isolated(isolated) => {
                isolated
                    .execute_command(command, args)
                    .await
                    .map_err(|e| ExtensionError::ExecutionFailed(e.to_string()))
            }
        }
    }

    /// Produce metrics
    pub async fn produce_metrics(&self) -> std::result::Result<Vec<ExtensionMetricValue>, ExtensionError> {
        match self {
            Self::Native(ext) => {
                let guard = ext.read().await;
                guard.produce_metrics()
            }
            Self::Isolated(isolated) => {
                isolated
                    .produce_metrics()
                    .await
                    .map_err(|e| ExtensionError::ExecutionFailed(e.to_string()))
            }
        }
    }

    /// Health check
    pub async fn health_check(&self) -> std::result::Result<bool, ExtensionError> {
        match self {
            Self::Native(ext) => {
                let guard = ext.read().await;
                guard.health_check().await
            }
            Self::Isolated(isolated) => {
                isolated
                    .health_check()
                    .await
                    .map_err(|e| ExtensionError::ExecutionFailed(e.to_string()))
            }
        }
    }

    /// Stop the extension (only meaningful for isolated extensions)
    pub async fn stop(&self) -> std::result::Result<(), ExtensionError> {
        match self {
            Self::Native(_) => Ok(()), // No-op for native extensions
            Self::Isolated(isolated) => {
                isolated
                    .stop()
                    .await
                    .map_err(|e| ExtensionError::ExecutionFailed(e.to_string()))
            }
        }
    }

    /// Check if the extension is alive
    pub fn is_alive(&self) -> bool {
        match self {
            Self::Native(_) => true, // Native extensions are always "alive" if loaded
            Self::Isolated(isolated) => isolated.is_alive(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_config_default() {
        let config = IsolatedLoaderConfig::default();
        // Default is now true for process isolation safety
        assert!(config.use_isolated_by_default);
        assert!(config.force_isolated.is_empty());
        assert!(config.force_in_process.is_empty());
    }

    #[test]
    fn test_should_use_isolated() {
        let config = IsolatedLoaderConfig {
            use_isolated_by_default: true,
            force_isolated: vec!["dangerous-ext".to_string()],
            force_in_process: vec!["safe-ext".to_string()],
            ..Default::default()
        };

        let loader = IsolatedExtensionLoader::new(config);

        assert!(loader.should_use_isolated("dangerous-ext"));
        assert!(!loader.should_use_isolated("safe-ext"));
        assert!(loader.should_use_isolated("other-ext"));
    }
}
