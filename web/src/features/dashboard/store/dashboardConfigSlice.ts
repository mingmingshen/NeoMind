/**
 * Dashboard Config Slice
 *
 * Handles component configuration updates and template application.
 * Operations modify currentDashboard components and trigger persistence.
 */

import type { StateCreator } from 'zustand'
import type {
  Dashboard,
  DashboardComponent,
  DashboardTemplate,
} from '@/types/dashboard'
import type { DashboardStore } from './index'
import { generateId } from './dashboardCrudSlice'

// ============================================================================
// Sync Helper
// ============================================================================

/** Get the scheduleSync function from the module-level window reference */
function getScheduleSync(): ((dashboard: any) => void) | null {
  if (typeof window !== 'undefined' && (window as any).__dashboardSync) {
    return (window as any).__dashboardSync.scheduleSync
  }
  return null
}

// ============================================================================
// Slice State Type
// ============================================================================

export interface DashboardConfigSlice {
  updateComponent: (
    id: string,
    updates: Partial<DashboardComponent>,
    persist?: boolean
  ) => void
  applyTemplate: (template: DashboardTemplate) => void
}

// ============================================================================
// Create Slice
// ============================================================================

export const createDashboardConfigSlice: StateCreator<
  DashboardStore,
  [],
  [],
  DashboardConfigSlice
> = (set, get) => {
  /** Helper: schedule a debounced sync for the given dashboard */
  function scheduleSync(dashboard: any): void {
    const sync = getScheduleSync()
    if (sync) {
      sync(dashboard)
    }
  }

  return {
    updateComponent(id, updates, persist = true) {
      const { currentDashboard, dashboards } = get()
      if (!currentDashboard) {
        console.error(
          '[DashboardConfigSlice] updateComponent: No current dashboard'
        )
        return
      }

      // Validate component exists
      const componentExists = currentDashboard.components.some((c) => c.id === id)
      if (!componentExists) {
        console.warn(
          '[DashboardConfigSlice] updateComponent: Component not found:',
          id
        )
        return
      }

      const updatedDashboard = {
        ...currentDashboard,
        components: currentDashboard.components.map((c) =>
          c.id === id ? { ...c, ...updates } : c
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

      // Only persist if persist=true (default for backward compatibility)
      if (persist) {
        scheduleSync(updatedDashboard)
      }
    },

    applyTemplate(template) {
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

      // Debounced sync — if user creates multiple templates rapidly, only the last syncs
      scheduleSync(newDashboard)
    },
  }
}
