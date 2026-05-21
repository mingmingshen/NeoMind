/**
 * useSystemFetching — System stats data fetching effect for useDataSource.
 *
 * Handles initial fetch and periodic refresh for system-type data sources.
 */

import { useEffect, useRef, useMemo } from 'react'
import type { DataSource } from '@/types/dashboard'
import { createStableKey } from '@/lib/stable-key'
import { logError } from '@/lib/errors'
import { fetchSystemStats } from './systemFetch'

interface UseSystemFetchingOptions {
  dataSources: DataSource[]
  enabled: boolean
  transform: ((data: unknown) => unknown) | undefined
  fallback: unknown
  setData: (data: unknown) => void
  setLoading: (loading: boolean) => void
  setError: (error: string | null) => void
  setLastUpdate: (ts: number) => void
}

export function useSystemFetching({
  dataSources,
  enabled,
  transform,
  fallback,
  setData,
  setLoading,
  setError,
  setLastUpdate,
}: UseSystemFetchingOptions) {
  const systemIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const initialFetchDoneRef = useRef(false)

  const systemDataSources = useMemo(() => {
    return dataSources.filter((ds) => ds.type === 'system')
  }, [dataSources])

  const hasSystemSource = systemDataSources.length > 0

  const systemKey = useMemo(() => {
    return systemDataSources
      .map((ds) => createStableKey({ systemMetric: ds.systemMetric }))
      .join('|')
  }, [systemDataSources])

  useEffect(() => {
    if (!hasSystemSource || !enabled) {
      if (systemIntervalRef.current) {
        clearInterval(systemIntervalRef.current)
        systemIntervalRef.current = null
      }
      return
    }

    const fetchSystemData = async () => {
      if (!initialFetchDoneRef.current) setLoading(true)
      setError(null)

      try {
        const results = await Promise.all(
          systemDataSources.map(async (ds) => {
            const metric = ds.systemMetric
            if (!metric) return { data: null }
            const response = await fetchSystemStats(metric)
            return { data: response.data, success: response.success }
          })
        )

        let finalData: unknown
        if (results.length > 1) {
          finalData = results.map((r) => r.data)
        } else {
          finalData = results[0]?.data ?? null
        }

        const transformedData = transform ? transform(finalData) : finalData
        setData(transformedData)
        setLastUpdate(Date.now())
        initialFetchDoneRef.current = true
      } catch (err) {
        logError(err, { operation: 'Fetch system data' })
        setError(err instanceof Error ? err.message : 'Failed to fetch system data')
        setData(fallback)
        initialFetchDoneRef.current = true
      } finally {
        setLoading(false)
      }
    }

    if (systemIntervalRef.current) { clearInterval(systemIntervalRef.current); systemIntervalRef.current = null }
    fetchSystemData()

    const refreshIntervals = systemDataSources.map((ds) => ds.refresh).filter(Boolean) as number[]
    const minRefresh = refreshIntervals.length > 0 ? Math.min(...refreshIntervals) : null
    if (minRefresh) systemIntervalRef.current = setInterval(fetchSystemData, minRefresh * 1000)

    return () => { if (systemIntervalRef.current) { clearInterval(systemIntervalRef.current); systemIntervalRef.current = null } }
  }, [systemKey, enabled])

  return { hasSystemSource }
}
