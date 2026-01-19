# NeoTalk 工具系统分析报告

## 执行摘要

NeoTalk 的工具系统是一个分层、模块化的架构，支持设备控制、数据查询、规则管理和工作流执行。本报告分析现有工具清单、新增自动化模块，并提出未来整合建议。

---

## 一、现有工具清单

### 1.1 工具系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Agent Layer                             │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │        EventIntegratedToolRegistry                      │ │
│  │  - 事件发布                                           │ │
│  │  - 执行历史                                           │ │
│  │  - 自动重试                                           │ │
│  └─────────────────────────────────────────────────────────┘ │
│                           ↓                                 │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              ToolRegistry                               │ │
│  │  - 工具注册与查找                                      │ │
│  │  - 并行执行                                           │ │
│  │  - LLM 格式化                                         │ │
│  └─────────────────────────────────────────────────────────┘ │
│                           ↓                                 │
│  ┌───────────────────┬───────────────────┬─────────────────┐ │
│  │   Built-in Tools  │   Core Tools      │  Real Tools    │ │
│  │   (测试/演示)      │   (业务场景)      │  (生产环境)     │ │
│  └───────────────────┴───────────────────┴─────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 核心工具 Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) &str;
    fn parameters(&self) -> Value;
    async fn execute(&self, args: Value) -> Result<ToolOutput>;
}
```

### 1.3 工具输出格式

```rust
pub struct ToolOutput {
    pub success: bool,           // 执行是否成功
    pub data: Value,            // 结果数据
    pub error: Option<String>,  // 错误信息
    pub metadata: Option<Value>, // 元数据
}
```

---

## 二、详细工具清单

### 2.1 Built-in 工具（测试/演示）

| 工具名称 | 描述 | 输入参数 | 输出格式 |
|---------|------|---------|---------|
| **QueryDataTool** | 查询设备数据 | ```json
{
  "device_id": "string",
  "metric": "string"
}
``` | ```json
{
  "device_id": "客厅温度传感器",
  "metric": "temperature",
  "value": 25.5,
  "unit": "°C",
  "timestamp": 1234567890
}
``` |
| **ListDevicesTool** | 列出所有设备 | ```json
{
  "location": "string (可选)",
  "type": "string (可选)"
}
``` | ```json
{
  "devices": [
    {
      "id": "light_1",
      "name": "客厅灯",
      "type": "light",
      "status": "on"
    }
  ]
}
``` |
| **ControlDeviceTool** | 控制设备 | ```json
{
  "device_id": "string",
  "command": "string",
  "parameters": {}
}
``` | ```json
{
  "device_id": "light_1",
  "command": "turn_on",
  "success": true,
  "new_state": "on"
}
``` |
| **CreateRuleTool** | 创建自动化规则 | ```json
{
  "name": "string",
  "trigger": {},
  "actions": []
}
``` | ```json
{
  "rule_id": "rule_123",
  "name": "温度控制规则",
  "enabled": true
}
``` |
| **ListRulesTool** | 列出所有规则 | ```json
{
  "enabled_only": "boolean"
}
``` | ```json
{
  "rules": [
    {"id": "rule_1", "name": "...", "enabled": true}
  ]
}
``` |
| **TriggerWorkflowTool** | 触发工作流 | ```json
{
  "workflow_id": "string",
  "parameters": {}
}
``` | ```json
{
  "execution_id": "exec_123",
  "status": "running",
  "steps": [...]
}
``` |

### 2.2 Core Tools（业务场景）

| 工具名称 | 描述 | 输入参数 | 输出格式 |
|---------|------|---------|---------|
| **DeviceDiscoverTool** | 设备发现与探索 | ```json
{
  "query": "string (搜索关键词)",
  "location": "string (可选)",
  "type": "string (可选)"
}
``` | ```json
{
  "summary": {
    "total": 10,
    "online": 8,
    "offline": 2
  },
  "devices": [...],
  "groups": [...]
}
``` |
| **DeviceQueryTool** | 设备数据查询 | ```json
{
  "device_id": "string",
  "metrics": ["string"],
  "time_range": {
    "start": "timestamp",
    "end": "timestamp"
  }
}
``` | ```json
{
  "device_id": "sensor_1",
  "data": {
    "temperature": [...],
    "humidity": [...]
  }
}
``` |
| **DeviceControlTool** | 设备控制操作 | ```json
{
  "device_id": "string",
  "command": "string",
  "parameters": {}
}
``` | ```json
{
  "success": true,
  "device_id": "light_1",
  "result": "Command executed"
}
``` |
| **DeviceAnalyzeTool** | 设备数据分析 | ```json
{
  "device_id": "string",
  "analysis_type": "string"
}
``` | ```json
{
  "device_id": "sensor_1",
  "analysis": {
    "trends": [...],
    "anomalies": [...]
  }
}
``` |
| **RuleFromContextTool** | 从对话上下文创建规则 | ```json
{
  "context": "对话历史",
  "intent": "用户意图"
}
``` | ```json
{
  "rule": {
    "id": "rule_123",
    "trigger": {...},
    "actions": [...]
  }
}
``` |

### 2.3 Real Tools（生产环境）

| 工具名称 | 描述 | 依赖服务 |
|---------|------|---------|
| **RealDeviceQueryTool** | 真实设备查询 | DeviceService, TimeSeriesStorage |
| **RealDeviceControlTool** | 真实设备控制 | DeviceService, MQTT/Modbus |
| **RealRuleCreateTool** | 规则创建 | RuleEngine |
| **RealWorkflowTriggerTool** | 工作流触发 | WorkflowEngine |

---

## 三、新增自动化模块分析

### 3.1 统一自动化类型

```rust
pub enum Automation {
    Transform(TransformAutomation),  // 数据转换
    Rule(RuleAutomation),           // 规则自动化
    Workflow(WorkflowAutomation),    // 工作流自动化
}
```

### 3.2 Transform Automation（数据转换）

**设计特点：AI-Native JavaScript**

```rust
pub struct TransformAutomation {
    pub intent: Option<String>,      // 用户自然语言描述
    pub js_code: Option<String>,     // AI 生成的 JavaScript 代码
    pub output_prefix: String,       // 输出前缀（防止命名冲突）
    pub complexity: u8,              // 复杂度评分 (1-5)
}
```

**JavaScript 执行 API：**
```javascript
// 可访问的变量
const input = { /* 原始设备数据 */ };

