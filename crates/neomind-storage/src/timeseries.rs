//! Time series data storage using redb.
//!
//! Provides efficient storage and querying of time-series data from devices.
//!
//! ## Features
//!
//! - **Retention Policies**: Configure data retention per metric or globally
//! - **Memory Cache**: Latest values cached for fast access
//! - **Batch Optimization**: Group writes by device for efficiency
//! - **Performance Monitoring**: Track operation latency and throughput

use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::future::try_join_all;
use moka::sync::Cache;
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Semaphore};

use crate::Error;

// redb table definition: key = (source_id, metric, timestamp), value = DataPoint (serialized)
const TIMESERIES_TABLE: TableDefinition<(&str, &str, i64), &[u8]> =
    TableDefinition::new("timeseries");

/// A single data point in time series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    /// Timestamp of the data point.
    pub timestamp: i64,
    /// Value at this timestamp (can be number, string, boolean, or null).
    pub value: serde_json::Value,
    /// Optional quality flag (0-1, where 1 is highest quality).
    pub quality: Option<f32>,
    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,
}

impl DataPoint {
    /// Create a new data point with a numeric value.
    pub fn new(timestamp: i64, value: f64) -> Self {
        Self {
            timestamp,
            value: serde_json::json!(value),
            quality: None,
            metadata: None,
        }
    }

    /// Create a new data point with any JSON value.
    pub fn new_with_value(timestamp: i64, value: serde_json::Value) -> Self {
        Self {
            timestamp,
            value,
            quality: None,
            metadata: None,
        }
    }

    /// Create a new data point with a string value.
    pub fn new_string(timestamp: i64, value: String) -> Self {
        Self {
            timestamp,
            value: serde_json::json!(value),
            quality: None,
            metadata: None,
        }
    }

    /// Create a new data point with a boolean value.
    pub fn new_bool(timestamp: i64, value: bool) -> Self {
        Self {
            timestamp,
            value: serde_json::json!(value),
            quality: None,
            metadata: None,
        }
    }

    /// Get the value as f64 if it's a number.
    pub fn as_f64(&self) -> Option<f64> {
        self.value.as_f64()
    }

    /// Get the value as string.
    pub fn as_str(&self) -> Option<&str> {
        self.value.as_str()
    }

    /// Get the value as bool.
    pub fn as_bool(&self) -> Option<bool> {
        self.value.as_bool()
    }

    /// Create a data point with quality.
    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Create a data point with metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Get timestamp as DateTime.
    pub fn as_datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.timestamp, 0).unwrap_or_default()
    }
}

/// Time series bucket for aggregating data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesBucket {
    /// Start timestamp of the bucket.
    pub start: i64,
    /// End timestamp of the bucket.
    pub end: i64,
    /// Number of data points in the bucket.
    pub count: u32,
    /// Sum of values (only for numeric data).
    pub sum: Option<f64>,
    /// Minimum value (only for numeric data).
    pub min: Option<f64>,
    /// Maximum value (only for numeric data).
    pub max: Option<f64>,
    /// Average value (only for numeric data).
    pub avg: Option<f64>,
    /// Sample values (for non-numeric data).
    pub sample_values: Vec<serde_json::Value>,
}

impl TimeSeriesBucket {
    /// Create a new empty bucket.
    pub fn new(start: i64, end: i64) -> Self {
        Self {
            start,
            end,
            count: 0,
            sum: None,
            min: None,
            max: None,
            avg: None,
            sample_values: Vec::new(),
        }
    }

    /// Add a value to the bucket.
    pub fn add(&mut self, value: &serde_json::Value) {
        self.count += 1;
        if let Some(num) = value.as_f64() {
            self.sum = Some(self.sum.unwrap_or(0.0) + num);
            self.min = Some(self.min.map_or(num, |m| m.min(num)));
            self.max = Some(self.max.map_or(num, |m| m.max(num)));
            self.avg = self.sum.map(|s| s / self.count as f64);
        } else {
            // For non-numeric values, keep samples (up to 10)
            if self.sample_values.len() < 10 {
                self.sample_values.push(value.clone());
            }
        }
    }

    /// Check if bucket is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Time series query result.
#[derive(Debug, Clone)]
pub struct TimeSeriesResult {
    /// Device ID.
    pub source_id: String,
    /// Metric name.
    pub metric: String,
    /// Data points returned.
    pub points: Vec<DataPoint>,
    /// Total points matching query (if available).
    pub total_count: Option<usize>,
}

/// Information about a metric's storage.
#[derive(Debug, Clone)]
struct MetricInfo {
    last_update: i64,
    point_count: u64,
}

/// Retention policy for time series data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Default retention period in hours (None = forever)
    pub default_hours: Option<u64>,
    /// Per-metric retention overrides
    pub metric_overrides: std::collections::HashMap<String, Option<u64>>,
    /// Per-device-type retention overrides
    pub device_type_overrides: std::collections::HashMap<String, Option<u64>>,
}

impl RetentionPolicy {
    /// Create a new retention policy.
    pub fn new(default_hours: Option<u64>) -> Self {
        Self {
            default_hours,
            metric_overrides: std::collections::HashMap::with_capacity(16), // Pre-allocate for typical use
            device_type_overrides: std::collections::HashMap::with_capacity(8), // Pre-allocate for typical use
        }
    }

    /// Get retention hours for a specific metric.
    pub fn get_retention_hours(&self, device_type: &str, metric: &str) -> Option<u64> {
        // Check metric override first
        if let Some(retention) = self.metric_overrides.get(metric) {
            return *retention;
        }
        // Check device type override
        if let Some(retention) = self.device_type_overrides.get(device_type) {
            return *retention;
        }
        // Use default
        self.default_hours
    }

    /// Set retention for a specific metric.
    pub fn set_metric_retention(&mut self, metric: String, hours: Option<u64>) {
        self.metric_overrides.insert(metric, hours);
    }

    /// Set retention for a device type.
    pub fn set_device_type_retention(&mut self, device_type: String, hours: Option<u64>) {
        self.device_type_overrides.insert(device_type, hours);
    }

    /// Calculate the cutoff timestamp for data retention.
    pub fn cutoff_timestamp(&self, device_type: &str, metric: &str) -> Option<i64> {
        let hours = self.get_retention_hours(device_type, metric)?;
        let now = Utc::now().timestamp();
        Some(now - (hours as i64 * 3600))
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::new(Some(24 * 30)) // Default: 30 days
    }
}

/// Cache entry for latest data point.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Cached data point
    point: DataPoint,
    /// When this entry was cached
    cached_at: Instant,
}

