# Core Module

**Package**: `neomind-core`
**Version**: 0.8.0
**Completion**: 95%
**Purpose**: Defines core traits and types for the entire project

## Overview

The Core module is the foundation of the NeoMind project, defining the core abstractions and types that all other modules depend on. It contains no concrete implementations, only interface definitions.

## Module Structure

```
crates/neomind-core/src/
├── lib.rs                  # Public interface exports
├── brand.rs                # Branding constants
├── event.rs                # Event type definitions
├── eventbus.rs             # Event bus implementation
├── message.rs              # Message type definitions
├── message/
│   └── convert.rs          # Message conversion utilities
├── session.rs              # Session type definitions
├── llm/
│   ├── backend.rs          # LLM runtime trait + BackendRegistry
│   ├── capability.rs       # Capability definitions
│   ├── modality.rs         # Multi-modal content support
│   ├── memory_consolidation.rs  # Memory consolidation
│   ├── models.rs           # Model definitions
│   ├── compaction.rs       # Context compaction
│   └── token_counter.rs    # Token counting utilities
├── tools/
│   └── mod.rs              # Tool trait definitions
├── storage/
│   └── mod.rs              # Storage trait definitions
├── datasource/
│   ├── mod.rs              # Data source ID system + types
│   └── query.rs            # Unified query service
├── extension/
│   ├── mod.rs              # Extension system
│   ├── types.rs            # Extension types
│   ├── registry.rs         # Extension registry
│   ├── executor.rs         # Extension executor
│   ├── proxy.rs            # Extension proxy
│   ├── runtime.rs          # Extension runtime
│   ├── safety.rs           # Safety/crash protection
│   ├── system.rs           # Extension system management
│   ├── context.rs          # Extension context
│   ├── package.rs          # Package management
│   ├── stream.rs           # Streaming support
│   ├── tracing.rs          # Tracing utilities
│   ├── capability_services.rs  # Capability services
│   ├── event_dispatcher.rs     # Event dispatching
│   ├── event_subscription.rs   # Event subscriptions
│   ├── extension_event_subscription.rs  # Extension event subscriptions
│   ├── loader/                 # Extension loaders
│   │   ├── mod.rs
│   │   ├── native.rs       # Native extension loader
│   │   └── isolated.rs     # Isolated process loader
│   └── isolated/           # Process-isolated extensions
│       ├── mod.rs
│       ├── manager.rs      # Process manager
│       ├── process.rs      # Process lifecycle
│       ├── ipc_local.rs    # Local IPC
│       └── in_flight.rs    # In-flight request tracking
├── error/
│   ├── mod.rs              # Error types
│   └── redb.rs             # Redb-specific errors
├── config.rs               # Configuration constants
└── macros.rs               # Macro definitions
```

## Core Traits

### 1. LlmRuntime - LLM Runtime Interface

Defines the interface that all LLM backends must implement.

```rust
#[async_trait]
pub trait LlmRuntime: Send + Sync {
    /// Get the backend type identifier
    fn backend_id(&self) -> BackendId;

    /// Get the current model name
    fn model_name(&self) -> &str;

    /// Check if the backend is available
    async fn is_available(&self) -> bool { true }

    /// Warm up the model (optional, eliminates first-request latency)
    async fn warmup(&self) -> Result<(), LlmError> { Ok(()) }

    /// Generate text (non-streaming)
    async fn generate(&self, input: LlmInput) -> Result<LlmOutput, LlmError>;

    /// Generate text (streaming)
    async fn generate_stream(
        &self,
        input: LlmInput,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamChunk> + Send>>, LlmError>;

    /// Get max context length
    fn max_context_length(&self) -> usize;

    /// Estimate token count
    fn estimate_tokens(&self, text: &str) -> usize { text.len() / 4 }

    /// Check if multimodal (vision) is supported
    fn supports_multimodal(&self) -> bool { false }

    /// Get backend capabilities
    fn capabilities(&self) -> BackendCapabilities { BackendCapabilities::default() }

    /// Get backend metrics (if supported)
    fn metrics(&self) -> BackendMetrics { BackendMetrics::default() }
}
```

**BackendCapabilities**:
```rust
pub struct BackendCapabilities {
    /// Supports streaming generation
    pub streaming: bool,
    /// Supports multimodal (vision)
    pub multimodal: bool,
    /// Supports function calling
    pub function_calling: bool,
    /// Supports multiple models
    pub multiple_models: bool,
    /// Maximum context length
    pub max_context: Option<usize>,
    /// Supported modalities
    pub modalities: Vec<String>,
    /// Supports thinking/reasoning display
    pub thinking_display: bool,
    /// Supports image input
    pub supports_images: bool,
    /// Supports audio input
    pub supports_audio: bool,
}
```

