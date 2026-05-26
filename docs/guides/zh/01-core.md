# Core 模块

**包名**: `neomind-core`
**版本**: 0.8.0
**完成度**: 95%
**用途**: 定义整个项目的核心trait和类型

## 概述

Core 模块是 NeoMind 项目的基石，定义了所有其他模块依赖的核心抽象和类型。它不包含具体实现，只提供接口定义。

## 模块结构

```
crates/neomind-core/src/
├── lib.rs                  # 公开接口导出
├── brand.rs                # 品牌常量
├── event.rs                # 事件类型定义
├── eventbus.rs             # 事件总线实现
├── message.rs              # 消息类型定义
├── message/
│   └── convert.rs          # 消息转换工具
├── session.rs              # 会话类型定义
├── llm/
│   ├── backend.rs          # LLM运行时trait + BackendRegistry
│   ├── capability.rs       # 能力定义
│   ├── modality.rs         # 多模态内容支持
│   ├── memory_consolidation.rs  # 内存整合
│   ├── models.rs           # 模型定义
│   ├── compaction.rs       # 上下文压缩
│   └── token_counter.rs    # Token计数工具
├── tools/
│   └── mod.rs              # 工具trait定义
├── storage/
│   └── mod.rs              # 存储trait定义
├── datasource/
│   ├── mod.rs              # 数据源ID系统 + 类型
│   └── query.rs            # 统一查询服务
├── extension/
│   ├── mod.rs              # 扩展系统
│   ├── types.rs            # 扩展类型
│   ├── registry.rs         # 扩展注册表
│   ├── executor.rs         # 扩展执行器
│   ├── proxy.rs            # 扩展代理
│   ├── runtime.rs          # 扩展运行时
│   ├── safety.rs           # 安全/崩溃保护
│   ├── system.rs           # 扩展系统管理
│   ├── context.rs          # 扩展上下文
│   ├── package.rs          # 包管理
│   ├── stream.rs           # 流式支持
│   ├── tracing.rs          # 追踪工具
│   ├── capability_services.rs  # 能力服务
│   ├── event_dispatcher.rs     # 事件分发
│   ├── event_subscription.rs   # 事件订阅
│   ├── extension_event_subscription.rs  # 扩展事件订阅
│   ├── loader/                 # 扩展加载器
│   │   ├── mod.rs
│   │   ├── native.rs       # 原生扩展加载器
│   │   └── isolated.rs     # 隔离进程加载器
│   └── isolated/           # 进程隔离扩展
│       ├── mod.rs
│       ├── manager.rs      # 进程管理器
│       ├── process.rs      # 进程生命周期
│       ├── ipc_local.rs    # 本地IPC
│       └── in_flight.rs    # 在途请求追踪
├── error/
│   ├── mod.rs              # 错误类型
│   └── redb.rs             # Redb特定错误
├── config.rs               # 配置常量
└── macros.rs               # 宏定义
```

## 核心Trait

### 1. LlmRuntime - LLM运行时接口

定义了所有LLM后端必须实现的接口。

```rust
#[async_trait]
pub trait LlmRuntime: Send + Sync {
    /// 获取后端类型标识符
    fn backend_id(&self) -> BackendId;

    /// 获取当前模型名称
    fn model_name(&self) -> &str;

    /// 检查后端是否可用
    async fn is_available(&self) -> bool { true }

    /// 预热模型（可选，消除首次请求延迟）
    async fn warmup(&self) -> Result<(), LlmError> { Ok(()) }

    /// 生成文本（非流式）
    async fn generate(&self, input: LlmInput) -> Result<LlmOutput, LlmError>;

    /// 生成文本（流式）
    async fn generate_stream(
        &self,
        input: LlmInput,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamChunk> + Send>>, LlmError>;

    /// 获取最大上下文长度
    fn max_context_length(&self) -> usize;

    /// 估算token数量
    fn estimate_tokens(&self, text: &str) -> usize { text.len() / 4 }

    /// 是否支持多模态（视觉）
    fn supports_multimodal(&self) -> bool { false }

    /// 获取后端能力
    fn capabilities(&self) -> BackendCapabilities { BackendCapabilities::default() }

    /// 获取后端指标（如支持）
    fn metrics(&self) -> BackendMetrics { BackendMetrics::default() }
}
```

