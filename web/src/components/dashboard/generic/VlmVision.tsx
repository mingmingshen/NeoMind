/**
 * VLM Vision — Main Dashboard Widget
 *
 * The top-level component that assembles the VLM Vision dashboard widget.
 * It integrates the header, timeline, input bar, and config panel,
 * binding together useVlmSession, useVlmQueue, and useDataSource.
 */

import { useCallback, useEffect, useMemo, useRef } from 'react'
import {
  Camera,
  Loader2,
  AlertCircle,
  Activity,
  Clock,
  MessageSquare,
  ListOrdered,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { useDataSource } from '@/hooks/useDataSource'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { VlmTimeline } from './vlm-vision/VlmTimeline'
import { VlmInputBar } from './vlm-vision/VlmInputBar'
import { useVlmSession } from './vlm-vision/useVlmSession'
import { useVlmQueue } from './vlm-vision/useVlmQueue'
import type { VlmVisionConfig } from './vlm-vision/types'

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface VlmVisionProps {
  className?: string
  editMode?: boolean
  agentId?: string
  sessionId?: string
  dataSource?: any
  modelId?: string
  systemPrompt?: string
  contextWindowSize?: number
  /** Persist config changes back to dashboard component (survives refresh) */
  onConfigChange?: (config: Record<string, any>) => void
}

// ---------------------------------------------------------------------------
// Image detection utilities
// ---------------------------------------------------------------------------

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
      magic.every((b, i) => binary.charCodeAt(i) === b),
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

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function VlmVision({
  className,
  editMode = false,
  agentId,
  sessionId: sessionIdProp,
  dataSource: dataSourceProp,
  modelId: modelIdProp,
  systemPrompt: systemPromptProp,
  contextWindowSize: contextWindowSizeProp,
  onConfigChange,
}: VlmVisionProps) {
  // Stable component ID — locked on first render so it doesn't change
  // when agentId is saved back as a prop (which would trigger cleanup and delete the agent)
  const componentIdRef = useRef<string | null>(null)
  if (!componentIdRef.current) {
    componentIdRef.current = agentId || sessionIdProp || `vlm-${Date.now()}`
  }
  const componentId = componentIdRef.current

  // Config from props (persisted in dashboard store/localStorage), not Zustand memory
  const config: VlmVisionConfig = useMemo(
    () => ({
      agentId,
      modelId: modelIdProp,
      systemPrompt: systemPromptProp ||
        'You are a professional image analysis assistant. Carefully observe the image content, describe the scene, and point out any notable changes or anomalies.',
      contextWindowSize: contextWindowSizeProp || 10,
    }),
    [agentId, modelIdProp, systemPromptProp, contextWindowSizeProp],
  )

  // Persist config back to dashboard via onConfigChange (survives page refresh)
  const handleConfigUpdate = useCallback(
    (updates: Partial<VlmVisionConfig>) => {
      if (onConfigChange) {
        onConfigChange({
          modelId: updates.modelId ?? modelIdProp,
          systemPrompt: updates.systemPrompt ?? systemPromptProp,
          contextWindowSize: updates.contextWindowSize ?? contextWindowSizeProp,
          agentId: updates.agentId ?? agentId,
        })
      }
    },
    [onConfigChange, modelIdProp, systemPromptProp, contextWindowSizeProp, agentId],
  )

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
    isConnected,
  } = useVlmSession({
    componentId,
    config,
    dataSource: dataSourceProp,
    onConfigUpdate: handleConfigUpdate,
  })

  // ---- Image queue ----
  const handleQueueProcess = useCallback(
    (image: string) => {
      sendImage(image, dataSourceProp?.id)
    },
    [sendImage, dataSourceProp],
  )

  const { enqueue, completeProcessing, pending, isProcessing } =
    useVlmQueue(handleQueueProcess)

  // Complete queue processing when streaming ends
  const prevStreamingRef = useRef(isStreaming)
  useEffect(() => {
    if (prevStreamingRef.current && !isStreaming) {
      completeProcessing()
    }
    prevStreamingRef.current = isStreaming
  }, [isStreaming, completeProcessing])

  // ---- Data source binding ----
  const { data: dsData } = useDataSource<string>(dataSourceProp)

  // Detect and enqueue incoming images
  const lastEnqueuedRef = useRef<string | null>(null)
  useEffect(() => {
    if (!dsData || !isConnected) return

    const strVal = typeof dsData === 'string' ? dsData : String(dsData)
    if (!isBase64Image(strVal)) return
    if (strVal === lastEnqueuedRef.current) return

    const dataUrl = normalizeToDataUrl(strVal)
    lastEnqueuedRef.current = strVal
    enqueue(dataUrl)
  }, [dsData, isConnected, enqueue])

  // ---- Auto-init agent when dataSource is set but no agentId ----
  const hasDataSource = dataSourceProp !== undefined && dataSourceProp !== null
  const hasAgent = !!config.agentId

  useEffect(() => {
    // Only auto-init in non-edit mode with a data source and no existing agent
    if (!editMode && hasDataSource && !hasAgent && !sessionError) {
      initSession()
    }
  }, [editMode, hasDataSource, hasAgent, sessionError, initSession])

  // ---- Stats ----
  const aiMessages = messages.filter((m) => m.type === 'ai')
  const messageCount = messages.length
  const avgDuration =
    aiMessages.length > 0
      ? Math.round(
          aiMessages.reduce((sum, m) => sum + (m.duration ?? 0), 0) /
            aiMessages.length,
        )
      : 0

  // ---- Render: Empty state ----
  if (!hasDataSource && !editMode) {
    return (
      <div
        className={cn(
          'bg-card rounded-xl border shadow-sm overflow-hidden flex items-center justify-center min-h-[200px]',
          className,
        )}
      >
        <div className="text-center p-6">
          <Camera className="h-12 w-12 opacity-20 text-muted-foreground mx-auto mb-3" />
          <p className="text-sm text-muted-foreground">
            Configure a data source to start
          </p>
        </div>
      </div>
    )
  }

  // ---- Render: Initializing ----
  if (initializing && messageCount === 0) {
    return (
      <div
        className={cn(
          'bg-card rounded-xl border shadow-sm overflow-hidden flex items-center justify-center min-h-[200px]',
          className,
        )}
      >
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground mx-auto mb-3" />
          <p className="text-sm text-muted-foreground">Initializing...</p>
        </div>
      </div>
    )
  }

  // ---- Render: Error (no agent) ----
  if (sessionError && !hasAgent) {
    return (
      <div
        className={cn(
          'bg-card rounded-xl border shadow-sm overflow-hidden flex items-center justify-center min-h-[200px]',
          className,
        )}
      >
        <div className="text-center">
          <AlertCircle className="h-12 w-12 opacity-20 text-muted-foreground mx-auto mb-3" />
          <p className="text-sm text-muted-foreground mb-3">{sessionError}</p>
          <Button size="sm" variant="outline" onClick={() => initSession()}>
            Retry
          </Button>
        </div>
      </div>
    )
  }

  // ---- Render: Active layout ----
  return (
    <div
      className={cn(
        'bg-card rounded-xl border shadow-sm overflow-hidden flex flex-col w-full h-full',
        className,
      )}
    >
      {/* Header */}
      <div className="shrink-0 px-4 py-3 border-b border-border/50">
        <div className="flex items-start gap-3">
          {/* Avatar */}
          <div
            className={cn(
              'w-10 h-10 rounded-lg flex items-center justify-center shrink-0',
              isStreaming ? 'bg-blue-500/20' : 'bg-primary/10',
            )}
          >
            {isStreaming ? (
              <Loader2 className="h-5 w-5 text-blue-500 animate-spin" />
            ) : (
              <Camera className="h-5 w-5 text-primary" />
            )}
          </div>

          {/* Info */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h3 className="font-semibold text-sm truncate">VLM Vision</h3>
              {isStreaming ? (
                <Badge
                  variant="default"
                  className="text-[10px] h-5 gap-0.5 px-1.5"
                >
                  <Loader2 className="h-2.5 w-2.5 animate-spin" />
                  Analyzing
                </Badge>
              ) : isConnected ? (
                <Badge
                  variant="outline"
                  className="text-[10px] h-5 text-green-600 border-green-200"
                >
                  Live
                </Badge>
              ) : (
                <Badge variant="secondary" className="text-[10px] h-5">
                  Offline
                </Badge>
              )}
            </div>

            {/* Stats row */}
            <div className="flex items-center gap-3 text-[11px] text-muted-foreground">
              <span className="flex items-center gap-1">
                <MessageSquare className="h-3 w-3" />
                {messageCount} msgs
              </span>
              {avgDuration > 0 && (
                <span className="flex items-center gap-1">
                  <Clock className="h-3 w-3" />
                  {avgDuration}s avg
                </span>
              )}
              {config.modelName && (
                <span className="flex items-center gap-1">
                  <Activity className="h-3 w-3" />
                  {config.modelName}
                </span>
              )}
              {(pending > 0 || isProcessing) && (
                <span className="flex items-center gap-1 text-blue-500">
                  <ListOrdered className="h-3 w-3" />
                  {pending > 0 ? `${pending} queued` : 'Processing'}
                </span>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Content: Timeline */}
      <div className="flex-1 min-h-0 overflow-hidden">
        <VlmTimeline
          messages={messages}
          streamingContent={streamingContent}
          streamingMsgId={streamingMsgId}
          contextWindowSize={config.contextWindowSize}
        />
      </div>

      {/* Footer: Input Bar */}
      <VlmInputBar onSend={sendText} disabled={isStreaming || !isConnected} />
    </div>
  )
}
