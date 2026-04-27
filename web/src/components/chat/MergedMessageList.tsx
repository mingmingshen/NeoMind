import { type Message, type ExecutionPlan } from "@/types"
import { ThinkingBlock } from "./ThinkingBlock"
import { ToolCallVisualization } from "./ToolCallVisualization"
import { MarkdownMessage } from "./MarkdownMessage"
import { MessageItem } from "./MessageItem"
import { ExecutionPlanPanel } from "./ExecutionPlanPanel"
import { useStore } from "@/store"
import { useMemo, useEffect, useRef, useCallback } from "react"
import { useVirtualizer } from "@tanstack/react-virtual"
import { mergeMessagesForDisplay } from "@/lib/messageUtils"
import { Loader2 } from "lucide-react"

interface MergedMessageListProps {
  messages: Message[]
  isStreaming?: boolean
  streamingContent?: string
  streamingThinking?: string
  streamingToolCalls?: any[]
  executionPlan?: ExecutionPlan | null
  planStepStates?: Record<number, 'pending' | 'running' | 'completed' | 'failed'>
  roundContents?: Record<number, string>
}

export function MergedMessageList({
  messages,
  isStreaming = false,
  streamingContent = "",
  streamingThinking = "",
  streamingToolCalls = [],
  executionPlan,
  planStepStates,
  roundContents = {},
}: MergedMessageListProps) {
  const user = useStore((s) => s.user)

  // Ref for the scroll container
  const listRef = useRef<HTMLDivElement>(null)

  // Memoize merged messages to avoid recalculation on every render
  const displayMessages = useMemo(() => mergeMessagesForDisplay(messages), [messages])

  // Virtual scrolling with @tanstack/react-virtual
  const virtualizer = useVirtualizer({
    count: displayMessages.length,
    getScrollElement: () => listRef.current,
    estimateSize: () => 120, // Average message height estimate
    overscan: 5, // Render 5 extra items above/below viewport
  })

  // Get user initials
  const getUserInitials = useCallback((username: string) => {
    return username.slice(0, 2).toUpperCase()
  }, [])

  // Auto-scroll to bottom when new messages arrive during streaming
  useEffect(() => {
    if (isStreaming && listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight
    }
  }, [isStreaming, displayMessages.length])

  // Show content when we have messages, or when streaming (even if messages are empty)
  const hasContent = displayMessages.length > 0 || isStreaming

  return (
    <>
      {!hasContent ? (
        // Empty state - no messages yet
        <div className="flex items-center justify-center py-16">
          <span className="text-sm text-muted-foreground">暂无消息</span>
        </div>
      ) : (
        // Actual messages with virtual scrolling
        <div ref={listRef} className="message-list-container overflow-y-auto" style={{ height: '100%' }}>
          {/* Virtual list */}
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: '100%',
              position: 'relative',
            }}
          >
            {virtualizer.getVirtualItems().map((virtualItem) => {
              const message = displayMessages[virtualItem.index]
              return (
                <div
                  key={virtualItem.key}
                  data-index={virtualItem.index}
                  ref={virtualizer.measureElement}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                >
                  <MessageItem
                    message={message}
                    user={user}
                    getUserInitials={getUserInitials}
                  />
                </div>
              )
            })}
          </div>

          {/* Streaming message - always visible */}
          {isStreaming && (
            <div className="flex gap-3 justify-start">
              <img src="/logo-square.png" alt="NeoMind" width={32} height={32} className="flex-shrink-0 w-8 h-8 rounded-lg animate-pulse" />
              <div className="max-w-[80%]">
                <div className="rounded-2xl px-4 py-3 bg-[var(--msg-ai-bg)] text-[var(--msg-ai-text)]">
                  {/* Execution plan */}
                  {executionPlan && (
                    <ExecutionPlanPanel
                      plan={executionPlan}
                      stepStates={planStepStates ?? {}}
                    />
                  )}

                  {/* Thinking */}
                  {streamingThinking && (
                    <ThinkingBlock thinking={streamingThinking} />
                  )}

                  {/* Tool calls with per-round intermediate text */}
                  {streamingToolCalls.length > 0 && (
                    <ToolCallVisualization
                      toolCalls={streamingToolCalls}
                      isStreaming={true}
                      roundContents={roundContents}
                    />
                  )}

                  {/* Streaming content with blinking cursor */}
                  {streamingContent ? (
                    <div className="relative inline">
                      <MarkdownMessage content={streamingContent} />
                      <span className="inline-block w-0.5 h-4 ml-0.5 bg-current align-middle animate-pulse" />
                    </div>
                  ) : !streamingThinking && streamingToolCalls.length === 0 && (
                    /* Loading indicator - shown when waiting for first response */
                    <div className="flex items-center gap-3 py-2">
                      <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                      <span className="text-sm text-muted-foreground">正在思考...</span>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </>
  )
}
