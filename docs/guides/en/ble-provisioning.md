# BLE Provisioning Protocol Specification

**Version**: 1.0.0
**Audience**: Device firmware developers implementing BLE provisioning compatible with NeoMind

## Overview

The NeoMind BLE Provisioning Protocol enables zero-touch device setup over Bluetooth Low Energy (BLE). A firmware developer implements the BLE GATT server described in this document on the device, and the NeoMind platform (mobile/desktop app) acts as the BLE central that configures the device with network and MQTT credentials.

The entire provisioning flow is:

1. Device advertises itself over BLE
2. Platform discovers and connects to the device
3. Platform reads device information
4. Platform pre-registers the device via the NeoMind API
5. Platform writes network and MQTT configuration to the device
6. Device connects to WiFi/CAT.1 and the MQTT broker
7. Device reports status transitions back to the platform

## GATT Service Definition

### Service UUID

```
9e5d1e47-5b13-4c4f-85b3-d0e6f5a7b8c9
```

The device must expose a single primary GATT service with this UUID containing exactly 5 characteristics.

### Characteristic Map

| # | Name         | UUID suffix | Full UUID                              | Properties      |
|---|--------------|-------------|----------------------------------------|-----------------|
| 1 | Device Info  | `...1e48`   | `9e5d1e48-5b13-4c4f-85b3-d0e6f5a7b8c9` | Read            |
| 2 | Network Scan | `...1e49`   | `9e5d1e49-5b13-4c4f-85b3-d0e6f5a7b8c9` | Write + Notify  |
| 3 | Config       | `...1e4a`   | `9e5d1e4a-5b13-4c4f-85b3-d0e6f5a7b8c9` | Write (encrypted) |
| 4 | Status       | `...1e4c`   | `9e5d1e4c-5b13-4c4f-85b3-d0e6f5a7b8c9` | Read + Notify   |
| 5 | Apply        | `...1e4d`   | `9e5d1e4d-5b13-4c4f-85b3-d0e6f5a7b8c9` | Write           |

---

## Characteristics

### 1. Device Info (Read)

**UUID**: `9e5d1e48-5b13-4c4f-85b3-d0e6f5a7b8c9`

Returns static device identification as a JSON object. The platform reads this characteristic immediately after connecting.

**Response format**:

```json
{
  "model": "NE101",
  "sn": "NE101-A2F003",
  "fw": "1.0.0",
  "netmod": "",
  "supported_netmods": ["wifi", "cat1"]
}
```

| Field               | Type     | Description                                                        |
|---------------------|----------|--------------------------------------------------------------------|
| `model`             | string   | Device model identifier (e.g. `NE101`, `NE301`)                    |
| `sn`                | string   | Serial number, unique per device                                   |
| `fw`                | string   | Firmware version in semver format                                  |
| `netmod`            | string   | Current active network module. Empty string if unconfigured        |
| `supported_netmods` | string[] | Network types the hardware supports: `wifi`, `cat1`, `halow`      |

