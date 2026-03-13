# NeoMind 扩展示例说明

## 目录

1. [扩展示例](#扩展示例)
   - [Virtual Metrics Extension](#virtual-metrics-extension)
   - [Event Monitor Extension](#event-monitor-extension)
   - [Virtual Weather Provider](#virtual-weather-provider)
   - [Device Helper Example](#device-helper-example)
2. [Capability Provider 示例](#capability-provider-示例)
   - [Device Capability Provider](#device-capability-provider)
   - [Runner Capability Provider](#runner-capability-provider)

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

- `injection_count` (Integer) - 已注入的虚拟指标计数

#### 提供的命令

1. **set_target_device**
   - 设置目标设备 ID，用于注入虚拟指标
   - 参数：
     - `device_id` (String, 必需) - 设备 ID

2. **inject_virtual_metrics**
   - 向目标设备注入虚拟指标
   - 参数：
     - `metric_name` (String, 必需) - 指标名称
     - `value` (Float, 必需) - 指标值

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

- `total_events` (Integer) - 接收到的总事件数
- `device_events` (Integer) - 设备相关事件数
- `rule_events` (Integer) - 规则相关事件数
- `agent_events` (Integer) - 代理相关事件数
- `last_event_type` (String) - 最后一个事件的类型
- `last_event_source` (String) - 最后一个事件源

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
     - `event_type` (String, 可选) - 事件类型
     - `source` (String, 可选) - 事件源

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

- `temperature` (Float, °C) - 当前温度
- `humidity` (Float, %) - 当前湿度
- `wind_speed` (Float, km/h) - 风速
- `weather_code` (Integer) - 天气代码
- `last_update` (Integer) - 最后更新时间戳

#### 提供的命令

1. **set_location**
   - 设置地理位置（经纬度）
   - 参数：
     - `latitude` (Float, 必需) - 纬度
     - `longitude` (Float, 必需) - 经度

2. **update_weather**
   - 手动更新天气数据
   - 无参数

3. **inject_to_device**
   - 将天气数据注入到指定设备
   - 参数：
     - `device_id` (String, 必需) - 设备 ID

4. **get_current_weather**
   - 获取当前天气数据
   - 无参数

5. **set_auto_update**
   - 设置自动更新间隔
   - 参数：
     - `interval_minutes` (Integer, 必需) - 更新间隔（分钟）

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

- `processed_count` (Integer) - 已处理的设备数量
- `avg_temperature` (Float, °C) - 平均温度
- `virtual_outdoor_temp` (Float, °C) - 虚拟室外温度

#### 提供的命令

1. **analyze_device**
   - 分析设备：读取指标、计算统计、注入虚拟指标
   - 参数：
     - `device_id` (String, 必需) - 设备 ID
   - 演示：
     - 读取所有设备指标
     - 获取特定类型的指标
     - 注入分析结果作为虚拟指标
     - 批量读取多个指标

2. **update_weather**
   - 更新天气：注入天气数据作为虚拟指标
   - 参数：
     - `device_id` (String, 必需) - 设备 ID
     - `temperature` (Float, 可选, 默认 25.0) - 温度
     - `humidity` (Float, 可选, 默认 60.0) - 湿度
   - 演示：
     - 批量写入虚拟指标
     - 类型安全的指标写入

3. **get_device_stats**
   - 获取设备统计：查询遥测并计算聚合
   - 参数：
     - `device_id` (String, 必需) - 设备 ID
   - 演示：
     - 查询 24 小时遥测历史
     - 计算平均值、最大值聚合

#### 使用场景

- **学习** DeviceHelper 框架的所有 API
- **理解**类型安全的设备交互模式
- **参考**用于开发自己的扩展
- **测试** DeviceHelper 的各项功能

#### API 涵盖

✅ 读取设备指标
✅ 写入虚拟指标
✅ 发送设备命令
✅ 查询遥测历史
✅ 聚合指标

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

在 NeoMind 中加载扩展：

```bash
# 通过 CLI 加载
neomind-cli extension load path/to/extension

# 或通过 Web UI 加载
# 导航到设置 -> 扩展 -> 添加扩展
```

### 3. 测试功能

使用 NeoMind CLI 或 API 测试扩展命令：

```bash
# 通过 CLI
neomind-cli extension execute virtual-weather-provider set_location \
    --latitude 39.9 \
    --longitude 116.4

neomind-cli extension execute virtual-weather-provider update_weather

# 通过 API
curl -X POST http://localhost:9375/api/extensions/virtual-weather-provider/commands/set_location \
    -H "Content-Type: application/json" \
    -d '{"latitude": 39.9, "longitude": 116.4}'
```

### 4. 查看指标

检查扩展提供的指标：

```bash
# 通过 CLI
neomind-cli extension metrics virtual-weather-provider

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

---

## 相关文档

- **扩展开发指南**: `docs/guides/en/16-extension-dev.md`
- **DeviceHelper 框架**: `docs/guides/en/framework-summary.md`
- **Extension SDK**: `crates/neomind-extension-sdk/`
- **Capability 系统**: `crates/neomind-core/src/extension/context.rs`

---

## 贡献

如果你想添加新的扩展示例：

1. 在 `examples/` 下创建新目录
2. 添加 `Cargo.toml` 和 `src/lib.rs`
3. 在根 `Cargo.toml` 的 `members` 中添加你的示例
4. 编写清晰的文档和注释
5. 提交 Pull Request

---

**最后更新**: 2026-03-08
