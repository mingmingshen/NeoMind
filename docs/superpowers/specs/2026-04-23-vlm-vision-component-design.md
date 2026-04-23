# VLM Vision Component Design

## Overview

A new Dashboard business component that binds to image data sources (devices or extensions), automatically triggers VLM (Vision Language Model) inference on data updates, and displays results in a timeline chat interface. Users can also manually send follow-up messages. The component reuses existing backend APIs — no new backend endpoints required.

## Requirements

- **Data source binding**: Bind to any device/extension data source that provides image data (base64)
- **Automatic inference**: When image data updates, automatically send to VLM for analysis
- **Manual follow-up**: Users can type text follow-up questions in the input bar
- **System prompt**: User-configurable system prompt for VLM analysis
- **Model selection**: Select from available multimodal models (vision-capable only)
- **Queue mechanism**: Drop intermediate frames while inference is in progress — only the latest image is kept for next inference
- **Context window**: Sliding window retaining the last N image+response pairs as context (user-configurable, default 10)
- **Streaming**: AI responses stream in real-time via WebSocket

## Architecture

### Backend: No Changes

All required APIs already exist:

| Capability | Frontend API Method | Backend Endpoint | Notes |
|---|---|---|---|
| Create Agent | `api.createAgent()` | `POST /api/agents` | Body: `{ name, user_prompt, llm_backend_id }` |
| Create Session | `api.createSession()` | `POST /api/sessions` | Returns `{ sessionId }` |
| Send Image Message | `ws.sendMessage(text, images)` | `WS /api/chat` | WebSocket, `ChatRequest` with `images: [{data, mimeType}]` |
| Send Text Message | `ws.sendMessage(text)` | `WS /api/chat` | Same WebSocket, text-only |
| Stream Response | WebSocket `onmessage` | `WS /api/chat` | ServerMessage events: Content, Thinking, end |
| List Backends | `api.getLlmBackends()` | `GET /api/llm-backends` | Filter client-side by vision capability |
| Session History | `api.getSessionHistory(id)` | `GET /api/sessions/{id}/history` | For context window and message rendering |
| Delete Session | `api.deleteSession(id)` | `DELETE /api/sessions/{id}` | Cleanup on component removal |

**Integration flow:**
1. On component config save: `api.createAgent()` creates a VLM agent, `api.createSession()` creates a session
2. Establish WebSocket via `new ChatWebSocket(sessionId)` — existing `web/src/lib/websocket.ts`
3. On image data update: `ws.sendMessage('', [ChatImage])` where `ChatImage = { data: base64DataUrl }`
4. On user follow-up: `ws.sendMessage(text)`
5. Stream responses arrive via WebSocket `onmessage` callback as `ServerMessage` events
6. Context window: Backend session manages history automatically; no client-side trimming needed — session `getSessionHistory()` is for UI rendering only

### Frontend: New Component

**Component registration** in `registry.ts`:

```typescript
'vlm-vision': {
  type: 'vlm-vision',
  name: 'VLM Vision',
  description: 'Image analysis with Vision Language Model — auto-analyze data source images and display results in a timeline chat',
  category: 'business',
  icon: Camera,  // lucide-react
  sizeConstraints: getSizeConstraints('vlm-vision'),  // Follow existing pattern
  hasDataSource: true,
  hasDisplayConfig: false,
  hasActions: true,
  acceptsProp: (prop) => ['agentId', 'sessionId', 'className', 'editMode'].includes(prop),
  defaultProps: { agentId: undefined },
  variants: ['default'],
}
```

**Config persistence:** The VLM-specific config (systemPrompt, modelId, contextWindowSize, agentId, sessionId) is stored in `vlmVisionSlice` keyed by `componentId`. The dashboard component props only store `agentId` and `sessionId` (via `acceptsProp`). This matches how `AgentMonitorWidget` stores `agentId` in its props.

## File Structure

```
web/src/components/dashboard/generic/VlmVision.tsx       # Main component (registered entry)
web/src/components/dashboard/generic/vlm-vision/
  ├── VlmTimeline.tsx          # Timeline message list with ScrollArea
  ├── VlmMessageBubble.tsx     # Single message bubble (image/ai/user)
  ├── VlmConfigPanel.tsx       # Config dialog (system prompt, model, window size)
  ├── VlmInputBar.tsx          # Bottom input bar for follow-up messages
  ├── useVlmSession.ts         # Hook: agent/session lifecycle, message send/receive, streaming
  ├── useVlmQueue.ts           # Hook: drop-intermediate-frame queue strategy
  ├── useVlmModels.ts          # Hook: fetch vision-capable models from backends
  └── types.ts                 # Type definitions

web/src/store/slices/vlmVisionSlice.ts  # Zustand: per-componentId config state
web/src/components/dashboard/registry/registry.ts  # Add vlm-vision registration
```

## Component Design

### VlmVision (Main Component)

Follows `AgentMonitorWidget` layout pattern exactly:

