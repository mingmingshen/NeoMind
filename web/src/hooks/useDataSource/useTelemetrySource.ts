/**
 * useTelemetrySource — handles telemetry fetch + periodic refresh + retry.
 * Also handles devices loading watcher that defers telemetry fetch.
 */

import { useEffect, useRef } from 'react'
import type { DataSource } from '@/types/dashboard'
import { getUnifiedId, getUnifiedField, getUnifiedSource } from '@/types/dashboard'
import type { NeoMindStore } from '@/store'
import { useStore } from '@/store'
import { logError } from '@/lib/errors'
import { fetchHistoricalTelemetry, telemetryCache } from './fetch'
import {
  isImageDataSource, getDataSourceLimit,
} from './helpers'

/**
 * Check if a data source truly represents image data (not just any raw source).
 * Uses metricId naming heuristic — more restrictive than isImageDataSource
 * which also matches any includeRawPoints/raw transform.
 */
function isActualImageSource(metricId: string | undefined): boolean {
  if (!metricId) return false
  const lower = metricId.toLowerCase()
  return lower.includes('image') || lower.includes('img') ||
         lower.includes('frame') || lower.includes('snapshot') ||
         lower.includes('photo') || lower.includes('capture') ||
         metricId.includes('values.image')
}
import {
  getTs, getNewestTimestamp, extractPointsNewerThan, mergeLiveData,
  sortTelemetryResults,
} from './eventProcessors'

export interface TelemetrySourceState {
  setData: (value: unknown | ((prev: unknown) => unknown)) => void
  setDataRaw: (d: unknown) => void
  setLoading: (l: boolean) => void
  setError: (e: string | null) => void
  setLastUpdate: (ts: number | null) => void
  optionsRef: React.MutableRefObject<{
    enabled: boolean
    transform?: (data: unknown) => unknown
    fallback?: unknown
    preserveMultiple: boolean
  }>
  readDataFromStore: () => void
  /** Source-scoped loading adapters for useReducer state machine */
  sourceAdapters?: {
    startLoading: () => void
    finishLoading: () => void
    retryLoading: () => void
    failLoading: (error: string) => void
  }
}

