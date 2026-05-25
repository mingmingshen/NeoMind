---
id: rule-management
name: Rule Management & DSL Guide
category: rule
origin: builtin
priority: 85
token_budget: 10000
triggers:
  keywords: [rule, 规则, 创建规则, create rule, alert, 告警, 报警, trigger, 触发, automation, 自动化, condition, 条件, action, 动作, WHEN, DO, DSL, threshold, 阈值, notification, 通知, 规则管理, rule create, rule update, rule enable, rule disable]
  tool_target:
    - tool: rule
      actions: [list, get, create, update, delete, enable, disable, test]
anti_triggers:
  keywords: [dashboard, 仪表盘, agent, 代理, extension develop, 扩展开发, device connect, 设备连接]
---

# Rule Management & DSL Guide

Rules are event-driven automations that trigger actions when conditions are met. They use a DSL (Domain Specific Language) for defining conditions and actions.

## CRITICAL: Rule DSL Format

Rules use DSL syntax, NOT JSON trigger/actions. The `--dsl` flag is required for `rule create`.

```
RULE "<rule_name>"
  WHEN <condition_expression>
  DO <action_expression>
END
```

**IMPORTANT: The rule name MUST be in double quotes:** `RULE "High Temp Alert"`, NOT `RULE high_temp`.

## Before Creating Rules

**Always discover real device IDs and metric names first:**

```bash
# Step 1: Find device IDs
neomind device list

# Step 2: Find metric names for a specific device
neomind device latest <DEVICE_ID>
```

**NEVER guess metric names.** Always use `device latest` to discover the actual field names.

## DSL Syntax

### Basic Structure

```
RULE "High Temperature Alert"
  WHEN sensor-001.temperature > 30
  DO
    NOTIFY "Temperature too high: {value}°C"
  END
```

### WHEN Conditions

**CRITICAL: Use the actual device ID directly — do NOT prefix with `device.`**

| Condition Type | Syntax | Example | WRONG |
|---------------|--------|---------|-------|
| Device metric comparison | `<device_id>.<metric> <op> <value>` | `sensor-001.temperature > 30` | ~~`device.sensor-001.temperature > 30`~~ |
| AND logic | `<cond1> AND <cond2>` | `sensor-001.temp > 20 AND sensor-001.humidity < 50` | |
| OR logic | `<cond1> OR <cond2>` | `sensor-001.battery < 10 OR sensor-002.battery < 10` | |
| NOT logic | `NOT <condition>` | `NOT sensor-001.status == "online"` | |
| Extension metric | `EXTENSION <ext_id>.<metric> <op> <value>` | `EXTENSION weather.temperature_c > 35` | |
| Range check | `<device_id>.<metric> BETWEEN <val1> AND <val2>` | `sensor-001.temperature BETWEEN 18 AND 28` | |

**Comparison operators:** `>`, `<`, `>=`, `<=`, `==`, `!=`

**Value types:**
- Numbers: `30`, `0.5`, `-10`
- Strings: `"online"`, `"error"`
- Booleans: `true`, `false`

### DO Actions

| Action Type | Syntax | Description |
|-------------|--------|-------------|
| Send notification | `NOTIFY "message text" [channel1, channel2]` | Send notification via message channel |
| Execute command | `EXECUTE <device_id>.<command>(key=value)` | Send command to device |
| Log message | `LOG <level> "message"` | Log (level: info, warn, error) |
| Alert | `ALERT "title" "message" <SEVERITY>` | Send alert (severity: info, warning, error, critical) |
| Trigger agent | `TRIGGER_AGENT <agent_id> "input text"` | Execute an AI agent |

**Placeholders in messages:**
- `{value}` — the triggering value
- `{device_id}` — the source device
- `{metric}` — the metric name
- `{timestamp}` — when it triggered

## Command Reference

### Create Rule

```bash
neomind rule create --name '<rule_name>' --dsl '<DSL>'
```

Required: `--dsl`
Optional: `--name` (can be embedded in DSL via `RULE "name"`)

### List & Get

```bash
neomind rule list                    # List all rules
neomind rule get <ID>                # Get rule details
```

### Update Rule

```bash
neomind rule update <ID> --name 'New Name' --dsl '<NEW_DSL>'
```

### Enable / Disable

```bash
neomind rule enable <ID>             # Enable rule
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

## Workflow Examples

### Temperature Alert

```bash
# Discover device ID and metrics first
neomind device list
neomind device latest sensor-001

# Create rule with discovered device_id and metric
neomind rule create --name 'High Temperature Alert' --dsl 'RULE "High Temperature Alert"
  WHEN sensor-001.temperature > 35
  DO
    NOTIFY "Sensor {device_id} temperature is {value}°C, exceeds threshold 35°C"
  END'
```

### Low Battery Warning

```bash
neomind rule create --name 'Low Battery' --dsl 'RULE "Low Battery Warning"
  WHEN sensor-001.battery < 20
  DO
    NOTIFY "Sensor {device_id} battery at {value}%"
  END'
```

### Multi-Device Alert

```bash
neomind rule create --name 'Any Sensor Low Battery' --dsl 'RULE "Multi Battery Alert"
  WHEN sensor-001.battery < 15 OR sensor-002.battery < 15 OR sensor-003.battery < 15
  DO
    ALERT "Battery Critical" "Critical battery level detected on sensors" critical
  END'
```

### Device Control Rule

```bash
neomind rule create --name 'Auto Cool Down' --dsl 'RULE "Auto Cool Down"
  WHEN sensor-001.temperature > 30
  DO
    EXECUTE ac-unit.turn_on(mode="cool", target=25)
  END'
```

### Extension-Based Rule

```bash
# Discover extension metrics first
neomind extension get weather

# Create rule using extension data
neomind rule create --name 'Extreme Weather' --dsl 'RULE "Extreme Weather Alert"
  WHEN EXTENSION weather.temperature_c > 38
  DO
    NOTIFY "Extreme heat: {value}°C from weather extension"
  END'
```

### Combined Conditions

```bash
neomind rule create --name 'Heat Index Alert' --dsl 'RULE "Heat Index Alert"
  WHEN sensor-001.temperature > 30 AND sensor-001.humidity > 70
  DO
    NOTIFY "High heat index: temp={value}°C, humidity >70%"
  END'
```

## Rule Lifecycle

1. **Create** — Rule is created in disabled state
2. **Enable** — Activate the rule for live monitoring
3. **Monitor** — Check execution history: `neomind rule history <ID>`
4. **Disable** — Temporarily stop without deleting
5. **Update** — Modify conditions or actions
6. **Delete** — Permanently remove

## Common Errors & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| Rule has no name | DSL uses unquoted name `RULE foo` | Use quoted name: `RULE "My Rule"` |
| "Device not found in condition" | Wrong device ID | Run `neomind device list` for valid IDs |
| "Unknown metric" | Wrong metric name | Run `neomind device latest <ID>` for valid metrics |
| Condition matches wrong device | Used `device.` prefix | Remove `device.` — use actual device ID directly: `sensor-001.temp` not `device.sensor-001.temp` |
| "Invalid DSL syntax" | Malformed DSL | Check RULE/WHEN/DO/END structure, ensure name is quoted |
| Rule not triggering | Rule is disabled | Run `neomind rule enable <ID>` |
| Rule triggers too often | No debounce | Add threshold margin or use AND with time conditions |
| "Missing END" | DSL not terminated | Ensure DSL ends with `END` on its own line |
