// WebSocket Manager for Chat
import type { ServerMessage, ClientChatMessage, ChatImage } from '@/types'
import { tokenManager } from './auth'
import { storage } from './utils/storage'
import { isTauriEnv } from './api'

type MessageHandler = (message: ServerMessage) => void
type ConnectionHandler = (connected: boolean, isReconnect?: boolean) => void
type StateChangeHandler = (state: ConnectionState) => void

// Connection state for UI display
export interface ConnectionState {
  status: 'connected' | 'disconnected' | 'reconnecting' | 'error'
  retryCount?: number
  nextRetryIn?: number  // seconds
  errorMessage?: string
}

// Persistence configuration
const STORAGE_KEY = 'neomind_ws_pending_messages'
const MAX_STORED_MESSAGES = 100  // Persistent storage limit
const MAX_MESSAGE_AGE_MS = 24 * 60 * 60 * 1000  // 24 hours

// Stored message with metadata
interface StoredMessage {
  message: ClientChatMessage
  timestamp: number
  retries: number
}

export class ChatWebSocket {
  private ws: WebSocket | null = null
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private countdownTimer: ReturnType<typeof setInterval> | null = null
  private tokenCheckTimer: ReturnType<typeof setInterval> | null = null
  private reconnectAttempts = 0
  private maxReconnectAttempts = 10  // 最多重连10次
  private baseReconnectDelay = 1000  // 初始重连延迟1秒
  private maxReconnectDelay = 30000   // 最大重连延迟30秒
  private isManualDisconnect = false  // 是否用户主动断开
  private wasConnected = false  // 跟踪是否曾经连接过（用于区分初始连接和重连）
  private messageHandlers: Set<MessageHandler> = new Set()
  private connectionHandlers: Set<ConnectionHandler> = new Set()
  private stateChangeHandlers: Set<StateChangeHandler> = new Set()
  private sessionId: string | null = null
  private activeBackendId: string | null = null
  private pendingMessages: ClientChatMessage[] = []
  private readonly MAX_PENDING_MESSAGES = 50  // P0: Limit pending messages to prevent memory leak
  private lastToken: string | null = null
  private currentState: ConnectionState = { status: 'disconnected' }

  constructor() {
    this.loadPendingMessages()
  }

  private loadPendingMessages() {
    const stored = storage.get<StoredMessage[]>(STORAGE_KEY, []) || []
    const now = Date.now()

    // Filter expired messages (24 hours)
    const valid = stored.filter(m =>
      now - m.timestamp < MAX_MESSAGE_AGE_MS
    )

    // Restore to memory queue (up to MAX_PENDING_MESSAGES)
    this.pendingMessages = valid
      .slice(0, this.MAX_PENDING_MESSAGES)
      .map(m => m.message)

  }

  connect(initialSessionId?: string) {
    // Clear any existing timers
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    if (this.tokenCheckTimer) {
      clearInterval(this.tokenCheckTimer)
      this.tokenCheckTimer = null
    }

    this.sessionId = initialSessionId || null

    // In Tauri desktop app, use localhost:9375 for WebSocket
    // because window.location would be tauri://localhost
    const isTauri = isTauriEnv()
    const isSecure = window.location.protocol === 'https:'
    const protocol = (isTauri ? false : isSecure) ? 'wss:' : 'ws:'
    const host = isTauri ? 'localhost:9375' : window.location.host
    let wsUrl = `${protocol}//${host}/api/chat`

    // Add JWT token as query parameter
    const token = tokenManager.getToken()

    if (!token) {
      this.disconnect()
      this.notifyConnection(false)

      // Poll for token
      this.tokenCheckTimer = setInterval(() => {
        const newToken = tokenManager.getToken()
        if (newToken) {
          if (this.tokenCheckTimer) {
            clearInterval(this.tokenCheckTimer)
            this.tokenCheckTimer = null
          }
          this.connect(this.sessionId || undefined)
        }
      }, 500)
      return
    }

    // If token changed, reconnect with new token
    if (this.lastToken !== token && this.isConnected()) {
      // Don't use disconnect() as it sets isManualDisconnect = true
      // Just close and let the loop reconnect with new token
      if (this.ws) {
        // Use a non-1000 code so onclose will trigger reconnect
        this.ws.close(4000, 'Token changed - reconnecting')
      }
      // Keep lastToken as the old one until we reconnect
      // After reconnect, the new token will be used
      return
    }

    this.lastToken = token

    if (token) {
      wsUrl += `?token=${encodeURIComponent(token)}`
    }

    // Don't create a new WebSocket if we're already connected with the same token
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      // Already connected, no need to reconnect
      return
    }

    // Close existing WebSocket if it's in a bad state
    if (this.ws) {
      this.ws.close()
      this.ws = null
    }

