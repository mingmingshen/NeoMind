# 数据推送模块

> **起始版本**：v0.8.0
> **模块**：`neomind-data-push`
> **存储**：`data/data-push.redb`

## 概述

数据推送模块用于将设备遥测数据和扩展模块输出实时或按计划转发到外部系统。支持两种推送目标类型 -- **Webhook**（HTTP POST/PUT）和 **MQTT**（发布到外部代理）-- 并提供可配置的数据过滤、负载模板、重试逻辑和批量聚合功能。

核心能力：

- **事件驱动推送** -- 订阅内部 EventBus，匹配数据到达时立即推送。
- **定时轮询** -- 定期从时序数据库拉取数据。
- **数据源过滤** -- 基于前缀的模式匹配，支持可选的变更检测。
- **Handlebars 模板** -- 在推送前通过模板转换负载，支持 `{{source_id}}`、`{{value}}`、`{{timestamp}}` 等变量。
- **指数退避重试** -- 可配置的 `max_retries`、`backoff_secs` 和 `max_backoff_secs`。
- **批量聚合** -- 将多个事件合并为单个负载后再发送。
- **投递日志** -- 记录每次投递尝试的状态、负载、响应和错误详情。
- **测试端点** -- 发送示例负载验证目标连通性，无需等待真实数据。

---

## 模块结构

```
crates/neomind-data-push/
  Cargo.toml
  src/
    lib.rs              -- 模块入口，公共导出
    types.rs            -- 核心类型（PushTarget、DeliveryLog、PushSchedule 等）
    store.rs            -- 基于 redb 的持久化存储（目标和投递日志）
    manager.rs          -- PushManager 编排器，CRUD + 生命周期管理
    scheduler.rs        -- PushScheduler，事件驱动和定时任务管理
    filter.rs           -- DataSourceMatcher，前缀匹配和变更检测
    template.rs         -- Handlebars 模板渲染器，内置辅助函数
    targets/
      mod.rs            -- PushDestination trait 和工厂函数
      webhook.rs        -- Webhook 目标（通过 reqwest 发送 HTTP POST/PUT）
      mqtt.rs           -- MQTT 目标（通过 rumqttc 发布，延迟连接）
```

---

## 核心类型

### PushTarget（推送目标）

定义数据转发目标位置和方式的配置实体。

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | `String` (UUID) | 自动生成的唯一标识符 |
| `name` | `String` | 人类可读的名称（必填） |
| `enabled` | `bool` | 是否启用（默认：`true`） |
| `target_type` | `PushTargetType` | `"webhook"` 或 `"mqtt"` |
| `config` | `serde_json::Value` | 目标特定配置（URL、代理地址等） |
| `schedule` | `PushSchedule` | 事件驱动或定时计划 |
| `data_filter` | `DataSourceFilter` | 数据源过滤模式和变更检测 |
| `template` | `Option<String>` | Handlebars 负载转换模板 |
| `retry_config` | `RetryConfig` | 重试策略（默认：3 次重试，5 秒退避） |
| `batch_config` | `BatchConfig` | 批量聚合设置（默认：batch_size=1，2 秒间隔） |
| `created_at` | `i64` | 创建时间的 Unix 时间戳 |
| `updated_at` | `i64` | 最后更新时间的 Unix 时间戳 |

### PushSchedule（推送计划，标签化枚举）

```json
// 事件驱动：匹配数据到达时立即推送
{
  "type": "event_driven",
  "event_types": ["device_metric", "extension_output"]
}

// 定时轮询：每隔 N 秒拉取一次数据
{
  "type": "interval",
  "interval_secs": 60
}
```

支持的 `event_types`：
- `"device_metric"` -- 设备遥测数据
- `"extension_output"` -- 扩展模块输出
- `"alert_created"` -- 告警事件

当 `event_types` 为空数组时，匹配所有事件类型。

### DataSourceFilter（数据源过滤器）

```json
{
  "source_patterns": ["device:sensor-001:", "extension:weather:"],
  "only_changes": false
}
```

