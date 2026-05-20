/**
 * Telemetry API client — new feature module
 *
 * Wraps existing telemetry endpoints with typed interfaces.
 */

import { fetchAPI } from '@/lib/api'
import type { TimeWindowConfig, TelemetryAggregate } from '../types'

// ============================================================================
// Types
// ============================================================================

export interface TelemetryPoint {
  timestamp: number
  value: number
}

export interface TelemetryQueryResult {
  sourceId: string
  data: TelemetryPoint[]
  count: number
  totalCount?: number
}

// ============================================================================
// Time window → start/end conversion
// ============================================================================

export function timeWindowToRange(window: TimeWindowConfig): { start: number; end: number } {
  const now = Math.floor(Date.now() / 1000)
  switch (window.type) {
    case 'now':
      return { start: now - 60, end: now }
    case 'last_5min':
      return { start: now - 5 * 60, end: now }
    case 'last_15min':
      return { start: now - 15 * 60, end: now }
    case 'last_30min':
      return { start: now - 30 * 60, end: now }
    case 'last_1hour':
      return { start: now - 3600, end: now }
    case 'last_6hours':
      return { start: now - 6 * 3600, end: now }
    case 'last_24hours':
      return { start: now - 24 * 3600, end: now }
    case 'today': {
      const d = new Date()
      d.setHours(0, 0, 0, 0)
      return { start: Math.floor(d.getTime() / 1000), end: now }
    }
    case 'yesterday': {
      const d = new Date()
      d.setHours(0, 0, 0, 0)
      const end = Math.floor(d.getTime() / 1000)
      return { start: end - 86400, end }
    }
    case 'this_week': {
      const d = new Date()
      d.setHours(0, 0, 0, 0)
      d.setDate(d.getDate() - d.getDay())
      return { start: Math.floor(d.getTime() / 1000), end: now }
    }
    case 'custom':
      return {
        start: window.startTime ?? now - 3600,
        end: window.endTime ?? now,
      }
    default:
      return { start: now - 3600, end: now }
  }
}

// ============================================================================
// Telemetry queries
// ============================================================================

/**
 * Query time-series telemetry for a device metric.
 * Uses GET /api/telemetry?source=...&metric=...&start=...&end=...
 */
export async function fetchDeviceTelemetry(
  deviceId: string,
  metric: string,
  window: TimeWindowConfig,
  limit = 500,
): Promise<TelemetryPoint[]> {
  const { start, end } = timeWindowToRange(window)
  const qs = new URLSearchParams({
    source: deviceId,
    metric,
    start: String(start),
    end: String(end),
    limit: String(limit),
  }).toString()
  const res = await fetchAPI<{
    source_id: string
    data: Array<{ timestamp: number; value: unknown; quality: number | null }>
    count: number
  }>(`/telemetry?${qs}`)
  return (res.data || []).map(p => ({
    timestamp: p.timestamp,
    value: typeof p.value === 'number' ? p.value : parseFloat(String(p.value)) || 0,
  }))
}

/**
 * Fetch current device telemetry (latest value).
 * Uses GET /api/devices/:id/current
 */
export async function fetchDeviceCurrentValue(
  deviceId: string,
  property?: string,
): Promise<{ value: unknown; timestamp?: number }> {
  const res = await fetchAPI<Record<string, unknown>>(`/devices/${deviceId}/current`)
  if (property && property in res) {
    return { value: res[property] }
  }
  return { value: res }
}

/**
 * Fetch extension metrics.
 * Uses GET /api/extensions/:id/metrics
 */
export async function fetchExtensionMetrics(
  extensionId: string,
): Promise<Record<string, unknown>> {
  return fetchAPI<Record<string, unknown>>(`/extensions/${extensionId}/metrics`)
}

/**
 * Fetch system metrics.
 * Uses GET /api/system/info
 */
export async function fetchSystemMetrics(): Promise<Record<string, unknown>> {
  return fetchAPI<Record<string, unknown>>('/system/info')
}
