# Web 前端模块

**版本**: 0.8.0
**完成度**: 90%
**用途**: React + TypeScript Web 应用 / Tauri 桌面应用

## 概述

Web 模块是 NeoMind 的前端应用，支持 Web 浏览器和 Tauri 桌面应用（双模式）。基于 React 18 + TypeScript + Vite 构建，使用 Zustand 进行状态管理，Radix UI + Tailwind CSS 构建用户界面。

## 重要变更 (v0.7.x – v0.8.x)

### 设计体系重构 (v0.7)

- **OKLCH 色彩系统** — CSS 色彩令牌从 HSL 迁移至 OKLCH，实现感知均匀的色彩过渡
- **设计令牌** — 所有硬编码的 Tailwind 调色板颜色替换为语义化设计令牌（`text-success`、`bg-error-light`、`text-accent-orange` 等）
- **极光背景与毛玻璃效果** — 全局极光渐变背景，导航栏采用毛玻璃风格
- **字体** — Plus Jakarta Sans（拉丁文）+ Noto Sans SC（中日韩）字体
- **UnifiedFormDialog** — 集中化的表单对话框组件
- **FullScreenDialog** — 全屏构建器对话框，带侧边栏布局，用于复杂编辑器

### 数据推送与通知通道 (v0.8)

- **数据探索页** (`/data`) — 统一页面，包含数据探索和推送目标选项卡
- **PushTargetsTab** — 管理数据推送目标，支持创建/编辑/测试操作
- **DeliveryHistoryPanel** — 查看投递日志，支持分页和状态过滤
- **ChannelEditorDialog** — FullScreenDialog 用于创建/编辑通知通道（5 种通道类型：webhook、email、telegram、dingtalk、wecom）
- **GlobalChatFab** — 浮动操作按钮，从任何页面快速访问 AI 聊天

### LLM 后端管理 (v0.8)

- **UnifiedLLMBackendsTab** — 设置页中的统一选项卡，管理 LLM 后端实例
- **LLMBackendConfigDialog** — 配置后端适配器的对话框（Ollama、OpenAI 兼容等）

### Plugin → Extension 迁移 (v0.5)

- **`/plugins` 路由已迁移到 `/extensions`**（保留重定向）
- `plugins.tsx` → `extensions.tsx`
- 扩展组件统一在 `components/extensions/` 目录
- API调用从 `api.listPlugins()` 改为 `api.listExtensions()`

## 模块结构

