/**
 * Telemetry Data Transformation Utilities
 *
 * Handles time-series data aggregation, time window conversion,
 * and data transformation for chart components.
 */

import type {
  TelemetryAggregate,
  TimeWindowConfig,
  TimeWindowType,
  FillMissingStrategy,
  ChartViewMode,
} from '@/types/dashboard'

// ============================================================================
// Time Point Interface
// ============================================================================

export interface TimePoint {
  timestamp: number  // Unix timestamp in seconds
  value: number
}

export interface TimeSeriesData {
  points: TimePoint[]
  deviceId?: string
  metricName?: string
}

// ============================================================================
// Time Window Conversion
// ============================================================================

/**
 * Convert TimeWindowType to hours (for backward compatibility with API).
 */
export function timeWindowToHours(timeWindow: TimeWindowType): number {
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
    'custom': 1,  // Default to 1 hour for custom
  }
  return conversions[timeWindow] ?? 1
}

/**
 * Get time range in seconds from TimeWindowConfig.
 * Returns { start, end } as Unix timestamps.
 */
export function getTimeRange(timeWindow: TimeWindowConfig): { start: number; end: number } {
  const now = Math.floor(Date.now() / 1000)
  const end = timeWindow.endTime ?? now

  if (timeWindow.type === 'custom' && timeWindow.startTime) {
    return { start: timeWindow.startTime, end }
  }

  const startOffsets: Record<TimeWindowType, number> = {
    'now': 0,
    'last_5min': 5 * 60,
    'last_15min': 15 * 60,
    'last_30min': 30 * 60,
    'last_1hour': 60 * 60,
    'last_6hours': 6 * 60 * 60,
    'last_24hours': 24 * 60 * 60,
    'today': getStartOfDay(now),
    'yesterday': getStartOfDay(now) - 24 * 60 * 60,
    'this_week': getStartOfWeek(now),
    'custom': timeWindow.startTime ? now - timeWindow.startTime : 60 * 60,
  }

  const start = timeWindow.type === 'now'
    ? now
    : now - startOffsets[timeWindow.type]

  return { start, end }
}

function getStartOfDay(timestamp: number): number {
  const date = new Date(timestamp * 1000)
  return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime() / 1000
}

function getStartOfWeek(timestamp: number): number {
  const date = new Date(timestamp * 1000)
  const day = date.getDay()
  const diff = date.getDate() - day + (day === 0 ? -6 : 1)  // Adjust to Monday
  const monday = new Date(date.getFullYear(), date.getMonth(), diff)
  monday.setHours(0, 0, 0, 0)
  return (monday.getTime() - timestamp * 1000) / 1000
}

// ============================================================================
// Data Aggregation
// ============================================================================

/**
 * Aggregate time-series data points according to the specified method.
 */
export function aggregateData(
  points: TimePoint[],
  method: TelemetryAggregate
): number | null {
  if (points.length === 0) return null

  const values = points.map(p => p.value).filter(v => v !== null && v !== undefined && !isNaN(v))

  if (values.length === 0) return null

  switch (method) {
    case 'raw':
      // Return all points (special case)
      return points[points.length - 1]?.value ?? null

    case 'latest':
      return values[values.length - 1] ?? null

    case 'first':
      return values[0] ?? null

    case 'avg':
      return values.reduce((sum, v) => sum + v, 0) / values.length

    case 'min':
      return Math.min(...values)

    case 'max':
      return Math.max(...values)

    case 'sum':
      return values.reduce((sum, v) => sum + v, 0)

    case 'count':
      return values.length

    case 'delta':
      if (values.length < 2) return 0
      return values[values.length - 1] - values[0]

    case 'rate': {
      if (points.length < 2) return 0
      const first = points[0]
      const last = points[points.length - 1]
      const timeDiff = last.timestamp - first.timestamp
      if (timeDiff <= 0) return 0
      return (last.value - first.value) / timeDiff  // Rate per second
    }

    default:
      return values[values.length - 1] ?? null
  }
}

/**
 * Aggregate multiple time series into a single value.
 * Useful for multi-source scenarios where you want a single aggregated value.
 */
export function aggregateMultiSeries(
  series: TimeSeriesData[],
  method: TelemetryAggregate
): Record<string, number> {
  const result: Record<string, number> = {}

  for (const s of series) {
    const key = s.deviceId ?? s.metricName ?? 'unknown'
    const aggregated = aggregateData(s.points, method)
    if (aggregated !== null) {
      result[key] = aggregated
    }
  }

  return result
}

