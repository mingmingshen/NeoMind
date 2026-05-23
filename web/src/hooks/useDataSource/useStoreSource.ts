/**
 * useStoreSource — handles store-based data reading, store subscriptions,
 * and device WebSocket event processing for device/metric/command/device-info sources.
 */

import { useEffect, useCallback, useRef, useMemo } from 'react'
import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import type { Device } from '@/types'
import type { NeoMindStore } from '@/store'
import { useStore } from '@/store'
import { useEvents } from '@/hooks/useEvents'
import {
  extractValueFromData, safeExtractValue, eventMetricMatches,
  getDataSourceLimit, dedupeTelemetryPoints,
} from './helpers'
import { fetchDeviceTelemetry, fetchedDevices, hasActiveFetch, telemetryCache } from './fetch'
import { processTelemetryEvent, processNonTelemetryEvent, sortArrayByTs, getTs } from './eventProcessors'

// Performance probe
const PERF_THRESHOLD = 100
function perfMark(label: string) {
  if (typeof localStorage !== 'undefined' && localStorage.getItem('DEBUG_SCROLL') === 'true') {
    performance.mark(`uds:${label}`)
  }
}
function perfEnd(label: string) {
  if (typeof localStorage !== 'undefined' && localStorage.getItem('DEBUG_SCROLL') === 'true') {
    try {
      performance.measure(`uds:${label}`, `uds:${label}`)
      const entries = performance.getEntriesByName(`uds:${label}`, 'measure')
      const last = entries[entries.length - 1]
      if (last && last.duration > PERF_THRESHOLD) {
        console.warn(`[perf] useDataSource ${label}: ${Math.round(last.duration)}ms`)
      }
    } catch { /* ignore */ }
  }
}

export interface StoreSourceState<T> {
  data: T | null
  setData: (value: T | ((prev: T | null) => T | null)) => void
  setDataRaw: (d: unknown) => void
  setLoading: (l: boolean) => void
  setError: (e: string | null) => void
  setLastUpdate: (ts: number | null) => void
  dataSourcesRef: React.MutableRefObject<DataSource[]>
  optionsRef: React.MutableRefObject<{
    enabled: boolean
    transform?: (data: unknown) => T
    fallback?: T
    preserveMultiple: boolean
  }>
}

/**
 * Process a single telemetry point into currentData.
 * Returns updated data or undefined if no change was needed.
 * Inlined from processTelemetryEvent for batch processing.
 */
function processSingleTelemetryPoint(
  currentData: unknown,
  newPoint: { timestamp: number; time: number; value: unknown },
  dataSources: DataSource[],
  ds: DataSource,
  preserveMultiple: boolean,
  maxLimit: number
): unknown | undefined {
  let currentArray: unknown[] = []
  if (Array.isArray(currentData)) {
    if (preserveMultiple && dataSources.length > 1) {
      const dsIndex = dataSources.indexOf(ds)
      if (dsIndex >= 0 && Array.isArray((currentData as unknown[])[dsIndex])) {
        currentArray = (currentData as unknown[])[dsIndex] as unknown[]
      }
    } else if (dataSources.length === 1 || !preserveMultiple) {
      currentArray = currentData as unknown[]
    }
  }

  const sorted = sortArrayByTs([newPoint, ...currentArray], getTs)
  sorted.reverse()
  const updated = dedupeTelemetryPoints(sorted, getTs, maxLimit)

  if (preserveMultiple && dataSources.length > 1) {
    const dsIndex = dataSources.indexOf(ds)
    const result = (currentData as unknown[]).map((item, i) =>
      i === dsIndex ? updated : item
    )
    return result
  }
  return updated
}