```
web/src/
├── main.tsx                    # 应用入口
├── App.tsx                     # 路由和认证
├── components/                 # UI 组件
│   ├── ui/                    # 基础 UI 组件 (Radix UI 封装)
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
│   ├── layout/                # 布局组件
│   │   └── TopNav.tsx
│   ├── chat/                  # 聊天相关组件
│   │   ├── MarkdownMessage.tsx
│   │   ├── ThinkingBlock.tsx
│   │   ├── StreamProgress.tsx
│   │   ├── ToolCallVisualization.tsx
│   │   ├── GlobalChatFab.tsx  # [v0.8] 浮动 AI 聊天按钮
│   │   └── ...
│   ├── dashboard/             # 仪表板组件
│   │   ├── DashboardGrid.tsx
│   │   ├── registry/
│   │   └── config/
│   ├── automation/            # 自动化组件
│   │   ├── rule/
│   │   ├── dialog/
│   │   │   └── FullScreenDialog.tsx  # [v0.7] 全屏构建器对话框
│   │   └── TransformTestDialog.tsx
│   ├── devices/               # 设备相关组件
│   │   ├── DeviceList.tsx
│   │   ├── DeviceDetail.tsx
│   │   └── ...
│   ├── datapush/              # [v0.8] 数据推送组件
│   │   ├── PushTargetsTab.tsx
│   │   ├── PushTargetDialog.tsx
│   │   └── DeliveryHistoryPanel.tsx
│   ├── llm/                   # [v0.8] LLM 后端管理
│   │   ├── UnifiedLLMBackendsTab.tsx
│   │   ├── LLMBackendConfigDialog.tsx
│   │   └── index.ts
│   ├── messages/              # 消息/通知组件
│   │   ├── ChannelEditorDialog.tsx  # [v0.8] FullScreenDialog 通道编辑器
│   │   ├── MessageChannelsTab.tsx
│   │   ├── MessagesTab.tsx
│   │   └── CreateMessageDialog.tsx
│   ├── session/               # 会话管理组件
│   ├── extensions/            # 扩展组件（统一管理Extension）
│   ├── dialog/                # 共享对话框组件
│   │   └── UnifiedFormDialog.tsx    # [v0.7] 集中化表单对话框
│   ├── shared/                # 共享组件
│   │   ├── FullScreenEditor.tsx
│   │   ├── ResponsiveTable.tsx      # [v0.7] 响应式表格，移动端卡片视图
│   │   ├── PaginatedContent.tsx     # [v0.7] 分页，支持无限滚动
│   │   ├── SearchBar.tsx
│   │   ├── ErrorBoundary.tsx
│   │   └── ...
│   └── design-system/         # 设计系统
├── pages/                     # 页面组件
│   ├── login.tsx              # 登录页
│   ├── setup.tsx              # 初始化设置
│   ├── chat.tsx               # 聊天页
│   ├── devices.tsx            # 设备页
│   ├── automation.tsx         # 自动化页
│   ├── agents.tsx             # AI Agent 页
│   ├── settings.tsx           # 设置页（含 LLM 后端选项卡）
│   ├── data-explorer.tsx      # [v0.8] 数据探索 + 推送目标
│   ├── messages.tsx           # 消息页
│   └── extensions.tsx         # 扩展页
├── store/                     # Zustand 状态管理
│   ├── index.ts               # Store 入口
│   ├── slices/                # 状态切片
│   │   ├── authSlice.ts       # 认证状态
│   │   ├── sessionSlice.ts    # 会话状态
│   │   ├── uiSlice.ts         # UI 状态
│   │   ├── deviceSlice.ts     # 设备状态
│   │   ├── alertSlice.ts      # 消息/告警状态
│   │   ├── settingsSlice.ts   # 设置状态
│   │   ├── extensionSlice.ts  # 扩展状态
│   │   ├── llmBackendSlice.ts # LLM 后端状态
│   │   ├── dashboardSlice.ts  # 仪表板状态
│   │   ├── dataPushSlice.ts   # [v0.8] 数据推送状态
│   │   ├── instanceSlice.ts   # [v0.7] 实例状态
│   │   ├── frontendComponentSlice.ts # [v0.7] 前端组件注册
│   │   ├── aiAnalystSlice.ts  # AI 分析师状态
│   │   └── updateSlice.ts     # 更新状态
│   ├── selectors/             # 选择器
│   └── persistence/           # 持久化
├── lib/                       # 工具库
│   ├── api.ts                 # API 客户端
│   ├── websocket.ts           # WebSocket 聊天
│   ├── events.ts              # 事件流 (SSE/WS)
│   ├── auth/                  # 认证工具
│   ├── notify.ts              # 通知系统
│   ├── errors.ts              # 错误处理工具
│   ├── portal.ts              # [v0.7] 模态框/弹出框的 Portal 根节点
│   └── utils/                 # 工具函数
├── hooks/                     # React Hooks
│   ├── useDialog.ts
│   ├── useForm.ts             # [v0.7] 自定义表单 Hook，带验证
│   ├── use-toast.ts
│   ├── use-confirm.ts
│   ├── useErrorHandler.ts
│   └── useMobile.ts           # 移动端检测
├── types/                     # TypeScript 类型
│   └── index.ts               # 统一类型定义
├── design-system/             # 设计系统
│   ├── components/
│   ├── icons/                 # 实体类型图标映射
│   ├── tokens/                # 设计令牌
│   │   ├── color.ts           # OKLCH 色彩令牌
│   │   ├── typography.ts
│   │   ├── size.ts
│   │   └── indicator.ts
│   └── utils/                 # 格式化工具（状态颜色等）
└── i18n/                      # 国际化
```

## 技术栈

### 核心框架

```json
{
  "react": "^18.3.1",
  "react-dom": "^18.3.1",
  "typescript": "^5.3.3",
  "vite": "^5.1.0"
}
```

### 状态管理

```json
{
  "zustand": "^4.5.0"
}
```

### UI 组件库

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

### 其他依赖

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

## 路由结构

