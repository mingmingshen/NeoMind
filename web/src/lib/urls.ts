/**
 * Dynamic URL management for multi-backend instance switching.
 *
 * Provides API key management and WebSocket URL construction.
 * The main getApiBase() / setApiBase() live in api.ts for backward compatibility.
 */

// API key state — separate from URL state
let _currentApiKey = ''

/**
 * Set the API key to use for all requests (remote instance auth).
 */
export function setApiKey(key: string): void {
  _currentApiKey = key
}

/**
 * Clear the API key (switch back to local instance).
 */
export function clearApiKey(): void {
  _currentApiKey = ''
}

/**
 * Get the current API key (empty string if none).
 */
export function getApiKey(): string {
  return _currentApiKey
}

/**
 * Construct a WebSocket URL from a server origin and path.
 * Handles http→ws and https→wss conversion.
 */
export function buildWsUrl(serverOrigin: string, path: string): string {
  const wsOrigin = serverOrigin
    .replace(/^http:/, 'ws:')
    .replace(/^https:/, 'wss:')
  return `${wsOrigin}${path}`
}

/**
 * Reset to default (local) instance: clear API key.
 * URL reset is done via setApiBase('') in api.ts.
 */
export function resetToDefault(): void {
  _currentApiKey = ''
}