**BackendCapabilities**:
```rust
pub struct BackendCapabilities {
    /// 支持流式生成
    pub streaming: bool,
    /// 支持多模态（视觉）
    pub multimodal: bool,
    /// 支持函数调用
    pub function_calling: bool,
    /// 支持多模型
    pub multiple_models: bool,
    /// 最大上下文长度
    pub max_context: Option<usize>,
    /// 支持的模态
    pub modalities: Vec<String>,
    /// 支持thinking/推理显示
    pub thinking_display: bool,
    /// 支持图片输入
    pub supports_images: bool,
    /// 支持音频输入
    pub supports_audio: bool,
}
```

### 2. Tool - 工具接口

定义了AI可调用工具的接口。

```rust
pub trait Tool: Send + Sync {
    /// 工具定义（名称、描述、参数）
    fn definition(&self) -> &ToolDefinition;

    /// 执行工具
    fn execute(&self, input: &serde_json::Value) -> Result<ToolOutput>;

    /// 验证输入
    fn validate(&self, input: &serde_json::Value) -> Result<()> {
        // 默认实现
    }
}
```

### 3. Integration - 集成接口

定义了外部系统集成的接口。

```rust
#[async_trait]
pub trait Integration: Send + Sync {
    /// 获取元数据
    fn metadata(&self) -> &IntegrationMetadata;

    /// 获取当前状态
    fn state(&self) -> IntegrationState;

    /// 启动集成
    async fn start(&self) -> Result<()>;

    /// 停止集成
    async fn stop(&self) -> Result<()>;

    /// 订阅事件流
    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = IntegrationEvent> + Send + '_>>;

    /// 发送命令
    async fn send_command(&self, command: IntegrationCommand) -> Result<IntegrationResponse>;
}
```

### 4. Extension - 扩展接口

定义了动态加载扩展的接口。V2 Extension trait 将指标和命令分离：

```rust
#[async_trait]
pub trait Extension: Send + Sync {
    /// 获取扩展元数据
    fn metadata(&self) -> &ExtensionMetadata;

    /// 声明扩展提供的指标
    fn metrics(&self) -> &[MetricDescriptor] { &[] }

    /// 声明扩展支持的命令
    fn commands(&self) -> &[ExtensionCommand] { &[] }

    /// 执行命令（异步）
    async fn execute_command(&self, command: &str, args: &Value) -> Result<Value>;

    /// 生成指标数据（同步，兼容动态库）
    fn produce_metrics(&self) -> Result<Vec<ExtensionMetricValue>> { Ok(Vec::new()) }

    /// 健康检查（异步，可选）
    async fn health_check(&self) -> Result<bool> { Ok(true) }

    /// 运行时配置（可选）
    async fn configure(&mut self, config: &Value) -> Result<()> { Ok(()) }
}
```

完整 API 详情请参考 [扩展开发指南](16-extension-dev.md)。

### 5. DataSourceId - 数据源标识

提供类型安全的数据源标识，支持设备、扩展和转换数据源。

```rust
pub struct DataSourceId {
    /// 数据源类型
    pub source_type: DataSourceType,
    /// 数据源ID
    pub source_id: String,
    /// 字段路径
    pub field_path: String,
}

pub enum DataSourceType {
    Device,
    Extension,
    Transform,
}

impl DataSourceId {
    /// 解析数据源ID
    pub fn new(id: &str) -> Result<Self>;

    /// 获取TimeSeriesStorage使用的device_id部分
    pub fn device_part(&self) -> String;

    /// 获取TimeSeriesStorage使用的metric部分
    pub fn metric_part(&self) -> &str;

    /// 获取完整的存储键
    pub fn storage_key(&self) -> String;
}
```

**格式**: `{source_type}:{source_id}:{field_path}`