    this.ws = new WebSocket(wsUrl)

    this.ws.onopen = () => {
      const isReconnect = this.wasConnected
      this.wasConnected = true  // 标记已连接过

      this.resetReconnectState()  // 重置重连状态
      this.notifyConnection(true, isReconnect)
      this.setState({ status: 'connected' })

      // Send pending messages
      while (this.pendingMessages.length > 0) {
        const msg = this.pendingMessages.shift()!
        this.sendRequest(msg)
      }

      // If all messages sent successfully, clear storage
      if (this.pendingMessages.length === 0) {
        storage.remove(STORAGE_KEY)
      }
    }

    this.ws.onclose = (event) => {
      this.notifyConnection(false)
      // Don't reconnect if the server rejected us (auth error) or normal close
      // Code 4000 is used for token change - we DO want to reconnect
      if (event.code !== 1000 && event.code !== 4001) {
        this.scheduleReconnect()
      } else {
        // Clear pending messages only on auth error (4001)
        if (event.code === 4001 && this.pendingMessages.length > 0) {
          console.warn(`[WebSocket] Auth error, clearing ${this.pendingMessages.length} pending messages`)
          this.pendingMessages = []
          storage.remove(STORAGE_KEY)
        } else if (event.code === 1000) {
          // Normal close - persist messages for next session
          this.persistPendingMessages()
        }
        this.setState({ status: 'disconnected' })
      }
    }

    this.ws.onerror = () => {
      this.setState({ status: 'error', errorMessage: 'Connection error' })
    }

    this.ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as ServerMessage