```
┌──────────────────────────────────┐
│ Header                           │  shrink-0, px-4 py-3, border-b
│  [Camera icon] VLM Vision [Live] │  Avatar(40x40) + title + Badge
│  12 msgs · 1.2s avg · model     │  stats: MessageSquare, Clock, Cpu icons
├──────────────────────────────────┤
│                                  │
│  [Timeline Messages]             │  flex-1 min-h-0, ScrollArea
│  ┌─────────────────────────────┐ │
│  │ Camera | data src | time    │ │  Image data source entry
│  │ ┌───────────────────────┐   │ │  blue avatar (24x24, Camera icon)
│  │ │   image preview       │   │ │
│  │ └───────────────────────┘   │ │
│  └─────────────────────────────┘ │
│  ┌─────────────────────────────┐ │
│  │ Bot | model | time/stream  │ │  AI response
│  │ analysis text content       │ │  purple avatar (24x24, Bot icon)
│  └─────────────────────────────┘ │
│  ┌─────────────────────────────┐ │  (repeat for each pair)
│  │ ...                         │ │
│  └─────────────────────────────┘ │
│                                  │
├──────────────────────────────────┤
│ [Input bar] [Send]              │  p-3, border-t, bg-muted/20
└──────────────────────────────────┘
```

**States:**
- **Empty** (no data source configured): Camera icon (large, faded) + "Configure data source to start"
- **Loading**: Skeleton rows matching message structure
- **Active**: Timeline with messages
- **Streaming**: Last AI message shows streaming cursor + `Loader2` spin in header
- **Error**: Error banner with retry

### VlmMessageBubble

Three message types, all use 24x24 icon avatars:

| Type | Avatar | Avatar BG | Bubble Style |
|---|---|---|---|
| Image source | `Camera` icon (12px, blue) | `bg-blue-500/20` | `bg-muted/30 border border-border/50`, contains image preview + data source label |
| AI response | `Bot` icon (12px, purple) | `bg-purple-500/20` | `bg-purple-500/6 border-purple-500/15`, markdown content, streaming cursor |
| User follow-up | `User` icon (12px, purple-pink) | `bg-purple-500/20` | Right-aligned, `bg-purple-500/10 border-purple-500/20` |

Each bubble shows:
- Header line: icon + source label (left) + timestamp (right)
- Content: image preview / markdown text
- Footer: model name + duration for AI responses

### VlmConfigPanel

Opened via `Settings2` icon in header (same pattern as other business components). Uses `Dialog` component.

**Sections:**
1. **Data Source** — Bound via Dashboard's standard DataSource selector (component-level `hasDataSource: true`), displayed as read-only in config panel
2. **Model Selection** — Dropdown listing vision-capable models. Fetched via `useVlmModels` hook, filtered by `supports_multimodal` or vision capability detection
3. **System Prompt** — `Textarea` with default prompt pre-filled
4. **Context Window** — Slider (range 1-20, default 10) + numeric display

**On save:** Creates/updates Agent (`POST /api/agents`) with new config, creates new Session if needed.

### VlmInputBar

- `Textarea` (same as AgentMonitorWidget footer): `min-h-[36px] max-h-[72px] h-9 text-xs`
- Send `Button` with `Send` icon
- Enter to send (Shift+Enter for newline)
- Disabled when streaming (inference in progress)

## Hooks Design

### useVlmSession(componentId, config)

Manages the agent/session lifecycle for one component instance.

```typescript
interface UseVlmSessionReturn {
  messages: VlmMessage[]           // Timeline messages (sorted by time)
  isStreaming: boolean             // Currently receiving AI response
  streamingContent: string | null  // Partial streaming text
  sendMessage: (text: string, images?: string[]) => void
  initSession: () => Promise<void> // Create agent + session
  destroySession: () => void       // Cleanup
}
```

**Lifecycle:**
1. On mount or config change: if no `sessionId` in Zustand store for this `componentId`, call `initSession()`:
   - `api.createAgent({ name: 'vlm-vision-{componentId}', user_prompt: config.systemPrompt, llm_backend_id: config.modelId })`
   - `api.createSession()` → get `sessionId`
   - Store both in `vlmVisionSlice` keyed by `componentId`
   - Create `new ChatWebSocket(sessionId)` from `web/src/lib/websocket.ts`
2. On data source image update: `ws.sendMessage('', [{ data: base64DataUrl }])`
3. On user follow-up: `ws.sendMessage(text)`
4. Stream responses: WebSocket `onMessage` callback receives `ServerMessage` events:
   - `Content` event → append to streaming text
   - `end` event → finalize message, mark streaming complete
5. Persist `sessionId` in Zustand store so session survives component remounts
6. On unmount: keep WebSocket connected (or reconnect on next mount using same `sessionId`)

**Context window:**
- Backend session manager maintains full conversation history automatically
- `contextWindowSize` config controls how many messages to render in the timeline UI (visual truncation, not API-level)
- History loaded via `api.getSessionHistory(sessionId)` on mount for rendering existing messages

