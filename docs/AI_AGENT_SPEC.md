# AI Agent 功能开发与测试目标

> 文档版本: v1.0
> 创建日期: 2025-01-22
> 目标: 实现用户自定义 AI Agent 系统，解决物联网场景碎片化问题

---

## 一、项目概述

### 1.1 核心目标

让最终用户能够：
1. **选择设备/指标/指令** - 简单勾选要监控的资源
2. **描述需求** - 用自然语言描述想要实现的功能
3. **AI 自动执行** - Agent 自动理解、执行并记住状态
4. **监控行为** - 查看每次执行的过程和决策原因

### 1.2 典型用户场景

| 场景 | 描述 | 复杂度 |
|------|------|--------|
| 每日监控报告 | 每天早上8点分析所有仓库温湿度，生成日报 | 中 |
| 周期能耗分析 | 每周一分析上周设备能耗，找出异常 | 中 |
| 复杂条件监控 | 多条件组合、时间窗口、级联动作 | 高 |
| AI 行为监控 | 查看Agent每天做了什么决策，为什么 | 中 |

---

## 二、开发目标

### 2.1 后端功能

#### 功能模块

| 模块 | 文件位置 | 功能描述 | 优先级 |
|------|----------|----------|--------|
| **数据模型** | `crates/automation/src/agent/` | Agent、资源、执行记录等数据结构 | P0 |
| **存储层** | `crates/automation/src/agent/store.rs` | Agent 持久化存储 | P0 |
| **执行引擎** | `crates/automation/src/agent/engine.rs` | Agent 执行和状态管理 | P0 |
| **调度器** | `crates/automation/src/agent/scheduler.rs` | 定时任务调度 | P0 |
| **意图分析** | `crates/automation/src/agent/intent.rs` | 解析用户需求 | P0 |
| **报告生成** | `crates/automation/src/agent/report.rs` | 生成分析报告 | P1 |

#### API 端点

| 端点 | 方法 | 功能 | 优先级 |
|------|------|------|--------|
| `/api/agents` | GET/POST | 列表/创建 Agent | P0 |
| `/api/agents/:id` | GET/PUT/DELETE | 详情/更新/删除 | P0 |
| `/api/agents/:id/execute` | POST | 手动触发执行 | P0 |
| `/api/agents/:id/pause` | POST | 暂停 Agent | P1 |
| `/api/agents/:id/resume` | POST | 恢复 Agent | P1 |
| `/api/agents/:id/executions` | GET | 执行历史 | P0 |
| `/api/agents/:id/executions/:eid` | GET | 执行详情 | P0 |
| `/api/agents/:id/behavior` | GET | AI 行为概览 | P1 |
| `/api/agents/:id/reports` | GET | 报告列表 | P0 |
| `/api/agents/:id/reports/:rid` | GET | 报告详情 | P0 |
| `/api/agents/analyze-intent` | POST | 分析用户意图 | P1 |
| `/api/agents/resources/devices` | GET | 可选设备列表 | P0 |
| `/api/agents/resources/metrics` | GET | 可选指标列表 | P0 |

#### 数据结构

```rust
// 核心数据结构
pub struct AiAgent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub user_prompt: String,        // 用户需求描述
    pub resources: AgentResources,   // 关联资源
    pub schedule: AgentSchedule,     // 执行配置
    pub understanding: Option<IntentUnderstanding>,
    pub memory: AgentMemory,         // 持久化状态
    pub enabled: bool,
}

pub struct AgentResources {
    pub data_sources: Vec<DataSource>,  // 数据源
    pub commands: Vec<AgentCommand>,     // 可执行指令
    pub outputs: Vec<OutputConfig>,      // 输出配置
}

pub struct AgentExecution {
    pub id: String,
    pub agent_id: String,
    pub timestamp: i64,
    pub status: ExecutionStatus,
    pub input: ExecutionInput,
    pub decision_process: DecisionProcess,  // AI 决策过程
    pub result: ExecutionResult,
    pub report: Option<GeneratedReport>,
}
```

### 2.2 前端功能

#### 页面组件

