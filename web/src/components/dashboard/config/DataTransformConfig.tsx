/**
 * DataTransformConfig Component
 *
 * Configuration UI for time-series data transformation.
 * Uses unified Field component for consistent styling.
 */

import { useMemo } from 'react'
import { Clock, BarChart3, Sliders } from 'lucide-react'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Field } from '@/components/ui/field'
import type {
  TelemetryAggregate,
  TimeWindowType,
  ChartViewMode,
  FillMissingStrategy,
  DataSource,
} from '@/types/dashboard'

export interface DataTransformConfigProps {
  dataSource?: DataSource
  onChange: (updates: Partial<DataSource>) => void
  // Show/hide specific options based on chart type
  chartType?: 'pie' | 'bar' | 'line' | 'area' | 'card' | 'sparkline' | 'led' | 'progress'
  readonly?: boolean
  // Simplified mode for single-value components (card, led, progress)
  simplified?: boolean
}

// ============================================================================
// Options Data
// ============================================================================

// Time window options
const TIME_WINDOW_OPTIONS: Array<{ value: TimeWindowType; label: string }> = [
  { value: 'now', label: '当前值' },
  { value: 'last_5min', label: '最近5分钟' },
  { value: 'last_15min', label: '最近15分钟' },
  { value: 'last_30min', label: '最近30分钟' },
  { value: 'last_1hour', label: '最近1小时' },
  { value: 'last_6hours', label: '最近6小时' },
  { value: 'last_24hours', label: '最近24小时' },
  { value: 'today', label: '今天' },
  { value: 'yesterday', label: '昨天' },
  { value: 'this_week', label: '本周' },
]

// Aggregation method options
const AGGREGATE_OPTIONS: Array<{ value: TelemetryAggregate; label: string }> = [
  { value: 'latest', label: '最新值' },
  { value: 'first', label: '首个值' },
  { value: 'avg', label: '平均值' },
  { value: 'min', label: '最小值' },
  { value: 'max', label: '最大值' },
  { value: 'sum', label: '总和' },
  { value: 'count', label: '计数' },
  { value: 'delta', label: '变化量' },
  { value: 'rate', label: '变化率' },
  { value: 'raw', label: '原始数据' },
]

// Chart view mode options
const CHART_VIEW_OPTIONS: Array<{ value: ChartViewMode; label: string }> = [
  { value: 'timeseries', label: '时序图' },
  { value: 'snapshot', label: '快照对比' },
  { value: 'distribution', label: '分布图' },
  { value: 'histogram', label: '直方图' },
]

// Data point limit options
const DATA_POINT_OPTIONS: Array<{ value: number; label: string }> = [
  { value: 12, label: '12点' },
  { value: 24, label: '24点' },
  { value: 50, label: '50点' },
  { value: 100, label: '100点' },
  { value: 200, label: '200点' },
]

// Fill missing strategy options
const FILL_MISSING_OPTIONS: Array<{ value: FillMissingStrategy; label: string }> = [
  { value: 'none', label: '无' },
  { value: 'zero', label: '填充零' },
  { value: 'previous', label: '前向填充' },
  { value: 'linear', label: '线性插值' },
]

// Sample interval options (seconds)
const SAMPLE_INTERVAL_OPTIONS: Array<{ value: number; label: string }> = [
  { value: 30, label: '30秒' },
  { value: 60, label: '1分钟' },
  { value: 300, label: '5分钟' },
  { value: 600, label: '10分钟' },
  { value: 900, label: '15分钟' },
  { value: 1800, label: '30分钟' },
  { value: 3600, label: '1小时' },
]

// ============================================================================
// Defaults by Chart Type
// ============================================================================

const DEFAULTS_BY_CHART: Record<string, { aggregate: TelemetryAggregate; limit: number }> = {
  pie: { aggregate: 'latest', limit: 10 },
  bar: { aggregate: 'raw', limit: 50 },
  line: { aggregate: 'raw', limit: 50 },
  area: { aggregate: 'raw', limit: 50 },
  card: { aggregate: 'latest', limit: 1 },
  sparkline: { aggregate: 'raw', limit: 50 },
  led: { aggregate: 'latest', limit: 1 },
  progress: { aggregate: 'latest', limit: 1 },
}

// Simplified aggregate options for single-value components (only latest makes sense)
const SIMPLIFIED_AGGREGATE_OPTIONS: Array<{ value: TelemetryAggregate; label: string }> = [
  { value: 'latest', label: '最新值' },
  { value: 'avg', label: '平均值' },
]

// ============================================================================
// Component
// ============================================================================

