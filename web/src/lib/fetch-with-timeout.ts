/**
 * HTTP Request with Timeout
 *
 * Adds timeout support to fetch requests to prevent hanging requests.
 * Critical for dashboard performance - prevents 14+ second delays.
 */

export interface FetchWithTimeoutOptions extends RequestInit {
  timeout?: number // milliseconds
  signal?: AbortSignal
}

/**
 * Wrapper around fetch that adds timeout support
 * @param url - URL to fetch
 * @param options - Fetch options with optional timeout
 * @returns Fetch response
 * @throws Error if timeout occurs or request fails
 */
export async function fetchWithTimeout(
  url: string,
  options: FetchWithTimeoutOptions = {}
): Promise<Response> {
  const { timeout = 8000, signal, ...fetchOptions } = options // Default 8 second timeout

  // Create abort controller for timeout
  const controller = new AbortController()
  const timeoutId = setTimeout(() => controller.abort(), timeout)

  // Combine external signal with timeout signal
  const combinedSignal = signal
    ? combineSignals([signal, controller.signal])
    : controller.signal

  try {
    const response = await fetch(url, {
      ...fetchOptions,
      signal: combinedSignal,
    })
    clearTimeout(timeoutId)
    return response
  } catch (error) {
    clearTimeout(timeoutId)

    // Check if error was caused by timeout
    if (error instanceof Error && error.name === 'AbortError') {
      if (controller.signal.aborted) {
        throw new Error(`Request timeout after ${timeout}ms`)
      }
    }

    throw error
  }
}

/**
 * Combines multiple AbortSignals into one
 * Aborts if any of the signals abort
 */
function combineSignals(signals: AbortSignal[]): AbortSignal {
  const controller = new AbortController()

  for (const signal of signals) {
    if (signal.aborted) {
      controller.abort()
      break
    }
    signal.addEventListener('abort', () => controller.abort(), { once: true })
  }

  return controller.signal
}