---
id: transform-management
name: Transform Management CLI Commands
category: general
origin: builtin
priority: 70
token_budget: 10000
triggers:
  keywords: [transform, 变换, 数据变换, virtual metric, 虚拟指标, transform list, transform test, test code, data source, 数据源, 计算, compute, derive, 派生, output prefix, js_code, scope, scope format]
  tool_target:
    - tool: transform
      actions: [list, get, create, update, delete, test, metrics, data-sources]
anti_triggers:
  keywords: [device, 设备, rule, 规则]
---

# Transform Management CLI Commands

Use `neomind` CLI commands via the `shell` tool to manage data transforms. Transforms process raw telemetry data into virtual metrics using JavaScript.

## ⚠️ CRITICAL: Common Operations Checklist

| User Request | Command |
|---|---|
| 列出转换 | `neomind transform list` |
| 查看详情 | `neomind transform get <ID>` |
| 创建转换 | `neomind transform create --name 'NAME' --scope global --code 'return value * 2'` |
| 删除转换 | `neomind transform delete <ID>` |
| 更新代码 | `neomind transform update <ID> --code 'NEW CODE'` |

> **Transform ≠ Rule**: Transforms process data (unit conversion, scaling). Rules trigger alerts on conditions.

## Quick Code Examples:
```bash
# Fahrenheit to Celsius
neomind transform create --name 'F to C' --scope global --code 'return (value - 32) * 5/9'

# Scale by 0.01
neomind transform create --name 'Scale' --scope global --code 'return value * 0.01' --output-prefix 'scaled_'

# Percentage to decimal
neomind transform create --name 'Pct to Decimal' --scope global --code 'return value / 100'
```

## Commands Reference

### List All Transforms

```bash
neomind transform list
```

Returns all configured transforms with their IDs, names, scopes, enabled status, and definitions.

### Get Transform Details

```bash
neomind transform get <ID>
```

Returns full details for a specific transform including its JavaScript code, scope, and output prefix.

### Create Transform

```bash
neomind transform create \
  --name '<name>' \
  --scope '<scope>' \
  --code '<js_code>' \
  [--output-prefix '<prefix>'] \
  [--description '<desc>'] \
  [--enabled <true|false>]
```

**Required flags:**
- `--name` — Human-readable transform name
- `--scope` — Determines which telemetry data the transform applies to
- `--code` — JavaScript code body. Use `input` to access the raw metric value. Must `return` the result.

**Optional flags:**
- `--output-prefix` — Prefix for the virtual metric name. Defaults to `"transform"` if omitted. The virtual metric DataSourceId will be `transform:<output_prefix>:<original_field>`.
- `--description` — Description of what the transform does
- `--enabled` — `true` or `false`. Defaults to `true` if omitted.

**Scope formats:**
- `global` — Applies to all devices
- `device_type:<TypeName>` — Applies only to devices of the specified type (e.g., `device_type:TemperatureSensor`)
- `device:<DeviceId>` — Applies only to a specific device (e.g., `device:Sensor01`)

**API body structure** (created internally):
```json
{
  "name": "<name>",
  "description": "<description>",
  "enabled": true,
  "type": "transform",
  "definition": {
    "scope": "<scope>",
    "js_code": "<code>",
    "output_prefix": "<output_prefix>"
  }
}
```

### Update Transform

```bash
neomind transform update <ID> \
  [--name '<new_name>'] \
  [--description '<new_desc>'] \
  [--code '<new_js_code>'] \
  [--scope '<new_scope>'] \
  [--output-prefix '<new_prefix>'] \
  [--enabled <true|false>]
```

All flags are optional — only the fields you specify will be changed. The transform ID is required as a positional argument.

### Delete Transform

```bash
neomind transform delete <ID>
```

Permanently removes the transform and its associated virtual metrics.

### Test Transform Code

```bash
neomind transform test --code '<js_code>' --input '<json_object>'
```

Tests JavaScript transform code without saving it. The `--input` flag accepts a JSON object that simulates the incoming telemetry data. Use this to validate your transform logic before creating or updating a transform.

