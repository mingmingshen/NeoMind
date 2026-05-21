/**
 * useExtensionEventProcessing — Extension WebSocket event handling for useDataSource.
 *
 * Processes extension output events and merges them into component state.
 */

import { useEffect, useRef, useMemo } from 'react'
import type { DataSource } from '@/types/dashboard'
import { useEvents } from '@/hooks/useEvents'
import { extensionDataCache } from './cache'

interface UseExtensionEventProcessingOptions {
  dataSources: DataSource[]
  dataSourceKey: string
  enabled: boolean
  preserveMultiple: boolean
  transform: ((data: unknown) => unknown) | undefined
  fallback: unknown
  dataRef: React.MutableRefObject<unknown>
  setData: (data: unknown) => void
  setLastUpdate: (ts: number) => void
}

// Normalize output_name: backend may send "ext-id:metric" or just "metric"
function normalizeOutputName(outputName: string): string {
  if (!outputName.includes(':')) return outputName
  return outputName.split(':').slice(1).join(':')
}

export function useExtensionEventProcessing({
  dataSources,
  dataSourceKey,
  enabled,
  preserveMultiple,
  transform,
  fallback,
  dataRef,
  setData,
  setLastUpdate,
}: UseExtensionEventProcessingOptions) {
  const processedEventsRef = useRef<Set<string>>(new Set())
  const lastProcessedEventIdRef = useRef<string | null>(null)
  const dataSourcesRef = useRef(dataSources)
  dataSourcesRef.current = dataSources

  const needsExtensionWebSocket = dataSources.some((ds) => ds.type === 'extension')

  const { events: extensionEvents } = useEvents({
    enabled: enabled && needsExtensionWebSocket,
    category: 'extension',
    onConnected: (connected) => {
      processedEventsRef.current.clear()
      lastProcessedEventIdRef.current = null
    },
  })

  const extensionEventsKey = useMemo(() => {
    if (extensionEvents.length === 0) return 'empty'
    const lastEvent = extensionEvents[extensionEvents.length - 1]
    return `ext-events-${extensionEvents.length}-${lastEvent?.id || 'unknown'}`
  }, [extensionEvents])

  const relevantExtensionIds = useMemo(() => {
    return new Set(
      dataSources
        .map((ds) => ds.type === 'extension' ? (ds as any).extensionId : null)
        .filter(Boolean) as string[]
    )
  }, [dataSources])

  useEffect(() => {
    if (!needsExtensionWebSocket || !enabled || extensionEvents.length === 0) return

    let extStartIndex = 0
    const lastProcessedExtId = lastProcessedEventIdRef.current
    if (lastProcessedExtId) {
      const lastIndex = extensionEvents.findIndex(e => e.id === lastProcessedExtId)
      if (lastIndex !== -1) extStartIndex = lastIndex + 1
      else {
        extStartIndex = 0
        const entries = Array.from(processedEventsRef.current)
        processedEventsRef.current = new Set(entries.slice(-50))
      }
    }
    if (extStartIndex > extensionEvents.length) {
      extStartIndex = 0
      processedEventsRef.current.clear()
    }

    const newEvents = extensionEvents.slice(extStartIndex)
    if (newEvents.length === 0) return

    const extensionDataSources = dataSourcesRef.current.filter((ds) => ds.type === 'extension') as Array<{
      extensionId: string
      extensionMetric: string
    }>
    if (extensionDataSources.length === 0) return

    let lastProcessedExtIdInBatch: string | null = null

    for (const latestEvent of newEvents) {
      const eventData = (latestEvent as any).data || latestEvent
      const eventType = (latestEvent as any).type

      if (eventType !== 'ExtensionOutput') continue

      const uniqueEventId = latestEvent.id || `${eventType}_${Date.now()}_${Math.random()}`
      if (processedEventsRef.current.has(uniqueEventId)) continue
      processedEventsRef.current.add(uniqueEventId)
      lastProcessedExtIdInBatch = uniqueEventId

      if (processedEventsRef.current.size > 100) {
        const entries = Array.from(processedEventsRef.current)
        processedEventsRef.current = new Set(entries.slice(-50))
      }

      const eventExtensionId = eventData.extension_id as string
      const eventOutputName = eventData.output_name as string

      if (!relevantExtensionIds.has(eventExtensionId)) continue

      const normalizedOutput = normalizeOutputName(eventOutputName)

      const matchingSources = extensionDataSources.filter((ds) => {
        if (ds.extensionId !== eventExtensionId) return false
        if (!ds.extensionMetric) return false
        const parts = ds.extensionMetric.split(':')
        const metricName = parts.length > 1 ? parts[1] : parts[0]
        return metricName === normalizedOutput || metricName === eventOutputName
      })

      if (matchingSources.length > 0) {
        // Invalidate cache so next fetch doesn't overwrite with stale data
        matchingSources.forEach((ds) => {
          if (ds.extensionId && ds.extensionMetric) {
            extensionDataCache.delete(`${ds.extensionId}|${ds.extensionMetric}`)
          }
        })

        const currentData = dataRef.current as any
        const eventValue = eventData.value
        const now = Math.floor(Date.now() / 1000)
        const newPoint = { timestamp: now, time: now, value: eventValue }

        let newData: unknown
        const allExtSources = dataSourcesRef.current.filter((ds) => ds.type === 'extension')

        if (preserveMultiple && allExtSources.length > 1 && Array.isArray(currentData)) {
          const nested = (currentData as unknown[][]).map((arr, i) => {
            const ds = allExtSources[i]
            if (!ds) return arr
            const parts = (ds.extensionMetric ?? '').split(':')
            const metricName = parts.length > 1 ? parts[1] : parts[0]
            if (ds.extensionId === eventExtensionId && (metricName === normalizedOutput || metricName === eventOutputName)) {
              return [newPoint, ...(Array.isArray(arr) ? arr : [])]
            }
            return arr
          })
          newData = nested
        } else if (Array.isArray(currentData)) {
          newData = [newPoint, ...currentData]
        } else {
          // Always wrap as array to ensure React detects the change (Object.is comparison
          // skips re-render when the primitive value is the same but timestamp differs).
          newData = [newPoint]
        }

        const transformedData = transform ? transform(newData) : newData
        setData(transformedData)
        setLastUpdate(Date.now())
      }
    }

    if (lastProcessedExtIdInBatch) {
      lastProcessedEventIdRef.current = lastProcessedExtIdInBatch
    }
  }, [enabled, dataSourceKey, extensionEventsKey, needsExtensionWebSocket, relevantExtensionIds.size])

  return { needsExtensionWebSocket }
}