### useVlmQueue(onProcess, isProcessing)

Implements drop-intermediate-frame strategy.

```typescript
interface UseVlmQueueReturn {
  enqueue: (image: string) => void  // Add image to queue
  pending: number                    // Count of pending images (0 or 1)
  isProcessing: boolean             // Currently processing
}
```

**Logic:**
- `isProcessing = true` while VLM inference is running
- New image arrives → if `isProcessing`, replace pending image (only keep latest)
- When inference completes → if pending image exists, process it immediately
- Timeout: if `isProcessing` exceeds 120s (matching backend timeout), reset `isProcessing` to false and process pending image
- Exposes `pending` count (0 or 1) for UI queue status display

### useVlmModels()

Fetches and filters vision-capable models.

```typescript
interface UseVlmModelsReturn {
  models: VisionModel[]    // Models with vision capability
  loading: boolean
  error: string | null
}

interface VisionModel {
  id: string
  name: string
  backendId: string
  backendName: string
}
```

**Source:** `GET /api/llm-backends` → filter backends → for Ollama backends, `GET /api/llm-backends/ollama/models` → filter by vision capability.

## State Management

### vlmVisionSlice (Zustand)

Per-component instance config, keyed by `componentId`:

```typescript
interface VlmVisionState {
  configs: Record<string, VlmVisionConfig>
}

interface VlmVisionConfig {
  agentId?: string
  sessionId?: string
  modelId?: string
  modelName?: string
  systemPrompt: string
  contextWindowSize: number
}
```

Actions: `setConfig(componentId, partial)`, `clearConfig(componentId)`

Follows existing Zustand selector patterns — single values without `shallow`, objects/arrays with `shallow`.

## Data Flow

```
Device/Extension
       │
       ▼
  DataSource update (image base64)
       │
       ▼
  useDataSource hook → detects image data
       │  (uses isBase64Image/normalizeToDataUrl from AgentMonitorWidget pattern)
       ▼
  useVlmQueue.enqueue(image)
       │
       ▼ (when not processing)
  ws.sendMessage('', [ChatImage]) via ChatWebSocket
       │
       ▼
  WebSocket onMessage → ServerMessage.Content events
       │
       ▼
  VlmTimeline renders: [image bubble] → [streaming AI response bubble]
       │
       ▼
  ServerMessage "end" → finalize, mark streaming complete
       │
       ▼
  If queue has pending image → process immediately

--- Manual follow-up path ---

  User types in VlmInputBar → Enter
       │
       ▼
  ws.sendMessage(text) via ChatWebSocket
       │
       ▼
  Same streaming flow as above
```

**Image data detection:**
- Reuse `isBase64Image()` and `normalizeToDataUrl()` utilities (already in `AgentMonitorWidget.tsx`, should be extracted to shared utils)
- Accept: raw base64, `data:image/...;base64,...` data URLs
- Supported formats: png, jpeg, gif, webp, bmp (detected via magic bytes)
- Large images (>10MB base64) show a warning but still process

## Error Handling

- **Agent creation failure**: Show error state in component, retry button in header
- **Session creation failure**: Show error state, retry button
- **Model not available**: Warning badge in config panel, prompt to select another model
- **WebSocket disconnected**: `ChatWebSocket` has built-in reconnection with exponential backoff. Show connection status indicator in header (green dot = connected, red dot = disconnected)
- **Streaming error**: Show error message inline in timeline as a special error bubble, auto-retry up to 2 times
- **Data source not sending images**: Show informational empty state ("Waiting for image data from {dataSourceId}...")
- **Image too large**: Show warning inline, still attempt to process
- **Component unmount during streaming**: WebSocket message completes server-side; result available in session history on next mount

## WebSocket Integration

Uses the existing `ChatWebSocket` class from `web/src/lib/websocket.ts`:

```typescript
// In useVlmSession hook:
const ws = useRef<ChatWebSocket | null>(null)

// Initialize:
ws.current = new ChatWebSocket(sessionId)
ws.current.onMessage((msg: ServerMessage) => {
  switch (msg.type) {
    case 'Content': appendToStreamingContent(msg.content); break
    case 'Thinking': // optionally show thinking state; break
    case 'end': finalizeMessage(); break
    case 'Error': handleError(msg); break
  }
})

// Send image:
ws.current.sendMessage('', [{ data: base64DataUrl }], [])

// Send text:
ws.current.sendMessage(text, [], [])
```

The `ChatWebSocket` handles reconnection, heartbeat, and pending message queuing internally.

## Accessibility & UX

- Keyboard navigation: Enter to send, Escape to close config dialog
- Scroll to bottom on new message (with smart detection — don't scroll if user manually scrolled up)
- Image preview click → fullscreen dialog (same as AgentMonitorWidget's image viewer)
- Loading skeletons matching message structure
- i18n via `useTranslation` (follow existing patterns)
