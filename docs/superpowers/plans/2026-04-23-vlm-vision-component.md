# VLM Vision Component Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a VLM Vision dashboard business component that binds to image data sources, auto-triggers VLM inference, and displays results in a timeline chat.

**Architecture:** Pure frontend — no backend changes. Creates Agent + Session via existing APIs, streams responses via `ChatWebSocket` from `web/src/lib/websocket.ts`. Queue hook drops intermediate frames. Zustand slice stores per-component config.

**Tech Stack:** React 18, TypeScript, Zustand, Tailwind CSS, Lucide icons, shadcn/ui components

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `web/src/components/dashboard/generic/vlm-vision/types.ts` | Type definitions for messages, config |
| Create | `web/src/store/slices/vlmVisionSlice.ts` | Zustand slice: per-componentId config |
| Modify | `web/src/store/index.ts` | Register new slice |
| Create | `web/src/components/dashboard/generic/vlm-vision/useVlmQueue.ts` | Drop-intermediate-frame queue hook |
| Create | `web/src/components/dashboard/generic/vlm-vision/useVlmModels.ts` | Fetch vision-capable models |
| Create | `web/src/components/dashboard/generic/vlm-vision/useVlmSession.ts` | Agent/session lifecycle + WebSocket streaming |
| Create | `web/src/components/dashboard/generic/vlm-vision/VlmMessageBubble.tsx` | Single message bubble (image/ai/user) |
| Create | `web/src/components/dashboard/generic/vlm-vision/VlmTimeline.tsx` | Timeline message list with ScrollArea |
| Create | `web/src/components/dashboard/generic/vlm-vision/VlmInputBar.tsx` | Bottom input bar |
| Create | `web/src/components/dashboard/generic/vlm-vision/VlmConfigPanel.tsx` | Config dialog |
| Create | `web/src/components/dashboard/generic/VlmVision.tsx` | Main component entry point |
| Modify | `web/src/components/dashboard/registry/registry.ts` | Register vlm-vision component |
| Modify | `web/src/components/dashboard/registry/ComponentRenderer.tsx` | Add lazy import + render mapping |

---

### Task 1: Types + Zustand Slice

**Files:**
- Create: `web/src/components/dashboard/generic/vlm-vision/types.ts`
- Create: `web/src/store/slices/vlmVisionSlice.ts`
- Modify: `web/src/store/index.ts`

- [ ] **Step 1: Create types.ts**

```typescript
// web/src/components/dashboard/generic/vlm-vision/types.ts

export interface VlmVisionConfig {
  agentId?: string
  sessionId?: string
  modelId?: string
  modelName?: string
  systemPrompt: string
  contextWindowSize: number
}

export type VlmMessageType = 'image' | 'ai' | 'user' | 'error'

export interface VlmMessage {
  id: string
  type: VlmMessageType
  content: string
  timestamp: number
  dataSource?: string
  modelName?: string
  duration?: number
  isStreaming?: boolean
}

export interface VisionModel {
  id: string
  name: string
  backendId: string
  backendName: string
}

export const DEFAULT_SYSTEM_PROMPT =
  'You are a professional image analysis assistant. Carefully observe the image content, describe the scene, and point out any notable changes or anomalies.'
```

- [ ] **Step 2: Create vlmVisionSlice.ts**

Follow the exact `StateCreator` pattern from `llmBackendSlice.ts`:

```typescript
// web/src/store/slices/vlmVisionSlice.ts

import type { StateCreator } from 'zustand'
import type { VlmVisionConfig } from '@/components/dashboard/generic/vlm-vision/types'

export interface VlmVisionState {
  vlmConfigs: Record<string, VlmVisionConfig>
}

export interface VlmVisionSlice extends VlmVisionState {
  getVlmConfig: (componentId: string) => VlmVisionConfig | undefined
  setVlmConfig: (componentId: string, config: Partial<VlmVisionConfig>) => void
  clearVlmConfig: (componentId: string) => void
}

const DEFAULT_CONFIG: VlmVisionConfig = {
  systemPrompt: 'You are a professional image analysis assistant. Carefully observe the image content, describe the scene, and point out any notable changes or anomalies.',
  contextWindowSize: 10,
}

export const createVlmVisionSlice: StateCreator<
  VlmVisionSlice,
  [],
  [],
  VlmVisionSlice
> = (set, get) => ({
  vlmConfigs: {},

  getVlmConfig: (componentId) => {
    return get().vlmConfigs[componentId]
  },

  setVlmConfig: (componentId, config) => {
    set((state) => ({
      vlmConfigs: {
        ...state.vlmConfigs,
        [componentId]: {
          ...(state.vlmConfigs[componentId] || DEFAULT_CONFIG),
          ...config,
        },
      },
    }))
  },

  clearVlmConfig: (componentId) => {
    set((state) => {
      const { [componentId]: _, ...rest } = state.vlmConfigs
      return { vlmConfigs: rest }
    })
  },
})
```

- [ ] **Step 3: Register slice in store**

Modify `web/src/store/index.ts`:
1. Import: `import { createVlmVisionSlice, VlmVisionSlice } from './slices/vlmVisionSlice'`
2. Add to `NeoMindStore` type: `& VlmVisionSlice`
3. Add to store creator: `...createVlmVisionSlice(set, get, api),`

- [ ] **Step 4: Commit**

```bash
git add web/src/components/dashboard/generic/vlm-vision/types.ts web/src/store/slices/vlmVisionSlice.ts web/src/store/index.ts
git commit -m "feat(vlm-vision): add types and Zustand slice"
```

