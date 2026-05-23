/**
 * useTelemetrySource — handles telemetry fetch + periodic refresh + retry.
 * Also handles devices loading watcher that defers telemetry fetch.
 */

import { useEffect, useRef } from 'react'
import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import type { NeoMindStore } from '@/store'
import { useStore } from '@/store'
import { logError } from '@/lib/errors'
import { fetchHistoricalTelemetry, telemetryCache } from './fetch'
import {
  isImageDataSource, getDataSourceLimit,
} from './helpers'
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
}

export function useTelemetrySource(
  telemetrySources: DataSource[],
  telemetryKey: string,
  enabled: boolean,
  hasTelemetrySource: boolean,
  relevantDeviceIds: Set<string>,
  state: TelemetrySourceState
): void {
  const initialTelemetryFetchDoneRef = useRef(false)
  const emptyRetryCountRef = useRef(0)
  const deferredByDevicesLoadingRef = useRef(false)
  const telemetryIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const prevTelemetryKeyRef = useRef('')

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
    if (configChanged && telemetryKey) initialTelemetryFetchDoneRef.current = false
    prevTelemetryKeyRef.current = telemetryKey

    const fetchTelemetryData = async () => {
      const isInitialFetch = !initialTelemetryFetchDoneRef.current
      if (isInitialFetch) state.setLoading(true)
      state.setError(null)

      const timeoutPromise = new Promise((_, reject) => setTimeout(() => reject(new Error('Fetch timeout')), 10000))

      try {
        const results = await Promise.race([
          Promise.all(
            telemetrySources.map(async (ds) => {
              if (!getSourceId(ds) || !ds.metricId) return { data: [], raw: undefined }
              const includeRawPoints = ds.params?.includeRawPoints === true || ds.transform === 'raw'
              const bypassCache = !initialTelemetryFetchDoneRef.current || includeRawPoints
              const isImg = isImageDataSource(ds.params, ds.transform, ds.metricId)
              const actualTimeRange = ds.timeRange ?? (isImg ? 48 : 1)
              const actualLimit = ds.limit ?? (isImg ? 200 : 50)
              const actualAggregate = ds.aggregateExt ?? 'raw'

              const response = await fetchHistoricalTelemetry(
                getSourceId(ds)!, ds.metricId, actualTimeRange, actualLimit, actualAggregate, includeRawPoints, bypassCache,
                ds.timeWindow
              )
              if (includeRawPoints && response.raw) return { data: response.data, raw: response.raw, success: response.success }
              return { data: response.success ? response.data : [], success: response.success }
            })
          ),
          timeoutPromise
        ]) as Array<{ data: unknown[]; raw?: unknown[]; success: boolean }>

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

        // Merge: preserve any real-time points from WebSocket that are newer
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

        // Empty result retry with exponential backoff
        const isEmpty = Array.isArray(finalData) ? finalData.length === 0 : finalData == null
        if (isEmpty) {
          const { devicesLoading } = useStore.getState()
          if (devicesLoading) {
            deferredByDevicesLoadingRef.current = true
            initialTelemetryFetchDoneRef.current = false
            state.setLoading(true)
          } else {
            emptyRetryCountRef.current += 1
            // Retry up to 5 times with exponential backoff: 2s, 4s, 8s, 16s, 32s
            if (emptyRetryCountRef.current <= 5) {
              const delay = Math.min(2000 * Math.pow(2, emptyRetryCountRef.current - 1), 32000)
              setTimeout(() => fetchTelemetryData(), delay)
            }
          }
        } else {
          emptyRetryCountRef.current = 0
        }
      } catch (err) {
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
        if (!deferredByDevicesLoadingRef.current) state.setLoading(false)
      }
    }

    if (telemetryIntervalRef.current) { clearInterval(telemetryIntervalRef.current); telemetryIntervalRef.current = null }
    fetchTelemetryData()

    const refreshIntervals = telemetrySources.map((ds) => ds.refresh).filter(Boolean) as number[]
    const minRefresh = refreshIntervals.length > 0 ? Math.min(...refreshIntervals) : null
    if (minRefresh) telemetryIntervalRef.current = setInterval(fetchTelemetryData, minRefresh * 1000)

    return () => { if (telemetryIntervalRef.current) { clearInterval(telemetryIntervalRef.current); telemetryIntervalRef.current = null } }
  }, [telemetryKey, enabled])
}
