# Agent 模块

**包名**: `neomind-agent`
**版本**: 0.8.0
**完成度**: 95%
**用途**: AI会话代理，集成LLM、内存和工具

## 概述

Agent模块实现了NeoMind的核心AI代理，负责处理用户对话、调用工具、管理会话、执行自主决策等。

## 模块结构

```
crates/neomind-agent/src/
├── lib.rs                      # 公开接口
├── agent/
│   ├── mod.rs                  # Agent核心实现
│   ├── types.rs                # Agent类型定义
│   ├── cache.rs                # 缓存管理
│   ├── fallback.rs             # 降级规则
│   ├── planner/                # 执行计划生成 (v0.6.4)
│   │   ├── mod.rs              #   规划模块入口
│   │   ├── types.rs            #   PlanStep, ExecutionPlan, PlanningConfig
│   │   ├── keyword.rs          #   KeywordPlanner（基于规则，零LLM开销）
│   │   ├── llm_planner.rs      #   LLMPlanner（结构化输出解析）
│   │   └── coordinator.rs      #   PlanningCoordinator（路由选择规划器）
│   ├── scheduler.rs            # 调度器
│   ├── streaming.rs            # 流式响应（包含只读死胡同检测）
│   └── tokenizer.rs            # 分词器
├── ai_agent/
│   ├── mod.rs                  # 自主Agent
│   ├── executor/               # 执行器模块
│   │   ├── mod.rs              #   执行器核心（支持设备/扩展指标采集）
│   │   └── memory.rs           #   执行器内存集成
│   └── intent_parser.rs        # 意图解析
├── tools/
│   ├── mod.rs                  # Agent工具包装（事件集成）
│   ├── event_integration.rs    #   带事件总线追踪的工具执行
│   ├── interaction.rs          #   AskUser、ClarifyIntent、ConfirmAction工具
│   ├── mapper.rs               #   工具名称映射和参数解析
│   ├── think.rs                #   ThinkTool推理工具
│   └── tool_search.rs          #   ToolSearchTool工具查找
├── toolkit/
│   ├── mod.rs                  # 工具包模块
│   ├── tool.rs                 #   Tool trait 和 ToolDefinition
│   ├── registry.rs             #   ToolRegistry 和 ToolRegistryBuilder
│   ├── resolver.rs             #   EntityResolver（模糊名称/ID匹配）(v0.6.4)
│   ├── shell.rs                #   Shell命令执行 (v0.6.10)
│   ├── skill_tool.rs           #   技能管理工具 (v0.6.10)
│   ├── extension_tools.rs      #   扩展工具生成器和执行器
│   ├── session_search.rs       #   会话搜索工具
│   ├── time_utils.rs           #   时间范围解析工具
│   └── error.rs                #   工具错误类型
├── skills/                     # 技能系统 (v0.6.10)
│   ├── mod.rs                  #   技能模块
│   ├── types.rs                #   技能数据类型
│   ├── parser.rs               #   YAML前置信息 + Markdown解析器
│   ├── matcher.rs              #   关键词匹配器
│   ├── registry.rs             #   技能注册表（CRUD + 持久化）
│   └── builtins/               #   内置技能定义
├── prompts/
│   └── builder.rs              # 提示词构建器
├── config/
│   └── mod.rs                  # 配置
├── context/                    # 上下文管理
├── context_selector.rs         # 上下文选择器
├── error.rs                    # 错误类型
├── hooks/                      # Hook系统
├── llm.rs                      # LLM集成
├── memory/                     # 记忆系统（详见 08-memory.md）
├── memory_extraction.rs        # 从对话中提取记忆
├── session.rs                  # 会话管理
├── smart_conversation.rs       # 智能对话功能
└── translation.rs              # 翻译
```

## 重要变更 (v0.6.x - v0.8.0)

### 聚合工具（Token 优化）
智能体现在使用**聚合工具**代替独立的工具函数，显著减少函数调用中的 token 消耗：

```rust
// 旧方式：~50 个独立工具定义（~3000 tokens）
tools: [query_device, control_device, create_rule, update_rule, ...]

// 新方式：~8 个聚合工具定义（~800 tokens）
tools: [device_tools, automation_tools, system_tools, ...]
```

优势：
- **减少 60%+ 的 token 消耗**
- 更快的 LLM 推理
- 更清晰的工具组织

