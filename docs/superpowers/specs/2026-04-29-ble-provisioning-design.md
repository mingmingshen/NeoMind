# BLE Provisioning Design — NE101 + NeoMind Platform

**Date**: 2026-04-29
**Status**: Approved
**Scope**: NE101 (ESP32-S3) firmware BLE provisioning + NeoMind platform-side BLE scanner and auto-registration

## Problem

CamThink NE101 devices have no network configuration on first boot. Currently users must connect to the device's WiFi AP hotspot, open a web page, and manually enter WiFi + MQTT settings. This is cumbersome and requires network knowledge.

BLE provisioning allows users to scan for nearby devices via the NeoMind desktop/web app, send WiFi + MQTT configuration over Bluetooth, and have the device automatically connect and appear in the system — no AP mode, no manual web configuration.

## Approach

**Web Bluetooth API** — single codebase for both Tauri desktop and web browser. No Rust BLE library needed.

## BLE GATT Protocol

### Service Definition

```
Service Name:  "NeoMind Provisioning"
Service UUID:  9e5d1e47-5b13-4c4f-85b3-d0e6f5a7b8c9

Characteristic 1: Device Info
  UUID: 9e5d1e48-... | Properties: Read
  Returns: { "model": "NE101", "sn": "NE101A2F003", "fw": "1.0.0" }

Characteristic 2: WiFi Scan
  UUID: 9e5d1e49-... | Properties: Read + Notify
  Trigger scan by subscribing to notifications.
  Returns array: [ { "ssid": "...", "rssi": -45, "auth": true }, ... ]

Characteristic 3: WiFi Config
  UUID: 9e5d1e4a-... | Properties: Write (requires encrypted link)
  Write: { "ssid": "...", "password": "..." }

Characteristic 4: MQTT Config
  UUID: 9e5d1e4b-... | Properties: Write (requires encrypted link)
  Write: { "host": "...", "port": 1883, "username": "", "password": "",
           "topic_prefix": "device/ne101_camera/ne101a2f003" }

Characteristic 5: Status
  UUID: 9e5d1e4c-... | Properties: Read + Notify
  Returns: { "step": "idle" | "wifi_connecting" | "mqtt_connecting" | "done" | "failed", "error": "" }

Characteristic 6: Apply
  UUID: 9e5d1e4d-... | Properties: Write
  Write: { "action": "apply" }
  Triggers device to apply configuration and connect to network.
```

### Security

WiFi Config and MQTT Config characteristics require BLE Secure Connections (pairing with encryption). ESP-IDF supports this natively via `esp_ble_set_encryption()`. This prevents WiFi credentials from being intercepted over the air.

### Provisioning Sequence

```
Platform (Web Bluetooth)           NE101 (BLE GATT Server)
  │                                    │
  ├── 1. Scan service UUID ──────────►│  Advertising "NE101-A2F003"
  ├── 2. Connect + Pair (encrypted) ─►│
  ├── 3. Read Device Info ◄──────────┤  { model, sn, fw }
  ├── 4. Subscribe WiFi Scan ◄───────┤  [ { ssid, rssi, auth }, ... ]
  │   (user selects SSID from list)   │
  ├── 5. Write WiFi Config ─────────►│  { ssid, password }
  ├── 6. Write MQTT Config ─────────►│  { host, port, topic_prefix }
  ├── 7. Write Apply ───────────────►│  { action: "apply" }
  ├── 8. Subscribe Status Notify ◄──┤  wifi_connecting → mqtt_connecting → done
  ├── 9. Disconnect BLE ────────────►│  Device disables BLE, enters normal mode
```

## Device Type Matching

Existing device types in the NeoMind runtime database (`devices.redb`):

| BLE Model | Device Type | Name | Metrics | Commands |
|-----------|------------|------|---------|----------|
| NE301 | `ne301_camera` | CamThink Edge AI Camera | 28 | 2 |
| NE101 | `ne101_camera` | CamThink Sensing Camera | 12 | 0 |

Matching logic: BLE Device Info `model` field → lookup in `MODEL_TO_DEVICE_TYPE` map. If no match, show template picker for user to select. If selected `device_type` does not exist in registry, the API returns 400 with available types.

### Device ID Generation

```
device_id = sn.to_lowercase().replace("-", "_")
```

Examples:
- SN `NE101-A2F003` → device_id `ne101_a2f003`
- SN `NE101-B1C042` → device_id `ne101_b1c042`

Topic prefix derived from device_id:
```
telemetry_topic = "device/{device_type}/{device_id}/uplink"
command_topic   = "device/{device_type}/{device_id}/downlink"
```

Example: `device/ne101_camera/ne101_a2f003/uplink`

## Auto-Registration Flow

### Key Design: Pre-registration

The platform pre-registers the device during BLE provisioning, before the device even connects to WiFi. When the device comes online via MQTT, it's already known — no draft/approval flow needed.

### Provisioning State Tracking

No new `ConnectionStatus` variant is needed. Pre-registered devices use the existing `DeviceConfig.connection_config.extra` field:

