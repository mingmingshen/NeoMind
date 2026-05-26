# Device Connection Technical Reference

> Protocol-level reference for connecting devices to NeoMind. For device management (types, dashboards, commands), see [Device Management](./04-devices.md).

## Connection Methods

| Method | Port | Protocol | Best For |
|--------|------|----------|----------|
| **MQTT** | 1883 (TCP) / 8883 (TLS) | MQTT 3.1.1 / 5.0 | Persistent connections, real-time telemetry, bidirectional control |
| **Webhook** | 9375 | HTTP POST | Scripts, cloud services, battery-powered devices, intermittent data |
| **BLE Provisioning** | -- | Bluetooth LE | One-time WiFi/MQTT setup for NE301/NE101 cameras |

---

## MQTT

### Broker Configuration

NeoMind includes an embedded MQTT broker. No external broker is required.

```
Default port:     1883
Protocol:         MQTT 3.1.1 and 5.0
Auth:             None (local network default)
Max connections:  1000 concurrent clients
Max payload:      256 KB
WebSocket:        Port 8083 (optional)
```

For production, configure an external broker under **Settings > MQTT**.

Verify: `neomind system info` or `nc -zv 192.168.1.100 1883`

### Topic Structure

```
Uplink (telemetry):   device/{device_type}/{device_id}/uplink
Downlink (commands):  device/{device_type}/{device_id}/downlink
```

Examples: `device/sensor/temp-living-01/uplink`, `device/camera/cam-gh-01/downlink`

### Payload Format

JSON with metric keys matching the device type definition:

```json
{"temperature": 23.5, "humidity": 65.0}
```

Optional timestamp field (`"timestamp": 1716200000`). If omitted, NeoMind uses server reception time. Values must be JSON numbers/booleans, not strings. QoS 0 or 1 supported.

### Receiving Commands

Subscribe to the downlink topic. NeoMind publishes JSON commands: `{"action": "set_state", "state": "on"}`. Handle the `action` field locally and optionally acknowledge on the uplink topic.

---

## Webhook

### URL and Request Format

```
POST http://{server}:9375/api/devices/{device_id}/webhook
Authorization: Bearer {device_token}
Content-Type: application/json

{"temperature": 25.5, "humidity": 60}
```

Use the device token from the device detail page. API keys also work but grant broader access.

| Status | Meaning |
|--------|---------|
| `200` | Success -- telemetry stored |
| `401` | Invalid or missing token |
| `404` | Device not found |
| `429` | Rate limited (60 req/min per device) |

Use **Webhook** for battery-powered or intermittent devices. Use **MQTT** for high-frequency data and bidirectional control.

---

## TLS / mTLS

Secure MQTT for production. Configure under **Settings > MQTT**.

| Mode | Certificates | Purpose |
|------|-------------|---------|
| **TLS** | Server cert + CA cert | Encrypt traffic |
| **mTLS** | Server + client certs | Mutual authentication |

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

## BLE Provisioning

WiFi/MQTT setup for NE301/NE101 cameras via the NeoMind desktop app or API.

`POST /api/devices/ble-provision`

```json
{"model": "NE101", "sn": "NE101-A2F003", "device_type": "ne101_camera",
 "device_name": "Front Door Camera", "broker_id": "embedded", "resolve_only": false}
```

Set `resolve_only: true` to generate config without writing to the device.

---

## Complete Code Examples

### 1. Python MQTT (paho-mqtt)

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

### 2. Python Webhook (requests)

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

### 3. ESP32 Arduino (PubSubClient + DHT22)

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

### 4. Node.js MQTT (mqtt.js)

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

### 5. curl Webhook Test

One-shot telemetry push:

```bash
curl -X POST http://192.168.1.100:9375/api/devices/my-sensor/webhook \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"temperature": 25.5, "humidity": 60}'
```

Scheduled via crontab (every 5 minutes):

```bash
*/5 * * * * /home/pi/report-weather.sh >> /home/pi/weather.log 2>&1
```

---

## Diagnostic Commands

```bash
neomind system info                                           # Broker + network status
nc -zv 192.168.1.100 1883                                    # Test broker port
mosquitto_sub -h 192.168.1.100 -t "device/#" -v              # Monitor all traffic
mosquitto_pub -h 192.168.1.100 -t "device/sensor/test/uplink" -m '{"temperature":25.5}'
curl -s http://192.168.1.100:9375/api/setup/status           # API health
```

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| MQTT connection refused | `neomind system info`, open port 1883 on firewall |
| No data in dashboard | Compare JSON keys with device type metric names |
| Frequent disconnects | Set keepalive to 60s, add reconnect logic |
| TLS handshake failure | All certs must be PEM-encoded |
| Webhook 401 | Must be `Authorization: Bearer TOKEN` |
| Webhook 429 | Max 60 req/min per device |
| BLE device not found | Hold pairing button 5s until LED flashes |
| Auto-discovery fails | Topic must match `device/{type}/{id}/uplink` exactly |

[< Back to Device Management](./04-devices.md) | [Index](./README.md) | [Next: Automation >](./05-automation.md)
