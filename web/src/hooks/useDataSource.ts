/**
 * useDataSource Hook
 *
 * Simplified data binding for dashboard components.
 * Orchestrates specialized sub-hooks for each data source type:
 * - useTelemetryFetching: historical time-series data
 * - useSystemFetching: system stats
 * - useExtensionFetching: extension metrics
 * - useDeviceEventProcessing: real-time device WebSocket events
 * - useExtensionEventProcessing: real-time extension WebSocket events
 */

import { useEffect, useState, useCallback, useRef, useMemo } from 'react'
import type { DataSourceOrList, DataSource, TelemetryAggregate } from '@/types/dashboard'
import { normalizeDataSource, getSourceId } from '@/types/dashboard'
import type { Device } from '@/types'
import type { NeoMindStore } from '@/store'
import { useStore } from '@/store'
import { toNumberArray } from '@/design-system/utils/format'
import { logError } from '@/lib/errors'
import { createStableKey } from '@/lib/stable-key'

// Performance probe — lightweight counters, only outputs when ENABLE_PERF_LOGS=true
const PERF = typeof localStorage !== 'undefined' && localStorage.getItem('ENABLE_PERF_LOGS') === 'true'
const perfCounters = {
  storeCallback: 0,
  readDataFromStore: 0,
  telemetryMerge: 0,
  setData: 0,
  lastReport: 0,
}
function perfReport() {
  if (!PERF) return
  const now = Date.now()
  if (now - perfCounters.lastReport < 5000) return // report every 5s
  perfCounters.lastReport = now
  console.log(`[useDataSource Perf] storeCB=${perfCounters.storeCallback} readStore=${perfCounters.readDataFromStore} telemMerge=${perfCounters.telemetryMerge} setData=${perfCounters.setData}`)
}

// Sub-hooks
import { useTelemetryFetching } from './useDataSource/useTelemetryFetching'
import { useSystemFetching } from './useDataSource/useSystemFetching'
import { useExtensionFetching } from './useDataSource/useExtensionFetching'
import { useDeviceEventProcessing } from './useDataSource/useDeviceEventProcessing'
import { useExtensionEventProcessing } from './useDataSource/useExtensionEventProcessing'

// Sub-modules
import { fetchDeviceTelemetry, fetchedDevices, hasActiveFetch } from './useDataSource/batchFetch'
import { extractValueFromData, safeExtractValue } from './useDataSource/extractors'
import { isDuplicatePoint, getPointValue, dedupeTelemetryPoints, isImageDataSource as checkIsImageDataSource, getDataSourceLimit } from './useDataSource/dedup'
import { clearGlobalCacheIntervals } from './useDataSource/cache'

// Re-export for backward compatibility
export { fetchHistoricalTelemetry } from './useDataSource/telemetryFetch'
export { clearGlobalCacheIntervals }

// ============================================================================
// Types
// ============================================================================

export interface UseDataSourceResult<T = unknown> {
  data: T | null
  loading: boolean
  error: string | null
  lastUpdate: number | null
  sendCommand?: (value?: unknown) => Promise<boolean>
  sending?: boolean
}

// ============================================================================
// Main Hook
// ============================================================================

