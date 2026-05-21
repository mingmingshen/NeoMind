/**
 * React Component Performance Monitor Hook
 *
 * Tracks component lifecycle from mount to data-ready.
 * Useful for identifying slow widget components.
 */

import { useEffect, useRef } from 'react'
import { perfStart, perfEnd, perfCheckpoint } from '@/lib/performance'

// 控制性能日志输出，默认关闭
const ENABLE_PERF_LOGS = typeof localStorage !== 'undefined' && localStorage.getItem('ENABLE_PERF_LOGS') === 'true'

export interface ComponentPerfOptions {
  /** Component name for logging */
  name: string
  /** Custom data-ready check (optional) */
  isDataReady?: () => boolean
  /** Log render cycles (optional, default false) */
  logRenders?: boolean
}

/**
 * Track component performance from mount to data-ready
 *
 * @example
 * const componentRef = useComponentPerf({
 *   name: 'LineChart',
 *   isDataReady: () => !!data && data.length > 0
 * })
 */
export function useComponentPerf({
  name,
  isDataReady,
  logRenders = false
}: ComponentPerfOptions) {
  const renderCount = useRef(0)
  const dataReadyRef = useRef(false)
  const mountedRef = useRef(false)
  const perfKeyRef = useRef(`${name}-Mount`)

  useEffect(() => {
    if (!mountedRef.current) {
      mountedRef.current = true

      // Check if performance mark already exists (prevent duplicates)
      const hasExistingMark = typeof performance !== 'undefined' &&
                                performance.getEntriesByName(`${perfKeyRef.current}-start`, 'mark').length > 0

      if (!hasExistingMark) {
        try {
          perfStart(perfKeyRef.current)
          if (ENABLE_PERF_LOGS) console.log(`📍 [Perf] ${name} mounted`)
        } catch (err) {
          if (ENABLE_PERF_LOGS) console.debug(`[Perf] Could not start tracking for ${name}:`, err)
        }
      }

      // Track time to data ready
      if (isDataReady) {
        const checkInterval = setInterval(() => {
          if (isDataReady()) {
            clearInterval(checkInterval)
            if (!dataReadyRef.current) {
              dataReadyRef.current = true
              try {
                perfEnd(perfKeyRef.current)
                perfCheckpoint(`${name}-DataReady`)
                if (ENABLE_PERF_LOGS) console.log(`✅ [Perf] ${name} data ready`)
              } catch (err) {
                if (ENABLE_PERF_LOGS) console.debug(`[Perf] Could not end tracking for ${name}:`, err)
              }
            }
          }
        }, 100)

        // Timeout after 30 seconds
        const timeout = setTimeout(() => {
          clearInterval(checkInterval)
          if (!dataReadyRef.current) {
            if (ENABLE_PERF_LOGS) console.warn(`⚠️ [Perf] ${name} data not ready after 30s`)
            try {
              perfEnd(perfKeyRef.current)
            } catch (err) {
              if (ENABLE_PERF_LOGS) console.debug(`[Perf] Could not end tracking for ${name}:`, err)
            }
          }
        }, 30000)

        return () => {
          clearInterval(checkInterval)
          clearTimeout(timeout)
        }
      } else {
        // No data ready check, mark ready immediately
        dataReadyRef.current = true
        try {
          perfEnd(perfKeyRef.current)
          if (ENABLE_PERF_LOGS) console.log(`✅ [Perf] ${name} ready (no data)`)
        } catch (err) {
          if (ENABLE_PERF_LOGS) console.debug(`[Perf] Could not end tracking for ${name}:`, err)
        }
      }
    }

    // Track re-renders
    if (logRenders && ENABLE_PERF_LOGS) {
      renderCount.current++
      console.log(`🔄 [Perf] ${name} render #${renderCount.current}`)
    }
  })

  return {
    markDataReady: () => {
      if (!dataReadyRef.current && mountedRef.current) {
        dataReadyRef.current = true
        try {
          perfEnd(perfKeyRef.current)
          perfCheckpoint(`${name}-DataReady`)
          if (ENABLE_PERF_LOGS) console.log(`✅ [Perf] ${name} data ready (manual)`)
        } catch (err) {
          if (ENABLE_PERF_LOGS) console.debug(`[Perf] Could not mark ready for ${name}:`, err)
        }
      }
    }
  }
}
