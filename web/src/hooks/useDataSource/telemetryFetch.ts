/**
 * Historical telemetry fetching with caching, deduplication, and aggregation.
 *
 * Extracted from useDataSource.ts to keep concerns separated:
 * - In-flight request deduplication via `telemetryInflight` Map
 * - 30s TTL cache via `telemetryCache`
 * - Full aggregation pipeline (raw, latest, first, avg, min, max, sum, delta, count)
 */

import type { TelemetryAggregate } from '@/types/dashboard'
import { telemetryCache } from './cache'
import { logError } from '@/lib/errors'

// ============================================================================
// In-flight request deduplication
// ============================================================================

/**
 * Tracks in-flight fetch promises to prevent duplicate API calls when multiple
 * widgets request the same telemetry simultaneously during progressive rendering.
 */
const telemetryInflight = new Map<string, Promise<{ data: number[]; raw?: unknown[]; success: boolean }>>()

// ============================================================================
// Public API
// ============================================================================

/**
 * Fetch historical telemetry data for a device metric.
 *
 * @param deviceId - Device identifier
 * @param metricId - Metric key (e.g. "temperature", "humidity")
 * @param timeRange - Time range in hours (default 1; 0 = last 5 minutes)
 * @param limit - Maximum number of points to fetch (default 50)
 * @param aggregate - Aggregation mode (default 'raw')
 * @param includeRawPoints - If true, return full TelemetryPoint[] in `raw` field
 * @param bypassCache - If true, skip cache lookup and in-flight dedup
 * @returns Promise with data array, optional raw points, and success flag
 */
