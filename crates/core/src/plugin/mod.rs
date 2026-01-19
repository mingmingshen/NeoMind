//! Plugin system for NeoTalk.
//!
//! This module defines the plugin interface and registry for extending
//! the NeoTalk platform with custom implementations at compile time or
//! runtime (via WASM or native libraries).
//!
//! ## Plugin Types
//!
//! - **LLM Backends**: Custom LLM implementations
//! - **Storage Backends**: Custom storage implementations
//! - **Device Adapters**: Custom device/protocol adapters
//! - **Tools**: Custom function calling tools
//! - **Alert Channels**: Custom notification channels
//!
//! ## Dynamic Loading
//!
//! - **WASM Plugins**: WebAssembly modules for secure sandboxed execution
//! - **Native Plugins**: Dynamic libraries (.so, .dylib, .dll) via libloading
//! - **Config Hot Reload**: File watching for configuration changes

use serde_json::Value;
use std::sync::Arc;

// Core plugin types and traits
pub mod dynamic;
pub mod native;
pub mod registry;
pub mod types;
pub mod wasm;
pub mod watcher;

// Re-exports
pub use dynamic::{
    DynamicPluginLoader, DynamicPluginWrapper, LoadedPlugin, PLUGIN_ABI_VERSION,
    PluginCapabilities, PluginDescriptor, SecurityContext,
};
pub use native::{LoadedNativePlugin, NativePluginLoader, NativePluginWrapper};
pub use registry::{PluginInfo, PluginInstance, PluginLoadOptions, UnifiedPluginRegistry};
pub use types::{
    DynUnifiedPlugin, ExtendedPluginMetadata, PluginDependency, PluginPermission,
    PluginRegistryEvent, PluginState, PluginStats, PluginType, ResourceLimits, StateMachine,
    UnifiedPlugin,
};
pub use wasm::{LoadedWasmPlugin, ValidationResult, WasmPlugin, WasmPluginLoader};
pub use watcher::{ConfigChangeCallback, ConfigReloadManager, ConfigWatcher, HotConfig};

/// Result type for plugin operations.
pub type Result<T> = std::result::Result<T, PluginError>;

/// Plugin error types.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// Plugin not found.
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// Plugin failed to initialize.
    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    /// Plugin execution failed.
    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),

    /// Invalid plugin configuration.
    #[error("Invalid plugin configuration: {0}")]
    InvalidConfiguration(String),

    /// Plugin version mismatch.
    #[error("Plugin version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },

    /// WASM loading failed.
    #[error("WASM loading failed: {0}")]
    WasmLoadFailed(String),

    /// Invalid plugin file.
    #[error("Invalid plugin: {0}")]
    InvalidPlugin(String),

    /// Failed to load plugin library.
    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Unsupported platform for native plugins.
    #[error("Unsupported platform for native plugins")]
    UnsupportedPlatform,

    /// Security violation detected.
    #[error("Security violation: {0}")]
    SecurityViolation(String),

    /// Other error.
    #[error("Plugin error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Plugin metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin identifier.
    pub id: String,

    /// Plugin name.
    pub name: String,

    /// Plugin version.
    pub version: String,

    /// Required NeoTalk version (semver).
    pub required_neotalk_version: String,

    /// Plugin description.
    pub description: String,

    /// Plugin author.
    pub author: Option<String>,

    /// Plugin types (llm, storage, device, tool, alert, etc.).
    pub types: Vec<String>,

    /// Additional metadata.
    #[serde(flatten)]
    pub extra: Value,
}

impl PluginMetadata {
    /// Create a new plugin metadata.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        required_neotalk_version: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            required_neotalk_version: required_neotalk_version.into(),
            description: String::new(),
            author: None,
            types: Vec::new(),
            extra: Value::Object(Default::default()),
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the author.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Add a plugin type.
    pub fn with_type(mut self, plugin_type: impl Into<String>) -> Self {
        self.types.push(plugin_type.into());
        self
    }
}

