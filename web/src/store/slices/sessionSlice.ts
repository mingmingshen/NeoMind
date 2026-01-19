/**
 * Session Slice
 *
 * Handles chat session management and message history.
 * Supports multiple sessions with switching capability.
 */

import type { StateCreator } from 'zustand'
import type { Message } from '@/types'

/**
 * Filter messages for display.
 * Backend now saves complete messages with all fields (thinking, tool_calls, content).
 * Simply filter out internal tool role messages.
 */
function mergeAssistantMessages(messages: Message[]): Message[] {
  // Filter out tool role messages (internal LLM context, not for display)
  return messages.filter(m => (m as any).role !== 'tool')
}

import type { SessionState } from '../types'
import type { ChatSession } from '@/types'
import { api } from '@/lib/api'

export interface SessionSlice extends SessionState {
  // Actions
  setSessionId: (id: string) => void
  addMessage: (message: Message) => void
  clearMessages: () => void
  createSession: () => Promise<string | null>
  switchSession: (sessionId: string) => Promise<void>
  deleteSession: (sessionId: string) => Promise<void>
  clearAllSessions: () => Promise<void>
  updateSessionTitle: (sessionId: string, title: string) => Promise<void>
  loadSessions: () => Promise<void>
  fetchSessionHistory: (sessionId: string) => Promise<void>
}

export const createSessionSlice: StateCreator<
  SessionSlice,
  [],
  [],
  SessionSlice
