---
id: transform-management
name: Transform Management & Data Processing
category: transform
origin: builtin
priority: 80
token_budget: 10000
triggers:
  keywords: [transform, 转换, 数据转换, data transform, 计算, calculate, formula, 公式, virtual metric, 虚拟指标, 转换数据, process data, 数据处理, 脚本, script, JS code, javascript, 华氏, 摄氏, 单位转换, unit conversion, transform create, transform test]
  tool_target:
    - tool: transform
      actions: [list, get, create, update, delete, test, metrics, data-sources]
anti_triggers:
  keywords: [dashboard, 仪表盘, agent, 代理, extension develop, 扩展开发, device connect, 设备连接, rule, 规则]
---

# Transform Management & Data Processing

Transforms process raw metric data into derived values using JavaScript code. They create virtual metrics that can be used in dashboards just like device data.

## CRITICAL: Transform Code Format

Transforms receive `input` as the **raw metric value** (not an object). Must `return` the result.

```javascript
// Simple calculation
return (input - 32) * 5 / 9

// Classification
if (input > 80) return "good"
if (input > 20) return "ok"
return "low"

// Math function
return Math.round(input * 100) / 100
```

**Rules:**
- `input` is a single value (number, string, boolean)
- Must use `return` to output the result
- No imports, no external libraries — plain JS only
- No async/await — synchronous code only

## Scope Levels

| Scope | Syntax | Applies To |
|-------|--------|-----------|
| Global | `global` | All device metrics with matching name |
| Device Type | `device_type:<type_id>` | All devices of a specific type |
| Device | `device:<device_id>` | Only one specific device |

**Output DataSourceId**: `transform:<output_prefix>:<field>`

Use this ID when binding to dashboards:
```json
{"type":"extension-metric","extensionId":"transform","extensionMetric":"<output_prefix>.<field>"}
```

## Command Reference

### Create Transform

```bash
neomind transform create --name '<name>' --scope <scope> --code '<JS>'
```

Required: `--name`, `--code`
Optional: `--scope` (default: `global`), `--output-prefix`, `--description`, `--enabled`

### Test Before Creating

```bash
# Test code with sample input
neomind transform test --code 'return (input - 32) * 5 / 9' --input '{"value": 212}'
```

**Always test before creating** to verify the code works correctly.

### List & Inspect

```bash
neomind transform list                  # List all transforms
neomind transform get <ID>              # Get transform details
neomind transform metrics               # List all virtual metrics produced by transforms
neomind transform data-sources          # List available data sources for transforms
```

### Update & Delete

```bash
neomind transform update <ID> --code '<NEW_JS>'    # Update code
neomind transform update <ID> --enabled true        # Enable/disable
neomind transform delete <ID>                       # Delete transform
```

## Workflow Examples

### Temperature Unit Conversion (F to C)

```bash
# Step 1: Test the code
neomind transform test --code 'return (input - 32) * 5 / 9' --input '{"value": 212}'

# Step 2: Create the transform
neomind transform create --name 'Fahrenheit to Celsius' \
  --scope global \
  --code 'return (input - 32) * 5 / 9'

# Step 3: Verify virtual metric exists
neomind transform metrics
```

### Battery Health Classification

```bash
neomind transform test --code 'if (input > 80) return "good"; if (input > 20) return "ok"; return "low"' --input '{"value": 15}'

neomind transform create --name 'Battery Health' \
  --scope global \
  --output-prefix battery \
  --code 'if (input > 80) return "good"; if (input > 20) return "ok"; return "low"'
```

### Device-Specific Transform

```bash
neomind transform create --name 'Sensor-001 Calibration' \
  --scope 'device:sensor-001' \
  --output-prefix calibrated \
  --code 'return input * 1.02 + 0.5'
```

### Percentage Formatter

```bash
neomind transform create --name 'Percentage Format' \
  --scope global \
  --code 'return Math.round(input * 100) / 100 + "%"'
```

### Status Mapper

```bash
neomind transform create --name 'Status Text' \
  --scope global \
  --code 'const map = {0: "offline", 1: "online", 2: "warning"}; return map[input] || "unknown"'
```

## Using Transform Output in Dashboards

After creating a transform, its output appears as a virtual metric. Use it in dashboard components:

```bash
# Discover available virtual metrics
neomind transform metrics

# Add to dashboard (transform output uses extension-metric binding)
neomind dashboard add-components <DASHBOARD_ID> --components '[{
  "id": "t1",
  "type": "value-card",
  "title": "Battery Status",
  "position": {"x": 0, "y": 0, "w": 4, "h": 2},
  "data_source": {
    "type": "extension-metric",
    "extensionId": "transform",
    "extensionMetric": "battery.health"
  }
}]'
```

## Common Errors & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| "Transform code error" | JS syntax error | Test with `transform test --code '...' --input '...'` first |
| "Unexpected token return" | Missing semicolons or bad syntax | Use simple one-liner expressions |
| No virtual metric appears | Transform disabled or scope mismatch | Check `transform get <ID>` for enabled status and scope |
| "input is not defined" | Using wrong variable name | Must use `input` (not `inputs` or `value`) |
| Dashboard shows no data | Wrong DataSourceId binding | Check `transform metrics` for correct output names |
| Transform runs but output wrong | Logic error in code | Test with known input values via `transform test` |
