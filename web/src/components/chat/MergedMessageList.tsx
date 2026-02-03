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

              {/* Content */}
              {streamingContent && (
                <MarkdownMessage content={streamingContent} />
              )}

              {/* Loading indicator */}
              {!streamingContent && !streamingThinking && streamingToolCalls.length === 0 && (
                <div className="flex items-center gap-1">
                  <span key="dot-1" className="w-2 h-2 rounded-full bg-current animate-bounce delay-0" />
                  <span key="dot-2" className="w-2 h-2 rounded-full bg-current animate-bounce delay-150" />
                  <span key="dot-3" className="w-2 h-2 rounded-full bg-current animate-bounce delay-300" />
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  )
}
