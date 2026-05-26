# Global Chat Entry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a floating AI chat button and side-sliding panel that lets users chat with the AI from any non-chat page, sharing the same session as the main `/chat` page.

**Architecture:** New `GlobalChatFab` component (FAB + Sheet container) and `PanelChatView` (simplified chat view) that read from the shared Zustand store and WebSocket singleton. No modifications to existing `ChatContainer`. The panel reuses existing child components (`MergedMessageList`, `StreamProgress`, `ChatInputField`).

**Tech Stack:** React 18, TypeScript, Radix UI Sheet (Dialog), Zustand store, WebSocket singleton, Tailwind CSS, lucide-react icons, i18next

**Spec:** `docs/superpowers/specs/2026-05-26-global-chat-entry-design.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `web/src/components/chat/ChatContainer.tsx` | Modify | Export `ChatInputField` (add `export` keyword) |
| `web/src/components/chat/GlobalChatFab.tsx` | Create | FAB button + Sheet panel container, open/close state, route detection, streaming-aware auto-close |
| `web/src/components/chat/PanelChatView.tsx` | Create | Chat content for the panel: streaming logic, message rendering, input, compact header |
| `web/src/App.tsx` | Modify | Add `<GlobalChatFab />` import and render after `<Confirmer />` inside ProtectedRoute |
| `web/src/i18n/locales/en/chat.json` | Modify | Add 4 panel i18n keys |
| `web/src/i18n/locales/zh/chat.json` | Modify | Add 4 panel i18n keys |

---

### Task 0: Export ChatInputField from ChatContainer

**Files:**
- Modify: `web/src/components/chat/ChatContainer.tsx` (line ~244)

`ChatInputField` is currently a non-exported `const`. PanelChatView needs to import it.

- [ ] **Step 1: Add export keyword**

In `web/src/components/chat/ChatContainer.tsx`, find line ~244:
```tsx
const ChatInputField = memo(forwardRef<ChatInputFieldHandle, {
```

Change to:
```tsx
export const ChatInputField = memo(forwardRef<ChatInputFieldHandle, {
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit --pretty 2>&1 | head -20`

Expected: No new errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/components/chat/ChatContainer.tsx
git commit -m "refactor(chat): export ChatInputField for reuse in side panel"
```

---

### Task 1: Add i18n keys for the chat panel

**Files:**
- Modify: `web/src/i18n/locales/en/chat.json`
- Modify: `web/src/i18n/locales/zh/chat.json`

- [ ] **Step 1: Add English translations**

Add these keys at the top level of the JSON object in `web/src/i18n/locales/en/chat.json`:

```json
{
  "openPanel": "Open AI Chat",
  "closePanel": "Close",
  "panelTitle": "AI Chat",
  "streamInProgress": "Stream in progress, closing after completion...",
  ...
}
```

- [ ] **Step 2: Add Chinese translations**

Add these keys at the top level of the JSON object in `web/src/i18n/locales/zh/chat.json`:

```json
{
  "openPanel": "打开 AI 对话",
  "closePanel": "关闭",
  "panelTitle": "AI 对话",
  "streamInProgress": "正在响应中，完成后自动关闭...",
  ...
}
```

- [ ] **Step 3: Verify JSON is valid**

Run: `cd web && node -e "JSON.parse(require('fs').readFileSync('src/i18n/locales/en/chat.json','utf8'));JSON.parse(require('fs').readFileSync('src/i18n/locales/zh/chat.json','utf8'));console.log('OK')"`

Expected: `OK`

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/locales/en/chat.json web/src/i18n/locales/zh/chat.json
git commit -m "feat(i18n): add global chat panel translation keys"
```

---

### Task 2: Create PanelChatView component

**Files:**
- Create: `web/src/components/chat/PanelChatView.tsx`

This is the largest task. The component reuses existing child components but needs its own WebSocket subscription and streaming reducer (same pattern as ChatContainer).

- [ ] **Step 1: Create PanelChatView with streaming logic**

Create `web/src/components/chat/PanelChatView.tsx`:

```tsx
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
import type { Message, ServerMessage, ExecutionPlan } from "@/types"
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
import { X } from "lucide-react"
import { Button } from "@/components/ui/button"

interface PanelChatViewProps {
  onClose: () => void
  onStreamingChange: (streaming: boolean) => void
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

export function PanelChatView({ onClose, onStreamingChange }: PanelChatViewProps) {
  const { t } = useTranslation("chat")

  // Store state
  const sessionId = useStore(selectSessionId)
  const messages = useStore(selectMessages)
  const { addMessage, createSession } = useStore(selectChatActions)

  // Streaming state
  const [streamState, dispatch] = useReducer(streamReducer, initialStreamState)

  // Refs
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const scrollContainerRef = useRef<HTMLDivElement>(null)
  const inputFieldRef = useRef<{ setText: (text: string) => void; focus: () => void }>(null)
  const isStreamingRef = useRef(false)
  const onStreamingChangeRef = useRef(onStreamingChange)
  const [currentStreamMessageId, setCurrentStreamMessageId] = useState<string | null>(null)
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
    <div className="flex flex-col h-full">
      {/* Compact header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border flex-shrink-0">
        <div className="flex items-center gap-2">
          <img src="/logo-square.png" alt="NeoMind" width={24} height={24} className="w-6 h-6 rounded-md" />
          <span className="text-sm font-semibold">{t("panelTitle")}</span>
        </div>
        <Button
          variant="ghost"
          size="icon"
          onClick={onClose}
          className="h-8 w-8 rounded-lg"
          aria-label={t("closePanel")}
        >
          <X className="h-4 w-4" />
        </Button>
      </div>

      {/* Messages */}
      <div
        ref={scrollContainerRef}
        className="flex-1 overflow-y-auto px-3 py-4 min-h-0"
      >
        <div className="space-y-4">
          {filteredMessages.length === 0 && !streamState.isStreaming && (
            <div className="text-center py-12">
              <img src="/logo-square.png" alt="NeoMind" width={48} height={48} className="w-12 h-12 rounded-xl mx-auto mb-4" />
              <p className="text-sm text-muted-foreground">{t("welcome.description")}</p>
            </div>
          )}

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
      <div className="border-t border-border bg-background backdrop-blur-xl px-3 py-3 pb-6 safe-bottom flex-shrink-0">
        <ChatInputField
          ref={inputFieldRef}
          isStreaming={streamState.isStreaming}
          onSend={handleSend}
          onSlash={() => {}} // No suggestions in panel
          onEscape={onClose}
          showSuggestions={false}
        />
      </div>
    </div>
  )
}
```

Note: `ChatInputField` is imported from `ChatContainer.tsx` where it was exported in Task 0. It only needs `isStreaming`, `onSend`, `onSlash`, `onEscape`, and `showSuggestions`.

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit --pretty 2>&1 | head -30`

Expected: No errors related to `PanelChatView.tsx`. There may be pre-existing errors in other files.

- [ ] **Step 3: Commit**

```bash
git add web/src/components/chat/PanelChatView.tsx
git commit -m "feat(chat): add PanelChatView for global side panel"
```

---

### Task 3: Create GlobalChatFab component

**Files:**
- Create: `web/src/components/chat/GlobalChatFab.tsx`

- [ ] **Step 1: Create GlobalChatFab**

Create `web/src/components/chat/GlobalChatFab.tsx`:

```tsx
/**
 * GlobalChatFab - Floating action button + side panel for quick AI chat access
 *
 * Shows a FAB on all non-chat pages. Clicking opens a Sheet with PanelChatView.
 * Auto-closes when navigating to /chat. Respects active streaming state.
 */

import { useState, useEffect } from "react"
import { useLocation } from "react-router-dom"
import { useTranslation } from "react-i18next"
import { MessageSquare } from "lucide-react"
import { Sheet, SheetContent } from "@/components/ui/sheet"
import { PanelChatView } from "./PanelChatView"
import { notifyInfo } from "@/lib/notify"
import { cn } from "@/lib/utils"

export function GlobalChatFab() {
  const [open, setOpen] = useState(false)
  const [isStreaming, setIsStreaming] = useState(false)
  const location = useLocation()
  const { t } = useTranslation("chat")

  // Detect chat pages: /, /chat, /chat/:sessionId
  const isChatPage = location.pathname === "/" || location.pathname.startsWith("/chat")

  // Auto-close on /chat navigation (delay if streaming)
  useEffect(() => {
    if (isChatPage && open) {
      if (isStreaming) {
        notifyInfo(t("streamInProgress"))
        return
      }
      setOpen(false)
    }
  }, [isChatPage, open, isStreaming, t])

  // Hide FAB entirely on chat pages
  if (isChatPage) return null

  return (
    <>
      {/* Floating action button */}
      {!open && (
        <button
          onClick={() => setOpen(true)}
          aria-label={t("openPanel")}
          className={cn(
            "fixed bottom-6 right-6 z-40",
            "w-14 h-14 rounded-full",
            "bg-info text-primary-foreground",
            "shadow-lg hover:shadow-xl",
            "flex items-center justify-center",
            "hover:scale-105 transition-all duration-200",
            "animate-in zoom-in-0 fade-in-0 duration-300",
            "safe-bottom"
          )}
        >
          <MessageSquare className="h-6 w-6" />
        </button>
      )}

      {/* Side panel */}
      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent
          className={cn(
            "z-[100]",
            "bg-surface-glass backdrop-blur-xl",
            "w-screen md:w-[400px] md:max-w-[400px]",
            "p-0 gap-0",
            "flex flex-col h-full"
          )}
        >
          <PanelChatView
            onClose={() => setOpen(false)}
            onStreamingChange={setIsStreaming}
          />
        </SheetContent>
      </Sheet>
    </>
  )
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit --pretty 2>&1 | head -30`

Expected: No errors related to `GlobalChatFab.tsx`.

- [ ] **Step 3: Commit**

```bash
git add web/src/components/chat/GlobalChatFab.tsx
git commit -m "feat(chat): add GlobalChatFab floating button and side panel"
```

---

### Task 4: Wire GlobalChatFab into App.tsx

**Files:**
- Modify: `web/src/App.tsx`

- [ ] **Step 1: Add import and render GlobalChatFab**

In `web/src/App.tsx`:

Add import after the existing lazy imports (around line 13):
```tsx
import { GlobalChatFab } from '@/components/chat/GlobalChatFab'
```

Add `<GlobalChatFab />` after `<Confirmer />` inside the ProtectedRoute layout. Find the section (around line 476-478):
```tsx
                  <Toaster />
                  <Confirmer />
```

Change to:
```tsx
                  <Toaster />
                  <Confirmer />
                  <GlobalChatFab />
```

- [ ] **Step 2: Verify the app compiles and renders**

Run: `cd web && npx tsc --noEmit --pretty 2>&1 | head -30`

Expected: No new TypeScript errors.

- [ ] **Step 3: Manual test**

1. Run `cd web && npm run dev`
2. Log in and navigate to `/devices`
3. Verify: FAB button appears at bottom-right
4. Click FAB → side panel slides in from right
5. Type a message and send → verify streaming response appears
6. Close panel → FAB reappears
7. Navigate to `/chat` (click Dashboard in nav) → FAB hidden, messages from panel visible

- [ ] **Step 4: Commit**

```bash
git add web/src/App.tsx
git commit -m "feat(chat): wire GlobalChatFab into app layout"
```

---

### Task 5: Visual polish and mobile verification

**Files:**
- Possibly modify: `web/src/components/chat/GlobalChatFab.tsx`
- Possibly modify: `web/src/components/chat/PanelChatView.tsx`

- [ ] **Step 1: Test on mobile viewport**

1. Open browser dev tools → toggle mobile viewport (iPhone 14 size)
2. Navigate to a non-chat page
3. Verify FAB position accounts for safe-area
4. Open panel → verify it fills the full screen
5. Verify input area has safe bottom padding

- [ ] **Step 2: Test streaming edge case**

1. Open panel on `/devices`
2. Send a message that triggers a long response
3. While streaming, click "Dashboard" in nav
4. Verify: toast appears saying stream in progress
5. Wait for stream to complete → panel closes
6. Navigate to `/chat` → verify the completed message appears

- [ ] **Step 3: Fix any visual issues found**

Adjust CSS classes as needed based on testing. Common fixes:
- Safe-area padding on mobile (`pb-safe` on FAB, `safe-bottom pb-6` on input area)
- Panel width on different screen sizes
- Z-index stacking with other overlays

- [ ] **Step 4: Commit any fixes**

```bash
git add -u
git commit -m "fix(chat): polish global chat panel visual and mobile layout"
```

---

### Task 6: Final verification

- [ ] **Step 1: Run full type check**

Run: `cd web && npx tsc --noEmit`

Expected: No new errors (pre-existing errors in other files are OK).

- [ ] **Step 2: Run build**

Run: `cd web && npm run build`

Expected: Build succeeds.

- [ ] **Step 3: Verify FAB hidden on all chat routes**

Check these routes show NO FAB:
- `/` (root → ChatPage)
- `/chat` (ChatPage)
- `/chat/abc123` (session route)

Check these routes DO show FAB:
- `/devices`
- `/visual-dashboard`
- `/agents`
- `/automation`
- `/data`
- `/messages`
- `/extensions`
- `/settings`
