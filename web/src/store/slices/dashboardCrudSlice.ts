/**
 * Dashboard CRUD Slice
 *
 * Dashboard lifecycle: fetch, create, update, delete.
 * Owns persistence (storage) and debounced sync.
 * scheduleSync/flushSync are exposed as slice methods for other slices.
 */

import type { StateCreator } from 'zustand'
import type {
  Dashboard,
  DashboardComponent,
  DashboardTemplate,
  DashboardLayout,
} from '@/types/dashboard'
import { createDashboardStorage, type DashboardStorage } from '../persistence'
import { logError } from '@/lib/errors'
import {
  generateId,
  cleanupAgentForComponent,
  isDataSourceValid,
  updateDashboardInState,
} from './dashboardHelpers'
import type { DashboardStore } from './dashboardHelpers'

// ============================================================================
// Defaults
// ============================================================================

export const DEFAULT_LAYOUT: DashboardLayout = {
  columns: 12,
  rows: 'auto',
  breakpoints: { lg: 1200, md: 996, sm: 768, xs: 480 },
}

export const DEFAULT_TEMPLATES: DashboardTemplate[] = [
  {
    id: 'overview', name: 'Overview',
    description: 'System overview with devices, agents, and events',
    category: 'overview', icon: 'LayoutDashboard',
    layout: DEFAULT_LAYOUT, components: [],
    requiredResources: { devices: 1 },
  },
  {
    id: 'blank', name: 'Blank Canvas',
    description: 'Start from scratch with an empty dashboard',
    category: 'custom', icon: 'Square',
    layout: DEFAULT_LAYOUT, components: [],
  },
]

// ============================================================================
// Slice type
// ============================================================================

export interface DashboardCrudSlice {
  dashboards: Dashboard[]
  currentDashboard: Dashboard | null
  currentDashboardId: string | null
  templates: DashboardTemplate[]
  _fetchId: number | null
  dashboardsLoading: boolean

  fetchDashboards: () => Promise<void>
  createDashboard: (dashboard: Omit<Dashboard, 'id' | 'createdAt' | 'updatedAt'>) => Promise<string>
  updateDashboard: (id: string, updates: Partial<Dashboard>) => Promise<void>
  deleteDashboard: (id: string) => Promise<void>
  setCurrentDashboard: (id: string | null) => void
  setDefaultDashboard: (id: string) => Promise<void>
  clearDashboards: () => void
  persistDashboard: (id?: string) => Promise<void>
  fetchTemplates: () => Promise<void>

  // Sync methods used by other slices
  scheduleSync: (dashboard: Dashboard) => void
  flushSync: () => Promise<void>
}

// ============================================================================
// Slice factory
// ============================================================================

export const createDashboardCrudSlice: StateCreator<
  DashboardStore, [], [], DashboardCrudSlice