```json
{
  "extra": {
    "ble_provisioned": true,
    "provisioned_at": "2026-04-29T10:30:00Z"
  }
}
```

Auto-onboard checks `ble_provisioned` flag to skip draft creation.

### Full Sequence

```
Frontend BLE UI                  NeoMind Backend                 NE101 Device
  │                                  │                              │
  ├── Scan + Connect BLE ──────────────────────────────────────────►│
  ├── Read Device Info ◄────────────────────────────────────────────┤
  │   { model:"NE101", sn:"NE101-A2F003" }                         │
  │                                  │                              │
  │   Match: "NE101" → "ne101_camera" │                              │
  │                                  │                              │
  ├── GET /api/mqtt/status ─────────►│ (fetch embedded broker)       │
  ├── GET /api/brokers ─────────────►│ (fetch external brokers)      │
  │   (user selects broker)          │                              │
  │                                  │                              │
  ├── POST /api/devices/ble-provision ►│                            │
  │   { model, sn, device_type,      │                              │
  │     device_name, broker_id }     │                              │
  │◄── { device_id, mqtt_config } ──┤                              │
  │                                  │                              │
  │   (Device pre-registered,        │                              │
  │    extra.ble_provisioned=true)    │                              │
  │                                  │                              │
  ├── BLE Subscribe WiFi Scan ◄─────────────────────────────────────┤
  │   User selects WiFi from scanned list                           │
  ├── BLE Write WiFi Config ────────────────────────────────────────►│
  ├── BLE Write MQTT Config ────────────────────────────────────────►│
  │   { host, port, username,        │                              │
  │     password, topic_prefix }      │                              │
  ├── BLE Write Apply ──────────────────────────────────────────────►│
  │                                  │                              │
  ├── BLE Status: done ◄────────────────────────────────────────────┤
  │                                  │                              │
  │                                  │◄── MQTT telemetry ──────────┤
  │                                  │   topic matches pre-registered
  │                                  │   DeviceMetric handler in
  │                                  │   service.rs recognizes device
  │                                  │   status → Connected
  │◄── WebSocket: DeviceOnline ─────┤                              │
  │                                  │                              │
  │   "NE101 已上线"                  │                              │
```

### Integration Point: MQTT Message Handling

When the pre-registered device sends its first MQTT telemetry on `device/ne101_camera/ne101_a2f003/uplink`, the flow in `service.rs` (lines 378-400) naturally handles it: the `DeviceMetric` event matches the already-registered device_id and transitions status from Disconnected to Connected, then publishes `DeviceOnline`. The change to `auto_onboard.rs` is: in the draft-creation path, check if the device_id already exists in the registry (via `DeviceRegistry::get_device()`). If it exists with `ble_provisioned=true` in extra, skip draft creation entirely.

### Error Recovery

| Scenario | Handling |
|----------|----------|
| BLE write fails after pre-registration | Frontend calls `DELETE /api/devices/{device_id}` to clean up |
| Device WiFi connection fails | BLE Status returns `{ step: "failed", error: "wifi_timeout" }`. Frontend shows retry button |
| Device MQTT connection fails | BLE Status returns `{ step: "failed", error: "mqtt_refused" }`. Frontend shows retry |
| User cancels mid-provisioning | Frontend calls cleanup API, disconnects BLE |
| Device never comes online (BLE succeeded but MQTT never arrives) | Scheduled task cleans up devices where `ble_provisioned=true` and status is still Disconnected after 1 hour. `DELETE /api/devices/{device_id}` |

## Implementation Plan

### Module Changes

#### 1. NE101 Firmware (Fork of camthink-ai/lowpower_camera)

New files in `main/`:

- `ble_prov.h` — Public interface
- `ble_prov.c` — BLE GATT Server implementation

```c
// ble_prov.h
void ble_prov_init(void);       // Init BLE stack + register GATT service
void ble_prov_start(void);      // Start advertising
void ble_prov_stop(void);       // Stop advertising (after provisioning complete)
bool ble_prov_is_active(void);  // Check if BLE provisioning is active
```

Integration points with existing code:
- `config.c` NVS functions: `cfg_set_wifi_attr()` / `cfg_set_mqtt_attr()` to persist config
- `system.c`: Trigger restart into work mode after provisioning
- `main.c` `mode_selector()`: Add BLE provisioning mode for first boot (no WiFi config in NVS)

BLE provisioning triggered when:
- First boot (no WiFi credentials in NVS)
- User button press (via deep sleep wakeup path)

Uses ESP-IDF BLE GATT API (ESP32-S3 native support). WiFi scan uses `esp_wifi_scan_start()` while in BLE+STA coex mode.

#### 2. NeoMind Backend API

New file: `crates/neomind-api/src/handlers/devices/ble_provision.rs`

