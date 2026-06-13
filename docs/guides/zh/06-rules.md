# Rules 模块

**包名**: `neomind-rules`
**版本**: 0.8.0
**用途**: 基于 JSON 的规则引擎，支持事件驱动评估

## 概述

Rules 模块实现了基于 JSON 的规则引擎，支持设备和扩展指标条件、事件驱动评估、上下文感知验证，以及三种动作类型（notify、execute、trigger_agent）。

## 模块结构

```
crates/neomind-rules/src/
├── lib.rs                      # 公共接口和重导出
├── models.rs                   # 核心数据模型（CompiledRule、条件、动作）
├── engine.rs                   # 规则评估引擎
├── preview.rs                  # 人类可读预览生成（只读）
├── validator.rs                # 规则验证
├── store.rs                    # 规则持久化（redb）
├── device_integration.rs       # 设备动作执行
├── extension_integration.rs    # 扩展动作执行
├── unified_provider.rs         # 统一值提供者
└── error.rs                    # 错误类型
```

## 规则 JSON 格式

### 规则结构

```json
{
  "name": "<规则名称>",
  "description": "<可选描述>",
  "enabled": true,
  "trigger": {"trigger_type": "data_change"},
  "condition": { "<条件类型>": "..." },
  "for_duration": <毫秒，可选>,
  "cooldown": <毫秒，默认 60000>,
  "actions": [ { "<类型>": "..." } ]
}
```

### 完整示例

```bash
# 简单比较规则（默认启用）
neomind rule create --json '{
  "name": "Temperature Alert",
  "condition": {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 50},
  "actions": [{"type": "notify", "message": "温度过高: {value}C", "severity": "critical"}]
}'

# 带持续时间的规则（条件须保持 5 分钟）
neomind rule create --json '{
  "name": "Sustained High Temperature",
  "condition": {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 30},
  "for_duration": 300000,
  "actions": [
    {"type": "notify", "message": "高温持续 5 分钟", "severity": "warning"},
    {"type": "execute", "target": "fan", "target_type": "device", "command": "set_speed", "params": {"speed": 100}}
  ]
}'

# 扩展指标规则
neomind rule create --json '{
  "name": "Weather Alert",
  "condition": {"condition_type": "comparison", "source": "extension:weather:temperature", "operator": "greater_than", "threshold": 30},
  "actions": [{"type": "notify", "message": "天气过热", "severity": "warning"}]
}'

# 带 AND 的复杂规则
neomind rule create --json '{
  "name": "Compound Alert",
  "condition": {
    "condition_type": "logical", "operator": "and",
    "conditions": [
      {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 30},
      {"condition_type": "comparison", "source": "extension:weather:humidity", "operator": "less_than", "threshold": 20}
    ]
  },
  "actions": [{"type": "notify", "message": "温度高且湿度低", "severity": "warning"}]
}'

# 范围条件
neomind rule create --json '{
  "name": "Temperature Range",
  "condition": {"condition_type": "range", "source": "device:sensor:temperature", "min": 20, "max": 25},
  "actions": [{"type": "notify", "message": "温度在舒适范围内", "severity": "info"}]
}'

# 定时规则（无需条件）
neomind rule create --json '{
  "name": "Periodic Check",
  "trigger": {"trigger_type": "schedule", "cron": "0 */5 * * *"},
  "actions": [{"type": "execute", "target": "sensor-controller", "target_type": "device", "command": "read_sensors", "params": {}}]
}'

# Agent 触发规则
neomind rule create --json '{
  "name": "Auto Analysis",
  "condition": {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 40},
  "actions": [{"type": "trigger_agent", "agent_id": "analyzer", "input": "检查温度异常"}]
}'
```

## 核心类型

### 1. CompiledRule - 完整规则定义

