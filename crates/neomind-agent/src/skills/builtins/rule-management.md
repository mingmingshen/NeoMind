---
id: rule-management
name: Rule Management Guide
category: rule
origin: builtin
priority: 85
token_budget: 12000
triggers:
  keywords: [rule, 规则, 创建规则, create rule, alert, 告警, 报警, trigger, 触发, automation, 自动化, condition, 条件, action, 动作, threshold, 阈值, notification, 通知, 规则管理, rule create, rule update, rule enable, rule disable, 阈值判断, 超过, 低于, 大于, 小于, temperature, 温度, humidity, 湿度, battery, 电池]
  tool_target:
    - tool: rule
      actions: [list, get, create, update, delete, enable, disable, test, history]
anti_triggers:
  keywords: [dashboard, 仪表盘, agent, 代理, extension develop, 扩展开发, device connect, 设备连接]
---

# Rule Management Guide

Rules are event-driven automations that trigger actions when device or extension metrics meet conditions.

## MANDATORY: 3-Step Rule Creation Flow

You MUST follow these 3 steps in order. Skipping Step 1 or 2 will result in wrong device IDs or metric names, causing rules to silently fail.

### Step 1: Discover Device IDs and Metric Names

```bash
neomind device list
```

This returns devices grouped by type. Each type group includes:
- **`metric_fields`**: array of actual metric field names (e.g., `["temperature", "humidity", "battery"]`)
- **`example`**: one online device's current values (e.g., `{"id": "sensor-001", "temperature": 25.3, "humidity": 60}`)
- **`devices`**: list of devices with `id`, `name`, `status`

**IMPORTANT**: If `metric_fields` is empty (device offline or >50 devices), you MUST run Step 2 for each target device.

### Step 2: Get Exact Metrics for Target Device (if needed)

```bash
neomind device get <DEVICE_ID>
```

This returns:
- All metric names with current values
- Available commands
- Device metadata

Use this when:
- `device list` showed empty `metric_fields` for the device type
- You need to confirm exact metric key names
- You need to know the current value range to set a reasonable threshold

### Step 3: Create Rule with Discovered Names

Only NOW can you create the rule, using the exact `device_id` and `metric` name discovered above.

## Rule JSON Format

Rules use JSON format. The `--json` flag is required for `rule create`.

### Basic Structure

```json
{
  "name": "Rule Name",
  "description": "Optional description",
  "trigger": {"trigger_type": "data_change"},
  "condition": {
    "condition_type": "comparison",
    "source": "device:SENSOR_ID:METRIC",
    "operator": "greater_than",
    "threshold": 30
  },
  "for_duration": 60000,
  "cooldown": 60000,
  "actions": [
    {"type": "notify", "message": "Alert: {value}", "severity": "critical"}
  ]
}
```

**Optional fields**: `trigger` (default: `data_change`), `for_duration` (ms, condition must hold before triggering), `cooldown` (ms, default 60000, min time between triggers), `enabled` (default: true)

### `for_duration` — Sustained Condition

By default, a rule fires the instant its condition is met. Use `for_duration` to require the condition to hold for a sustained period (in milliseconds) before triggering:

```json
{
  "name": "Sustained High Temp",
  "condition": {"condition_type": "comparison", "source": "device:sensor-001:temperature", "operator": "greater_than", "threshold": 30},
  "for_duration": 60000,
  "cooldown": 300000,
  "actions": [{"type": "notify", "message": "Temperature above 30°C for over 1 minute: {value}°C", "severity": "warning"}]
}
```
- `for_duration: 60000` = condition must hold for 60 seconds before firing
- `for_duration: 0` or omitted = fires immediately when condition is met
- Combine with `cooldown` to avoid repeat alerts

### How Notifications Are Delivered

Rule `notify` actions create messages that are sent to **all configured channels** (Telegram, email, webhook, etc.). Channels receive all messages by default — no per-channel routing setup needed.

**Typical alert setup** (see `message-management` skill for channel creation):
```bash
# 1. Create a channel first (e.g., Telegram)
neomind message channel-create --name tg-alerts --type telegram --config '{"token":"...","chat_id":"..."}'

# 2. Create the rule — notifications auto-deliver to tg-alerts
neomind rule create --json '{"name":"High Temp","condition":{"condition_type":"comparison","source":"device:sensor-001:temperature","operator":"greater_than","threshold":30},"actions":[{"type":"notify","message":"Temp: {value}°C","severity":"critical"}]}'
```

