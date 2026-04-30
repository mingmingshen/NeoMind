# BLE 配网协议规范

**版本**: 1.0.0
**目标读者**: 需要实现与 NeoMind 平台兼容的 BLE 配网功能的设备固件开发者

## 概述

NeoMind BLE 配网协议支持通过蓝牙低功耗 (BLE) 实现设备的零接触配置。固件开发者在设备端实现本文档描述的 BLE GATT 服务端，NeoMind 平台（移动端/桌面应用）作为 BLE 中央设备，向设备写入网络和 MQTT 凭据。

完整配网流程如下：

1. 设备通过 BLE 广播自身信息
2. 平台发现并连接设备
3. 平台读取设备信息
4. 平台通过 NeoMind API 预注册设备
5. 平台向设备写入网络和 MQTT 配置
6. 设备连接 WiFi/CAT.1 和 MQTT 代理
7. 设备通过通知将状态变化报告给平台

## GATT 服务定义

### 服务 UUID

```
9e5d1e47-5b13-4c4f-85b3-d0e6f5a7b8c9
```

设备必须暴露一个以此 UUID 作为主 GATT 服务，其中包含恰好 5 个特征值。

### 特征值一览

| # | 名称       | UUID 后缀 | 完整 UUID                                | 属性             |
|---|-----------|-----------|------------------------------------------|-----------------|
| 1 | Device Info | `...1e48` | `9e5d1e48-5b13-4c4f-85b3-d0e6f5a7b8c9`  | Read            |
| 2 | Network Scan| `...1e49` | `9e5d1e49-5b13-4c4f-85b3-d0e6f5a7b8c9`  | Write + Notify  |
| 3 | Config     | `...1e4a` | `9e5d1e4a-5b13-4c4f-85b3-d0e6f5a7b8c9`  | Write（需加密）    |
| 4 | Status     | `...1e4c` | `9e5d1e4c-5b13-4c4f-85b3-d0e6f5a7b8c9`  | Read + Notify   |
| 5 | Apply      | `...1e4d` | `9e5d1e4d-5b13-4c4f-85b3-d0e6f5a7b8c9`  | Write           |

---

## 特征值详细说明

### 1. Device Info（设备信息，读）

**UUID**: `9e5d1e48-5b13-4c4f-85b3-d0e6f5a7b8c9`

返回静态设备标识信息，格式为 JSON 对象。平台在连接建立后立即读取此特征值。

**响应格式**:

```json
{
  "model": "NE101",
  "sn": "NE101-A2F003",
  "fw": "1.0.0",
  "netmod": "",
  "supported_netmods": ["wifi", "cat1"]
}
```

| 字段                 | 类型     | 说明                                                       |
|---------------------|----------|-----------------------------------------------------------|
| `model`             | string   | 设备型号标识（如 `NE101`、`NE301`）                          |
| `sn`                | string   | 序列号，每台设备唯一                                          |
| `fw`                | string   | 固件版本，采用 semver 格式                                    |
| `netmod`            | string   | 当前激活的网络模块。未配置时为空字符串                           |
| `supported_netmods` | string[] | 硬件支持的网络类型：`wifi`、`cat1`、`halow`                   |

