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
 * Merge fragmented assistant messages for display.
 *
 * Same logic as sessionSlice.mergeAssistantMessages:
 * Reconstruct ONE message per user turn, combining all tool calls across rounds,
 * building round_contents, and using the final content as message content.
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

    // Collect all assistant messages in this turn
    const turnMessages: Message[] = [msg]
    let j = i + 1
    while (j < messages.length) {
      const nextMsg = messages[j]
      if ((nextMsg as any).role === "tool") { j++; continue }
      if (nextMsg.role !== "assistant") break
      turnMessages.push(nextMsg)
      j++
    }

    const merged: Message = { ...msg, content: '' }
    let allToolCalls: any[] = []
    let roundContents: Record<number, string> = {}
    let roundCounter = 1
    let finalContent = ''
    let thinking: string | undefined

    // Detect backend storage pattern
    const hasTrailingContentMsg = turnMessages.some(m =>
      !(m.tool_calls && m.tool_calls.length > 0) && !!(m.content && m.content.trim())
    )

    let lastToolCallMsgIndex = -1
    for (let k = turnMessages.length - 1; k >= 0; k--) {
      if (turnMessages[k].tool_calls && turnMessages[k].tool_calls!.length > 0) {
        lastToolCallMsgIndex = k
        break
      }
    }

    for (let k = 0; k < turnMessages.length; k++) {
      const cur = turnMessages[k]
      const hasTools = !!(cur.tool_calls && cur.tool_calls.length > 0)
      const hasContent = !!(cur.content && cur.content.trim())

      if (cur.thinking) {
        // Accumulate thinking across all messages in this turn (interleaved thinking pattern)
        thinking = thinking ? thinking + '\n' + cur.thinking : cur.thinking
      }
      if (cur.round_contents) {
        roundContents = { ...roundContents, ...cur.round_contents }
      }

      if (hasTools) {
        const roundNum = cur.tool_calls![0]?.round ?? roundCounter
        allToolCalls = [...allToolCalls, ...cur.tool_calls!.map(tc => ({ ...tc, round: tc.round ?? roundNum }))]
        roundCounter = roundNum + 1

        if (hasContent) {
          if (hasTrailingContentMsg) {
            roundContents[roundNum] = cur.content!
          } else if (k === lastToolCallMsgIndex) {
            finalContent = cur.content!
          } else {
            roundContents[roundNum] = cur.content!
          }
        }
      } else if (hasContent) {
        finalContent = dedupeContentConcat(finalContent, cur.content!)
      }
    }

    merged.content = finalContent
    merged.thinking = thinking || undefined
    merged.tool_calls = allToolCalls.length > 0 ? allToolCalls : undefined
    merged.round_contents = Object.keys(roundContents).length > 0 ? roundContents : undefined

    if (merged.content || merged.thinking || merged.tool_calls) {
      result.push(merged)
    }

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
