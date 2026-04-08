# Agent Module

**Package**: `neomind-agent`
**Version**: 0.6.4
**Completion**: 95%
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
│   ├── planner/                # Execution plan generation (v0.6.4)
│   │   ├── mod.rs              #   Planner module entry
│   │   ├── types.rs            #   PlanStep, ExecutionPlan, PlanningConfig
│   │   ├── keyword.rs          #   KeywordPlanner (rule-based, zero LLM cost)
│   │   ├── llm_planner.rs      #   LLMPlanner (structured output parsing)
│   │   └── coordinator.rs      #   PlanningCoordinator (routes between planners)
│   ├── scheduler.rs            # Scheduler
│   ├── streaming.rs            # Streaming response
│   └── tokenizer.rs            # Tokenizer
├── ai_agent/
│   ├── mod.rs                  # Autonomous Agent
│   ├── executor/               # Executor module
│   │   ├── mod.rs              #   Executor core (supports device/extension metrics collection)
│   │   └── memory.rs           #   Memory integration for executor
│   └── intent_parser.rs        # Intent parser
├── tools/
│   ├── mod.rs                  # Agent tools
│   ├── dsl.rs                  # DSL tools
│   ├── mapper.rs               # Mapping tools
│   └── rule_gen.rs             # Rule generation
├── toolkit/
│   ├── mod.rs                  # Toolkit module
│   ├── resolver.rs             # EntityResolver (fuzzy name/ID matching) (v0.6.4)
│   └── simplified.rs           # Simplified tool definitions for prompts
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

## Important Changes (v0.6.x)

### Aggregated Tools (Token Efficiency)
Agent now uses **aggregated tools** instead of individual tool functions. This significantly reduces token usage in function calling:

```rust
// Old: ~50 individual tool definitions (~3000 tokens)
tools: [query_device, control_device, create_rule, update_rule, ...]

// New: ~8 aggregated tool definitions (~800 tokens)
tools: [device_tools, automation_tools, system_tools, ...]
```

Benefits:
- **60%+ token reduction** in tool definitions
- Faster LLM inference
- Cleaner tool organization

### Execution Mode
Agents now support different execution modes:

```rust
pub enum ExecutionMode {
    /// Single-step execution (default)
    Single,
    /// Multi-step with planning
    MultiStep,
    /// Autonomous continuous execution
    Autonomous,
}
```

### Per-Step Results
Agent execution now captures per-step results for better observability:

```rust
pub struct StepResult {
    pub step_number: u32,
    pub action: String,
    pub result: String,
    pub duration_ms: u64,
}
```

### LLM Backend Decoupling
Agent LLM backends are now **decoupled** from chat model selection:
- Changing chat model no longer overwrites agent LLM backend
- Agents can use different LLM backends for different purposes
- Separate configuration for extraction vs. execution

### Removed Modules (v0.5.x)
- `agent/intent_classifier.rs` - Intent classification integrated into executor
- `task_orchestrator.rs` - Task orchestration integrated into executor
- `tools/automation.rs` - Automation tools migrated to automation module

### Extension Metrics Support
- **Extension Metrics Support**: executor.rs now collects extension (Extension) metrics
- **DataSourceId Integration**: Uses type-safe DataSourceId for metrics queries
- **Unified Time-Series Database**: Uses `data/telemetry.redb` unified storage for device and extension metrics

### Planning System (v0.6.4)

The agent planner generates structured execution plans before tool calls, enabling parallel execution of independent steps.

```rust
pub enum PlanningMode {
    /// Rule-based mapping from IntentCategory (fast, zero LLM cost)
    Keyword,
    /// LLM-generated plan for complex multi-step tasks
    LLM,
}

pub struct ExecutionPlan {
    /// Steps in the plan, ordered by intended execution sequence.
    pub steps: Vec<PlanStep>,
    /// How the plan was generated.
    pub mode: PlanningMode,
}

pub struct PlanStep {
    /// Unique step identifier.
    pub id: StepId,
    /// Tool name: "device", "agent", "rule", "alert", "extension"
    pub tool_name: String,
    /// Action within the tool: "list", "get", "query", "control"
    pub action: String,
    /// Parameters for the tool call.
    pub params: serde_json::Value,
    /// Steps that must complete before this one. Empty = parallelizable.
    pub depends_on: Vec<StepId>,
    /// Human-readable description for frontend display.
    pub description: String,
}
```

**Planners**:

| Planner | Speed | LLM Cost | Best For |
|---------|-------|----------|----------|
| `KeywordPlanner` | Instant | Zero | Simple device/rule/agent queries |
| `LLMPlanner` | ~2s timeout | 1 LLM call | Complex multi-step tasks |

**PlanningCoordinator** routes between planners:
1. If confidence > `keyword_threshold` (0.8) → `KeywordPlanner`
2. If entities ≤ `max_entities_for_keyword` (3) → `KeywordPlanner`
3. Otherwise → `LLMPlanner` with structured output parsing

**WebSocket Events** for plan progress:
```rust
AgentEvent::ExecutionPlanCreated { plan }
AgentEvent::PlanStepStarted { step_id, description }
AgentEvent::PlanStepCompleted { step_id, result }
```

**Configuration**:
```rust
pub struct PlanningConfig {
    /// Enable planning stage (default: true)
    pub enabled: bool,
    /// Confidence threshold for KeywordPlanner (default: 0.8)
    pub keyword_threshold: f32,
    /// Max entities before falling back to LLM (default: 3)
    pub max_entities_for_keyword: usize,
    /// Timeout for LLM planner call in seconds (default: 2)
    pub llm_timeout_secs: u64,
}
```

### EntityResolver (v0.6.4)

Fuzzy entity name/ID matching for all LLM tool parameters. Reduces tool round-trips by resolving human-readable names to internal IDs.

```rust
use crate::toolkit::resolver::EntityResolver;

// Resolve a user-provided name to an entity ID
let device_id = EntityResolver::resolve(
    "temperature sensor",           // user input
    &candidates,                    // Vec<(id, name)>
    "device"                        // entity type for error messages
)?;
```

**Matching strategy** (in order):
1. **Exact ID match** — input matches a candidate ID
2. **Exact name match** — case-insensitive name comparison
3. **Substring match** — input is a substring of name or ID

Returns the matched ID, or an error with helpful suggestions if ambiguous.

### Device Info Enrichment (v0.6.4)

Device query results now include:
- **Live metrics** — latest telemetry values embedded in device info
- **Available commands** — device-specific control options
- **Metric name resolution** — user-friendly aliases mapped to internal metric names

This reduces the need for follow-up tool calls to get device details.

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

**Important Change**: AgentExecutor now uses `data/telemetry.redb` (unified time-series database).

This enables Agent to access:
- Device telemetry data (written via DeviceService)
- Extension metrics data (written via ExtensionMetricsStorage)
- Transform metrics data

```rust
// crates/neomind-api/src/server/types.rs
let time_series_store = match neomind_storage::TimeSeriesStore::open("data/telemetry.redb") {
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
