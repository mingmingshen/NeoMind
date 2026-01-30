/**
 * Dashboard Slice (Refactored)
 *
 * Simplified state management using the persistence layer.
 * Removes code duplication and separates concerns.
 */

import type { StateCreator } from 'zustand'
import type {
  Dashboard,
  DashboardComponent,
  DashboardTemplate,
  ComponentPosition,
  DashboardLayout,
} from '@/types/dashboard'
import { createDashboardStorage, type DashboardStorage } from '../persistence'

// ============================================================================
// Default Layout
// ============================================================================

export const DEFAULT_LAYOUT: DashboardLayout = {
  columns: 12,
  rows: 'auto',
  breakpoints: {
    lg: 1200,
    md: 996,
    sm: 768,
    xs: 480,
  },
}

// ============================================================================
// Dashboard State
// ============================================================================

export interface DashboardState {
  // Data
  dashboards: Dashboard[]
  currentDashboard: Dashboard | null
  currentDashboardId: string | null
  templates: DashboardTemplate[]

  // Internal: Track current fetch request to prevent race conditions
  _fetchId: number | null

  // UI State
  dashboardsLoading: boolean
  editMode: boolean
  selectedComponent: string | null

  // Panels
  componentLibraryOpen: boolean
  configPanelOpen: boolean
  configComponentId: string | null
  templateDialogOpen: boolean

  // Actions
  // Dashboard management
  fetchDashboards: () => Promise<void>
  createDashboard: (dashboard: Omit<Dashboard, 'id' | 'createdAt' | 'updatedAt'>) => Promise<string>
  updateDashboard: (id: string, updates: Partial<Dashboard>) => Promise<void>
  deleteDashboard: (id: string) => Promise<void>
  setCurrentDashboard: (id: string | null) => void
  setDefaultDashboard: (id: string) => Promise<void>
  clearDashboards: () => void

  // Component management
  addComponent: (component: Omit<DashboardComponent, 'id'>) => void
  updateComponent: (id: string, updates: Partial<DashboardComponent>, persist?: boolean) => void
  removeComponent: (id: string) => void
  moveComponent: (id: string, position: ComponentPosition) => void
  duplicateComponent: (id: string) => void

  // Persistence
  persistDashboard: (id?: string) => Promise<void>

  // UI state
  setEditMode: (edit: boolean) => void
  setSelectedComponent: (id: string | null) => void
  setComponentLibraryOpen: (open: boolean) => void
  setConfigPanelOpen: (open: boolean, componentId?: string) => void
  setTemplateDialogOpen: (open: boolean) => void

  // Templates
  applyTemplate: (template: DashboardTemplate) => void
  fetchTemplates: () => Promise<void>
}

// ============================================================================
// Default Templates
// ============================================================================

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
  targetId: string,  // ID of the dashboard to update
  updates: Partial<Dashboard>,
  currentDashboardId: string | null  // Current active dashboard ID
): { dashboards: Dashboard[]; currentDashboard: Dashboard | null } {
  const updatedDashboards = dashboards.map((d) =>
    d.id === targetId ? { ...d, ...updates } : d
  )

  // If the updated dashboard is the current one, update currentDashboard too
  const currentDashboard = updatedDashboards.find((d) => d.id === currentDashboardId) || null

  return { dashboards: updatedDashboards, currentDashboard }
}

/**
 * Generate unique ID
 */
function generateId(): string {
  return crypto.randomUUID()
}

/**
 * Deep clone utility using structuredClone with fallback
 * Preserves more types than JSON.stringify (Date, Map, Set, etc.)
 */
function deepClone<T>(obj: T): T {
  if (typeof structuredClone !== 'undefined') {
    try {
      return structuredClone(obj)
    } catch {
      // Fallback for circular references or unsupported types
    }
  }
  // Fallback to JSON method for older browsers
  return JSON.parse(JSON.stringify(obj))
}

