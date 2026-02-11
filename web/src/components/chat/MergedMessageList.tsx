/**
 * MergedMessageList - Merges fragmented assistant messages at render time
 *
 * This component handles the case where the backend stores assistant responses
 * as multiple messages (thinking+tools first, then content separately).
 * It merges them for display without modifying the original data.
 *
 * Message merging logic is centralized in @/lib/messageUtils for consistency.
 */

import { type Message } from "@/types"
import { ThinkingBlock } from "./ThinkingBlock"
import { ToolCallVisualization } from "./ToolCallVisualization"
import { MarkdownMessage } from "./MarkdownMessage"
import { useStore } from "@/store"
import { MessageItem } from "./MessageItem"
import { useMemo } from "react"
import { mergeMessagesForDisplay } from "@/lib/messageUtils"
import { Loader2 } from "lucide-react"

interface MergedMessageListProps {
  messages: Message[]
  isStreaming?: boolean
  streamingContent?: string
  streamingThinking?: string
  streamingToolCalls?: any[]
}

export function MergedMessageList({
  messages,
  isStreaming = false,
  streamingContent = "",
  streamingThinking = "",
  streamingToolCalls = [],
}: MergedMessageListProps) {
  const { user } = useStore()

  // Memoize merged messages to avoid recalculation on every render
  // Only recompute when messages array reference changes
  const displayMessages = useMemo(() => mergeMessagesForDisplay(messages), [messages])

  // Get user initials
  const getUserInitials = (username: string) => {
    return username.slice(0, 2).toUpperCase()
  }

  return (
    <>
      {/* Merged messages */}
      {displayMessages.map((message) => (
        <MessageItem
          key={message.id}
          message={message}
          user={user}
          getUserInitials={getUserInitials}
        />
      ))}

      {/* Streaming message */}
      {isStreaming && (
        <div className="flex gap-3 justify-start">
          <img src="/logo-square.png" alt="NeoMind" className="flex-shrink-0 w-8 h-8 rounded-lg animate-pulse" />
          <div className="max-w-[80%]">
            <div className="rounded-2xl px-4 py-3 bg-[var(--msg-ai-bg)] text-[var(--msg-ai-text)]">
              {/* Thinking */}
              {streamingThinking && (
                <ThinkingBlock thinking={streamingThinking} />
              )}

              {/* Tool calls */}
              {streamingToolCalls.length > 0 && (
                <ToolCallVisualization
                  toolCalls={streamingToolCalls}
                  isStreaming={true}
                />
              )}

              {/* Content with blinking cursor when streaming */}
              {(streamingContent || streamingThinking || streamingToolCalls.length > 0) ? (
                <>
                  {streamingThinking && (
                    <ThinkingBlock thinking={streamingThinking} />
                  )}

                  {streamingToolCalls.length > 0 && (
                    <ToolCallVisualization
                      toolCalls={streamingToolCalls}
                      isStreaming={true}
                    />
                  )}

                  {streamingContent && (
                    <div className="relative inline">
                      <MarkdownMessage content={streamingContent} isStreaming={true} />
                      {/* Blinking cursor at the end of streaming content */}
                      <span className="inline-block w-0.5 h-4 ml-0.5 bg-current align-middle animate-pulse" />
                    </div>
                  )}
                </>
              ) : (
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
    </>
  )
}
