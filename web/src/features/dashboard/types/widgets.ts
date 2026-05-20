/**
 * Widget types — new feature module
 */

import type { LucideIcon } from 'lucide-react'
import type { ComponentSizeConstraints, ImplementedComponentType, DataSource, DisplayConfig, ActionConfig } from '@/types/dashboard'
import type { ResolvedDataSource } from './dataSources'

// ============================================================================
// Widget type union
// ============================================================================

/** All 19 widget types (17 generic + 2 business) */
export type WidgetType = ImplementedComponentType

// ============================================================================
// Widget props — what every widget receives
// ============================================================================

export interface WidgetProps {
  widgetId: string
  dataSource: ResolvedDataSource | null
  isEditing: boolean
  title?: string
}

export interface WidgetConfigProps {
  widgetId: string
  config: WidgetConfig
  onSave: (updates: Partial<WidgetConfig>) => void
}

/** Widget config stored in the dashboard document */
export interface WidgetConfig {
  dataSource?: DataSource
  display?: DisplayConfig
  actions?: ActionConfig[]
  [key: string]: unknown
}

// ============================================================================
// Widget registry types
// ============================================================================

export interface StaticWidgetDefinition {
  source: 'static'
  type: WidgetType
  displayName: string
  icon: LucideIcon
  defaultSize: { w: number; h: number }
  sizeConstraints: ComponentSizeConstraints
  component: React.LazyExoticComponent<React.ComponentType<WidgetProps>>
  configComponent: React.LazyExoticComponent<React.ComponentType<WidgetConfigProps>>
}

export interface DynamicWidgetDefinition {
  source: 'dynamic'
  type: string
  displayName: string
  icon: LucideIcon
  defaultSize: { w: number; h: number }
  sizeConstraints: ComponentSizeConstraints
  loader: () => Promise<{ default: React.ComponentType<WidgetProps> }>
  configLoader?: () => Promise<{ default: React.ComponentType<WidgetConfigProps> }>
  onMount?: () => void
  onUnmount?: () => void
}

export type WidgetDefinition = StaticWidgetDefinition | DynamicWidgetDefinition

// ============================================================================
// Widget category
// ============================================================================

export type WidgetCategory = 'indicators' | 'charts' | 'controls' | 'display' | 'spatial' | 'business'

export interface CategoryInfo {
  id: WidgetCategory
  label: string
  icon: LucideIcon
}

/** Get category for a widget type */
export function getWidgetCategory(type: WidgetType): WidgetCategory {
  switch (type) {
    case 'value-card':
    case 'led-indicator':
    case 'sparkline':
    case 'progress-bar':
      return 'indicators'
    case 'line-chart':
    case 'area-chart':
    case 'bar-chart':
    case 'pie-chart':
      return 'charts'
    case 'toggle-switch':
      return 'controls'
    case 'image-display':
    case 'image-history':
    case 'web-display':
    case 'markdown-display':
      return 'display'
    case 'map-display':
    case 'video-display':
    case 'custom-layer':
      return 'spatial'
    case 'agent-monitor-widget':
    case 'ai-analyst':
      return 'business'
    default:
      return 'display'
  }
}
