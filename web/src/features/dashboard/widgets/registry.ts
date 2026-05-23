/**
 * Widget Registry — static registry for all built-in widgets
 *
 * Provides metadata, default sizes, and lazy-loaded components for all 19 built-in widgets.
 * Also includes helpers for grouping by category and querying the registry.
 */

import {
  Hash,
  Circle,
  TrendingUp,
  Layers,
  LineChart as LineChartIcon,
  BarChart3,
  PieChart as PieChartIcon,
  ToggleLeft,
  Send,
  Image,
  Play,
  Globe,
  FileText,
  Map,
  Webcam,
  Square as SquareIcon,
  Bot,
  ScanEye,
} from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import type { WidgetCategory, WidgetType } from '../types'
import { COMPONENT_SIZE_CONSTRAINTS } from '@/types/dashboard'

// ============================================================================
// Registry Entry (simplified from old ComponentMeta)
// ============================================================================

export interface WidgetRegistryEntry {
  type: WidgetType
  displayName: string
  description: string
  category: WidgetCategory
  icon: LucideIcon
  defaultSize: { w: number; h: number }
  sizeConstraints: { minW: number; minH: number; maxW: number; maxH: number }
}

// ============================================================================
// Static Widget Registry
// ============================================================================

const WIDGET_REGISTRY: Record<string, WidgetRegistryEntry> = {
  // Indicators
  'value-card': {
    type: 'value-card',
    displayName: 'Value Card',
    description: 'Display a single value with optional unit and trend',
    category: 'indicators',
    icon: Hash,
    defaultSize: { w: 3, h: 2 },
    sizeConstraints: getSizeConstraints('value-card'),
  },
  'led-indicator': {
    type: 'led-indicator',
    displayName: 'LED Indicator',
    description: 'Simple LED status indicator light',
    category: 'indicators',
    icon: Circle,
    defaultSize: { w: 2, h: 2 },
    sizeConstraints: getSizeConstraints('led-indicator'),
  },
  'sparkline': {
    type: 'sparkline',
    displayName: 'Sparkline',
    description: 'Mini trend chart showing data history',
    category: 'indicators',
    icon: TrendingUp,
    defaultSize: { w: 3, h: 2 },
    sizeConstraints: getSizeConstraints('sparkline'),
  },
  'progress-bar': {
    type: 'progress-bar',
    displayName: 'Progress Bar',
    description: 'Linear progress indicator',
    category: 'indicators',
    icon: Layers,
    defaultSize: { w: 4, h: 2 },
    sizeConstraints: getSizeConstraints('progress-bar'),
  },

  // Charts
  'line-chart': {
    type: 'line-chart',
    displayName: 'Line Chart',
    description: 'Time series line chart',
    category: 'charts',
    icon: LineChartIcon,
    defaultSize: { w: 6, h: 4 },
    sizeConstraints: getSizeConstraints('line-chart'),
  },
  'area-chart': {
    type: 'area-chart',
    displayName: 'Area Chart',
    description: 'Area chart with filled region',
    category: 'charts',
    icon: LineChartIcon,
    defaultSize: { w: 6, h: 4 },
    sizeConstraints: getSizeConstraints('area-chart'),
  },
  'bar-chart': {
    type: 'bar-chart',
    displayName: 'Bar Chart',
    description: 'Vertical bar chart for categorical data',
    category: 'charts',
    icon: BarChart3,
    defaultSize: { w: 6, h: 4 },
    sizeConstraints: getSizeConstraints('bar-chart'),
  },
  'pie-chart': {
    type: 'pie-chart',
    displayName: 'Pie Chart',
    description: 'Pie/donut chart for part-to-whole',
    category: 'charts',
    icon: PieChartIcon,
    defaultSize: { w: 4, h: 4 },
    sizeConstraints: getSizeConstraints('pie-chart'),
  },

  // Controls
  'toggle-switch': {
    type: 'toggle-switch',
    displayName: 'Command Button',
    description: 'Trigger button for device or extension commands',
    category: 'controls',
    icon: Send,
    defaultSize: { w: 2, h: 2 },
    sizeConstraints: getSizeConstraints('toggle-switch'),
  },

  // Display
  'image-display': {
    type: 'image-display',
    displayName: 'Image Display',
    description: 'Display images from URLs or data sources',
    category: 'display',
    icon: Image,
    defaultSize: { w: 4, h: 4 },
    sizeConstraints: getSizeConstraints('image-display'),
  },
  'image-history': {
    type: 'image-history',
    displayName: 'Image History',
    description: 'Historical images with slider navigation',
    category: 'display',
    icon: Play,
    defaultSize: { w: 6, h: 4 },
    sizeConstraints: getSizeConstraints('image-history'),
  },
  'web-display': {
    type: 'web-display',
    displayName: 'Web Display',
    description: 'Display web content via iframe',
    category: 'display',
    icon: Globe,
    defaultSize: { w: 6, h: 4 },
    sizeConstraints: getSizeConstraints('web-display'),
  },
  'markdown-display': {
    type: 'markdown-display',
    displayName: 'Markdown Display',
    description: 'Render markdown content',
    category: 'display',
    icon: FileText,
    defaultSize: { w: 4, h: 3 },
    sizeConstraints: getSizeConstraints('markdown-display'),
  },

  // Spatial
  'map-display': {
    type: 'map-display',
    displayName: 'Map Display',
    description: 'Interactive map with device markers',
    category: 'spatial',
    icon: Map,
    defaultSize: { w: 6, h: 5 },
    sizeConstraints: getSizeConstraints('map-display'),
  },
  'video-display': {
    type: 'video-display',
    displayName: 'Video Display',
    description: 'Video player for streams and camera feeds',
    category: 'spatial',
    icon: Webcam,
    defaultSize: { w: 6, h: 4 },
    sizeConstraints: getSizeConstraints('video-display'),
  },
  'custom-layer': {
    type: 'custom-layer',
    displayName: 'Custom Layer',
    description: 'Free-form container for devices and metrics',
    category: 'spatial',
    icon: SquareIcon,
    defaultSize: { w: 6, h: 5 },
    sizeConstraints: getSizeConstraints('custom-layer'),
  },

  // Business
  'agent-monitor-widget': {
    type: 'agent-monitor-widget',
    displayName: 'Agent Monitor',
    description: 'AI agent monitoring with status and execution history',
    category: 'business',
    icon: Bot,
    defaultSize: { w: 6, h: 4 },
    sizeConstraints: getSizeConstraints('agent-monitor-widget'),
  },
  'ai-analyst': {
    type: 'ai-analyst',
    displayName: 'AI Analyst',
    description: 'AI-powered data analysis in a timeline chat',
    category: 'business',
    icon: ScanEye,
    defaultSize: { w: 6, h: 5 },
    sizeConstraints: getSizeConstraints('ai-analyst'),
  },
}

