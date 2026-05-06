/**
 * Instance Slice
 *
 * Manages remote NeoMind backend instances with CRUD operations,
 * health testing, and runtime instance switching.
 *
 * Switching strategy: full-screen overlay + page reload.
 * When the user selects a different instance we persist the target
 * config to localStorage and reload the page.  This eliminates all
 * race-conditions from in-flight requests / stale tokens.
 */

import type { StateCreator } from 'zustand'
import { api, getApiBase, setApiBase, setApiKey, clearApiKey } from '@/lib/api'
import { tokenManager } from '@/lib/auth'
import { logError } from '@/lib/errors'

// ============================================================================
// Constants
// ============================================================================

const INSTANCE_CACHE_KEY = 'neomind_instance_cache'
const CURRENT_INSTANCE_KEY = 'currentInstanceId'
const PENDING_SWITCH_KEY = 'neomind_pending_switch'

// ============================================================================
// Types
// ============================================================================

export interface InstanceInfo {
  id: string
  name: string
  url: string
  api_key?: string
  is_local: boolean
  last_status: string
  last_checked_at: number | null
  created_at: number
}

export interface InstanceTestResult {
  success: boolean
  latency_ms: number
  status: string
  error?: string
}

export type SwitchingState = 'idle' | 'switching' | 'success' | 'error'

export interface InstanceSlice {
  // State
  instances: InstanceInfo[]
  currentInstanceId: string
  instanceLoading: boolean
  switchingState: SwitchingState
  switchingError: string | null
  previousInstanceId: string | null

  // CRUD
  fetchInstances: () => Promise<void>
  addInstance: (data: { name: string; url: string; api_key?: string }) => Promise<string>
  updateInstance: (id: string, data: { name?: string; url?: string; api_key?: string }) => Promise<void>
  deleteInstance: (id: string) => Promise<void>

  // Testing
  testInstance: (id: string) => Promise<InstanceTestResult>

  // Switching
  switchInstance: (id: string) => Promise<void>
  revertSwitch: () => void
  clearSwitchingError: () => void

  // Helpers
  getCurrentInstance: () => InstanceInfo | null
  isRemoteInstance: () => boolean
}

// ============================================================================
// API Helpers
// ============================================================================

async function fetchInstancesApi(): Promise<InstanceInfo[]> {
  const data = await api.get<{ instances: InstanceInfo[] }>('/instances')
  return data.instances || []
}

async function createInstanceApi(data: { name: string; url: string; api_key?: string }): Promise<InstanceInfo> {
  return api.post<InstanceInfo>('/instances', data)
}

async function updateInstanceApi(id: string, data: { name?: string; url?: string; api_key?: string }): Promise<InstanceInfo> {
  return api.put<InstanceInfo>(`/instances/${id}`, data)
}

async function deleteInstanceApi(id: string): Promise<void> {
  await api.delete(`/instances/${id}`)
}

async function testInstanceApi(id: string): Promise<InstanceTestResult> {
  return api.post<InstanceTestResult>(`/instances/${id}/test`, {})
}

/** Read cached instance list from localStorage (available before any API call). */
function getCachedInstances(): InstanceInfo[] {
  try {
    const raw = localStorage.getItem(INSTANCE_CACHE_KEY)
    return raw ? JSON.parse(raw) : []
  } catch {
    return []
  }
}

/** Sync instance list to localStorage cache (used after mutations). */
function syncCache(instances: InstanceInfo[]) {
  try {
    localStorage.setItem(INSTANCE_CACHE_KEY, JSON.stringify(instances))
  } catch { /* ignore storage errors */ }
}

// ============================================================================
// Boot-time: apply pending switch BEFORE anything else
// ============================================================================

interface PendingSwitch {
  targetId: string
  previousId: string
  apiUrl: string
  apiKey: string
}

/**
 * Apply a pending instance switch synchronously at module-load time.
 * This runs BEFORE React renders anything, so the API base is already
 * correct when components mount and ProtectedRoute checks auth.
 */