### 执行模式
智能体支持两种执行模式（v0.6.10 中由 Chat/React 重命名）：

```rust
pub enum ExecutionMode {
    /// 聚焦模式 — 用户定义范围，单次分析，绑定资源。
    /// 设置 enable_tool_chaining 后支持工具调用。
    #[serde(rename = "focused", alias = "chat")]
    Focused,
    /// 自由模式 — LLM 自由探索，完整工具访问，多轮推理。
    /// 支持最多 max_chain_depth 轮工具链调用。
    #[serde(rename = "free", alias = "react")]
    Free,
}
```

**聚焦模式（Focused）**：
- 用户绑定资源（必需）— 智能体在定义范围内工作
- 结构化 Markdown 表格（数据表 + 命令表 + 决策模板）确保 LLM 输出可靠
- 单次执行，token 高效
- 范围校验：超出绑定资源的命令会被拒绝
- 适用场景：监控、告警、数据分析

**自由模式（Free）**：
- 无需绑定资源 — LLM 自由使用所有工具
- 多轮推理，支持最多 8 种工具（设备、Agent、规则、消息、扩展、转换、技能、Shell）
- 适用场景：复杂自动化、设备控制、探索性任务

**向后兼容**：旧值 `"chat"` 和 `"react"` 仍通过 serde alias 接受。

### 每步结果
智能体执行现在捕获每步结果，提升可观测性：

```rust
pub struct StepResult {
    pub step_number: u32,
    pub action: String,
    pub result: String,
    pub duration_ms: u64,
}
```

### LLM 后端解耦
智能体 LLM 后端与聊天模型选择**解耦**：
- 更改聊天模型不再覆盖智能体 LLM 后端
- 智能体可使用不同的 LLM 后端用于不同用途
- 提取和执行使用独立配置

### 移除的模块 (v0.5.x)
- `agent/intent_classifier.rs` - 意图分类已整合到executor
- `task_orchestrator.rs` - 任务编排已整合到executor
- `tools/automation.rs` - 自动化工具已迁移到automation模块

### 扩展指标支持
- **扩展指标支持**: executor.rs现在可以采集扩展(Extension)指标
- **DataSourceId集成**: 使用类型安全的DataSourceId进行指标查询
- **统一时序数据库**: 使用`data/telemetry.redb`统一存储设备和扩展指标

### 规划系统 (v0.6.4)

Agent规划器在工具调用前生成结构化执行计划，支持独立步骤的并行执行。

```rust
pub enum PlanningMode {
    /// 基于IntentCategory的规则映射（快速，零LLM开销）
    Keyword,
    /// LLM生成的计划，用于复杂多步任务
    LLM,
}

pub struct ExecutionPlan {
    /// 计划中的步骤，按预期执行顺序排列
    pub steps: Vec<PlanStep>,
    /// 计划的生成方式
    pub mode: PlanningMode,
}

pub struct PlanStep {
    /// 唯一步骤标识符
    pub id: StepId,
    /// 工具名称："device", "agent", "rule", "alert", "extension"
    pub tool_name: String,
    /// 工具内的动作："list", "get", "query", "control"
    pub action: String,
    /// 工具调用参数
    pub params: serde_json::Value,
    /// 必须在此之前完成的步骤。空 = 可并行
    pub depends_on: Vec<StepId>,
    /// 人类可读的描述，用于前端显示
    pub description: String,
}
```

**规划器**:

| 规划器 | 速度 | LLM开销 | 适用场景 |
|--------|------|---------|----------|
| `KeywordPlanner` | 即时 | 零 | 简单的设备/规则/Agent查询 |
| `LLMPlanner` | ~2秒超时 | 1次LLM调用 | 复杂的多步任务 |

**PlanningCoordinator** 路由逻辑：
1. 置信度 > `keyword_threshold`（0.8）→ `KeywordPlanner`
2. 实体数 ≤ `max_entities_for_keyword`（3）→ `KeywordPlanner`
3. 否则 → `LLMPlanner` 结构化输出解析

**WebSocket事件** 用于计划进度：
```rust
AgentEvent::ExecutionPlanCreated { plan }
AgentEvent::PlanStepStarted { step_id, description }
AgentEvent::PlanStepCompleted { step_id, result }
```