// ============================================================================
// Helpers
// ============================================================================

function getSizeConstraints(type: string): { minW: number; minH: number; maxW: number; maxH: number } {
  const c = COMPONENT_SIZE_CONSTRAINTS[type as keyof typeof COMPONENT_SIZE_CONSTRAINTS]
  if (c) {
    return { minW: c.minW, minH: c.minH, maxW: c.maxW, maxH: c.maxH }
  }
  return { minW: 2, minH: 2, maxW: 12, maxH: 12 }
}

/** Get the full registry map */
export function getWidgetRegistry(): Record<string, WidgetRegistryEntry> {
  return WIDGET_REGISTRY
}

/** Get a single entry by type */
export function getWidgetMeta(type: string): WidgetRegistryEntry | undefined {
  return WIDGET_REGISTRY[type]
}

/** Group entries by category */
export function groupComponentsByCategory(
  entries: WidgetRegistryEntry[]
): Array<{ category: WidgetCategory; items: WidgetRegistryEntry[] }> {
  const grouped: Partial<Record<WidgetCategory, WidgetRegistryEntry[]>> = {}

  for (const entry of entries) {
    if (!grouped[entry.category]) {
      grouped[entry.category] = []
    }
    grouped[entry.category]!.push(entry)
  }

  const order: WidgetCategory[] = ['indicators', 'charts', 'controls', 'display', 'spatial', 'business']

  return order
    .filter((cat) => grouped[cat] && grouped[cat]!.length > 0)
    .map((cat) => ({ category: cat, items: grouped[cat]! }))
}

/** Get category display info */
export function getCategoryInfo(category: WidgetCategory): { label: string; icon: LucideIcon } {
  const map: Record<WidgetCategory, { label: string; icon: LucideIcon }> = {
    indicators: { label: 'Indicators', icon: Hash },
    charts: { label: 'Charts', icon: LineChartIcon },
    controls: { label: 'Controls', icon: ToggleLeft },
    display: { label: 'Display & Content', icon: Image },
    spatial: { label: 'Spatial & Media', icon: Map },
    business: { label: 'Business', icon: Bot },
  }
  return map[category]
}
