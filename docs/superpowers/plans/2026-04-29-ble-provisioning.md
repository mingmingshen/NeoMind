# BLE Provisioning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement BLE provisioning for NE101 devices — scan, configure network + MQTT, auto-register in NeoMind platform.

**Architecture:** NE101 firmware adds BLE GATT Server with 5 characteristics. NeoMind backend adds a pre-registration API. NeoMind frontend uses Web Bluetooth API to orchestrate the flow. Device type matching and auto-registration skip the draft/approval flow.

**Tech Stack:** ESP-IDF (C, ESP32-S3 BLE GATT), Rust/Axum (backend API), React/TypeScript + Web Bluetooth API (frontend)

**Spec:** `docs/superpowers/specs/2026-04-29-ble-provisioning-design.md`

---

## File Structure

### NE101 Firmware (fork of camthink-ai/lowpower_camera)

```
main/
  ble_prov.h              # Create — public BLE provisioning interface
  ble_prov.c              # Create — BLE GATT Server implementation
  main.c                  # Modify — add BLE provisioning mode to mode_selector()
```

### NeoMind Backend (this repo)

```
crates/neomind-api/src/
  handlers/devices/
    ble_provision.rs      # Create — BLE pre-registration API handler
    mod.rs                # Modify — register new handler module + re-export
  server/
    router.rs             # Modify — add /api/devices/ble-provision to protected routes
```

### NeoMind Frontend (this repo)

```
web/src/
  pages/devices/
    AddDeviceGlobalDialog.tsx   # Create — global dialog with BLE/Manual/Auto tabs
    BleProvisionTab.tsx         # Create — BLE provisioning tab content
    AddDeviceDialog.tsx         # Keep — reused inside GlobalDialog's Manual tab
  pages/
    devices.tsx                 # Modify — open GlobalDialog instead of AddDeviceDialog
  hooks/
    useBleProvision.ts          # Create — Web Bluetooth API hook
  lib/
    ble-protocol.ts             # Create — UUID constants + type definitions
docs/
  guides/en/ble-provisioning.md  # Create — standalone protocol document for device makers
  guides/zh/ble-provisioning.md  # Create — Chinese version
```

---

## Part 1: NE101 Firmware — BLE GATT Server

> These tasks are in the forked `camthink-ai/lowpower_camera` repository.
> ESP-IDF C development — no automated TDD, test on hardware.

### Task 1: BLE GATT Service Skeleton

**Files:**
- Create: `main/ble_prov.h`
- Create: `main/ble_prov.c`

- [ ] **Step 1: Create ble_prov.h with public interface and UUID definitions**

```c
#ifndef __BLE_PROV_H__
#define __BLE_PROV_H__

#include <stdbool.h>

// Service UUID: 9e5d1e47-5b13-4c4f-85b3-d0e6f5a7b8c9
// Characteristic UUIDs: 9e5d1e4x-5b13-4c4f-85b3-d0e6f5a7b8c9

void ble_prov_init(void);       // Init BLE stack + register GATT service
void ble_prov_start(void);      // Start advertising
void ble_prov_stop(void);       // Stop advertising
bool ble_prov_is_active(void);  // Check if BLE provisioning is active

#endif
```

- [ ] **Step 2: Create ble_prov.c with GATT service table and init**

Implement:
- `esp_ble_gattc_app_register()` to register the GATT app
- GATT service table with 5 characteristics (Device Info, Network Scan, Config, Status, Apply)
- All UUIDs as 128-bit: base `9e5d1e47-5b13-4c4f-85b3-d0e6f5a7b8c9`, replace first 2 bytes for each char
- `ble_prov_init()`: init NimBLE stack, register service
- `ble_prov_start()`: configure advertising data as `"NE101-XXXXXX"` (from device SN), start advertising
- `ble_prov_stop()`: stop advertising, deinit BLE

BLE Secure Connections: set `esp_ble_set_encryption()` in connection callback to require pairing.

- [ ] **Step 3: Build and verify GATT service appears in BLE scanner**

Build: `cd lowpower_camera && idf.py build`
Flash: `idf.py -p /dev/ttyUSB0 flash monitor`
Verify: Use `nRF Connect` or `BLE Scanner` phone app to find `"NE101-XXXXXX"` device, see 5 characteristics.

- [ ] **Step 4: Commit**