**配置**:
```rust
pub struct PlanningConfig {
    /// 启用规划阶段（默认：true）
    pub enabled: bool,
    /// KeywordPlanner的置信度阈值（默认：0.8）
    pub keyword_threshold: f32,
    /// 回退到LLM规划器前的最大实体数（默认：3）
    pub max_entities_for_keyword: usize,
    /// LLM规划器调用超时（秒，默认：2）
    pub llm_timeout_secs: u64,
}
```

### EntityResolver 实体解析器 (v0.6.4)

所有LLM工具参数的模糊实体名称/ID匹配。通过将人类可读名称解析为内部ID，减少工具往返次数。

```rust
use crate::toolkit::resolver::EntityResolver;

// 将用户提供的名称解析为实体ID
let device_id = EntityResolver::resolve(
    "温度传感器",                   // 用户输入
    &candidates,                    // Vec<(id, name)>
    "device"                        // 实体类型（用于错误信息）
)?;
```

**匹配策略**（按顺序）：
1. **精确ID匹配** — 输入与候选ID完全匹配
2. **精确名称匹配** — 不区分大小写的名称比较
3. **子串匹配** — 输入是名称或ID的子串

返回匹配的ID，如果存在歧义则返回带有建议的错误信息。

### 设备信息增强 (v0.6.4)

设备查询结果现在包含：
- **实时指标** — 最新遥测值嵌入设备信息中
- **可用命令** — 设备特定的控制选项
- **指标名称解析** — 用户友好的别名映射到内部指标名称

这减少了获取设备详情所需的后续工具调用。

### 技能系统 (v0.6.10)

技能系统支持用户自定义的场景驱动操作指南，供 AI 智能体使用。技能由 YAML 前置信息 + Markdown 文件组成，提供多工具工作流指令。

```rust
// 技能工具操作：
// - search: 按关键词搜索技能
// - list: 列出所有技能
// - get: 获取技能内容
// - create: 创建新技能
// - update: 更新已有技能
// - delete: 删除技能
```

技能通过 `skill` 聚合工具管理。前端在智能体设置中提供技能面板，支持使用代码编辑器创建、编辑和删除技能。技能包含关键词匹配、token 预算注入和持久化等功能。

### Shell 工具 (v0.6.10)

`shell` 工具使 AI 智能体能够在主机上执行系统命令。

功能特性：
- **登录 Shell**: 使用 `$SHELL -l -c` 获取完整用户环境（PATH、别名）；在最小环境（Docker、IoT 边缘）中回退到 `/bin/sh -c`
- **跨平台**: 支持 Unix/macOS/Windows
- **可配置超时**: 最大 600 秒，默认 30 秒
- **输出截断**: 10K 字符限制，UTF-8 安全截断
- **进程组隔离**: 通过进程组实现干净的超时终止

参数说明：
- `command`（必需）: 要执行的 Shell 命令
- `timeout`: 执行超时时间（秒）
- `working_dir`: 工作目录
- `description`: 审计日志描述

### Agent 状态同步 (v0.6.12)

智能体的暂停/激活操作现在能正确与调度器同步。暂停智能体会将其从执行器中取消调度；激活智能体会重新调度。这确保了 UI 状态与后端执行状态一致。

### 流式处理改进 (v0.7.0+)

**只读死胡同检测**：流式处理模块检测用户请求执行操作（创建、删除、控制等）但 LLM 仅执行了只读命令（list、get、query）的情况。此时会注入强制续行提示，使 LLM 完成请求的操作。

`streaming.rs` 中的关键函数：
- `user_message_requires_action()` — 检测用户消息中的动作动词（中英文）
- `all_tools_were_read_only()` — 检查所有已执行命令是否为只读
- `extract_action_hint()` — 提取请求操作的提示信息

**错误恢复提示**：Shell 工具检测失败的 CLI 命令，并在响应中通过 `suggestion` 字段追加领域相关的恢复提示，引导 LLM 正确重试。

**最大工具迭代次数**：默认值从 10 提高到 20，允许更多多步骤工具调用以支持复杂工作流。

**思考模型修复**：对于非聊天 LLM 调用（记忆提取、压缩），将 `thinking_enabled` 设为 `false`，避免在思考模型（qwen3.x、deepseek-r1）上浪费 token。

## 核心组件

### 1. Agent - 核心代理