- **`source_patterns`**：与 DataSourceId 匹配的前缀模式。空数组匹配所有数据源。
- **`only_changes`**：为 `true` 时，仅当该数据源的值与上次推送不同时才推送。

DataSourceId 格式：`{type}:{id}:{field}`（如 `device:sensor-001:temperature`）。

匹配逻辑：数据源 ID 以任一模式开头或完全等于任一模式时即匹配。

### RetryConfig（重试配置）

```json
{
  "max_retries": 3,
  "backoff_secs": 5,
  "max_backoff_secs": 300
}
```

| 字段 | 默认值 | 说明 |
|---|---|---|
| `max_retries` | `3` | 最大重试次数 |
| `backoff_secs` | `5` | 初始退避时间（秒），每次重试翻倍 |
| `max_backoff_secs` | `300` | 退避时间上限 |

退避序列：5 秒、10 秒、20 秒、40 秒...（上限为 `max_backoff_secs`）。

### BatchConfig（批量配置）

```json
{
  "batch_size": 10,
  "batch_interval_ms": 2000
}
```

| 字段 | 默认值 | 说明 |
|---|---|---|
| `batch_size` | `1` | 触发刷新的最大事件数。`1` 表示无批量（立即发送） |
| `batch_interval_ms` | `2000` | 刷新不完整批次的最大等待时间（毫秒） |

当 `batch_size > 1` 时，事件会被缓冲，直到批次填满或间隔计时器触发。批量负载结构：

```json
{
  "batch": true,
  "count": 5,
  "items": [
    { "source_id": "device:s1:temp", "value": 25.5, "timestamp": 1700000000, "metadata": null },
    ...
  ]
}
```

### DeliveryLog（投递日志）

每次投递尝试都会创建一条日志记录：

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | `String` (UUID) | 日志条目 ID |
| `target_id` | `String` | 所属的推送目标 |
| `status` | `DeliveryStatus` | `pending`（等待中）、`success`（成功）、`failed`（失败）、`retrying`（重试中） |
| `data_source_id` | `String` | 触发此次投递的数据源 |
| `payload_sent` | `String` | 实际发送到目标的负载 |
| `response` | `Option<String>` | 目标的响应内容（如适用） |
| `attempts` | `u32` | 已尝试的次数 |
| `created_at` | `i64` | 首次尝试的 Unix 时间戳 |
| `completed_at` | `Option<i64>` | 最终结果的 Unix 时间戳 |
| `error` | `Option<String>` | 失败时的错误信息 |

### PushStats（推送统计）

所有目标的聚合统计信息：

```json
{
  "total_targets": 5,
  "active_targets": 3,
  "total_deliveries": 0,
  "successful_deliveries": 0,
  "failed_deliveries": 0
}
```

### TemplateContext（模板上下文）

Handlebars 模板中可用的变量：

| 变量 | 类型 | 说明 |
|---|---|---|
| `{{source_id}}` | `String` | 完整的 DataSourceId（如 `device:s1:temp`） |
| `{{value}}` | `Value` | 数据值（数字、字符串、布尔、JSON） |
| `{{timestamp}}` | `i64` | 数据点的 Unix 时间戳 |
| `{{metadata}}` | `Value` | 附着在事件上的可选元数据 |

内置 Handlebars 辅助函数：
- `{{json value}}` -- 将值序列化为 JSON 字符串
- `{{timestamp_format timestamp}}` -- 将 Unix 时间戳格式化为 ISO 8601 / RFC 3339

---

## 推送目标类型

### Webhook

向指定 URL 发送 HTTP POST（或 PUT）请求，请求体为 JSON。

**配置：**

```json
{
  "url": "https://example.com/api/webhook",
  "method": "POST",
  "headers": {
    "X-Custom-Header": "value"
  },
  "auth_token": "Bearer token here",
  "timeout_secs": 30
}
```

