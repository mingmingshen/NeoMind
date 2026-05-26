# Automation 模块

**包名**: `neomind-rules`（规则生成）、`neomind-devices`（自动入板）
**版本**: 0.8.0
**完成度**: 85%
**用途**: 数据转换、NL规则生成、自动入板和设备类型生成

## 概述

Automation模块提供从自然语言生成规则、从示例生成设备类型、以及自动入板发现设备的功能。这些功能分布在 `neomind-rules` 和 `neomind-devices` crate中。

## 重要变更 (v0.8.0)

### 统一架构

自动化功能现在集成到核心crate中，而不是独立的 `neomind-automation` crate：
- **规则生成**: 基于LLM的规则生成位于 `neomind-rules/src/generator.rs`
- **自动入板**: 草稿设备管理位于 `neomind-devices/src/service.rs`
- **设备类型生成**: 基于LLM的类型生成位于设备API层
- **规则验证**: 上下文感知验证位于 `neomind-rules/src/validator.rs`

## 模块结构

```
crates/neomind-rules/src/
├── generator.rs                # 基于LLM的自然语言规则生成
├── validator.rs                # 带上下文感知的规则验证
├── device_integration.rs       # 规则中的设备动作执行
├── extension_integration.rs    # 规则中的扩展动作执行
└── dsl.rs                      # DSL解析器（RULE...WHEN...DO...END）

crates/neomind-devices/src/
├── service.rs                  # 带自动入板的DeviceService
├── registry.rs                 # 带类型管理的DeviceRegistry
└── adapters/                   # 带自动发现的适配器
```

## 核心功能

### 1. 自然语言规则生成

```rust
// 在 neomind-rules/src/generator.rs 中

pub struct GeneratorConfig {
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
}

/// 从自然语言描述中提取的信息
pub struct ExtractedRuleInfo {
    pub name: String,
    pub device_id: Option<String>,
    pub metric: Option<String>,
    pub operator: Option<ComparisonOperator>,
    pub threshold: Option<f64>,
    pub action_type: Option<ActionType>,
    pub message: Option<String>,
}
```

### 2. 上下文感知的规则验证

```rust
// 在 neomind-rules/src/validator.rs 中

pub struct RuleValidator {
    // 根据可用设备、指标、命令验证规则
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
```

### 3. 自动入板流程

```
适配器发现 -> 草稿设备 -> LLM分析 -> 类型建议 -> 用户批准 -> 完整设备
```

自动入板流程：
1. 适配器发现新设备并发出 `DeviceEvent::Discovery` 事件
2. 发现的设备存储为草稿设备
3. LLM分析示例数据以建议设备类型
4. 用户审查并批准/拒绝
5. 批准的草稿成为完整的设备实例

### 4. 从示例生成设备类型

```
POST /api/device-types/generate-from-samples
```

通过LLM分析从示例数据生成设备类型模板。

## 规则DSL语法

### 基本规则

```neo
RULE "温度告警"
WHEN sensor.temperature > 50
FOR 5 minutes
DO
    NOTIFY "设备温度过高: {temperature}C"
    EXECUTE device.fan(speed=100)
    LOG alert, severity="high"
END
```

### 扩展规则

```neo
RULE "天气告警"
WHEN EXTENSION weather.temperature > 30
DO
    NOTIFY "天气过热"
END
```

### 带AND/OR的复杂规则

```neo
RULE "复合条件告警"
WHEN (sensor.temperature > 30) AND (EXTENSION weather.humidity < 20)
DO
    NOTIFY "温度高且湿度低"
    EXECUTE device.humidifier(on=true)
END
```

### 范围条件规则

```neo
RULE "温度范围告警"
WHEN sensor.temperature BETWEEN 20 AND 25
DO
    NOTIFY "温度在舒适范围内"
END
```

### 定时规则

```neo
RULE "周期检查"
TRIGGER SCHEDULE "0 */5 * * * *"
DO
    EXECUTE device.read_sensors()
END
```

## API端点

```
# 规则生成
POST   /api/rules/validate                  # 验证规则DSL

# 设备类型生成
POST   /api/device-types/generate-from-samples  # 从示例生成设备类型

# 自动入板（草稿）
GET    /api/devices/drafts                      # 列出草稿
GET    /api/devices/drafts/:device_id           # 获取草稿
PUT    /api/devices/drafts/:device_id           # 更新草稿
POST   /api/devices/drafts/:device_id/approve   # 批准设备
POST   /api/devices/drafts/:device_id/reject    # 拒绝设备
POST   /api/devices/drafts/:device_id/analyze   # LLM分析
POST   /api/devices/drafts/:device_id/enhance   # LLM增强
GET    /api/devices/drafts/:device_id/suggest-types  # 建议类型
POST   /api/devices/drafts/cleanup              # 清理草稿
GET    /api/devices/drafts/type-signatures      # 获取类型签名
GET    /api/devices/drafts/config               # 获取入板配置
PUT    /api/devices/drafts/config               # 更新入板配置
POST   /api/devices/drafts/upload               # 上传设备数据
```

## 使用示例

### 自然语言规则生成

```rust
use neomind_rules::generator::GeneratorConfig;

// 生成器使用LLM将自然语言描述转换为DSL规则
// 输入: "当温度超过30度时发送告警"
// 输出:
// RULE "温度告警"
// WHEN sensor.temperature > 30
// DO
//     NOTIFY "温度过高"
// END
```

### 规则验证

```rust
use neomind_rules::validator::{RuleValidator, ValidationContext};

let validator = RuleValidator::new();
let context = ValidationContext {
    devices: vec![/* 可用设备 */],
    metrics: vec![/* 可用指标 */],
    commands: vec![/* 可用命令 */],
    alert_channels: vec![/* 已配置的通道 */],
};

let result = validator.validate(&rule, &context);
```

## 功能状态

| 功能 | 状态 | 说明 |
|------|------|------|
| DSL规则引擎 | 完成 | 完整的DSL解析器，RULE/WHEN/DO/END语法 |
| NL规则生成 | 完成 | 基于LLM的自然语言规则生成 |
| 规则验证 | 完成 | 上下文感知的资源验证 |
| 扩展条件 | 完成 | 规则中支持EXTENSION指标条件 |
| 设备类型生成 | 完成 | 基于LLM的示例生成类型 |
| 自动入板 | 完成 | 完整的草稿设备流水线 |
| Agent触发动作 | 完成 | 规则可触发AI Agent |
| Transform引擎 | 计划中 | 数据转换管道 |

## 设计原则

1. **LLM驱动**: 使用LLM进行NL到规则和示例到类型的生成
2. **上下文感知**: 根据可用设备和指标验证规则
3. **DSL优先**: 人类可读的规则定义语言
4. **可扩展**: 支持规则中的设备和扩展条件
5. **流水线**: 带LLM分析和用户批准的自动入板