### 2. Tool - Tool Interface

Defines the interface for AI-callable tools.

```rust
pub trait Tool: Send + Sync {
    /// Tool definition (name, description, parameters)
    fn definition(&self) -> &ToolDefinition;

    /// Execute tool
    fn execute(&self, input: &serde_json::Value) -> Result<ToolOutput>;

    /// Validate input
    fn validate(&self, input: &serde_json::Value) -> Result<()> {
        // Default implementation
    }
}
```

### 3. Integration - Integration Interface

Defines the interface for external system integration.

```rust
#[async_trait]
pub trait Integration: Send + Sync {
    /// Get metadata
    fn metadata(&self) -> &IntegrationMetadata;

    /// Get current state
    fn state(&self) -> IntegrationState;

    /// Start integration
    async fn start(&self) -> Result<()>;

    /// Stop integration
    async fn stop(&self) -> Result<()>;

    /// Subscribe to event stream
    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = IntegrationEvent> + Send + '_>>;

    /// Send command
    async fn send_command(&self, command: IntegrationCommand) -> Result<IntegrationResponse>;
}
```

### 4. Extension - Extension Interface

Defines the interface for dynamically loaded extensions. The V2 Extension trait separates metrics from commands:

```rust
#[async_trait]
pub trait Extension: Send + Sync {
    /// Get extension metadata
    fn metadata(&self) -> &ExtensionMetadata;

    /// Declare metrics provided by this extension
    fn metrics(&self) -> &[MetricDescriptor] { &[] }

    /// Declare commands supported by this extension
    fn commands(&self) -> &[ExtensionCommand] { &[] }

    /// Execute a command (async)
    async fn execute_command(&self, command: &str, args: &Value) -> Result<Value>;

    /// Produce metric data (sync for dylib compatibility)
    fn produce_metrics(&self) -> Result<Vec<ExtensionMetricValue>> { Ok(Vec::new()) }

    /// Health check (async, optional)
    async fn health_check(&self) -> Result<bool> { Ok(true) }

    /// Runtime configuration (optional)
    async fn configure(&mut self, config: &Value) -> Result<()> { Ok(()) }
}
```

See [Extension Development Guide](16-extension-dev.md) for complete API details.

### 5. DataSourceId - Data Source Identifier

Provides type-safe data source identification, supporting device, extension, and transform data sources.

```rust
pub struct DataSourceId {
    /// Data source type
    pub source_type: DataSourceType,
    /// Data source ID
    pub source_id: String,
    /// Field path
    pub field_path: String,
}

pub enum DataSourceType {
    Device,
    Extension,
    Transform,
}

impl DataSourceId {
    /// Parse data source ID
    pub fn new(id: &str) -> Result<Self>;

    /// Get device_id part for TimeSeriesStorage
    pub fn device_part(&self) -> String;

    /// Get metric part for TimeSeriesStorage
    pub fn metric_part(&self) -> &str;

    /// Get full storage key
    pub fn storage_key(&self) -> String;
}
```

**Format**: `{source_type}:{source_id}:{field_path}`

| Type | Format | device_part | metric_part |
|------|------|-------------|-------------|
| Device | `{device_id}:{field_path}` | `{device_id}` | `{field_path}` |
| Extension | `extension:{ext_id}:{field_path}` | `extension:{ext_id}` | `{field_path}` |
| Transform | `transform:{trans_id}:{field_path}` | `transform:{trans_id}` | `{field_path}` |

### 6. StorageBackend - Storage Interface

Defines the generic storage interface.

```rust
pub trait StorageBackend: Send + Sync {
    /// Write data
    async fn write(&self, key: &str, value: &[u8]) -> Result<()>;

    /// Read data
    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Delete data
    async fn delete(&self, key: &str) -> Result<()>;

    /// List keys
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
}
```

## Core Types

### Message - Message Type

```rust
pub struct Message {
    pub role: MessageRole,
    pub content: Content,
    pub timestamp: Option<i64>,
    pub metadata: Option<EventMetadata>,
}

pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

pub enum Content {
    Text(String),
    Parts(Vec<ContentPart>),  // Multi-modal support
}

pub enum ContentPart {
    Text { text: String },
    Image { image: ImageContent },
    Thinking { content: String },  // Reasoning content
}
```

### NeoMindEvent - Event Type