```typescript
// 公开路由
/login                    // 登录页（检查是否需要初始化）
/setup                    // 初始化设置页

// 共享路由（无需认证）
/share/:token             // 共享仪表板（代理访问）

// 受保护路由（需要认证）
/                         // 默认跳转到聊天页
/chat                    // AI 聊天页
/chat/:sessionId         // 指定会话聊天
/visual-dashboard        // 可视化仪表板
/visual-dashboard/:id    // 指定仪表板
/data                    // [v0.8] 数据探索（遥测数据 + 推送目标）
/devices                 // 设备管理页
/devices/:id             // 设备详情页
/devices/types           // 设备类型页
/devices/drafts          // 草稿设备页（自动入板）
/automation              // 自动化页
/automation/transforms   // 数据转换页
/agents                  // AI Agents 页
/agents/memory           // Agent 记忆配置
/agents/skills           // Agent 技能管理
/settings                // 设置页（包含 LLM 后端选项卡）
/messages                // 消息页
/messages/channels       // 消息通道页
/extensions              // 扩展管理页（统一Extension系统）
/plugins                 // 重定向到 /extensions（兼容旧版）
```

## 仪表板组件

仪表板组件支持两种数据指定方式：**手动输入**（如 URL、文本）和 **数据源绑定**（关联到设备指标、扩展等）。由 `DualModeSourceField` 组件实现。

### 组件数据源类型

每个组件通过 `allowedTypes` 指定可接受的数据源类型：

| 组件 | 允许的类型 | 说明 |
|------|-----------|------|
| `video-display` | `device`, `device-info`, `device-metric` | 视频流 — 可绑定到提供视频 URL（如 RTSP/HLS 地址）的设备指标 |
| `image-display` | `device-metric`, `system`, `extension`, `transform` | 图片 — 绑定到提供图片数据的指标或扩展 |
| `gauge-chart` | `device-metric`, `system`, `extension`, `transform` | 仪表盘值 — 绑定到数值型指标 |
| `stat-card` | `device-metric`, `system`, `extension`, `transform` | 统计值 — 绑定到任意数值型指标 |

### DataSourceId 格式

```
{type}:{id}:{field}
```

示例：`extension:weather:temp` — `weather` 扩展的 `temp` 字段。

## 状态管理 (Zustand)

### Store 结构

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

### AuthSlice - 认证状态

```typescript
interface AuthSlice {
  // 状态
  token: string | null
  user: UserInfo | null
  isAuthenticated: boolean

  // 操作
  setToken: (token: string, remember?: boolean) => void
  clearToken: () => void
  setUser: (user: UserInfo) => void
  checkAuthStatus: () => Promise<void>
}
```

### SessionSlice - 会话状态

```typescript
interface SessionSlice {
  // 状态
  messages: Message[]
  sessionId: string
  sessions: ChatSession[]
  isStreaming: boolean

  // 操作
  setMessages: (messages: Message[]) => void
  addMessage: (message: Message) => void
  updateLastMessage: (content: string, isPartial?: boolean) => void
  setSessionId: (id: string) => void
  setSessions: (sessions: ChatSession[]) => void
}
```

### DeviceSlice - 设备状态

```typescript
interface DeviceSlice {
  // 状态
  devices: Device[]
  deviceTypes: DeviceType[]
  loading: boolean

  // 操作
  setDevices: (devices: Device[]) => void
  setDeviceTypes: (types: DeviceType[]) => void
  updateDevice: (id: string, updates: Partial<Device>) => void
  fetchDevices: () => Promise<void>
}
```

## API 客户端

### fetchAPI - 统一请求函数

```typescript
async function fetchAPI<T>(
  path: string,
  options: FetchOptions = {}
): Promise<T>

interface FetchOptions extends RequestInit {
  skipAuth?: boolean          // 跳过 JWT 认证
  skipGlobalError?: boolean    // 跳过全局错误处理
  skipErrorToast?: boolean     // 跳过错误提示
  successMessage?: string      // 成功提示消息
}
```

### API 端点分组

