# Custom Dashboard Components Guide

**Version**: 1.0.0
**Last Updated**: 2026-05-18

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Quick Start](#quick-start)
4. [manifest.json Reference](#manifest-reference)
5. [bundle.js IIFE Format](#bundle-format)
6. [Component Props API](#props-api)
7. [Styling with CSS Variables](#styling)
8. [Data Source Binding](#data-source)
9. [Installation](#installation)
10. [Using Components in Dashboards](#using-in-dashboards)
11. [Complete Examples](#examples)
12. [Troubleshooting](#troubleshooting)

---

## Overview

Custom dashboard components let you extend NeoMind's visualization capabilities with your own React widgets. Components are **pure frontend** — no Rust backend code needed.

Key characteristics:
- **IIFE JavaScript format** — no build tools required, runs directly in the browser
- **React runtime provided** — uses `window.React` from the dashboard shell
- **CSS variable theming** — automatic light/dark mode support
- **ZIP packaging** — simple `manifest.json` + `bundle.js` structure

---

## Architecture

```
Your Component (ZIP)
├── manifest.json        ← Metadata + config schema
└── bundle.js            ← IIFE React component

Installation Flow:
  ZIP → API upload → data/frontend-components/{id}/
                    → manifest.json + bundle.js on disk

Rendering Flow:
  Dashboard → ComponentRegistry → loads bundle.js via <script>
           → IIFE assigns to window[global_name]
           → ComponentRenderer calls the function with props
```

---

## Quick Start

### 1. Scaffold

```bash
neomind widget create "Temperature Gauge" --widget-type gauge
```

This creates a `temperature-gauge/` directory with template files.

### 2. Edit `manifest.json`

```json
{
  "id": "temperature-gauge",
  "name": { "en": "Temperature Gauge", "zh": "温度表" },
  "description": { "en": "Displays temperature with min/max range" },
  "icon": "thermometer",
  "category": "indicators",
  "global_name": "NeoMindTemperatureGauge",
  "export_name": "default",
  "version": "1.0.0",
  "size_constraints": {
    "min_w": 2, "min_h": 2,
    "default_w": 3, "default_h": 3,
    "max_w": 6, "max_h": 6
  },
  "has_data_source": true,
  "max_data_sources": 1,
  "has_display_config": true,
  "config_schema": {
    "display": {
      "type": "object",
      "properties": {
        "unit": { "type": "string", "description": "Temperature unit (°C, °F)" },
        "minValue": { "type": "number", "description": "Minimum value on gauge" },
        "maxValue": { "type": "number", "description": "Maximum value on gauge" }
      }
    },
    "config": { "type": "object", "properties": {} }
  },
  "default_config": {
    "display": { "unit": "°C", "minValue": -20, "maxValue": 50 }
  }
}
```

### 3. Edit `bundle.js`

```javascript
(function(global) {
  'use strict';
  var React = global.React;

  function TemperatureGauge(props) {
    var value = props.dataSource && props.dataSource[0]
      ? props.dataSource[0].value : null;
    var display = props.display || {};
    var unit = display.unit || '°C';
    var min = display.minValue || -20;
    var max = display.maxValue || 50;
    var pct = value !== null ? Math.max(0, Math.min(100, (value - min) / (max - min) * 100)) : 0;

    return React.createElement('div', {
      style: { width: '100%', height: '100%', display: 'flex',
               flexDirection: 'column', alignItems: 'center',
               justifyContent: 'center', gap: '0.5rem' }
    },
      React.createElement('div', {
        style: { fontSize: '2.5rem', fontWeight: 'bold',
                 color: 'var(--color-text-primary)' }
      }, value !== null ? value.toFixed(1) + unit : '--'),
      React.createElement('div', {
        style: { width: '80%', height: '6px', borderRadius: '3px',
                 background: 'var(--color-border)' }
      },
        React.createElement('div', {
          style: { width: pct + '%', height: '100%', borderRadius: '3px',
                   background: 'var(--color-success)',
                   transition: 'width 0.3s ease' }
        })
      )
    );
  }

  global['NeoMindTemperatureGauge'] = TemperatureGauge;
})(window);
```

### 4. Package and Install

```bash
cd temperature-gauge
zip -r ../temperature-gauge.zip manifest.json bundle.js
neomind widget install ../temperature-gauge.zip
```

### 5. Verify

```bash
neomind widget list                    # Should show temperature-gauge
neomind widget get temperature-gauge   # Check full manifest
```

---

## manifest.json Reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | YES | Unique identifier. Lowercase, hyphens only. Cannot match built-in widget IDs |
| `name` | object/string | YES | Display name. Supports i18n: `{"en": "Name", "zh": "名称"}` |
| `description` | object/string | YES | Widget description. Supports i18n |
| `icon` | string | NO | Lucide icon name (default: "Box") |
| `category` | string | YES | One of: `indicators`, `charts`, `controls`, `display`, `spatial`, `business`, `custom` |
| `global_name` | string | YES | JS global variable name. Convention: `NeoMind{PascalCaseId}` |
| `export_name` | string | NO | Export method (default: "default") |
| `version` | string | NO | Semantic version (default: "1.0.0") |
| `author` | string | NO | Author name |
| `size_constraints` | object | YES | Grid size limits |
| `has_data_source` | boolean | YES | Whether widget accepts data source bindings |
| `max_data_sources` | number | NO | Maximum data sources (0 = none, omit = unlimited) |
| `has_display_config` | boolean | NO | Whether widget has display configuration |
| `has_actions` | boolean | NO | Whether widget sends commands (e.g., toggle) |
| `config_schema` | object | NO | JSON Schema for `display` and `config` fields |
| `default_config` | object | NO | Default configuration values |

### Built-in Widget IDs (reserved, cannot be used)

`value-card`, `led-indicator`, `sparkline`, `progress-bar`, `line-chart`, `area-chart`, `bar-chart`, `pie-chart`, `radar-chart`, `toggle-switch`, `markdown-display`, `image-display`, `image-history`, `web-display`, `map-display`, `video-display`, `custom-layer`, `agent-monitor-widget`, `ai-analyst`

### size_constraints

The dashboard uses a 12-column grid. Specify min/default/max width and height in grid units:

```json
{
  "min_w": 2, "min_h": 2,
  "default_w": 4, "default_h": 3,
  "max_w": 12, "max_h": 8
}
```

### config_schema

Describes the fields your widget accepts. Structure:

```json
{
  "display": {
    "type": "object",
    "properties": {
      "fieldName": {
        "type": "string | number | boolean",
        "description": "Human-readable description of the field"
      }
    }
  },
  "config": {
    "type": "object",
    "properties": {
      "settingName": {
        "type": "string | number | boolean",
        "description": "Description of the setting"
      }
    }
  }
}
```

- `display` — visual configuration set by users in the dashboard editor (unit, color, etc.)
- `config` — internal configuration (content for markdown, URL for web display, etc.)

---

## bundle.js IIFE Format

### Required Structure

```javascript
(function(global) {
  'use strict';

  // Access React runtime provided by NeoMind
  var React = global.React;

  function MyWidget(props) {
    // Your component implementation
    return React.createElement('div', {
      style: { width: '100%', height: '100%' }
    }, 'Hello');
  }

  // Register component on global scope
  // MUST match global_name in manifest.json
  global['NeoMindMyWidget'] = MyWidget;

})(window);
```

### Rules

1. **IIFE only** — no `import`, `require`, or ES modules
2. **`React.createElement` only** — JSX is not available
3. **Use `global.React`** — React is provided by the dashboard shell
4. **Root element fills container** — `width: '100%', height: '100%'`
5. **CSS variables for colors** — use `var(--color-*)` tokens
6. **Match `global_name`** — the global assignment must match manifest
7. **Keep small** — target under 50KB

---

## Component Props API

```typescript
interface WidgetProps {
  config: Record<string, any>;        // Internal config from manifest config_schema
  display: Record<string, any>;       // Display config from manifest config_schema
  dataSource: Array<{                 // Data source values
    value: number | string;           // Current value
    timestamp: number;                // Unix timestamp (ms)
    values?: Array<{                  // Time-series (for charts)
      value: number;
      timestamp: number;
    }>;
  }>;
  id: string;                         // Component instance ID
  title: string;                      // Widget title
  type: string;                       // Widget type
  actions?: {                         // Command actions (if has_actions: true)
    sendCommand: (cmd: string, payload?: any) => void;
  };
}
```

---

## Styling with CSS Variables

Never hardcode colors. Use these design tokens:

| Variable | Usage |
|----------|-------|
| `var(--color-text-primary)` | Primary text |
| `var(--color-text-secondary)` | Secondary text |
| `var(--color-text-muted)` | Muted/hint text |
| `var(--color-bg-primary)` | Main background |
| `var(--color-bg-secondary)` | Card background |
| `var(--color-border)` | Borders |
| `var(--color-success)` | Positive/success |
| `var(--color-error)` | Error/danger |
| `var(--color-warning)` | Warning |
| `var(--color-info)` | Information |
| `var(--color-accent)` | Accent/highlight |

---

## Data Source Binding

When `has_data_source: true`, users bind metrics to your widget. Access the data:

```javascript
// Single value
var currentTemp = props.dataSource[0].value;

// Time-series for charts
var history = props.dataSource[0].values || [];
```

### Multi-source (charts)

For chart widgets with `max_data_sources > 1`, `dataSource` is an array:

```javascript
// Each element is a separate data source/series
props.dataSource.forEach(function(ds, i) {
  var label = ds.label || 'Series ' + (i + 1);
  var points = ds.values || [];
  // render each series...
});
```

---

## Installation

### Method 1: Local ZIP file

```bash
# Create ZIP with manifest.json + bundle.js at root level
cd my-widget && zip -r ../my-widget.zip manifest.json bundle.js

# Install
neomind widget install ../my-widget.zip
```

### Method 2: Marketplace

```bash
# Browse available community components
neomind widget market-list

# Install from marketplace
neomind widget market-install clock
```

### Method 3: Uninstall

```bash
neomind widget uninstall my-widget
```

---

## Using Components in Dashboards

```bash
# Check the component's config_schema first
neomind widget get my-widget

# Add to a dashboard
neomind dashboard update <DASHBOARD_ID> --components '[{
  "id": "c1",
  "type": "my-widget",
  "title": "My Widget Title",
  "position": {"x": 0, "y": 0, "w": 4, "h": 3},
  "data_source": {
    "type": "device",
    "sourceId": "sensor-01",
    "property": "temperature"
  },
  "display": {"unit": "°C"},
  "config": {}
}]'
```

---

## Troubleshooting

| Problem | Cause | Solution |
|---------|-------|----------|
| Widget not in library | IIFE didn't assign to global | Verify `global['{global_name}'] = Component` matches manifest |
| Renders blank | Root not filling container | Add `width: '100%', height: '100%'` to outer div |
| "Reserved ID" error | ID matches built-in | Check `neomind widget list`, choose different ID |
| Data not showing | Wrong data source field | Verify with `neomind device latest <ID>` |
| Colors wrong | Hardcoded CSS | Use `var(--color-*)` variables |
| Install fails | Invalid ZIP structure | ZIP must have `manifest.json` + `bundle.js` at root |