**Flags:**
- `--code` — JavaScript code to test (required)
- `--input` — JSON object simulating input data (required). Defaults to `{}` if not provided.

### List Virtual Metrics

```bash
neomind transform metrics
```

Lists all virtual metrics produced by active transforms. Each virtual metric has a DataSourceId in the format `transform:<output_prefix>:<field_name>` and can be used in dashboards.

### List Data Sources

```bash
neomind transform data-sources
```

Lists all available data sources that transforms can consume. Use this to discover which raw metrics exist before writing transform code.

## JavaScript Code Examples

### Simple Math: Fahrenheit to Celsius

```bash
neomind transform create \
  --name 'Fahrenheit to Celsius' \
  --scope 'global' \
  --code 'return (input - 32) * 5 / 9' \
  --output-prefix 'celsius' \
  --description 'Convert Fahrenheit to Celsius'
```

Test it first:
```bash
neomind transform test --code 'return (input - 32) * 5 / 9' --input '{"value": 98.6}'
```

### Conditional Classification: Battery Health

```bash
neomind transform create \
  --name 'Battery Health' \
  --scope 'global' \
  --code 'if (input > 80) return "good"; if (input > 20) return "ok"; return "low"' \
  --output-prefix 'battery_health' \
  --description 'Classify battery level into health categories'
```

Test it:
```bash
neomind transform test --code 'if (input > 80) return "good"; if (input > 20) return "ok"; return "low"' --input '{"value": 45}'
```

### String Formatting: Status Label

```bash
neomind transform create \
  --name 'Device Status Label' \
  --scope 'device_type:Gateway' \
  --code 'const s = Number(input); if (s === 1) return "Online"; if (s === 0) return "Offline"; return "Unknown"' \
  --output-prefix 'status_label' \
  --description 'Convert numeric status code to readable label'
```

### Complex Calculation: Heat Index

```bash
neomind transform create \
  --name 'Heat Index' \
  --scope 'device_type:WeatherStation' \
  --code 'const T = input.temperature; const H = input.humidity; if (T < 80) return T; const hi = -42.379 + 2.04901523*T + 10.14333127*H - 0.22475541*T*H - 6.83783e-3*T*T - 5.481717e-2*H*H + 1.22874e-3*T*T*H + 8.5282e-4*T*H*H - 1.99e-6*T*T*H*H; return Math.round(hi * 100) / 100' \
  --output-prefix 'heat_index' \
  --description 'Calculate heat index from temperature and humidity'
```

### Rounding and Precision

```bash
neomind transform create \
  --name 'Round to 2 Decimals' \
  --scope 'global' \
  --code 'return Math.round(input * 100) / 100' \
  --output-prefix 'rounded' \
  --description 'Round raw value to 2 decimal places'
```

### Unit Conversion: PSI to kPa

```bash
neomind transform create \
  --name 'PSI to kPa' \
  --scope 'device_type:PressureSensor' \
  --code 'return Math.round(input * 6.89476 * 100) / 100' \
  --output-prefix 'kpa' \
  --description 'Convert PSI to kilopascals'
```

### Percentage Calculation

```bash
neomind transform create \
  --name 'Usage Percentage' \
  --scope 'global' \
  --code 'return Math.min(100, Math.max(0, Math.round(input)))' \
  --output-prefix 'usage_pct' \
  --description 'Clamp value to 0-100 percentage range'
```

## Workflows

### Workflow 1: Test Transform Code Before Creating

Always test your JavaScript logic before creating a transform to avoid producing incorrect virtual metrics.

```bash
# Step 1: Test the code with sample input
neomind transform test --code 'return (input - 32) * 5 / 9' --input '{"value": 212}'

# Step 2: Verify the result is correct (should be 100)
# If the output looks correct, create the transform

# Step 3: Create the transform
neomind transform create \
  --name 'F to C' \
  --scope 'global' \
  --code 'return (input - 32) * 5 / 9' \
  --output-prefix 'celsius' \
  --description 'Convert Fahrenheit to Celsius'
```

### Workflow 2: Create a Virtual Metric and Verify It

