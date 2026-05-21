// ============================================================================
// Data Extraction Utilities
// ============================================================================

/**
 * Safely extract a value from unknown data
 */
export function safeExtractValue(data: unknown, fallback: number | string | boolean = 0): unknown {
  if (data === null || data === undefined) return fallback
  const type = typeof data

  if (type === 'string' || type === 'number' || type === 'boolean') return data

  if (typeof data === 'object' && data !== null) {
    if ('value' in data) {
      return safeExtractValue((data as { value: unknown }).value, fallback)
    }
    return data
  }

  return fallback
}

/**
 * Find property value with various naming conventions
 */
export function findPropertyValue(obj: Record<string, unknown>, property: string): unknown {
  if (property in obj) return obj[property]

  const lowerProp = property.toLowerCase()
  for (const key of Object.keys(obj)) {
    if (key.toLowerCase() === lowerProp) return obj[key]
  }

  // Common aliases
  const aliases: Record<string, string[]> = {
    temperature: ['temperature', 'temp', 'value', 'temp_c', 'tempC'],
    humidity: ['humidity', 'hum', 'rh', 'relative_humidity'],
    status: ['status', 'state', 'connection_status', 'online'],
    value: ['value', 'val', 'current', 'presentValue', 'pv'],
  }

  for (const [key, aliasList] of Object.entries(aliases)) {
    if (lowerProp === key || lowerProp === key.slice(0, -1)) {
      for (const alias of aliasList) {
        if (alias in obj) return obj[alias]
      }
    }
  }

  return undefined
}

/**
 * Transform-generated metric namespaces (same as backend).
 * Metrics starting with these prefixes are virtual/computed metrics from Transforms.
 */
export const TRANSFORM_NAMESPACES = ['transform.', 'virtual.', 'computed.', 'derived.', 'aggregated.']

/**
 * Check if a metric name is a transform-generated virtual metric.
 */
export function isVirtualMetric(metricId: string): boolean {
  return TRANSFORM_NAMESPACES.some(ns => metricId.startsWith(ns))
}

/**
 * Check if event metric matches widget metricId (supports nested paths like values.image vs image).
 * This handles both directions:
 * - Event "values.image" matches widget "image" (event is nested, widget is simple)
 * - Event "image" matches widget "values.image" (event is simple, widget is nested)
 *
 * IMPORTANT: Transform-generated virtual metrics (e.g., "transform.temperature") should NOT
 * match real metrics (e.g., "temperature"). This prevents dashboard widgets bound to real
 * metrics from incorrectly displaying virtual metric values.
 */
export function eventMetricMatches(eventMetric: string, targetMetric: string): boolean {
  if (!eventMetric || !targetMetric) return false
  if (eventMetric === targetMetric) return true

  // CRITICAL: If event is a virtual metric but widget is not (or vice versa), don't match
  // This prevents "transform.temperature" from matching "temperature"
  const eventIsVirtual = isVirtualMetric(eventMetric)
  const widgetIsVirtual = isVirtualMetric(targetMetric)
  if (eventIsVirtual !== widgetIsVirtual) {
    return false
  }

  // Case 1: Event has nested path, widget is simple
  // e.g., event "values.image" matches widget "image"
  if (eventMetric.endsWith('.' + targetMetric)) return true
  if (eventMetric.endsWith('/' + targetMetric)) return true

  // Case 2: Event is simple, widget has nested path
  // e.g., event "image" matches widget "values.image"
  if (targetMetric.endsWith('.' + eventMetric)) return true
  if (targetMetric.endsWith('/' + eventMetric)) return true

  // NOTE: We intentionally do NOT match based on last segment alone.
  // Matching "foo.image" with "bar.image" is too fuzzy and causes cross-metric
  // interference — images from one metric bleed into another metric's widget.
  // Only Cases 1 and 2 above (direct prefix/suffix relationship) are allowed.

  return false
}

/**
 * Extract value from a parsed JSON object using dot notation.
 * Helper for extractValueFromData to handle _raw events.
 */
export function extractValueFromParsed(parsed: unknown, property: string): unknown {
  if (parsed === null || parsed === undefined) return undefined
  if (typeof parsed !== 'object') return parsed

  const dataObj = parsed as Record<string, unknown>

  // Direct key match
  if (property in dataObj) return dataObj[property]

  // Dot notation for nested paths
  if (property.includes('.')) {
    const parts = property.split('.')
    let current: unknown = dataObj

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i]
      if (typeof current === 'object' && current !== null && part in current) {
        current = (current as Record<string, unknown>)[part]
        if (i === parts.length - 1 || typeof current !== 'object') return current
      } else {
        return undefined
      }
    }
    return current
  }

  // Case-insensitive match
  const lowerProp = property.toLowerCase()
  for (const key of Object.keys(dataObj)) {
    if (key.toLowerCase() === lowerProp) return dataObj[key]
  }

  return undefined
}