| 字段 | 默认值 | 说明 |
|---|---|---|
| `url` | （必填） | 目标 URL |
| `method` | `"POST"` | HTTP 方法（`"POST"` 或 `"PUT"`） |
| `headers` | `{}` | 自定义 HTTP 请求头 |
| `auth_token` | `null` | Bearer 令牌，用于 Authorization 请求头 |
| `auth_basic` | `null` | 基本认证（`{ "username": "...", "password": "..." }`） |
| `timeout_secs` | `30` | 请求超时时间（秒） |

`auth_token` 和 `auth_basic` 互斥。如果同时提供，`auth_token` 优先。

### MQTT

使用 rumqttc 异步客户端发布到外部 MQTT 代理。

**配置：**

```json
{
  "broker": "broker.hivemq.com",
  "port": 1883,
  "topic": "neomind/data/sensor-001",
  "username": "user",
  "password": "pass",
  "qos": 1,
  "client_id": "neomind-push"
}
```

| 字段 | 默认值 | 说明 |
|---|---|---|
| `broker` | （必填） | MQTT 代理主机名 |
| `port` | `1883` | 代理端口 |
| `topic` | （必填） | 发布的主题 |
| `username` | `null` | 认证用户名 |
| `password` | `null` | 认证密码 |
| `qos` | `1` | QoS 级别：`0`（最多一次）、`1`（至少一次）、`2`（恰好一次） |
| `client_id` | `"neomind-push"` | 客户端 ID 前缀（自动追加随机后缀保证唯一性） |

MQTT 连接在首次发布时延迟建立，并保持用于后续投递。

---

## 计划类型

### 事件驱动

订阅 NeoMind EventBus，匹配事件到达时立即推送数据。

```json
{
  "type": "event_driven",
  "event_types": ["device_metric", "extension_output"]
}
```

工作流程：
1. 订阅 EventBus 广播通道。
2. 按 `event_types` 过滤传入事件。
3. 从匹配事件中提取 source_id、value 和 timestamp。
4. 应用 `DataSourceFilter` 模式匹配和变更检测。
5. 通过 Handlebars 模板（或默认 JSON）渲染负载。
6. 执行投递，附带重试逻辑。

启用批量聚合时（`batch_size > 1`），事件先缓冲，在批次填满或 `batch_interval_ms` 计时器触发时刷新。

### 定时轮询

按固定间隔定期查询数据（当前为查询 TimeSeriesStore 的占位实现）。

```json
{
  "type": "interval",
  "interval_secs": 60
}
```

---

## 投递追踪与重试逻辑

### 投递流程

1. 数据通过 EventBus（事件驱动）或计时器触发（定时轮询）到达。
2. 根据 `DataSourceFilter` 模式过滤数据。
3. 如启用 `only_changes`，匹配器检查该数据源的值是否与上次投递时不同。
4. 通过模板引擎渲染负载。
5. 创建 `DeliveryLog` 条目，状态为 `pending`。
6. 向目标发送负载。
7. 成功时：日志状态设为 `success`，记录 `completed_at`。
8. 失败时：使用指数退避进行重试。
   - 重试之间日志状态更新为 `retrying`。
   - 达到 `max_retries` 次失败后，日志状态设为 `failed`。

### 重试序列

默认 `RetryConfig`（`max_retries: 3`、`backoff_secs: 5`、`max_backoff_secs: 300`）的退避时间：

| 尝试 | 退避 | 累计耗时 |
|---|---|---|
| 1（初始） | - | 0 秒 |
| 2 | 5 秒 | 5 秒 |
| 3 | 10 秒 | 15 秒 |
| 4（最终） | 20 秒 | 35 秒 |

### 日志清理

使用 `PushManager::cleanup_logs(older_than_days)` 清除旧投递日志，防止数据库无限增长。

---

## API 端点

所有端点位于 `/api/data-push` 路径下。

### 创建推送目标

```
POST /api/data-push
```

**请求体：**

