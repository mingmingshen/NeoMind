/**
 * Pure functions extracted from useDataSource for event processing and data manipulation.
 * No React imports — these are stateless utilities.
 */

import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import type { Device } from '@/types'
import { useStore } from '@/store'
import {
  extractValueFromData, safeExtractValue, eventMetricMatches,
  getPointValue, isImageDataSource, getDataSourceLimit,
  isDuplicatePoint, dedupeTelemetryPoints, sortAndDedup,
} from './helpers'
import { timeWindowToHours } from '@/lib/telemetryTransform'

// ============================================================================
// Timestamp utilities
// ============================================================================

export const getTs = (p: unknown): number => {
  if (p == null) return 0
  if (typeof p === 'number') return 0
  const o = p as Record<string, unknown>
  return (o.timestamp ?? o.time ?? o.t ?? 0) as number
}

/**
 * Get the newest timestamp from telemetry data (handles both flat arrays
 * and nested preserveMultiple arrays).
 */
export function getNewestTimestamp(data: unknown, tsFn: (p: unknown) => number): number {
  if (!Array.isArray(data)) return 0
  if (data.length === 0) return 0

  // Nested arrays (preserveMultiple)
  if (Array.isArray(data[0])) {
    let max = 0
    for (const arr of data as unknown[][]) {
      for (const p of arr) { const ts = tsFn(p); if (ts > max) max = ts }
    }
    return max
  }

  let max = 0
  for (const p of data) { const ts = tsFn(p); if (ts > max) max = ts }
  return max
}

/**
 * Extract live points from prevData that are strictly newer than `afterTs`.
 */
export function extractPointsNewerThan(
  prevData: unknown, afterTs: number,
  tsFn: (p: unknown) => number,
  preserveMultiple: boolean, sourceCount: number
): unknown[][] {
  if (!Array.isArray(prevData)) return []
  if (preserveMultiple && sourceCount > 1 && Array.isArray(prevData[0])) {
    return (prevData as unknown[][]).map(arr => {
      if (!Array.isArray(arr)) return []
      return arr.filter(p => tsFn(p) > afterTs)
    })
  }
  const live = (prevData as unknown[]).filter(p => tsFn(p) > afterTs)
  return live.length > 0 ? [live] : []
}

/**
 * Merge live points back into fetched data, deduplicating by timestamp.
 */
export function mergeLiveData(
  fetchedData: unknown, livePoints: unknown[][],
  tsFn: (p: unknown) => number,
  preserveMultiple: boolean, sources: DataSource[]
): unknown {
  if (preserveMultiple && sources.length > 1 && Array.isArray(fetchedData)) {
    const maxLimit = (ds: DataSource) => getDataSourceLimit(ds)
    return sources.map((ds, i) => {
      const fetched = (fetchedData as unknown[][])[i] ?? []
      const live = livePoints[i] ?? []
      const merged = [...live, ...fetched]
      const sorted = sortAndDedup(merged, tsFn, maxLimit(ds), isImageDataSource(ds.params, ds.transform, ds.metricId))
      sorted.reverse()  // sortAndDedup returns newest-first; reverse to oldest-first for chart X-axis
      return sorted
    })
  }
  const fetched = Array.isArray(fetchedData) ? fetchedData : []
  const live = livePoints[0] ?? []
  const ds = sources[0]
  const maxLimit = ds ? getDataSourceLimit(ds) : 50
  const sorted = sortAndDedup([...live, ...fetched], tsFn, maxLimit, ds ? isImageDataSource(ds.params, ds.transform, ds.metricId) : false)
  sorted.reverse()  // sortAndDedup returns newest-first; reverse to oldest-first for chart X-axis
  return sorted
}

// ============================================================================
// Sorting helpers
// ============================================================================

export function normalizeOutputName(outputName: string): string {
  if (!outputName.includes(':')) return outputName
  return outputName.split(':').slice(1).join(':')
}

export function sortArrayByTs(points: unknown[], tsFn: (p: unknown) => number): unknown[] {
  const idx = points.map((p, i) => ({ p, i }))
  idx.sort((a, b) => { const d = tsFn(b.p) - tsFn(a.p); return d !== 0 ? d : a.i - b.i })
  return idx.map(({ p }) => p)
}