### Condition Types

**1. Comparison** — metric compared to threshold:
```json
{
  "condition_type": "comparison",
  "source": "device:sensor-001:temperature",
  "operator": "greater_than",
  "threshold": 30
}
```

**2. Range** — metric within min/max (inclusive):
```json
{
  "condition_type": "range",
  "source": "device:sensor-001:temperature",
  "min": 18,
  "max": 28
}
```

**3. Logical** — AND/OR/NOT combining sub-conditions:
```json
{
  "condition_type": "logical",
  "operator": "and",
  "conditions": [
    {"condition_type": "comparison", "source": "device:sensor-001:temperature", "operator": "greater_than", "threshold": 30},
    {"condition_type": "comparison", "source": "device:sensor-001:humidity", "operator": "greater_than", "threshold": 70}
  ]
}
```

### Source Format

| Type | Format | Example |
|------|--------|---------|
| Device metric | `device:DEVICE_ID:METRIC` | `device:sensor-001:temperature` |
| Extension metric | `extension:EXT_ID:METRIC` | `extension:weather:temperature_c` |
| Transform output | `transform:OUTPUT_PREFIX:FIELD` | `transform:battery:health` |

### Operators

**Numeric** (threshold is a number):

| Operator | JSON value |
|----------|-----------|
| > | `greater_than` |
| < | `less_than` |
| >= | `greater_equal` |
| <= | `less_equal` |
| == | `equal` |
| != | `not_equal` |

**String** (threshold is a string, for text-based metrics):

| Operator | JSON value |
|----------|-----------|
| Contains | `contains` |
| Starts with | `starts_with` |
| Ends with | `ends_with` |
| Regex match | `regex` |

### Action Types

**1. Notify** — send notification:
```json
{"type": "notify", "message": "Temperature too high: {value}°C", "severity": "critical"}
```
Severities: `info`, `warning`, `critical`, `emergency`

**2. Execute** — run device/extension command:
```json
{"type": "execute", "target": "ac-unit", "target_type": "device", "command": "turn_on", "params": {"mode": "cool", "target": 25}}
```

**3. TriggerAgent** — hand off to AI agent:
```json
{"type": "trigger_agent", "agent_id": "agent-001", "input": "Check temperature anomaly"}
```

**Message placeholders:** `{value}`, `{source_id}`

### Trigger Types

| Type | JSON | Description |
|------|------|-------------|
| Data Change | `{"trigger_type": "data_change"}` | Default. Fires when subscribed metric changes. Sources auto-extracted from condition. |
| Schedule | `{"trigger_type": "schedule", "cron": "0 */5 * * *"}` | Fires on cron schedule. No condition needed. |
| Manual | `{"trigger_type": "manual"}` | Fires only via API/CLI `rule test`. No condition needed. |

## Complete Workflow Examples

### Example 1: Temperature Threshold Alert

User says: "创建一个规则，温度超过30度时通知我"

**Correct flow:**
```bash
# Step 1: Discover devices and metrics
neomind device list
# → See type "sensor" with metric_fields: ["temperature", "humidity", "battery"]
# → See device id: "living-room-sensor", status: online

# Step 2 (optional): Confirm exact metric name
neomind device get living-room-sensor
# → metrics: {"temperature": {"value": 24.5, "unit": "°C"}, ...}

# Step 3: Create rule with REAL device ID and REAL metric name
neomind rule create --json '{"name":"High Temperature Alert","condition":{"condition_type":"comparison","source":"device:living-room-sensor:temperature","operator":"greater_than","threshold":30},"actions":[{"type":"notify","message":"Temperature too high: {value}°C","severity":"critical"}]}'

# Rule is enabled by default — no need to run enable
```

### Example 2: Low Battery Warning

```bash
neomind device list
# → device id: "outdoor-sensor", metric_fields includes "battery"

neomind rule create --json '{"name":"Low Battery Warning","condition":{"condition_type":"comparison","source":"device:outdoor-sensor:battery","operator":"less_than","threshold":20},"actions":[{"type":"notify","message":"Sensor battery at {value}%","severity":"warning"}]}'
```

### Example 3: Multi-Condition Rule (AND)

