/**
 * Dashboard Store - Composed from slices
 *
 * Combines all dashboard-related slices into a single store.
 * Uses Zustand's slice pattern for modular state management.
 */

import { create } from 'zustand'
import {
  createDashboardCrudSlice,
  type DashboardCrudSlice,
} from './dashboardCrudSlice'
import {
  createDashboardLayoutSlice,
  type DashboardLayoutSlice,
} from './dashboardLayoutSlice'
import {
  createDashboardEditSlice,
  type DashboardEditSlice,
} from './dashboardEditSlice'
import {
  createDashboardConfigSlice,
  type DashboardConfigSlice,
} from './dashboardConfigSlice'

// ============================================================================
// Combined Store Type
// ============================================================================

export type DashboardStore = DashboardCrudSlice &
  DashboardLayoutSlice &
  DashboardEditSlice &
  DashboardConfigSlice

// ============================================================================
// Create Store
// ============================================================================

export const useDashboardStore = create<DashboardStore>()((...a) => ({
  ...createDashboardCrudSlice(...a),
  ...createDashboardLayoutSlice(...a),
  ...createDashboardEditSlice(...a),
  ...createDashboardConfigSlice(...a),
}))

// ============================================================================
// Re-exports
// ============================================================================

export type { DashboardCrudSlice } from './dashboardCrudSlice'
export type { DashboardLayoutSlice } from './dashboardLayoutSlice'
export type { DashboardEditSlice } from './dashboardEditSlice'
export type { DashboardConfigSlice } from './dashboardConfigSlice'
export { DEFAULT_LAYOUT, DEFAULT_TEMPLATES, generateId } from './dashboardCrudSlice'
