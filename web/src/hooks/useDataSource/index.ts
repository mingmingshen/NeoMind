/**
 * useDataSource submodule — unified exports.
 */

// Cache
export { telemetryCache, systemStatsCache, extensionDataCache, clearGlobalCacheIntervals, cleanupTelemetryCache } from './cache'
export type { TelemetryRefreshMeta, TelemetryCacheData } from './cache'

// Batch fetching
export { fetchDeviceTelemetry, fetchedDevices, hasActiveFetch } from './batchFetch'
export type { BatchFetchResult } from './batchFetch'

// Telemetry fetching
export { fetchHistoricalTelemetry, readTelemetryCacheSync } from './telemetryFetch'

// System fetching
export { fetchSystemStats } from './systemFetch'

// Data extraction
export {
  safeExtractValue,
  findPropertyValue,
  extractValueFromData,
  extractValueFromParsed,
  eventMetricMatches,
  isVirtualMetric,
  TRANSFORM_NAMESPACES,
  hasCurrentValuesChanged,
} from './extractors'

// Dedup
export {
  isImageData,
  getPointValue,
  isDuplicatePoint,
  dedupeTelemetryPoints,
  isImageMetric,
  isImageDataSource,
  getDataSourceLimit,
} from './dedup'

// Event bus
export { registerDeviceListener, registerExtensionListener } from './eventBus'

// Store watcher
export { registerStoreWatcher, registerDevicesLoadingWatcher, getSharedDeviceMap } from './storeWatcher'

// Sub-hooks (not re-exported — used internally by useDataSource)
