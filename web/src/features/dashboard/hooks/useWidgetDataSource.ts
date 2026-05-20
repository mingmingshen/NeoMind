/**
 * useWidgetDataSource — unified data source hook
 *
 * Inspects DataSource.type and delegates to the correct per-source hook.
 * Returns ResolvedDataSource for widgets to consume.
 */

import type { DataSource, TimeWindowConfig, ResolvedDataSource } from '../types'
import { emptyResolvedDataSource } from '../types'
import { useDeviceTelemetry } from './useDeviceTelemetry'
import { useExtensionMetric } from './useExtensionMetric'
import { useSystemMetric } from './useSystemMetric'

export function useWidgetDataSource(
  source: DataSource | null | undefined,
  timeWindow?: TimeWindowConfig,
): ResolvedDataSource | null {
  // All hooks must be called unconditionally (Rules of Hooks)
  const deviceResult = useDeviceTelemetry(
    source?.type === 'device' || source?.type === 'telemetry' || source?.type === 'metric' || source?.type === 'command' || source?.type === 'device-info'
      ? source
      : null,
    timeWindow,
  )

  const extensionResult = useExtensionMetric(
    source?.type === 'extension' || source?.type === 'extension-metric' || source?.type === 'extension-command'
      ? source
      : null,
  )

  const systemResult = useSystemMetric(
    source?.type === 'system' ? source : null,
  )

  if (!source) return null

  switch (source.type) {
    case 'device':
    case 'telemetry':
    case 'metric':
    case 'command':
    case 'device-info':
      return {
        source,
        sourceType: source.type,
        value: deviceResult.value,
        timeSeries: deviceResult.timeSeries,
        isLoading: deviceResult.isLoading,
        error: deviceResult.error,
        unit: deviceResult.unit,
        lastUpdated: deviceResult.lastUpdated,
      }

    case 'extension':
    case 'extension-metric':
    case 'extension-command':
      return {
        source,
        sourceType: source.type,
        value: extensionResult.value,
        isLoading: extensionResult.isLoading,
        error: extensionResult.error,
        unit: extensionResult.unit,
        lastUpdated: extensionResult.lastUpdated,
      }

    case 'system':
      return {
        source,
        sourceType: source.type,
        value: systemResult.value,
        isLoading: systemResult.isLoading,
        error: systemResult.error,
        lastUpdated: systemResult.lastUpdated,
      }

    case 'transform':
    case 'ai-metric':
    case 'agent':
      // These source types are handled by the widgets themselves
      // (they have their own data fetching logic)
      return emptyResolvedDataSource(source)

    default:
      // Handle static values (indicated by staticValue field, not a separate type)
      if (source.staticValue != null) {
        return {
          source,
          sourceType: source.type,
          value: String(source.staticValue),
          isLoading: false,
          error: null,
        }
      }
      return null
  }
}
