/**
 * useExtensionMetric — TanStack Query hook for extension metric data
 */

import { useQuery } from '@tanstack/react-query'
import { dashboardKeys } from './queries'
import { fetchExtensionMetrics } from '../api/telemetry'
import type { DataSource } from '../types'

const POLLING_INTERVAL = 30_000

export interface ExtensionMetricResult {
  value: number | string | null
  unit?: string
  isLoading: boolean
  error: Error | null
  lastUpdated?: number
}

export function useExtensionMetric(
  source: DataSource | null,
): ExtensionMetricResult {
  const extensionId = source?.extensionId ?? ''
  const metricName = source?.extensionMetric ?? ''
  const enabled = !!extensionId

  const query = useQuery({
    queryKey: dashboardKeys.extensionMetrics(extensionId),
    queryFn: () => fetchExtensionMetrics(extensionId),
    enabled,
    refetchInterval: POLLING_INTERVAL,
    staleTime: 10_000,
  })

  const metrics = query.data ?? {}
  const raw = metricName ? metrics[metricName] : null
  const value = typeof raw === 'number' ? raw
    : typeof raw === 'string' ? raw
    : raw != null ? String(raw)
    : null

  return {
    value,
    unit: source?.extensionUnit,
    isLoading: query.isLoading,
    error: query.error as Error | null,
    lastUpdated: query.dataUpdatedAt || undefined,
  }
}
