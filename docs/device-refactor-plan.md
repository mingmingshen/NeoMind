# 设备管理完全重构计划

## 目标
彻底移除对 `MqttDeviceManager` 的依赖，所有设备管理功能通过新架构（`DeviceService` + `DeviceRegistry` + `DeviceAdapter`）实现。

## 当前状况分析

### 1. MqttDeviceManager 的职责
- ✅ **已迁移**：设备实例管理 → `DeviceService` + `DeviceRegistry`
- ✅ **已迁移**：设备类型管理 → `DeviceService` + `DeviceRegistry`
- ⚠️ **待迁移**：MQTT 连接管理
- ⚠️ **待迁移**：命令发送（`send_command`）
- ⚠️ **待迁移**：HASS 发现功能
- ⚠️ **待迁移**：设备订阅管理
- ⚠️ **待迁移**：连接状态查询

### 2. DeviceAdapter 接口现状
- ✅ 已定义基本接口：`start()`, `stop()`, `subscribe()`
- ❌ **缺失**：`send_command()` 方法
- ❌ **缺失**：`connection_status()` 方法
- ❌ **缺失**：HASS 发现相关方法

### 3. DeviceService 现状
- ✅ 设备配置 CRUD
- ✅ 设备类型模板 CRUD
- ✅ 遥测数据查询
- ✅ 当前指标读取
- ⚠️ **占位符**：`send_command()` 未实现（需要适配器支持）

## 重构步骤

### 阶段 1: 扩展 DeviceAdapter 接口

#### 1.1 添加命令发送方法
```rust
// crates/devices/src/adapter.rs

#[async_trait]
pub trait DeviceAdapter: Send + Sync {
    // ... 现有方法 ...
    
    /// Send a command to a device
    /// Returns the command result or an error
    async fn send_command(
        &self,
        device_id: &str,
        command_name: &str,
        payload: String,
        topic: Option<String>,
    ) -> AdapterResult<()>;
    
    /// Get connection status for this adapter
    fn connection_status(&self) -> ConnectionStatus;
    
    /// Subscribe to a device topic (for MQTT adapters)
    async fn subscribe_device(&self, device_id: &str) -> AdapterResult<()>;
}
```

#### 1.2 添加 HASS 发现接口（可选）
```rust
/// HASS discovery functionality
#[async_trait]
pub trait HassDiscoveryAdapter: DeviceAdapter {
    /// Start HASS discovery
    async fn start_hass_discovery(&self) -> AdapterResult<()>;
    
    /// Stop HASS discovery
    async fn stop_hass_discovery(&self) -> AdapterResult<()>;
    
    /// Get discovered HASS devices
    async fn get_discovered_devices(&self) -> Vec<DiscoveredHassDevice>;
    
    /// Get aggregated HASS devices
    async fn get_aggregated_devices(&self) -> Vec<AggregatedHassDevice>;
}
```

### 阶段 2: 创建 MQTT 适配器实现

#### 2.1 创建 MqttAdapter 结构
```rust
// crates/devices/src/adapters/mqtt_adapter.rs

pub struct MqttAdapter {
    manager: Arc<MqttDeviceManager>,
    // ... 其他字段
}
```

#### 2.2 实现 DeviceAdapter trait
- 包装 `MqttDeviceManager` 的方法
- 实现 `send_command()` → 调用 `manager.send_command()`
- 实现 `connection_status()` → 调用 `manager.connection_status()`
- 实现 `subscribe_device()` → 调用 `manager.subscribe_device()`

#### 2.3 实现 HassDiscoveryAdapter（如果需要）
- 包装 HASS 发现相关方法

### 阶段 3: 实现 DeviceService.send_command()

#### 3.1 完善命令发送逻辑
```rust
// crates/devices/src/service.rs

pub async fn send_command(...) -> Result<Option<MetricValue>, DeviceError> {
    // 1. 获取设备和模板
    // 2. 验证命令和参数
    // 3. 构建 payload
    // 4. 获取适配器
    // 5. 调用适配器的 send_command()
    // 6. 返回结果
}
```

### 阶段 4: 重构 ServerState

#### 4.1 移除 mqtt_device_manager
```rust
// crates/api/src/server/types.rs

pub struct ServerState {
    // ❌ 移除: pub mqtt_device_manager: Arc<MqttDeviceManager>,
    // ✅ 保留: pub device_service: Arc<DeviceService>,
    // ✅ 保留: pub device_registry: Arc<DeviceRegistry>,
    // ✅ 保留: pub multi_broker_manager: Arc<MultiBrokerManager>,
}
```

#### 4.2 初始化 MQTT 适配器
```rust
impl ServerState {
    pub async fn init_device_storage(&self) {
        // 1. 创建 MqttDeviceManager（仅作为适配器内部使用）
        // 2. 创建 MqttAdapter
        // 3. 注册适配器到 DeviceService
        // 4. 启动适配器
    }
}
```

### 阶段 5: 迁移剩余 Handlers

#### 5.1 hass.rs
- 通过 `DeviceService` 获取适配器
- 如果适配器实现 `HassDiscoveryAdapter`，调用相应方法
- 否则，暂时保留对 `MqttDeviceManager` 的访问（通过适配器）

#### 5.2 mqtt/status.rs
- 从适配器获取连接状态
- 通过 `DeviceService` 获取设备列表

#### 5.3 mqtt/subscriptions.rs
- 通过 `DeviceService` 获取设备列表
- 通过适配器管理订阅

### 阶段 6: 清理和标记 Deprecated

#### 6.1 标记 MqttDeviceManager 为 deprecated
```rust
// crates/devices/src/mqtt_v2.rs

#[deprecated(note = "Use DeviceService and DeviceAdapter instead. MqttDeviceManager is now only used internally by MqttAdapter.")]
pub struct MqttDeviceManager {
    // ...
}
```

#### 6.2 更新文档
- 说明新的架构
- 迁移指南

## 实施顺序

1. ✅ **阶段 1**: 扩展 DeviceAdapter 接口
2. ✅ **阶段 2**: 创建 MQTT 适配器包装
3. ✅ **阶段 3**: 实现 DeviceService.send_command()
4. ✅ **阶段 4**: 重构 ServerState，移除直接依赖
5. ✅ **阶段 5**: 迁移剩余 handlers
6. ✅ **阶段 6**: 标记 deprecated 和清理

## 风险评估

### 高风险
- HASS 发现功能复杂，可能需要大量重构
- MQTT 连接管理可能与适配器生命周期相关

### 中风险
- 命令发送需要在适配器接口中设计好抽象
- 向后兼容性（API 响应格式）

### 低风险
- 设备列表、统计等功能已迁移
- 遥测数据查询已迁移

## 测试计划

1. 单元测试：适配器接口实现
2. 集成测试：DeviceService.send_command()
3. E2E 测试：设备添加 → 命令发送 → 数据查询
4. 回归测试：现有 API 端点功能

## 时间估算

- 阶段 1-2: 2-3 小时
- 阶段 3: 1-2 小时
- 阶段 4: 2-3 小时
- 阶段 5: 3-4 小时
- 阶段 6: 1 小时

总计：约 9-13 小时
