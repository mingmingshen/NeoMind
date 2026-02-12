# Storage Module

**Package**: `neomind-storage`
**Version**: 0.5.8
**Completion**: 95%
**Purpose**: Persistent storage layer

## Overview

The Storage module provides a unified persistent storage interface supporting time-series data, vector search, session history, and more.

## Module Structure

```
crates/storage/src/
├── lib.rs                      # Public interface
├── backends/
│   ├── mod.rs                  # Storage backends
│   └── redb.rs                 # Redb implementation
├── timeseries.rs               # Time-series storage
├── vector.rs                   # Vector storage
├── session.rs                  # Session storage
├── messages.rs                 # Message storage
├── settings.rs                 # Settings storage
├── agents.rs                   # Agent storage
├── decisions.rs                # Decision storage
├── device_state.rs             # Device state
├── device_registry.rs          # Device registry
├── llm_backends.rs             # LLM backend storage
├── llm_data.rs                 # LLM data storage
├── dashboards.rs               # Dashboard storage
├── business.rs                 # Business data
├── backup.rs                   # Backup management
├── maintenance.rs              # Maintenance scheduling
├── monitoring.rs               # Monitoring
└── multimodal.rs               # Multimodal storage
```

## Storage Backends

### RedbBackend

```rust
pub struct RedbBackend {
    /// Database path
    path: PathBuf,

    /// Database instance
    db: Arc<RwLock<Database>>,
}

impl RedbBackend {
    /// Open database
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;

    /// Create in-memory database
    pub fn memory() -> Result<Self>;

    /// Get table
    pub fn get_table(&self, name: &str) -> Result<Table>;
}

/// Storage backend factory function
pub fn create_backend(
    backend_type: &str,
    config: &serde_json::Value,
) -> Result<Arc<dyn StorageBackend>> {
    match backend_type {
        "redb" => {
            let path = config["path"].as_str().unwrap_or("./data");
            Ok(Arc::new(RedbBackend::open(path)?))
        }
        "memory" => {
            Ok(Arc::new(RedbBackend::memory()?))
        }
        _ => Err(Error::UnsupportedBackend(backend_type.to_string())),
    }
}
```

## Time-Series Storage

**Important Change (v0.5.x)**: All time-series data is now unified in `data/timeseries.redb`:

| Data Type | device_part | metric_part | Description |
|-----------|-------------|-------------|-------------|
| Device telemetry | `{device_id}` | `{metric_name}` | Metrics reported by devices |
| Extension metrics | `extension:{ext_id}` | `{metric_name}` | Metrics collected by extensions |
| Transform metrics | `transform:{trans_id}` | `{metric_name}` | Virtual metrics from transforms |

```rust
pub struct TimeSeriesStore {
    /// Storage backend
    backend: StorageBackend,
}

impl TimeSeriesStore {
    /// Create memory storage
    pub fn memory() -> Result<Self>;

    /// Create persistent storage
    pub fn persistent(path: &str) -> Result<Self>;

    /// Write data point
    pub async fn write(
        &self,
        device_id: &str,
        metric: &str,
        point: DataPoint,
    ) -> Result<()>;

    /// Batch write
    pub async fn write_batch(
        &self,
        request: BatchWriteRequest,
    ) -> Result<()>;

    /// Read data
    pub async fn read(
        &self,
        device_id: &str,
        metric: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<DataPoint>>;

    /// Aggregate query
    pub async fn aggregate(
        &self,
        device_id: &str,
        metric: &str,
        start: i64,
        end: i64,
        aggregation: AggregationType,
        window: Option<u64>,
    ) -> Result<Vec<DataPoint>>;

    /// Get latest value
    pub async fn latest(
        &self,
        device_id: &str,
        metric: &str,
    ) -> Result<Option<DataPoint>>;
}

pub struct DataPoint {
    pub timestamp: i64,
    pub value: MetricValue,
}

pub enum MetricValue {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
    Array(Vec<MetricValue>),
    Object(HashMap<String, MetricValue>),
}

pub enum AggregationType {
    Avg,
    Sum,
    Min,
    Max,
    Count,
    First,
    Last,
}
```

## Vector Storage

