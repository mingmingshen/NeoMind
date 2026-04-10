/**
 * ToolCallVisualization - Displays AI tool/function calls
 * Supports multi-round grouping with collapsible sections
 */

import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Wrench, ChevronDown, Clock, CheckCircle2, Loader2, Code } from "lucide-react"
import { cn } from "@/lib/utils"
import type { ToolCall } from "@/types"

interface ToolCallVisualizationProps {
  toolCalls: ToolCall[]
  isStreaming?: boolean
  roundContents?: Record<number, string>
}

interface ToolCallWithDuration extends ToolCall {
  duration_ms?: number
}

type ToolStatus = "pending" | "running" | "completed"

function getToolStatus(hasResult: boolean, isStreaming: boolean): ToolStatus {
  if (hasResult) return "completed"
  if (isStreaming) return "running"
  return "pending"
}

function formatDuration(ms?: number): string {
  if (!ms) return ""
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

/** Extract a descriptive display name from tool call name + arguments */
function getToolDisplayName(name: string, args?: unknown): string {
  if (!args || typeof args !== 'object') return name
  const a = args as Record<string, unknown>
  const action = typeof a.action === 'string' ? a.action : ''
  if (!action) return name

  // Build "tool(action)" display name with contextual info
  const extras: string[] = []
  if (typeof a.device_id === 'string') extras.push(a.device_id)
  if (typeof a.metric === 'string') extras.push(a.metric)
  if (typeof a.command === 'string') extras.push(a.command)

  const suffix = extras.length > 0 ? ` ${extras.join(' · ')}` : ''
  return `${name}(${action})${suffix}`
}

function formatJson(data: unknown): string {
  if (typeof data === "string") {
    try {
      const parsed = JSON.parse(data)
      return JSON.stringify(parsed, null, 2)
    } catch {
      return data
    }
  }
  return JSON.stringify(data, null, 2)
}

function groupByRound(toolCalls: ToolCall[]): Map<number, ToolCall[]> {
  const groups = new Map<number, ToolCall[]>()
  for (const tc of toolCalls) {
    const round = tc.round ?? 1
    if (!groups.has(round)) groups.set(round, [])
    groups.get(round)!.push(tc)
  }
  return groups
}

function ToolCallItem({
  toolCall,
  isExpanded,
  isStreaming,
  onToggle,
  t
}: {
  toolCall: ToolCallWithDuration
  isExpanded: boolean
  isStreaming: boolean
  onToggle: () => void
  t: (key: string) => string
}) {
  const status = getToolStatus(
    toolCall.result !== undefined && toolCall.result !== null,
    isStreaming
  )
  const hasArguments = toolCall.arguments !== undefined && toolCall.arguments !== null
  const hasResult = toolCall.result !== undefined && toolCall.result !== null

  const statusLabels = {
    pending: t("toolCall.status.pending"),
    running: t("toolCall.status.running"),
    completed: t("toolCall.status.completed"),
  }

  return (
    <div className="border-b border-border/40 last:border-b-0">
      <div className="flex items-center gap-3 px-4 py-2.5">
        {/* Status icon */}
        <div className={cn(
          "h-5 w-5 rounded flex items-center justify-center shrink-0",
          status === "completed" && "bg-emerald-500/10 text-emerald-600",
          status === "running" && "bg-amber-500/10 text-amber-600",
          status === "pending" && "bg-muted text-muted-foreground"
        )}>
          {status === "running" ? (
            <Loader2 className="h-3 w-3 animate-spin" />
          ) : status === "completed" ? (
            <CheckCircle2 className="h-3.5 w-3.5" />
          ) : (
            <Wrench className="h-3 w-3" />
          )}
        </div>

        {/* Tool name with action context */}
        <span className="font-mono text-sm">{getToolDisplayName(toolCall.name, toolCall.arguments)}</span>

        {/* Status badge */}
        <span className={cn(
          "text-[10px] px-1.5 py-0.5 rounded shrink-0",
          status === "completed" && "bg-emerald-500/10 text-emerald-600",
          status === "running" && "bg-amber-500/10 text-amber-600",
          status === "pending" && "bg-muted text-muted-foreground"
        )}>
          {statusLabels[status]}
        </span>

        {/* Duration */}
        {toolCall.duration_ms && (
          <span className="text-xs text-muted-foreground flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {formatDuration(toolCall.duration_ms)}
          </span>
        )}

        <div className="flex-1" />

        {/* Expand button */}
        {(hasArguments || hasResult) && (
          <button
            onClick={onToggle}
            className="p-1 rounded hover:bg-muted/50 text-muted-foreground"
          >
            <ChevronDown className={cn(
              "h-4 w-4 transition-transform",
              isExpanded && "rotate-180"
            )} />
          </button>
        )}
      </div>

      {/* Expandable content */}
      {isExpanded && (
        <div className="px-4 pb-3 space-y-2">
          {hasArguments && (
            <div className="rounded-lg bg-muted/50 p-2">
              <div className="text-[10px] text-muted-foreground mb-1 flex items-center gap-1">
                <Code className="h-3 w-3" />
                {t("toolCall.arguments")}
              </div>
              <pre className="text-xs font-mono text-muted-foreground whitespace-pre-wrap break-words">
                {formatJson(toolCall.arguments)}
              </pre>
            </div>
          )}
          {hasResult && (
            <div className="rounded-lg bg-muted/50 p-2">
              <div className="text-[10px] text-muted-foreground mb-1 flex items-center gap-1">
                <CheckCircle2 className="h-3 w-3" />
                {t("toolCall.result")}
              </div>
              <pre className="text-xs font-mono text-muted-foreground whitespace-pre-wrap break-words max-h-32 overflow-y-auto">
                {formatJson(toolCall.result)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

function RoundGroup({
  round,
  toolCalls,
  isStreaming,
  roundContent,
  t
}: {
  round: number
  toolCalls: ToolCall[]
  isStreaming: boolean
  roundContent?: string
  t: (key: string, options?: Record<string, unknown>) => string
}) {
  const [collapsed, setCollapsed] = useState(false)
  const [expandedItems, setExpandedItems] = useState<Set<number>>(new Set())

  const toggleItem = (index: number) => {
    setExpandedItems(prev => {
      const next = new Set(prev)
      if (next.has(index)) next.delete(index)
      else next.add(index)
      return next
    })
  }

  const completedCount = toolCalls.filter(
    tc => tc.result !== undefined && tc.result !== null
  ).length
  const isCurrentRound = completedCount < toolCalls.length
  const isCompleted = completedCount === toolCalls.length

  return (
    <div className="border-b border-border/40 last:border-b-0">
      {/* Round header */}
      <button
        onClick={() => setCollapsed(!collapsed)}
        className="flex items-center gap-2 w-full px-4 py-2 text-sm hover:bg-muted/30 transition-colors"
      >
        <ChevronDown className={cn(
          "h-3.5 w-3.5 text-muted-foreground transition-transform",
          collapsed && "-rotate-90"
        )} />
        <span className="text-muted-foreground font-medium">
          {t("toolCall.roundTitle", { round })}
        </span>
        <span className="text-xs text-muted-foreground">
          {completedCount}/{toolCalls.length}
        </span>
        {isCurrentRound && isStreaming && (
          <Loader2 className="h-3 w-3 text-amber-600 animate-spin ml-1" />
        )}
        {isCompleted && (
          <CheckCircle2 className="h-3 w-3 text-emerald-600 ml-1" />
        )}
      </button>

      {/* Tool calls in this round */}
      {!collapsed && (
        <div>
          {/* Intermediate text for this round (ChatGPT Deep Research pattern) */}
          {roundContent && (
            <div className="px-4 py-1.5 text-xs text-muted-foreground italic border-b border-border/20">
              {roundContent.length > 120 ? roundContent.slice(0, 120) + '...' : roundContent}
            </div>
          )}
          {toolCalls.map((tc, i) => (
            <ToolCallItem
              key={`${round}-${i}`}
              toolCall={tc}
              isExpanded={expandedItems.has(i)}
              isStreaming={isStreaming && isCurrentRound}
              onToggle={() => toggleItem(i)}
              t={t as (key: string) => string}
            />
          ))}
        </div>
      )}
    </div>
  )
}

export function ToolCallVisualization({
  toolCalls,
  isStreaming = false,
  roundContents = {},
}: ToolCallVisualizationProps) {
  const { t } = useTranslation("chat")
  const [expandedItems, setExpandedItems] = useState<Set<number>>(new Set())

  if (!toolCalls || toolCalls.length === 0) return null

  const roundGroups = groupByRound(toolCalls)
  const totalRounds = roundGroups.size

  const toggleItem = (index: number) => {
    setExpandedItems(prev => {
      const next = new Set(prev)
      if (next.has(index)) {
        next.delete(index)
      } else {
        next.add(index)
      }
      return next
    })
  }

  const completedCount = toolCalls.filter(
    tc => tc.result !== undefined && tc.result !== null
  ).length

  // Single round with round content: show step-style layout
  const round1Content = roundContents[1]
  if (totalRounds <= 1) {
    return (
      <div className="my-3 rounded-xl border border-border/60 bg-muted/30 overflow-hidden">
        {/* Header */}
        <div className="flex items-center gap-3 px-4 py-2.5 border-b border-border/40">
          <Wrench className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">{t("toolCall.title")}</span>
          <span className="text-xs text-muted-foreground">
            {completedCount}/{toolCalls.length}
          </span>
          {completedCount === toolCalls.length && (
            <CheckCircle2 className="h-3.5 w-3.5 text-emerald-600 ml-auto" />
          )}
        </div>

        {/* Step description - show what the AI was doing */}
        {round1Content && (
          <div className="px-4 py-2 text-xs text-muted-foreground border-b border-border/20 flex items-center gap-2">
            <span className="shrink-0">{isStreaming ? "⏳" : "✓"}</span>
            <span className="line-clamp-2">{round1Content}</span>
          </div>
        )}

        {/* Tool list */}
        <div>
          {toolCalls.map((tc, i) => (
            <ToolCallItem
              key={i}
              toolCall={tc}
              isExpanded={expandedItems.has(i)}
              isStreaming={isStreaming}
              onToggle={() => toggleItem(i)}
              t={t}
            />
          ))}
        </div>
      </div>
    )
  }

  // Multi-round: show grouped layout
  return (
    <div className="my-3 rounded-xl border border-border/60 bg-muted/30 overflow-hidden">
      {/* Header with round count */}
      <div className="flex items-center gap-3 px-4 py-2.5 border-b border-border/40">
        <Wrench className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium">{t("toolCall.multiRoundTitle")}</span>
        <span className="text-xs text-muted-foreground">
          {completedCount}/{toolCalls.length}
        </span>
        <span className="text-xs text-muted-foreground">
          {totalRounds} {t("toolCall.rounds")}
        </span>
      </div>

      {/* Round groups */}
      <div>
        {Array.from(roundGroups.entries()).map(([round, calls]) => (
          <RoundGroup
            key={round}
            round={round}
            toolCalls={calls}
            isStreaming={isStreaming}
            roundContent={roundContents[round]}
            t={t}
          />
        ))}
      </div>
    </div>
  )
}