| 组件 | 文件位置 | 功能描述 | 优先级 |
|------|----------|----------|--------|
| **Agent 列表页** | `web/src/pages/agents.tsx` | 展示所有 Agent | P0 |
| **Agent 创建向导** | `web/src/components/agent/AgentCreator.tsx` | 分步创建 Agent | P0 |
| **Agent 详情页** | `web/src/pages/agents/AgentDetail.tsx` | Agent 状态和历史 | P0 |
| **资源选择器** | `web/src/components/agent/ResourceSelector.tsx` | 选择设备/指标/指令 | P0 |
| **执行记录** | `web/src/components/agent/ExecutionTimeline.tsx` | 执行历史时间线 | P0 |
| **AI 行为监控** | `web/src/components/agent/BehaviorMonitor.tsx` | AI 决策过程 | P1 |
| **报告查看器** | `web/src/components/agent/ReportViewer.tsx` | 查看生成的报告 | P0 |

#### UI 流程

```
创建 Agent 流程：
1. 基本信息 → 2. 选择资源 → 3. 配置执行 → 4. 描述需求 → 5. 确认创建

查看 Agent 流程：
列表页 → 详情页 → [执行记录] / [AI 行为] / [报告]
```

### 2.3 核心特性

| 特性 | 描述 | 优先级 |
|------|------|--------|
| **持久化记忆** | Agent 跨执行保持状态 | P0 |
| **自然语言交互** | 用户用文本描述需求 | P0 |
| **批量资源选择** | 支持多设备、多指标选择 | P0 |
| **定时执行** | Cron 表达式调度 | P0 |
| **事件触发** | 设备数据变化触发 | P1 |
| **决策透明** | 记录 AI 决策过程 | P0 |
| **报告生成** | 自动生成分析报告 | P0 |
| **异常处理** | 检测并通知异常 | P1 |

---

## 三、测试目标

### 3.1 测试策略

```
┌─────────────────────────────────────────────────────────────┐
│                      测试金字塔                              │
├─────────────────────────────────────────────────────────────┤
│  ▓▓▓▓▓▓▓▓  E2E 测试 (端到端)                                │
│   关键用户场景                                       │
│                                                            │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  集成测试                               │
│   Agent 执行引擎、API 集成、存储                        │
│                                                            │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  单元测试              │
│   数据结构、工具函数、组件                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 测试场景

#### 场景 1: 基础监控 Agent

```
前置条件：
- 设备模拟器运行中
- 有 3 个温度传感器在线

操作步骤：
1. 用户创建 Agent，选择 3 个温度传感器
2. 描述需求："每天早上8点检查温度，超过30度就通知我"
3. 保存并启用 Agent

预期结果：
✓ Agent 创建成功
✓ AI 正确理解意图（触发：定时，条件：温度>30）
✓ Agent 在下次 8:00 执行
✓ 执行记录显示正确的决策过程
```

#### 场景 2: 数据分析报告

```
前置条件：
- 设备模拟器有历史数据（7天）
- 有 5 个设备的多种指标

操作步骤：
1. 用户创建 Agent，选择 5 个设备的所有指标
2. 描述需求："每周一生成上周能耗分析报告"
3. 模拟周一到达，Agent 执行

预期结果：
✓ Agent 按计划执行
✓ 收集到完整的数据（7天 × 5设备 × N指标）
✓ 生成结构化报告（趋势、异常、对比）
✓ 报告可查看和导出
```

#### 场景 3: 复杂条件监控

```
前置条件：
- 设备模拟器有温湿度传感器
- 有加湿器和风扇控制设备

操作步骤：
1. 用户创建 Agent，选择温湿度传感器和控制设备
2. 描述需求："当温度连续3次超过30度且湿度低于20%时，
    开启加湿器和风扇，如果5分钟内没改善就通知管理员"
3. 模拟触发条件达成

预期结果：
✓ Agent 正确解析复杂条件
✓ 状态变量记录"连续次数"
✓ 执行级联动作（加湿器+风扇）
✓ 检测改善情况，未改善则通知
✓ 完整的执行日志
```

#### 场景 4: 持久化记忆

```
前置条件：
- Agent 已运行过一段时间

操作步骤：
1. Agent 执行后，检查状态变量
2. 等待下次执行
3. 验证状态是否保持