---

### Task 2: useVlmQueue Hook

**Files:**
- Create: `web/src/components/dashboard/generic/vlm-vision/useVlmQueue.ts`

- [ ] **Step 1: Implement useVlmQueue**

```typescript
// web/src/components/dashboard/generic/vlm-vision/useVlmQueue.ts

import { useRef, useCallback, useState } from 'react'

const PROCESSING_TIMEOUT_MS = 120_000

export function useVlmQueue(
  onProcess: (image: string) => void,
) {
  const [pending, setPending] = useState(0)
  const [isProcessing, setIsProcessing] = useState(false)
  const pendingImageRef = useRef<string | null>(null)
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const processNext = useCallback(() => {
    if (pendingImageRef.current) {
      const image = pendingImageRef.current
      pendingImageRef.current = null
      setPending(0)
      setIsProcessing(true)

      // Timeout safeguard
      timeoutRef.current = setTimeout(() => {
        setIsProcessing(false)
        processNext()
      }, PROCESSING_TIMEOUT_MS)

      onProcess(image)
    } else {
      setIsProcessing(false)
    }
  }, [onProcess])

  const enqueue = useCallback((image: string) => {
    if (isProcessing) {
      pendingImageRef.current = image
      setPending(1)
    } else {
      setIsProcessing(true)
      setPending(0)

      timeoutRef.current = setTimeout(() => {
        setIsProcessing(false)
        processNext()
      }, PROCESSING_TIMEOUT_MS)

      onProcess(image)
    }
  }, [isProcessing, onProcess, processNext])

  const completeProcessing = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
      timeoutRef.current = null
    }
    setIsProcessing(false)
    processNext()
  }, [processNext])

  return { enqueue, completeProcessing, pending, isProcessing }
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/components/dashboard/generic/vlm-vision/useVlmQueue.ts
git commit -m "feat(vlm-vision): add useVlmQueue hook with drop-intermediate-frame"
```

---

### Task 3: useVlmModels Hook

**Files:**
- Create: `web/src/components/dashboard/generic/vlm-vision/useVlmModels.ts`

- [ ] **Step 1: Implement useVlmModels**

```typescript
// web/src/components/dashboard/generic/vlm-vision/useVlmModels.ts

import { useState, useEffect, useCallback } from 'react'
import { api } from '@/lib/api'
import type { VisionModel } from './types'

// Model name patterns that indicate vision capability
const VISION_MODEL_PATTERNS = ['vl', 'vision', 'llava', 'bakllava', 'qwen-vl']

function isVisionModel(modelName: string): boolean {
  const lower = modelName.toLowerCase()
  return VISION_MODEL_PATTERNS.some((p) => lower.includes(p))
}

export function useVlmModels() {
  const [models, setModels] = useState<VisionModel[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const loadModels = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const resp = await api.listLlmBackends({ active_only: true })
      const backends = resp.backends || resp || []
      const visionModels: VisionModel[] = []

      for (const backend of Array.isArray(backends) ? backends : []) {
        const backendId = backend.id
        const backendName = backend.name || backendId

        // Check backend-level multimodal support
        if (backend.capabilities?.multimodal || backend.supports_multimodal) {
          // If backend reports multimodal, include its default model
          if (backend.model || backend.default_model) {
            visionModels.push({
              id: backend.model || backend.default_model,
              name: backend.model || backend.default_model,
              backendId,
              backendName,
            })
          }
        }

        // For Ollama backends, try to list models and filter by vision capability
        if (backend.type === 'ollama' || backend.backend_type === 'ollama') {
          try {
            const modelsResp = await api.getOllamaModels(backendId)
            const modelList = modelsResp.models || []
            for (const m of modelList) {
              if (isVisionModel(m.name || m.model || '')) {
                visionModels.push({
                  id: m.name || m.model,
                  name: m.name || m.model,
                  backendId,
                  backendName,
                })
              }
            }
          } catch {
            // Skip if model listing fails for this backend
          }
        }
      }

      setModels(visionModels)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load models')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    loadModels()
  }, [loadModels])

  return { models, loading, error, reload: loadModels }
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/components/dashboard/generic/vlm-vision/useVlmModels.ts
git commit -m "feat(vlm-vision): add useVlmModels hook for vision model listing"
```

---

### Task 4: useVlmSession Hook

**Files:**
- Create: `web/src/components/dashboard/generic/vlm-vision/useVlmSession.ts`

This is the core hook. It manages Agent/Session creation and the ChatWebSocket lifecycle.

- [ ] **Step 1: Implement useVlmSession**

Key implementation notes:
- Uses `ChatWebSocket` from `web/src/lib/websocket.ts` — singleton class with `connect(sessionId)`, `sendMessage(content, images?, skills?)`, `onMessage(handler)`
- `onMessage` returns an unsubscribe function
- `ServerMessage` types: `Content` (streaming text), `Thinking`, `end` (stream complete), `Error`
- Creates Agent via `api.createAgent({ name, user_prompt, llm_backend_id })`
- Creates Session via `api.createSession()`
- Loads history via `api.getSessionHistory(sessionId)`