/// Plugin trait for compile-time registered plugins.
///
/// Plugins can implement this trait to provide custom functionality
/// such as LLM backends, storage backends, device adapters, etc.
pub trait Plugin: Send + Sync {
    /// Get the plugin metadata.
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin with the given configuration.
    fn initialize(&mut self, config: &Value) -> Result<()>;

    /// Check if the plugin is initialized.
    fn is_initialized(&self) -> bool;

    /// Shutdown the plugin and clean up resources.
    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Dynamic plugin wrapper for trait objects.
pub type DynPlugin = Arc<std::sync::Mutex<dyn Plugin>>;

/// Plugin registry for managing plugins.
///
/// The registry supports both compile-time registered plugins
/// and runtime-loaded WASM plugins (future feature).
pub struct PluginRegistry {
    /// Registered plugins by ID.
    plugins: std::collections::HashMap<String, DynPlugin>,

    /// NeoTalk version for compatibility checking.
    neotalk_version: String,
}

impl PluginRegistry {
    /// Create a new plugin registry.
    pub fn new(neotalk_version: impl Into<String>) -> Self {
        Self {
            plugins: std::collections::HashMap::new(),
            neotalk_version: neotalk_version.into(),
        }
    }

    /// Register a plugin.
    ///
    /// # Example
    /// ```no_run
    /// use edge_ai_core::plugin::{PluginRegistry, Plugin, PluginMetadata};
    /// use std::sync::Arc;
    ///
    /// struct MyPlugin;
    ///
    /// impl Plugin for MyPlugin {
    ///     fn metadata(&self) -> &PluginMetadata {
    ///         // Return metadata
    ///         &metadata
    ///     }
    ///
    ///     fn initialize(&mut self, config: &serde_json::Value) -> edge_ai_core::plugin::Result<()> {
    ///         // Initialize
    ///         Ok(())
    ///     }
    ///
    ///     fn is_initialized(&self) -> bool {
    ///         true
    ///     }
    /// }
    /// ```
    pub fn register(&mut self, plugin: DynPlugin) -> Result<()> {
        let metadata = plugin.lock().unwrap().metadata().clone();

        // Check version compatibility
        if !self.check_version_compatibility(&metadata)? {
            return Err(PluginError::VersionMismatch {
                expected: self.neotalk_version.clone(),
                found: metadata.version,
            });
        }

        self.plugins.insert(metadata.id.clone(), plugin);
        Ok(())
    }

    /// Get a plugin by ID.
    pub fn get(&self, id: &str) -> Option<DynPlugin> {
        self.plugins.get(id).cloned()
    }

