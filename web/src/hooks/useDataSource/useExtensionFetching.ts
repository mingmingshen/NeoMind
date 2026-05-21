/**
 * useExtensionFetching — Extension data fetching effect for useDataSource.
 *
 * Handles initial fetch and periodic refresh for extension-type data sources.
 */

import { useEffect, useRef, useMemo } from 'react'
import type { DataSource } from '@/types/dashboard'
import { createStableKey } from '@/lib/stable-key'
import { logError } from '@/lib/errors'
import { extensionDataCache } from './cache'

interface UseExtensionFetchingOptions {
  dataSources: DataSource[]
  enabled: boolean
  transform: ((data: unknown) => unknown) | undefined
  fallback: unknown
  setData: (data: unknown) => void
  setLoading: (loading: boolean) => void
  setError: (error: string | null) => void
  setLastUpdate: (ts: number) => void
}

export function useExtensionFetching({
  dataSources,
  enabled,
  transform,
  fallback,
  setData,
  setLoading,
  setError,
  setLastUpdate,
}: UseExtensionFetchingOptions) {
  const extensionIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const initialFetchDoneRef = useRef(false)

  const extensionDataSources = useMemo(() => {
    return dataSources.filter((ds) => ds.type === 'extension')
  }, [dataSources])

  const hasExtensionSource = extensionDataSources.length > 0

  const extensionKey = useMemo(() => {
    return extensionDataSources
      .map((ds) => createStableKey({
        extensionId: ds.extensionId,
        extensionMetric: ds.extensionMetric,
      }))
      .join('|')
  }, [extensionDataSources])

  useEffect(() => {
    if (!hasExtensionSource || !enabled) {
      if (extensionIntervalRef.current) {
        clearInterval(extensionIntervalRef.current)
        extensionIntervalRef.current = null
      }
      return
    }

    const fetchExtensionData = async () => {
      if (!initialFetchDoneRef.current) setLoading(true)
      setError(null)

      try {
        const api = (await import('@/lib/api')).api
        const results = await Promise.all(
          extensionDataSources.map(async (ds) => {
            const extensionId = ds.extensionId
            const metric = ds.extensionMetric
            if (!extensionId || !metric) return { data: null }

            // Check shared cache
            const extCacheKey = `${extensionId}|${metric}`
            const extCached = extensionDataCache.get(extCacheKey)
            if (extCached !== undefined) return { data: extCached, success: true }

            // V2 data source (format: command:field)
            const isV2 = metric.includes(':')
            const parts = metric.split(':')

            try {
              if (isV2 && parts.length >= 2) {
                const command = parts[0]
                const field = parts[1]

                if (command !== 'produce') {
                  try {
                    const result = await api.executeExtensionCommand(extensionId, command, {})
                    const resultData = (result as Record<string, unknown>).result ?? result
                    if (field === 'result') return { data: resultData, success: true }
                    if (typeof resultData === 'object' && resultData !== null) {
                      const fieldValue = (resultData as Record<string, unknown>)[field]
                      return { data: fieldValue ?? resultData, success: true }
                    }
                    return { data: resultData, success: true }
                  } catch {
                    const result = await api.queryData({
                      extension_id: extensionId, command, field,
                      start_time: Date.now() - (24 * 60 * 60 * 1000), end_time: Date.now(), limit: 100,
                    })
                    if (result?.data_points?.length > 0) return { data: result.data_points, success: true }
                    return { data: null, success: false }
                  }
                }

                // produce:* format
                const endTime = Date.now()
                const result = await api.queryData({
                  extension_id: extensionId, command, field,
                  start_time: endTime - (24 * 60 * 60 * 1000), end_time: endTime, limit: 100,
                })
                if (result?.data_points?.length > 0) return { data: result.data_points, success: true }
                return { data: null, success: false }
              } else {
                return { data: null, success: false }
              }
            } catch {
              return { data: null, success: false }
            }
          })
        )

        // Cache successful results
        extensionDataSources.forEach((ds, i) => {
          if (ds.extensionId && ds.extensionMetric && results[i]?.success) {
            extensionDataCache.set(`${ds.extensionId}|${ds.extensionMetric}`, results[i].data)
          }
        })

        let finalData: unknown
        if (results.length > 1) {
          finalData = results.map((r) => r.data)
        } else {
          finalData = results[0]?.data ?? null
        }

        // Wrap scalar values into time-series array format for consistent event merging
        // This ensures useExtensionEventProcessing always works with arrays
        if (finalData !== null && finalData !== undefined && !Array.isArray(finalData)) {
          const now = Math.floor(Date.now() / 1000)
          finalData = [{ timestamp: now, time: now, value: finalData }]
        }

        const transformedData = transform ? transform(finalData) : finalData
        setData(transformedData)
        setLastUpdate(Date.now())
        initialFetchDoneRef.current = true
      } catch (err) {
        logError(err, { operation: 'Fetch extension data' })
        setError(err instanceof Error ? err.message : 'Failed to fetch extension data')
        initialFetchDoneRef.current = true
      } finally {
        setLoading(false)
      }
    }

    if (extensionIntervalRef.current) { clearInterval(extensionIntervalRef.current); extensionIntervalRef.current = null }
    fetchExtensionData()

    const refreshIntervals = extensionDataSources.map((ds) => ds.refresh).filter(Boolean) as number[]
    const minRefresh = refreshIntervals.length > 0 ? Math.min(...refreshIntervals) : null
    if (minRefresh) extensionIntervalRef.current = setInterval(fetchExtensionData, minRefresh * 1000)

    return () => { if (extensionIntervalRef.current) { clearInterval(extensionIntervalRef.current); extensionIntervalRef.current = null } }
  }, [extensionKey, enabled])

  return { hasExtensionSource }
}
