/**
 * PanelChatView - Simplified chat interface for the global side panel
 *
 * Shares session with ChatContainer via Zustand store.
 * Handles WebSocket streaming, message rendering, and input.
 * No model selector, skill selector, or session history.
 */

import { useState, useRef, useEffect, useCallback, useReducer, useMemo } from "react"
import { useTranslation } from "react-i18next"
import { useStore } from "@/store"
import { generateId } from "@/lib/id"
import { ws } from "@/lib/websocket"
import type { ServerMessage, ExecutionPlan } from "@/types"
import type { StreamProgress as StreamProgressType } from "@/types"
import { filterPartialMessages } from "@/lib/messageUtils"
import {
  selectSessionId,
  selectMessages,
  selectChatActions,
} from "@/store/selectors"
import { MergedMessageList } from "./MergedMessageList"
import { StreamProgress } from "./StreamProgress"
import { ChatInputField } from "./ChatContainer"
import { X, Minimize2, Bot } from "lucide-react"
import { Button } from "@/components/ui/button"

interface PanelChatViewProps {
  onClose: () => void
  onStreamingChange: (streaming: boolean) => void
  showMinimize?: boolean
}

// Stream state - same structure as ChatContainer
interface StreamState {
  isStreaming: boolean
  streamingContent: string
  streamingThinking: string
  streamingToolCalls: any[]
  streamProgress: StreamProgressType
  currentPlanStep: string
  executionPlan: ExecutionPlan | null
  planStepStates: Record<number, 'pending' | 'running' | 'completed' | 'failed'>
  roundContents: Record<number, string>
  currentRound: number
}

type StreamAction =
  | { type: 'START_STREAM' }
  | { type: 'THINKING'; content: string }
  | { type: 'CONTENT'; content: string }
  | { type: 'TOOL_START'; tool: string; arguments?: any; round?: number }
  | { type: 'TOOL_END'; tool: string; result: any }
  | { type: 'PROGRESS'; progress: Partial<StreamProgressType> }
  | { type: 'PLAN'; step: string }
  | { type: 'WARNING'; message: string }
  | { type: 'EXECUTION_PLAN'; plan: ExecutionPlan }
  | { type: 'PLAN_STEP_STARTED'; stepId: number; description: string }
  | { type: 'PLAN_STEP_COMPLETED'; stepId: number; success: boolean; summary: string }
  | { type: 'ROUND_END' }
  | { type: 'END_STREAM' }
  | { type: 'ERROR' }
  | { type: 'RESET' }

const initialStreamState: StreamState = {
  isStreaming: false,
  streamingContent: "",
  streamingThinking: "",
  streamingToolCalls: [],
  streamProgress: {
    elapsed: 0,
    stage: 'thinking',
    warnings: [],
    remainingTime: 300,
  },
  currentPlanStep: "",
  executionPlan: null,
  planStepStates: {},
  roundContents: {},
  currentRound: 1,
}

function streamReducer(state: StreamState, action: StreamAction): StreamState {
  switch (action.type) {
    case 'START_STREAM':
      return { ...state, isStreaming: true }
    case 'THINKING':
      return {
        ...state,
        isStreaming: true,
        streamingThinking: state.streamingThinking + action.content,
        streamProgress: { ...state.streamProgress, stage: 'thinking' },
      }
    case 'CONTENT':
      return {
        ...state,
        isStreaming: true,
        streamingContent: state.streamingContent + action.content,
        streamProgress: { ...state.streamProgress, stage: 'generating' },
      }
    case 'TOOL_START':
      return {
        ...state,
        isStreaming: true,
        streamingToolCalls: [
          ...state.streamingToolCalls,
          { id: generateId(), name: action.tool, arguments: action.arguments, result: null, round: action.round },
        ],
        streamProgress: { ...state.streamProgress, stage: 'tool_execution' },
      }
    case 'TOOL_END': {
      const idx = state.streamingToolCalls.findIndex(
        tc => tc.name === action.tool && tc.result === null
      )
      if (idx === -1) return state
      const updated = [...state.streamingToolCalls]
      updated[idx] = { ...updated[idx], result: action.result }
      return { ...state, streamingToolCalls: updated }
    }
    case 'PROGRESS':
      return {
        ...state,
        streamProgress: {
          ...state.streamProgress,
          ...action.progress,
          warnings: action.progress.warnings ?? state.streamProgress.warnings,
        },
      }
    case 'PLAN':
      return { ...state, currentPlanStep: action.step }
    case 'WARNING':
      return {
        ...state,
        streamProgress: {
          ...state.streamProgress,
          warnings: [...state.streamProgress.warnings, action.message],
        },
      }
    case 'ROUND_END':
      return {
        ...state,
        roundContents: {
          ...state.roundContents,
          [state.currentRound]: state.streamingContent,
        },
        streamingContent: "",
        streamingThinking: "",
        currentRound: state.currentRound + 1,
      }
    case 'EXECUTION_PLAN':
      return { ...state, executionPlan: action.plan, planStepStates: {} }
    case 'PLAN_STEP_STARTED':
      return { ...state, planStepStates: { ...state.planStepStates, [action.stepId]: 'running' } }
    case 'PLAN_STEP_COMPLETED':
      return { ...state, planStepStates: { ...state.planStepStates, [action.stepId]: action.success ? 'completed' : 'failed' } }
    case 'END_STREAM':
      return { ...initialStreamState, isStreaming: false }
    case 'ERROR':
      return { ...initialStreamState, isStreaming: false }
    case 'RESET':
      return initialStreamState
    default:
      return state
  }
}