```bash
neomind device list
neomind device get living-room-sensor

neomind rule create --json '{"name":"Heat Index Alert","condition":{"condition_type":"logical","operator":"and","conditions":[{"condition_type":"comparison","source":"device:living-room-sensor:temperature","operator":"greater_than","threshold":30},{"condition_type":"comparison","source":"device:living-room-sensor:humidity","operator":"greater_than","threshold":70}]},"actions":[{"type":"notify","message":"High heat index: temp > 30, humidity > 70","severity":"warning"}]}'
```

### Example 4: Auto Control Rule

```bash
neomind device list
# → sensor: "temp-probe", metric_fields: ["temperature"]
# → actuator: "ac-unit", command_fields: ["turn_on"]

neomind rule create --json '{"name":"Auto Cool Down","condition":{"condition_type":"comparison","source":"device:temp-probe:temperature","operator":"greater_than","threshold":30},"actions":[{"type":"execute","target":"ac-unit","target_type":"device","command":"turn_on","params":{"mode":"cool","target":25}}]}'
```

### Example 5: Extension-Based Rule

```bash
neomind extension get weather
# → output_fields include: "temperature_c", "humidity_pct"

neomind rule create --json '{"name":"Extreme Weather","condition":{"condition_type":"comparison","source":"extension:weather:temperature_c","operator":"greater_than","threshold":38},"actions":[{"type":"notify","message":"Extreme heat: {value}°C","severity":"critical"}]}'
```

### Example 6: Multi-Device Alert (OR)

```bash
neomind device list
# → 3 sensors: sensor-a, sensor-b, sensor-c, all have "battery" metric

neomind rule create --json '{"name":"Multi Battery Alert","condition":{"condition_type":"logical","operator":"or","conditions":[{"condition_type":"comparison","source":"device:sensor-a:battery","operator":"less_than","threshold":15},{"condition_type":"comparison","source":"device:sensor-b:battery","operator":"less_than","threshold":15},{"condition_type":"comparison","source":"device:sensor-c:battery","operator":"less_than","threshold":15}]},"actions":[{"type":"notify","message":"Critical battery on sensors","severity":"critical"}]}'
```

## Command Reference

### Create Rule
```bash
neomind rule create --json '<JSON_BODY>'
```
Required: `--json` (rule definition in JSON format)

### List & Get
```bash
neomind rule list                    # List all rules
neomind rule get <ID>                # Get rule details
```

### Update Rule
```bash
neomind rule update <ID> --json '<NEW_JSON>'
```

### Enable / Disable
```bash
neomind rule enable <ID>             # Re-enable a disabled rule
neomind rule disable <ID>            # Disable rule (keeps config)
```

### Delete Rule
```bash
neomind rule delete <ID>             # Permanently delete
```

### Test Rule
```bash
neomind rule test <ID> --input '{"temperature": 35}'
```

### View Execution History
```bash
neomind rule history <ID>
```

## Rule Lifecycle

1. **Create** — Rule is created and **enabled by default**
2. **Monitor** — Check execution history: `neomind rule history <ID>`
3. **Disable** — Temporarily stop without deleting
4. **Update** — Modify conditions or actions
5. **Delete** — Permanently remove

## Common Errors & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| "Missing 'name' field" | JSON body missing name | Add `"name": "Rule Name"` |
| "Invalid DataSourceId" | Wrong source format | Use `device:ID:METRIC` or `extension:ID:METRIC` |
| "Device not found in condition" | Wrong device ID (guessed) | Run `neomind device list` for real IDs |
| "Unknown metric" | Wrong metric name (guessed) | Run `neomind device list` (check `metric_fields`) or `neomind device get <ID>` |
| Rule not triggering | Rule is disabled | Run `neomind rule enable <ID>` |
| Rule triggers too often | No cooldown / threshold too tight | Add `"cooldown": 300000` (ms) |
| Invalid JSON | Malformed JSON body | Check JSON syntax: quotes, brackets, commas |

## Decision Tree: How to Choose the Right Condition

```
User request
├── Single device, single metric threshold
│   → condition_type: "comparison"
├── Single device, multiple metrics (all must be true)
│   → condition_type: "logical", operator: "and"
├── Multiple devices, same metric (any triggers)
│   → condition_type: "logical", operator: "or"
├── Extension data
│   → source: "extension:EXT_ID:METRIC"
├── Value in range
│   → condition_type: "range"
└── Negation (alert when NOT something)
    → condition_type: "logical", operator: "not"
```