```
POST /api/devices/ble-provision
  Request: {
    "model": "NE101",
    "sn": "NE101-A2F003",
    "device_type": "ne101_camera",
    "device_name": "门口摄像头",
    "broker_id": "embedded"              // "embedded" for built-in, or external broker ID
  }
  Response: {
    "device_id": "ne101_a2f003",
    "mqtt_config": {
      "host": "192.168.1.100",
      "port": 1883,
      "username": "",                    // from broker config (empty for embedded)
      "password": "",                    // from broker config (empty for embedded)
      "topic_prefix": "device/ne101_camera/ne101_a2f003"
    }
  }
```

Broker selection:
- Frontend fetches available brokers via `GET /api/mqtt/settings` (embedded broker) and `GET /api/brokers` (external brokers)
- User selects a broker in the provisioning dialog (dropdown)
- `broker_id: "embedded"` → reads EmbeddedBrokerConfig (host/port from server_ip:1883, no auth)
- `broker_id: "<uuid>"` → reads ExternalBroker (host/port/username/password/tls from settings store)
- Topic auto-generated: `device/{device_type}/{device_id}/uplink` and `downlink`

Logic:
1. Validate `device_type` exists in `DeviceRegistry::get_template()`. Return 400 with available types if not found.
2. Generate `device_id` = `sn.to_lowercase().replace("-", "_")`
3. Check device_id not already registered. Return 409 if exists.
4. Resolve broker config from `broker_id`: load embedded or external broker settings.
5. Build `DeviceConfig` with MQTT connection config from resolved broker. Set `extra.ble_provisioned = true`.
6. Call `DeviceService::register_device()` to pre-register.
7. Return MQTT config for BLE write (host, port, username, password, topic_prefix).

Cleanup endpoint: `DELETE /api/devices/{device_id}` — existing endpoint, usable for failed provisioning cleanup.

#### 3. NeoMind Auto-Onboard Change

File: `crates/neomind-api/src/handlers/devices/auto_onboard.rs`

Change: In the draft-creation path, check if device_id already exists in registry:
- If device exists with `extra.ble_provisioned=true` → skip draft creation (device will be handled by service.rs DeviceMetric handler)
- If device doesn't exist → existing draft flow unchanged

#### 4. NeoMind Frontend

**New global dialog: `web/src/pages/devices/AddDeviceGlobalDialog.tsx`**

Replaces the current `AddDeviceDialog`. Integrates all device-adding methods in a single full-screen or large dialog:

```
┌─────────────────────────────────────────────┐
│  添加设备                                    │
├────────┬──────────┬──────────┬──────────────┤
│ 蓝牙配网 │ 手动添加  │ 自动发现  │              │
├────────┴──────────┴──────────┴──────────────┤
│                                             │
│  [Tab content area]                          │
│                                             │
│  Bluetooth Tab:                              │
│  ┌───────────────────────────────────────┐  │
│  │ 扫描 BLE 设备...                      │  │
│  │                                       │  │
│  │ NE101-A2F003  ████████░░  -45dBm      │  │
│  │ NE101-B1C042  ██████░░░░  -62dBm      │  │
│  └───────────────────────────────────────┘  │
│  MQTT Broker: [NeoMind 内置 ▾]                │
│  WiFi: [扫描结果下拉选择 ▾]                    │
│  密码: [____________]                        │
│  设备名称: [NE101-A2F003 ▾]                   │
│  设备类型: CamThink Sensing Camera (自动匹配)  │
│                                             │
│  [ 开始配网 ]                                │
│                                             │
└─────────────────────────────────────────────┘
```

Tabs:
- **蓝牙配网** — BLE scan + WiFi/MQTT provisioning (new)
- **手动添加** — existing MQTT/HTTP/Webhook manual config (from AddDeviceDialog)
- **自动发现** — pending devices from auto-onboard (from PendingDevicesList)

This replaces the current entry point. The "添加设备" button on the devices page opens `AddDeviceGlobalDialog` instead of `AddDeviceDialog`.

BLE state managed with React hooks (no Zustand slice needed — BLE is session-scoped, not persistent).

Broker selector fetches from:
- `GET /api/mqtt/status` → embedded broker (server_ip + listen_port)
- `GET /api/brokers` → external brokers list
- Dropdown shows: "NeoMind 内置 (192.168.1.100:1883)" + any external brokers with name/host/port
- Selected broker_id sent to `POST /api/devices/ble-provision`

### Device Model Mapping

```typescript
const MODEL_TO_DEVICE_TYPE: Record<string, string> = {
  "NE101": "ne101_camera",
  "NE301": "ne301_camera",
};
```

Extendable for future CamThink devices.

## Constraints

- Web Bluetooth API only works in Chrome/Edge/Chromium WebView (covers Tauri desktop)
- BLE range limited to ~10 meters
- One device provisioned at a time (sequential)
- ESP32-S3 BLE and WiFi share the same radio — BLE must be stopped before WiFi connects
- WiFi password transmitted over encrypted BLE link (pairing required)
- User must manually type WiFi password (SSID comes from device-side scan)

## Out of Scope

- NE301 firmware BLE support (future phase — SiWx917 BLE requires Silicon Labs SDK)
- Bulk provisioning of multiple devices
- BLE OTA firmware updates
- BLE-based device diagnostics
