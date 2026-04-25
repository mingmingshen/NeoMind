import { useState, useCallback, useRef, useEffect } from 'react'
import { api } from '@/lib/api'
import { useAgentEvents } from '@/hooks/useAgentEvents'
import type { ResourceRequest } from '@/types'
import type { DataSource, DataSourceOrList } from '@/types/dashboard'
import { getSourceId, normalizeDataSource } from '@/types/dashboard'
import type { AiAnalystConfig, AnalystMessage } from './types'

interface UseAnalystSessionParams {
  componentId: string
  config: AiAnalystConfig
  dataSource?: DataSourceOrList
  onConfigUpdate: (updates: Partial<AiAnalystConfig>) => void
}

interface UseAnalystSessionReturn {
  messages: AnalystMessage[]
  isStreaming: boolean
  streamingContent: string
  streamingMsgId: string | null
  error: string | null
  initializing: boolean
  initSession: () => Promise<void>
  sendImage: (imageDataUrl: string, dataSource?: string) => void
  /** Add a non-image data event to timeline (shows a compact summary, not raw value) */
  sendData: (value: unknown, dataSource?: string) => void
  sendText: (text: string) => void
  isConnected: boolean
}

let msgCounter = 0
function nextId(): string {
  return `analyst-${Date.now()}-${++msgCounter}`
}

/** Internal/metadata fields to hide from data display */
const META_FIELDS = new Set([
  '_is_event_data', '_is_image', '_is_binary', '_source_type',
  'image_mime_type', 'mime_type',
  // Backend data_collected metadata fields for extension metrics
  'extension_id', 'timestamp', 'points_count', 'has_history',
])

/** Check if a field key is internal metadata that should be hidden */
function isMetaField(key: string): boolean {
  return META_FIELDS.has(key) || key.startsWith('_')
}

/** Find index of the last completed (non-streaming) AI/error message */
function findLastCompletedAi(msgs: AnalystMessage[]): number {
  for (let i = msgs.length - 1; i >= 0; i--) {
    const m = msgs[i]
    if ((m.type === 'ai' || m.type === 'error') && !m.isStreaming) return i
  }
  return -1
}

/** Build a compact display string from data values */
function summarizeData(value: unknown): string {
  if (value === null || value === undefined) return ''
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  if (typeof value === 'string') {
    return (value as string).length > 100 ? (value as string).slice(0, 100) + '...' : value as string
  }
  if (typeof value === 'object') {
    try {
      if (Array.isArray(value)) {
        if (value.length > 0 && typeof value[0] === 'object' && value[0] !== null) {
          // Check if these are data_points with {timestamp, value} structure
          const first = value[0] as Record<string, unknown>
          if ('value' in first && 'timestamp' in first) {
            // Extract just the values, show latest first
            const vals = (value as Array<Record<string, unknown>>)
              .map(p => p.value)
              .filter(v => v !== undefined)
            if (vals.length === 0) return ''
            if (vals.length === 1) return summarizeData(vals[0])
            return vals.slice(0, 6).map(v => summarizeData(v)).join(', ') +
              (vals.length > 6 ? ` ... (+${vals.length - 6})` : '')
          }
          // Generic object array: show key: value pairs from first object
          // (e.g. [{cloud_cover: 100, temp: 23.8, ...}] → "cloud_cover: 100\ntemp: 23.8")
          const entries = Object.entries(first)
            .filter(([k]) => !isMetaField(k))
            .slice(0, 6)
          if (entries.length === 0) return ''
          const summary = entries.map(([k, v]) => {
            const vs = typeof v === 'object' && v !== null
              ? `(${Array.isArray(v) ? v.length + ' items' : Object.keys(v).length + ' keys'})`
              : String(v)
            return `${k}: ${vs.length > 30 ? vs.slice(0, 30) + '...' : vs}`
          }).join('\n')
          return value.length > 1 ? summary + `\n(${value.length} records)` : summary
        }
        // Primitive array
        const strs = value.map(v => String(v)).slice(0, 6)
        return strs.join(', ') + (value.length > 6 ? ` ... (+${value.length - 6})` : '')
      }
      // Object: show key: value summary, filtering out metadata fields
      const obj = value as Record<string, unknown>
      const entries = Object.entries(obj).filter(([k]) => !isMetaField(k)).slice(0, 6)
      if (entries.length === 0) return ''
      return entries.map(([k, v]) => {
        const vs = typeof v === 'object' && v !== null
          ? `(${Array.isArray(v) ? v.length + ' items' : Object.keys(v).length + ' keys'})`
          : String(v)
        return `${k}: ${vs.length > 30 ? vs.slice(0, 30) + '...' : vs}`
      }).join('\n')
    } catch {
      return String(value)
    }
  }
  return String(value)
}