/**
 * Extract value from nested object using dot notation.
 * Dotted keys like "xx.ss.xx" are tried as a single string key first, then as nested path.
 *
 * Special handling for "_raw" events: if data.value is a JSON string, parse it first.
 */
export function extractValueFromData(rawData: string | unknown, property: string): unknown {
  if (rawData === null || rawData === undefined) return undefined

  // Handle strings directly (return as-is for base64 images, etc.)
  if (typeof rawData !== 'object') return rawData

  const dataObj = rawData as Record<string, unknown>

  // Special handling for "_raw" style events where value is a JSON string
  if ('value' in dataObj && typeof dataObj.value === 'string' && 'metric' in dataObj) {
    const metric = dataObj.metric as string
    // If this is a "_raw" metric event, try to parse the JSON value
    if (metric === '_raw' || (dataObj.value as string).trim().startsWith('{')) {
      try {
        const parsed = JSON.parse(dataObj.value as string)
        // Extract from the parsed JSON
        const extracted = extractValueFromParsed(parsed, property)
        if (extracted !== undefined) return extracted
      } catch {
        // Not valid JSON, continue with normal extraction
      }
    }
  }

  // Prefer direct access so "xx.ss.xx" is treated as a single string key when present
  if (property in dataObj) return dataObj[property]

  // Dot notation for nested paths (e.g. values.image or obj.a.b)
  if (property.includes('.')) {
    const parts = property.split('.')
    let current: unknown = dataObj

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i]
      if (typeof current === 'object' && current !== null && part in current) {
        current = (current as Record<string, unknown>)[part]
        if (i === parts.length - 1 || typeof current !== 'object') return current
      } else {
        // Try flexible matching
        if (typeof current === 'object' && current !== null) {
          const found = findPropertyValue(current as Record<string, unknown>, part)
          if (found !== undefined) {
            current = found
            if (i === parts.length - 1 || typeof current !== 'object') return current
          }
        }
        return undefined
      }
    }
    return current
  }

  // Flexible matching
  const found = findPropertyValue(dataObj, property)
  if (found !== undefined) return found

  // Try nested in common properties
  for (const nestedProp of ['current_values', 'currentValues', 'metrics', 'data', 'values', 'device_info', 'deviceInfo']) {
    if (nestedProp in dataObj && typeof dataObj[nestedProp] === 'object') {
      const nested = dataObj[nestedProp] as Record<string, unknown>
      if (property.includes('.')) {
        const remainingParts = property.split('.')
        if (remainingParts[0].toLowerCase() === nestedProp.toLowerCase()) {
          return extractValueFromData(nested, remainingParts.slice(1).join('.'))
        }
      }
      const nestedValue = findPropertyValue(nested, property)
      if (nestedValue !== undefined) return nestedValue
    }
  }

  return undefined
}

/**
 * Shallow comparison for current_values objects
 * Returns true if values are different, false if they're the same
 * Much faster than JSON.stringify for large objects
 */
export function hasCurrentValuesChanged(
  prev: Record<string, unknown> | undefined,
  next: Record<string, unknown> | undefined,
  useDeepComparison: boolean = false
): boolean {
  // Quick reference check - most common case
  if (prev === next) return false

  // Both are undefined/null
  if (!prev && !next) return false

  // One is undefined/null, other isn't
  if (!prev || !next) return true

  // Check key count first (fast rejection without allocations)
  const prevKeys = Object.keys(prev)
  const nextKeys = Object.keys(next)
  if (prevKeys.length !== nextKeys.length) return true

  // Check each key's value reference
  for (let i = 0; i < prevKeys.length; i++) {
    const key = prevKeys[i]
    const prevValue = prev[key]
    const nextValue = next[key]

    // For primitive values (number, string, boolean), shallow comparison is sufficient
    if (typeof prevValue !== 'object' || prevValue === null) {
      if (prevValue !== nextValue) {
        return true
      }
    } else {
      // For object values, use deep comparison if requested
      // This handles nested objects like { values: { temperature: 25 } }
      if (useDeepComparison) {
        try {
          if (JSON.stringify(prevValue) !== JSON.stringify(nextValue)) {
            return true
          }
        } catch {
          // Fallback for circular references or non-serializable values
          return false
        }
      } else {
        // Shallow comparison for objects (reference check only)
        if (prevValue !== nextValue) {
          return true
        }
      }
    }
  }

  return false
}