export function sortTelemetryResults(finalData: unknown, sources: DataSource[], preserveMultiple: boolean): unknown {
  const isPM = preserveMultiple && sources.length > 1
  const process = (points: unknown[], ds: DataSource): unknown[] => {
    if (!Array.isArray(points) || points.length === 0) return points
    const isImg = isImageDataSource(ds.params, ds.transform, ds.metricId)
    const maxLimit = getDataSourceLimit(ds)
    const sorted = sortAndDedup(points, getTs, maxLimit, isImg)
    // Reverse to ascending order (oldest first) so charts render left→right timeline
    sorted.reverse()
    return sorted
  }
  if (isPM && Array.isArray(finalData)) return sources.map((ds, i) => process((finalData as unknown[][])[i], ds))
  if (Array.isArray(finalData) && sources.length > 0) return process(finalData, sources[0])
  return finalData
}

// ============================================================================
// Event processors
// ============================================================================

export function processTelemetryEvent(
  eventData: any, eventMetric: string, eventDeviceId: string,
  dataSources: DataSource[], preserveMultiple: boolean,
  transform: ((data: unknown) => unknown) | undefined,
  setData: (updater: (prev: unknown) => unknown) => void, setLastUpdate: (ts: number) => void
) {
  const now = Math.floor(Date.now() / 1000)
  const rawEventTimestamp = eventData.timestamp
  const eventTimestamp = rawEventTimestamp !== undefined
    ? (typeof rawEventTimestamp === 'number' && rawEventTimestamp > 10000000000
        ? Math.floor(rawEventTimestamp / 1000) : rawEventTimestamp)
    : now

  // Pre-compute event values outside the updater to avoid recomputation
  const matchedResults = dataSources.map((ds) => {
    if (ds.type !== 'telemetry' || getSourceId(ds) !== eventDeviceId) return undefined
    // Use timeWindow for accurate range; fall back to timeRange (hours) for legacy sources
    const dsTimeRangeHours = ds.timeWindow
      ? timeWindowToHours(ds.timeWindow.type)
      : (ds.timeRange ?? 1)
    const rangeStartSec = now - Math.floor(dsTimeRangeHours * 3600)
    if (eventTimestamp < rangeStartSec) return undefined

    const metricId = ds.metricId || ds.property || 'value'
    let eventValue: unknown
    const metricMatches = eventMetric === metricId || eventMetricMatches(eventMetric, metricId)

    if ('value' in eventData && metricMatches) { eventValue = eventData.value }
    else if (!eventMetric) { eventValue = extractValueFromData(eventData, metricId) }
    else return undefined
    if (eventValue === undefined) return undefined

    const isImg = isImageDataSource(ds.params, ds.transform, metricId)
    return { eventValue, isImg, metricId }
  })

  if (!matchedResults.some((r) => r !== undefined)) return

  setData((prevData: unknown) => {
    const currentData = prevData
    const updatedResults = dataSources.map((ds, index) => {
      const matched = matchedResults[index]
      if (!matched) return undefined

      const maxLimit = getDataSourceLimit(ds)
      const newPoint = { timestamp: eventTimestamp, time: eventTimestamp, value: matched.eventValue }

      let currentArray: unknown[] = []
      if (Array.isArray(currentData)) {
        if (preserveMultiple && dataSources.length > 1 && Array.isArray(currentData[index])) {
          currentArray = currentData[index] as unknown[]
        } else if (dataSources.length === 1 || !preserveMultiple) {
          currentArray = currentData as unknown[]
        }
      }

      if (matched.isImg && isDuplicatePoint(currentArray, eventTimestamp, matched.eventValue, getTs)) return undefined

      const sorted = sortArrayByTs([newPoint, ...currentArray], getTs)
      // sortArrayByTs returns newest-first; reverse to oldest-first for chart X-axis
      sorted.reverse()
      return matched.isImg ? sorted.slice(0, maxLimit) : dedupeTelemetryPoints(sorted, getTs, maxLimit)
    })

    if (!updatedResults.some((r) => r !== undefined)) return currentData

    const validResults = updatedResults.filter((r) => r !== undefined)
    let finalData: unknown
    if (preserveMultiple && dataSources.length > 1) {
      finalData = updatedResults.map((r, i) => r !== undefined ? r : (Array.isArray(currentData) && currentData[i] !== undefined ? currentData[i] : []))
    } else {
      finalData = validResults[0] ?? currentData
    }

    return transform ? transform(finalData) : finalData
  })
  setLastUpdate(Date.now())
}

