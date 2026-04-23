# NeoMind 事件驱动架构全面分析

## 一、系统现状总览

### 1.1 EventBus 架构

**实现**: `crates/neomind-core/src/eventbus.rs`
**机制**: tokio broadcast channel, 容量 1000, fire-and-forget, **无持久化, 无回放**

```
NeoMindEvent (broadcast)
├── DeviceMetric ──────── 核心事件，驱动一切
├── DeviceOnline/Offline
├── ExtensionOutput ──── 定义了，但发布被禁用 ❌
├── ExtensionLifecycle ─ 有发布者 ✅
├── RuleTriggered/Executed
├── AgentExecution*
├── ToolExecution*
├── AlertCreated
├── Workflow* ─────────── 仅占位，未实现 ❌
└── Custom
```

**发布者 → 订阅者映射:**

| 事件类型 | 发布者 | 订阅者 | 状态 |
|----------|--------|--------|------|
| DeviceMetric | 设备适配器(MQTT/HTTP/Webhook) | 规则引擎、Transform、Dashboard WS、扩展 | ✅ 正常 |
| ExtensionOutput | 扩展指标采集服务 | 无（发布被禁用） | ❌ 禁用 |
| ExtensionLifecycle | 扩展运行器 | Dashboard WS | ✅ 正常 |
| RuleTriggered | 规则引擎 | DeviceActionExecutor | ✅ 正常 |
| AgentExecution* | Agent 执行器 | Dashboard WS | ✅ 正常 |
| WorkflowTriggered | 无 | 无 | ❌ 占位 |

### 1.2 数据流断点图

```
                          ┌─ DeviceMetric ──► 规则引擎 ✅
                          │                ──► Transform ✅
                          │                ──► Dashboard WS ✅
                          │                ──► Agent 触发 ✅
    设备 (MQTT/HTTP) ─────►│
                          └─ telemetry.redb 存储 ✅


                          ┌─ produce_metrics() ─► 定时采集 ─► 存储 ✅
                          │                                       │
                          │                    ExtensionOutput 事件
                          │                    发布被禁用 ❌ ("no reactor running" + 循环风险)
                          │
    扩展 ─────────────────►├─ push_output() ─► Runner ─► WebSocket ─► 前端 ✅
                          │
                          ├─ extension.call() ─► 扩展间调用 ✅
                          │
                          └─ EventBus 订阅 ──► 转发给扩展 ✅
                                               (有虚拟指标循环防护)


                          ┌─ check_and_trigger_event()
                          │    参数: (device_id, metric, value) ── 硬编码
    Agent 执行器 ─────────►│
                          ├─ matches_event_filter()
                          │    只匹配 Device/Metric ── ExtensionMetric 返回 false
                          │
                          └─ 工具调用 ──► device/agent/rule/message/extension ✅
```

---

## 二、关键瓶颈深度分析

### 2.1 ExtensionOutput 事件发布被禁用

**位置**: `crates/neomind-api/src/handlers/extensions.rs:929-931`

```rust
// DISABLED: Publish ExtensionOutput events - causes "no reactor running" crashes
// Event publishing will be re-enabled after fixing the Tokio runtime issue
// publish_extension_metrics_safe(&state, &id, &result).await;
```

**禁用原因 — 双重问题:**

1. **"no reactor running" 崩溃**: `publish_extension_metrics` 在扩展命令处理的同步上下文中被调用，但 EventBus 的 `publish()` 需要 tokio runtime。当不在异步上下文时直接 panic。

2. **反馈循环风险**: 即使运行时问题解决，还存在循环:
   ```
   扩展命令执行 → 返回 metrics → publish ExtensionOutput 事件
       ↓
   ExtensionEventSubscriptionService 订阅所有事件 → 转发给扩展
       ↓
   扩展收到自己的 ExtensionOutput → 可能触发处理 → 再次产生 metrics
       ↓
   无限循环 ♻️
   ```

**已有的防护机制**: `extension_event_subscription.rs:123-130` 已有虚拟 DeviceMetric 的循环防护，但未覆盖 ExtensionOutput。

### 2.2 Agent 事件触发只支持设备

**后端**: `crates/neomind-agent/src/ai_agent/executor/mod.rs`

```rust
// Line 1475: 参数硬编码为设备维度
pub async fn check_and_trigger_event(
    &self,
    device_id: String,     // 只有 device_id
    metric: &str,          // 只有 metric 名
    value: &MetricValue,   // 只有值
) -> AgentResult<()>

// Line 1665: 过滤逻辑只处理 Device/Metric
match r.resource_type {
    ResourceType::Device => { /* device_id 精确匹配 */ }
    ResourceType::Metric => { /* device_id:metric 模式匹配 */ }
    _ => false,  // ExtensionMetric/ExtensionTool 直接返回 false
}
```

**前端**: `web/src/pages/agents-components/AgentEditorFullScreen.tsx:293`

```typescript
const [eventConfig, setEventConfig] = useState<{
  type: 'device.metric' | 'manual'  // 只有这两种！
  deviceId?: string
}>({ type: 'device.metric', deviceId: 'all' })
```

**调度器**: `crates/neomind-agent/src/ai_agent/scheduler.rs:664`

