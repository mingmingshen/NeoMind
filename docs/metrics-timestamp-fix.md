# 修复 Real-time Metrics 历史查询问题

## 问题描述

Real-time Metrics 组件无法查询到指标历史数据，导致图表和统计信息显示为空。

## 根本原因

**时间戳单位不一致**导致查询范围与数据存储范围不匹配：

### 问题链条

1. **数据写入**：MQTT adapter 使用 `now.timestamp()` → **秒级**时间戳
2. **数据存储**：TimeSeriesStorage 存储数据时使用秒级时间戳作为 key
3. **前端查询**：使用 `Date.now() / 1000` → **秒级**时间戳
4. **API handler**（修复前）：使用 `to_milliseconds()` 转换 → **毫秒级**时间戳
5. **数据库查询**：使用毫秒级时间戳查询，但数据存储的 key 是秒级
6. **结果**：查询范围不匹配，返回空结果

### 具体示例

```
当前时间：2026-03-16 12:00:00
时间戳（秒）：1740460800
时间戳（毫秒）：1740460800000

数据库中存储的 key：
  device_id:metric:1740460800  ← 秒级

API handler（修复前）查询范围：
  [device_id:metric:1740460800000, device_id:metric:1740460800000]  ← 毫秒级

结果：找不到匹配的数据！
```

## 修复方案

统一使用**秒级时间戳**，移除不必要的毫秒转换。

### 修改的文件

`crates/neomind-api/src/handlers/devices/telemetry.rs`

### 修改内容

#### 1. `get_device_telemetry_handler` (第 62-74 行)

**修复前：**
```rust
// Parse query parameters
// Note: All timestamps in the system are milliseconds since epoch
let metric = params.get("metric").cloned();
let start = params
    .get("start")
    .and_then(|s| s.parse::<i64>().ok())
    .map(|ts| to_milliseconds(ts))  // ← 转换为毫秒
    .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() - 86400 * 1000);
let end = params
    .get("end")
    .and_then(|s| s.parse::<i64>().ok())
    .map(|ts| to_milliseconds(ts))  // ← 转换为毫秒
    .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
```

**修复后：**
```rust
// Parse query parameters
// Note: All timestamps in the storage layer are seconds since epoch
let metric = params.get("metric").cloned();
let start = params
    .get("start")
    .and_then(|s| s.parse::<i64>().ok())
    .unwrap_or_else(|| chrono::Utc::now().timestamp() - 86400); // ← 秒级
let end = params
    .get("end")
    .and_then(|s| s.parse::<i64>().ok())
    .unwrap_or_else(|| chrono::Utc::now().timestamp()); // ← 秒级
```

#### 2. `get_device_telemetry_summary_handler` (第 314-320 行)

**修复前：**
```rust
// Default to last 24 hours (timestamps in milliseconds)
let end = chrono::Utc::now().timestamp_millis();
let start = params
    .get("hours")
    .and_then(|s| s.parse::<i64>().ok())
    .map(|h| end - h * 3600 * 1000)
    .unwrap_or_else(|| end - 86400 * 1000);
```

**修复后：**
```rust
// Default to last 24 hours (timestamps in seconds)
let end = chrono::Utc::now().timestamp();
let start = params
    .get("hours")
    .and_then(|s| s.parse::<i64>().ok())
    .map(|h| end - h * 3600)
    .unwrap_or_else(|| end - 86400);
```

#### 3. `analyze_metric_timestamps_handler` (第 701-706 行)

**修复前：**
```rust
// Get current time for comparison (timestamps in milliseconds)
let now = chrono::Utc::now().timestamp_millis();
// Query all data for this metric (wide time range)
let start = now - 86400 * 2 * 1000; // 2 days in ms
let end = now + 60 * 1000; // 1 minute in future in ms
```

**修复后：**
```rust
// Get current time for comparison (timestamps in seconds)
let now = chrono::Utc::now().timestamp();
// Query all data for this metric (wide time range)
let start = now - 86400 * 2; // 2 days in seconds
let end = now + 60; // 1 minute in future in seconds
```

#### 4. 移除不需要的 `to_milliseconds` 函数 (第 32-53 行)

```rust
// 删除了整个函数，因为不再需要毫秒转换
```

#### 5. 修复 summary handler 中的时间戳 (第 521 行)

**修复前：**
```rust
"current_timestamp": chrono::Utc::now().timestamp_millis(),
```

**修复后：**
```rust
"current_timestamp": chrono::Utc::now().timestamp(),
```

## 验证

### 编译检查

```bash
cargo check -p neomind-api
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.30s
# ✅ 编译成功，无错误
```

### 时间戳一致性检查

修复后的时间戳流程：

```
数据写入：秒级 (timestamp())
  ↓
数据存储：秒级 (key: device_id:metric:timestamp)
  ↓
前端查询：秒级 (Math.floor(Date.now() / 1000))
  ↓
API handler：秒级 (timestamp())
  ↓
数据库查询：秒级 (query(start, end))
  ↓
✅ 查询成功！
```

## 影响范围

- ✅ **Real-time Metrics** 历史数据查询
- ✅ **Device Detail** 页面遥测数据展示
- ✅ **Dashboard** 组件数据源绑定
- ✅ **Telemetry Summary** 统计信息
- ✅ **Debug Endpoints** 时间戳分析

## 测试建议

1. **Real-time Metrics 组件**
   - 检查历史数据是否正常显示
   - 验证时间范围选择器功能
   - 确认图表数据点正确

2. **Device Detail 页面**
   - 查看设备遥测数据
   - 验证时间范围过滤
   - 检查数据点数量

3. **Dashboard 组件**
   - 测试各种数据源类型（LineChart, ValueCard, Sparkline 等）
   - 验证实时更新和历史数据查询
   - 检查聚合数据正确性

## 相关文件

- `crates/neomind-api/src/handlers/devices/telemetry.rs` - API handler 修复
- `crates/neomind-api/src/handlers/capabilities.rs` - Virtual metrics handler 修复
- `crates/neomind-devices/src/telemetry.rs` - 存储层（已正确使用秒级）
- `crates/neomind-devices/src/adapters/mqtt.rs` - 数据写入（已正确使用秒级）
- `web/src/hooks/useDataSource.ts` - 前端查询（已正确使用秒级）

### capabilities.rs 修复

#### 6. `write_virtual_metric_handler` (第 157 行)

**修复前：**
```rust
let timestamp = chrono::Utc::now().timestamp_millis();
```

**修复后：**
```rust
let timestamp = chrono::Utc::now().timestamp();
```

#### 7. `aggregate_metrics_handler` (第 235-263 行)

**修复前：**
```rust
let now = chrono::Utc::now();
let start = _query.start.unwrap_or(now.timestamp_millis() - (86400 * 1000));
let end = _query.end.unwrap_or(now.timestamp_millis());
// ...
"timestamp": now.timestamp_millis(),
```

**修复后：**
```rust
let now = chrono::Utc::now();
let start = _query.start.unwrap_or(now.timestamp() - 86400);
let end = _query.end.unwrap_or(now.timestamp());
// ...
"timestamp": now.timestamp(),
```

## 修复日期

2026-03-16

## 修复版本

v0.5.11
