/**
 * API calls, caching, and in-flight deduplication for useDataSource.
 * Consolidates telemetryFetch.ts, systemFetch.ts, cache.ts, and batchFetch.ts.
 */

import type { TelemetryAggregate } from '@/types/dashboard'
import { useStore } from '@/store'
import { logError } from '@/lib/errors'

// ============================================================================
// Cache (replaces TypedCache class instances)
// ============================================================================

interface CacheEntry<T> { data: T; timestamp: number; meta?: Record<string, unknown> }

function createCache<T>(ttl: number, maxSize: number) {
  const cache = new Map<string, CacheEntry<T>>()

  return {
    get(key: string): T | undefined {
      const entry = cache.get(key)
      if (!entry) return undefined
      if (Date.now() - entry.timestamp > ttl) { cache.delete(key); return undefined }
      return entry.data
    },
    getWithMeta(key: string): { data: T; meta?: Record<string, unknown> } | undefined {
      const entry = cache.get(key)
      if (!entry) return undefined
      if (Date.now() - entry.timestamp > ttl) { cache.delete(key); return undefined }
      return { data: entry.data, meta: entry.meta }
    },
    set(key: string, data: T, meta?: Record<string, unknown>): void {
      if (cache.size >= maxSize) { const k = cache.keys().next().value; if (k) cache.delete(k) }
      cache.set(key, { data, timestamp: Date.now(), meta })
    },
    updateMeta(key: string, meta: Record<string, unknown>): boolean {
      const entry = cache.get(key)
      if (!entry) return false
      entry.meta = meta
      return true
    },
    delete(key: string): boolean { return cache.delete(key) },
    deleteWhere(predicate: (meta: Record<string, unknown> | undefined, key: string) => boolean): void {
      for (const [key, entry] of cache) {
        if (predicate(entry.meta, key)) cache.delete(key)
      }
    },
    cleanup(): void {
      const now = Date.now()
      for (const [key, entry] of cache) {
        if (now - entry.timestamp > ttl) cache.delete(key)
      }
    },
  }
}

const telemetryCache = createCache<{ data: number[]; raw?: unknown[] }>(30000, 100)
const systemStatsCache = createCache<unknown>(30000, 40)
export const extensionDataCache = createCache<unknown>(30000, 100)

let cacheCleanupInterval: ReturnType<typeof setInterval> | null = null
if (typeof window !== 'undefined') {
  cacheCleanupInterval = setInterval(() => {
    telemetryCache.cleanup()
    systemStatsCache.cleanup()
    extensionDataCache.cleanup()
  }, 60000)
  window.addEventListener('beforeunload', () => {
    if (cacheCleanupInterval) { clearInterval(cacheCleanupInterval); cacheCleanupInterval = null }
  })
}

export function clearGlobalCacheIntervals() {
  if (cacheCleanupInterval) { clearInterval(cacheCleanupInterval); cacheCleanupInterval = null }
}

// ============================================================================
// Telemetry fetching (from telemetryFetch.ts)
// ============================================================================

const telemetryInflight = new Map<string, Promise<{ data: number[]; raw?: unknown[]; success: boolean }>>()