| 类型 | 格式 | device_part | metric_part |
|------|------|-------------|-------------|
| Device | `{device_id}:{field_path}` | `{device_id}` | `{field_path}` |
| Extension | `extension:{ext_id}:{field_path}` | `extension:{ext_id}` | `{field_path}` |
| Transform | `transform:{trans_id}:{field_path}` | `transform:{trans_id}` | `{field_path}` |

### 6. StorageBackend - 存储接口

定义了通用存储接口。

```rust
pub trait StorageBackend: Send + Sync {
    /// 写入数据
    async fn write(&self, key: &str, value: &[u8]) -> Result<()>;

    /// 读取数据
    async fn read(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// 删除数据
    async fn delete(&self, key: &str) -> Result<()>;

    /// 列出键
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
}
```

## 核心类型

### Message - 消息类型

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
    Parts(Vec<ContentPart>),  // 支持多模态
}

pub enum ContentPart {
    Text { text: String },
    Image { image: ImageContent },
    Thinking { content: String },  // 推理内容
}
```

### NeoMindEvent - 事件类型

```rust
pub enum NeoMindEvent {
    // 设备事件
    DeviceOnline { device_id: String, device_type: String, timestamp: i64 },
    DeviceOffline { device_id: String, reason: Option<String>, timestamp: i64 },
    DeviceMetric { device_id: String, metric: String, value: MetricValue, timestamp: i64,
                   quality: Option<f32>, is_virtual: Option<bool> },
    DeviceCommandResult { device_id: String, command: String, success: bool,
                          result: Option<serde_json::Value>, timestamp: i64 },
    DeviceDiscovered { device_id: String, source: String, adapter_id: Option<String>,
                       metadata: serde_json::Value, sample: serde_json::Value,
                       is_binary: bool, timestamp: i64 },

    // 规则事件
    RuleEvaluated { rule_id: String, rule_name: String, condition_met: bool, timestamp: i64 },
    RuleTriggered { rule_id: String, rule_name: String, trigger_value: f64,
                    actions: Vec<String>, timestamp: i64 },
    RuleExecuted { rule_id: String, rule_name: String, success: bool,
                   duration_ms: u64, timestamp: i64 },

    // 工作流事件
    WorkflowTriggered { workflow_id: String, trigger_type: String,
                        trigger_data: Option<serde_json::Value>, execution_id: String, timestamp: i64 },
    WorkflowStepCompleted { workflow_id: String, execution_id: String, step_id: String,
                            result: serde_json::Value, timestamp: i64 },
    WorkflowCompleted { workflow_id: String, execution_id: String, success: bool,
                        duration_ms: u64, timestamp: i64 },

    // 告警事件
    AlertCreated { alert_id: String, title: String, severity: String, message: String, timestamp: i64 },
    AlertAcknowledged { alert_id: String, acknowledged_by: String, timestamp: i64 },

    // 消息事件
    MessageCreated { message_id: String, title: String, severity: String, message: String, timestamp: i64 },
    MessageAcknowledged { message_id: String, acknowledged_by: String, timestamp: i64 },
    MessageResolved { message_id: String, timestamp: i64 },

    // Agent事件（用户自定义AI Agent）
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

    // LLM事件（自主Agent）
    PeriodicReviewTriggered { review_id: String, review_type: String, timestamp: i64 },
    LlmDecisionProposed { decision_id: String, title: String, description: String,
                          reasoning: String, actions: Vec<ProposedAction>,
                          confidence: f32, timestamp: i64 },
    LlmDecisionExecuted { decision_id: String, success: bool,
                          result: Option<serde_json::Value>, timestamp: i64 },

    // 用户事件
    UserMessage { session_id: String, content: String, timestamp: i64 },
    LlmResponse { session_id: String, content: String, tools_used: Vec<String>,
                  processing_time_ms: u64, timestamp: i64 },

    // 工具执行事件
    ToolExecutionStart { tool_name: String, arguments: serde_json::Value,
                         session_id: Option<String>, timestamp: i64 },
    ToolExecutionSuccess { tool_name: String, arguments: serde_json::Value, result: serde_json::Value,
                           duration_ms: u64, session_id: Option<String>, timestamp: i64 },
    ToolExecutionFailure { tool_name: String, arguments: serde_json::Value, error: String,
                           error_type: String, duration_ms: u64, session_id: Option<String>, timestamp: i64 },

