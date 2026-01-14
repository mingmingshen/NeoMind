/**
 * Session Slice
 *
 * Handles chat session management and message history.
 * Supports multiple sessions with switching capability.
 */

import type { StateCreator } from 'zustand'
import type { SessionState } from '../types'
import type { Message, ChatSession } from '@/types'
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
    set((state) => ({
      messages: [...state.messages, message],
    }))
  },

  clearMessages: () => {
    set({ messages: [] })
  },

  createSession: async () => {
    try {
      const result = await api.createSession()
      const newSession: ChatSession = {
        sessionId: result.sessionId,
        id: result.sessionId,
        createdAt: Date.now(),
      }
      set((state) => ({
        sessionId: result.sessionId,
        messages: [],
        sessions: [newSession, ...state.sessions],
      }))

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
      set({
        sessionId,
        messages: historyResult.messages || [],
      })

      // Update WebSocket to use the new session
      const { ws } = await import('@/lib/websocket')
      ws.setSessionId(sessionId)
    } catch (error: any) {
      console.error('Failed to switch session:', error)

      // If session not found (404), remove it from the list
      if (error?.error?.code === 'NOT_FOUND' || error?.status === 404) {
        // Remove the invalid session from list
        set((state) => ({
          sessions: state.sessions.filter(s => s.sessionId !== sessionId),
        }))

        // Try to load sessions from server to get fresh list
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

          set((state) => ({
            sessions,
            // If we still have sessions, switch to the first one
            sessionId: sessions.length > 0 ? sessions[0].sessionId : null,
            messages: sessions.length > 0 ? [] : state.messages,
          }))

          // Update WebSocket if we switched sessions
          if (sessions.length > 0 && sessions[0].sessionId) {
            const { ws } = await import('@/lib/websocket')
            ws.setSessionId(sessions[0].sessionId)
          }

          // If no sessions left, create a new one
          if (sessions.length === 0) {
            const createResult = await api.createSession()
            const newSession: ChatSession = {
              sessionId: createResult.sessionId,
              id: createResult.sessionId,
              createdAt: Date.now(),
            }
            set({
              sessionId: createResult.sessionId,
              messages: [],
              sessions: [newSession],
            })

            const { ws } = await import('@/lib/websocket')
            ws.setSessionId(createResult.sessionId)
          }
        } catch (loadError) {
          console.error('Failed to reload sessions:', loadError)
        }
        return
      }

      // For other errors, just clear messages and set the session ID
      set({
        sessionId,
        messages: [],
      })
    }
  },

  deleteSession: async (sessionId: string) => {
    try {
      await api.deleteSession(sessionId)
      set((state) => {
        const filtered = state.sessions.filter(s => s.sessionId !== sessionId)
        // If deleting current session, switch to the first available or null
        const wasCurrentSession = state.sessionId === sessionId
        const newSessionId = wasCurrentSession
          ? (filtered.length > 0 ? filtered[0].sessionId : null)
          : state.sessionId
        return {
          sessions: filtered,
          sessionId: newSessionId,
          messages: wasCurrentSession ? [] : state.messages, // Clear messages if we switched
        }
      })

      // Reload sessions from server to ensure sync
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

        // Check if we need to create a new session or update WebSocket
        set((state) => {
          // Update sessions list with server data
          const updatedSessions = sessions

          // If no sessions exist after reload, create one
          if (updatedSessions.length === 0) {
            // Async create will be handled by SessionSidebar component
            return { sessions: [] }
          }

          // Update WebSocket if session changed
          const currentSessionId = state.sessionId
          if (currentSessionId && updatedSessions.some(s => s.sessionId === currentSessionId)) {
            // Current session still valid, keep it
            return { sessions: updatedSessions }
          } else if (updatedSessions.length > 0) {
            // Switch to first available session
            const firstSessionId = updatedSessions[0].sessionId
            import('@/lib/websocket').then(({ ws }) => {
              ws.setSessionId(firstSessionId)
            })
            return {
              sessions: updatedSessions,
              sessionId: firstSessionId,
              messages: [],
            }
          }

          return { sessions: updatedSessions }
        })
      } catch (loadError) {
        console.error('Failed to reload sessions after delete:', loadError)
      }
    } catch (error) {
      console.error('Failed to delete session:', error)
    }
  },

  clearAllSessions: async () => {
    // First get sessions from current state before clearing
    let sessionIds: string[] = []
    set((state) => {
      sessionIds = state.sessions.map(s => s.sessionId).filter((id): id is string => id != null)
      return state // Just reading, no change yet
    })

    if (sessionIds.length === 0) {
      return
    }

    try {
      // Bulk delete all sessions
      await api.bulkDeleteSessions(sessionIds)
    } catch (error) {
      console.error('Failed to bulk delete sessions:', error)
      throw error
    }

    // Clear local state
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
      }
      set({
        sessionId: result.sessionId,
        sessions: [newSession],
      })

      // Update WebSocket
      const { ws } = await import('@/lib/websocket')
      ws.setSessionId(result.sessionId)
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
      set({ sessions })
    } catch (error) {
      console.error('Failed to load sessions:', error)
    }
  },

  fetchSessionHistory: async (sessionId: string) => {
    try {
      const result = await api.getSessionHistory(sessionId)
      set({ messages: result.messages || [] })
    } catch (error) {
      console.error('Failed to fetch session history:', error)
    }
  },
})