export async function fetchHistoricalTelemetry(
  deviceId: string,
  metricId: string,
  timeRange: number = 1,
  limit: number = 50,
  aggregate: TelemetryAggregate = 'raw',
  includeRawPoints: boolean = false,
  bypassCache: boolean = false
): Promise<{ data: number[]; raw?: unknown[]; success: boolean }> {
  const cacheKey = `${deviceId}|${metricId}|${timeRange}|${limit}|${aggregate}`
  const cached = telemetryCache.get(cacheKey)

  if (!bypassCache && cached) return { data: cached.data, raw: cached.raw, success: true }
  if (!bypassCache && telemetryInflight.has(cacheKey)) return telemetryInflight.get(cacheKey)!

  const fetchPromise = (async () => {
    try {
      const api = (await import('@/lib/api')).api
      const now = Date.now()
      const effectiveTimeRange = timeRange > 0 ? timeRange : 5 / 60
      const start = now - effectiveTimeRange * 60 * 60 * 1000
      const fetchLimit = aggregate === 'raw' ? limit : Math.max(limit, 100)
      const startSec = Math.floor(start / 1000)
      const endSec = Math.floor(now / 1000)

      // Use unified telemetry endpoint for transform/ai sources, device endpoint otherwise
      const isUnifiedSource = deviceId.startsWith('transform:') || deviceId.startsWith('ai:')
      let metricData: unknown[] | undefined

      if (isUnifiedSource) {
        const response = await api.queryTelemetry(deviceId, metricId, startSec, endSec, fetchLimit)
        metricData = response?.data as unknown[] | undefined
      } else {
        const response = await api.getDeviceTelemetry(deviceId, metricId, startSec, endSec, fetchLimit)

        // Find metric data — exact match, then case-insensitive
        if (response?.data && typeof response.data === 'object') {
          const dataObj = response.data as Record<string, unknown>
          if (Array.isArray(dataObj[metricId])) {
            metricData = dataObj[metricId] as unknown[]
          } else {
            const lowerMetricId = metricId.toLowerCase()
            for (const key of Object.keys(dataObj)) {
              if (key.toLowerCase() === lowerMetricId && Array.isArray(dataObj[key])) {
                metricData = dataObj[key] as unknown[]
                break
              }
            }
          }
        }
      }

      if (Array.isArray(metricData) && metricData.length > 0) {
        const extractValue = (point: unknown): number => {
          if (typeof point === 'number') return point
          if (typeof point === 'object' && point !== null) {
            const p = point as Record<string, unknown>
            const rawValue = p.value ?? p.v ?? p.avg ?? p.min ?? p.max ?? 0
            if (typeof rawValue === 'number') return rawValue
            if (typeof rawValue === 'string') { const parsed = parseFloat(rawValue); return isNaN(parsed) ? 0 : parsed }
            if (typeof rawValue === 'boolean') return rawValue ? 1 : 0
            return 0
          }
          return 0
        }

        const extractTimestamp = (point: unknown): number => {
          if (typeof point === 'object' && point !== null) {
            const p = point as Record<string, unknown>
            const ts = p.timestamp ?? p.time ?? p.t
            if (typeof ts === 'number') return ts > 10000000000 ? Math.floor(ts / 1000) : ts
          }
          return Math.floor(Date.now() / 1000)
        }

        const allValues = metricData.map(extractValue).filter((v: number) => typeof v === 'number' && !isNaN(v))

        let values: number[]
        let rawPoints: unknown[] | undefined

        if (aggregate === 'latest') {
          values = [allValues[0] ?? 0]
          if (includeRawPoints) {
            const pt = metricData[0]
            const rv = typeof pt === 'object' && pt !== null ? (pt as Record<string, unknown>).value ?? (pt as Record<string, unknown>).v ?? pt : pt
            rawPoints = [{ timestamp: extractTimestamp(pt), value: rv }]
          }
        } else if (aggregate === 'first') {
          values = [allValues[allValues.length - 1] ?? 0]
          if (includeRawPoints) {
            const pt = metricData[metricData.length - 1]
            const rv = typeof pt === 'object' && pt !== null ? (pt as Record<string, unknown>).value ?? (pt as Record<string, unknown>).v ?? pt : pt
            rawPoints = [{ timestamp: extractTimestamp(pt), value: rv }]
          }
        } else if (aggregate === 'avg') {
          values = [allValues.length > 0 ? allValues.reduce((a, b) => a + b, 0) / allValues.length : 0]
        } else if (aggregate === 'min') {
          values = [Math.min(...allValues)]
        } else if (aggregate === 'max') {
          values = [Math.max(...allValues)]
        } else if (aggregate === 'sum') {
          values = [allValues.reduce((a, b) => a + b, 0)]
        } else if (aggregate === 'delta') {
          values = [(allValues[0] ?? 0) - (allValues[allValues.length - 1] ?? 0)]
        } else if (aggregate === 'count') {
          values = [allValues.length]
        } else {
          // raw
          values = allValues
          rawPoints = includeRawPoints ? metricData.map((point) => {
            if (typeof point === 'number') return { timestamp: Math.floor(Date.now() / 1000), value: point }
            if (typeof point === 'object' && point !== null) {
              const p = point as Record<string, unknown>
              const ts = p.timestamp ?? p.time ?? p.t
              const timestamp = typeof ts === 'number' ? (ts > 10000000000 ? Math.floor(ts / 1000) : ts) : Math.floor(Date.now() / 1000)
              return { timestamp, value: p.value ?? p.v ?? 0 }
            }
            return { timestamp: Math.floor(Date.now() / 1000), value: point }
          }) : undefined
        }

        telemetryCache.set(cacheKey, { data: values, raw: rawPoints })
        return { data: values, raw: rawPoints, success: true }
      }

      return { data: [], success: false }
    } catch (error) {
      logError(error, { operation: 'Fetch historical telemetry' })
      return { data: [], success: false }
    }
  })()

  if (!bypassCache) {
    telemetryInflight.set(cacheKey, fetchPromise)
    fetchPromise.finally(() => telemetryInflight.delete(cacheKey))
  }

  return fetchPromise
}

// Expose cache for event processing (telemetry cache refresh scheduling)
export { telemetryCache }

// ============================================================================
// System stats fetching (from systemFetch.ts)
// ============================================================================

export async function fetchSystemStats(metric: string): Promise<{ data: unknown; success: boolean }> {
  const cacheKey = `system|${metric}`
  const cached = systemStatsCache.get(cacheKey)
  if (cached !== undefined) return { data: cached, success: true }

  try {
    const api = (await import('@/lib/api')).api
    const stats = await api.getSystemStats()
    if (!stats) return { data: null, success: false }

    let value: unknown = null
    switch (metric) {
      case 'uptime': value = stats.uptime; break
      case 'cpu_count': value = stats.cpu_count; break
      case 'total_memory': value = stats.total_memory / (1024 * 1024 * 1024); break
      case 'used_memory': value = stats.used_memory / (1024 * 1024 * 1024); break
      case 'free_memory': value = stats.free_memory / (1024 * 1024 * 1024); break
      case 'available_memory': value = stats.available_memory / (1024 * 1024 * 1024); break
      case 'memory_percent': value = stats.used_memory / stats.total_memory * 100; break
      case 'platform': value = stats.platform; break
      case 'arch': value = stats.arch; break
      case 'version': value = stats.version; break
    }

    systemStatsCache.set(cacheKey, value)
    return { data: value, success: true }
  } catch (error) {
    logError(error, { operation: 'Fetch system stats' })
    return { data: null, success: false }
  }
}

