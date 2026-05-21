---
id: device-management
name: Device Management CLI Commands
category: device
origin: builtin
priority: 80
token_budget: 10000
triggers:
  keywords: [device, 设备, 设备列表, 设备管理, device list, list device, telemetry, 遥测, 设备状态, device status, device control, 控制设备, device create, 创建设备, device type, 设备类型, metric, 指标, 电量, battery, 温度, temperature, history, 历史数据, write-metric, 写入指标, device update, 更新设备, device delete, 删除设备, adapter, 适配器, MQTT]
  tool_target:
    - tool: device
      actions: [list, get, latest, history, control, create, update, delete, write-metric, types]
anti_triggers:
  keywords: [rule, 规则, agent, 代理, message, 消息]
---

# Device Management CLI Commands

All device commands are executed via the `shell` tool using `neomind device <action>`.

## ⚠️ CRITICAL: Common Operations Checklist

| User Request | Command |
|---|---|
| 列出设备 | `neomind device list --json` |
| 设备详情 | `neomind device get <ID>` |
| 最新指标 | `neomind device latest <ID>` |
| 历史数据 | `neomind device history <ID> --metric <M> --time-range 24h` |
| 创建设备 | `neomind device create --name 'NAME' --device-type 'TYPE'` |
| 控制设备 | `neomind device control <ID> --command <CMD> --params '{"state": true}'` |
| 写入指标 | `neomind device write-metric <ID> --metric <M> --value <V>` |
| 删除设备 | `neomind device delete <ID>` |
| 更新设备 | `neomind device update <ID> --name 'NEW_NAME'` |

> **Note**: Both `--device-type` and `--type` work. Use `device latest <ID>` to discover real metric names before creating dashboards/rules.

---

## Command Reference

### List Devices

```bash
neomind device list
neomind device list --device-type temperature-sensor
neomind device list --status online
neomind device list --json
```

### Get Device Details

```bash
neomind device get <ID>
```

### Create Device

```bash
neomind device create --name '<name>' --device-type '<device_type>' [--adapter-type '<adapter>'] [--config '<json>']
```

```bash
neomind device create --name 'Sensor-01' --device-type 'temperature-sensor'
neomind device create --name 'LivingRoom' --device-type 'temperature-sensor' --adapter-type mqtt --config '{"topic":"sensors/livingroom/temp"}'
```

### Update Device

```bash
neomind device update <ID> --name '<new_name>'
neomind device update <ID> --config '{"topic":"sensors/new"}'
```

### Delete Device

```bash
neomind device delete <ID>
```

### Get Latest Metrics

```bash
neomind device latest <ID>
```

Returns all current metric values. **Use this to discover available metric names** for dashboards and history queries.

### Get Telemetry History

```bash
neomind device history <ID> --metric <name> [--time-range <range>] [--compress] [--json]
```

**Time range formats**: `30s`, `5m`, `1h`, `24h`, `7d`, `1mo`

```bash
neomind device history sensor-001 --metric temperature --time-range 24h
neomind device history sensor-001 --metric battery --time-range 7d --compress
```

### Send Control Command

```bash
neomind device control <ID> --command <cmd> [--params '<json>']
```

```bash
neomind device control switch-001 --command switch --params '{"state": true}'
```

### Write Metric Data Point

```bash
neomind device write-metric <ID> --metric <name> --value <value> [--timestamp <unix_ts>]
```

```bash
neomind device write-metric sensor-001 --metric temperature --value 23.5
neomind device write-metric sensor-001 --metric alarm --value true
```

### Device Types

```bash
neomind device types list
neomind device types get <TYPE_ID>
neomind device types create --name '<name>' --metrics '[{"name":"temperature","unit":"°C","type":"number"}]'
```

Multi-metric:
```bash
neomind device types create --name 'weather-station' --metrics '[{"name":"temperature","unit":"°C","type":"number"},{"name":"humidity","unit":"%","type":"number"}]'
```

---

## Important Notes

- **Both `--device-type`/`--type` and `--adapter-type`/`--adapter` work** — either flag name is accepted
- **Metric names must match exactly** — use `device latest <ID>` to discover available metrics
- **Value auto-detection**: numbers (23.5), booleans (true/false), strings (fallback)
- **MQTT is default adapter** — omit `--adapter` for MQTT devices
- **Device types are schemas** — devices must reference existing device type
- **No device type update** — delete and recreate to modify
- **List limit**: 100 results; use filters for large fleets

## Common Errors & Solutions

- **"Device not found"**: Run `neomind device list --json` to list all valid device IDs. Use the exact ID from the output.
- **Create fails with missing fields**: Both `--name` and `--device-type` are required. Use `neomind device types list` to see valid device types.
- **Control command fails**: Not all device types support every command. Run `neomind device get <ID>` to check the device type and adapter, then verify the command is supported.
- **No telemetry data returned**: The device may be offline. Run `neomind device list --status` to check connectivity. Offline devices have no recent metrics.
- **Write metric fails**: The metric name must match a field defined in the device type schema. Use `neomind device latest <ID>` to see which metric names actually exist for that device.
- **Flag aliases**: Both `--type`/`--device-type` and `--adapter`/`--adapter-type` are accepted. Either works.