// ============================================================================
// Data Transformation for Charts
// ============================================================================

/**
 * Transform time-series data for chart display.
 * Handles aggregation, filling missing values, and formatting for specific chart types.
 */
export interface TransformOptions {
  aggregate?: TelemetryAggregate
  fillMissing?: FillMissingStrategy
  maxPoints?: number  // For downsampling
  chartViewMode?: ChartViewMode
}

export function transformTimeSeries(
  points: TimePoint[],
  options: TransformOptions = {}
): TimePoint[] {
  const {
    aggregate = 'raw',
    fillMissing = 'none',
    maxPoints,
    chartViewMode = 'timeseries',
  } = options

  let result = [...points]

  // Sort by timestamp
  result.sort((a, b) => a.timestamp - b.timestamp)

  // Apply aggregation if not 'raw'
  if (aggregate !== 'raw') {
    const aggregatedValue = aggregateData(result, aggregate)
    if (aggregatedValue !== null) {
      // For non-timeseries modes, return single point
      if (chartViewMode === 'snapshot' || chartViewMode === 'distribution') {
        return [{ timestamp: result[result.length - 1]?.timestamp ?? Date.now() / 1000, value: aggregatedValue }]
      }
      // For timeseries, keep last point with aggregated value
      result = [{ timestamp: result[result.length - 1]?.timestamp ?? Date.now() / 1000, value: aggregatedValue }]
    }
  }

  // Fill missing values
  if (fillMissing !== 'none' && result.length > 0) {
    result = fillMissingValues(result, fillMissing)
  }

  // Downsample if needed
  if (maxPoints && result.length > maxPoints) {
    result = downsample(result, maxPoints)
  }

  return result
}

/**
 * Fill missing values in time series.
 */
function fillMissingValues(
  points: TimePoint[],
  strategy: FillMissingStrategy
): TimePoint[] {
  if (points.length === 0) return points

  const result: TimePoint[] = []
  const interval = estimateInterval(points)

  for (let i = 0; i < points.length - 1; i++) {
    result.push(points[i])

    const current = points[i]
    const next = points[i + 1]
    const gap = next.timestamp - current.timestamp

    // If gap is more than 2x the estimated interval, fill in between
    if (gap > interval * 2) {
      const missingCount = Math.round(gap / interval) - 1

      for (let j = 1; j <= missingCount; j++) {
        const timestamp = current.timestamp + j * interval
        let value: number

        switch (strategy) {
          case 'zero':
            value = 0
            break
          case 'previous':
            value = current.value
            break
          case 'linear':
            value = current.value + (next.value - current.value) * (j / (missingCount + 1))
            break
          default:
            continue
        }

        result.push({ timestamp, value })
      }
    }
  }

  result.push(points[points.length - 1])
  return result
}

/**
 * Estimate the average time interval between points.
 */
function estimateInterval(points: TimePoint[]): number {
  if (points.length < 2) return 60  // Default 1 minute

  let totalInterval = 0
  let count = 0

  for (let i = 0; i < points.length - 1; i++) {
    const interval = points[i + 1].timestamp - points[i].timestamp
    // Ignore large gaps (they're probably intentional, not missing data)
    if (interval > 0 && interval < 3600) {  // Less than 1 hour
      totalInterval += interval
      count++
    }
  }

  return count > 0 ? totalInterval / count : 60
}

/**
 * Downsample time series to max points using LTTB-inspired approach.
 * Simplified version - just takes evenly spaced points.
 */
function downsample(points: TimePoint[], maxPoints: number): TimePoint[] {
  if (points.length <= maxPoints) return points

  const step = (points.length - 1) / (maxPoints - 1)
  const result: TimePoint[] = []

  for (let i = 0; i < maxPoints; i++) {
    const index = Math.min(Math.round(i * step), points.length - 1)
    result.push(points[index])
  }

  return result
}

// ============================================================================
// Chart-Specific Data Transformation
// ============================================================================

export interface ChartDataPoint {
  name: string      // Label (time or category)
  value: number     // Value
  color?: string
  timestamp?: number  // Original timestamp for time-series charts
}

/**
 * Transform time-series data to pie/donut chart format.
 * For pie charts, we typically show:
 * - Distribution across devices (multi-source)
 * - Distribution of value ranges (histogram-style)
 */