// ============================================================================
// Batch device fetch (from batchFetch.ts)
// ============================================================================

export const fetchedDevices = new Set<string>()

const activeFetches = new Map<string, Promise<{ success: boolean; metricsCount: number }>>()
let pendingDeviceIds = new Set<string>()
let pendingResolvers = new Map<string, Array<(result: { success: boolean; metricsCount: number }) => void>>()
let batchScheduled = false

function applyBatchResults(
  results: Record<string, unknown>,
  deviceIds: string[],
  resolvers: Map<string, Array<(result: { success: boolean; metricsCount: number }) => void>>
) {
  const store = useStore.getState()
  for (const id of deviceIds) {
    const entry = results[id] as { metrics?: Record<string, { value?: unknown }> } | undefined
    let metricsCount = 0
    if (entry?.metrics) {
      Object.entries(entry.metrics).forEach(([metricName, metricData]) => {
        if (metricData.value !== null && metricData.value !== undefined) {
          store.updateDeviceMetric(id, metricName, metricData.value)
          metricsCount++
        }
      })
    }
    if (metricsCount > 0) {
      fetchedDevices.add(id)
      if (fetchedDevices.size > 200) { const first = fetchedDevices.values().next().value; if (first) fetchedDevices.delete(first) }
    }
    activeFetches.delete(id)
    resolvers.get(id)?.forEach(r => r({ success: metricsCount > 0, metricsCount }))
    resolvers.delete(id)
  }
}

async function flushBatch() {
  const ids = Array.from(pendingDeviceIds)
  const resolvers = pendingResolvers
  pendingDeviceIds = new Set()
  pendingResolvers = new Map()
  batchScheduled = false

  if (ids.length === 0) return

  try {
    const api = (await import('@/lib/api')).api
    const batchResult = await api.getDevicesCurrentBatch(ids)
    if (batchResult?.devices && typeof batchResult.devices === 'object') {
      applyBatchResults(batchResult.devices as Record<string, unknown>, ids, resolvers)
    } else {
      for (const id of ids) {
        activeFetches.delete(id)
        resolvers.get(id)?.forEach(r => r({ success: false, metricsCount: 0 }))
        resolvers.delete(id)
      }
    }
  } catch {
    // Fallback to individual fetches
    try {
      const api = (await import('@/lib/api')).api
      const CHUNK_SIZE = 5
      const individualResults: Array<PromiseSettledResult<{ id: string; success: boolean; metricsCount: number }>> = []
      for (let i = 0; i < ids.length; i += CHUNK_SIZE) {
        const chunk = ids.slice(i, i + CHUNK_SIZE)
        const chunkResults = await Promise.allSettled(chunk.map(async (id) => {
          try {
            const details = await api.getDeviceCurrent(id)
            const store = useStore.getState()
            let metricsCount = 0
            if (details?.metrics) {
              Object.entries(details.metrics).forEach(([metricName, metricData]: [string, unknown]) => {
                const value = (metricData as { value?: unknown }).value
                if (value !== null && value !== undefined) { store.updateDeviceMetric(id, metricName, value); metricsCount++ }
              })
            }
            if (metricsCount > 0) fetchedDevices.add(id)
            return { id, success: metricsCount > 0, metricsCount }
          } catch { return { id, success: false, metricsCount: 0 } }
        }))
        individualResults.push(...chunkResults)
      }
      for (let i = 0; i < ids.length; i++) {
        const id = ids[i]
        const settled = individualResults[i]
        const result = settled?.status === 'fulfilled' ? settled.value : { success: false, metricsCount: 0 }
        activeFetches.delete(id)
        resolvers.get(id)?.forEach(r => r(result))
        resolvers.delete(id)
      }
    } catch {
      for (const id of ids) {
        activeFetches.delete(id)
        resolvers.get(id)?.forEach(r => r({ success: false, metricsCount: 0 }))
        resolvers.delete(id)
      }
    }
  }
}

export async function fetchDeviceTelemetry(deviceId: string): Promise<{ success: boolean; metricsCount: number }> {
  const existing = activeFetches.get(deviceId)
  if (existing) return existing

  const promise = new Promise<{ success: boolean; metricsCount: number }>((resolve) => {
    pendingResolvers.set(deviceId, [...(pendingResolvers.get(deviceId) ?? []), resolve])
  })

  activeFetches.set(deviceId, promise)
  pendingDeviceIds.add(deviceId)

  if (!batchScheduled) { batchScheduled = true; queueMicrotask(flushBatch) }

  return promise
}

export function hasActiveFetch(deviceId: string): boolean {
  return activeFetches.has(deviceId)
}
