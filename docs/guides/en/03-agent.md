# Agent Module

**Package**: `neomind-agent`
**Version**: 0.5.8
**Completion**: 90%
**Purpose**: AI chat agent with LLM, memory, and tools integration

## Overview

The Agent module implements NeoMind's core AI agent, responsible for handling user conversations, calling tools, managing sessions, and executing autonomous decisions.

## Module Structure

```
crates/neomind-agent/src/
├── lib.rs                      # Public interface
├── agent/
│   ├── mod.rs                  # Agent core implementation
│   ├── types.rs                # Agent type definitions
│   ├── cache.rs                # Cache management
│   ├── fallback.rs             # Fallback rules
│   ├── scheduler.rs            # Scheduler
│   ├── streaming.rs            # Streaming response
│   └── tokenizer.rs            # Tokenizer
├── ai_agent/
│   ├── mod.rs                  # Autonomous Agent
│   ├── executor.rs             # Executor (supports device/extension metrics collection)
│   └── intent_parser.rs        # Intent parser
├── tools/
│   ├── mod.rs                  # Agent tools
│   ├── dsl.rs                  # DSL tools
│   ├── mapper.rs               # Mapping tools
│   └── rule_gen.rs             # Rule generation
├── prompts/
│   └── builder.rs              # Prompt builder
├── config/
│   └── mod.rs                  # Configuration
├── context_selector.rs         # Context selector
├── error.rs                    # Error types
├── hooks/                      # Hook system
├── llm.rs                      # LLM integration
├── session.rs                  # Session management
└── translation.rs              # Translation
```

## Important Changes (v0.5.x)

### Removed Modules
- `agent/intent_classifier.rs` - Intent classification integrated into executor
- `task_orchestrator.rs` - Task orchestration integrated into executor
- `tools/automation.rs` - Automation tools migrated to automation module

### New Features
- **Extension Metrics Support**: executor.rs now collects extension (Extension) metrics
- **DataSourceId Integration**: Uses type-safe DataSourceId for metrics queries
- **Unified Time-Series Database**: Uses `data/timeseries.redb` unified storage for device and extension metrics

## Core Components

### 1. Agent - Core Agent

```rust
pub struct Agent {
    /// Agent configuration
    config: AgentConfig,

    /// LLM backend
    llm: Arc<dyn LlmRuntime>,

    /// Tool registry
    tools: Arc<ToolRegistry>,

    /// State machine
    state_machine: StateMachine,

    /// Hook chain
    hooks: HookChain,

    /// Short-term memory
    short_term_memory: ShortTermMemory,
}

pub struct AgentConfig {
    /// LLM backend configuration
    pub llm_backend: LlmBackend,

    /// Maximum token count
    pub max_tokens: usize,

    /// Temperature parameter
    pub temperature: f32,

    /// Timeout duration
    pub timeout_secs: u64,
}
```

### 2. SessionManager - Session Management

```rust
pub struct SessionManager {
    /// Active sessions
    sessions: HashMap<SessionId, Session>,

    /// Storage backend
    store: Arc<SessionStore>,

    /// Agent configuration
    agent_config: AgentConfig,
}

impl SessionManager {
    /// Create new session
    pub async fn create_session(&self) -> Result<SessionId>;

    /// Process message
    pub async fn process_message(
        &self,
        session_id: &str,
        message: &str,
    ) -> Result<AgentResponse>;

    /// Get history
    pub async fn get_history(&self, session_id: &str) -> Result<Vec<Message>>;

    /// Delete session
    pub async fn delete_session(&self, session_id: &str) -> Result<()>;
}
```

### 3. AgentResponse - Response Type

```rust
pub struct AgentResponse {
    /// Message content
    pub message: AgentMessage,

    /// Tools used
    pub tools_used: Vec<ToolCall>,

    /// Processing duration
    pub duration_ms: u64,

    /// Token usage
    pub token_usage: TokenUsage,
}

pub struct AgentMessage {
    /// Main content
    pub content: String,

    /// Reasoning content (thinking)
    pub thinking: Option<String>,

    /// Role information
    pub role: MessageRole,
}
```

## State Machine

```rust
pub enum ProcessState {
    /// Idle state
    Idle,

    /// Processing
    Processing {
        stage: ProcessingStage,
        progress: f32,
    },

    /// Waiting for tool execution
    WaitingForTools {
        tools: Vec<String>,
    },

    /// Completed
    Completed {
        result: ProcessResult,
    },

    /// Error
    Error {
        error: String,
        recovery_action: RecoveryAction,
    },
}

pub enum ProcessingStage {
    ParsingIntent,
    SelectingTools,
    CallingLlm,
    ExecutingTools,
    FormattingResponse,
}
```

## Hook System

