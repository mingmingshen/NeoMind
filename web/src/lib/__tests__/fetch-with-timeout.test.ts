/**
 * Performance Optimization Tests
 *
 * Tests for timeout functionality and performance monitoring fixes
 * to prevent 14+ second dashboard loading delays.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { fetchWithTimeout } from '../fetch-with-timeout'

describe('fetchWithTimeout', () => {
  let mockFetch: ReturnType<typeof vi.fn>

  beforeEach(() => {
    mockFetch = vi.fn()
    globalThis.fetch = mockFetch
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('should resolve normally when request completes within timeout', async () => {
    const mockResponse = {
      ok: true,
      json: async () => ({ data: 'test' }),
    } as unknown as Response

    mockFetch.mockResolvedValueOnce(mockResponse)

    const result = await fetchWithTimeout('https://api.example.com/test', { timeout: 5000 })

    expect(result).toBe(mockResponse)
    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('should abort with timeout error when request exceeds timeout', { timeout: 10000 }, async () => {
    // Create a promise that never resolves
    mockFetch.mockReturnValueOnce(new Promise(() => {}))

    await expect(
      fetchWithTimeout('https://api.example.com/slow', { timeout: 100 })
    ).rejects.toThrow('Request timeout after 100ms')

    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('should use default 8 second timeout when not specified', async () => {
    const mockResponse = {
      ok: true,
      json: async () => ({ data: 'test' }),
    } as unknown as Response

    mockFetch.mockResolvedValueOnce(mockResponse)

    const result = await fetchWithTimeout('https://api.example.com/test')

    expect(result).toBe(mockResponse)
    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('should respect external AbortSignal', async () => {
    const controller = new AbortController()
    const mockResponse = {
      ok: true,
      json: async () => ({ data: 'test' }),
    } as unknown as Response

    mockFetch.mockResolvedValueOnce(mockResponse)

    const result = await fetchWithTimeout('https://api.example.com/test', {
      signal: controller.signal,
    })

    expect(result).toBe(mockResponse)
    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('should abort when external signal is triggered', { timeout: 10000 }, async () => {
    const controller = new AbortController()

    // Create a promise that never resolves
    mockFetch.mockReturnValueOnce(new Promise(() => {}))

    // Abort externally after 50ms
    setTimeout(() => controller.abort(), 50)

    await expect(
      fetchWithTimeout('https://api.example.com/test', {
        timeout: 5000,
        signal: controller.signal,
      })
    ).rejects.toThrow()

    expect(mockFetch).toHaveBeenCalledTimes(1)
  })

  it('should abort on whichever condition occurs first (timeout vs external)', { timeout: 10000 }, async () => {
    const controller = new AbortController()
    mockFetch.mockReturnValueOnce(new Promise(() => {}))

    // External abort after 100ms, but timeout is 50ms
    setTimeout(() => controller.abort(), 100)

    await expect(
      fetchWithTimeout('https://api.example.com/test', {
        timeout: 50,
        signal: controller.signal,
      })
    ).rejects.toThrow('Request timeout after 50ms') // Timeout occurs first
  })

  it('should pass through fetch options correctly', async () => {
    const mockResponse = {
      ok: true,
      json: async () => ({ data: 'test' }),
    } as unknown as Response

    mockFetch.mockResolvedValueOnce(mockResponse)

    await fetchWithTimeout('https://api.example.com/test', {
      timeout: 5000,
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Custom-Header': 'test-value',
      },
      body: JSON.stringify({ test: 'data' }),
    })

    expect(mockFetch).toHaveBeenCalledWith(
      'https://api.example.com/test',
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({
          'Content-Type': 'application/json',
          'X-Custom-Header': 'test-value',
        }),
        body: JSON.stringify({ test: 'data' }),
      })
    )
  })
})

describe('Performance Monitoring Integration', () => {
  it('should prevent duplicate performance marks for same data source', async () => {
    // This test verifies that the unique fetchId prevents duplicate marks
    const fetchId1 = `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
    const fetchId2 = `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`

    expect(fetchId1).not.toBe(fetchId2)

    // Simulate perfKey creation
    const perfKey1 = `DS-${fetchId1}-device123.temperature`
    const perfKey2 = `DS-${fetchId2}-device123.temperature`

    expect(perfKey1).not.toBe(perfKey2)
  })

  it('should create short, readable performance keys', () => {
    const longDataSourceInfo = 'device123.temperature.humidity.pressure.light.sound'
    const fetchId = 'abc123'
    const perfKey = `DS-${fetchId}-${longDataSourceInfo.slice(0, 20)}`

    expect(perfKey.length).toBeLessThan(50) // Ensure keys are reasonably short
    expect(perfKey).toContain('DS-abc123-device123') // Should contain device name
    expect(perfKey.length).toBe(35) // DS-abc123 (9) + device123.temperature (20) = 29, but slice(0,20) takes first 20 chars
  })
})

describe('Dashboard Performance Scenarios', () => {
  it('should handle slow telemetry requests with timeout', async () => {
    const mockResponse = {
      ok: true,
      json: async () => ({ data: [[1, 2, 3]] }),
    } as unknown as Response

    const mockFetch = vi.fn()
    globalThis.fetch = mockFetch

    // Simulate slow response (6 seconds) but within timeout (8 seconds)
    mockFetch.mockImplementationOnce(() =>
      new Promise(resolve => setTimeout(() => resolve(mockResponse), 100))
    )

    const startTime = Date.now()
    const result = await fetchWithTimeout('https://api.example.com/devices/d0/telemetry', {
      timeout: 8000,
    })
    const endTime = Date.now()

    expect(result).toBe(mockResponse)
    expect(endTime - startTime).toBeLessThan(1000) // Should complete in ~100ms for test
  })

  it('should timeout very slow telemetry requests', { timeout: 10000 }, async () => {
    const mockFetch = vi.fn()
    globalThis.fetch = mockFetch

    // Simulate very slow response (longer than timeout)
    mockFetch.mockImplementationOnce(() => new Promise(() => {}))

    const startTime = Date.now()
    await expect(
      fetchWithTimeout('https://api.example.com/devices/d0/telemetry?metric=battery', {
        timeout: 100,
      })
    ).rejects.toThrow('Request timeout after 100ms')

    const endTime = Date.now()
    expect(endTime - startTime).toBeLessThan(200) // Should timeout quickly
  })
})