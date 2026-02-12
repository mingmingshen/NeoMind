# Web Frontend Module

**Version**: 0.5.8
**Completion**: 85%
**Purpose**: React + TypeScript Web Application / Tauri Desktop Application

## Overview

The Web module is NeoMind's frontend application, supporting both Web browser and Tauri desktop application (dual mode). Built with React 18 + TypeScript + Vite, using Zustand for state management, Radix UI + Tailwind CSS for the user interface.

## Important Changes (v0.5.x)

### Plugin → Extension Migration

- **`/plugins` routes migrated to `/extensions`**
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
│   │   └── ...
│   ├── dashboard/             # Dashboard components
│   │   ├── DashboardGrid.tsx
│   │   ├── registry/
│   │   └── config/
│   ├── automation/            # Automation components
│   │   ├── rule/
│   │   └── TransformTestDialog.tsx
│   ├── devices/               # Device-related components
│   │   ├── DeviceList.tsx
│   │   ├── DeviceDetail.tsx
│   │   └── ...
│   ├── session/               # Session management components
│   ├── extensions/            # Extension components (unified Extension management)
│   ├── shared/                # Shared components
│   └── design-system/         # Design system
├── pages/                     # Page components
│   ├── login.tsx              # Login page
│   ├── setup.tsx              # Initial setup
│   ├── chat.tsx               # Chat page
│   ├── devices.tsx            # Devices page
│   ├── automation.tsx         # Automation page
│   ├── agents.tsx             # AI Agent page
│   ├── settings.tsx           # Settings page
│   ├── commands.tsx           # Commands page
│   ├── decisions.tsx          # Decisions page
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
│   │   ├── decisionSlice.ts   # Decision state
│   │   ├── extensionSlice.ts  # Extension state
│   │   ├── llmBackendSlice.ts # LLM backend state
│   │   └── dashboardSlice.ts  # Dashboard state
│   ├── selectors/             # Selectors
│   └── persistence/           # Persistence
├── lib/                       # Utility libraries
│   ├── api.ts                 # API client
│   ├── websocket.ts           # WebSocket chat
│   ├── events.ts              # Event stream (SSE/WS)
│   ├── auth/                  # Auth utilities
│   ├── notify.ts              # Notification system
│   └── utils/                 # Helper functions
├── hooks/                     # React Hooks
│   ├── useDialog.ts
│   ├── useForm.ts
│   ├── use-toast.ts
│   └── use-confirm.ts
├── types/                     # TypeScript types
│   └── index.ts               # Unified type definitions
├── design-system/             # Design system
│   ├── components/
│   └── tokens/
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

// Protected routes (require auth)
/                         // Default redirects to chat page
/chat                    // AI chat page
/chat/:sessionId         // Specific session chat
/devices                 // Device management page
/devices/:id             // Device details page
/devices/types           // Device types page
/devices/drafts          // Draft devices page (auto-onboarding)
/automation              // Automation page
/automation/transforms   // Data transform page
/agents                  // AI Agents page
/settings                // Settings page
/commands                // Commands page
/decisions               // Decisions page
/messages                // Messages page
/messages/channels       // Message channels page
/extensions              // Extension management page (unified Extension system)
/visual-dashboard        // Visual dashboard
/visual-dashboard/:id    // Specific dashboard
```

## State Management (Zustand)

### Store Structure

```typescript
export type NeoMindStore = AuthSlice
  & SessionSlice
  & UISlice
  & AlertSlice
  & DeviceSlice
  & SettingsSlice
  & DecisionSlice
  & ExtensionSlice
  & LlmBackendSlice
  & DashboardState
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
| `Pagination` | Pagination component |
| `EmptyState` | Empty state indicator |
| `LoadingState` | Loading state |
| `StatusBadge` | Status badge |
| `ActionBar` | Action bar |
| `MonitorStatsGrid` | Monitoring stats grid |

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
        border: 'hsl(var(--border))',
        input: 'hsl(var(--input))',
        ring: 'hsl(var(--ring))',
        background: 'hsl(var(--background))',
        foreground: 'hsl(var(--foreground))',
        primary: {
          DEFAULT: 'hsl(var(--primary))',
          foreground: 'hsl(var(--primary-foreground))',
        },
        // ...
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

### Color System

```css
:root {
  --background: 0 0% 100%;
  --foreground: 240 10% 3.9%;
  --card: 0 0% 100%;
  --card-foreground: 240 10% 3.9%;
  --popover: 0 0% 100%;
  --popover-foreground: 240 10% 3.9%;
  --primary: 240 5.9% 10%;
  --primary-foreground: 0 0% 98%;
  --secondary: 240 4.8% 95.9%;
  --secondary-foreground: 240 5.9% 10%;
  --muted: 240 4.8% 95.9%;
  --muted-foreground: 240 3.8% 46.1%;
  --accent: 240 4.8% 95.9%;
  --accent-foreground: 240 5.9% 10%;
  --destructive: 0 84.2% 60.2%;
  --destructive-foreground: 0 0% 98%;
  --border: 240 5.9% 90%;
  --input: 240 5.9% 90%;
  --ring: 240 5.9% 10%;
  --radius: 0.5rem;
}

.dark {
  --background: 240 10% 3.9%;
  --foreground: 0 0% 98%;
  --card: 240 10% 3.9%;
  --card-foreground: 0 0% 98%;
  --popover: 240 10% 3.9%;
  --popover-foreground: 0 0% 98%;
  --primary: 0 0% 98%;
  --primary-foreground: 240 5.9% 10%;
  --secondary: 240 3.7% 15.9%;
  --secondary-foreground: 0 0% 98%;
  --muted: 240 3.7% 15.9%;
  --muted-foreground: 240 5% 64.9%;
  --accent: 240 3.7% 15.9%;
  --accent-foreground: 0 0% 98%;
  --destructive: 0 62.8% 30.6%;
  --destructive-foreground: 0 0% 98%;
  --border: 240 3.7% 15.9%;
  --input: 240 3.7% 15.9%;
  --ring: 240 4.9% 83.9%;
}
```

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
