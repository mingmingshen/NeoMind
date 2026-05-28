/**
 * Performance Monitoring Utilities
 *
 * Helper functions for tracking component render times and operations.
 *
 * NOTE: Console logging is DISABLED by default to prevent main thread blocking.
 * Set ENABLE_PERF_LOGS=true in localStorage to enable for debugging.
 */

import { useEffect, useRef, useCallback } from 'react'

type PerformanceMark = string

// Check if performance logging is enabled (for debugging only)
const ENABLE_PERF_LOGS = typeof localStorage !== 'undefined' && localStorage.getItem('ENABLE_PERF_LOGS') === 'true'

/**
 * Start a performance measurement
 * @param name - Unique identifier for this measurement
 */
export function perfStart(name: string): void {
  if (typeof performance !== 'undefined') {
    performance.mark(`${name}-start`)
    if (ENABLE_PERF_LOGS) {
      console.log(`⏱️ [Perf] ${name} - START`, performance.now())
    }
  }
}

/**
 * End a performance measurement and log the duration
 * @param name - Unique identifier for this measurement (must match perfStart call)
 */
export function perfEnd(name: string): void {
  if (typeof performance !== 'undefined') {
    performance.mark(`${name}-end`)
    performance.measure(name, `${name}-start`, `${name}-end`)

    const measures = performance.getEntriesByName(name, 'measure')
    if (measures.length > 0) {
      const duration = measures[0].duration
      const durationStr = duration > 1000 ? `${(duration / 1000).toFixed(2)}s` : `${duration.toFixed(0)}ms`

      // Only log slow operations (>500ms) or if explicitly enabled
      if (ENABLE_PERF_LOGS || duration > 500) {
        console.log(`✅ [Perf] ${name} - ${durationStr}`)
      }

      // Clean up marks to avoid memory leaks
      performance.clearMarks(`${name}-start`)
      performance.clearMarks(`${name}-end`)
      performance.clearMeasures(name)
    }
  }
}

/**
 * Log a performance checkpoint (without ending the measurement)
 * @param name - Checkpoint name
 * @param markName - The mark name to measure against (optional, defaults to 'initial')
 */
export function perfCheckpoint(name: string, markName: string = 'initial'): void {
  if (typeof performance !== 'undefined') {
    const now = performance.now()
    performance.mark(`${name}-checkpoint`)

    const startMark = `${markName}-start`
    if (performance.getEntriesByName(startMark, 'mark').length > 0) {
      performance.measure(`${name} from ${markName}`, startMark, `${name}-checkpoint`)
      const measures = performance.getEntriesByName(`${name} from ${markName}`, 'measure')
      if (measures.length > 0) {
        const duration = measures[0].duration
        const durationStr = duration > 1000 ? `${(duration / 1000).toFixed(2)}s` : `${duration.toFixed(0)}ms`

        // Only log slow checkpoints (>300ms) or if explicitly enabled
        if (ENABLE_PERF_LOGS || duration > 300) {
          console.log(`📍 [Perf] ${name} - ${durationStr}`)
        }

        performance.clearMeasures(`${name} from ${markName}`)
      }
    }

    performance.clearMarks(`${name}-checkpoint`)
  }
}

/**
 * React Hook for tracking component render performance
 * @param componentName - Name of the component being tracked
 */
export function usePerfRender(componentName: string): void {
  const renderCount = useRef(0)

  useEffect(() => {
    renderCount.current++
    perfStart(`${componentName}-render`)

    return () => {
      perfEnd(`${componentName}-render`)
      // Only log excessive re-renders (>5) or if explicitly enabled
      if (ENABLE_PERF_LOGS && renderCount.current > 5) {
        console.warn(`🔄 [Perf] ${componentName} re-rendered (count: ${renderCount.current})`)
      }
    }
  })
}

/**
 * React Hook for tracking async operation performance
 * @param name - Operation name
 * @param asyncFn - The async function to track
 */
export function usePerfAsync<T>(
  name: string,
  asyncFn: () => Promise<T>
): () => Promise<T> {
  return useCallback(() => {
    perfStart(name)
    return asyncFn().finally(() => {
      perfEnd(name)
    })
  }, [name, asyncFn])
}

/**
 * Get all performance entries as a summary
 */
export function getPerfSummary(): void {
  if (typeof performance !== 'undefined' && ENABLE_PERF_LOGS) {
    console.log('📊 [Perf] === Performance Summary ===')

    const measures = performance.getEntriesByType('measure')
    const summary = new Map<string, number[]>()

    measures.forEach(measure => {
      const name = measure.name
      if (!summary.has(name)) {
        summary.set(name, [])
      }
      summary.get(name)!.push(measure.duration)
    })

    summary.forEach((durations, name) => {
      const avg = durations.reduce((a, b) => a + b, 0) / durations.length
      const max = durations.reduce((a, b) => Math.max(a, b), -Infinity)
      const min = durations.reduce((a, b) => Math.min(a, b), Infinity)
      const count = durations.length

      const avgStr = avg > 1000 ? `${(avg / 1000).toFixed(2)}s` : `${avg.toFixed(0)}ms`

      console.log(`  ${name}: avg=${avgStr}, count=${count}${count > 1 ? `, min=${min.toFixed(0)}ms, max=${max.toFixed(0)}ms` : ''}`)
    })
  }
}
