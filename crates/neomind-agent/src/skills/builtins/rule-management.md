---
id: rule-management
name: Rule Management & DSL Guide
category: rule
origin: builtin
priority: 85
token_budget: 12000
triggers:
  keywords: [rule, 规则, 创建规则, create rule, alert, 告警, 报警, trigger, 触发, automation, 自动化, condition, 条件, action, 动作, WHEN, DO, DSL, threshold, 阈值, notification, 通知, 规则管理, rule create, rule update, rule enable, rule disable, 阈值判断, 超过, 低于, 大于, 小于, temperature, 温度, humidity, 湿度, battery, 电池]
  tool_target:
    - tool: rule
      actions: [list, get, create, update, delete, enable, disable, test, history]
anti_triggers:
  keywords: [dashboard, 仪表盘, agent, 代理, extension develop, 扩展开发, device connect, 设备连接]
---

# Rule Management & DSL Guide

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

Only NOW can you write the DSL, using the exact `device_id` and `metric` name discovered above.

## CRITICAL: Rule DSL Format

Rules use DSL syntax, NOT JSON trigger/actions. The `--dsl` flag is required for `rule create`.

```
RULE "<rule_name>"
  WHEN <device_id>.<metric> <operator> <value>
  DO <action>
END
```

### DSL Syntax Rules

| Rule | Correct | Wrong |
|------|---------|-------|
| Rule name in double quotes | `RULE "High Temp Alert"` | ~~`RULE high_temp`~~ |
| Use actual device ID directly | `sensor-001.temperature > 30` | ~~`device.sensor-001.temperature > 30`~~ |
| Use exact metric name from discovery | `sensor-001.temperature` | ~~`sensor-001.temp`~~ (unless discovered as `temp`) |
| DSL ends with `END` | `... DO NOTIFY "msg" END` | ~~missing `END`~~ |
| New rules are disabled | Must `neomind rule enable <ID>` after create | ~~forgetting to enable~~ |

### WHEN Condition Syntax

**Basic comparison:**
```
<device_id>.<metric> <op> <value>
```

| Pattern | Example | Notes |
|---------|---------|-------|
| Device metric | `sensor-001.temperature > 30` | Use real device ID + real metric name |
| Extension metric | `EXTENSION weather.temperature_c > 35` | Prefix with `EXTENSION` |
| AND logic | `sensor-001.temperature > 30 AND sensor-001.humidity < 50` | Multiple conditions |
| OR logic | `sensor-001.battery < 10 OR sensor-002.battery < 10` | Any condition triggers |
| NOT logic | `NOT sensor-001.status == "online"` | Negation |
| Range check | `sensor-001.temperature BETWEEN 18 AND 28` | Inclusive range |

**Operators:** `>`, `<`, `>=`, `<=`, `==`, `!=`, `BETWEEN ... AND ...`

**Value types:**
- Numbers: `30`, `0.5`, `-10`
- Strings: `"online"`, `"error"` (must be in quotes)
- Booleans: `true`, `false`

### DO Action Syntax

| Action | Syntax | Example |
|--------|--------|---------|
| Send notification | `NOTIFY "message"` | `NOTIFY "Temperature too high: {value}°C"` |
| Execute device command | `EXECUTE <device_id>.<command>(key=val)` | `EXECUTE ac-unit.turn_on(mode="cool", target=25)` |
| Send alert | `ALERT "title" "message" <SEVERITY>` | `ALERT "Critical" "Check sensor" critical` |
| Trigger AI agent | `TRIGGER_AGENT <agent_id> "input"` | `TRIGGER_AGENT agent-001 "Check temperature"` |
| Log message | `LOG <level> "message"` | `LOG warn "Temperature spike detected"` |

**Message placeholders:** `{value}`, `{device_id}`, `{metric}`, `{timestamp}`

**Alert severity levels:** `info`, `warning`, `error`, `critical`

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
neomind rule create --dsl 'RULE "High Temperature Alert"
  WHEN living-room-sensor.temperature > 30
  DO
    NOTIFY "Temperature too high: {value}°C on {device_id}"
  END'

# Step 4: Enable the rule
neomind rule enable <RULE_ID>
```

**WRONG (will silently fail):**
```bash
# ❌ Guessed device ID "sensor-001" (not real)
# ❌ Skipped device list, used imaginary names
neomind rule create --dsl 'RULE "High Temp" WHEN sensor-001.temperature > 30 DO NOTIFY "hot" END'
```

### Example 2: Low Battery Warning

```bash
# Discover first
neomind device list
# → device id: "outdoor-sensor", metric_fields includes "battery"