```bash
git add main/ble_prov.h main/ble_prov.c
git commit -m "feat(ble): add BLE GATT provisioning service skeleton"
```

### Task 2: Device Info + Network Scan Characteristics

**Files:**
- Modify: `main/ble_prov.c`

- [ ] **Step 1: Implement Device Info characteristic read handler**

Read handler returns JSON:
```c
{
  "model": "NE101",
  "sn": "NE101A2F003",
  "fw": "1.0.0",
  "netmod": "",
  "supported_netmods": ["wifi"]
}
```

Read from existing NVS functions:
- `cfg_get_device_info()` for model/sn/fw
- `netModule_check()` to detect hardware → populate `supported_netmods`

- [ ] **Step 2: Implement Network Scan characteristic**

Write handler: parse `{"type":"wifi"|"halow"|"cat1_status"}`, trigger scan.

For WiFi/HaLow:
- Call `wifi_get_list()` (existing function in `wifi.c`)
- Format results as JSON array, send via `esp_ble_gatts_send_indicate()` (Notify)

For CAT.1:
- Call `cat1_get_cellular_status()` (existing function in `cat1.c`)
- Return SIM status, signal, IMEI, ICCID, ISP

- [ ] **Step 3: Build and test scan via BLE**

Verify: Write `{"type":"wifi"}` to Network Scan char → receive Notify with scan results.

- [ ] **Step 4: Commit**

```bash
git add main/ble_prov.c
git commit -m "feat(ble): implement Device Info and Network Scan characteristics"
```

### Task 3: Config Characteristic (Network + MQTT)

**Files:**
- Modify: `main/ble_prov.c`

- [ ] **Step 1: Implement Config characteristic write handler**

Parse incoming JSON, dispatch by `type` field:

```c
static void handle_config_write(const char *json_str, size_t len) {
    cJSON *json = cJSON_Parse(json_str);
    const char *type = cJSON_GetStringValue(cJSON_GetObjectItem(json, "type"));

    if (strcmp(type, "net_wifi") == 0) {
        // Parse ssid, password → call cfg_set_wifi_attr()
        // Call set_netmod("wifi")
    } else if (strcmp(type, "net_cat1") == 0) {
        // Parse apn, user, password, pin, auth_type → call cfg_set_cellular_param_attr()
        // Call set_netmod("cat1")
    } else if (strcmp(type, "net_halow") == 0) {
        // Parse ssid, password → call cfg_set_wifi_attr()
        // Call set_netmod("halow")
    } else if (strcmp(type, "mqtt") == 0) {
        // Parse host, port, username, password, topic_prefix → call cfg_set_mqtt_attr()
    }
    // Unknown types: silently ignored (forward compatibility)
}
```

All config writes go to NVS via existing `config.c` functions — same path as HTTP API.

- [ ] **Step 2: Implement Status characteristic**

Maintain a state machine. After Apply, transition:
```
idle → net_connecting → net_connected → mqtt_connecting → done
                                                   ↘ failed (with error)
```

Send Notify on each transition via `esp_ble_gatts_send_indicate()`.

On `net_connected`: read IP from `esp_netif_get_ip_info()`, include in Notify.
On `failed`: include error string (`wifi_timeout`, `cat1_no_sim`, `mqtt_refused`).

- [ ] **Step 3: Implement Apply characteristic**

Write handler for `{"action":"apply"}`:
1. Commit all pending NVS config
2. Send Status Notify: `net_connecting`
3. Initiate network connection (WiFi/CAT.1/HaLow based on `netmod`)
4. Wait for network connected event → send `net_connected` Notify
5. Initiate MQTT connection
6. Wait for MQTT connected → send `mqtt_connecting` then `done`
7. Stop BLE (`ble_prov_stop()`), enter normal work mode

On any failure → send `failed` Notify with error details.

- [ ] **Step 4: Build and full flow test on hardware**

Build, flash, test full provisioning flow with `nRF Connect` app:
1. Connect BLE
2. Read Device Info
3. Write Network Scan → get WiFi list
4. Write Config `net_wifi`
5. Write Config `mqtt`
6. Write Apply
7. Observe Status Notify sequence → `done`
8. Verify device connects to WiFi + MQTT

- [ ] **Step 5: Commit**

```bash
git add main/ble_prov.c
git commit -m "feat(ble): implement Config, Status, and Apply characteristics"
```