```rust
pub struct Agent {
    /// Agent配置
    config: AgentConfig,

    /// LLM后端
    llm: Arc<dyn LlmRuntime>,

    /// 工具注册表
    tools: Arc<ToolRegistry>,

    /// 状态机
    state_machine: StateMachine,

    /// Hook链
    hooks: HookChain,

    /// 短期内存
    short_term_memory: ShortTermMemory,
}

pub struct AgentConfig {
    /// LLM后端配置
    pub llm_backend: LlmBackend,

    /// 最大token数
    pub max_tokens: usize,

    /// 温度参数
    pub temperature: f32,

    /// 超时时间
    pub timeout_secs: u64,
}
```

### 2. SessionManager - 会话管理

```rust
pub struct SessionManager {
    /// 活跃会话
    sessions: HashMap<SessionId, Session>,

    /// 存储后端
    store: Arc<SessionStore>,

    /// Agent配置
    agent_config: AgentConfig,
}

impl SessionManager {
    /// 创建新会话
    pub async fn create_session(&self) -> Result<SessionId>;

    /// 处理消息
    pub async fn process_message(
        &self,
        session_id: &str,
        message: &str,
    ) -> Result<AgentResponse>;

    /// 获取历史
    pub async fn get_history(&self, session_id: &str) -> Result<Vec<Message>>;

    /// 删除会话
    pub async fn delete_session(&self, session_id: &str) -> Result<()>;
}
```

### 3. AgentResponse - 响应类型

```rust
pub struct AgentResponse {
    /// 消息内容
    pub message: AgentMessage,

    /// 使用的工具
    pub tools_used: Vec<ToolCall>,

    /// 处理时长
    pub duration_ms: u64,

    /// Token使用
    pub token_usage: TokenUsage,
}

pub struct AgentMessage {
    /// 主要内容
    pub content: String,

    /// 推理内容（thinking）
    pub thinking: Option<String>,

    /// 角色信息
    pub role: MessageRole,
}
```

## 状态机

```rust
pub enum ProcessState {
    /// 空闲状态
    Idle,

    /// 处理中
    Processing {
        stage: ProcessingStage,
        progress: f32,
    },

    /// 等待工具执行
    WaitingForTools {
        tools: Vec<String>,
    },

    /// 完成
    Completed {
        result: ProcessResult,
    },

    /// 错误
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

## Hook系统

```rust
pub trait AgentHook: Send + Sync {
    /// 前置处理（在LLM调用前）
    fn pre_process(&self, ctx: &HookContext) -> HookResult;

    /// 后置处理（在LLM调用后）
    fn post_process(&self, ctx: &HookContext, output: &str) -> HookResult;

    /// 工具调用前
    fn pre_tool(&self, ctx: &HookContext, tool: &str) -> HookResult;

    /// 工具调用后
    fn post_tool(&self, ctx: &HookContext, tool: &str, result: &ToolOutput) -> HookResult;
}

/// 内置Hook
pub enum BuiltInHook {
    /// 日志记录
    Logging(LoggingHook),

    /// 指标收集
    Metrics(MetricsHook),

    /// 内容审核
    ContentModeration(ContentModerationHook),

    /// 输入净化
    InputSanitization(InputSanitizationHook),
}
```

## 并发控制

```rust
pub struct GlobalConcurrencyLimiter {
    /// 全局限制
    max_concurrent: usize,
    /// 当前计数
    current: Arc<AtomicUsize>,
    /// 信号量
    semaphore: Arc<Semaphore>,
}

pub struct SessionConcurrencyLimiter {
    /// 每会话限制
    max_per_session: usize,
    /// 会话计数
    sessions: Arc<RwLock<HashMap<SessionId, usize>>>,
}
```

## 工具调用

```rust
pub struct ToolCall {
    /// 工具名称
    pub name: String,

    /// 参数
    pub arguments: serde_json::Value,

    /// 执行结果
    pub result: Option<ToolOutput>,

    /// 执行状态
    pub status: ToolCallStatus,
}

pub enum ToolCallStatus {
    Pending,
    Executing,
    Succeeded,
    Failed(String),
}
```

## 内置工具

### Agent专用工具 (tools/)
```rust
/// 交互工具
- AskUserTool              // 提示用户输入
- ClarifyIntentTool        // 澄清模糊意图
- ConfirmActionTool        // 确认危险操作

/// 思考工具
- ThinkTool                // 推理和思考记录

/// 工具搜索
- ToolSearchTool           // 工具查找和发现