```typescript
export const api = {
  // 认证
  login: (username: string, password: string) => Promise<LoginResponse>
  logout: () => Promise<{ message: string }>
  getCurrentUser: () => Promise<UserInfo>

  // 设备
  getDevices: () => Promise<{ devices: Device[]; count: number }>
  getDevice: (id: string) => Promise<Device>
  addDevice: (req: AddDeviceRequest) => Promise<{ device_id: string }>
  deleteDevice: (id: string) => Promise<{ deleted: boolean }>
  sendCommand: (deviceId: string, command: string, params: Record<string, unknown>) => Promise<...>

  // 自动化
  listAutomations: (params?: AutomationFilter) => Promise<AutomationListResponse>
  createAutomation: (req: CreateAutomationRequest) => Promise<...>
  analyzeAutomationIntent: (description: string) => Promise<IntentResult>

  // Agents
  listAgents: () => Promise<AgentListResponse>
  getAgent: (id: string) => Promise<AiAgentDetail>
  executeAgent: (id: string, req?: ExecuteAgentRequest) => Promise<...>

  // ... 更多端点
}
```

## WebSocket 聊天

### ChatWebSocket 类

```typescript
export class ChatWebSocket {
  // 连接
  connect(initialSessionId?: string): void
  disconnect(): void
  manualReconnect(): void
  isConnected(): boolean

  // 消息发送
  sendMessage(content: string, images?: ChatImage[]): void
  sendRequest(request: ClientChatMessage): void

  // 会话管理
  setSessionId(sessionId: string): void
  getSessionId(): string | null
  setActiveBackend(backendId: string | null): void

  // 事件监听
  onMessage(handler: (message: ServerMessage) => void): () => void
  onConnection(handler: (connected: boolean, isReconnect?: boolean) => void): () => void
  onStateChange(handler: (state: ConnectionState) => void): () => void

  // 状态
  getState(): ConnectionState
}

// 单例
export const ws = new ChatWebSocket()
```

### 服务器消息类型

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

## 事件流 (SSE/WebSocket)

### EventsWebSocket 类

```typescript
export class EventsWebSocket {
  // 连接
  connect(): void
  disconnect(): void
  isConnected(): boolean

  // 订阅
  onEvent(handler: (event: NeoMindEvent) => void): () => void
  on<T extends EventType>(eventType: T, handler: (event: Extract<NeoMindEvent, { type: T }>) => void): () => void
  onDeviceEvent(handler: (event: DeviceMetricEvent) => void): () => void
  onRuleEvent(handler: (event: RuleTriggeredEvent) => void): () => void
  onConnection(handler: (connected: boolean) => void): () => void

  // 配置
  updateConfig(config: Partial<EventsConfig>): void
}

// 获取连接
export function getEventsConnection(key?: string, config?: EventsConfig): EventsWebSocket
```

### 事件类型

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

## UI 组件

### 基础组件

基于 Radix UI 的封装组件，使用 `class-variance-authority` 管理变体：

```typescript
// Button 示例
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

### 共享组件

| 组件 | 描述 |
|------|------|
| `StatsCard` | 统计卡片 |
| `BulkActionBar` | 批量操作栏 |
| `Pagination` | 分页组件（默认每页 10 条） |
| `PaginatedContent` | [v0.7] 分页组件，移动端支持无限滚动 |
| `ResponsiveTable` | [v0.7] 响应式表格，移动端卡片视图，内置骨架屏加载 |
| `EmptyState` | 空状态提示 |
| `LoadingState` | 加载状态（页面级使用骨架屏，行内使用加载动画） |
| `StatusBadge` | 状态徽章 |
| `ActionBar` | 操作栏 |
| `MonitorStatsGrid` | 监控统计网格 |
| `ErrorBoundary` | React 错误边界包装器 |
| `SearchBar` | 带防抖的搜索输入 |
| `FullScreenEditor` | 全屏编辑器包装器 |
| `KeepAlive` | 组件保活包装器 |

### 聊天组件

| 组件 | 描述 |
|------|------|
| `MarkdownMessage` | Markdown 消息渲染 |
| `ThinkingBlock` | 思考过程展示 |
| `StreamProgress` | 流式输出进度 |
| `ToolCallVisualization` | 工具调用可视化 |
| `SessionSidebar` | 会话侧边栏 |
| `ConnectionStatus` | 连接状态指示 |

## Tauri 桌面应用

### 检测 Tauri 环境

```typescript
const isTauri = !!(window as any).__TAURI__