```typescript
// web/src/components/dashboard/generic/vlm-vision/useVlmSession.ts

import { useState, useRef, useCallback, useEffect } from 'react'
import { ChatWebSocket } from '@/lib/websocket'
import { api } from '@/lib/api'
import type { ServerMessage } from '@/types'
import type { VlmMessage, VlmVisionConfig } from './types'

interface UseVlmSessionParams {
  componentId: string
  config: VlmVisionConfig
  onConfigUpdate: (updates: Partial<VlmVisionConfig>) => void
}

export function useVlmSession({ componentId, config, onConfigUpdate }: UseVlmSessionParams) {
  const [messages, setMessages] = useState<VlmMessage[]>([])
  const [isStreaming, setIsStreaming] = useState(false)
  const [streamingContent, setStreamingContent] = useState<string>('')
  const [streamingMsgId, setStreamingMsgId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [initializing, setInitializing] = useState(false)

  const wsRef = useRef<ChatWebSocket | null>(null)
  const unsubMessageRef = useRef<(() => void) | null>(null)
  const unsubConnectionRef = useRef<(() => void) | null>(null)
  const currentStreamStartRef = useRef<number>(0)

  // Initialize agent + session + websocket
  const initSession = useCallback(async () => {
    if (initializing || config.sessionId) return
    setInitializing(true)
    setError(null)

    try {
      // 1. Create Agent
      const agentResp = await api.createAgent({
        name: `vlm-vision-${componentId.slice(-6)}`,
        user_prompt: config.systemPrompt,
        llm_backend_id: config.modelId,
      })
      const agentId = agentResp.id

      // 2. Create Session
      const sessionResp = await api.createSession()
      const sessionId = sessionResp.sessionId

      // 3. Persist to store
      onConfigUpdate({ agentId, sessionId })

      // 4. Connect WebSocket
      const ws = new ChatWebSocket()
      ws.connect(sessionId)
      wsRef.current = ws

      // 5. Listen for messages
      const unsubMsg = ws.onMessage((msg: ServerMessage) => {
        handleServerMessage(msg)
      })
      unsubMessageRef.current = unsubMsg

      const unsubConn = ws.onConnection(() => {})
      unsubConnectionRef.current = unsubConn

      // 6. Load existing history
      try {
        const history = await api.getSessionHistory(sessionId)
        const historyMessages = convertHistoryToMessages(history)
        setMessages(historyMessages)
      } catch {
        // No history yet, that's fine
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to initialize session')
    } finally {
      setInitializing(false)
    }
  }, [componentId, config.sessionId, config.systemPrompt, config.modelId, initializing, onConfigUpdate])

  // Handle WebSocket messages
  const handleServerMessage = useCallback((msg: ServerMessage) => {
    switch (msg.type) {
      case 'Content': {
        setStreamingContent((prev) => prev + msg.content)
        break
      }
      case 'end': {
        setStreamingContent((prev) => {
          const finalContent = prev
          // Create final message
          const msgId = `ai-${Date.now()}`
          setMessages((prevMsgs) => [
            ...prevMsgs,
            {
              id: msgId,
              type: 'ai',
              content: finalContent,
              timestamp: Date.now(),
              modelName: config.modelName,
              duration: Date.now() - currentStreamStartRef.current,
            },
          ])
          return ''
        })
        setIsStreaming(false)
        setStreamingMsgId(null)
        break
      }
      case 'Error': {
        const errMsg = (msg as any).message || (msg as any).content || 'Stream error'
        setMessages((prev) => [
          ...prev,
          {
            id: `error-${Date.now()}`,
            type: 'error',
            content: errMsg,
            timestamp: Date.now(),
          },
        ])
        setIsStreaming(false)
        setStreamingContent('')
        break
      }
    }
  }, [config.modelName])

  // Send image for VLM analysis
  const sendImage = useCallback((imageDataUrl: string, dataSource?: string) => {
    const ws = wsRef.current
    if (!ws || !ws.isConnected()) return

    // Add image message to timeline
    const imgMsgId = `img-${Date.now()}`
    setMessages((prev) => [
      ...prev,
      {
        id: imgMsgId,
        type: 'image',
        content: imageDataUrl,
        timestamp: Date.now(),
        dataSource,
      },
    ])

    // Start streaming
    setIsStreaming(true)
    currentStreamStartRef.current = Date.now()
    setStreamingMsgId(`ai-stream-${Date.now()}`)

    ws.sendMessage('', [{ data: imageDataUrl }], [])
  }, [])

  // Send text follow-up
  const sendText = useCallback((text: string) => {
    const ws = wsRef.current
    if (!ws || !ws.isConnected()) return

    setMessages((prev) => [
      ...prev,
      {
        id: `user-${Date.now()}`,
        type: 'user',
        content: text,
        timestamp: Date.now(),
      },
    ])

    setIsStreaming(true)
    currentStreamStartRef.current = Date.now()

    ws.sendMessage(text, [], [])
  }, [])

  // Cleanup
  useEffect(() => {
    return () => {
      unsubMessageRef.current?.()
      unsubConnectionRef.current?.()
    }
  }, [])

  // Reconnect on config change (new sessionId)
  useEffect(() => {
    if (config.sessionId && !wsRef.current) {
      const ws = new ChatWebSocket()
      ws.connect(config.sessionId)
      wsRef.current = ws

      const unsubMsg = ws.onMessage((msg: ServerMessage) => {
        handleServerMessage(msg)
      })
      unsubMessageRef.current = unsubMsg

      const unsubConn = ws.onConnection(() => {})
      unsubConnectionRef.current = unsubConn

      // Load history
      api.getSessionHistory(config.sessionId)
        .then((history) => {
          setMessages(convertHistoryToMessages(history))
        })
        .catch(() => {})
    }
  }, [config.sessionId, handleServerMessage])

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
    isConnected: wsRef.current?.isConnected() ?? false,
  }
}

// Convert session history to VlmMessage[]
function convertHistoryToMessages(history: any): VlmMessage[] {
  const msgs: VlmMessage[] = []
  const messages = history?.messages || history?.history || []
  for (const m of Array.isArray(messages) ? messages : []) {
    if (m.role === 'user') {
      if (m.images && m.images.length > 0) {
        msgs.push({
          id: m.id || `hist-img-${msgs.length}`,
          type: 'image',
          content: m.images[0].data || m.images[0],
          timestamp: m.timestamp ? m.timestamp * 1000 : Date.now(),
        })
      }
      if (m.content && (!m.images || m.images.length === 0)) {
        msgs.push({
          id: m.id || `hist-user-${msgs.length}`,
          type: 'user',
          content: m.content,
          timestamp: m.timestamp ? m.timestamp * 1000 : Date.now(),
        })
      }
    } else if (m.role === 'assistant' && m.content) {
      msgs.push({
        id: m.id || `hist-ai-${msgs.length}`,
        type: 'ai',
        content: m.content,
        timestamp: m.timestamp ? m.timestamp * 1000 : Date.now(),
      })
    }
  }
  return msgs
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/components/dashboard/generic/vlm-vision/useVlmSession.ts
git commit -m "feat(vlm-vision): add useVlmSession hook with WebSocket streaming"
```

