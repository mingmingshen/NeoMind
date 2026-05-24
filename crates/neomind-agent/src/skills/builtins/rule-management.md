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
RULE <rule_name>
  WHEN <condition_expression>
  DO <action_expression>
END
```

## DSL Syntax

### Basic Structure

```
RULE high_temperature
  WHEN device.sensor-001.temperature > 30
  DO SEND_MESSAGE severity=warning message="Temperature too high: {value}°C"
END
```

### WHEN Conditions

| Condition Type | Syntax | Example |
|---------------|--------|---------|
| Device metric comparison | `device.<device_id>.<metric> <op> <value>` | `device.sensor-001.temperature > 30` |
| AND logic | `<cond1> AND <cond2>` | `device.s1.temp > 20 AND device.s1.humidity < 50` |
| OR logic | `<cond1> OR <cond2>` | `device.s1.battery < 10 OR device.s2.battery < 10` |
| NOT logic | `NOT <condition>` | `NOT device.s1.status == "online"` |

**Comparison operators:** `>`, `<`, `>=`, `<=`, `==`, `!=`

**Value types:**
- Numbers: `30`, `0.5`, `-10`
- Strings: `"online"`, `"error"`
- Booleans: `true`, `false`

### DO Actions

| Action Type | Syntax | Description |
|-------------|--------|-------------|
| Send message | `SEND_MESSAGE severity=<level> message="<text>"` | Send notification via message channel |
| Control device | `CONTROL_DEVICE device=<id> command=<cmd> params='<json>'` | Send command to device |
| Trigger agent | `TRIGGER_AGENT agent=<id> input="<text>"` | Execute an AI agent |

**Severity levels:** `info`, `warning`, `error`, `critical`

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
Optional: `--name` (can be embedded in DSL)

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
neomind rule create --name 'High Temperature Alert' --dsl 'RULE high_temp
  WHEN device.sensor-001.temperature > 35
  DO SEND_MESSAGE severity=warning message="Sensor {device_id} temperature is {value}°C, exceeds threshold 35°C"
END'
```

### Low Battery Warning

```bash
neomind rule create --name 'Low Battery' --dsl 'RULE low_battery
  WHEN device.sensor-001.battery < 20
  DO SEND_MESSAGE severity=error message="Sensor {device_id} battery at {value}%"
END'
```

### Multi-Device Alert

```bash
neomind rule create --name 'Any Sensor Low Battery' --dsl 'RULE multi_battery
  WHEN device.sensor-001.battery < 15 OR device.sensor-002.battery < 15 OR device.sensor-003.battery < 15
  DO SEND_MESSAGE severity=critical message="Critical battery level detected"
END'
```

### Device Control Rule

```bash
neomind rule create --name 'Auto Cool Down' --dsl 'RULE auto_cool
  WHEN device.sensor-001.temperature > 30
  DO CONTROL_DEVICE device=ac-unit command=turn_on params='"mode":"cool","target":25}'
END'
```

### Agent Trigger Rule

```bash
neomind rule create --name 'Anomaly Detection' --dsl 'RULE anomaly
  WHEN device.sensor-001.temperature > 40
  DO TRIGGER_AGENT agent=analyzer-agent input="Temperature anomaly: {value}°C from {device_id}"
END'
```

### Combined Conditions

```bash
neomind rule create --name 'Heat Index Alert' --dsl 'RULE heat_index
  WHEN device.sensor-001.temperature > 30 AND device.sensor-001.humidity > 70
  DO SEND_MESSAGE severity=warning message="High heat index: temp={value}°C, humidity >70%"
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
| "Invalid DSL syntax" | Malformed DSL | Check RULE/WHEN/DO/END structure |
| "Device not found in condition" | Wrong device ID | Run `neomind device list` for valid IDs |
| "Unknown metric" | Wrong metric name | Run `neomind device latest <ID>` for valid metrics |
| "Rule not found" | Wrong rule ID | Run `neomind rule list` for valid IDs |
| Rule not triggering | Rule is disabled | Run `neomind rule enable <ID>` |
| Rule triggers too often | No debounce | Add threshold margin or use AND with time conditions |
| "Missing END" | DSL not terminated | Ensure DSL ends with `END` on its own line |
