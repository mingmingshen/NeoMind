# Tools 模块

**包名**: `neomind-agent` (toolkit 和 tools 子模块)
**版本**: 0.8.0
**完成度**: 90%
**用途**: AI函数调用工具

## 概述

Tools模块实现了AI可调用的工具系统，包括设备控制、规则管理、数据分析等功能。工具组织在 `neomind-agent` 的两个子模块中：`tools/` 用于智能体专用工具包装（事件集成、交互），`toolkit/` 用于核心工具实现（shell、技能、扩展、指标）。

## 模块结构

```
crates/neomind-agent/src/
├── toolkit/                            # 核心工具实现
│   ├── mod.rs                          # 公开接口和导出
│   ├── tool.rs                         # Tool trait 和 ToolDefinition
│   ├── registry.rs                     # ToolRegistry 和 ToolRegistryBuilder
│   ├── resolver.rs                     # EntityResolver（模糊名称/ID匹配）
│   ├── shell.rs                        # Shell工具（neomind CLI执行）
│   ├── skill_tool.rs                   # 技能管理工具
│   ├── extension_tools.rs              # 扩展工具生成器和执行器
│   ├── ai_metric.rs                    # AI指标查询工具
│   ├── session_search.rs              # 会话历史搜索工具
│   ├── time_utils.rs                  # 时间范围解析工具
│   └── error.rs                        # 工具错误类型
├── tools/                              # 智能体工具包装
│   ├── mod.rs                          # 公开接口和导出
│   ├── event_integration.rs            # 带事件总线追踪的工具执行
│   ├── interaction.rs                  # AskUser、ClarifyIntent、ConfirmAction
│   ├── mapper.rs                       # 工具名称映射和参数解析
│   ├── think.rs                        # ThinkTool推理工具
│   └── tool_search.rs                  # ToolSearchTool工具查找
```

## 核心Trait

### Tool - 工具接口

```rust
pub trait Tool: Send + Sync {
    /// 获取工具定义
    fn definition(&self) -> &ToolDefinition;

    /// 执行工具
    fn execute(&self, input: &serde_json::Value) -> Result<ToolOutput>;

    /// 验证输入
    fn validate(&self, input: &serde_json::Value) -> Result<()> {
        // 默认实现：基于schema验证
    }

    /// 获取工具schema（用于LLM）
    fn schema(&self) -> serde_json::Value {
        // 返回OpenAI function calling格式
    }
}
```

### ToolDefinition - 工具定义

```rust
pub struct ToolDefinition {
    /// 工具名称（唯一标识）
    pub name: String,

    /// 显示名称
    pub display_name: String,

    /// 工具描述（给AI看）
    pub description: String,

    /// 参数定义
    pub parameters: Vec<Parameter>,

    /// 返回值描述
    pub returns: Option<String>,

    /// 使用示例
    pub examples: Vec<ToolExample>,
}

pub struct Parameter {
    /// 参数名称
    pub name: String,

    /// 参数类型
    pub param_type: ParameterType,

    /// 描述
    pub description: String,

    /// 是否必需
    pub required: bool,

    /// 默认值
    pub default: Option<serde_json::Value>,

    /// 枚举值
    pub enum_values: Option<Vec<String>>,
}

pub enum ParameterType {
    String,
    Number,
    Integer,
    Boolean,
    Array,
    Object,
}
```

### ToolOutput - 输出

```rust
pub struct ToolOutput {
    /// 是否成功
    pub success: bool,

    /// 输出数据
    pub data: serde_json::Value,

    /// 错误信息（如果失败）
    pub error: Option<String>,

    /// 元数据
    pub metadata: HashMap<String, String>,
}
```

## 工具注册表

```rust
pub struct ToolRegistry {
    /// 工具映射
    tools: HashMap<String, Arc<dyn Tool>>,

    /// 执行历史
    history: Arc<RwLock<ToolExecutionHistory>>,
}

impl ToolRegistry {
    /// 创建空注册表
    pub fn new() -> Self;

    /// 添加工具
    pub fn register(&mut self, tool: Arc<dyn Tool>);

    /// 批量添加工具
    pub fn register_all(&mut self, tools: Vec<Arc<dyn Tool>>);

    /// 获取工具
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>>;

    /// 列出所有工具
    pub fn list(&self) -> Vec<String>;

    /// 执行工具
    pub async fn execute(
        &self,
        name: &str,
        input: &serde_json::Value,
    ) -> Result<ToolOutput>;

    /// 格式化为LLM格式
    pub fn format_for_llm(&self) -> serde_json::Value;
}

pub struct ToolRegistryBuilder {
    registry: ToolRegistry,
}

impl ToolRegistryBuilder {
    pub fn new() -> Self;

    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self;

    pub fn with_standard_tools(self) -> Self;

    pub fn build(self) -> ToolRegistry;
}
```

## 内置工具

### Shell 工具（CLI驱动）

主要工具接口使用 `shell` 工具执行 `neomind` CLI 命令。LLM 构建 CLI 命令，shell 工具将其路由到进程内 CLI 操作。

```rust
/// Shell 工具，用于 neomind CLI 执行
pub struct ShellTool;

// 10 个 CLI 域：device、dashboard、rule、extension、widget、
//   transform、agent、message、system、data-push
// 输入: { "command": "neomind device list --type sensor" }
// 输出: { "success": true, "data": {...}, "suggestion": null }
```

**CLI 命令参考**（嵌入工具描述中供 LLM 发现）：

