/**
 * useTelemetryFetching — Telemetry data fetching effect for useDataSource.
 *
 * Handles:
 * - Initial fetch and periodic refresh for telemetry/transform/ai-metric sources
 * - Store merge for real-time updates
 * - Sort, dedup, and aggregation of telemetry points
 * - Empty result retry and deferred loading
 */

import { useEffect, useRef, useMemo } from 'react'
import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import { useStore } from '@/store'
import { createStableKey } from '@/lib/stable-key'
import { logError } from '@/lib/errors'
import { fetchHistoricalTelemetry } from './telemetryFetch'
import { isDuplicatePoint, isImageDataSource, getDataSourceLimit, dedupeTelemetryPoints, getPointValue } from './dedup'

// ============================================================================
// Types
// ============================================================================

interface UseTelemetryFetchingOptions {
  dataSources: DataSource[]
  enabled: boolean
  telemetryRefreshTrigger: number
  preserveMultiple: boolean
  setData: (data: unknown) => void
  setLoading: (loading: boolean) => void
  setError: (error: string | null) => void
  setLastUpdate: (ts: number) => void
  setTelemetryRefreshTrigger: (fn: (n: number) => number) => void
  initialFetchDoneRef: React.MutableRefObject<boolean>
  emptyRetryCountRef: React.MutableRefObject<number>
  deferredByDevicesLoadingRef: React.MutableRefObject<boolean>
}

// ============================================================================
// Main Hook
// ============================================================================

export function useTelemetryFetching({
  dataSources,
  enabled,
  telemetryRefreshTrigger,
  preserveMultiple,
  setData,
  setLoading,
  setError,
  setLastUpdate,
  setTelemetryRefreshTrigger,
  initialFetchDoneRef,
  emptyRetryCountRef,
  deferredByDevicesLoadingRef,
}: UseTelemetryFetchingOptions) {
  const telemetryIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const prevTelemetryKeyRef = useRef<string>('')

  const telemetryDataSources = useMemo(() => {
    return dataSources.filter((ds) => ds.type === 'telemetry' || ds.type === 'transform' || ds.type === 'ai-metric')
  }, [dataSources])

  const hasTelemetrySource = telemetryDataSources.length > 0

  const telemetryKey = useMemo(() => {
    return telemetryDataSources
      .map((ds) => {
        const isImg = isImageDataSource(ds.params, ds.transform, ds.metricId)
        const actualTimeRange = ds.timeRange ?? (isImg ? 48 : 1)
        const actualLimit = ds.limit ?? (isImg ? 200 : 50)
        const actualAggregate = ds.aggregate ?? ds.aggregateExt ?? 'raw'
        return createStableKey({
          deviceId: getSourceId(ds),
          metricId: ds.metricId,
          timeRange: actualTimeRange,
          limit: actualLimit,
          aggregate: actualAggregate
        })
      })
      .join('|')
  }, [telemetryDataSources])

  useEffect(() => {
    if (!hasTelemetrySource || !enabled) {
      if (telemetryIntervalRef.current) {
        clearInterval(telemetryIntervalRef.current)
        telemetryIntervalRef.current = null
      }
      return
    }

    const configChanged = prevTelemetryKeyRef.current !== telemetryKey
    if (configChanged && telemetryKey) {
      initialFetchDoneRef.current = false
    }
    prevTelemetryKeyRef.current = telemetryKey

    const fetchTelemetryData = async () => {
      const isInitialFetch = !initialFetchDoneRef.current
      if (isInitialFetch) setLoading(true)
      setError(null)

      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error('Fetch timeout')), 10000)
      )

      try {
        const results = await Promise.race([
          Promise.all(
            telemetryDataSources.map(async (ds) => {
              if (!getSourceId(ds) || !ds.metricId) return { data: [], raw: undefined }
              const includeRawPoints = ds.params?.includeRawPoints === true || ds.transform === 'raw'
              const bypassCache = !initialFetchDoneRef.current || includeRawPoints
              const isImg = isImageDataSource(ds.params, ds.transform, ds.metricId)
              const actualTimeRange = ds.timeRange ?? (isImg ? 48 : 1)
              const actualLimit = ds.limit ?? (isImg ? 200 : 50)
              const actualAggregate = ds.aggregate ?? ds.aggregateExt ?? 'raw'

              const response = await fetchHistoricalTelemetry(
                getSourceId(ds)!, ds.metricId, actualTimeRange, actualLimit, actualAggregate, includeRawPoints, bypassCache
              )
              if (includeRawPoints && response.raw) return { data: response.data, raw: response.raw, success: response.success }
              return { data: response.success ? response.data : [], success: response.success }
            })
          ),
          timeoutPromise
        ]) as Array<{ data: unknown[]; raw?: unknown[]; success: boolean }>

        let finalData: unknown
        if (results.length > 1) {
          if (preserveMultiple) {
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
        finalData = sortAndDedup(finalData, telemetryDataSources, preserveMultiple)

        setData(finalData)
        setLastUpdate(Date.now())
        initialFetchDoneRef.current = true

        // Empty result retry
        const isEmpty = Array.isArray(finalData) ? finalData.length === 0 : finalData == null
        if (isEmpty) {
          const { devicesLoading } = useStore.getState()
          if (devicesLoading) {
            deferredByDevicesLoadingRef.current = true
            initialFetchDoneRef.current = false
            setLoading(true)
          } else {
            emptyRetryCountRef.current += 1
            if (emptyRetryCountRef.current <= 3) setTimeout(() => fetchTelemetryData(), 3000)
          }
        } else {
          emptyRetryCountRef.current = 0
        }
      } catch (err) {
        logError(err, { operation: 'Fetch telemetry data' })
        setError(err instanceof Error ? err.message : 'Failed to fetch telemetry')
        setData([])
        initialFetchDoneRef.current = true
      } finally {
        if (!deferredByDevicesLoadingRef.current) setLoading(false)
      }
    }

    if (telemetryIntervalRef.current) { clearInterval(telemetryIntervalRef.current); telemetryIntervalRef.current = null }
    fetchTelemetryData()

    const refreshIntervals = telemetryDataSources.map((ds) => ds.refresh).filter(Boolean) as number[]
    const minRefresh = refreshIntervals.length > 0 ? Math.min(...refreshIntervals) : null
    if (minRefresh) telemetryIntervalRef.current = setInterval(fetchTelemetryData, minRefresh * 1000)

    return () => { if (telemetryIntervalRef.current) { clearInterval(telemetryIntervalRef.current); telemetryIntervalRef.current = null } }
  }, [telemetryKey, enabled, telemetryRefreshTrigger])

  return { hasTelemetrySource }
}