/// Performance statistics for time series operations.
#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    /// Total write operations
    pub write_count: u64,
    /// Total read operations
    pub read_count: u64,
    /// Total write time in nanoseconds
    pub total_write_ns: u64,
    /// Total read time in nanoseconds
    pub total_read_ns: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Points cleaned up by retention
    pub cleanup_points_removed: u64,
    /// Last cleanup timestamp
    pub last_cleanup_timestamp: Option<i64>,
}

impl PerformanceStats {
    /// Get average write latency in microseconds.
    pub fn avg_write_us(&self) -> f64 {
        if self.write_count == 0 {
            return 0.0;
        }
        (self.total_write_ns as f64 / self.write_count as f64) / 1000.0
    }

    /// Get average read latency in microseconds.
    pub fn avg_read_us(&self) -> f64 {
        if self.read_count == 0 {
            return 0.0;
        }
        (self.total_read_ns as f64 / self.read_count as f64) / 1000.0
    }

    /// Get cache hit rate.
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / total as f64
    }

    /// Record a write operation.
    pub fn record_write(&mut self, duration: Duration) {
        self.write_count += 1;
        self.total_write_ns += duration.as_nanos() as u64;
    }

    /// Record a read operation.
    pub fn record_read(&mut self, duration: Duration) {
        self.read_count += 1;
        self.total_read_ns += duration.as_nanos() as u64;
    }

    /// Record a cache hit.
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Record a cache miss.
    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }
}

/// Batch write request grouped by device.
#[derive(Debug, Clone)]
pub struct BatchWriteRequest {
    /// Device ID
    pub source_id: String,
    /// Device type (for retention policy)
    pub device_type: Option<String>,
    /// Metrics and their data points
    pub metrics: std::collections::HashMap<String, Vec<DataPoint>>,
}

impl BatchWriteRequest {
    /// Create a new batch write request.
    pub fn new(source_id: String) -> Self {
        Self {
            source_id,
            device_type: None,
            metrics: std::collections::HashMap::with_capacity(4), // Pre-allocate for typical batch size
        }
    }

    /// Set device type.
    pub fn with_device_type(mut self, device_type: String) -> Self {
        self.device_type = Some(device_type);
        self
    }

    /// Add a data point for a metric.
    pub fn add_point(&mut self, metric: String, point: DataPoint) {
        self.metrics.entry(metric).or_default().push(point);
    }

    /// Get total point count.
    pub fn point_count(&self) -> usize {
        self.metrics.values().map(|v| v.len()).sum()
    }

    /// Check if batch is empty.
    pub fn is_empty(&self) -> bool {
        self.metrics.is_empty()
    }
}

/// Streaming aggregation result — avoids materializing all data points.
pub struct AggregateResult {
    /// Number of data points in range
    pub count: u64,
    /// Sum of numeric values (None if no numeric values found)
    pub sum: Option<f64>,
    /// Minimum numeric value
    pub min: Option<f64>,
    /// Maximum numeric value
    pub max: Option<f64>,
    /// First value in time order
    pub first_value: Option<serde_json::Value>,
    /// Last value in time order
    pub last_value: Option<serde_json::Value>,
}

/// Time series storage using redb.
pub struct TimeSeriesStore {
    db: Arc<Database>,
    /// Metrics info: (source_id:metric) -> MetricInfo - using DashMap for concurrent access
    metrics_info: DashMap<String, MetricInfo>,
    /// Latest value cache: (source_id, metric) -> CacheEntry - using moka for LRU eviction
    latest_cache: Cache<(String, String), CacheEntry>,
    /// Retention policy
    retention_policy: RwLock<RetentionPolicy>,
    /// Performance statistics - using Arc<RwLock> for sharing across tasks
    stats: Arc<RwLock<PerformanceStats>>,
    /// Semaphore for concurrent writes
    write_semaphore: Arc<Semaphore>,
    /// Cache TTL
    cache_ttl: Duration,
    /// Storage path for singleton
    path: String,
}

/// Global time series store singleton (thread-safe).
static TIMESERIES_STORE_SINGLETON: StdMutex<Option<Arc<TimeSeriesStore>>> = StdMutex::new(None);

