/**
 * Telemetry Transform Utilities
 *
 * Aggregation methods, time window handling, and data formatting
 * for dashboard time-series data.
 */

import type { TelemetryAggregate } from '../types'
import type { TelemetryPoint } from '../api/telemetry'

// ============================================================================
// Aggregation
// ============================================================================

/** Apply aggregation to a set of data points */
export function aggregateData(
  points: TelemetryPoint[],
  method: TelemetryAggregate,
): number | null {
  if (points.length === 0) return null

  switch (method) {
    case 'raw':
      return points[points.length - 1].value
    case 'latest':
      return points[points.length - 1].value
    case 'first':
      return points[0].value
    case 'avg':
      return points.reduce((sum, p) => sum + p.value, 0) / points.length
    case 'min':
      return Math.min(...points.map(p => p.value))
    case 'max':
      return Math.max(...points.map(p => p.value))
    case 'sum':
      return points.reduce((sum, p) => sum + p.value, 0)
    case 'count':
      return points.length
    case 'delta':
      return points[points.length - 1].value - points[0].value
    case 'rate': {
      if (points.length < 2) return 0
      const timeDiffSeconds = (points[points.length - 1].timestamp - points[0].timestamp)
      if (timeDiffSeconds === 0) return 0
      return (points[points.length - 1].value - points[0].value) / timeDiffSeconds
    }
    default:
      return points[points.length - 1].value
  }
}

// ============================================================================
// Data point merging (for real-time updates)
// ============================================================================

/** Append a new data point to an existing time-series, maintaining order */
export function appendDataPoint(
  existing: TelemetryPoint[] | undefined,
  newPoint: TelemetryPoint,
  maxPoints = 500,
): TelemetryPoint[] {
  const series = existing ?? []
  const updated = [...series, newPoint]
  // Trim to max length (remove oldest)
  if (updated.length > maxPoints) {
    return updated.slice(updated.length - maxPoints)
  }
  return updated
}

/** Merge multiple new points into existing series */
export function mergeDataPoints(
  existing: TelemetryPoint[] | undefined,
  newPoints: TelemetryPoint[],
  maxPoints = 500,
): TelemetryPoint[] {
  const series = existing ?? []
  // Deduplicate by timestamp
  const tsSet = new Set(series.map(p => p.timestamp))
  const unique = newPoints.filter(p => !tsSet.has(p.timestamp))
  const merged = [...series, ...unique].sort((a, b) => a.timestamp - b.timestamp)
  if (merged.length > maxPoints) {
    return merged.slice(merged.length - maxPoints)
  }
  return merged
}

// ============================================================================
// Downsampling
// ============================================================================

/** Downsample time-series data by interval */
export function downsample(
  points: TelemetryPoint[],
  intervalSeconds: number,
  method: TelemetryAggregate = 'avg',
): TelemetryPoint[] {
  if (points.length === 0 || intervalSeconds <= 0) return points

  const buckets: Map<number, TelemetryPoint[]> = new Map()
  for (const p of points) {
    const bucketKey = Math.floor(p.timestamp / intervalSeconds) * intervalSeconds
    const bucket = buckets.get(bucketKey)
    if (bucket) {
      bucket.push(p)
    } else {
      buckets.set(bucketKey, [p])
    }
  }

  const result: TelemetryPoint[] = []
  for (const [ts, bucket] of buckets) {
    const value = aggregateData(bucket, method)
    if (value !== null) {
      result.push({ timestamp: ts, value })
    }
  }

  return result.sort((a, b) => a.timestamp - b.timestamp)
}

// ============================================================================
// Formatting
// ============================================================================

/** Format a numeric value with unit and precision */
export function formatTelemetryValue(
  value: number | string | null,
  options?: { unit?: string; precision?: number; format?: string },
): string {
  if (value === null || value === undefined) return '--'
  if (typeof value === 'string') return options?.unit ? `${value} ${options.unit}` : value

  const precision = options?.precision ?? 1
  const formatted = Number.isInteger(value) ? String(value) : value.toFixed(precision)
  return options?.unit ? `${formatted} ${options.unit}` : formatted
}
