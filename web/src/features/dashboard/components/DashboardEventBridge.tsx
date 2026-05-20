/**
 * DashboardEventBridge
 *
 * Bridges WebSocket/SSE events into TanStack Query cache.
 * Mounts inside VisualDashboard, subscribes to device and extension events,
 * and routes data into the correct query cache entries.
 */

import { useEffect, useRef } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { dashboardKeys } from '../hooks/queries'
import { appendDataPoint } from '../utils/telemetryTransform'
import type { TelemetryPoint } from '../api/telemetry'
import type { DataSource } from '../types'

interface DashboardEventBridgeProps {
  /** Data sources from all widgets in the current dashboard */
  dataSources: DataSource[]
}

/**
 * Renderless component that bridges real-time events into TanStack Query cache.
 * Returns null (no UI).
 */
export function DashboardEventBridge({ dataSources }: DashboardEventBridgeProps) {
  const queryClient = useQueryClient()
  const dataSourcesRef = useRef(dataSources)
  dataSourcesRef.current = dataSources

  useEffect(() => {
    // Subscribe to WebSocket events for device telemetry
    // Uses the existing event infrastructure (useEvents pattern)
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const wsBase = import.meta.env.DEV
      ? 'ws://localhost:9375'
      : `${protocol}//${window.location.host}`
    const wsUrl = `${wsBase}/api/events/ws`

    let ws: WebSocket | null = null
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null
    let isDestroyed = false

    function connect() {
      if (isDestroyed) return
      try {
        ws = new WebSocket(wsUrl)
        ws.onmessage = (event) => {
          try {
            const msg = JSON.parse(event.data)
            handleEvent(msg)
          } catch {
            // Ignore malformed messages
          }
        }
        ws.onclose = () => {
          if (!isDestroyed) {
            reconnectTimer = setTimeout(connect, 5000)
          }
        }
        ws.onerror = () => {
          ws?.close()
        }
      } catch {
        // WebSocket not available
      }
    }

    function handleEvent(msg: { type?: string; device_id?: string; metric?: string; value?: unknown; timestamp?: number; extension_id?: string }) {
      if (msg.type === 'telemetry' && msg.device_id && msg.metric) {
        const point: TelemetryPoint = {
          timestamp: msg.timestamp ?? Math.floor(Date.now() / 1000),
          value: typeof msg.value === 'number' ? msg.value : parseFloat(String(msg.value)) || 0,
        }
        const sourceKey = `${msg.device_id}:${msg.metric}`

        // Update all matching telemetry queries in cache
        // We don't know exactly which time windows are active, so update all
        queryClient.setQueriesData<TelemetryPoint[]>(
          { queryKey: [...dashboardKeys.all, 'telemetry', sourceKey] },
          (old) => appendDataPoint(old, point),
        )

        // Also update the device current value cache
        queryClient.setQueryData(
          dashboardKeys.deviceCurrent(msg.device_id),
          (old: Record<string, unknown> | undefined) => ({
            ...old,
            [msg.metric!]: msg.value,
          }),
        )
      }

      if (msg.type === 'extension_metric' && msg.extension_id && msg.metric) {
        queryClient.setQueryData(
          dashboardKeys.extensionMetrics(msg.extension_id),
          (old: Record<string, unknown> | undefined) => ({
            ...old,
            [msg.metric!]: msg.value,
          }),
        )
      }
    }

    connect()

    return () => {
      isDestroyed = true
      if (reconnectTimer) clearTimeout(reconnectTimer)
      ws?.close()
    }
  }, [queryClient])

  return null
}
