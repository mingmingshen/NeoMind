import { useState, useRef, useCallback, useEffect } from 'react'
import { ChatWebSocket } from '@/lib/websocket'
import { api } from '@/lib/api'
import type { ServerMessage } from '@/types'
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

export function useVlmSession({
  componentId,
  config,
  dataSource,
  onConfigUpdate,
}: UseVlmSessionParams): UseVlmSessionReturn {
  const [messages, setMessages] = useState<VlmMessage[]>([])
  const [isStreaming, setIsStreaming] = useState(false)
  const [streamingContent, setStreamingContent] = useState('')
  const [streamingMsgId, setStreamingMsgId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [initializing, setInitializing] = useState(false)
  const [isConnected, setIsConnected] = useState(false)

  const wsRef = useRef<ChatWebSocket | null>(null)
  const unsubMessageRef = useRef<(() => void) | null>(null)
  const unsubConnectionRef = useRef<(() => void) | null>(null)
  const streamingStartRef = useRef<number>(0)

  // Cleanup helpers
  const cleanup = useCallback(() => {
    if (unsubMessageRef.current) {
      unsubMessageRef.current()
      unsubMessageRef.current = null
    }
    if (unsubConnectionRef.current) {
      unsubConnectionRef.current()
      unsubConnectionRef.current = null
    }
    if (wsRef.current) {
      wsRef.current.disconnect()
      wsRef.current = null
    }
  }, [])

  // Reconnect on mount if sessionId already exists in config
  useEffect(() => {
    if (config.sessionId) {
      const reconnect = async () => {
        try {
          setInitializing(true)
          const ws = new ChatWebSocket()
          wsRef.current = ws

          // Register connection handler
          unsubConnectionRef.current = ws.onConnection((connected) => {
            setIsConnected(connected)
          })

          // Connect with existing session
          ws.connect(config.sessionId!)

          // Load history
          try {
            const history = await api.getSessionHistory(config.sessionId!)
            const converted: VlmMessage[] = history.messages
              .filter((m) => m.role !== 'tool' && m.role !== 'system')
              .map((m) => ({
                id: m.id || nextId(),
                type: m.role === 'assistant' ? 'ai' : m.role === 'user' && (m.images?.length ?? 0) > 0 ? 'image' : 'user',
                content: m.content || '',
                timestamp: m.timestamp,
                modelName: config.modelName,
              }))
            setMessages(converted)
          } catch {
            // History load failure is non-fatal — session may be empty
          }

          setInitializing(false)
        } catch {
          setInitializing(false)
          setError('Failed to reconnect to session')
        }
      }
      reconnect()
    }

    return () => {
      cleanup()
    }
    // Only run on mount/unmount
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Register message handler (called after ws is created)
  const registerMessageHandler = useCallback(
    (ws: ChatWebSocket) => {
      unsubMessageRef.current = ws.onMessage((msg: ServerMessage) => {
        switch (msg.type) {
          case 'Content': {
            // Append streaming content
            setStreamingContent((prev) => {
              const next = prev + msg.content
              // Create streaming placeholder on first chunk
              if (!prev) {
                const id = nextId()
                setStreamingMsgId(id)
                streamingStartRef.current = Date.now()
                setMessages((prevMsgs) => [
                  ...prevMsgs,
                  {
                    id,
                    type: 'ai',
                    content: '',
                    timestamp: Date.now(),
                    modelName: config.modelName,
                    isStreaming: true,
                  },
                ])
              }
              return next
            })
            break
          }

          case 'Thinking': {
            // Ignore thinking content for VLM — we only display final responses
            break
          }

          case 'end': {
            // Finalize AI message
            const duration = streamingStartRef.current
              ? Math.round((Date.now() - streamingStartRef.current) / 1000)
              : undefined

            setStreamingContent((finalContent) => {
              if (finalContent) {
                setStreamingMsgId((finalId) => {
                  if (finalId) {
                    setMessages((prevMsgs) =>
                      prevMsgs.map((m) =>
                        m.id === finalId
                          ? {
                              ...m,
                              content: finalContent,
                              isStreaming: false,
                              duration,
                            }
                          : m,
                      ),
                    )
                  }
                  return null
                })
              }
              return ''
            })

            setIsStreaming(false)
            streamingStartRef.current = 0
            break
          }

          case 'Error': {
            setMessages((prevMsgs) => [
              ...prevMsgs,
              {
                id: nextId(),
                type: 'error',
                content: msg.message,
                timestamp: Date.now(),
              },
            ])
            setIsStreaming(false)
            setStreamingContent('')
            setStreamingMsgId(null)
            streamingStartRef.current = 0
            break
          }

          case 'response': {
            // Non-streaming fallback
            setMessages((prevMsgs) => [
              ...prevMsgs,
              {
                id: nextId(),
                type: 'ai',
                content: msg.content,
                timestamp: Date.now(),
                modelName: config.modelName,
                duration: msg.processingTimeMs
                  ? Math.round(msg.processingTimeMs / 1000)
                  : undefined,
              },
            ])
            setIsStreaming(false)
            setStreamingContent('')
            setStreamingMsgId(null)
            streamingStartRef.current = 0
            break
          }

          default:
            // Ignore other message types (ToolCallStart, ToolCallEnd, Progress, etc.)
            break
        }
      })
    },
    [config.modelName],
  )

  const initSession = useCallback(async () => {
    if (initializing) return

    try {
      setInitializing(true)
      setError(null)
      cleanup()

      // 1. Create agent with data source as resource binding
      let resources: any[] | undefined
      if (dataSource) {
        const sourceId = getSourceId(dataSource)
        if (dataSource.type === 'extension' && dataSource.extensionId && dataSource.extensionMetric) {
          // Extension metric: extension:ext_id:metric_name
          resources = [{
            resource_id: `extension:${dataSource.extensionId}:${dataSource.extensionMetric}`,
            resource_type: 'extension_metric',
            name: dataSource.extensionDisplayName || dataSource.extensionMetric,
            config: {
              extension_id: dataSource.extensionId,
              metric_name: dataSource.extensionMetric,
            },
          }]
        } else if (sourceId) {
          // Device or other: source_id:property
          resources = [{
            resource_id: `${sourceId}:${dataSource.property || 'image'}`,
            resource_type: dataSource.type === 'extension-metric' ? 'extension_metric' : 'metric',
            name: dataSource.property || sourceId,
            config: {
              device_id: sourceId,
              metric_name: dataSource.property || 'image',
            },
          }]
        }
      }

      const agent = await api.createAgent({
        name: `VLM Vision - ${componentId}`,
        user_prompt: config.systemPrompt,
        llm_backend_id: config.modelId,
        schedule: { schedule_type: 'event' },
        execution_mode: 'chat',
        context_window_size: config.contextWindowSize,
        resources,
      })

      // 2. Create session
      const session = await api.createSession()

      // 3. Persist agentId/sessionId
      onConfigUpdate({
        agentId: agent.id,
        sessionId: session.sessionId,
      })

      // 4. Create WebSocket and connect
      const ws = new ChatWebSocket()
      wsRef.current = ws
      ws.setSessionId(session.sessionId)
      ws.setActiveBackend(config.modelId ?? null)

      // Register handlers
      unsubConnectionRef.current = ws.onConnection((connected) => {
        setIsConnected(connected)
      })
      registerMessageHandler(ws)

      ws.connect(session.sessionId)

      setInitializing(false)
    } catch (err) {
      setInitializing(false)
      const message = err instanceof Error ? err.message : 'Failed to initialize session'
      setError(message)
    }
  }, [
    componentId,
    config.systemPrompt,
    config.modelId,
    config.contextWindowSize,
    dataSource,
    initializing,
    cleanup,
    onConfigUpdate,
    registerMessageHandler,
  ])

  const sendImage = useCallback(
    (imageDataUrl: string, dataSource?: string) => {
      if (!wsRef.current) return

      // Add image placeholder to timeline
      const imgMsg: VlmMessage = {
        id: nextId(),
        type: 'image',
        content: imageDataUrl,
        timestamp: Date.now(),
        dataSource,
      }
      setMessages((prev) => [...prev, imgMsg])

      // Send via WebSocket
      wsRef.current.sendMessage('', [{ data: imageDataUrl }], [])
      setIsStreaming(true)
    },
    [],
  )

  const sendText = useCallback(
    (text: string) => {
      if (!wsRef.current) return

      // Add user message to timeline
      const userMsg: VlmMessage = {
        id: nextId(),
        type: 'user',
        content: text,
        timestamp: Date.now(),
      }
      setMessages((prev) => [...prev, userMsg])

      // Send via WebSocket
      wsRef.current.sendMessage(text, [], [])
      setIsStreaming(true)
    },
    [],
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
    sendText,
    isConnected,
  }
}
