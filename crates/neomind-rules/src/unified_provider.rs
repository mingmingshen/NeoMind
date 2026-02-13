//! Unified value provider for rule engine.
//!
//! This module provides a value provider that supports:
//! - Device metrics (via device storage)
//! - Extension metrics (via extension storage)
//! - Transform outputs (future)
//!
//! Uses the unified DataSourceId format for all data sources.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::engine::ValueProvider;
use neomind_core::datasource::{DataSourceId, DataSourceType};

/// Cache entry for metric values.
#[derive(Debug, Clone)]
struct CacheEntry {
    value: f64,
    timestamp: i64,
    ttl_ms: u64,
}

impl CacheEntry {
    fn new(value: f64, ttl_ms: u64) -> Self {
        Self {
            value,
            timestamp: chrono::Utc::now().timestamp_millis(),
            ttl_ms,
        }
    }

    fn is_expired(&self) -> bool {
        if self.ttl_ms == 0 {
            return false; // Never expires
        }
        let now = chrono::Utc::now().timestamp_millis();
        now - self.timestamp > self.ttl_ms as i64
    }
}

/// Unified value provider for rule engine.
///
/// Supports querying metrics from:
/// - Devices: `device:sensor1:temperature`
/// - Extensions: `extension:weather:temperature`
/// - Extension commands: `extension:weather:get_current_weather.temperature_c`
pub struct UnifiedValueProvider {
    /// Cached metric values
    /// Key: (source_type, source_id, metric) -> value
    cache: Arc<RwLock<HashMap<(String, String, String), CacheEntry>>>,
    /// Default TTL for cached values (milliseconds)
    default_ttl_ms: u64,
    /// Optional device storage for querying current values
    device_storage: Arc<RwLock<Option<Arc<dyn DeviceStorageLike>>>>,
    /// Optional extension storage for querying current values
    extension_storage: Arc<RwLock<Option<Arc<dyn ExtensionStorageLike>>>>,
}

/// Trait for device storage abstraction.
#[async_trait::async_trait]
pub trait DeviceStorageLike: Send + Sync {
    async fn query_latest(&self, device_id: &str, metric: &str) -> Option<f64>;
}

/// Trait for extension storage abstraction.
#[async_trait::async_trait]
pub trait ExtensionStorageLike: Send + Sync {
    async fn query_latest(&self, extension_id: &str, metric: &str) -> Option<f64>;
}

impl UnifiedValueProvider {
    /// Create a new unified value provider.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl_ms: 5000, // 5 seconds default TTL
            device_storage: Arc::new(RwLock::new(None)),
            extension_storage: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the default TTL for cached values.
    pub fn with_ttl(mut self, ttl_ms: u64) -> Self {
        self.default_ttl_ms = ttl_ms;
        self
    }

    /// Set the device storage (async setter).
    pub async fn set_device_storage(&self, storage: Arc<dyn DeviceStorageLike>) {
        let mut ds = self.device_storage.write().await;
        *ds = Some(storage);
    }

    /// Set the extension storage (async setter).
    pub async fn set_extension_storage(&self, storage: Arc<dyn ExtensionStorageLike>) {
        let mut es = self.extension_storage.write().await;
        *es = Some(storage);
    }

    /// Set both storages (async setter).
    pub async fn set_storages(
        &self,
        device_storage: Option<Arc<dyn DeviceStorageLike>>,
        extension_storage: Option<Arc<dyn ExtensionStorageLike>>,
    ) {
        if let Some(ds) = device_storage {
            let mut storage = self.device_storage.write().await;
            *storage = Some(ds);
        }
        if let Some(es) = extension_storage {
            let mut storage = self.extension_storage.write().await;
            *storage = Some(es);
        }
    }

    /// Update a cached metric value.
    pub async fn update_value(&self, source_type: &str, source_id: &str, metric: &str, value: f64) {
        self.update_value_with_ttl(source_type, source_id, metric, value, self.default_ttl_ms)
            .await;
    }

    /// Update a cached metric value with custom TTL.
    pub async fn update_value_with_ttl(
        &self,
        source_type: &str,
        source_id: &str,
        metric: &str,
        value: f64,
        ttl_ms: u64,
    ) {
        let mut cache = self.cache.write().await;
        cache.insert(
            (
                source_type.to_string(),
                source_id.to_string(),
                metric.to_string(),
            ),
            CacheEntry::new(value, ttl_ms),
        );
    }

