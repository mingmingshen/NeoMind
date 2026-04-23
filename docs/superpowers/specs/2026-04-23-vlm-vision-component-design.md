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

| Capability | Endpoint | Notes |
|---|---|---|
| Create Agent | `POST /api/agents` | System prompt + model selection |
| Create Session | `POST /api/sessions` | Independent conversation per component instance |
| Send Image Message | `POST /api/sessions/{id}/chat` | `images` field in `ChatRequest` |
| Stream Response | `GET /api/chat` (WebSocket) | Real-time streaming |
| List Vision Models | `GET /api/llm-backends` | Filter by `supports_multimodal` |
| Session History | `GET /api/sessions/{id}/history` | For context window |

**Integration flow:**
1. On component config save: `POST /api/agents` creates a VLM agent (with user's system prompt + model), `POST /api/sessions` creates a session
2. On image data update: `POST /api/sessions/{id}/chat` with `{ message, images: [base64], selected_skills: [] }`
3. On user follow-up: Same endpoint, text-only message
4. Context window managed client-side: fetch history via `GET /api/sessions/{id}/history`, trim to N pairs before sending

### Frontend: New Component

**Component registration** in `registry.ts`:

```typescript
'vlm-vision': {
  type: 'vlm-vision',
  name: 'VLM Vision',
  description: 'Image analysis with Vision Language Model — auto-analyze data source images and display results in a timeline chat',
  category: 'business',
  icon: Camera,  // lucide-react
  sizeConstraints: { minW: 2, minH: 2, defaultW: 3, defaultH: 3, maxW: 6, maxH: 6 },
  hasDataSource: true,
  hasDisplayConfig: false,
  hasActions: true,
  acceptsProp: (prop) => ['systemPrompt', 'modelId', 'contextWindowSize', 'agentId', 'sessionId', 'className', 'editMode'].includes(prop),
  defaultProps: {
    systemPrompt: 'You are a professional image analysis assistant. Carefully observe the image content, describe the scene, and point out any notable changes or anomalies.',
    contextWindowSize: 10,
  },
  variants: ['default'],
}
```

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
1. On mount or config change: if no `agentId`/`sessionId` in component props, call `initSession()` → `POST /api/agents` + `POST /api/sessions`
2. Store `agentId` and `sessionId` in component state (persisted via dashboard layout props)
3. On data source image update: `sendMessage('', [base64Image])`
4. On user follow-up: `sendMessage(text)`
5. Streaming via WebSocket `GET /api/chat` with session context
6. On unmount: no explicit destroy — agent/session persist for resume

**Context window implementation:**
- Before sending, fetch `GET /api/sessions/{id}/history`
- Take last N message pairs (image + AI response)
- Backend handles context assembly via session management

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
- Exposes `pending` count for UI status display

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
       │
       ▼
  useVlmQueue.enqueue(image)
       │
       ▼ (when not processing)
  useVlmSession.sendMessage('', [image])
       │
       ├──► POST /api/sessions/{id}/chat { message, images }
       │
       ▼
  WebSocket stream response
       │
       ▼
  VlmTimeline renders new messages
       │
       ▼
  User sees: [image] → [streaming AI analysis]

--- Manual follow-up path ---

  User types in VlmInputBar
       │
       ▼
  useVlmSession.sendMessage(text)
       │
       ▼
  POST /api/sessions/{id}/chat { message }
       │
       ▼
  WebSocket stream response
```

## Error Handling

- **Agent creation failure**: Show error state, retry button
- **Session creation failure**: Show error state, retry button
- **Model not available**: Warning badge, prompt to reconfigure
- **Streaming error**: Show error message inline in timeline, allow retry
- **Data source not sending images**: Show informational empty state ("Waiting for image data...")
- **Network disconnected**: Show connection lost indicator, auto-reconnect via existing WebSocket mechanism

## Accessibility & UX

- Keyboard navigation: Enter to send, Escape to close config dialog
- Scroll to bottom on new message (with smart detection — don't scroll if user manually scrolled up)
- Image preview click → fullscreen dialog (same as AgentMonitorWidget's image viewer)
- Loading skeletons matching message structure
- i18n via `useTranslation` (follow existing patterns)
