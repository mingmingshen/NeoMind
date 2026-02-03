/**
 * Message Utilities - Shared logic for message handling
 *
 * This module contains utilities for working with messages,
 * including deduplication and merging of fragmented responses.
 */

import type { Message } from "@/types"

/**
 * Combine two content strings without duplicating when backend sent the same content twice.
 */
export function dedupeContentConcat(a: string, b: string): string {
  const x = (a || '').trim()
  const y = (b || '').trim()
  if (!y) return a || ''
  if (!x) return b || ''
  if (x === y) return a
  if (x.endsWith(y)) return a
  if (y.startsWith(x)) return b
  if (x.includes(y)) return a
  return (a || '') + (b || '')
}

/**
 * Check if two assistant messages should be merged.
 *
 * They should be merged if:
 * - First has thinking OR tools
 * - Second has content OR first is missing content (split response)
 * - They are consecutive assistant messages
 *
 * Backend pattern: [thinking+tools] + [content] or [thinking] + [tools+content]
 */
export function shouldMergeMessages(first: Message, second: Message): boolean {
  const firstHasThinking = !!first.thinking && first.thinking.length > 0
  const firstHasTools = !!first.tool_calls && first.tool_calls.length > 0
  const firstHasContent = !!first.content && first.content.length > 0

  const secondHasThinking = !!second.thinking && second.thinking.length > 0
  const secondHasTools = !!second.tool_calls && second.tool_calls.length > 0
  const secondHasContent = !!second.content && second.content.length > 0

  // Always merge consecutive assistant messages where first has thinking or tools
  // This handles the backend pattern of splitting responses
  if (firstHasThinking || firstHasTools) {
    // Merge if second has content, OR if first is missing content (split response)
    return !firstHasContent || secondHasContent
  }

  // Also merge if second has thinking or tools and first only has content
  if ((secondHasThinking || secondHasTools) && firstHasContent) {
    return true
  }

  return false
}

/**
 * Merge fragmented assistant messages for display.
 *
 * Rules:
 * 1. User messages are kept as-is
 * 2. Consecutive assistant messages are merged if they should be:
 *    - Take thinking from the first one
 *    - Take tool_calls from the first one (or any that has them)
 *    - Concatenate all content
 *    - Use the earliest timestamp
 * 3. Other roles (system, tool) are filtered out
 */
export function mergeMessagesForDisplay(messages: Message[]): Message[] {
  const result: Message[] = []

  for (let i = 0; i < messages.length; i++) {
    const msg = messages[i]

    // Skip tool messages (internal use)
    if ((msg as any).role === "tool") continue

    // User messages and system messages are kept as-is
    if (msg.role !== "assistant") {
      result.push(msg)
      continue
    }

    // Assistant messages - check if we should merge with following assistant messages
    const mergedAssistant: Message = { ...msg }
    const contentParts: string[] = []
    if (msg.content) {
      contentParts.push(msg.content)
    }

    // Look ahead for consecutive assistant messages to merge
    let j = i + 1
    while (j < messages.length && messages[j].role === "assistant") {
      const nextMsg = messages[j]

      // Only merge if they should be merged
      if (!shouldMergeMessages(mergedAssistant, nextMsg)) {
        break
      }

      // Collect content (will dedupe when joining)
      if (nextMsg.content) {
        contentParts.push(nextMsg.content)
      }

      // Use thinking from first message that has it
      if (!mergedAssistant.thinking && nextMsg.thinking) {
        mergedAssistant.thinking = nextMsg.thinking
      }

      // Use tool_calls from first message that has them
      if (!mergedAssistant.tool_calls && nextMsg.tool_calls) {
        mergedAssistant.tool_calls = nextMsg.tool_calls
      }

      j++
    }

    // Set merged content, deduplicating so the same text is not shown twice
    mergedAssistant.content = contentParts.reduce(
      (acc, part) => dedupeContentConcat(acc, part),
      ""
    )

    // Only add if there's something to show
    if (mergedAssistant.content || mergedAssistant.thinking || mergedAssistant.tool_calls) {
      result.push(mergedAssistant)
    }

    // Skip the merged messages
    i = j - 1
  }

  return result
}

/**
 * Check if a message is a partial stream message.
 * Partial messages should not be displayed in the message list.
 */
export function isPartialMessage(message: Message): boolean {
  return (message as any).isPartial === true
}

/**
 * Filter out partial messages from a message list.
 */
export function filterPartialMessages(messages: Message[]): Message[] {
  return messages.filter(msg => !isPartialMessage(msg))
}

/**
 * Get a preview of the last assistant message from a session.
 * Used for session list display.
 */
export function getLastAssistantMessage(messages: Message[]): string | null {
  const assistantMessages = messages.filter(m => m.role === "assistant" && !isPartialMessage(m))
  if (assistantMessages.length === 0) return null

  const lastMsg = assistantMessages[assistantMessages.length - 1]
  if (lastMsg.content) {
    // Return first 100 chars
    return lastMsg.content.slice(0, 100) + (lastMsg.content.length > 100 ? "..." : "")
  }
  return null
}