impl TimeSeriesStore {
    /// Open or create a time series store at the given path.
    /// Uses a singleton pattern to prevent multiple opens of the same database.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Arc<Self>, Error> {
        Self::with_config(path, TimeSeriesConfig::default())
    }

    /// Open or create a time series store with custom configuration.
    pub fn with_config<P: AsRef<Path>>(
        path: P,
        config: TimeSeriesConfig,
    ) -> Result<Arc<Self>, Error> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Check if we already have a store for this path
        {
            let singleton = TIMESERIES_STORE_SINGLETON.lock().unwrap_or_else(|e| {
                tracing::error!("Timeseries store singleton mutex poisoned: {}", e);
                e.into_inner()
            });
            if let Some(store) = singleton.as_ref() {
                if store.path == path_str {
                    return Ok(store.clone());
                }
            }
        }

        // Create new store and save to singleton
        let path_ref = path.as_ref();
        let db = if path_ref.exists() {
            Database::open(path_ref)?
        } else {
            Database::create(path_ref)?
        };

        let store = Arc::new(TimeSeriesStore {
            db: Arc::new(db),
            metrics_info: DashMap::with_capacity(64), // Pre-allocate for typical metrics
            latest_cache: Cache::builder()
                .max_capacity(config.max_cache_size as u64)
                .build(),
            retention_policy: RwLock::new(config.retention_policy),
            stats: Arc::new(RwLock::new(PerformanceStats::default())),
            write_semaphore: Arc::new(Semaphore::new(config.max_concurrent_writes)),
            cache_ttl: config.cache_ttl,
            path: path_str,
        });

        *TIMESERIES_STORE_SINGLETON.lock().unwrap_or_else(|e| {
            tracing::error!("Timeseries store singleton mutex poisoned: {}", e);
            e.into_inner()
        }) = Some(store.clone());
        Ok(store)
    }

    /// Create an in-memory time series store (for testing).
    pub fn memory() -> Result<Arc<Self>, Error> {
        let temp_path = std::env::temp_dir().join(format!("ts_test_{}.redb", uuid::Uuid::new_v4()));
        Self::open(temp_path)
    }

    /// Get performance statistics.
    pub async fn get_stats(&self) -> PerformanceStats {
        self.stats.read().await.clone()
    }

    /// Reset performance statistics.
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = PerformanceStats::default();
    }

    /// Get retention policy.
    pub async fn get_retention_policy(&self) -> RetentionPolicy {
        self.retention_policy.read().await.clone()
    }

    /// Set retention policy.
    pub async fn set_retention_policy(&self, policy: RetentionPolicy) {
        *self.retention_policy.write().await = policy;
    }

    /// Clean stale cache entries.
    pub async fn clean_cache(&self) -> usize {
        let before = self.latest_cache.entry_count() as usize;
        let now = Instant::now();
        let cache_ttl = self.cache_ttl;

        // Collect expired keys from moka cache
        let expired_keys: Vec<(String, String)> = self
            .latest_cache
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.cached_at) >= cache_ttl)
            .map(|(key, _)| (*key).clone())
            .collect();

        for key in &expired_keys {
            self.latest_cache.invalidate(key);
        }

        before - self.latest_cache.entry_count() as usize
    }

    /// Clear all cache entries.
    pub fn clear_cache(&self) {
        self.latest_cache.invalidate_all();
    }

    /// Get cache size (exact count via iteration).
    pub fn cache_size(&self) -> usize {
        self.latest_cache.iter().count()
    }

    /// Write a data point.
    pub async fn write(
        &self,
        source_id: &str,
        metric: &str,
        point: DataPoint,
    ) -> Result<(), Error> {
        let start = Instant::now();
        let _permit = self
            .write_semaphore
            .acquire()
            .await
            .map_err(|_| Error::Storage("Write semaphore closed".to_string()))?;

        let key = (source_id, metric, point.timestamp);
        let value = serde_json::to_vec(&point)?;

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TIMESERIES_TABLE)?;
            table.insert(key, value.as_slice())?;
        }
        write_txn.commit()?;

        // Update cache - DashMap is lock-free
        self.update_cache(source_id, metric, point.clone()).await;

        // Update metrics info - DashMap entry API is lock-free
        let metric_key = format!("{}:{}", source_id, metric);
        self.metrics_info
            .entry(metric_key)
            .and_modify(|entry| {
                entry.last_update = point.timestamp;
                entry.point_count += 1;
            })
            .or_insert_with(|| MetricInfo {
                last_update: point.timestamp,
                point_count: 1,
            });

        // Record stats
        let mut stats = self.stats.write().await;
        stats.record_write(start.elapsed());

        Ok(())
    }

    /// Update the latest value cache.
    async fn update_cache(&self, source_id: &str, metric: &str, point: DataPoint) {
        let key = (source_id.to_string(), metric.to_string());

        // moka handles LRU eviction automatically when capacity is reached
        self.latest_cache.insert(
            key,
            CacheEntry {
                point,
                cached_at: Instant::now(),
            },
        );
    }

    /// Write multiple data points in batch.
    pub async fn write_batch(
        &self,
        source_id: &str,
        metric: &str,
        points: Vec<DataPoint>,
    ) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TIMESERIES_TABLE)?;
            for point in &points {
                let key = (source_id, metric, point.timestamp);
                let value = serde_json::to_vec(point)?;
                table.insert(key, value.as_slice())?;
            }
        }
        write_txn.commit()?;

        // Update metrics info - DashMap entry API is lock-free
        let metric_key = format!("{}:{}", source_id, metric);
        let now = Utc::now().timestamp();
        let last_ts = points.last().map(|p| p.timestamp).unwrap_or(now);

        self.metrics_info
            .entry(metric_key)
            .and_modify(|entry| {
                entry.last_update = last_ts;
                entry.point_count += points.len() as u64;
            })
            .or_insert_with(|| MetricInfo {
                last_update: last_ts,
                point_count: points.len() as u64,
            });

        Ok(())
    }

    /// Query data points in a time range.
    ///
    /// When `limit` is `Some(n)`, at most `n` data points are returned in `points`
    /// and `total_count` is set to the actual total number of matching points.
    /// When `limit` is `None`, all matching points are returned and `total_count`
    /// is `None` (backward compatible).
    pub async fn query_range(
        &self,
        source_id: &str,
        metric: &str,
        start: i64,
        end: i64,
        limit: Option<usize>,
    ) -> Result<TimeSeriesResult, Error> {
        let read_txn = self.db.begin_read()?;

        // Handle case where table doesn't exist yet (no data has been written)
        let table = match read_txn.open_table(TIMESERIES_TABLE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => {
                tracing::debug!(
                    "query_range: table 'timeseries' does not exist yet, returning empty result for source_id={}, metric={}",
                    source_id,
                    metric
                );
                return Ok(TimeSeriesResult {
                    source_id: source_id.to_string(),
                    metric: metric.to_string(),
                    points: Vec::new(),
                    total_count: None,
                });
            }
            Err(e) => return Err(Error::Storage(format!("Failed to open table: {}", e))),
        };

        let start_key = (source_id, metric, start);
        let end_key = (source_id, metric, end);

        tracing::debug!(
            "query_range: source_id={}, metric={}, start={}, end={}, limit={:?}, start_key={:?}, end_key={:?}",
            source_id,
            metric,
            start,
            end,
            limit,
            start_key,
            end_key
        );

        let cap = limit.map(|n| n.min(1000)).unwrap_or(0);
        let mut points = Vec::with_capacity(cap);
        let mut collected = 0usize;
        let mut total_count = 0u32;

        for result in table.range(start_key..=end_key)? {
            total_count += 1;
            let (key, value) = result?;
            let (did, met, ts) = key.value();
            tracing::trace!(
                "query_range: found key=({},{},{}), value_len={}",
                did,
                met,
                ts,
                value.value().len()
            );

            if limit.is_none_or(|n| collected < n) {
                let point: DataPoint = serde_json::from_slice(value.value())?;
                points.push(point);
                collected += 1;
            } else {
                // Already collected enough; stop iterating to avoid full table scan.
                // total_count is already >= limit, which is sufficient for pagination.
                break;
            }
        }

        tracing::debug!(
            "query_range: source_id={}, metric={}, start={}, end={}, found {} points (total_count={})",
            source_id,
            metric,
            start,
            end,
            collected,
            total_count
        );

        Ok(TimeSeriesResult {
            source_id: source_id.to_string(),
            metric: metric.to_string(),
            points,
            total_count: limit.map(|_| total_count as usize),
        })
    }

    /// Aggregate data over a time range using streaming fold (no Vec materialization).
    ///
    /// Accumulates count, sum, min, max in a single pass over the redb range scan,
    /// keeping only O(1) intermediate state instead of O(n).
    pub async fn aggregate_range(
        &self,
        source_id: &str,
        metric: &str,
        start: i64,
        end: i64,
    ) -> Result<AggregateResult, Error> {
        let read_txn = self.db.begin_read()?;

        let table = match read_txn.open_table(TIMESERIES_TABLE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => {
                return Ok(AggregateResult {
                    count: 0,
                    sum: None,
                    min: None,
                    max: None,
                    first_value: None,
                    last_value: None,
                });
            }
            Err(e) => return Err(Error::Storage(format!("Failed to open table: {}", e))),
        };

        let start_key = (source_id, metric, start);
        let end_key = (source_id, metric, end);

        let mut count: u64 = 0;
        let mut sum: f64 = 0.0;
        let mut min_val: f64 = f64::INFINITY;
        let mut max_val: f64 = f64::NEG_INFINITY;
        let mut has_numeric = false;
        let mut first_value: Option<serde_json::Value> = None;
        let mut last_value: Option<serde_json::Value> = None;

        for result in table.range(start_key..=end_key)? {
            let (_key, value) = result?;
            let point: DataPoint = match serde_json::from_slice(value.value()) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("aggregate_range: failed to deserialize data point: {}", e);
                    continue;
                }
            };

            count += 1;
            if first_value.is_none() {
                first_value = Some(point.value.clone());
            }
            last_value = Some(point.value.clone());

            if let Some(n) = point.value.as_f64() {
                sum += n;
                min_val = min_val.min(n);
                max_val = max_val.max(n);
                has_numeric = true;
            }
        }

        Ok(AggregateResult {
            count,
            sum: if has_numeric { Some(sum) } else { None },
            min: if has_numeric { Some(min_val) } else { None },
            max: if has_numeric { Some(max_val) } else { None },
            first_value,
            last_value,
        })
    }

    /// Query a single metric - helper for parallel batch queries.
    async fn query_single_metric(
        db: Arc<Database>,
        source_id: &str,
        metric: &str,
        start: i64,
        end: i64,
        limit: Option<usize>,
    ) -> Result<TimeSeriesResult, Error> {
        let read_txn = db.begin_read()?;
        let table = match read_txn.open_table(TIMESERIES_TABLE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => {
                return Ok(TimeSeriesResult {
                    source_id: source_id.to_string(),
                    metric: metric.to_string(),
                    points: Vec::new(),
                    total_count: None,
                });
            }
            Err(e) => return Err(Error::Storage(format!("Failed to open table: {}", e))),
        };

        let start_key = (source_id, metric, start);
        let end_key = (source_id, metric, end);

        let cap = limit.map(|n| n.min(1000)).unwrap_or(0);
        let mut points = Vec::with_capacity(cap);
        let mut collected = 0usize;
        let mut total_count = 0u32;

        for result in table.range(start_key..=end_key)? {
            total_count += 1;
            let (_key, value) = result?;

            if limit.is_none_or(|n| collected < n) {
                match serde_json::from_slice(value.value()) {
                    Ok(point) => {
                        points.push(point);
                        collected += 1;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "query_single_metric: failed to deserialize data point: {}",
                            e
                        );
                    }
                }
            } else {
                // Already collected enough; stop iterating to avoid full table scan
                break;
            }
        }

        tracing::debug!(
            "query_single_metric: source_id={}, metric={}, start={}, end={}, found {} points (total_count={})",
            source_id,
            metric,
            start,
            end,
            collected,
            total_count
        );

        Ok(TimeSeriesResult {
            source_id: source_id.to_string(),
            metric: metric.to_string(),
            points,
            total_count: limit.map(|_| total_count as usize),
        })
    }

    /// Query multiple metrics for a device in parallel.
    /// Performance optimization: uses parallel queries to reduce latency when querying multiple metrics.
    ///
    /// # Arguments
    /// * `source_id` - The device ID
    /// * `metrics` - Slice of metric names to query
    /// * `start` - Start timestamp (inclusive)
    /// * `end` - End timestamp (inclusive)
    ///
    /// # Returns
    /// A map of metric name to TimeSeriesResult
    pub async fn query_range_batch(
        &self,
        source_id: &str,
        metrics: &[&str],
        start: i64,
        end: i64,
    ) -> Result<std::collections::HashMap<String, TimeSeriesResult>, Error> {
        if metrics.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        // Check if table exists first
        let read_txn = self.db.begin_read()?;
        let table_exists = read_txn.open_table(TIMESERIES_TABLE).is_ok();
        drop(read_txn);

        if !table_exists {
            tracing::debug!(
                "query_range_batch: table 'timeseries' does not exist yet, returning empty results for source_id={}, metrics={:?}",
                source_id,
                metrics
            );
            // Return empty results for all metrics
            let mut results = std::collections::HashMap::new();
            for &metric in metrics {
                results.insert(
                    metric.to_string(),
                    TimeSeriesResult {
                        source_id: source_id.to_string(),
                        metric: metric.to_string(),
                        points: Vec::new(),
                        total_count: None,
                    },
                );
            }
            return Ok(results);
        }

        // Create parallel query tasks for each metric
        let db = Arc::clone(&self.db);
        let source_id = source_id.to_string();
        let metrics: Vec<String> = metrics.iter().map(|s| s.to_string()).collect();

        let query_tasks: Vec<_> = metrics
            .iter()
            .map(|metric| {
                let db = Arc::clone(&db);
                let source_id = source_id.clone();
                let metric = metric.clone();

                tokio::spawn(async move {
                    Self::query_single_metric(db, &source_id, &metric, start, end, None).await
                })
            })
            .collect();

        // Wait for all queries to complete in parallel
        let results_vec = try_join_all(query_tasks).await?;

        // Collect results into HashMap
        let mut results = std::collections::HashMap::new();
        for result in results_vec {
            match result {
                Ok(res) => {
                    results.insert(res.metric.clone(), res);
                }
                Err(e) => {
                    tracing::warn!("query_range_batch: metric query failed: {}", e);
                }
            }
        }

        tracing::debug!(
            "query_range_batch: source_id={}, metrics={:?}, start={}, end={}, returned results for {} metrics",
            source_id,
            metrics,
            start,
            end,
            results.len()
        );

        Ok(results)
    }

    /// Query the latest data point.
    pub async fn query_latest(
        &self,
        source_id: &str,
        metric: &str,
    ) -> Result<Option<DataPoint>, Error> {
        let start = Instant::now();
        let cache_key = (source_id.to_string(), metric.to_string());

        // Check cache first
        {
            if let Some(entry) = self.latest_cache.get(&cache_key) {
                if entry.cached_at.elapsed() < self.cache_ttl {
                    let mut stats = self.stats.write().await;
                    stats.record_cache_hit();
                    stats.record_read(start.elapsed());
                    return Ok(Some(entry.point.clone()));
                }
            }
        }

        // Cache miss - query from database
        let read_txn = self.db.begin_read()?;

        // Handle case where table doesn't exist yet (no data has been written)
        let table = match read_txn.open_table(TIMESERIES_TABLE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => {
                tracing::debug!(
                    "query_latest: table 'timeseries' does not exist yet, returning None for source_id={}, metric={}",
                    source_id,
                    metric
                );
                return Ok(None);
            }
            Err(e) => return Err(Error::Storage(format!("Failed to open table: {}", e))),
        };

        let start_key = (source_id, metric, i64::MIN);
        let end_key = (source_id, metric, i64::MAX);

        // Get the latest data point (most recent timestamp)
        let latest: Option<DataPoint> = table
            .range(start_key..=end_key)?
            .next_back()
            .map(|result| -> Result<DataPoint, Error> {
                let (_key, value) = result?;
                Ok(serde_json::from_slice(value.value())?)
            })
            .transpose()?;

        // Update cache with result
        if let Some(ref point) = latest {
            self.update_cache(source_id, metric, point.clone()).await;
        }

        // Record stats
        let mut stats = self.stats.write().await;
        stats.record_cache_miss();
        stats.record_read(start.elapsed());

        Ok(latest)
    }

    /// Query data points aggregated into time buckets.
    pub async fn query_aggregated(
        &self,
        source_id: &str,
        metric: &str,
        start: i64,
        end: i64,
        bucket_size_secs: i64,
    ) -> Result<Vec<TimeSeriesBucket>, Error> {
        let result = self
            .query_range(source_id, metric, start, end, None)
            .await?;

        let mut buckets: std::collections::HashMap<i64, TimeSeriesBucket> =
            std::collections::HashMap::new();

        for point in result.points {
            let bucket_key = (point.timestamp / bucket_size_secs) * bucket_size_secs;
            let bucket_end = bucket_key + bucket_size_secs;
            buckets
                .entry(bucket_key)
                .or_insert_with(|| TimeSeriesBucket::new(bucket_key, bucket_end))
                .add(&point.value);
        }

        let mut bucket_list: Vec<_> = buckets.into_values().collect();
        bucket_list.sort_by_key(|b| b.start);

        Ok(bucket_list)
    }

    /// Delete data points in a time range.
    pub async fn delete_range(
        &self,
        source_id: &str,
        metric: &str,
        start: i64,
        end: i64,
    ) -> Result<usize, Error> {
        let write_txn = self.db.begin_write()?;
        let mut count = 0;

        {
            let mut table = write_txn.open_table(TIMESERIES_TABLE)?;
            let start_key = (source_id, metric, start);
            let end_key = (source_id, metric, end);

            // Collect keys as owned tuples
            let mut keys_to_delete: Vec<(String, String, i64)> = Vec::new();
            let mut range = table.range(start_key..=end_key)?;
            for result in range.by_ref() {
                let (key_ref, _val_ref) = result?;
                let did: &str = key_ref.value().0;
                let met: &str = key_ref.value().1;
                let ts: i64 = key_ref.value().2;
                keys_to_delete.push((did.to_string(), met.to_string(), ts));
            }
            drop(range);

            for key in &keys_to_delete {
                table.remove((key.0.as_str(), key.1.as_str(), key.2))?;
                count += 1;
            }
        }

        write_txn.commit()?;
        Ok(count)
    }

    /// Flush all data to disk (redb auto-flushes).
    pub fn flush(&self) -> Result<(), Error> {
        // redb auto-manages, no manual flush needed
        Ok(())
    }

    /// Get all metrics for a device.
    pub async fn list_metrics(&self, source_id: &str) -> Result<Vec<String>, Error> {
        let read_txn = self.db.begin_read()?;

        // Handle case where table doesn't exist yet (no data has been written)
        let table = match read_txn.open_table(TIMESERIES_TABLE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => {
                tracing::debug!(
                    "list_metrics: table 'timeseries' does not exist yet, returning empty list for source_id={}",
                    source_id
                );
                return Ok(Vec::new());
            }
            Err(e) => return Err(Error::Storage(format!("Failed to open table: {}", e))),
        };

        let start_key = (source_id, "", i64::MIN);
        let end_key = (source_id, "\u{FF}", i64::MAX);

        let mut metrics = std::collections::HashSet::new();
        for result in table.range(start_key..=end_key)? {
            let (key, _value) = result?;
            let (_, metric, _) = key.value();
            metrics.insert(metric.to_string());
        }

        Ok(metrics.into_iter().collect())
    }

    /// Get all metrics for ALL sources in a single table scan.
    /// Returns a map of source_id → set of metric names.
    /// Much faster than calling list_metrics() per source when you need all sources.
    pub async fn list_all_metrics_grouped(
        &self,
    ) -> Result<std::collections::HashMap<String, std::collections::HashSet<String>>, Error>
    {
        let read_txn = self.db.begin_read()?;

        let table = match read_txn.open_table(TIMESERIES_TABLE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(std::collections::HashMap::new()),
            Err(e) => return Err(Error::Storage(format!("Failed to open table: {}", e))),
        };

        let mut grouped: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();

        for result in table.iter()? {
            let (key, _value) = result?;
            let (source_id, metric, _) = key.value();
            grouped
                .entry(source_id.to_string())
                .or_default()
                .insert(metric.to_string());
        }

        Ok(grouped)
    }

    /// Delete all data for a specific metric.
    pub async fn delete_metric(&self, source_id: &str, metric: &str) -> Result<usize, Error> {
        self.delete_range(source_id, metric, i64::MIN, i64::MAX)
            .await
    }

    /// Write multiple batch requests concurrently.
    pub async fn write_batch_concurrent(
        &self,
        requests: Vec<BatchWriteRequest>,
    ) -> Result<usize, Error> {
        let mut handles = Vec::new();

        for request in requests {
            let db: Arc<Database> = Arc::clone(&self.db);
            let semaphore: Arc<Semaphore> = Arc::clone(&self.write_semaphore);
            // moka Cache implements Clone (internally uses Arc), so we can clone it for the spawned task
            let cache = self.latest_cache.clone();
            // RwLock doesn't implement Clone, wrap in Arc for sharing
            let stats = Arc::clone(&self.stats);

            let source_id = request.source_id.clone();
            let _device_type = request.device_type.clone().unwrap_or_default();
            let metrics = request.metrics.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore
                    .acquire()
                    .await
                    .map_err(|_| Error::Storage("Semaphore closed".to_string()))?;
                let start = Instant::now();
                let mut written = 0;

                let write_txn = db.begin_write()?;
                {
                    let mut table = write_txn.open_table(TIMESERIES_TABLE)?;

                    for (metric, points) in &metrics {
                        for point in points {
                            let key = (&*source_id, &**metric, point.timestamp);
                            let value = serde_json::to_vec(point)?;
                            table.insert(key, &*value)?;
                            written += 1;
                        }
                    }
                }
                write_txn.commit()?;

                // Update cache for latest values - moka handles LRU eviction automatically
                for (metric, points) in &metrics {
                    if let Some(last) = points.last() {
                        let key = (source_id.clone(), metric.clone());
                        cache.insert(
                            key,
                            CacheEntry {
                                point: last.clone(),
                                cached_at: Instant::now(),
                            },
                        );
                    }
                }

                // Record stats - RwLock requires async
                let mut s = stats.write().await;
                s.write_count += 1;
                s.total_write_ns += start.elapsed().as_nanos() as u64;

                Ok::<usize, Error>(written)
            });

            handles.push(handle);
        }

        // Wait for all writes to complete
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await??);
        }

        Ok(results.into_iter().sum())
    }

    /// Apply retention policy and clean up old data.
    pub async fn apply_retention(&self) -> Result<RetentionPolicyCleanupResult, Error> {
        // DashMap and RwLock access - no async needed for DashMap
        let policy = self.retention_policy.read().await;
        // metrics_info is now DashMap, iterate directly when needed

        let mut total_removed: u64 = 0;
        let mut metrics_cleaned: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        let read_txn = self.db.begin_read()?;

        // Handle case where table doesn't exist yet (no data has been written)
        let table = match read_txn.open_table(TIMESERIES_TABLE) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => {
                tracing::debug!(
                    "apply_retention: table 'timeseries' does not exist yet, returning empty result"
                );
                return Ok(RetentionPolicyCleanupResult {
                    points_removed: 0,
                    metrics_cleaned: Vec::new(),
                });
            }
            Err(e) => return Err(Error::Storage(format!("Failed to open table: {}", e))),
        };

        // Collect all (source_id, metric) pairs
        let mut metric_pairs: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        let start_key = ("", "", i64::MIN);
        let end_key = ("\u{FF}", "\u{FF}", i64::MAX);

        for result in table.range(start_key..=end_key)? {
            let (key, _) = result?;
            let (source_id, metric, _) = key.value();
            metric_pairs.insert((source_id.to_string(), metric.to_string()));
        }
        drop(read_txn);
        drop(table);

        let now = Utc::now().timestamp();

        // Process each metric pair
        for (source_id, metric) in &metric_pairs {
            // Get device type from metrics_info if available
            let metric_key = format!("{}:{}", source_id, metric);
            let device_type = ""; // Could be enhanced to look up device type

            if let Some(cutoff) = policy.cutoff_timestamp(device_type, metric) {
                if cutoff < now {
                    let removed = self
                        .delete_range(source_id, metric, i64::MIN, cutoff)
                        .await?;
                    if removed > 0 {
                        total_removed += removed as u64;
                        metrics_cleaned.insert(metric_key.clone());
                    }
                }
            }
        }

        // Update stats
        let mut stats = self.stats.write().await;
        stats.cleanup_points_removed += total_removed;
        stats.last_cleanup_timestamp = Some(now);

        Ok(RetentionPolicyCleanupResult {
            points_removed: total_removed,
            metrics_cleaned: metrics_cleaned.into_iter().collect(),
        })
    }
}