---

### Task 5: VlmMessageBubble Component

**Files:**
- Create: `web/src/components/dashboard/generic/vlm-vision/VlmMessageBubble.tsx`

- [ ] **Step 1: Implement VlmMessageBubble**

Follow exact Lucide icon + Tailwind patterns from AgentMonitorWidget:

```tsx
// web/src/components/dashboard/generic/vlm-vision/VlmMessageBubble.tsx

import { useState } from 'react'
import {
  Camera,
  Bot,
  User,
  AlertCircle,
  Loader2,
  Maximize2,
  Clock,
  Cpu,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import type { VlmMessage } from './types'

interface VlmMessageBubbleProps {
  message: VlmMessage
  streamingContent?: string
}

export function VlmMessageBubble({ message, streamingContent }: VlmMessageBubbleProps) {
  const [fullscreenImage, setFullscreenImage] = useState<string | null>(null)
  const isStreaming = message.isStreaming || !!streamingContent
  const displayContent = streamingContent || message.content

  const formatTime = (ts: number) =>
    new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })

  const formatDuration = (ms: number) =>
    ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`

  // Image data source entry
  if (message.type === 'image') {
    return (
      <>
        <div className="flex items-start gap-2">
          <div className="w-6 h-6 rounded-md bg-blue-500/20 flex items-center justify-center shrink-0 mt-0.5">
            <Camera className="h-3 w-3 text-blue-500" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex justify-between items-center">
              <span className="text-[10px] text-muted-foreground truncate">
                {message.dataSource || 'Data source'}
              </span>
              <span className="text-[10px] text-muted-foreground">{formatTime(message.timestamp)}</span>
            </div>
            <div
              className="mt-1 rounded-lg overflow-hidden border border-border/50 bg-muted/30 cursor-pointer max-w-[240px]"
              onClick={() => setFullscreenImage(message.content)}
            >
              <img
                src={message.content}
                alt="Data source image"
                className="w-full h-auto max-h-[120px] object-cover"
              />
            </div>
          </div>
        </div>
        {fullscreenImage && (
          <Dialog open={!!fullscreenImage} onOpenChange={() => setFullscreenImage(null)}>
            <DialogContent className="max-w-4xl p-2">
              <img
                src={fullscreenImage}
                alt="Fullscreen"
                className="w-full h-auto max-h-[80vh] object-contain rounded-lg"
              />
            </DialogContent>
          </Dialog>
        )}
      </>
    )
  }

  // AI response
  if (message.type === 'ai') {
    return (
      <div className="flex items-start gap-2">
        <div className="w-6 h-6 rounded-md bg-purple-500/20 flex items-center justify-center shrink-0 mt-0.5">
          <Bot className="h-3 w-3 text-purple-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex justify-between items-center">
            <span className="text-[10px] text-purple-500 flex items-center gap-1">
              <Cpu className="h-2.5 w-2.5" />
              {message.modelName || 'VLM'}
            </span>
            {isStreaming ? (
              <span className="text-[10px] text-purple-500 flex items-center gap-1">
                <Loader2 className="h-2.5 w-2.5 animate-spin" />
                streaming...
              </span>
            ) : (
              <span className="text-[10px] text-muted-foreground flex items-center gap-1">
                <Clock className="h-2.5 w-2.5" />
                {message.duration ? formatDuration(message.duration) : formatTime(message.timestamp)}
              </span>
            )}
          </div>
          <div className="mt-1 rounded-lg px-3 py-2 bg-purple-500/6 border border-purple-500/15">
            <p className="text-xs text-foreground/90 leading-relaxed whitespace-pre-wrap">
              {displayContent}
              {isStreaming && <span className="text-purple-500">|</span>}
            </p>
          </div>
        </div>
      </div>
    )
  }

  // User follow-up
  if (message.type === 'user') {
    return (
      <div className="flex items-start gap-2 justify-end">
        <div className="flex-1 min-w-0 flex flex-col items-end">
          <div className="flex justify-between items-center w-full">
            <span className="text-[10px] text-muted-foreground">{formatTime(message.timestamp)}</span>
            <span className="text-[10px] text-purple-500/70 flex items-center gap-1">
              <User className="h-2.5 w-2.5" />
              User
            </span>
          </div>
          <div className="mt-1 rounded-lg px-3 py-2 bg-purple-500/10 border border-purple-500/20 max-w-[260px]">
            <p className="text-xs text-foreground/90 leading-relaxed">{message.content}</p>
          </div>
        </div>
      </div>
    )
  }

  // Error
  if (message.type === 'error') {
    return (
      <div className="flex items-start gap-2">
        <div className="w-6 h-6 rounded-md bg-red-500/20 flex items-center justify-center shrink-0 mt-0.5">
          <AlertCircle className="h-3 w-3 text-red-500" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="rounded-lg px-3 py-2 bg-red-500/10 border border-red-500/20">
            <p className="text-xs text-red-500">{message.content}</p>
          </div>
        </div>
      </div>
    )
  }

  return null
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/components/dashboard/generic/vlm-vision/VlmMessageBubble.tsx
git commit -m "feat(vlm-vision): add VlmMessageBubble component"
```

---

### Task 6: VlmTimeline + VlmInputBar

**Files:**
- Create: `web/src/components/dashboard/generic/vlm-vision/VlmTimeline.tsx`
- Create: `web/src/components/dashboard/generic/vlm-vision/VlmInputBar.tsx`

- [ ] **Step 1: Implement VlmTimeline**

```tsx
// web/src/components/dashboard/generic/vlm-vision/VlmTimeline.tsx