        // Auto-respond to ping from server
        if (data.type === 'ping') {
          if (this.ws?.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify({ type: 'pong' }))
          }
          return // Don't notify handlers about ping/pong
        }

        // Handle auth error message from server
        // IMPORTANT: Only trigger reload for actual auth errors, NOT for LLM errors
        // that happen to contain "token" (e.g., "request exceeds context size, 8225 tokens")
        if (data.type === 'Error') {
          const msg = (data.message || '').toLowerCase()
          const isAuthError = msg.includes('authentication') ||
            msg.includes('unauthorized') ||
            msg.includes('invalid api key') ||
            msg.includes('access denied') ||
            msg.includes('jwt') ||
            (msg.includes('token') && (msg.includes('expired') || msg.includes('invalid')))
          if (isAuthError) {
            // Stop reconnecting on auth failure
            this.disconnect()
            // Trigger a page reload to show login screen
            setTimeout(() => window.location.reload(), 1000)
            return
          }
        }
        this.notifyMessage(data)
      } catch {
        // Silent error handling - ignore malformed messages
      }
    }
  }

  sendRequest(request: ClientChatMessage) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(request))
      this.removeFromStorage(request)  // Remove from storage on successful send
    } else {
      // Queue message for when connected
      // P0: Prevent unbounded growth - evict oldest if limit reached
      if (this.pendingMessages.length >= this.MAX_PENDING_MESSAGES) {
        // Remove oldest message (FIFO)
        this.pendingMessages.shift()
        console.warn('[WebSocket] Pending messages limit reached, dropping oldest message')
      }
      this.pendingMessages.push(request)
      this.persistPendingMessages()  // Immediately persist to storage
    }
  }

  sendMessage(content: string, images?: ChatImage[], selectedSkills?: string[]) {
    this.sendRequest({
      message: content,
      images: images,
      sessionId: this.sessionId || undefined,
      backendId: this.activeBackendId || undefined,
      selectedSkills: selectedSkills && selectedSkills.length > 0 ? selectedSkills : undefined,
    })
  }

  setActiveBackend(backendId: string | null) {
    this.activeBackendId = backendId
  }

  setSessionId(sessionId: string) {
    this.sessionId = sessionId
  }

  getSessionId() {
    return this.sessionId
  }

  private scheduleReconnect() {
    // 如果是用户主动断开，不重连
    if (this.isManualDisconnect) {
      this.setState({
        status: 'disconnected',
        errorMessage: undefined
      })
      return
    }

    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      // Persist messages when we give up reconnecting (don't clear)
      if (this.pendingMessages.length > 0) {
        console.warn(`[WebSocket] Max reconnect attempts reached. ${this.pendingMessages.length} messages persisted for next session`)
        this.persistPendingMessages()
      }
      this.setState({
        status: 'error',
        errorMessage: '连接已断开，请点击重新连接',
        retryCount: this.reconnectAttempts,
        nextRetryIn: undefined
      })
      return
    }

    // 使用指数退避算法，但有最大延迟限制
    const delay = Math.min(
      this.baseReconnectDelay * Math.pow(2, this.reconnectAttempts),
      this.maxReconnectDelay
    )
    this.reconnectAttempts++

    // Set reconnecting state
    this.setState({
      status: 'reconnecting',
      retryCount: this.reconnectAttempts,
      nextRetryIn: Math.ceil(delay / 1000)
    })

    // Start countdown
    let countdown = Math.ceil(delay / 1000)
    if (this.countdownTimer) {
      clearInterval(this.countdownTimer)
    }
    this.countdownTimer = setInterval(() => {
      countdown--
      this.setState({
        status: 'reconnecting',
        retryCount: this.reconnectAttempts,
        nextRetryIn: countdown
      })
      if (countdown <= 0) {
        if (this.countdownTimer) {
          clearInterval(this.countdownTimer)
          this.countdownTimer = null
        }
      }
    }, 1000)

    this.reconnectTimer = setTimeout(() => {
      if (this.countdownTimer) {
        clearInterval(this.countdownTimer)
        this.countdownTimer = null
      }
      this.connect(this.sessionId || undefined)
    }, delay)
  }

  onMessage(handler: MessageHandler) {
    this.messageHandlers.add(handler)
    return () => this.messageHandlers.delete(handler)
  }

  onConnection(handler: ConnectionHandler) {
    this.connectionHandlers.add(handler)
    handler(this.ws?.readyState === WebSocket.OPEN, false)
    return () => this.connectionHandlers.delete(handler)
  }

  private notifyMessage(message: ServerMessage) {
    this.messageHandlers.forEach(handler => handler(message))
  }

  private notifyConnection(connected: boolean, isReconnect = false) {
    this.connectionHandlers.forEach(handler => handler(connected, isReconnect))
  }

  private setState(state: ConnectionState) {
    this.currentState = state
    this.stateChangeHandlers.forEach(handler => handler(state))
  }

  // Public API for connection state
  getState(): ConnectionState {
    return this.currentState
  }

  onStateChange(handler: StateChangeHandler): () => void {
    this.stateChangeHandlers.add(handler)
    handler(this.currentState) // Immediately call with current state
    return () => this.stateChangeHandlers.delete(handler)
  }

  /**
   * 手动断开连接（不会触发重连）
   */
  disconnect() {
    this.isManualDisconnect = true
    this.reconnectAttempts = 0

    // Clear timers
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    if (this.countdownTimer) {
      clearInterval(this.countdownTimer)
      this.countdownTimer = null
    }
    if (this.tokenCheckTimer) {
      clearInterval(this.tokenCheckTimer)
      this.tokenCheckTimer = null
    }

    // Close WebSocket
    if (this.ws) {
      this.ws.close(1000, 'User disconnected')
      this.ws = null
    }

    // Persist pending messages on manual disconnect (don't clear)
    if (this.pendingMessages.length > 0) {
      this.persistPendingMessages()
    }

    this.setState({ status: 'disconnected' })
  }

  /**
   * 手动重新连接（重置重连计数器）
   */
  manualReconnect() {
    this.isManualDisconnect = false
    this.reconnectAttempts = 0

    // Clear existing timers
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    if (this.countdownTimer) {
      clearInterval(this.countdownTimer)
      this.countdownTimer = null
    }

    // Close existing WebSocket and clear reference before connecting
    // This prevents race conditions where the old socket triggers events
    if (this.ws) {
      if (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING) {
        this.ws.close()
      }
      this.ws = null  // Clear immediately to prevent race conditions
    }

    this.connect(this.sessionId || undefined)
  }

  /**
   * 重置重连状态（用于连接成功后）
   */
  private resetReconnectState() {
    this.reconnectAttempts = 0
    this.isManualDisconnect = false
  }

  /**
   * Persist pending messages to localStorage
   */
  private persistPendingMessages() {
    const now = Date.now()
    const toStore: StoredMessage[] = this.pendingMessages.map(msg => ({
      message: msg,
      timestamp: now,
      retries: 0
    }))

    // Limit storage to MAX_STORED_MESSAGES
    const limited = toStore.slice(-MAX_STORED_MESSAGES)
    storage.set(STORAGE_KEY, limited)
  }

  /**
   * Remove a specific message from storage after successful send
   */
  private removeFromStorage(request: ClientChatMessage) {
    const stored = storage.get<StoredMessage[]>(STORAGE_KEY, []) || []
    const filtered = stored.filter(m =>
      m.message.sessionId !== request.sessionId ||
      m.message.message !== request.message
    )

    if (filtered.length < stored.length) {
      storage.set(STORAGE_KEY, filtered)
    }
  }

  isConnected() {
    return this.ws?.readyState === WebSocket.OPEN
  }
}

// Singleton instance
export const ws = new ChatWebSocket()
