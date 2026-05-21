/**
 * System stats fetching with caching.
 */

import { systemStatsCache } from './cache'
import { logError } from '@/lib/errors'

/**
 * Fetch system stats for a specific metric.
 * Returns cached data if fresh (30s TTL).
 */
export async function fetchSystemStats(
  metric: string
): Promise<{ data: unknown; success: boolean }> {
  const cacheKey = `system|${metric}`
  const cached = systemStatsCache.get(cacheKey)

  if (cached !== undefined) {
    return { data: cached, success: true }
  }

  try {
    const api = (await import('@/lib/api')).api
    const stats = await api.getSystemStats()

    if (!stats) {
      return { data: null, success: false }
    }

    let value: unknown = null
    switch (metric) {
      case 'uptime':
        value = stats.uptime
        break
      case 'cpu_count':
        value = stats.cpu_count
        break
      case 'total_memory':
        value = stats.total_memory / (1024 * 1024 * 1024)
        break
      case 'used_memory':
        value = stats.used_memory / (1024 * 1024 * 1024)
        break
      case 'free_memory':
        value = stats.free_memory / (1024 * 1024 * 1024)
        break
      case 'available_memory':
        value = stats.available_memory / (1024 * 1024 * 1024)
        break
      case 'memory_percent':
        value = stats.used_memory / stats.total_memory * 100
        break
      case 'platform':
        value = stats.platform
        break
      case 'arch':
        value = stats.arch
        break
      case 'version':
        value = stats.version
        break
      default:
        value = null
    }

    systemStatsCache.set(cacheKey, value)
    return { data: value, success: true }
  } catch (error) {
    logError(error, { operation: 'Fetch system stats' })
    return { data: null, success: false }
  }
}