import { useRef, useEffect } from 'react'
import { ScrollArea } from '@/components/ui/scroll-area'
import { CircleDot } from 'lucide-react'
import { VlmMessageBubble } from './VlmMessageBubble'
import type { VlmMessage } from './types'

interface VlmTimelineProps {
  messages: VlmMessage[]
  streamingContent: string
  streamingMsgId: string | null
  contextWindowSize: number
}

export function VlmTimeline({ messages, streamingContent, streamingMsgId, contextWindowSize }: VlmTimelineProps) {
  const bottomRef = useRef<HTMLDivElement>(null)
  const scrollAreaRef = useRef<HTMLDivElement>(null)

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    // Smart scroll: only auto-scroll if user is near the bottom
    const el = scrollAreaRef.current?.querySelector('[data-radix-scroll-area-viewport]')
    if (el) {
      const isNearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 100
      if (isNearBottom) {
        bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
      }
    }
  }, [messages, streamingContent])

  // Apply context window — show last N messages
  const visibleMessages = messages.slice(-contextWindowSize * 2) // *2 because image+ai pairs

  if (messages.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-8 text-center h-full">
        <CircleDot className="h-8 w-8 text-muted-foreground opacity-30 mb-2" />
        <p className="text-xs text-muted-foreground">No activity yet</p>
      </div>
    )
  }

  return (
    <ScrollArea className="h-full" ref={scrollAreaRef}>
      <div className="p-3 space-y-2">
        {visibleMessages.map((msg) => (
          <VlmMessageBubble
            key={msg.id}
            message={msg}
            streamingContent={msg.id === streamingMsgId ? streamingContent : undefined}
          />
        ))}
        {/* Streaming AI message (not yet finalized) */}
        {streamingMsgId && !messages.find((m) => m.id === streamingMsgId) && streamingContent && (
          <VlmMessageBubble
            message={{
              id: streamingMsgId,
              type: 'ai',
              content: '',
              timestamp: Date.now(),
              isStreaming: true,
            }}
            streamingContent={streamingContent}
          />
        )}
        <div ref={bottomRef} />
      </div>
    </ScrollArea>
  )
}
```

- [ ] **Step 2: Implement VlmInputBar**

```tsx
// web/src/components/dashboard/generic/vlm-vision/VlmInputBar.tsx

import { useState, useCallback } from 'react'
import { Send, Loader2 } from 'lucide-react'
import { Textarea } from '@/components/ui/textarea'
import { Button } from '@/components/ui/button'

interface VlmInputBarProps {
  onSend: (text: string) => void
  disabled?: boolean
}

export function VlmInputBar({ onSend, disabled }: VlmInputBarProps) {
  const [text, setText] = useState('')

  const handleSend = useCallback(() => {
    if (!text.trim() || disabled) return
    onSend(text.trim())
    setText('')
  }, [text, disabled, onSend])

  return (
    <div className="shrink-0 p-3 border-t border-border/50 bg-muted/20">
      <div className="flex gap-2">
        <Textarea
          placeholder="Send a follow-up message..."
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault()
              handleSend()
            }
          }}
          className="min-h-[36px] max-h-[72px] h-9 text-xs placeholder:text-xs resize-none py-1.5"
          disabled={disabled}
        />
        <Button
          size="sm"
          onClick={handleSend}
          disabled={!text.trim() || disabled}
          className="h-9 px-3 shrink-0"
        >
          {disabled ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Send className="h-3.5 w-3.5" />
          )}
        </Button>
      </div>
    </div>
  )
}
```

- [ ] **Step 3: Commit**

```bash
git add web/src/components/dashboard/generic/vlm-vision/VlmTimeline.tsx web/src/components/dashboard/generic/vlm-vision/VlmInputBar.tsx
git commit -m "feat(vlm-vision): add VlmTimeline and VlmInputBar components"
```

---

### Task 7: VlmConfigPanel

**Files:**
- Create: `web/src/components/dashboard/generic/vlm-vision/VlmConfigPanel.tsx`

- [ ] **Step 1: Implement VlmConfigPanel**

```tsx
// web/src/components/dashboard/generic/vlm-vision/VlmConfigPanel.tsx

