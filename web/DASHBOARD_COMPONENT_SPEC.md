# Dashboard 组件规范

本文档定义了 NeoTalk Dashboard 组件系统的统一规范。

## 目录

1. [组件分类](#组件分类)
2. [组件元数据](#组件元数据)
3. [尺寸规范](#尺寸规范)
4. [样式规范](#样式规范)
5. [配置规范](#配置规范)
6. [数据源规范](#数据源规范)
7. [实现规范](#实现规范)

---

## 1. 组件分类

### 分类体系

| 分类 | 说明 | 组件示例 |
|------|------|----------|
| `indicators` | 指标显示 | value-card, led-indicator, sparkline, progress-bar |
| `charts` | 图表 | line-chart, area-chart, bar-chart, pie-chart |
| `controls` | 交互控制 | toggle-switch, button-group, slider, dropdown, input-field |
| `display` | 内容展示 | image-display, image-history, web-display, markdown-display |
| `spatial` | 空间与媒体 | map-display, video-display, custom-layer |

---

## 2. 组件元数据

### ComponentMeta 接口

每个组件必须在 `registry.ts` 中注册元数据：

```typescript
interface ComponentMeta {
  // 基础信息
  type: ComponentType           // 组件类型标识 (kebab-case)
  name: string                  // 显示名称
  description: string           // 简短描述
  category: ComponentCategory   // 所属分类

  // 显示
  icon: React.ComponentType     // Lucide 图标

  // 尺寸约束
  sizeConstraints: ComponentSizeConstraints

  // 能力标识
  hasDataSource: boolean        // 是否支持数据绑定
  maxDataSources?: number       // 最大数据源数量 (默认1)
  hasDisplayConfig: boolean     // 是否支持样式配置
  hasActions: boolean           // 是否支持动作

  // Props 验证
  acceptsProp: (prop: string) => boolean

  // 默认配置
  defaultProps?: Record<string, unknown>
  variants?: string[]           // 可选变体
}
```

### 尺寸约束规范

```typescript
interface ComponentSizeConstraints {
  minW: number      // 最小宽度 (网格单位)
  minH: number      // 最小高度 (网格单位)
  defaultW: number  // 默认宽度
  defaultH: number  // 默认高度
  maxW: number      // 最大宽度
  maxH: number      // 最大高度
  preserveAspect?: boolean  // 是否保持宽高比
}
```

### 尺寸推荐值

| 分类 | minW | minH | defaultW | defaultH | 说明 |
|------|------|------|----------|----------|------|
| **Indicators** | | | | | |
| value-card | 2 | 1 | 3 | 2 | 数值卡片 |
| led-indicator | 1 | 1 | 2 | 1 | LED 指示灯 (保持比例) |
| sparkline | 2 | 1 | 4 | 2 | 迷你趋势图 |
| progress-bar | 2 | 1 | 4 | 1 | 进度条 |
| **Charts** | | | | | |
| line-chart | 3 | 2 | 6 | 4 | 折线图 |
| area-chart | 3 | 2 | 6 | 4 | 面积图 |
| bar-chart | 3 | 2 | 6 | 4 | 柱状图 |
| pie-chart | 2 | 2 | 4 | 4 | 饼图 (保持比例) |
| **Controls** | | | | | |
| toggle-switch | 1 | 1 | 2 | 1 | 开关 |
| button-group | 2 | 1 | 3 | 1 | 按钮组 |
| slider | 2 | 1 | 3 | 1 | 滑块 |
| dropdown | 2 | 1 | 3 | 1 | 下拉选择 |
| input-field | 2 | 1 | 3 | 1 | 输入框 |
| **Display** | | | | | |
| image-display | 2 | 2 | 4 | 3 | 图片显示 |
| image-history | 3 | 3 | 6 | 4 | 历史图片 |
| web-display | 3 | 3 | 6 | 4 | 网页内嵌 |
| markdown-display | 2 | 2 | 4 | 3 | Markdown |
| **Spatial** | | | | | |
| map-display | 3 | 3 | 6 | 5 | 地图 |
| video-display | 3 | 2 | 6 | 4 | 视频 |
| custom-layer | 2 | 2 | 6 | 4 | 自定义层 |

---

## 3. 尺寸规范

### 组件尺寸层级

| 尺寸 | padding | title | label | value | icon | 适用场景 |
|------|---------|-------|-------|-------|------|----------|
| `xs` | p-2 | text-xs | text-[10px] | text-xs | w-3 | 1x1 网格超小卡片 |
| `sm` | p-3 | text-sm | text-xs | text-sm | w-4 | 紧凑布局 |
| `md` | p-4 | text-sm | text-xs | text-base | w-4 | 标准布局 (默认) |
| `lg` | p-5 | text-base | text-sm | text-lg | w-5 | 大尺寸强调 |

### 统一样式类

```typescript
// 基础卡片样式 (所有组件必须使用)
import { dashboardCardBase, dashboardComponentSize } from '@/design-system/tokens/size'

export function MyComponent({ size = 'md', className }: Props) {
  const config = dashboardComponentSize[size]

  return (
    <div className={cn(dashboardCardBase, config.padding, className)}>
      {/* 内容 */}
    </div>
  )
}
```

---

## 4. 样式规范

### 颜色系统

使用 OKLCH 色彩空间保证感知一致性：

```typescript
// 图表颜色
chartColors = {
  1: 'oklch(0.646 0.222 264.38)',  // 蓝紫
  2: 'oklch(0.646 0.222 142.5)',   // 绿
  3: 'oklch(0.646 0.222 48.85)',   // 黄
  4: 'oklch(0.646 0.222 24.85)',   // 橙
  5: 'oklch(0.646 0.222 304.38)',  // 粉
  6: 'oklch(0.646 0.222 188.38)',  // 青
}

// 状态颜色
statusColors = {
  success: 'oklch(0.646 0.222 142.5)',  // 在线
  warning: 'oklch(0.646 0.222 85.85)',   // 警告
  error: 'oklch(0.576 0.222 25.85)',     // 错误
  info: 'oklch(0.646 0.222 264.38)',     // 信息
  neutral: 'oklch(0.551 0.0 264.38)',    // 中性
}
```

### 状态映射

| 语义状态 | 颜色 |
|----------|------|
| 设备在线 | success |
| 设备离线 | neutral |
| 运行中 | info |
| 已暂停 | warning |
| 已完成 | success |
| 失败 | error |
| 趋势上升 | success |
| 趋势下降 | error |

---

## 5. 配置规范

### ConfigSchema 结构

```typescript
interface ComponentConfigSchema {
  title?: string
  // 分离式配置 (三栏布局)
  dataSourceSections?: ConfigSection[]   // 数据源配置
  styleSections?: ConfigSection[]         // 样式配置
  displaySections?: ConfigSection[]       // 显示配置
  // 兼容旧版
  sections?: ConfigSection[]
}
```

### 可用配置段类型

| 类型 | 说明 | 适用场景 |
|------|------|----------|
| `data-source` | 数据源选择 | 所有需要数据的组件 |
| `value` | 数值输入 | 静态数值设置 |
| `range` | 范围设置 | min/max/step |
| `size` | 尺寸选择 | sm/md/lg |
| `color` | 颜色选择 | 单色配置 |
| `multi-color` | 多色配置 | 主/次/错误/警告/成功色 |
| `label` | 标签配置 | prefix/suffix/unit |
| `boolean` | 开关选项 | 显示/隐藏配置 |
| `select` | 下拉选择 | 枚举值配置 |
| `text` | 文本输入 | 长文本内容 |
| `orientation` | 方向选择 | horizontal/vertical |
| `animation` | 动画配置 | duration/enable |
| `data-mapping` | 数据映射 | 格式化配置 |
| `custom` | 自定义 | 特殊配置 |

### 表单控件规范

所有下拉选择器必须使用统一模式：

```tsx
// 标准选择器模式
<div className="space-y-2">
  <Label className="text-xs flex items-center gap-1.5">
    <Icon className="h-3.5 w-3.5" />
    字段名称
  </Label>
  <Select value={value} onValueChange={onChange} disabled={readonly}>
    <SelectTrigger className="h-9">
      <SelectValue placeholder="选择..." />
    </SelectTrigger>
    <SelectContent>
      {OPTIONS.map((option) => (
        <SelectItem key={option.value} value={option.value}>
          {option.label}
        </SelectItem>
      ))}
    </SelectContent>
  </Select>
</div>
```

**规范要点：**
- 外层容器使用 `space-y-2`
- Label 使用 `text-xs` 大小
- SelectTrigger 使用固定高度 `h-9`
- SelectItem 只显示 `{option.label}`，不嵌套其他元素
- 选项类型：`{ value: string; label: string }`

---

## 6. 数据源规范

### DataSource 接口

```typescript
interface DataSource {
  // 数据源类型
  type: DataSourceType

  // 设备相关
  deviceId?: string
  property?: string
  metricId?: string

  // 命令相关
  command?: string
  commandParams?: Record<string, unknown>
  valueMapping?: ValueMapping

  // 时序数据转换
  timeWindow?: TimeWindowConfig        // 新版: 时间窗口
  aggregateExt?: TelemetryAggregate    // 新版: 扩展聚合
  chartViewMode?: ChartViewMode        // 图表视图模式
  sampleInterval?: number              // 采样间隔 (秒)
  fillMissing?: FillMissingStrategy    // 缺失值处理
  groupBy?: 'device' | 'metric' | 'time'

  // 兼容旧版
  timeRange?: number
  limit?: number
  aggregate?: 'raw' | 'avg' | 'min' | 'max' | 'sum'
}
```

### 时序数据转换

**聚合方法 (TelemetryAggregate):**

| 值 | 说明 |
|---|---|
| `raw` | 原始数据点 |
| `latest` | 最新值 |
| `first` | 第一个值 |
| `avg` | 平均值 |
| `min` | 最小值 |
| `max` | 最大值 |
| `sum` | 总和 |
| `count` | 计数 |
| `delta` | 变化量 (last - first) |
| `rate` | 变化率 |

**时间窗口 (TimeWindowType):**

| 值 | 说明 |
|---|---|
| `now` | 当前值 |
| `last_5min` | 最近5分钟 |
| `last_15min` | 最近15分钟 |
| `last_30min` | 最近30分钟 |
| `last_1hour` | 最近1小时 |
| `last_6hours` | 最近6小时 |
| `last_24hours` | 最近24小时 |
| `today` | 今天 |
| `yesterday` | 昨天 |
| `this_week` | 本周 |
| `custom` | 自定义 |

**图表视图模式 (ChartViewMode):**

| 值 | 说明 |
|---|---|
| `timeseries` | 时序模式：X轴=时间 |
| `snapshot` | 快照模式：显示当前值或聚合值对比 |
| `distribution` | 分布模式：显示占比（适合饼图） |
| `histogram` | 直方图模式：显示频率分布 |

---

## 7. 实现规范

### 组件文件结构

```
web/src/components/dashboard/
├── generic/              # 通用组件实现
│   ├── ValueCard.tsx
│   ├── LEDIndicator.tsx
│   ├── LineChart.tsx
│   └── ...
├── config/               # 配置相关
│   ├── ComponentConfigBuilder.tsx
│   ├── ComponentConfigDialog.tsx
│   ├── UIConfigSections.tsx
│   ├── DataTransformConfig.tsx
│   └── UnifiedDataSourceConfig.tsx
├── registry/             # 组件注册
│   ├── types.ts
│   ├── registry.ts
│   └── ComponentRenderer.tsx
└── shared/               # 共享组件
    ├── DefaultStates.tsx
    └── index.ts
```

### 组件实现模板

```tsx
/**
 * Component Name
 *
 * Brief description of what this component does.
 */

import { useMemo } from 'react'
import { cn } from '@/lib/utils'
import { dashboardCardBase, dashboardComponentSize } from '@/design-system/tokens/size'
import { useDataSource } from '@/hooks/useDataSource'
import { EmptyState } from '../shared'

export interface MyComponentProps {
  // Data source
  dataSource?: DataSourceOrList

  // Display options
  title?: string
  size?: 'sm' | 'md' | 'lg'
  className?: string

  // Component-specific props
  // ...
}

export function MyComponent({
  dataSource,
  title,
  size = 'md',
  className,
}: MyComponentProps) {
  const config = dashboardComponentSize[size]

  // Data fetching
  const { data, loading } = useDataSource(
    dataSource,
    { fallback: undefined }
  )

  // Loading state
  if (loading) {
    return (
      <div className={cn(dashboardCardBase, config.padding, className)}>
        <Skeleton className={cn('w-full', size === 'sm' ? 'h-[120px]' : 'h-[180px]')} />
      </div>
    )
  }

  // Empty state
  if (!data) {
    return <EmptyState size={size} className={className} />
  }

  // Render
  return (
    <div className={cn(dashboardCardBase, config.padding, className)}>
      {title && (
        <div className={cn('mb-3', config.titleText)}>{title}</div>
      )}
      {/* Component content */}
    </div>
  )
}
```

### 必须遵循的规则

1. **导出规范**
   - 组件必须使用命名导出 `export function ComponentName`
   - Props 接口命名为 `ComponentNameProps`

2. **样式规范**
   - 使用 `dashboardCardBase` 作为基础类
   - 使用 `dashboardComponentSize[size]` 获取尺寸配置
   - 使用 `cn()` 工具合并 className

3. **数据获取**
   - 使用 `useDataSource` hook 统一获取数据
   - 处理 `loading` 和 `empty` 状态
   - 使用 `EmptyState` 组件显示空状态

4. **类型安全**
   - 使用 `DataSourceOrList` 支持单/多数据源
   - 使用 `normalizeDataSource` 标准化数据源

5. **响应式设计**
   - 支持移动端 (xs 断点: 4 列)
   - 组件最小宽度不超过 2 个网格单位

---

## 附录：快速参考

### 颜色对照表

```typescript
// 图表默认颜色顺序
const chartColorPalette = [
  '#8b5cf6', // 紫色
  '#22c55e', // 绿色
  '#f59e0b', // 黄色
  '#f97316', // 橙色
  '#ec4899', // 粉色
  '#06b6d4', // 青色
]
```

### 状态对应颜色

| 状态 | 颜色值 | 语义 |
|------|--------|------|
| 在线/成功 | 绿色 | 正常运行 |
| 警告 | 黄色 | 需要注意 |
| 错误 | 红色 | 故障/离线 |
| 信息 | 蓝色 | 中性信息 |
| 未知 | 灰色 | 未确定 |

### 组件 Props 命名规范

| 用途 | Prop 名称 | 类型 |
|------|-----------|------|
| 数据源 | `dataSource` | `DataSourceOrList` |
| 标题 | `title` | `string` |
| 尺寸 | `size` | `'sm' \| 'md' \| 'lg'` |
| 颜色 | `color` | `string` |
| 样式类 | `className` | `string` |
| 禁用状态 | `disabled` | `boolean` |
| 只读状态 | `readonly` | `boolean` |
