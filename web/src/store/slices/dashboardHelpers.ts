/**
 * Shared helpers for dashboard slices.
 * Extracted from dashboardSlice.ts to avoid duplication across split slices.
 */

import type {
  Dashboard,
  DashboardComponent,
  DashboardTemplate,
  DataSource,
} from '@/types/dashboard'
import { isRealtimeSource, isDeviceInfoSource, isCommandSource, isExtensionSource } from '@/types/dashboard'

/** Combined dashboard store type for StateCreator generics.
 *  Includes only the fields accessed by dashboard slices — enough for
 *  type-safe StateCreator usage without circular imports. */
export type DashboardStore = {
  // CrudSlice
  dashboards: Dashboard[]
  currentDashboard: Dashboard | null
  currentDashboardId: string | null
  templates: DashboardTemplate[]
  _fetchId: number | null
  dashboardsLoading: boolean
  scheduleSync: (dashboard: Dashboard) => void
  flushSync: () => Promise<void>
  // UISlice
  editMode: boolean
  selectedComponent: string | null
  configComponentId: string | null
  componentLibraryOpen: boolean
  configPanelOpen: boolean
  templateDialogOpen: boolean
  // DeviceSlice
  devices: import('@/types').Device[]
  // ExtensionSlice
  extensions: import('@/types').Extension[]
  // FrontendComponentSlice
  installed: unknown[]
}

/** Generate unique ID */
export function generateId(): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID && typeof crypto.randomUUID === 'function') {
    try { return crypto.randomUUID() } catch { /* fall through */ }
  }
  return 'id_' + Date.now().toString(36) + '_' + Math.random().toString(36).substring(2, 15)
}

/** Delete the associated AI Agent when an ai-analyst component is removed */
export function cleanupAgentForComponent(component: DashboardComponent | undefined) {
  if (!component || component.type !== 'ai-analyst') return
  const agentId = (component as any).config?.agentId as string | undefined
  if (!agentId) return
  import('@/lib/api').then(({ api }) => {
    api.deleteAgent(agentId).catch((err) => {
      console.warn('[Dashboard] Failed to delete agent', agentId, err)
    })
  })
}

/** Check if a single data source references a valid entity */
export function isSingleDataSourceValid(
  ds: DataSource,
  validDeviceIds: Set<string>,
  validExtensionIds: Set<string>,
): boolean {
  if ((isRealtimeSource(ds) || isDeviceInfoSource(ds) || isCommandSource(ds)) && ds.sourceId) {
    return validDeviceIds.has(ds.sourceId)
  }
  if (isExtensionSource(ds) && ds.extensionId) {
    return validExtensionIds.has(ds.extensionId)
  }
  return true
}

/** Check if a component's data source references a valid entity */
export function isDataSourceValid(
  comp: DashboardComponent,
  validDeviceIds: Set<string>,
  validExtensionIds: Set<string>,
): boolean {
  const ds = 'dataSource' in comp ? comp.dataSource : undefined
  if (!ds) return true
  if (Array.isArray(ds)) {
    return ds.every((d) => isSingleDataSourceValid(d, validDeviceIds, validExtensionIds))
  }
  return isSingleDataSourceValid(ds, validDeviceIds, validExtensionIds)
}

/** Update currentDashboard and dashboards array atomically */
export function updateDashboardInState(
  dashboards: Dashboard[],
  targetId: string,
  updates: Partial<Dashboard>,
  currentDashboardId: string | null,
): { dashboards: Dashboard[]; currentDashboard: Dashboard | null } {
  const updatedDashboards = dashboards.map((d) =>
    d.id === targetId ? { ...d, ...updates } : d,
  )
  const currentDashboard = updatedDashboards.find((d) => d.id === currentDashboardId) || null
  return { dashboards: updatedDashboards, currentDashboard }
}
