---
id: component-development
name: Custom Dashboard Component Development
category: general
origin: builtin
priority: 75
token_budget: 10000
triggers:
  keywords: [widget, component, IIFE, bundle.js, manifest.json, 自定义组件, 组件开发, widget create, widget install, widget build, create widget, install widget, custom widget, custom component, component scaffold, bundle, frontend component]
  tool_target:
    tool: widget
    actions: [create, install, uninstall, list, get]
anti_triggers:
  keywords: [rule, 规则, message, 消息, memory, 记忆]
---

# Custom Dashboard Component Development

Custom components are pure frontend IIFE JavaScript bundles using React runtime from NeoMind dashboard shell. No build tools needed.

## Workflow: scaffold → edit → install → use

### 1. Scaffold
```bash
neomind widget create "My Widget" --widget-type <TYPE>
# Types: chart, gauge, stat, table, image, custom
# Output: data/frontend-components/{widget-id}/
```

### 2. File Structure (auto-created under data/frontend-components/)
```
data/frontend-components/my-widget/
├── manifest.json    # Metadata + config schema
└── bundle.js        # IIFE React component
```

### 3. manifest.json (required fields)

| Field | Description |
|-------|-------------|
| `id` | Unique lowercase-hyphen ID. Cannot match built-ins (value-card, line-chart, etc.) |
| `global_name` | JS global var. Convention: `NeoMind{PascalCase}` |
| `category` | indicators, charts, controls, display, spatial, business, custom |
| `size_constraints` | `{min_w, min_h, default_w, default_h, max_w, max_h}`. Grid is 12 columns |
| `has_data_source` | Boolean. Whether widget accepts data bindings |
| `max_data_sources` | Number. 0=none, omit=unlimited |
| `config_schema` | JSON Schema with `display` and `config` sections |

### 4. bundle.js — IIFE (REQUIRED FORMAT)

```javascript
(function(global) {
  'use strict';
  var React = global.React;
  function MyWidget(props) {
    return React.createElement('div', {
      style: { width: '100%', height: '100%' }
    }, 'Hello');
  }
  global['NeoMindMyWidget'] = MyWidget;
})(window);
```

**RULES:**
1. **React.createElement only** — no JSX, no import/require
2. **global.React** — provided by dashboard shell
3. **Root fills container** — `width: '100%', height: '100%'`
4. **CSS variables only** — `var(--color-text-primary)`, `var(--color-bg-secondary)`, `var(--color-border)`, `var(--color-success)`, `var(--color-error)`, `var(--color-warning)`, `var(--color-accent)`
5. **global['{global_name}']** — must match manifest
6. **Under 50KB**

### 5. Props API

```
props.config      — config section values from config_schema
props.display     — display section values from config_schema
props.dataSource  — [{value, timestamp, values?: [{value, timestamp}]}]
props.id          — instance ID
props.title       — widget title
props.actions?.sendCommand(cmd, payload) — if has_actions: true
```

### 6. Install & Use

```bash
cd data/frontend-components/my-widget && zip -r ../my-widget.zip manifest.json bundle.js
neomind widget install ../my-widget.zip
neomind widget list    # verify
neomind widget get my-widget  # check config_schema

# Add to dashboard
neomind dashboard update <ID> --components '[{
  "id": "c1", "type": "my-widget",
  "title": "Title",
  "position": {"x": 0, "y": 0, "w": 4, "h": 3},
  "display": {"unit": "°C"}, "config": {}
}]'
```

## Common Errors & Solutions

- **"Invalid widget type"**: Valid scaffold types are `chart`, `gauge`, `stat`, `table`, `image`, `custom`. Other types will be rejected at the scaffold step.
- **Install fails**: The zip file must contain both `manifest.json` and `bundle.js` at the root level. Files nested in subdirectories inside the zip will not be found.
- **Manifest validation fails**: The `manifest.json` must include `id`, `name`, `description`, and `global_name` as top-level fields. The `id` must be lowercase-hyphen format and must not collide with built-in widget types (e.g., `value-card`, `line-chart`).
- **Bundle.js not loading**: The bundle MUST be an IIFE that assigns to `window[global_name]`. The `global_name` in the assignment must exactly match the `global_name` field in `manifest.json`. No `import`, `require`, or JSX is allowed.
- **Widget renders blank**: Verify the component fills its container with `width: '100%', height: '100%'`. Check that `global.React` is used (provided by the dashboard shell), not imported. Use `React.createElement` exclusively.
- **Config schema mismatch**: The `display` and `config` sections in `config_schema` define what the dashboard passes via `props.display` and `props.config`. If the widget ignores these props, the configuration UI will appear to have no effect.
