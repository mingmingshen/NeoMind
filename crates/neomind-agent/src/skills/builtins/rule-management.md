---
id: rule-management
name: Rule Management CLI Commands
category: rule
origin: builtin
priority: 80
token_budget: 10000
triggers:
  keywords: [rule, 规则, automation, 自动化, rule list, list rule, rule create, 创建规则, rule enable, 启用规则, rule disable, 禁用规则, trigger, 触发器, condition, 条件, 告警, alert, 通知, notify, DSL, rule delete, rule update, rule test, rule history, battery alert, temperature alert, offline alert, device offline]
  tool_target:
    - tool: rule
      actions: [list, get, create, update, delete, enable, disable, test, history]
anti_triggers:
  keywords: [device, 设备, agent, 代理, transform, 变换]
---

# Rule Management CLI Commands

Use `neomind` CLI commands via the `shell` tool to manage automation rules. Rules use a DSL (Domain-Specific Language) format and respond to device metric changes or status events.

## ⚠️ CRITICAL: Common Operations Checklist

When user asks to **enable/disable/delete** a rule, you MUST:
1. `neomind rule list` → find the rule ID
2. Then execute the actual action command:

| User Request | Command |
|---|---|
| 启用/Enable rule | `neomind rule enable <ID>` |
| 禁用/Disable rule | `neomind rule disable <ID>` |
| 删除/Delete rule | `neomind rule delete <ID>` |
| 更新阈值 | `neomind rule update <ID> --dsl 'RULE ... WHEN device.metric(x) > NEW_VALUE ... END'` |
| 查看执行历史 | `neomind rule history <ID>` |

> **DO NOT stop after `rule list`!** Listing is only step 1. You MUST proceed to the action command.

## Quick DSL Reference

```
RULE <name> WHEN <condition> DO <action> END
```
- Conditions: `device.metric(<metric_name>) <op> <value>`, `device.status == "offline"`
- Operators: `< > <= >= == !=`, combine with `AND`, `OR`
- Actions: `notify("message")`

### DSL Examples (copy & modify):
```bash
neomind rule create --dsl 'RULE high_temp WHEN device.metric(temperature) > 30 DO notify("Temperature exceeded 30C") END'
neomind rule create --dsl 'RULE low_battery WHEN device.metric(battery) < 20 DO notify("Battery below 20%") END'
neomind rule create --dsl 'RULE offline WHEN device.status == "offline" DO notify("Device went offline") END'
neomind rule create --dsl 'RULE critical WHEN device.metric(temperature) > 35 AND device.metric(humidity) < 20 DO notify("Critical: hot and dry") END'
```

> **Rule ≠ Agent**: Rules are event-triggered conditions (IF metric > threshold THEN notify). Agents are LLM-powered scheduled tasks. "定时规则" should use agent with schedule, NOT rule.

---

## Command Reference

All rule commands follow the pattern `neomind rule <action>`.

### List Rules

List all rules in the system.

```bash
neomind rule list
```

### Get Rule Details

Retrieve full details for a specific rule by its ID.

```bash
neomind rule get <ID>
```

- `<ID>` — Required. The rule ID (find IDs via `neomind rule list`).

### Create Rule

Create a new rule. The `--dsl` flag is **required**. The `--name` flag is optional (the DSL may also contain the rule name internally).

```bash
neomind rule create --dsl '<DSL_STRING>' --name '<rule_name>'
```

- `--dsl` — **Required**. The rule definition in DSL format (see DSL section below).
- `--name` — Optional. A human-readable name for the rule.

The command returns the created rule including its assigned ID. The ID is needed for all subsequent operations (enable, test, update, etc.).

### Update Rule

Update an existing rule's name and/or DSL definition.

```bash
neomind rule update <ID> --name '<new_name>'
neomind rule update <ID> --dsl '<new_DSL>'
neomind rule update <ID> --name '<new_name>' --dsl '<new_DSL>'
```

- `<ID>` — Required. The rule ID to update.
- `--name` — Optional. New display name for the rule.
- `--dsl` — Optional. New DSL definition for the rule.
- At least one of `--name` or `--dsl` must be provided.

**Important:** Updating the DSL replaces the entire rule logic. The rule remains in its current enabled/disabled state after update. If the rule was enabled and you change the DSL, it continues running with the new logic immediately.

### Delete Rule

Permanently delete a rule.

```bash
neomind rule delete <ID>
```

- `<ID>` — Required. The rule ID to delete.
- This action is irreversible.

### Enable Rule

Activate a rule so it starts evaluating conditions and firing actions.

```bash
neomind rule enable <ID>
```

- `<ID>` — Required. The rule ID to enable.
- Newly created rules are **disabled by default**. You must enable them explicitly.

### Disable Rule

Deactivate a rule without deleting it. The rule stops evaluating but retains its configuration.