**注意事项**:
- 此特征值无需加密即可读取（无需配对）。
- 平台通过 `model` 字段查找对应的设备类型模板。
- 平台通过 `sn` 字段派生设备 ID（参见 [设备 ID 生成](#设备-id-生成)）。

---

### 2. Network Scan（网络扫描，写 + 通知）

**UUID**: `9e5d1e49-5b13-4c4f-85b3-d0e6f5a7b8c9`

平台通过此特征值触发设备进行网络扫描。平台写入扫描请求后，订阅通知以接收扫描结果。

**写入请求格式**:

```json
{"type": "wifi"}
```

**支持的扫描类型**:

| 类型           | 说明                |
|---------------|---------------------|
| `wifi`        | 扫描附近的 WiFi 接入点 |
| `halow`       | 扫描 HaLow (802.11ah) 网络 |
| `cat1_status` | 查询 CAT.1 模组状态   |

#### WiFi / HaLow 扫描结果（通过通知返回）

以 JSON 数组形式返回接入点列表，按信号强度从强到弱排序：

```json
[
  {"ssid": "MyNetwork", "rssi": -45, "auth": true, "channel": 6},
  {"ssid": "OpenNet", "rssi": -72, "auth": false, "channel": 11}
]
```

| 字段      | 类型     | 说明               |
|----------|----------|-------------------|
| `ssid`   | string   | 接入点 SSID        |
| `rssi`   | integer  | 信号强度（dBm）    |
| `auth`   | boolean  | 是否需要认证       |
| `channel`| integer  | WiFi 信道号        |

#### CAT.1 状态结果（通过通知返回）

以 JSON 对象形式返回蜂窝模组状态：

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

| 字段               | 类型     | 说明                                                    |
|-------------------|----------|--------------------------------------------------------|
| `sim_ready`       | boolean  | 是否检测到 SIM 卡且就绪                                   |
| `signal_level`    | string   | 信号质量：`excellent`、`good`、`fair`、`poor`              |
| `signal_dbm`      | integer  | 信号强度（dBm）                                          |
| `imei`            | string   | 设备 IMEI                                               |
| `iccid`           | string   | SIM 卡 ICCID                                            |
| `isp`             | string   | 检测到的运营商名称                                        |
| `network_type`    | string   | 网络类型：`LTE`、`WCDMA`、`GSM` 等                        |
| `register_status` | string   | 网络注册状态：`registered`、`searching`、`denied`、`unknown` |

**注意事项**:
- 扫描可能耗时数秒。设备应仅在扫描完成后发送通知。
- 扫描失败时，WiFi/HaLow 返回空数组 `[]`，CAT.1 返回 `sim_ready: false` 的对象。
- BLE 通知默认 MTU 为 20 字节。对于较大响应，建议使用 Indicate 或协商更大的 MTU（推荐 512 字节）。如无法协商 MTU，需将 JSON 分片发送。

---

### 3. Config（配置，写，需加密）

**UUID**: `9e5d1e4a-5b13-4c4f-85b3-d0e6f5a7b8c9`

接收平台发送的网络和 MQTT 配置。**此特征值仅在 BLE 链路加密时（即完成 BLE Secure Connections 配对后）才允许写入。**

Config 特征值使用类型判别 JSON。每次写入包含一个配置对象，通过 `type` 字段标识配置类别。

#### WiFi 网络配置

```json
{"type": "net_wifi", "ssid": "MyWiFi", "password": "pass123"}
```

| 字段        | 类型     | 说明           |
|------------|----------|---------------|
| `type`     | string   | `"net_wifi"`  |
| `ssid`     | string   | WiFi SSID     |
| `password` | string   | WiFi 密码      |

#### CAT.1 网络配置

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

| 字段         | 类型     | 说明                                          |
|-------------|----------|-----------------------------------------------|
| `type`      | string   | `"net_cat1"`                                  |
| `apn`       | string   | 接入点名称 (APN)                                |
| `user`      | string   | 认证用户名（无则留空）                            |
| `password`  | string   | 认证密码（无则留空）                              |
| `pin`       | string   | SIM 卡 PIN（无则留空）                           |
| `auth_type` | integer  | 认证类型：`0` = 无, `1` = PAP, `2` = CHAP       |

#### HaLow 网络配置

```json
{"type": "net_halow", "ssid": "HaLowNet", "password": "pass123"}
```

| 字段        | 类型     | 说明              |
|------------|----------|------------------|
| `type`     | string   | `"net_halow"`    |
| `ssid`     | string   | HaLow 网络 SSID   |
| `password` | string   | 网络密码          |

#### MQTT 代理配置

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

| 字段            | 类型     | 说明                                                   |
|----------------|----------|--------------------------------------------------------|
| `type`         | string   | `"mqtt"`                                               |
| `host`         | string   | MQTT 代理 IP 地址或主机名                                |
| `port`         | integer  | MQTT 代理端口（通常为 1883）                              |
| `username`     | string   | 认证用户名（内置代理为空）                                 |
| `password`     | string   | 认证密码（内置代理为空）                                   |
| `topic_prefix` | string   | 设备的主题前缀（参见 [MQTT 主题格式](#mqtt-主题格式)）      |

**向前兼容性**: 未知的 `type` 值必须被静默忽略。这允许协议在未来版本中扩展而不破坏旧固件。

**配置缓冲**: 平台通常先写入网络配置，再写入 MQTT 配置。设备应缓冲所有配置，仅在收到 Apply 命令时才应用。在 Apply 触发之前，不要尝试连接网络或代理。

---

### 4. Status（状态，读 + 通知）

**UUID**: `9e5d1e4c-5b13-4c4f-85b3-d0e6f5a7b8c9`

通过状态机报告配网进度。平台在发送 Apply 命令后订阅此特征值的通知。

#### 状态机

```
idle --> net_connecting --> net_connected --> mqtt_connecting --> done
  |                                                                         ^
  +-------> failed <--------------------------------------------------------+
```

设备按顺序经过各状态。任何状态在出错时可转为 `failed`。到达 `done` 后，设备配网完成并已连接到 MQTT 代理。

#### 通知负载示例

**正在连接网络**:
```json
{"step": "net_connecting"}
```

**网络已连接**（包含获取的 IP 和网络类型）:
```json
{"step": "net_connected", "ip": "192.168.1.42", "net_type": "wifi"}
```

**正在连接 MQTT**:
```json
{"step": "mqtt_connecting"}
```

**配网完成**:
```json
{"step": "done"}
```

**配网失败**（包含错误码）:
```json
{"step": "failed", "error": "wifi_timeout"}
```

#### 错误码

| 错误码                | 说明                          |
|---------------------|-------------------------------|
| `wifi_timeout`      | WiFi 连接超时                  |
| `wifi_auth_failed`  | WiFi 认证失败（密码错误）       |
| `cat1_no_sim`       | 未检测到 SIM 卡                |
| `cat1_no_signal`    | 无蜂窝信号                     |
| `mqtt_refused`      | MQTT 代理拒绝连接（认证失败）   |
| `mqtt_timeout`      | MQTT 连接超时                  |
| `unknown`           | 未指定错误                     |

**注意事项**:
- 平台使用这些状态更新向用户展示配网进度。
- `ip` 和 `net_type` 字段仅在 `net_connected` 步骤中出现。
- `error` 字段仅在 `failed` 步骤中出现。
- 随时读取此特征值应返回当前状态。

---

### 5. Apply（执行，写）

**UUID**: `9e5d1e4d-5b13-4c4f-85b3-d0e6f5a7b8c9`

触发设备提交所有已缓冲的配置并启动配网流程。

**写入负载**:

```json
{"action": "apply"}
```

收到此命令后，设备必须：

1. 验证已收到所有必要配置（网络 + MQTT）。
2. 将配置持久化到非易失性存储。
3. 将 Status 特征值从 `idle` 转为 `net_connecting`。
4. 尝试连接到已配置的网络。
5. 成功后，转为 `net_connected`。
6. 尝试连接 MQTT 代理。
7. 成功后，依次转为 `mqtt_connecting` 和 `done`。
8. 任何失败时，转为 `failed` 并附带相应的错误码。

**重要提示**: 收到 Apply 后，设备不应再接受 Config 写入，直到配网尝试完成（`done` 或 `failed`）。进入 `failed` 状态后，设备应恢复接受新的 Config 写入以允许重试。

---

## 配网流程

### 时序图

```
平台 (BLE Central)                        设备 (BLE Peripheral)
========================                  ======================
    |                                              |
    |  <-- 广播包（服务 UUID）------------------- |
    |                                              |
    |  ---- BLE 连接 ---------------------------> |
    |                                              |
    |  ---- 配对 (Secure Connections) ----------> |
    |  <--- 配对完成 ---------------------------- |
    |                                              |
    |  ---- 读取 Device Info ------------------>  |
    |  <--- {"model":"NE101","sn":"NE101-..."} --- |
    |                                              |
    |  ====== (平台调用 NeoMind API) ============ |
    |  POST /api/devices/ble-provision             |
    |  --> 获取 mqtt_config                        |
    |  ==========================================  |
    |                                              |
    |  ---- 写入 Network Scan (wifi) ---------->   |
    |  <--- 通知: 扫描结果 ----------------------  |
    |                                              |
    |  ---- 写入 Config (net_wifi) ------------>   |
    |  <--- 写入确认 ---------------------------- |
    |                                              |
    |  ---- 写入 Config (mqtt) ---------------->   |
    |  <--- 写入确认 ---------------------------- |
    |                                              |
    |  ---- 写入 Apply -------------------------> |
    |                                              |
    |  <--- 通知: {"step":"net_connecting"} ----- |
    |         ...（用户看到"正在连接..."）           |
    |  <--- 通知: {"step":"net_connected"} ------ |
    |  <--- 通知: {"step":"mqtt_connecting"} ---- |
    |  <--- 通知: {"step":"done"} --------------- |
    |                                              |
    |  ---- BLE 断开连接 -----------------------> |
    |                                              |
    |         设备已通过 MQTT 上线                   |
```

### 步骤说明

1. **BLE 发现**: 平台扫描广播服务 UUID `9e5d1e47-...` 的 BLE 设备。设备名称格式为 `{型号}-{序列号后缀}`（如 `NE101-A2F003`）。

2. **连接**: 用户从扫描结果中选择设备。平台建立 BLE 连接。

3. **配对**: 平台发起 BLE Secure Connections（LE Secure Connections 配对）。这是写入 Config 特征值之前的必要步骤。

4. **读取设备信息**: 平台读取 Device Info 特征值，获取型号、序列号、固件版本和支持的网络模块。

5. **API 预注册**: 平台调用 NeoMind REST API 注册设备并获取 MQTT 代理配置。

   **API 端点**: `POST /api/devices/ble-provision`

   **请求体**:
   ```json
   {
     "model": "NE101",
     "sn": "NE101-A2F003",
     "device_type": "ne101_camera",
     "device_name": "门口摄像头",
     "broker_id": "embedded"
   }
   ```

   **响应**:
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

6. **网络扫描**（可选）: 如果用户需要选择 WiFi 网络，平台通过 Network Scan 特征值触发扫描。

7. **写入网络配置**: 平台将用户选择的网络凭据写入 Config 特征值。每次配网会话只应写入一种网络类型配置。

8. **写入 MQTT 配置**: 平台将 MQTT 代理配置（来自 API 响应）写入 Config 特征值。

9. **执行**: 平台写入 Apply 命令。设备开始连接。

10. **状态监控**: 平台订阅 Status 通知并展示进度。收到 `done` 时设备已上线。收到 `failed` 时向用户显示错误，可重试。

---

## 设备 ID 生成

设备 ID 由序列号通过以下确定性公式派生：

```
device_id = sn.to_lowercase().replace("-", "_")
```

**示例**:

| 序列号            | 设备 ID          |
|------------------|------------------|
| `NE101-A2F003`   | `ne101_a2f003`   |
| `NE301-B1C042`   | `ne301_b1c042`   |

此转换确保在 BLE 配网流程和 NeoMind 平台之间保持一致的设备标识。

---

## MQTT 主题格式

配网完成后，设备使用以下主题结构与 NeoMind MQTT 代理通信：

| 方向           | 主题模式                                    | 说明          |
|---------------|---------------------------------------------|--------------|
| 设备 -> 平台  | `device/{device_type}/{device_id}/uplink`   | 遥测数据       |
| 平台 -> 设备  | `device/{device_type}/{device_id}/downlink` | 命令和控制     |

**示例主题**：类型为 `ne101_camera` 的设备 `ne101_a2f003`：

- 上行: `device/ne101_camera/ne101_a2f003/uplink`
- 下行: `device/ne101_camera/ne101_a2f003/downlink`

MQTT 配置中提供的 `topic_prefix` 为 `device/{device_type}/{device_id}`。设备在发布时附加 `/uplink`，订阅 `{topic_prefix}/downlink` 接收命令。

### MQTT 上行负载格式

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

## 型号到设备类型的映射

平台使用 Device Info 中的 `model` 字段确定设备类型模板。

| 型号     | 设备类型          | 说明         |
|---------|-------------------|-------------|
| `NE101` | `ne101_camera`    | NE101 摄像头 |
| `NE301` | `ne301_camera`    | NE301 摄像头 |

设备类型模板必须在配网前已在 NeoMind 中注册。如果模板不存在，`POST /api/devices/ble-provision` API 调用将返回 400 错误。

---

## BLE 广播要求

### 广播数据

设备必须在广播数据中包含以下内容：

1. **完整本地名称**: `{型号}-{序列号后缀}`（如 `NE101-A2F003`）
   - `型号` 为设备型号（如 `NE101`）
   - `序列号后缀` 为序列号的最后部分（如 `A2F003`）

2. **服务 UUID**: `9e5d1e47-5b13-4c4f-85b3-d0e6f5a7b8c9` 必须包含在广播包中，以便平台过滤设备。

### 广播参数

- **间隔**: 建议 100ms - 500ms（间隔越短发现越快，但功耗越高）
- **可连接**: 必须为可连接模式 (ADV_IND)
- **超时**: 无超时；持续广播直到被连接或配网完成

### 扫描响应数据

建议在扫描响应中包含：

- **完整本地名称**（如果广播数据中未完整包含）
- **发射功率等级**

---

## 安全

### BLE Secure Connections

配网协议使用 BLE Secure Connections（LE Secure Connections 配对）来保护传输到设备的配置数据。

**要求**:

1. **配对方式**: LE Secure Connections（非传统配对）
2. **加密要求**: Config 特征值 (`...1e4a`) 必须拒绝未加密链路上的写入
3. **认证方式**: 根据设备的 I/O 能力，使用 Just Works 或 Passkey 配对

**实现指南**:

- 在 ESP-IDF 中，启用 `CONFIG_BT_NIMBLE_SM_SC`（Secure Connections）并将 `CONFIG_BT_NIMBLE_SM_LVL_REQ` 设为至少 2（加密）。
- 设备应在处理 Config 写入前检查链路是否已加密。如果链路未加密，返回 `ATT_ERR_INSUFFICIENT_ENCRYPTION` (0x0F) 或 `ATT_ERR_INSUFFICIENT_AUTHEN` (0x0E)。
- Device Info 和 Network Scan 可以在未加密时访问，以允许在配对前进行初始设备发现和网络列表获取。

### 配置保护

- 网络凭据（WiFi 密码、APN 凭据）和 MQTT 凭据通过加密的 Config 特征值传输。
- 配网完成后，设备应将凭据存储在加密闪存中（如 ESP-IDF NVS 加密），而非明文存储。

---

## ESP-IDF 实现指南

本节提供在 ESP-IDF（ESP32 系列）上实现 BLE 配网 GATT 服务端的指导。

### 前置条件

- ESP-IDF v5.0 或更高版本
- NimBLE BLE 协议栈（推荐，内存占用低于 Bluedroid）
- `esp_wifi` 组件用于 WiFi 连接
- `esp_mqtt` 组件用于 MQTT 通信

### sdkconfig 默认配置

```
# BLE 配置
CONFIG_BT_ENABLED=y
CONFIG_BT_NIMBLE_ENABLED=y
CONFIG_BT_NIMBLE_SM_SC=y
CONFIG_BT_NIMBLE_SM_LVL_REQ=2

# NVS 加密（推荐）
CONFIG_NVS_ENCRYPTION=y
```

### GATT 服务注册（伪代码）

```c
#include "host/ble_hs.h"
#include "host/ble_gap.h"

#define SERVICE_UUID \
    ((ble_uuid128_t[]) {{ .u = { .type = BLE_UUID_TYPE_128 }, \
        .value = { 0xc9, 0xb8, 0xa7, 0xf5, 0xe6, 0xd0, 0xb3, 0x85, \
                   0x4f, 0x4c, 0x13, 0x5b, 0x47, 0x1e, 0x5d, 0x9e }}})

// 特征值 UUID 共享相同的基础值；仅第 14 字节不同
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
                // Device Info（读）
                .uuid = &CHAR_UUID(CHAR_UUID_DEVICE_INFO)->u,
                .access_cb = device_info_access_cb,
                .flags = BLE_GATT_CHR_F_READ,
            },
            {
                // Network Scan（写 + 通知）
                .uuid = &CHAR_UUID(CHAR_UUID_NETWORK_SCAN)->u,
                .access_cb = network_scan_access_cb,
                .flags = BLE_GATT_CHR_F_WRITE | BLE_GATT_CHR_F_NOTIFY,
            },
            {
                // Config（写，需加密）
                .uuid = &CHAR_UUID(CHAR_UUID_CONFIG)->u,
                .access_cb = config_access_cb,
                .flags = BLE_GATT_CHR_F_WRITE |
                         BLE_GATT_CHR_F_WRITE_AUTHEN,  // 要求加密
            },
            {
                // Status（读 + 通知）
                .uuid = &CHAR_UUID(CHAR_UUID_STATUS)->u,
                .access_cb = status_access_cb,
                .flags = BLE_GATT_CHR_F_READ | BLE_GATT_CHR_F_NOTIFY,
            },
            {
                // Apply（写）
                .uuid = &CHAR_UUID(CHAR_UUID_APPLY)->u,
                .access_cb = apply_access_cb,
                .flags = BLE_GATT_CHR_F_WRITE,
            },
            { 0 }  // 结束标记
        },
    },
    { 0 }  // 结束标记
};
```

### Device Info 处理函数

```c
static int device_info_access_cb(uint16_t conn_handle, uint16_t attr_handle,
                                  struct ble_gatt_access_ctxt *ctxt, void *arg)
{
    if (ctxt->op == BLE_GATT_ACCESS_OP_READ_CHR) {
        // 构建 JSON 响应
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

### Config 处理函数（含加密检查）

```c
static int config_access_cb(uint16_t conn_handle, uint16_t attr_handle,
                             struct ble_gatt_access_ctxt *ctxt, void *arg)
{
    if (ctxt->op == BLE_GATT_ACCESS_OP_WRITE_CHR) {
        // 验证加密状态
        struct ble_gap_conn_desc desc;
        ble_gap_conn_find(conn_handle, &desc);
        if (!desc.sec_state.encrypted) {
            return BLE_ATT_ERR_INSUFFICIENT_AUTHEN;
        }

        // 从写入数据解析 JSON
        char buf[256] = {0};
        uint16_t om_len = OS_MBUF_PKTLEN(ctxt->om);
        os_mbuf_copydata(ctxt->om, 0, om_len > 255 ? 255 : om_len, buf);

        // 解析 type 字段并分发处理
        cJSON *root = cJSON_Parse(buf);
        cJSON *type = cJSON_GetObjectItem(root, "type");

        if (strcmp(type->valuestring, "net_wifi") == 0) {
            // 存储 WiFi 配置
            store_wifi_config(
                cJSON_GetObjectItem(root, "ssid")->valuestring,
                cJSON_GetObjectItem(root, "password")->valuestring
            );
        } else if (strcmp(type->valuestring, "mqtt") == 0) {
            // 存储 MQTT 配置
            store_mqtt_config(
                cJSON_GetObjectItem(root, "host")->valuestring,
                cJSON_GetObjectItem(root, "port")->valueint,
                cJSON_GetObjectItem(root, "username")->valuestring,
                cJSON_GetObjectItem(root, "password")->valuestring,
                cJSON_GetObjectItem(root, "topic_prefix")->valuestring
            );
        }
        // 静默忽略未知类型以保持向前兼容性

        cJSON_Delete(root);
        return 0;
    }
    return BLE_ATT_ERR_UNLIKELY;
}
```

### Status 通知辅助函数

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

### Apply 处理函数与状态机

```c
static void provisioning_task(void *arg)
{
    // 步骤 1：连接网络
    notify_status("net_connecting", NULL, NULL, NULL);

    if (!wifi_connect()) {
        notify_status("failed", "wifi_timeout", NULL, NULL);
        return;
    }

    char ip[16];
    get_ip_address(ip);
    notify_status("net_connected", NULL, ip, "wifi");

    // 步骤 2：连接 MQTT 代理
    notify_status("mqtt_connecting", NULL, NULL, NULL);

    if (!mqtt_connect()) {
        notify_status("failed", "mqtt_refused", NULL, NULL);
        return;
    }

    notify_status("done", NULL, NULL, NULL);

    // 设备已上线；BLE 可以断开
    ble_gap_terminate(status_conn_handle, BLE_ERR_REM_USER_CONN_TERM);
}
```

### 广播配置

```c
static void start_advertising(void)
{
    struct ble_gap_adv_params adv_params;
    struct ble_hs_adv_fields fields;

    memset(&fields, 0, sizeof(fields));

    // 在广播数据中包含服务 UUID
    fields.uuids128 = SERVICE_UUID;
    fields.num_uuids128 = 1;
    fields.uuids128_is_complete = 1;

    // 包含设备名称
    fields.name = (const uint8_t *)"NE101-A2F003";
    fields.name_len = strlen("NE101-A2F003");
    fields.name_is_complete = 1;

    ble_gap_adv_set_fields(&fields);

    memset(&adv_params, 0, sizeof(adv_params));
    adv_params.conn_mode = BLE_GAP_CONN_MODE_UND;  // 可连接
    adv_params.disc_mode = BLE_GAP_DISC_MODE_GEN;  // 通用可发现
    adv_params.itvl_min = 0x00A0;  // 100ms
    adv_params.itvl_max = 0x01A0;  // 260ms

    ble_gap_adv_start(BLE_OWN_ADDR_PUBLIC, NULL, BLE_HS_FOREVER,
                      &adv_params, gap_event_handler, NULL);
}
```

---

## 错误处理

### BLE 层错误

设备应返回适当的 BLE ATT 错误码：

| 场景                                  | 错误码                            |
|--------------------------------------|----------------------------------|
| 未加密时写入 Config                    | `0x0E` (Insufficient Authentication) |
| 写入只读特征值                         | `0x06` (Request Not Supported)   |
| 写入数据包含格式错误的 JSON             | `0x0D` (Invalid Attribute Value)  |
| 特征值未找到                           | `0x0A` (Attribute Not Found)      |

### 配网重试

如果配网失败（Status 报告 `failed`）：

1. 平台向用户显示错误信息。
2. 用户可以更新配置（如更正 WiFi 密码）。
3. 平台写入新的 Config 数据。
4. 平台再次写入 Apply。
5. 设备必须将状态机重置为 `idle` 后再接受新的 Apply。

### 超时建议

| 操作             | 超时时间   |
|-----------------|-----------|
| BLE 连接         | 30 秒     |
| 配对             | 60 秒     |
| 网络扫描         | 15 秒     |
| WiFi 连接        | 30 秒     |
| CAT.1 连接       | 60 秒     |
| MQTT 连接        | 15 秒     |

---

## 附录：JSON 消息快速参考

### 平台写入的消息

| 特征值          | 方向  | 负载                                                            |
|----------------|------|----------------------------------------------------------------|
| Network Scan   | 写入 | `{"type":"wifi"}`                                              |
| Network Scan   | 写入 | `{"type":"cat1_status"}`                                       |
| Network Scan   | 写入 | `{"type":"halow"}`                                             |
| Config         | 写入 | `{"type":"net_wifi","ssid":"...","password":"..."}`            |
| Config         | 写入 | `{"type":"net_cat1","apn":"...","user":"","password":"","pin":"","auth_type":0}` |
| Config         | 写入 | `{"type":"net_halow","ssid":"...","password":"..."}`           |
| Config         | 写入 | `{"type":"mqtt","host":"...","port":1883,"username":"","password":"","topic_prefix":"..."}` |
| Apply          | 写入 | `{"action":"apply"}`                                           |

### 设备返回的消息

| 特征值          | 方向  | 负载                                                            |
|----------------|------|----------------------------------------------------------------|
| Device Info    | 读取 | `{"model":"NE101","sn":"NE101-A2F003","fw":"1.0.0","netmod":"","supported_netmods":["wifi"]}` |
| Network Scan   | 通知 | `[{"ssid":"...","rssi":-45,"auth":true,"channel":6}]`          |
| Network Scan   | 通知 | `{"sim_ready":true,"signal_level":"good","signal_dbm":-75,...}`|
| Status         | 通知 | `{"step":"net_connecting"}`                                    |
| Status         | 通知 | `{"step":"net_connected","ip":"192.168.1.42","net_type":"wifi"}` |
| Status         | 通知 | `{"step":"mqtt_connecting"}`                                   |
| Status         | 通知 | `{"step":"done"}`                                              |
| Status         | 通知 | `{"step":"failed","error":"wifi_timeout"}`                     |
