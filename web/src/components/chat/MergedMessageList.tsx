import { type Message, type ExecutionPlan } from "@/types"
import { ThinkingBlock } from "./ThinkingBlock"
import { ToolCallVisualization } from "./ToolCallVisualization"
import { MarkdownMessage } from "./MarkdownMessage"
import { MessageItem } from "./MessageItem"
import { ExecutionPlanPanel } from "./ExecutionPlanPanel"
import { useStore } from "@/store"
import { useMemo, useCallback, useRef, type RefObject } from "react"
import { useVirtualizer } from "@tanstack/react-virtual"
import { mergeMessagesForDisplay } from "@/lib/messageUtils"
import { Loader2 } from "lucide-react"

interface MergedMessageListProps {
  messages: Message[]
  scrollElementRef: RefObject<HTMLDivElement | null>
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
  scrollElementRef,
  isStreaming = false,
  streamingContent = "",
  streamingThinking = "",
  streamingToolCalls = [],
  executionPlan,
  planStepStates,
  roundContents = {},
}: MergedMessageListProps) {
  const user = useStore((s) => s.user)

  // Memoize merged messages to avoid recalculation on every render
  const displayMessages = useMemo(() => mergeMessagesForDisplay(messages), [messages])

  // Get user initials
  const getUserInitials = useCallback((username: string) => {
    return username.slice(0, 2).toUpperCase()
  }, [])

  // Virtual scrolling — uses the parent scroll container (ChatContainer's overflow-y-auto div)
  // to avoid nested scrolling issues
  const virtualizer = useVirtualizer({
    count: displayMessages.length,
    getScrollElement: () => scrollElementRef.current,
    estimateSize: () => 120,
    overscan: 5,
  })

  // Show content when we have messages, or when streaming (even if messages are empty)
  const hasContent = displayMessages.length > 0 || isStreaming

  return (
    <>
      {!hasContent ? (
        <div className="flex items-center justify-center py-16">
          <span className="text-sm text-muted-foreground">暂无消息</span>
        </div>
      ) : (
        <div className="space-y-6">
          {/* Virtual list for existing messages */}
          {displayMessages.length > 0 && (
            <div
              style={{
                height: virtualizer.getTotalSize(),
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
          )}

          {/* Streaming message — always rendered outside the virtualizer */}
          {isStreaming && (
            <div className="flex gap-3 justify-start">
              <img src="/logo-square.png" alt="NeoMind" width={32} height={32} className="flex-shrink-0 w-8 h-8 rounded-lg mt-0.5 animate-pulse" />
              <div className="flex-1 min-w-0">
                {/* Execution plan */}
                {executionPlan && (
                  <ExecutionPlanPanel
                    plan={executionPlan}
                    stepStates={planStepStates ?? {}}
                  />
                )}

                {/* Thinking */}
                {streamingThinking && (
                  <ThinkingBlock thinking={streamingThinking} isStreaming={true} />
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
                  <div className="flex items-center gap-3 py-2">
                    <Loader2 className="h-4 w-4 animate-spin opacity-60" />
                    <span className="text-sm opacity-60">正在思考...</span>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </>
  )
}
