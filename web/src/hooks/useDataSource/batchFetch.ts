/**
 * Batch-Aware Device Fetch Collector
 *
 * Collects device IDs across the same microtask and fires a single
 * POST /devices/current-batch instead of N individual GET requests.
 */

import { useStore } from '@/store'

// ============================================================================
// Types
// ============================================================================

export interface BatchFetchResult {
  success: boolean
  metricsCount: number
}

// ============================================================================
// Module State
// ============================================================================

/** Tracks which devices have already been fetched successfully. */
export const fetchedDevices = new Set<string>()

/** In-flight promises per device (deduplication). */
const activeFetches = new Map<string, Promise<BatchFetchResult>>()

/** Devices queued in the current microtask. */
let pendingDeviceIds = new Set<string>()

/** Per-device resolver arrays for pending promises. */
let pendingResolvers = new Map<string, Array<(result: BatchFetchResult) => void>>()

/** Whether a microtask flush has been scheduled. */
let batchScheduled = false

// ============================================================================
// Internal Helpers
// ============================================================================

/**
 * Write batch results into the store and resolve pending promises.
 */
function applyBatchResults(
  results: Record<string, unknown>,
  deviceIds: string[],
  resolvers: Map<string, Array<(result: BatchFetchResult) => void>>
) {
  const store = useStore.getState()

  for (const id of deviceIds) {
    const entry = results[id] as { metrics?: Record<string, { value?: unknown }> } | undefined
    let metricsCount = 0

    if (entry?.metrics) {
      Object.entries(entry.metrics).forEach(([metricName, metricData]) => {
        if (metricData.value !== null && metricData.value !== undefined) {
          store.updateDeviceMetric(id, metricName, metricData.value)
          metricsCount++
        }
      })
    }

    if (metricsCount > 0) {
      fetchedDevices.add(id)
      // Cap the set at 200 entries to avoid unbounded memory growth
      if (fetchedDevices.size > 200) {
        const first = fetchedDevices.values().next().value
        if (first) fetchedDevices.delete(first)
      }
    }

    const result: BatchFetchResult = { success: metricsCount > 0, metricsCount }
    activeFetches.delete(id)
    resolvers.get(id)?.forEach(resolve => resolve(result))
    resolvers.delete(id)
  }
}

/**
 * Drain the pending queue — fire a single batch request.
 */
async function flushBatch() {
  const ids = Array.from(pendingDeviceIds)
  const resolvers = pendingResolvers
  pendingDeviceIds = new Set()
  pendingResolvers = new Map()
  batchScheduled = false

  if (ids.length === 0) return

  try {
    const api = (await import('@/lib/api')).api
    const batchResult = await api.getDevicesCurrentBatch(ids)

    if (batchResult?.devices && typeof batchResult.devices === 'object') {
      applyBatchResults(batchResult.devices as Record<string, unknown>, ids, resolvers)
    } else {
      // Batch succeeded but returned unexpected shape — resolve all with failure
      for (const id of ids) {
        activeFetches.delete(id)
        resolvers.get(id)?.forEach(r => r({ success: false, metricsCount: 0 }))
        resolvers.delete(id)
      }
    }
  } catch {
    // Batch endpoint failed — fall back to individual fetches in chunks
    try {
      const api = (await import('@/lib/api')).api
      const CHUNK_SIZE = 5
      const individualResults: Array<PromiseSettledResult<{ id: string } & BatchFetchResult>> = []

      for (let i = 0; i < ids.length; i += CHUNK_SIZE) {
        const chunk = ids.slice(i, i + CHUNK_SIZE)
        const chunkResults = await Promise.allSettled(
          chunk.map(async (id) => {
            try {
              const details = await api.getDeviceCurrent(id)
              const store = useStore.getState()
              let metricsCount = 0
              if (details?.metrics) {
                Object.entries(details.metrics).forEach(([metricName, metricData]: [string, unknown]) => {
                  const value = (metricData as { value?: unknown }).value
                  if (value !== null && value !== undefined) {
                    store.updateDeviceMetric(id, metricName, value)
                    metricsCount++
                  }
                })
              }
              if (metricsCount > 0) fetchedDevices.add(id)
              return { id, success: metricsCount > 0, metricsCount }
            } catch {
              return { id, success: false, metricsCount: 0 }
            }
          })
        )
        individualResults.push(...chunkResults)
      }
      for (let i = 0; i < ids.length; i++) {
        const id = ids[i]
        const settled = individualResults[i]
        const result: BatchFetchResult = settled?.status === 'fulfilled'
          ? settled.value
          : { success: false, metricsCount: 0 }
        activeFetches.delete(id)
        resolvers.get(id)?.forEach(r => r(result))
        resolvers.delete(id)
      }
    } catch {
      // Complete failure — resolve all with failure
      for (const id of ids) {
        activeFetches.delete(id)
        resolvers.get(id)?.forEach(r => r({ success: false, metricsCount: 0 }))
        resolvers.delete(id)
      }
    }
  }
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Fetch device telemetry — uses batch-aware collector.
 *
 * Adds the device ID to a pending queue and schedules a microtask flush.
 * All calls within the same microtask are batched into a single
 * `POST /devices/current-batch` request.
 *
 * @param deviceId - The device to fetch telemetry for
 * @returns Promise that resolves when the device data has been written to the store
 */
export async function fetchDeviceTelemetry(deviceId: string): Promise<BatchFetchResult> {
  // Already has an in-flight fetch — reuse the promise
  const existing = activeFetches.get(deviceId)
  if (existing) return existing

  const promise = new Promise<BatchFetchResult>((resolve) => {
    pendingResolvers.set(deviceId, [...(pendingResolvers.get(deviceId) ?? []), resolve])
  })

  activeFetches.set(deviceId, promise)
  pendingDeviceIds.add(deviceId)

  if (!batchScheduled) {
    batchScheduled = true
    queueMicrotask(flushBatch)
  }

  return promise
}

/**
 * Check if a device has an active in-flight fetch.
 */
export function hasActiveFetch(deviceId: string): boolean {
  return activeFetches.has(deviceId)
}