export function processNonTelemetryEvent(
  eventData: any, eventType: string, isDeviceMetricEvent: boolean, eventMetric: string,
  hasDeviceId: boolean, dataSources: DataSource[],
  preserveMultiple: boolean, transform: ((data: unknown) => unknown) | undefined,
  fallback: unknown, setData: (updater: (prev: unknown) => unknown) => void, setLastUpdate: (ts: number) => void
) {
  const currentDevices = useStore.getState().devices

  setData((prevData: unknown) => {
    const results = dataSources.map((ds, index) => {
      let result: unknown

      switch (ds.type) {
        case 'device': {
          const deviceId = getSourceId(ds)!
          const property = ds.property as string | undefined
          if (!property) {
            result = currentDevices.find((d: Device) => d.id === deviceId || d.device_id === deviceId) ?? null; break
          }
          if (isDeviceMetricEvent && eventData.device_id === deviceId) {
            const metricMatches = eventMetric === property || eventMetricMatches(eventMetric, property)
            if ('metric' in eventData && 'value' in eventData && metricMatches) { result = eventData.value; break }
            if (!eventMetric) { const extracted = extractValueFromData(eventData, property); if (extracted !== undefined) { result = extracted; break } }
          }
          const device = currentDevices.find((d: Device) => d.id === deviceId)
          if (device?.current_values && typeof device.current_values === 'object') {
            result = extractValueFromData(device.current_values, property) ?? '-'
          } else { result = '-' }
          result = safeExtractValue(result, '-')
          break
        }
        case 'metric': {
          const metricId = ds.metricId ?? 'value'
          if (isDeviceMetricEvent) {
            const metricMatches = eventMetric === metricId || eventMetricMatches(eventMetric, metricId)
            if ('metric' in eventData && 'value' in eventData && metricMatches) { result = eventData.value; break }
            if (!eventMetric) { const extracted = extractValueFromData(eventData, metricId); if (extracted !== undefined) { result = extracted; break } }
          }
          for (const device of currentDevices) {
            if (device.current_values && typeof device.current_values === 'object') {
              const value = extractValueFromData(device.current_values, metricId)
              if (value !== undefined) { result = value; break }
            }
          }
          if (result === undefined) result = fallback ?? '-'
          result = safeExtractValue(result, '-')
          break
        }
        case 'command': {
          const deviceId = getSourceId(ds)
          const property = ds.property || 'state'
          if (isDeviceMetricEvent && eventData.device_id === deviceId) {
            const metricMatches = eventMetric === property || eventMetricMatches(eventMetric, property)
            if ('metric' in eventData && 'value' in eventData && metricMatches) { result = eventData.value; break }
            if (!eventMetric) { const extracted = extractValueFromData(eventData, property); if (extracted !== undefined) { result = extracted; break } }
          }
          const device = currentDevices.find((d: Device) => d.id === deviceId)
          result = device?.current_values ? (extractValueFromData(device.current_values, property) ?? false) : false
          result = safeExtractValue(result, false)
          break
        }
        case 'device-info': {
          const deviceId = getSourceId(ds)
          const infoProperty = ds.infoProperty || 'name'
          const device = currentDevices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)
          if (!device) { result = fallback ?? '-' }
          else {
            switch (infoProperty) {
              case 'name': result = device.name || '-'; break
              case 'status': result = device.status || 'unknown'; break
              case 'online': result = device.online ?? false; break
              case 'last_seen': result = device.last_seen || '-'; break
              case 'device_type': result = device.device_type || '-'; break
              case 'plugin_name': result = device.plugin_name || device.adapter_id || '-'; break
              case 'adapter_id': result = device.adapter_id || '-'; break
              default: result = fallback ?? '-'
            }
          }
          result = safeExtractValue(result as unknown, (fallback ?? '-') as any)
          break
        }
        case 'telemetry': {
          if (Array.isArray(prevData) && (prevData as unknown[])[index] !== undefined) {
            result = dataSources.length > 1 ? (prevData as unknown[])[index] : prevData
          } else { result = fallback ?? [] }
          break
        }
        default: return undefined
      }
      return result
    })

    let finalData: unknown = dataSources.length > 1 ? results : results[0]
    return transform ? transform(finalData) : finalData
  })
  setLastUpdate(Date.now())
}