export function useStoreSource<T = unknown>(
  dataSources: DataSource[],
  dataSourceKey: string,
  enabled: boolean,
  relevantDeviceIds: Set<string>,
  deviceInfoIds: Set<string>,
  hasTelemetrySource: boolean,
  needsWebSocket: boolean,
  state: StoreSourceState<T>,
  hasExtensionSource?: boolean
): { readDataFromStore: () => void } {
  const initialFetchDoneRef = useRef<Set<string>>(new Set())
  const lastValidDataRef = useRef<Record<string, unknown>>({})
  const prevStoreStateRef = useRef<{ devices: NeoMindStore['devices'] } | null>(null)

  // Event processing refs
  const processedDeviceEventsRef = useRef<Set<string>>(new Set())
  const lastProcessedDeviceEventIdRef = useRef<string | null>(null)

  const { fallback } = state.optionsRef.current

  // ============================================================================
  // readDataFromStore (device/metric/command/device-info)
  // ============================================================================

  const readDataFromStore = useCallback(() => {
    perfMark('readData')
    const { transform: transformFn, fallback: fallbackVal } = state.optionsRef.current
    const currentDataSources = state.dataSourcesRef.current
    const currentDevices = useStore.getState().devices

    if (currentDataSources.length === 0) {
      if (fallbackVal !== undefined) state.setData(fallbackVal)
      state.setLoading(false)
      return
    }

    const nonTelemetrySources = currentDataSources.filter(
      (ds) => ds.type !== 'telemetry' && ds.type !== 'system' && ds.type !== 'extension' && ds.type !== 'transform' && ds.type !== 'ai-metric'
    )

    // When all sources are telemetry/system/extension, readDataFromStore has nothing to do.
    // Don't touch loading state — those hooks manage their own loading lifecycle.
    if (nonTelemetrySources.length === 0) return

    try {
      const results = nonTelemetrySources.map((ds) => {
        let result: unknown

        switch (ds.type) {
          case 'device': {
            const deviceId = getSourceId(ds)!
            const property = ds.property as string | undefined
            const device = currentDevices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)

            if (!property) { result = device ?? null; break }

            const cacheKey = `${deviceId}:${property}`
            if (device?.current_values && typeof device.current_values === 'object' && Object.keys(device.current_values).length > 0) {
              const extracted = extractValueFromData(device.current_values, property)
              if (extracted !== undefined) {
                result = extracted
                lastValidDataRef.current[cacheKey] = extracted
              } else {
                let foundNested = false
                for (const nestedKey of ['values', 'metrics', 'data']) {
                  if (device.current_values[nestedKey] && typeof device.current_values[nestedKey] === 'object') {
                    const nestedValue = extractValueFromData(device.current_values[nestedKey] as Record<string, unknown>, property)
                    if (nestedValue !== undefined) { result = nestedValue; foundNested = true; lastValidDataRef.current[cacheKey] = nestedValue; break }
                  }
                }
                if (!foundNested) result = lastValidDataRef.current[cacheKey] ?? '-'
              }
            } else if (device) {
              if (initialFetchDoneRef.current.has(deviceId) || fetchedDevices.has(deviceId) || hasActiveFetch(deviceId)) {
                result = lastValidDataRef.current[cacheKey] ?? '-'
              } else {
                initialFetchDoneRef.current.add(deviceId)
                fetchDeviceTelemetry(deviceId).catch(() => {})
                result = lastValidDataRef.current[cacheKey] ?? '-'
              }
            } else {
              if (initialFetchDoneRef.current.has(deviceId) || hasActiveFetch(deviceId)) {
                result = lastValidDataRef.current[cacheKey] ?? '-'
              } else {
                initialFetchDoneRef.current.add(deviceId)
                import('@/lib/api').then(({ api }) => {
                  api.getDevices().then(() => fetchDeviceTelemetry(deviceId)).catch(() => {})
                })
                result = lastValidDataRef.current[cacheKey] ?? '-'
              }
            }
            result = safeExtractValue(result, '-')
            break
          }
          case 'metric': {
            const metricId = ds.metricId ?? 'value'
            for (const device of currentDevices) {
              if (device.current_values && typeof device.current_values === 'object') {
                const value = extractValueFromData(device.current_values, metricId)
                if (value !== undefined) { result = value; break }
              }
            }
            if (result === undefined) result = fallbackVal ?? '-'
            result = safeExtractValue(result, '-')
            break
          }
          case 'command': {
            const deviceId = getSourceId(ds)
            const property = ds.property || 'state'
            const device = currentDevices.find((d: Device) => d.id === deviceId)
            if (device?.current_values && typeof device.current_values === 'object') {
              result = extractValueFromData(device.current_values, property) ?? false
            } else { result = false }
            result = safeExtractValue(result, false)
            break
          }
          case 'device-info': {
            const deviceId = getSourceId(ds)
            const infoProperty = ds.infoProperty || 'name'
            const device = currentDevices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)
            if (!device) { result = fallbackVal ?? '-' }
            else {
              switch (infoProperty) {
                case 'name': result = device.name || '-'; break
                case 'status': result = device.status || 'unknown'; break
                case 'online': result = device.online ?? false; break
                case 'last_seen': result = device.last_seen || '-'; break
                case 'device_type': result = device.device_type || '-'; break
                case 'plugin_name': result = device.plugin_name || device.adapter_id || '-'; break
                case 'adapter_id': result = device.adapter_id || '-'; break
                default: result = fallbackVal ?? '-'
              }
            }
            result = safeExtractValue(result as unknown, (fallbackVal ?? '-') as any)
            break
          }
          default: result = fallbackVal ?? null
        }
        return result
      })

      let finalData: unknown
      if (nonTelemetrySources.length > 0 && currentDataSources.length > 1) {
        finalData = results
      } else {
        finalData = results[0]
      }

      const transformedData = transformFn ? transformFn(finalData) : (finalData as T)
      state.setData(transformedData)
      state.setLastUpdate(Date.now())
    } catch (err) {
      const { devicesLoading } = useStore.getState()
      if (!devicesLoading) state.setError(err instanceof Error ? err.message : 'Unknown error')
      state.setData((fallback ?? 0) as T)
    } finally {
      state.setLoading(false)
      perfEnd('readData')
    }
  }, [])

  // ============================================================================
  // Store subscription (device/metric/command/device-info real-time)
  // ============================================================================

  useEffect(() => {
    if (dataSources.length === 0) {
      const { fallback: fallbackVal } = state.optionsRef.current
      if (fallbackVal !== undefined) state.setData(fallbackVal)
      state.setLoading(false)
      return
    }
    if (!enabled) { state.setLoading(false); return }
    if (relevantDeviceIds.size === 0) {
      readDataFromStore()
      // readDataFromStore returns early for telemetry-only/extension-only sources
      // without touching loading state — those hooks manage their own lifecycle.
      // Only set loading=false when no async source will handle it.
      if (!hasTelemetrySource && !hasExtensionSource) state.setLoading(false)
      return
    }

    readDataFromStore()
    prevStoreStateRef.current = { devices: useStore.getState().devices }

    let unsubscribed = false

    const processStoreChange = (s: NeoMindStore) => {
      if (unsubscribed) return
      perfMark('storeChange')

      const prev = prevStoreStateRef.current
      if (!prev) return
      if (s.devices === prev.devices) return

      const devicesChanged = s.devices !== prev.devices
      const devicesLengthChanged = s.devices.length !== prev.devices.length
      let currentValuesChanged = false
      const changedDeviceIds = new Set<string>()

      if (!devicesLengthChanged) {
        const currDevices = s.devices
        const prevDevices = prev.devices

        for (const deviceId of relevantDeviceIds) {
          const device = currDevices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)
          const prevDevice = prevDevices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)

          if (device && prevDevice) {
            if (device.current_values !== prevDevice.current_values) {
              if (device.current_values && Object.keys(device.current_values).length > 0) {
                currentValuesChanged = true
                changedDeviceIds.add(deviceId)
              }
            }
            if (deviceInfoIds.has(deviceId)) {
              if (device.status !== prevDevice.status || device.online !== prevDevice.online || device.last_seen !== prevDevice.last_seen) {
                currentValuesChanged = true
                changedDeviceIds.add(deviceId)
              }
            }
          } else if (device && !prevDevice) {
            if (device.current_values && Object.keys(device.current_values).length > 0) {
              currentValuesChanged = true
              changedDeviceIds.add(deviceId)
            }
          } else if (!device && prevDevice) {
            currentValuesChanged = true
          }
        }
      }

      if (devicesChanged || devicesLengthChanged || currentValuesChanged) {
        const hasRelevantChange = devicesLengthChanged ||
          Array.from(changedDeviceIds).some((id) => relevantDeviceIds.has(id)) ||
          (devicesChanged && changedDeviceIds.size === 0)
        if (!hasRelevantChange && !currentValuesChanged) return

        prevStoreStateRef.current = { devices: s.devices }
        readDataFromStore()

        // Telemetry merge from store changes — build synthetic events for all
        // changed devices/metrics, then process in a single setData call
        const currentDataSources = state.dataSourcesRef.current
        const telSources = currentDataSources.filter((ds) => ds.type === 'telemetry')
        if (telSources.length > 0 && currentValuesChanged && changedDeviceIds.size > 0) {
          const { preserveMultiple: pm, transform: tf } = state.optionsRef.current
          const now = Math.floor(Date.now() / 1000)
          const cacheKeysToInvalidate: string[] = []

          state.setData((prevData) => {
            let currentData = prevData as unknown

            for (const deviceId of changedDeviceIds) {
              const device = s.devices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)
              if (!device?.current_values || typeof device.current_values !== 'object') continue

              for (const ds of currentDataSources) {
                if (ds.type !== 'telemetry' || getSourceId(ds) !== deviceId) continue
                const metricId = ds.metricId || ds.property || 'value'
                const latestValue = extractValueFromData(device.current_values, metricId)
                if (latestValue === undefined) continue

                // Inline the processTelemetryEvent logic for a single point
                const newPoint = { timestamp: now, time: now, value: latestValue }
                const maxLimit = getDataSourceLimit(ds)

                // Update currentData in-place for next iteration
                const updated = processSingleTelemetryPoint(
                  currentData, newPoint, currentDataSources, ds, pm, maxLimit
                )
                if (updated !== undefined) {
                  currentData = updated
                  cacheKeysToInvalidate.push(`${getSourceId(ds)}|${ds.metricId}|${ds.aggregateExt ?? 'raw'}|`)
                }
              }
            }

            if (currentData === prevData) return prevData
            return (tf ? tf(currentData) : currentData) as T
          })

          // Invalidate cache outside of setData
          for (const prefix of cacheKeysToInvalidate) {
            telemetryCache.deleteWhere((_, key) => key.startsWith(prefix))
          }
          if (cacheKeysToInvalidate.length > 0) {
            state.setLastUpdate(Date.now())
          }
        }
      }
      perfEnd('storeChange')
    }

    const unsubscribe = useStore.subscribe((s: NeoMindStore) => {
      processStoreChange(s)
    })

    return () => {
      unsubscribed = true
      unsubscribe()
    }
  }, [dataSources.length, enabled, relevantDeviceIds, deviceInfoIds])

  // ============================================================================
  // Device WebSocket event processing
  // ============================================================================

  const { events } = useEvents({
    enabled: enabled && needsWebSocket,
    category: 'device',
    onConnected: () => {
      processedDeviceEventsRef.current.clear()
      lastProcessedDeviceEventIdRef.current = null
      // Re-fetch data to fill gaps from disconnect period
      if (relevantDeviceIds.size > 0) {
        readDataFromStore()
      }
    },
  })

  const eventsKey = useMemo(() => {
    if (events.length === 0) return 'empty'
    const lastEvent = events[events.length - 1]
    return `events-${events.length}-${lastEvent?.id || 'unknown'}`
  }, [events])

  useEffect(() => {
    if (dataSources.length === 0 || !enabled || events.length === 0) return
    perfMark('events')

    let startIndex = 0
    const lastProcessedId = lastProcessedDeviceEventIdRef.current
    if (lastProcessedId) {
      const lastIndex = events.findIndex(e => e.id === lastProcessedId)
      if (lastIndex !== -1) startIndex = lastIndex + 1
      else { startIndex = 0; const entries = Array.from(processedDeviceEventsRef.current); processedDeviceEventsRef.current = new Set(entries.slice(-50)) }
    }
    if (startIndex > events.length) { startIndex = 0; processedDeviceEventsRef.current.clear() }

    const newEvents = events.slice(startIndex)
    if (newEvents.length === 0) return

    let lastProcessedIdInBatch: string | null = null

    for (const latestEvent of newEvents) {
      const eventData = (latestEvent as any).data || latestEvent
      const eventType = (latestEvent as any).type

      const hasDeviceId = eventData && typeof eventData === 'object' && 'device_id' in eventData
      if (hasDeviceId && relevantDeviceIds.size > 0) {
        if (!relevantDeviceIds.has(eventData.device_id as string)) continue
      }

      // Deterministic event ID: use event content hash to avoid duplicates
      // even when event.id is missing
      const uniqueEventId = latestEvent.id || `${eventType}_${eventData.device_id || ''}_${eventData.metric || ''}_${eventData.timestamp || ''}_${hasDeviceId ? JSON.stringify(eventData.value) : ''}`
      if (processedDeviceEventsRef.current.has(uniqueEventId)) continue
      processedDeviceEventsRef.current.add(uniqueEventId)
      lastProcessedIdInBatch = uniqueEventId

      if (processedDeviceEventsRef.current.size > 100) {
        const entries = Array.from(processedDeviceEventsRef.current)
        processedDeviceEventsRef.current = new Set(entries.slice(-50))
      }

      const normalizedEventType = eventType?.toLowerCase().replace('.', '')
      const isDeviceMetricEvent = normalizedEventType?.includes('devicemetric') ||
                                   normalizedEventType?.includes('metric') || eventType === 'DeviceMetric'
      const eventMetric = typeof (eventData as any).metric === 'string' ? (eventData as any).metric : ''

      let shouldUpdate = false

      // Update store metrics
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
      const currentDataSources = state.dataSourcesRef.current
      for (const ds of currentDataSources) {
        if (ds.type === 'device' && hasDeviceId && eventData.device_id === getSourceId(ds) && isDeviceMetricEvent) { shouldUpdate = true; break }
        else if (ds.type === 'metric' && (isDeviceMetricEvent || eventType === 'metric.update')) { shouldUpdate = true; break }
        else if (ds.type === 'command' && hasDeviceId && eventData.device_id === getSourceId(ds) && (isDeviceMetricEvent || eventType === 'device.command_result')) { shouldUpdate = true; break }
        else if (ds.type === 'telemetry' && hasDeviceId && eventData.device_id === getSourceId(ds) && isDeviceMetricEvent && (!eventMetric || eventMetricMatches(eventMetric, ds.metricId || ds.property || 'value'))) { shouldUpdate = true; break }
        else if (ds.type === 'device-info' && hasDeviceId && eventData.device_id === getSourceId(ds) && (isDeviceMetricEvent || eventType === 'DeviceOnline' || eventType === 'DeviceOffline')) { shouldUpdate = true; break }
      }

      // Telemetry merge (optimized path)
      const hasTelemetrySrc = currentDataSources.some((ds) => ds.type === 'telemetry')
      let telemetryAlreadyProcessed = false

      if (hasTelemetrySrc && isDeviceMetricEvent && hasDeviceId) {
        const eventDeviceId = eventData.device_id as string
        const matchingSources = currentDataSources.filter((ds) => {
          if (ds.type !== 'telemetry' || getSourceId(ds) !== eventDeviceId) return false
          if (!eventMetric) return true
          const metricId = ds.metricId || ds.property || 'value'
          return eventMetric === metricId || eventMetricMatches(eventMetric, metricId)
        })

        if (matchingSources.length > 0) {
          telemetryAlreadyProcessed = true
          const { preserveMultiple: pm, transform: tf } = state.optionsRef.current
          processTelemetryEvent(eventData, eventMetric, eventDeviceId, currentDataSources, pm, tf, (updater) => state.setData((prev) => updater(prev) as T), state.setLastUpdate)

          // Mark cache entries as stale — include aggregate in prefix to avoid
          // deleting entries for other components with different aggregation
          matchingSources.forEach((ds) => {
            const keyPrefix = `${getSourceId(ds)}|${ds.metricId}|${ds.aggregateExt ?? 'raw'}|`
            telemetryCache.deleteWhere((_, key) => key.startsWith(keyPrefix))
          })
        }
      }

      // Non-telemetry event processing
      if (shouldUpdate && !telemetryAlreadyProcessed) {
        const { preserveMultiple: pm, transform: tf, fallback: fb } = state.optionsRef.current
        processNonTelemetryEvent(eventData, eventType, isDeviceMetricEvent, eventMetric, hasDeviceId, currentDataSources, pm, tf, fb, (updater) => state.setData((prev) => updater(prev) as T), state.setLastUpdate)
      }
    }

    if (lastProcessedIdInBatch) lastProcessedDeviceEventIdRef.current = lastProcessedIdInBatch
    perfEnd('events')
  }, [enabled, dataSourceKey, eventsKey])

  // Expose readDataFromStore for useTelemetrySource
  return { readDataFromStore }
}
