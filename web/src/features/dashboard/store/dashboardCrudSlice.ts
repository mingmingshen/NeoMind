/**
 * Dashboard CRUD Slice
 *
 * Handles dashboard lifecycle operations: fetch, create, update, delete.
 * Manages persistence via DashboardStorage with debounced sync.
 */

import type { StateCreator } from 'zustand'
import type {
  Dashboard,
  DashboardComponent,
  DashboardTemplate,
  DataSource,
} from '@/types/dashboard'
import { createDashboardStorage, type DashboardStorage } from '@/store/persistence'
import { logError } from '@/lib/errors'
import type { DashboardStore } from './index'

// ============================================================================
// Agent Cleanup Helper
// ============================================================================

/** Delete the associated AI Agent when an ai-analyst component is removed */
function cleanupAgentForComponent(component: DashboardComponent | undefined) {
  if (!component || component.type !== 'ai-analyst') return
  const agentId = (component as any).config?.agentId as string | undefined
  if (!agentId) return
  import('@/lib/api').then(({ api }) => {
    api.deleteAgent(agentId).catch((err) => {
      console.warn('[DashboardCrudSlice] Failed to delete agent', agentId, err)
    })
  })
}

// ============================================================================
// Data Source Validation
// ============================================================================

/** Check if a component's data source references a valid entity */
function isDataSourceValid(
  comp: DashboardComponent,
  validDeviceIds: Set<string>,
  validExtensionIds: Set<string>
): boolean {
  const ds = ('dataSource' in comp ? comp.dataSource : undefined) as DataSource | undefined
  if (!ds) return true

  // Validate device data sources
  if (
    (ds.type === 'device' ||
      ds.type === 'telemetry' ||
      ds.type === 'metric' ||
      ds.type === 'command' ||
      ds.type === 'device-info') &&
    ds.sourceId
  ) {
    return validDeviceIds.has(ds.sourceId)
  }

  // Validate extension data sources
  if (
    (ds.type === 'extension' ||
      ds.type === 'extension-metric' ||
      ds.type === 'extension-command') &&
    ds.extensionId
  ) {
    return validExtensionIds.has(ds.extensionId)
  }

  return true
}

// ============================================================================
// Default Layout & Templates
// ============================================================================

export const DEFAULT_LAYOUT = {
  columns: 12,
  rows: 'auto' as const,
  breakpoints: {
    lg: 1200,
    md: 996,
    sm: 768,
    xs: 480,
  },
}

export const DEFAULT_TEMPLATES: DashboardTemplate[] = [
  {
    id: 'overview',
    name: 'Overview',
    description: 'System overview with devices, agents, and events',
    category: 'overview',
    icon: 'LayoutDashboard',
    layout: DEFAULT_LAYOUT,
    components: [],
    requiredResources: { devices: 1 },
  },
  {
    id: 'blank',
    name: 'Blank Canvas',
    description: 'Start from scratch with an empty dashboard',
    category: 'custom',
    icon: 'Square',
    layout: DEFAULT_LAYOUT,
    components: [],
  },
]

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Update currentDashboard and dashboards array atomically
 */
function updateDashboardInState(
  dashboards: Dashboard[],
  targetId: string,
  updates: Partial<Dashboard>,
  currentDashboardId: string | null
): { dashboards: Dashboard[]; currentDashboard: Dashboard | null } {
  const updatedDashboards = dashboards.map((d) =>
    d.id === targetId ? { ...d, ...updates } : d
  )

  const currentDashboard =
    updatedDashboards.find((d) => d.id === currentDashboardId) || null

  return { dashboards: updatedDashboards, currentDashboard }
}

/**
 * Generate unique ID
 */
export function generateId(): string {
  if (
    typeof crypto !== 'undefined' &&
    crypto.randomUUID &&
    typeof crypto.randomUUID === 'function'
  ) {
    try {
      return crypto.randomUUID()
    } catch {
      // Fall through to fallback
    }
  }
  return 'id_' + Date.now().toString(36) + '_' + Math.random().toString(36).substring(2, 15)
}

