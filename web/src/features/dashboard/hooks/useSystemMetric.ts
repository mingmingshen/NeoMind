/**
 * useSystemMetric — TanStack Query hook for system metrics
 */

import { useQuery } from '@tanstack/react-query'
import { dashboardKeys } from './queries'
import { fetchSystemMetrics } from '../api/telemetry'
import type { DataSource } from '../types'

const POLLING_INTERVAL = 60_000 // 1min

export interface SystemMetricResult {
  value: number | string | null
  isLoading: boolean
  error: Error | null
  lastUpdated?: number
}

export function useSystemMetric(
  source: DataSource | null,
): SystemMetricResult {
  const metricName = source?.systemMetric ?? ''
  const enabled = !!metricName

  const query = useQuery({
    queryKey: dashboardKeys.systemMetrics(),
    queryFn: fetchSystemMetrics,
    enabled,
    refetchInterval: POLLING_INTERVAL,
    staleTime: 30_000,
  })

  const metrics = query.data ?? {}
  const raw = metricName ? metrics[metricName] : null
  const value = typeof raw === 'number' ? raw
    : typeof raw === 'string' ? raw
    : raw != null ? String(raw)
    : null

  return {
    value,
    isLoading: query.isLoading,
    error: query.error as Error | null,
    lastUpdated: query.dataUpdatedAt || undefined,
  }
}