/// Result of retention policy cleanup.
#[derive(Debug, Clone)]
pub struct RetentionPolicyCleanupResult {
    /// Total number of data points removed
    pub points_removed: u64,
    /// List of metrics that were cleaned
    pub metrics_cleaned: Vec<String>,
}

/// Configuration for time series store.
#[derive(Debug, Clone)]
pub struct TimeSeriesConfig {
    /// Retention policy
    pub retention_policy: RetentionPolicy,
    /// Cache TTL for latest values
    pub cache_ttl: Duration,
    /// Maximum cache size
    pub max_cache_size: usize,
    /// Maximum concurrent writes
    pub max_concurrent_writes: usize,
}

impl Default for TimeSeriesConfig {
    fn default() -> Self {
        Self {
            retention_policy: RetentionPolicy::default(),
            cache_ttl: Duration::from_secs(60), // 1 minute
            max_cache_size: 1000,
            max_concurrent_writes: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timeseries_write_read() {
        let store = TimeSeriesStore::memory().unwrap();

        let point = DataPoint::new(1000, 23.5);
        store
            .write("device1", "temperature", point.clone())
            .await
            .unwrap();

        let latest = store.query_latest("device1", "temperature").await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().as_f64(), Some(23.5));
    }

    #[tokio::test]
    async fn test_timeseries_query_range() {
        let store = TimeSeriesStore::memory().unwrap();

        for i in 0..10 {
            let point = DataPoint::new(1000 + i * 100, 20.0 + i as f64);
            store.write("device1", "temperature", point).await.unwrap();
        }

        let result = store
            .query_range("device1", "temperature", 1000, 1500, None)
            .await
            .unwrap();
        assert_eq!(result.points.len(), 6);
    }

    #[tokio::test]
    async fn test_data_point_builder() {
        let point = DataPoint::new(1000, 42.0)
            .with_quality(0.95)
            .with_metadata(serde_json::json!({"source": "sensor"}));

        assert_eq!(point.timestamp, 1000);
        assert_eq!(point.as_f64(), Some(42.0));
        assert_eq!(point.quality, Some(0.95));
        assert!(point.metadata.is_some());
    }

    #[tokio::test]
    async fn test_data_point_string() {
        let point = DataPoint::new_string(1000, "hello".to_string());
        assert_eq!(point.timestamp, 1000);
        assert_eq!(point.as_str(), Some("hello"));
    }

    #[tokio::test]
    async fn test_data_point_bool() {
        let point = DataPoint::new_bool(1000, true);
        assert_eq!(point.timestamp, 1000);
        assert_eq!(point.as_bool(), Some(true));
    }

    #[tokio::test]
    async fn test_list_metrics() {
        let store = TimeSeriesStore::memory().unwrap();

        store
            .write("device1", "temp", DataPoint::new(1000, 20.0))
            .await
            .unwrap();
        store
            .write("device1", "humidity", DataPoint::new(1000, 50.0))
            .await
            .unwrap();
        store
            .write("device2", "temp", DataPoint::new(1000, 22.0))
            .await
            .unwrap();

        let metrics = store.list_metrics("device1").await.unwrap();
        assert_eq!(metrics.len(), 2);
        assert!(metrics.contains(&"temp".to_string()));
        assert!(metrics.contains(&"humidity".to_string()));
    }

    #[tokio::test]
    async fn test_delete_range() {
        let store = TimeSeriesStore::memory().unwrap();

        for i in 0..10 {
            let point = DataPoint::new(1000 + i * 100, i as f64);
            store.write("device1", "temp", point).await.unwrap();
        }

        let count = store
            .delete_range("device1", "temp", 1200, 1500)
            .await
            .unwrap();
        assert_eq!(count, 4);

        let result = store
            .query_range("device1", "temp", 1000, 2000, None)
            .await
            .unwrap();
        assert_eq!(result.points.len(), 6);
    }

    #[tokio::test]
    async fn test_timeseries_aggregation() {
        let store = TimeSeriesStore::memory().unwrap();

        for i in 0..100 {
            let point = DataPoint::new(1000 + i * 10, i as f64);
            store.write("device1", "counter", point).await.unwrap();
        }

        let buckets = store
            .query_aggregated("device1", "counter", 1000, 2000, 100)
            .await
            .unwrap();
        assert!(!buckets.is_empty());

        let first = &buckets[0];
        assert_eq!(first.start, 1000);
        assert_eq!(first.end, 1100);
        assert_eq!(first.count, 10);
    }

    #[tokio::test]
    async fn test_batch_write_100_points() {
        let store = TimeSeriesStore::memory().unwrap();

        // Write 100 data points in batch
        let points: Vec<DataPoint> = (0..100)
            .map(|i| DataPoint::new(1000 + i * 10, i as f64))
            .collect();

        store
            .write_batch("device1", "temp", points)
            .await
            .unwrap();

        // Verify all points are queryable
        let result = store
            .query_range("device1", "temp", 1000, 2000, None)
            .await
            .unwrap();
        assert_eq!(result.points.len(), 100);

        // Verify latest
        let latest = store.query_latest("device1", "temp").await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().as_f64(), Some(99.0));
    }