```rust
pub struct VectorStore {
    /// Storage backend
    backend: StorageBackend,
}

impl VectorStore {
    /// Create vector store
    pub fn new() -> Self;

    /// Insert document
    pub async fn insert(&self, doc: VectorDocument) -> Result<()>;

    /// Search
    pub async fn search(
        &self,
        embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<SearchResult>>;

    /// Remove document
    pub async fn remove(&self, id: &str) -> Result<()>;
}

pub struct VectorDocument {
    pub id: String,
    pub embedding: Vec<f32>,
    pub payload: serde_json::Value,
    pub metadata: HashMap<String, String>,
}

pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub payload: serde_json::Value,
}

pub enum SimilarityMetric {
    Cosine,
    Dot,
    Euclidean,
}
```

## Session Storage

```rust
pub struct SessionStore {
    db: Database,
}

impl SessionStore {
    /// Open session storage
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;

    /// Memory storage
    pub fn memory() -> Result<Self>;

    /// Save session ID
    pub fn save_session_id(&self, session_id: &str) -> Result<()>;

    /// Save history
    pub fn save_history(&self, session_id: &str, messages: &[SessionMessage]) -> Result<()>;

    /// Load history
    pub fn load_history(&self, session_id: &str) -> Result<Vec<SessionMessage>>;

    /// Save stream state
    pub fn save_pending_stream(
        &self,
        session_id: &str,
        state: PendingStreamState,
    ) -> Result<()>;

    /// Load stream state
    pub fn load_pending_stream(&self, session_id: &str) -> Result<Option<PendingStreamState>>;
}

pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub thinking: Option<String>,
}

pub struct PendingStreamState {
    pub session_id: String,
    pub stage: StreamStage,
    pub accumulated_content: String,
    pub accumulated_thinking: String,
    pub timestamp: i64,
}

pub enum StreamStage {
    Thinking,
    Content,
    ToolCall,
    Complete,
}
```

## Settings Storage

```rust
pub struct SettingsStore {
    db: Database,
}

impl SettingsStore {
    /// Open settings storage
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;

    /// Load LLM config
    pub fn load_llm_config(&self) -> Result<LlmSettings>;

    /// Save LLM config
    pub fn save_llm_config(&self, config: &LlmSettings) -> Result<()>;

    /// Load MQTT config
    pub fn load_mqtt_config(&self) -> Result<MqttSettings>;

    /// Save MQTT config
    pub fn save_mqtt_config(&self, config: &MqttSettings) -> Result<()>;

    /// Global timezone setting
    pub fn load_global_timezone(&self) -> Result<String>;

    pub fn save_global_timezone(&self, tz: &str) -> Result<()>;
}

pub struct LlmSettings {
    pub backend: LlmBackendType,
    pub model: String,
    pub endpoint: String,
    pub api_key: Option<String>,
}

pub struct MqttSettings {
    pub mode: MqttMode,
    pub port: u16,
    pub external_brokers: Vec<ExternalBroker>,
}
```

## Extension Storage

**Added (v0.5.x)**: Unified extension metrics storage service.

```rust
pub struct ExtensionMetricsStorage {
    metrics_storage: Arc<TimeSeriesStore>,
}

impl ExtensionMetricsStorage {
    /// Store extension metric to unified time-series database
    pub async fn store_metric_value(
        &self,
        extension_id: &str,
        metric_value: &MetricValue,
    ) -> Result<()>;

    /// Query extension metric latest value
    pub async fn query_latest(
        &self,
        extension_id: &str,
        metric_name: &str,
    ) -> Result<Option<DataPoint>>;

    /// Query extension metric history range
    pub async fn query_range(
        &self,
        extension_id: &str,
        metric_name: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<DataPoint>>;
}
```

**Storage Format**: Extension metrics stored in `timeseries.redb` using DataSourceId format:

```
DataSourceId: "extension:weather:temperature"
- device_part: "extension:weather"
- metric_part: "temperature"
```

**API Endpoints**:
```
GET    /api/extensions/:id/metrics         # List extension metrics
POST   /api/extensions/:id/metrics         # Register metric
DELETE /api/extensions/:id/metrics/:name   # Delete metric
```

## Backup Management