// ============================================================================
// Slice State Type
// ============================================================================

export interface DashboardCrudSlice {
  // Data
  dashboards: Dashboard[]
  currentDashboard: Dashboard | null
  currentDashboardId: string | null
  templates: DashboardTemplate[]

  // Internal: Track current fetch request to prevent race conditions
  _fetchId: number | null

  // Loading
  dashboardsLoading: boolean

  // Actions
  fetchDashboards: () => Promise<void>
  createDashboard: (
    dashboard: Omit<Dashboard, 'id' | 'createdAt' | 'updatedAt'>
  ) => Promise<string>
  updateDashboard: (id: string, updates: Partial<Dashboard>) => Promise<void>
  deleteDashboard: (id: string) => Promise<void>
  setCurrentDashboard: (id: string | null) => void
  setDefaultDashboard: (id: string) => Promise<void>
  clearDashboards: () => void
  persistDashboard: (id?: string) => Promise<void>
  fetchTemplates: () => Promise<void>
}

// ============================================================================
// Create Slice
// ============================================================================

export const createDashboardCrudSlice: StateCreator<
  DashboardStore,
  [],
  [],
  DashboardCrudSlice
> = (set, get) => {
  // Clear old localStorage dashboard data on initialization
  try {
    localStorage.removeItem('neomind_dashboards')
    localStorage.removeItem('neomind_current_dashboard_id')
  } catch {
    // Ignore
  }

  // Initialize storage - use API as primary with localStorage fallback for caching
  const storage: DashboardStorage = createDashboardStorage({
    type: 'hybrid',
    cacheEnabled: true,
  })

  // Shared debounce for background persistence to reduce write amplification
  let syncDebounceTimer: ReturnType<typeof setTimeout> | null = null
  let pendingSyncDashboard: Dashboard | null = null

  /**
   * Schedule a debounced sync (500ms trailing). Only the latest dashboard state is synced,
   * which reduces write amplification during rapid operations like component updates.
   * Centralizes ID-change handling to eliminate boilerplate.
   */
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
        console.warn('[DashboardCrudSlice] Background sync failed:', err)
      }
    }, 500)
  }

  /** Handle server-assigned ID change in sync result */
  function handleIdChange(
    dash: Dashboard,
    result: { data: Dashboard | null }
  ): void {
    if (result.data && result.data.id !== dash.id) {
      const { dashboards: currentDashboards } = get()
      const activeDashboardId = get().currentDashboardId
      const newDashboards = currentDashboards.map((d) =>
        d.id === dash.id ? result.data! : d
      )
      set({
        dashboards: newDashboards,
        // Only update currentDashboard/Id if the user hasn't switched away
        ...(activeDashboardId === dash.id
          ? { currentDashboard: result.data, currentDashboardId: result.data.id }
          : {}),
      })
    }
  }

  /** Flush any pending debounced sync immediately */
  function flushSync(): void {
    if (syncDebounceTimer) {
      clearTimeout(syncDebounceTimer)
      syncDebounceTimer = null
      const dash = pendingSyncDashboard
      pendingSyncDashboard = null
      if (dash) {
        storage
          .sync(dash)
          .then((result) => {
            handleIdChange(dash, result)
          })
          .catch((err) => {
            console.warn('[DashboardCrudSlice] flushSync failed:', err)
          })
      }
    }
  }

  // Expose scheduleSync and flushSync on the storage object so other slices can access them
  // We use a module-level export pattern via closure instead
  ;(scheduleSync as any).__storage = storage
  ;(scheduleSync as any).__flushSync = flushSync

  // Attach to window for cross-slice access in this module
  if (typeof window !== 'undefined') {
    ;(window as any).__dashboardSync = { scheduleSync, flushSync, storage }
  }

  return {
    // Initial state
    dashboards: [],
    currentDashboard: null,
    currentDashboardId: null,
    templates: DEFAULT_TEMPLATES,
    _fetchId: null,
    dashboardsLoading: false,

    // ========================================================================
    // Dashboard CRUD
    // ========================================================================

    fetchDashboards: async () => {
      // Generate unique ID for this fetch request
      const fetchId = Date.now()
      set({ _fetchId: fetchId, dashboardsLoading: true })

      try {
        const result = await storage.load()

        // Check if this is still the latest fetch request
        const currentState = get()
        if (currentState._fetchId !== fetchId) {
          return
        }

        if (result.error) {
          console.warn(
            '[DashboardCrudSlice] Failed to load dashboards:',
            result.error.message
          )
          set({ dashboards: [], dashboardsLoading: false })
        } else if (result.data) {
          // Migration: Move dataSource from config to component level
          const migratedDashboards = result.data.map((dashboard: Dashboard) => {
            return {
              ...dashboard,
              components: dashboard.components.map((component: any) => {
                const config = component.config || {}
                const configDataSource = config.dataSource

                if (configDataSource) {
                  const { dataSource, ...configWithoutDataSource } = config
                  return {
                    ...component,
                    config: configWithoutDataSource,
                    dataSource: configDataSource,
                  }
                }

                return component
              }),
            }
          })

          // Validate data sources: remove components referencing deleted devices/extensions
          try {
            const storeState = get() as any
            const validDeviceIds = new Set<string>(
              (storeState.devices || [])
                .map((d: any) => d.id || d.device_id)
                .filter(Boolean)
            )
            const validExtensionIds = new Set<string>(
              (storeState.extensions || [])
                .map((e: any) => e.id || e.extension_id)
                .filter(Boolean)
            )

            if (validDeviceIds.size > 0 || validExtensionIds.size > 0) {
              for (const dashboard of migratedDashboards) {
                dashboard.components = dashboard.components.filter(
                  (comp: DashboardComponent) =>
                    isDataSourceValid(comp, validDeviceIds, validExtensionIds)
                )
              }
            }
          } catch {
            // Validation is best-effort, don't block dashboard loading
          }

          // Final check before updating state
          const finalState = get()
          if (finalState._fetchId !== fetchId) {
            return
          }

          set({ dashboards: migratedDashboards })

          // Set current dashboard if not set
          const savedId = (storage as any).getCurrentDashboardId?.()

          let dashboardToSet: Dashboard | null = null

          if (savedId && migratedDashboards.length > 0) {
            const savedDashboard = migratedDashboards.find((d) => d.id === savedId)
            if (savedDashboard) {
              dashboardToSet = savedDashboard
            }
          }

          if (
            !dashboardToSet &&
            !currentState.currentDashboardId &&
            migratedDashboards.length > 0
          ) {
            dashboardToSet =
              migratedDashboards.find((d) => d.isDefault) || migratedDashboards[0]
          }

          if (dashboardToSet && !currentState.currentDashboard) {
            set({
              currentDashboardId: dashboardToSet.id,
              currentDashboard: dashboardToSet,
            })
          }
        } else {
          set({ dashboards: [] })
        }
      } catch (err) {
        logError(err, { operation: 'Load dashboards' })
        const currentState = get()
        if (currentState._fetchId === fetchId) {
          set({ dashboards: [] })
        }
      } finally {
        const currentState = get()
        if (currentState._fetchId === fetchId) {
          set({ dashboardsLoading: false })
        }
      }
    },

    createDashboard: async (dashboard) => {
      const localDashboard: Dashboard = {
        ...dashboard,
        id: generateId(),
        createdAt: Date.now(),
        updatedAt: Date.now(),
      }

      set((state) => ({
        dashboards: [...state.dashboards, localDashboard],
        currentDashboardId: localDashboard.id,
        currentDashboard: localDashboard,
      }))

      // For local dashboards, we wait for API to get the server ID
      const isLocalDashboard = !localDashboard.id.startsWith('dashboard_')
      if (isLocalDashboard) {
        try {
          const result = await storage.sync(localDashboard)
          if (result.data && result.data.id !== localDashboard.id) {
            const { dashboards: currentDashboards, currentDashboardId: activeId } = get()
            const newDashboards = currentDashboards
              .map((d) => (d.id === localDashboard.id ? result.data : d))
              .filter((d): d is Dashboard => d !== null)
            // Only update currentDashboard if user hasn't switched away
            const isStillActive = activeId === localDashboard.id
            set({
              dashboards: newDashboards,
              ...(isStillActive ? {
                currentDashboard: result.data,
                currentDashboardId: result.data?.id,
              } : {}),
            })
            return isStillActive ? result.data.id : localDashboard.id
          }
        } catch (err) {
          console.warn('[DashboardCrudSlice] Background sync failed:', err)
        }
      } else {
        scheduleSync(localDashboard)
      }

      return localDashboard.id
    },

    updateDashboard: async (id, updates) => {
      const { dashboards, currentDashboardId } = get()

      const updated = {
        ...updates,
        updatedAt: Date.now(),
      } as Partial<Dashboard>

      const { dashboards: updatedDashboards, currentDashboard } =
        updateDashboardInState(dashboards, id, updated, currentDashboardId)

      set({
        dashboards: updatedDashboards,
        currentDashboard:
          currentDashboardId === id ? currentDashboard : get().currentDashboard,
      })

      // Debounced sync
      const targetDashboard = updatedDashboards.find((d) => d.id === id)
      if (targetDashboard) {
        scheduleSync(targetDashboard)
      }
    },

    deleteDashboard: async (id) => {
      const { dashboards, currentDashboardId } = get()

      // Clean up AI agents for all ai-analyst components in the deleted dashboard
      const dashboard = dashboards.find((d) => d.id === id)
      if (dashboard) {
        dashboard.components.forEach((comp) => cleanupAgentForComponent(comp))
      }

      const updated = dashboards.filter((d) => d.id !== id)
      const newCurrentId =
        currentDashboardId === id ? updated[0]?.id || null : currentDashboardId

      set({
        dashboards: updated,
        currentDashboardId: newCurrentId,
        currentDashboard: updated.find((d) => d.id === newCurrentId) || null,
      })

      await storage.delete(id)
    },

    setCurrentDashboard: (id) => {
      const { dashboards } = get()
      const dashboard = id
        ? dashboards.find((d) => d.id === id) || null
        : dashboards[0] || null

      // Flush pending debounced sync before switching dashboards
      flushSync()

      set({
        currentDashboardId: id,
        currentDashboard: dashboard,
        editMode: false,
        selectedComponent: null,
      })

      // Save to localStorage
      ;(storage as any).setCurrentDashboardId?.(id)
    },

    setDefaultDashboard: async (id) => {
      const { dashboards } = get()

      const updated = dashboards.map((d) => ({
        ...d,
        isDefault: d.id === id,
      }))

      set({ dashboards: updated })

      await storage.save(updated)
    },

    clearDashboards: () => {
      // Clean up AI agents for all ai-analyst components across all dashboards
      const { dashboards } = get()
      dashboards.forEach((d) => {
        d.components.forEach((comp) => cleanupAgentForComponent(comp))
      })

      storage.clear()
      set({
        dashboards: [],
        currentDashboard: null,
        currentDashboardId: null,
      })
    },

    persistDashboard: async (id) => {
      const { currentDashboard, dashboards } = get()
      const dashboardToPersist = id
        ? dashboards.find((d) => d.id === id)
        : currentDashboard

      if (!dashboardToPersist) {
        console.warn('[persistDashboard] No dashboard to persist')
        return
      }

      await storage.sync(dashboardToPersist)
    },

    fetchTemplates: async () => {
      try {
        const api = (await import('@/lib/api')).api
        const data = await api.getDashboardTemplates()
        const validTemplates = (data || []).filter((t: any) =>
          ['overview', 'monitoring', 'automation', 'agents', 'custom'].includes(
            t.category
          )
        ) as DashboardTemplate[]
        set({ templates: [...DEFAULT_TEMPLATES, ...validTemplates] })
      } catch {
        set({ templates: DEFAULT_TEMPLATES })
      }
    },
  }
}
