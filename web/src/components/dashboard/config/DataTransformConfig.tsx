/**
 * DataTransformConfig Component
 *
 * Configuration UI for time-series data transformation.
 * Uses unified Field component for consistent styling.
 */

import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
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

// Time window options factory (uses translations)
function getTimeWindowOptions(t: (key: string) => string): Array<{ value: TimeWindowType; label: string }> {
  return [
    { value: 'now', label: t('dataTransform.timeWindow.now') },
    { value: 'last_5min', label: t('dataTransform.timeWindow.last5min') },
    { value: 'last_15min', label: t('dataTransform.timeWindow.last15min') },
    { value: 'last_30min', label: t('dataTransform.timeWindow.last30min') },
    { value: 'last_1hour', label: t('dataTransform.timeWindow.last1hour') },
    { value: 'last_6hours', label: t('dataTransform.timeWindow.last6hours') },
    { value: 'last_24hours', label: t('dataTransform.timeWindow.last24hours') },
    { value: 'today', label: t('dataTransform.timeWindow.today') },
    { value: 'yesterday', label: t('dataTransform.timeWindow.yesterday') },
    { value: 'this_week', label: t('dataTransform.timeWindow.thisWeek') },
  ]
}

// Aggregation method options factory (uses translations)
function getAggregateOptions(t: (key: string) => string): Array<{ value: TelemetryAggregate; label: string }> {
  return [
    { value: 'latest', label: t('dataTransform.aggregate.latest') },
    { value: 'first', label: t('dataTransform.aggregate.first') },
    { value: 'avg', label: t('dataTransform.aggregate.avg') },
    { value: 'min', label: t('dataTransform.aggregate.min') },
    { value: 'max', label: t('dataTransform.aggregate.max') },
    { value: 'sum', label: t('dataTransform.aggregate.sum') },
    { value: 'count', label: t('dataTransform.aggregate.count') },
    { value: 'delta', label: t('dataTransform.aggregate.delta') },
    { value: 'rate', label: t('dataTransform.aggregate.rate') },
    { value: 'raw', label: t('dataTransform.aggregate.raw') },
  ]
}

// Chart view mode options factory (uses translations)
function getChartViewOptions(t: (key: string) => string): Array<{ value: ChartViewMode; label: string }> {
  return [
    { value: 'timeseries', label: t('dataTransform.chartViewTimeseries') },
    { value: 'snapshot', label: t('dataTransform.chartViewSnapshot') },
    { value: 'distribution', label: t('dataTransform.chartViewDistribution') },
    { value: 'histogram', label: t('dataTransform.chartViewHistogram') },
  ]
}

// Data point limit options factory (uses translations)
function getDataPointOptions(t: (key: string) => string): Array<{ value: number; label: string }> {
  return [
    { value: 12, label: t('dataTransform.dataPoints.12') },
    { value: 24, label: t('dataTransform.dataPoints.24') },
    { value: 50, label: t('dataTransform.dataPoints.50') },
    { value: 100, label: t('dataTransform.dataPoints.100') },
    { value: 200, label: t('dataTransform.dataPoints.200') },
  ]
}

// Fill missing strategy options factory (uses translations)
function getFillMissingOptions(t: (key: string) => string): Array<{ value: FillMissingStrategy; label: string }> {
  return [
    { value: 'none', label: t('dataTransform.fillMissing.none') },
    { value: 'zero', label: t('dataTransform.fillMissing.zero') },
    { value: 'previous', label: t('dataTransform.fillMissing.previous') },
    { value: 'linear', label: t('dataTransform.fillMissing.linear') },
  ]
}

// Sample interval options factory (uses translations)
function getSampleIntervalOptions(t: (key: string) => string): Array<{ value: number; label: string }> {
  return [
    { value: 30, label: t('dataTransform.sampleInterval.30') },
    { value: 60, label: t('dataTransform.sampleInterval.60') },
    { value: 300, label: t('dataTransform.sampleInterval.300') },
    { value: 600, label: t('dataTransform.sampleInterval.600') },
    { value: 900, label: t('dataTransform.sampleInterval.900') },
    { value: 1800, label: t('dataTransform.sampleInterval.1800') },
    { value: 3600, label: t('dataTransform.sampleInterval.3600') },
  ]
}

// Simplified aggregate options factory (uses translations)
function getSimplifiedAggregateOptions(t: (key: string) => string): Array<{ value: TelemetryAggregate; label: string }> {
  return [
    { value: 'latest', label: t('dataTransform.aggregate.latest') },
    { value: 'avg', label: t('dataTransform.aggregate.avg') },
  ]
}

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
  const { t } = useTranslation('dashboardComponents')

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
  const aggregateOptions = isSimplified ? getSimplifiedAggregateOptions(t) : getAggregateOptions(t)

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
        <Label>{t('dataTransform.timeRange')}</Label>
        <Select value={currentTimeWindow} onValueChange={handleTimeWindowChange} disabled={readonly}>
          <SelectTrigger>
            <SelectValue placeholder={t('dataTransform.selectTimeRange')} />
          </SelectTrigger>
          <SelectContent>
            {getTimeWindowOptions(t).filter(o => {
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
        <Label>{t('dataTransform.aggregation')}</Label>
        <Select value={currentAggregate} onValueChange={handleAggregateChange} disabled={readonly}>
          <SelectTrigger>
            <SelectValue placeholder={t('dataTransform.selectAggregation')} />
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
          <Label>{t('dataTransform.chartView')}</Label>
          <Select value={currentChartViewMode} onValueChange={handleChartViewModeChange} disabled={readonly}>
            <SelectTrigger>
              <SelectValue placeholder={t('dataTransform.selectChartView')} />
            </SelectTrigger>
            <SelectContent>
              {getChartViewOptions(t).filter(o => {
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
            <div className="text-xs font-medium text-muted-foreground mb-3">{t('dataTransform.advancedOptions')}</div>
          </div>

          {/* Data Points Limit */}
          <Field>
            <Label>{t('dataTransform.dataPointLimit')}</Label>
            <Select
              value={String(currentLimit)}
              onValueChange={handleLimitChange}
              disabled={readonly}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {getDataPointOptions(t).map((option) => (
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
              <Label>{t('dataTransform.sampleIntervalLabel')}</Label>
              <Select
                value={String(currentSampleInterval)}
                onValueChange={handleSampleIntervalChange}
                disabled={readonly}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {getSampleIntervalOptions(t).map((option) => (
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
              <Label>{t('dataTransform.fillMissingLabel')}</Label>
              <Select
                value={currentFillMissing}
                onValueChange={handleFillMissingChange}
                disabled={readonly}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {getFillMissingOptions(t).map((option) => (
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