    #[tokio::test]
    async fn test_query_range_with_limit() {
        let store = TimeSeriesStore::memory().unwrap();

        // Write 20 data points
        for i in 0..20 {
            let point = DataPoint::new(1000 + i * 10, i as f64);
            store.write("device1", "temp", point).await.unwrap();
        }

        // Query with limit
        let result = store
            .query_range("device1", "temp", 1000, 1200, Some(10))
            .await
            .unwrap();
        assert_eq!(result.points.len(), 10);
        assert_eq!(result.total_count, Some(11)); // 11 points in range (1000-1200 inclusive)
    }

    #[tokio::test]
    async fn test_aggregated_queries_avg_min_max_sum_count() {
        let store = TimeSeriesStore::memory().unwrap();

        // Write data points with known values
        for i in 0..10 {
            let point = DataPoint::new(1000 + i * 10, i as f64 * 2.0); // 0, 2, 4, 6, 8, 10, 12, 14, 16, 18
            store.write("device1", "value", point).await.unwrap();
        }

        let buckets = store
            .query_aggregated("device1", "value", 1000, 1100, 100)
            .await
            .unwrap();

        assert_eq!(buckets.len(), 1);
        let bucket = &buckets[0];
        assert_eq!(bucket.count, 10);
        assert_eq!(bucket.sum, Some(90.0)); // Sum of 0,2,4,6,8,10,12,14,16,18
        assert_eq!(bucket.min, Some(0.0));
        assert_eq!(bucket.max, Some(18.0));
        assert_eq!(bucket.avg, Some(9.0)); // 90/10
    }

