# NeoMind Examples Guide

**Version**: 0.8.0
**Last Updated**: 2026-05-26

## Table of Contents

1. [Extension Examples](#extension-examples)
   - [Virtual Metrics Extension](#virtual-metrics-extension)
   - [Event Monitor Extension](#event-monitor-extension)
   - [Virtual Weather Provider](#virtual-weather-provider)
   - [Device Helper Example](#device-helper-example)
2. [Capability Provider Examples](#capability-provider-examples)
   - [Device Capability Provider](#device-capability-provider)
   - [Runner Capability Provider](#runner-capability-provider)
3. [Data Push Configuration](#data-push-configuration)
4. [Notification Channel Setup](#notification-channel-setup)
5. [Using AI Chat to Manage Devices](#using-ai-chat-to-manage-devices)
6. [Creating Automation Rules](#creating-automation-rules)

---

## Extension Examples

### Virtual Metrics Extension

**Location**: `examples/virtual-metrics-extension/`

**ID**: `virtual-metrics`

**Version**: 0.1.0

#### Description

Virtual metrics extension example, demonstrating how to inject virtual metrics from external data sources into device telemetry.

**Scenario**: Simulate external data sources (e.g., APIs, databases, computed values) injecting data into devices.

#### Provided Metrics

- `injection_count` (Integer) - Count of injected virtual metrics

#### Provided Commands

1. **set_target_device**
   - Set target device ID for virtual metric injection
   - Parameters:
     - `device_id` (String, required) - Device ID

2. **inject_virtual_metrics**
   - Inject virtual metrics into target device
   - Parameters:
     - `metric_name` (String, required) - Metric name
     - `value` (Float, required) - Metric value

3. **get_injection_count**
   - Get count of injected virtual metrics
   - No parameters

#### Use Cases

- Learn how to create and inject virtual metrics
- Understand extension state management
- Demonstrate external data integration patterns

---

### Event Monitor Extension

**Location**: `examples/event-monitor-extension/`

**ID**: `event-monitor`

**Version**: 0.1.0

#### Description

Event monitor extension example, demonstrating how to subscribe to and respond to NeoMind system events.

**Scenario**: Listen to device events, rule events, agent events, etc., for statistics and analysis.

#### Provided Metrics

- `total_events` (Integer) - Total received events
- `device_events` (Integer) - Device-related events
- `rule_events` (Integer) - Rule-related events
- `agent_events` (Integer) - Agent-related events
- `last_event_type` (String) - Last event type
- `last_event_source` (String) - Last event source

#### Provided Commands

1. **get_stats**
   - Get event statistics
   - No parameters

2. **reset_stats**
   - Reset event statistics
   - No parameters

3. **set_filter**
   - Set event filter
   - Parameters:
     - `event_type` (String, optional) - Event type
     - `source` (String, optional) - Event source

4. **clear_filter**
   - Clear event filter
   - No parameters

#### Use Cases

- Learn event subscription mechanisms
- Understand event filtering and routing
- Demonstrate event-driven automation

---

### Virtual Weather Provider

**Location**: `examples/virtual-weather-provider/`

**ID**: `virtual-weather-provider`

**Version**: 0.1.0

#### Description

Virtual weather provider extension example that fetches weather data from the Open-Meteo API (free, no API key required) and injects it as virtual metrics into device telemetry.

**Scenario**: Inject real-world weather data into a smart home system for weather-based automation.

#### Provided Metrics

- `temperature` (Float, C) - Current temperature
- `humidity` (Float, %) - Current humidity
- `wind_speed` (Float, km/h) - Wind speed
- `weather_code` (Integer) - Weather code
- `last_update` (Integer) - Last update timestamp

#### Provided Commands

1. **set_location**
   - Set geographic location (latitude/longitude)
   - Parameters:
     - `latitude` (Float, required) - Latitude
     - `longitude` (Float, required) - Longitude

2. **update_weather**
   - Manually update weather data
   - No parameters

3. **inject_to_device**
   - Inject weather data into specified device
   - Parameters:
     - `device_id` (String, required) - Device ID

4. **get_current_weather**
   - Get current weather data
   - No parameters

5. **set_auto_update**
   - Set auto-update interval
   - Parameters:
     - `interval_minutes` (Integer, required) - Update interval (minutes)

#### Use Cases

- Integrate real weather data into IoT systems
- Learn how to fetch data from external APIs
- Understand virtual metrics in practice
- Demonstrate scheduled tasks and background updates

#### Notes

- Uses Open-Meteo API (free, no registration required)
- Network connection is required
- API has rate limits (typically 1000 requests per day)

---

### Device Helper Example

**Location**: `examples/device-helper-example/`

**ID**: `device-helper-example`

**Version**: 1.0.0

#### Description

DeviceHelper framework example, demonstrating how to use the type-safe DeviceHelper API to interact with devices.

**Scenario**: Teaching example showcasing all DeviceHelper framework features.

#### Provided Metrics

- `processed_count` (Integer) - Processed device count
- `avg_temperature` (Float, C) - Average temperature
- `virtual_outdoor_temp` (Float, C) - Virtual outdoor temperature

#### Provided Commands

1. **analyze_device**
   - Analyze device: read metrics, compute statistics, inject virtual metrics
   - Parameters:
     - `device_id` (String, required) - Device ID
   - Demonstrates:
     - Read all device metrics
     - Get metrics by specific type
     - Inject analysis results as virtual metrics
     - Batch read multiple metrics

2. **update_weather**
   - Update weather: inject weather data as virtual metrics
   - Parameters:
     - `device_id` (String, required) - Device ID
     - `temperature` (Float, optional, default 25.0) - Temperature
     - `humidity` (Float, optional, default 60.0) - Humidity
   - Demonstrates:
     - Batch write virtual metrics
     - Type-safe metric writing

3. **get_device_stats**
   - Get device statistics: query telemetry and compute aggregations
   - Parameters:
     - `device_id` (String, required) - Device ID
   - Demonstrates:
     - Query 24-hour telemetry history
     - Compute average and max aggregations

#### Use Cases

- **Learn** all DeviceHelper framework APIs
- **Understand** type-safe device interaction patterns
- **Reference** for developing your own extensions
- **Test** DeviceHelper features

#### API Coverage

- Read device metrics
- Write virtual metrics
- Send device commands
- Query telemetry history
- Aggregate metrics

---

## Capability Provider Examples

### Device Capability Provider

**Location**: `examples/device-capability-provider/`

#### Description

Device Capability Provider, providing device-related capabilities to extensions.

**Note**: This is not an extension, but a capability provider library.

#### Provided Capabilities

1. **DeviceMetricsRead** - Read device metrics
   - `get_current_metrics(device_id)` - Get current metrics
   - `get_metric(device_id, metric_name)` - Get single metric

2. **DeviceMetricsWrite** - Write device metrics (including virtual metrics)
   - `write_metric(device_id, metric, value, is_virtual)` - Write metric
   - `write_metrics(device_id, metrics)` - Batch write metrics

3. **DeviceControl** - Control device
   - `send_command(device_id, command, params)` - Send command

4. **TelemetryHistory** - Query telemetry history
   - `query_telemetry(device_id, metric, start, end)` - Query historical data

#### Use Cases

- Learn how to create custom capability providers
- Provide specific system capabilities to extensions
- Understand capability system architecture

---

### Runner Capability Provider

**Location**: `examples/runner-capability-provider/`

#### Description

Runner Capability Provider, providing capabilities to extensions (through direct access to core system services within the extension runner process).

**Note**: This is not an extension, but a capability provider library.

#### Provided Capabilities

More efficient API calls through direct access to core services:
- Device service
- Event bus
- Storage service
- Agent system
- Rule engine

#### Use Cases

- Learn how to create high-performance capability providers
- Provide capabilities inside the extension runner
- Understand extension runner internal architecture

---

## Data Push Configuration

The Data Push module allows you to configure push targets that deliver device telemetry data to external services on a schedule.

### CLI Examples

```bash
# List all push targets
neomind push list

# Create a webhook push target
neomind push create \
  --name "temperature-webhook" \
  --type webhook \
  --config '{"url":"https://example.com/api/telemetry","headers":{"Authorization":"Bearer token123"}}' \
  --schedule '{"type":"interval","interval_secs":60}' \
  --sources '{"source_patterns":["device:sensor1:temperature"],"only_changes":true}'

# Create an MQTT push target
neomind push create \
  --name "mqtt-broker" \
  --type mqtt \
  --config '{"broker":"mqtt://broker.example.com:1883","topic":"neomind/telemetry"}' \
  --schedule '{"type":"event_driven","event_types":["device_metric"]}'

# Test a push target
neomind push test <target-id>

# Start a push target
neomind push start <target-id>

# Stop a push target
neomind push stop <target-id>

# View delivery logs
neomind push logs <target-id> --limit 50

# View push statistics
neomind push stats

# Update a push target
neomind push update <target-id> --config '{"url":"https://new-url.example.com/hook"}'

# Delete a push target
neomind push delete <target-id>
```

### API Examples

```bash
# Create a webhook push target
curl -X POST http://localhost:9375/api/data-push \
  -H "Content-Type: application/json" \
  -d '{
    "name": "temperature-webhook",
    "target_type": "webhook",
    "config": {
      "url": "https://example.com/api/telemetry",
      "headers": {"Authorization": "Bearer token123"}
    },
    "schedule": {
      "type": "interval",
      "interval_secs": 60
    },
    "data_filter": {
      "source_patterns": ["device:sensor1:temperature"],
      "only_changes": true
    }
  }'

# List push targets
curl http://localhost:9375/api/data-push

# Test a push target
curl -X POST http://localhost:9375/api/data-push/<target-id>/test

# View delivery logs
curl "http://localhost:9375/api/data-push/<target-id>/logs?limit=20&offset=0"
```

### Push Target Types

| Type | Description | Config Fields |
|------|-------------|---------------|
| `webhook` | HTTP POST to external URL | `url`, `headers`, `method` |
| `mqtt` | Publish to MQTT broker | `broker`, `topic`, `username`, `password` |

### Schedule Types

| Type | Description |
|------|-------------|
| `interval` | Periodically pull latest data from time-series store |
| `event_driven` | Push immediately when matching data arrives via EventBus |

### Web UI

Navigate to **Data Explorer** (`/data`) and switch to the **Push Targets** tab to manage push targets visually.

---

## Notification Channel Setup

NeoMind supports 7 notification channel types for sending alerts and messages.

### CLI Examples

```bash
# List available channel types
neomind channel types

# List configured channels
neomind channel list

# Create a webhook channel
neomind channel create \
  --name "alert-webhook" \
  --type webhook \
  --config '{"url":"https://hooks.example.com/alert","method":"POST"}'

# Create a DingTalk robot channel
neomind channel create \
  --name "team-alerts" \
  --type dingtalk \
  --config '{"webhook_url":"https://oapi.dingtalk.com/robot/send?access_token=xxx","secret":"your-secret"}'

# Create a WeCom robot channel
neomind channel create \
  --name "ops-alerts" \
  --type wecom \
  --config '{"webhook_url":"https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx"}'

# Create an email channel
neomind channel create \
  --name "email-alerts" \
  --type email \
  --config '{"smtp_host":"smtp.example.com","smtp_port":587,"from":"neo@example.com","to":["admin@example.com"],"username":"neo","password":"pass"}'

# Test a channel
neomind channel test --name "alert-webhook"

# Enable/disable a channel
neomind channel enable --name "alert-webhook"
neomind channel disable --name "alert-webhook"

# Delete a channel
neomind channel delete --name "alert-webhook"
```

### API Examples

```bash
# List channels
curl http://localhost:9375/api/messages/channels

# Create a webhook channel
curl -X POST http://localhost:9375/api/messages/channels \
  -H "Content-Type: application/json" \
  -d '{
    "name": "alert-webhook",
    "type": "webhook",
    "config": {"url": "https://hooks.example.com/alert"}
  }'

# Test a channel
curl -X POST http://localhost:9375/api/messages/channels/alert-webhook/test

# Get channel stats
curl http://localhost:9375/api/messages/channels/stats
```

### Supported Channel Types

| Type | Description | Required Config |
|------|-------------|-----------------|
| `webhook` | HTTP webhook | `url` |
| `email` | Email via SMTP | `smtp_host`, `from`, `to` |
| `telegram` | Telegram Bot | `bot_token`, `chat_id` |
| `dingtalk` | DingTalk robot | `webhook_url` |
| `wecom` | WeCom robot | `webhook_url` |
| `slack` | Slack webhook | `webhook_url` |
| `feishu` | Feishu/Lark robot | `webhook_url` |

### Web UI

Navigate to **Messages** (`/messages`) and switch to the **Channels** tab. Click "Add Channel" to open the ChannelEditorDialog (FullScreenDialog with sidebar for channel type selection).

---

## Using AI Chat to Manage Devices

NeoMind's AI chat interface allows you to manage devices through natural language. The AI agent uses CLI tools internally to execute commands.

### Example Conversations

**Register a new device:**

```
User: "I have a new temperature sensor I want to add. It connects via MQTT."
AI: [Uses shell tool to run `neomind device create --name "temp-sensor-1" --type mqtt ...`]
    "Device 'temp-sensor-1' has been created. It will connect to the embedded MQTT broker at localhost:1883."
```

**Check device status:**

```
User: "Show me all my devices"
AI: [Runs `neomind device list`]
    "You have 3 devices:
    - temp-sensor-1 (online, last seen 2 min ago)
    - camera-01 (offline)
    - smart-plug-3 (online)"
```

**Send a command to a device:**

```
User: "Turn off smart-plug-3"
AI: [Runs `neomind device command smart-plug-3 --command power_off`]
    "Command 'power_off' sent to smart-plug-3."
```

**Get device onboarding info:**

```
User: "How do I connect an ESP32 device?"
AI: [Runs `neomind system info` to get broker info]
    "Your MQTT broker is running at localhost:1883. For ESP32, use this code:
    ```cpp
    WiFiClient espClient;
    PubSubClient client(espClient);
    client.setServer("YOUR_SERVER_IP", 1883);
    client.connect("esp32-device-1");
    client.publish("neomind/device/esp32-device-1/telemetry", "{\"temperature\":25.5}");
    ```"
```

### Tips

- Be specific about device names and types
- The AI can create, list, update, and delete devices
- Use the GlobalChatFab (floating button) for quick access from any page
- The AI can diagnose offline devices and suggest fixes

---

## Creating Automation Rules

NeoMind uses a JSON-based API for automation rules. Rules can be created via CLI (`--json`), REST API, or AI chat.

### Rule JSON Structure

```json
{
  "name": "Rule Name",
  "condition": {
    "condition_type": "comparison",
    "source": "device:SENSOR_ID:METRIC",
    "operator": "greater_than",
    "threshold": 30
  },
  "actions": [
    {"type": "notify", "message": "Alert: {value}", "severity": "critical"}
  ]
}
```

**Condition types**: `comparison` (operator + threshold), `range` (min + max), `logical` (AND/OR/NOT combining sub-conditions)
**Operators**: `greater_than`, `less_than`, `greater_equal`, `less_equal`, `equal`, `not_equal`
**Actions**: `notify` (message + severity), `execute` (target + command), `trigger_agent` (agent_id)

### CLI Examples

```bash
# Create a temperature alert rule (enabled by default)
neomind rule create --json '{"name":"High Temp Alert","condition":{"condition_type":"comparison","source":"device:sensor1:temperature","operator":"greater_than","threshold":30},"actions":[{"type":"notify","message":"Temperature too high: {value}","severity":"critical"}]}'

# List all rules
neomind rule list

# Get rule details
neomind rule get <rule-id>

# Update a rule
neomind rule update <rule-id> --json '{"name":"High Temp Alert","condition":{"condition_type":"comparison","source":"device:sensor1:temperature","operator":"greater_than","threshold":35},"actions":[{"type":"notify","message":"CRITICAL: {value}","severity":"critical"}]}'

# Delete a rule
neomind rule delete <rule-id>
```

### API Examples

```bash
# Create a rule via API
curl -X POST http://localhost:9375/api/rules \
  -H "Content-Type: application/json" \
  -d '{"name":"High Temp Alert","condition":{"condition_type":"comparison","source":"device:sensor1:temperature","operator":"greater_than","threshold":30},"actions":[{"type":"notify","message":"Temperature too high","severity":"critical"}]}'

# List rules
curl http://localhost:9375/api/rules
```

### Using AI Chat

```
User: "Create a rule that sends me a webhook notification when the temperature exceeds 30 degrees"
AI: [Creates rule via CLI tool]
    "Rule 'high_temp_alert' created. It will notify via 'alert-webhook' when device:sensor1:temperature > 30."
```

### Web UI

Navigate to **Automation** (`/automation`) to manage rules visually with the rule builder interface.

---

## How to Use These Examples

### 1. Build Examples

```bash
# Build all examples
cargo build --workspace

# Build specific example
cargo build -p virtual-metrics-extension
cargo build -p event-monitor-extension
cargo build -p virtual-weather-provider
cargo build -p device-helper-example
```

### 2. Load into NeoMind

```bash
# Via CLI
neomind extension load path/to/extension

# Or via Web UI
# Navigate to Extensions -> Add Extension
```

### 3. Test Features

```bash
# Via CLI
neomind extension execute virtual-weather-provider set_location \
    --latitude 39.9 \
    --longitude 116.4

neomind extension execute virtual-weather-provider update_weather

# Via API
curl -X POST http://localhost:9375/api/extensions/virtual-weather-provider/commands/set_location \
    -H "Content-Type: application/json" \
    -d '{"latitude": 39.9, "longitude": 116.4}'
```

### 4. View Metrics

```bash
# Via CLI
neomind extension metrics virtual-weather-provider

# Via API
curl http://localhost:9375/api/extensions/virtual-weather-provider/metrics
```

---

## Example Comparison

| Example | Type | Primary Use | Learning Focus |
|---------|------|-------------|----------------|
| **Virtual Metrics** | Extension | Inject virtual metrics | State management, virtual metrics API |
| **Event Monitor** | Extension | Listen to system events | Event subscription, event filtering |
| **Virtual Weather** | Extension | Integrate external weather data | External API calls, scheduled tasks |
| **Device Helper** | Extension Example | Demonstrate DeviceHelper framework | Type-safe API, device interaction |
| **Device Capability Provider** | Capability Provider | Provide device capabilities | Capability system architecture |
| **Runner Capability Provider** | Capability Provider | Provide runner capabilities | High-performance capability provision |

---

## Extension Development Recommendations

### Beginners

Recommended learning order:
1. **Virtual Metrics Extension** - Simplest, understand basic structure
2. **Device Helper Example** - Learn complete device interaction API
3. **Event Monitor Extension** - Understand event subscription mechanisms

### Advanced Developers

Recommended learning order:
1. **Virtual Weather Provider** - Learn external API integration
2. **Device Capability Provider** - Understand capability system design
3. **Runner Capability Provider** - Learn high-performance architecture

### Real-World Projects

Based on these examples, you can develop:
- Smart home automation extensions
- Data analysis and visualization extensions
- Third-party service integration extensions
- Custom automation rule extensions
- Device adapter extensions

---

## Troubleshooting

### Extension Fails to Load

- Check if the extension compiled successfully: `cargo build -p <example-name>`
- Check if ABI version matches
- Review log files for error messages

### Command Execution Fails

- Verify parameter format is correct
- Check if device ID exists
- Confirm extension has sufficient permissions

### Virtual Metrics Not Showing

- Check if device ID is correct
- Confirm telemetry storage is enabled
- Verify metric name spelling is correct

### Event Monitor Shows No Data

- Confirm event bus is running
- Check event filter configuration
- Verify event source is producing events

### Push Target Not Delivering

- Check if push target is started: `neomind push list`
- Review delivery logs: `neomind push logs <target-id>`
- Test the target: `neomind push test <target-id>`
- Verify external endpoint is reachable

### Notification Channel Not Working

- Test the channel: `neomind channel test --name <channel-name>`
- Check if channel is enabled: `neomind channel list`
- Verify channel configuration (URL, credentials)

---

## Related Documentation

- **Extension Development Guide**: `docs/guides/en/16-extension-dev.md`
- **DeviceHelper Framework**: `docs/guides/en/framework-summary.md`
- **Extension SDK**: `crates/neomind-extension-sdk/`
- **Capability System**: `crates/neomind-core/src/extension/context.rs`
- **API Reference**: `docs/guides/en/14-api.md`
- **LLM Configuration**: `docs/guides/en/02-llm.md`
- **Device Management**: `docs/guides/en/04-devices.md`

---

## Contributing

If you want to add new extension examples:

1. Create a new directory under `examples/`
2. Add `Cargo.toml` and `src/lib.rs`
3. Add your example to root `Cargo.toml` `members`
4. Write clear documentation and comments
5. Submit a Pull Request

---

**Last Updated**: 2026-05-26
