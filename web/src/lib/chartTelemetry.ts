/**
 * Shared telemetry source conversion for chart components.
 *
 * Converts device/metric data sources into telemetry sources suitable
 * for fetching historical time-series data.
 *
 * Always fetches raw data points — aggregation is handled by the frontend
 * (chart components or telemetryTransform) so timestamps are preserved.
 */

import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import {
  getEffectiveTimeWindow,
  timeWindowToHours,
} from '@/lib/telemetryTransform'

/**
 * Convert a data source to a telemetry source for chart data fetching.
 * Handles telemetry, device, and metric source types.
 *
 * Always uses aggregate='raw' so the API returns data points with timestamps.
 * Frontend applies the user's chosen aggregation (via aggregateExt) locally.
 */
export function toTelemetrySource(
  dataSource?: DataSource,
  limit: number = 50,
  timeRange: number = 1
): DataSource | undefined {
  if (!dataSource) return undefined

  const effectiveTimeWindow = getEffectiveTimeWindow(dataSource)

  if (dataSource.type === 'telemetry') {
    return {
      ...dataSource,
      limit: dataSource.limit ?? limit,
      timeRange: dataSource.timeRange ?? timeWindowToHours(effectiveTimeWindow.type),
      aggregate: 'raw',
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
      aggregate: 'raw',
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
      aggregate: 'raw',
      params: { includeRawPoints: true },
      transform: 'raw',
    }
  }

  return dataSource
}
