/**
 * useDataSource — unified data binding for dashboard components.
 *
 * Single hook handling:
 * - Store subscription (device/metric/command/device-info real-time updates)
 * - Telemetry fetching (historical API + periodic refresh + retry)
 * - System fetching (system stats + periodic refresh)
 * - Extension fetching (V2 command:field format + periodic refresh)
 * - Device WebSocket event processing (DeviceMetric → store update + telemetry merge)
 * - Extension WebSocket event processing (ExtensionOutput → cache invalidation + merge)
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
import { useEvents } from '@/hooks/useEvents'
import {
  extractValueFromData, safeExtractValue, eventMetricMatches,
  getPointTimestamp, getPointValue, isImageDataSource, getDataSourceLimit,
  isDuplicatePoint, dedupeTelemetryPoints, sortAndDedup,
} from './helpers'
import {
  fetchHistoricalTelemetry, fetchSystemStats, fetchDeviceTelemetry,
  fetchedDevices, hasActiveFetch, telemetryCache, extensionDataCache,
  clearGlobalCacheIntervals,
} from './fetch'

// Re-export for backward compatibility
export { fetchHistoricalTelemetry, clearGlobalCacheIntervals }

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

  // ============================================================================
  // A. State + Refs
  // ============================================================================

  const [data, setData] = useState<T | null>(fallback ?? null)
  const [loading, setLoading] = useState(!enabled || !hasDataSourceValue ? false : true)
  const [error, setError] = useState<string | null>(null)
  const [lastUpdate, setLastUpdate] = useState<number | null>(null)
  const [sending, setSending] = useState(false)
  const [telemetryRefreshTrigger, setTelemetryRefreshTrigger] = useState(0)

  const telemetryRefreshTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const dataRef = useRef<T | null>(null)
  dataRef.current = data

  const dataSourceKey = useMemo(() => createStableKey(dataSource), [dataSource])
  const dataSources = useMemo(() => dataSource ? normalizeDataSource(dataSource) : [], [dataSourceKey])

  const relevantDeviceIds = useMemo(() => {
    return new Set(
      dataSources
        .map((ds) =>
          ds.type === 'device' || ds.type === 'command' || ds.type === 'telemetry' || ds.type === 'device-info'
            ? getSourceId(ds) : null
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

  const initialTelemetryFetchDoneRef = useRef(false)
  const emptyRetryCountRef = useRef(0)
  const deferredByDevicesLoadingRef = useRef(false)
  const prevStoreStateRef = useRef<{ devices: NeoMindStore['devices'] } | null>(null)

  // Event processing refs
  const processedDeviceEventsRef = useRef<Set<string>>(new Set())
  const lastProcessedDeviceEventIdRef = useRef<string | null>(null)
  const processedExtEventsRef = useRef<Set<string>>(new Set())
  const lastProcessedExtEventIdRef = useRef<string | null>(null)

  // Fetch interval refs
  const telemetryIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const systemIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const extensionIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  // ============================================================================
  // B. Computed values
  // ============================================================================

  const hasCommandSource = dataSources.some((ds) => ds.type === 'command')
  const commandSource = dataSources.find((ds) => ds.type === 'command')

  const telemetrySources = useMemo(() =>
    dataSources.filter((ds) => ds.type === 'telemetry' || ds.type === 'transform' || ds.type === 'ai-metric'),
    [dataSources]
  )
  const systemSources = useMemo(() => dataSources.filter((ds) => ds.type === 'system'), [dataSources])
  const extensionSources = useMemo(() => dataSources.filter((ds) => ds.type === 'extension'), [dataSources])

  const hasTelemetrySource = telemetrySources.length > 0
  const hasSystemSource = systemSources.length > 0
  const hasExtensionSource = extensionSources.length > 0
  const needsWebSocket = dataSources.some((ds) =>
    ds.type === 'device' || ds.type === 'metric' || ds.type === 'command' || ds.type === 'telemetry'
  )
  const needsExtWebSocket = extensionSources.length > 0

  // Telemetry stable key for fetch effect
  const telemetryKey = useMemo(() => {
    return telemetrySources
      .map((ds) => {
        const isImg = isImageDataSource(ds.params, ds.transform, ds.metricId)
        const actualTimeRange = ds.timeRange ?? (isImg ? 48 : 1)
        const actualLimit = ds.limit ?? (isImg ? 200 : 50)
        const actualAggregate = ds.aggregate ?? ds.aggregateExt ?? 'raw'
        return createStableKey({ deviceId: getSourceId(ds), metricId: ds.metricId, timeRange: actualTimeRange, limit: actualLimit, aggregate: actualAggregate })
      })
      .join('|')
  }, [telemetrySources])

  const systemKey = useMemo(() => {
    return systemSources.map((ds) => createStableKey({ systemMetric: ds.systemMetric })).join('|')
  }, [systemSources])

  const extensionKey = useMemo(() => {
    return extensionSources
      .map((ds) => createStableKey({ extensionId: ds.extensionId, extensionMetric: ds.extensionMetric }))
      .join('|')
  }, [extensionSources])

  const relevantExtensionIds = useMemo(() => {
    return new Set(extensionSources.map((ds) => ds.extensionId).filter(Boolean) as string[])
  }, [extensionSources])

  // ============================================================================
  // C. sendCommand
  // ============================================================================

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
  // D. readDataFromStore (device/metric/command/device-info)
  // ============================================================================

  const readDataFromStore = useCallback(() => {
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

  // Adapt setData for raw unknown type
  const setDataRaw = useCallback((d: unknown) => setData(d as T), [])

  // ============================================================================
  // E. Store subscription (device/metric/command/device-info real-time)
  // ============================================================================

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

      const devicesChanged = state.devices !== prev.devices
      const devicesLengthChanged = state.devices.length !== prev.devices.length
      let currentValuesChanged = false
      const changedDeviceIds = new Set<string>()

      if (!devicesLengthChanged) {
        for (const deviceId of relevantDeviceIds) {
          const device = state.devices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)
          const prevDevice = prev.devices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)

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

        prevStoreStateRef.current = { devices: state.devices }
        readDataFromStore()

        // Telemetry merge from store changes
        const currentDataSources = dataSourcesRef.current
        const telSources = currentDataSources.filter((ds) => ds.type === 'telemetry')
        if (telSources.length > 0 && currentValuesChanged && changedDeviceIds.size > 0) {
          const now = Math.floor(Date.now() / 1000)
          const getTs = (p: unknown): number => { if (p == null) return 0; const o = p as Record<string, unknown>; return (o.timestamp ?? o.time ?? o.t ?? 0) as number }

          setData((prevData) => {
            const currentData = prevData as unknown
            const results = currentDataSources.map((ds, index) => {
              if (ds.type !== 'telemetry') return undefined
              const deviceId = getSourceId(ds)!
              const metricId = ds.metricId || ds.property || 'value'
              if (!changedDeviceIds.has(deviceId)) return undefined

              const device = state.devices.find((d: Device) => d.id === deviceId || d.device_id === deviceId)
              const latestValue = device?.current_values ? extractValueFromData(device.current_values, metricId) : undefined
              if (latestValue === undefined) return undefined

              const newPoint = { timestamp: now, time: now, value: latestValue }
              const isPreserveMultiple = optionsRef.current.preserveMultiple
              let currentArray: unknown[] = []
              if (Array.isArray(currentData)) {
                if (isPreserveMultiple && currentDataSources.length > 1) {
                  if (Array.isArray(currentData[index])) currentArray = currentData[index] as unknown[]
                  else if (currentData[index] !== undefined) currentArray = [currentData[index]]
                } else if (currentDataSources.length === 1 || !isPreserveMultiple) {
                  currentArray = currentData as unknown[]
                }
              }

              const isImageDS = isImageDataSource(ds.params, ds.transform, metricId)
              const maxLimit = getDataSourceLimit(ds)

              // Image dedup
              if (isImageDS && currentArray.length > 0) {
                const newContent = typeof latestValue === 'string' ? latestValue : JSON.stringify(latestValue)
                if (newContent) {
                  const extractContent = (str: string): string => {
                    if (str.startsWith('data:')) { const ci = str.indexOf(','); if (ci !== -1) return str.slice(ci + 1) }
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
                      return newBase.slice(0, 500) === existingBase.slice(0, 500) && newBase.slice(-500) === existingBase.slice(-500)
                    }
                    return newBase === existingBase
                  })
                  if (alreadyExists) return undefined
                }
              }

              const merged = [newPoint, ...currentArray]
              const sorted = sortArrayByTs(merged, getTs)
              return isImageDS ? sorted.slice(0, maxLimit) : dedupeTelemetryPoints(sorted, getTs, maxLimit)
            })

            if (!results.some((r) => r !== undefined)) return prevData

            const finalData = currentDataSources.length > 1
              ? results.map((r, i) => (r !== undefined ? r : (Array.isArray(currentData) && currentData[i] !== undefined ? currentData[i] : [])))
              : results[0] ?? currentData
            const { transform: transformFn } = optionsRef.current
            setLastUpdate(Date.now())
            return (transformFn ? transformFn(finalData) : finalData) as T
          })
        }
      }
    })

    return () => { unsubscribed = true; unsubscribe() }
  }, [dataSources.length, enabled])

  // ============================================================================
  // F. Devices loading watcher
  // ============================================================================

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
  // G. Telemetry fetch effect
  // ============================================================================

  const prevTelemetryKeyRef = useRef('')

  useEffect(() => {
    if (!hasTelemetrySource || !enabled) {
      if (telemetryIntervalRef.current) { clearInterval(telemetryIntervalRef.current); telemetryIntervalRef.current = null }
      return
    }

    const configChanged = prevTelemetryKeyRef.current !== telemetryKey
    if (configChanged && telemetryKey) initialTelemetryFetchDoneRef.current = false
    prevTelemetryKeyRef.current = telemetryKey

    const fetchTelemetryData = async () => {
      const isInitialFetch = !initialTelemetryFetchDoneRef.current
      if (isInitialFetch) setLoading(true)
      setError(null)

      const timeoutPromise = new Promise((_, reject) => setTimeout(() => reject(new Error('Fetch timeout')), 10000))

      try {
        const results = await Promise.race([
          Promise.all(
            telemetrySources.map(async (ds) => {
              if (!getSourceId(ds) || !ds.metricId) return { data: [], raw: undefined }
              const includeRawPoints = ds.params?.includeRawPoints === true || ds.transform === 'raw'
              const bypassCache = !initialTelemetryFetchDoneRef.current || includeRawPoints
              const isImg = isImageDataSource(ds.params, ds.transform, ds.metricId)
              const actualTimeRange = ds.timeRange ?? (isImg ? 48 : 1)
              const actualLimit = ds.limit ?? (isImg ? 200 : 50)
              const actualAggregate = ds.aggregate ?? ds.aggregateExt ?? 'raw'

              const response = await fetchHistoricalTelemetry(
                getSourceId(ds)!, ds.metricId, actualTimeRange, actualLimit, actualAggregate, includeRawPoints, bypassCache
              )
              if (includeRawPoints && response.raw) return { data: response.data, raw: response.raw, success: response.success }
              return { data: response.success ? response.data : [], success: response.success }
            })
          ),
          timeoutPromise
        ]) as Array<{ data: unknown[]; raw?: unknown[]; success: boolean }>

        let finalData: unknown
        if (results.length > 1) {
          if (preserveMultiple) {
            const hasRawData = results.some((r) => r.raw !== undefined)
            finalData = hasRawData ? results.map((r) => r.raw ?? []) : results.map((r) => r.data ?? [])
          } else {
            const hasRawData = results.some((r) => r.raw !== undefined)
            finalData = hasRawData ? results.flatMap((r) => r.raw ?? []) : results.map((r) => r.data ?? []).flat()
          }
        } else {
          const r = results[0]
          finalData = (r?.raw ?? r?.data) ?? []
        }

        // Sort and dedup
        finalData = sortTelemetryResults(finalData, telemetrySources, preserveMultiple)

        setDataRaw(finalData)
        setLastUpdate(Date.now())
        initialTelemetryFetchDoneRef.current = true

        // Empty result retry
        const isEmpty = Array.isArray(finalData) ? finalData.length === 0 : finalData == null
        if (isEmpty) {
          const { devicesLoading } = useStore.getState()
          if (devicesLoading) {
            deferredByDevicesLoadingRef.current = true
            initialTelemetryFetchDoneRef.current = false
            setLoading(true)
          } else {
            emptyRetryCountRef.current += 1
            if (emptyRetryCountRef.current <= 3) setTimeout(() => fetchTelemetryData(), 3000)
          }
        } else {
          emptyRetryCountRef.current = 0
        }
      } catch (err) {
        logError(err, { operation: 'Fetch telemetry data' })
        setError(err instanceof Error ? err.message : 'Failed to fetch telemetry')
        setDataRaw([])
        initialTelemetryFetchDoneRef.current = true
      } finally {
        if (!deferredByDevicesLoadingRef.current) setLoading(false)
      }
    }

    if (telemetryIntervalRef.current) { clearInterval(telemetryIntervalRef.current); telemetryIntervalRef.current = null }
    fetchTelemetryData()

    const refreshIntervals = telemetrySources.map((ds) => ds.refresh).filter(Boolean) as number[]
    const minRefresh = refreshIntervals.length > 0 ? Math.min(...refreshIntervals) : null
    if (minRefresh) telemetryIntervalRef.current = setInterval(fetchTelemetryData, minRefresh * 1000)

    return () => { if (telemetryIntervalRef.current) { clearInterval(telemetryIntervalRef.current); telemetryIntervalRef.current = null } }
  }, [telemetryKey, enabled, telemetryRefreshTrigger])

  // ============================================================================
  // H. System fetch effect
  // ============================================================================

  const systemInitialDoneRef = useRef(false)
  const prevSystemKeyRef = useRef('')

  useEffect(() => {
    if (!hasSystemSource || !enabled) {
      if (systemIntervalRef.current) { clearInterval(systemIntervalRef.current); systemIntervalRef.current = null }
      return
    }

    // Reset when system config changes
    if (prevSystemKeyRef.current !== systemKey) {
      systemInitialDoneRef.current = false
      prevSystemKeyRef.current = systemKey
    }

    const fetchSystemData = async () => {
      if (!systemInitialDoneRef.current) setLoading(true)
      setError(null)

      try {
        const results = await Promise.all(
          systemSources.map(async (ds) => {
            const metric = ds.systemMetric
            if (!metric) return { data: null }
            const response = await fetchSystemStats(metric)
            return { data: response.data, success: response.success }
          })
        )

        let finalData: unknown
        if (results.length > 1) finalData = results.map((r) => r.data)
        else finalData = results[0]?.data ?? null

        const { transform: transformFn, fallback: fallbackVal } = optionsRef.current
        const transformedData = transformFn ? transformFn(finalData) : finalData
        setDataRaw(transformedData)
        setLastUpdate(Date.now())
        systemInitialDoneRef.current = true
      } catch (err) {
        logError(err, { operation: 'Fetch system data' })
        setError(err instanceof Error ? err.message : 'Failed to fetch system data')
        setDataRaw(optionsRef.current.fallback ?? null)
        systemInitialDoneRef.current = true
      } finally {
        setLoading(false)
      }
    }

    if (systemIntervalRef.current) { clearInterval(systemIntervalRef.current); systemIntervalRef.current = null }
    fetchSystemData()

    const refreshIntervals = systemSources.map((ds) => ds.refresh).filter(Boolean) as number[]
    const minRefresh = refreshIntervals.length > 0 ? Math.min(...refreshIntervals) : null
    if (minRefresh) systemIntervalRef.current = setInterval(fetchSystemData, minRefresh * 1000)

    return () => { if (systemIntervalRef.current) { clearInterval(systemIntervalRef.current); systemIntervalRef.current = null } }
  }, [systemKey, enabled])

  // ============================================================================
  // I. Extension fetch effect
  // ============================================================================

  const extInitialDoneRef = useRef(false)
  const prevExtKeyRef = useRef('')

  useEffect(() => {
    if (!hasExtensionSource || !enabled) {
      if (extensionIntervalRef.current) { clearInterval(extensionIntervalRef.current); extensionIntervalRef.current = null }
      return
    }

    // Reset when extension config changes
    if (prevExtKeyRef.current !== extensionKey) {
      extInitialDoneRef.current = false
      prevExtKeyRef.current = extensionKey
    }

    const fetchExtensionData = async () => {
      if (!extInitialDoneRef.current) setLoading(true)
      setError(null)

      try {
        const { transform: transformFn } = optionsRef.current
        const api = (await import('@/lib/api')).api
        const results = await Promise.all(
          extensionSources.map(async (ds) => {
            const extensionId = ds.extensionId
            const metric = ds.extensionMetric
            if (!extensionId || !metric) return { data: null }

            // Check shared cache
            const extCacheKey = `${extensionId}|${metric}`
            const extCached = extensionDataCache.get(extCacheKey)
            if (extCached !== undefined) return { data: extCached, success: true }

            // V2 data source (format: command:field)
            const isV2 = metric.includes(':')
            const parts = metric.split(':')

            try {
              if (isV2 && parts.length >= 2) {
                const command = parts[0]
                const field = parts[1]

                if (command !== 'produce') {
                  try {
                    const result = await api.executeExtensionCommand(extensionId, command, {})
                    const resultData = (result as Record<string, unknown>).result ?? result
                    if (field === 'result') return { data: resultData, success: true }
                    if (typeof resultData === 'object' && resultData !== null) {
                      const fieldValue = (resultData as Record<string, unknown>)[field]
                      return { data: fieldValue ?? resultData, success: true }
                    }
                    return { data: resultData, success: true }
                  } catch {
                    const result = await api.queryData({
                      extension_id: extensionId, command, field,
                      start_time: Date.now() - (24 * 60 * 60 * 1000), end_time: Date.now(), limit: 100,
                    })
                    if (result?.data_points?.length > 0) return { data: result.data_points, success: true }
                    return { data: null, success: false }
                  }
                }

                // produce:* format
                const endTime = Date.now()
                const result = await api.queryData({
                  extension_id: extensionId, command, field,
                  start_time: endTime - (24 * 60 * 60 * 1000), end_time: endTime, limit: 100,
                })
                if (result?.data_points?.length > 0) return { data: result.data_points, success: true }
                return { data: null, success: false }
              } else {
                return { data: null, success: false }
              }
            } catch {
              return { data: null, success: false }
            }
          })
        )

        // Cache successful results
        extensionSources.forEach((ds, i) => {
          if (ds.extensionId && ds.extensionMetric && results[i]?.success) {
            extensionDataCache.set(`${ds.extensionId}|${ds.extensionMetric}`, results[i].data)
          }
        })

        let finalData: unknown
        if (results.length > 1) finalData = results.map((r) => r.data)
        else finalData = results[0]?.data ?? null

        // Wrap scalar values into time-series array format
        if (finalData !== null && finalData !== undefined && !Array.isArray(finalData)) {
          const now = Math.floor(Date.now() / 1000)
          finalData = [{ timestamp: now, time: now, value: finalData }]
        }

        const transformedData = transformFn ? transformFn(finalData) : finalData
        setDataRaw(transformedData)
        setLastUpdate(Date.now())
        extInitialDoneRef.current = true
      } catch (err) {
        logError(err, { operation: 'Fetch extension data' })
        setError(err instanceof Error ? err.message : 'Failed to fetch extension data')
        extInitialDoneRef.current = true
      } finally {
        setLoading(false)
      }
    }

    if (extensionIntervalRef.current) { clearInterval(extensionIntervalRef.current); extensionIntervalRef.current = null }
    fetchExtensionData()

    const refreshIntervals = extensionSources.map((ds) => ds.refresh).filter(Boolean) as number[]
    const minRefresh = refreshIntervals.length > 0 ? Math.min(...refreshIntervals) : null
    if (minRefresh) extensionIntervalRef.current = setInterval(fetchExtensionData, minRefresh * 1000)

    return () => { if (extensionIntervalRef.current) { clearInterval(extensionIntervalRef.current); extensionIntervalRef.current = null } }
  }, [extensionKey, enabled])

  // ============================================================================
  // J. Device WebSocket event processing
  // ============================================================================

  const { events } = useEvents({
    enabled: enabled && needsWebSocket,
    category: 'device',
    onConnected: () => {
      processedDeviceEventsRef.current.clear()
      lastProcessedDeviceEventIdRef.current = null
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

      // Skip events for irrelevant devices
      const hasDeviceId = eventData && typeof eventData === 'object' && 'device_id' in eventData
      if (hasDeviceId && relevantDeviceIds.size > 0) {
        if (!relevantDeviceIds.has(eventData.device_id as string)) continue
      }

      const uniqueEventId = latestEvent.id || `${eventType}_${Date.now()}_${Math.random()}`
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
      const currentDataSources = dataSourcesRef.current
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
          const { preserveMultiple: pm, transform: tf } = optionsRef.current
          processTelemetryEvent(eventData, eventMetric, eventDeviceId, currentDataSources, dataRef.current, pm, tf, setDataRaw, setLastUpdate)

          // Schedule cache refresh
          const refreshDelay = 10000
          matchingSources.forEach((ds) => {
            const cacheKey = `${getSourceId(ds)}|${ds.metricId}|${ds.timeRange ?? 1}|${ds.limit ?? 50}|${ds.aggregate ?? ds.aggregateExt ?? 'raw'}`
            const cached = telemetryCache.getWithMeta(cacheKey)
            if (cached && !cached.meta?.refreshing) {
              telemetryCache.updateMeta(cacheKey, { refreshing: true, refreshAfter: Date.now() + refreshDelay })
            }
          })

          if (telemetryRefreshTimerRef.current) clearTimeout(telemetryRefreshTimerRef.current)
          telemetryRefreshTimerRef.current = setTimeout(() => {
            telemetryCache.deleteWhere((meta) => !!meta?.refreshing && !!meta?.refreshAfter && Date.now() >= (meta.refreshAfter as number))
            telemetryRefreshTimerRef.current = null
            setTelemetryRefreshTrigger(prev => prev + 1)
          }, refreshDelay)
        }
      }

      // Non-telemetry event processing
      if (shouldUpdate && !telemetryAlreadyProcessed) {
        const { preserveMultiple: pm, transform: tf, fallback: fb } = optionsRef.current
        processNonTelemetryEvent(eventData, eventType, isDeviceMetricEvent, eventMetric, hasDeviceId, currentDataSources, dataRef.current, pm, tf, fb, setDataRaw, setLastUpdate)
      }
    }

    if (lastProcessedIdInBatch) lastProcessedDeviceEventIdRef.current = lastProcessedIdInBatch
  }, [enabled, dataSourceKey, eventsKey])

  // ============================================================================
  // K. Extension WebSocket event processing
  // ============================================================================

  const { events: extensionEvents } = useEvents({
    enabled: enabled && needsExtWebSocket,
    category: 'extension',
    onConnected: () => {
      processedExtEventsRef.current.clear()
      lastProcessedExtEventIdRef.current = null
    },
  })

  const extensionEventsKey = useMemo(() => {
    if (extensionEvents.length === 0) return 'empty'
    const lastEvent = extensionEvents[extensionEvents.length - 1]
    return `ext-events-${extensionEvents.length}-${lastEvent?.id || 'unknown'}`
  }, [extensionEvents])

  useEffect(() => {
    if (!needsExtWebSocket || !enabled || extensionEvents.length === 0) return

    let extStartIndex = 0
    const lastProcessedExtId = lastProcessedExtEventIdRef.current
    if (lastProcessedExtId) {
      const lastIndex = extensionEvents.findIndex(e => e.id === lastProcessedExtId)
      if (lastIndex !== -1) extStartIndex = lastIndex + 1
      else { extStartIndex = 0; const entries = Array.from(processedExtEventsRef.current); processedExtEventsRef.current = new Set(entries.slice(-50)) }
    }
    if (extStartIndex > extensionEvents.length) { extStartIndex = 0; processedExtEventsRef.current.clear() }

    const newEvents = extensionEvents.slice(extStartIndex)
    if (newEvents.length === 0) return

    const extDataSources = dataSourcesRef.current.filter((ds) => ds.type === 'extension') as Array<{
      extensionId: string; extensionMetric: string
    }>
    if (extDataSources.length === 0) return

    let lastProcessedExtIdInBatch: string | null = null

    for (const latestEvent of newEvents) {
      const eventData = (latestEvent as any).data || latestEvent
      const eventType = (latestEvent as any).type

      if (eventType !== 'ExtensionOutput') continue

      const uniqueEventId = latestEvent.id || `${eventType}_${Date.now()}_${Math.random()}`
      if (processedExtEventsRef.current.has(uniqueEventId)) continue
      processedExtEventsRef.current.add(uniqueEventId)
      lastProcessedExtIdInBatch = uniqueEventId

      if (processedExtEventsRef.current.size > 100) {
        const entries = Array.from(processedExtEventsRef.current)
        processedExtEventsRef.current = new Set(entries.slice(-50))
      }

      const eventExtensionId = eventData.extension_id as string
      const eventOutputName = eventData.output_name as string

      if (!relevantExtensionIds.has(eventExtensionId)) continue

      const normalizedOutput = normalizeOutputName(eventOutputName)

      const matchingSources = extDataSources.filter((ds) => {
        if (ds.extensionId !== eventExtensionId) return false
        if (!ds.extensionMetric) return false
        const parts = ds.extensionMetric.split(':')
        const metricName = parts.length > 1 ? parts[1] : parts[0]
        return metricName === normalizedOutput || metricName === eventOutputName
      })

      if (matchingSources.length > 0) {
        const { transform: transformFn, preserveMultiple: pm } = optionsRef.current
        const eventValue = eventData.value
        const now = Math.floor(Date.now() / 1000)
        const newPoint = { timestamp: now, time: now, value: eventValue }
        const allExtSources = dataSourcesRef.current.filter((ds) => ds.type === 'extension')

        // Use setData(prev => ...) to avoid stale dataRef race condition
        setData((prevData) => {
          const currentData = prevData as unknown
          let newData: unknown

          if (pm && allExtSources.length > 1 && Array.isArray(currentData)) {
            const nested = (currentData as unknown[][]).map((arr, i) => {
              const ds = allExtSources[i]
              if (!ds) return arr
              const parts = (ds.extensionMetric ?? '').split(':')
              const metricName = parts.length > 1 ? parts[1] : parts[0]
              if (ds.extensionId === eventExtensionId && (metricName === normalizedOutput || metricName === eventOutputName)) {
                return [newPoint, ...(Array.isArray(arr) ? arr : [])]
              }
              return arr
            })
            newData = nested
          } else if (Array.isArray(currentData)) {
            newData = [newPoint, ...currentData]
          } else {
            newData = [newPoint]
          }

          return (transformFn ? transformFn(newData) : newData) as T
        })

        // Update cache with merged data so next fetch doesn't overwrite
        matchingSources.forEach((ds) => {
          if (ds.extensionId && ds.extensionMetric) {
            extensionDataCache.delete(`${ds.extensionId}|${ds.extensionMetric}`)
          }
        })

        setLastUpdate(Date.now())
      }
    }

    if (lastProcessedExtIdInBatch) lastProcessedExtEventIdRef.current = lastProcessedExtIdInBatch
  }, [enabled, dataSourceKey, extensionEventsKey, needsExtWebSocket, relevantExtensionIds.size])

  // ============================================================================
  // L. Cleanup + return
  // ============================================================================

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
// Specialized hooks
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

  return { data: data ?? [], loading, error, lastUpdate }
}

// ============================================================================
// Internal helpers (event processors)
// ============================================================================

function normalizeOutputName(outputName: string): string {
  if (!outputName.includes(':')) return outputName
  return outputName.split(':').slice(1).join(':')
}

function sortArrayByTs(points: unknown[], getTs: (p: unknown) => number): unknown[] {
  const idx = points.map((p, i) => ({ p, i }))
  idx.sort((a, b) => { const d = getTs(b.p) - getTs(a.p); return d !== 0 ? d : a.i - b.i })
  return idx.map(({ p }) => p)
}

function sortTelemetryResults(finalData: unknown, sources: DataSource[], preserveMultiple: boolean): unknown {
  const isPM = preserveMultiple && sources.length > 1
  const process = (points: unknown[], ds: DataSource): unknown[] => {
    if (!Array.isArray(points) || points.length === 0) return points
    const isImg = isImageDataSource(ds.params, ds.transform, ds.metricId)
    const maxLimit = getDataSourceLimit(ds)
    const getTs = (p: unknown): number => { if (p == null) return 0; const o = p as Record<string, unknown>; return (o.timestamp ?? o.time ?? o.t ?? 0) as number }
    return sortAndDedup(points, getTs, maxLimit, isImg)
  }
  if (isPM && Array.isArray(finalData)) return sources.map((ds, i) => process((finalData as unknown[][])[i], ds))
  if (Array.isArray(finalData) && sources.length > 0) return process(finalData, sources[0])
  return finalData
}

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

    if ('value' in eventData && metricMatches) { eventValue = eventData.value }
    else if (!eventMetric) { eventValue = extractValueFromData(eventData, metricId) }
    else return undefined
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

    if (isImg && isDuplicatePoint(currentArray, eventTimestamp, eventValue, getTs)) return undefined

    const sorted = sortArrayByTs([newPoint, ...currentArray], getTs)
    return isImg ? sorted.slice(0, maxLimit) : dedupeTelemetryPoints(sorted, getTs, maxLimit)
  })

  if (!updatedResults.some((r) => r !== undefined)) return

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
        if (Array.isArray(currentData) && currentData[index] !== undefined) {
          result = dataSources.length > 1 ? currentData[index] : currentData
        } else { result = fallback ?? [] }
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
