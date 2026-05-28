/**
 * useDataSource — unified data binding for dashboard components.
 *
 * Thin router that delegates to focused sub-hooks:
 * - useStoreSource: device/metric/command/device-info + device WS
 * - useTelemetrySource: telemetry fetch + periodic refresh
 * - useSystemSource: system stats fetch
 * - useExtensionSource: extension fetch + extension WS
 */

import { useState, useCallback, useRef, useMemo } from 'react'
import type { DataSourceOrList, DataSource } from '@/types/dashboard'
import { normalizeDataSource, getSourceId, isRealtimeSource, isCommandSource, isTelemetrySource, isDeviceInfoSource, isTransformSource, isAIMetricSource, isSystemSource, isExtensionSource } from '@/types/dashboard'
import { toNumberArray } from '@/design-system/utils/format'
import { createStableKey } from '@/lib/stable-key'
import { isImageDataSource } from './helpers'
import {
  fetchHistoricalTelemetry,
  clearGlobalCacheIntervals,
} from './fetch'
import { useSystemSource } from './useSystemSource'
import { useExtensionSource } from './useExtensionSource'
import { useTelemetrySource } from './useTelemetrySource'
import { useStoreSource } from './useStoreSource'

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
  sources: DataSource[]
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
    sourceTransform?: (ds: DataSource) => DataSource | undefined
  }
): UseDataSourceResult<T> {
  const { enabled = true, transform, fallback, preserveMultiple = false, sourceTransform } = options ?? {}

  const hasDataSourceValue = dataSource !== undefined &&
                             dataSource !== null &&
                             (Array.isArray(dataSource) ? dataSource.length > 0 : true)

  // ============================================================================
  // A. State — flat useState instead of useReducer
  // ============================================================================

  const [data, setDataInternal] = useState<T | null>(fallback ?? null)
  const [error, setErrorInternal] = useState<string | null>(null)
  const [sending, setSending] = useState(false)
  const [lastUpdate, setLastUpdateInternal] = useState<number | null>(null)

  // Loading tracked via ref counter: loading = counter > 0
  const loadingRef = useRef(hasDataSourceValue && enabled ? 1 : 0)
  const [loading, setLoading] = useState(loadingRef.current > 0)

  // Track current data key for reset detection
  const currentKey = useMemo(() => createStableKey(dataSource), [dataSource])
  const prevKeyRef = useRef(currentKey)
  const initialRef = useRef(true)

  // Stable callbacks
  const setData = useCallback((value: T | ((prev: T | null) => T | null)) => {
    setDataInternal(prev => typeof value === 'function' ? (value as (p: T | null) => T | null)(prev) : value)
  }, [])

  const setDataRaw = useCallback((d: unknown) => {
    setDataInternal(d as T)
  }, [])

  const setError = useCallback((e: string | null) => {
    setErrorInternal(e)
  }, [])

  const setLastUpdate = useCallback((ts: number | null) => {
    if (ts !== null) setLastUpdateInternal(ts)
  }, [])

  const startLoading = useCallback(() => {
    loadingRef.current++
    if (loadingRef.current === 1) setLoading(true)
  }, [])

  const finishLoading = useCallback(() => {
    loadingRef.current = Math.max(0, loadingRef.current - 1)
    if (loadingRef.current === 0) setLoading(false)
    setLastUpdateInternal(Date.now())
  }, [])

  const setErrorLoading = useCallback((err: string) => {
    loadingRef.current = Math.max(0, loadingRef.current - 1)
    if (loadingRef.current === 0) setLoading(false)
    setErrorInternal(err)
  }, [])

  // ============================================================================
  // A2. Reset when dataSource changes
  // ============================================================================

  if (prevKeyRef.current !== currentKey) {
    prevKeyRef.current = currentKey
    loadingRef.current = 1
    setLoading(true)
    setErrorInternal(null)
  }
  // Skip initial loading clear on first render when no data source
  if (initialRef.current) {
    initialRef.current = false
    if (!hasDataSourceValue || !enabled) {
      loadingRef.current = 0
    }
  }

  // ============================================================================
  // B. Data source normalization + computed values
  // ============================================================================

  const dataSources = useMemo(() => {
    const normalized = dataSource ? normalizeDataSource(dataSource) : []
    if (sourceTransform) return normalized.map(sourceTransform).filter(Boolean) as DataSource[]
    return normalized
  }, [dataSource, sourceTransform])

  const relevantDeviceIds = useMemo(() => {
    return new Set(
      dataSources
        .map((ds) =>
          isRealtimeSource(ds) || isDeviceInfoSource(ds) || isCommandSource(ds)
            ? getSourceId(ds) : null
        )
        .filter(Boolean) as string[]
    )
  }, [dataSources])

  const deviceInfoIds = useMemo(() => {
    return new Set(
      dataSources.filter(isDeviceInfoSource).map((ds) => getSourceId(ds)).filter(Boolean) as string[]
    )
  }, [dataSources])

  const optionsRef = useRef({ enabled, transform, fallback, preserveMultiple })
  optionsRef.current = { enabled, transform, fallback, preserveMultiple }
  const dataSourcesRef = useRef(dataSources)
  dataSourcesRef.current = dataSources

  const hasCommandSource = dataSources.some(isCommandSource)
  const commandSource = dataSources.find(isCommandSource)

  const telemetrySources = useMemo(() =>
    dataSources.filter((ds) => isTelemetrySource(ds) || isTransformSource(ds) || isAIMetricSource(ds)),
    [dataSources]
  )

  const systemSources = useMemo(() => dataSources.filter(isSystemSource), [dataSources])
  const extensionSources = useMemo(() => dataSources.filter(isExtensionSource), [dataSources])

  const hasTelemetrySource = telemetrySources.length > 0
  const hasExtensionSource = extensionSources.length > 0
  const needsWebSocket = dataSources.some(isRealtimeSource)

  const telemetryKey = useMemo(() => {
    return telemetrySources
      .map((ds) => {
        const isImg = isImageDataSource(ds.params, ds.transform, ds.metricId)
        const actualTimeRange = ds.timeRange ?? (isImg ? 48 : 1)
        const actualLimit = ds.limit ?? (isImg ? 200 : 50)
        const actualAggregate = ds.aggregateExt ?? 'raw'
        const tw = ds.timeWindow
          ? `${ds.timeWindow.type}:${ds.timeWindow.startTime ?? ''}:${ds.timeWindow.endTime ?? ''}`
          : ''
        return `${getSourceId(ds)}|${ds.metricId}|${actualTimeRange}|${actualLimit}|${actualAggregate}|${tw}`
      })
      .join('|')
  }, [telemetrySources])

  const systemKey = useMemo(() => {
    return systemSources.map((ds) => createStableKey({ systemMetric: ds.systemMetric })).join('|')
  }, [systemSources])

  const extensionKey = useMemo(() => {
    return extensionSources
      .map((ds) => {
        const tw = ds.timeWindow ? `${ds.timeWindow.type}:${ds.timeWindow.startTime ?? ''}:${ds.timeWindow.endTime ?? ''}` : ''
        return createStableKey({ extensionId: ds.extensionId, extensionMetric: ds.extensionMetric, timeRange: ds.timeRange, limit: ds.limit, timeWindow: tw })
      })
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
      setErrorInternal(err instanceof Error ? err.message : 'Command failed')
      return false
    } finally {
      setSending(false)
    }
  }, [commandSource, enabled])

  // ============================================================================
  // D-J. Sub-hook adapters (bridge to legacy sub-hook interface)
  // ============================================================================

  // Sub-hooks expect { startLoading, finishLoading, retryLoading, failLoading }
  const makeAdapters = useCallback((_source: string) => ({
    startLoading,
    finishLoading,
    retryLoading: startLoading, // retry = keep loading active
    failLoading: setErrorLoading,
  }), [startLoading, finishLoading, setErrorLoading])

  const storeAdapters = useMemo(() => makeAdapters('store'), [makeAdapters])
  const telemetryAdapters = useMemo(() => makeAdapters('telemetry'), [makeAdapters])
  const systemAdapters = useMemo(() => makeAdapters('system'), [makeAdapters])
  const extensionAdapters = useMemo(() => makeAdapters('extension'), [makeAdapters])

  // Legacy setLoading adapter for sub-hooks that use it
  const legacySetLoading = useCallback((_l: boolean) => {
    // No-op: loading is managed via ref counter
  }, [])

  // ============================================================================
  // D. Store source
  // ============================================================================

  const { readDataFromStore, wsConnected } = useStoreSource<T>(
    dataSources, currentKey, enabled,
    relevantDeviceIds, deviceInfoIds, hasTelemetrySource, needsWebSocket,
    {
      data, setData, setDataRaw, setLoading: legacySetLoading, setError, setLastUpdate,
      dataSourcesRef, optionsRef,
      sourceAdapters: storeAdapters,
    },
    hasExtensionSource
  )

  // ============================================================================
  // F+G. Telemetry source
  // ============================================================================

  useTelemetrySource(telemetrySources, telemetryKey, enabled, hasTelemetrySource, relevantDeviceIds, wsConnected, {
    setData: (updater) => setData((prev) => typeof updater === 'function' ? (updater as (p: unknown) => unknown)(prev) as T : updater as T),
    setDataRaw, setLoading: legacySetLoading, setError, setLastUpdate, optionsRef,
    readDataFromStore,
    sourceAdapters: telemetryAdapters,
  })

  // ============================================================================
  // H. System source
  // ============================================================================

  useSystemSource(systemSources, systemKey, enabled, {
    setDataRaw, setLoading: legacySetLoading, setError, setLastUpdate, optionsRef,
    sourceAdapters: systemAdapters,
  })

  // ============================================================================
  // I+K. Extension source
  // ============================================================================

  useExtensionSource(extensionSources, extensionKey, enabled, currentKey, relevantExtensionIds, {
    setData: (updater) => setData((prev) => typeof updater === 'function' ? (updater as (p: unknown) => unknown)(prev) as T : updater as T),
    setDataRaw, setLoading: legacySetLoading, setError, setLastUpdate, dataSourcesRef, optionsRef,
    sourceAdapters: extensionAdapters,
  })

  return {
    data,
    loading,
    error,
    lastUpdate,
    sources: dataSources,
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