// 必须返回转换后的结果
return {
    temperature: input.temp,
    humidity: input.hum
};
```

### 3.3 支持的转换操作

| 操作类型 | 描述 | 示例 |
|---------|------|------|
| **Extract** | JSONPath 提取 | `$.data.temperature` |
| **Map** | 数组映射 | `items.map(x => x * 2)` |
| **Reduce** | 聚合计算 | `values.reduce((a,b) => a+b)` |
| **Format** | 模板格式化 | `"温度: ${temp}°C"` |
| **Compute** | 数学表达式 | `temp * 1.8 + 32` |
| **Pipeline** | 管道链 | 多步骤转换 |
| **If** | 条件分支 | `if (temp > 25) "hot" else "cold"` |
| **GroupBy** | 分组聚合 | `按类型分组统计` |
| **Decode/Encode** | 编码转换 | Base64, Hex 等 |

### 3.4 Rule Automation（规则自动化）

```rust
pub struct RuleAutomation {
    pub metadata: AutomationMetadata,
    pub trigger: Trigger,           // 触发条件
    pub actions: Vec<Action>,        // 执行动作
    pub schedule: Option<Schedule>,  // 定时执行
}
```

### 3.5 Workflow Automation（工作流自动化）

```rust
pub struct WorkflowAutomation {
    pub metadata: AutomationMetadata,
    pub steps: Vec<WorkflowStep>,   // 工作流步骤
    pub variables: HashMap<String, Value>,  // 变量
}
```

---

## 四、工具与自动化整合分析

### 4.1 当前整合状态

| 模块 | 整合状态 | 说明 |
|-----|---------|------|
| Agent ↔ ToolRegistry | ✅ 已整合 | 通过 EventIntegratedToolRegistry |
| ToolRegistry ↔ Automation | ⚠️ 部分整合 | 独立的 API 端点 |
| Agent ↔ Automation | ❌ 未整合 | Agent 无法直接调用自动化 |
| SmartFollowUp ↔ Automation | ❌ 未整合 | 追问不涉及自动化创建 |

### 4.2 API 端点对比

| 功能 | 工具 API | 自动化 API |
|-----|---------|-----------|
| 列表 | `GET /api/tools` | `GET /api/automations` |
| 创建 | - | `POST /api/automations` |
| 执行 | `POST /api/tools/:name/execute` | `POST /api/automations/:id/trigger` |
| 启用/禁用 | - | `POST /api/automations/:id/enable` |
| 意图分析 | - | `POST /api/automations/analyze` |

---

## 五、未来整合建议

### 5.1 Phase 1: Agent 工具集成自动化能力

**目标：让 Agent 能够通过工具调用管理自动化**

#### 新增工具设计

| 工具名称 | 功能 | 输入 | 输出 |
|---------|------|-----|------|
| **ListAutomationsTool** | 列出自动化 | ```json
{
  "type": "transform|rule|workflow",
  "enabled_only": false
}
``` | 自动化列表 |
| **CreateAutomationTool** | 创建自动化 | ```json
{
  "type": "transform|rule|workflow",
  "name": "string",
  "description": "string",
  "config": {}
}
``` | 创建的自动化 ID |
| **TriggerAutomationTool** | 触发自动化 | ```json
{
  "automation_id": "string",
  "parameters": {}
}
``` | 执行结果 |
| **ToggleAutomationTool** | 启用/禁用 | ```json
{
  "automation_id": "string",
  "enabled": true
}
``` | 新状态 |

#### 实现步骤

1. 在 `crates/tools/src/` 创建 `automation_tools.rs`
2. 实现上述 4 个工具
3. 在 ToolRegistry 中注册
4. Agent 自动获得自动化管理能力

### 5.2 Phase 2: 智能自动化创建

**目标：Agent 能够根据对话上下文自动创建自动化**

#### 整合点

```
ConversationContext + SmartFollowUp + Automation
         ↓
    检测用户创建自动化的意图
         ↓
    提取必要信息（追问）
         ↓
    调用 CreateAutomationTool
         ↓
    返回创建结果
