/**
 * Shared telemetry source conversion for chart components.
 *
 * Converts device/metric data sources into telemetry sources suitable
 * for fetching historical time-series data.
 */

import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import {
  getEffectiveAggregate,
  getEffectiveTimeWindow,
  timeWindowToHours,
} from '@/lib/telemetryTransform'

/**
 * Convert a data source to a telemetry source for chart data fetching.
 * Handles telemetry, device, and metric source types.
 */
export function toTelemetrySource(
  dataSource?: DataSource,
  limit: number = 50,
  timeRange: number = 1
): DataSource | undefined {
  if (!dataSource) return undefined

  const effectiveTimeWindow = getEffectiveTimeWindow(dataSource)
  const effectiveAggregate = getEffectiveAggregate(dataSource)

  if (dataSource.type === 'telemetry') {
    return {
      ...dataSource,
      limit: dataSource.limit ?? limit,
      timeRange: dataSource.timeRange ?? timeWindowToHours(effectiveTimeWindow.type),
      aggregate: dataSource.aggregate ?? (effectiveAggregate === 'raw' ? 'raw' : 'avg'),
      params: {
        ...dataSource.params,
        includeRawPoints: true,
      },
      transform: dataSource.transform ?? 'raw',
    }
  }

  const sourceId = getSourceId(dataSource)
  if (dataSource.type === 'device' && sourceId) {
    return {
      type: 'telemetry',
      sourceId,
      metricId: dataSource.metricId ?? dataSource.property ?? 'value',
      timeRange: timeWindowToHours(effectiveTimeWindow.type),
      limit,
      aggregate: effectiveAggregate === 'raw' ? 'raw' : 'avg',
      params: { includeRawPoints: true },
      transform: 'raw',
    }
  }

  if (dataSource.type === 'metric' && sourceId) {
    return {
      type: 'telemetry',
      sourceId,
      metricId: dataSource.metricId ?? dataSource.property ?? 'value',
      timeRange: timeWindowToHours(effectiveTimeWindow.type),
      limit,
      aggregate: effectiveAggregate === 'raw' ? 'raw' : 'avg',
      params: { includeRawPoints: true },
      transform: 'raw',
    }
  }

  return dataSource
}