export async function fetchHistoricalTelemetry(
  deviceId: string,
  metricId: string,
  timeRange: number = 1, // hours
  limit: number = 50,
  aggregate: TelemetryAggregate = 'raw',
  includeRawPoints: boolean = false,
  bypassCache: boolean = false
): Promise<{ data: number[]; raw?: unknown[]; success: boolean }> {
  const cacheKey = `${deviceId}|${metricId}|${timeRange}|${limit}|${aggregate}`
  const cached = telemetryCache.get(cacheKey)

  // Return cached data if fresh (unless bypassing cache)
  if (!bypassCache && cached) {
    return { data: cached.data, raw: cached.raw, success: true }
  }

  // Deduplicate in-flight requests — if another widget already started the same fetch, reuse it
  if (!bypassCache && telemetryInflight.has(cacheKey)) {
    return telemetryInflight.get(cacheKey)!
  }

  const fetchPromise = (async () => {
    try {
      const api = (await import('@/lib/api')).api
      const now = Date.now()

      // Ensure minimum time range of 5 minutes for "now" (timeRange = 0) to get at least some data
      const effectiveTimeRange = timeRange > 0 ? timeRange : 5 / 60
      const start = now - effectiveTimeRange * 60 * 60 * 1000

      // Fetch with larger limit for non-raw aggregates to get enough data points
      const fetchLimit = aggregate === 'raw' ? limit : Math.max(limit, 100)
      const startSec = Math.floor(start / 1000)
      const endSec = Math.floor(now / 1000)

      // Fetch telemetry using the exact metricId from device type definition
      const response = await api.getDeviceTelemetry(deviceId, metricId, startSec, endSec, fetchLimit)

      // Try to find the metric data - first try exact match, then try case-insensitive
      let metricData: unknown[] | undefined = undefined
      if (response?.data && typeof response.data === 'object') {
        const dataObj = response.data as Record<string, unknown>
        // Try exact match first
        if (Array.isArray(dataObj[metricId])) {
          metricData = dataObj[metricId] as unknown[]
        } else {
          // Try case-insensitive match
          const lowerMetricId = metricId.toLowerCase()
          for (const key of Object.keys(dataObj)) {
            if (key.toLowerCase() === lowerMetricId && Array.isArray(dataObj[key])) {
              metricData = dataObj[key] as unknown[]
              break
            }
          }
        }
      }

      // TelemetryDataResponse has structure: { data: Record<string, TelemetryPoint[]> }
      if (response?.data && typeof response.data === 'object' && Array.isArray(metricData) && metricData.length > 0) {
        // Helper to extract value from a point
        const extractValue = (point: unknown): number => {
          if (typeof point === 'number') return point
          if (typeof point === 'object' && point !== null) {
            const p = point as unknown as Record<string, unknown>
            const rawValue = p.value ?? p.v ?? p.avg ?? p.min ?? p.max ?? 0
            if (typeof rawValue === 'number') return rawValue
            if (typeof rawValue === 'string') {
              const parsed = parseFloat(rawValue)
              return isNaN(parsed) ? 0 : parsed
            }
            if (typeof rawValue === 'boolean') return rawValue ? 1 : 0
            return 0
          }
          return 0
        }

        // Helper to extract timestamp from a point (converts ms to seconds if needed)
        const extractTimestamp = (point: unknown): number => {
          if (typeof point === 'object' && point !== null) {
            const p = point as unknown as Record<string, unknown>
            const ts = p.timestamp ?? p.time ?? p.t
            if (typeof ts === 'number') {
              // Convert milliseconds to seconds if needed (timestamps > 10000000000 are in ms)
              return ts > 10000000000 ? Math.floor(ts / 1000) : ts
            }
          }
          return Math.floor(Date.now() / 1000)
        }

        // Extract all values
        const allValues = metricData.map(extractValue).filter((v: number) => typeof v === 'number' && !isNaN(v))

        // Apply aggregation
        // NOTE: API returns data in DESCENDING order (newest first: index 0 = newest)
        let values: number[]
        let rawPoints: unknown[] | undefined

        if (aggregate === 'latest') {
          const newValue = allValues[0] ?? 0
          values = [newValue]
          if (includeRawPoints) {
            const newPoint = metricData[0]
            const rawValue = typeof newPoint === 'object' && newPoint !== null
              ? (newPoint as unknown as Record<string, unknown>).value ?? (newPoint as unknown as Record<string, unknown>).v ?? newPoint
              : newPoint
            rawPoints = [{
              timestamp: extractTimestamp(newPoint),
              value: rawValue,
            }]
          }
        } else if (aggregate === 'first') {
          const oldValue = allValues[allValues.length - 1] ?? 0
          values = [oldValue]
          if (includeRawPoints) {
            const oldPoint = metricData[metricData.length - 1]
            const rawValue = typeof oldPoint === 'object' && oldPoint !== null
              ? (oldPoint as unknown as Record<string, unknown>).value ?? (oldPoint as unknown as Record<string, unknown>).v ?? oldPoint
              : oldPoint
            rawPoints = [{
              timestamp: extractTimestamp(oldPoint),
              value: rawValue,
            }]
          }
        } else if (aggregate === 'avg') {
          const sum = allValues.reduce((a, b) => a + b, 0)
          const avg = allValues.length > 0 ? sum / allValues.length : 0
          values = [avg]
        } else if (aggregate === 'min') {
          const min = Math.min(...allValues)
          values = [min]
        } else if (aggregate === 'max') {
          const max = Math.max(...allValues)
          values = [max]
        } else if (aggregate === 'sum') {
          const sum = allValues.reduce((a, b) => a + b, 0)
          values = [sum]
        } else if (aggregate === 'delta') {
          const newValue = allValues[0] ?? 0
          const oldValue = allValues[allValues.length - 1] ?? 0
          values = [newValue - oldValue]
        } else if (aggregate === 'count') {
          values = [allValues.length]
        } else {
          // 'raw' or unknown: return all values
          values = allValues
          rawPoints = includeRawPoints ? metricData.map((point) => {
            if (typeof point === 'number') {
              return { timestamp: Math.floor(Date.now() / 1000), value: point }
            }
            if (typeof point === 'object' && point !== null) {
              const p = point as unknown as Record<string, unknown>
              const ts = p.timestamp ?? p.time ?? p.t
              let timestamp: number
              if (typeof ts === 'number') {
                timestamp = ts > 10000000000 ? Math.floor(ts / 1000) : ts
              } else {
                timestamp = Math.floor(Date.now() / 1000)
              }
              const value = p.value ?? p.v ?? 0

              return { timestamp, value }
            }
            return { timestamp: Math.floor(Date.now() / 1000), value: point }
          }) : undefined
        }

        // Cache the result
        telemetryCache.set(cacheKey, {
          data: values,
          raw: rawPoints,
        })

        return { data: values, raw: rawPoints, success: true }
      }

      return { data: [], success: false }
    } catch (error) {
      logError(error, { operation: 'Fetch historical telemetry' })
      return { data: [], success: false }
    }
  })()

  // Register in-flight and clean up when done
  if (!bypassCache) {
    telemetryInflight.set(cacheKey, fetchPromise)
    fetchPromise.finally(() => telemetryInflight.delete(cacheKey))
  }

  return fetchPromise
}

/**
 * Synchronously read telemetry data from cache (no fetch).
 * Used to initialize widget state on mount — if useDashboardPrefetch has
 * already warmed the cache, the widget gets data immediately without any
 * loading → loaded re-render cycle.
 */
export function readTelemetryCacheSync(
  deviceId: string,
  metricId: string,
  timeRange: number = 1,
  limit: number = 50,
  aggregate: TelemetryAggregate = 'raw'
): { data: number[]; raw?: unknown[] } | null {
  const cacheKey = `${deviceId}|${metricId}|${timeRange}|${limit}|${aggregate}`
  const cached = telemetryCache.get(cacheKey)
  if (cached) return cached
  return null
}