```bash
neomind rule disable <ID>
```

- `<ID>` — Required. The rule ID to disable.

### Test Rule

Dry-run a rule against sample input data. The rule is evaluated but no actions (notifications) are actually sent. Use this to verify a rule works correctly before enabling it.

```bash
neomind rule test <ID> --input '<JSON_DATA>'
```

- `<ID>` — Required. The rule ID to test.
- `--input` — Required. JSON object containing sample metric values to test against.

### Get Execution History

View the execution history of a rule, showing when it was evaluated and whether it triggered.

```bash
neomind rule history <ID>
```

- `<ID>` — Required. The rule ID to query history for.

---

## DSL Format

Rules are defined using a DSL (Domain-Specific Language) with the following structure:

```
RULE <rule_name> WHEN <condition> DO <action> END
```

### Components

| Component | Description |
|-----------|-------------|
| `RULE <name>` | Declares the rule with an internal name (used in logs and history) |
| `WHEN <condition>` | The trigger condition that must be met |
| `DO <action>` | The action to take when the condition is true |
| `END` | Closes the rule definition |

### Conditions

Conditions reference device data and compare it against thresholds:

**Device metric reference:**
```
device.metric(<metric_name>)
```
- References a numeric metric from a device (e.g., `battery`, `temperature`, `humidity`, `cpu`, `memory`).
- The metric name must match the field name the device reports.

**Device status reference:**
```
device.status
```
- References the device's connection status. Values are `"online"` or `"offline"`.

**Comparison operators:**

| Operator | Meaning |
|----------|---------|
| `<` | Less than |
| `>` | Greater than |
| `<=` | Less than or equal |
| `>=` | Greater than or equal |
| `==` | Equal to |
| `!=` | Not equal to |

For numeric metrics, use numeric comparisons:
```
device.metric(temperature) > 35
device.metric(battery) < 20
device.metric(humidity) >= 80
```

For status checks, use string equality:
```
device.status == "offline"
device.status != "online"
```

### Actions

**Notification action:**
```
notify("<message>")
```
- Sends a notification with the given message text.
- Supports template variables for dynamic content:

| Variable | Description |
|----------|-------------|
| `{{device.name}}` | The name of the device that triggered the rule |
| `{{value}}` | The actual metric value that was evaluated |

---

## DSL Examples

### Battery Threshold Alert

Triggers when any device reports battery level below 20%.

```bash
neomind rule create \
  --name 'Low Battery Alert' \
  --dsl 'RULE low_battery WHEN device.metric(battery) < 20 DO notify("Low battery on {{device.name}}: {{value}}%") END'
```

### Temperature Alert

Triggers when temperature exceeds 35 degrees.

```bash
neomind rule create \
  --name 'High Temperature Alert' \
  --dsl 'RULE high_temp WHEN device.metric(temperature) > 35 DO notify("High temperature on {{device.name}}: {{value}} C") END'
```

### Device Offline Notification

Triggers when a device goes offline.

```bash
neomind rule create \
  --name 'Device Offline Alert' \
  --dsl 'RULE device_offline WHEN device.status == "offline" DO notify("Device {{device.name}} went offline") END'
```

### Humidity Warning

Triggers when humidity reaches 80% or above.

```bash
neomind rule create \
  --name 'High Humidity Warning' \
  --dsl 'RULE high_humidity WHEN device.metric(humidity) >= 80 DO notify("High humidity on {{device.name}}: {{value}}%") END'
```

### CPU Usage Alert

Triggers when CPU usage exceeds 90%.

```bash
neomind rule create \
  --name 'CPU Overload Alert' \
  --dsl 'RULE cpu_overload WHEN device.metric(cpu) > 90 DO notify("CPU overload on {{device.name}}: {{value}}%") END'
```

### Device Status Inequality Check

Triggers for any device that is not online (covers offline and unknown states).

```bash
neomind rule create \
  --name 'Device Not Online' \
  --dsl 'RULE not_online WHEN device.status != "online" DO notify("Device {{device.name}} is not online") END'
```

---

## Workflows

### Workflow: Create and Test a Rule Before Enabling

Best practice is to create a rule, test it with sample data, then enable it.

```bash
# Step 1: Create the rule (created in disabled state)
neomind rule create \
  --name 'Battery Monitor' \
  --dsl 'RULE battery_monitor WHEN device.metric(battery) < 20 DO notify("Battery low on {{device.name}}: {{value}}%") END'

# Note the rule ID from the response (e.g., "rule_abc123")

# Step 2: Test the rule with sample data that should trigger
neomind rule test rule_abc123 --input '{"battery": 15}'

# Step 3: Test with data that should NOT trigger (verify no false positives)
neomind rule test rule_abc123 --input '{"battery": 85}'

# Step 4: If tests pass, enable the rule
neomind rule enable rule_abc123

# Step 5: Verify it is active
neomind rule get rule_abc123
```

