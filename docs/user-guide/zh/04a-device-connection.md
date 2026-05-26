# 设备连接技术参考

> 协议层面的设备连接参考文档。设备管理相关内容（类型、仪表盘、命令）请参阅[设备管理](./04-devices.md)。

## 连接方式概览

| 连接方式 | 端口 | 协议 | 适用场景 |
|----------|------|------|----------|
| **MQTT** | 1883（TCP）/ 8883（TLS） | MQTT 3.1.1 / 5.0 | 持久连接、实时遥测、双向控制 |
| **Webhook** | 9375 | HTTP POST | 脚本、云服务、电池供电设备、间歇性数据上报 |
| **BLE 配网** | -- | Bluetooth LE | NE301/NE101 摄像头的一次性 WiFi/MQTT 配置 |

---

## MQTT

### 代理配置

NeoMind 内嵌 MQTT 代理，无需外部代理。

```
默认端口：       1883
协议：           MQTT 3.1.1 和 5.0
认证：           无（局域网默认配置）
最大连接数：     1000 个并发客户端
最大负载：       256 KB
WebSocket：      端口 8083（可选）
```

生产环境建议在**设置 > MQTT** 中配置外部代理。

验证命令：`neomind system info` 或 `nc -zv 192.168.1.100 1883`

### 主题结构

```
上行（遥测数据）：  device/{device_type}/{device_id}/uplink
下行（命令）：      device/{device_type}/{device_id}/downlink
```

示例：`device/sensor/temp-living-01/uplink`、`device/camera/cam-gh-01/downlink`

### 数据格式

JSON 格式，键名需与设备类型定义中的指标名一致：

```json
{"temperature": 23.5, "humidity": 65.0}
```

可选的 `timestamp` 字段（`"timestamp": 1716200000`）。如果省略，NeoMind 使用服务器接收时间。值必须是 JSON 数字或布尔类型，不能是字符串。支持 QoS 0 或 1。

### 接收命令

订阅下行主题。NeoMind 以 JSON 格式发布命令：`{"action": "set_state", "state": "on"}`。在设备端本地处理 `action` 字段，并可选择在上行主题上回复确认。

---

## Webhook

### URL 和请求格式

```
POST http://{server}:9375/api/devices/{device_id}/webhook
Authorization: Bearer {device_token}
Content-Type: application/json

{"temperature": 25.5, "humidity": 60}
```

使用设备详情页中的设备令牌。API Key 也可以使用，但权限范围更广。

| 状态码 | 含义 |
|--------|------|
| `200` | 成功——遥测数据已存储 |
| `401` | 令牌无效或缺失 |
| `404` | 设备未找到 |
| `429` | 请求频率超限（每个设备 60 次/分钟） |

电池供电或间歇性上报的设备建议使用 **Webhook**。高频数据和双向控制建议使用 **MQTT**。

---

## TLS / mTLS

生产环境下的 MQTT 安全配置。在**设置 > MQTT** 中配置。

| 模式 | 证书 | 用途 |
|------|------|------|
| **TLS** | 服务端证书 + CA 证书 | 加密通信 |
| **mTLS** | 服务端证书 + 客户端证书 | 双向认证 |

```python
import ssl, paho.mqtt.client as mqtt
client = mqtt.Client()
ctx = ssl.create_default_context()
ctx.load_verify_locations("/path/to/ca.crt")
ctx.load_cert_chain(certfile="/path/to/client.crt", keyfile="/path/to/client.key")
client.tls_set_context(ctx)
client.connect("neomind.example.com", 8883)
```

---

## BLE 配网

通过 NeoMind 桌面应用或 API 为 NE301/NE101 摄像头进行 WiFi/MQTT 配置。

`POST /api/devices/ble-provision`

```json
{"model": "NE101", "sn": "NE101-A2F003", "device_type": "ne101_camera",
 "device_name": "Front Door Camera", "broker_id": "embedded", "resolve_only": false}
```

设置 `resolve_only: true` 仅生成配置而不写入设备。

---

## 完整代码示例

### 1. Python MQTT（paho-mqtt）