export function transformToPieData(
  data: TimeSeriesData | TimeSeriesData[],
  aggregate: TelemetryAggregate = 'latest'
): ChartDataPoint[] {
  const series = Array.isArray(data) ? data : [data]
  const result: ChartDataPoint[] = []

  for (let i = 0; i < series.length; i++) {
    const s = series[i]
    const aggregated = aggregateData(s.points, aggregate)

    if (aggregated !== null) {
      result.push({
        name: s.deviceId ?? s.metricName ?? `Series ${i + 1}`,
        value: aggregated,
      })
    }
  }

  return result
}

/**
 * Transform time-series data to bar chart format.
 * Can be:
 * - Time series (X-axis = time)
 * - Comparison (X-axis = device/metric)
 */
export function transformToBarData(
  data: TimeSeriesData | TimeSeriesData[],
  options: {
    aggregate?: TelemetryAggregate
    chartViewMode?: ChartViewMode
    maxPoints?: number
  } = {}
): ChartDataPoint[] {
  const { aggregate = 'raw', chartViewMode = 'timeseries', maxPoints = 24 } = options
  const series = Array.isArray(data) ? data : [data]

  // Timeseries mode - show data points over time
  if (chartViewMode === 'timeseries') {
    if (series.length === 1) {
      const points = transformTimeSeries(series[0].points, { aggregate, maxPoints, chartViewMode })
      return points.map(p => ({
        name: formatTimestamp(p.timestamp),
        value: p.value,
        timestamp: p.timestamp,
      }))
    }
    // Multi-series timeseries - return combined format
    // This is handled differently in the chart component
    return []
  }

  // Snapshot/Comparison mode - aggregate each series
  const result: ChartDataPoint[] = []
  for (let i = 0; i < series.length; i++) {
    const s = series[i]
    const aggregated = aggregateData(s.points, aggregate)

    if (aggregated !== null) {
      result.push({
        name: s.deviceId ?? s.metricName ?? `Series ${i + 1}`,
        value: aggregated,
      })
    }
  }

  return result
}

/**
 * Format timestamp for chart labels.
 */
function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp * 1000)
  const now = new Date()
  const isToday = date.toDateString() === now.toDateString()

  if (isToday) {
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  }

  return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
}

// ============================================================================
// Helper Functions for DataSource Integration
// ============================================================================

/**
 * Get effective aggregate method from DataSource.
 * Handles both legacy `aggregate` and new `aggregateExt`.
 */
export function getEffectiveAggregate(
  dataSource: { aggregate?: string; aggregateExt?: TelemetryAggregate }
): TelemetryAggregate {
  if (dataSource.aggregateExt) {
    return dataSource.aggregateExt
  }

  // Map legacy aggregate values
  const legacyMap: Record<string, TelemetryAggregate> = {
    'raw': 'raw',
    'avg': 'avg',
    'min': 'min',
    'max': 'max',
    'sum': 'sum',
  }

  return legacyMap[dataSource.aggregate ?? ''] ?? 'raw'
}

/**
 * Get effective time window from DataSource.
 * Handles both legacy `timeRange` and new `timeWindow`.
 */
export function getEffectiveTimeWindow(
  dataSource: { timeRange?: number; timeWindow?: TimeWindowConfig }
): TimeWindowConfig {
  if (dataSource.timeWindow) {
    return dataSource.timeWindow
  }

  // Convert legacy timeRange (hours) to TimeWindowConfig
  const hours = dataSource.timeRange ?? 1
  if (hours === 0) {
    return { type: 'now' }
  }

  // Find closest match
  const timeWindowTypes: TimeWindowType[] = ['last_1hour', 'last_6hours', 'last_24hours']
  for (const type of timeWindowTypes) {
    if (timeWindowToHours(type) >= hours) {
      return { type }
    }
  }

  return { type: 'last_24hours' }
}

/**
 * Parse telemetry API response to TimeSeriesData format.
 * API returns: [{ timestamp, value }, ...]
 */
export function parseTelemetryResponse(data: unknown): TimePoint[] {
  if (!Array.isArray(data)) return []

  return data
    .filter((item): item is { timestamp: number; value: number } =>
      typeof item === 'object' &&
      item !== null &&
      'timestamp' in item &&
      'value' in item &&
      typeof item.timestamp === 'number' &&
      typeof item.value === 'number'
    )
    .map(item => ({
      timestamp: item.timestamp,
      value: item.value,
    }))
}
