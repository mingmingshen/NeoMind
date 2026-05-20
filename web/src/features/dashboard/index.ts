/**
 * Dashboard Feature Module — public API
 *
 * Import from this file to use the new dashboard feature module.
 */

// Main page component
export { VisualDashboard } from './components/VisualDashboard'

// Store
export { useDashboardStore } from './store'
export type { DashboardStore } from './store'

// Types
export type { WidgetProps, WidgetConfigProps, WidgetConfig } from './types'
export type { ResolvedDataSource } from './types'

// Widget registry
export { getWidgetRegistry, getWidgetMeta, groupComponentsByCategory, getCategoryInfo } from './widgets/registry'
export { getWidgetComponent, hasWidgetAdapter } from './widgets/adapters'

// Hooks
export { useWidgetDataSource } from './hooks/useWidgetDataSource'