```python
import paho.mqtt.client as mqtt
import json, time, random, signal, sys, logging

# -- Configuration --
MQTT_BROKER = "192.168.1.100"
MQTT_PORT = 1883
DEVICE_TYPE = "sensor"
DEVICE_ID = "py-sensor-01"
TELEMETRY_TOPIC = f"device/{DEVICE_TYPE}/{DEVICE_ID}/uplink"
COMMAND_TOPIC = f"device/{DEVICE_TYPE}/{DEVICE_ID}/downlink"
REPORT_INTERVAL = 10

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
log = logging.getLogger("device")
running = True

def on_connect(client, userdata, flags, rc, properties=None):
    if rc == 0:
        log.info("Connected to broker")
        client.subscribe(COMMAND_TOPIC, qos=1)
    else:
        log.error("Connection failed: rc=%d", rc)

def on_message(client, userdata, msg):
    try:
        cmd = json.loads(msg.payload.decode())
        log.info("Command: %s", cmd)
        if cmd.get("action") == "ping":
            client.publish(TELEMETRY_TOPIC, json.dumps({"pong": True}), qos=1)
        elif cmd.get("action") == "reboot":
            sys.exit(0)
    except Exception as e:
        log.error("Command error: %s", e)

def on_disconnect(client, userdata, flags, rc, properties=None):
    if rc != 0:
        log.warning("Unexpected disconnect (rc=%d), auto-reconnecting", rc)

signal.signal(signal.SIGINT, lambda s, f: globals().update(running=False))

client = mqtt.Client(client_id=DEVICE_ID, protocol=mqtt.MQTTv311)
client.on_connect = on_connect
client.on_message = on_message
client.on_disconnect = on_disconnect
client.reconnect_delay_set(min_delay=1, max_delay=30)
client.connect(MQTT_BROKER, MQTT_PORT, keepalive=60)
client.loop_start()

try:
    while running:
        payload = {
            "temperature": round(20 + random.uniform(-2, 5), 1),
            "humidity": round(50 + random.uniform(-10, 20), 1),
            "timestamp": int(time.time()),
        }
        client.publish(TELEMETRY_TOPIC, json.dumps(payload), qos=1)
        log.info("Published: %s", payload)
        time.sleep(REPORT_INTERVAL)
finally:
    client.loop_stop()
    client.disconnect()
```

### 2. Python Webhook（requests）

```python
import requests, time, random, logging

API_URL = "http://192.168.1.100:9375/api/devices/my-sensor/webhook"
TOKEN = "YOUR_DEVICE_TOKEN"

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
log = logging.getLogger("webhook")

def send_telemetry(data, retries=3):
    headers = {"Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"}
    for attempt in range(retries):
        try:
            resp = requests.post(API_URL, headers=headers, json=data, timeout=10)
            if resp.status_code == 200:
                log.info("Sent: %s", data)
                return True
            elif resp.status_code in (401, 404):
                log.error("HTTP %d: %s", resp.status_code, resp.text)
                return False
            else:
                time.sleep(2 ** attempt)
        except requests.exceptions.RequestException as e:
            log.error("Request error: %s (attempt %d/%d)", e, attempt + 1, retries)
            time.sleep(2 ** attempt)
    return False

if __name__ == "__main__":
    while True:
        send_telemetry({
            "temperature": round(20 + random.uniform(-2, 5), 1),
            "humidity": round(50 + random.uniform(-10, 20), 1),
        })
        time.sleep(10)
```

### 3. ESP32 Arduino（PubSubClient + DHT22）

```cpp
#include <WiFi.h>
#include <PubSubClient.h>
#include <DHT.h>
#include <ArduinoJson.h>

const char* WIFI_SSID = "YOUR_WIFI_SSID";
const char* WIFI_PASS = "YOUR_WIFI_PASSWORD";
const char* MQTT_SERVER = "192.168.1.100";
const int MQTT_PORT = 1883;
const char* DEVICE_ID = "esp32-dht22-01";
const char* TELEMETRY_TOPIC = "device/sensor/esp32-dht22-01/uplink";
const char* COMMAND_TOPIC = "device/sensor/esp32-dht22-01/downlink";

#define DHT_PIN 4
#define DHT_TYPE DHT22
DHT dht(DHT_PIN, DHT_TYPE);

WiFiClient espClient;
PubSubClient mqtt(espClient);
unsigned long lastReport = 0;
const unsigned long REPORT_MS = 10000;

void connectWiFi() {
    Serial.print("Connecting WiFi");
    WiFi.begin(WIFI_SSID, WIFI_PASS);
    for (int i = 0; i < 40 && WiFi.status() != WL_CONNECTED; i++) {
        delay(500); Serial.print(".");
    }
    Serial.println(WiFi.status() == WL_CONNECTED ? " OK" : " FAILED");
}

void connectMQTT() {
    while (!mqtt.connected()) {
        Serial.print("MQTT connect...");
        if (mqtt.connect(DEVICE_ID)) {
            Serial.println(" OK");
            mqtt.subscribe(COMMAND_TOPIC, 1);
        } else {
            Serial.printf(" fail (rc=%d), retry 5s\n", mqtt.state());
            delay(5000);
        }
    }
}

void onCommand(char* topic, byte* payload, unsigned int len) {
    StaticJsonDocument<256> doc;
    if (deserializeJson(doc, payload, len)) return;
    const char* action = doc["action"] | "";
    if (strcmp(action, "reboot") == 0) ESP.restart();
}

void publishTelemetry() {
    float t = dht.readTemperature(), h = dht.readHumidity();
    if (isnan(t) || isnan(h)) { Serial.println("DHT read error"); return; }
    StaticJsonDocument<128> doc;
    doc["temperature"] = round(t * 10.0) / 10.0;
    doc["humidity"] = round(h * 10.0) / 10.0;
    char buf[128]; serializeJson(doc, buf);
    mqtt.publish(TELEMETRY_TOPIC, buf, false);
    Serial.println(buf);
}

void setup() {
    Serial.begin(115200);
    dht.begin();
    connectWiFi();
    mqtt.setServer(MQTT_SERVER, MQTT_PORT);
    mqtt.setCallback(onCommand);
    mqtt.setBufferSize(512);
}

void loop() {
    if (WiFi.status() != WL_CONNECTED) connectWiFi();
    if (!mqtt.connected()) connectMQTT();
    mqtt.loop();
    if (millis() - lastReport >= REPORT_MS) { lastReport = millis(); publishTelemetry(); }
}
```