### Workflow: Debug a Rule That Is Not Firing

If a rule is enabled but does not seem to be triggering, follow these steps.

```bash
# Step 1: Confirm the rule exists and is enabled
neomind rule get <ID>
# Check that "enabled" is true

# Step 2: Test with known data that should trigger
neomind rule test <ID> --input '{"battery": 5}'
# If this does not trigger, the DSL condition may be wrong

# Step 3: Check execution history to see if the rule was evaluated
neomind rule history <ID>
# Look for recent evaluations — if empty, the rule may not be receiving data

# Step 4: Verify the metric name matches what the device reports
# The metric name in device.metric(<name>) must exactly match the device field name
# For example, if the device sends "batteryLevel", you must use device.metric(batteryLevel)

# Step 5: If needed, update the DSL to fix the condition
neomind rule update <ID> --dsl 'RULE battery_alert WHEN device.metric(batteryLevel) < 20 DO notify("Low battery: {{value}}%") END'
```

### Workflow: Update a Rule's Threshold or Logic

To change a rule's behavior without recreating it:

```bash
# Step 1: View the current rule
neomind rule get <ID>

# Step 2: Update the DSL with the new threshold (e.g., change battery from 20% to 30%)
neomind rule update <ID> --dsl 'RULE low_battery WHEN device.metric(battery) < 30 DO notify("Battery low on {{device.name}}: {{value}}%") END'

# Step 3: Test the updated rule
neomind rule test <ID> --input '{"battery": 25}'

# Step 4: The rule remains enabled — no need to re-enable after a DSL update
neomind rule get <ID>
```

You can also update just the name without changing the DSL:

```bash
neomind rule update <ID> --name 'Critical Battery Alert'
```

### Workflow: View Execution History and Audit Rule Activity

```bash
# Step 1: List all rules to find the one you want to audit
neomind rule list

# Step 2: Get detailed history for a specific rule
neomind rule history <ID>

# Step 3: If you see unexpected behavior, disable the rule while investigating
neomind rule disable <ID>

# Step 4: After fixing the issue, re-enable
neomind rule enable <ID>
```

### Workflow: Clean Up Unused Rules

```bash
# Step 1: List all rules to identify ones to remove
neomind rule list

# Step 2: For each rule to remove, first disable it (optional safety step)
neomind rule disable <ID>

# Step 3: Delete the rule permanently
neomind rule delete <ID>
```

---

## Important Notes

- **Rule IDs**: Find rule IDs using `neomind rule list`. All commands that target a specific rule require its ID.
- **New rules are disabled by default**: Always run `neomind rule enable <ID>` after creating a rule if you want it active.
- **DSL is required for create**: The `--dsl` flag is mandatory for `neomind rule create`. The `--name` flag is optional.
- **Test before enabling**: Use `neomind rule test <ID> --input '<JSON>'` to verify rule logic with sample data before enabling it in production.
- **Metric names must match exactly**: The metric name in `device.metric(<name>)` must match the field name the device sends. If a device reports `temp` instead of `temperature`, the rule must use `device.metric(temp)`.
- **Template variables**: Use `{{device.name}}` for the device name and `{{value}}` for the triggering metric value in notify messages.
- **Update preserves state**: Updating a rule's DSL or name does not change its enabled/disabled status. An enabled rule continues running with updated logic immediately.
- **Status values**: Device status uses string values `"online"` and `"offline"`. Always use `==` or `!=` with quoted strings for status comparisons.

## Common Errors & Solutions

- **DSL syntax error on create**: The DSL must follow the exact format `RULE <name> WHEN <condition> DO <action> END`. All four keywords (`RULE`, `WHEN`, `DO`, `END`) are required. Do not omit `END`.
- **"Rule not found"**: Run `neomind rule list` to find valid rule IDs. Use the exact ID returned by list or create.
- **Enable/disable fails**: The rule must exist first. Always run `neomind rule list` to get the ID, then `neomind rule enable <ID>` or `neomind rule disable <ID>`.
- **Rule doesn't trigger**: Verify the metric name in `device.metric(<name>)` matches exactly what the device reports. Run `neomind device latest <ID>` to discover real metric names. Also check the rule is enabled with `neomind rule get <ID>`.
- **Common DSL patterns for reference**:
  - Threshold: `RULE name WHEN device.metric(temperature) > 30 DO notify("msg") END`
  - Status change: `RULE name WHEN device.status == "offline" DO notify("msg") END`
  - Combined: `RULE name WHEN device.metric(cpu) > 90 AND device.metric(memory) > 80 DO notify("msg") END`
- **New rules are disabled by default**: After `neomind rule create`, you MUST run `neomind rule enable <ID>` to activate it.