**Notes**:
- This characteristic should be readable without encryption (no pairing required).
- The `model` field is used by the platform to look up the device type template.
- The `sn` field is used to derive the device ID (see [Device ID Generation](#device-id-generation)).

---

### 2. Network Scan (Write + Notify)

**UUID**: `9e5d1e49-5b13-4c4f-85b3-d0e6f5a7b8c9`

Used by the platform to trigger a network scan on the device. The platform writes a scan request, then subscribes to notifications to receive results.

**Write request format**:

```json
{"type": "wifi"}
```

**Supported scan types**:

| Type          | Description                       |
|---------------|-----------------------------------|
| `wifi`        | Scan for nearby WiFi access points|
| `halow`       | Scan for HaLow (802.11ah) networks|
| `cat1_status` | Query CAT.1 modem status          |

#### WiFi / HaLow Response (via Notify)

Returned as a JSON array of access points, sorted by signal strength (strongest first):

```json
[
  {"ssid": "MyNetwork", "rssi": -45, "auth": true, "channel": 6},
  {"ssid": "OpenNet", "rssi": -72, "auth": false, "channel": 11}
]
```

| Field     | Type    | Description                           |
|-----------|---------|---------------------------------------|
| `ssid`    | string  | Access point SSID                     |
| `rssi`    | integer | Signal strength in dBm                |
| `auth`    | boolean | Whether authentication is required    |
| `channel` | integer | WiFi channel number                   |

#### CAT.1 Status Response (via Notify)

Returned as a JSON object describing the cellular modem state:

```json
{
  "sim_ready": true,
  "signal_level": "good",
  "signal_dbm": -75,
  "imei": "867891023456789",
  "iccid": "89860123456789012345",
  "isp": "China Mobile",
  "network_type": "LTE",
  "register_status": "registered"
}
```

| Field             | Type    | Description                                       |
|-------------------|---------|---------------------------------------------------|
| `sim_ready`       | boolean | Whether a SIM card is detected and ready           |
| `signal_level`    | string  | Signal quality: `excellent`, `good`, `fair`, `poor`|
| `signal_dbm`      | integer | Signal strength in dBm                             |
| `imei`            | string  | Device IMEI                                        |
| `iccid`           | string  | SIM card ICCID                                     |
| `isp`             | string  | Detected carrier/ISP name                          |
| `network_type`    | string  | Network type: `LTE`, `WCDMA`, `GSM`, etc.          |
| `register_status` | string  | Network registration status: `registered`, `searching`, `denied`, `unknown` |

**Notes**:
- The scan may take several seconds. The device should send the notification only when the scan completes.
- If the scan fails, send an empty response: WiFi/HaLow sends `[]`, CAT.1 sends an object with `sim_ready: false`.
- BLE notifications have a 20-byte default MTU. For large responses, use Indicate or negotiate a larger MTU (recommended: 512 bytes). If MTU negotiation is not possible, chunk the JSON across multiple notifications with a framing protocol.

---

### 3. Config (Write, Encrypted)

**UUID**: `9e5d1e4a-5b13-4c4f-85b3-d0e6f5a7b8c9`

Receives network and MQTT configuration from the platform. **This characteristic must only be writable when the BLE link is encrypted** (i.e., after completing BLE Secure Connections pairing).

The Config characteristic uses type-discriminated JSON. Each write contains a single configuration object with a `type` field that indicates the configuration category.

#### WiFi Network Config

```json
{"type": "net_wifi", "ssid": "MyWiFi", "password": "pass123"}
```

| Field      | Type   | Description         |
|------------|--------|---------------------|
| `type`     | string | `"net_wifi"`        |
| `ssid`     | string | WiFi SSID           |
| `password` | string | WiFi password       |

#### CAT.1 Network Config

```json
{
  "type": "net_cat1",
  "apn": "cmnet",
  "user": "",
  "password": "",
  "pin": "",
  "auth_type": 0
}
```

| Field        | Type    | Description                                  |
|--------------|---------|----------------------------------------------|
| `type`       | string  | `"net_cat1"`                                 |
| `apn`        | string  | Access Point Name                            |
| `user`       | string  | Authentication username (empty if none)      |
| `password`   | string  | Authentication password (empty if none)      |
| `pin`        | string  | SIM PIN (empty if none)                      |
| `auth_type`  | integer | Authentication type: `0` = None, `1` = PAP, `2` = CHAP |

#### HaLow Network Config

```json
{"type": "net_halow", "ssid": "HaLowNet", "password": "pass123"}
```

| Field      | Type   | Description         |
|------------|--------|---------------------|
| `type`     | string | `"net_halow"`       |
| `ssid`     | string | HaLow network SSID  |
| `password` | string | Network password    |

#### MQTT Broker Config

```json
{
  "type": "mqtt",
  "host": "192.168.1.100",
  "port": 1883,
  "username": "",
  "password": "",
  "topic_prefix": "device/ne101_camera/ne101_a2f003"
}
```

| Field          | Type    | Description                                              |
|----------------|---------|----------------------------------------------------------|
| `type`         | string  | `"mqtt"`                                                 |
| `host`         | string  | MQTT broker IP address or hostname                       |
| `port`         | integer | MQTT broker port (typically 1883)                        |
| `username`     | string  | Authentication username (empty for embedded broker)      |
| `password`     | string  | Authentication password (empty for embedded broker)      |
| `topic_prefix` | string  | Topic prefix for this device (see [MQTT Topics](#mqtt-topic-format)) |

**Forward compatibility**: Unknown `type` values must be silently ignored. This allows the protocol to be extended in future versions without breaking older firmware.

**Config buffering**: The platform typically writes network config first, then MQTT config. The device should buffer all configs and only apply them when the Apply command is received. Do not attempt to connect to the network or broker until Apply is triggered.

---

### 4. Status (Read + Notify)

**UUID**: `9e5d1e4c-5b13-4c4f-85b3-d0e6f5a7b8c9`

Reports the provisioning progress through a state machine. The platform subscribes to notifications on this characteristic after sending the Apply command.

#### State Machine

```
idle --> net_connecting --> net_connected --> mqtt_connecting --> done
  |                                                                         ^
  +-------> failed <--------------------------------------------------------+
```

The device transitions through these states sequentially. Any state can transition to `failed` on error. After reaching `done`, the device is fully provisioned and connected to the MQTT broker.

#### Notify Payload Examples

**Network connecting**:
```json
{"step": "net_connecting"}
```

**Network connected** (includes acquired IP and network type):
```json
{"step": "net_connected", "ip": "192.168.1.42", "net_type": "wifi"}
```

**MQTT connecting**:
```json
{"step": "mqtt_connecting"}
```

**Provisioning complete**:
```json
{"step": "done"}
```

**Provisioning failed** (includes error code):
```json
{"step": "failed", "error": "wifi_timeout"}
```

#### Error Codes

| Error Code          | Description                              |
|---------------------|------------------------------------------|
| `wifi_timeout`      | WiFi connection timed out                 |
| `wifi_auth_failed`  | WiFi authentication failed (wrong password) |
| `cat1_no_sim`       | No SIM card detected                      |
| `cat1_no_signal`    | No cellular signal available              |
| `mqtt_refused`      | MQTT broker refused connection (auth failed) |
| `mqtt_timeout`      | MQTT connection timed out                 |
| `unknown`           | Unspecified error                         |

**Notes**:
- The platform uses these status updates to display progress to the user.
- The `ip` and `net_type` fields are only present in the `net_connected` step.
- The `error` field is only present in the `failed` step.
- Reading this characteristic at any time should return the current status.

---

### 5. Apply (Write)

**UUID**: `9e5d1e4d-5b13-4c4f-85b3-d0e6f5a7b8c9`

Triggers the device to commit all buffered configuration and start the provisioning process.

**Write payload**:

```json
{"action": "apply"}
```

After receiving this command, the device must:

1. Validate that all required configurations have been received (network + MQTT).
2. Persist the configuration to non-volatile storage.
3. Transition the Status characteristic from `idle` to `net_connecting`.
4. Attempt to connect to the configured network.
5. On success, transition to `net_connected`.
6. Attempt to connect to the MQTT broker.
7. On success, transition to `mqtt_connecting` then `done`.
8. On any failure, transition to `failed` with the appropriate error code.

**Important**: Once Apply is received, the device should not accept further Config writes until the provisioning attempt completes (either `done` or `failed`). After a `failed` state, the device should return to accepting new Config writes to allow retry.

---

## Provisioning Flow

### Sequence Diagram

```
Platform (BLE Central)                    Device (BLE Peripheral)
========================                  ======================
    |                                              |
    |  <-- Advertisement (service UUID) -----------|
    |                                              |
    |  ---- BLE Connect -------------------------> |
    |                                              |
    |  ---- Pair (Secure Connections) -----------> |
    |  <--- Pairing Complete --------------------  |
    |                                              |
    |  ---- Read Device Info ------------------->  |
    |  <--- {"model":"NE101","sn":"NE101-..."} --- |
    |                                              |
    |  ====== (Platform calls NeoMind API) ====== |
    |  POST /api/devices/ble-provision             |
    |  --> receives mqtt_config                    |
    |  ==========================================  |
    |                                              |
    |  ---- Write Network Scan (wifi) --------->   |
    |  <--- Notify: Scan Results ---------------   |
    |                                              |
    |  ---- Write Config (net_wifi) ----------->   |
    |  <--- Write Confirmation ----------------   |
    |                                              |
    |  ---- Write Config (mqtt) --------------->   |
    |  <--- Write Confirmation ----------------   |
    |                                              |
    |  ---- Write Apply ------------------------>  |
    |                                              |
    |  <--- Notify: {"step":"net_connecting"} ---  |
    |         ... (user sees "Connecting...")      |
    |  <--- Notify: {"step":"net_connected"} ---   |
    |  <--- Notify: {"step":"mqtt_connecting"} --  |
    |  <--- Notify: {"step":"done"} ------------   |
    |                                              |
    |  ---- BLE Disconnect --------------------->  |
    |                                              |
    |         Device is now online on MQTT         |
```

### Step-by-Step

1. **BLE Discovery**: The platform scans for BLE devices advertising the service UUID `9e5d1e47-...`. The device name format is `{MODEL}-{SN_SUFFIX}` (e.g., `NE101-A2F003`).

2. **Connection**: The user selects a device from the scan results. The platform establishes a BLE connection.

3. **Pairing**: The platform initiates BLE Secure Connections (LE Secure Connections pairing). This is required before writing to the Config characteristic.

4. **Read Device Info**: The platform reads the Device Info characteristic to obtain the model, serial number, firmware version, and supported network modules.

5. **API Pre-registration**: The platform calls the NeoMind REST API to register the device and obtain MQTT broker configuration.

   **API endpoint**: `POST /api/devices/ble-provision`

   **Request body**:
   ```json
   {
     "model": "NE101",
     "sn": "NE101-A2F003",
     "device_type": "ne101_camera",
     "device_name": "Front Door Camera",
     "broker_id": "embedded"
   }
   ```

   **Response**:
   ```json
   {
     "device_id": "ne101_a2f003",
     "mqtt_config": {
       "host": "192.168.1.100",
       "port": 1883,
       "username": "",
       "password": "",
       "topic_prefix": "device/ne101_camera/ne101_a2f003"
     }
   }
   ```

6. **Network Scan** (optional): If the user needs to select a WiFi network, the platform triggers a scan via the Network Scan characteristic.

7. **Write Network Config**: The platform writes the selected network credentials to the Config characteristic. Only one network type config should be written per provisioning session.

8. **Write MQTT Config**: The platform writes the MQTT broker configuration (from the API response) to the Config characteristic.

9. **Apply**: The platform writes the Apply command. The device begins connecting.

10. **Status Monitoring**: The platform subscribes to Status notifications and displays progress. On `done`, the device is online. On `failed`, the user is shown the error and can retry.

---

## Device ID Generation

The device ID is derived from the serial number using the following deterministic formula:

```
device_id = sn.to_lowercase().replace("-", "_")
```

**Examples**:

| Serial Number     | Device ID        |
|-------------------|------------------|
| `NE101-A2F003`    | `ne101_a2f003`   |
| `NE301-B1C042`    | `ne301_b1c042`   |

This transformation ensures consistent device identity across the BLE provisioning flow and the NeoMind platform.

---

## MQTT Topic Format

After provisioning, the device communicates with the NeoMind MQTT broker using the following topic structure:

| Direction   | Topic Pattern                              | Description          |
|-------------|-------------------------------------------|----------------------|
| Device -> Platform | `device/{device_type}/{device_id}/uplink`   | Telemetry data       |
| Platform -> Device | `device/{device_type}/{device_id}/downlink` | Commands and control |

**Example topics** for device `ne101_a2f003` of type `ne101_camera`:

- Uplink: `device/ne101_camera/ne101_a2f003/uplink`
- Downlink: `device/ne101_camera/ne101_a2f003/downlink`

The `topic_prefix` provided in the MQTT config is `device/{device_type}/{device_id}`. The device appends `/uplink` for publishing and subscribes to `{topic_prefix}/downlink` for commands.

### MQTT Uplink Payload Format

```json
{
  "device_id": "ne101_a2f003",
  "timestamp": 1700000000,
  "data": {
    "temperature": 23.5,
    "humidity": 65.0
  }
}
```

---

## Model to Device Type Mapping

The platform uses the `model` field from Device Info to determine the device type template.

| Model   | Device Type       | Description       |
|---------|-------------------|-------------------|
| `NE101` | `ne101_camera`    | NE101 Camera      |
| `NE301` | `ne301_camera`    | NE301 Camera      |

The device type template must be registered in NeoMind before provisioning. If the template does not exist, the `POST /api/devices/ble-provision` API call will return a 400 error.

---

## BLE Advertising Requirements

### Advertising Data

The device must include the following in its advertising data:

1. **Complete Local Name**: `{MODEL}-{SN_SUFFIX}` (e.g., `NE101-A2F003`)
   - `MODEL` is the device model (e.g., `NE101`)
   - `SN_SUFFIX` is the last portion of the serial number (e.g., `A2F003`)

2. **Service UUID**: `9e5d1e47-5b13-4c4f-85b3-d0e6f5a7b8c9` must be included in the advertising packet so the platform can filter devices.

### Advertising Parameters

- **Interval**: Recommended 100ms - 500ms (faster interval = quicker discovery, but higher power consumption)
- **Connectable**: Must be connectable (ADV_IND)
- **Timeout**: No timeout; continue advertising until connected or provisioning is complete

### Scan Response Data

It is recommended to include the following in the scan response:

- **Complete Local Name** (if not fully included in advertising data)
- **TX Power Level**

---

## Security

### BLE Secure Connections

The provisioning protocol uses BLE Secure Connections (LE Secure Connections pairing) to protect the configuration data transmitted to the device.

**Requirements**:

1. **Pairing method**: LE Secure Connections (not legacy pairing)
2. **Encryption requirement**: The Config characteristic (`...1e4a`) must reject writes that are not sent over an encrypted link
3. **Authentication**: The device should use either Just Works or Passkey entry pairing depending on the device's I/O capabilities

**Implementation guidance**:

- On ESP-IDF, enable `CONFIG_BT_NIMBLE_SM_SC` (Secure Connections) and `CONFIG_BT_NIMBLE_SM_LVL_REQ` set to at least 2 (encrypted).
- The device should check that the link is encrypted before processing Config writes. Return `ATT_ERR_INSUFFICIENT_ENCRYPTION` (0x0F) or `ATT_ERR_INSUFFICIENT_AUTHEN` (0x0E) if the link is not encrypted.
- Device Info and Network Scan may be accessible without encryption to allow initial device discovery and network listing before pairing.

### Configuration Protection

- Network credentials (WiFi password, APN credentials) and MQTT credentials are transmitted through the encrypted Config characteristic.
- After provisioning, the device should store credentials in encrypted flash (e.g., ESP-IDF NVS encryption) rather than plaintext.

---

## ESP-IDF Implementation Guide

This section provides guidance for implementing the BLE provisioning GATT server on ESP-IDF (ESP32 series).

### Prerequisites

- ESP-IDF v5.0 or later
- NimBLE BLE stack (recommended over Bluedroid for lower memory footprint)
- `esp_wifi` component for WiFi connectivity
- `esp_mqtt` component for MQTT communication

### sdkconfig Defaults

```
# BLE Configuration
CONFIG_BT_ENABLED=y
CONFIG_BT_NIMBLE_ENABLED=y
CONFIG_BT_NIMBLE_SM_SC=y
CONFIG_BT_NIMBLE_SM_LVL_REQ=2

# NVS Encryption (recommended)
CONFIG_NVS_ENCRYPTION=y
```

### GATT Service Registration (Pseudocode)

```c
#include "host/ble_hs.h"
#include "host/ble_gap.h"

#define SERVICE_UUID \
    ((ble_uuid128_t[]) {{ .u = { .type = BLE_UUID_TYPE_128 }, \
        .value = { 0xc9, 0xb8, 0xa7, 0xf5, 0xe6, 0xd0, 0xb3, 0x85, \
                   0x4f, 0x4c, 0x13, 0x5b, 0x47, 0x1e, 0x5d, 0x9e }}})

// Characteristic UUIDs share the same base; only the 14th byte differs
#define CHAR_UUID_DEVICE_INFO  0x48  // ...1e48
#define CHAR_UUID_NETWORK_SCAN 0x49  // ...1e49
#define CHAR_UUID_CONFIG       0x4a  // ...1e4a
#define CHAR_UUID_STATUS       0x4c  // ...1e4c
#define CHAR_UUID_APPLY        0x4d  // ...1e4d

static const struct ble_gatt_svc_def gatt_svcs[] = {
    {
        .type = BLE_GATT_SVC_TYPE_PRIMARY,
        .uuid = BLE_UUID128(SERVICE_UUID),
        .characteristics = (struct ble_gatt_chr_def[]) {
            {
                // Device Info (Read)
                .uuid = &CHAR_UUID(CHAR_UUID_DEVICE_INFO)->u,
                .access_cb = device_info_access_cb,
                .flags = BLE_GATT_CHR_F_READ,
            },
            {
                // Network Scan (Write + Notify)
                .uuid = &CHAR_UUID(CHAR_UUID_NETWORK_SCAN)->u,
                .access_cb = network_scan_access_cb,
                .flags = BLE_GATT_CHR_F_WRITE | BLE_GATT_CHR_F_NOTIFY,
            },
            {
                // Config (Write, encrypted)
                .uuid = &CHAR_UUID(CHAR_UUID_CONFIG)->u,
                .access_cb = config_access_cb,
                .flags = BLE_GATT_CHR_F_WRITE |
                         BLE_GATT_CHR_F_WRITE_AUTHEN,  // Require encryption
            },
            {
                // Status (Read + Notify)
                .uuid = &CHAR_UUID(CHAR_UUID_STATUS)->u,
                .access_cb = status_access_cb,
                .flags = BLE_GATT_CHR_F_READ | BLE_GATT_CHR_F_NOTIFY,
            },
            {
                // Apply (Write)
                .uuid = &CHAR_UUID(CHAR_UUID_APPLY)->u,
                .access_cb = apply_access_cb,
                .flags = BLE_GATT_CHR_F_WRITE,
            },
            { 0 }  // Terminator
        },
    },
    { 0 }  // Terminator
};
```

### Device Info Handler

```c
static int device_info_access_cb(uint16_t conn_handle, uint16_t attr_handle,
                                  struct ble_gatt_access_ctxt *ctxt, void *arg)
{
    if (ctxt->op == BLE_GATT_ACCESS_OP_READ_CHR) {
        // Build JSON response
        const char *json = "{"
            "\"model\":\"NE101\","
            "\"sn\":\"NE101-A2F003\","
            "\"fw\":\"1.0.0\","
            "\"netmod\":\"\","
            "\"supported_netmods\":[\"wifi\",\"cat1\"]"
        "}";
        os_mbuf_append(ctxt->om, json, strlen(json));
        return 0;
    }
    return BLE_ATT_ERR_UNLIKELY;
}
```

### Config Handler with Encryption Check

```c
static int config_access_cb(uint16_t conn_handle, uint16_t attr_handle,
                             struct ble_gatt_access_ctxt *ctxt, void *arg)
{
    if (ctxt->op == BLE_GATT_ACCESS_OP_WRITE_CHR) {
        // Verify encryption
        struct ble_gap_conn_desc desc;
        ble_gap_conn_find(conn_handle, &desc);
        if (!desc.sec_state.encrypted) {
            return BLE_ATT_ERR_INSUFFICIENT_AUTHEN;
        }

        // Parse JSON from write data
        char buf[256] = {0};
        uint16_t om_len = OS_MBUF_PKTLEN(ctxt->om);
        os_mbuf_copydata(ctxt->om, 0, om_len > 255 ? 255 : om_len, buf);

        // Parse type field and dispatch
        cJSON *root = cJSON_Parse(buf);
        cJSON *type = cJSON_GetObjectItem(root, "type");

        if (strcmp(type->valuestring, "net_wifi") == 0) {
            // Store WiFi config
            store_wifi_config(
                cJSON_GetObjectItem(root, "ssid")->valuestring,
                cJSON_GetObjectItem(root, "password")->valuestring
            );
        } else if (strcmp(type->valuestring, "mqtt") == 0) {
            // Store MQTT config
            store_mqtt_config(
                cJSON_GetObjectItem(root, "host")->valuestring,
                cJSON_GetObjectItem(root, "port")->valueint,
                cJSON_GetObjectItem(root, "username")->valuestring,
                cJSON_GetObjectItem(root, "password")->valuestring,
                cJSON_GetObjectItem(root, "topic_prefix")->valuestring
            );
        }
        // Silently ignore unknown types for forward compatibility

        cJSON_Delete(root);
        return 0;
    }
    return BLE_ATT_ERR_UNLIKELY;
}
```

### Status Notification Helper

```c
static uint16_t status_conn_handle;
static uint16_t status_attr_handle;

void notify_status(const char *step, const char *error,
                   const char *ip, const char *net_type)
{
    cJSON *root = cJSON_CreateObject();
    cJSON_AddStringToObject(root, "step", step);
    if (error) cJSON_AddStringToObject(root, "error", error);
    if (ip) cJSON_AddStringToObject(root, "ip", ip);
    if (net_type) cJSON_AddStringToObject(root, "net_type", net_type);

    char *json = cJSON_PrintUnformatted(root);

    struct os_mbuf *om = ble_hs_mbuf_from_flat(json, strlen(json));
    ble_gatts_notify_custom(status_conn_handle, status_attr_handle, om);

    free(json);
    cJSON_Delete(root);
}
```

### Apply Handler and State Machine

```c
static void provisioning_task(void *arg)
{
    // Step 1: Connect to network
    notify_status("net_connecting", NULL, NULL, NULL);

    if (!wifi_connect()) {
        notify_status("failed", "wifi_timeout", NULL, NULL);
        return;
    }

    char ip[16];
    get_ip_address(ip);
    notify_status("net_connected", NULL, ip, "wifi");

    // Step 2: Connect to MQTT broker
    notify_status("mqtt_connecting", NULL, NULL, NULL);

    if (!mqtt_connect()) {
        notify_status("failed", "mqtt_refused", NULL, NULL);
        return;
    }

    notify_status("done", NULL, NULL, NULL);

    // Device is now online; BLE can disconnect
    ble_gap_terminate(status_conn_handle, BLE_ERR_REM_USER_CONN_TERM);
}
```

### Advertising Configuration

```c
static void start_advertising(void)
{
    struct ble_gap_adv_params adv_params;
    struct ble_hs_adv_fields fields;

    memset(&fields, 0, sizeof(fields));

    // Include service UUID in advertising data
    fields.uuids128 = SERVICE_UUID;
    fields.num_uuids128 = 1;
    fields.uuids128_is_complete = 1;

    // Include device name
    fields.name = (const uint8_t *)"NE101-A2F003";
    fields.name_len = strlen("NE101-A2F003");
    fields.name_is_complete = 1;

    ble_gap_adv_set_fields(&fields);

    memset(&adv_params, 0, sizeof(adv_params));
    adv_params.conn_mode = BLE_GAP_CONN_MODE_UND;  // Connectable
    adv_params.disc_mode = BLE_GAP_DISC_MODE_GEN;  // General discoverable
    adv_params.itvl_min = 0x00A0;  // 100ms
    adv_params.itvl_max = 0x01A0;  // 260ms

    ble_gap_adv_start(BLE_OWN_ADDR_PUBLIC, NULL, BLE_HS_FOREVER,
                      &adv_params, gap_event_handler, NULL);
}
```

---

## Error Handling

### BLE-Level Errors

The device should return appropriate BLE ATT error codes:

| Scenario                                | Error Code                       |
|-----------------------------------------|----------------------------------|
| Config write without encryption         | `0x0E` (Insufficient Authentication) |
| Write to read-only characteristic       | `0x06` (Request Not Supported)  |
| Malformed JSON in write                 | `0x0D` (Invalid Attribute Value) |
| Characteristic not found                | `0x0A` (Attribute Not Found)     |

### Provisioning Retry

If the provisioning fails (Status reports `failed`):

1. The platform displays the error to the user.
2. The user can update the configuration (e.g., correct WiFi password).
3. The platform writes new Config data.
4. The platform writes Apply again.
5. The device must reset its state machine to `idle` before accepting the new Apply.

### Timeout Recommendations

| Operation         | Timeout   |
|-------------------|-----------|
| BLE Connection    | 30 seconds|
| Pairing           | 60 seconds|
| Network Scan      | 15 seconds|
| WiFi Connect      | 30 seconds|
| CAT.1 Connect     | 60 seconds|
| MQTT Connect      | 15 seconds|

---

## Appendix: JSON Message Quick Reference

### Messages Written by Platform

| Characteristic   | Direction | Payload                                                         |
|------------------|-----------|----------------------------------------------------------------|
| Network Scan     | Write     | `{"type":"wifi"}`                                              |
| Network Scan     | Write     | `{"type":"cat1_status"}`                                       |
| Network Scan     | Write     | `{"type":"halow"}`                                             |
| Config           | Write     | `{"type":"net_wifi","ssid":"...","password":"..."}`            |
| Config           | Write     | `{"type":"net_cat1","apn":"...","user":"","password":"","pin":"","auth_type":0}` |
| Config           | Write     | `{"type":"net_halow","ssid":"...","password":"..."}`           |
| Config           | Write     | `{"type":"mqtt","host":"...","port":1883,"username":"","password":"","topic_prefix":"..."}` |
| Apply            | Write     | `{"action":"apply"}`                                           |

### Messages Returned by Device

| Characteristic   | Direction | Payload                                                         |
|------------------|-----------|----------------------------------------------------------------|
| Device Info      | Read      | `{"model":"NE101","sn":"NE101-A2F003","fw":"1.0.0","netmod":"","supported_netmods":["wifi"]}` |
| Network Scan     | Notify    | `[{"ssid":"...","rssi":-45,"auth":true,"channel":6}]`          |
| Network Scan     | Notify    | `{"sim_ready":true,"signal_level":"good","signal_dbm":-75,...}`|
| Status           | Notify    | `{"step":"net_connecting"}`                                    |
| Status           | Notify    | `{"step":"net_connected","ip":"192.168.1.42","net_type":"wifi"}` |
| Status           | Notify    | `{"step":"mqtt_connecting"}`                                   |
| Status           | Notify    | `{"step":"done"}`                                              |
| Status           | Notify    | `{"step":"failed","error":"wifi_timeout"}`                     |
