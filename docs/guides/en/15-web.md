# Web Frontend Module

**Version**: 0.8.0
**Completion**: 90%
**Purpose**: React + TypeScript Web Application / Tauri Desktop Application

## Overview

The Web module is NeoMind's frontend application, supporting both Web browser and Tauri desktop application (dual mode). Built with React 18 + TypeScript + Vite, using Zustand for state management, Radix UI + Tailwind CSS for the user interface.

## Important Changes (v0.7.x – v0.8.x)

### Design System Overhaul (v0.7)

- **OKLCH Color System** — CSS color tokens migrated from HSL to OKLCH for perceptually uniform colors across light and dark modes
- **Design Tokens** — All hardcoded Tailwind palette colors replaced with semantic design tokens (`text-success`, `bg-error-light`, `text-accent-orange`, etc.)
- **Aurora Background & Glass Morphism** — App-wide aurora gradient background with glass-style navigation
- **Typography** — Plus Jakarta Sans (Latin) + Noto Sans SC (CJK) fonts
- **UnifiedFormDialog** — Centralized dialog component for consistent form dialogs
- **FullScreenDialog** — Full-screen builder dialog with sidebar layout for complex editors

### Data Push & Notification Channels (v0.8)

- **Data Explorer page** (`/data`) — Unified page with Data Explorer and Push Targets tabs
- **PushTargetsTab** — Manage data push targets with create/edit/test workflows
- **DeliveryHistoryPanel** — View delivery logs with pagination and status filtering
- **ChannelEditorDialog** — FullScreenDialog for creating/editing notification channels (5 channel types: webhook, email, telegram, dingtalk, wecom)
- **GlobalChatFab** — Floating action button for quick access to AI chat from any page

### LLM Backends Management (v0.8)

- **UnifiedLLMBackendsTab** — Unified tab in Settings page for managing LLM backend instances
- **LLMBackendConfigDialog** — Dialog for configuring backend adapters (Ollama, OpenAI-compatible, etc.)

### Plugin → Extension Migration (v0.5)

- **`/plugins` routes migrated to `/extensions`** (redirect preserved)
- `plugins.tsx` → `extensions.tsx`
- Extension components unified in `components/extensions/` directory
- API calls changed from `api.listPlugins()` to `api.listExtensions()`

## Module Structure