```

#### 工作流示例

```
用户: 当客厅温度超过 28 度时，自动打开空调
Agent: [检测到规则创建意图]
      [提取触发条件: 温度 > 28]
      [提取动作: 打开空调]
      → 调用 CreateAutomationTool
      → 规则已创建，ID: rule_123
```

### 5.3 Phase 3: 自动化工具化

**目标：将现有的自动化转换为可被 Agent 调用的工具**

| 自动化类型 | 转换为工具 | 工具名称 |
|-----------|-----------|---------|
| Transform | `ExecuteTransformTool` | 执行数据转换 |
| Rule | `EvaluateRuleTool` | 评估规则条件 |
| Rule | `TriggerRuleTool` | 触发规则动作 |
| Workflow | `ExecuteWorkflowStepTool` | 执行工作流步骤 |
| Workflow | `GetWorkflowStatusTool` | 获取工作流状态 |

### 5.4 Phase 4: 智能工具推荐

**目标：根据对话上下文推荐合适的工具**

```rust
pub struct ToolRecommendationEngine {
    // 基于以下因素推荐：
    // 1. 对话历史工具使用模式
    // 2. 当前对话主题
    // 3. 设备状态和类型
    // 4. 时间和上下文
}

pub fn recommend_tools(&self, context: &ConversationContext) -> Vec<ToolSuggestion> {
    // 返回推荐的工具列表，带推荐理由
}
```

---

## 六、技术架构建议

### 6.1 统一工具执行器

```rust
pub struct UnifiedToolExecutor {
    registry: Arc<ToolRegistry>,
    automation_manager: Arc<AutomationManager>,
}