export function useDataSource<T = unknown>(
  dataSource: DataSourceOrList | undefined,
  options?: {
    enabled?: boolean
    transform?: (data: unknown) => T
    fallback?: T
    preserveMultiple?: boolean
  }
): UseDataSourceResult<T> {
  const { enabled = true, transform, fallback, preserveMultiple = false } = options ?? {}

  const hasDataSourceValue = dataSource !== undefined &&
                             dataSource !== null &&
                             (Array.isArray(dataSource) ? dataSource.length > 0 : true)

  const [data, setData] = useState<T | null>(fallback ?? null)
  const [loading, setLoading] = useState(!enabled || !hasDataSourceValue ? false : true)
  const [error, setError] = useState<string | null>(null)
  const [lastUpdate, setLastUpdate] = useState<number | null>(null)
  const [sending, setSending] = useState(false)
  const [telemetryRefreshTrigger, setTelemetryRefreshTrigger] = useState(0)

  // Track intervals
  const telemetryRefreshTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Stable data ref — updated every render so sub-hooks can read current data
  const dataRef = useRef<T | null>(null)
  dataRef.current = data

  // Memoize dataSources
  const dataSourceKey = useMemo(() => createStableKey(dataSource), [dataSource])
  const dataSources = useMemo(() => dataSource ? normalizeDataSource(dataSource) : [], [dataSourceKey])

  // Memoize relevant device IDs
  const relevantDeviceIds = useMemo(() => {
    return new Set(
      dataSources
        .map((ds) =>
          ds.type === 'device' || ds.type === 'command' || ds.type === 'telemetry' || ds.type === 'device-info'
            ? getSourceId(ds)
            : null
        )
        .filter(Boolean) as string[]
    )
  }, [dataSources])

  const deviceInfoIds = useMemo(() => {
    return new Set(
      dataSources.filter((ds) => ds.type === 'device-info').map((ds) => getSourceId(ds)).filter(Boolean) as string[]
    )
  }, [dataSources])

  const initialFetchDoneRef = useRef<Set<string>>(new Set())
  const lastValidDataRef = useRef<Record<string, unknown>>({})
  const optionsRef = useRef({ enabled, transform, fallback, preserveMultiple })
  optionsRef.current = { enabled, transform, fallback, preserveMultiple }
  const dataSourcesRef = useRef(dataSources)
  dataSourcesRef.current = dataSources

  // Refs for deferred loading
  const initialTelemetryFetchDoneRef = useRef(false)
  const emptyRetryCountRef = useRef(0)
  const deferredByDevicesLoadingRef = useRef(false)

  // Command source
  const hasCommandSource = dataSources.some((ds) => ds.type === 'command')
  const commandSource = dataSources.find((ds) => ds.type === 'command')

  const sendCommand = useCallback(async (value?: unknown): Promise<boolean> => {
    if (!commandSource || !enabled) return false
    setSending(true)
    setError(null)

    try {
      const deviceId = getSourceId(commandSource)
      const command = commandSource.command || 'setValue'
      let params: Record<string, unknown> = { value }

      if (commandSource.valueMapping && value !== undefined) {
        const mapping = commandSource.valueMapping
        if (value === true || value === 'on' || value === 1) {
          params = mapping.on !== undefined ? { value: mapping.on } : { value }
        } else if (value === false || value === 'off' || value === 0) {
          params = mapping.off !== undefined ? { value: mapping.off } : { value }
        } else {
          params = mapping.true !== undefined ? { value: mapping.true } : { value }
        }
      }
      if (commandSource.commandParams) params = { ...params, ...commandSource.commandParams }

      const { api } = await import('@/lib/api')
      await api.sendCommand(deviceId!, command, params)
      return true
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Command failed')
      return false
    } finally {
      setSending(false)
    }
  }, [commandSource, enabled])

  // ============================================================================
  // Read data from store (for device/metric/command/device-info sources)
  // ============================================================================

  const readDataFromStore = useCallback(() => {
    if (PERF) perfCounters.readDataFromStore++
    const { transform: transformFn, fallback: fallbackVal } = optionsRef.current
    const currentDataSources = dataSourcesRef.current
    const currentDevices = useStore.getState().devices

    if (currentDataSources.length === 0) {
      if (fallbackVal !== undefined) setData(fallbackVal)
      setLoading(false)
      return
    }

    try {
      const nonTelemetrySources = currentDataSources.filter(
        (ds) => ds.type !== 'telemetry' && ds.type !== 'system' && ds.type !== 'extension' && ds.type !== 'transform' && ds.type !== 'ai-metric'
      )

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
      } else if (nonTelemetrySources.length === 1) {
        finalData = results[0]
      } else if (currentDataSources.length === 0) {
        return
      } else {
        return
      }

      const transformedData = transformFn ? transformFn(finalData) : (finalData as T)
      setData(transformedData)
      setLastUpdate(Date.now())
    } catch (err) {
      const { devicesLoading } = useStore.getState()
      if (!devicesLoading) setError(err instanceof Error ? err.message : 'Unknown error')
      setData((fallback ?? 0) as T)
    } finally {
      setLoading(false)
    }
  }, [])

  // ============================================================================
  // Store subscription (for device/metric/command/device-info real-time updates)
  // ============================================================================

  const prevStoreStateRef = useRef<{ devices: NeoMindStore['devices'] } | null>(null)

  useEffect(() => {
    if (dataSources.length === 0) {
      const { fallback: fallbackVal } = optionsRef.current
      if (fallbackVal !== undefined) setData(fallbackVal)
      setLoading(false)
      return
    }
    if (!enabled) { setLoading(false); return }
    if (relevantDeviceIds.size === 0) { readDataFromStore(); setLoading(false); return }

    readDataFromStore()
    prevStoreStateRef.current = { devices: useStore.getState().devices }

    let unsubscribed = false
    let lastDevicesRef = useStore.getState().devices
    const unsubscribe = useStore.subscribe((state: NeoMindStore) => {
      if (unsubscribed) return
      if (state.devices === lastDevicesRef) return
      lastDevicesRef = state.devices

      const prev = prevStoreStateRef.current
      if (!prev) return

      // Detect changes
      const devicesChanged = state.devices !== prev.devices
      const devicesLengthChanged = state.devices.length !== prev.devices.length
      let currentValuesChanged = false
      const changedDeviceIds = new Set<string>()

      if (!devicesLengthChanged) {
        const currentRelevant = relevantDeviceIds
        for (const deviceId of currentRelevant) {
          const device = state.devices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)
          const prevDevice = prev.devices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)

          if (device && prevDevice) {
            if (device.current_values !== prevDevice.current_values) {
              // Use a simple check — deep comparison is too expensive in subscribe
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
        // Skip if no relevant device actually changed
        const hasRelevantChange = devicesLengthChanged ||
          Array.from(changedDeviceIds).some((id) => relevantDeviceIds.has(id)) ||
          (devicesChanged && changedDeviceIds.size === 0) // array reorder without specific detection
        if (!hasRelevantChange && !currentValuesChanged) return

        if (PERF) { perfCounters.storeCallback++; perfReport() }
        prevStoreStateRef.current = { devices: state.devices }
        readDataFromStore()

        // Telemetry merge: merge latest device values into telemetry data arrays
        const currentDataSources = dataSourcesRef.current
        const telemetrySources = currentDataSources.filter((ds) => ds.type === 'telemetry')
        if (telemetrySources.length > 0 && currentValuesChanged && changedDeviceIds.size > 0) {
          const now = Math.floor(Date.now() / 1000)
          const getTs = (p: unknown): number => {
            if (p == null) return 0
            const o = p as Record<string, unknown>
            return (o.timestamp ?? o.time ?? o.t ?? 0) as number
          }

          // Use setData(prev => ...) to avoid stale dataRef race condition
          setData((prevData) => {
            const currentData = prevData as unknown
            const results = currentDataSources.map((ds, index) => {
              if (ds.type !== 'telemetry') return undefined
              const deviceId = getSourceId(ds)!
              const metricId = ds.metricId || ds.property || 'value'

              if (!changedDeviceIds.has(deviceId)) return undefined

              const device = state.devices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)
              const latestValue = device?.current_values
                ? extractValueFromData(device.current_values, metricId)
                : undefined
              if (latestValue === undefined) return undefined

              const newPoint = { timestamp: now, time: now, value: latestValue }

              const isPreserveMultiple = optionsRef.current.preserveMultiple
              let currentArray: unknown[] = []
              if (Array.isArray(currentData)) {
                if (isPreserveMultiple && currentDataSources.length > 1) {
                  if (Array.isArray(currentData[index])) {
                    currentArray = currentData[index] as unknown[]
                  } else if (currentData[index] !== undefined) {
                    currentArray = [currentData[index]]
                  }
                } else if (currentDataSources.length === 1 || !isPreserveMultiple) {
                  currentArray = currentData as unknown[]
                }
              }

              const isImageDS = checkIsImageDataSource(ds.params, ds.transform, metricId)
              const maxLimit = getDataSourceLimit(ds)

              // Image dedup: skip if same image content already exists
              if (isImageDS && currentArray.length > 0) {
                const newContent = typeof latestValue === 'string' ? latestValue : JSON.stringify(latestValue)
                if (newContent) {
                  const extractContent = (str: string): string => {
                    if (str.startsWith('data:')) {
                      const commaIndex = str.indexOf(',')
                      if (commaIndex !== -1) return str.slice(commaIndex + 1)
                    }
                    return str
                  }
                  const newBase = extractContent(newContent)
                  const alreadyExists = currentArray.some((p) => {
                    const val = getPointValue(p)
                    if (val === undefined || val === null) return false
                    const existingStr = typeof val === 'string' ? val : JSON.stringify(val)
                    if (!existingStr) return false
                    const existingBase = extractContent(existingStr)
                    if (newBase.length > 1000 && existingBase.length > 1000) {
                      return newBase.slice(0, 500) === existingBase.slice(0, 500) &&
                             newBase.slice(-500) === existingBase.slice(-500)
                    }
                    return newBase === existingBase
                  })
                  if (alreadyExists) return undefined
                }
              }

              const merged = [newPoint, ...currentArray]
              const idx1 = merged.map((p, i) => ({ p, i }))
              idx1.sort((a, b) => {
                const tsDiff = getTs(b.p) - getTs(a.p)
                return tsDiff !== 0 ? tsDiff : a.i - b.i
              })
              const sorted = idx1.map(({ p }) => p)

              if (isImageDS) return sorted.slice(0, maxLimit)
              return dedupeTelemetryPoints(sorted, getTs, maxLimit)
            })

            if (!results.some((r) => r !== undefined)) return prevData

            const finalData = currentDataSources.length > 1
              ? results.map((r, i) => (r !== undefined ? r : (Array.isArray(currentData) && currentData[i] !== undefined ? currentData[i] : [])))
              : results[0] ?? currentData
            const { transform: transformFn } = optionsRef.current
            if (PERF) perfCounters.telemetryMerge++
            setLastUpdate(Date.now())
            return (transformFn ? transformFn(finalData) : finalData) as T
          })
        }
      }
    })

    return () => { unsubscribed = true; unsubscribe() }
  }, [dataSources.length, enabled])

  // ============================================================================
  // Devices loading watcher
  // ============================================================================

  const hasTelemetrySource = dataSources.some((ds) => ds.type === 'telemetry' || ds.type === 'transform' || ds.type === 'ai-metric')

  useEffect(() => {
    if (relevantDeviceIds.size === 0 && !hasTelemetrySource) return

    let prevLoading = useStore.getState().devicesLoading
    const unsubscribe = useStore.subscribe((state: NeoMindStore) => {
      if (state.devicesLoading === prevLoading) return
      prevLoading = state.devicesLoading

      if (!state.devicesLoading && deferredByDevicesLoadingRef.current) {
        deferredByDevicesLoadingRef.current = false
        readDataFromStore()
        if (hasTelemetrySource) setTelemetryRefreshTrigger((n) => n + 1)
      }
    })

    return () => unsubscribe()
  }, [relevantDeviceIds, hasTelemetrySource, readDataFromStore])

  // ============================================================================
  // Sub-hooks for data fetching and event processing
  // ============================================================================

  // Wrap setData to adapt T | null type to unknown
  const setDataRaw = useCallback((d: unknown) => setData(d as T), [])

  // Telemetry fetching
  useTelemetryFetching({
    dataSources,
    enabled,
    telemetryRefreshTrigger,
    preserveMultiple,
    setData: setDataRaw,
    setLoading,
    setError,
    setLastUpdate,
    setTelemetryRefreshTrigger,
    initialFetchDoneRef: initialTelemetryFetchDoneRef,
    emptyRetryCountRef,
    deferredByDevicesLoadingRef,
  })

  // System fetching
  useSystemFetching({
    dataSources,
    enabled,
    transform,
    fallback,
    setData: setDataRaw,
    setLoading,
    setError,
    setLastUpdate,
  })

  // Extension fetching
  useExtensionFetching({
    dataSources,
    enabled,
    transform,
    fallback,
    setData: setDataRaw,
    setLoading,
    setError,
    setLastUpdate,
  })

  // Device event processing
  useDeviceEventProcessing({
    dataSources,
    dataSourceKey,
    enabled,
    preserveMultiple,
    transform,
    fallback,
    relevantDeviceIds,
    dataRef: dataRef as React.MutableRefObject<unknown>,
    setData: setDataRaw,
    setLastUpdate,
    telemetryRefreshTimerRef,
    setTelemetryRefreshTrigger,
  })

  // Extension event processing
  useExtensionEventProcessing({
    dataSources,
    dataSourceKey,
    enabled,
    preserveMultiple,
    transform,
    fallback,
    dataRef: dataRef as React.MutableRefObject<unknown>,
    setData: setDataRaw,
    setLastUpdate,
  })

  // Cleanup telemetry refresh timer
  useEffect(() => {
    return () => {
      if (telemetryRefreshTimerRef.current) {
        clearTimeout(telemetryRefreshTimerRef.current)
        telemetryRefreshTimerRef.current = null
      }
    }
  }, [])

  return {
    data,
    loading,
    error,
    lastUpdate,
    ...(hasCommandSource && { sendCommand, sending }),
  }
}

// ============================================================================
// Specialized Hooks
// ============================================================================

export function useNumberArrayDataSource(
  dataSource: DataSourceOrList | undefined,
  options?: {
    enabled?: boolean
    fallback?: number[]
  }
) {
  const { data, loading, error, lastUpdate } = useDataSource<number[]>(dataSource, {
    ...options,
    transform: (raw): number[] => toNumberArray(raw, options?.fallback ?? []),
    fallback: options?.fallback ?? [],
  })

  return {
    data: data ?? [],
    loading,
    error,
    lastUpdate,
  }
}