> = (set, get) => {
  // Clear stale localStorage
  try {
    localStorage.removeItem('neomind_dashboards')
    localStorage.removeItem('neomind_current_dashboard_id')
  } catch { /* ignore */ }

  const storage: DashboardStorage = createDashboardStorage({ type: 'hybrid', cacheEnabled: true })

  // Debounced sync — captured in closure
  let syncDebounceTimer: ReturnType<typeof setTimeout> | null = null
  let pendingSyncDashboard: Dashboard | null = null

  function handleIdChange(dash: Dashboard, result: { data: Dashboard | null }): void {
    if (result.data && result.data.id !== dash.id) {
      const { dashboards: currentDashboards } = get()
      const activeDashboardId = get().currentDashboardId
      const newDashboards = currentDashboards.map((d: Dashboard) =>
        d.id === dash.id ? result.data! : d,
      )
      set({
        dashboards: newDashboards,
        ...(activeDashboardId === dash.id
          ? { currentDashboard: result.data, currentDashboardId: result.data.id }
          : {}),
      })
    }
  }

  function scheduleSync(dashboard: Dashboard): void {
    pendingSyncDashboard = dashboard
    if (syncDebounceTimer) clearTimeout(syncDebounceTimer)
    syncDebounceTimer = setTimeout(async () => {
      syncDebounceTimer = null
      const dash = pendingSyncDashboard
      pendingSyncDashboard = null
      if (!dash) return
      try {
        const result = await storage.sync(dash)
        handleIdChange(dash, result)
      } catch (err) {
        console.warn('[DashboardCrudSlice] sync failed:', err)
      }
    }, 500)
  }

  async function flushSync(): Promise<void> {
    if (syncDebounceTimer) {
      clearTimeout(syncDebounceTimer)
      syncDebounceTimer = null
      const dash = pendingSyncDashboard
      pendingSyncDashboard = null
      if (dash) {
        try {
          const result = await storage.sync(dash)
          handleIdChange(dash, result)
        } catch (err) {
          console.warn('[DashboardCrudSlice] flushSync failed:', err)
        }
      }
    }
  }

  return {
    // --- Initial state ---
    dashboards: [],
    currentDashboard: null,
    currentDashboardId: null,
    templates: DEFAULT_TEMPLATES,
    _fetchId: null,
    dashboardsLoading: false,

    // --- Sync methods (used by layout/config slices) ---
    scheduleSync,
    flushSync,

    // --- CRUD ---

    fetchDashboards: async () => {
      const fetchId = Date.now()
      set({ _fetchId: fetchId, dashboardsLoading: true })
      try {
        const result = await storage.load()
        if (get()._fetchId !== fetchId) return

        if (result.error) {
          set({ dashboards: [], dashboardsLoading: false })
        } else if (result.data) {
          // Migration: dataSource from config → component level
          const migrated = result.data.map((d: Dashboard) => ({
            ...d,
            components: d.components.map((comp) => {
              const config = (comp.config || {}) as Record<string, unknown>
              if ('dataSource' in config && config.dataSource) {
                const { dataSource, ...rest } = config
                return { ...comp, config: rest, dataSource } as DashboardComponent
              }
              return comp
            }),
          })) as Dashboard[]

          // Validate data sources
          try {
            const storeState = get()
            const validDeviceIds = new Set<string>(
              (storeState.devices || []).map((d) => d.id || d.device_id).filter((id): id is string => typeof id === 'string'),
            )
            const validExtensionIds = new Set<string>(
              (storeState.extensions || []).map((e) => e.id).filter((id): id is string => typeof id === 'string'),
            )
            if (validDeviceIds.size > 0 || validExtensionIds.size > 0) {
              for (const d of migrated) {
                d.components = d.components.filter(
                  (c) => isDataSourceValid(c, validDeviceIds, validExtensionIds),
                )
              }
            }
          } catch { /* best-effort */ }

          if (get()._fetchId !== fetchId) return
          set({ dashboards: migrated })

          const savedId = (storage as any).getCurrentDashboardId?.()
          const currentState = get()
          let target: Dashboard | null = null
          if (savedId && migrated.length > 0) {
            target = migrated.find((d: Dashboard) => d.id === savedId) || null
          }
          if (!target && !currentState.currentDashboardId && migrated.length > 0) {
            target = migrated.find((d: Dashboard) => d.isDefault) || migrated[0]
          }
          if (target && !currentState.currentDashboard) {
            set({ currentDashboardId: target.id, currentDashboard: target })
          }
        } else {
          set({ dashboards: [] })
        }
      } catch (err) {
        logError(err, { operation: 'Load dashboards' })
        if (get()._fetchId === fetchId) set({ dashboards: [] })
      } finally {
        if (get()._fetchId === fetchId) set({ dashboardsLoading: false })
      }
    },

    createDashboard: async (dashboard) => {
      const local: Dashboard = { ...dashboard, id: generateId(), createdAt: Date.now(), updatedAt: Date.now() }
      set((s) => ({
        dashboards: [...s.dashboards, local],
        currentDashboardId: local.id,
        currentDashboard: local,
      }))
      const isLocal = !local.id.startsWith('dashboard_')
      if (isLocal) {
        try {
          const result = await storage.sync(local)
          if (result.data && result.data.id !== local.id) {
            const { dashboards } = get()
            const updated = dashboards.map((d: Dashboard) => (d.id === local.id ? result.data! : d)).filter(Boolean) as Dashboard[]
            set({ dashboards: updated, currentDashboard: result.data, currentDashboardId: result.data?.id })
            return result.data.id
          }
        } catch (err) { console.warn('[DashboardCrudSlice] create sync failed:', err) }
      } else {
        scheduleSync(local)
      }
      return local.id
    },

    updateDashboard: async (id, updates) => {
      const { dashboards, currentDashboardId } = get()
      const updated = { ...updates, updatedAt: Date.now() } as Partial<Dashboard>
      const { dashboards: newDashboards, currentDashboard } = updateDashboardInState(dashboards, id, updated, currentDashboardId)
      set({ dashboards: newDashboards, currentDashboard: currentDashboardId === id ? currentDashboard : get().currentDashboard })
      const target = newDashboards.find((d: Dashboard) => d.id === id)
      if (target) scheduleSync(target)
    },

    deleteDashboard: async (id) => {
      const { dashboards, currentDashboardId } = get()
      const dashboard = dashboards.find((d: Dashboard) => d.id === id)
      if (dashboard) (dashboard.components as DashboardComponent[]).forEach(cleanupAgentForComponent)
      const updated = dashboards.filter((d: Dashboard) => d.id !== id)
      const newCurrentId = currentDashboardId === id ? (updated[0]?.id || null) : currentDashboardId
      set({ dashboards: updated, currentDashboardId: newCurrentId, currentDashboard: updated.find((d: Dashboard) => d.id === newCurrentId) || null })
      await storage.delete(id)
    },

    setCurrentDashboard: async (id) => {
      await flushSync()
      const { dashboards } = get()
      const dashboard = id ? dashboards.find((d: Dashboard) => d.id === id) || null : dashboards[0] || null
      set({ currentDashboardId: id, currentDashboard: dashboard, editMode: false, selectedComponent: null })
      ;(storage as any).setCurrentDashboardId?.(id)
    },

    setDefaultDashboard: async (id) => {
      const { dashboards } = get()
      const updated = dashboards.map((d: Dashboard) => ({ ...d, isDefault: d.id === id }))
      set({ dashboards: updated })
      await storage.save(updated)
    },

    clearDashboards: () => {
      const { dashboards } = get()
      dashboards.forEach((d: Dashboard) => (d.components as DashboardComponent[]).forEach(cleanupAgentForComponent))
      storage.clear()
      set({ dashboards: [], currentDashboard: null, currentDashboardId: null })
    },

    persistDashboard: async (id) => {
      const { currentDashboard, dashboards } = get()
      const target = id ? dashboards.find((d: Dashboard) => d.id === id) : currentDashboard
      if (!target) { console.warn('[persistDashboard] no dashboard'); return }
      await storage.sync(target)
    },

    fetchTemplates: async () => {
      try {
        const { api } = await import('@/lib/api')
        const data = await api.getDashboardTemplates()
        const valid = (data || []).filter((t: { category: string }) =>
          ['overview', 'monitoring', 'automation', 'agents', 'custom'].includes(t.category),
        ) as DashboardTemplate[]
        set({ templates: [...DEFAULT_TEMPLATES, ...valid] })
      } catch {
        set({ templates: DEFAULT_TEMPLATES })
      }
    },
  }
}
