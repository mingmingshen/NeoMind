/**
 * useDeviceTelemetry — TanStack Query hook for device time-series data
 */

import { useQuery, useQueryClient } from '@tanstack/react-query'
import { dashboardKeys } from './queries'
import { fetchDeviceTelemetry, fetchDeviceCurrentValue } from '../api/telemetry'
import type { DataSource, TimeWindowConfig } from '../types'
import type { TelemetryPoint } from '../api/telemetry'

const DEFAULT_WINDOW: TimeWindowConfig = { type: 'last_1hour' }
const POLLING_INTERVAL = 30_000 // 30s

export interface DeviceTelemetryResult {
  value: number | string | null
  timeSeries: TelemetryPoint[]
  unit?: string
  isLoading: boolean
  error: Error | null
  lastUpdated?: number
}

export function useDeviceTelemetry(
  source: DataSource | null,
  timeWindow?: TimeWindowConfig,
): DeviceTelemetryResult {
  const window = timeWindow ?? source?.timeWindow ?? DEFAULT_WINDOW
  const windowKey = window.type === 'custom'
    ? `custom_${window.startTime}_${window.endTime}`
    : window.type

  const deviceId = source?.sourceId ?? ''
  const metric = source?.property ?? source?.metricId ?? ''
  const enabled = !!deviceId && !!metric

  // For 'now' window: fetch current value only
  const isCurrentOnly = window.type === 'now'

  const currentQuery = useQuery({
    queryKey: dashboardKeys.deviceCurrent(deviceId),
    queryFn: () => fetchDeviceCurrentValue(deviceId, metric),
    enabled: enabled && isCurrentOnly,
    refetchInterval: POLLING_INTERVAL,
    staleTime: 10_000,
  })

  const historyQuery = useQuery({
    queryKey: dashboardKeys.telemetry(
      `${deviceId}:${metric}`,
      windowKey,
    ),
    queryFn: () => fetchDeviceTelemetry(deviceId, metric, window),
    enabled: enabled && !isCurrentOnly,
    refetchInterval: POLLING_INTERVAL,
    staleTime: 10_000,
  })

  if (isCurrentOnly) {
    const raw = currentQuery.data?.value
    const value = typeof raw === 'number' ? raw
      : typeof raw === 'string' ? raw
      : raw != null ? String(raw)
      : null
    return {
      value,
      timeSeries: [],
      isLoading: currentQuery.isLoading,
      error: currentQuery.error as Error | null,
      lastUpdated: currentQuery.dataUpdatedAt || undefined,
    }
  }

  const series = historyQuery.data ?? []
  const latest = series.length > 0 ? series[series.length - 1].value : null

  return {
    value: latest,
    timeSeries: series,
    isLoading: historyQuery.isLoading,
    error: historyQuery.error as Error | null,
    lastUpdated: historyQuery.dataUpdatedAt || undefined,
  }
}
