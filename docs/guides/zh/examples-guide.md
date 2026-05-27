# NeoMind 示例指南

**版本**: 0.8.0
**最后更新**: 2026-05-26

## 目录

1. [扩展示例](#扩展示例)
   - [Virtual Metrics Extension](#virtual-metrics-extension)
   - [Event Monitor Extension](#event-monitor-extension)
   - [Virtual Weather Provider](#virtual-weather-provider)
   - [Device Helper Example](#device-helper-example)
2. [Capability Provider 示例](#capability-provider-示例)
   - [Device Capability Provider](#device-capability-provider)
   - [Runner Capability Provider](#runner-capability-provider)
3. [数据推送配置](#数据推送配置)
4. [通知通道设置](#通知通道设置)
5. [使用 AI 聊天管理设备](#使用-ai-聊天管理设备)
6. [创建自动化规则](#创建自动化规则)

---

## 扩展示例

### Virtual Metrics Extension

**位置**: `examples/virtual-metrics-extension/`

**ID**: `virtual-metrics`

**版本**: 0.1.0

#### 功能描述

虚拟指标扩展示例，演示如何从外部数据源注入虚拟指标到设备遥测中。

**场景**：模拟外部数据源（如 API、数据库、计算值）向设备注入数据。

#### 提供的指标

- `injection_count` (整数) - 已注入的虚拟指标计数

#### 提供的命令

1. **set_target_device**
   - 设置目标设备 ID，用于注入虚拟指标
   - 参数：
     - `device_id` (字符串, 必需) - 设备 ID

2. **inject_virtual_metrics**
   - 向目标设备注入虚拟指标
   - 参数：
     - `metric_name` (字符串, 必需) - 指标名称
     - `value` (浮点数, 必需) - 指标值

3. **get_injection_count**
   - 获取已注入的虚拟指标计数
   - 无参数

#### 使用场景

- 学习如何创建和注入虚拟指标
- 理解扩展状态管理
- 演示外部数据集成模式

---

### Event Monitor Extension

**位置**: `examples/event-monitor-extension/`

**ID**: `event-monitor`

**版本**: 0.1.0

#### 功能描述

事件监控扩展示例，演示如何订阅和响应 NeoMind 系统事件。

**场景**：监听设备事件、规则事件、代理事件等，统计和分析事件。

#### 提供的指标

- `total_events` (整数) - 接收到的总事件数
- `device_events` (整数) - 设备相关事件数
- `rule_events` (整数) - 规则相关事件数
- `agent_events` (整数) - 代理相关事件数
- `last_event_type` (字符串) - 最后一个事件的类型
- `last_event_source` (字符串) - 最后一个事件源

#### 提供的命令

1. **get_stats**
   - 获取事件统计信息
   - 无参数

2. **reset_stats**
   - 重置事件统计
   - 无参数

3. **set_filter**
   - 设置事件过滤器
   - 参数：
     - `event_type` (字符串, 可选) - 事件类型
     - `source` (字符串, 可选) - 事件源

4. **clear_filter**
   - 清除事件过滤器
   - 无参数

#### 使用场景

- 学习事件订阅机制
- 理解事件过滤和路由
- 演示事件驱动的自动化

---

### Virtual Weather Provider

**位置**: `examples/virtual-weather-provider/`

**ID**: `virtual-weather-provider`

**版本**: 0.1.0

#### 功能描述

虚拟天气提供者扩展示例，从 Open-Meteo API（免费，无需 API Key）获取天气数据，并作为虚拟指标注入到设备遥测中。

**场景**：将真实世界的天气数据注入到智能家居系统，用于基于天气的自动化。

#### 提供的指标

- `temperature` (浮点数, C) - 当前温度
- `humidity` (浮点数, %) - 当前湿度
- `wind_speed` (浮点数, km/h) - 风速
- `weather_code` (整数) - 天气代码
- `last_update` (整数) - 最后更新时间戳

#### 提供的命令

1. **set_location**
   - 设置地理位置（经纬度）
   - 参数：
     - `latitude` (浮点数, 必需) - 纬度
     - `longitude` (浮点数, 必需) - 经度

2. **update_weather**
   - 手动更新天气数据
   - 无参数

3. **inject_to_device**
   - 将天气数据注入到指定设备
   - 参数：
     - `device_id` (字符串, 必需) - 设备 ID

4. **get_current_weather**
   - 获取当前天气数据
   - 无参数

5. **set_auto_update**
   - 设置自动更新间隔
   - 参数：
     - `interval_minutes` (整数, 必需) - 更新间隔（分钟）

#### 使用场景

- 集成真实的天气数据到 IoT 系统
- 学习如何从外部 API 获取数据
- 理解虚拟指标的实际应用
- 演示定时任务和后台更新

#### 注意事项

- 使用 Open-Meteo API（免费，无需注册）
- 网络连接是必需的
- API 有速率限制（通常每天 1000 次请求）

---

### Device Helper Example

**位置**: `examples/device-helper-example/`

**ID**: `device-helper-example`

**版本**: 1.0.0

#### 功能描述

DeviceHelper 框架示例，演示如何使用类型安全的 DeviceHelper API 与设备交互。

**场景**：教学示例，展示 DeviceHelper 框架的所有功能。

#### 提供的指标

- `processed_count` (整数) - 已处理的设备数量
- `avg_temperature` (浮点数, C) - 平均温度
- `virtual_outdoor_temp` (浮点数, C) - 虚拟室外温度

#### 提供的命令

1. **analyze_device**
   - 分析设备：读取指标、计算统计、注入虚拟指标
   - 参数：
     - `device_id` (字符串, 必需) - 设备 ID
   - 演示：
     - 读取所有设备指标
     - 获取特定类型的指标
     - 注入分析结果作为虚拟指标
     - 批量读取多个指标

2. **update_weather**
   - 更新天气：注入天气数据作为虚拟指标
   - 参数：
     - `device_id` (字符串, 必需) - 设备 ID
     - `temperature` (浮点数, 可选, 默认 25.0) - 温度
     - `humidity` (浮点数, 可选, 默认 60.0) - 湿度
   - 演示：
     - 批量写入虚拟指标
     - 类型安全的指标写入

3. **get_device_stats**
   - 获取设备统计：查询遥测并计算聚合
   - 参数：
     - `device_id` (字符串, 必需) - 设备 ID
   - 演示：
     - 查询 24 小时遥测历史
     - 计算平均值、最大值聚合

#### 使用场景

- **学习** DeviceHelper 框架的所有 API
- **理解**类型安全的设备交互模式
- **参考**用于开发自己的扩展
- **测试** DeviceHelper 的各项功能

#### API 涵盖

- 读取设备指标
- 写入虚拟指标
- 发送设备命令
- 查询遥测历史
- 聚合指标

---

## Capability Provider 示例

### Device Capability Provider

**位置**: `examples/device-capability-provider/`

#### 功能描述

设备 Capability Provider，为扩展提供设备相关的能力。

**注意**：这不是一个扩展，而是一个 capability provider 库。

#### 提供的能力

1. **DeviceMetricsRead** - 读取设备指标
   - `get_current_metrics(device_id)` - 获取当前指标
   - `get_metric(device_id, metric_name)` - 获取单个指标

2. **DeviceMetricsWrite** - 写入设备指标（包括虚拟指标）
   - `write_metric(device_id, metric, value, is_virtual)` - 写入指标
   - `write_metrics(device_id, metrics)` - 批量写入指标

3. **DeviceControl** - 控制设备
   - `send_command(device_id, command, params)` - 发送命令

4. **TelemetryHistory** - 查询遥测历史
   - `query_telemetry(device_id, metric, start, end)` - 查询历史数据

#### 使用场景

- 学习如何创建自定义 capability provider
- 为扩展提供特定的系统能力
- 理解 capability 系统架构

---

### Runner Capability Provider

**位置**: `examples/runner-capability-provider/`

#### 功能描述

Runner Capability Provider，为扩展提供能力（通过直接访问扩展运行器进程中的核心系统服务）。

**注意**：这不是一个扩展，而是一个 capability provider 库。

#### 提供的能力

通过直接访问核心服务提供更高效的 API 调用：
- 设备服务
- 事件总线
- 存储服务
- 代理系统
- 规则引擎

#### 使用场景

- 学习如何创建高性能的 capability provider
- 在扩展运行器内部提供能力
- 理解扩展运行器的内部架构

---

## 数据推送配置

数据推送模块允许配置推送目标，按计划将设备遥测数据投递到外部服务。

### CLI 示例

```bash
# 列出所有推送目标
neomind push list

# 创建 Webhook 推送目标
neomind push create \
  --name "temperature-webhook" \
  --type webhook \
  --config '{"url":"https://example.com/api/telemetry","headers":{"Authorization":"Bearer token123"}}' \
  --schedule '{"type":"interval","interval_secs":60}' \
  --sources '{"source_patterns":["device:sensor1:temperature"],"only_changes":true}'

# 创建 MQTT 推送目标
neomind push create \
  --name "mqtt-broker" \
  --type mqtt \
  --config '{"broker":"mqtt://broker.example.com:1883","topic":"neomind/telemetry"}' \
  --schedule '{"type":"event_driven","event_types":["device_metric"]}'

# 测试推送目标
neomind push test <target-id>

# 启动推送目标
neomind push start <target-id>

# 停止推送目标
neomind push stop <target-id>

# 查看投递日志
neomind push logs <target-id> --limit 50

# 查看推送统计
neomind push stats

# 更新推送目标
neomind push update <target-id> --config '{"url":"https://new-url.example.com/hook"}'

# 删除推送目标
neomind push delete <target-id>
```

### API 示例

```bash
# 创建 Webhook 推送目标
curl -X POST http://localhost:9375/api/data-push \
  -H "Content-Type: application/json" \
  -d '{
    "name": "temperature-webhook",
    "target_type": "webhook",
    "config": {
      "url": "https://example.com/api/telemetry",
      "headers": {"Authorization": "Bearer token123"}
    },
    "schedule": {
      "type": "interval",
      "interval_secs": 60
    },
    "data_filter": {
      "source_patterns": ["device:sensor1:temperature"],
      "only_changes": true
    }
  }'

# 列出推送目标
curl http://localhost:9375/api/data-push

# 测试推送目标
curl -X POST http://localhost:9375/api/data-push/<target-id>/test

# 查看投递日志
curl "http://localhost:9375/api/data-push/<target-id>/logs?limit=20&offset=0"
```

### 推送目标类型

| 类型 | 描述 | 配置字段 |
|------|------|----------|
| `webhook` | HTTP POST 到外部 URL | `url`、`headers`、`method` |
| `mqtt` | 发布到 MQTT Broker | `broker`、`topic`、`username`、`password` |

### 调度类型

| 类型 | 描述 |
|------|------|
| `interval` | 定期从时序存储拉取最新数据 |
| `event_driven` | 匹配数据到达时通过 EventBus 立即推送 |

### Web UI

导航到**数据探索** (`/data`)，切换到**推送目标**选项卡，可视化地管理推送目标。

---

## 通知通道设置

NeoMind 支持 7 种通知通道类型，用于发送告警和消息。

### CLI 示例

```bash
# 列出可用通道类型
neomind channel types

# 列出已配置的通道
neomind channel list

# 创建 Webhook 通道
neomind channel create \
  --name "alert-webhook" \
  --type webhook \
  --config '{"url":"https://hooks.example.com/alert","method":"POST"}'

# 创建钉钉机器人通道
neomind channel create \
  --name "team-alerts" \
  --type dingtalk \
  --config '{"webhook_url":"https://oapi.dingtalk.com/robot/send?access_token=xxx","secret":"your-secret"}'

# 创建企业微信机器人通道
neomind channel create \
  --name "ops-alerts" \
  --type wecom \
  --config '{"webhook_url":"https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx"}'

# 创建邮件通道
neomind channel create \
  --name "email-alerts" \
  --type email \
  --config '{"smtp_host":"smtp.example.com","smtp_port":587,"from":"neo@example.com","to":["admin@example.com"],"username":"neo","password":"pass"}'

# 测试通道
neomind channel test --name "alert-webhook"

# 启用/禁用通道
neomind channel enable --name "alert-webhook"
neomind channel disable --name "alert-webhook"

# 删除通道
neomind channel delete --name "alert-webhook"
```

### API 示例

```bash
# 列出通道
curl http://localhost:9375/api/messages/channels

# 创建 Webhook 通道
curl -X POST http://localhost:9375/api/messages/channels \
  -H "Content-Type: application/json" \
  -d '{
    "name": "alert-webhook",
    "type": "webhook",
    "config": {"url": "https://hooks.example.com/alert"}
  }'

# 测试通道
curl -X POST http://localhost:9375/api/messages/channels/alert-webhook/test

# 获取通道统计
curl http://localhost:9375/api/messages/channels/stats
```

### 支持的通道类型

| 类型 | 描述 | 必需配置 |
|------|------|----------|
| `webhook` | HTTP Webhook | `url` |
| `email` | 通过 SMTP 发送邮件 | `smtp_host`、`from`、`to` |
| `telegram` | Telegram Bot | `bot_token`、`chat_id` |
| `dingtalk` | 钉钉机器人 | `webhook_url` |
| `wecom` | 企业微信机器人 | `webhook_url` |
| `slack` | Slack Webhook | `webhook_url` |
| `feishu` | 飞书/Lark 机器人 | `webhook_url` |

### Web UI

导航到**消息** (`/messages`)，切换到**通道**选项卡。点击"添加通道"打开 ChannelEditorDialog（带侧边栏的全屏对话框，用于选择通道类型）。

---

## 使用 AI 聊天管理设备

NeoMind 的 AI 聊天界面允许通过自然语言管理设备。AI Agent 内部使用 CLI 工具执行命令。

### 示例对话

**注册新设备：**

```
用户："我有一个新的温度传感器要添加，它通过 MQTT 连接。"
AI：[使用 shell 工具运行 `neomind device create --name "temp-sensor-1" --type mqtt ...`]
    "设备 'temp-sensor-1' 已创建。它将连接到内嵌的 MQTT Broker (localhost:1883)。"
```

**查看设备状态：**

```
用户："显示我所有的设备"
AI：[运行 `neomind device list`]
    "您有 3 个设备：
    - temp-sensor-1 (在线，2 分钟前活跃)
    - camera-01 (离线)
    - smart-plug-3 (在线)"
```

**向设备发送命令：**

```
用户："关闭 smart-plug-3"
AI：[运行 `neomind device command smart-plug-3 --command power_off`]
    "命令 'power_off' 已发送到 smart-plug-3。"
```

**获取设备接入信息：**

```
用户："如何连接 ESP32 设备？"
AI：[运行 `neomind system info` 获取 Broker 信息]
    "您的 MQTT Broker 运行在 localhost:1883。对于 ESP32，请使用以下代码：
    ```cpp
    WiFiClient espClient;
    PubSubClient client(espClient);
    client.setServer("YOUR_SERVER_IP", 1883);
    client.connect("esp32-device-1");
    client.publish("neomind/device/esp32-device-1/telemetry", "{\"temperature\":25.5}");
    ```"
```

### 使用技巧

- 尽量明确设备名称和类型
- AI 可以创建、列出、更新和删除设备
- 使用 GlobalChatFab（浮动按钮）从任何页面快速访问
- AI 可以诊断离线设备并建议修复方案

---

## 创建自动化规则

NeoMind 使用 DSL（领域特定语言）编写自动化规则。规则可以通过 CLI、API 或 AI 聊天创建。

### 规则 DSL 语法

```
RULE <规则名>
  WHEN <触发条件>
  DO <动作>
END
```

### CLI 示例

```bash
# 创建温度告警规则
neomind rule create --dsl 'RULE high_temp_alert
  WHEN device:sensor1:temperature > 30
  DO notify(channel="alert-webhook", message="温度过高: {{value}}")
END'

# 列出所有规则
neomind rule list

# 获取规则详情
neomind rule get <rule-id>

# 更新规则
neomind rule update <rule-id> --dsl 'RULE high_temp_alert
  WHEN device:sensor1:temperature > 35
  DO notify(channel="alert-webhook", message="严重: 温度 {{value}}")
END'

# 删除规则
neomind rule delete <rule-id>
```

### API 示例

```bash
# 通过 API 创建规则
curl -X POST http://localhost:9375/api/rules \
  -H "Content-Type: application/json" \
  -d '{"dsl": "RULE high_temp_alert\n  WHEN device:sensor1:temperature > 30\n  DO notify(channel=\"alert-webhook\", message=\"温度过高\")\nEND"}'

# 列出规则
curl http://localhost:9375/api/rules
```

### 使用 AI 聊天

```
用户："创建一个规则，当温度超过 30 度时通过 Webhook 通知我"
AI：[通过 CLI 工具创建规则]
    "规则 'high_temp_alert' 已创建。当 device:sensor1:temperature > 30 时将通过 'alert-webhook' 发送通知。"
```

### Web UI

导航到**自动化** (`/automation`) 使用规则构建器界面可视化地管理规则。

---

## 如何使用这些示例

### 1. 构建示例

```bash
# 构建所有示例
cargo build --workspace

# 构建特定示例
cargo build -p virtual-metrics-extension
cargo build -p event-monitor-extension
cargo build -p virtual-weather-provider
cargo build -p device-helper-example
```

### 2. 加载到 NeoMind

```bash
# 通过 CLI 加载
neomind extension load path/to/extension

# 或通过 Web UI 加载
# 导航到扩展 -> 添加扩展
```

### 3. 测试功能

```bash
# 通过 CLI
neomind extension execute virtual-weather-provider set_location \
    --latitude 39.9 \
    --longitude 116.4

neomind extension execute virtual-weather-provider update_weather

# 通过 API
curl -X POST http://localhost:9375/api/extensions/virtual-weather-provider/commands/set_location \
    -H "Content-Type: application/json" \
    -d '{"latitude": 39.9, "longitude": 116.4}'
```

### 4. 查看指标

```bash
# 通过 CLI
neomind extension metrics virtual-weather-provider

# 通过 API
curl http://localhost:9375/api/extensions/virtual-weather-provider/metrics
```

---

## 示例对比

| 示例 | 类型 | 主要用途 | 学习重点 |
|------|------|----------|----------|
| **Virtual Metrics** | 扩展 | 注入虚拟指标 | 状态管理、虚拟指标 API |
| **Event Monitor** | 扩展 | 监听系统事件 | 事件订阅、事件过滤 |
| **Virtual Weather** | 扩展 | 集成外部天气数据 | 外部 API 调用、定时任务 |
| **Device Helper** | 扩展示例 | 展示 DeviceHelper 框架 | 类型安全 API、设备交互 |
| **Device Capability Provider** | Capability Provider | 提供设备能力 | Capability 系统架构 |
| **Runner Capability Provider** | Capability Provider | 提供运行器能力 | 高性能能力提供 |

---

## 扩展开发建议

### 初学者

推荐学习顺序：
1. **Virtual Metrics Extension** - 最简单，了解基本结构
2. **Device Helper Example** - 学习完整的设备交互 API
3. **Event Monitor Extension** - 了解事件订阅机制

### 进阶开发者

推荐学习顺序：
1. **Virtual Weather Provider** - 学习外部 API 集成
2. **Device Capability Provider** - 了解 capability 系统设计
3. **Runner Capability Provider** - 学习高性能架构

### 实战项目

基于这些示例，你可以开发：
- 智能家居自动化扩展
- 数据分析和可视化扩展
- 第三方服务集成扩展
- 自定义自动化规则扩展
- 设备适配器扩展

---

## 故障排查

### 示例无法加载

- 检查扩展是否编译成功：`cargo build -p <example-name>`
- 检查 ABI 版本是否匹配
- 查看日志文件中的错误信息

### 命令执行失败

- 验证参数格式是否正确
- 检查设备 ID 是否存在
- 确认扩展有足够的权限

### 虚拟指标未显示

- 检查设备 ID 是否正确
- 确认遥测存储已启用
- 验证指标名称拼写是否正确

### 事件监控无数据

- 确认事件总线已启动
- 检查事件过滤器配置
- 验证事件源是否产生事件

### 推送目标未投递

- 检查推送目标是否已启动：`neomind push list`
- 查看投递日志：`neomind push logs <target-id>`
- 测试目标：`neomind push test <target-id>`
- 确认外部端点可访问

### 通知通道不工作

- 测试通道：`neomind channel test --name <channel-name>`
- 检查通道是否已启用：`neomind channel list`
- 验证通道配置（URL、凭证）

---

## 相关文档

- **扩展开发指南**: `docs/guides/zh/16-extension-dev.md`
- **DeviceHelper 框架**: `docs/guides/zh/framework-summary.md`
- **Extension SDK**: `crates/neomind-extension-sdk/`
- **Capability 系统**: `crates/neomind-core/src/extension/context.rs`
- **API 参考**: `docs/guides/zh/14-api.md`
- **LLM 配置**: `docs/guides/zh/02-llm.md`
- **设备管理**: `docs/guides/zh/04-devices.md`

---

## 贡献

如果你想添加新的扩展示例：

1. 在 `examples/` 下创建新目录
2. 添加 `Cargo.toml` 和 `src/lib.rs`
3. 在根 `Cargo.toml` 的 `members` 中添加你的示例
4. 编写清晰的文档和注释
5. 提交 Pull Request

---

**最后更新**: 2026-05-26
