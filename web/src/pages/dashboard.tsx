import { useState, useEffect, useRef, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { useStore } from "@/store"
import { ws } from "@/lib/websocket"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Send, Bot, User, Settings, Copy, Check, Wrench, Brain, MessageSquare } from "lucide-react"
import { cn } from "@/lib/utils"
import type { Message, ServerMessage, ToolCall } from "@/types"
import { SessionSidebar, MarkdownMessage, ConnectionStatus, ThinkingBlock, ToolCallVisualization } from "@/components/chat"
import type { ConnectionState } from "@/lib/websocket"

export function DashboardPage() {
  const { t } = useTranslation(['common', 'dashboard'])

  // Use selectors to subscribe only to specific state fields
  // This prevents unnecessary re-renders when other parts of the store change
  const messages = useStore((state) => state.messages)
  const sessions = useStore((state) => state.sessions)
  const sessionId = useStore((state) => state.sessionId)
  const setSessionId = useStore((state) => state.setSessionId)
  const addMessage = useStore((state) => state.addMessage)
  const setWsConnected = useStore((state) => state.setWsConnected)
  const wsConnected = useStore((state) => state.wsConnected)
  const setCurrentPage = useStore((state) => state.setCurrentPage)
  const loadSessions = useStore((state) => state.loadSessions)
  const llmBackends = useStore((state) => state.llmBackends)
  const llmBackendLoading = useStore((state) => state.llmBackendLoading)
  const loadBackends = useStore((state) => state.loadBackends)
  const activateBackend = useStore((state) => state.activateBackend)
  const updateBackend = useStore((state) => state.updateBackend)
  const [input, setInput] = useState("")
  const [isStreaming, setIsStreaming] = useState(false)
  const [streamingContent, setStreamingContent] = useState("")
  const [streamingThinking, setStreamingThinking] = useState("")
  const [streamingToolCalls, setStreamingToolCalls] = useState<ToolCall[]>([])
  const [copiedId, setCopiedId] = useState<string | null>(null)
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const [connectionState, setConnectionState] = useState<ConnectionState>({ status: 'disconnected' })
  const [hasLoadedBackendsOnce, setHasLoadedBackendsOnce] = useState(false)

  const scrollRef = useRef<HTMLDivElement>(null)
  const streamingContentRef = useRef("")
  const streamingThinkingRef = useRef("")
  const streamingToolCallsRef = useRef<ToolCall[]>([])
  const isMountedRef = useRef(true)
  // Track partial message ID for incremental saving
  const partialMessageIdRef = useRef<string | null>(null)
  // Track last save time to avoid too frequent updates
  const lastPartialSaveRef = useRef<number>(0)

  // Cleanup on unmount
  useEffect(() => {
    isMountedRef.current = true
    return () => {
      isMountedRef.current = false
    }
  }, [])

  // Load LLM backends
  const hasFetchedBackends = useRef(false)
  useEffect(() => {
    if (!hasFetchedBackends.current && isMountedRef.current) {
      hasFetchedBackends.current = true
      loadBackends().then(() => {
        if (isMountedRef.current) {
          setHasLoadedBackendsOnce(true)
        }
      })
    }
  }, [])

  // Load sessions on mount (once)
  const hasFetchedSessions = useRef(false)
  useEffect(() => {
    if (!hasFetchedSessions.current) {
      hasFetchedSessions.current = true
      loadSessions()
    }
  }, [])

  // Handle backend change
  const handleBackendChange = async (backendId: string) => {
    await activateBackend(backendId)
    ws.setActiveBackend(backendId)
  }

  // Handle thinking toggle
  const handleThinkingToggle = async (checked: boolean) => {
    const activeBackend = llmBackends?.find(b => b.is_active)
    if (activeBackend) {
      await updateBackend(activeBackend.id, { thinking_enabled: checked })
    }
  }

  // Initialize active backend in WebSocket when backends are loaded
  useEffect(() => {
    if (hasLoadedBackendsOnce && llmBackends && llmBackends.length > 0) {
      const activeBackend = llmBackends.find(b => b.is_active)
      if (activeBackend) {
        ws.setActiveBackend(activeBackend.id)
      }
    }
  }, [hasLoadedBackendsOnce, llmBackends])

  // Initialize session
  useEffect(() => {
    if (sessionId) {
      ws.setSessionId(sessionId)
      if (!ws.isConnected()) {
        ws.connect(sessionId)
      }
    }
  }, [sessionId])

  // Setup WebSocket connection handlers
  useEffect(() => {
    const unsubscribeConn = ws.onConnection((connected) => {
      setWsConnected(connected)
    })

    // Listen for detailed connection state changes
    const unsubscribeState = ws.onStateChange((state) => {
      setConnectionState(state)
    })

    return () => {
      unsubscribeConn()
      unsubscribeState()
    }
  }, [setWsConnected])

  // Auto-create session when connected and no session exists
  useEffect(() => {
    if (wsConnected && !sessionId) {
      // Create a new session via API
      fetch('/api/sessions', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
      })
        .then(res => res.json())
        .then(data => {
          if (data.data?.id) {
            setSessionId(data.data.id)
            ws.setSessionId(data.data.id)
          }
        })
        .catch(err => {
          console.error('Failed to create session:', err)
        })
    }
  }, [wsConnected, sessionId, setSessionId])

  // Reset streaming states when sessionId changes
  useEffect(() => {
    // Before clearing, save any partial message that was being streamed
    // This prevents data loss when switching sessions during an active stream
    const content = streamingContentRef.current
    const thinking = streamingThinkingRef.current
    const toolCalls = streamingToolCallsRef.current

    // Only save if we have actual content (not just empty state)
    if (content || thinking || toolCalls.length > 0) {
      const partialMsg: Message = {
        id: partialMessageIdRef.current || crypto.randomUUID(),
        role: "assistant",
        content: content,
        timestamp: Date.now(),
        thinking: thinking || undefined,
        tool_calls: toolCalls.length > 0 ? toolCalls : undefined,
        // Mark as non-partial since we're finalizing it
        isPartial: false,
      }
      // Save the message before switching sessions
      addMessage(partialMsg)
      console.log('[dashboard] Saved partial message before session switch:', {
        contentLength: content.length,
        thinkingLength: thinking.length,
        toolCallsCount: toolCalls.length,
      })
    }

    // Now reset all streaming states when switching sessions
    setIsStreaming(false)
    setStreamingContent("")
    setStreamingThinking("")
    setStreamingToolCalls([])
    // Also reset refs to ensure no stale data persists
    streamingContentRef.current = ""
    streamingThinkingRef.current = ""
    streamingToolCallsRef.current = []
    partialMessageIdRef.current = null
    lastPartialSaveRef.current = 0
  }, [sessionId, addMessage])

  // Setup WebSocket message handler
  // Use ref to avoid re-subscription when addMessage changes
  const addMessageRef = useRef(addMessage)
  const loadSessionsRef = useRef(loadSessions)
  const setSessionIdRef = useRef(setSessionId)

  useEffect(() => {
    addMessageRef.current = addMessage
    loadSessionsRef.current = loadSessions
    setSessionIdRef.current = setSessionId
  }, [addMessage, loadSessions, setSessionId])

  // Function to save/update partial message during streaming
  const savePartialMessage = useCallback(() => {
    const content = streamingContentRef.current
    const thinking = streamingThinkingRef.current
    const toolCalls = streamingToolCallsRef.current

    // Skip if nothing to save
    if (!content && !thinking && toolCalls.length === 0) return

    const partialMsg: Message = {
      id: partialMessageIdRef.current || crypto.randomUUID(),
      role: "assistant",
      content: content,
      timestamp: Date.now(),
      thinking: thinking || undefined,
      tool_calls: toolCalls.length > 0 ? toolCalls : undefined,
      // Mark as partial to indicate it's still being updated
      isPartial: true,
    }

    // Store the ID for future updates
    if (!partialMessageIdRef.current) {
      partialMessageIdRef.current = partialMsg.id
      // Add new partial message
      addMessageRef.current(partialMsg)
    } else {
      // Update existing partial message
      // We need to update the message in the list
      addMessageRef.current(partialMsg)
    }
  }, [addMessageRef])

  const handleMessage = useCallback((msg: ServerMessage) => {
    // Debug: Log all incoming message types
    console.log('[dashboard WS] Received message type:', msg.type, {
      hasSessionId: 'sessionId' in msg,
      sessionId: (msg as any).sessionId,
      currentSessionId: sessionId,
    })

    // Only process messages that belong to the current session
    // Messages without sessionId (device_update) are always processed
    // Control messages (session_created, session_switched) are always processed
    const hasSessionId = 'sessionId' in msg && (msg as any).sessionId !== undefined
    if (hasSessionId && msg.type !== 'session_created' && msg.type !== 'session_switched') {
      // If there's no active session, or sessionId doesn't match, ignore
      if (!sessionId || (msg as any).sessionId !== sessionId) {
        console.log('[dashboard] Ignoring message for different session', {
          msgSessionId: (msg as any).sessionId,
          currentSessionId: sessionId,
          msgType: msg.type,
        })
        return
      }
    }

    switch (msg.type) {
      case 'system':
        break

      case 'session_created':
      case 'session_switched':
        console.log('[dashboard] Processing session event:', msg.type, msg.sessionId)
        if (msg.sessionId) {
          // Update BOTH React state and WebSocket instance
          setSessionIdRef.current(msg.sessionId)
          ws.setSessionId(msg.sessionId)
          // Refresh the sessions list to show the new/updated session
          loadSessionsRef.current()
        }
        break

      case 'Thinking':
        console.log('[dashboard] Thinking event, content length:', (msg.content || '').length)
        setIsStreaming(true)
        // Update ref synchronously to avoid stale data in end event
        streamingThinkingRef.current += (msg.content || "")
        setStreamingThinking(streamingThinkingRef.current)

        // Incremental save: Update partial message every 500ms
        const now = Date.now()
        if (now - lastPartialSaveRef.current > 500) {
          lastPartialSaveRef.current = now
          savePartialMessage()
        }
        break

      case 'Content':
        console.log('[dashboard] Content event, content length:', (msg.content || '').length)
        setIsStreaming(true)
        // Update ref synchronously to avoid stale data in end event
        streamingContentRef.current += (msg.content || "")
        setStreamingContent(streamingContentRef.current)

        // Incremental save: Update partial message every 500ms
        const now2 = Date.now()
        if (now2 - lastPartialSaveRef.current > 500) {
          lastPartialSaveRef.current = now2
          savePartialMessage()
        }
        break

      case 'ToolCallStart':
        console.log('[dashboard] ToolCallStart event:', msg.tool)
        setIsStreaming(true)
        const newToolCall: ToolCall = {
          id: crypto.randomUUID(),
          name: msg.tool || "",
          arguments: msg.arguments,
        }
        streamingToolCallsRef.current.push(newToolCall)
        setStreamingToolCalls([...streamingToolCallsRef.current])
        // Save partial message when tool call starts
        savePartialMessage()
        break

      case 'ToolCallEnd':
        streamingToolCallsRef.current = streamingToolCallsRef.current.map((tc) =>
          tc.name === msg.tool
            ? { ...tc, result: msg.result }
            : tc
        )
        setStreamingToolCalls(streamingToolCallsRef.current)
        // Save partial message when tool call ends
        savePartialMessage()
        break

      case 'Error':
        console.error('Server error:', msg.message)
        setIsStreaming(false)
        // Clear partial message state on error
        partialMessageIdRef.current = null
        lastPartialSaveRef.current = 0
        break

      case 'end':
        console.log('*** [dashboard] END EVENT RECEIVED ***', {
          contentLength: streamingContentRef.current.length,
          thinkingLength: streamingThinkingRef.current.length,
          toolCallsCount: streamingToolCallsRef.current.length,
        })
        // Use refs directly since they're updated synchronously now
        const finalContent = streamingContentRef.current
        const finalThinking = streamingThinkingRef.current
        const finalCalls = streamingToolCallsRef.current

        console.log('[dashboard] end event received', {
          contentLength: finalContent.length,
          thinkingLength: finalThinking.length,
          toolCallsCount: finalCalls.length,
        })

        if (finalContent || finalThinking || finalCalls.length > 0) {
          // Convert partial message to final message, or create new one
          const assistantMsg: Message = {
            id: partialMessageIdRef.current || crypto.randomUUID(),
            role: "assistant",
            content: finalContent,
            timestamp: Date.now(),
            thinking: finalThinking || undefined,
            tool_calls: finalCalls.length > 0 ? finalCalls : undefined,
            // Remove partial flag to indicate this is the final message
            isPartial: false,
          }
          console.log('[dashboard] Adding final assistant message')
          addMessageRef.current(assistantMsg)
        } else {
          console.warn('[dashboard] end event received but no content to save')
        }

        // Clear refs immediately
        streamingContentRef.current = ""
        streamingThinkingRef.current = ""
        streamingToolCallsRef.current = []
        partialMessageIdRef.current = null
        lastPartialSaveRef.current = 0

        setStreamingContent("")
        setStreamingThinking("")
        setStreamingToolCalls([])
        setIsStreaming(false)
        console.log('[dashboard] Streaming state cleared, isStreaming=false')
        break

      case 'response':
        if (msg.content) {
          const assistantMsg: Message = {
            id: crypto.randomUUID(),
            role: "assistant",
            content: msg.content,
            timestamp: Date.now(),
          }
          addMessageRef.current(assistantMsg)
        }
        setIsStreaming(false)
        break

      case 'device_update':
        break
    }
  }, [sessionId]) // sessionId is used for message filtering

  // Register message handler once
  useEffect(() => {
    const unsubscribe = ws.onMessage(handleMessage)
    return () => {
      unsubscribe()
    }
  }, [handleMessage])

  // Auto-scroll to bottom
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [messages, streamingContent, streamingThinking, streamingToolCalls])

  const handleSend = () => {
    if (!input.trim() || isStreaming) return

    const userMsg: Message = {
      id: crypto.randomUUID(),
      role: "user",
      content: input,
      timestamp: Date.now(),
    }

    addMessage(userMsg)
    ws.sendMessage(input)
    setInput("")
    setIsStreaming(true)
  }

  const handleCopy = (content: string, id: string) => {
    navigator.clipboard.writeText(content)
    setCopiedId(id)
    setTimeout(() => setCopiedId(null), 2000)
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  // Show LLM setup prompt if no LLM backend is configured (only after first load attempt)
  // This prevents UI flashing during backend switching/updates
  if (hasLoadedBackendsOnce && (!llmBackends || llmBackends.length === 0)) {
    return (
      <div className="flex h-full flex-row relative">
        {/* Sidebar - always expanded on desktop/tablet */}
        <aside className="hidden md:flex w-64 flex-col border-r bg-muted/10 overflow-hidden">
          <SessionSidebar onNewChat={() => setInput("")} />
        </aside>

        {/* Mobile Sidebar - overlay drawer */}
        <div
          className={cn(
            "fixed inset-0 z-50 md:hidden transition-opacity duration-300",
            sidebarOpen ? "opacity-100" : "opacity-0 pointer-events-none"
          )}
        >
          {/* Backdrop */}
          <div
            className={cn(
              "absolute inset-0 bg-black/40 backdrop-blur-sm transition-opacity duration-300",
              sidebarOpen ? "opacity-100" : "opacity-0"
            )}
            onClick={() => setSidebarOpen(false)}
          />
          {/* Drawer - slides from left */}
          <div className={cn(
            "absolute left-0 top-0 bottom-0 w-72 max-w-[85vw] bg-background shadow-2xl transition-transform duration-300 ease-in-out",
            sidebarOpen ? "translate-x-0" : "-translate-x-full"
          )}>
            <SessionSidebar onNewChat={() => setInput("")} onClose={() => setSidebarOpen(false)} />
          </div>
        </div>

        {/* Main content */}
        <div className="flex h-full flex-1 flex-col relative">
          {/* Mobile FAB - left side of chat area */}
          <button
            onClick={() => setSidebarOpen(true)}
            className="md:hidden absolute left-4 top-4 z-10 bg-primary text-primary-foreground p-2 rounded-lg shadow-md hover:bg-primary/90 active:scale-95 transition-all duration-200 flex items-center gap-1.5"
          >
            <MessageSquare className="h-4 w-4" />
            {sessions && sessions.length > 0 && (
              <span className="text-xs font-medium">{sessions.length}</span>
            )}
          </button>

          <div className="flex h-[calc(100vh-100px)] items-center justify-center">
            <div className="text-center max-w-md">
              <div className="mx-auto mb-6 flex h-16 w-16 items-center justify-center rounded-xl bg-muted">
                <Settings className="h-8 w-8 text-muted-foreground" />
              </div>
              <h2 className="mb-3 text-lg font-semibold">{t('dashboard:llmNotConfigured')}</h2>
              <p className="mb-6 text-sm text-muted-foreground">
                {t('dashboard:llmNotConfiguredDesc')}
              </p>
              <Button onClick={() => setCurrentPage('settings')}>
                <Settings className="mr-2 h-4 w-4" />
                {t('dashboard:goToSettings')}
              </Button>
            </div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="flex h-full flex-row relative">
      {/* Sidebar - always expanded on desktop/tablet */}
      <aside className="hidden md:flex w-64 flex-col border-r bg-muted/10 overflow-hidden">
        <SessionSidebar onNewChat={() => setInput("")} />
      </aside>

      {/* Mobile Sidebar - overlay drawer */}
      <div
        className={cn(
          "fixed inset-0 z-50 md:hidden transition-opacity duration-300",
          sidebarOpen ? "opacity-100" : "opacity-0 pointer-events-none"
        )}
      >
        {/* Backdrop */}
        <div
          className={cn(
            "absolute inset-0 bg-black/40 backdrop-blur-sm transition-opacity duration-300",
            sidebarOpen ? "opacity-100" : "opacity-0"
          )}
          onClick={() => setSidebarOpen(false)}
        />
        {/* Drawer - slides from left */}
        <div className={cn(
          "absolute left-0 top-0 bottom-0 w-72 max-w-[85vw] bg-background shadow-2xl transition-transform duration-300 ease-in-out",
          sidebarOpen ? "translate-x-0" : "-translate-x-full"
        )}>
          <SessionSidebar onNewChat={() => setInput("")} onClose={() => setSidebarOpen(false)} />
        </div>
      </div>

      {/* Main content */}
      <div className="flex h-full flex-1 flex-col relative">
        {/* Mobile FAB - left side of chat area */}
        <button
          onClick={() => setSidebarOpen(true)}
          className="md:hidden absolute left-4 top-4 z-10 bg-primary text-primary-foreground p-2 rounded-lg shadow-md hover:bg-primary/90 active:scale-95 transition-all duration-200 flex items-center gap-1.5"
        >
          <MessageSquare className="h-4 w-4" />
          {sessions && sessions.length > 0 && (
            <span className="text-xs font-medium">{sessions.length}</span>
          )}
        </button>

        <ScrollArea className="flex-1">
          <div className="sticky top-0 z-10 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 border-b px-4 py-2">
            <div className="flex items-center justify-between">
              <ConnectionStatus state={connectionState} />
              {sessionId && (
                <span className="text-xs text-muted-foreground">
                  会话: {sessionId.slice(0, 8)}
                </span>
              )}
            </div>
          </div>
          <div ref={scrollRef} className="p-4">
          {messages.length === 0 && !isStreaming && (
            <div className="flex h-[calc(100vh-200px)] items-center justify-center">
              <div className="text-center">
                <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-muted">
                  <Bot className="h-6 w-6 text-muted-foreground" />
                </div>
                <h3 className="mb-2 text-sm font-medium">{t('dashboard:startChat')}</h3>
                <p className="text-xs text-muted-foreground mb-4">
                  {wsConnected ? t('dashboard:chatReady') : t('dashboard:connecting')}
                </p>
              </div>
            </div>
          )}

          <div className="space-y-4">
            {messages.filter(msg => !msg.isPartial).map((msg) => {
              return (
              <div
                key={msg.id}
                className={cn(
                  "flex gap-3",
                  msg.role === "user" ? "justify-end" : "justify-start"
                )}
              >
                {msg.role === "assistant" && (
                  <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-muted">
                    <Bot className="h-4 w-4" />
                  </div>
                )}

                <div className="max-w-[80%]">
                  {/* Thinking */}
                  {msg.role === "assistant" && msg.thinking && (
                    <div className="mb-2">
                      <ThinkingBlock
                        thinking={msg.thinking}
                        defaultExpanded={true}
                      />
                    </div>
                  )}

                  {/* Tool Calls */}
                  {msg.role === "assistant" && msg.tool_calls && msg.tool_calls.length > 0 && (
                    <div className="mb-3">
                      <ToolCallVisualization toolCalls={msg.tool_calls} />
                    </div>
                  )}

                  {/* Message */}
                  <div
                    className={cn(
                      "rounded-lg px-3 py-2 text-sm",
                      msg.role === "user"
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted"
                    )}
                  >
                    {msg.role === "assistant" ? (
                      <MarkdownMessage content={msg.content} />
                    ) : (
                      <p className="whitespace-pre-wrap">{msg.content}</p>
                    )}
                    {msg.role === "assistant" && (
                      <button
                        onClick={() => handleCopy(msg.content, msg.id)}
                        className="mt-2 text-xs text-muted-foreground hover:text-foreground"
                      >
                        {copiedId === msg.id ? <Check className="h-3 w-3 inline" /> : <Copy className="h-3 w-3 inline" />}
                      </button>
                    )}
                  </div>
                </div>

                {msg.role === "user" && (
                  <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-muted">
                    <User className="h-4 w-4" />
                  </div>
                )}
              </div>
              )
            })}

            {/* Streaming Message */}
            {isStreaming && (streamingContent || streamingThinking || streamingToolCalls.length > 0) && (
              <div className="flex gap-3">
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-muted">
                  <Bot className="h-4 w-4" />
                </div>
                <div className="max-w-[80%]">
                  {streamingThinking && (
                    <div className="mb-2">
                      <ThinkingBlock
                        thinking={streamingThinking}
                        isStreaming={true}
                        defaultExpanded={true}
                      />
                    </div>
                  )}
                  {streamingToolCalls.length > 0 && (
                    <div className="mb-3">
                      <ToolCallVisualization toolCalls={streamingToolCalls} isStreaming={true} />
                    </div>
                  )}
                  <div className="rounded-lg px-3 py-2 text-sm bg-muted">
                    <div className="relative">
                      <MarkdownMessage content={streamingContent} />
                      <span className="inline-block w-1 h-4 bg-foreground animate-pulse ml-0.5 align-middle" />
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Loading indicator */}
            {isStreaming && !streamingContent && !streamingThinking && streamingToolCalls.length === 0 && (
              <div className="flex gap-3">
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-muted">
                  <Bot className="h-4 w-4" />
                </div>
                <div className="flex items-center px-3 py-2 bg-muted rounded-lg text-sm">
                  <span className="flex gap-1">
                    <span className="w-1 h-1 bg-foreground rounded-full animate-bounce" style={{ animationDelay: "0ms" }} />
                    <span className="w-1 h-1 bg-foreground rounded-full animate-bounce" style={{ animationDelay: "150ms" }} />
                    <span className="w-1 h-1 bg-foreground rounded-full animate-bounce" style={{ animationDelay: "300ms" }} />
                  </span>
                </div>
              </div>
            )}
          </div>
        </div>
      </ScrollArea>

      <div className="border-t bg-background pb-3">
        <div className="px-4 pt-3">
          {/* Unified input container with inline divider */}
          <div className="flex flex-col rounded-xl border border-input bg-background shadow-sm overflow-hidden">
            {/* Text input area with embedded send button */}
            <div className="relative">
              <textarea
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={t('dashboard:messagePlaceholder')}
                disabled={isStreaming}
                rows={2}
                className="w-full px-3 py-2 pr-10 text-sm bg-transparent placeholder:text-muted-foreground focus:outline-none resize-none leading-[1.25rem]"
                style={{ height: '42px', minHeight: '42px', maxHeight: '42px' }}
              />
              {/* Send button embedded in text area */}
              <button
                onClick={handleSend}
                disabled={!input.trim() || isStreaming}
                className="absolute right-1.5 top-1/2 -translate-y-1/2 shrink-0 h-7 w-7 rounded-md bg-black text-white hover:bg-black/80 dark:bg-white dark:text-black dark:hover:bg-white/90 disabled:opacity-40 disabled:cursor-not-allowed flex items-center justify-center transition-colors"
              >
                <Send className="h-4 w-4" />
              </button>
            </div>

            {/* Divider line */}
            <div className="h-px bg-border/40" />

            {/* Controls area */}
            <div className="flex items-center gap-2 px-2 py-1.5">
              {/* Model selector */}
              <Select
                value={llmBackends?.find(b => b.is_active)?.id || ""}
                onValueChange={handleBackendChange}
                disabled={isStreaming || llmBackendLoading || !llmBackends || llmBackends.length === 0}
              >
                <SelectTrigger className="h-7 w-[100px] text-xs shrink-0 border-0 bg-transparent hover:bg-muted/50 focus:ring-0 focus:ring-offset-0 rounded px-2 text-foreground">
                  <SelectValue placeholder={t('dashboard:selectBackend')}>
                    {llmBackends?.find(b => b.is_active)?.name}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent align="start" className="w-[160px]">
                  {llmBackends && llmBackends.length > 0 ? (
                    llmBackends.map((backend) => (
                      <SelectItem key={backend.id} value={backend.id} className="gap-2">
                        <div className="flex items-center gap-2 flex-1 min-w-0">
                          <span className="text-xs truncate">{backend.name}</span>
                          <div className="flex items-center gap-1 shrink-0 ml-auto">
                            {backend.capabilities?.supports_thinking && (
                              <Brain className="h-3 w-3 text-blue-500" />
                            )}
                            {backend.capabilities?.supports_tools && (
                              <Wrench className="h-3 w-3 text-green-500" />
                            )}
                            {backend.capabilities?.supports_multimodal && (
                              <svg className="h-3 w-3 text-purple-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                                <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                                <circle cx="8.5" cy="8.5" r="1.5" />
                                <polyline points="21 15 16 10 5 21" />
                            </svg>
                            )}
                          </div>
                        </div>
                      </SelectItem>
                    ))
                  ) : (
                    <div className="p-2 text-xs text-muted-foreground text-center">
                      {t('dashboard:noBackends')}
                    </div>
                  )}
                </SelectContent>
              </Select>

              {/* Thinking toggle */}
              {llmBackends?.find(b => b.is_active)?.capabilities.supports_thinking && (
                <button
                  onClick={() => {
                    const activeBackend = llmBackends?.find(b => b.is_active)
                    if (activeBackend) {
                      handleThinkingToggle(!activeBackend.thinking_enabled)
                    }
                  }}
                  disabled={isStreaming}
                  className={cn(
                    "h-7 w-7 rounded-md flex items-center justify-center transition-all shrink-0 hover:bg-muted/50",
                    llmBackends?.find(b => b.is_active)?.thinking_enabled
                      ? "text-blue-600 bg-blue-50 dark:bg-blue-950/30"
                      : "text-foreground/70 hover:text-foreground hover:bg-muted"
                  )}
                  title={llmBackends?.find(b => b.is_active)?.thinking_enabled ? t('dashboard:thinking') + ': ' + t('common:on') : t('dashboard:thinking') + ': ' + t('common:off')}
                >
                  <Brain className="h-4 w-4" />
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
      </div>
    </div>
  )
}
