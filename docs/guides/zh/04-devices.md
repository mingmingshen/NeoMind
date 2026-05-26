# Devices 模块

**包名**: `neomind-devices`
**版本**: 0.8.0
**完成度**: 90%
**用途**: 设备管理与协议适配

## 概述

Devices模块负责设备注册、发现、管理和与各种设备协议的通信。

## 模块结构

```
crates/neomind-devices/src/
├── lib.rs                      # 公开接口与重导出
├── adapter.rs                  # DeviceAdapter trait、DeviceEvent、ConnectionStatus
├── adapters/
│   ├── mod.rs                  # 适配器工厂 (create_adapter, available_adapters)
│   ├── mqtt.rs                 # MQTT适配器（完整实现）
│   └── webhook.rs              # Webhook适配器（被动数据接收）
├── mdl.rs                      # MDL核心类型（MetricValue、DeviceState等）
├── mdl_format.rs               # MDL格式（DeviceTypeDefinition、CommandDefinition）
├── mqtt.rs                     # MQTT协议工具
├── protocol/
│   ├── mod.rs                  # 协议映射模块
│   ├── mapping.rs              # 协议映射定义
│   └── mqtt_mapping.rs         # MQTT主题/负载映射
├── registry.rs                 # DeviceRegistry（DeviceConfig、DeviceTypeTemplate）
├── service.rs                  # DeviceService（统一API、命令、健康检查）
├── telemetry.rs                # TimeSeriesStorage与MetricCache
├── unified_extractor.rs        # 所有适配器的UnifiedExtractor
└── embedded_broker.rs          # 嵌入式MQTT Broker（feature门控）
```

## 核心概念

### 1. DeviceConfig - 设备配置

```rust
pub struct DeviceConfig {
    /// 设备ID（唯一）
    pub device_id: String,

    /// 设备名称
    pub name: String,

    /// 设备类型（关联DeviceTypeTemplate）
    pub device_type: String,

    /// 适配器类型（mqtt, webhook, hass）
    pub adapter_type: String,

    /// 连接配置（统一结构体）
    pub connection_config: ConnectionConfig,

    /// 管理该设备的适配器ID
    pub adapter_id: Option<String>,

    /// 最后在线时间戳（0 = 从未连接）
    pub last_seen: i64,
}

/// 统一的协议连接配置
pub struct ConnectionConfig {
    // MQTT专用
    pub telemetry_topic: Option<String>,
    pub command_topic: Option<String>,
    pub json_path: Option<String>,

    // HASS专用
    pub entity_id: Option<String>,

    // 通用元数据
    pub extra: HashMap<String, serde_json::Value>,
}
```

### 2. DeviceTypeTemplate - 设备类型模板

```rust
pub struct DeviceTypeTemplate {
    /// 设备类型ID（如: dht22_sensor）
    pub device_type: String,

    /// 显示名称
    pub name: String,

    /// 描述
    pub description: String,

    /// 分类标签（可多个）
    pub categories: Vec<String>,

    /// 定义模式: Simple（原始数据+LLM）或 Full（结构化定义）
    pub mode: DeviceTypeMode,

    /// 指标定义
    pub metrics: Vec<MetricDefinition>,

    /// Simple模式的上行示例数据
    pub uplink_samples: Vec<serde_json::Value>,

    /// 命令定义
    pub commands: Vec<CommandDefinition>,
}

pub enum DeviceTypeMode {
    Simple,  // 原始数据 + LLM自动发现
    Full,    // 结构化定义
}
```

### 3. DeviceAdapter - 设备适配器接口

```rust
#[async_trait]
pub trait DeviceAdapter: Send + Sync {
    /// 获取适配器名称
    fn name(&self) -> &str;

    /// 获取适配器类型标识（如 "mqtt"、"webhook"）
    fn adapter_type(&self) -> &'static str;

    /// 检查适配器是否运行中
    fn is_running(&self) -> bool;

    /// 启动适配器
    async fn start(&self) -> AdapterResult<()>;

    /// 停止适配器
    async fn stop(&self) -> AdapterResult<()>;

    /// 订阅设备事件
    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = DeviceEvent> + Send + '_>>;

    /// 发送命令到设备
    async fn send_command(
        &self,
        device_id: &str,
        command_name: &str,
        payload: String,
        topic: Option<String>,
    ) -> AdapterResult<()>;

    /// 获取连接状态
    fn connection_status(&self) -> ConnectionStatus;

    /// 获取设备数量
    fn device_count(&self) -> usize;

    /// 订阅设备数据流
    async fn subscribe_device(&self, device_id: &str) -> AdapterResult<()>;

    /// 取消订阅设备数据流
    async fn unsubscribe_device(&self, device_id: &str) -> AdapterResult<()>;
}
```

