// useMessages Hook for React Components
//
// Provides message/notification management functionality to React components.

import { useState, useCallback, useEffect } from 'react'
import type {
  NotificationMessage,
  MessageStats,
  MessageSeverity,
  MessageStatus,
  MessageCategory,
  CreateMessageRequest,
} from '@/types'
import { api } from '@/lib/api'

export interface UseMessagesOptions {
  /**
   * Auto-refresh interval in milliseconds (0 = no auto-refresh)
   */
  refreshInterval?: number

  /**
   * Filter by category
   */
  category?: MessageCategory

  /**
   * Filter by severity
   */
  severity?: MessageSeverity

  /**
   * Filter by status
   */
  status?: MessageStatus

  /**
   * Whether to fetch on mount
   */
  enabled?: boolean
}

export interface UseMessagesResult {
  /**
   * List of messages
   */
  messages: NotificationMessage[]

  /**
   * Message statistics
   */
  stats: MessageStats | null

  /**
   * Loading state
   */
  isLoading: boolean

  /**
   * Error state
   */
  error: Error | null

  /**
   * Refresh messages
   */
  refresh: () => Promise<void>

  /**
   * Create a new message
   */
  createMessage: (req: CreateMessageRequest) => Promise<void>

  /**
   * Acknowledge a single message
   */
  acknowledgeMessage: (id: string) => Promise<void>

  /**
   * Resolve a single message
   */
  resolveMessage: (id: string) => Promise<void>

  /**
   * Archive a single message
   */
  archiveMessage: (id: string) => Promise<void>

  /**
   * Delete a single message
   */
  deleteMessage: (id: string) => Promise<void>

  /**
   * Bulk acknowledge messages
   */
  bulkAcknowledge: (ids: string[]) => Promise<number>

  /**
   * Bulk resolve messages
   */
  bulkResolve: (ids: string[]) => Promise<number>

  /**
   * Bulk delete messages
   */
  bulkDelete: (ids: string[]) => Promise<number>

  /**
   * Cleanup old messages
   */
  cleanup: (olderThanDays: number) => Promise<void>

  /**
   * Clear local error
   */
  clearError: () => void
}

/**
 * useMessages - Hook for managing messages/notifications
 *
 * @example
 * ```tsx
 * function MessagesPage() {
 *   const { messages, stats, isLoading, acknowledgeMessage, resolveMessage } = useMessages({
 *     enabled: true,
 *     refreshInterval: 5000,
 *   })
 *
 *   return (
 *     <div>
 *       <p>Active: {stats?.active || 0}</p>
 *       {messages.map(m => (
 *         <div key={m.id}>
 *           <h3>{m.title}</h3>
 *           <p>{m.message}</p>
 *           <button onClick={() => acknowledgeMessage(m.id)}>Acknowledge</button>
 *           <button onClick={() => resolveMessage(m.id)}>Resolve</button>
 *         </div>
 *       ))}
 *     </div>
 *   )
 * }
 * ```
 */
