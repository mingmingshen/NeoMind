# Commands Module

**Package**: `neomind-devices` (command execution in DeviceService)
**Version**: 0.8.0
**Completion**: 80%
**Purpose**: Device command execution and status tracking

## Overview

The Commands functionality is integrated into the DeviceService within the `neomind-devices` crate. It manages device command sending, status tracking, and history persistence. Commands are routed through the appropriate adapter based on the device's configuration.

## Architecture

Commands are not a separate crate. They are handled by:
- `DeviceService` in `neomind-devices/src/service.rs` - command execution and routing
- `DeviceAdapter` trait - protocol-specific command sending
- `neomind-storage` - command history persistence (CommandHistoryRecord)

## Core Types

### 1. CommandHistoryRecord - Command Record

```rust
pub struct CommandHistoryRecord {
    /// Unique command ID
    pub command_id: String,

    /// Device ID
    pub device_id: String,

    /// Command name
    pub command_name: String,

    /// Command parameters
    pub parameters: HashMap<String, serde_json::Value>,

    /// Command status
    pub status: CommandStatus,

    /// Result message (if available)
    pub result: Option<String>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Created timestamp
    pub created_at: i64,

    /// Completed timestamp
    pub completed_at: Option<i64>,
}
```

### 2. CommandStatus - Command Status

```rust
pub enum CommandStatus {
    /// Pending execution
    Pending,
    /// Currently executing
    Executing,
    /// Completed successfully
    Success,
    /// Failed
    Failed,
    /// Timed out
    Timeout,
}
```

### 3. Command Execution Flow

```
API -> DeviceService::send_command()
     -> Build payload from device template
     -> Route to appropriate handler:
        a. Extension device -> ExtensionCommandRouter
        b. MQTT device -> MqttAdapter::send_command()
        c. Other -> DeviceAdapter::send_command()
     -> Record command history
     -> Update status (Success/Failed)
```

## Command Routing

### MQTT Commands

Commands are sent to MQTT devices via the adapter:
1. DeviceService looks up the device's adapter
2. Builds the command payload from the device type template
3. Calls `adapter.send_command()` with the device ID, command name, and payload
4. The adapter publishes to the configured command topic

### Extension Commands

Commands to extension-managed devices use the ExtensionCommandRouter:
1. DeviceService detects the device is extension-managed
2. Routes command through the `ExtensionCommandRouterFn` callback
3. The extension processes the command and returns result

## API Endpoints

```
# Device Commands
POST   /api/devices/:id/command/:command      # Send command to device
GET    /api/devices/:id/commands              # Get command history
```

### Send Command

```bash
# Send command with parameters
curl -X POST http://localhost:9375/api/devices/relay_1/command/turn_on \
  -H "Content-Type: application/json" \
  -d '{}'

# Send command with parameters
curl -X POST http://localhost:9375/api/devices/fan_1/command/set_speed \
  -H "Content-Type: application/json" \
  -d '{"speed": 100, "direction": "clockwise"}'
```

### Get Command History

```bash
curl http://localhost:9375/api/devices/relay_1/commands
```

Response:
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

## Command Status Lifecycle

```
Pending -> Executing -> Success
                    -> Failed
                    -> Timeout
```

## Usage Examples

### Send Command via DeviceService

```rust
use neomind_devices::DeviceService;

let result = service.send_command(
    "greenhouse_fan_1",
    "turn_on",
    HashMap::new(),
).await?;

println!("Command status: {:?}", result.status);
```

### Send Command with Parameters

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

## Cleaned Up Features

The following were removed as part of architecture simplification:
- Separate `neomind-commands` crate (merged into DeviceService)
- CommandQueue with background workers (now synchronous in DeviceService)
- DownlinkAdapter trait (replaced by DeviceAdapter.send_command())
- RetryPolicy/QueueConfig (simplified - commands fail immediately)

## Design Principles

1. **Unified**: Commands are part of DeviceService, not a separate module
2. **Template-Based**: Command payloads are built from device type templates
3. **Adapter-Routed**: Commands automatically use the correct adapter
4. **Extension-Support**: Extension-managed devices route through extension router
5. **History-Tracked**: All commands are recorded with status and timestamps