## 适配器实现

### MQTT适配器

MQTT适配器提供完整的MQTT连接能力：
- 新设备自动发现
- 基于模板的数据解析
- 协议映射（主题/负载到指标）
- 通过MQTT主题发送命令
- 与嵌入式Broker集成（feature门控）

**MQTT主题规范**:
```
{telemetry_topic}              # 设备数据（按设备配置）
{command_topic}                # 设备命令（按设备配置）
```

### Webhook适配器

Webhook适配器通过HTTP POST接收设备数据：

```rust
pub struct WebhookAdapterConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub allowed_ips: Vec<String>,
    pub blocked_ips: Vec<String>,
    pub rate_limit_per_minute: Option<u32>,
    pub storage_dir: Option<String>,
}
```

**Webhook URL格式**: `POST /api/devices/{device_id}/webhook`

**特性**：
- 通过webhook端点被动接收数据
- 设备认证支持（API密钥）
- IP白名单/黑名单
- 请求速率限制
- 自动设备发现
- 命令支持（通过响应体）

## 设备发现

设备发现通过适配器系统处理。发现的设备会产生 `DeviceEvent::Discovery` 事件，触发自动入板流程：

```rust
pub struct DiscoveredDeviceInfo {
    pub device_id: String,
    pub device_type: String,
    pub name: Option<String>,
    pub endpoint: Option<String>,
    pub capabilities: Vec<String>,
    pub timestamp: i64,
    pub metadata: serde_json::Value,
}
```

发现的设备可以：
1. 通过LLM自动分析
2. 通过建议设备类型增强
3. 由用户批准或拒绝
4. 转换为完整的设备实例

## 设备注册表

```rust
pub struct DeviceRegistry {
    /// 存储后端（内存或通过redb持久化）
    store: Arc<DeviceRegistryStore>,

    /// 设备类型模板
    templates: DashMap<String, DeviceTypeTemplate>,
}

impl DeviceRegistry {
    /// 创建内存注册表
    pub fn new() -> Self;

    /// 创建持久化注册表（redb）
    pub async fn with_persistence(path: impl AsRef<Path>) -> Result<Self>;

    /// 注册设备类型模板
    pub async fn register_template(&self, template: DeviceTypeTemplate) -> Result<()>;

    /// 注册设备
    pub async fn register_device(&self, config: DeviceConfig) -> Result<()>;

    /// 获取设备
    pub async fn get_device(&self, id: &str) -> Option<DeviceConfig>;

    /// 列出设备
    pub async fn list_devices(&self) -> Vec<DeviceConfig>;

    /// 更新设备
    pub async fn update_device(&self, id: &str, config: DeviceConfig) -> Result<()>;

    /// 删除设备
    pub async fn delete_device(&self, id: &str) -> Result<()>;
}
```

## 设备服务

```rust
pub struct DeviceService {
    registry: Arc<DeviceRegistry>,
    adapters: Arc<RwLock<HashMap<String, Arc<dyn DeviceAdapter>>>>,
    event_bus: EventBus,
    telemetry: Arc<TimeSeriesStorage>,
    extension_command_router: Option<ExtensionCommandRouterFn>,
}

impl DeviceService {
    /// 创建设备服务
    pub fn new(
        registry: Arc<DeviceRegistry>,
        event_bus: EventBus,
        telemetry: Arc<TimeSeriesStorage>,
    ) -> Self;

    /// 注册适配器
    pub async fn register_adapter(
        &self,
        id: String,
        adapter: Arc<dyn DeviceAdapter>,
    ) -> Result<()>;

    /// 发送命令（自动选择适配器并使用模板构建负载）
    pub async fn send_command(
        &self,
        device_id: &str,
        command_name: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<CommandHistoryRecord>;

    /// 获取适配器统计
    pub async fn get_adapter_stats(&self) -> AdapterStats;

    /// 获取设备健康状态
    pub async fn get_device_health(&self, device_id: &str) -> DeviceHealth;

    /// 设置扩展命令路由
    pub fn set_extension_command_router(&self, router: ExtensionCommandRouterFn);
}
```

## MDL格式

MDL (Device Description Language) 用于描述设备类型：

```rust
pub struct MetricDataType;

impl MetricDataType {
    pub const BOOLEAN: &str = "boolean";
    pub const INTEGER: &str = "integer";
    pub const FLOAT: &str = "float";
    pub const STRING: &str = "string";
    pub const ARRAY: &str = "array";
    pub const OBJECT: &str = "object";
}
```