// Tauri 使用本地后端
const API_BASE = isTauri ? 'http://localhost:9375/api' : '/api'
const wsHost = isTauri ? 'localhost:9375' : window.location.host
```

### 启动加载

```typescript
// Tauri 应用等待后端就绪
if (isTauri && !backendReady) {
  return <StartupLoading onReady={() => setBackendReady(true)} />
}
```

## 开发命令

```bash
# 开发模式
npm run dev

# 构建
npm run build

# 预览
npm run preview

# Tauri 开发
npm run tauri:dev

# Tauri 构建
npm run tauri:build

# 测试 (Playwright)
npm run test
npm run test:ui
npm run test:headed

# 代码检查
npm run lint
```

## 构建配置

### Vite 配置

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

### Tailwind 配置

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
        // ... 使用 OKLCH CSS 变量的语义化设计令牌
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

## 设计系统

### 色彩系统 (OKLCH)

所有 CSS 色彩令牌使用 OKLCH 实现感知均匀的色彩过渡：

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
  /* ... 暗模式令牌遵循相同的 OKLCH 模式 */
}
```

设计令牌定义在 `web/src/design-system/tokens/color.ts`：

```typescript
// OKLCH 图表颜色 — 高对比度调色板，用于数据可视化
export const chartColors = {
  1: 'oklch(0.62 0.22 270)',   // 靛蓝
  2: 'oklch(0.65 0.20 155)',   // 翡翠绿
  3: 'oklch(0.72 0.17 65)',    // 琥珀
  4: 'oklch(0.67 0.20 25)',    // 橙色
  5: 'oklch(0.65 0.18 340)',   // 玫红
  6: 'oklch(0.68 0.12 210)',   // 天蓝
} as const

// 状态颜色 — 按语义含义调整
export const statusColors = {
  success: 'oklch(0.65 0.20 155)',   // 翡翠绿
  warning: 'oklch(0.72 0.17 65)',    // 琥珀
  error: 'oklch(0.58 0.22 25)',      // 深红橙
  info: 'oklch(0.62 0.22 270)',      // 靛蓝
  neutral: 'oklch(0.55 0.02 260)',   // 冷灰
} as const
```

**重要**：只使用设计令牌类名（`text-success`、`bg-error-light`、`text-accent-orange` 等）。禁止使用硬编码的 Tailwind 调色板颜色（`bg-blue-500`、`text-green-600`）。基于 CSS 变量的颜色不支持 Tailwind `/` 透明度修饰符 — 请使用预定义的令牌（`bg-muted-30`、`bg-success-light`）。

### 图标系统

使用 `lucide-react` 图标库，统一风格：

```typescript
import {
  Bot,   // AI/Agent
  Cpu,   // 设备
  Zap,   // 自动化
  Bell,  // 消息/通知
  Settings,  // 设置
  // ...
} from 'lucide-react'
```

## 国际化

使用 `i18next` + `react-i18next`：

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

## 性能优化

### 代码分割

```typescript
// 路由懒加载
const ChatPage = lazy(() => import('@/pages/chat').then(m => ({ default: m.ChatPage })))
const DevicesPage = lazy(() => import('@/pages/devices').then(m => ({ default: m.DevicesPage })))
```

### 虚拟列表

```typescript
// 大列表使用虚拟滚动
import { VirtualList } from '@/components/ui/VirtualList'

<VirtualList
  items={items}
  itemHeight={50}
  renderItem={(item) => <ListItem item={item} />}
/>
```

### 防抖/节流

```typescript
import { useDebounce, useThrottle } from '@/hooks/useDebounce'

const debouncedSearch = useDebounce(searchTerm, 300)
const throttledSave = useThrottle(saveData, 1000)
```

## 设计原则

1. **组件化**: 小而专注的组件，单一职责
2. **类型安全**: 完整的 TypeScript 类型定义
3. **响应式**: 移动端和桌面端适配
4. **可访问性**: 遵循 WCAG 标准，使用 ARIA
5. **性能优先**: 代码分割、懒加载、虚拟滚动

## 数据推送模块 (v0.8)

数据推送模块提供可配置的推送目标，按计划将设备遥测数据投递到外部服务。

### 组件

| 组件 | 描述 |
|------|------|
| `PushTargetsTab` | 主选项卡，显示推送目标列表，支持 CRUD 操作 |
| `PushTargetDialog` | 创建/编辑推送目标配置的对话框 |
| `DeliveryHistoryPanel` | 查看投递日志，支持分页和状态过滤 |