```json
{
  "name": "我的 Webhook",
  "target_type": "webhook",
  "config": {
    "url": "https://example.com/api/webhook",
    "headers": { "Authorization": "Bearer TOKEN" }
  },
  "schedule": {
    "type": "event_driven",
    "event_types": ["device_metric"]
  },
  "data_filter": {
    "source_patterns": ["device:sensor-001:"],
    "only_changes": false
  },
  "template": "{\"device\":\"{{source_id}}\",\"value\":{{value}},\"ts\":{{timestamp}}}",
  "enabled": true,
  "retry_config": { "max_retries": 3, "backoff_secs": 5, "max_backoff_secs": 300 },
  "batch_config": { "batch_size": 1, "batch_interval_ms": 2000 }
}
```

**响应：**

```json
{
  "success": true,
  "data": {
    "id": "目标-uuid",
    "name": "我的 Webhook",
    "target_type": "webhook",
    "enabled": true
  }
}
```

### 列出推送目标

```
GET /api/data-push
```

**查询参数：**
- `enabled`（可选）-- 按启用状态过滤

**响应：**

```json
{
  "success": true,
  "data": {
    "targets": [...],
    "total": 5
  }
}
```

### 获取推送目标

```
GET /api/data-push/:id
```

**响应：** 完整的 `PushTarget` 对象。

### 更新推送目标

```
PUT /api/data-push/:id
```

所有字段均为可选（部分更新）。更改 `config` 或 `schedule` 时，目标会自动停止并重新启动。

**请求体：**

```json
{
  "name": "新名称",
  "enabled": false,
  "config": { "url": "https://new-url.com/webhook" }
}
```

### 删除推送目标

```
DELETE /api/data-push/:id
```

停止目标并删除配置及投递历史。

**响应：**

```json
{
  "success": true,
  "data": { "message": "Push target deleted" }
}
```

### 测试推送目标

```
POST /api/data-push/:id/test
```

发送示例负载（`{ "test": true, "value": 42 }`）到目标，返回投递结果。

**响应：** 测试尝试的完整 `DeliveryLog` 对象。

### 启动推送目标

```
POST /api/data-push/:id/start
```

启用并启动目标的计划任务。在存储中将 `enabled` 设为 `true`。

### 停止推送目标

```
POST /api/data-push/:id/stop
```

停止目标的计划任务，在存储中将 `enabled` 设为 `false`。目标配置保留。

### 获取投递日志

```
GET /api/data-push/:id/logs
```

**查询参数：**
- `limit`（可选，默认：50，最大：200）-- 日志条目数量
- `offset`（可选，默认：0）-- 分页偏移量

**响应：**

```json
{
  "success": true,
  "data": {
    "logs": [...],
    "total": 150
  }
}
```

### 获取推送统计

```
GET /api/data-push/stats
```

**响应：**

```json
{
  "success": true,
  "data": {
    "total_targets": 5,
    "active_targets": 3,
    "total_deliveries": 0,
    "successful_deliveries": 0,
    "failed_deliveries": 0
  }
}
```

---

## CLI 命令

`neomind push` 命令组提供推送目标的完整管理功能。

### push list

```
neomind push list
```

列出所有推送目标及其状态、类型和配置。

### push get

```
neomind push get <ID>
```

显示指定推送目标的详细信息。

### push create

```
neomind push create \
  --name "我的 Webhook" \
  --type webhook \
  --config '{"url":"https://example.com/webhook"}' \
  --schedule event \
  --sources "device:sensor-001:temperature,device:sensor-001:humidity"
```

**参数：**
- `--name`（必填）-- 目标名称
- `--type` / `-t` -- 目标类型：`webhook`（默认）或 `mqtt`
- `--config` -- 目标配置 JSON 字符串
- `--schedule` -- `event`（默认）或 `interval`
- `--sources` -- 逗号分隔的数据源模式（如 `"device:s1:"`）

**Webhook 配置示例：**

```json
{
  "url": "https://httpbin.org/post",
  "headers": { "Authorization": "Bearer my-token" }
}
```

**MQTT 配置示例：**

```json
{
  "broker": "broker.hivemq.com",
  "port": 1883,
  "topic": "neomind/data",
  "username": "user",
  "password": "pass"
}
```

### push update