    /// Parse and update from DataSourceId.
    pub async fn update_from_data_source_id(&self, data_source_id: &DataSourceId, value: f64) {
        let source_type = match data_source_id.source_type {
            DataSourceType::Device => "device",
            DataSourceType::Extension => "extension",
            DataSourceType::Transform => "transform",
        };
        self.update_value(
            source_type,
            &data_source_id.source_id,
            &data_source_id.field_path,
            value,
        )
        .await;
    }

    /// Update a device metric value (convenience method).
    pub async fn update_device_value(&self, device_id: &str, metric: &str, value: f64) {
        self.update_value("device", device_id, metric, value).await;
    }

    /// Update an extension metric value (convenience method).
    pub async fn update_extension_value(&self, extension_id: &str, metric: &str, value: f64) {
        self.update_value("extension", extension_id, metric, value)
            .await;
    }

    /// Update an extension command output value (convenience method).
    pub async fn update_extension_command_value(
        &self,
        extension_id: &str,
        command: &str,
        field: &str,
        value: f64,
    ) {
        let metric = format!("{}.{}", command, field);
        self.update_value("extension", extension_id, &metric, value)
            .await;
    }

    /// Get a value from storage (bypassing cache).
    #[allow(dead_code)]
    async fn fetch_from_storage(
        &self,
        source_type: &str,
        source_id: &str,
        metric: &str,
    ) -> Option<f64> {
        match source_type {
            "device" => {
                let storage = self.device_storage.read().await;
                if let Some(s) = storage.as_ref() {
                    s.query_latest(source_id, metric).await
                } else {
                    None
                }
            }
            "extension" => {
                let storage = self.extension_storage.read().await;
                if let Some(s) = storage.as_ref() {
                    s.query_latest(source_id, metric).await
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Clear expired cache entries.
    pub async fn clear_expired(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
    }

    /// Clear all cached values.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get all cached values for a source.
    pub async fn get_source_values(
        &self,
        source_type: &str,
        source_id: &str,
    ) -> HashMap<String, f64> {
        let cache = self.cache.read().await;
        cache
            .iter()
            .filter(|((t, id, _), _)| t == source_type && id == source_id)
            .filter(|(_, entry)| !entry.is_expired())
            .map(|((_, _, m), entry)| (m.clone(), entry.value))
            .collect()
    }

    /// Get all device values.
    pub async fn get_device_values(&self, device_id: &str) -> HashMap<String, f64> {
        self.get_source_values("device", device_id).await
    }

    /// Get all extension values.
    pub async fn get_extension_values(&self, extension_id: &str) -> HashMap<String, f64> {
        self.get_source_values("extension", extension_id).await
    }

    /// Get cache statistics.
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|e| e.is_expired()).count();
        CacheStats {
            total_entries: total,
            expired_entries: expired,
            active_entries: total - expired,
        }
    }
}

impl Default for UnifiedValueProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ValueProvider for UnifiedValueProvider {
    fn get_value(&self, source_id: &str, metric: &str) -> Option<f64> {
        // Try to determine the source type from source_id
        // Format could be:
        // - "device_id" (device, assumed)
        // - "extension:extension_id" (extension with prefix)
        // - "transform:transform_id" (transform with prefix)

        let (source_type, actual_id) = if let Some(rest) = source_id.strip_prefix("extension:") {
            ("extension", rest)
        } else if let Some(rest) = source_id.strip_prefix("transform:") {
            ("transform", rest)
        } else {
            // Default to device for backward compatibility
            ("device", source_id)
        };

        // Try cache first (synchronous)
        if let Ok(cache) = self.cache.try_read() {
            let key = (
                source_type.to_string(),
                actual_id.to_string(),
                metric.to_string(),
            );
            if let Some(entry) = cache.get(&key) {
                if !entry.is_expired() {
                    return Some(entry.value);
                }
            }
        }

        // Cache miss or expired - return None
        // The caller should ensure the value is pre-cached via update methods
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Cache statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
}

// ============================================================================
// Storage Implementations
// ============================================================================

/// Device storage adapter for TimeSeriesStorage.
pub struct TimeSeriesStorageAdapter {
    storage: Arc<neomind_devices::TimeSeriesStorage>,
}

impl TimeSeriesStorageAdapter {
    pub fn new(storage: Arc<neomind_devices::TimeSeriesStorage>) -> Self {
        Self { storage }
    }
}

#[async_trait::async_trait]
impl DeviceStorageLike for TimeSeriesStorageAdapter {
    async fn query_latest(&self, device_id: &str, metric: &str) -> Option<f64> {
        match self.storage.latest(device_id, metric).await {
            Ok(Some(dp)) => match &dp.value {
                neomind_devices::MetricValue::Float(f) => Some(*f),
                neomind_devices::MetricValue::Integer(i) => Some(*i as f64),
                neomind_devices::MetricValue::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
                _ => None,
            },
            Ok(None) => None,
            Err(_) => None,
        }
    }
}

// ExtensionMetricsStorageAdapter is defined in neomind-api to avoid circular dependency
// This is a placeholder type reference for documentation purposes
pub type ExtensionMetricsStorageAdapter = Arc<dyn ExtensionStorageLike>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new(42.0, 100);
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_unified_value_provider_get_value() {
        let provider = UnifiedValueProvider::new();

        // Initially no values
        assert_eq!(provider.get_value("sensor1", "temperature"), None);

        // Update and check
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            provider
                .update_device_value("sensor1", "temperature", 25.5)
                .await;
        });

        assert_eq!(provider.get_value("sensor1", "temperature"), Some(25.5));
    }

