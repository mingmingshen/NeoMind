/**
 * ToolCallVisualization - Displays AI tool/function calls as a "task process"
 *
 * Mental model: tool calls are a process the AI goes through to complete a task.
 * The final answer (message.content) is always shown separately below this block.
 *
 * Structure:
 *   ┌─ 🔧 N tool calls · M steps  ▾ ─────────┐
 *   │ Step 1: 🧠 Thinking + 🔧 Tools          │  ← collapsible
 *   │ Step 2: 🧠 Thinking + 🔧 Tools          │
 *   └──────────────────────────────────────────┘
 *   Markdown content (the final answer)        ← rendered separately in chat.tsx
 */

import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Wrench, ChevronDown, CheckCircle2, Loader2, Code, FileText } from "lucide-react"
import { cn } from "@/lib/utils"
import type { ToolCall } from "@/types"

// ─── Helpers ───

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

function getToolDisplayName(name: string, args?: unknown): string {
  if (!args || typeof args !== 'object') return name
  const a = args as Record<string, unknown>
  const action = typeof a.action === 'string' ? a.action : ''
  if (!action) return name
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

/** Check if thinking duplicates the content (Phase 2 LLM may echo response as "thinking") */
export function isThinkingDuplicate(thinking: string | undefined, content: string | undefined): boolean {
  if (!thinking || !content) return false
  const tPreview = thinking.slice(0, 200)
  const cPreview = content.slice(0, 200)
  if (!tPreview || !cPreview) return false
  return cPreview.includes(tPreview) || tPreview.includes(cPreview)
}

// ─── ToolProcessBlock: the main exported component ───

interface ToolProcessBlockProps {
  toolCalls: ToolCall[]
  /** Per-round intermediate content (shown alongside tools in each round) */
  roundContents?: Record<number, string>
  isStreaming?: boolean
}

/**
 * Renders the tool-calling process as a collapsible card.
 * Shows: summary header + per-round content & tool calls.
 * Does NOT render thinking or final content — those are handled by chat.tsx.
 */
export function ToolProcessBlock({
  toolCalls,
  roundContents = {},
  isStreaming = false,
}: ToolProcessBlockProps) {
  const { t } = useTranslation("chat")
  const [isExpanded, setIsExpanded] = useState(true)

  if (!toolCalls || toolCalls.length === 0) return null

  const toolGroups = groupByRound(toolCalls)
  const steps = Array.from(toolGroups.keys()).sort((a, b) => a - b)
  const completedCount = toolCalls.filter(tc => tc.result != null).length
  const allComplete = completedCount === toolCalls.length && !isStreaming

  return (
    <div className="mb-4 rounded-lg bg-muted-30 overflow-hidden">
      {/* Summary header */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center gap-2.5 px-3 py-2 text-left hover:bg-muted-30 transition-colors"
      >
        <div className={cn(
          "h-4 w-4 rounded flex items-center justify-center shrink-0",
          allComplete && "bg-emerald-500/10 text-emerald-600",
          isStreaming && "bg-amber-500/10 text-amber-600",
          !allComplete && !isStreaming && "bg-muted text-muted-foreground"
        )}>
          {isStreaming ? (
            <Loader2 className="h-2.5 w-2.5 animate-spin" />
          ) : allComplete ? (
            <CheckCircle2 className="h-4 w-4" />
          ) : (
            <Wrench className="h-2.5 w-2.5" />
          )}
        </div>
        <span className="text-[13px] font-medium text-foreground">
          {isStreaming
            ? `${completedCount}/${toolCalls.length} ${t("toolCall.status.running")}`
            : `${toolCalls.length} ${t("toolCall.title")} · ${steps.length} ${t("toolCall.rounds")}`
          }
        </span>
        <div className="flex-1" />
        <ChevronDown className={cn(
          "h-4 w-4 text-muted-foreground transition-transform duration-200",
          isExpanded && "rotate-180"
        )} />
      </button>

      {/* Steps */}
      {isExpanded && (
        <div className="border-t border-border">
          {steps.map((step, idx) => {
            const calls = toolGroups.get(step) ?? []
            const isLastStep = idx === steps.length - 1
            const stepStreaming = isStreaming && isLastStep
            const roundContent = roundContents[step]

            return (
              <div key={step} className={cn(
                idx > 0 && "border-t border-border"
              )}>
                {/* Round content (intermediate results) */}
                {roundContent && (
                  <RoundContent content={roundContent} />
                )}
                {calls.map((tc, i) => (
                  <ToolCallItem
                    key={`${step}-${i}`}
                    toolCall={tc}
                    isStreaming={stepStreaming && tc.result == null}
                    t={t}
                  />
                ))}
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}

/** Round content block - shows intermediate results from each round */
function RoundContent({ content }: { content: string }) {
  const [isExpanded, setIsExpanded] = useState(false)

  if (!content || content.trim().length === 0) return null

  return (
    <div className="border-b border-border">
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center gap-2 px-3 py-1.5 text-left hover:bg-muted-30 transition-colors"
      >
        <FileText className="h-4 w-4 text-muted-foreground shrink-0" />
        {!isExpanded && (
          <span className="text-[13px] text-muted-foreground truncate">
            {content.slice(0, 100)}{content.length > 100 && "..."}
          </span>
        )}
        {isExpanded && (
          <span className="text-[13px] text-muted-foreground">Details</span>
        )}
        <div className="flex-1" />
        <ChevronDown className={cn(
          "h-4 w-4 text-muted-foreground transition-transform",
          isExpanded && "rotate-180"
        )} />
      </button>
      {isExpanded && (
        <div className="px-3 pb-2">
          <div className="text-[13px] font-mono text-muted-foreground whitespace-pre-wrap break-words bg-muted-30 rounded p-2 leading-relaxed">
            {content}
          </div>
        </div>
      )}
    </div>
  )
}

// ─── ToolCallItem: single tool call with expandable args/result ───

function ToolCallItem({
  toolCall,
  isStreaming,
  t
}: {
  toolCall: ToolCall
  isStreaming: boolean
  t: (key: string) => string
}) {
  const [isExpanded, setIsExpanded] = useState(false)
  const status = getToolStatus(
    toolCall.result !== undefined && toolCall.result !== null,
    isStreaming
  )
  const hasArguments = toolCall.arguments !== undefined && toolCall.arguments !== null
  const hasResult = toolCall.result !== undefined && toolCall.result !== null
  const hasDetails = hasArguments || hasResult

  const statusLabels = {
    pending: t("toolCall.status.pending"),
    running: t("toolCall.status.running"),
    completed: t("toolCall.status.completed"),
  }

  return (
    <div className="border-t border-border first:border-t-0">
      <div className="flex items-center gap-2.5 px-3 py-1.5">
        <div className={cn(
          "h-4 w-4 rounded flex items-center justify-center shrink-0",
          status === "completed" && "text-emerald-600",
          status === "running" && "text-amber-600",
          status === "pending" && "text-muted-foreground"
        )}>
          {status === "running" ? (
            <Loader2 className="h-2.5 w-2.5 animate-spin" />
          ) : status === "completed" ? (
            <CheckCircle2 className="h-2.5 w-2.5" />
          ) : (
            <Wrench className="h-2.5 w-2.5" />
          )}
        </div>
        <span className="font-mono text-[13px] truncate">{getToolDisplayName(toolCall.name, toolCall.arguments)}</span>
        <span className={cn(
          "text-[11px] px-1.5 py-0.5 rounded shrink-0",
          status === "completed" && "bg-emerald-500/10 text-emerald-600",
          status === "running" && "bg-amber-500/10 text-amber-600",
          status === "pending" && "bg-muted text-muted-foreground"
        )}>
          {statusLabels[status]}
        </span>
        <div className="flex-1" />
        {hasDetails && (
          <button onClick={() => setIsExpanded(!isExpanded)} className="p-0.5 rounded hover:bg-muted-50 text-muted-foreground">
            <ChevronDown className={cn("h-4 w-4 transition-transform", isExpanded && "rotate-180")} />
          </button>
        )}
      </div>
      {isExpanded && hasDetails && (
        <div className="px-3 pb-2 space-y-1.5">
          {hasArguments && (
            <div className="rounded-lg bg-muted-50 p-1.5">
              <div className="text-[11px] text-muted-foreground mb-0.5 flex items-center gap-1">
                <Code className="h-4 w-4" />{t("toolCall.arguments")}
              </div>
              <pre className="text-[13px] font-mono text-muted-foreground whitespace-pre-wrap break-words">
                {formatJson(toolCall.arguments)}
              </pre>
            </div>
          )}
          {hasResult && (
            <div className="rounded-lg bg-muted-50 p-1.5">
              <div className="text-[11px] text-muted-foreground mb-0.5 flex items-center gap-1">
                <CheckCircle2 className="h-4 w-4" />{t("toolCall.result")}
              </div>
              <pre className="text-[13px] font-mono text-muted-foreground whitespace-pre-wrap break-words max-h-32 overflow-y-auto">
                {formatJson(toolCall.result)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

// ─── Legacy exports (keep for backward compat if other files import them) ───

export function PerRoundBlocks(_props: any) {
  return null // Deprecated: use ToolProcessBlock + MarkdownMessage instead
}

export function ToolCallVisualization(_props: any) {
  return null // Deprecated: use ToolProcessBlock instead
}