    /// List all registered plugin IDs.
    pub fn list(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Unregister a plugin.
    pub fn unregister(&mut self, id: &str) -> Result<()> {
        self.plugins
            .remove(id)
            .ok_or_else(|| PluginError::NotFound(id.to_string()))?;
        Ok(())
    }

    /// Get plugins by type.
    pub fn get_by_type(&self, plugin_type: &str) -> Vec<DynPlugin> {
        self.plugins
            .values()
            .filter(|p| {
                let guard = p.lock().unwrap();
                guard.metadata().types.contains(&plugin_type.to_string())
            })
            .cloned()
            .collect()
    }

    /// Shutdown all plugins.
    pub fn shutdown_all(&mut self) -> Result<()> {
        for plugin in self.plugins.values_mut() {
            let mut guard = plugin.lock().unwrap();
            guard.shutdown()?;
        }
        self.plugins.clear();
        Ok(())
    }

    /// Check version compatibility.
    fn check_version_compatibility(&self, metadata: &PluginMetadata) -> Result<bool> {
        // Simple version check - in production, use semver crate
        // For now, just check if major version matches
        let required = &metadata.required_neotalk_version;
        let current = &self.neotalk_version;

        if required == "*" || required == "any" {
            return Ok(true);
        }

        // Check if current version satisfies requirement
        // This is a simplified check - use semver crate in production
        if let Some(req_ver) = required.strip_prefix('^') {
            // Caret requirement - same logic as Cargo
            Ok(current.starts_with(req_ver))
        } else if let Some(req_ver) = required.strip_prefix('~') {
            // Tilde requirement
            Ok(current.starts_with(req_ver))
        } else if required.contains('=') {
            // Exact version
            Ok(current == required)
        } else {
            // Default to compatible
            Ok(current.starts_with(required))
        }
    }

    /// Load a WASM plugin from a file path.
    ///
    /// This method:
    /// 1. Validates and loads the WASM file
    /// 2. Extracts metadata from sidecar JSON or WASM custom section
    /// 3. Creates a WASM plugin wrapper
    /// 4. Registers the plugin in the registry
    ///
    /// # Arguments
    ///
    /// * `wasm_path` - Path to the `.wasm` file (or directory containing plugin)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use edge_ai_core::plugin::PluginRegistry;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut registry = PluginRegistry::new("1.0.0");
    /// registry.load_wasm_plugin("./plugins/my-plugin.wasm").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_wasm_plugin(&mut self, wasm_path: impl AsRef<std::path::Path>) -> Result<()> {
        use std::sync::Mutex as StdMutex;

        let path = wasm_path.as_ref();

        // If path is a directory, look for .wasm file inside
        let wasm_file = if path.is_dir() {
            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| PluginError::InvalidPlugin("Invalid directory path".into()))?;

            let wasm_path = path.join(format!("{}.wasm", dir_name));
            if !wasm_path.exists() {
                // Try to find any .wasm file in the directory
                let entries = std::fs::read_dir(path)
                    .map_err(|e| PluginError::LoadFailed(format!("Cannot read directory: {}", e)))?;

                let wasm_entry = entries
                    .filter_map(|e| e.ok())
                    .find(|e| e.path().extension().is_some_and(|ext| ext == "wasm"));

                let wasm_entry = wasm_entry.ok_or_else(|| {
                    PluginError::LoadFailed(format!(
                        "No .wasm file found in directory: {}",
                        path.display()
                    ))
                })?;

                wasm_entry.path()
            } else {
                wasm_path
            }
        } else {
            path.to_path_buf()
        };

        tracing::info!("Loading WASM plugin from: {}", wasm_file.display());

        // Use the WasmPluginLoader to load and validate the plugin
        let loader = wasm::WasmPluginLoader::new();

        // First validate the plugin
        let validation = loader.validate_plugin(&wasm_file);
        if !validation.is_valid {
            let error_msg = loader.format_load_error(&wasm_file, &PluginError::InvalidPlugin(
                validation.errors.join("; ")
            ));
            tracing::error!("{}", error_msg);
            return Err(PluginError::WasmLoadFailed(format!(
                "Validation failed: {}",
                validation.errors.join("; ")
            )));
        }

        // Load the plugin metadata
        let loaded = loader
            .load_metadata(&wasm_file)
            .map_err(|e| PluginError::WasmLoadFailed(e.to_string()))?;

        // Instantiate the plugin
        let wasm_plugin = loader
            .instantiate(loaded)
            .await
            .map_err(|e| PluginError::InitializationFailed(e.to_string()))?;

        // Create a wrapper that implements the Plugin trait
        let plugin_id = wasm_plugin.metadata().base.id.clone();
        let wrapper = WasmPluginWrapper::new(wasm_plugin);

        // Register the plugin
        self.plugins
            .insert(plugin_id.clone(), Arc::new(StdMutex::new(wrapper)));

        tracing::info!("Successfully loaded WASM plugin: {}", plugin_id);

        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new(env!("CARGO_PKG_VERSION"))
    }
}

/// Wrapper that adapts `WasmPlugin` to implement the `Plugin` trait.
///
/// This allows WASM plugins to be registered in the `PluginRegistry`
/// alongside native Rust plugins.
pub struct WasmPluginWrapper {
    /// The wrapped WASM plugin
    plugin: Arc<tokio::sync::RwLock<wasm::WasmPlugin>>,
    /// Cached metadata for quick access
    metadata: PluginMetadata,
    /// Whether the plugin has been initialized
    initialized: bool,
}