export function DataTransformConfig({
  dataSource,
  onChange,
  chartType = 'bar',
  readonly = false,
  simplified = false,
}: DataTransformConfigProps) {
  // Determine if this is a single-value component that needs simplified options
  const isSimplified = simplified || ['card', 'led', 'progress'].includes(chartType)

  // Get current values with defaults
  const currentAggregate = useMemo(() => {
    return dataSource?.aggregateExt ?? dataSource?.aggregate ?? DEFAULTS_BY_CHART[chartType]?.aggregate ?? 'raw'
  }, [dataSource, chartType])

  const currentTimeWindow = useMemo(() => {
    return dataSource?.timeWindow?.type ?? 'last_1hour'
  }, [dataSource])

  const currentChartViewMode = useMemo(() => {
    return dataSource?.chartViewMode ?? (chartType === 'pie' ? 'distribution' : 'timeseries')
  }, [dataSource, chartType])

  const currentLimit = useMemo(() => {
    return dataSource?.limit ?? DEFAULTS_BY_CHART[chartType]?.limit ?? 50
  }, [dataSource, chartType])

  const currentFillMissing = useMemo(() => {
    return dataSource?.fillMissing ?? 'none'
  }, [dataSource])

  const currentSampleInterval = useMemo(() => {
    return dataSource?.sampleInterval ?? 60
  }, [dataSource])

  // Choose aggregate options based on mode
  const aggregateOptions = isSimplified ? SIMPLIFIED_AGGREGATE_OPTIONS : AGGREGATE_OPTIONS

  // Update handlers
  const handleAggregateChange = (value: string) => {
    onChange({
      aggregateExt: value as TelemetryAggregate,
      // For backward compatibility, also set aggregate if it maps
      ...(value === 'raw' || value === 'avg' || value === 'min' || value === 'max' || value === 'sum'
        ? { aggregate: value }
        : {}),
    })
  }

  const handleTimeWindowChange = (value: string) => {
    const type = value as TimeWindowType
    onChange({
      timeWindow: { type },
      // Also update legacy timeRange for backward compatibility
      timeRange: getTimeWindowHours(type),
    })
  }

  const handleChartViewModeChange = (value: string) => {
    onChange({ chartViewMode: value as ChartViewMode })
  }

  const handleLimitChange = (value: string) => {
    onChange({ limit: parseInt(value) || 50 })
  }

  const handleFillMissingChange = (value: string) => {
    onChange({ fillMissing: value as FillMissingStrategy })
  }

  const handleSampleIntervalChange = (value: string) => {
    onChange({ sampleInterval: parseInt(value) || 60 })
  }

  return (
    <div className="space-y-3">
      {/* Time Window - simplified mode only shows now and last_1hour */}
      <Field>
        <Label>时间范围</Label>
        <Select value={currentTimeWindow} onValueChange={handleTimeWindowChange} disabled={readonly}>
          <SelectTrigger>
            <SelectValue placeholder="选择时间范围" />
          </SelectTrigger>
          <SelectContent>
            {TIME_WINDOW_OPTIONS.filter(o => {
              if (isSimplified) {
                // Single-value components only need current/recent values
                return o.value === 'now' || o.value === 'last_5min' || o.value === 'last_15min' || o.value === 'last_30min' || o.value === 'last_1hour'
              }
              return true
            }).map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>

      {/* Aggregation Method */}
      <Field>
        <Label>聚合方式</Label>
        <Select value={currentAggregate} onValueChange={handleAggregateChange} disabled={readonly}>
          <SelectTrigger>
            <SelectValue placeholder="选择聚合方式" />
          </SelectTrigger>
          <SelectContent>
            {aggregateOptions.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>

      {/* Chart View Mode - only for bar/line/area charts (not simplified) */}
      {!isSimplified && (chartType === 'bar' || chartType === 'line' || chartType === 'area') && (
        <Field>
          <Label>图表视图</Label>
          <Select value={currentChartViewMode} onValueChange={handleChartViewModeChange} disabled={readonly}>
            <SelectTrigger>
              <SelectValue placeholder="选择图表视图" />
            </SelectTrigger>
            <SelectContent>
              {CHART_VIEW_OPTIONS.filter(o => {
                // Filter based on chart type
                if (chartType === 'line' || chartType === 'area') {
                  return o.value === 'timeseries' || o.value === 'snapshot'
                }
                return true
              }).map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
      )}

      {/* Advanced Options - only for non-simplified charts with raw aggregate */}
      {!isSimplified && (currentAggregate === 'raw' || chartType === 'bar' || chartType === 'line' || chartType === 'area' || chartType === 'sparkline') && (
        <>
          <div className="pt-2 border-t">
            <div className="text-xs font-medium text-muted-foreground mb-3">高级选项</div>
          </div>

          {/* Data Points Limit */}
          <Field>
            <Label>数据点数量</Label>
            <Select
              value={String(currentLimit)}
              onValueChange={handleLimitChange}
              disabled={readonly}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {DATA_POINT_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={String(option.value)}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          {/* Sample Interval - only for raw aggregate */}
          {currentAggregate === 'raw' && (
            <Field>
              <Label>采样间隔（秒）</Label>
              <Select
                value={String(currentSampleInterval)}
                onValueChange={handleSampleIntervalChange}
                disabled={readonly}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {SAMPLE_INTERVAL_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={String(option.value)}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
          )}

          {/* Fill Missing Strategy - only for raw aggregate */}
          {currentAggregate === 'raw' && (
            <Field>
              <Label>缺失值处理</Label>
              <Select
                value={currentFillMissing}
                onValueChange={handleFillMissingChange}
                disabled={readonly}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {FILL_MISSING_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
          )}
        </>
      )}
    </div>
  )
}

// ============================================================================
// Helper Functions
// ============================================================================

// Helper function to convert time window type to hours
function getTimeWindowHours(type: TimeWindowType): number {
  const conversions: Record<TimeWindowType, number> = {
    'now': 0,
    'last_5min': 5 / 60,
    'last_15min': 15 / 60,
    'last_30min': 30 / 60,
    'last_1hour': 1,
    'last_6hours': 6,
    'last_24hours': 24,
    'today': 24,
    'yesterday': 24,
    'this_week': 24 * 7,
    'custom': 1,
  }
  return conversions[type] ?? 1
}
