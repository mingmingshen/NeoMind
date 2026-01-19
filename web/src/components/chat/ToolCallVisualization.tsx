// ToolCallVisualization component - displays AI tool calls with execution details
import { useState } from "react"
import { Wrench, ChevronDown, ChevronUp, Clock, CheckCircle2, Loader2, AlertCircle, Code } from "lucide-react"
import { cn } from "@/lib/utils"
import type { ToolCall } from "@/types"

interface ToolCallVisualizationProps {
  toolCalls: ToolCall[]
  isStreaming?: boolean
}

// Extended type for tool calls with optional duration
interface ToolCallWithDuration extends ToolCall {
  duration_ms?: number
}

function getToolStatusIcon(hasResult: boolean, isStreaming: boolean) {
  if (isStreaming && !hasResult) {
    return <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
  }
  if (hasResult) {
    return <CheckCircle2 className="h-3 w-3 text-muted-foreground" />
  }
  return <AlertCircle className="h-3 w-3 text-muted-foreground" />
}

function formatDuration(ms?: number): string {
  if (!ms) return ""
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function formatArguments(args: unknown): string {
  if (typeof args === "string") {
    try {
      const parsed = JSON.parse(args)
      return JSON.stringify(parsed, null, 2)
    } catch {
      return args
    }
  }
  return JSON.stringify(args, null, 2)
}

function formatResult(result: unknown): string {
  if (typeof result === "string") {
    try {
      const parsed = JSON.parse(result)
      return JSON.stringify(parsed, null, 2)
    } catch {
      return result
    }
  }
  return JSON.stringify(result, null, 2)
}

export function ToolCallVisualization({
  toolCalls,
  isStreaming = false
}: ToolCallVisualizationProps) {
  const [expandedResults, setExpandedResults] = useState<Set<string>>(new Set())
  const [expandedArguments, setExpandedArguments] = useState<Set<string>>(new Set())

  if (!toolCalls || toolCalls.length === 0) return null

  const toggleResult = (key: string) => {
    setExpandedResults(prev => {
      const next = new Set(prev)
      if (next.has(key)) {
        next.delete(key)
      } else {
        next.add(key)
      }
      return next
    })
  }

  const toggleArguments = (key: string) => {
    setExpandedArguments(prev => {
      const next = new Set(prev)
      if (next.has(key)) {
        next.delete(key)
      } else {
        next.add(key)
      }
      return next
    })
  }

  const completedCount = toolCalls.filter(tc => tc.result !== undefined && tc.result !== null).length

  return (
    <div className="tool-call-visualization rounded-md border border-border/50 bg-muted/30 overflow-hidden text-sm">
      {/* Header - minimal style */}
      <div className="flex items-center justify-between gap-2 px-3 py-2 border-b border-border/30">
        <div className="flex items-center gap-1.5 text-muted-foreground">
          <Wrench className="h-3.5 w-3.5" />
          <span className="text-xs">
            工具调用 · {completedCount}/{toolCalls.length}
          </span>
        </div>
      </div>

      {/* Tool List - compact */}
      <div className="divide-y divide-border/30">
        {toolCalls.map((tc, i) => {
          const resultKey = `result-${i}`
          const argsKey = `args-${i}`
          const isResultExpanded = expandedResults.has(resultKey)
          const isArgsExpanded = expandedArguments.has(argsKey)
          const hasResult = tc.result !== undefined && tc.result !== null
          const hasArguments = tc.arguments !== undefined && tc.arguments !== null

          return (
            <div key={i} className="px-3 py-2 hover:bg-muted/50 transition-colors">
              {/* Tool Name Row */}
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 flex-1 min-w-0">
                  {/* Status icon */}
                  <div className={cn(
                    "h-5 w-5 rounded flex items-center justify-center shrink-0",
                    hasResult ? "bg-muted" : "bg-muted/50"
                  )}>
                    {getToolStatusIcon(hasResult, isStreaming)}
                  </div>

                  {/* Tool name */}
                  <span className="font-mono text-xs text-foreground/80 truncate">
                    {tc.name}
                  </span>

                  {/* Status badge */}
                  <span className={cn(
                    "text-[10px] px-1.5 py-0.5 rounded shrink-0",
                    hasResult
                      ? "bg-muted text-muted-foreground"
                      : isStreaming
                        ? "bg-muted text-muted-foreground"
                        : "bg-muted/50 text-muted-foreground/70"
                  )}>
                    {hasResult ? '完成' : isStreaming ? '执行中' : '等待'}
                  </span>

                  {/* Duration */}
                  {(tc as ToolCallWithDuration).duration_ms && (
                    <span className="text-[10px] text-muted-foreground flex items-center gap-0.5 shrink-0">
                      <Clock className="h-2.5 w-2.5" />
                      {formatDuration((tc as ToolCallWithDuration).duration_ms)}
                    </span>
                  )}
                </div>

                {/* Expand buttons */}
                <div className="flex items-center gap-0.5 shrink-0">
                  {hasArguments && (
                    <button
                      onClick={() => toggleArguments(argsKey)}
                      className={cn(
                        "p-1 rounded transition-colors",
                        isArgsExpanded 
                          ? "bg-muted text-foreground" 
                          : "hover:bg-muted/50 text-muted-foreground hover:text-foreground"
                      )}
                      title="查看参数"
                    >
                      <Code className="h-3 w-3" />
                    </button>
                  )}
                  {hasResult && (
                    <button
                      onClick={() => toggleResult(resultKey)}
                      className={cn(
                        "p-1 rounded transition-colors",
                        isResultExpanded 
                          ? "bg-muted text-foreground" 
                          : "hover:bg-muted/50 text-muted-foreground hover:text-foreground"
                      )}
                      title="查看结果"
                    >
                      {isResultExpanded ? (
                        <ChevronUp className="h-3 w-3" />
                      ) : (
                        <ChevronDown className="h-3 w-3" />
                      )}
                    </button>
                  )}
                </div>
              </div>

              {/* Expandable Arguments */}
              {isArgsExpanded && hasArguments && (
                <div className="mt-2 ml-7 p-2 bg-background/50 rounded border border-border/30">
                  <div className="text-[10px] font-medium text-muted-foreground mb-1 flex items-center gap-1">
                    <Code className="h-2.5 w-2.5" />
                    参数
                  </div>
                  <pre className="text-[11px] font-mono text-foreground/70 overflow-x-auto whitespace-pre-wrap break-words">
                    {formatArguments(tc.arguments)}
                  </pre>
                </div>
              )}

              {/* Expandable Result */}
              {isResultExpanded && hasResult && (
                <div className="mt-2 ml-7 p-2 bg-background/50 rounded border border-border/30">
                  <div className="text-[10px] font-medium text-muted-foreground mb-1 flex items-center gap-1">
                    <CheckCircle2 className="h-2.5 w-2.5" />
                    结果
                  </div>
                  <pre className="text-[11px] font-mono text-foreground/70 overflow-x-auto whitespace-pre-wrap break-words max-h-40 overflow-y-auto">
                    {formatResult(tc.result)}
                  </pre>
                </div>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