import { useState, useEffect } from 'react'
import { Settings2, Cpu, MessageSquare, Gauge } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { Slider } from '@/components/ui/slider'
import { Badge } from '@/components/ui/badge'
import { useVlmModels } from './useVlmModels'
import type { VlmVisionConfig } from './types'

interface VlmConfigPanelProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  config: VlmVisionConfig
  onSave: (config: Partial<VlmVisionConfig>) => void
  dataSource?: string
}

export function VlmConfigPanel({ open, onOpenChange, config, onSave, dataSource }: VlmConfigPanelProps) {
  const [systemPrompt, setSystemPrompt] = useState(config.systemPrompt)
  const [modelId, setModelId] = useState(config.modelId || '')
  const [contextWindowSize, setContextWindowSize] = useState(config.contextWindowSize)
  const { models, loading } = useVlmModels()

  useEffect(() => {
    setSystemPrompt(config.systemPrompt)
    setModelId(config.modelId || '')
    setContextWindowSize(config.contextWindowSize)
  }, [config])

  const handleSave = () => {
    const selectedModel = models.find((m) => m.id === modelId)
    onSave({
      systemPrompt,
      modelId: modelId || undefined,
      modelName: selectedModel?.name,
      contextWindowSize,
    })
    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-base">
            <Settings2 className="h-4 w-4" />
            VLM Vision Configuration
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* Data Source */}
          <div>
            <Label className="text-xs text-muted-foreground">Data Source</Label>
            <div className="mt-1 px-3 py-2 rounded-lg bg-muted/30 border border-border/50 text-sm">
              {dataSource || 'Not configured (use component settings)'}
            </div>
          </div>

          {/* Model Selection */}
          <div>
            <Label className="text-xs text-muted-foreground flex items-center gap-1">
              <Cpu className="h-3 w-3" />
              Vision Model
            </Label>
            {loading ? (
              <div className="mt-1 text-xs text-muted-foreground">Loading models...</div>
            ) : models.length === 0 ? (
              <div className="mt-1 text-xs text-yellow-600">No vision models found. Configure an LLM backend with a vision model first.</div>
            ) : (
              <select
                value={modelId}
                onChange={(e) => setModelId(e.target.value)}
                className="mt-1 w-full rounded-lg border border-border/50 bg-background px-3 py-2 text-sm"
              >
                <option value="">Select a model</option>
                {models.map((m) => (
                  <option key={`${m.backendId}-${m.id}`} value={m.id}>
                    {m.name} ({m.backendName})
                  </option>
                ))}
              </select>
            )}
          </div>

          {/* System Prompt */}
          <div>
            <Label className="text-xs text-muted-foreground flex items-center gap-1">
              <MessageSquare className="h-3 w-3" />
              System Prompt
            </Label>
            <Textarea
              value={systemPrompt}
              onChange={(e) => setSystemPrompt(e.target.value)}
              className="mt-1 min-h-[80px] text-xs"
              placeholder="Describe how the VLM should analyze images..."
            />
          </div>

          {/* Context Window */}
          <div>
            <Label className="text-xs text-muted-foreground flex items-center gap-1">
              <Gauge className="h-3 w-3" />
              Context Window
            </Label>
            <div className="mt-1 flex items-center gap-3">
              <Slider
                value={[contextWindowSize]}
                onValueChange={([v]) => setContextWindowSize(v)}
                min={1}
                max={20}
                step={1}
                className="flex-1"
              />
              <Badge variant="outline" className="text-xs h-6 px-2">
                {contextWindowSize}
              </Badge>
            </div>
            <p className="text-[10px] text-muted-foreground mt-1">
              Number of recent image+response pairs to display in timeline
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSave}>Save</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/components/dashboard/generic/vlm-vision/VlmConfigPanel.tsx
git commit -m "feat(vlm-vision): add VlmConfigPanel dialog"
```

---

### Task 8: VlmVision Main Component

**Files:**
- Create: `web/src/components/dashboard/generic/VlmVision.tsx`

- [ ] **Step 1: Implement VlmVision**

This is the main entry point. Follows AgentMonitorWidget layout pattern exactly.

```tsx
// web/src/components/dashboard/generic/VlmVision.tsx

import { useState, useCallback, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Camera,
  Loader2,
  AlertCircle,
  Settings2,
  MessageSquare,
  Clock,
  Cpu,
  CheckCircle2,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useDataSource } from '@/hooks/useDataSource'
import { useStore } from '@/store'
import { VlmTimeline } from './vlm-vision/VlmTimeline'
import { VlmInputBar } from './vlm-vision/VlmInputBar'
import { VlmConfigPanel } from './vlm-vision/VlmConfigPanel'
import { useVlmSession } from './vlm-vision/useVlmSession'
import { useVlmQueue } from './vlm-vision/useVlmQueue'
import { DEFAULT_SYSTEM_PROMPT } from './vlm-vision/types'