/// 事件集成
- EventIntegratedToolRegistry  // 带事件总线追踪的工具执行
```

### 工具包工具 (toolkit/)
```rust
/// 核心工具
- ShellTool                // 通过 neomind CLI 执行系统命令 (v0.6.10)
- SkillTool                // 用户自定义技能管理 (v0.6.10)
- ExtensionTool            // 扩展工具生成器和执行器
- SessionSearchTool        // 会话历史搜索

/// 基础设施
- ToolRegistry             // 工具注册和查找
- EntityResolver           // 模糊名称/ID匹配 (v0.6.4)
- ToolNameMapper           // 工具名称映射和参数解析
```

## 技能系统（详细）

技能系统提供场景驱动的操作指南，在运行时动态注入到 LLM 提示词中。技能通过提供分步指令、CLI 命令示例和常见错误解决方案，帮助 AI 智能体正确执行复杂的多工具工作流。

### 架构概览

```
[IDENTITY] → [TOOL_STRATEGY] → [TOOL_DEFINITIONS] → [SKILL_GUIDES] → [INTENT] → [CONTEXT]
```

技能注入到提示词流水线的 `SKILL_GUIDES` 位置。系统由四个核心组件构成：

| 组件 | 文件 | 用途 |
|------|------|------|
| **Parser** | `skills/parser.rs` | 解析 YAML 前置信息 + Markdown 正文 |
| **Registry** | `skills/registry.rs` | 加载、索引和管理技能（CRUD + 持久化） |
| **Matcher** | `skills/matcher.rs` | 根据用户输入对技能评分 |
| **SkillTool** | `toolkit/skill_tool.rs` | LLM 工具，用于搜索、加载、创建和管理技能 |

### 什么是技能？

**技能**是一个带有 YAML 前置信息的 Markdown 文件，描述了 AI 智能体在特定场景下的操作指南。每个技能包含：

- **元数据**（YAML 前置信息）：ID、名称、分类、触发关键词、工具-动作目标、反触发词、优先级和 token 预算
- **正文**（Markdown）：分步指令、CLI 命令示例、常见错误及解决方案

技能具有两个作用：
1. **自动提示词注入**：当用户消息匹配到技能的触发条件时，技能指南会自动注入到 LLM 提示词中（在 token 预算内）
2. **按需加载**：LLM 可以通过 `skill` 工具主动搜索和加载技能，获取操作指导

### 技能文件格式

每个技能文件使用 YAML 前置信息后跟 Markdown 正文：

```yaml
---
id: my-custom-skill
name: 我的自定义技能
category: general          # device | rule | agent | message | extension | general
origin: user               # user | builtin（自动设置）
priority: 50               # 0-100，值越高在多个匹配时越优先
token_budget: 500          # 注入提示词的最大 token 数
triggers:
  keywords: [delete device, remove device, 删除设备]
  tool_target:
    - tool: device
      actions: [delete]
anti_triggers:
  keywords: [create device, 新建设备]
---

# 我的自定义技能指南

## 操作步骤

1. 首先，列出所有设备以找到目标 ID
2. 确认设备存在
3. 删除设备

## CLI 示例

```bash
neomind device list
neomind device delete <device_id>
```

## 常见错误

