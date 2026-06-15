import { useState, useEffect, useCallback } from 'react'
import { api } from '@/lib/api'
import { fetchCache } from '@/lib/utils/async'
import { useErrorHandler } from '@/hooks/useErrorHandler'

/**
 * Fetches all transforms once and computes per-device / per-device-type counts.
 * Call this ONCE in the parent list component and pass counts down to TransformsBadge.
 * This replaces the old pattern where each TransformsBadge independently fetched
 * with a shared cache key — which caused all but the first instance to bail out
 * and show count=0.
 */
export function useTransformCounts() {
  const { handleError } = useErrorHandler()
  const [deviceCounts, setDeviceCounts] = useState<Record<string, number>>({})
  const [deviceTypeCounts, setDeviceTypeCounts] = useState<Record<string, number>>({})
  const [loading, setLoading] = useState(true)

  const fetchCounts = useCallback(async () => {
    const cacheKey = 'transforms-list'
    if (!fetchCache.shouldFetch(cacheKey)) return

    fetchCache.markFetching(cacheKey)
    setLoading(true)
    try {
      const result = await api.listTransforms()
      const dMap: Record<string, number> = {}
      const tMap: Record<string, number> = {}

      for (const tr of result.transforms || []) {
        if (typeof tr.scope === 'object' && tr.scope !== null) {
          if ('device' in tr.scope && tr.scope.device) {
            dMap[tr.scope.device] = (dMap[tr.scope.device] || 0) + 1
          } else if ('device_type' in tr.scope && tr.scope.device_type) {
            tMap[tr.scope.device_type] = (tMap[tr.scope.device_type] || 0) + 1
          }
        }
      }

      setDeviceCounts(dMap)
      setDeviceTypeCounts(tMap)
      fetchCache.markFetched(cacheKey)
    } catch (error) {
      fetchCache.invalidate('transforms-list')
      handleError(error, { operation: 'Fetch transform counts', showToast: false })
    } finally {
      setLoading(false)
    }
  }, [handleError])

  useEffect(() => {
    fetchCounts()
  }, [fetchCounts])

  const refresh = useCallback(() => {
    fetchCache.invalidate('transforms-list')
    fetchCounts()
  }, [fetchCounts])

  return { deviceCounts, deviceTypeCounts, loading, refresh }
}
