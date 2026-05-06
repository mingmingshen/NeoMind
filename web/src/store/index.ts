/**
 * Zustand Store
 *
 * This is the main store entry point that combines all slices.
 * It uses Zustand's slice pattern for better code organization.
 */

import { create } from 'zustand'
import { devtools, persist, createJSONStorage, type StateStorage } from 'zustand/middleware'
import { onUnauthorized, tokenManager } from '@/lib/api'

// ============================================================================
// Storage Configuration
// ============================================================================

// Storage strategy: Do NOT persist messages to LocalStorage
// Messages are already persisted in the backend database.
// This prevents QuotaExceededError during long conversations.
/**
 * Custom storage that handles QuotaExceededError gracefully.
 * When storage is full, it clears old data and retries.
 */
const baseStorage: StateStorage = {
  getItem: (name: string): string | null => {
    return localStorage.getItem(name)
  },
  setItem: (name: string, value: string): void => {
    try {
      localStorage.setItem(name, value)
    } catch (error) {
      if (error instanceof DOMException && error.name === 'QuotaExceededError') {
        console.warn('[Storage] LocalStorage quota exceeded, clearing old data...')
        // Clear the store and try again
        localStorage.removeItem('neomind-store')
        try {
          localStorage.setItem(name, value)
        } catch (retryError) {
          console.error('[Storage] Failed to save even after clearing:', retryError)
        }
      } else {
        throw error
      }
    }
  },
  removeItem: (name: string): void => {
    localStorage.removeItem(name)
  },
}

// Create JSON storage wrapper for Zustand
const safeStorage = createJSONStorage(() => baseStorage)

// Import all slices
import { createAuthSlice } from './slices/authSlice'
import { createSessionSlice } from './slices/sessionSlice'
import { createUISlice } from './slices/uiSlice'
import { createAlertSlice } from './slices/alertSlice'
import { createDeviceSlice } from './slices/deviceSlice'
import { createSettingsSlice } from './slices/settingsSlice'
import { createExtensionSlice } from './slices/extensionSlice'
import { createLlmBackendSlice } from './slices/llmBackendSlice'
import { createDashboardSlice } from './slices/dashboardSlice'
import { createUpdateSlice } from './slices/updateSlice'
import { createAiAnalystSlice } from './slices/aiAnalystSlice'
import { createInstanceSlice } from './slices/instanceSlice'

// Import types
import type { AuthSlice } from './slices/authSlice'
import type { SessionSlice } from './slices/sessionSlice'
import type { UISlice } from './slices/uiSlice'
import type { AlertSlice } from './slices/alertSlice'
import type { DeviceSlice } from './slices/deviceSlice'
import type { SettingsSlice } from './slices/settingsSlice'
import type { ExtensionSlice } from './slices/extensionSlice'
import type { LlmBackendSlice } from './slices/llmBackendSlice'
import type { DashboardState } from './slices/dashboardSlice'
import type { UpdateSlice } from './slices/updateSlice'
import type { AiAnalystSlice } from './slices/aiAnalystSlice'
import type { InstanceSlice } from './slices/instanceSlice'

// ============================================================================
// Combined Store Type
// ============================================================================

export type NeoMindStore = AuthSlice
  & SessionSlice
  & UISlice
  & AlertSlice
  & DeviceSlice
  & SettingsSlice
  & ExtensionSlice
  & LlmBackendSlice
  & DashboardState
  & UpdateSlice
  & AiAnalystSlice
  & InstanceSlice

// ============================================================================
// Create Store
// ============================================================================

export const useStore = create<NeoMindStore>()(
  devtools(
    persist(
      (set, get, api) => ({
        // Combine all slices
        ...createAuthSlice(set, get, api),
        ...createSessionSlice(set, get, api),
        ...createUISlice(set, get, api),
        ...createAlertSlice(set, get, api),
        ...createDeviceSlice(set, get, api),
        ...createSettingsSlice(set, get, api),
        ...createExtensionSlice(set, get, api),
        ...createLlmBackendSlice(set, get, api),
        ...createDashboardSlice(set, get, api),
        ...createUpdateSlice(set, get, api),
        ...createAiAnalystSlice(set, get, api),
        ...createInstanceSlice(set, get, api),
      }),
      {
        name: 'neomind-store',
        version: 2,
        storage: safeStorage,
        partialize: (state) => ({
          // NOTE: Do NOT persist messages to LocalStorage!
          // Messages are already persisted in the backend database.
          // This prevents QuotaExceededError during long conversations.
          // When the user switches sessions, messages are loaded from the backend.
          sessionId: state.sessionId,
          // Only persist essential UI state
          sidebarOpen: state.sidebarOpen,
          // Persist update state so users are reminded after app restart
          updateInfo: state.updateInfo,
          updateDialogOpen: state.updateDialogOpen,
          // NOTE: currentInstanceId is NOT persisted here — it's managed
          // separately via localStorage('currentInstanceId') in instanceSlice
          // so that applyPendingSwitch() and the initial state stay in sync.
        }),
        migrate: (persisted, version) => {
          if (version < 2) {
            // v0/v1→v2: remove stale currentInstanceId that overrides
            // the one managed by instanceSlice's own localStorage key.
            // This fixes instances where the old persisted data caused
            // isRemoteInstance() to return true, skipping API calls.
            const old = persisted as Record<string, unknown>
            delete old.currentInstanceId
            return old
          }
          return persisted
        },
      }
    ),
    // Performance optimization: Disable Redux DevTools in production
    // Reduces state update overhead by ~15%
    { enabled: import.meta.env.DEV }
  )
)

// Re-export the store as useAppStore for convenience
export const useAppStore = useStore

// Performance optimization: Only expose store globally in development
if (typeof window !== 'undefined' && import.meta.env.DEV) {
  (window as any).neomindStore = useStore
}

// ============================================================================
// Re-export page titles
// ============================================================================

export { getPageTitle } from './types'

// ============================================================================
// Global 401 Handler Registration
// ============================================================================

// Register a global callback for all 401 errors
// This clears the invalid token and shows the login screen
onUnauthorized(() => {
  // Clear the invalid token
  tokenManager.clearToken()
  // Update store state
  useStore.setState({ token: null, user: null, isAuthenticated: false })
})
