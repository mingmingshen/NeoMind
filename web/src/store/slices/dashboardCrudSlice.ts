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
  const storage: DashboardStorage = createDashboardStorage({ type: 'hybrid', cacheEnabled: true })

  // Debounced sync — captured in closure
  let syncDebounceTimer: ReturnType<typeof setTimeout> | null = null
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

  let syncVersion = 0
  let pendingSyncDashboard: Dashboard | null = null
  function scheduleSync(dashboard: Dashboard): void {
    const version = ++syncVersion
    pendingSyncDashboard = dashboard
    if (syncDebounceTimer) clearTimeout(syncDebounceTimer)
    syncDebounceTimer = setTimeout(async () => {
      syncDebounceTimer = null
      // Only sync if no newer schedule call has been made
      if (version !== syncVersion) return
      try {
        const result = await storage.sync(dashboard)
        handleIdChange(dashboard, result)
      } catch (err) {
        console.warn('[DashboardCrudSlice] sync failed:', err)
      }
    }, 500)
  }

  async function flushSync(): Promise<void> {
    if (syncDebounceTimer) {
      clearTimeout(syncDebounceTimer)
      syncDebounceTimer = null
      // Execute the pending sync immediately instead of discarding it
      const dashboard = pendingSyncDashboard
      pendingSyncDashboard = null
      if (dashboard) {
        syncVersion++
        try {
          const result = await storage.sync(dashboard)
          handleIdChange(dashboard, result)
        } catch (err) {
          console.warn('[DashboardCrudSlice] flush sync failed:', err)
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

          // Note: We intentionally do NOT filter out components with invalid data sources
          // on load. Previous behavior silently deleted components whose device/extension
          // was temporarily unavailable — this caused permanent data loss.
          // Instead, invalid data sources are handled at the UI layer (component shows
          // a "device unavailable" state) so user config is preserved.

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
      scheduleSync(local)
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
      set({
        dashboards: updated,
        currentDashboardId: newCurrentId,
        currentDashboard: updated.find((d: Dashboard) => d.id === newCurrentId) || null,
        ...(currentDashboardId === id ? {
          editMode: false,
          selectedComponent: null,
          configComponentId: null,
          configPanelOpen: false,
        } : {}),
      })
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
      const { dashboards, currentDashboardId } = get()
      const updated = dashboards.map((d: Dashboard) => ({ ...d, isDefault: d.id === id }))
      const updatedCurrent = currentDashboardId ? updated.find((d: Dashboard) => d.id === currentDashboardId) || null : null
      set({ dashboards: updated, ...(updatedCurrent ? { currentDashboard: updatedCurrent } : {}) })
      await storage.save(updated)
    },

    clearDashboards: () => {
      const { dashboards } = get()
      dashboards.forEach((d: Dashboard) => (d.components as DashboardComponent[]).forEach(cleanupAgentForComponent))
      storage.clear()
      set({
        dashboards: [],
        currentDashboard: null,
        currentDashboardId: null,
        editMode: false,
        selectedComponent: null,
        configComponentId: null,
        configPanelOpen: false,
        componentLibraryOpen: false,
        templateDialogOpen: false,
      })
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