```
web/src/
├── main.tsx                    # App entry
├── App.tsx                     # Routing and authentication
├── components/                 # UI components
│   ├── ui/                    # Base UI components (Radix UI wrappers)
│   │   ├── button.tsx
│   │   ├── input.tsx
│   │   ├── card.tsx
│   │   ├── switch.tsx
│   │   ├── tabs.tsx
│   │   ├── toast.tsx
│   │   ├── dialog.tsx
│   │   ├── dropdown-menu.tsx
│   │   ├── select.tsx
│   │   ├── slider.tsx
│   │   ├── checkbox.tsx
│   │   ├── popover.tsx
│   │   ├── tooltip.tsx
│   │   ├── scroll-area.tsx
│   │   ├── sheet.tsx
│   │   ├── accordion.tsx
│   │   ├── code-editor.tsx
│   │   └── ...
│   ├── layout/                # Layout components
│   │   └── TopNav.tsx
│   ├── chat/                  # Chat-related components
│   │   ├── MarkdownMessage.tsx
│   │   ├── ThinkingBlock.tsx
│   │   ├── StreamProgress.tsx
│   │   ├── ToolCallVisualization.tsx
│   │   ├── GlobalChatFab.tsx  # [v0.8] Floating AI chat button
│   │   └── ...
│   ├── dashboard/             # Dashboard components
│   │   ├── DashboardGrid.tsx
│   │   ├── registry/
│   │   └── config/
│   ├── automation/            # Automation components
│   │   ├── rule/
│   │   ├── dialog/
│   │   │   └── FullScreenDialog.tsx  # [v0.7] Full-screen builder dialog
│   │   └── TransformTestDialog.tsx
│   ├── devices/               # Device-related components
│   │   ├── DeviceList.tsx
│   │   ├── DeviceDetail.tsx
│   │   └── ...
│   ├── datapush/              # [v0.8] Data push components
│   │   ├── PushTargetsTab.tsx
│   │   ├── PushTargetDialog.tsx
│   │   └── DeliveryHistoryPanel.tsx
│   ├── llm/                   # [v0.8] LLM backend management
│   │   ├── UnifiedLLMBackendsTab.tsx
│   │   ├── LLMBackendConfigDialog.tsx
│   │   └── index.ts
│   ├── messages/              # Message/Notification components
│   │   ├── ChannelEditorDialog.tsx  # [v0.8] FullScreenDialog channel editor
│   │   ├── MessageChannelsTab.tsx
│   │   ├── MessagesTab.tsx
│   │   └── CreateMessageDialog.tsx
│   ├── session/               # Session management components
│   ├── extensions/            # Extension components (unified Extension management)
│   ├── dialog/                # Shared dialog components
│   │   └── UnifiedFormDialog.tsx    # [v0.7] Centralized form dialog
│   ├── shared/                # Shared components
│   │   ├── FullScreenEditor.tsx
│   │   ├── ResponsiveTable.tsx      # [v0.7] Responsive table with mobile card view
│   │   ├── PaginatedContent.tsx     # [v0.7] Pagination with infinite scroll
│   │   ├── SearchBar.tsx
│   │   ├── ErrorBoundary.tsx
│   │   └── ...
│   └── design-system/         # Design system
├── pages/                     # Page components
│   ├── login.tsx              # Login page
│   ├── setup.tsx              # Initial setup
│   ├── chat.tsx               # Chat page
│   ├── devices.tsx            # Devices page
│   ├── automation.tsx         # Automation page
│   ├── agents.tsx             # AI Agent page
│   ├── settings.tsx           # Settings page (with LLM backends tab)
│   ├── data-explorer.tsx      # [v0.8] Data explorer + push targets
│   ├── messages.tsx           # Messages page
│   └── extensions.tsx         # Extensions page
├── store/                     # Zustand state management
│   ├── index.ts               # Store entry
│   ├── slices/                # State slices
│   │   ├── authSlice.ts       # Auth state
│   │   ├── sessionSlice.ts    # Session state
│   │   ├── uiSlice.ts         # UI state
│   │   ├── deviceSlice.ts     # Device state
│   │   ├── alertSlice.ts      # Message/Alert state
│   │   ├── settingsSlice.ts   # Settings state
│   │   ├── extensionSlice.ts  # Extension state
│   │   ├── llmBackendSlice.ts # LLM backend state
│   │   ├── dashboardSlice.ts  # Dashboard state
│   │   ├── dataPushSlice.ts   # [v0.8] Data push state
│   │   ├── instanceSlice.ts   # [v0.7] Instance state
│   │   ├── frontendComponentSlice.ts # [v0.7] Frontend component registry
│   │   ├── aiAnalystSlice.ts  # AI analyst state
│   │   └── updateSlice.ts     # Update state
│   ├── selectors/             # Selectors
│   └── persistence/           # Persistence
├── lib/                       # Utility libraries
│   ├── api.ts                 # API client
│   ├── websocket.ts           # WebSocket chat
│   ├── events.ts              # Event stream (SSE/WS)
│   ├── auth/                  # Auth utilities
│   ├── notify.ts              # Notification system
│   ├── errors.ts              # Error handling utilities
│   ├── portal.ts              # [v0.7] Portal root for modals/popovers
│   └── utils/                 # Helper functions
├── hooks/                     # React Hooks
│   ├── useDialog.ts
│   ├── useForm.ts             # [v0.7] Custom form hook with validation
│   ├── use-toast.ts
│   ├── use-confirm.ts
│   ├── useErrorHandler.ts
│   └── useMobile.ts           # Mobile detection
├── types/                     # TypeScript types
│   └── index.ts               # Unified type definitions
├── design-system/             # Design system
│   ├── components/
│   ├── icons/                 # Entity type icon mappings
│   ├── tokens/                # Design tokens
│   │   ├── color.ts           # OKLCH color tokens
│   │   ├── typography.ts
│   │   ├── size.ts
│   │   └── indicator.ts
│   └── utils/                 # Format utilities (status colors, etc.)
└── i18n/                      # Internationalization
```

## Tech Stack

### Core Framework

```json
{
  "react": "^18.3.1",
  "react-dom": "^18.3.1",
  "typescript": "^5.3.3",
  "vite": "^5.1.0"
}
```

### State Management

```json
{
  "zustand": "^4.5.0"
}
```

### UI Component Library

```json
{
  "@radix-ui/react-accordion": "^1.2.12",
  "@radix-ui/react-alert-dialog": "^1.1.15",
  "@radix-ui/react-checkbox": "^1.3.3",
  "@radix-ui/react-dialog": "^1.0.5",
  "@radix-ui/react-dropdown-menu": "^2.0.6",
  "@radix-ui/react-label": "^2.0.2",
  "@radix-ui/react-popover": "^1.1.15",
  "@radix-ui/react-progress": "^1.1.8",
  "@radix-ui/react-scroll-area": "^1.0.5",
  "@radix-ui/react-select": "^2.0.0",
  "@radix-ui/react-separator": "^1.1.8",
  "@radix-ui/react-slider": "^1.3.6",
  "@radix-ui/react-switch": "^1.0.3",
  "@radix-ui/react-tabs": "^1.0.4",
  "@radix-ui/react-toast": "^1.1.5",
  "@radix-ui/react-tooltip": "^1.2.8"
}
```

### Other Dependencies

```json
{
  "react-router-dom": "^6.22.0",
  "react-hook-form": "^7.71.0",
  "@hookform/resolvers": "^5.2.2",
  "recharts": "^3.7.0",
  "react-grid-layout": "^2.2.2",
  "react-markdown": "^10.1.0",
  "remark-gfm": "^4.0.1",
  "lucide-react": "^0.344.0",
  "tailwindcss": "^3.4.1",
  "class-variance-authority": "^0.7.0",
  "clsx": "^2.1.0",
  "tailwind-merge": "^2.3.0",
  "zod": "^4.3.5",
  "i18next": "^25.7.4",
  "react-i18next": "^16.5.2",
  "@tauri-apps/api": "^2.0.0"
}
```