```rust
pub enum NeoMindEvent {
    // Device events
    DeviceOnline { device_id: String, device_type: String, timestamp: i64 },
    DeviceOffline { device_id: String, reason: Option<String>, timestamp: i64 },
    DeviceMetric { device_id: String, metric: String, value: MetricValue, timestamp: i64,
                   quality: Option<f32>, is_virtual: Option<bool> },
    DeviceCommandResult { device_id: String, command: String, success: bool,
                          result: Option<serde_json::Value>, timestamp: i64 },
    DeviceDiscovered { device_id: String, source: String, adapter_id: Option<String>,
                       metadata: serde_json::Value, sample: serde_json::Value,
                       is_binary: bool, timestamp: i64 },

    // Rule events
    RuleEvaluated { rule_id: String, rule_name: String, condition_met: bool, timestamp: i64 },
    RuleTriggered { rule_id: String, rule_name: String, trigger_value: f64,
                    actions: Vec<String>, timestamp: i64 },
    RuleExecuted { rule_id: String, rule_name: String, success: bool,
                   duration_ms: u64, timestamp: i64 },

    // Workflow events
    WorkflowTriggered { workflow_id: String, trigger_type: String,
                        trigger_data: Option<serde_json::Value>, execution_id: String, timestamp: i64 },
    WorkflowStepCompleted { workflow_id: String, execution_id: String, step_id: String,
                            result: serde_json::Value, timestamp: i64 },
    WorkflowCompleted { workflow_id: String, execution_id: String, success: bool,
                        duration_ms: u64, timestamp: i64 },

    // Alert events
    AlertCreated { alert_id: String, title: String, severity: String, message: String, timestamp: i64 },
    AlertAcknowledged { alert_id: String, acknowledged_by: String, timestamp: i64 },

    // Message events
    MessageCreated { message_id: String, title: String, severity: String, message: String, timestamp: i64 },
    MessageAcknowledged { message_id: String, acknowledged_by: String, timestamp: i64 },
    MessageResolved { message_id: String, timestamp: i64 },

    // Agent events (User-defined AI Agents)
    AgentExecutionStarted { agent_id: String, agent_name: String, execution_id: String,
                            trigger_type: String, timestamp: i64 },
    AgentThinking { agent_id: String, execution_id: String, step_number: u32,
                    step_type: String, description: String, details: Option<serde_json::Value>, timestamp: i64 },
    AgentDecision { agent_id: String, execution_id: String, description: String,
                    rationale: String, action: String, confidence: f32, timestamp: i64 },
    AgentProgress { agent_id: String, execution_id: String, stage: String, stage_label: String,
                    progress: Option<f32>, details: Option<String>, timestamp: i64 },
    AgentExecutionCompleted { agent_id: String, execution_id: String, success: bool,
                              duration_ms: u64, error: Option<String>, timestamp: i64 },
    AgentMemoryUpdated { agent_id: String, memory_type: String, timestamp: i64 },

    // LLM events (Autonomous Agent)
    PeriodicReviewTriggered { review_id: String, review_type: String, timestamp: i64 },
    LlmDecisionProposed { decision_id: String, title: String, description: String,
                          reasoning: String, actions: Vec<ProposedAction>,
                          confidence: f32, timestamp: i64 },
    LlmDecisionExecuted { decision_id: String, success: bool,
                          result: Option<serde_json::Value>, timestamp: i64 },

    // User events
    UserMessage { session_id: String, content: String, timestamp: i64 },
    LlmResponse { session_id: String, content: String, tools_used: Vec<String>,
                  processing_time_ms: u64, timestamp: i64 },

    // Tool execution events
    ToolExecutionStart { tool_name: String, arguments: serde_json::Value,
                         session_id: Option<String>, timestamp: i64 },
    ToolExecutionSuccess { tool_name: String, arguments: serde_json::Value, result: serde_json::Value,
                           duration_ms: u64, session_id: Option<String>, timestamp: i64 },
    ToolExecutionFailure { tool_name: String, arguments: serde_json::Value, error: String,
                           error_type: String, duration_ms: u64, session_id: Option<String>, timestamp: i64 },

    // Extension events (Phase 2.1)
    ExtensionOutput { extension_id: String, output_name: String, value: MetricValue,
                      timestamp: i64, labels: Option<HashMap<String, String>>, quality: Option<f32> },
    ExtensionLifecycle { extension_id: String, state: String, message: Option<String>, timestamp: i64 },
    ExtensionCommandStarted { extension_id: String, extension_name: String, command_id: String,
                              execution_id: String, args: serde_json::Value, timestamp: i64 },
    ExtensionCommandCompleted { extension_id: String, extension_name: String, command_id: String,
                                execution_id: String, args: serde_json::Value, outputs: Vec<serde_json::Value>,
                                duration_ms: u64, timestamp: i64 },
    ExtensionCommandFailed { extension_id: String, extension_name: String, command_id: String,
                             execution_id: String, error: String, duration_ms: u64, timestamp: i64 },

    // Custom events (for extensions and plugins)
    Custom { event_type: String, data: serde_json::Value },
}
```