### Task 4: Integrate BLE Provisioning into main.c

**Files:**
- Modify: `main/main.c`

- [ ] **Step 1: Add BLE provisioning mode to mode_selector()**

In `mode_selector()`, add check: if no network config exists in NVS (no `wifi:ssid` AND no `cat1:apn`) AND not a deep sleep wakeup for work → enter BLE provisioning mode.

```c
// In mode_selector(), before existing checks:
if (rst == RST_POWER_ON || rst == RST_SOFTWARE) {
    // Check if any network config exists
    char netmod[MAX_LEN_8] = {0};
    cfg_get_str(KEY_DEVICE_NETMOD, netmod, sizeof(netmod), "");
    if (strlen(netmod) == 0) {
        ESP_LOGI(TAG, "No network config, entering BLE provisioning");
        ble_prov_init();
        ble_prov_start();
        return MODE_CONFIG;  // or a new MODE_BLE_PROV
    }
}
```

- [ ] **Step 2: Handle deep sleep wakeup for BLE provisioning**

Add `WAKEUP_TODO_BLE_PROV` to `wakeupTodo_e` in `system.h`. In sleep wakeup handler, if button held for 5+ seconds → enter BLE provisioning mode.

- [ ] **Step 3: Build, flash, test cold-boot BLE provisioning**

Power on device with no config → verify BLE advertising starts automatically.

- [ ] **Step 4: Commit**

```bash
git add main/main.c main/system.h
git commit -m "feat(ble): integrate BLE provisioning into boot flow"
```

---

## Part 2: NeoMind Backend — BLE Provision API

### Task 5: BLE Provision Handler

**Files:**
- Create: `crates/neomind-api/src/handlers/devices/ble_provision.rs`
- Modify: `crates/neomind-api/src/handlers/devices/mod.rs`
- Modify: `crates/neomind-api/src/server/router.rs`

- [ ] **Step 1: Write failing test for ble_provision handler**

Create test in a new test module or inline. Test:
- Device type validation (400 if type not found)
- Device ID generation from SN
- Duplicate device detection (409)
- Pre-registration with `ble_provisioned=true` in extra

- [ ] **Step 2: Create ble_provision.rs with request/response types**

```rust
// crates/neomind-api/src/handlers/devices/ble_provision.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct BleProvisionRequest {
    pub model: String,
    pub sn: String,
    pub device_type: String,
    pub device_name: String,
    #[serde(default = "default_broker_id")]
    pub broker_id: String,
}

fn default_broker_id() -> String {
    "embedded".to_string()
}

#[derive(Debug, Serialize)]
pub struct BleProvisionResponse {
    pub device_id: String,
    pub mqtt_config: MqttConfigResponse,
}

#[derive(Debug, Serialize)]
pub struct MqttConfigResponse {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub topic_prefix: String,
}
```

- [ ] **Step 3: Implement handler logic**

```rust
pub async fn ble_provision_handler(
    State(state): State<ServerState>,
    Json(req): Json<BleProvisionRequest>,
) -> HandlerResult<BleProvisionResponse> {
    // 1. Validate device_type exists
    let template = state.devices.service.get_template(&req.device_type)
        .ok_or_else(|| ErrorResponse::bad_request(
            format!("Device type '{}' not found", req.device_type)
        ))?;

    // 2. Generate device_id from SN
    let device_id = req.sn.to_lowercase().replace("-", "_");

    // 3. Check not already registered
    if state.devices.service.get_device(&device_id).is_some() {
        return Err(ErrorResponse::conflict(format!("Device {} already registered", device_id)));
    }

    // 4. Resolve broker config
    let (host, port, username, password) = resolve_broker_config(&req.broker_id)?;

    // 5. Build topic prefix
    let topic_prefix = format!("device/{}/{}", req.device_type, device_id);

    // 6. Build DeviceConfig with ble_provisioned flag
    let mut extra = HashMap::new();
    extra.insert("ble_provisioned".to_string(), serde_json::Value::Bool(true));
    extra.insert("provisioned_at".to_string(), serde_json::Value::String(
        chrono::Utc::now().to_rfc3339()
    ));

    let config = DeviceConfig {
        device_id: device_id.clone(),
        name: req.device_name,
        device_type: req.device_type,
        adapter_type: "mqtt".to_string(),
        connection_config: ConnectionConfig {
            telemetry_topic: Some(format!("{}/uplink", topic_prefix)),
            command_topic: Some(format!("{}/downlink", topic_prefix)),
            json_path: None,
            entity_id: None,
            extra,
        },
        adapter_id: None,
    };

    // 7. Register
    state.devices.service.register_device(config).await?;

    // 8. Return MQTT config for BLE write
    ok(BleProvisionResponse {
        device_id,
        mqtt_config: MqttConfigResponse {
            host,
            port,
            username,
            password,
            topic_prefix,
        },
    })
}
```