## Route Structure

```typescript
// Public routes
/login                    // Login page (checks if setup needed)
/setup                    // Initial setup

// Shared routes (no auth required)
/share/:token             // Shared dashboard (proxy access)

// Protected routes (require auth)
/                         // Default redirects to chat page
/chat                    // AI chat page
/chat/:sessionId         // Specific session chat
/visual-dashboard        // Visual dashboard
/visual-dashboard/:id    // Specific dashboard
/data                    // [v0.8] Data explorer (telemetry data + push targets)
/devices                 // Device management page
/devices/:id             // Device details page
/devices/types           // Device types page
/devices/drafts          // Draft devices page (auto-onboarding)
/automation              // Automation page
/automation/transforms   // Data transform page
/agents                  // AI Agents page
/agents/memory           // Agent memory configuration
/agents/skills           // Agent skill management
/settings                // Settings page (includes LLM backends tab)
/messages                // Messages page
/messages/channels       // Message channels page
/extensions              // Extension management page (unified Extension system)
/plugins                 // Redirects to /extensions (legacy)
```

## Dashboard Components

Dashboard components support two modes for specifying data: **manual input** (e.g., URL, text) and **data source binding** (linking to device metrics, extensions, etc.). This is handled by the `DualModeSourceField` component.

### Component Data Source Types

Each component specifies which data source types it accepts via `allowedTypes`:

| Component | Allowed Types | Description |
|-----------|---------------|-------------|
| `video-display` | `device`, `device-info`, `device-metric` | Video streams — bind to device metrics that provide video URLs (e.g., RTSP/HLS addresses) |
| `image-display` | `device-metric`, `system`, `extension`, `transform`, `ai-metric` | Images — bind to metrics or extensions providing image data |
| `gauge-chart` | `device-metric`, `system`, `extension`, `transform`, `ai-metric` | Gauge values — bind to numeric metrics |
| `stat-card` | `device-metric`, `system`, `extension`, `transform`, `ai-metric` | Stat values — bind to any numeric metric |

### DataSourceId Format

```
{type}:{id}:{field}
```

Example: `extension:weather:temp` — the `temp` field from the `weather` extension.

## State Management (Zustand)

### Store Structure

```typescript
export type NeoMindStore = AuthSlice
  & SessionSlice
  & UISlice
  & AlertSlice
  & DeviceSlice
  & SettingsSlice
  & ExtensionSlice
  & LlmBackendSlice
  & DashboardState
  & DataPushSlice
  & InstanceSlice
  & FrontendComponentSlice
```

### AuthSlice - Auth State

```typescript
interface AuthSlice {
  // State
  token: string | null
  user: UserInfo | null
  isAuthenticated: boolean

  // Actions
  setToken: (token: string, remember?: boolean) => void
  clearToken: () => void
  setUser: (user: UserInfo) => void
  checkAuthStatus: () => Promise<void>
}
```

### SessionSlice - Session State

```typescript
interface SessionSlice {
  // State
  messages: Message[]
  sessionId: string
  sessions: ChatSession[]
  isStreaming: boolean

  // Actions
  setMessages: (messages: Message[]) => void
  addMessage: (message: Message) => void
  updateLastMessage: (content: string, isPartial?: boolean) => void
  setSessionId: (id: string) => void
  setSessions: (sessions: ChatSession[]) => void
}
```

### DeviceSlice - Device State

```typescript
interface DeviceSlice {
  // State
  devices: Device[]
  deviceTypes: DeviceType[]
  loading: boolean

  // Actions
  setDevices: (devices: Device[]) => void
  setDeviceTypes: (types: DeviceType[]) => void
  updateDevice: (id: string, updates: Partial<Device>) => void
  fetchDevices: () => Promise<void>
}
```

## API Client

### fetchAPI - Unified Request Function

```typescript
async function fetchAPI<T>(
  path: string,
  options: FetchOptions = {}
): Promise<T>

interface FetchOptions extends RequestInit {
  skipAuth?: boolean          // Skip JWT auth
  skipGlobalError?: boolean    // Skip global error handling
  skipErrorToast?: boolean     // Skip error toast
  successMessage?: string      // Success toast message
}
```

### API Endpoint Groups

```typescript
export const api = {
  // Auth
  login: (username: string, password: string) => Promise<LoginResponse>
  logout: () => Promise<{ message: string }>
  getCurrentUser: () => Promise<UserInfo>

  // Devices
  getDevices: () => Promise<{ devices: Device[]; count: number }>
  getDevice: (id: string) => Promise<Device>
  addDevice: (req: AddDeviceRequest) => Promise<{ device_id: string }>
  deleteDevice: (id: string) => Promise<{ deleted: boolean }>
  sendCommand: (deviceId: string, command: string, params: Record<string, unknown>) => Promise<...>

  // Automation
  listAutomations: (params?: AutomationFilter) => Promise<AutomationListResponse>
  createAutomation: (req: CreateAutomationRequest) => Promise<...>
  analyzeAutomationIntent: (description: string) => Promise<IntentResult>

  // Agents
  listAgents: () => Promise<AgentListResponse>
  getAgent: (id: string) => Promise<AgentDetail>
  executeAgent: (id: string, req?: ExecuteAgentRequest) => Promise<...>

  // ... more endpoints
}
```