impl WasmPluginWrapper {
    /// Create a new wrapper around a WASM plugin.
    pub fn new(plugin: wasm::WasmPlugin) -> Self {
        // Extract the metadata we need before moving plugin
        let plugin_id = plugin.metadata().base.id.clone();
        let plugin_name = plugin.metadata().base.name.clone();
        let plugin_version = plugin.metadata().base.version.clone();
        let plugin_required_version = plugin.metadata().base.required_neotalk_version.clone();
        let plugin_description = plugin.metadata().base.description.clone();
        let plugin_author = plugin.metadata().base.author.clone();
        let plugin_types = plugin.metadata().base.types.clone();

        let metadata = PluginMetadata::new(
            &plugin_id,
            &plugin_name,
            &plugin_version,
            &plugin_required_version,
        )
        .with_description(&plugin_description)
        .with_author(plugin_author.unwrap_or_else(|| "Unknown".to_string()));

        // Create wrapper with collected types
        let mut wrapper = Self {
            plugin: Arc::new(tokio::sync::RwLock::new(plugin)),
            metadata,
            initialized: false,
        };

        // Add all plugin types from the extended metadata
        for plugin_type_str in &plugin_types {
            wrapper.metadata.types.push(plugin_type_str.clone());
        }

        wrapper
    }

    /// Get a reference to the underlying WASM plugin.
    pub async fn plugin(&self) -> Arc<tokio::sync::RwLock<wasm::WasmPlugin>> {
        self.plugin.clone()
    }
}

impl Plugin for WasmPluginWrapper {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self, config: &Value) -> Result<()> {
        // For async initialization, we need to use a runtime
        let rt = tokio::runtime::Handle::try_current();
        let plugin = self.plugin.clone();

        let init_result = if let Ok(handle) = rt {
            handle.block_on(async move {
                let p = plugin.read().await;
                p.initialize(config).await
            })
        } else {
            // Create a new runtime if none exists
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PluginError::InitializationFailed(format!("Failed to create runtime: {}", e)))?;
            rt.block_on(async move {
                let p = plugin.read().await;
                p.initialize(config).await
            })
        };

        match init_result {
            Ok(()) => {
                self.initialized = true;
                Ok(())
            }
            Err(e) => Err(PluginError::InitializationFailed(format!(
                "WASM plugin initialization failed: {}",
                e
            ))),
        }
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn shutdown(&mut self) -> Result<()> {
        let rt = tokio::runtime::Handle::try_current();
        let plugin = self.plugin.clone();

        let shutdown_result = if let Ok(handle) = rt {
            handle.block_on(async move {
                let p = plugin.read().await;
                p.shutdown().await
            })
        } else {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PluginError::InitializationFailed(format!("Failed to create runtime: {}", e)))?;
            rt.block_on(async move {
                let p = plugin.read().await;
                p.shutdown().await
            })
        };

        shutdown_result.map_err(|e| PluginError::ExecutionFailed(format!("Shutdown failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockPlugin {
        metadata: PluginMetadata,
        initialized: bool,
    }

    impl MockPlugin {
        fn new(id: &str) -> Self {
            Self {
                metadata: PluginMetadata::new(id, "Mock Plugin", "1.0.0", "*")
                    .with_description("A mock plugin for testing")
                    .with_type("test"),
                initialized: false,
            }
        }
    }

    impl Plugin for MockPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        fn initialize(&mut self, _config: &Value) -> Result<()> {
            self.initialized = true;
            Ok(())
        }

        fn is_initialized(&self) -> bool {
            self.initialized
        }
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new("1.0.0");

        let plugin = Arc::new(std::sync::Mutex::new(MockPlugin::new("test-plugin")));
        registry.register(plugin).unwrap();

        assert_eq!(registry.list().len(), 1);
        assert!(registry.get("test-plugin").is_some());
    }

    #[test]
    fn test_plugin_metadata() {
        let metadata = PluginMetadata::new("test", "Test Plugin", "1.0.0", "1.0.0")
            .with_description("A test plugin")
            .with_author("Test Author")
            .with_type("llm");

        assert_eq!(metadata.id, "test");
        assert_eq!(metadata.name, "Test Plugin");
        assert_eq!(metadata.description, "A test plugin");
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert!(metadata.types.contains(&"llm".to_string()));
    }
}