// Image detection utilities (from AgentMonitorWidget pattern)
const IMAGE_MAGIC_BYTES: Record<string, number[]> = {
  png: [0x89, 0x50, 0x4e, 0x47],
  jpeg: [0xff, 0xd8, 0xff],
  gif: [0x47, 0x49, 0x46],
  webp: [0x52, 0x49, 0x46, 0x46],
}

function isBase64Image(str: string): boolean {
  if (!str || str.length < 100) return false
  if (str.startsWith('data:image/')) return true
  if (str.startsWith('http://') || str.startsWith('https://')) return false
  try {
    const binary = atob(str.slice(0, 32))
    return Object.values(IMAGE_MAGIC_BYTES).some((magic) =>
      magic.every((b, i) => binary.charCodeAt(i) === b)
    )
  } catch {
    return false
  }
}

function normalizeToDataUrl(str: string): string {
  if (str.startsWith('data:image/')) return str
  try {
    const binary = atob(str.slice(0, 32))
    for (const [format, magic] of Object.entries(IMAGE_MAGIC_BYTES)) {
      if (magic.every((b, i) => binary.charCodeAt(i) === b)) {
        return `data:image/${format};base64,${str}`
      }
    }
  } catch {}
  return `data:image/png;base64,${str}`
}

interface VlmVisionProps {
  className?: string
  editMode?: boolean
  agentId?: string
  sessionId?: string
  dataSource?: any
}

