import { useState, useCallback, useRef, useEffect } from 'react'
import { api } from '@/lib/api'
import { useAgentEvents } from '@/hooks/useAgentEvents'
import type { ResourceRequest } from '@/types'
import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import type { AiAnalystConfig, AnalystMessage } from './types'

interface UseAnalystSessionParams {
  componentId: string
  config: AiAnalystConfig
  dataSource?: DataSource
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

/**
 * Resolve the metric field name from a DataSource.
 * Dashboard picker uses different property names depending on the source type:
 * - Device metrics → metricId (e.g., "values.image")
 * - Extension metrics → extensionMetric
 * - Others → property or fallback to "image"
 */
function resolveMetricField(dataSource: DataSource): string {
  return dataSource.metricId
    || dataSource.extensionMetric
    || dataSource.property
    || dataSource.infoProperty
    || 'image'
}

function buildResources(dataSource?: DataSource): ResourceRequest[] | undefined {
  if (!dataSource) return undefined
  const sourceId = getSourceId(dataSource)
  const field = resolveMetricField(dataSource)

  if (dataSource.type === 'extension' && dataSource.extensionId) {
    return [{
      resource_id: `extension:${dataSource.extensionId}:${field}`,
      resource_type: 'extension_metric',
      name: dataSource.extensionDisplayName || field,
      config: {
        extension_id: dataSource.extensionId,
        metric_name: field,
      },
    }]
  }
  if (sourceId) {
    return [{
      resource_id: `${sourceId}:${field}`,
      resource_type: dataSource.type === 'extension-metric' ? 'extension_metric' : 'metric',
      name: field,
      config: {
        device_id: sourceId,
        metric_name: field,
      },
    }]
  }
  return undefined
}

/**
 * Build event_filter JSON for schedule_type: 'event' so the agent editor
 * can display the Data-driven trigger sources.
 * Format: {"sources": [{"type": "device"|"extension", "id": "...", "name": "...", "field": "..."}]}
 */
function buildEventFilter(dataSource?: DataSource): string | undefined {
  if (!dataSource) return undefined
  const sourceId = getSourceId(dataSource)
  const field = resolveMetricField(dataSource)

  if (dataSource.type === 'extension' && dataSource.extensionId) {
    return JSON.stringify({
      sources: [{
        type: 'extension',
        id: dataSource.extensionId,
        name: dataSource.extensionDisplayName || dataSource.extensionId,
        field,
      }],
    })
  }
  if (sourceId) {
    return JSON.stringify({
      sources: [{
        type: 'device',
        id: sourceId,
        name: sourceId,
        field,
      }],
    })
  }
  return undefined
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
    const messages: AnalystMessage[] = []

    for (const exec of sorted) {
      if (exec.status === 'Running') continue

      // Get full execution detail for conclusion
      try {
        const detail = await api.getExecution(agentId, exec.id)
        const conclusion = detail?.decision_process?.conclusion
        if (!conclusion) continue

        const ts = new Date(exec.timestamp).getTime() || Date.now()
        messages.push({
          id: `hist-${exec.id}`,
          type: exec.status === 'Failed' ? 'error' : 'ai',
          content: conclusion,
          timestamp: ts,
          duration: Math.round(exec.duration_ms / 1000),
        })
      } catch {
        // Skip executions that fail to load detail
      }
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

  // Stable ref for onConfigUpdate to avoid initSession recreating on every render
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
  const initSession = useCallback(async () => {
    if (initGuardRef.current || agentIdRef.current) return

    try {
      initGuardRef.current = true
      setInitializing(true)
      setError(null)

      const resources = buildResources(dataSource)
      const eventFilter = buildEventFilter(dataSource)

      const agent = await api.createAgent({
        name: `AI Analyst - ${componentId}`,
        user_prompt: config.systemPrompt,
        llm_backend_id: config.modelId,
        schedule: { schedule_type: 'event', event_filter: eventFilter },
        execution_mode: 'chat',
        context_window_size: config.contextWindowSize,
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
  }, [
    componentId,
    config.systemPrompt,
    config.modelId,
    config.contextWindowSize,
    dataSource,
    // onConfigUpdate removed — uses ref to avoid cascading re-creates
  ])

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

  // Send image to timeline only — backend handles execution via event trigger
  const sendImage = useCallback(
    (imageDataUrl: string, ds?: string) => {
      const imgMsg: AnalystMessage = {
        id: nextId(),
        type: 'image',
        content: imageDataUrl,
        timestamp: Date.now(),
        dataSource: ds,
      }
      setMessages((prev) => [...prev, imgMsg])
    },
    [],
  )

  // Send non-image data to timeline as a compact summary
  const sendData = useCallback(
    (value: unknown, ds?: string) => {
      // Build a compact display string — never dump raw large values
      let summary: string
      if (value === null || value === undefined) {
        summary = '(empty)'
      } else if (typeof value === 'number') {
        summary = String(value)
      } else if (typeof value === 'boolean') {
        summary = value ? 'true' : 'false'
      } else if (typeof value === 'string') {
        // Truncate long strings, strip base64 data
        const s = value as string
        if (s.length > 200) {
          summary = s.slice(0, 200) + '...'
        } else {
          summary = s
        }
      } else if (typeof value === 'object') {
        // For objects/arrays, show a count-based summary
        try {
          const json = JSON.stringify(value)
          if (json.length > 150) {
            // Summarize: show keys or array length
            if (Array.isArray(value)) {
              summary = `[${value.length} items]`
            } else {
              const keys = Object.keys(value as Record<string, unknown>)
              summary = `{${keys.slice(0, 5).join(', ')}${keys.length > 5 ? ', ...' : ''}}`
            }
          } else {
            summary = json
          }
        } catch {
          summary = String(value)
        }
      } else {
        summary = String(value)
      }

      const dataMsg: AnalystMessage = {
        id: nextId(),
        type: 'data',
        content: summary,
        timestamp: Date.now(),
        dataSource: ds,
      }
      setMessages((prev) => [...prev, dataMsg])
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