```
neomind push update <ID> --name "新名称"
neomind push update <ID> --config '{"url":"https://new-url.com"}'
neomind push update <ID> --enabled false
```

**参数：**
- `--name` -- 新名称
- `--config` -- 新配置 JSON
- `--enabled` -- 启用或禁用（`true`/`false`）

### push delete

```
neomind push delete <ID>
```

删除目标及其投递历史。

### push start

```
neomind push start <ID>
```

启用并启动推送目标。

### push stop

```
neomind push stop <ID>
```

停止推送目标但不删除。

### push test

```
neomind push test <ID>
```

发送测试负载以验证连通性和配置。

### push logs

```
neomind push logs <ID> --limit 20
```

显示指定目标的投递日志。

**参数：**
- `--limit`（默认：20）-- 最大日志条目数

### push stats

```
neomind push stats
```

显示所有推送目标的聚合统计信息。

---

## 配置示例

### 示例 1：转发设备温度到 Webhook

创建事件驱动推送目标，转发特定设备的温度读数：

```bash
neomind push create \
  --name "温度 Webhook" \
  --type webhook \
  --config '{"url":"https://myapp.example.com/api/temperature"}' \
  --schedule event \
  --sources "device:temp-sensor:temperature"
```

### 示例 2：带认证的 MQTT 推送

将所有设备数据转发到外部 MQTT 代理：

```bash
neomind push create \
  --name "MQTT 转发器" \
  --type mqtt \
  --config '{"broker":"mqtt.mycompany.com","port":1883,"topic":"iot/neomind/data","username":"neomind","password":"secret","qos":1}' \
  --schedule event \
  --sources "device:"
```

### 示例 3：定时轮询 Webhook

每 60 秒通过 HTTP PUT 推送数据：

```bash
neomind push create \
  --name "定期同步" \
  --type webhook \
  --config '{"url":"https://api.myapp.com/sync","method":"PUT","auth_token":"my-bearer-token"}' \
  --schedule interval \
  --sources "device:sensor-group-1:"
```

### 示例 4：带自定义负载模板的 Webhook

直接使用 API 创建带 Handlebars 模板的目标：

```json
{
  "name": "自定义负载 Webhook",
  "target_type": "webhook",
  "config": {
    "url": "https://myapp.example.com/api/data",
    "auth_token": "my-token"
  },
  "schedule": {
    "type": "event_driven",
    "event_types": ["device_metric"]
  },
  "data_filter": {
    "source_patterns": ["device:"],
    "only_changes": true
  },
  "template": "{\"sensor\":\"{{source_id}}\",\"reading\":{{value}},\"captured_at\":\"{{timestamp_format timestamp}}\"}"
}
```

### 示例 5：批量聚合

将最多 50 个事件聚合成单次批量发送：

```json
{
  "name": "批量 Webhook",
  "target_type": "webhook",
  "config": { "url": "https://myapp.example.com/api/batch" },
  "schedule": {
    "type": "event_driven",
    "event_types": ["device_metric"]
  },
  "data_filter": {
    "source_patterns": [],
    "only_changes": false
  },
  "batch_config": {
    "batch_size": 50,
    "batch_interval_ms": 5000
  }
}
```

---

## 前端管理

数据推送模块通过 NeoMind Web UI 的 **数据探索器** 页面管理。推送目标以标签页形式与数据源并列展示。

### 组件

| 组件 | 位置 | 说明 |
|---|---|---|
| `PushTargetsTab` | `web/src/components/datapush/PushTargetsTab.tsx` | 列表视图，展示所有推送目标 |
| `PushTargetDialog` | `web/src/components/datapush/PushTargetDialog.tsx` | 全屏对话框，用于创建/编辑目标 |
| `DeliveryHistoryPanel` | `web/src/components/datapush/DeliveryHistoryPanel.tsx` | 全屏对话框，展示分页投递日志 |

### 推送目标列表

