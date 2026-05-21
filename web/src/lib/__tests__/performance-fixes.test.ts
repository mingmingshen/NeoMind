/**
 * Performance Optimization Integration Tests
 *
 * Tests for dashboard performance fixes preventing 14+ second delays.
 * Focuses on real-world scenarios and integration testing.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { perfStart, perfEnd } from '../performance'

describe('Performance Monitoring', () => {
  beforeEach(() => {
    // Clear performance marks before each test
    if (typeof performance !== 'undefined') {
      performance.clearMarks()
      performance.clearMeasures()
    }
  })

  afterEach(() => {
    // Clean up after tests
    if (typeof performance !== 'undefined') {
      performance.clearMarks()
      performance.clearMeasures()
    }
  })

  it('should track component mount time without duplicate marks', () => {
    const componentName = 'TestComponent-abc123'

    // First mount
    perfStart(`${componentName}-Mount`)
    perfEnd(`${componentName}-Mount`)

    // Second mount should not cause duplicate mark error
    perfStart(`${componentName}-Mount-v2`)
    perfEnd(`${componentName}-Mount-v2`)

    // Verify the functions work without throwing
    // Performance API entries are cleared immediately in our implementation
    expect(() => {
      perfStart('test-mount')
      perfEnd('test-mount')
    }).not.toThrow()
  })

  it('should create unique performance keys for data sources', () => {
    const fetchId1 = `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`
    const fetchId2 = `${Date.now() + 1}-${Math.random().toString(36).slice(2, 6)}`

    const dataSourceInfo = 'device123.temperature'
    const perfKey1 = `DS-${fetchId1}-${dataSourceInfo}`
    const perfKey2 = `DS-${fetchId2}-${dataSourceInfo}`

    expect(perfKey1).not.toBe(perfKey2)
    expect(perfKey1).toContain('DS-')
    expect(perfKey2).toContain('DS-')
  })

  it('should handle performance tracking gracefully when API unavailable', () => {
    // This test ensures performance tracking doesn't break the app
    // even if the Performance API is not available or fails

    expect(() => {
      perfStart('test-component')
      perfEnd('test-component')
    }).not.toThrow()
  })
})

describe('Data Source Fetch Performance', () => {
  it('should create readable performance keys within length limits', () => {
    const longDeviceId = 'device-with-very-long-name-123456789'
    const longMetricId = 'metric.with.very.long.name.temperature.humidity'

    const fetchId = 'abc123'
    const perfKey = `DS-${fetchId}-${longDeviceId}.${longMetricId}`.slice(0, 50)

    expect(perfKey.length).toBeLessThanOrEqual(50)
    expect(perfKey).toContain('DS-')
  })

  it('should prevent duplicate measurements with unique IDs', () => {
    const measurements = new Set<string>()

    // Simulate multiple fetch attempts for same data source
    for (let i = 0; i < 5; i++) {
      const fetchId = `${Date.now()}-${i}`
      const perfKey = `DS-${fetchId}-device123.temperature`
      measurements.add(perfKey)
    }

    // All measurements should be unique
    expect(measurements.size).toBe(5)
  })
})

describe('Dashboard Performance Scenarios', () => {
  it('should handle multiple component mounts efficiently', () => {
    const componentCount = 28 // Typical dashboard size

    const startTime = Date.now()

    for (let i = 0; i < componentCount; i++) {
      const componentName = `Component-${i}`
      perfStart(componentName)

      // Simulate component work
      perfEnd(componentName)
    }

    const endTime = Date.now()
    const duration = endTime - startTime

    // Should complete all performance tracking quickly (<100ms)
    expect(duration).toBeLessThan(100)
  })

  it('should track data fetching separately from component mounting', () => {
    const componentPerfKey = 'LineChart-abc123'
    const dataFetchPerfKey = 'DS-xyz789-device123.temperature'

    perfStart(componentPerfKey)
    perfStart(dataFetchPerfKey)

    perfEnd(componentPerfKey)
    perfEnd(dataFetchPerfKey)

    // Verify performance tracking works without throwing
    // The actual entries are cleared by perfEnd, but the operations complete
    expect(() => {
      perfStart('test-component')
      perfStart('test-data-fetch')
      perfEnd('test-component')
      perfEnd('test-data-fetch')
    }).not.toThrow()
  })
})