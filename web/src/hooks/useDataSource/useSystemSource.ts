/**
 * useSystemSource — handles system stats fetch + periodic refresh.
 * Self-contained: no WebSocket events.
 */

import { useEffect, useRef } from 'react'
import type { DataSource } from '@/types/dashboard'
import { logError } from '@/lib/errors'
import { fetchSystemStats } from './fetch'

export interface SystemSourceState {
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
  sourceAdapters?: {
    startLoading: () => void
    finishLoading: () => void
    failLoading: (error: string) => void
  }
}

export function useSystemSource(
  systemSources: DataSource[],
  systemKey: string,
  enabled: boolean,
  state: SystemSourceState
): void {
  const systemInitialDoneRef = useRef(false)
  const prevSystemKeyRef = useRef('')
  const systemIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const fetchGenerationRef = useRef(0)

  useEffect(() => {
    if (systemSources.length === 0 || !enabled) {
      if (systemIntervalRef.current) { clearInterval(systemIntervalRef.current); systemIntervalRef.current = null }
      return
    }

    // Reset when system config changes
    if (prevSystemKeyRef.current !== systemKey) {
      systemInitialDoneRef.current = false
      prevSystemKeyRef.current = systemKey
    }

    const currentGeneration = ++fetchGenerationRef.current

    const fetchSystemData = async () => {
      // Discard stale invocations
      if (fetchGenerationRef.current !== currentGeneration) return

      if (!systemInitialDoneRef.current) {
        if (state.sourceAdapters) state.sourceAdapters.startLoading()
        else state.setLoading(true)
      }
      state.setError(null)

      try {
        const results = await Promise.all(
          systemSources.map(async (ds) => {
            const metric = ds.systemMetric
            if (!metric) return { data: null }
            const response = await fetchSystemStats(metric)
            return { data: response.data, success: response.success }
          })
        )

        // Discard stale results
        if (fetchGenerationRef.current !== currentGeneration) return

        let finalData: unknown
        if (results.length > 1) finalData = results.map((r) => r.data)
        else finalData = results[0]?.data ?? null

        const { transform: transformFn, fallback: fallbackVal } = state.optionsRef.current
        const transformedData = transformFn ? transformFn(finalData) : finalData
        state.setDataRaw(transformedData)
        state.setLastUpdate(Date.now())
        systemInitialDoneRef.current = true
      } catch (err) {
        if (fetchGenerationRef.current !== currentGeneration) return
        logError(err, { operation: 'Fetch system data' })
        state.setError(err instanceof Error ? err.message : 'Failed to fetch system data')
        state.setDataRaw(state.optionsRef.current.fallback ?? null)
        systemInitialDoneRef.current = true
      } finally {
        if (fetchGenerationRef.current === currentGeneration) {
          if (state.sourceAdapters) state.sourceAdapters.finishLoading()
          else state.setLoading(false)
        }
      }
    }

    if (systemIntervalRef.current) { clearInterval(systemIntervalRef.current); systemIntervalRef.current = null }
    fetchSystemData()

    const refreshIntervals = systemSources.map((ds) => ds.refresh).filter(Boolean) as number[]
    const minRefresh = refreshIntervals.length > 0 ? Math.min(...refreshIntervals) : null
    if (minRefresh) systemIntervalRef.current = setInterval(fetchSystemData, minRefresh * 1000)

    return () => { if (systemIntervalRef.current) { clearInterval(systemIntervalRef.current); systemIntervalRef.current = null } }
  }, [systemKey, enabled])
}