目标列表展示以下信息：
- **名称** -- 带图标（Webhook 显示 Globe，MQTT 显示 Radio）和截断的 ID
- **类型** -- 显示 `webhook` 或 `mqtt` 的标签
- **状态** -- 绿色圆点表示运行中，灰色表示已停止
- **计划** -- "事件驱动" 或 "每 N 秒"
- **数据源** -- 逗号分隔的数据源模式，或 "全部数据源"
- **更新时间** -- 最后修改日期
- **操作** -- 切换状态、测试、查看日志、编辑、删除

### 推送目标编辑器

编辑器对话框（`PushTargetDialog`）采用单页布局（无向导步骤），包含：
- 名称输入框
- 目标类型选择器（Webhook / MQTT）
- 目标特定配置字段（Webhook：URL、请求头、认证；MQTT：代理、主题、凭据）
- 计划类型选择器（事件驱动 / 定时轮询）
- 数据源选择器，支持搜索、分组和多选
- 模板编辑器（可选）
- 重试配置
- 批量聚合设置

### 投递历史

投递历史面板以分页方式展示日志条目（每页 10 条），包含：
- **状态** -- 颜色编码标签（成功、失败、等待中、重试中）
- **数据源** -- 触发此次投递的数据源
- **负载** -- 发送负载的截断预览
- **尝试次数** -- 投递尝试次数
- **时间** -- 投递时间戳

### Store 切片

状态管理使用 Zustand store 中的 `DataPushSlice`（`web/src/store/slices/dataPushSlice.ts`）：

| 状态 | 类型 | 说明 |
|---|---|---|
| `pushTargets` | `PushTarget[]` | 所有目标列表 |
| `pushTargetsLoading` | `boolean` | 加载状态 |
| `pushStats` | `PushStats \| null` | 聚合统计信息 |
| `pushTargetDialogOpen` | `boolean` | 对话框可见性 |
| `editingPushTarget` | `PushTarget \| null` | 正在编辑的目标 |
| `deliveryLogs` | `DeliveryLog[]` | 所选目标的日志 |
| `deliveryLogsTotal` | `number` | 日志总数（用于分页） |

### API 客户端

所有 API 调用在 `web/src/lib/api.ts` 的数据推送部分：

| 方法 | API 函数 | 端点 |
|---|---|---|
| `GET` | `listPushTargets()` | `/data-push` |
| `GET` | `getPushTarget(id)` | `/data-push/:id` |
| `POST` | `createPushTarget(data)` | `/data-push` |
| `PUT` | `updatePushTarget(id, data)` | `/data-push/:id` |
| `DELETE` | `deletePushTarget(id)` | `/data-push/:id` |
| `POST` | `testPushTarget(id)` | `/data-push/:id/test` |
| `POST` | `startPushTarget(id)` | `/data-push/:id/start` |
| `POST` | `stopPushTarget(id)` | `/data-push/:id/stop` |
| `GET` | `listPushDeliveryLogs(id, limit?, offset?)` | `/data-push/:id/logs` |
| `GET` | `getPushStats()` | `/data-push/stats` |

---

## 架构

```
  EventBus
    |
    v
  PushScheduler
    |-- 事件驱动任务 (tokio::spawn)
    |     |-- 订阅 EventBus
    |     |-- DataSourceMatcher（过滤 + 变更检测）
    |     |-- TemplateRenderer（Handlebars）
    |     |-- PushDestination（Webhook 或 MQTT）
    |     |-- 指数退避重试
    |     |-- 批量聚合（可选）
    |
    |-- 定时任务 (tokio::spawn)
          |-- 定期计时器触发
          |-- 查询 TimeSeriesStore（计划中）

  PushManager
    |-- CRUD 操作（创建、读取、更新、删除）
    |-- 生命周期管理（启动、停止、测试）
    |-- 委托 PushScheduler 管理运行中的目标
    |-- 委托 DataPushStore 进行持久化

  DataPushStore (redb)
    |-- push_targets 表（键：目标 ID）
    |-- delivery_logs 表（键：日志 ID）
```

`PushManager` 在服务器启动时初始化，打开 `data/data-push.redb` 数据库，加载所有已持久化的目标，并启动之前已启用的目标。