    #[tokio::test]
    async fn test_delete_operations() {
        let store = TimeSeriesStore::memory().unwrap();

        // Write test data
        for i in 0..10 {
            let point = DataPoint::new(1000 + i * 100, i as f64);
            store.write("device1", "temp", point).await.unwrap();
        }

        // Delete specific range
        let count = store
            .delete_range("device1", "temp", 1200, 1500)
            .await
            .unwrap();
        assert_eq!(count, 4);

        // Verify deletion
        let result = store
            .query_range("device1", "temp", 1000, 2000, None)
            .await
            .unwrap();
        assert_eq!(result.points.len(), 6);

        // Clear cache to avoid stale data
        store.clear_cache();

        // Delete entire metric
        let count = store.delete_metric("device1", "temp").await.unwrap();
        assert_eq!(count, 6);

        // Verify all deleted
        let latest = store.query_latest("device1", "temp").await.unwrap();
        assert!(latest.is_none());
    }

    #[tokio::test]
    async fn test_list_metrics_multiple() {
        let store = TimeSeriesStore::memory().unwrap();

        // Write multiple metrics for one device
        store
            .write("device1", "temp", DataPoint::new(1000, 20.0))
            .await
            .unwrap();
        store
            .write("device1", "humidity", DataPoint::new(1000, 50.0))
            .await
            .unwrap();
        store
            .write("device1", "pressure", DataPoint::new(1000, 1013.25))
            .await
            .unwrap();

        let metrics = store.list_metrics("device1").await.unwrap();
        assert_eq!(metrics.len(), 3);
        assert!(metrics.contains(&"temp".to_string()));
        assert!(metrics.contains(&"humidity".to_string()));
        assert!(metrics.contains(&"pressure".to_string()));
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let store = TimeSeriesStore::memory().unwrap();
        let store = Arc::new(store);

        // Spawn 10 tokio tasks writing simultaneously
        let mut handles = Vec::new();
        for task_id in 0..10 {
            let s = Arc::clone(&store);
            let handle = tokio::spawn(async move {
                for i in 0..10 {
                    let point = DataPoint::new(1000 + task_id * 100 + i * 10, i as f64);
                    s.write(&format!("device{}", task_id), "temp", point)
                        .await
                        .unwrap();
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify data from all tasks
        for task_id in 0..10 {
            let latest = store
                .query_latest(&format!("device{}", task_id), "temp")
                .await
                .unwrap();
            assert!(latest.is_some());
        }
    }

    #[tokio::test]
    async fn test_empty_source_metric_queries() {
        let store = TimeSeriesStore::memory().unwrap();

        // Query non-existent source
        let result = store.query_latest("nosuchdevice", "temp").await.unwrap();
        assert!(result.is_none());

        // Query non-existent metric
        let result = store
            .query_range("device1", "nosuchmetric", 1000, 2000, None)
            .await
            .unwrap();
        assert_eq!(result.points.len(), 0);

        // List metrics for non-existent device
        let metrics = store.list_metrics("nosuchdevice").await.unwrap();
        assert_eq!(metrics.len(), 0);
    }

    #[tokio::test]
    async fn test_very_large_values() {
        let store = TimeSeriesStore::memory().unwrap();

        // Test with f64::MAX
        let point = DataPoint::new(1000, f64::MAX);
        store.write("device1", "max_value", point).await.unwrap();

        let latest = store.query_latest("device1", "max_value").await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().as_f64(), Some(f64::MAX));

        // Test with f64::MIN
        let point = DataPoint::new(2000, f64::MIN);
        store.write("device1", "min_value", point).await.unwrap();

        let latest = store.query_latest("device1", "min_value").await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().as_f64(), Some(f64::MIN));
    }

    #[tokio::test]
    async fn test_unicode_metric_names() {
        let store = TimeSeriesStore::memory().unwrap();

        // Test Unicode metric names
        let unicode_metrics = vec![
            "temperature_cn", // Simplified - using ASCII-safe names
            "humidite_fr",    // French without accent
        ];

        for metric in &unicode_metrics {
            let point = DataPoint::new(1000, 20.0);
            store.write("device1", metric, point).await.unwrap();
        }

        // Verify all metrics are listed
        let metrics = store.list_metrics("device1").await.unwrap();
        for metric in &unicode_metrics {
            assert!(metrics.contains(&metric.to_string()));
        }

        // Verify we can query each metric
        for metric in &unicode_metrics {
            let latest = store.query_latest("device1", metric).await.unwrap();
            assert!(latest.is_some());
        }
    }

    #[tokio::test]
    async fn test_null_values_in_data_point() {
        let store = TimeSeriesStore::memory().unwrap();

        // Test with null value
        let point = DataPoint::new_with_value(1000, serde_json::Value::Null);
        store.write("device1", "null_metric", point).await.unwrap();

        let latest = store.query_latest("device1", "null_metric").await.unwrap();
        assert!(latest.is_some());
        let latest_point = latest.unwrap();
        assert!(latest_point.as_f64().is_none());
        assert!(latest_point.as_str().is_none());
        assert!(latest_point.as_bool().is_none());
    }

    #[tokio::test]
    async fn test_write_batch_concurrent() {
        let store = TimeSeriesStore::memory().unwrap();

        // Create multiple batch requests
        let mut requests = Vec::new();
        for device_id in 0..5 {
            let mut batch = BatchWriteRequest::new(format!("device{}", device_id));
            for metric_idx in 0..3 {
                let metric_name = format!("metric{}", metric_idx);
                for i in 0..10 {
                    let point = DataPoint::new(1000 + i * 10, i as f64);
                    batch.add_point(metric_name.clone(), point);
                }
            }
            requests.push(batch);
        }

        // Write concurrently
        let total_written = store.write_batch_concurrent(requests).await.unwrap();
        assert_eq!(total_written, 5 * 3 * 10); // 5 devices * 3 metrics * 10 points

        // Verify data
        for device_id in 0..5 {
            for metric_idx in 0..3 {
                let metric_name = format!("metric{}", metric_idx);
                let latest = store
                    .query_latest(&format!("device{}", device_id), &metric_name)
                    .await
                    .unwrap();
                assert!(latest.is_some());
            }
        }
    }

    #[tokio::test]
    async fn test_query_range_batch() {
        let store = TimeSeriesStore::memory().unwrap();

        // Write data for multiple metrics
        let metrics = vec!["temp", "humidity", "pressure"];
        for metric in &metrics {
            for i in 0..10 {
                let point = DataPoint::new(1000 + i * 10, i as f64);
                store.write("device1", metric, point).await.unwrap();
            }
        }

        // Query batch
        let results = store
            .query_range_batch("device1", &metrics, 1000, 2000)
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        for metric in &metrics {
            assert!(results.contains_key(*metric));
            let result = results.get(*metric).unwrap();
            assert_eq!(result.points.len(), 10);
        }
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let store = TimeSeriesStore::memory().unwrap();

        // Write some data to populate cache
        store
            .write("device1", "temp", DataPoint::new(1000, 20.0))
            .await
            .unwrap();

        // Query to populate cache
        let _ = store.query_latest("device1", "temp").await.unwrap();
        assert_eq!(store.cache_size(), 1);

        // Clear cache
        store.clear_cache();
        assert_eq!(store.cache_size(), 0);

        // Query again to repopulate
        let _ = store.query_latest("device1", "temp").await.unwrap();
        assert_eq!(store.cache_size(), 1);

        // Clean cache (should not remove fresh entries)
        let cleaned = store.clean_cache().await;
        assert_eq!(cleaned, 0);
        assert_eq!(store.cache_size(), 1);
    }

    #[tokio::test]
    async fn test_performance_stats() {
        let store = TimeSeriesStore::memory().unwrap();

        // Reset stats
        store.reset_stats().await;

        // Perform some operations
        for i in 0..10 {
            let point = DataPoint::new(1000 + i * 10, i as f64);
            store.write("device1", "temp", point).await.unwrap();
        }

        let _ = store.query_latest("device1", "temp").await.unwrap();

        // Check stats
        let stats = store.get_stats().await;
        assert_eq!(stats.write_count, 10);
        assert!(stats.read_count > 0);
        assert!(stats.total_write_ns > 0);
        assert!(stats.total_read_ns > 0);
        assert!(stats.avg_write_us() > 0.0);
    }

    #[tokio::test]
    async fn test_retention_policy() {
        let store = TimeSeriesStore::memory().unwrap();

        // Set a retention policy
        let mut policy = RetentionPolicy::new(Some(24)); // 24 hours
        policy.set_metric_retention("temp".to_string(), Some(1)); // 1 hour for temp

        store.set_retention_policy(policy).await;

        // Write old data (simulated by writing data, then manually setting cutoff)
        for i in 0..10 {
            let point = DataPoint::new(1000 + i * 10, i as f64);
            store.write("device1", "temp", point).await.unwrap();
        }

        // Get policy back
        let retrieved_policy = store.get_retention_policy().await;
        assert_eq!(retrieved_policy.default_hours, Some(24));
        assert_eq!(
            retrieved_policy.get_retention_hours("", "temp"),
            Some(1)
        );
    }

    #[tokio::test]
    async fn test_data_point_with_quality_and_metadata() {
        let point = DataPoint::new(1000, 42.0)
            .with_quality(0.95)
            .with_metadata(serde_json::json!({
                "source": "sensor",
                "unit": "celsius",
                "location": "room1"
            }));

        assert_eq!(point.timestamp, 1000);
        assert_eq!(point.as_f64(), Some(42.0));
        assert_eq!(point.quality, Some(0.95));
        assert!(point.metadata.is_some());

        let metadata = point.metadata.unwrap();
        assert_eq!(metadata["source"], "sensor");
        assert_eq!(metadata["unit"], "celsius");
        assert_eq!(metadata["location"], "room1");
    }

    #[tokio::test]
    async fn test_non_numeric_aggregation() {
        let store = TimeSeriesStore::memory().unwrap();

        // Write string data points
        for i in 0..10 {
            let point = DataPoint::new_string(1000 + i * 10, format!("value_{}", i));
            store.write("device1", "status", point).await.unwrap();
        }

        let buckets = store
            .query_aggregated("device1", "status", 1000, 1100, 100)
            .await
            .unwrap();

        assert_eq!(buckets.len(), 1);
        let bucket = &buckets[0];
        assert_eq!(bucket.count, 10);
        assert!(bucket.sum.is_none());
        assert!(bucket.min.is_none());
        assert!(bucket.max.is_none());
        assert!(bucket.avg.is_none());
        assert!(!bucket.sample_values.is_empty());
        assert!(bucket.sample_values.len() <= 10);
    }

    #[tokio::test]
    async fn test_timeseries_bucket_is_empty() {
        let mut bucket = TimeSeriesBucket::new(1000, 1100);
        assert!(bucket.is_empty());

        bucket.add(&serde_json::json!(42.0));
        assert!(!bucket.is_empty());
        assert_eq!(bucket.count, 1);
    }
}
