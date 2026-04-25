/**
 * AI Analyst — Main Dashboard Widget
 *
 * The top-level component that assembles the AI Analyst dashboard widget.
 * It integrates the header, timeline, input bar, and config panel,
 * binding together useAnalystSession and useDataSource.
 *
 * Backend handles agent execution via event triggers (schedule_type: 'event').
 * Frontend listens for WebSocket events to display results in the timeline.
 */

import { useCallback, useEffect, useMemo, useRef } from 'react'
import {
  ScanEye,
  Loader2,
  AlertCircle,
  Activity,
  Clock,
  MessageSquare,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { useDataSource } from '@/hooks/useDataSource'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { AnalystTimeline } from './ai-analyst/AnalystTimeline'
import { AnalystInputBar } from './ai-analyst/AnalystInputBar'
import { useAnalystSession } from './ai-analyst/useAnalystSession'
import type { AiAnalystConfig } from './ai-analyst/types'

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface AiAnalystProps {
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
  // Default to JPEG (camera/vision frames are overwhelmingly JPEG)
  return `data:image/jpeg;base64,${str}`
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AiAnalyst({
  className,
  editMode = false,
  agentId,
  sessionId: sessionIdProp,
  dataSource: dataSourceProp,
  modelId: modelIdProp,
  systemPrompt: systemPromptProp,
  contextWindowSize: contextWindowSizeProp,
  onConfigChange,
}: AiAnalystProps) {
  // Stable component ID — locked on first render so it doesn't change
  // when agentId is saved back as a prop (which would trigger cleanup and delete the agent)
  const componentIdRef = useRef<string | null>(null)
  if (!componentIdRef.current) {
    componentIdRef.current = agentId || sessionIdProp || `analyst-${Date.now()}`
  }
  const componentId = componentIdRef.current

  /** Get a display label for the data source (handles both single and multi-source) */
  const dataSourceLabel = useMemo(() => {
    if (!dataSourceProp) return undefined
    if (Array.isArray(dataSourceProp)) {
      const labels = dataSourceProp.map((ds: any) =>
        ds.extensionMetric?.replace('produce:', '')
        || ds.metricId
        || ds.property
        || ds.extensionId
      ).filter(Boolean)
      return labels.length > 0 ? labels.join(', ') : undefined
    }
    return (dataSourceProp as any)?.id
  }, [dataSourceProp])

  // Config from props (persisted in dashboard store/localStorage), not Zustand memory
  const config: AiAnalystConfig = useMemo(
    () => ({
      agentId,
      modelId: modelIdProp,
      systemPrompt: systemPromptProp ||
        'You are a professional data analysis assistant. Analyze the provided data — images, metrics, or structured data — describe what you observe, and point out any notable patterns, changes, or anomalies.',
      contextWindowSize: contextWindowSizeProp || 10,
    }),
    [agentId, modelIdProp, systemPromptProp, contextWindowSizeProp],
  )

  // Persist config back to dashboard via onConfigChange (survives page refresh)
  const handleConfigUpdate = useCallback(
    (updates: Partial<AiAnalystConfig>) => {
      if (onConfigChange) {
        const newConfig: Record<string, any> = {}
        // Only include fields that have actual values (skip undefined)
        const agentIdVal = updates.agentId ?? agentId
        if (agentIdVal) newConfig.agentId = agentIdVal
        const modelIdVal = updates.modelId ?? modelIdProp
        if (modelIdVal) newConfig.modelId = modelIdVal
        const promptVal = updates.systemPrompt ?? systemPromptProp
        if (promptVal) newConfig.systemPrompt = promptVal
        const cwVal = updates.contextWindowSize ?? contextWindowSizeProp
        if (cwVal) newConfig.contextWindowSize = cwVal
        onConfigChange(newConfig)
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
    sendData,
    sendText,
    isConnected,
  } = useAnalystSession({
    componentId,
    config,
    dataSource: dataSourceProp,
    onConfigUpdate: handleConfigUpdate,
  })

  // ---- Data source binding ----
  const { data: dsData } = useDataSource<string>(dataSourceProp)

  // Show the latest data point in the timeline while LLM processes in background.
  // useDataSource returns different shapes depending on source type:
  //   - telemetry: DataPoint[] like [{timestamp, value}, ...]
  //   - device property: raw value (string, number, object)
  // We extract the latest value for display.
  const lastEnqueuedRef = useRef<string | null>(null)

  /** Extract the latest value from useDataSource output.
   *  Handles multiple shapes:
   *  - Single source data_points: [{timestamp, value}, ...] → latest .value
   *  - Multi-source array: [data_points_1, data_points_2, ...] → {metric_0: val, metric_1: val}
   *  - Raw scalar value (device property) → as-is
   */
  const extractLatestValue = useCallback(() => {
    if (dsData == null) return null

    // Multi-source: useDataSource returns [dataForSource1, dataForSource2, ...]
    // Each entry is either data_points[] or null
    if (Array.isArray(dsData) && dsData.length > 0 && Array.isArray(dsData[0])) {
      const summary: Record<string, unknown> = {}
      const dsList = Array.isArray(dataSourceProp) ? dataSourceProp : [dataSourceProp]
      dsData.forEach((sourceData: unknown, idx: number) => {
        const ds = dsList[idx] as any
        const label = ds?.extensionMetric?.replace('produce:', '')
          || ds?.metricId
          || ds?.property
          || `metric_${idx}`
        if (Array.isArray(sourceData) && sourceData.length > 0) {
          const latest = sourceData[0]
          summary[label] = latest && typeof latest === 'object' && 'value' in latest
            ? (latest as { value: unknown }).value
            : latest
        } else if (sourceData != null) {
          summary[label] = sourceData
        }
      })
      return Object.keys(summary).length > 0 ? summary : null
    }

    // Single source: data_points array
    if (Array.isArray(dsData) && dsData.length > 0) {
      const point = dsData[0]
      if (point && typeof point === 'object' && 'value' in point) {
        return (point as { value: unknown }).value
      }
      return point
    }

    // Raw scalar
    return dsData
  }, [dsData, dataSourceProp])

  // Only send data to timeline during active execution rounds.
  // Data outside of execution is meaningless noise.
  useEffect(() => {
    if (editMode || dsData == null || !isStreaming) return
    const latestValue = extractLatestValue()
    if (latestValue == null) return

    // Deduplicate: skip if the value hasn't changed
    const strVal = typeof latestValue === 'string' ? latestValue : JSON.stringify(latestValue)
    if (strVal === lastEnqueuedRef.current || strVal.length < 1) return
    lastEnqueuedRef.current = strVal

    if (typeof latestValue === 'string' && isBase64Image(latestValue)) {
      sendImage(normalizeToDataUrl(latestValue), dataSourceLabel)
    } else {
      sendData(latestValue, dataSourceLabel)
    }
    // NOTE: intentionally omits sendImage/sendData from deps to avoid re-triggers
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editMode, dsData, isStreaming, dataSourceProp])

  // When a new execution round starts, ensure data appears BEFORE the streaming UI.
  // Reset dedup and force-send current data so it's inserted before the streaming placeholder.
  const prevStreamingRef = useRef(false)
  useEffect(() => {
    if (!isStreaming || prevStreamingRef.current || editMode || dsData == null) {
      prevStreamingRef.current = isStreaming
      return
    }
    prevStreamingRef.current = true
    lastEnqueuedRef.current = null

    const latestValue = extractLatestValue()
    if (latestValue == null) return

    // Set dedup AFTER force-send so the normal dsData effect won't duplicate
    const strVal = typeof latestValue === 'string' ? latestValue : JSON.stringify(latestValue)

    if (typeof latestValue === 'string' && isBase64Image(latestValue)) {
      sendImage(normalizeToDataUrl(latestValue), dataSourceLabel)
    } else {
      sendData(latestValue, dataSourceLabel)
    }
    lastEnqueuedRef.current = strVal
    // NOTE: only depends on isStreaming — fires once per round start
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isStreaming, editMode, dsData])

  // ---- Auto-init agent when dataSource is set but no agentId ----
  const hasDataSource = dataSourceProp !== undefined && dataSourceProp !== null
  const hasAgent = !!config.agentId
  const initCalledRef = useRef(false)

  useEffect(() => {
    // Only auto-init in non-edit mode with a data source and no existing agent
    // Use ref guard to prevent double-call from StrictMode or state cascading
    if (!editMode && hasDataSource && !hasAgent && !sessionError && !initCalledRef.current) {
      initCalledRef.current = true
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
          <div className="w-12 h-12 rounded-lg bg-primary/10 flex items-center justify-center mx-auto mb-3">
            <ScanEye className="h-6 w-6 text-primary" />
          </div>
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
              <ScanEye className="h-5 w-5 text-primary" />
            )}
          </div>

          {/* Info */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h3 className="font-semibold text-sm truncate">AI Analyst</h3>
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
            </div>
          </div>
        </div>
      </div>

      {/* Content: Timeline */}
      <div className="flex-1 min-h-0 overflow-hidden">
        <AnalystTimeline
          messages={messages}
          streamingContent={streamingContent}
          streamingMsgId={streamingMsgId}
          contextWindowSize={config.contextWindowSize}
        />
      </div>

      {/* Footer: Input Bar */}
      <AnalystInputBar onSend={sendText} disabled={isStreaming || !isConnected} />
    </div>
  )
}