export function VlmVision({ className, editMode = false, agentId, sessionId, dataSource }: VlmVisionProps) {
  const { t } = useTranslation('dashboardComponents')
  const [configOpen, setConfigOpen] = useState(false)

  // Generate stable componentId from props (dashboard provides a unique id)
  const componentId = agentId || sessionId || 'vlm-default'

  // Zustand store
  const vlmConfig = useStore((s) => s.vlmConfigs[componentId])
  const setVlmConfig = useStore((s) => s.setVlmConfig)

  const config = vlmConfig || {
    systemPrompt: DEFAULT_SYSTEM_PROMPT,
    contextWindowSize: 10,
    agentId,
    sessionId,
  }

  // Session hook
  const {
    messages,
    isStreaming,
    streamingContent,
    streamingMsgId,
    error: sessionError,
    initializing,
    initSession,
    sendImage,
    sendText,
  } = useVlmSession({
    componentId,
    config,
    onConfigUpdate: (updates) => setVlmConfig(componentId, updates),
  })

  // Queue hook
  const { enqueue, completeProcessing, pending, isProcessing } = useVlmQueue(
    useCallback(
      (image: string) => {
        const dsId = dataSource?.id || dataSource?.dataSourceId
        sendImage(image, dsId)
      },
      [sendImage, dataSource]
    )
  )

  // Complete processing when streaming ends
  useEffect(() => {
    if (!isStreaming && isProcessing) {
      completeProcessing()
    }
  }, [isStreaming, isProcessing, completeProcessing])

  // Data source binding
  const { data: dsData } = useDataSource(dataSource, {
    enabled: !!dataSource && !!config.sessionId,
  })

  // Detect image data from data source and enqueue
  useEffect(() => {
    if (!dsData || !config.sessionId) return
    const value = typeof dsData === 'string' ? dsData : JSON.stringify(dsData)
    if (isBase64Image(value)) {
      enqueue(normalizeToDataUrl(value))
    }
  }, [dsData, config.sessionId, enqueue])

  // Auto-init session when dataSource is set but no session exists
  useEffect(() => {
    if (dataSource && !config.sessionId && !initializing && !sessionError) {
      initSession()
    }
  }, [dataSource, config.sessionId, initializing, sessionError, initSession])

  // Stats
  const messageCount = messages.length
  const aiMessages = messages.filter((m) => m.type === 'ai')
  const avgDuration = aiMessages.length > 0
    ? Math.round(aiMessages.reduce((sum, m) => sum + (m.duration || 0), 0) / aiMessages.length)
    : 0

  // Empty state — no data source
  if (!dataSource && !editMode) {
    return (
      <div className={cn('bg-card rounded-xl border shadow-sm overflow-hidden flex items-center justify-center min-h-[200px]', className)}>
        <div className="text-center p-6">
          <Camera className="h-12 w-12 opacity-20 text-muted-foreground mx-auto mb-3" />
          <p className="text-sm text-muted-foreground">Configure a data source to start</p>
        </div>
      </div>
    )
  }

  // Loading state
  if (initializing && messages.length === 0) {
    return (
      <div className={cn('bg-card rounded-xl border shadow-sm overflow-hidden flex items-center justify-center min-h-[200px]', className)}>
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mx-auto mb-3" />
          <p className="text-sm text-muted-foreground">{t('common:loading', 'Initializing...')}</p>
        </div>
      </div>
    )
  }

  // Error state
  if (sessionError && !config.sessionId) {
    return (
      <div className={cn('bg-card rounded-xl border shadow-sm overflow-hidden flex items-center justify-center min-h-[200px]', className)}>
        <div className="text-center">
          <AlertCircle className="h-12 w-12 opacity-20 text-red-500 mx-auto mb-3" />
          <p className="text-sm text-red-500 mb-2">{sessionError}</p>
          <Button size="sm" variant="outline" onClick={initSession}>Retry</Button>
        </div>
      </div>
    )
  }

  return (
    <>
      <div className={cn('bg-card rounded-xl border shadow-sm overflow-hidden flex flex-col w-full h-full', className)}>
        {/* Header */}
        <div className="shrink-0 px-4 py-3 border-b border-border/50">
          <div className="flex items-start gap-3">
            <div className={cn(
              'w-10 h-10 rounded-lg flex items-center justify-center shrink-0',
              isProcessing ? 'bg-blue-500/20' : 'bg-primary/10'
            )}>
              {isProcessing ? (
                <Loader2 className="h-5 w-5 text-blue-500 animate-spin" />
              ) : (
                <Camera className="h-5 w-5 text-primary" />
              )}
            </div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <h3 className="font-semibold text-sm truncate">VLM Vision</h3>
                {isStreaming ? (
                  <Badge variant="default" className="text-[10px] h-5 gap-0.5 px-1.5">
                    <Loader2 className="h-2.5 w-2.5 animate-spin" />
                    Analyzing
                  </Badge>
                ) : config.sessionId ? (
                  <Badge variant="outline" className="text-[10px] h-5 text-green-600 border-green-200">
                    <CheckCircle2 className="h-2.5 w-2.5 mr-0.5" />
                    Live
                  </Badge>
                ) : null}
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6 ml-auto"
                  onClick={() => setConfigOpen(true)}
                >
                  <Settings2 className="h-3.5 w-3.5" />
                </Button>
              </div>
              <div className="flex items-center gap-3 text-[11px] text-muted-foreground">
                <span className="flex items-center gap-1">
                  <MessageSquare className="h-3 w-3" />
                  {messageCount} msgs
                </span>
                {avgDuration > 0 && (
                  <span className="flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {avgDuration < 1000 ? `${avgDuration}ms` : `${(avgDuration / 1000).toFixed(1)}s`}
                  </span>
                )}
                {config.modelName && (
                  <span className="flex items-center gap-1">
                    <Cpu className="h-3 w-3" />
                    {config.modelName}
                  </span>
                )}
                {pending > 0 && (
                  <span className="text-yellow-600">Queue: {pending}</span>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Timeline */}
        <div className="flex-1 min-h-0 overflow-hidden">
          <VlmTimeline
            messages={messages}
            streamingContent={streamingContent}
            streamingMsgId={streamingMsgId}
            contextWindowSize={config.contextWindowSize}
          />
        </div>

        {/* Input Bar */}
        <VlmInputBar onSend={sendText} disabled={isStreaming} />
      </div>

      {/* Config Dialog */}
      <VlmConfigPanel
        open={configOpen}
        onOpenChange={setConfigOpen}
        config={config}
        onSave={(updates) => {
          setVlmConfig(componentId, updates)
          // If model or prompt changed, need to reinitialize
          if (updates.modelId !== config.modelId || updates.systemPrompt !== config.systemPrompt) {
            // Re-init will happen via useEffect detecting config change
            setVlmConfig(componentId, { sessionId: undefined, agentId: undefined })
          }
        }}
        dataSource={dataSource?.id || dataSource?.dataSourceId}
      />
    </>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/components/dashboard/generic/VlmVision.tsx
git commit -m "feat(vlm-vision): add main VlmVision component"
```

---

### Task 9: Registry Registration + ComponentRenderer Wiring

**Files:**
- Modify: `web/src/components/dashboard/registry/registry.ts`
- Modify: `web/src/components/dashboard/registry/ComponentRenderer.tsx`

- [ ] **Step 1: Add to registry.ts**

Add import at top:
```typescript
import { Camera } from 'lucide-react'
```

Add entry to `componentRegistry`:
```typescript
'vlm-vision': {
  type: 'vlm-vision',
  name: 'VLM Vision',
  description: 'Image analysis with Vision Language Model — auto-analyze data source images in a timeline chat',
  category: 'business',
  icon: Camera,
  sizeConstraints: getSizeConstraints('vlm-vision'),
  hasDataSource: true,
  hasDisplayConfig: false,
  hasActions: true,
  acceptsProp: (prop) => ['agentId', 'sessionId', 'className', 'editMode'].includes(prop),
  defaultProps: { agentId: undefined },
  variants: ['default'],
},
```

Add size constraint to `COMPONENT_SIZE_CONSTRAINTS`:
```typescript
'vlm-vision': { minW: 2, minH: 2, defaultW: 3, defaultH: 3, maxW: 6, maxH: 6 },
```

- [ ] **Step 2: Add to ComponentRenderer.tsx**

Add lazy import:
```typescript
const VlmVision = lazy(() => import('../generic/VlmVision').then((m) => ({ default: m.VlmVision })))
```

Add to `businessComponentMap`:
```typescript
'vlm-vision': VlmVision,
```

- [ ] **Step 3: Commit**

```bash
git add web/src/components/dashboard/registry/registry.ts web/src/components/dashboard/registry/ComponentRenderer.tsx
git commit -m "feat(vlm-vision): register component in dashboard registry"
```

---

### Task 10: Integration Test + Polish

- [ ] **Step 1: Start dev server and verify**

```bash
cd web && npm run dev
```

Open the dashboard, verify:
1. VLM Vision appears in the component palette
2. Can drag it onto the dashboard
3. Config dialog opens with settings
4. Component shows "Configure a data source" empty state

- [ ] **Step 2: Run type check**

```bash
cd web && npx tsc --noEmit
```

Fix any type errors.

- [ ] **Step 3: Run lint**

```bash
cd web && npx eslint src/components/dashboard/generic/VlmVision.tsx src/components/dashboard/generic/vlm-vision/ --max-warnings=0
```

Fix any lint issues.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat(vlm-vision): integration polish and type fixes"
```