### 状态管理 (DataPushSlice)

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

## 通知通道 (v0.8)

通知通道系统支持 5 种通道类型，通过 `ChannelEditorDialog`（FullScreenDialog）进行管理。

### 支持的通道类型

| 类型 | 描述 |
|------|------|
| `webhook` | HTTP Webhook（POST 到外部 URL） |
| `email` | 通过 SMTP 配置发送邮件 |
| `telegram` | Telegram Bot API |
| `dingtalk` | 钉钉机器人 Webhook |
| `wecom` | 企业微信机器人 Webhook |

### ChannelEditorDialog

使用带侧边栏布局的 FullScreenDialog：

```typescript
<FullScreenDialog open={open} onOpenChange={onOpenChange}>
  <FullScreenDialogHeader />
  <FullScreenDialogContent>
    <FullScreenDialogSidebar>  {/* 通道类型选择器 */}
    <FullScreenDialogMain>     {/* 通道配置表单 */}
    <FullScreenDialogFooter>   {/* 操作按钮 */}
  </FullScreenDialogContent>
</FullScreenDialog>
```

## FetchCache 模式 (v0.7)

Store 层级的请求去重，基于 TTL 的缓存（默认 10 秒），防止重复 API 调用：

```typescript
// 模式：shouldFetch -> markFetching -> API 调用 -> markFetched
// 变更时失效缓存

// 在 slice 中：
fetchCache: Record<string, { timestamp: number; fetching: boolean }>

shouldFetch(key: string): boolean     // TTL 过期且未在请求中
markFetching(key: string): void       // 设置请求标志
markFetched(key: string): void        // 更新时间戳
invalidateCache(key?: string): void   // 清除缓存（指定 key 或全部）
```

用于：`deviceSlice`、`extensionSlice`、`sessionSlice`、`llmBackendSlice`、`instanceSlice`、`frontendComponentSlice`。

## 全局聊天 FAB (v0.8)

`GlobalChatFab` 组件在所有非聊天页面上渲染一个浮动操作按钮，点击后展开为全屏聊天覆盖层，带有从 FAB 位置缩放的平滑动画。

### 架构

- **文件**: `web/src/components/chat/GlobalChatFab.tsx`
- **渲染位置**: `App.tsx` 中受保护路由布局内，与 `TopNav` 并列
- **面板状态机**: `closed` -> `opening` -> `open` -> `closing` -> `closed`

### 行为

| 功能 | 描述 |
|------|------|
| FAB 可见性 | 在聊天页（`/`、`/chat`、`/chat/:sessionId`）隐藏，其他页面显示 |
| 面板会话 | 通过 `localStorage` 键 `neomind:panelSessionId` 跨关闭/刷新持久化 |
| 自动关闭 | 用户导航到聊天页时自动关闭（流式输出中则延迟） |
| 流式保护 | 流式响应进行中时通知用户，面板保持打开 |
| 最小化按钮 | 反向动画折叠回 FAB |

### 视觉设计

- FAB：固定 `bottom-6 right-6`，56px 圆形按钮，毛玻璃效果 + 橙色光晕
- 覆盖层：`z-[90]` 背景模糊，点击关闭（流式输出中禁用）
- 面板：`z-[100]` 移动端全屏，大屏幕 inset-4/inset-8，毛玻璃背景

### 关键组件

```typescript
// GlobalChatFab 渲染：
<GlobalChatFab>
  <button />              // FAB（打开时隐藏）
  <div />                 // 背景覆盖层
  <div>                   // 全屏面板
    <PanelChatView        // 复用聊天视图逻辑
      onClose={handleClose}
      onStreamingChange={setIsStreaming}
      ensureSession={ensurePanelSession}
      showMinimize
    />
  </div>
</GlobalChatFab>
```

## 仪表板分享 (v0.7)

仪表板分享功能允许生成公开链接，任何人无需认证即可查看仪表板。系统支持每个仪表板创建多个分享链接，可配置权限和过期时间。

### 组件

| 组件 | 文件 | 描述 |
|------|------|------|
| `ShareManagerDialog` | `components/dashboard/ShareManagerDialog.tsx` | FullScreenDialog 管理分享链接（列表、创建、删除） |
| `SharedDashboard` | `pages/share/SharedDashboard.tsx` | 公开分享仪表板查看器（无需认证） |