```rust
pub trait AgentHook: Send + Sync {
    /// Pre-processing (before LLM call)
    fn pre_process(&self, ctx: &HookContext) -> HookResult;

    /// Post-processing (after LLM call)
    fn post_process(&self, ctx: &HookContext, output: &str) -> HookResult;

    /// Before tool call
    fn pre_tool(&self, ctx: &HookContext, tool: &str) -> HookResult;

    /// After tool call
    fn post_tool(&self, ctx: &HookContext, tool: &str, result: &ToolOutput) -> HookResult;
}

/// Built-in Hooks
pub enum BuiltInHook {
    /// Logging
    Logging(LoggingHook),

    /// Metrics collection
    Metrics(MetricsHook),

    /// Content moderation
    ContentModeration(ContentModerationHook),

    /// Input sanitization
    InputSanitization(InputSanitizationHook),
}
```

## Concurrency Control

```rust
pub struct GlobalConcurrencyLimiter {
    /// Global limit
    pub max_concurrent: usize,
    /// Current count
    pub current: Arc<AtomicUsize>,
    /// Semaphore
    pub semaphore: Arc<Semaphore>,
}

pub struct SessionConcurrencyLimiter {
    /// Per-session limit
    pub max_per_session: usize,
    /// Session count
    pub sessions: Arc<RwLock<HashMap<SessionId, usize>>>,
}
```

## Tool Calling

```rust
pub struct ToolCall {
    /// Tool name
    pub name: String,

    /// Arguments
    pub arguments: serde_json::Value,

    /// Execution result
    pub result: Option<ToolOutput>,

    /// Execution status
    pub status: ToolCallStatus,
}

pub enum ToolCallStatus {
    Pending,
    Executing,
    Succeeded,
    Failed(String),
}
```

## Built-in Tools

### Agent-Specific Tools
```rust
/// Analysis tools
- AnomaliesAnalysis     // Anomaly detection
- TrendsAnalysis        // Trend analysis
- DecisionsAnalysis     // Decision analysis

/// Automation tools
- AutomationTool        // Automation operations

/// DSL tools
- DslTool               // DSL parsing and generation

/// Event tools
- EventIntegrationTool  // Event subscription

/// Interaction tools
- InteractionTool       // User interaction

/// Mapping tools
- MapperTool            // Data mapping

/// MDL tools
- MdlTool               // MDL operations

/// Rule tools
- RuleGenTool           // Rule generation

/// Thinking tools
- ThinkTool             // Reasoning

/// Tool search
- ToolSearchTool        // Tool lookup
```

## AgentExecutor - Executor

AgentExecutor is the core component of the autonomous Agent, responsible for executing agents, collecting data, calling LLM, and executing decisions.

### Data Collection

AgentExecutor supports data collection from multiple resource types:

```rust
pub enum ResourceType {
    /// Device metrics
    Metric,

    /// Extension metrics
    ExtensionMetric,

    /// Device resources
    Device,

    /// Extension tools
    ExtensionTool,
}
```

### DataSourceId Integration

AgentExecutor uses type-safe DataSourceId for metrics queries:

```rust
use neomind_core::datasource::DataSourceId;

// Parse DataSourceId
let ds_id = DataSourceId::new("extension:weather:temperature")?;
let device_part = ds_id.device_part();  // "extension:weather"
let metric_part = ds_id.metric_part();   // "temperature"

// Query time-series data
let result = time_series_storage.query_latest(&device_part, &metric_part).await?;
```

### Device Metrics Collection

```rust
async fn collect_single_metric(
    storage: Arc<TimeSeriesStore>,
    device_id: &str,
    metric_name: &str,
    time_range_minutes: u32,
) -> AgentResult<Option<DataCollected>> {
    let end_time = chrono::Utc::now().timestamp();
    let start_time = end_time - ((time_range_minutes * 60) as i64);

    let result = storage.query_range(device_id, metric_name, start_time, end_time).await?;
    // ...
}
```

### Extension Metrics Collection

```rust
async fn collect_extension_metric_data_parallel(
    &self,
    agent: &AiAgent,
    resources: Vec<AgentResource>,
    timestamp: i64,
) -> AgentResult<Vec<DataCollected>> {
    // Use DataSourceId's device_part and metric_part
    let device_part = ds_id.device_part();  // "extension:extension_id"
    let metric_part = ds_id.metric_part();   // metric_name

    let result = storage.query_latest(&device_part, metric_part).await?;
    // ...
}
```

### Unified Time-Series Database

**Important Change**: AgentExecutor now uses `data/timeseries.redb` instead of `data/timeseries_agents.redb`.

This enables Agent to access:
- Device telemetry data (written via DeviceService)
- Extension metrics data (written via ExtensionMetricsStorage)

```rust
// crates/neomind-api/src/server/types.rs
let time_series_store = match neomind_storage::TimeSeriesStore::open("data/timeseries.redb") {
    Ok(store) => Some(store),
    Err(e) => {
        tracing::warn!("Failed to open TimeSeriesStore: {}", e);
        None
    }
};
```

### WebSocket Events

AgentExecutor sends real-time events via EventBus:

```rust
// Execution started
event_bus.publish(NeoMindEvent::AgentExecutionStarted(AgentExecutionStartedEvent {
    agent_id: agent.id.clone(),
    execution_id: execution_id.clone(),
    trigger_type: "manual".to_string(),
}));

// Execution completed
event_bus.publish(NeoMindEvent::AgentExecutionCompleted(AgentExecutionCompletedEvent {
    agent_id: agent.id.clone(),
    execution_id: execution_id.clone(),
    success: result.is_ok(),
    error: result.as_ref().err().map(|e| e.to_string()),
}));

// Thinking
event_bus.publish(NeoMindEvent::AgentThinking(AgentThinkingEvent {
    agent_id: agent.id.clone(),
    description: format!("Analyzing {} data sources", data_collected.len()),
}));
```

## Autonomous Agent

```rust
pub struct AutonomousAgent {
    /// Agent state
    state: AgentState,

    /// Configuration
    config: AutonomousConfig,

    /// LLM runtime
    llm: Arc<dyn LlmRuntime>,

    /// Decision history
    decisions: Vec<Decision>,
}

pub enum AgentState {
    Idle,
    Observing,
    Analyzing,
    Deciding,
    Acting,
    Reviewing,
}

pub struct AutonomousConfig {
    /// Review interval (seconds)
    pub review_interval_secs: u64,

    /// Decision threshold
    pub decision_threshold: f32,

    /// Max concurrent decisions
    pub max_concurrent_decisions: usize,
}
```

## Usage Examples

### Basic Chat

```rust
use neomind_agent::{SessionManager, AgentConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = SessionManager::new()?;

    // Create session
    let session_id = manager.create_session().await?;

    // Send message
    let response = manager.process_message(
        &session_id,
        "List all temperature sensors"
    ).await?;

    println!("AI: {}", response.message.content);
    println!("Tools: {:?}", response.tools_used);

    Ok(())
}
```

### Chat with Tools

```rust
use neomind_agent::{SessionManager, ToolRegistryBuilder};
use neomind_tools::{QueryDataTool, ControlDeviceTool};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create session manager with tools
    let tools = ToolRegistryBuilder::new()
        .with_tool(Arc::new(QueryDataTool::mock()))
        .with_tool(Arc::new(ControlDeviceTool::mock()))
        .build();

    let manager = SessionManager::with_tools(tools)?;

    let session_id = manager.create_session().await?;

    // AI automatically calls tools
    let response = manager.process_message(
        &session_id,
        "Turn on the living room light"
    ).await?;

    // Check which tools were called
    for tool_call in response.tools_used {
        println!("Called: {}", tool_call.name);
        println!("Args: {}", tool_call.arguments);
        println!("Result: {:?}", tool_call.result);
    }

    Ok(())
}
```

### Streaming Response

```rust
use futures::StreamExt;
use neomind_agent::{SessionManager, StreamingConfig};

async fn chat_stream(
    manager: &SessionManager,
    session_id: &str,
    message: &str,
) -> Result<String> {
    let mut stream = manager
        .process_message_stream(session_id, message)
        .await?;

    let mut full_response = String::new();

    while let Some(chunk) = stream.next().await {
        match chunk? {
            StreamChunk::Thinking(content) => {
                println!("[Thinking] {}", content);
            }
            StreamChunk::Content(content) => {
                print!("{}", content);
                std::io::stdout().flush()?;
                full_response.push_str(&content);
            }
            StreamChunk::ToolCall(tool) => {
                println!("[Tool] {}", tool.name);
            }
        }
    }

    Ok(full_response)
}
```

## Configuration

```rust
pub struct AgentConfig {
    /// LLM backend
    pub llm_backend: LlmBackend,

    /// Max message history
    pub max_history_messages: usize,

    /// Max token count
    pub max_tokens: usize,

    /// Temperature parameter
    pub temperature: f32,

    /// Timeout (seconds)
    pub timeout_secs: u64,

    /// Enable streaming
    pub streaming: bool,

    /// Enable tool calling
    pub enable_tools: bool,

    /// Concurrency limit
    pub max_concurrent_requests: usize,
}

pub fn get_default_config() -> AgentConfig {
    AgentConfig {
        llm_backend: LlmBackend::Ollama,
        max_history_messages: 50,
        max_tokens: 4000,
        temperature: 0.7,
        timeout_secs: 120,
        streaming: true,
        enable_tools: true,
        max_concurrent_requests: 10,
    }
}
```

## Error Handling

```rust
pub enum NeoMindError {
    /// LLM errors
    Llm(LlmError),

    /// Tool errors
    Tool(ToolError),

    /// Storage errors
    Storage(StorageError),

    /// Session not found
    SessionNotFound(String),

    /// Timeout
    Timeout,

    /// Concurrency limit
    ConcurrencyLimit,

    /// Other errors
    Other(anyhow::Error),
}

pub enum FallbackAction {
    /// Retry
    Retry { max_attempts: usize },

    /// Use default response
    DefaultResponse(String),

    /// Simplify mode
    SimplifyMode,

    /// Skip tool calls
    SkipTools,
}
```

## Design Principles

1. **State-Driven**: Uses state machine for Agent lifecycle
2. **Tool-First**: Tool calling enabled by default
3. **Streaming-First**: Streaming responses by default
4. **Extensible**: Hook system supports custom behavior
5. **Fault-Tolerant**: Multi-level fallback and error recovery
