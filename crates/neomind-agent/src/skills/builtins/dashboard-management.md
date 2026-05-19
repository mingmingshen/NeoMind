---
id: dashboard-management
name: Dashboard Management CLI Commands
category: general
origin: builtin
priority: 70
token_budget: 10000
triggers:
  keywords: [dashboard, 仪表盘, 仪表板, dashboard list, list dashboard, dashboard create, 创建仪表盘, dashboard widget, widget, 组件, 监控面板, monitoring dashboard, 组件, 天气, weather, 电量, battery]
  tool_target:
    - tool: dashboard
      actions: [list, get, create, update, delete, share]
anti_triggers:
  keywords: [rule, 规则, agent, 代理]
---

# Dashboard Management CLI Commands

Use `neomind` CLI commands via the `shell` tool to manage dashboards.

## List Dashboards

```bash
neomind dashboard list
```

## Get Dashboard Details

```bash
neomind dashboard get <ID>
```

## Create Dashboard

```bash
neomind dashboard create --name '<name>'
neomind dashboard create --name 'Battery Monitor' --description 'Battery status for all devices'
```

## Update Dashboard

```bash
neomind dashboard update <ID> --name '<new_name>'
neomind dashboard update <ID> --description '<new_desc>'
```

## Delete Dashboard

```bash
neomind dashboard delete <ID>
```

## Share Dashboard

```bash
neomind dashboard share <ID> --public
neomind dashboard share <ID> --expires 3600
```

## Widget Commands

```bash
neomind widget list                          # List all installed widget types
neomind widget get <ID>                      # Get widget details
neomind widget create <NAME> --widget-type <TYPE>  # Create widget scaffold
neomind widget install <PATH>                # Install from file
neomind widget uninstall <ID>                # Remove widget
neomind widget market-list                   # Browse marketplace
neomind widget market-install <ID>           # Install from marketplace
```

## Adding Components to a Dashboard

Use `--components` to add widgets. The `--layout` only sets grid settings (columns/rows), NOT widgets.

> **Before choosing widget types and configuring components, query the widget metadata:**
> 1. `neomind widget list` — shows all available widget types with size constraints and data source info
> 2. `neomind widget get <type>` — returns the full `config_schema` for a widget, describing accepted `display` and `config` fields
> 3. Use the `config_schema` to populate `display` and `config` correctly when creating dashboard components

### Component JSON Format (array of objects)

Each component object:
```json
{
  "id": "comp_1",
  "type": "value-card",
  "title": "Temperature",
  "position": {"x": 0, "y": 0, "w": 4, "h": 3},
  "data_source": {
    "type": "device",
    "sourceId": "device-123",
    "property": "temperature"
  },
  "display": {"unit": "°C", "format": ".1f"},
  "config": {}
}
```

Required fields: `id`, `type`, `position`
Optional fields: `title`, `data_source`, `display`, `config`, `actions`

### Available Widget Types

**Indicators:** `value-card`, `led-indicator`, `sparkline`, `progress-bar`
**Charts:** `line-chart`, `area-chart`, `bar-chart`, `pie-chart`, `radar-chart`
**Controls:** `toggle-switch`
**Display:** `markdown-display`, `image-display`, `image-history`, `web-display`
**Spatial:** `map-display`, `video-display`, `custom-layer`
**Business:** `agent-monitor-widget`, `ai-analyst`

### DataSource Binding — CRITICAL RULES

> **IMPORTANT: ALWAYS query existing metrics BEFORE binding. NEVER guess or fabricate property names.**
>
> 1. For devices: run `neomind device latest <ID>` or `neomind device get <ID>` to discover available metric names
> 2. For extensions: run `neomind extension info <ID>` to see exposed metrics
> 3. Use `data_source.type` = `device` for device metrics, `extension-metric` for extension metrics
> 4. **Do NOT use `ai-metric` type** — always bind directly to real device/extension metrics
> 5. **Field names MUST match exactly** — device uses `sourceId`+`property`, extension uses `extensionId`+`extensionMetric`

#### Device data source (type: "device")

```json
{
  "type": "device",
  "sourceId": "device-123",
  "property": "battery"
}
```

#### Extension data source (type: "extension-metric")

```json
{
  "type": "extension-metric",
  "extensionId": "weather-forecast-v2",
  "extensionMetric": "temperature_c"
}
```

> **WARNING: Extension data source MUST use `extensionMetric` field (NOT `metricId` or `property`).**

#### Optional fields for time-series data

```json
{
  "type": "device",
  "sourceId": "sensor-01",
  "property": "temperature",
  "timeWindow": {"type": "last_24hours"},
  "aggregateExt": "avg"
}
```

Time windows: `now`, `last_5min`, `last_15min`, `last_30min`, `last_1hour`, `last_6hours`, `last_24hours`, `today`, `this_week`
Aggregates: `raw`, `latest`, `avg`, `min`, `max`, `sum`, `count`, `delta`, `rate`

### Example: Add components to a dashboard

```bash
neomind dashboard update <DASHBOARD_ID> --components '[{"id":"c1","type":"value-card","title":"Temperature","position":{"x":0,"y":0,"w":4,"h":3},"data_source":{"type":"device","sourceId":"sensor-01","property":"temperature"},"display":{"unit":"°C"}},{"id":"c2","type":"value-card","title":"Humidity","position":{"x":4,"y":0,"w":4,"h":3},"data_source":{"type":"device","sourceId":"sensor-01","property":"humidity"},"display":{"unit":"%"}}]'
```