### API 端点

```typescript
// 分享管理（需认证）
GET    /api/dashboards/:id/share       // 列出分享令牌
POST   /api/dashboards/:id/share       // 创建分享链接
DELETE /api/dashboards/:id/share/:token // 撤销分享链接

// 公开访问（无需认证）
GET    /api/share/:token               // 获取分享仪表板数据
GET    /api/share/:token/proxy/*       // 小组件数据获取代理
```

### 分享令牌结构

```typescript
interface ShareToken {
  token: string
  permissions: { allow_interactive: boolean }
  created_at: number
  expires_at: number | null      // null = 永不过期
  share_url: string
}
```

### 分享链接创建选项

- **权限**: 只读（默认）或交互式（允许小组件交互）
- **过期时间**: 可选，以小时为单位配置（1h、6h、24h、7d、30d、永不过期）

### 公开查看器架构

`SharedDashboard` 页面（`/share/:token`）的独特之处在于无需认证运行：

1. **请求代理**: 所有 `/api/...` 请求通过全局 `window.fetch` 拦截器（`installShareProxy()`）重写为 `/api/share/:token/proxy/...`
2. **数据转换**: 后端返回 snake_case 字段；`fromDashboardDTO()` 转换为 camelCase
3. **组件注册表**: 社区和扩展组件注册表使用 `skipAuth: true` 加载
4. **实时更新**: 无 WebSocket 可用；设备数据改为每 30 秒轮询一次
5. **错误边界**: 每个组件用 `ComponentErrorBoundary` 包裹，优雅降级不支持的组件

## 初始化向导 (v0.7)

初始化向导是新 NeoMind 安装的首次启动体验，创建管理员账户并引导用户完成初始配置。

### 流程

```
App.tsx 检查 /api/setup/status
  -> setup_required: true  -> 重定向到 /setup
  -> setup_required: false -> 正常登录流程
```

### 步骤

| 步骤 | 组件 | 描述 |
|------|------|------|
| 1. 账户 | `AccountStep` | 创建管理员账户（用户名、密码、可选邮箱）+ 时区选择（自动检测） |
| 2. 完成 | `CompleteStep` | 成功界面 + 快速入门指南（聊天、LLM 设置、探索） |

### SetupRoute 守卫

```typescript
// 仅在需要设置时（无用户存在）可访问
// 如果已完成设置，重定向到 /login
// Tauri 环境：等待后端启动时最多重试 15 次，带退避策略
```

### API 端点

```typescript
GET  /api/setup/status     // { setup_required: boolean }
POST /api/setup/complete   // 标记设置完成（需要账户创建返回的 JWT）
```

### 设计说明

- LLM 配置有意延后 -- 用户首次使用 AI 功能时再配置
- 可选 Mailchimp 新闻订阅（通过 JSONP，AccountStep 中）
- 设置背景使用与主应用相同的极光渐变

## 完整 Store Slices 参考

所有 Zustand store slices 组合为单一的 `NeoMindStore` 类型。以下是完整参考及描述。

| Slice | 文件 | 描述 |
|-------|------|------|
| `AuthSlice` | `authSlice.ts` | 用户认证，JWT 令牌，API 密钥支持，登录/注册/登出，当前用户信息 |
| `SessionSlice` | `sessionSlice.ts` | 聊天会话管理 -- 创建、切换、删除会话；加载消息历史；会话列表无限滚动 |
| `UISlice` | `uiSlice.ts` | UI 状态 -- 侧边栏开关切换，WebSocket 连接状态 |
| `DeviceSlice` | `deviceSlice.ts` | 设备管理 -- CRUD、设备类型、遥测、批量当前值更新、BLE 配网、MDL 生成 |
| `AlertSlice` | `alertSlice.ts` | 告警/通知状态 -- 获取告警、确认、创建（内部使用统一消息 API） |
| `SettingsSlice` | `settingsSlice.ts` | 系统设置 -- 配置导入/导出/验证，设置对话框状态 |
| `ExtensionSlice` | `extensionSlice.ts` | 扩展管理 -- 列表/安装/卸载扩展，命令，数据源，日志，查询执行 |
| `LlmBackendSlice` | `llmBackendSlice.ts` | LLM 后端实例 -- CRUD，激活切换，连接测试，后端类型定义 |
| `DashboardSlice` | `dashboardSlice.ts` | 仪表板管理 -- 通过持久化层 CRUD，组件数据源验证，组件移除时清理 Agent |
| `DataPushSlice` | `dataPushSlice.ts` | 推送目标管理 -- CRUD，启动/停止，测试，带分页的投递日志，推送统计 |
| `InstanceSlice` | `instanceSlice.ts` | 远程 NeoMind 实例管理 -- CRUD，健康测试，运行时切换（全页刷新策略），API 密钥解密 |
| `FrontendComponentSlice` | `frontendComponentSlice.ts` | 社区市场组件 -- 获取已安装、浏览市场、从市场安装或手动 ZIP 上传，注册表同步 |
| `AiAnalystSlice` | `aiAnalystSlice.ts` | AI 分析师小组件配置 -- 每组件系统提示和上下文窗口设置 |
| `UpdateSlice` | `updateSlice.ts` | 应用更新状态 -- 可用更新，下载进度，安装状态，更新对话框 |