- **设备未找到**：使用 `neomind device list` 检查设备 ID
- **设备正在被规则使用**：先删除相关规则再删除设备
```

#### YAML 前置信息字段

| 字段 | 必需 | 类型 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | 是 | string | - | 唯一标识符（字母数字、`-`、`_`） |
| `name` | 是 | string | - | 人类可读的名称 |
| `category` | 否 | enum | `general` | 取值：`device`、`rule`、`agent`、`message`、`extension`、`general` |
| `origin` | 否 | enum | `user` | `builtin` 或 `user`（自动设置，无需手动指定） |
| `priority` | 否 | integer | `50` | 0-100，优先级高的技能在多匹配时更受青睐 |
| `token_budget` | 否 | integer | `500` | 注入提示词的最大 token 数 |
| `triggers.keywords` | 否 | string[] | `[]` | 触发该技能的关键词（不区分大小写） |
| `triggers.tool_target` | 否 | object[] | `[]` | 触发该技能的工具+动作对 |
| `anti_triggers.keywords` | 否 | string[] | `[]` | 排除该技能匹配的关键词 |

#### Token 预算指南

技能注入的总预算取决于模型的上下文窗口大小：

| 上下文大小 | 最大技能 Token |
|-----------|--------------|
| <= 4,000 | 400 |
| <= 8,000 | 800 |
| <= 16,000 | 4,000 |
| > 16,000 | 8,000 |

每个技能通过 `token_budget` 指定自己的预算（默认：500）。超过预算的正文内容会在最近的段落边界处截断。

### 内置技能

系统附带 10 个编译嵌入的内置技能，在启动时加载。用户创建的同 ID 技能可以覆盖内置技能。

| 技能 ID | 分类 | 说明 |
|--------|------|------|
| `device-onboarding` | device | 设备接入、MQTT broker 配置、Webhook 设置、ESP32/Python 示例 |
| `connector-management` | device | MQTT 连接器配置和管理 |
| `dashboard-management` | general | 仪表盘 CRUD、组件布局、数据绑定 |
| `rule-management` | rule | 规则 DSL 语法、触发器、动作、CRUD 操作 |
| `agent-management` | agent | AI Agent CRUD、调度、执行模式、控制 |
| `message-management` | message | 消息通道配置、发送、查询 |
| `extension-development` | extension | 扩展 SDK 使用、manifest 格式、构建和部署 |
| `transform-management` | general | 数据转换 CRUD 和配置 |
| `data-push-management` | message | 数据推送目标配置和投递管理 |
| `widget-development` | general | 自定义组件开发指南 |

### 技能匹配算法

当收到用户消息时，匹配器对所有已注册技能进行评分：

1. **关键词匹配**（每次匹配 +0.4）：用户输入中每找到一个触发关键词（不区分大小写的子串匹配），分数增加 0.4。

2. **工具-动作匹配**（工具+动作 +0.5，仅工具 +0.2）：如果工具名称和其中一个动作同时出现在用户输入中，技能获得 0.5 分。如果只有工具名称出现，获得 0.2 分。

3. **反触发排除**（每次匹配 -1.0）：如果用户输入中找到任何反触发关键词，分数减去 1.0。例如，这可以防止"删除规则"技能在用户说"创建规则"时匹配。

4. **优先级权重**（0-0.1）：将 `priority / 1000` 加到分数中，给高优先级技能一个轻微的提升。

分数 > 0 的技能按分数降序排列，在 token 预算耗尽前注入到提示词中。超出剩余预算的技能会在段落边界处截断。

### 技能发现与加载过程

```
启动时：
  1. 从编译的二进制文件加载内置技能（include_str!）
  2. 从 data/skills/*.md 加载用户技能（同 ID 覆盖内置技能）
  3. 构建关键词索引和工具-动作索引以支持快速查找

每条消息：
  1. 对所有技能针对用户输入进行评分
  2. 过滤出分数 > 0 的技能
  3. 按分数降序排列
  4. 在 token 预算内注入到提示词
```

### `skill` 工具

LLM 可以使用 `skill` 工具按需管理技能：

| 操作 | 说明 | 参数 |
|------|------|------|
| `search` | 按关键词搜索技能 | `query` 或 `id` |
| `load` | 加载技能的完整指南内容 | `id` |
| `create` | 创建新的用户技能 | `content`（YAML + Markdown） |
| `update` | 更新已有技能 | `id`、`content` |
| `delete` | 删除用户技能 | `id` |

`skill` 工具在 LLM 遇到不熟悉的领域或需要特定 CLI 命令语法时特别有用。它可以在执行操作之前搜索相关技能并加载完整指南。

### 创建自定义技能

用户技能存储在 `data/skills/*.md` 中。可以通过以下方式创建：

1. **LLM 工具调用**：智能体可以使用 `skill` 工具配合 `action: "create"` 创建技能
2. **手动创建文件**：在 `data/skills/` 目录中放置带 YAML 前置信息的 `.md` 文件
3. **前端 UI**：在智能体设置的技能面板中使用代码编辑器

用户技能会覆盖同 ID 的内置技能，允许自定义内置指南的行为。

示例：创建自定义温度监控技能：

```yaml
---
id: temperature-monitoring
name: 温度监控工作流
category: device
priority: 70
token_budget: 600
triggers:
  keywords: [temperature alert, 温度告警, monitor temperature, 监控温度]
  tool_target:
    - tool: device
      actions: [list, query, control]
    - tool: rule
      actions: [create]
anti_triggers:
  keywords: [delete, 删除]
---

# 温度监控工作流

## 设置步骤

1. 查找温度传感器设备
2. 创建带阈值的监控规则
3. 配置通知通道

## CLI 命令

```bash
# 列出温度设备
neomind device list --type temperature

# 创建温度规则（默认启用）
neomind rule create --json '{"name":"Temp Alert","condition":{"condition_type":"comparison","source":"device:sensor1:temp","operator":"greater_than","threshold":30},"actions":[{"type":"notify","message":"温度过高","severity":"warning"}]}'

# 查询最新读数
neomind device get <device_id>
```

## 常见错误
- **无数据**：检查设备是否在线并发送遥测数据
- **规则未触发**：确认指标名称与设备的数据源 ID 格式匹配
```

### 技能系统 API

技能也可以通过代码管理：

```rust
use neomind_agent::skills::{SkillRegistry, create_shared_registry, match_skills, TokenBudgetConfig};

// 创建注册表（加载内置 + 用户技能）
let registry = create_shared_registry(Some(Path::new("data")));

// 根据用户输入匹配技能
let budget = TokenBudgetConfig::for_context(8000);
let matches = match_skills(&registry.read().await, "删除规则 temp-alert", budget);

// 访问单个技能
let skill = registry.read().await.get("rule-management");
```

### Agent 调度

智能体支持三种调度模式，通过 `schedule` 配置：

| 调度类型 | 说明 | 配置 |
|---------|------|------|
| `event` | 由系统事件触发（设备数据、告警） | `schedule_type: "event"` |
| `cron` | 基于 Cron 表达式的调度 | `schedule_type: "cron"`, `cron_expression: "*/5 * * * *"` |
| `interval` | 固定间隔执行 | `schedule_type: "interval"`, `interval_seconds: 300` |