# Create with discovered names
neomind rule create --dsl 'RULE "Low Battery Warning"
  WHEN outdoor-sensor.battery < 20
  DO
    NOTIFY "Sensor {device_id} battery at {value}%"
  END'

neomind rule enable <RULE_ID>
```

### Example 3: Multi-Condition Rule

```bash
# Discover devices
neomind device list
neomind device get living-room-sensor

# Both conditions must be true
neomind rule create --dsl 'RULE "Heat Index Alert"
  WHEN living-room-sensor.temperature > 30 AND living-room-sensor.humidity > 70
  DO
    ALERT "Heat Index Warning" "High heat index: temp={value}°C, humidity >70%" warning
  END'

neomind rule enable <RULE_ID>
```

### Example 4: Auto Control Rule

```bash
neomind device list
# → sensor: "temp-probe", metric_fields: ["temperature"]
# → actuator: "ac-unit", command_fields: ["turn_on"]

neomind rule create --dsl 'RULE "Auto Cool Down"
  WHEN temp-probe.temperature > 30
  DO
    EXECUTE ac-unit.turn_on(mode="cool", target=25)
  END'

neomind rule enable <RULE_ID>
```

### Example 5: Extension-Based Rule

```bash
# Discover extension metrics
neomind extension get weather
# → output_fields include: "temperature_c", "humidity_pct"

neomind rule create --dsl 'RULE "Extreme Weather"
  WHEN EXTENSION weather.temperature_c > 38
  DO
    NOTIFY "Extreme heat: {value}°C from weather extension"
  END'

neomind rule enable <RULE_ID>
```

### Example 6: Multi-Device Alert (same metric across devices)

```bash
neomind device list
# → 3 sensors: sensor-a, sensor-b, sensor-c, all have "battery" metric

neomind rule create --dsl 'RULE "Multi Battery Alert"
  WHEN sensor-a.battery < 15 OR sensor-b.battery < 15 OR sensor-c.battery < 15
  DO
    ALERT "Battery Critical" "Critical battery level on sensors" critical
  END'

neomind rule enable <RULE_ID>
```

## Command Reference

### Create Rule
```bash
neomind rule create --dsl '<DSL>'
```
Required: `--dsl` (rule name can be embedded in DSL via `RULE "name"`)

### List & Get
```bash
neomind rule list                    # List all rules
neomind rule get <ID>                # Get rule details
```

### Update Rule
```bash
neomind rule update <ID> --dsl '<NEW_DSL>'
```

### Enable / Disable
```bash
neomind rule enable <ID>             # Enable rule (REQUIRED after create!)
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

1. **Create** — Rule is created in **disabled** state
2. **Enable** — MUST run `neomind rule enable <ID>` to activate
3. **Monitor** — Check execution history: `neomind rule history <ID>`
4. **Disable** — Temporarily stop without deleting
5. **Update** — Modify conditions or actions
6. **Delete** — Permanently remove

## Common Errors & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| Rule has no name | DSL uses unquoted name `RULE foo` | Use quoted name: `RULE "My Rule"` |
| "Device not found in condition" | Wrong device ID (guessed) | Run `neomind device list` for real IDs |
| "Unknown metric" | Wrong metric name (guessed) | Run `neomind device list` (check `metric_fields`) or `neomind device get <ID>` |
| Condition matches wrong device | Used `device.` prefix | Remove `device.` — use actual ID: `sensor-001.temp` not `device.sensor-001.temp` |
| "Invalid DSL syntax" | Malformed DSL | Check RULE/WHEN/DO/END structure, name in quotes |
| Rule not triggering | Rule is disabled | Run `neomind rule enable <ID>` |
| Rule triggers too often | No debounce / threshold too tight | Add threshold margin or combine with AND condition |
| "Missing END" | DSL not terminated | Ensure DSL ends with `END` on its own line |
| Rule created but metric empty | Used guessed metric name | ALWAYS run `device list` first to get real `metric_fields` |

## Decision Tree: How to Choose the Right Condition

```
User request
├── Single device, single metric threshold
│   → WHEN <device_id>.<metric> <op> <value>
├── Single device, multiple metrics (all must be true)
│   → WHEN <device_id>.<metric1> <op1> <val1> AND <device_id>.<metric2> <op2> <val2>
├── Multiple devices, same metric (any triggers)
│   → WHEN <dev1>.<metric> <op> <val> OR <dev2>.<metric> <op> <val>
├── Extension data
│   → WHEN EXTENSION <ext_id>.<metric> <op> <value>
├── Value in range
│   → WHEN <device_id>.<metric> BETWEEN <low> AND <high>
└── Negation (alert when NOT something)
    → WHEN NOT <device_id>.<metric> == <value>
```