### 完整 Store 类型

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

## 完整页面路由参考

### 公开路由（无需认证）

| 路由 | 页面组件 | 描述 |
|------|---------|------|
| `/login` | `LoginPage` | 登录页，用户名/密码；检查是否需要初始化 |
| `/setup` | `SetupPage` | 首次启动初始化向导（仅无用户时可用） |
| `/share/:token` | `SharedDashboardPage` | 公开分享仪表板查看器（无需认证） |

### 受保护路由（需要认证）

| 路由 | 页面组件 | 导航标签 | 描述 |
|------|---------|---------|------|
| `/` | `ChatPage` | 聊天 | 默认路由；跳转到 AI 聊天 |
| `/chat` | `ChatPage` | 聊天 | AI 聊天页，带会话管理 |
| `/chat/:sessionId` | `ChatPage` | 聊天 | 指定聊天会话 |
| `/visual-dashboard` | `VisualDashboard` | 仪表板 | 可视化仪表板构建器，拖拽网格布局 |
| `/visual-dashboard/:id` | `VisualDashboard` | 仪表板 | 编辑指定仪表板 |
| `/data` | `DataExplorerPage` | 数据 | 数据探索（遥测指标）+ 推送目标选项卡 |
| `/devices` | `DevicesPage` | 设备 | 设备管理，带选项卡（列表、类型、草稿） |
| `/devices/:id` | `DevicesPage` | 设备 | 设备详情视图 |
| `/devices/types` | `DevicesPage` | 设备 | 设备类型定义 |
| `/devices/drafts` | `DevicesPage` | 设备 | 自动入板的待确认设备 |
| `/automation` | `AutomationPage` | 自动化 | 规则引擎管理 |
| `/automation/transforms` | `AutomationPage` | 自动化 | 数据转换脚本 |
| `/agents` | `AgentsPage` | Agent | AI Agent 管理，带选项卡（列表、记忆、技能） |
| `/agents/memory` | `AgentsPage` | Agent | Agent 记忆配置 |
| `/agents/skills` | `AgentsPage` | Agent | Agent 技能管理 |
| `/settings` | `SettingsPage` | 设置 | 系统设置（偏好、LLM 后端、关于） |
| `/messages` | `MessagesPage` | 消息 | 通知消息 |
| `/messages/channels` | `MessagesPage` | 消息 | 通知通道配置 |
| `/extensions` | `ExtensionsPage` | 扩展 | 扩展管理（统一系统） |
| `/plugins` | 重定向 | -- | 重定向到 `/extensions`（兼容旧版） |
| `*` | 重定向 | -- | 兜底路由重定向到 `/` |

### 路由守卫

```typescript
// SetupRoute - 仅在需要设置时可访问
// ProtectedRoute - 检查 JWT 令牌或 API 密钥，缺失则重定向到 /login
// 同时后台检查 /api/setup/status，需要时重定向到 /setup
```

### 懒加载

所有页面组件通过 `React.lazy()` 懒加载实现代码分割：

```typescript
const ChatPage = lazy(() => import('@/pages/chat').then(m => ({ default: m.ChatPage })))
const VisualDashboard = lazy(() => import('@/pages/dashboard-components/VisualDashboard').then(m => ({ default: m.VisualDashboard })))
// ... 其他所有页面类似
```