| 域 | 操作 |
|----|------|
| `device` | list, get, create, update, delete, control |
| `dashboard` | list, get, create, update, delete |
| `rule` | list, get, create, update, delete, enable, disable |
| `extension` | list, get, install, uninstall, enable, disable |
| `widget` | list, get, create, update, delete |
| `transform` | list, get, create, update, delete |
| `agent` | list, get, create, update, delete, status, executions |
| `message` | list, get, send, channel-list, channel-create, channel-update, channel-delete |
| `system` | info |
| `data-push` | list, get, create, update, delete |

### 技能工具

```rust
/// 用户自定义技能管理
pub struct SkillTool;

// 操作：search、list、get、create、update、delete
// 输入: { "action": "search", "keyword": "temperature" }
```

### 扩展工具

```rust
/// 扩展工具生成器和执行器
pub struct ExtensionTool;
pub struct ExtensionToolGenerator;

// 从扩展清单动态生成工具定义
// 通过扩展运行器执行扩展工具调用
```

### AI 指标工具

```rust
/// AI 指标查询工具
pub struct AiMetricTool;
pub struct AiMetricsRegistry;

// 查询设备和扩展的时序指标
// 输入: { "data_source_id": "device:sensor_1:temperature", "time_range": "1h" }
```

### 会话搜索工具

```rust
/// 会话历史搜索
pub struct SessionSearchTool;

// 搜索对话历史
// 输入: { "query": "温度告警", "limit": 5 }
```

### 设备工具

```rust
/// 列出设备
pub struct ListDevicesTool {
    device_service: Arc<DeviceService>,
}

// 输入: { "device_type": "sensor" }（可选过滤）
// 输出: { "devices": [...] }

/// 查询设备数据
pub struct QueryDataTool {
    device_service: Arc<DeviceService>,
    time_series: Arc<TimeSeriesStore>,
}

// 输入: { "device_id": "sensor_1", "metric": "temperature" }
// 输出: { "current": 25.5, "history": [...] }

/// 控制设备
pub struct ControlDeviceTool {
    device_service: Arc<DeviceService>,
}

// 输入: { "device_id": "relay_1", "command": "turn_on" }
// 输出: { "success": true, "result": ... }
```

### 规则工具

```rust
/// 列出规则
pub struct ListRulesTool {
    rule_service: Arc<RuleService>,
}

/// 创建规则
pub struct CreateRuleTool {
    rule_service: Arc<RuleService>,
    parser: RuleParser,
}
```

### Agent工具

```rust
/// 列出Agent
pub struct ListAgentsTool {
    agent_service: Arc<AgentService>,
}
```

## 工具执行历史

```rust
pub struct ToolExecutionHistory {
    /// 执行记录
    records: Vec<ToolExecutionRecord>,
}

pub struct ToolExecutionRecord {
    /// 执行ID
    pub id: String,

    /// 工具名称
    pub tool_name: String,

    /// 输入参数
    pub input: serde_json::Value,

    /// 输出结果
    pub output: Option<ToolOutput>,

    /// 执行时间
    pub executed_at: i64,

    /// 耗时（毫秒）
    pub duration_ms: u64,

    /// 是否成功
    pub success: bool,
}

pub struct ToolExecutionStats {
    /// 总执行次数
    pub total_executions: usize,

    /// 成功次数
    pub success_count: usize,

    /// 失败次数
    pub failure_count: usize,

    /// 平均耗时
    pub avg_duration_ms: f64,

    /// 最常用工具
    pub most_used_tools: Vec<(String, usize)>,
}
```

## 简化接口

```rust
/// 简化的工具定义（用于前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolDefinition {
    /// 工具名称
    pub name: String,

    /// 显示名称
    pub display_name: String,

    /// 描述
    pub description: String,

    /// 参数schema（JSON Schema格式）
    pub parameters: serde_json::Value,
}

/// 格式化工具为LLM格式
pub fn format_tools_for_llm(tools: &[Arc<dyn Tool>]) -> Vec<LlmToolDefinition> {
    tools.iter().map(|tool| {
        let def = tool.definition();
        LlmToolDefinition {
            name: def.name.clone(),
            display_name: def.display_name.clone(),
            description: def.description.clone(),
            parameters: tool.schema(),
        }
    }).collect()
}
```

## API端点

```
# Tools
GET    /api/tools                           # 列出工具
GET    /api/tools/:name/schema              # 获取工具schema
POST   /api/tools/:name/execute             # 执行工具
GET    /api/tools/format-for-llm            # 格式化为LLM格式
GET    /api/tools/metrics                   # 工具执行统计
```

## 使用示例

### 创建工具注册表

```rust
use neomind_agent::toolkit::{ToolRegistryBuilder, ShellTool, SkillTool};
use std::sync::Arc;

let registry = ToolRegistryBuilder::new()
    .with_tool(Arc::new(ShellTool::new(shell_config)))
    .with_tool(Arc::new(SkillTool::new(skill_registry)))
    .build();
```

### 通过 Shell 执行工具

```rust
use neomind_agent::toolkit::ToolRegistry;

let result = registry.execute(
    "shell",
    serde_json::json!({
        "command": "neomind device list --type sensor"
    }),
).await?;

if result.success {
    println!("结果: {}", result.data);
} else {
    eprintln!("错误: {}", result.error.unwrap());
}
```

## 设计原则

1. **接口统一**: 所有工具实现相同的Trait
2. **类型安全**: 输入输出使用强类型
3. **CLI优先**: 工具通过 shell 工具使用 `neomind` CLI 命令，保持一致性
4. **LLM友好**: 标准化的函数调用格式，嵌入 CLI 参考供 LLM 发现
5. **可追踪**: 通过事件总线集成记录所有执行历史
6. **错误恢复**: 失败的 CLI 命令返回领域相关的恢复提示
