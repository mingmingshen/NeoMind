/**
 * Dashboard Layout Slice
 *
 * Handles component layout operations: add, move, batch update positions,
 * remove, duplicate. All operations modify currentDashboard.components
 * and trigger debounced persistence via the crud slice's scheduleSync.
 */

import type { StateCreator } from 'zustand'
import type {
  DashboardComponent,
  ComponentPosition,
} from '@/types/dashboard'
import type { DashboardStore } from './index'
import { generateId } from './dashboardCrudSlice'

// ============================================================================
// Agent Cleanup Helper (duplicated to avoid circular dependency)
// ============================================================================

/** Delete the associated AI Agent when an ai-analyst component is removed */
function cleanupAgentForComponent(component: DashboardComponent | undefined) {
  if (!component || component.type !== 'ai-analyst') return
  const agentId = (component as any).config?.agentId as string | undefined
  if (!agentId) return
  import('@/lib/api').then(({ api }) => {
    api.deleteAgent(agentId).catch((err) => {
      console.warn('[DashboardLayoutSlice] Failed to delete agent', agentId, err)
    })
  })
}

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

export interface DashboardLayoutSlice {
  addComponent: (component: Omit<DashboardComponent, 'id'>) => void
  moveComponent: (id: string, position: ComponentPosition) => void
  batchUpdatePositions: (
    positions: Array<{ id: string; position: ComponentPosition }>
  ) => void
  removeComponent: (id: string) => void
  removeComponentsByExtension: (extensionId: string) => void
  removeComponentsByDevice: (deviceId: string) => void
  duplicateComponent: (id: string) => void
}

// ============================================================================
// Create Slice
// ============================================================================

export const createDashboardLayoutSlice: StateCreator<
  DashboardStore,
  [],
  [],
  DashboardLayoutSlice