`resolve_broker_config()` helper:
- `broker_id == "embedded"` → read server IP from embedded broker status + port 1883 (no auth)
- Otherwise → load from `config::open_settings_store().load_external_broker(&broker_id)`, return host/port/username/password from external broker config

- [ ] **Step 4: Register module and route**

In `crates/neomind-api/src/handlers/devices/mod.rs`:
```rust
pub mod ble_provision;
pub use ble_provision::*;
```

In `crates/neomind-api/src/server/router.rs`, add to protected device routes (around line 327):
```rust
.route("/api/devices/ble-provision", post(devices::ble_provision_handler))
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p neomind-api ble_provision
```

- [ ] **Step 6: Start server and test with curl**

```bash
cargo run -p neomind-cli -- serve
# In another terminal:
curl -X POST http://localhost:9375/api/devices/ble-provision \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{"model":"NE101","sn":"NE101-A2F003","device_type":"ne101_camera","device_name":"Test Camera","broker_id":"embedded"}'
```

Expected: 200 with device_id and mqtt_config.

- [ ] **Step 7: Commit**

```bash
git add crates/neomind-api/src/handlers/devices/ble_provision.rs \
        crates/neomind-api/src/handlers/devices/mod.rs \
        crates/neomind-api/src/server/router.rs
git commit -m "feat(api): add BLE provision endpoint for device pre-registration"
```

### Task 6: Verify Auto-Onboard Skip (no code change needed)

> **No code change required.** The auto-onboard path in `crates/neomind-api/src/server/types.rs:1662-1669` already checks:
> ```rust
> if device_service_clone.get_device(device_id).is_some() {
>     continue; // skip auto-onboarding
> }
> ```
> Since Task 5 pre-registers the device via `register_device()`, the device already exists when the first MQTT telemetry arrives. The auto-onboard path naturally skips it.

- [ ] **Step 1: Write a test to verify the skip behavior**

Test: pre-register a device via BLE provision API, then send an MQTT message matching the device topic. Verify no draft is created and device status transitions to Connected.

- [ ] **Step 2: Run test**

```bash
cargo test -p neomind-api auto_onboard
```

- [ ] **Step 3: Commit if test file was created**

```bash
git commit -m "test: verify auto-onboard skips BLE pre-registered devices"
```

---

## Part 3: NeoMind Frontend — BLE Provisioning UI

### Task 7: BLE Protocol Constants and Types

**Files:**
- Create: `web/src/lib/ble-protocol.ts`

- [ ] **Step 1: Create BLE protocol constants**

```typescript
// web/src/lib/ble-protocol.ts

// Service UUID
export const BLE_SERVICE_UUID = '9e5d1e47-5b13-4c4f-85b3-d0e6f5a7b8c9'

// Characteristic UUIDs (replace first 2 bytes of service UUID)
export const BLE_CHAR_DEVICE_INFO  = '9e5d1e48-5b13-4c4f-85b3-d0e6f5a7b8c9'
export const BLE_CHAR_NETWORK_SCAN = '9e5d1e49-5b13-4c4f-85b3-d0e6f5a7b8c9'
export const BLE_CHAR_CONFIG       = '9e5d1e4a-5b13-4c4f-85b3-d0e6f5a7b8c9'
export const BLE_CHAR_STATUS       = '9e5d1e4c-5b13-4c4f-85b3-d0e6f5a7b8c9'
export const BLE_CHAR_APPLY        = '9e5d1e4d-5b13-4c4f-85b3-d0e6f5a7b8c9'

// Config types
export type BleConfigType = 'net_wifi' | 'net_cat1' | 'net_halow' | 'mqtt'

// Scan types
export type BleScanType = 'wifi' | 'halow' | 'cat1_status'

// Status steps
export type BleStatusStep = 'idle' | 'net_connecting' | 'net_connected' | 'mqtt_connecting' | 'done' | 'failed'

// Device Info (read from characteristic)
export interface BleDeviceInfo {
  model: string
  sn: string
  fw: string
  netmod: string
  supported_netmods: string[]
}

// Network scan result
export interface BleWifiScanResult {
  ssid: string
  rssi: number
  auth: boolean
  channel?: number
}

export interface BleCat1Status {
  sim_ready: boolean
  signal_level: string
  signal_dbm: number
  imei: string
  iccid: string
  isp: string
  network_type: string
  register_status: string
}

// Status notification
export interface BleStatusNotification {
  step: BleStatusStep
  error?: string
  ip?: string
  net_type?: string
}

// Model to device type mapping
export const MODEL_TO_DEVICE_TYPE: Record<string, string> = {
  'NE101': 'ne101_camera',
  'NE301': 'ne301_camera',
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/lib/ble-protocol.ts
git commit -m "feat(ble): add BLE protocol constants and type definitions"
```