**MDL示例**:
```json
{
  "device_type": "dht22_sensor",
  "name": "DHT22温湿度传感器",
  "description": "基于DHT22的温湿度传感器",
  "categories": ["sensor", "climate"],
  "metrics": [
    {
      "name": "temperature",
      "display_name": "温度",
      "data_type": "float",
      "unit": "°C",
      "min": -40.0,
      "max": 80.0
    },
    {
      "name": "humidity",
      "display_name": "湿度",
      "data_type": "float",
      "unit": "%",
      "min": 0.0,
      "max": 100.0
    }
  ]
}
```

## API端点

```
# 设备管理
GET    /api/devices                           # 列出设备
POST   /api/devices                           # 添加设备
GET    /api/devices/:id                       # 获取设备详情
PUT    /api/devices/:id                       # 更新设备
DELETE /api/devices/:id                       # 删除设备
GET    /api/devices/:id/current               # 当前指标值
POST   /api/devices/current-batch             # 批量当前值

# 命令
POST   /api/devices/:id/command/:command      # 发送命令
GET    /api/devices/:id/commands              # 命令历史

# 指标数据
POST   /api/devices/:id/metrics               # 写入指标数据

# 遥测
GET    /api/devices/:id/telemetry             # 遥测数据
GET    /api/devices/:id/telemetry/summary     # 遥测摘要

# BLE配网
POST   /api/devices/ble-provision             # BLE设备配网

# 设备类型
GET    /api/device-types                      # 列出设备类型
POST   /api/device-types                      # 创建设备类型
GET    /api/device-types/:id                  # 获取设备类型
PUT    /api/device-types                      # 验证设备类型
DELETE /api/device-types/:id                  # 删除设备类型
POST   /api/device-types/generate-from-samples  # 从示例生成
POST   /api/device-types/cloud/import         # 从云端导入

# MDL生成
POST   /api/devices/generate-mdl              # 生成MDL

# 设备发现与自动入板（草稿）
GET    /api/devices/drafts                    # 列出草稿设备
GET    /api/devices/drafts/:device_id         # 获取草稿设备
PUT    /api/devices/drafts/:device_id         # 更新草稿
POST   /api/devices/drafts/:device_id/approve # 批准草稿
POST   /api/devices/drafts/:device_id/reject  # 拒绝草稿
POST   /api/devices/drafts/:device_id/analyze # LLM分析
POST   /api/devices/drafts/:device_id/enhance # LLM增强
GET    /api/devices/drafts/:device_id/suggest-types  # 建议类型
POST   /api/devices/drafts/cleanup            # 清理草稿
GET    /api/devices/drafts/type-signatures    # 获取类型签名
GET    /api/devices/drafts/config             # 获取入板配置
PUT    /api/devices/drafts/config             # 更新入板配置
POST   /api/devices/drafts/upload             # 上传设备数据
```

## 使用示例

### 注册设备类型

```rust
use neomind_devices::{DeviceTypeTemplate, DeviceTypeMode};

let template = DeviceTypeTemplate::new("dht22_sensor", "DHT22温湿度传感器")
    .with_description("基于DHT22的温湿度传感器")
    .with_category("sensor")
    .with_category("climate");

service.register_template(template).await?;
```

### 添加设备

```rust
use neomind_devices::{DeviceConfig, ConnectionConfig};

let device = DeviceConfig {
    device_id: "greenhouse_temp_1".to_string(),
    name: "温室温度传感器1".to_string(),
    device_type: "dht22_sensor".to_string(),
    adapter_type: "mqtt".to_string(),
    connection_config: ConnectionConfig::new()
        .with_telemetry_topic("sensors/greenhouse/temp1"),
    adapter_id: Some("main-mqtt".to_string()),
    last_seen: 0,
};

service.register_device(device).await?;
```

### 发送命令

```rust
let result = service.send_command(
    "greenhouse_fan_1",
    "turn_on",
    HashMap::new(),
).await?;
```

## 已清理的功能

以下功能已在代码清理中移除：
- Modbus适配器（未实现）
- Home Assistant发现模块（已废弃）
- Agent分析工具（异常检测、趋势分析 - 未使用）
- HTTP轮询适配器（已由Webhook适配器替代）
- 独立的发现模块（已集成到适配器中）

## 设计原则

1. **协议解耦**: 通过适配器模式支持多种协议
2. **类型驱动**: 通过DeviceTypeTemplate定义设备能力
3. **事件驱动**: 设备状态变化通过EventBus通知
4. **基于模板的解析**: 适配器使用设备模板解析协议数据
5. **可扩展**: 易于添加新的协议适配器
6. **统一提取**: UnifiedExtractor处理所有适配器的数据提取
