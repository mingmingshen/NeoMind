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
import type { DataSourceOrList } from '@/types/dashboard'
import { normalizeDataSource, getSourceId } from '@/types/dashboard'
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
  // v0.7.0 approach: simple useState for data. React 18 automatically batches
  // state updates, preventing mid-scroll re-renders that produce blank frames.
  // The useReducer + RAF approach caused forceUpdate to fire during animation
  // frames, conflicting with browser scroll compositing.

  const [data, setData] = useState<T | null>(fallback ?? null)
  const [loading, setLoading] = useState(!enabled || !hasDataSourceValue ? false : true)
  const [error, setError] = useState<string | null>(null)
  const [lastUpdate, setLastUpdate] = useState<number | null>(null)
  const [sending, setSending] = useState(false)

  // Stable wrapper for setData — handles functional updates
  const setDataRaw = useCallback((d: unknown) => {
    setData(d as T)
  }, [])

  const dataSources = useMemo(() => dataSource ? normalizeDataSource(dataSource) : [], [dataSource])

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

  const optionsRef = useRef({ enabled, transform, fallback, preserveMultiple })
  optionsRef.current = { enabled, transform, fallback, preserveMultiple }
  const dataSourcesRef = useRef(dataSources)
  dataSourcesRef.current = dataSources

  // ============================================================================
  // A2. Reset state when dataSource changes (e.g., switching dashboards)
  // ============================================================================
  // Keep previous data visible while new fetch is in progress to avoid
  // chart flicker. Only reset error and loading states.
  const currentKey = useMemo(() => createStableKey(dataSource), [dataSource])
  const prevDataSourceKeyRef = useRef(currentKey)
  if (prevDataSourceKeyRef.current !== currentKey) {
    prevDataSourceKeyRef.current = currentKey
    setLoading(true)
    setError(null)
  }

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
  // D+E+J. Store source (readDataFromStore + store subscription + device WS)
  // ============================================================================

  const { readDataFromStore } = useStoreSource<T>(
    dataSources, currentKey, enabled,
    relevantDeviceIds, deviceInfoIds, hasTelemetrySource, needsWebSocket,
    {
      data, setData, setDataRaw, setLoading, setError, setLastUpdate,
      dataSourcesRef, optionsRef,
    }
  )

  // ============================================================================
  // F+G. Telemetry fetch + devices loading watcher (delegated to useTelemetrySource)
  // ============================================================================

  useTelemetrySource(telemetrySources, telemetryKey, enabled, hasTelemetrySource, relevantDeviceIds, {
    setData: (updater) => setData((prev) => typeof updater === 'function' ? (updater as (p: unknown) => unknown)(prev) as T : updater as T),
    setDataRaw, setLoading, setError, setLastUpdate, optionsRef,
    readDataFromStore,
  })

  // ============================================================================
  // H. System fetch (delegated to useSystemSource)
  // ============================================================================

  useSystemSource(systemSources, systemKey, enabled, {
    setDataRaw, setLoading, setError, setLastUpdate, optionsRef,
  })

  // ============================================================================
  // I+K. Extension fetch + WebSocket (delegated to useExtensionSource)
  // ============================================================================

  useExtensionSource(extensionSources, extensionKey, enabled, currentKey, relevantExtensionIds, {
    setData: (updater) => setData((prev) => typeof updater === 'function' ? (updater as (p: unknown) => unknown)(prev) as T : updater as T),
    setDataRaw, setLoading, setError, setLastUpdate, dataSourcesRef, optionsRef,
  })

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