```bash
# Step 1: List available data sources to understand the raw metrics
neomind transform data-sources

# Step 2: Test the transform logic
neomind transform test --code 'if (input > 80) return "good"; if (input > 20) return "ok"; return "low"' --input '{"value": 50}'

# Step 3: Create the transform
neomind transform create \
  --name 'Battery Health' \
  --scope 'global' \
  --code 'if (input > 80) return "good"; if (input > 20) return "ok"; return "low"' \
  --output-prefix 'battery_health'

# Step 4: Verify the virtual metric was registered
neomind transform metrics

# Step 5: Get the transform details to confirm configuration
neomind transform get <ID>
```

### Workflow 3: Update an Existing Transform

```bash
# Step 1: List transforms to find the ID
neomind transform list

# Step 2: Get current details to review
neomind transform get <ID>

# Step 3: Test the updated code first
neomind transform test --code 'if (input > 90) return "excellent"; if (input > 50) return "good"; if (input > 20) return "ok"; return "low"' --input '{"value": 75}'

# Step 4: Apply the update
neomind transform update <ID> --code 'if (input > 90) return "excellent"; if (input > 50) return "good"; if (input > 20) return "ok"; return "low"'

# Step 5: Optionally update other fields
neomind transform update <ID> --description 'Updated: 4-tier battery health classification'

# Step 6: Verify the change
neomind transform get <ID>
```

### Workflow 4: List Data Sources to Understand What Is Available

```bash
# List all data sources that transforms can consume
neomind transform data-sources

# This returns DataSourceId entries like:
#   device:Sensor01:temperature
#   device:Sensor01:humidity
#   extension:weather:temp
#
# Use these field names when designing transform logic.
# The `input` variable in your JS code receives the value of the matched field.
```

### Workflow 5: Use Virtual Metric in Dashboard

```bash
# Step 1: Create the transform
neomind transform create \
  --name 'F to C' \
  --scope 'global' \
  --code 'return (input - 32) * 5 / 9' \
  --output-prefix 'celsius'

# Step 2: Verify the virtual metric DataSourceId
neomind transform metrics
# Look for: transform:celsius:<field_name>

# Step 3: Use the virtual metric DataSourceId in a dashboard widget
# The DataSourceId format is: transform:<output_prefix>:<original_field>
# For example: transform:celsius:temperature
# This can be used as a data source when creating dashboard widgets.
```

## Important Notes

- Transforms run JavaScript. The `input` variable contains the raw metric value. Your code **must** use `return` to output the result.
- `output_prefix` defaults to `"transform"` if not specified, producing DataSourceIds like `transform:transform:<field>`.
- Scope determines which devices' telemetry feeds into the transform. Use `global` for all devices, or target specific device types or individual devices.
- Transform type is always `"transform"` in the API body — this is set automatically by the CLI.
- Use `neomind transform test` to validate code before creating or updating. This prevents broken virtual metrics from entering the system.
- Virtual metrics produced by transforms are queryable like any other data source and can be displayed on dashboards.
- To find a transform's ID, use `neomind transform list`.
- When updating, only the flags you specify are changed — all other fields remain unchanged.

## Common Errors & Solutions

- **"Transform not found"**: Run `neomind transform list` to find valid transform IDs. Use the exact ID from the output.
- **Create fails with missing fields**: Both `--name` and `--code` are required. The `--scope` flag is also required (use `global` if unsure). The code must be a valid JavaScript function body that includes a `return` statement.
- **JS code produces errors**: Always test code first with `neomind transform test --code '<js>' --input '{"value": 42}'` before creating. Common issues: missing `return`, using `value` instead of `input` (the variable is `input`), syntax errors in complex expressions.
- **Scope format is wrong**: Valid scopes are `global`, `device_type:<TypeName>`, or `device:<DeviceId>`. Do not use arbitrary strings. Check device type names with `neomind device types list`.
- **Data source not found**: DataSourceId format is `{type}:{id}:{field}`. For transforms, the output format is `transform:<output_prefix>:<field>`. Use `neomind transform data-sources` to list available sources.
- **Transform not producing output**: Check that the transform is enabled (`neomind transform get <ID>`). Also verify the scope matches devices that actually report telemetry data.
