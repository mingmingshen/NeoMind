/**
 * Store state type definitions
 *
 * This file contains all the state interfaces for the Zustand store slices.
 */

import type {
  Device,
  DeviceType,
  Alert,
  Message,
  DiscoveredDevice,
  TelemetryDataResponse,
  TelemetrySummaryResponse,
  CommandHistoryResponse,
  DecisionDto,
  ChatSession,
  UserInfo,
  AdapterPluginDto,
} from '@/types'

// ============================================================================
// Auth State
// ============================================================================

export interface AuthState {
  apiKey: string | null
  isAuthenticated: boolean
  // User authentication (JWT)
  user: UserInfo | null
  token: string | null
}

// ============================================================================
// Session State
// ============================================================================

export interface SessionState {
  sessionId: string | null
  messages: Message[]
  sessions: ChatSession[]
}

// ============================================================================
// Device State
// ============================================================================

export interface DeviceState {
  devices: Device[]
  deviceTypes: DeviceType[]
  selectedDevice: Device | null
  selectedDeviceId: string | null
  deviceDetails: Device | null
  deviceTypeDetails: DeviceType | null
  discovering: boolean
  discoveredDevices: DiscoveredDevice[]
  devicesLoading: boolean
  deviceTypesLoading: boolean

  // Device Adapters
  deviceAdapters: AdapterPluginDto[]
  deviceAdaptersLoading: boolean

  // Dialog states
  addDeviceDialogOpen: boolean
  addDeviceTypeDialogOpen: boolean
  deviceDetailsDialogOpen: boolean
}

// ============================================================================
// Telemetry State
// ============================================================================

export interface TelemetryState {
  telemetryData: TelemetryDataResponse | null
  telemetrySummary: TelemetrySummaryResponse | null
  commandHistory: CommandHistoryResponse | null
  telemetryLoading: boolean
}

// ============================================================================
// Alert State
// ============================================================================

export interface AlertState {
  alerts: Alert[]
  alertsLoading: boolean
}

// ============================================================================
// Decision State
// ============================================================================

export interface DecisionState {
  decisions: DecisionDto[]
  decisionsLoading: boolean
}

// ============================================================================
// Settings State
// ============================================================================
// LLM/MQTT/Device settings are now managed via the Plugin system.
// Only general system settings remain here.

export interface SettingsState {
  settingsDialogOpen: boolean
}

// ============================================================================
// UI State
// ============================================================================

export type PageName =
  | 'dashboard'
  | 'devices'
  | 'alerts'
  | 'automation'
  | 'commands'
  | 'decisions'
  | 'plugins'
  | 'settings'
  | 'events'

export interface UIState {
  currentPage: PageName
  sidebarOpen: boolean
  wsConnected: boolean
}

// ============================================================================
// WebSocket State
// ============================================================================

export interface WebSocketState {
  wsConnected: boolean
}

// ============================================================================
// Combined Root State
// ============================================================================

export interface RootState
  extends AuthState,
    SessionState,
    DeviceState,
    TelemetryState,
    AlertState,
    DecisionState,
    SettingsState,
    UIState {}

// ============================================================================
// Page Titles Mapping
// ============================================================================

export const pageTitles: Record<PageName, string> = {
  dashboard: '对话',
  devices: '设备',
  alerts: '告警',
  automation: '自动化',
  commands: '命令',
  decisions: 'AI决策',
  plugins: '插件',
  settings: '设置',
  events: '事件中心',
}