### Task 8: useBleProvision Hook

**Files:**
- Create: `web/src/hooks/useBleProvision.ts`

- [ ] **Step 1: Create the BLE provisioning hook**

Encapsulates all Web Bluetooth API interactions:

```typescript
// web/src/hooks/useBleProvision.ts
import { useState, useCallback, useRef } from 'react'
import {
  BLE_SERVICE_UUID, BLE_CHAR_DEVICE_INFO, BLE_CHAR_NETWORK_SCAN,
  BLE_CHAR_CONFIG, BLE_CHAR_STATUS, BLE_CHAR_APPLY,
  type BleDeviceInfo, type BleWifiScanResult, type BleCat1Status,
  type BleStatusNotification, type BleScanType, type BleConfigType,
} from '@/lib/ble-protocol'

interface BleProvisionState {
  scanning: boolean
  connecting: boolean
  provisioning: boolean
  deviceInfo: BleDeviceInfo | null
  scanResults: BleWifiScanResult[]
  cat1Status: BleCat1Status | null
  status: BleStatusNotification | null
  error: string | null
}

export function useBleProvision() {
  const [state, setState] = useState<BleProvisionState>({ ... })
  const deviceRef = useRef<BluetoothDevice | null>(null)
  const serverRef = useRef<BluetoothRemoteGATTServer | null>(null)

  // scan() — requestDevice with service UUID filter
  const scan = useCallback(async () => { ... }, [])

  // connect() — GATT connect + read Device Info
  const connect = useCallback(async (device: BluetoothDevice) => { ... }, [])

  // networkScan(type) — write to scan char, subscribe to notify, return results
  const networkScan = useCallback(async (type: BleScanType) => { ... }, [])

  // writeConfig(type, payload) — write JSON to Config characteristic
  const writeConfig = useCallback(async (type: BleConfigType, payload: Record<string, unknown>) => { ... }, [])

  // apply() — write to Apply char, subscribe to Status notify, return final status
  const apply = useCallback(async () => { ... }, [])

  // disconnect() — cleanup BLE connection
  const disconnect = useCallback(async () => { ... }, [])

  return {
    ...state,
    scan,
    connect,
    networkScan,
    writeConfig,
    apply,
    disconnect,
  }
}
```

Key implementation details:
- `scan()`: calls `navigator.bluetooth.requestDevice({ filters: [{ services: [BLE_SERVICE_UUID] }] })`
- `connect()`: `device.gatt.connect()` → `server.getPrimaryService()` → `service.getCharacteristic(BLE_CHAR_DEVICE_INFO)` → `characteristic.readValue()` → decode JSON
- `networkScan()`: write `{"type":"wifi"}` to scan char, add event listener for `characteristicvaluechanged`, return parsed Notify
- `writeConfig()`: encode JSON to Uint8Array via `TextEncoder`, write to Config char
- `apply()`: write `{"action":"apply"}` to Apply char, subscribe to Status Notify, resolve Promise on `done` or `failed`
- `disconnect()`: cleanup BLE connection. **If device was pre-registered (has device_id) but provisioning not complete**, call `DELETE /api/devices/{device_id}` to clean up the orphaned pre-registration.
- `cleanup()`: internal helper — calls `fetch('/api/devices/{device_id}', { method: 'DELETE' })` for error recovery.

