# Global Chat Entry - Floating Button + Side Panel

## Overview

Add a global AI chat entry point: a floating action button (FAB) on non-chat pages that opens a compact side-sliding chat panel. The panel shares the same session as the main `/chat` page, so messages are continuous across both interfaces.

## Requirements

- **FAB visible only on non-`/chat` pages**: Hidden when user is already on any `/chat*` route (including `/chat/:sessionId`) or `/` (root renders ChatPage)
- **Shared session**: Panel and `/chat` page use the same Zustand session — messages sync automatically
- **Compact layout**: ~400px wide on desktop, full-screen on mobile
- **Core features only**: Basic chat, tool call visualization, stream progress. No model selector, skill selector, or session history drawer (those belong to the full `/chat` page)
- **Session lifecycle**: Panel closing only hides UI; session persists in store. App restart creates a new session (existing behavior)
- **Accessibility**: Icon-only FAB button must have `aria-label`. All user-visible text via `t()`

## Architecture

### Component Hierarchy

```
App.tsx (ProtectedRoute > div.flex.flex-col)
├── TopNav
├── Main Content (routes)
├── <Toaster />
├── <Confirmer />
└── <GlobalChatFab />                ← NEW: inserted after Confirmer, inside flex column
    └── Sheet (Radix Dialog)
        └── PanelChatView            ← simplified chat interface
            ├── MergedMessageList    (reused from existing)
            ├── StreamProgress       (reused from existing)
            └── ChatInputField       (reused from existing)
```

**Insertion point**: In `App.tsx`, line ~477, between `</main>` closing tag and `<Toaster />`:
```tsx
// Before:
                  </main>
                  <Toaster />
                  <Confirmer />

// After:
                  </main>
                  <Toaster />
                  <Confirmer />
                  <GlobalChatFab />
```

### New Files

1. **`web/src/components/chat/GlobalChatFab.tsx`** — FAB button + Sheet panel container. Manages open/close state, auto-closes on navigation to `/chat`.
2. **`web/src/components/chat/PanelChatView.tsx`** — Chat content for the panel. Handles WebSocket streaming, message rendering, and input. Reads from shared Zustand store.

### Modified Files

1. **`web/src/App.tsx`** — Add `<GlobalChatFab />` after `<Confirmer />` inside the ProtectedRoute flex column.
2. **`web/src/locales/en/chat.json`** — Add panel-specific translations.
3. **`web/src/locales/zh/chat.json`** — Add panel-specific translations.

## Visual Design

### Floating Action Button

| Property | Value |
|----------|-------|
| Position | `fixed bottom-6 right-6` with `pb-safe` for iOS safe area |
| Size | 56px circular (`w-14 h-14 rounded-full`) |
| Color | `bg-info text-primary-foreground` |
| Icon | `MessageSquare` (lucide-react) |
| Animation | Scale 0→1 on mount (CSS transition), hover scale 1.05 |
| Shadow | `shadow-lg` |
| Z-index | `z-40` (below Sheet overlay at z-50, avoids stacking ambiguity) |
| Accessibility | `aria-label={t('chat.openPanel')}` |

### Side Panel (Sheet)

| Property | Value |
|----------|-------|
| Width | Desktop: `className="w-[400px] sm:max-w-[400px]"` on SheetContent. Mobile: `w-screen` (full screen, via `useIsMobile()` at 768px breakpoint) |
| Slide direction | Right to left |
| Z-index | Override SheetContent to `z-[100]` via className (default Sheet is z-50, too low for overlays) |
| Overlay | Use existing Sheet overlay (`bg-bg-80 backdrop-blur-sm`) at z-50 — no change needed |
| Content background | `bg-surface-glass backdrop-blur-xl` on the SheetContent inner area (not the overlay) |
| Header | Compact: logo + title + close button (single row, ~48px height) |
| Content | Message list, max-w-full (no 3xl constraint), auto-scroll |
| Footer | Input + send button only (no model/skill selectors). Use `safe-bottom pb-6 sm:pb-4` for iOS safe area |
| Mobile dismiss | Close button always visible. Sheet supports Escape key and overlay click by default |

### Sheet Z-Index Strategy

The default `Sheet` component uses `z-50` for both overlay and content. For the chat panel:
- Overlay: keep default `z-50` (covers main content)
- SheetContent: override to `z-[100]` via `className="z-[100]"` to ensure it renders above all page-level elements
- If the panel is ever opened from within a FullScreenDialog (z-[100]), nested panels would need `z-[110]` — but this is out of scope for v1 since FAB is hidden behind FullScreenDialog

## Data Flow

### Session Sharing

The Zustand store's `sessionId` and `messages` are global. Both `ChatContainer` and `PanelChatView` read from the same store. The WebSocket singleton (`ws`) is shared.

```
PanelChatView                    ChatContainer (only on /chat page)
       │                                         │
       ├── ws.onMessage(handleMessage)           ├── ws.onMessage(handleMessage)
       ├── dispatch(stream actions)              ├── dispatch(stream actions)
       │   → local streamState                   │   → local streamState
       └── addMessage(final) → store             └── addMessage(final) → store
```