## WebSocket Chat

### ChatWebSocket Class

```typescript
export class ChatWebSocket {
  // Connection
  connect(initialSessionId?: string): void
  disconnect(): void
  manualReconnect(): void
  isConnected(): boolean

  // Message sending
  sendMessage(content: string, images?: ChatImage[]): void
  sendRequest(request: ClientChatMessage): void

  // Session management
  setSessionId(sessionId: string): void
  getSessionId(): string | null
  setActiveBackend(backendId: string | null): void

  // Event listeners
  onMessage(handler: (message: ServerMessage) => void): () => void
  onConnection(handler: (connected: boolean, isReconnect?: boolean) => void): () => void
  onStateChange(handler: (state: ConnectionState) => void): () => void

  // State
  getState(): ConnectionState
}

// Singleton
export const ws = new ChatWebSocket()
```

### Server Message Types

```typescript
type ServerMessage =
  | { type: 'Thinking'; content: string; sessionId: string }
  | { type: 'Content'; content: string; sessionId: string }
  | { type: 'ToolCallStart'; tool: string; arguments: Record<string, unknown>; sessionId: string }
  | { type: 'ToolCallEnd'; tool: string; result: string; sessionId: string; success?: boolean }
  | { type: 'Progress'; elapsed: number; stage?: string; message?: string; sessionId: string }
  | { type: 'Intent'; category: string; displayName: string; confidence?: number; sessionId: string }
  | { type: 'Warning'; message: string; elapsed?: number; remainingTime?: number; sessionId: string }
  | { type: 'Error'; message: string; sessionId: string }
  | { type: 'end'; sessionId: string }
  | { type: 'ping'; timestamp: number }
```

## Event Stream (SSE/WebSocket)

### EventsWebSocket Class

```typescript
export class EventsWebSocket {
  // Connection
  connect(): void
  disconnect(): void
  isConnected(): boolean

  // Subscribe
  onEvent(handler: (event: NeoMindEvent) => void): () => void
  on<T extends EventType>(eventType: T, handler: (event: Extract<NeoMindEvent, { type: T }>) => void): () => void
  onDeviceEvent(handler: (event: DeviceMetricEvent) => void): () => void
  onRuleEvent(handler: (event: RuleTriggeredEvent) => void): () => void
  onConnection(handler: (connected: boolean) => void): () => void

  // Config
  updateConfig(config: Partial<EventsConfig>): void
}

// Get connection
export function getEventsConnection(key?: string, config?: EventsConfig): EventsWebSocket
```

### Event Types

```typescript
export type EventType =
  | 'DeviceOnline' | 'DeviceOffline' | 'DeviceMetric' | 'DeviceCommandResult'
  | 'RuleEvaluated' | 'RuleTriggered'
  | 'AgentExecutionStarted' | 'AgentThinking' | 'AgentDecision' | 'AgentProgress' | 'AgentExecutionCompleted'
  | 'AlertCreated' | 'AlertAcknowledged'
  | 'LlmDecisionProposed' | 'LlmDecisionExecuted'
  | 'ToolExecutionStart' | 'ToolExecutionSuccess' | 'ToolExecutionFailure'
  | 'Custom'

export interface NeoMindEvent {
  id: string
  type: EventType
  timestamp: number
  source: string
  data: unknown
}
```

## UI Components

### Base Components

Radix UI wrapper components using `class-variance-authority` for variants:

```typescript
// Button example
const buttonVariants = cva(
  "inline-flex items-center justify-center rounded-md text-sm font-medium",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        destructive: "bg-destructive text-destructive-foreground hover:bg-destructive/90",
        outline: "border border-input bg-background hover:bg-accent",
        ghost: "hover:bg-accent hover:text-accent-foreground",
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-10 px-4 py-2",
        sm: "h-9 rounded-md px-3",
        lg: "h-11 rounded-md px-8",
        icon: "h-10 w-10",
      },
    },
  }
)
```

### Shared Components

| Component | Description |
|-----------|-------------|
| `StatsCard` | Statistics card |
| `BulkActionBar` | Bulk action bar |
| `Pagination` | Pagination component (default: 10 items per page) |
| `PaginatedContent` | [v0.7] Pagination with infinite scroll for mobile |
| `ResponsiveTable` | [v0.7] Responsive table with mobile card view and built-in skeleton loading |
| `EmptyState` | Empty state indicator |
| `LoadingState` | Loading state (skeleton for page-level, spinner for inline) |
| `StatusBadge` | Status badge |
| `ActionBar` | Action bar |
| `MonitorStatsGrid` | Monitoring stats grid |
| `ErrorBoundary` | React Error Boundary wrapper |
| `SearchBar` | Search input with debounce |
| `FullScreenEditor` | Full-screen editor wrapper |
| `KeepAlive` | Component keep-alive wrapper |