- [ ] **Step 2: Commit**

```bash
git add web/src/hooks/useBleProvision.ts
git commit -m "feat(ble): add useBleProvision hook for Web Bluetooth API"
```

### Task 9: BleProvisionTab Component

**Files:**
- Create: `web/src/pages/devices/BleProvisionTab.tsx`

- [ ] **Step 1: Create the BLE provisioning tab component**

Steps through the provisioning flow as a state machine:

```
States: scan → device_selected → network_scan → configuring → applying → done | failed
```

UI per state:
- **scan**: Button "扫描BLE设备" → list found devices with name + RSSI
- **device_selected**: Show device info (model, SN, fw, supported_netmods)
  - Auto-match device type via `MODEL_TO_DEVICE_TYPE`
  - MQTT broker dropdown (fetched from `/api/mqtt/status` + `/api/brokers`)
  - Network config form (dynamic based on supported_netmods):
    - WiFi: SSID dropdown (from device scan) + password input
    - CAT.1: APN input + auth fields (if supported_netmods includes "cat1")
    - HaLow: SSID dropdown + password (if supported_netmods includes "halow")
  - Device name input
- **configuring**: Progress indicator while writing configs via BLE
- **applying**: Animated status display showing Notify transitions
- **done**: Success message, "查看设备" button
- **failed**: Error message, "重试" button

Use existing UI components: `Button`, `Input`, `Select`, `Label` from `@/components/ui/`.
Use i18n: all visible text via `t()` from `react-i18next`.
Use design tokens: `text-success`, `bg-error-light`, etc. — never hardcoded colors.

- [ ] **Step 2: Commit**

```bash
git add web/src/pages/devices/BleProvisionTab.tsx
git commit -m "feat(ble): add BleProvisionTab component with full provisioning flow"
```

### Task 10: AddDeviceGlobalDialog

**Files:**
- Create: `web/src/pages/devices/AddDeviceGlobalDialog.tsx`
- Modify: `web/src/pages/devices.tsx` (note: page file is directly in `pages/`, not `pages/devices/`)

- [ ] **Step 1: Create the global dialog with tabs**

```tsx
// web/src/pages/devices/AddDeviceGlobalDialog.tsx
// Uses existing dialog pattern (UnifiedFormDialog or Dialog from @/components/ui)
// Three tabs: BLE Provisioning | Manual Add | Auto Discovery

interface AddDeviceGlobalDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  deviceTypes: DeviceType[]
}

function AddDeviceGlobalDialog({ open, onOpenChange, deviceTypes }: Props) {
  const [activeTab, setActiveTab] = useState<'ble' | 'manual' | 'auto'>('ble')

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <Tabs value={activeTab} onValueChange={setActiveTab}>
          <TabsList>
            <TabsTrigger value="ble">{t('devices:ble.title')}</TabsTrigger>
            <TabsTrigger value="manual">{t('devices:add.title')}</TabsTrigger>
            <TabsTrigger value="auto">{t('devices:auto.title')}</TabsTrigger>
          </TabsList>
          <TabsContent value="ble">
            <BleProvisionTab deviceTypes={deviceTypes} />
          </TabsContent>
          <TabsContent value="manual">
            {/* Reuse existing AddDeviceDialog content */}
          </TabsContent>
          <TabsContent value="auto">
            {/* Reuse existing PendingDevicesList or link to it */}
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
```

- [ ] **Step 2: Modify devices.tsx to open GlobalDialog**

Replace `setAddDeviceDialogOpen(true)` with new global dialog:
```tsx
// Before:
<Button onClick={() => setAddDeviceDialogOpen(true)}>

// After:
<Button onClick={() => setGlobalDialogOpen(true)}>
```

Add `AddDeviceGlobalDialog` component to the page, keep existing `AddDeviceDialog` for backward compatibility but the button now opens the global dialog.

- [ ] **Step 3: Add i18n keys**

Add to `web/src/i18n/locales/en/devices.json` and `zh/devices.json`:
```json
{
  "ble": {
    "title": "Bluetooth Provisioning",
    "scan": "Scan BLE Devices",
    "scanning": "Scanning...",
    "no_devices": "No devices found",
    "select_device": "Select a device",
    "device_info": "Device Info",
    "network_config": "Network Configuration",
    "mqtt_broker": "MQTT Broker",
    "start_provision": "Start Provisioning",
    "provisioning": "Provisioning...",
    "status": {
      "net_connecting": "Connecting to network...",
      "net_connected": "Network connected",
      "mqtt_connecting": "Connecting to MQTT...",
      "done": "Device online!",
      "failed": "Provisioning failed"
    }
  },
  "auto": {
    "title": "Auto Discovery"
  }
}
```