```rust
ScheduleType::Event => {
    // Event-triggered, no scheduled execution
    Ok((i64::MAX, None))  // 完全跳过调度
}
```

### 2.3 Dashboard 指标绑定无维度

**DataSourceId 格式**: `{type}:{id}:{field}` — 完全扁平

```
当前:  extension:yolo-video-v2:latest_capture     → 一个值
需要:  extension:yolo-video-v2:latest_capture{roi=entrance,stream=cam1}  → 可区分
```

**DataMapper 过滤**: 只有 `eq/gt/lt/contains` 字段级过滤，无维度/标签过滤。

**存储层**: `ExtensionOutput` 事件定义中已有 `labels: Option<HashMap<String,String>>`，但存储时未使用。

### 2.4 Workflow 系统 — 仅占位

- Cargo.toml 无 workflow crate
- 事件类型 `WorkflowTriggered/StepCompleted/Completed` 已定义但无发布者无消费者
- 前端无 workflow 页面
- **结论: 纯占位代码，可清理**

---

## 三、Agent 工具系统分析

### 3.1 工具架构

5 个聚合工具替代 34+ 个单独工具（token 效率提升 ~60%）:

| 工具 | 动作 | 说明 |
|------|------|------|
| **device** | list, latest, history, control, write_metric | 设备查询与控制 |
| **agent** | list, get, create, update, control, memory, executions, conversation | Agent 管理 |
| **rule** | list, get, delete, history | 规则管理 |
| **message** | list, send, read/acknowledge | 消息通知 |
| **extension** | list, get, status | 扩展查询 |

### 3.2 扩展工具集成

扩展命令自动转换为工具:
- 工具名: `{extension_id}:{command_name}`
- 参数 schema 从 `ExtensionCommand` 描述符自动生成
- 超时: 30 秒
- **Agent 可以调用扩展命令** ✅

### 3.3 缺失的工具能力

| 能力 | 状态 | 说明 |
|------|------|------|
| 调用扩展命令 | ✅ | 通过 extension_id:command 工具 |
| 查询扩展指标历史 | ❌ | extension 工具只有 list/get/status |
| 订阅扩展事件 | ❌ | Agent 触发系统不支持 |
| 跨组件调用 | ❌ | 无 Dashboard 组件级工具 |
| 图像分析触发 | ❌ | 无 VLM 相关工具 |

### 3.4 Agent 创建流程

```
CreateAgentRequest {
    name, description, user_prompt,
    device_ids,           // Legacy
    metrics,              // Legacy
    commands,             // Legacy
    resources,            // 新格式: Vec<ResourceRequest>
    schedule,             // ScheduleType: Event/Cron/Interval
    llm_backend_id,
    enable_tool_chaining,
    max_chain_depth,
    execution_mode,       // "focused" | "free"
}
```

**ResourceRequest 支持**:
```rust
pub enum ResourceType {
    Device,           // ✅ 事件触发 + 工具使用
    Metric,           // ✅ 事件触发
    Command,          // ✅ 工具使用
    DataStream,       // 定义了但未使用
    ExtensionTool,    // 定义了，工具使用 ✅，事件触发 ❌
    ExtensionMetric,  // 定义了，工具使用 ❌，事件触发 ❌
}
```

---

## 四、连锁问题依赖图

```
问题 1: ExtensionOutput 事件发布被禁用
    ↓ 阻塞
问题 2: Agent 无法触发于扩展事件
    ↓ 阻塞
问题 3: Dashboard 无法按维度过滤扩展指标
    ↓ 阻塞
问题 4: VLM Vision 无法按 ROI 区分图像源
    ↓ 阻塞
问题 5: 跨组件协作无法实现
```

**修复顺序**: 1 → 2 → 3 → 4/5 并行

---

## 五、解决方案方向

### 方案 A: 最小可行修复（修复事件循环）

只解决问题 1，让扩展事件流通:
1. 恢复 ExtensionOutput 发布（修复 async 上下文问题）
2. 增加循环防护（ExtensionOutput 不转发给产生它的扩展）
3. Agent executor 增加 ExtensionMetric 事件处理

**影响范围**: 小，3-4 个文件
**解决程度**: 部分解决，Agent 可以触发于扩展指标更新

### 方案 B: 指标标签/维度系统

在 DataSourceId 格式上扩展 Prometheus 风格标签:
```
extension:yolo-video-v2:latest_capture{roi=entrance,stream=cam1}
```

**影响范围**: 中，存储层 + API + 前端
**解决程度**: 全面解决，支持多维度过滤

### 方案 C: 完整事件系统重构

包含方案 A + B，额外增加:
- 事件持久化与回放
- Agent 直接订阅 EventBus（绕过 polling）
- Dashboard 组件间通信
- 清理 Workflow 占位代码

**影响范围**: 大，跨多个 crate
**解决程度**: 根本性解决

---

## 六、建议实施路径

```
Phase 1 (修复): 恢复 ExtensionOutput 事件 + 修复循环
Phase 2 (增强): Agent 支持扩展事件触发 + 前端 UI
Phase 3 (扩展): 指标标签/维度系统
Phase 4 (优化): 事件持久化、组件间通信
```