// ============================================================================
// Create Slice
// ============================================================================

export const createDashboardSlice: StateCreator<
  DashboardState,
  [],
  [],
  DashboardState
> = (set, get) => {
  // Clear old localStorage dashboard data on initialization
  try {
    localStorage.removeItem('neotalk_dashboards')
    localStorage.removeItem('neotalk_current_dashboard_id')
    console.log('[DashboardSlice] Cleared old localStorage dashboard data')
  } catch (e) {
    // Ignore
  }

  // Initialize storage - use API as primary with localStorage fallback for caching
  const storage: DashboardStorage = createDashboardStorage({ type: 'hybrid', cacheEnabled: true })

  return {
    // Initial state
    dashboards: [],
    currentDashboard: null,
    currentDashboardId: null,
    templates: DEFAULT_TEMPLATES,
    _fetchId: null,

    dashboardsLoading: false,
    editMode: false,
    selectedComponent: null,

    componentLibraryOpen: false,
    configPanelOpen: false,
    configComponentId: null,
    templateDialogOpen: false,

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
          // A newer fetch request has been initiated, discard this result
          console.log('[DashboardSlice] Discarding stale fetch result')
          return
        }

        if (result.error) {
          console.warn('[DashboardSlice] Failed to load dashboards:', result.error.message)
          // Set empty array when loading fails
          set({ dashboards: [], dashboardsLoading: false })
        } else if (result.data) {
          // Migration: Move dataSource from config to component level
          // This handles legacy data where dataSource was stored inside config
          const migratedDashboards = result.data.map((dashboard: Dashboard) => {
            return {
              ...dashboard,
              components: dashboard.components.map((component: any) => {
                const config = component.config || {}
                const configDataSource = config.dataSource

                // If dataSource is in config but not at component level, migrate it
                if (configDataSource) {
                  // Remove from config and add to component level
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

          // Final check before updating state
          const finalState = get()
          if (finalState._fetchId !== fetchId) {
            console.log('[DashboardSlice] Discarding stale fetch result (final check)')
            return
          }

          set({ dashboards: migratedDashboards })

          // Set current dashboard if not set
          const savedId = (storage as any).getCurrentDashboardId?.()

          // Determine which dashboard to set as current - use MIGRATED data, not original
          let dashboardToSet: Dashboard | null = null

          if (savedId && migratedDashboards.length > 0) {
            // First try to use the saved ID from localStorage - use MIGRATED dashboards
            const savedDashboard = migratedDashboards.find((d) => d.id === savedId)
            if (savedDashboard) {
              dashboardToSet = savedDashboard
            }
          }

          // If no saved dashboard or saved ID not found, use default or first - use MIGRATED dashboards
          if (!dashboardToSet && !currentState.currentDashboardId && migratedDashboards.length > 0) {
            dashboardToSet = migratedDashboards.find((d) => d.isDefault) || migratedDashboards[0]
          }

          // Only update if we found a dashboard and don't already have one set
          if (dashboardToSet && !currentState.currentDashboard) {
            set({
              currentDashboardId: dashboardToSet.id,
              currentDashboard: dashboardToSet,
            })
            console.log('[DashboardSlice] Set current dashboard:', dashboardToSet.name, 'from', result.source)
          }
        } else {
          // No data and no error - treat as empty
          set({ dashboards: [] })
        }
      } catch (err) {
        console.error('[DashboardSlice] Unexpected error loading dashboards:', err)
        // Only update state if this is still the latest fetch
        const currentState = get()
        if (currentState._fetchId === fetchId) {
          // Set empty array on unexpected errors
          set({ dashboards: [] })
        }
      } finally {
        // Only clear loading if this is still the latest fetch
        const currentState = get()
        if (currentState._fetchId === fetchId) {
          set({ dashboardsLoading: false })
        }
      }
    },

    createDashboard: async (dashboard) => {
      // Always create locally first (fast, reliable)
      const localDashboard: Dashboard = {
        ...dashboard,
        id: generateId(),
        createdAt: Date.now(),
        updatedAt: Date.now(),
      }

      // Update state immediately
      set((state) => ({
        dashboards: [...state.dashboards, localDashboard],
        currentDashboardId: localDashboard.id,
        currentDashboard: localDashboard,
      }))

      // Persist to storage (localStorage + try API in background)
      storage.sync(localDashboard).catch((err) => {
        // Sync failed - but dashboard is already saved locally
        console.warn('[DashboardSlice] Background sync failed:', err)
      })

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

      // Update state immediately
      set({
        dashboards: updatedDashboards,
        currentDashboard: currentDashboardId === id ? currentDashboard : get().currentDashboard,
      })

      // Persist to storage in background
      const targetDashboard = updatedDashboards.find(d => d.id === id)
      if (targetDashboard) {
        storage.sync(targetDashboard).catch((err) => {
          // Sync failed - but state is already saved locally
          console.warn('[DashboardSlice] Background sync failed:', err)
        })
      }
    },

    deleteDashboard: async (id) => {
      const { dashboards, currentDashboardId } = get()

      const updated = dashboards.filter((d) => d.id !== id)
      const newCurrentId = currentDashboardId === id ? (updated[0]?.id || null) : currentDashboardId

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
      // Clear storage and reset state
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

    addComponent(component) {
      const { currentDashboard, dashboards, currentDashboardId } = get()
      if (!currentDashboard) return

      const newComponent = { ...component, id: generateId() }
      const updatedDashboard = {
        ...currentDashboard,
        components: [...currentDashboard.components, newComponent],
        updatedAt: Date.now(),
      }

      const updatedDashboards = dashboards.map((d) =>
        d.id === currentDashboard.id ? updatedDashboard : d
      )

      set({
        dashboards: updatedDashboards,
        currentDashboard: updatedDashboard,
      })

      storage.sync(updatedDashboard).catch(() => {})
    },

    updateComponent(id, updates, persist = true) {
      const { currentDashboard, dashboards } = get()
      if (!currentDashboard) return

      console.log('[DashboardSlice] updateComponent called:', { id, updates })
      console.log('[DashboardSlice] currentDashboard.components before:', currentDashboard.components.find(c => c.id === id))

      const updatedDashboard = {
        ...currentDashboard,
        components: currentDashboard.components.map((c) =>
          c.id === id ? { ...c, ...updates } : c
        ),
        updatedAt: Date.now(),
      }

      const updatedComponent = updatedDashboard.components.find(c => c.id === id)
      console.log('[DashboardSlice] updatedDashboard.components after:', updatedComponent)

      const updatedDashboards = dashboards.map((d) =>
        d.id === currentDashboard.id ? updatedDashboard : d
      )

      set({
        dashboards: updatedDashboards,
        currentDashboard: updatedDashboard,
      })

      // Only persist to localStorage if persist=true (default for backward compatibility)
      if (persist) {
        storage.sync(updatedDashboard).catch(() => {})
      }
    },

    removeComponent(id) {
      const { currentDashboard, dashboards, selectedComponent, configComponentId } = get()
      if (!currentDashboard) return

      const updatedDashboard = {
        ...currentDashboard,
        components: currentDashboard.components.filter((c) => c.id !== id),
        updatedAt: Date.now(),
      }

      const updatedDashboards = dashboards.map((d) =>
        d.id === currentDashboard.id ? updatedDashboard : d
      )

      // Clear selection if the deleted component was selected
      const newSelectedComponent = selectedComponent === id ? null : selectedComponent
      const newConfigComponentId = configComponentId === id ? null : configComponentId

      set({
        dashboards: updatedDashboards,
        currentDashboard: updatedDashboard,
        selectedComponent: newSelectedComponent,
        configComponentId: newConfigComponentId,
      })

      storage.sync(updatedDashboard).catch(() => {})
    },

    moveComponent(id, position) {
      const { currentDashboard, dashboards } = get()
      if (!currentDashboard) return

      const updatedDashboard = {
        ...currentDashboard,
        components: currentDashboard.components.map((c) =>
          c.id === id
            ? { ...c, position: { ...c.position, ...position } }
            : c
        ),
        updatedAt: Date.now(),
      }

      const updatedDashboards = dashboards.map((d) =>
        d.id === currentDashboard.id ? updatedDashboard : d
      )

      set({
        dashboards: updatedDashboards,
        currentDashboard: updatedDashboard,
      })

      storage.sync(updatedDashboard).catch(() => {})
    },

    duplicateComponent(id) {
      const { currentDashboard, dashboards } = get()
      if (!currentDashboard) return

      const original = currentDashboard.components.find((c) => c.id === id)
      if (!original) return

      const newComponent = {
        ...deepClone(original),
        id: generateId(),
        position: {
          ...original.position,
          x: original.position.x + original.position.w,
        },
      } as DashboardComponent

      const updatedDashboard = {
        ...currentDashboard,
        components: [...currentDashboard.components, newComponent],
        updatedAt: Date.now(),
      }

      const updatedDashboards = dashboards.map((d) =>
        d.id === currentDashboard.id ? updatedDashboard : d
      )

      set({
        dashboards: updatedDashboards,
        currentDashboard: updatedDashboard,
      })

      storage.sync(updatedDashboard).catch(() => {})
    },

    // ========================================================================
    // UI State
    // ========================================================================

    setEditMode: (edit) => set({ editMode: edit, selectedComponent: null }),

    setSelectedComponent: (id) => {
      const { currentDashboard } = get()
      // Validate that the component exists before selecting it
      if (id && currentDashboard?.components) {
        const componentExists = currentDashboard.components.some((c) => c.id === id)
        if (!componentExists) {
          // Component no longer exists, clear selection
          set({ selectedComponent: null })
          return
        }
      }
      set({ selectedComponent: id })
    },

    setComponentLibraryOpen: (open) => set({ componentLibraryOpen: open }),

    setConfigPanelOpen: (open, componentId) => {
      const { currentDashboard } = get()
      // Validate that the component exists before opening config
      if (open && componentId && currentDashboard?.components) {
        const componentExists = currentDashboard.components.some((c) => c.id === componentId)
        if (!componentExists) {
          // Component no longer exists, don't open config panel
          set({ configPanelOpen: false, configComponentId: null })
          return
        }
      }
      set({
        configPanelOpen: open,
        configComponentId: componentId || null,
      })
    },

    setTemplateDialogOpen: (open) => set({ templateDialogOpen: open }),

    // ========================================================================
    // Templates
    // ========================================================================

    fetchTemplates: async () => {
      try {
        const api = (await import('@/lib/api')).api
        const data = await api.getDashboardTemplates()
        const validTemplates = data.filter((t: any) =>
          ['overview', 'monitoring', 'automation', 'agents', 'custom'].includes(t.category)
        ) as DashboardTemplate[]
        set({ templates: [...DEFAULT_TEMPLATES, ...validTemplates] })
      } catch {
        set({ templates: DEFAULT_TEMPLATES })
      }
    },

    applyTemplate: (template) => {
      const newDashboard: Dashboard = {
        id: generateId(),
        name: template.name,
        layout: template.layout,
        components: template.components.map((c) => ({
          ...c,
          id: generateId(),
        })) as DashboardComponent[],
        createdAt: Date.now(),
        updatedAt: Date.now(),
      }

      set((state) => ({
        dashboards: [...state.dashboards, newDashboard],
        currentDashboardId: newDashboard.id,
        currentDashboard: newDashboard,
      }))

      storage.sync(newDashboard).catch(() => {})
    },
  }
}