- [ ] **Step 4: Run frontend build**

```bash
cd web && npm run build
```

Expected: no type errors, clean build.

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/devices/AddDeviceGlobalDialog.tsx \
        web/src/pages/devices/devices.tsx \
        web/src/i18n/locales/en/devices.json \
        web/src/i18n/locales/zh/devices.json
git commit -m "feat(ui): add global AddDevice dialog with BLE provisioning tab"
```

---

## Part 4: Protocol Documentation

### Task 11: NeoMind BLE Provisioning Protocol Document

**Files:**
- Create: `docs/guides/en/ble-provisioning.md`
- Create: `docs/guides/zh/ble-provisioning.md`

- [ ] **Step 1: Create the English protocol document**

Standalone document for device firmware developers. Contains everything needed to implement BLE provisioning on any device, without needing to read the NeoMind codebase.

Structure:
```
# NeoMind BLE Provisioning Protocol

## Overview
- Purpose: configure network + MQTT over BLE for zero-touch device onboarding
- Requirements: BLE 4.2+ with GATT Server support

## GATT Service Definition
- Service UUID: 9e5d1e47-5b13-4c4f-85b3-d0e6f5a7b8c9
- Advertising name format: "{MODEL}-{SERIAL}"
- 5 Characteristics with exact UUIDs, properties, and JSON payloads

## Characteristic Details
### Device Info (Read)
- UUID, JSON schema, example

### Network Scan (Write + Notify)
- UUID, scan request types, response formats for WiFi/CAT.1/HaLow

### Config (Write, encrypted)
- UUID, all config types with JSON schemas
- net_wifi, net_cat1, net_halow, mqtt
- Future reserved types

### Status (Read + Notify)
- UUID, step values, notify sequence, error codes

### Apply (Write)
- UUID, behavior description

## Provisioning Flow
- ASCII sequence diagram (platform → device)
- WiFi example, CAT.1 example
- Error handling

## Security Requirements
- BLE Secure Connections (pairing + encryption)
- Required for Config and Network Scan characteristics

## Device Implementation Guide
- ESP-IDF example snippets
- NVS storage recommendations
- Advertising data format
- How to detect supported network modules
- How to report connection status

## Platform Integration
- Model-to-device-type mapping
- MQTT topic format: device/{device_type}/{device_id}/uplink
- Device ID generation: sn.lowercase().replace("-","_")
```

- [ ] **Step 2: Create the Chinese version**

Translate the English document to Chinese, maintaining the same structure and technical accuracy.

- [ ] **Step 3: Commit**

```bash
git add docs/guides/en/ble-provisioning.md docs/guides/zh/ble-provisioning.md
git commit -m "docs: add NeoMind BLE provisioning protocol specification"
```

---

## Part 5: Integration Test

### Task 11: End-to-End BLE Provisioning Test

- [ ] **Step 1: Prepare NE101 device**

Flash NE101 with BLE provisioning firmware. Ensure no network config in NVS.

- [ ] **Step 2: Run NeoMind server**

```bash
cargo run -p neomind-cli -- serve
```

- [ ] **Step 3: Open NeoMind web UI in Chrome**

Navigate to `http://localhost:9375`, go to Devices page, click "添加设备".

- [ ] **Step 4: Execute BLE provisioning flow**

1. Click "蓝牙配网" tab
2. Click "扫描BLE设备" → should find `NE101-XXXXXX`
3. Select device → Device Info shown, device type auto-matched
4. Select MQTT broker → "NeoMind 内置"
5. Network scan → WiFi list shown
6. Select WiFi, enter password
7. Click "开始配网"
8. Observe status: `net_connecting → net_connected → mqtt_connecting → done`
9. Verify device appears in device list with "Connected" status

- [ ] **Step 5: Verify MQTT telemetry flowing**

Check device detail page shows live metrics from the NE101.

- [ ] **Step 6: Final commit**

```bash
git commit --allow-empty -m "test: BLE provisioning end-to-end verified"
```