### Chat Components

| Component | Description |
|-----------|-------------|
| `MarkdownMessage` | Markdown message rendering |
| `ThinkingBlock` | Thinking process display |
| `StreamProgress` | Streaming output progress |
| `ToolCallVisualization` | Tool call visualization |
| `SessionSidebar` | Session sidebar |
| `ConnectionStatus` | Connection status indicator |

## Tauri Desktop Application

### Tauri Environment Detection

```typescript
const isTauri = !!(window as any).__TAURI__

// Tauri uses local backend
const API_BASE = isTauri ? 'http://localhost:9375/api' : '/api'
const wsHost = isTauri ? 'localhost:9375' : window.location.host
```

### Startup Loading

```typescript
// Tauri app waits for backend readiness
if (isTauri && !backendReady) {
  return <StartupLoading onReady={() => setBackendReady(true)} />
}
```

## Development Commands

```bash
# Development mode
npm run dev

# Build
npm run build

# Preview
npm run preview

# Tauri development
npm run tauri:dev

# Tauri build
npm run tauri:build

# Testing (Playwright)
npm run test
npm run test:ui
npm run test:headed

# Linting
npm run lint
```

## Build Configuration

### Vite Config

```typescript
// vite.config.ts
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:9375',
        changeOrigin: true,
      },
    },
  },
  build: {
    target: 'esnext',
    minify: 'terser',
  },
})
```

### Tailwind Config

```javascript
// tailwind.config.js
module.exports = {
  darkMode: ['class'],
  content: ['./src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        border: 'var(--border)',
        input: 'var(--input)',
        ring: 'var(--ring)',
        background: 'var(--background)',
        foreground: 'var(--foreground)',
        primary: {
          DEFAULT: 'var(--primary)',
          foreground: 'var(--primary-foreground)',
        },
        // ... semantic design tokens using OKLCH CSS variables
      },
      borderRadius: {
        lg: 'var(--radius)',
        md: 'calc(var(--radius) - 2px)',
        sm: 'calc(var(--radius) - 4px)',
      },
    },
  },
  plugins: [require('@tailwindcss/typography')],
}
```

## Design System

### Color System (OKLCH)

All CSS color tokens use OKLCH for perceptually uniform colors:

```css
:root {
  --background: oklch(1 0 0 / 97%);
  --foreground: oklch(0.18 0.02 270);
  --card: oklch(1 0 0 / 85%);
  --card-foreground: oklch(0.18 0.02 270);
  --popover: oklch(1 0 0);
  --popover-foreground: oklch(0.18 0.02 270);
  --primary: oklch(0.18 0.02 270);
  --primary-foreground: oklch(1 0 0);
  --secondary: oklch(0.96 0.003 270);
  --secondary-foreground: oklch(0.18 0.02 270);
  --muted: oklch(0.96 0.003 270);
  --muted-foreground: oklch(0.45 0.01 270);
  --accent: oklch(0.96 0.003 270);
  --accent-foreground: oklch(0.18 0.02 270);
  --destructive: oklch(0.577 0.245 27);
  --destructive-foreground: oklch(1 0 0);
  --border: oklch(0.88 0.01 270);
  --input: oklch(0.88 0.01 270);
  --ring: oklch(0.18 0.02 270);
  --radius: 0.75rem;
}

.dark {
  --background: oklch(0.14 0.02 270 / 97%);
  --foreground: oklch(0.93 0.01 270);
  --card: oklch(0.17 0.02 270 / 85%);
  --card-foreground: oklch(0.93 0.01 270);
  /* ... dark mode tokens follow same OKLCH pattern */
}
```

Design tokens defined in `web/src/design-system/tokens/color.ts`:

```typescript
// OKLCH chart colors — vibrant, high-contrast palette for data visualization
export const chartColors = {
  1: 'oklch(0.62 0.22 270)',   // Indigo-Blue
  2: 'oklch(0.65 0.20 155)',   // Emerald
  3: 'oklch(0.72 0.17 65)',    // Amber
  4: 'oklch(0.67 0.20 25)',    // Orange
  5: 'oklch(0.65 0.18 340)',   // Rose
  6: 'oklch(0.68 0.12 210)',   // Sky Blue
} as const

// Status colors — tuned for semantic meaning
export const statusColors = {
  success: 'oklch(0.65 0.20 155)',   // Emerald green
  warning: 'oklch(0.72 0.17 65)',    // Amber
  error: 'oklch(0.58 0.22 25)',      // Deep red-orange
  info: 'oklch(0.62 0.22 270)',      // Indigo-blue
  neutral: 'oklch(0.55 0.02 260)',   // Cool gray
} as const
```

**Important**: Only use design token classes (`text-success`, `bg-error-light`, `text-accent-orange`, etc.). NEVER use hardcoded Tailwind palette colors (`bg-blue-500`, `text-green-600`). CSS variable-based colors do NOT support Tailwind `/` opacity modifier — use pre-defined tokens (`bg-muted-30`, `bg-success-light`) instead.