```rust
pub struct CompiledRule {
    pub id: RuleId,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub tags: Vec<String>,
    pub trigger: RuleTrigger,
    pub condition: Option<RuleCondition>,
    pub actions: Vec<RuleAction>,
    pub cooldown: Duration,
    pub for_duration: Option<Duration>,
    pub state: RuleState,
    pub dsl_preview: String,
    pub source: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 2. RuleTrigger - 触发类型

```rust
pub enum RuleTrigger {
    DataChange { sources: Vec<DataSourceId> },
    Schedule { cron: String },
    Manual,
}
```

### 3. RuleCondition - 3 种条件类型

```rust
pub enum RuleCondition {
    Comparison {
        source: DataSourceId,
        operator: ComparisonOperator,
        threshold: f64,
    },
    Range {
        source: DataSourceId,
        min: f64,
        max: f64,
    },
    Logical {
        operator: LogicalOperator,  // And | Or | Not
        conditions: Vec<RuleCondition>,
    },
}
```

```rust
pub enum ComparisonOperator {
    GreaterThan,    // >
    LessThan,       // <
    GreaterEqual,   // >=
    LessEqual,      // <=
    Equal,          // ==
    NotEqual,       // !=
}
```

### 4. RuleAction - 3 种动作类型

```rust
pub enum RuleAction {
    Notify {
        message: String,
        severity: NotifySeverity,  // Info | Warning | Critical | Emergency
    },
    Execute {
        target: String,
        target_type: ExecuteTarget,  // Device | Extension
        command: String,
        params: serde_json::Value,
    },
    TriggerAgent {
        agent_id: String,
        input: Option<String>,
        data: Option<serde_json::Value>,
    },
}
```

## 规则引擎

```rust
pub struct RuleEngine {
    store: Arc<RuleStore>,
    value_provider: Arc<dyn ValueProvider>,
    device_executor: Option<Arc<DeviceActionExecutor>>,
    extension_executor: Option<Arc<ExtensionActionExecutor>>,
    message_manager: Option<Arc<MessageManager>>,
    agent_trigger: Option<AgentTriggerCallback>,
}
```

引擎在数据更新时（`on_data_update`）评估规则，使用订阅索引将 DataSourceId 映射到相关规则。冷却时间通过 `try_claim_cooldown()` 原子性强制执行，防止 TOCTOU 竞争。

### 规则执行结果

```rust
pub struct RuleExecutionResult {
    pub rule_id: RuleId,
    pub rule_name: String,
    pub success: bool,
    pub actions_executed: Vec<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub triggered_at: DateTime<Utc>,
}
```

## API 端点

```
# 规则 CRUD
GET    /api/rules                           # 列出规则
POST   /api/rules                           # 创建规则（JSON body）
GET    /api/rules/:id                       # 获取规则
PUT    /api/rules/:id                       # 更新规则（JSON body）
DELETE /api/rules/:id                       # 删除规则
POST   /api/rules/:id/enable                # 启用/禁用规则

# 规则操作
POST   /api/rules/:id/test                  # 测试规则
GET    /api/rules/:id/history               # 规则执行历史
```

## 使用示例

### 创建规则

```bash
curl -X POST http://localhost:9375/api/rules \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Temperature Alert",
    "condition": {"condition_type": "comparison", "source": "device:sensor:temperature", "operator": "greater_than", "threshold": 30},
    "actions": [{"type": "notify", "message": "高温", "severity": "warning"}]
  }'
```

### 测试规则

```bash
curl -X POST http://localhost:9375/api/rules/<rule-id>/test \
  -H "Content-Type: application/json" \
  -d '{"test_value": 35}'
```

## 设计原则

1. **JSON 优先**: 纯 JSON 规则定义 — 无需 DSL 解析
2. **事件驱动**: 规则通过订阅索引在数据变化时评估
3. **可组合**: 逻辑条件支持 AND/OR/NOT 组合
4. **可扩展**: 支持设备、扩展和定时触发
5. **Agent 集成**: 规则可触发 AI Agent 进行复杂分析
6. **冷却安全**: 原子性冷却声明防止并发触发竞争