### 资源绑定

**聚焦模式**的智能体需要绑定资源，定义数据采集和分析的范围：

```rust
pub struct AgentResource {
    pub resource_type: ResourceType,  // Metric, ExtensionMetric, Device, ExtensionTool
    pub resource_id: String,          // 例如 "device:temp-sensor:temperature"
    pub name: String,                 // 显示名称
    pub config: serde_json::Value,    // 额外配置
}
```

资源绑定确保聚焦智能体仅在定义范围内操作，超出绑定资源的命令会被范围校验拒绝。

### Agent 状态管理

```rust
pub enum AgentStatus {
    Active,    // 智能体正在运行且已调度
    Paused,    // 智能体已暂停（已从执行器取消调度）
}
```

状态变更会与调度器同步：暂停会取消调度，激活会重新调度。

## AgentExecutor - 执行器

AgentExecutor是自主Agent的核心组件，负责执行Agent、采集数据、调用LLM并执行决策。

### 数据采集

AgentExecutor支持多种资源类型的数据采集：

```rust
pub enum ResourceType {
    /// 设备指标
    Metric,
    /// 扩展指标
    ExtensionMetric,
    /// 设备资源
    Device,
    /// 扩展工具
    ExtensionTool,
}
```

### DataSourceId集成

AgentExecutor使用类型安全的DataSourceId进行指标查询：

```rust
use neomind_core::datasource::DataSourceId;

// 解析DataSourceId
let ds_id = DataSourceId::new("extension:weather:temperature")?;
let device_part = ds_id.device_part();  // "extension:weather"
let metric_part = ds_id.metric_part();   // "temperature"

// 查询时序数据
let result = time_series_storage.query_latest(&device_part, &metric_part).await?;
```

### 设备指标采集

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

### 扩展指标采集

```rust
async fn collect_extension_metric_data_parallel(
    &self,
    agent: &AiAgent,
    resources: Vec<AgentResource>,
    timestamp: i64,
) -> AgentResult<Vec<DataCollected>> {
    // 使用DataSourceId的device_part和metric_part
    let device_part = ds_id.device_part();  // "extension:extension_id"
    let metric_part = ds_id.metric_part();   // metric_name

    let result = storage.query_latest(&device_part, metric_part).await?;
    // ...
}
```

### 统一时序数据库

**重要变更**: AgentExecutor现在使用`data/telemetry.redb`（统一时序数据库）。

这使得Agent可以访问：
- 设备遥测数据（通过DeviceService写入）
- 扩展指标数据（通过ExtensionMetricsStorage写入）
- 转换指标数据

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

### WebSocket事件

AgentExecutor通过EventBus发送实时事件：

