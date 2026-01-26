/**
 * Unified State Management Hook for Dashboard Components
 *
 * Provides consistent state logic and copy for all dashboard components.
 */

import { useMemo, useState, useCallback } from 'react'
import { useDataSource } from '@/hooks/useDataSource'
import type { DataSourceOrList } from '@/types/dashboard'

// Unified copy constants
export const DASHBOARD_COPY = {
  empty: {
    generic: 'No data available',
    data: 'No data available',
    telemetry: 'No telemetry data',
    markers: 'No markers',
    images: 'No images',
    video: 'No video source',
    commands: 'No commands configured',
    device: 'Device not found',
  },
  error: {
    generic: 'Failed to load data',
    network: 'Network error',
    format: 'Invalid data format',
    permission: 'Access denied',
    device: 'Device offline',
  },
  loading: {
    generic: 'Loading...',
    data: 'Loading data...',
    device: 'Connecting to device...',
  },
  retry: 'Retry',
} as const

export type DashboardState = 'loading' | 'error' | 'empty' | 'loaded'

export interface UseDashboardStateOptions<T> {
  dataSource?: DataSourceOrList
  defaultValue?: T
  // Customize when data is considered empty
  isEmptyFn?: (data: T | null) => boolean
  // Customize error message
  errorMessage?: string
  // Customize empty message
  emptyMessage?: string
}

export interface UseDashboardStateReturn<T> {
  // Data from source
  data: T | null
  // Loading state
  loading: boolean
  // Error from source
  error: unknown
  // Current dashboard state
  state: DashboardState
  // Derived states
  isEmpty: boolean
  hasError: boolean
  isLoaded: boolean
  // Retry mechanism
  retry: () => void
  retryCount: number
}

/**
 * Hook that provides unified state management for dashboard components
 */
export function useDashboardState<T = unknown>({
  dataSource,
  defaultValue,
  isEmptyFn,
  errorMessage,
  emptyMessage,
}: UseDashboardStateOptions<T> = {}): UseDashboardStateReturn<T> {
  const [retryCount, setRetryCount] = useState(0)

  // Use dataSource hook to fetch data
  const { data, loading, error } = useDataSource<T>(dataSource, {
    fallback: defaultValue,
  })

  // Default empty check for arrays and null/undefined
  const defaultIsEmpty = useCallback((value: T | null) => {
    if (value === null || value === undefined) return true
    if (Array.isArray(value) && value.length === 0) return true
    return false
  }, [])

  // Determine current state
  const state: DashboardState = useMemo(() => {
    if (loading) return 'loading'
    if (error) return 'error'
    const isEmptyCheck = isEmptyFn || defaultIsEmpty
    if (isEmptyCheck(data)) return 'empty'
    return 'loaded'
  }, [loading, error, data, isEmptyFn, defaultIsEmpty])

  // Derived states
  const isEmpty = state === 'empty'
  const hasError = state === 'error'
  const isLoaded = state === 'loaded'

  // Retry function
  const retry = useCallback(() => {
    setRetryCount(prev => prev + 1)
  }, [])

  return {
    data,
    loading,
    error,
    state,
    isEmpty,
    hasError,
    isLoaded,
    retry,
    retryCount,
  }
}

/**
 * Hook specifically for data that requires minimum length (like charts)
 */
export function useChartDataState(options: UseDashboardStateOptions<unknown[]> & {
  minLength?: number
} = {}) {
  const { minLength = 2, ...restOptions } = options

  return useDashboardState<unknown[]>({
    ...restOptions,
    isEmptyFn: (data) => {
      if (!data || data.length < minLength) return true
      return false
    },
  })
}

/**
 * Hook specifically for device data
 */
export function useDeviceDataState<T = unknown>(
  deviceId: string | undefined,
  options?: Omit<UseDashboardStateOptions<T>, 'dataSource'>
) {
  const dataSource: DataSourceOrList | undefined = useMemo(() => {
    if (!deviceId) return undefined
    return { type: 'device', deviceId }
  }, [deviceId])

  return useDashboardState<T>({
    ...options,
    isEmptyFn: (data: T | null) => {
      // Device is offline or no data
      if (data === null || data === undefined) return true
      return options?.isEmptyFn?.(data) ?? false
    },
  })
}
