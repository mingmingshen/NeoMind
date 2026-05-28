/**
 * Dashboard Layout Slice
 *
 * Component layout operations: add, move, batchUpdatePositions,
 * remove, duplicate. Accesses currentDashboard via get().
 * Uses scheduleSync from DashboardCrudSlice via get().
 */

import type { StateCreator } from 'zustand'
import type {
  DashboardComponent,
  ComponentPosition,
  DataSource,
} from '@/types/dashboard'
import {
  generateId,
  cleanupAgentForComponent,
} from './dashboardHelpers'
import type { DashboardStore } from './dashboardHelpers'

export interface DashboardLayoutSlice {
  addComponent: (component: Omit<DashboardComponent, 'id'>) => void
  moveComponent: (id: string, position: ComponentPosition) => void
  batchUpdatePositions: (positions: Array<{ id: string; position: ComponentPosition }>) => void
  removeComponent: (id: string) => void
  removeComponentsByExtension: (extensionId: string) => void
  removeComponentsByDevice: (deviceId: string) => void
  duplicateComponent: (id: string) => void
}

export const createDashboardLayoutSlice: StateCreator<
  DashboardStore, [], [], DashboardLayoutSlice
> = (set, get) => ({
  addComponent(component) {
    const { currentDashboard, dashboards } = get()
    if (!currentDashboard) return
    const newComponent = { ...component, id: generateId() } as DashboardComponent
    const updatedDashboard = {
      ...currentDashboard,
      components: [...currentDashboard.components, newComponent],
      updatedAt: Date.now(),
    }
    const updatedDashboards = dashboards.map((d) =>
      d.id === currentDashboard.id ? updatedDashboard : d,
    )
    set({ dashboards: updatedDashboards, currentDashboard: updatedDashboard })
    get().scheduleSync(updatedDashboard)
  },

  moveComponent(id, position) {
    const { currentDashboard, dashboards } = get()
    if (!currentDashboard) return
    if (!currentDashboard.components.some((c) => c.id === id)) return
    const updatedDashboard = {
      ...currentDashboard,
      components: currentDashboard.components.map((c) =>
        c.id === id ? { ...c, position: { ...c.position, ...position } } : c,
      ),
      updatedAt: Date.now(),
    }
    const updatedDashboards = dashboards.map((d) =>
      d.id === currentDashboard.id ? updatedDashboard : d,
    )
    set({ dashboards: updatedDashboards, currentDashboard: updatedDashboard })
    get().scheduleSync(updatedDashboard)
  },

  batchUpdatePositions(positions) {
    const { currentDashboard, dashboards } = get()
    if (!currentDashboard || positions.length === 0) return
    const posMap = new Map(positions.map((p) => [p.id, p.position]))
    const updatedComponents = currentDashboard.components.map((c) => {
      const newPos = posMap.get(c.id)
      return newPos ? { ...c, position: newPos } : c
    })
    const updatedDashboard = { ...currentDashboard, components: updatedComponents, updatedAt: Date.now() }
    const updatedDashboards = dashboards.map((d) =>
      d.id === currentDashboard.id ? updatedDashboard : d,
    )
    set({ dashboards: updatedDashboards, currentDashboard: updatedDashboard })
    get().scheduleSync(updatedDashboard)
  },

  removeComponent(id) {
    const { currentDashboard, dashboards, selectedComponent, configComponentId } = get()
    if (!currentDashboard) return
    const removed = currentDashboard.components.find((c) => c.id === id)
    cleanupAgentForComponent(removed)
    const updatedDashboard = {
      ...currentDashboard,
      components: currentDashboard.components.filter((c) => c.id !== id),
      updatedAt: Date.now(),
    }
    const updatedDashboards = dashboards.map((d) =>
      d.id === currentDashboard.id ? updatedDashboard : d,
    )
    set({
      dashboards: updatedDashboards,
      currentDashboard: updatedDashboard,
      selectedComponent: selectedComponent === id ? null : selectedComponent,
      configComponentId: configComponentId === id ? null : configComponentId,
    })
    get().scheduleSync(updatedDashboard)
  },

  removeComponentsByExtension(extensionId) {
    const { currentDashboard, dashboards, selectedComponent, configComponentId } = get()
    if (!currentDashboard) return
    const toRemove = currentDashboard.components.filter((comp) => {
      if (comp.type.startsWith(`${extensionId}:`) || comp.type.includes(`-${extensionId}-`)) return true
      const ds = comp.dataSource
      if (!ds) return false
      const sources: DataSource[] = Array.isArray(ds) ? ds : [ds]
      return sources.some((s) => s.extensionId === extensionId)
    })
    if (toRemove.length === 0) return
    toRemove.forEach(cleanupAgentForComponent)
    const idsToRemove = new Set(toRemove.map((c) => c.id))
    const updatedDashboard = {
      ...currentDashboard,
      components: currentDashboard.components.filter((c) => !idsToRemove.has(c.id)),
      updatedAt: Date.now(),
    }
    const updatedDashboards = dashboards.map((d) =>
      d.id === currentDashboard.id ? updatedDashboard : d,
    )
    set({
      dashboards: updatedDashboards,
      currentDashboard: updatedDashboard,
      selectedComponent: selectedComponent && idsToRemove.has(selectedComponent) ? null : selectedComponent,
      configComponentId: configComponentId && idsToRemove.has(configComponentId) ? null : configComponentId,
    })
    get().scheduleSync(updatedDashboard)
  },

  removeComponentsByDevice(deviceId) {
    const { currentDashboard, dashboards, selectedComponent, configComponentId } = get()
    if (!currentDashboard) return
    const toRemove = currentDashboard.components.filter((comp) => {
      const ds = comp.dataSource
      if (!ds) return false
      const sources: DataSource[] = Array.isArray(ds) ? ds : [ds]
      return sources.some(
        (s) => s.sourceId === deviceId || (s.type === 'device' && s.property && s.sourceId === deviceId),
      )
    })
    if (toRemove.length === 0) return
    toRemove.forEach(cleanupAgentForComponent)
    const idsToRemove = new Set(toRemove.map((c) => c.id))
    const updatedDashboard = {
      ...currentDashboard,
      components: currentDashboard.components.filter((c) => !idsToRemove.has(c.id)),
      updatedAt: Date.now(),
    }
    const updatedDashboards = dashboards.map((d) =>
      d.id === currentDashboard.id ? updatedDashboard : d,
    )
    set({
      dashboards: updatedDashboards,
      currentDashboard: updatedDashboard,
      selectedComponent: selectedComponent && idsToRemove.has(selectedComponent) ? null : selectedComponent,
      configComponentId: configComponentId && idsToRemove.has(configComponentId) ? null : configComponentId,
    })
    get().scheduleSync(updatedDashboard)
  },

  duplicateComponent(id) {
    const { currentDashboard, dashboards } = get()
    if (!currentDashboard) return
    const original = currentDashboard.components.find((c) => c.id === id)
    if (!original) return
    const newComponent = {
      ...structuredClone(original),
      id: generateId(),
      position: { ...original.position, y: original.position.y + original.position.h },
    } as DashboardComponent
    const updatedDashboard = {
      ...currentDashboard,
      components: [...currentDashboard.components, newComponent],
      updatedAt: Date.now(),
    }
    const updatedDashboards = dashboards.map((d) =>
      d.id === currentDashboard.id ? updatedDashboard : d,
    )
    set({ dashboards: updatedDashboards, currentDashboard: updatedDashboard })
    get().scheduleSync(updatedDashboard)
  },
})