impl UnifiedToolExecutor {
    pub async fn execute(&self, tool_call: &ToolCall) -> ToolOutput {
        match tool_call.name.as_str() {
            // 标准工具
            "list_devices" => self.registry.execute(...).await,

            // 自动化工具
            "list_automations" => self.list_automations(...).await,
            "create_automation" => self.create_automation(...).await,

            _ => ToolOutput::error("Unknown tool")
        }
    }
}
```

### 6.2 工具-自动化映射表

```rust
// 工具与自动化的双向映射
pub struct ToolAutomationMapping {
    // 工具 → 自动化（工具可以创建的自动化类型）
    pub tool_to_automation: HashMap<String, Vec<AutomationType>>,

    // 自动化 → 工具（自动化可以转换为的工具）
    pub automation_to_tools: HashMap<AutomationType, Vec<String>>,
}

// 预定义映射
impl Default for ToolAutomationMapping {
    fn default() -> Self {
        let mut tool_to_automation = HashMap::new();
        tool_to_automation.insert("create_rule".to_string(), vec![AutomationType::Rule]);
        tool_to_automation.insert("create_workflow".to_string(), vec![AutomationType::Workflow]);
        tool_to_automation.insert("create_transform".to_string(), vec![AutomationType::Transform]);

        let mut automation_to_tools = HashMap::new();
        automation_to_tools.insert(AutomationType::Rule, vec!["evaluate_rule".to_string(), "trigger_rule".to_string()]);
        automation_to_tools.insert(AutomationType::Workflow, vec!["execute_workflow".to_string()]);

        Self { tool_to_automation, automation_to_tools }
    }
}
```

---

## 七、实施路线图

### P0 - 核心工具集成（1-2 周）

- [ ] 实现 `ListAutomationsTool`
- [ ] 实现 `TriggerAutomationTool`
- [ ] 实现 `ToggleAutomationTool`
- [ ] Agent 集成测试

### P1 - 智能创建（2-3 周）

- [ ] 实现 `CreateAutomationTool`
- [ ] 整合 SmartFollowUp 进行意图识别
- [ ] 整合 ConversationContext 进行信息提取
- [ ] 对话到自动化的转换器

### P2 - 自动化工具化（2-3 周）

- [ ] Transform 转换为工具
- [ ] Rule 评估和触发工具
- [ ] Workflow 步骤执行工具

### P3 - 智能推荐（3-4 周）

- [ ] 工具推荐引擎
- [ ] 基于历史的工具排序
- [ ] 自动化建议系统

---

## 八、数据流图

### 8.1 当前数据流

```
用户输入
    ↓
Agent (LLM + ToolRegistry)
    ↓
Tool Execution
    ↓
Device/Rule/Workflow Service
    ↓
结果返回
```

### 8.2 建议数据流（整合后）

```
用户输入
    ↓
ConversationContext + SmartFollowUp
    ↓
Agent (LLM + UnifiedToolExecutor)
    ↓
Tool Routing
    ├── Standard Tools → Device Service
    ├── Automation Tools → Automation Manager
    └── Smart Recommendations → 用户建议
    ↓
结果返回 + 上下文更新
```

---

## 九、总结与建议

### 9.1 现状总结

| 方面 | 现状 | 评分 |
|-----|------|------|
| 工具完整性 | 基础工具齐全，缺少自动化工具 | 7/10 |
| 工具质量 | 统一接口，事件追踪完善 | 9/10 |
| 自动化能力 | Transform/Rule/Workflow 独立 | 8/10 |
| Agent集成 | 工具已集成，自动化未集成 | 6/10 |
| 智能程度 | 基础执行，缺少推荐 | 6/10 |

### 9.2 核心建议

1. **优先级最高**：实现自动化管理工具，让 Agent 能够查询和触发自动化
2. **高优先级**：整合智能追问和上下文，实现对话式自动化创建
3. **中优先级**：将自动化转换为可执行工具
4. **低优先级**：实现智能工具推荐系统

### 9.3 架构原则

- **统一接口**：所有工具和自动化通过统一的 Tool trait 访问
- **事件驱动**：所有执行通过 EventBus 追踪
- **AI-Native**：支持自然语言描述和 JavaScript 转换
- **可扩展性**：支持插件和自定义工具

---

生成日期: 2026-01-18
版本: v1.0
