/**
 * Generic typed cache with TTL and size limits.
 * Supports optional per-entry metadata (e.g. refresh scheduling).
 */
export class TypedCache<T, M = void> {
  private cache = new Map<string, { data: T; timestamp: number; meta?: M }>()

  constructor(
    private ttl: number,
    private maxSize: number
  ) {}

  get(key: string): T | undefined {
    const entry = this.cache.get(key)
    if (!entry) return undefined
    if (Date.now() - entry.timestamp > this.ttl) {
      this.cache.delete(key)
      return undefined
    }
    return entry.data
  }

  /** Get data along with metadata. */
  getWithMeta(key: string): { data: T; meta?: M } | undefined {
    const entry = this.cache.get(key)
    if (!entry) return undefined
    if (Date.now() - entry.timestamp > this.ttl) {
      this.cache.delete(key)
      return undefined
    }
    return { data: entry.data, meta: entry.meta }
  }

  set(key: string, data: T, meta?: M): void {
    if (this.cache.size >= this.maxSize) {
      const oldestKey = this.cache.keys().next().value
      if (oldestKey) this.cache.delete(oldestKey)
    }
    this.cache.set(key, { data, timestamp: Date.now(), meta })
  }

  /** Update metadata for an existing entry. Returns true if entry existed. */
  updateMeta(key: string, meta: M): boolean {
    const entry = this.cache.get(key)
    if (!entry) return false
    entry.meta = meta
    return true
  }

  /** Delete a specific entry. */
  delete(key: string): boolean {
    return this.cache.delete(key)
  }

  /** Iterate entries that match a predicate (for batch operations). */
  forEach(predicate: (data: T, meta: M | undefined, key: string) => void): void {
    for (const [key, entry] of this.cache) {
      predicate(entry.data, entry.meta, key)
    }
  }

  /** Delete entries matching a predicate. */
  deleteWhere(predicate: (meta: M | undefined, key: string) => boolean): void {
    for (const [key, entry] of this.cache) {
      if (predicate(entry.meta, key)) {
        this.cache.delete(key)
      }
    }
  }

  cleanup(): void {
    const now = Date.now()
    for (const [key, entry] of this.cache) {
      if (now - entry.timestamp > this.ttl) {
        this.cache.delete(key)
      }
    }
  }
}

/**
 * Cache for historical telemetry data.
 * Metadata tracks scheduled refreshes for real-time coalescing.
 */
export interface TelemetryRefreshMeta {
  refreshing?: boolean
  refreshAfter?: number
}

export interface TelemetryCacheData {
  data: number[]
  raw?: unknown[]
}

export const telemetryCache = new TypedCache<TelemetryCacheData, TelemetryRefreshMeta>(30000, 100)

/**
 * Cache for system stats data
 */
export const systemStatsCache = new TypedCache<unknown>(30000, 40)

/**
 * Cache for extension data
 */
export const extensionDataCache = new TypedCache<unknown>(30000, 100)

/**
 * Clear expired cache entries across all caches.
 */
export function cleanupTelemetryCache() {
  telemetryCache.cleanup()
  systemStatsCache.cleanup()
  extensionDataCache.cleanup()
}

// Periodic cache cleanup - store reference for cleanup
let cacheCleanupInterval: ReturnType<typeof setInterval> | null = null
if (typeof window !== 'undefined') {
  cacheCleanupInterval = setInterval(cleanupTelemetryCache, 60000) // Clean up every minute
  // Auto-cleanup on page unload (prevents HMR interval accumulation in dev)
  window.addEventListener('beforeunload', () => {
    if (cacheCleanupInterval) {
      clearInterval(cacheCleanupInterval)
      cacheCleanupInterval = null
    }
  })
}

/**
 * Clear all global cache intervals (call on app unmount)
 */
export function clearGlobalCacheIntervals() {
  if (cacheCleanupInterval) {
    clearInterval(cacheCleanupInterval)
    cacheCleanupInterval = null
  }
}