// ============================================================================
// Helpers
// ============================================================================

function sortAndDedup(finalData: unknown, sources: DataSource[], preserveMultiple: boolean): unknown {
  const isPM = preserveMultiple && sources.length > 1
  const process = (points: unknown[], ds: DataSource): unknown[] => {
    if (!Array.isArray(points) || points.length === 0) return points
    const isImg = isImageDataSource(ds.params, ds.transform, ds.metricId)
    const maxLimit = getDataSourceLimit(ds)
    const getTs = (p: unknown): number => { if (p == null) return 0; const o = p as Record<string, unknown>; return (o.timestamp ?? o.time ?? o.t ?? 0) as number }
    const idx = points.map((p, i) => ({ p, i }))
    idx.sort((a, b) => { const d = getTs(b.p) - getTs(a.p); return d !== 0 ? d : a.i - b.i })
    const sorted = idx.map(({ p }) => p)
    if (isImg) {
      const out: unknown[] = []
      for (const pt of sorted) {
        if (!isDuplicatePoint(out, getTs(pt), getPointValue(pt), getTs)) out.push(pt)
        if (out.length >= maxLimit) break
      }
      return out
    }
    return dedupeTelemetryPoints(sorted, getTs, maxLimit)
  }
  if (isPM && Array.isArray(finalData)) return sources.map((ds, i) => process((finalData as unknown[][])[i], ds))
  if (Array.isArray(finalData) && sources.length > 0) return process(finalData, sources[0])
  return finalData
}