    // 扩展事件（Phase 2.1）
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

    // 自定义事件（用于扩展和插件）
    Custom { event_type: String, data: serde_json::Value },
}
```

### Session - 会话类型

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

## EventBus - 事件总线

```rust
pub struct EventBus {
    // 通道容量
    capacity: usize,
    // 广播发送器
    sender: broadcast::Sender<NeoMindEvent>,
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new() -> Self;

    /// 创建带容量的事件总线
    pub fn with_capacity(capacity: usize) -> Self;

    /// 发布事件
    pub fn publish(&self, event: NeoMindEvent);

    /// 订阅所有事件
    pub fn subscribe(&self) -> EventBusReceiver;

    /// 创建过滤器
    pub fn filter(&self) -> FilterBuilder;
}
```

### FilterBuilder - 事件过滤

```rust
pub struct FilterBuilder<'a> {
    bus: &'a EventBus,
    filters: Vec<FilterFn>,
}

impl<'a> FilterBuilder<'a> {
    /// 只接收设备事件
    pub fn device_events(self) -> FilteredReceiver;

    /// 只接收规则事件
    pub fn rule_events(self) -> FilteredReceiver;

    /// 自定义过滤
    pub fn custom<F>(self, f: F) -> FilteredReceiver
    where
        F: Fn(&NeoMindEvent) -> bool + Send + 'static;
}
```

## 配置常量

```rust
// LLM提供商配置
pub const DEFAULT_OLLAMA_ENDPOINT: &str = "http://localhost:11434";
pub const DEFAULT_OPENAI_ENDPOINT: &str = "https://api.openai.com/v1";

// 默认模型
pub fn models() -> Vec<&'static str> {
    vec![
        "qwen3.5:4b",      // Ollama默认
    ]
}

// API端点
pub fn endpoints() -> HashMap<String, String> {
    // ...
}
```

## 使用示例

### 创建EventBus

```rust
use neomind-core::EventBus;
use neomind-core::NeoMindEvent;

#[tokio::main]
async fn main() {
    let bus = EventBus::new();

    // 订阅所有事件
    let mut rx = bus.subscribe();

    // 订阅设备事件
    let mut device_rx = bus.filter().device_events();

    // 发布事件
    bus.publish(NeoMindEvent::DeviceOnline {
        device_id: "sensor_1".to_string(),
        device_type: "sensor".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    });

    // 接收事件
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            println!("Received: {:?}", event);
        }
    });
}
```

### 使用LlmRuntime trait

```rust
use neomind_core::llm::backend::{LlmRuntime, LlmInput, GenerationParams};

async fn call_llm(runtime: &dyn LlmRuntime, prompt: &str) -> Result<String, LlmError> {
    let input = LlmInput::new(prompt)
        .with_params(GenerationParams::default());

    let output = runtime.generate(input).await?;
    Ok(output.text)
}
```

## 错误处理

```rust
pub enum Error {
    /// LLM相关错误
    Llm(LlmError),

    /// 存储错误
    Storage(StorageError),

    /// 工具错误
    Tool(ToolError),

    /// 集成错误
    Integration(IntegrationError),

    /// 扩展错误
    Extension(ExtensionError),

    /// IO错误
    Io(std::io::Error),

    /// 其他错误
    Other(anyhow::Error),
}
```

## 依赖关系

```
Core (neomind-core)
    │
    ├── 无外部依赖（仅定义trait）
    │
    └── 被所有其他crate依赖
        ├── llm
        ├── agent
        ├── devices
        ├── tools
        ├── storage
        ├── integrations
        └── ...
```

## 设计原则

1. **最小依赖**: Core模块不依赖任何其他业务模块
2. **trait优先**: 通过trait定义接口，允许不同实现
3. **类型安全**: 使用Rust类型系统确保正确性
4. **异步优先**: 所有I/O操作都是异步的
5. **可扩展性**: 通过trait和事件实现松耦合
