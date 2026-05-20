/**
 * Dashboard API client — new feature module
 *
 * Thin wrapper around the existing api.ts methods with proper typing.
 * Uses existing fetchAPI + DTO converters for API compatibility.
 */

import { fetchAPI } from '@/lib/api'
import { fromDashboardDTO, toDashboardDTO, toCreateDashboardDTO, toUpdateDashboardDTO } from '@/store/persistence/types'
import type { DashboardDTO } from '@/store/persistence/types'
import type { Dashboard } from '../types'

// ============================================================================
// Dashboard CRUD
// ============================================================================

/** List all dashboards */
export async function listDashboards(): Promise<Dashboard[]> {
  const res = await fetchAPI<{ dashboards: DashboardDTO[]; count: number }>('/dashboards')
  return (res.dashboards || []).map(fromDashboardDTO)
}

/** Get a single dashboard by ID */
export async function getDashboard(id: string): Promise<Dashboard> {
  const dto = await fetchAPI<DashboardDTO>(`/dashboards/${id}`)
  return fromDashboardDTO(dto)
}

/** Create a new dashboard */
export async function createDashboard(dashboard: Omit<Dashboard, 'id' | 'createdAt' | 'updatedAt'>): Promise<Dashboard> {
  const dto = toCreateDashboardDTO(dashboard)
  const res = await fetchAPI<DashboardDTO>('/dashboards', {
    method: 'POST',
    body: JSON.stringify(dto),
  })
  return fromDashboardDTO(res)
}

/** Update an existing dashboard */
export async function updateDashboard(id: string, updates: Partial<Dashboard>): Promise<Dashboard> {
  const dto = toUpdateDashboardDTO(updates)
  const res = await fetchAPI<DashboardDTO>(`/dashboards/${id}`, {
    method: 'PUT',
    body: JSON.stringify(dto),
  })
  return fromDashboardDTO(res)
}

/** Delete a dashboard */
export async function deleteDashboard(id: string): Promise<void> {
  await fetchAPI<{ ok: boolean }>(`/dashboards/${id}`, { method: 'DELETE' })
}

/** Set a dashboard as default */
export async function setDefaultDashboard(id: string): Promise<void> {
  await fetchAPI<{ id: string }>(`/dashboards/${id}/default`, { method: 'POST' })
}

// ============================================================================
// Share
// ============================================================================

export interface ShareConfig {
  shareId: string
  url: string
  expiresAt?: number
}

/** Share a dashboard (generate public URL) */
export async function shareDashboard(id: string): Promise<ShareConfig> {
  return fetchAPI<ShareConfig>(`/dashboards/${id}/share`, { method: 'POST' })
}

/** Remove dashboard sharing */
export async function unshareDashboard(id: string): Promise<void> {
  await fetchAPI<{ ok: boolean }>(`/dashboards/${id}/share`, { method: 'DELETE' })
}

// ============================================================================
// Templates
// ============================================================================

export interface DashboardTemplateResponse {
  id: string
  name: string
  description: string
  category: string
  components: DashboardDTO['components']
  layout: DashboardDTO['layout']
}

/** List available templates */
export async function listTemplates(): Promise<DashboardTemplateResponse[]> {
  return fetchAPI<DashboardTemplateResponse[]>('/dashboards/templates')
}
