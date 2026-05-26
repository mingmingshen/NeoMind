# Rules 模块

**包名**: `neomind-rules`
**版本**: 0.8.0
**完成度**: 85%
**用途**: 带LLM生成的DSL规则引擎

## 概述

Rules模块实现了基于DSL（领域特定语言）的规则引擎，支持设备和扩展指标条件、基于LLM的自然语言规则生成、上下文感知验证和多种动作类型。

## 模块结构

```
crates/neomind-rules/src/
├── lib.rs                      # 公开接口与重导出
├── dsl.rs                      # DSL解析器和类型
├── engine.rs                   # 规则评估引擎
├── generator.rs                # 基于LLM的自然语言规则生成
├── validator.rs                # 上下文感知的规则验证
├── store.rs                    # 规则持久化（redb）
├── history.rs                  # 规则执行历史
├── dependencies.rs             # 依赖管理
├── device_integration.rs       # 设备动作执行
├── extension_integration.rs    # 扩展动作执行
├── unified_provider.rs         # 统一值提供者
└── error.rs                    # 错误类型
```

## DSL语法

### 规则结构

```neo
RULE "<名称>"
[TRIGGER SCHEDULE "<cron>"]
WHEN <条件>
[FOR <持续时间>]
DO
    <动作>
    [<动作> ...]
END
```

### 完整示例

```neo
# 简单设备规则
RULE "温度告警"
WHEN sensor.temperature > 50
DO
    NOTIFY "设备温度过高: {temperature}C"
END

# 带持续时间的设备规则
RULE "持续高温"
WHEN sensor.temperature > 30
FOR 5 minutes
DO
    NOTIFY "温度持续过高5分钟"
    EXECUTE device.fan(speed=100)
END

# 扩展指标规则
RULE "天气告警"
WHEN EXTENSION weather.temperature > 30
DO
    NOTIFY "天气过热"
END

# 带AND/OR的复杂规则
RULE "复合告警"
WHEN (sensor.temperature > 30) AND (EXTENSION weather.humidity < 20)
DO
    NOTIFY "高温且低湿度"
    EXECUTE device.humidifier(on=true)
END

# 范围条件
RULE "温度范围"
WHEN sensor.temperature BETWEEN 20 AND 25
DO
    NOTIFY "温度在舒适范围内"
END

# 定时规则
RULE "周期检查"
TRIGGER SCHEDULE "0 */5 * * * *"
DO
    EXECUTE device.read_sensors()
END

# Agent触发规则
RULE "自动分析"
WHEN sensor.temperature > 40
DO
    TRIGGER_AGENT "analyzer" INPUT "检查温度异常"
END
```

## 核心类型

### 1. ParsedRule - 解析后的规则定义

```rust
pub struct ParsedRule {
    /// 规则名称
    pub name: String,
    /// 要评估的条件
    pub condition: RuleCondition,
    /// 条件触发前需要持续的时长
    pub for_duration: Option<Duration>,
    /// 要执行的动作
    pub actions: Vec<RuleAction>,
    /// 描述（可选）
    pub description: Option<String>,
    /// 标签
    pub tags: Vec<String>,
    /// 触发类型
    pub trigger_type: TriggerType,
}
```

### 2. TriggerType - 触发类型

```rust
pub enum TriggerType {
    /// 设备状态变化触发（默认）
    DeviceState,
    /// Cron定时触发
    Schedule { cron: String },
    /// 手动通过API触发
    Manual,
}
```

### 3. RuleCondition - 条件定义

```rust
pub enum RuleCondition {
    /// 设备条件: device.metric 操作符 值
    Device {
        device_id: String,
        metric: String,
        operator: ComparisonOperator,
        threshold: f64,
    },
    /// 扩展条件: extension.metric 操作符 值
    Extension {
        extension_id: String,
        metric: String,
        operator: ComparisonOperator,
        threshold: f64,
    },
    /// 设备范围条件
    DeviceRange {
        device_id: String,
        metric: String,
        min: f64,
        max: f64,
    },
    /// 扩展范围条件
    ExtensionRange {
        extension_id: String,
        metric: String,
        min: f64,
        max: f64,
    },
    /// 逻辑与
    And(Vec<RuleCondition>),
    /// 逻辑或
    Or(Vec<RuleCondition>),
    /// 逻辑非
    Not(Box<RuleCondition>),
    /// 始终为真（用于定时/手动规则）
    Always,
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

### 4. RuleAction - 动作定义

```rust
pub enum RuleAction {
    /// 发送通知
    Notify {
        message: String,
        channels: Option<Vec<String>>,
    },
    /// 执行设备命令
    Execute {
        device_id: String,
        command: String,
        params: HashMap<String, serde_json::Value>,
    },
    /// 记录日志
    Log {
        level: LogLevel,
        message: String,
        severity: Option<String>,
    },
    /// 设置设备属性
    Set {
        device_id: String,
        property: String,
        value: serde_json::Value,
    },
    /// 延迟执行
    Delay { duration: Duration },
    /// 创建告警
    CreateAlert {
        title: String,
        message: String,
        severity: AlertSeverity,
    },
    /// 发送HTTP请求
    HttpRequest {
        method: HttpMethod,
        url: String,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    },
    /// 触发AI Agent
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

impl RuleEngine {
    /// 创建规则引擎
    pub fn new(value_provider: Arc<dyn ValueProvider>) -> Self;

