/**
 * Dashboard data source types — new feature module
 *
 * Re-exports existing types from @/types/dashboard for compatibility,
 * and adds new ResolvedDataSource for live widget data.
 */

export type {
  DataSourceType,
  DataSource,
  DataSourceOrList,
  ValueMapping,
  TelemetryAggregate,
  TimeWindowType,
  TimeWindowConfig,
} from '@/types/dashboard'

export {
  isDataSourceList,
  normalizeDataSource,
  getSourceId,
} from '@/types/dashboard'

// ============================================================================
// Resolved Data Source — live data for widgets
// ============================================================================

import type { DataSource, DataSourceType } from '@/types/dashboard'

/** A single telemetry data point */
export interface TelemetryPoint {
  timestamp: number
  value: number
}

/** Live data wrapper that widgets receive */
export interface ResolvedDataSource {
  /** The raw config from the dashboard document */
  source: DataSource
  /** Source type discriminator */
  sourceType: DataSourceType
  /** Current single value (for gauges, indicators) */
  value: number | string | null
  /** Time-series data (for charts, sparklines) */
  timeSeries?: TelemetryPoint[]
  /** Loading state */
  isLoading: boolean
  /** Error if fetch failed */
  error: Error | null
  /** Unit string (e.g., "°C", "%") */
  unit?: string
  /** Last updated timestamp (ms) */
  lastUpdated?: number
  /** Min/max for range-based widgets */
  min?: number
  max?: number
}

/** Empty resolved data source (default before data loads) */
export function emptyResolvedDataSource(source: DataSource): ResolvedDataSource {
  return {
    source,
    sourceType: source.type,
    value: null,
    isLoading: true,
    error: null,
  }
}

// ============================================================================
// Data source type-specific config extraction helpers
// ============================================================================

/** Get a human-readable label for a data source */
export function getDataSourceLabel(ds: DataSource): string {
  switch (ds.type) {
    case 'device':
    case 'telemetry':
      return ds.sourceId ?? 'Unknown Device'
    case 'metric':
      return ds.metricId ?? 'Unknown Metric'
    case 'command':
      return `${ds.sourceId ?? 'Device'} / ${ds.command ?? 'Command'}`
    case 'device-info':
      return `${ds.sourceId ?? 'Device'} . ${ds.infoProperty ?? 'info'}`
    case 'system':
      return ds.systemMetric ?? 'System'
    case 'extension':
    case 'extension-metric':
      return `${ds.extensionId ?? 'Ext'} / ${ds.extensionMetric ?? 'metric'}`
    case 'extension-command':
      return `${ds.extensionId ?? 'Ext'} / ${ds.extensionCommand ?? 'command'}`
    case 'transform':
      return ds.transformId ?? 'Transform'
    case 'ai-metric':
      return ds.aiGroup ?? 'AI'
    case 'agent':
      return ds.agentId ?? 'Agent'
    default:
      return 'Unknown'
  }
}

/** Build a unique cache key for a data source + time window */
export function getDataSourceCacheKey(
  ds: DataSource,
  timeWindow?: string,
): string {
  const parts: string[] = [ds.type]
  if (ds.sourceId) parts.push(ds.sourceId)
  if (ds.metricId) parts.push(ds.metricId)
  if (ds.command) parts.push(ds.command)
  if (ds.extensionId) parts.push(ds.extensionId)
  if (ds.extensionMetric) parts.push(ds.extensionMetric)
  if (ds.systemMetric) parts.push(ds.systemMetric)
  if (ds.transformId) parts.push(ds.transformId)
  if (ds.agentId) parts.push(ds.agentId)
  if (ds.infoProperty) parts.push(ds.infoProperty)
  if (ds.property) parts.push(ds.property)
  if (timeWindow) parts.push(timeWindow)
  return parts.join(':')
}