> = (set, get) => {
  /** Helper: schedule a debounced sync for the given dashboard */
  function scheduleSync(dashboard: any): void {
    const sync = getScheduleSync()
    if (sync) {
      sync(dashboard)
    }
  }

  return {
    addComponent(component) {
      const { currentDashboard, dashboards } = get()
      if (!currentDashboard) {
        console.error('[DashboardLayoutSlice] addComponent: No current dashboard')
        return
      }

      const newComponent = { ...component, id: generateId() } as DashboardComponent
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

      scheduleSync(updatedDashboard)
    },

    moveComponent(id, position) {
      const { currentDashboard, dashboards } = get()
      if (!currentDashboard) {
        console.error('[DashboardLayoutSlice] moveComponent: No current dashboard')
        return
      }

      const componentExists = currentDashboard.components.some((c) => c.id === id)
      if (!componentExists) {
        console.warn('[DashboardLayoutSlice] moveComponent: Component not found:', id)
        return
      }

      const updatedDashboard = {
        ...currentDashboard,
        components: currentDashboard.components.map((c) =>
          c.id === id ? { ...c, position: { ...c.position, ...position } } : c
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

      // Use shared debounced sync (500ms trailing) to coalesce rapid drag events
      scheduleSync(updatedDashboard)
    },

    batchUpdatePositions(positions) {
      const { currentDashboard, dashboards } = get()
      if (!currentDashboard || positions.length === 0) return

      const posMap = new Map(positions.map((p) => [p.id, p.position]))
      const updatedComponents = currentDashboard.components.map((c) => {
        const newPos = posMap.get(c.id)
        return newPos ? { ...c, position: newPos } : c
      })

      const updatedDashboard = {
        ...currentDashboard,
        components: updatedComponents,
        updatedAt: Date.now(),
      }

      const updatedDashboards = dashboards.map((d) =>
        d.id === currentDashboard.id ? updatedDashboard : d
      )

      set({
        dashboards: updatedDashboards,
        currentDashboard: updatedDashboard,
      })

      scheduleSync(updatedDashboard)
    },

    removeComponent(id) {
      const { currentDashboard, dashboards, selectedComponent, configComponentId } =
        get()
      if (!currentDashboard) return

      // Clean up AI Analyst agent before removing the component
      const removed = currentDashboard.components.find((c) => c.id === id)
      cleanupAgentForComponent(removed)

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
      const newConfigComponentId =
        configComponentId === id ? null : configComponentId

      set({
        dashboards: updatedDashboards,
        currentDashboard: updatedDashboard,
        selectedComponent: newSelectedComponent,
        configComponentId: newConfigComponentId,
      })

      scheduleSync(updatedDashboard)
    },

    removeComponentsByExtension(extensionId: string) {
      const { currentDashboard, dashboards, selectedComponent, configComponentId } =
        get()
      if (!currentDashboard) return

      // Find all components that belong to this extension
      const componentsToRemove = currentDashboard.components.filter((comp) => {
        const dataSource = 'dataSource' in comp ? (comp.dataSource as any) : undefined
        return (
          comp.type.startsWith(`${extensionId}:`) ||
          comp.type.includes(`-${extensionId}-`) ||
          dataSource?.extensionId === extensionId ||
          dataSource?.extension_id === extensionId
        )
      })

      if (componentsToRemove.length === 0) return

      // Clean up AI agents for any ai-analyst components being removed
      componentsToRemove.forEach((comp) => cleanupAgentForComponent(comp))

      const componentIdsToRemove = new Set(componentsToRemove.map((c) => c.id))

      const updatedDashboard = {
        ...currentDashboard,
        components: currentDashboard.components.filter(
          (c) => !componentIdsToRemove.has(c.id)
        ),
        updatedAt: Date.now(),
      }

      const updatedDashboards = dashboards.map((d) =>
        d.id === currentDashboard.id ? updatedDashboard : d
      )

      const newSelectedComponent =
        selectedComponent && componentIdsToRemove.has(selectedComponent)
          ? null
          : selectedComponent
      const newConfigComponentId =
        configComponentId && componentIdsToRemove.has(configComponentId)
          ? null
          : configComponentId

      set({
        dashboards: updatedDashboards,
        currentDashboard: updatedDashboard,
        selectedComponent: newSelectedComponent,
        configComponentId: newConfigComponentId,
      })

      scheduleSync(updatedDashboard)
    },

    removeComponentsByDevice(deviceId: string) {
      const { currentDashboard, dashboards, selectedComponent, configComponentId } =
        get()
      if (!currentDashboard) return

      // Find all components that reference this device
      const componentsToRemove = currentDashboard.components.filter((comp) => {
        const dataSource = 'dataSource' in comp ? (comp.dataSource as any) : undefined
        return (
          dataSource?.sourceId === deviceId ||
          (dataSource?.type === 'device' &&
            dataSource?.property &&
            dataSource.sourceId === deviceId)
        )
      })

      if (componentsToRemove.length === 0) return

      // Clean up AI agents for any ai-analyst components being removed
      componentsToRemove.forEach((comp) => cleanupAgentForComponent(comp))

      const componentIdsToRemove = new Set(componentsToRemove.map((c) => c.id))

      const updatedDashboard = {
        ...currentDashboard,
        components: currentDashboard.components.filter(
          (c) => !componentIdsToRemove.has(c.id)
        ),
        updatedAt: Date.now(),
      }

      const updatedDashboards = dashboards.map((d) =>
        d.id === currentDashboard.id ? updatedDashboard : d
      )

      const newSelectedComponent =
        selectedComponent && componentIdsToRemove.has(selectedComponent)
          ? null
          : selectedComponent
      const newConfigComponentId =
        configComponentId && componentIdsToRemove.has(configComponentId)
          ? null
          : configComponentId

      set({
        dashboards: updatedDashboards,
        currentDashboard: updatedDashboard,
        selectedComponent: newSelectedComponent,
        configComponentId: newConfigComponentId,
      })

      scheduleSync(updatedDashboard)
    },

    duplicateComponent(id) {
      const { currentDashboard, dashboards } = get()
      if (!currentDashboard) return

      const original = currentDashboard.components.find((c) => c.id === id)
      if (!original) return

      const newComponent = {
        ...structuredClone(original),
        id: generateId(),
        position: {
          ...original.position,
          y: original.position.y + original.position.h,
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

      scheduleSync(updatedDashboard)
    },
  }
}