    /// 从DSL文本添加规则
    pub async fn add_rule_from_dsl(&self, dsl: &str) -> Result<RuleId>;

    /// 启用/禁用规则
    pub async fn set_rule_enabled(&self, id: &RuleId, enabled: bool) -> Result<()>;

    /// 评估所有规则
    pub async fn evaluate_all(&self) -> Vec<RuleExecutionResult>;

    /// 获取规则状态
    pub async fn get_rule_state(&self, id: &RuleId) -> Option<RuleState>;

    /// 启动评估循环
    pub async fn start(&self) -> Result<()>;

    /// 停止评估循环
    pub async fn stop(&self) -> Result<()>;
}
```

### 规则执行结果

```rust
pub struct RuleExecutionResult {
    pub rule_id: RuleId,
    pub rule_name: String,
    pub triggered: bool,
    pub condition_met: bool,
    pub actions_executed: usize,
    pub action_results: Vec<ActionResult>,
    pub evaluation_duration: Duration,
}
```

## 规则验证

```rust
pub struct RuleValidator {
    // 根据可用资源验证规则
}

pub struct ValidationContext {
    pub devices: Vec<DeviceInfo>,
    pub metrics: Vec<MetricInfo>,
    pub commands: Vec<CommandInfo>,
    pub alert_channels: Vec<AlertChannelInfo>,
}

pub struct RuleValidationResult {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub resource_summary: ResourceSummary,
}

pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}
```

## 规则历史

```rust
pub struct RuleHistoryEntry {
    pub rule_id: String,
    pub rule_name: String,
    pub triggered_at: i64,
    pub condition_met: bool,
    pub actions_executed: usize,
    pub duration_ms: u64,
}

pub struct RuleHistoryStorage {
    db: Database,
}
```

## API端点

```
# Rules CRUD
GET    /api/rules                           # 列出规则
POST   /api/rules                           # 创建规则（需要 {"dsl": "RULE ... END"}）
GET    /api/rules/:id                       # 获取规则
PUT    /api/rules/:id                       # 更新规则
DELETE /api/rules/:id                       # 删除规则
POST   /api/rules/:id/enable                # 启用/禁用规则

# 规则操作
POST   /api/rules/:id/test                  # 测试规则
GET    /api/rules/:id/history               # 规则执行历史
POST   /api/rules/validate                  # 验证规则DSL

# 规则导入/导出
GET    /api/rules/export                    # 导出所有规则
POST   /api/rules/import                    # 导入规则

# 规则资源
GET    /api/rules/resources                 # 验证可用的资源
```

## 使用示例

### 通过DSL创建规则

```bash
curl -X POST http://localhost:9375/api/rules \
  -H "Content-Type: application/json" \
  -d '{
    "dsl": "RULE \"温度告警\" WHEN sensor.temperature > 30 DO NOTIFY \"高温\" END"
  }'
```

### 测试规则

```bash
curl -X POST http://localhost:9375/api/rules/test \
  -H "Content-Type: application/json" \
  -d '{
    "dsl": "RULE \"测试\" WHEN sensor.temperature > 30 DO NOTIFY \"高温\" END",
    "context": {
      "sensor": {
        "temperature": 35
      }
    }
  }'
```

### 验证规则

```bash
curl -X POST http://localhost:9375/api/rules/validate \
  -H "Content-Type: application/json" \
  -d '{
    "dsl": "RULE \"测试\" WHEN sensor.temperature > 30 DO NOTIFY \"高温\" END"
  }'
```

## 设计原则

1. **DSL优先**: 人类可读的规则定义语言（RULE/WHEN/DO/END）
2. **可测试**: 所有规则都可以用模拟上下文测试
3. **事件驱动**: 规则基于数据变化评估
4. **可组合**: 支持复杂条件组合（AND/OR/NOT）
5. **可扩展**: 支持设备、扩展和定时触发
6. **验证**: 上下文感知的资源验证
7. **Agent集成**: 规则可触发AI Agent进行复杂分析