### Example: Markdown display widget (static content)

```bash
neomind dashboard update <DASHBOARD_ID> --components '[{"id":"c1","type":"markdown-display","title":"Weather Report","position":{"x":0,"y":0,"w":12,"h":4},"config":{"content":"# Weather\\n\\nTemperature: 28°C\\nHumidity: 58%"}}]'
```

### Example: Line chart with time series data

```bash
neomind dashboard update <DASHBOARD_ID> --components '[{"id":"c1","type":"line-chart","title":"Temperature History","position":{"x":0,"y":0,"w":8,"h":4},"data_source":{"type":"device","sourceId":"sensor-01","property":"temperature","timeWindow":{"type":"last_24hours"},"aggregateExt":"avg"},"display":{"unit":"°C","showLegend":true}}]'
```

## Common Workflows — ALWAYS follow these steps

> **Before creating ANY dashboard with data-bound components, you MUST:**
> 1. Run `neomind device list` or `neomind extension list` to find entity IDs
> 2. Run `neomind device latest <ID>` or `neomind extension info <ID>` to discover available metrics
> 3. Use the REAL metric names from step 2 — device metrics go in `property`, extension metrics go in `extensionMetric`
> 4. Do NOT fabricate or guess metric names — if you can't find a metric, tell the user
> 5. Do NOT use `ai-metric` as a middle layer — bind device/extension metrics directly

### Create a battery monitoring dashboard for all devices

```bash
# Step 1: List devices to get IDs
neomind device list

# Step 2: Check available metrics for each device (CRITICAL - do not skip)
neomind device latest <DEVICE1_ID>
neomind device latest <DEVICE2_ID>

# Step 3: Create dashboard
neomind dashboard create --name 'Battery Monitor'

# Step 4: Add components using REAL metric names from step 2
neomind dashboard update <DASHBOARD_ID> --components '[{"id":"batt_1","type":"value-card","title":"Sensor 1 Battery","position":{"x":0,"y":0,"w":4,"h":3},"data_source":{"type":"device","sourceId":"<DEVICE1_ID>","property":"battery"},"display":{"unit":"%","format":".0f"}},{"id":"batt_2","type":"value-card","title":"Sensor 2 Battery","position":{"x":4,"y":0,"w":4,"h":3},"data_source":{"type":"device","sourceId":"<DEVICE2_ID>","property":"battery"},"display":{"unit":"%","format":".0f"}}]'

# Step 5: Add a line chart showing all batteries over time
neomind dashboard update <DASHBOARD_ID> --components '[{"id":"batt_chart","type":"line-chart","title":"Battery Trends","position":{"x":0,"y":3,"w":12,"h":4},"data_source":[{"type":"device","sourceId":"<DEVICE1_ID>","property":"battery","timeWindow":{"type":"last_24hours"}},{"type":"device","sourceId":"<DEVICE2_ID>","property":"battery","timeWindow":{"type":"last_24hours"}}],"display":{"unit":"%","showLegend":true}}]'
```

## Notes

- Dashboard IDs can be found via `neomind dashboard list`
- Use `neomind dashboard get <ID>` to inspect current components and layout
- `--components` replaces ALL components — include all existing + new ones in one update
- `--layout` is only for grid settings (columns/rows), NOT for adding widgets
- Widget types available: check `neomind widget list` for currently installed types
- Each widget's `config_schema` describes its accepted `display` and `config` fields — use `neomind widget get <type>` to inspect
- **CRITICAL**: DataSource field names MUST be exact — device uses `sourceId` + `property`, extension uses `extensionId` + `extensionMetric`. Do NOT use `metricId` or `property` for extensions.
- **CRITICAL**: DataSource `property`/`extensionMetric` value MUST be a real metric name discovered via `neomind device latest <ID>` or `neomind extension info <ID>`. NEVER guess metric names.
- **Do NOT use `ai-metric` as data source type** — always bind directly to real device metrics (`device`) or extension metrics (`extension-metric`)
- Charts (`line-chart`, `area-chart`, `bar-chart`) accept an array in `data_source` for multiple series
- Grid is 12 columns wide

## Common Errors & Solutions

- **"Dashboard not found"**: Run `neomind dashboard list` to find valid dashboard IDs. Use the exact ID from the output.
- **Update with components fails**: The `--components` flag requires a valid JSON array. Use single quotes around the JSON string. Run `neomind widget list` to verify widget types exist.
- **"Invalid widget type"**: Widget types must match an entry from `neomind widget list` exactly. Built-in types include `value-card`, `line-chart`, `bar-chart`, `toggle-switch`, etc. Custom widgets use their registered ID.
- **Layout position invalid**: Position must be an object with `x`, `y`, `w`, `h` keys. Grid is 12 columns wide. Values must be non-negative integers. Components must not overlap.
- **Share fails with "dashboard not found"**: The dashboard ID must exist. Run `neomind dashboard list` to verify, then retry the share command.
- **`--components` replaces all components**: When adding new widgets, you must include ALL existing components plus the new ones in a single update. Check current components with `neomind dashboard get <ID>` first.