### 4. Node.js MQTT（mqtt.js）

```javascript
const mqtt = require('mqtt');

const DEVICE_ID = 'node-sensor-01';
const DEVICE_TYPE = 'sensor';
const TELEMETRY_TOPIC = `device/${DEVICE_TYPE}/${DEVICE_ID}/uplink`;
const COMMAND_TOPIC = `device/${DEVICE_TYPE}/${DEVICE_ID}/downlink`;

const client = mqtt.connect('mqtt://192.168.1.100:1883', {
  clientId: DEVICE_ID, clean: true, keepalive: 60, reconnectPeriod: 5000,
});

client.on('connect', () => {
  console.log('Connected to broker');
  client.subscribe(COMMAND_TOPIC, { qos: 1 });
});

client.on('message', (topic, msg) => {
  try {
    const cmd = JSON.parse(msg.toString());
    console.log('Command:', cmd);
    if (cmd.action === 'ping') {
      client.publish(TELEMETRY_TOPIC, JSON.stringify({ pong: true }), { qos: 1 });
    }
  } catch (e) { console.error('Parse error:', e.message); }
});

client.on('error', (err) => console.error('MQTT error:', err.message));
client.on('offline', () => console.warn('Offline, reconnecting...'));

setInterval(() => {
  const data = {
    temperature: +(20 + Math.random() * 5).toFixed(1),
    humidity: +(50 + Math.random() * 20).toFixed(1),
    timestamp: Math.floor(Date.now() / 1000),
  };
  client.publish(TELEMETRY_TOPIC, JSON.stringify(data), { qos: 1 }, (err) => {
    if (err) console.error('Publish failed:', err);
    else console.log('Published:', data);
  });
}, 10000);

process.on('SIGINT', () => { client.end(false, () => process.exit(0)); });
```

### 5. curl Webhook 测试

单次遥测数据推送：

```bash
curl -X POST http://192.168.1.100:9375/api/devices/my-sensor/webhook \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"temperature": 25.5, "humidity": 60}'
```

通过 crontab 定时执行（每 5 分钟）：

```bash
*/5 * * * * /home/pi/report-weather.sh >> /home/pi/weather.log 2>&1
```

---

## 诊断命令

```bash
neomind system info                                           # 代理 + 网络状态
nc -zv 192.168.1.100 1883                                    # 测试代理端口
mosquitto_sub -h 192.168.1.100 -t "device/#" -v              # 监控所有流量
mosquitto_pub -h 192.168.1.100 -t "device/sensor/test/uplink" -m '{"temperature":25.5}'
curl -s http://192.168.1.100:9375/api/setup/status           # API 健康检查
```

## 故障排查

| 症状 | 解决方法 |
|------|----------|
| MQTT 连接被拒绝 | 运行 `neomind system info`，检查防火墙是否开放 1883 端口 |
| 仪表盘无数据 | 比对 JSON 键名与设备类型的指标名是否一致 |
| 频繁断连 | 将 keepalive 设为 60 秒，增加重连逻辑 |
| TLS 握手失败 | 确认所有证书均为 PEM 格式 |
| Webhook 返回 401 | 检查请求头格式：`Authorization: Bearer TOKEN` |
| Webhook 返回 429 | 每个设备上限 60 次/分钟 |
| BLE 找不到设备 | 长按配对按钮 5 秒，直到 LED 闪烁 |
| 自动发现失败 | 主题必须严格匹配 `device/{type}/{id}/uplink` 格式 |

[< 返回设备管理](./04-devices.md) | [目录](./README.md) | [下一篇：自动化 >](./05-automation.md)