export function useMessages(options: UseMessagesOptions = {}): UseMessagesResult {
  const {
    refreshInterval = 0,
    category,
    severity,
    status,
    enabled = true,
  } = options

  const [messages, setMessages] = useState<NotificationMessage[]>([])
  const [stats, setStats] = useState<MessageStats | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  // Fetch messages and stats
  const refresh = useCallback(async () => {
    if (!enabled) return

    setIsLoading(true)
    setError(null)

    try {
      const [messagesResponse, statsResponse] = await Promise.all([
        api.getMessages(),
        api.getMessageStats(),
      ])

      let filteredMessages = messagesResponse.messages || []

      // Apply filters
      if (category) {
        filteredMessages = filteredMessages.filter(m => m.category === category)
      }
      if (severity) {
        filteredMessages = filteredMessages.filter(m => m.severity === severity)
      }
      if (status) {
        filteredMessages = filteredMessages.filter(m => m.status === status)
      }

      // Sort by timestamp descending
      filteredMessages.sort((a, b) =>
        new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
      )

      setMessages(filteredMessages)
      setStats(statsResponse)
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
    } finally {
      setIsLoading(false)
    }
  }, [enabled, category, severity, status])

  // Create a new message
  const createMessage = useCallback(async (req: CreateMessageRequest) => {
    setIsLoading(true)
    setError(null)

    try {
      await api.createMessage(req)
      await refresh()
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [refresh])

  // Acknowledge a single message
  const acknowledgeMessage = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)

    try {
      await api.acknowledgeMessage(id)
      setMessages(prev => prev.map(m =>
        m.id === id ? { ...m, status: 'acknowledged' as MessageStatus } : m
      ))
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Resolve a single message
  const resolveMessage = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)

    try {
      await api.resolveMessage(id)
      setMessages(prev => prev.map(m =>
        m.id === id ? { ...m, status: 'resolved' as MessageStatus } : m
      ))
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Archive a single message
  const archiveMessage = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)

    try {
      await api.archiveMessage(id)
      setMessages(prev => prev.map(m =>
        m.id === id ? { ...m, status: 'archived' as MessageStatus } : m
      ))
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Delete a single message
  const deleteMessage = useCallback(async (id: string) => {
    setIsLoading(true)
    setError(null)

    try {
      await api.deleteMessage(id)
      setMessages(prev => prev.filter(m => m.id !== id))
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Bulk acknowledge messages
  const bulkAcknowledge = useCallback(async (ids: string[]) => {
    setIsLoading(true)
    setError(null)

    try {
      const result = await api.bulkAcknowledgeMessages(ids)
      setMessages(prev => prev.map(m =>
        ids.includes(m.id) ? { ...m, status: 'acknowledged' as MessageStatus } : m
      ))
      return result.acknowledged
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Bulk resolve messages
  const bulkResolve = useCallback(async (ids: string[]) => {
    setIsLoading(true)
    setError(null)

    try {
      const result = await api.bulkResolveMessages(ids)
      setMessages(prev => prev.map(m =>
        ids.includes(m.id) ? { ...m, status: 'resolved' as MessageStatus } : m
      ))
      return result.resolved
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Bulk delete messages
  const bulkDelete = useCallback(async (ids: string[]) => {
    setIsLoading(true)
    setError(null)

    try {
      const result = await api.bulkDeleteMessages(ids)
      setMessages(prev => prev.filter(m => !ids.includes(m.id)))
      return result.deleted
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [])

  // Cleanup old messages
  const cleanup = useCallback(async (olderThanDays: number) => {
    setIsLoading(true)
    setError(null)

    try {
      await api.cleanupMessages({ older_than_days: olderThanDays })
      await refresh()
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [refresh])

  const clearError = useCallback(() => {
    setError(null)
  }, [])

  // Fetch on mount
  useEffect(() => {
    refresh()
  }, [refresh])

  // Auto-refresh
  useEffect(() => {
    if (refreshInterval > 0) {
      const interval = setInterval(refresh, refreshInterval)
      return () => clearInterval(interval)
    }
  }, [refresh, refreshInterval])

  return {
    messages,
    stats,
    isLoading,
    error,
    refresh,
    createMessage,
    acknowledgeMessage,
    resolveMessage,
    archiveMessage,
    deleteMessage,
    bulkAcknowledge,
    bulkResolve,
    bulkDelete,
    cleanup,
    clearError,
  }
}

/**
 * useMessageChannels - Hook for managing message channels
 */
export interface UseMessageChannelsResult {
  channels: import('@/types').MessageChannel[]
  channelTypes: import('@/types').ChannelTypeInfo[]
  stats: import('@/types').ChannelStats | null
  isLoading: boolean
  error: Error | null
  refresh: () => Promise<void>
  createChannel: (req: import('@/types').CreateMessageChannelRequest) => Promise<void>
  deleteChannel: (name: string) => Promise<void>
  testChannel: (name: string) => Promise<import('@/types').ChannelTestResult>
  clearError: () => void
}

export function useMessageChannels(): UseMessageChannelsResult {
  const [channels, setChannels] = useState<import('@/types').MessageChannel[]>([])
  const [channelTypes, setChannelTypes] = useState<import('@/types').ChannelTypeInfo[]>([])
  const [stats, setStats] = useState<import('@/types').ChannelStats | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  const refresh = useCallback(async () => {
    setIsLoading(true)
    setError(null)

    try {
      const [channelsResponse, typesResponse, statsResponse] = await Promise.all([
        api.listMessageChannels(),
        api.listChannelTypes(),
        api.getChannelStats(),
      ])

      setChannels(channelsResponse.channels || [])
      setChannelTypes(typesResponse.types || [])
      setStats(statsResponse)
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
    } finally {
      setIsLoading(false)
    }
  }, [])

  const createChannel = useCallback(async (req: import('@/types').CreateMessageChannelRequest) => {
    setIsLoading(true)
    setError(null)

    try {
      await api.createMessageChannel(req)
      await refresh()
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [refresh])

  const deleteChannel = useCallback(async (name: string) => {
    setIsLoading(true)
    setError(null)

    try {
      await api.deleteMessageChannel(name)
      await refresh()
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [refresh])

  const testChannel = useCallback(async (name: string) => {
    setError(null)

    try {
      const result = await api.testMessageChannel(name)
      return result
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
      throw err
    }
  }, [])

  const clearError = useCallback(() => {
    setError(null)
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  return {
    channels,
    channelTypes,
    stats,
    isLoading,
    error,
    refresh,
    createChannel,
    deleteChannel,
    testChannel,
    clearError,
  }
}