预期结果：
✓ 状态变量跨执行保持
✓ 趋势数据累积
✓ 学习的模式被记录
```

#### 场景 5: AI 行为透明化

```
前置条件：
- Agent 有执行记录

操作步骤：
1. 进入 Agent 详情页
2. 查看 AI 行为标签
3. 点击某次执行查看决策过程

预期结果：
✓ 显示行为概览（决策分布、频率）
✓ 时间线展示每次执行
✓ 决策过程分阶段展示（数据收集→分析→决策→执行）
✓ 每个阶段有清晰的说明
```

### 3.3 测试工具

#### 设备模拟器

位置: `crates/testing/src/device_simulator.rs`

功能：
- 模拟多种设备类型（温度、湿度、能耗等）
- 生成时序数据
- 模拟异常情况
- 可配置的数据模式

```rust
pub struct DeviceSimulator {
    pub devices: Vec<SimulatedDevice>,
}

impl DeviceSimulator {
    pub async fn start(&self) -> Result<()>;
    pub async fn inject_event(&self, event: SimulatedEvent) -> Result<()>;
    pub async fn get_telemetry(&self, device_id: &str) -> Vec<MetricData>;
}
```

#### 测试数据生成器

位置: `crates/testing/src/test_data_generator.rs`

功能：
- 生成各种场景的测试数据
- 模拟不同时间段的数据模式
- 生成异常数据

### 3.4 验收标准

#### 功能验收

| 功能 | 验收标准 |
|------|----------|
| **创建 Agent** | 用户可以通过向导创建 Agent，选择资源和描述需求 |
| **定时执行** | Agent 按照配置的时间准确执行 |
| **数据收集** | Agent 能正确收集指定设备的数据 |
| **AI 决策** | AI 根据需求和数据做出正确决策 |
| **状态持久化** | Agent 的状态在多次执行间正确保持 |
| **执行记录** | 每次执行都有完整的记录 |
| **报告生成** | 能生成结构化的分析报告 |
| **异常处理** | 能检测异常并按配置处理 |

#### 性能验收

| 指标 | 目标 |
|------|------|
| Agent 创建响应 | < 500ms |
| Agent 执行延迟 | < 5s（不含 LLM） |
| 数据收集（100设备） | < 3s |
| 报告生成 | < 10s |

#### 用户体验验收

| 指标 | 目标 |
|------|------|
| 界面响应性 | 所有操作 < 200ms 反馈 |
| 错误提示 | 清晰的错误信息和解决建议 |
| 易用性 | 5分钟内完成第一个 Agent 创建 |
| 可读性 | 执行记录和报告对非技术人员可读 |

---

## 四、实现计划

### Phase 1: 基础设施 (1天)
- [x] 制定开发和测试目标
- [ ] 创建设备模拟器
- [ ] 创建测试数据生成器
- [ ] 设置测试环境

### Phase 2: 后端核心 (2-3天)
- [ ] 数据结构定义
- [ ] 存储层实现
- [ ] 执行引擎实现
- [ ] 调度器实现
- [ ] API 端点实现

### Phase 3: 前端界面 (2-3天)
- [ ] Agent 列表页
- [ ] Agent 创建向导
- [ ] 资源选择器组件
- [ ] Agent 详情页
- [ ] 执行记录展示
- [ ] AI 行为监控

### Phase 4: 集成测试 (1-2天)
- [ ] 端到端场景测试
- [ ] 性能测试
- [ ] 用户体验验证
- [ ] Bug 修复

### Phase 5: 优化和文档 (1天)
- [ ] 性能优化
- [ ] 错误处理完善
- [ ] 用户文档
- [ ] API 文档

---

## 五、风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| LLM 响应不稳定 | 高 | 添加重试和降级机制 |
| 数据量过大导致超时 | 中 | 实现数据采样和分页 |
| 用户意图理解错误 | 高 | 添加确认步骤和反馈机制 |
| 存储容量问题 | 中 | 实现数据清理和归档 |

---

## 六、成功标准

项目完成需满足：

1. ✅ 所有 P0 功能实现并通过测试
2. ✅ 5个测试场景全部通过
3. ✅ 性能指标达标
4. ✅ 无 P0/P1 级别的 Bug
5. ✅ 代码通过 clippy 检查
6. ✅ 关键模块有单元测试（覆盖率 > 70%）
