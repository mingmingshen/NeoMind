// ToolCallVisualization component - displays AI tool calls with execution details
import { useState } from "react"
import { Wrench, ChevronDown, ChevronUp, Clock, CheckCircle2, Loader2, AlertCircle, Code } from "lucide-react"
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
    return <Loader2 className="h-3.5 w-3.5 animate-spin text-amber-600" />
  }
  if (hasResult) {
    return <CheckCircle2 className="h-3.5 w-3.5 text-green-600" />
  }
  return <AlertCircle className="h-3.5 w-3.5 text-muted-foreground" />
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

  return (
    <div className="tool-call-visualization rounded-lg border border-blue-200/50 bg-blue-50/30 overflow-hidden">
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-2.5 bg-blue-100/50 border-b border-blue-200/50">
        <Wrench className="h-4 w-4 text-blue-600" />
        <span className="font-medium text-sm text-blue-900">
          工具调用 ({toolCalls.length})
        </span>
      </div>

      {/* Tool List with Timeline */}
      <div className="divide-y divide-blue-100/50">
        {toolCalls.map((tc, i) => {
          const resultKey = `result-${i}`
          const argsKey = `args-${i}`
          const isResultExpanded = expandedResults.has(resultKey)
          const isArgsExpanded = expandedArguments.has(argsKey)
          const hasResult = tc.result !== undefined && tc.result !== null
          const hasArguments = tc.arguments !== undefined && tc.arguments !== null

          return (
            <div key={i} className="px-3 py-3 hover:bg-blue-50/30 transition-colors">
              {/* Tool Name Row */}
              <div className="flex items-start justify-between gap-2">
                <div className="flex items-start gap-3 flex-1 min-w-0">
                  {/* Status & Timeline */}
                  <div className="flex flex-col items-center pt-0.5">
                    <div className={`h-7 w-7 rounded-full flex items-center justify-center ${
                      hasResult ? 'bg-green-100 border border-green-300' : 'bg-amber-100 border border-amber-300'
                    }`}>
                      {getToolStatusIcon(hasResult, isStreaming)}
                    </div>
                    {i < toolCalls.length - 1 && (
                      <div className="w-0.5 h-6 bg-gradient-to-b from-blue-300 to-transparent my-1" />
                    )}
                  </div>

                  {/* Tool info */}
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="font-mono text-sm font-semibold bg-white px-2.5 py-1 rounded-md border border-blue-200 text-blue-800">
                        {tc.name}
                      </span>
                      <span className={`text-xs font-medium px-2 py-0.5 rounded-full ${
                        hasResult
                          ? 'bg-green-100 text-green-700'
                          : isStreaming
                            ? 'bg-amber-100 text-amber-700'
                            : 'bg-gray-100 text-gray-600'
                      }`}>
                        {hasResult ? '已完成' : isStreaming ? '执行中...' : '无结果'}
                      </span>
                      {(tc as ToolCallWithDuration).duration_ms && (
                        <span className="text-xs text-muted-foreground flex items-center gap-1">
                          <Clock className="h-3 w-3" />
                          {formatDuration((tc as ToolCallWithDuration).duration_ms)}
                        </span>
                      )}
                    </div>
                  </div>
                </div>

                {/* Expand buttons */}
                <div className="flex items-center gap-1">
                  {hasArguments && (
                    <button
                      onClick={() => toggleArguments(argsKey)}
                      className="p-1.5 hover:bg-white/70 rounded-md transition-colors text-muted-foreground hover:text-foreground"
                      title="查看参数"
                    >
                      <Code className="h-3.5 w-3.5" />
                    </button>
                  )}
                  {hasResult && (
                    <button
                      onClick={() => toggleResult(resultKey)}
                      className="p-1.5 hover:bg-white/70 rounded-md transition-colors text-muted-foreground hover:text-foreground"
                      title="查看结果"
                    >
                      {isResultExpanded ? (
                        <ChevronUp className="h-3.5 w-3.5" />
                      ) : (
                        <ChevronDown className="h-3.5 w-3.5" />
                      )}
                    </button>
                  )}
                </div>
              </div>

              {/* Expandable Arguments */}
              {isArgsExpanded && hasArguments && (
                <div className="mt-2 ml-10 p-2.5 bg-white/70 rounded-md border border-blue-200">
                  <div className="text-xs font-medium text-blue-800 mb-1.5 flex items-center gap-1.5">
                    <Code className="h-3 w-3" />
                    调用参数
                  </div>
                  <pre className="text-xs font-mono text-gray-700 overflow-x-auto whitespace-pre-wrap break-words bg-gray-50 p-2 rounded">
                    {formatArguments(tc.arguments)}
                  </pre>
                </div>
              )}

              {/* Expandable Result */}
              {isResultExpanded && hasResult && (
                <div className="mt-2 ml-10 p-2.5 bg-green-50/70 rounded-md border border-green-200">
                  <div className="text-xs font-medium text-green-800 mb-1.5 flex items-center gap-1.5">
                    <CheckCircle2 className="h-3 w-3" />
                    执行结果
                  </div>
                  <pre className="text-xs font-mono text-gray-700 overflow-x-auto whitespace-pre-wrap break-words bg-white p-2 rounded max-h-60 overflow-y-auto">
                    {formatResult(tc.result)}
                  </pre>
                </div>
              )}
            </div>
          )
        })}
      </div>

      {/* Footer with summary */}
      <div className="px-3 py-2 bg-blue-100/30 border-t border-blue-200/50">
        <div className="flex items-center justify-between text-xs text-blue-700">
          <span>共 {toolCalls.length} 个工具调用</span>
          <span>
            {toolCalls.filter(tc => tc.result !== undefined && tc.result !== null).length} 个已完成
          </span>
        </div>
      </div>
    </div>
  )
}