> = (set) => ({
  // Initial state
  sessionId: null,
  messages: [],
  sessions: [],

  // Actions
  setSessionId: (id: string) => {
    set({ sessionId: id })
  },

  addMessage: (message: Message) => {
    set((state) => {
      // If message with same ID exists, update it (especially important for partial->final transition)
      const existingIndex = state.messages.findIndex(m => m.id === message.id)

      if (existingIndex !== -1) {
        // Update existing message in place
        // This handles both: partial->partial updates AND partial->final transition
        const updatedMessages = [...state.messages]
        updatedMessages[existingIndex] = message
        return { messages: updatedMessages }
      }

      // Otherwise append new message
      return { messages: [...state.messages, message] }
    })
  },

  clearMessages: () => {
    set({ messages: [] })
  },

  createSession: async () => {
    try {
      const result = await api.createSession()

      // Reload sessions from server to ensure consistency
      // This avoids issues where local state diverges from server state
      const listResult = await api.listSessions()
      const sessionsArray = Array.isArray(listResult) ? listResult : (listResult as any).sessions || []
      const sessions: ChatSession[] = sessionsArray.map((s: any) => ({
        sessionId: s.sessionId || s.id,
        id: s.sessionId || s.id,
        createdAt: s.createdAt || s.created_at || Date.now(),
        updatedAt: s.updatedAt || s.updated_at,
        messageCount: s.messageCount || s.message_count || 0,
        title: s.title ?? undefined,
        preview: s.preview,
      }))

      set({
        sessionId: result.sessionId,
        messages: [],
        sessions,
      })

      // Update WebSocket to use the new session
      const { ws } = await import('@/lib/websocket')
      ws.setSessionId(result.sessionId)

      return result.sessionId
    } catch (error) {
      console.error('Failed to create session:', error)
      return null
    }
  },

  switchSession: async (sessionId: string) => {
    try {
      // Fetch the session history
      const historyResult = await api.getSessionHistory(sessionId)

      // Debug: Check if backend returns tool call results
      const assistantMessages = (historyResult.messages || []).filter((m: any) => m.role === 'assistant' && m.tool_calls?.length > 0)
      if (assistantMessages.length > 0) {
        const toolsWithResults = assistantMessages[0].tool_calls?.filter((tc: any) => tc.result !== undefined && tc.result !== null).length || 0
        console.log(`[sessionSlice] Found ${assistantMessages.length} assistant messages with tool calls, ${toolsWithResults} have results`)
        if (toolsWithResults === 0) {
          console.warn('[sessionSlice] No tool calls have results! Backend may be using old code.')
          console.log('[sessionSlice] Tool call data:', assistantMessages[0].tool_calls)
        }
      }

      // Merge fragmented assistant messages from backend
      // Backend stores: [msg1(thinking+tools)] + [tool results] + [msg2(content only)]
      // Frontend expects: [msg1(thinking+tools+content)]
      const mergedMessages = mergeAssistantMessages(historyResult.messages || [])

      set({
        sessionId,
        messages: mergedMessages,
      })

      // Update WebSocket to use the new session
      const { ws } = await import('@/lib/websocket')
      ws.setSessionId(sessionId)
    } catch (error: any) {
      console.error('Failed to switch session:', error)

      // If session not found (404), show error but don't auto-delete
      // This prevents data loss when database has inconsistencies
      if (error?.error?.code === 'NOT_FOUND' || error?.status === 404) {
        console.warn(`Session ${sessionId} not found in database. This may indicate data corruption.`)

        // Try to reload sessions from server to get accurate list
        try {
          const result = await api.listSessions()
          const sessionsArray = Array.isArray(result) ? result : (result as any).sessions || []
          const sessions: ChatSession[] = sessionsArray.map((s: any) => ({
            sessionId: s.sessionId || s.id,
            id: s.sessionId || s.id,
            createdAt: s.createdAt || s.created_at || Date.now(),
            updatedAt: s.updatedAt || s.updated_at,
            messageCount: s.messageCount || s.message_count || 0,
            title: s.title ?? undefined,
            preview: s.preview,
          }))

          // Only update sessions if server returned a valid list
          if (sessions.length > 0) {
            set((state) => ({
              sessions,
              // Keep current session if it still exists, otherwise use first session
              sessionId: sessions.some(s => s.sessionId === sessionId)
                ? state.sessionId
                : sessions[0].sessionId,
              messages: sessions.some(s => s.sessionId === sessionId)
                ? state.messages
                : [],
            }))
          } else {
            // No sessions on server - keep local state and notify user
            console.error('No sessions found on server. Database may be corrupted.')
          }
        } catch (loadError) {
          console.error('Failed to reload sessions:', loadError)
        }
        return
      }

      // For other errors, just keep current state
      console.error('Error switching session:', error)
    }
  },

  deleteSession: async (sessionIdToDelete: string) => {
    try {
      await api.deleteSession(sessionIdToDelete)

      // After successful deletion, reload sessions from server
      // This ensures consistency between server and client state
      const result = await api.listSessions()
      const sessionsArray = Array.isArray(result) ? result : (result as any).sessions || []
      const sessions: ChatSession[] = sessionsArray.map((s: any) => ({
        sessionId: s.sessionId || s.id,
        id: s.sessionId || s.id,
        createdAt: s.createdAt || s.created_at || Date.now(),
        updatedAt: s.updatedAt || s.updated_at,
        messageCount: s.messageCount || s.message_count || 0,
        title: s.title ?? undefined,
        preview: s.preview,
      }))

      set((state) => {
        const wasCurrentSession = state.sessionId === sessionIdToDelete

        // If no sessions left, return empty list
        if (sessions.length === 0) {
          return {
            sessions: [],
            sessionId: null,
            messages: [],
          }
        }

        // If we deleted the current session, switch to the first available
        // and load its history
        if (wasCurrentSession) {
          const firstSessionId = sessions[0].sessionId
          import('@/lib/websocket').then(({ ws }) => {
            ws.setSessionId(firstSessionId)
          })
          // Load history for the first session asynchronously
          api.getSessionHistory(firstSessionId).then(historyResult => {
            const mergedMessages = mergeAssistantMessages(historyResult.messages || [])
            set((state) => {
              // Only update if we're still on the same session
              if (state.sessionId === firstSessionId) {
                return { messages: mergedMessages }
              }
              return {}
            })
          }).catch(err => {
            console.error('Failed to load history for first session:', err)
          })
          return {
            sessions,
            sessionId: firstSessionId,
            messages: [], // Will be populated by the async call above
          }
        }

        // Otherwise, just update the sessions list, keep current state
        return {
          ...state,
          sessions,
        }
      })
    } catch (error) {
      console.error('Failed to delete session:', error)
      throw error
    }
  },

  clearAllSessions: async () => {
    try {
      // IMPORTANT: Fetch ALL sessions from backend, not just from local state
      // This ensures we delete all sessions including those not currently loaded
      // (e.g., from tests, other browser windows, or previous runs)
      const allSessions = await api.listSessions(1, 1000) // Get up to 1000 sessions

      if (!Array.isArray(allSessions) || allSessions.length === 0) {
        console.log('[clearAllSessions] No sessions to delete')
        return
      }

      const sessionIds = allSessions
        .map((s: any) => s.sessionId || s.id)
        .filter((id: string): id is string => id != null)

      console.log(`[clearAllSessions] Deleting ${sessionIds.length} sessions from backend`)

      // Bulk delete all sessions
      await api.bulkDeleteSessions(sessionIds)
    } catch (error) {
      console.error('Failed to bulk delete sessions:', error)
      throw error
    }

    // Clear local state temporarily
    set({
      sessions: [],
      sessionId: null,
      messages: [],
    })

    // Create a new session for continued use
    try {
      const result = await api.createSession()
      const newSession: ChatSession = {
        sessionId: result.sessionId,
        id: result.sessionId,
        createdAt: Date.now(),
        messageCount: 0,
      }

      // Update WebSocket
      const { ws } = await import('@/lib/websocket')
      ws.setSessionId(result.sessionId)

      // Set new state with empty messages (new session has no history)
      set({
        sessionId: result.sessionId,
        sessions: [newSession],
        messages: [],
      })
    } catch (createError) {
      console.error('Failed to create new session after clearing:', createError)
    }
  },

  updateSessionTitle: async (sessionId: string, title: string) => {
    try {
      await api.updateSession(sessionId, title)
      set((state) => ({
        sessions: state.sessions.map(s =>
          s.sessionId === sessionId ? { ...s, title } : s
        ),
      }))
    } catch (error) {
      console.error('Failed to update session title:', error)
      throw error
    }
  },

  loadSessions: async () => {
    try {
      const result = await api.listSessions()
      // Backend returns an array directly (after ApiResponse auto-unwrap)
      const sessionsArray = Array.isArray(result) ? result : (result as any).sessions || []
      const sessions: ChatSession[] = sessionsArray.map((s: any) => ({
        sessionId: s.sessionId || s.id,
        id: s.sessionId || s.id,
        createdAt: s.createdAt || s.created_at || Date.now(),
        updatedAt: s.updatedAt || s.updated_at,
        messageCount: s.messageCount || s.message_count || 0,
        title: s.title ?? undefined,
        preview: s.preview,
      }))

      // Only update sessions list, preserve sessionId and messages
      // This prevents accidental session switching or message loss
      set((state) => ({
        ...state,
        sessions,
      }))
    } catch (error) {
      console.error('Failed to load sessions:', error)
    }
  },

  fetchSessionHistory: async (sessionId: string) => {
    try {
      const result = await api.getSessionHistory(sessionId)
      // Merge fragmented assistant messages from backend
      const mergedMessages = mergeAssistantMessages(result.messages || [])
      set({ messages: mergedMessages })
    } catch (error) {
      console.error('Failed to fetch session history:', error)
    }
  },
})