```rust
pub struct BackupManager {
    /// Data directory
    data_dir: PathBuf,

    /// Configuration
    config: BackupConfig,
}

impl BackupManager {
    /// Create backup
    pub async fn create_backup(&self, backup_type: BackupType) -> Result<BackupMetadata>;

    /// Restore backup
    pub async fn restore_backup(&self, backup_path: &Path) -> Result<()>;

    /// List backups
    pub async fn list_backups(&self) -> Result<Vec<BackupMetadata>>;

    /// Delete backup
    pub async fn delete_backup(&self, backup_id: &str) -> Result<()>;
}

pub enum BackupType {
    /// Full backup
    Full,

    /// Data only
    DataOnly,

    /// Config only
    ConfigOnly,
}

pub struct BackupMetadata {
    pub backup_id: String,
    pub backup_type: BackupType,
    pub created_at: i64,
    pub size_bytes: u64,
    pub file_path: PathBuf,
}
```

## Maintenance Scheduler

```rust
pub struct MaintenanceScheduler {
    /// Configuration
    config: MaintenanceConfig,

    /// Running tasks
    tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

impl MaintenanceScheduler {
    /// Start scheduler
    pub async fn start(&self) -> Result<()>;

    /// Stop scheduler
    pub async fn stop(&self) -> Result<()>;

    /// Execute cleanup
    pub async fn run_cleanup(&self) -> Result<MaintenanceResult>;

    /// Execute compaction
    pub async fn run_compaction(&self) -> Result<MaintenanceResult>;
}

pub struct MaintenanceConfig {
    /// Data retention days
    pub retention_days: u32,

    /// Cleanup schedule (cron)
    pub cleanup_schedule: String,

    /// Auto compact
    pub auto_compact: bool,
}
```

## Multimodal Storage

```rust
pub struct MultimodalStore {
    db: Database,
}

impl MultimodalStore {
    /// Open storage
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;

    /// Save image
    pub async fn save_image(&self, image: &ImageMetadata) -> Result<()>;

    /// Save document
    pub async fn save_document(&self, doc: &DocumentMetadata) -> Result<()>;

    /// Get image
    pub async fn get_image(&self, id: &str) -> Result<Option<ImageMetadata>>;

    /// Get document
    pub async fn get_document(&self, id: &str) -> Result<Option<DocumentMetadata>>;
}

pub struct ImageMetadata {
    pub id: String,
    pub session_id: String,
    pub format: ImageFormat,
    pub size_bytes: usize,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub stored_path: String,
    pub created_at: i64,
}

pub struct DocumentMetadata {
    pub id: String,
    pub session_id: String,
    pub file_type: String,
    pub size_bytes: usize,
    pub stored_path: String,
    pub created_at: i64,
}
```

## Database Migration

```rust
// Migrate from sled to redb
pub async fn migrate_from_sled(sled_path: &Path, redb_path: &Path) -> Result<()> {
    // 1. Open sled database
    let sled_db = sled::Db::open(sled_path)?;

    // 2. Create redb database
    let redb_db = Database::create(redb_path)?;

    // 3. Migrate each table
    // ...

    Ok(())
}

// v0.1 -> v0.2 migration
// - Session storage format changed
// - VectorStore serialization format changed
```

## API Endpoints

```
# Access via handlers in each module
```

## Usage Examples

```rust
use neomind_storage::{TimeSeriesStore, DataPoint};

// Create time-series storage
let store = TimeSeriesStore::persistent("./data/telemetry.redb")?;

// Write data
let point = DataPoint {
    timestamp: chrono::Utc::now().timestamp(),
    value: MetricValue::Float(25.5),
};

store.write("sensor_1", "temperature", point).await?;

// Read data
let data = store.read(
    "sensor_1",
    "temperature",
    start,
    end,
).await?;

// Aggregate query
let avg = store.aggregate(
    "sensor_1",
    "temperature",
    start,
    end,
    AggregationType::Avg,
    Some(3600),  // 1 hour window
).await?;
```

## Design Principles

1. **Unified Interface**: All storage uses the same trait
2. **Type Safety**: Strongly typed data values
3. **Backward Compatible**: Support database migration
4. **Performance**: Batch write and aggregate query