export function useTelemetrySource(
  telemetrySources: DataSource[],
  telemetryKey: string,
  enabled: boolean,
  hasTelemetrySource: boolean,
  relevantDeviceIds: Set<string>,
  wsConnected: boolean,
  state: TelemetrySourceState
): void {
  const initialTelemetryFetchDoneRef = useRef(false)
  const emptyRetryCountRef = useRef(0)
  const retryInProgressRef = useRef(false)
  const deferredByDevicesLoadingRef = useRef(false)
  const telemetryIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const fetchTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const prevTelemetryKeyRef = useRef('')
  const fetchGenerationRef = useRef(0)

  // ============================================================================
  // Devices loading watcher
  // ============================================================================

  useEffect(() => {
    if (relevantDeviceIds.size === 0 && !hasTelemetrySource) return

    let prevLoading = useStore.getState().devicesLoading
    const unsubscribe = useStore.subscribe((s: NeoMindStore) => {
      if (s.devicesLoading === prevLoading) return
      prevLoading = s.devicesLoading

      if (!s.devicesLoading && deferredByDevicesLoadingRef.current) {
        deferredByDevicesLoadingRef.current = false
        state.readDataFromStore()
      }
    })

    return () => unsubscribe()
  }, [relevantDeviceIds, hasTelemetrySource])

  // ============================================================================
  // Telemetry fetch effect
  // ============================================================================

  useEffect(() => {
    if (!hasTelemetrySource || !enabled) {
      if (telemetryIntervalRef.current) { clearInterval(telemetryIntervalRef.current); telemetryIntervalRef.current = null }
      return
    }

    const configChanged = prevTelemetryKeyRef.current !== telemetryKey
    if (configChanged && telemetryKey) {
      initialTelemetryFetchDoneRef.current = false
      emptyRetryCountRef.current = 0
      retryInProgressRef.current = false
      // Invalidate cache for changed sources to prevent stale data
      telemetrySources.forEach(ds => {
        const deviceId = getUnifiedId(ds)
        const metricId = getUnifiedField(ds)
        if (deviceId && metricId) {
          telemetryCache.deleteWhere((_, key) => key.startsWith(`${deviceId}|${metricId}|`))
        }
      })
    }
    prevTelemetryKeyRef.current = telemetryKey

    // Bump generation so stale in-flight fetches can be discarded
    const currentGeneration = ++fetchGenerationRef.current

    const fetchTelemetryData = async () => {
      const isInitialFetch = !initialTelemetryFetchDoneRef.current
      if (isInitialFetch) {
        if (state.sourceAdapters) state.sourceAdapters.startLoading()
        else state.setLoading(true)
      }
      state.setError(null)

      // Track timeout so it can be cleaned up on unmount
      if (fetchTimeoutRef.current) clearTimeout(fetchTimeoutRef.current)
      const timeoutPromise = new Promise<never>((_, reject) => {
        fetchTimeoutRef.current = setTimeout(() => {
          fetchTimeoutRef.current = null
          reject(new Error('Fetch timeout'))
        }, 10000)
      })

      try {
        const results = await Promise.race([
          Promise.all(
            telemetrySources.map(async (ds) => {
              let dsSourceId = getUnifiedId(ds)
              const dsMetricId = getUnifiedField(ds)
              if (!dsSourceId || !dsMetricId) return { data: [], raw: undefined }
              // Restore prefix for transform/ai sources — fetchHistoricalTelemetry
              // uses the prefix to route to the correct API endpoint
              const dsSource = getUnifiedSource(ds)
              if (dsSource === 'transform' && !dsSourceId.startsWith('transform:')) {
                dsSourceId = `transform:${dsSourceId}`
              } else if (dsSource === 'ai' && !dsSourceId.startsWith('ai:')) {
                dsSourceId = `ai:${dsSourceId}`
              }
              const includeRawPoints = ds.params?.includeRawPoints === true || ds.transform === 'raw'
              const bypassCache = !initialTelemetryFetchDoneRef.current || includeRawPoints
              const isImg = isImageDataSource(ds)
              const actualTimeRange = ds.timeRange ?? (isImg ? 48 : 1)
              const actualLimit = ds.limit ?? (isImg ? 200 : 50)
              // Charts (includeRawPoints=true) must always fetch raw data to preserve
              // timestamps for time-series rendering. Value components (includeRawPoints=false)
              // use aggregateExt for single-value aggregation.
              const actualAggregate = includeRawPoints ? 'raw' : (ds.aggregateExt ?? 'raw')

              const response = await fetchHistoricalTelemetry(
                dsSourceId, dsMetricId, actualTimeRange, actualLimit, actualAggregate, includeRawPoints, bypassCache,
                ds.timeWindow,
                ds.params?.isImage === true || isActualImageSource(dsMetricId),
              )
              if (includeRawPoints && response.raw) return { data: response.data, raw: response.raw, success: response.success }
              return { data: response.success ? response.data : [], success: response.success }
            })
          ),
          timeoutPromise
        ]) as Array<{ data: unknown[]; raw?: unknown[]; success: boolean }>

        // Clear timeout — fetch succeeded before the deadline
        if (fetchTimeoutRef.current) { clearTimeout(fetchTimeoutRef.current); fetchTimeoutRef.current = null }
        // Discard stale results if config changed while fetching
        if (fetchGenerationRef.current !== currentGeneration) return

        let finalData: unknown
        const pm = state.optionsRef.current.preserveMultiple
        if (results.length > 1) {
          if (pm) {
            const hasRawData = results.some((r) => r.raw !== undefined)
            finalData = hasRawData ? results.map((r) => r.raw ?? []) : results.map((r) => r.data ?? [])
          } else {
            const hasRawData = results.some((r) => r.raw !== undefined)
            finalData = hasRawData ? results.flatMap((r) => r.raw ?? []) : results.map((r) => r.data ?? []).flat()
          }
        } else {
          const r = results[0]
          finalData = (r?.raw ?? r?.data) ?? []
        }

        // Sort and dedup
        finalData = sortTelemetryResults(finalData, telemetrySources, pm)

        state.setData((prevData: unknown) => {
          // No previous data — just set fetched data
          if (prevData == null) return finalData

          // Don't merge when fetched data is empty — let empty handling work
          const fetchedArr = Array.isArray(finalData) ? finalData : []
          if (fetchedArr.length === 0) {
            if (prevData == null || (Array.isArray(prevData) && (prevData as unknown[]).length === 0)) return finalData
            // Periodic refresh: preserve existing data instead of blanking charts
            return prevData
          }

          // Only merge live points that are strictly newer than the newest fetched point
          const newestFetchedTs = getNewestTimestamp(finalData, getTs)
          if (newestFetchedTs === 0) return finalData

          const sixtySecondsAgo = Math.floor(Date.now() / 1000) - 60
          const cutoffTs = Math.max(newestFetchedTs, sixtySecondsAgo)
          const livePoints = extractPointsNewerThan(prevData, cutoffTs, getTs, pm, telemetrySources.length)

          if (livePoints.length === 0) return finalData

          // Merge live points into fetched data
          return mergeLiveData(finalData, livePoints, getTs, pm, telemetrySources)
        })
        state.setLastUpdate(Date.now())
        initialTelemetryFetchDoneRef.current = true

        // Empty result — quick retry then accept empty state
        const isEmpty = Array.isArray(finalData) ? finalData.length === 0 : finalData == null
        if (isEmpty) {
          const { devicesLoading } = useStore.getState()
          if (devicesLoading) {
            deferredByDevicesLoadingRef.current = true
            initialTelemetryFetchDoneRef.current = false
            if (state.sourceAdapters) state.sourceAdapters.startLoading()
            else state.setLoading(true)
          } else {
            emptyRetryCountRef.current += 1
            // Single-value components (LED, ValueCard) benefit from more aggressive
            // retries — they only need 1 data point and it may arrive shortly after
            // mount. Use up to 5 retries at 1s intervals (total ~5s coverage).
            // Time-series components use the original 2-retry limit since they
            // depend on polling for bulk data anyway.
            const isSingleValueFetch = telemetrySources.every(
              ds => (ds.aggregateExt === 'latest' || ds.aggregateExt === 'first') && (ds.limit ?? 50) <= 1
            )
            const maxRetries = isSingleValueFetch ? 5 : 2
            if (emptyRetryCountRef.current <= maxRetries) {
              const delay = isSingleValueFetch ? 1000 : 1500 * emptyRetryCountRef.current
              retryInProgressRef.current = true
              if (state.sourceAdapters) state.sourceAdapters.retryLoading()
              else state.setLoading(true)
              if (retryTimerRef.current) clearTimeout(retryTimerRef.current)
              retryTimerRef.current = setTimeout(() => {
                retryTimerRef.current = null
                if (fetchGenerationRef.current !== currentGeneration) return
                fetchTelemetryData()
              }, delay)
            } else {
              // Retries exhausted — clear loading so widget shows empty state
              retryInProgressRef.current = false
            }
          }
        } else {
          emptyRetryCountRef.current = 0
          retryInProgressRef.current = false
        }
      } catch (err) {
        // Discard stale error if config changed while fetching
        if (fetchGenerationRef.current !== currentGeneration) return
        logError(err, { operation: 'Fetch telemetry data' })
        state.setError(err instanceof Error ? err.message : 'Failed to fetch telemetry')
        // Preserve previous data on error
        if (initialTelemetryFetchDoneRef.current) {
          const { transform: transformFn } = state.optionsRef.current
          state.setData((prev: unknown) => {
            if (prev == null || (Array.isArray(prev) && (prev as unknown[]).length === 0)) return (transformFn ? transformFn([]) : [])
            return prev
          })
        } else {
          state.setDataRaw([])
        }
        initialTelemetryFetchDoneRef.current = true
      } finally {
        // Only clear loading when not retrying for empty results and not deferred
        if (fetchGenerationRef.current === currentGeneration && !deferredByDevicesLoadingRef.current && !retryInProgressRef.current) {
          if (state.sourceAdapters) state.sourceAdapters.finishLoading()
          else state.setLoading(false)
        }
      }
    }

    if (telemetryIntervalRef.current) { clearInterval(telemetryIntervalRef.current); telemetryIntervalRef.current = null }
    fetchTelemetryData()

    // Always start polling as a safety net. WS delivers real-time updates,
    // but if a device hasn't sent data yet or data was missed, polling fills the gap.
    // Use a longer interval when WS is connected (just a backup) vs disconnected (primary source).
    const refreshIntervals = telemetrySources.map((ds) => ds.refresh).filter(Boolean) as number[]
    const baseRefresh = refreshIntervals.length > 0 ? refreshIntervals.reduce((a, b) => Math.min(a, b), Infinity) : 30
    const pollingInterval = wsConnected ? baseRefresh * 2 : baseRefresh
    telemetryIntervalRef.current = setInterval(fetchTelemetryData, pollingInterval * 1000)

    return () => {
      if (telemetryIntervalRef.current) { clearInterval(telemetryIntervalRef.current); telemetryIntervalRef.current = null }
      if (retryTimerRef.current) { clearTimeout(retryTimerRef.current); retryTimerRef.current = null }
      if (fetchTimeoutRef.current) { clearTimeout(fetchTimeoutRef.current); fetchTimeoutRef.current = null }
    }
  }, [telemetryKey, enabled, wsConnected])
}