### Icon System

Using `lucide-react` icon library, unified style:

```typescript
import {
  Bot,   // AI/Agent
  Cpu,   // Device
  Zap,   // Automation
  Bell,  // Messages/Notifications
  Settings,  // Settings
  // ...
} from 'lucide-react'
```

## Internationalization

Using `i18next` + `react-i18next`:

```typescript
import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: { /* ... */ } },
      zh: { translation: { /* ... */ } },
    },
    fallbackLng: 'en',
    interpolation: { escapeValue: false },
  })
```

## Performance Optimization

### Code Splitting

```typescript
// Route lazy loading
const ChatPage = lazy(() => import('@/pages/chat').then(m => ({ default: m.ChatPage })))
const DevicesPage = lazy(() => import('@/pages/devices').then(m => ({ default: m.DevicesPage })))
```

### Virtual Lists

```typescript
// Large lists use virtual scrolling
import { VirtualList } from '@/components/ui/VirtualList'

<VirtualList
  items={items}
  itemHeight={50}
  renderItem={(item) => <ListItem item={item} />}
/>
```

### Debounce/Throttle

```typescript
import { useDebounce, useThrottle } from '@/hooks/useDebounce'

const debouncedSearch = useDebounce(searchTerm, 300)
const throttledSave = useThrottle(saveData, 1000)
```

## Design Principles

1. **Component-Based**: Small, focused components with single responsibility
2. **Type Safety**: Complete TypeScript type definitions
3. **Responsive**: Mobile and desktop adaptation
4. **Accessibility**: Follow WCAG standards, use ARIA
5. **Performance First**: Code splitting, lazy loading, virtual scrolling

## Data Push Module (v0.8)

The Data Push module provides configurable push targets that deliver device telemetry data to external services on a schedule.

### Components

| Component | Description |
|-----------|-------------|
| `PushTargetsTab` | Main tab showing list of push targets with CRUD operations |
| `PushTargetDialog` | Dialog for creating/editing push target configuration |
| `DeliveryHistoryPanel` | View delivery logs with pagination and status filtering |

### State Management (DataPushSlice)

```typescript
interface DataPushSlice {
  pushTargets: PushTarget[]
  pushTargetsLoading: boolean
  pushStats: PushStats | null
  deliveryLogs: DeliveryLog[]
  deliveryLogsTotal: number

  fetchPushTargets: () => Promise<void>
  createPushTarget: (data: CreatePushTargetRequest) => Promise<boolean>
  updatePushTarget: (id: string, data: UpdatePushTargetRequest) => Promise<boolean>
  deletePushTarget: (id: string) => Promise<boolean>
  startPushTarget: (id: string) => Promise<boolean>
  stopPushTarget: (id: string) => Promise<boolean>
  testPushTarget: (id: string) => Promise<DeliveryLog | null>
  fetchDeliveryLogs: (targetId: string, limit?: number, offset?: number) => Promise<void>
}
```

## Notification Channels (v0.8)

The notification channel system supports 5 channel types, each managed via the `ChannelEditorDialog` (a FullScreenDialog).

### Supported Channel Types

| Type | Description |
|------|-------------|
| `webhook` | HTTP webhook (POST to external URL) |
| `email` | Email via SMTP configuration |
| `telegram` | Telegram Bot API |
| `dingtalk` | DingTalk robot webhook |
| `wecom` | WeCom (WeChat Work) robot webhook |

### ChannelEditorDialog

Uses FullScreenDialog with sidebar layout:

```typescript
<FullScreenDialog open={open} onOpenChange={onOpenChange}>
  <FullScreenDialogHeader />
  <FullScreenDialogContent>
    <FullScreenDialogSidebar>  {/* Channel type selector */}
    <FullScreenDialogMain>     {/* Channel configuration form */}
    <FullScreenDialogFooter>   {/* Actions */}
  </FullScreenDialogContent>
</FullScreenDialog>
```

## FetchCache Pattern (v0.7)

Store-level fetch deduplication with TTL-based caching (default 10 seconds) prevents redundant API calls:

```typescript
// Pattern: shouldFetch -> markFetching -> API call -> markFetched
// Invalidate on mutations

// In slice:
fetchCache: Record<string, { timestamp: number; fetching: boolean }>

shouldFetch(key: string): boolean     // TTL expired and not currently fetching
markFetching(key: string): void       // Set fetching flag
markFetched(key: string): void        // Update timestamp
invalidateCache(key?: string): void   // Clear cache (key or all)
```

Used in: `deviceSlice`, `extensionSlice`, `sessionSlice`, `llmBackendSlice`, `instanceSlice`, `frontendComponentSlice`.

## Global Chat FAB (v0.8)

The `GlobalChatFab` component renders a floating action button on all non-chat pages that expands into a full-screen chat overlay with a smooth scale-up animation from the FAB position.

### Architecture

- **File**: `web/src/components/chat/GlobalChatFab.tsx`
- **Rendered in**: `App.tsx` inside the protected route layout, alongside `TopNav`
- **Panel state machine**: `closed` -> `opening` -> `open` -> `closing` -> `closed`