    #[test]
    fn test_unified_value_provider_extension() {
        let provider = UnifiedValueProvider::new();

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            provider
                .update_extension_value("weather", "temperature", 30.0)
                .await;
        });

        // Query with extension: prefix
        assert_eq!(
            provider.get_value("extension:weather", "temperature"),
            Some(30.0)
        );
    }

    #[test]
    fn test_unified_value_provider_extension_command() {
        let provider = UnifiedValueProvider::new();

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            provider
                .update_extension_command_value(
                    "weather",
                    "get_current_weather",
                    "temperature_c",
                    28.5,
                )
                .await;
        });

        // Extension command output is stored as "command.field" metric
        assert_eq!(
            provider.get_value("extension:weather", "get_current_weather.temperature_c"),
            Some(28.5)
        );
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let provider = UnifiedValueProvider::new().with_ttl(100);
        provider.update_device_value("sensor1", "temp", 25.0).await;
        provider
            .update_extension_value("weather", "temp", 30.0)
            .await;

        let stats = provider.cache_stats().await;
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.active_entries, 2);
        assert_eq!(stats.expired_entries, 0);
    }

    #[tokio::test]
    async fn test_get_source_values() {
        let provider = UnifiedValueProvider::new();
        provider.update_device_value("sensor1", "temp", 25.0).await;
        provider
            .update_device_value("sensor1", "humidity", 60.0)
            .await;
        provider
            .update_extension_value("weather", "temp", 30.0)
            .await;

        let device_values = provider.get_device_values("sensor1").await;
        assert_eq!(device_values.len(), 2);
        assert_eq!(device_values.get("temp"), Some(&25.0));
        assert_eq!(device_values.get("humidity"), Some(&60.0));

        let ext_values = provider.get_extension_values("weather").await;
        assert_eq!(ext_values.len(), 1);
        assert_eq!(ext_values.get("temp"), Some(&30.0));
    }

    #[tokio::test]
    async fn test_update_from_data_source_id() {
        let provider = UnifiedValueProvider::new();

        let device_id = DataSourceId::device("sensor1", "temperature");
        provider.update_from_data_source_id(&device_id, 25.5).await;

        assert_eq!(provider.get_value("sensor1", "temperature"), Some(25.5));

        let ext_id = DataSourceId::extension("weather", "temperature");
        provider.update_from_data_source_id(&ext_id, 30.0).await;

        assert_eq!(
            provider.get_value("extension:weather", "temperature"),
            Some(30.0)
        );

        let cmd_id =
            DataSourceId::extension_command("weather", "get_current_weather", "temperature_c");
        provider.update_from_data_source_id(&cmd_id, 28.5).await;

        assert_eq!(
            provider.get_value("extension:weather", "get_current_weather.temperature_c"),
            Some(28.5)
        );
    }
}
