// WebSocket Manager for Chat
import type { ServerMessage, ClientChatMessage, ChatImage } from '@/types'
import { tokenManager } from './auth'

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
    const isTauri = !!(window as any).__TAURI__
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
    }

    this.ws.onclose = (event) => {
      this.notifyConnection(false)
      // Don't reconnect if the server rejected us (auth error) or normal close
      // Code 4000 is used for token change - we DO want to reconnect
      if (event.code !== 1000 && event.code !== 4001) {
        this.scheduleReconnect()
      } else {
        // P0: Clear pending messages on permanent disconnect (auth error or normal close)
        if (this.pendingMessages.length > 0) {
          console.warn(`[WebSocket] Clearing ${this.pendingMessages.length} pending messages on close (code: ${event.code})`)
          this.pendingMessages = []
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
        if (data.type === 'Error') {
          if (data.message?.includes('token') || data.message?.includes('Authentication')) {
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
    } else {
      // Queue message for when connected
      // P0: Prevent unbounded growth - evict oldest if limit reached
      if (this.pendingMessages.length >= this.MAX_PENDING_MESSAGES) {
        // Remove oldest message (FIFO)
        this.pendingMessages.shift()
        console.warn('[WebSocket] Pending messages limit reached, dropping oldest message')
      }
      this.pendingMessages.push(request)
    }
  }

  sendMessage(content: string, images?: ChatImage[]) {
    this.sendRequest({
      message: content,
      images: images,
      sessionId: this.sessionId || undefined,
      backendId: this.activeBackendId || undefined,
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
      // P0: Clear pending messages when we give up reconnecting
      if (this.pendingMessages.length > 0) {
        console.warn(`[WebSocket] Clearing ${this.pendingMessages.length} pending messages after failed reconnect`)
        this.pendingMessages = []
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

    // P0: Clear pending messages on manual disconnect to prevent memory leak
    if (this.pendingMessages.length > 0) {
      console.warn(`[WebSocket] Clearing ${this.pendingMessages.length} pending messages on disconnect`)
      this.pendingMessages = []
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

  isConnected() {
    return this.ws?.readyState === WebSocket.OPEN
  }
}

// Singleton instance
export const ws = new ChatWebSocket()