### Behavior

| Feature | Description |
|---------|-------------|
| FAB visibility | Hidden on chat pages (`/`, `/chat`, `/chat/:sessionId`), visible on all other pages |
| Panel session | Persistent across FAB close/reopen and page refreshes via `localStorage` key `neomind:panelSessionId` |
| Auto-close | When user navigates to a chat page, the panel auto-closes (delayed if streaming is in progress) |
| Streaming guard | If a response is streaming, the user is notified and the panel stays open |
| Minimize button | Collapses back to FAB with reverse animation |

### Visual Design

- FAB: fixed `bottom-6 right-6`, 56px round button with glass morphism and orange glow ring
- Overlay: `z-[90]` backdrop blur, click-to-close (disabled during streaming)
- Panel: `z-[100]` full-screen on mobile, inset-4/inset-8 on larger screens, glass morphism background

### Key Components

```typescript
// GlobalChatFab renders:
<GlobalChatFab>
  <button />              // FAB (hidden when open)
  <div />                 // Backdrop overlay
  <div>                   // Full-screen panel
    <PanelChatView        // Reuses chat view logic
      onClose={handleClose}
      onStreamingChange={setIsStreaming}
      ensureSession={ensurePanelSession}
      showMinimize
    />
  </div>
</GlobalChatFab>
```

## Dashboard Sharing (v0.7)

Dashboard sharing allows generating public links that anyone can view without authentication. The system supports multiple share links per dashboard with configurable permissions and expiration.

### Components

| Component | File | Description |
|-----------|------|-------------|
| `ShareManagerDialog` | `components/dashboard/ShareManagerDialog.tsx` | FullScreenDialog for managing share links (list, create, delete) |
| `SharedDashboard` | `pages/share/SharedDashboard.tsx` | Public-facing shared dashboard viewer (no auth required) |

### API Endpoints

```typescript
// Share management (authenticated)
GET    /api/dashboards/:id/share       // List share tokens
POST   /api/dashboards/:id/share       // Create share link
DELETE /api/dashboards/:id/share/:token // Revoke share link

// Public access (no auth)
GET    /api/share/:token               // Get shared dashboard data
GET    /api/share/:token/proxy/*       // Proxy for widget data fetching
```

### Share Token Structure

```typescript
interface ShareToken {
  token: string
  permissions: { allow_interactive: boolean }
  created_at: number
  expires_at: number | null      // null = never expires
  share_url: string
}
```

### Share Link Creation Options

- **Permissions**: Read-only (default) or Interactive (allows widget interactions)
- **Expiration**: Optional, configurable in hours (1h, 6h, 24h, 7d, 30d, never)

### Public Viewer Architecture

The `SharedDashboard` page (`/share/:token`) is unique because it operates without authentication:

1. **Fetch proxy**: All `/api/...` requests are rewritten to `/api/share/:token/proxy/...` via a global `window.fetch` interceptor (`installShareProxy()`)
2. **Data conversion**: Backend returns snake_case fields; `fromDashboardDTO()` converts to camelCase
3. **Component registries**: Community and extension component registries are loaded with `skipAuth: true`
4. **Real-time updates**: No WebSocket available; device data is polled every 30 seconds instead
5. **Error boundary**: Each component is wrapped in `ComponentErrorBoundary` for graceful degradation of unsupported components

## Setup Wizard (v0.7)

The Setup Wizard is the first-launch experience for new NeoMind installations. It creates the admin account and guides the user through initial configuration.

### Flow

```
App.tsx checks /api/setup/status
  -> setup_required: true  -> redirect to /setup
  -> setup_required: false -> normal login flow
```

### Steps

| Step | Component | Description |
|------|-----------|-------------|
| 1. Account | `AccountStep` | Create admin account (username, password, optional email) + timezone selection (auto-detected) |
| 2. Complete | `CompleteStep` | Success screen with quick-start guide (Chat, LLM Setup, Explore) |

### SetupRoute Guard

```typescript
// Only accessible when setup is required (no users exist yet)
// If setup is already completed, redirects to /login
// In Tauri: retries up to 15 times with backoff while waiting for backend
```

### API Endpoints

```typescript
GET  /api/setup/status     // { setup_required: boolean }
POST /api/setup/complete   // Mark setup as done (requires JWT from account creation)
```

### Design Notes

- LLM configuration is intentionally deferred -- users configure it when they first use AI features
- Optional Mailchimp newsletter subscription via JSONP (AccountStep)
- Setup background uses the same aurora gradient as the main app

## Complete Store Slices Reference

All Zustand store slices are composed into a single `NeoMindStore` type. Below is the complete reference with descriptions.

