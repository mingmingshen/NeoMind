/**
 * useDeviceEventProcessing — Device WebSocket event handling for useDataSource.
 *
 * Processes device metric events and merges them into component state.
 * Handles:
 * - Event deduplication and truncation detection
 * - Device metric store updates
 * - Telemetry point merging with sorting and dedup
 * - Cross-metric interference prevention
 * - Telemetry cache refresh scheduling
 */

import { useEffect, useRef, useMemo } from 'react'
import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import type { Device } from '@/types'
import { useStore } from '@/store'
import { useEvents } from '@/hooks/useEvents'
import { telemetryCache } from './cache'
import { extractValueFromData, eventMetricMatches, safeExtractValue } from './extractors'
import { isDuplicatePoint, isImageDataSource, getDataSourceLimit, dedupeTelemetryPoints } from './dedup'

// ============================================================================
// Types
// ============================================================================

interface UseDeviceEventProcessingOptions {
  dataSources: DataSource[]
  dataSourceKey: string
  enabled: boolean
  preserveMultiple: boolean
  transform: ((data: unknown) => unknown) | undefined
  fallback: unknown
  relevantDeviceIds: Set<string>
  dataRef: React.MutableRefObject<unknown>
  setData: (data: unknown) => void
  setLastUpdate: (ts: number) => void
  telemetryRefreshTimerRef: React.MutableRefObject<ReturnType<typeof setTimeout> | null>
  setTelemetryRefreshTrigger: (fn: (n: number) => number) => void
}

// ============================================================================
// Main Hook
// ============================================================================