**No conflict guarantee**: FAB is hidden on all `/chat*` routes (via `location.pathname.startsWith('/chat')` or `=== '/'`), and panel auto-closes when navigating to `/chat`. `PanelChatView` and `ChatContainer` never coexist.

### Edge Case: Active Stream During Navigation

If the user navigates to `/chat` while streaming is in progress in the panel:

1. Panel auto-closes → `PanelChatView` unmounts → `ws.onMessage` unsubscribe fires
2. Stream accumulators (local to PanelChatView's `useEffect` closure) are destroyed
3. `ChatContainer` mounts → subscribes to `ws.onMessage`
4. **Mid-stream content from the old subscription is lost**

**Solution**: When panel has an active stream, delay navigation. Show a brief toast "Stream in progress..." and prevent auto-close until the stream completes. The `end` event triggers the close.

```tsx
// In GlobalChatFab
const [isStreaming, setIsStreaming] = useState(false)

useEffect(() => {
  if (isChatPage && open) {
    if (isStreaming) {
      // Show toast, delay close
      toast.info(t('chat.streamInProgress'))
      return
    }
    setOpen(false)
  }
}, [location.pathname, open, isStreaming])
```

### Session Initialization

```
Panel opens → check store.sessionId
  → exists: load session history if messages empty
  → null: call createSession() → ws.setSessionId()
```

### Message Sending

```typescript
const handleSend = (text: string) => {
  addMessage(userMessage)           // → global Zustand store
  dispatch({ type: 'START_STREAM' })
  ws.sendMessage(text)              // → shared WebSocket
}
```

### Panel Close Cleanup

- Unsubscribe from `ws.onMessage()` (via useEffect cleanup function)
- Session and messages remain in store (visible when user visits `/chat`)
- No session deletion on panel close

## Implementation Details

### GlobalChatFab

```tsx
// Pseudocode
function GlobalChatFab() {
  const [open, setOpen] = useState(false)
  const [isStreaming, setIsStreaming] = useState(false)
  const location = useLocation()

  // Chat page detection: /, /chat, /chat/:sessionId
  const isChatPage = location.pathname === '/' || location.pathname.startsWith('/chat')

  // Auto-close on /chat navigation (respect active stream)
  useEffect(() => {
    if (isChatPage && open) {
      if (isStreaming) {
        notifyInfo(t('chat.streamInProgress'))
        return
      }
      setOpen(false)
    }
  }, [isChatPage, open, isStreaming])

  // Hide FAB on chat pages
  if (isChatPage) return null

  return (
    <>
      {/* FAB - only when panel closed */}
      {!open && (
        <button
          onClick={() => setOpen(true)}
          aria-label={t('chat.openPanel')}
          className="fixed bottom-6 right-6 z-40 w-14 h-14 rounded-full
                     bg-info text-primary-foreground shadow-lg
                     flex items-center justify-center
                     hover:scale-105 transition-all duration-200
                     animate-in zoom-in-0 duration-300
                     pb-safe"
        >
          <MessageSquare className="h-6 w-6" />
        </button>
      )}

      {/* Side panel */}
      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent
          className="z-[100] w-screen sm:w-[400px] sm:max-w-[400px] p-0 flex flex-col"
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

### PanelChatView

- Reuses `useReducer(streamReducer, initialStreamState)` from ChatContainer pattern
- Subscribes to `ws.onMessage()` for streaming events
- Renders `MergedMessageList` with filtered messages from store
- Renders `StreamProgress` during streaming
- Renders `ChatInputField` for input
- Header: compact logo + title + close button
- Calls `onStreamingChange(true/false)` to sync streaming state with parent

### Streaming State

PanelChatView maintains its own local `streamState` via `useReducer`, identical in structure to ChatContainer's. This handles:
- `Thinking` / `Content` / `ToolCallStart` / `ToolCallEnd` accumulation
- `Progress` / `Plan` / `ExecutionPlan` events
- `end` → assemble final message → `addMessage()` → clear stream state

### Route Detection

```typescript
const isChatPage = location.pathname === '/' || location.pathname.startsWith('/chat')
```

This correctly handles `/`, `/chat`, and `/chat/:sessionId`.

## i18n Keys

| Key | English | Chinese |
|-----|---------|---------|
| `chat.openPanel` | "Open AI Chat" | "打开 AI 对话" |
| `chat.closePanel` | "Close" | "关闭" |
| `chat.panelTitle` | "AI Chat" | "AI 对话" |
| `chat.streamInProgress` | "Stream in progress, closing after completion..." | "正在响应中，完成后自动关闭..." |

## Scope Exclusions

These features are available in the full `/chat` page only:
- LLM model selector
- Skill selector
- Session history drawer
- Input suggestions (`/` commands)
- Panel inside FullScreenDialogs (out of scope for v1)

## Files Changed Summary

| File | Action | Description |
|------|--------|-------------|
| `components/chat/GlobalChatFab.tsx` | Create | FAB + Sheet container |
| `components/chat/PanelChatView.tsx` | Create | Simplified chat view |
| `App.tsx` | Modify | Add GlobalChatFab after Confirmer in ProtectedRoute |
| `locales/en/chat.json` | Modify | Add 4 panel i18n keys |
| `locales/zh/chat.json` | Modify | Add 4 panel i18n keys |