/**
 * Resolve the metric field name from a DataSource.
 * Dashboard picker uses different property names depending on the source type:
 * - Device metrics → metricId (e.g., "values.image")
 * - Extension metrics → extensionMetric
 * - Others → property or fallback to "image"
 */
function resolveMetricField(dataSource: DataSource): string {
  const raw = dataSource.metricId
    || dataSource.extensionMetric
    || dataSource.property
    || dataSource.infoProperty
    || 'image'
  // Frontend data source picker uses "produce:" prefix for extension metrics
  // but backend stores/publishes raw metric names without this prefix
  return raw.startsWith('produce:') ? raw.slice(8) : raw
}

function buildResources(dataSources: DataSource[]): ResourceRequest[] | undefined {
  if (!dataSources.length) return undefined
  const resources: ResourceRequest[] = dataSources.flatMap((ds): ResourceRequest[] => {
    const sourceId = getSourceId(ds)
    const field = resolveMetricField(ds)

    if (ds.type === 'extension' && ds.extensionId) {
      return [{
        resource_id: `extension:${ds.extensionId}:${field}`,
        resource_type: 'extension_metric',
        name: ds.extensionDisplayName || field,
        config: {
          extension_id: ds.extensionId,
          metric_name: field,
        },
      }]
    }
    if (sourceId) {
      return [{
        resource_id: `${sourceId}:${field}`,
        resource_type: ds.type === 'extension-metric' ? 'extension_metric' : 'metric',
        name: field,
        config: {
          device_id: sourceId,
          metric_name: field,
        },
      }]
    }
    return []
  })
  return resources.length > 0 ? resources : undefined
}

/**
 * Build event_filter JSON for schedule_type: 'event' so the agent editor
 * can display the Data-driven trigger sources.
 * Format: {"sources": [{"type": "device"|"extension", "id": "...", "name": "...", "field": "..."}]}
 */
function buildEventFilter(dataSources: DataSource[]): string | undefined {
  if (!dataSources.length) return undefined
  const sources = dataSources.flatMap((ds) => {
    const sourceId = getSourceId(ds)
    const field = resolveMetricField(ds)

    if (ds.type === 'extension' && ds.extensionId) {
      return [{
        type: 'extension',
        id: ds.extensionId,
        name: ds.extensionDisplayName || ds.extensionId,
        field,
      }]
    }
    if (sourceId) {
      return [{
        type: 'device',
        id: sourceId,
        name: sourceId,
        field,
      }]
    }
    return []
  })
  return sources.length > 0 ? JSON.stringify({ sources }) : undefined
}

/**
 * Load existing agent execution history and convert to AnalystMessage[].
 * Only loads the most recent executions that fit within contextWindowSize.
 */
async function loadHistoryMessages(
  agentId: string,
  contextWindowSize: number,
): Promise<AnalystMessage[]> {
  try {
    const resp = await api.getAgentExecutions(agentId, contextWindowSize)
    if (!resp?.executions?.length) return []

    // Executions come newest-first; reverse to chronological order
    const sorted = [...resp.executions].reverse()

    // Fetch all execution details in parallel
    const details = await Promise.all(
      sorted.map(async (exec) => {
        if (exec.status === 'Running') return null
        try {
          const detail = await api.getExecution(agentId, exec.id)
          return { exec, detail }
        } catch {
          return null
        }
      })
    )

    const messages: AnalystMessage[] = []
    for (const entry of details) {
      if (!entry) continue
      const { exec, detail } = entry

      const conclusion = detail?.decision_process?.conclusion
      if (!conclusion) continue

      const ts = new Date(exec.timestamp).getTime() || Date.now()

      // Restore ALL data_collected entries as ONE data message before AI response
      const dataCollected = detail?.decision_process?.data_collected
      if (dataCollected && dataCollected.length > 0) {
        const allImages: string[] = []
        const allLines: string[] = []
        for (const dc of dataCollected) {
          const values = dc.values
          const dataType = dc.data_type

          if (values && typeof values === 'object' && !Array.isArray(values)) {
            const record = values as Record<string, unknown>

            // Detect backend data_collected format for extension metrics:
            // { extension_id, value, timestamp, points_count, has_history }
            // Or event-triggered data: { value, timestamp, _is_event_data }
            // Only the "value" field is meaningful data; everything else is metadata.
            const isStructuredData = ('extension_id' in record && 'value' in record)
              || ('value' in record && '_is_event_data' in record)
            if (isStructuredData) {
              const val = record.value
              if (typeof val === 'string' && val.length > 100 && (
                val.startsWith('data:image/') || val.startsWith('/9j/') || val.startsWith('iVBOR')
              )) {
                allImages.push(val.startsWith('data:') ? val : `data:image/png;base64,${val}`)
              } else {
                const s = summarizeData(val)
                if (s) allLines.push(dataType ? `${dataType}: ${s}` : s)
              }
            } else {
              // Generic object: iterate entries, filter metadata
              for (const [key, val] of Object.entries(record)) {
                if (typeof val === 'string' && val.length > 100 && (
                  val.startsWith('data:image/') || val.startsWith('/9j/') || val.startsWith('iVBOR')
                )) {
                  allImages.push(val.startsWith('data:') ? val : `data:image/png;base64,${val}`)
                } else if (!isMetaField(key)) {
                  const s = summarizeData(val)
                  if (s) allLines.push(`${key}: ${s}`)
                }
              }
            }
          } else {
            const s = summarizeData(values)
            if (s) allLines.push(dataType ? `${dataType}: ${s}` : s)
          }
        }
        if (allImages.length > 0 || allLines.length > 0) {
          messages.push({
            id: `hist-data-${exec.id}`,
            type: 'data',
            content: allLines.join('\n'),
            images: allImages.length > 0 ? allImages : undefined,
            timestamp: ts,
          })
        }
      }

      // AI response after data
      messages.push({
        id: `hist-${exec.id}`,
        type: exec.status === 'Failed' ? 'error' : 'ai',
        content: conclusion,
        timestamp: ts,
        duration: Math.round(exec.duration_ms / 1000),
      })
    }

    return messages
  } catch {
    return []
  }
}