export function useDeviceEventProcessing({
  dataSources,
  dataSourceKey,
  enabled,
  preserveMultiple,
  transform,
  fallback,
  relevantDeviceIds,
  dataRef,
  setData,
  setLastUpdate,
  telemetryRefreshTimerRef,
  setTelemetryRefreshTrigger,
}: UseDeviceEventProcessingOptions) {
  // Event processing state
  const processedEventsRef = useRef<Set<string>>(new Set())
  const lastProcessedEventIdRef = useRef<string | null>(null)
  const relevantDeviceIdsRef = useRef(relevantDeviceIds)
  relevantDeviceIdsRef.current = relevantDeviceIds
  const dataSourcesRef = useRef(dataSources)
  dataSourcesRef.current = dataSources

  const needsWebSocket = dataSources.some((ds) =>
    ds.type === 'device' || ds.type === 'metric' || ds.type === 'command' || ds.type === 'telemetry'
  )

  const { events } = useEvents({
    enabled: enabled && needsWebSocket,
    category: 'device',
    onConnected: (connected) => {
      processedEventsRef.current.clear()
      lastProcessedEventIdRef.current = null
    },
  })

  const eventsKey = useMemo(() => {
    if (events.length === 0) return 'empty'
    const lastEvent = events[events.length - 1]
    return `events-${events.length}-${lastEvent?.id || 'unknown'}`
  }, [events])

  useEffect(() => {
    if (dataSources.length === 0 || !enabled || events.length === 0) return

    let startIndex = 0
    const lastProcessedId = lastProcessedEventIdRef.current
    if (lastProcessedId) {
      const lastIndex = events.findIndex(e => e.id === lastProcessedId)
      if (lastIndex !== -1) {
        startIndex = lastIndex + 1
      } else {
        startIndex = 0
        const entries = Array.from(processedEventsRef.current)
        processedEventsRef.current = new Set(entries.slice(-50))
      }
    }
    if (startIndex > events.length) {
      startIndex = 0
      processedEventsRef.current.clear()
    }

    const newEvents = events.slice(startIndex)
    if (newEvents.length === 0) return

    let lastProcessedIdInBatch: string | null = null

    for (const latestEvent of newEvents) {
      const eventData = (latestEvent as any).data || latestEvent
      const eventType = (latestEvent as any).type

      // Skip events for irrelevant devices
      const currentRelevantDeviceIds = relevantDeviceIdsRef.current
      const hasDeviceId = eventData && typeof eventData === 'object' && 'device_id' in eventData
      if (hasDeviceId && currentRelevantDeviceIds.size > 0) {
        const eventDeviceId = eventData.device_id as string
        if (!currentRelevantDeviceIds.has(eventDeviceId)) continue
      }

      // Skip already processed events
      const uniqueEventId = latestEvent.id || `${eventType}_${Date.now()}_${Math.random()}`
      if (processedEventsRef.current.has(uniqueEventId)) continue
      processedEventsRef.current.add(uniqueEventId)
      lastProcessedIdInBatch = uniqueEventId

      if (processedEventsRef.current.size > 100) {
        const entries = Array.from(processedEventsRef.current)
        processedEventsRef.current = new Set(entries.slice(-50))
      }

      // Normalize event type
      const normalizedEventType = eventType?.toLowerCase().replace('.', '')
      const isDeviceMetricEvent = normalizedEventType?.includes('devicemetric') ||
                                   normalizedEventType?.includes('metric') ||
                                   eventType === 'DeviceMetric'
      const eventMetric = typeof (eventData as any).metric === 'string' ? (eventData as any).metric : ''

      let shouldUpdate = false

      // Update store
      if (isDeviceMetricEvent && hasDeviceId) {
        const deviceId = eventData.device_id as string
        const store = useStore.getState()
        if ('metric' in eventData && 'value' in eventData) {
          store.updateDeviceMetric(deviceId, eventData.metric as string, eventData.value)
        }
        for (const [key, value] of Object.entries(eventData)) {
          if (key !== 'device_id' && key !== 'timestamp' && key !== 'type' && key !== 'id' && key !== 'metric' && key !== 'value') {
            store.updateDeviceMetric(deviceId, key, value)
          }
        }
        shouldUpdate = true
      }

      // Check if event matches data sources
      const currentDataSources = dataSourcesRef.current
      for (const ds of currentDataSources) {
        if (ds.type === 'device' && hasDeviceId && eventData.device_id === getSourceId(ds) && isDeviceMetricEvent) {
          shouldUpdate = true; break
        } else if (ds.type === 'metric' && (isDeviceMetricEvent || eventType === 'metric.update')) {
          shouldUpdate = true; break
        } else if (ds.type === 'command' && hasDeviceId && eventData.device_id === getSourceId(ds) && (isDeviceMetricEvent || eventType === 'device.command_result')) {
          shouldUpdate = true; break
        } else if (ds.type === 'telemetry' && hasDeviceId && eventData.device_id === getSourceId(ds) && isDeviceMetricEvent && (!eventMetric || eventMetricMatches(eventMetric, ds.metricId || ds.property || 'value'))) {
          shouldUpdate = true; break
        } else if (ds.type === 'device-info' && hasDeviceId && eventData.device_id === getSourceId(ds) && (isDeviceMetricEvent || eventType === 'DeviceOnline' || eventType === 'DeviceOffline')) {
          shouldUpdate = true; break
        }
      }

      // Telemetry merge (optimized path)
      const hasTelemetrySource = currentDataSources.some((ds) => ds.type === 'telemetry')
      let telemetryAlreadyProcessed = false

      if (hasTelemetrySource && isDeviceMetricEvent && hasDeviceId) {
        const eventDeviceId = eventData.device_id as string
        const matchingTelemetrySources = currentDataSources.filter((ds) => {
          if (ds.type !== 'telemetry' || getSourceId(ds) !== eventDeviceId) return false
          if (!eventMetric) return true
          const metricId = ds.metricId || ds.property || 'value'
          return eventMetric === metricId || eventMetricMatches(eventMetric, metricId)
        })

        if (matchingTelemetrySources.length > 0) {
          telemetryAlreadyProcessed = true
          processTelemetryEvent(eventData, eventMetric, eventDeviceId, currentDataSources, dataRef.current, preserveMultiple, transform, setData, setLastUpdate)

          // Schedule cache refresh
          const refreshDelay = 10000
          matchingTelemetrySources.forEach((ds) => {
            const cacheKey = `${getSourceId(ds)}|${ds.metricId}|${ds.timeRange ?? 1}|${ds.limit ?? 50}|${ds.aggregate ?? ds.aggregateExt ?? 'raw'}`
            const cached = telemetryCache.getWithMeta(cacheKey)
            if (cached && !cached.meta?.refreshing) {
              telemetryCache.updateMeta(cacheKey, { refreshing: true, refreshAfter: Date.now() + refreshDelay })
            }
          })

          if (telemetryRefreshTimerRef.current) clearTimeout(telemetryRefreshTimerRef.current)
          telemetryRefreshTimerRef.current = setTimeout(() => {
            telemetryCache.deleteWhere((meta) => !!meta?.refreshing && !!meta?.refreshAfter && Date.now() >= meta.refreshAfter)
            telemetryRefreshTimerRef.current = null
            setTelemetryRefreshTrigger(prev => prev + 1)
          }, refreshDelay)
        }
      }

      // Non-telemetry event processing
      if (shouldUpdate && !telemetryAlreadyProcessed) {
        processNonTelemetryEvent(eventData, eventType, isDeviceMetricEvent, eventMetric, hasDeviceId, currentDataSources, dataRef.current, preserveMultiple, transform, fallback, setData, setLastUpdate)
      }
    }

    if (lastProcessedIdInBatch) {
      lastProcessedEventIdRef.current = lastProcessedIdInBatch
    }
  }, [enabled, dataSourceKey, eventsKey])

  return { needsWebSocket }
}

// ============================================================================
// Event Processors
// ============================================================================