### Session - Session Type

```rust
pub struct Session {
    pub id: SessionId,
    pub created_at: i64,
    pub updated_at: i64,
    pub messages: Vec<Message>,
    pub metadata: SessionMetadata,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);
```

## EventBus - Event Bus

```rust
pub struct EventBus {
    // Channel capacity
    capacity: usize,
    // Broadcast sender
    sender: broadcast::Sender<NeoMindEvent>,
}

impl EventBus {
    /// Create new event bus
    pub fn new() -> Self;

    /// Create event bus with capacity
    pub fn with_capacity(capacity: usize) -> Self;

    /// Publish event
    pub fn publish(&self, event: NeoMindEvent);

    /// Subscribe to all events
    pub fn subscribe(&self) -> EventBusReceiver;

    /// Create filter
    pub fn filter(&self) -> FilterBuilder;
}
```

### FilterBuilder - Event Filtering

```rust
pub struct FilterBuilder<'a> {
    bus: &'a EventBus,
    filters: Vec<FilterFn>,
}

impl<'a> FilterBuilder<'a> {
    /// Only receive device events
    pub fn device_events(self) -> FilteredReceiver;

    /// Only receive rule events
    pub fn rule_events(self) -> FilteredReceiver;

    /// Custom filter
    pub fn custom<F>(self, f: F) -> FilteredReceiver
    where
        F: Fn(&NeoMindEvent) -> bool + Send + 'static;
}
```

## Configuration Constants

```rust
// LLM provider configuration
pub const DEFAULT_OLLAMA_ENDPOINT: &str = "http://localhost:11434";
pub const DEFAULT_OPENAI_ENDPOINT: &str = "https://api.openai.com/v1";

// Default models
pub fn models() -> Vec<&'static str> {
    vec![
        "qwen3.5:4b",      // Ollama default
    ]
}

// API endpoints
pub fn endpoints() -> HashMap<String, String> {
    // ...
}
```

## Usage Examples

### Creating EventBus

```rust
use neomind_core::EventBus;
use neomind_core::NeoMindEvent;

#[tokio::main]
async fn main() {
    let bus = EventBus::new();

    // Subscribe to all events
    let mut rx = bus.subscribe();

    // Subscribe to device events
    let mut device_rx = bus.filter().device_events();

    // Publish event
    bus.publish(NeoMindEvent::DeviceOnline {
        device_id: "sensor_1".to_string(),
        device_type: "sensor".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    });

    // Receive events
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            println!("Received: {:?}", event);
        }
    });
}
```

### Using LlmRuntime Trait

```rust
use neomind_core::llm::backend::{LlmRuntime, LlmInput, GenerationParams};

async fn call_llm(runtime: &dyn LlmRuntime, prompt: &str) -> Result<String, LlmError> {
    let input = LlmInput::new(prompt)
        .with_params(GenerationParams::default());

    let output = runtime.generate(input).await?;
    Ok(output.text)
}
```

## Error Handling

```rust
pub enum Error {
    /// LLM related errors
    Llm(LlmError),

    /// Storage errors
    Storage(StorageError),

    /// Tool errors
    Tool(ToolError),

    /// Integration errors
    Integration(IntegrationError),

    /// Extension errors
    Extension(ExtensionError),

    /// IO errors
    Io(std::io::Error),

    /// Other errors
    Other(anyhow::Error),
}
```

## Dependencies

```
Core (neomind-core)
    │
    ├── No external dependencies (trait definitions only)
    │
    └── Depends on all other crates
        ├── llm
        ├── agent
        ├── devices
        ├── tools
        ├── storage
        ├── integrations
        └── ...
```

## Design Principles

1. **Minimal Dependencies**: Core module has no dependencies on other business modules
2. **Trait-First**: Define interfaces through traits, allowing different implementations
3. **Type Safety**: Use Rust type system to ensure correctness
4. **Async-First**: All I/O operations are asynchronous
5. **Extensibility**: Loose coupling through traits and events
