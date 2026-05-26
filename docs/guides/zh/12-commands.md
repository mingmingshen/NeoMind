# Commands 模块

**包名**: `neomind-devices`（命令执行在DeviceService中）
**版本**: 0.8.0
**完成度**: 80%
**用途**: 设备命令执行和状态跟踪

## 概述

Commands功能已集成到 `neomind-devices` crate的DeviceService中。它管理设备命令发送、状态跟踪和历史记录持久化。命令根据设备配置通过相应的适配器路由。

## 架构

命令不是独立的crate，而是由以下模块处理：
- `neomind-devices/src/service.rs` 中的 `DeviceService` - 命令执行和路由
- `DeviceAdapter` trait - 协议特定的命令发送
- `neomind-storage` - 命令历史持久化（CommandHistoryRecord）

## 核心类型

### 1. CommandHistoryRecord - 命令记录

```rust
pub struct CommandHistoryRecord {
    /// 唯一命令ID
    pub command_id: String,

    /// 设备ID
    pub device_id: String,

    /// 命令名称
    pub command_name: String,

    /// 命令参数
    pub parameters: HashMap<String, serde_json::Value>,

    /// 命令状态
    pub status: CommandStatus,

    /// 结果消息（如果有）
    pub result: Option<String>,

    /// 错误消息（如果失败）
    pub error: Option<String>,

    /// 创建时间戳
    pub created_at: i64,

    /// 完成时间戳
    pub completed_at: Option<i64>,
}
```

### 2. CommandStatus - 命令状态

```rust
pub enum CommandStatus {
    /// 待执行
    Pending,
    /// 执行中
    Executing,
    /// 成功完成
    Success,
    /// 失败
    Failed,
    /// 超时
    Timeout,
}
```

### 3. 命令执行流程

```
API -> DeviceService::send_command()
     -> 从设备模板构建负载
     -> 路由到相应的处理程序：
        a. 扩展设备 -> ExtensionCommandRouter
        b. MQTT设备 -> MqttAdapter::send_command()
        c. 其他 -> DeviceAdapter::send_command()
     -> 记录命令历史
     -> 更新状态（Success/Failed）
```

## 命令路由

### MQTT命令

命令通过适配器发送到MQTT设备：
1. DeviceService查找设备的适配器
2. 从设备类型模板构建命令负载
3. 调用 `adapter.send_command()` 并传入设备ID、命令名称和负载
4. 适配器发布到配置的命令主题

### 扩展命令

扩展管理的设备命令使用ExtensionCommandRouter：
1. DeviceService检测设备是否由扩展管理
2. 通过 `ExtensionCommandRouterFn` 回调路由命令
3. 扩展处理命令并返回结果

## API端点

```
# 设备命令
POST   /api/devices/:id/command/:command      # 发送命令到设备
GET    /api/devices/:id/commands              # 获取命令历史
```

### 发送命令

```bash
# 发送不带参数的命令
curl -X POST http://localhost:9375/api/devices/relay_1/command/turn_on \
  -H "Content-Type: application/json" \
  -d '{}'

# 发送带参数的命令
curl -X POST http://localhost:9375/api/devices/fan_1/command/set_speed \
  -H "Content-Type: application/json" \
  -d '{"speed": 100, "direction": "clockwise"}'
```

### 获取命令历史

```bash
curl http://localhost:9375/api/devices/relay_1/commands
```

响应：
```json
{
  "success": true,
  "data": [
    {
      "command_id": "cmd_abc123",
      "device_id": "relay_1",
      "command_name": "turn_on",
      "parameters": {},
      "status": "Success",
      "result": "Command sent successfully",
      "error": null,
      "created_at": 1717000000,
      "completed_at": 1717000001
    }
  ]
}
```

## 命令状态生命周期

```
Pending -> Executing -> Success
                    -> Failed
                    -> Timeout
```

## 使用示例

### 通过DeviceService发送命令

```rust
use neomind_devices::DeviceService;

let result = service.send_command(
    "greenhouse_fan_1",
    "turn_on",
    HashMap::new(),
).await?;

println!("命令状态: {:?}", result.status);
```

### 发送带参数的命令

```rust
let mut params = HashMap::new();
params.insert("speed".to_string(), serde_json::json!(100));
params.insert("direction".to_string(), serde_json::json!("clockwise"));

let result = service.send_command(
    "fan_1",
    "set_speed",
    params,
).await?;
```

## 已清理的功能

以下功能在架构简化中被移除：
- 独立的 `neomind-commands` crate（合并到DeviceService中）
- 带后台工作线程的CommandQueue（现在在DeviceService中同步执行）
- DownlinkAdapter trait（由DeviceAdapter.send_command()替代）
- RetryPolicy/QueueConfig（简化 - 命令立即失败）

## 设计原则

1. **统一**: 命令是DeviceService的一部分，不是独立模块
2. **基于模板**: 命令负载从设备类型模板构建
3. **适配器路由**: 命令自动使用正确的适配器
4. **扩展支持**: 扩展管理的设备通过扩展路由器路由
5. **历史跟踪**: 所有命令都记录状态和时间戳
