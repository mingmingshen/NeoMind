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
Service UUID:  0xFEA0

Characteristic 1: Device Info
  UUID: FEA1 | Properties: Read
  Returns: { "model": "NE101", "sn": "NE101-XXXXXX", "fw": "1.0.0" }

Characteristic 2: WiFi Config
  UUID: FEA2 | Properties: Write
  Write: { "ssid": "...", "password": "..." }

Characteristic 3: MQTT Config
  UUID: FEA3 | Properties: Write
  Write: { "host": "...", "port": 1883, "user": "", "password": "", "topic_prefix": "device/ne101/xxx" }

Characteristic 4: Status
  UUID: FEA4 | Properties: Read + Notify
  Returns: { "step": "idle" | "wifi_connecting" | "mqtt_connecting" | "done" | "failed", "error": "" }

Characteristic 5: Apply
  UUID: FEA5 | Properties: Write
  Write: { "action": "apply" }
  Triggers device to apply configuration and connect to network.
```

### Provisioning Sequence

```
Platform (Web Bluetooth)           NE101 (BLE GATT Server)
  │                                    │
  ├── 1. Scan FEA0 Service ──────────►│  Advertising "NE101-XXXXXX"
  ├── 2. Connect BLE ────────────────►│
  ├── 3. Read Device Info ◄──────────┤  { model, sn, fw }
  ├── 4. Write WiFi Config ─────────►│  { ssid, password }
  ├── 5. Write MQTT Config ─────────►│  { host, port, topic_prefix }
  ├── 6. Write Apply ───────────────►│  { action: "apply" }
  ├── 7. Subscribe Status Notify ◄──┤  wifi_connecting → mqtt_connecting → done
  ├── 8. Disconnect BLE ────────────►│  Device disables BLE, enters normal mode
```

## Device Type Matching

Existing device types in the NeoMind runtime database (`devices.redb`):

| BLE Model | Device Type | Name | Metrics | Commands |
|-----------|------------|------|---------|----------|
| NE301 | `ne301_camera` | CamThink Edge AI Camera | 28 | 2 |
| NE101 | `ne101_camera` | CamThink Sensing Camera | 12 | 0 |

Matching logic: BLE Device Info `model` field → lowercase → look up in existing templates. If no match, fall back to user selection from template list.

## Auto-Registration Flow

### Key Design: Pre-registration

The platform pre-registers the device during BLE provisioning, before the device even connects to WiFi. When the device comes online via MQTT, it's already known — no draft/approval flow needed.

### Full Sequence

```
Frontend BLE UI                  NeoMind Backend                 NE101 Device
  │                                  │                              │
  ├── Scan + Connect BLE ──────────────────────────────────────────►│
  ├── Read Device Info ◄────────────────────────────────────────────┤
  │   { model:"NE101", sn:"xxx" }   │                              │
  │                                  │                              │
  ├── GET /api/devices/types ──────►│                              │
  │◄── templates list ──────────────┤                              │
  │                                  │                              │
  │   Match model→device_type:       │                              │
  │   "NE101" → "ne101_camera"       │                              │
  │                                  │                              │
  ├── POST /api/devices/ble-provision ►│                            │
  │   { model, sn, device_type,      │                              │
  │     device_name }                │                              │
  │◄── { device_id, mqtt_config } ──┤                              │
  │                                  │                              │
  │   (Device pre-registered,        │                              │
  │    status: provisioning)          │                              │
  │                                  │                              │
  ├── BLE Write WiFi Config ────────────────────────────────────────►│
  ├── BLE Write MQTT Config ────────────────────────────────────────►│
  │   (using mqtt_config from backend)                              │
  ├── BLE Write Apply ──────────────────────────────────────────────►│
  │                                  │                              │
  ├── BLE Status: done ◄────────────────────────────────────────────┤
  │                                  │                              │
  │                                  │◄── MQTT telemetry ──────────┤
  │                                  │   matches pre-registered id
  │                                  │   status → connected
  │◄── WebSocket: DeviceOnline ─────┤                              │
  │                                  │                              │
  │   "NE101 已上线"                  │                              │
```

## Implementation Plan

### Module Changes

#### 1. NE101 Firmware (Fork of camthink-ai/lowpower_camera)

New files in `main/`:

- `ble_prov.h` — Public interface
- `ble_prov.c` — BLE GATT Server implementation

```c
// ble_prov.h
void ble_prov_init(void);    // Init BLE stack + register GATT service
void ble_prov_start(void);   // Start advertising
void ble_prov_stop(void);    // Stop advertising (after provisioning complete)
bool ble_prov_is_active(void); // Check if BLE provisioning is active
```

Integration points with existing code:
- `config.c` NVS functions: `cfg_set_wifi_attr()` / `cfg_set_mqtt_attr()` to persist config
- `system.c`: Trigger restart into work mode after provisioning
- `main.c` `mode_selector()`: Add BLE provisioning mode for first boot (no WiFi config in NVS)

BLE provisioning triggered when:
- First boot (no WiFi credentials in NVS)
- User button press (via deep sleep wakeup path)

Uses ESP-IDF BLE GATT API (ESP32-S3 native support).

#### 2. NeoMind Backend API

New file: `crates/neomind-api/src/handlers/devices/ble_provision.rs`

```
POST /api/devices/ble-provision
  Request: {
    "model": "NE101",
    "sn": "NE101-XXXXXX",
    "device_type": "ne101_camera",
    "device_name": "门口摄像头"
  }
  Response: {
    "device_id": "ne101_xxx",
    "mqtt_config": {
      "host": "192.168.1.100",
      "port": 1883,
      "topic_prefix": "device/ne101/ne101_xxx"
    }
  }
```

Logic:
1. Validate `device_type` exists in `DeviceRegistry::get_template()`
2. Generate `device_id` from SN or auto-generate
3. Build `DeviceConfig` with MQTT connection config using EmbeddedBroker settings
4. Call `DeviceService::register_device()` to pre-register
5. Return MQTT config for BLE write

#### 3. NeoMind Auto-Onboard Change

File: `crates/neomind-api/src/handlers/devices/auto_onboard.rs`

Change: Before creating a draft device, check if device already exists in registry:
- If device exists with status "provisioning" → update to "connected", publish DeviceOnline event, skip draft
- If device doesn't exist → existing draft flow unchanged

#### 4. NeoMind Frontend

New file: `web/src/pages/devices/BleProvisionDialog.tsx`

Features:
- BLE device scanning via Web Bluetooth API
- Device list with signal strength
- Auto-read Device Info on selection
- Auto-match device type (model → device_type)
- WiFi SSID/password input form
- MQTT config auto-filled from backend
- Progress display: scanning → connected → configuring → done
- Success/error states

Modified file: `web/src/pages/devices/AddDeviceDialog.tsx`

Change: Add "蓝牙配网" tab alongside existing MQTT/HTTP/Webhook options.

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

## Out of Scope

- NE301 firmware BLE support (future phase — SiWx917 BLE requires Silicon Labs SDK)
- Bulk provisioning of multiple devices
- BLE OTA firmware updates
- BLE-based device diagnostics