export function useAnalystSession({
  componentId,
  config,
  dataSource,
  onConfigUpdate,
}: UseAnalystSessionParams): UseAnalystSessionReturn {
  const [messages, setMessages] = useState<AnalystMessage[]>([])
  const [isStreaming, setIsStreaming] = useState(false)
  const [streamingContent, setStreamingContent] = useState('')
  const [streamingMsgId, setStreamingMsgId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [initializing, setInitializing] = useState(false)
  const [isConnected, setIsConnected] = useState(false)

  const agentIdRef = useRef<string | null>(null)
  // Ref guard survives StrictMode unmount/remount — prevents double agent creation
  const initGuardRef = useRef(false)
  // Track whether history has been loaded for the current agent
  const historyLoadedRef = useRef(false)

  // Stable refs for config values — avoids recreating initSession when config changes
  const configRef = useRef(config)
  configRef.current = config
  const dataSourceRef = useRef(dataSource)
  dataSourceRef.current = dataSource
  const onConfigUpdateRef = useRef(onConfigUpdate)
  onConfigUpdateRef.current = onConfigUpdate

  // Ref to track streaming msg ID for reliable cleanup in onExecutionCompleted
  const streamingMsgIdRef = useRef<string | null>(null)

  // ---- Agent event listeners ----
  // Must be called at hook top level. Disabled when no agentId yet.
  const activeAgentId = config.agentId || agentIdRef.current || ''
  useAgentEvents(activeAgentId, {
    enabled: !!activeAgentId,
    onExecutionStarted: () => {
      setIsStreaming(true)
      // Add a streaming placeholder — just shows "Analyzing..." spinner, no raw text
      const id = `stream-${Date.now()}`
      setStreamingMsgId(id)
      streamingMsgIdRef.current = id
      setStreamingContent('')
      const analyzingMsg: AnalystMessage = {
        id,
        type: 'ai',
        content: '',
        timestamp: Date.now(),
        isStreaming: true,
      }
      setMessages((prev) => [...prev, analyzingMsg])
    },
    onThinking: () => {
      // Keep isStreaming active but don't surface raw thinking text
      // in the timeline — it's too technical for this widget.
      // The AgentMonitorWidget already shows full thinking steps.
    },
    onExecutionCompleted: (data) => {
      setIsStreaming(false)
      setStreamingContent('')
      const placeholderId = streamingMsgIdRef.current
      setStreamingMsgId(null)
      streamingMsgIdRef.current = null

      // Fetch the conclusion from the execution detail
      const agentId = data.agent_id
      api.getExecution(agentId, data.execution_id)
        .then((detail) => {
          const conclusion = detail?.decision_process?.conclusion
          const aiMsg: AnalystMessage = {
            id: `exec-${data.execution_id}`,
            type: data.success ? 'ai' : 'error',
            content: conclusion || 'No result',
            timestamp: Date.now(),
            duration: Math.round(data.duration_ms / 1000),
          }
          setMessages((prev) => {
            // Replace the streaming placeholder with the final result
            const withoutStreaming = placeholderId
              ? prev.filter((m) => m.id !== placeholderId)
              : prev
            return [...withoutStreaming, aiMsg]
          })
        })
        .catch((err) => {
          const errMsg: AnalystMessage = {
            id: `exec-err-${data.execution_id}`,
            type: 'error',
            content: err instanceof Error ? err.message : 'Failed to fetch execution result',
            timestamp: Date.now(),
          }
          setMessages((prev) => {
            const withoutStreaming = placeholderId
              ? prev.filter((m) => m.id !== placeholderId)
              : prev
            return [...withoutStreaming, errMsg]
          })
        })
    },
  })

  // Create agent (once per component lifecycle)
  // Uses refs for config/dataSource so the callback identity is stable —
  // this prevents the auto-init useEffect in AiAnalyst.tsx from re-triggering
  // when config props change (e.g., modelId, systemPrompt updates during editing).
  const initSession = useCallback(async () => {
    if (initGuardRef.current || agentIdRef.current) return

    try {
      initGuardRef.current = true
      setInitializing(true)
      setError(null)

      const cfg = configRef.current
      const ds = dataSourceRef.current
      const dsList = normalizeDataSource(ds)
      const resources = buildResources(dsList)
      const eventFilter = buildEventFilter(dsList)

      const agent = await api.createAgent({
        name: `AI Analyst - ${componentId}`,
        user_prompt: cfg.systemPrompt,
        llm_backend_id: cfg.modelId,
        schedule: { schedule_type: 'event', event_filter: eventFilter },
        execution_mode: 'chat',
        context_window_size: cfg.contextWindowSize,
        resources,
      })

      agentIdRef.current = agent.id
      setIsConnected(true)
      onConfigUpdateRef.current({ agentId: agent.id })
      setInitializing(false)
    } catch (err) {
      initGuardRef.current = false
      setInitializing(false)
      setError(err instanceof Error ? err.message : 'Failed to create agent')
    }
  }, [componentId])

  // On mount: if config has a saved agentId, verify it still exists.
  // If the agent was deleted, clear the stale ID so a new agent will be created.
  useEffect(() => {
    const savedId = config.agentId
    if (!savedId) return

    let cancelled = false
    setInitializing(true)

    api.getAgent(savedId)
      .then(() => {
        if (cancelled) return
        agentIdRef.current = savedId
        setIsConnected(true)
        setInitializing(false)
      })
      .catch(() => {
        if (cancelled) return
        // Agent no longer exists — clear stale config and allow fresh creation
        agentIdRef.current = null
        initGuardRef.current = false
        onConfigUpdateRef.current({ agentId: undefined })
        setInitializing(false)
      })

    return () => { cancelled = true }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // Sync llm_backend_id and user_prompt when config changes after agent creation
  const prevModelIdRef = useRef(config.modelId)
  const prevPromptRef = useRef(config.systemPrompt)
  useEffect(() => {
    const agentId = agentIdRef.current
    if (!agentId || !isConnected) return

    const updates: Record<string, unknown> = {}
    if (config.modelId && config.modelId !== prevModelIdRef.current) {
      updates.llm_backend_id = config.modelId
      prevModelIdRef.current = config.modelId
    }
    if (config.systemPrompt && config.systemPrompt !== prevPromptRef.current) {
      updates.user_prompt = config.systemPrompt
      prevPromptRef.current = config.systemPrompt
    }

    if (Object.keys(updates).length === 0) return
    api.updateAgent(agentId, updates).catch(() => {
      // Non-critical: agent will use whatever configuration it has
    })
  }, [config.modelId, config.systemPrompt, isConnected])

  // Sync event_filter and resources when dataSource changes after agent creation
  // This handles component editing (changing data source) without creating a new agent
  const prevDataSourceRef = useRef(dataSource)
  useEffect(() => {
    const agentId = agentIdRef.current
    if (!agentId || !isConnected) return
    // Skip initial mount — agent was just created with this dataSource
    if (prevDataSourceRef.current === dataSource) return
    prevDataSourceRef.current = dataSource

    const dsList = normalizeDataSource(dataSource)
    const resources = buildResources(dsList)
    const eventFilter = buildEventFilter(dsList)

    api.updateAgent(agentId, {
      schedule: { schedule_type: 'event', event_filter: eventFilter },
      resources,
    }).catch(() => {
      // Non-critical: agent will use previous configuration
    })
  }, [dataSource, isConnected])

  // Load execution history when agent connects (verification or creation)
  useEffect(() => {
    const agentId = agentIdRef.current
    if (!isConnected || !agentId || historyLoadedRef.current) return

    historyLoadedRef.current = true
    loadHistoryMessages(agentId, config.contextWindowSize)
      .then((history) => {
        if (history.length > 0) {
          setMessages(history)
        }
      })
      .catch(() => {
        // History load failure is non-fatal
      })
  }, [isConnected, config.contextWindowSize])

  // Send image — merges into current round's data message (shared bubble with text).
  // If no data message exists yet, creates one with just the image.
  const sendImage = useCallback(
    (imageDataUrl: string, ds?: string) => {
      setMessages((prev) => {
        const lastCompletedAiIdx = findLastCompletedAi(prev)
        // Look for existing data message in current round to merge into
        for (let i = prev.length - 1; i > lastCompletedAiIdx; i--) {
          if (prev[i].type === 'data') {
            const existing = prev[i]
            return [
              ...prev.slice(0, i),
              { ...existing, images: [...(existing.images || []), imageDataUrl] },
              ...prev.slice(i + 1),
            ]
          }
        }
        // No data message in current round — create one with the image
        const newMsg: AnalystMessage = {
          id: nextId(),
          type: 'data',
          content: '',
          images: [imageDataUrl],
          timestamp: Date.now(),
          dataSource: ds,
        }
        const insertIdx = prev.findIndex((m, idx) => idx > lastCompletedAiIdx && (m.type === 'ai' || m.type === 'error'))
        if (insertIdx === -1) return [...prev, newMsg]
        return [...prev.slice(0, insertIdx), newMsg, ...prev.slice(insertIdx)]
      })
    },
    [],
  )

  // Send non-image data — merges into current round's data message (shared bubble with images).
  const sendData = useCallback(
    (value: unknown, ds?: string) => {
      const summary = summarizeData(value)
      // Skip if nothing meaningful to show after filtering metadata
      if (!summary) return

      setMessages((prev) => {
        const lastCompletedAiIdx = findLastCompletedAi(prev)

        // Look for a data message in the current round to merge into
        for (let i = prev.length - 1; i > lastCompletedAiIdx; i--) {
          if (prev[i].type === 'data') {
            const existing = prev[i]
            const newContent = existing.content ? existing.content + '\n' + summary : summary
            return [
              ...prev.slice(0, i),
              { ...existing, content: newContent, timestamp: Date.now(), dataSource: ds || existing.dataSource },
              ...prev.slice(i + 1),
            ]
          }
        }

        // No data message in current round — insert BEFORE any streaming AI message
        const insertIdx = prev.findIndex((m, idx) => idx > lastCompletedAiIdx && (m.type === 'ai' || m.type === 'error'))
        if (insertIdx === -1) {
          return [...prev, {
            id: nextId(),
            type: 'data' as const,
            content: summary,
            timestamp: Date.now(),
            dataSource: ds,
          }]
        }
        return [
          ...prev.slice(0, insertIdx),
          { id: nextId(), type: 'data' as const, content: summary, timestamp: Date.now(), dataSource: ds },
          ...prev.slice(insertIdx),
        ]
      })
    },
    [],
  )

  // Send text to agent via invoke (user-initiated, synchronous)
  const sendText = useCallback(
    (text: string) => {
      const agentId = agentIdRef.current
      if (!agentId) return

      const userMsg: AnalystMessage = {
        id: nextId(),
        type: 'user',
        content: text,
        timestamp: Date.now(),
      }
      setMessages((prev) => [...prev, userMsg])

      setIsStreaming(true)
      const startTime = Date.now()

      api.invokeAgent(agentId, { input: text })
        .then((result) => {
          const duration = Math.round((Date.now() - startTime) / 1000)
          const aiMsg: AnalystMessage = {
            id: nextId(),
            type: result.has_error ? 'error' : 'ai',
            content: result.conclusion || result.error || 'No result',
            timestamp: Date.now(),
            modelName: config.modelName,
            duration,
          }
          setMessages((prev) => [...prev, aiMsg])
        })
        .catch((err) => {
          const errMsg: AnalystMessage = {
            id: nextId(),
            type: 'error',
            content: err instanceof Error ? err.message : 'Request failed',
            timestamp: Date.now(),
          }
          setMessages((prev) => [...prev, errMsg])
        })
        .finally(() => {
          setIsStreaming(false)
        })
    },
    [config.modelName],
  )

  return {
    messages,
    isStreaming,
    streamingContent,
    streamingMsgId,
    error,
    initializing,
    initSession,
    sendImage,
    sendData,
    sendText,
    isConnected,
  }
}