function applyPendingSwitch(): PendingSwitch | null {
  let pending: PendingSwitch | null = null
  try {
    const raw = localStorage.getItem(PENDING_SWITCH_KEY)
    pending = raw ? JSON.parse(raw) : null
  } catch { /* ignore */ }

  if (!pending) return null

  // Apply API base
  if (pending.apiUrl) {
    setApiBase(pending.apiUrl)
  } else {
    setApiBase('')
  }

  // Always clear JWT on instance switch — a token from one instance
  // is never valid on another (different signing keys / user databases).
  tokenManager.clearToken()

  // Apply API key if provided (for remote instances with key-based auth).
  // When set, ProtectedRoute allows access without JWT login.
  if (pending.apiKey) {
    setApiKey(pending.apiKey)
  } else {
    clearApiKey()
  }

  // DON'T clear the flag yet — the overlay needs it to show the spinner
  return pending
}

// Execute synchronously at import time (before React)
const _pendingSwitch = applyPendingSwitch()

// ============================================================================
// Slice
// ============================================================================

export const createInstanceSlice: StateCreator<
  InstanceSlice,
  [],
  [],
  InstanceSlice
> = (set, get) => ({
  // Initial state
  instances: getCachedInstances(),
  currentInstanceId: (() => {
    try {
      return localStorage.getItem(CURRENT_INSTANCE_KEY) || 'local-default'
    } catch {
      return 'local-default'
    }
  })(),
  instanceLoading: false,
  switchingState: _pendingSwitch ? 'switching' : 'idle',
  switchingError: null,
  previousInstanceId: _pendingSwitch?.previousId ?? null,

  // Fetch all instances — only works when connected to local instance
  fetchInstances: async () => {
    const { isRemoteInstance } = get()
    // On remote instance, skip API call — use cached list
    if (isRemoteInstance()) {
      const cached = getCachedInstances()
      if (cached.length > 0) {
        set({ instances: cached, instanceLoading: false })
      }
      return
    }

    set({ instanceLoading: true })
    try {
      const instances = await fetchInstancesApi()
      try {
        localStorage.setItem(INSTANCE_CACHE_KEY, JSON.stringify(instances))
      } catch { /* ignore storage errors */ }

      // Self-heal: if currentInstanceId points to a non-existent instance,
      // reset to local-default (stale state from a previous failed switch)
      const { currentInstanceId } = get()
      if (currentInstanceId !== 'local-default' &&
          !instances.some((i) => i.id === currentInstanceId)) {
        localStorage.setItem(CURRENT_INSTANCE_KEY, 'local-default')
        set({ instances, currentInstanceId: 'local-default', instanceLoading: false })
      } else {
        set({ instances, instanceLoading: false })
      }
    } catch (err) {
      console.error('[fetchInstances] failed:', err)
      logError(err, { operation: 'fetchInstances' })
      set({ instanceLoading: false })
    }
  },

  // Add a new instance
  addInstance: async (data) => {
    const instance = await createInstanceApi(data)
    const instances = [...get().instances, instance]
    set({ instances })
    syncCache(instances)
    return instance.id
  },

  // Update an existing instance
  updateInstance: async (id, data) => {
    const updated = await updateInstanceApi(id, data)
    const instances = get().instances.map((i) => (i.id === id ? updated : i))
    set({ instances })
    syncCache(instances)
  },

  // Delete an instance
  deleteInstance: async (id) => {
    await deleteInstanceApi(id)
    const instances = get().instances.filter((i) => i.id !== id)
    set({ instances })
    syncCache(instances)
  },

  // Test instance connectivity
  testInstance: async (id) => {
    return testInstanceApi(id)
  },

  // Switch to a different instance — validate first, then overlay + reload
  switchInstance: async (id) => {
    const { instances, currentInstanceId, switchingState } = get()
    if (id === currentInstanceId) return
    if (switchingState === 'switching') return

    // Look up target in current instances, fall back to localStorage cache
    let targetInstance = instances.find((i) => i.id === id)
    if (!targetInstance) {
      const cached = getCachedInstances()
      targetInstance = cached.find((i) => i.id === id)
    }
    if (!targetInstance) {
      set({ switchingState: 'error', switchingError: 'Instance not found' })
      return
    }

    // For remote instances with API key: validate the key before switching
    if (!targetInstance.is_local && targetInstance.api_key) {
      set({ switchingState: 'switching', switchingError: null })
      try {
        const apiUrl = targetInstance.url.replace(/\/+$/, '') + '/api'
        // Use an authenticated endpoint to verify the API key actually works.
        // /api/health is public and doesn't validate the key.
        const res = await fetch(`${apiUrl}/auth/verify`, {
          headers: { 'X-API-Key': targetInstance.api_key },
          signal: AbortSignal.timeout(8000),
        })
        if (res.status === 401) {
          set({
            switchingState: 'error',
            switchingError: 'apiKeyRejected',
          })
          return
        }
        // Any non-401 response means auth passed
      } catch {
        set({
          switchingState: 'error',
          switchingError: 'unreachable',
        })
        return
      }
    } else if (!targetInstance.is_local) {
      // Remote without API key — verify the server is reachable at least
      try {
        const res = await fetch(`${targetInstance.url.replace(/\/+$/, '')}/api/setup/status`, {
          signal: AbortSignal.timeout(8000),
        })
        if (!res.ok) {
          set({
            switchingState: 'error',
            switchingError: `Cannot reach ${targetInstance.url}. Please check the address.`,
          })
          return
        }
      } catch {
        set({
          switchingState: 'error',
          switchingError: 'unreachable',
        })
        return
      }
    }

    // Show overlay
    set({ switchingState: 'switching', switchingError: null })

    // Write pending-switch so applyPendingSwitch() picks it up on reload
    localStorage.setItem(CURRENT_INSTANCE_KEY, id)
    localStorage.setItem(PENDING_SWITCH_KEY, JSON.stringify({
      targetId: id,
      previousId: currentInstanceId,
      apiUrl: targetInstance.is_local ? '' : `${targetInstance.url}/api`,
      apiKey: targetInstance.is_local ? '' : (targetInstance.api_key || ''),
    }))

    // Reload — the simplest and most reliable way to switch
    window.location.reload()
  },

  // Revert to previous instance (called from overlay after a failed switch)
  revertSwitch: () => {
    let pending: PendingSwitch | null = null
    try {
      const raw = localStorage.getItem(PENDING_SWITCH_KEY)
      pending = raw ? JSON.parse(raw) : null
    } catch { /* ignore */ }

    if (!pending?.previousId) return

    // Write a new pending switch back to the previous instance
    const { instances } = get()
    const prevInstance = instances.find((i) => i.id === pending.previousId)
    if (!prevInstance) return

    localStorage.setItem(CURRENT_INSTANCE_KEY, pending.previousId)
    localStorage.setItem(PENDING_SWITCH_KEY, JSON.stringify({
      targetId: pending.previousId,
      previousId: pending.targetId,
      apiUrl: prevInstance.is_local ? '' : `${prevInstance.url}/api`,
      apiKey: prevInstance.is_local ? '' : (prevInstance.api_key || ''),
    }))

    window.location.reload()
  },

  // Clear switching error / dismiss the overlay
  clearSwitchingError: () => {
    localStorage.removeItem(PENDING_SWITCH_KEY)
    set({ switchingState: 'idle', switchingError: null, previousInstanceId: null })
  },

  // Get current instance info
  getCurrentInstance: () => {
    const { instances, currentInstanceId } = get()
    return instances.find((i) => i.id === currentInstanceId) || null
  },

  // Check if currently connected to a remote instance
  // Must verify BOTH stored ID AND actual API base — stale localStorage
  // can have a remote ID while we're actually connected locally.
  isRemoteInstance: () => {
    if (get().currentInstanceId === 'local-default') return false
    const apiBase = getApiBase()
    if (!apiBase || apiBase === '/api') return false
    if (apiBase.includes('localhost') || apiBase.includes('127.0.0.1')) return false
    return true
  },
})

/**
 * Called from InstanceSwitchOverlay on mount to clear the pending flag
 * after the app has successfully booted with the new instance.
 */
export function clearPendingSwitch() {
  localStorage.removeItem(PENDING_SWITCH_KEY)
}
