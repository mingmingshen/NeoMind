# Modbus Adapter Design

## Overview

Add Modbus TCP and RTU support to NeoMind's device connection layer as a new adapter type, enabling direct connection to industrial sensors, PLCs, energy meters, and other Modbus devices.

## Motivation

- Modbus is the de facto standard protocol in industrial/energy/agriculture IoT (billions of deployed devices)
- Covers temperature sensors, power meters, PLCs, VFDs, and all devices behind Modbus gateways
- Pure software implementation, no hardware dependency for TCP mode
- Rust ecosystem has a mature library (`tokio-modbus`) with unified TCP/RTU async API

## Architecture

### Position in Adapter Layer

```
neomind-devices/adapters/
├── mqtt.rs      (existing)
├── http.rs      (existing)
├── webhook.rs   (existing)
└── modbus.rs    (new)
```

### Core Components

```
ModbusAdapterConfig
  ├── transport: tcp | rtu
  ├── TCP params: host, port
  ├── RTU params: serial_port, baud_rate, data_bits, stop_bits, parity
  └── devices: Vec<ModbusDeviceConfig>
        ├── device_id, name, slave_id, poll_interval
        └── registers: Vec<RegisterMapping>

ModbusAdapter (impl DeviceAdapter)
  ├── connection_pool: shared TCP/serial connections
  ├── polling_tasks: one tokio task per device
  └── event_tx: broadcast::Sender<DeviceEvent>
```

### Data Flow

```
[Modbus Device] <-TCP/RTU-> [ModbusAdapter.poll_loop]
                                ↓ DeviceEvent::Metric
                              [EventPublishingAdapter]
                                ↓ NeoMindEvent::DeviceMetric
                              [EventBus] → TimeSeriesStorage / RuleEngine / Agent
```

## Data Model

### Register Mapping

| Field | Description |
|-------|-------------|
| `metric` | Output metric name (e.g., "temperature") |
| `function` | Modbus function: `coil`, `discrete_input`, `holding_register`, `input_register` |
| `address` | Starting register address (0-based) |
| `data_type` | `uint16`, `int16`, `uint32`, `float32`, `string(N)` |
| `scale` | Optional multiplier (e.g., 0.1 means raw_value * 0.1) |
| `unit` | Optional unit metadata (e.g., "C") |

### Connection Config (TCP)

```json
{
  "name": "factory_modbus",
  "transport": "tcp",
  "host": "192.168.1.100",
  "port": 502,
  "devices": [
    {
      "device_id": "temp-sensor-1",
      "name": "Temperature Sensor",
      "slave_id": 1,
      "poll_interval": 5,
      "registers": [
        {
          "metric": "temperature",
          "function": "input_register",
          "address": 0,
          "data_type": "float32",
          "scale": 0.1,
          "unit": "°C"
        }
      ]
    }
  ]
}
```

### Connection Config (RTU)

```json
{
  "name": "serial_sensors",
  "transport": "rtu",
  "serial_port": "/dev/ttyUSB0",
  "baud_rate": 9600,
  "data_bits": 8,
  "stop_bits": 1,
  "parity": "none",
  "devices": [
    {
      "device_id": "power-meter",
      "name": "Power Meter",
      "slave_id": 3,
      "poll_interval": 10,
      "registers": [
        {
          "metric": "voltage",
          "function": "holding_register",
          "address": 0,
          "data_type": "uint16",
          "scale": 0.1,
          "unit": "V"
        }
      ]
    }
  ]
}
```

## Runtime Behavior

### Startup

1. Parse config, establish TCP connection or open serial port
2. For each device, spawn a `tokio::spawn` poll loop task
3. Emit `DeviceEvent::State { Connected }` for each device

### Poll Loop (per device)

```
loop {
    for each register_mapping in device.registers {
        result = read_registers(slave_id, address, count)
        match result {
            Ok(data) => parse(data) → apply scale → DeviceEvent::Metric
            Err(Timeout) => skip, log warning
            Err(ConnectionLost) => emit State::Disconnected, reconnect with backoff
        }
    }
    tokio::time::sleep(poll_interval)
}
```

### Command Execution

- `send_command()` maps command to register write operation
- Supports: `write_single_coil`, `write_single_register`, `write_multiple_registers`
- Command payload contains: target address + value(s)

### Connection Management

- **TCP**: One TCP connection shared across all slave_ids on same host:port
- **RTU**: One serial port shared across all slave_ids on same bus
- **Reconnection**: Exponential backoff (1s → 2s → 4s → ... → 30s max)
- **Health**: 3 consecutive timeouts triggers Disconnected state event

### Error Handling

| Scenario | Behavior |
|----------|----------|
| Single read timeout | Skip round, log warning |
| 3 consecutive timeouts | Emit `State::Disconnected` |
| Connection lost | Exponential backoff reconnect |
| Reconnect success | Emit `State::Connected`, resume polling |
| Register parse error | Skip that register, others unaffected |
| All devices offline | Adapter stays running, keeps retrying |

## File Changes

### New Files

- `crates/neomind-devices/src/adapters/modbus.rs` (~600 lines)

### Modified Files

- `crates/neomind-devices/Cargo.toml` — add `tokio-modbus`, `tokio-serial`, feature flag
- `crates/neomind-devices/src/adapters/mod.rs` — register modbus module + factory method

### Dependencies

```toml
tokio-modbus = { version = "0.15", optional = true }
tokio-serial = { version = "5.4", optional = true }

[features]
default = ["mqtt", "http", "modbus"]
modbus = ["tokio-modbus", "tokio-serial"]
```

## Out of Scope

- Auto-discovery / device scanning (future iteration)
- Modbus ASCII mode (rarely used)
- Modbus UDP (rare)
- Modbus Security / TLS over Modbus TCP
- Frontend register visual editor
- Batch read optimization (merge consecutive registers)

## Testing

- **Unit tests**: RegisterMapping parsing, data type conversion, scale calculation
- **Integration tests**: Mock Modbus server via `tokio-modbus` server API
- **Manual tests**: TCP mode with Modbus simulator (`modrssim`); RTU requires hardware
