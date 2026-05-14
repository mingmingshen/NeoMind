/**
 * Create a stable string key from any value for memoization/comparison purposes.
 * Handles objects with keys in sorted order to ensure stability.
 */
export function createStableKey(obj: unknown): string {
  if (obj === null || obj === undefined) return ''
  if (typeof obj !== 'object') return String(obj)
  if (Array.isArray(obj)) return '[' + obj.map(createStableKey).join(',') + ']'
  const sortedKeys = Object.keys(obj as Record<string, unknown>).sort()
  return '{' + sortedKeys.map(k => `"${k}":${createStableKey((obj as Record<string, unknown>)[k])}`).join(',') + '}'
}