| Slice | File | Description |
|-------|------|-------------|
| `AuthSlice` | `authSlice.ts` | User authentication with JWT tokens, API key support, login/register/logout, current user info |
| `SessionSlice` | `sessionSlice.ts` | Chat session management -- create, switch, delete sessions; load message history; infinite scroll for session list |
| `UISlice` | `uiSlice.ts` | UI state -- sidebar open/close toggle, WebSocket connection status |
| `DeviceSlice` | `deviceSlice.ts` | Device management -- CRUD, device types, telemetry, batch current-value updates, BLE provisioning, MDL generation |
| `AlertSlice` | `alertSlice.ts` | Alert/notification state -- fetch alerts, acknowledge, create (uses unified messages API internally) |
| `SettingsSlice` | `settingsSlice.ts` | System settings -- config import/export/validation, settings dialog state |
| `ExtensionSlice` | `extensionSlice.ts` | Extension management -- list/install/uninstall extensions, commands, data sources, logs, query execution |
| `LlmBackendSlice` | `llmBackendSlice.ts` | LLM backend instances -- CRUD, activation switching, connection testing, backend type definitions |
| `DashboardSlice` | `dashboardSlice.ts` | Dashboard management -- CRUD via persistence layer, component data source validation, agent cleanup on component removal |
| `DataPushSlice` | `dataPushSlice.ts` | Push target management -- CRUD, start/stop, test, delivery logs with pagination, push statistics |
| `InstanceSlice` | `instanceSlice.ts` | Remote NeoMind instance management -- CRUD, health testing, runtime switching with full-page reload strategy, API key decryption |
| `FrontendComponentSlice` | `frontendComponentSlice.ts` | Community marketplace components -- fetch installed, browse market, install from market or manual ZIP upload, registry sync |
| `AiAnalystSlice` | `aiAnalystSlice.ts` | AI Analyst widget configuration -- per-component system prompts and context window settings |
| `UpdateSlice` | `updateSlice.ts` | Application update state -- available updates, download progress, installation status, update dialog |

### Complete Store Type

```typescript
export type NeoMindStore = AuthSlice
  & SessionSlice
  & UISlice
  & AlertSlice
  & DeviceSlice
  & SettingsSlice
  & ExtensionSlice
  & LlmBackendSlice
  & DashboardSlice
  & DataPushSlice
  & InstanceSlice
  & FrontendComponentSlice
  & AiAnalystSlice
  & UpdateSlice
```

## Complete Page Routes Reference

### Public Routes (No Authentication)

| Route | Page Component | Description |
|-------|---------------|-------------|
| `/login` | `LoginPage` | Login page with username/password; checks if setup is needed |
| `/setup` | `SetupPage` | First-launch setup wizard (only accessible when no users exist) |
| `/share/:token` | `SharedDashboardPage` | Public shared dashboard viewer (no auth required) |

### Protected Routes (Authentication Required)

| Route | Page Component | Nav Tab | Description |
|-------|---------------|---------|-------------|
| `/` | `ChatPage` | Chat | Default route; redirects to AI chat |
| `/chat` | `ChatPage` | Chat | AI chat page with session management |
| `/chat/:sessionId` | `ChatPage` | Chat | Specific chat session |
| `/visual-dashboard` | `VisualDashboard` | Dashboard | Visual dashboard builder with drag-and-drop grid |
| `/visual-dashboard/:id` | `VisualDashboard` | Dashboard | Edit specific dashboard |
| `/data` | `DataExplorerPage` | Data | Data explorer (telemetry metrics) + push targets tab |
| `/devices` | `DevicesPage` | Devices | Device management with tabs (list, types, drafts) |
| `/devices/:id` | `DevicesPage` | Devices | Device detail view |
| `/devices/types` | `DevicesPage` | Devices | Device type definitions |
| `/devices/drafts` | `DevicesPage` | Devices | Auto-onboarded pending devices |
| `/automation` | `AutomationPage` | Automation | Rule engine management |
| `/automation/transforms` | `AutomationPage` | Automation | Data transform scripts |
| `/agents` | `AgentsPage` | Agents | AI Agent management with tabs (list, memory, skills) |
| `/agents/memory` | `AgentsPage` | Agents | Agent memory configuration |
| `/agents/skills` | `AgentsPage` | Agents | Agent skill management |
| `/settings` | `SettingsPage` | Settings | System settings (preferences, LLM backends, about) |
| `/messages` | `MessagesPage` | Messages | Notification messages |
| `/messages/channels` | `MessagesPage` | Messages | Notification channel configuration |
| `/extensions` | `ExtensionsPage` | Extensions | Extension management (unified system) |
| `/plugins` | Redirect | -- | Redirects to `/extensions` (legacy compatibility) |
| `*` | Redirect | -- | Catch-all redirects to `/` |

### Route Guards

```typescript
// SetupRoute - Only accessible when setup is required
// ProtectedRoute - Checks JWT token or API key, redirects to /login if missing
// Also checks /api/setup/status in background, redirects to /setup if needed
```

### Lazy Loading

All page components are lazy-loaded via `React.lazy()` for code splitting:

```typescript
const ChatPage = lazy(() => import('@/pages/chat').then(m => ({ default: m.ChatPage })))
const VisualDashboard = lazy(() => import('@/pages/dashboard-components/VisualDashboard').then(m => ({ default: m.VisualDashboard })))
// ... all other pages similarly
```