```rust
// 执行开始
event_bus.publish(NeoMindEvent::AgentExecutionStarted(AgentExecutionStartedEvent {
    agent_id: agent.id.clone(),
    execution_id: execution_id.clone(),
    trigger_type: "manual".to_string(),
}));

// 执行完成
event_bus.publish(NeoMindEvent::AgentExecutionCompleted(AgentExecutionCompletedEvent {
    agent_id: agent.id.clone(),
    execution_id: execution_id.clone(),
    success: result.is_ok(),
    error: result.as_ref().err().map(|e| e.to_string()),
}));

// 思考中
event_bus.publish(NeoMindEvent::AgentThinking(AgentThinkingEvent {
    agent_id: agent.id.clone(),
    description: format!("分析{}个数据源", data_collected.len()),
}));
```

## 自主Agent

```rust
pub struct AutonomousAgent {
    /// Agent状态
    state: AgentState,

    /// 配置
    config: AutonomousConfig,

    /// LLM运行时
    llm: Arc<dyn LlmRuntime>,

    /// 决策历史
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
    /// 审查间隔（秒）
    pub review_interval_secs: u64,

    /// 决策阈值
    pub decision_threshold: f32,

    /// 最大并发决策
    pub max_concurrent_decisions: usize,
}
```

## 使用示例

### 基本对话

```rust
use neomind-agent::{SessionManager, AgentConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = SessionManager::new()?;

    // 创建会话
    let session_id = manager.create_session().await?;

    // 发送消息
    let response = manager.process_message(
        &session_id,
        "列出所有温度传感器"
    ).await?;

    println!("AI: {}", response.message.content);
    println!("工具: {:?}", response.tools_used);

    Ok(())
}
```

### 带工具的对话

```rust
use neomind-agent::{SessionManager, ToolRegistryBuilder};
use neomind-tools::{QueryDataTool, ControlDeviceTool};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建带工具的会话管理器
    let tools = ToolRegistryBuilder::new()
        .with_tool(Arc::new(QueryDataTool::mock()))
        .with_tool(Arc::new(ControlDeviceTool::mock()))
        .build();

    let manager = SessionManager::with_tools(tools)?;

    let session_id = manager.create_session().await?;

    // AI会自动调用工具
    let response = manager.process_message(
        &session_id,
        "打开客厅的灯"
    ).await?;

    // 检查调用的工具
    for tool_call in response.tools_used {
        println!("调用: {}", tool_call.name);
        println!("参数: {}", tool_call.arguments);
        println!("结果: {:?}", tool_call.result);
    }

    Ok(())
}
```

### 流式响应

```rust
use futures::StreamExt;
use neomind-agent::{SessionManager, StreamingConfig};

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
                println!("[思考] {}", content);
            }
            StreamChunk::Content(content) => {
                print!("{}", content);
                std::io::stdout().flush()?;
                full_response.push_str(&content);
            }
            StreamChunk::ToolCall(tool) => {
                println!("[工具] {}", tool.name);
            }
        }
    }

    Ok(full_response)
}
```

## 配置

```rust
pub struct AgentConfig {
    /// LLM后端
    pub llm_backend: LlmBackend,

    /// 最大消息历史
    pub max_history_messages: usize,

    /// 最大token数
    pub max_tokens: usize,

    /// 温度参数
    pub temperature: f32,

    /// 超时时间（秒）
    pub timeout_secs: u64,

    /// 是否启用流式
    pub streaming: bool,

    /// 是否启用工具调用
    pub enable_tools: bool,

    /// 并发限制
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

## 错误处理

```rust
pub enum NeoMindError {
    /// LLM错误
    Llm(LlmError),

    /// 工具错误
    Tool(ToolError),

    /// 存储错误
    Storage(StorageError),

    /// 会话不存在
    SessionNotFound(String),

    /// 超时
    Timeout,

    /// 并发限制
    ConcurrencyLimit,

    /// 其他错误
    Other(anyhow::Error),
}

pub enum FallbackAction {
    /// 重试
    Retry { max_attempts: usize },

    /// 使用默认响应
    DefaultResponse(String),

    /// 降级到简单模式
    SimplifyMode,

    /// 跳过工具调用
    SkipTools,
}
```

## 设计原则

1. **状态驱动**: 使用状态机管理Agent生命周期
2. **工具优先**: 默认启用工具调用
3. **流式输出**: 默认使用流式响应
4. **可扩展**: Hook系统支持自定义行为
5. **容错性**: 多级降级和错误恢复
