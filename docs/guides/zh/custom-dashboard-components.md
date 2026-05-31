# 自定义仪表盘组件开发指南

**版本**: 1.0.0
**最后更新**: 2026-05-18

---

## 目录

1. [概述](#概述)
2. [架构设计](#架构设计)
3. [快速开始](#快速开始)
4. [manifest.json 完整参考](#manifest-参考)
5. [bundle.js IIFE 格式](#bundle-格式)
6. [组件 Props 接口](#props-接口)
7. [CSS 变量样式](#css-变量样式)
8. [数据源绑定](#数据源绑定)
9. [安装与卸载](#安装与卸载)
10. [在仪表盘中使用](#在仪表盘中使用)
11. [常见问题](#常见问题)

---

## 概述

自定义仪表盘组件让你可以用 React 编写自己的可视化组件来扩展 NeoMind。组件是**纯前端**的，不需要写任何 Rust 后端代码。

核心特点：
- **IIFE JavaScript 格式** — 无需构建工具，浏览器直接运行
- **React 运行时由平台提供** — 使用 `window.React`
- **CSS 变量主题** — 自动支持亮色/暗色模式
- **ZIP 打包** — 简单的 `manifest.json` + `bundle.js` 结构

---

## 架构设计

```
你的组件 (ZIP 包)
├── manifest.json        ← 元数据 + 配置 schema
└── bundle.js            ← IIFE React 组件

安装流程:
  ZIP → API 上传 → data/frontend-components/{id}/
                   → manifest.json + bundle.js 存储在磁盘

渲染流程:
  仪表盘 → ComponentRegistry → 通过 <script> 标签加载 bundle.js
         → IIFE 将组件赋值到 window[global_name]
         → ComponentRenderer 调用组件函数并传入 props
```

---

## 快速开始

### 1. 创建脚手架

```bash
neomind widget create "温度表" --widget-type gauge
```

此命令创建 `temperature-gauge/` 目录，包含模板文件。

### 2. 编辑 `manifest.json`

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
        "unit": { "type": "string", "description": "温度单位 (°C, °F)" },
        "minValue": { "type": "number", "description": "最小值" },
        "maxValue": { "type": "number", "description": "最大值" }
      }
    },
    "config": { "type": "object", "properties": {} }
  },
  "default_config": {
    "display": { "unit": "°C", "minValue": -20, "maxValue": 50 }
  }
}
```

### 3. 编辑 `bundle.js`

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
    var pct = value !== null
      ? Math.max(0, Math.min(100, (value - min) / (max - min) * 100))
      : 0;

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

### 4. 打包并安装

```bash
cd temperature-gauge
zip -r ../temperature-gauge.zip manifest.json bundle.js
neomind widget install ../temperature-gauge.zip
```

### 5. 验证安装

```bash
neomind widget list                    # 应显示 temperature-gauge
neomind widget get temperature-gauge   # 查看完整 manifest
```

---

## manifest.json 完整参考

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 唯一标识符，小写+连字符。不能与内置组件 ID 冲突 |
| `name` | object/string | 是 | 显示名称，支持国际化：`{"en": "Name", "zh": "名称"}` |
| `description` | object/string | 是 | 组件描述，支持国际化 |
| `icon` | string | 否 | Lucide 图标名称（默认: "Box"） |
| `category` | string | 是 | 分类：`indicators`、`charts`、`controls`、`display`、`spatial`、`business`、`custom` |
| `global_name` | string | 是 | JS 全局变量名。约定格式：`NeoMind{大驼峰ID}` |
| `export_name` | string | 否 | 导出方式（默认: "default"） |
| `version` | string | 否 | 语义化版本号（默认: "1.0.0"） |
| `author` | string | 否 | 作者名称 |
| `size_constraints` | object | 是 | 网格尺寸限制 |
| `has_data_source` | boolean | 是 | 是否接受数据源绑定 |
| `max_data_sources` | number | 否 | 最大数据源数量（0=无，省略=无限制） |
| `has_display_config` | boolean | 否 | 是否有显示配置 |
| `has_actions` | boolean | 否 | 是否发送命令（如开关控制） |
| `config_schema` | object | 否 | `display` 和 `config` 字段的 JSON Schema |
| `default_config` | object | 否 | 默认配置值 |

### 内置组件 ID（保留，不可使用）

`value-card`、`led-indicator`、`sparkline`、`progress-bar`、`line-chart`、`area-chart`、`bar-chart`、`pie-chart`、`radar-chart`、`toggle-switch`、`markdown-display`、`image-display`、`image-history`、`web-display`、`map-display`、`video-display`、`custom-layer`、`agent-monitor-widget`、`ai-analyst`

### size_constraints

仪表盘使用 12 列网格。用网格单位指定最小/默认/最大宽高：

```json
{
  "min_w": 2, "min_h": 2,
  "default_w": 4, "default_h": 3,
  "max_w": 12, "max_h": 8
}
```

### config_schema

描述组件接受的配置字段：

```json
{
  "display": {
    "type": "object",
    "properties": {
      "fieldName": {
        "type": "string | number | boolean",
        "description": "字段的中文描述"
      }
    }
  },
  "config": {
    "type": "object",
    "properties": {
      "settingName": {
        "type": "string | number | boolean",
        "description": "设置项描述"
      }
    }
  }
}
```

- `display` — 用户在仪表盘编辑器中设置的视觉配置（单位、颜色等）
- `config` — 内部配置（Markdown 内容、嵌入 URL 等）

---

## bundle.js IIFE 格式

### 必须的结构

```javascript
(function(global) {
  'use strict';

  // 使用 NeoMind 提供的 React 运行时
  var React = global.React;

  function MyWidget(props) {
    // 组件实现
    return React.createElement('div', {
      style: { width: '100%', height: '100%' }
    }, '你好');
  }

  // 注册组件到全局作用域
  // 必须与 manifest.json 中的 global_name 匹配
  global['NeoMindMyWidget'] = MyWidget;

})(window);
```

### 规则

1. **只能用 IIFE** — 不能用 `import`、`require` 或 ES modules
2. **只能用 `React.createElement`** — JSX 不可用
3. **使用 `global.React`** — React 由仪表盘 Shell 提供
4. **根元素填满容器** — 设置 `width: '100%', height: '100%'`
5. **用 CSS 变量配色** — 使用 `var(--color-*)` 令牌
6. **匹配 `global_name`** — 全局赋值必须与 manifest 一致
7. **保持精简** — 目标 50KB 以内

---

## 组件 Props 接口

```typescript
interface WidgetProps {
  config: Record<string, any>;        // manifest config_schema 中的 config 配置
  display: Record<string, any>;       // manifest config_schema 中的 display 配置
  dataSource: Array<{                 // 数据源值
    value: number | string;           // 当前值
    timestamp: number;                // Unix 时间戳（毫秒）
    values?: Array<{                  // 时间序列（图表组件）
      value: number;
      timestamp: number;
    }>;
  }>;
  id: string;                         // 组件实例 ID
  title: string;                      // 组件标题
  type: string;                       // 组件类型
  actions?: {                         // 命令操作（仅 has_actions: true 时）
    sendCommand: (cmd: string, payload?: any) => void;
  };
}
```

---

## CSS 变量样式

绝不要硬编码颜色。使用设计令牌：

| 变量 | 用途 |
|------|------|
| `var(--color-text-primary)` | 主要文本 |
| `var(--color-text-secondary)` | 次要文本 |
| `var(--color-text-muted)` | 提示文本 |
| `var(--color-bg-primary)` | 主背景 |
| `var(--color-bg-secondary)` | 卡片背景 |
| `var(--color-border)` | 边框 |
| `var(--color-success)` | 成功/正面 |
| `var(--color-error)` | 错误/危险 |
| `var(--color-warning)` | 警告 |
| `var(--color-info)` | 信息 |
| `var(--color-accent)` | 强调/高亮 |

---

## 数据源绑定

当 `has_data_source: true` 时，用户可以将指标绑定到组件。访问数据：

```javascript
// 单值
var currentTemp = props.dataSource[0].value;

// 图表时间序列
var history = props.dataSource[0].values || [];
```

---

## 安装与卸载

### 本地 ZIP 安装

```bash
cd my-widget && zip -r ../my-widget.zip manifest.json bundle.js
neomind widget install ../my-widget.zip
```

### 从市场安装

```bash
neomind widget market-list          # 浏览社区组件
neomind widget market-install clock  # 安装时钟组件
```

### 卸载

```bash
neomind widget uninstall my-widget
```

---

## 在仪表盘中使用

```bash
# 先查看组件的 config_schema
neomind widget get my-widget

# 添加到仪表盘
neomind dashboard update <DASHBOARD_ID> --components '[{
  "id": "c1",
  "type": "my-widget",
  "title": "我的组件标题",
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

## 常见问题

| 问题 | 原因 | 解决方案 |
|------|------|----------|
| 组件不在组件库中 | IIFE 未赋值到全局 | 检查 `global['{global_name}'] = Component` 与 manifest 一致 |
| 渲染空白 | 根元素未填满容器 | 外层 div 添加 `width: '100%', height: '100%'` |
| "保留 ID" 错误 | ID 与内置组件冲突 | 用 `neomind widget list` 查看已有组件，换一个 ID |
| 数据不显示 | 数据源绑定错误 | 用 `neomind device get <ID>` 验证指标名 |
| 颜色不对 | CSS 硬编码 | 使用 `var(--color-*)` 变量 |
| 安装失败 | ZIP 结构错误 | ZIP 根目录必须包含 `manifest.json` + `bundle.js` |