export function PanelChatView({ onClose, onStreamingChange, showMinimize }: PanelChatViewProps) {
  const { t } = useTranslation("chat")

  // Store state
  const sessionId = useStore(selectSessionId)
  const messages = useStore(selectMessages)
  const { addMessage, createSession } = useStore(selectChatActions)

  // Streaming state
  const [streamState, dispatch] = useReducer(streamReducer, initialStreamState)
  const [currentStreamMessageId, setCurrentStreamMessageId] = useState<string | null>(null)

  // Refs
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const scrollContainerRef = useRef<HTMLDivElement>(null)
  const inputFieldRef = useRef<{ setText: (text: string) => void; focus: () => void }>(null)
  const isStreamingRef = useRef(false)
  const onStreamingChangeRef = useRef(onStreamingChange)
  useEffect(() => { onStreamingChangeRef.current = onStreamingChange }, [onStreamingChange])

  // Sync streaming state to parent
  useEffect(() => {
    isStreamingRef.current = streamState.isStreaming
    onStreamingChangeRef.current(streamState.isStreaming)
  }, [streamState.isStreaming])

  // Auto-scroll
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({
      behavior: streamState.isStreaming ? "smooth" : "instant",
    })
  }, [messages, streamState.streamingContent, streamState.isStreaming])

  // Initialize session if needed
  useEffect(() => {
    if (!sessionId) {
      createSession()
    }
  }, [sessionId, createSession])

  // Handle WebSocket events
  useEffect(() => {
    let streamingContentAcc = ""
    let streamingThinkingAcc = ""
    let streamingToolCallsAcc: any[] = []
    let roundContentsAcc: Record<number, string> = {}
    let currentRound = 1

    const handleMessage = (data: ServerMessage) => {
      switch (data.type) {
        case "Thinking":
          streamingThinkingAcc += (data.content || "")
          dispatch({ type: 'THINKING', content: data.content || "" })
          break
        case "Content":
          streamingContentAcc += (data.content || "")
          dispatch({ type: 'CONTENT', content: data.content || "" })
          break
        case "ToolCallStart":
          dispatch({ type: 'TOOL_START', tool: data.tool, arguments: data.arguments, round: data.round ?? currentRound })
          streamingToolCallsAcc.push({
            id: generateId(), name: data.tool, arguments: data.arguments, result: null, round: data.round ?? currentRound,
          })
          break
        case "ToolCallEnd": {
          const idx = streamingToolCallsAcc.findIndex(tc => tc.name === data.tool && tc.result === null)
          if (idx !== -1) {
            streamingToolCallsAcc[idx] = { ...streamingToolCallsAcc[idx], result: data.result }
          }
          dispatch({ type: 'TOOL_END', tool: data.tool, result: data.result })
          break
        }
        case "IntermediateEnd":
        case "intermediate_end":
          if (streamingContentAcc) roundContentsAcc[currentRound] = streamingContentAcc
          streamingContentAcc = ""
          streamingThinkingAcc = ""
          currentRound += 1
          dispatch({ type: 'ROUND_END' })
          break
        case "Progress":
          dispatch({ type: 'PROGRESS', progress: { elapsed: data.elapsed, stage: data.stage, remainingTime: data.remainingTime ?? 300 } })
          if (data.message) dispatch({ type: 'PLAN', step: data.message })
          break
        case "Plan":
          dispatch({ type: 'PLAN', step: data.step })
          break
        case "ExecutionPlanCreated":
          dispatch({ type: 'EXECUTION_PLAN', plan: data.plan })
          break
        case "PlanStepStarted":
          dispatch({ type: 'PLAN_STEP_STARTED', stepId: data.stepId, description: data.description })
          break
        case "PlanStepCompleted":
          dispatch({ type: 'PLAN_STEP_COMPLETED', stepId: data.stepId, success: data.success ?? true, summary: data.summary ?? '' })
          break
        case "Warning":
          dispatch({ type: 'WARNING', message: data.message })
          break
        case "end":
          if (streamingContentAcc || streamingThinkingAcc || streamingToolCallsAcc.length > 0) {
            if (streamingContentAcc) roundContentsAcc[currentRound] = streamingContentAcc
            const hasMultipleRounds = Object.keys(roundContentsAcc).length > 1
            addMessage({
              id: generateId(),
              role: "assistant",
              content: streamingContentAcc,
              timestamp: Math.floor(Date.now() / 1000),
              thinking: streamingThinkingAcc || undefined,
              tool_calls: streamingToolCallsAcc.length > 0 ? streamingToolCallsAcc : undefined,
              round_contents: hasMultipleRounds ? roundContentsAcc : undefined,
            })
          }
          dispatch({ type: 'END_STREAM' })
          setCurrentStreamMessageId(null)
          streamingContentAcc = ""
          streamingThinkingAcc = ""
          streamingToolCallsAcc = []
          roundContentsAcc = {}
          currentRound = 1
          break
        case "Error":
          addMessage({
            id: generateId(),
            role: "assistant",
            content: `**${t("errors.llmError")}**\n\n${data.message}`,
            timestamp: Math.floor(Date.now() / 1000),
          })
          dispatch({ type: 'ERROR' })
          break
      }
    }

    const unsubscribe = ws.onMessage(handleMessage)
    return () => { void unsubscribe() }
  }, [addMessage, t])

  // Send message
  const handleSend = useCallback((text: string) => {
    if (!text || streamState.isStreaming) return

    addMessage({
      id: generateId(),
      role: "user",
      content: text,
      timestamp: Math.floor(Date.now() / 1000),
    })

    dispatch({ type: 'START_STREAM' })
    setCurrentStreamMessageId(generateId())
    ws.sendMessage(text)
  }, [streamState.isStreaming, addMessage])

  const filteredMessages = useMemo(() => filterPartialMessages(messages), [messages])

  return (
    <div className="flex flex-col h-full bg-background">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-3.5 border-b border-border/60 flex-shrink-0">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-info/15 flex items-center justify-center">
            <Bot className="h-4.5 w-4.5 text-info" />
          </div>
          <div>
            <span className="text-sm font-semibold leading-tight">{t("panelTitle")}</span>
            {isStreamingRef.current && (
              <span className="ml-2 inline-flex items-center gap-1 text-xs text-muted-foreground">
                <span className="w-1.5 h-1.5 rounded-full bg-info animate-pulse" />
              </span>
            )}
          </div>
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={onClose}
          className="h-8 w-8 rounded-lg text-muted-foreground hover:text-foreground"
          aria-label={t("closePanel")}
        >
          {showMinimize ? <Minimize2 className="h-4 w-4" /> : <X className="h-4 w-4" />}
        </Button>
      </div>

      {/* Messages */}
      <div
        ref={scrollContainerRef}
        className="flex-1 overflow-y-auto px-4 py-5 min-h-0"
      >
        <div className="space-y-4">
          <MergedMessageList
            messages={filteredMessages}
            scrollElementRef={scrollContainerRef}
            isStreaming={streamState.isStreaming && !(currentStreamMessageId && filteredMessages.some(m => m.id === currentStreamMessageId))}
            streamingContent={streamState.streamingContent}
            streamingThinking={streamState.streamingThinking}
            streamingToolCalls={streamState.streamingToolCalls}
            executionPlan={streamState.executionPlan}
            planStepStates={streamState.planStepStates}
            roundContents={streamState.roundContents}
          />

          {streamState.isStreaming && (
            <StreamProgress
              elapsed={streamState.streamProgress.elapsed}
              totalDuration={300}
              stage={streamState.streamProgress.stage}
              warning={streamState.streamProgress.warnings[streamState.streamProgress.warnings.length - 1]}
              currentStep={streamState.currentPlanStep}
            />
          )}

          <div ref={messagesEndRef} />
        </div>
      </div>

      {/* Input area */}
      <div className="border-t border-border/60 px-4 py-4 safe-bottom flex-shrink-0">
        <ChatInputField
          ref={inputFieldRef}
          isStreaming={streamState.isStreaming}
          onSend={handleSend}
          onSlash={() => {}}
          onEscape={onClose}
          showSuggestions={false}
        />
      </div>
    </div>
  )
}
