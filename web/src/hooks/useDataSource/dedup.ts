/**
 * Telemetry point deduplication utilities.
 *
 * Pure functions – no external imports required.
 */

/**
 * Check if a point contains image data (base64, URLs, data: URIs).
 * Inspects 'src'/'url' fields, value fields with base64/URL content,
 * and data fields with long strings.
 */
export function isImageData(p: unknown): boolean {
  if (p == null || typeof p !== 'object') return false
  const o = p as Record<string, unknown>
  // Check for transformed format (src, url)
  if ('src' in o || 'url' in o) return true
  // Check for raw telemetry format (value with base64 or URL)
  if ('value' in o || 'v' in o) {
    const val = (o.value ?? o.v) as string | undefined
    if (typeof val === 'string') {
      const isImage =
        val.startsWith('data:image/') ||
        val.startsWith('data:base64,') ||
        val.startsWith('http') ||
        val.startsWith('/') ||
        val.length > 2000 // Long base64 string (images)
      return isImage
    }
  }
  // Check for data field with long string (base64)
  if ('data' in o && typeof o.data === 'string' && o.data.length > 100) {
    return true
  }
  return false
}

/**
 * Extract the value from a telemetry point.
 * Handles common shapes: `{value}`, `{v}`, `{val}`.
 */
export function getPointValue(p: unknown): unknown {
  if (p == null) return undefined
  const o = p as Record<string, unknown>
  return o.value ?? o.v ?? o.val
}

/**
 * Check if a data point already exists in an array based on
 * (timestamp + value) combination.
 *
 * A point is considered duplicate only if BOTH timestamp and value match an
 * existing point.  This preserves different images at the same timestamp and
 * same images at different timestamps.
 *
 * Timestamps are compared with a 1-second tolerance to account for clock skew.
 * For long content (e.g. base64 images) the comparison uses the first and last
 * 500 characters instead of the full string.
 */
export function isDuplicatePoint(
  existingPoints: unknown[],
  newTimestamp: number,
  newValue: unknown,
  getTs: (p: unknown) => number,
): boolean {
  if (existingPoints.length === 0) return false

  const newStr = typeof newValue === 'string' ? newValue : JSON.stringify(newValue)
  if (!newStr) return false

  // Extract the actual content for comparison (handles data URLs)
  const extractContent = (str: string): string => {
    if (str.startsWith('data:')) {
      const commaIndex = str.indexOf(',')
      if (commaIndex !== -1) {
        return str.slice(commaIndex + 1)
      }
    }
    return str
  }

  const newContent = extractContent(newStr)

  return existingPoints.some((p) => {
    // Check timestamp match first (within 1 second tolerance for clock skew)
    const existingTs = getTs(p)
    const tsDiff = Math.abs(existingTs - newTimestamp)
    if (tsDiff > 1) return false // Different timestamps = not duplicate

    // Same timestamp, now check value
    const existingVal = getPointValue(p)
    if (existingVal === undefined || existingVal === null) return false

    const existingStr = typeof existingVal === 'string' ? existingVal : JSON.stringify(existingVal)
    if (!existingStr) return false

    const existingContent = extractContent(existingStr)

    // For long content (e.g., base64 images), compare first and last 500 chars
    if (newContent.length > 100 && existingContent.length > 100) {
      const newStart = newContent.slice(0, 500)
      const existingStart = existingContent.slice(0, 500)
      if (newStart !== existingStart) return false

      const newEnd = newContent.slice(-500)
      const existingEnd = existingContent.slice(-500)
      return newEnd === existingEnd
    }

    return newContent === existingContent
  })
}

/**
 * Deduplicate telemetry points.
 *
 * - For image data: skip deduplication entirely (each image is unique)
 * - For normal data: only remove exact timestamp duplicates (backend double-write)
 * - Caps to maxLimit for memory management
 */
export function dedupeTelemetryPoints(
  points: unknown[],
  getTs: (p: unknown) => number,
  maxLimit: number,
): unknown[] {
  // Check if this batch contains any image data
  const hasImageData = points.some(isImageData)

  // If all data is images, skip deduplication entirely
  if (hasImageData) {
    return points.slice(0, maxLimit)
  }

  // Normal deduplication: only remove EXACT timestamp duplicates
  const deduped: unknown[] = []
  for (const p of points) {
    const ts = getTs(p)

    // Only check for exact timestamp duplicates (same second)
    const exactDup = deduped.some((k) => {
      const kTs = getTs(k)
      return Math.abs(ts - kTs) < 1
    })
    if (exactDup) continue

    deduped.push(p)
    if (deduped.length >= maxLimit) break
  }
  return deduped
}

/**
 * Detect if a metric is an image-related metric.
 * Used to apply special handling (no dedup, higher limits).
 */
export function isImageMetric(metricId: string | undefined): boolean {
  if (!metricId) return false
  const lower = metricId.toLowerCase()
  return lower.includes('image') || lower.includes('img') || metricId.includes('values.image')
}

/**
 * Detect if a data source is an image data source based on params and metric.
 */
export function isImageDataSource(
  params: { includeRawPoints?: boolean } | undefined,
  transform: string | undefined,
  metricId: string | undefined
): boolean {
  return (params?.includeRawPoints === true || transform === 'raw') ||
         isImageMetric(metricId)
}

/**
 * Get the max limit for a data source (image sources get higher limits).
 */
export function getDataSourceLimit(
  ds: { params?: { includeRawPoints?: boolean }; transform?: string; metricId?: string; limit?: number }
): number {
  const isImage = isImageDataSource(ds.params, ds.transform, ds.metricId)
  return ds.limit ?? (isImage ? 200 : 50)
}
