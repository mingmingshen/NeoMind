import { useState, useCallback, useRef, useEffect } from 'react'
import { api } from '@/lib/api'
import type { ResourceRequest } from '@/types'
import type { DataSource } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'
import type { VlmVisionConfig, VlmMessage } from './types'

interface UseVlmSessionParams {
  componentId: string
  config: VlmVisionConfig
  dataSource?: DataSource
  onConfigUpdate: (updates: Partial<VlmVisionConfig>) => void
}

interface UseVlmSessionReturn {
  messages: VlmMessage[]
  isStreaming: boolean
  streamingContent: string
  streamingMsgId: string | null
  error: string | null
  initializing: boolean
  initSession: () => Promise<void>
  sendImage: (imageDataUrl: string, dataSource?: string) => void
  sendText: (text: string) => void
  isConnected: boolean
}

let msgCounter = 0
function nextId(): string {
  return `vlm-${Date.now()}-${++msgCounter}`
}

function buildResources(dataSource?: DataSource): ResourceRequest[] | undefined {
  if (!dataSource) return undefined
  const sourceId = getSourceId(dataSource)
  if (dataSource.type === 'extension' && dataSource.extensionId && dataSource.extensionMetric) {
    return [{
      resource_id: `extension:${dataSource.extensionId}:${dataSource.extensionMetric}`,
      resource_type: 'extension_metric',
      name: dataSource.extensionDisplayName || dataSource.extensionMetric,
      config: {
        extension_id: dataSource.extensionId,
        metric_name: dataSource.extensionMetric,
      },
    }]
  }
  if (sourceId) {
    return [{
      resource_id: `${sourceId}:${dataSource.property || 'image'}`,
      resource_type: dataSource.type === 'extension-metric' ? 'extension_metric' : 'metric',
      name: dataSource.property || sourceId,
      config: {
        device_id: sourceId,
        metric_name: dataSource.property || 'image',
      },
    }]
  }
  return undefined
}

export function useVlmSession({
  componentId,
  config,
  dataSource,
  onConfigUpdate,
}: UseVlmSessionParams): UseVlmSessionReturn {
  const [messages, setMessages] = useState<VlmMessage[]>([])
  const [isStreaming, setIsStreaming] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [initializing, setInitializing] = useState(false)
  const [isConnected, setIsConnected] = useState(false)

  const agentIdRef = useRef<string | null>(null)
  // Ref guard survives StrictMode unmount/remount — prevents double agent creation
  const initGuardRef = useRef(false)

  // Create agent (once per component lifecycle)
  const initSession = useCallback(async () => {
    if (initGuardRef.current || agentIdRef.current) return

    try {
      initGuardRef.current = true
      setInitializing(true)
      setError(null)

      const resources = buildResources(dataSource)

      const agent = await api.createAgent({
        name: `VLM Vision - ${componentId}`,
        user_prompt: config.systemPrompt,
        llm_backend_id: config.modelId,
        schedule: { schedule_type: 'event' },
        execution_mode: 'chat',
        context_window_size: config.contextWindowSize,
        resources,
      })

      agentIdRef.current = agent.id
      setIsConnected(true)
      onConfigUpdate({ agentId: agent.id })
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
    onConfigUpdate,
  ])

  // On mount: if config has a saved agentId, verify it still exists.
  // If the agent was deleted (e.g. by cleanup on unmount during page navigation),
  // clear the stale ID so a new agent will be created.
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
        onConfigUpdate({ agentId: undefined })
        setInitializing(false)
      })

    return () => { cancelled = true }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // Send image to agent via invoke (synchronous)
  const sendImage = useCallback(
    (imageDataUrl: string, ds?: string) => {
      const agentId = agentIdRef.current
      if (!agentId) return

      // Add image placeholder to timeline
      const imgMsg: VlmMessage = {
        id: nextId(),
        type: 'image',
        content: imageDataUrl,
        timestamp: Date.now(),
        dataSource: ds,
      }
      setMessages((prev) => [...prev, imgMsg])

      // Invoke agent with image data
      setIsStreaming(true)
      const startTime = Date.now()

      api.invokeAgent(agentId, {
        input: 'Analyze this image and describe what you observe.',
        data: { image_base64: imageDataUrl },
      })
        .then((result) => {
          const duration = Math.round((Date.now() - startTime) / 1000)
          const aiMsg: VlmMessage = {
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
          const errMsg: VlmMessage = {
            id: nextId(),
            type: 'error',
            content: err instanceof Error ? err.message : 'Analysis failed',
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

  // Send text to agent via invoke
  const sendText = useCallback(
    (text: string) => {
      const agentId = agentIdRef.current
      if (!agentId) return

      const userMsg: VlmMessage = {
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
          const aiMsg: VlmMessage = {
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
          const errMsg: VlmMessage = {
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
    streamingContent: '',
    streamingMsgId: null,
    error,
    initializing,
    initSession,
    sendImage,
    sendText,
    isConnected,
  }
}