function processTelemetryEvent(
  eventData: any, eventMetric: string, eventDeviceId: string,
  dataSources: DataSource[], currentData: unknown, preserveMultiple: boolean,
  transform: ((data: unknown) => unknown) | undefined,
  setData: (data: unknown) => void, setLastUpdate: (ts: number) => void
) {
  const now = Math.floor(Date.now() / 1000)
  const rawEventTimestamp = eventData.timestamp
  const eventTimestamp = rawEventTimestamp !== undefined
    ? (typeof rawEventTimestamp === 'number' && rawEventTimestamp > 10000000000
        ? Math.floor(rawEventTimestamp / 1000) : rawEventTimestamp)
    : now

  const getTs = (p: unknown): number => {
    if (p == null) return 0
    const o = p as Record<string, unknown>
    return (o.timestamp ?? o.time ?? o.t ?? 0) as number
  }

  const updatedResults = dataSources.map((ds, index) => {
    if (ds.type !== 'telemetry' || getSourceId(ds) !== eventDeviceId) return undefined

    const dsTimeRange = ds.timeRange ?? 1
    const rangeStartSec = now - Math.floor(dsTimeRange * 3600)
    if (eventTimestamp < rangeStartSec) return undefined

    const metricId = ds.metricId || ds.property || 'value'
    let eventValue: unknown
    const metricMatches = eventMetric === metricId || eventMetricMatches(eventMetric, metricId)

    if ('value' in eventData && metricMatches) {
      eventValue = eventData.value
    } else if (!eventMetric) {
      eventValue = extractValueFromData(eventData, metricId)
    } else {
      return undefined
    }
    if (eventValue === undefined) return undefined

    const newPoint = { timestamp: eventTimestamp, time: eventTimestamp, value: eventValue }
    const isImg = isImageDataSource(ds.params, ds.transform, metricId)
    const maxLimit = getDataSourceLimit(ds)

    let currentArray: unknown[] = []
    if (Array.isArray(currentData)) {
      if (preserveMultiple && dataSources.length > 1 && Array.isArray(currentData[index])) {
        currentArray = currentData[index] as unknown[]
      } else if (dataSources.length === 1 || !preserveMultiple) {
        currentArray = currentData as unknown[]
      }
    }

    // Image dedup check
    if (isImg && isDuplicatePoint(currentArray, eventTimestamp, eventValue, getTs)) return undefined

    const merged = [newPoint, ...currentArray]
    const idx = merged.map((p, i) => ({ p, i }))
    idx.sort((a, b) => { const d = getTs(b.p) - getTs(a.p); return d !== 0 ? d : a.i - b.i })
    const sorted = idx.map(({ p }) => p)

    return isImg ? sorted.slice(0, maxLimit) : dedupeTelemetryPoints(sorted, getTs, maxLimit)
  })

  const hasUpdated = updatedResults.some((r) => r !== undefined)
  if (!hasUpdated) return

  const validResults = updatedResults.filter((r) => r !== undefined)
  let finalData: unknown
  if (preserveMultiple && dataSources.length > 1) {
    finalData = updatedResults.map((r, i) => r !== undefined ? r : (Array.isArray(currentData) && currentData[i] !== undefined ? currentData[i] : []))
  } else {
    finalData = validResults[0] ?? currentData
  }

  const transformedData = transform ? transform(finalData) : finalData
  setData(transformedData)
  setLastUpdate(Date.now())
}

function processNonTelemetryEvent(
  eventData: any, eventType: string, isDeviceMetricEvent: boolean, eventMetric: string,
  hasDeviceId: boolean, dataSources: DataSource[], currentData: unknown,
  preserveMultiple: boolean, transform: ((data: unknown) => unknown) | undefined,
  fallback: unknown, setData: (data: unknown) => void, setLastUpdate: (ts: number) => void
) {
  const currentDevices = useStore.getState().devices
  const results = dataSources.map((ds, index) => {
    let result: unknown

    switch (ds.type) {
      case 'device': {
        const deviceId = getSourceId(ds)!
        const property = ds.property as string | undefined
        if (!property) {
          result = currentDevices.find((d: Device) => d.id === deviceId || d.device_id === deviceId) ?? null
          break
        }
        if (isDeviceMetricEvent && eventData.device_id === deviceId) {
          const metricMatches = eventMetric === property || eventMetricMatches(eventMetric, property)
          if ('metric' in eventData && 'value' in eventData && metricMatches) { result = eventData.value; break }
          if (!eventMetric) {
            const extracted = extractValueFromData(eventData, property)
            if (extracted !== undefined) { result = extracted; break }
          }
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
          if (!eventMetric) {
            const extracted = extractValueFromData(eventData, metricId)
            if (extracted !== undefined) { result = extracted; break }
          }
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
          if (!eventMetric) {
            const extracted = extractValueFromData(eventData, property)
            if (extracted !== undefined) { result = extracted; break }
          }
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
        // Handled by processTelemetryEvent
        if (Array.isArray(currentData) && currentData[index] !== undefined) {
          result = dataSources.length > 1 ? currentData[index] : currentData
        } else {
          result = fallback ?? []
        }
        break
      }
      default: return
    }
    return result
  })

  let finalData: unknown = dataSources.length > 1 ? results : results[0]
  const transformedData = transform ? transform(finalData) : finalData
  setData(transformedData)
  setLastUpdate(Date.now())
}
