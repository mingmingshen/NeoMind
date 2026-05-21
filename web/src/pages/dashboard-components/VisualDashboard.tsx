/**
 * Visual Dashboard Page
 *
 * Main dashboard page with grid layout, drag-and-drop, and component library.
 * Supports both generic IoT components and business components.
 */

import { useEffect, useState, useCallback, useRef, useMemo, memo } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { useStore } from '@/store'
import { shallow } from 'zustand/shallow'
import { useErrorHandler } from '@/hooks/useErrorHandler'
import { useExtensionLifecycle } from '@/hooks/useExtensionLifecycle'
import { useCommunityComponentLifecycle } from '@/hooks/useCommunityComponentLifecycle'
import { useDashboardPrefetch } from '@/hooks/useDashboardPrefetch'
import { logError } from '@/lib/errors'
import { fetchCache } from '@/lib/utils/async'
import { cn } from '@/lib/utils'
import { chartColorsHex } from '@/design-system/tokens/color'
import { createStableKey as createStableCacheKey } from '@/lib/stable-key'
import { useIsMobile, useTouchHover } from '@/hooks/useMobile'
import {
  LayoutDashboard,
  Plus,
  Check,
  Settings2,
  PanelsTopLeft,
  Copy,
  Trash2,
  Settings as SettingsIcon,
  ChevronRight,
  MoreVertical,
  Maximize,
  Minimize,
  // Indicator icons
  Hash,
  Circle,
  TrendingUp,
  Timer as TimerIcon,
  Hourglass,
  // Chart icons
  LineChart as LineChartIcon,
  BarChart3,
  PieChart as PieChartIcon,
  ScatterChart as ScatterChartIcon,
  Radar as RadarIcon,
  Filter,
  CandlestickChart,
  // Control icons
  ToggleLeft,
  Sliders as SliderIcon,
  RadioIcon,
  CheckSquare,
  ToggleLeft as SwitchIcon,
  Star,
  MapPin,
  Monitor,
  RotateCw,
  // Media icons
  Image as ImageIcon,
  Video as VideoIcon,
  Camera,
  Music,
  Globe,
  QrCode,
  Square as SquareIcon,
  Type,
  Code,
  Link,
  Share2,
  // Layout icons
  Layers,
  Container as ContainerIcon,
  MinusSquare,
  Grid,
  Minus,
  // Visualization icons
  Calendar as CalendarIcon,
  GitBranch,
  Network,
  Map as MapIcon,
  Zap,
  Box,
  Cloud,
  Sparkles,
  Database,
  // Agent icons
  Bot,
  ListTodo,
  Clock,
  Brain,
  // Device icons
  Workflow,
  Activity,
  SlidersHorizontal,
  HeartPulse,
  // More icons
  FileText,
  Table,
  Search,
  ChevronDown,
  LayoutGrid,
  List,
  Scroll,
  Play,
  Upload,
  Store as StoreIcon,
  Download,
  PackagePlus,
  Loader2,
} from 'lucide-react'
import { useParams, useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Checkbox } from '@/components/ui/checkbox'
import { Switch } from '@/components/ui/switch'
import { Badge } from '@/components/ui/badge'
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '@/components/ui/collapsible'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import {
  FullScreenDialog,
  FullScreenDialogHeader,
  FullScreenDialogContent,
} from '@/components/automation/dialog'
import { toast } from '@/components/ui/use-toast'

// Config system
import {
  createDataDisplayConfig,
  createProgressConfig,
  createControlConfig,
  createIndicatorConfig,
  createContentConfig,
  createChartConfig,
  ComponentConfigDialog,
  DualModeSourceField,
  UnifiedDataSourceConfig,
} from '@/components/dashboard/config'
import type { ComponentConfigSchema, ConfigSection } from '@/components/dashboard/config/ComponentConfigBuilder'
import { ValueMapEditor } from '@/components/dashboard/config/ValueMapEditor'
import { DataMappingConfig } from '@/components/dashboard/config/UIConfigSections'
import { LEDStateRulesConfig } from '@/components/dashboard/config/LEDStateRulesConfig'
import type { StateRule } from '@/components/dashboard/generic/LEDIndicator'
import type { SingleValueMappingConfig, TimeSeriesMappingConfig, CategoricalMappingConfig } from '@/lib/dataMapping'

// UI components
import { ColorPicker } from '@/components/ui/color-picker'
import { IconPicker } from '@/components/ui/icon-picker'
import { EntityIconPicker } from '@/components/ui/entity-icon-picker'

// Dashboard components
import { DashboardGrid } from '@/components/dashboard/DashboardGrid'
import { LayerEditorDialog } from '@/components/dashboard/generic/LayerEditorDialog'
import { MapEditorDialog, type MapBinding, type MapBindingType } from '@/components/dashboard/generic/MapEditorDialog'
import { CenterPickerDialog } from '@/components/dashboard/generic/CenterPickerDialog'
import type { LayerBinding, LayerBindingType } from '@/components/dashboard/generic/CustomLayer'
import { DashboardListSidebar } from '@/components/dashboard/DashboardListSidebar'
import { ShareManagerDialog } from '@/components/dashboard/ShareManagerDialog'
import { InstallComponentDialog } from '@/pages/dashboard-components/InstallComponentDialog'
import type { MarketComponentEntry } from '@/types/frontend-component'
import { MobileEditBar } from '@/components/dashboard/MobileEditBar'
import type { DashboardComponent, DataSourceOrList, DataSource, GenericComponent } from '@/types/dashboard'
import { getSourceId, normalizeDataSource } from '@/types/dashboard'
import type { Device } from '@/types'
import { COMPONENT_SIZE_CONSTRAINTS } from '@/types/dashboard'
import { dynamicRegistry, dtoToComponentMeta } from '@/components/dashboard/registry/DynamicRegistry'
import { communityRegistry } from '@/components/dashboard/registry/CommunityRegistry'
import { DeviceBindingConfig } from '@/components/dashboard/config/DeviceBindingConfig'
import { componentRegistry, groupComponentsByCategory, getCategoryInfo } from '@/components/dashboard/registry/registry'
import * as lucideReact from 'lucide-react'
import { api, fetchAPI } from '@/lib/api'
import { notifySuccess, notifyError } from '@/lib/notify'
import { confirm } from '@/hooks/use-confirm'

// Import ComponentRenderer for extension components
import ComponentRenderer from '@/components/dashboard/registry/ComponentRenderer'

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Memoized cache for converted telemetry data sources
 * Caches both individual DataSource objects AND complete arrays to prevent reference changes
 */
const telemetryCache: Record<string, any> = {}
const MAX_CACHE_SIZE = 100  // Prevent memory leaks by limiting cache size
const cacheKeys: string[] = []  // Track insertion order for LRU eviction

function scheduleDashboardIdleTask(task: () => void, timeout = 1500): () => void {
  if (typeof window === 'undefined') {
    task()
    return () => {}
  }

  const requestIdle = (window as any).requestIdleCallback as
    | ((cb: () => void, options?: { timeout: number }) => number)
    | undefined
  const cancelIdle = (window as any).cancelIdleCallback as ((id: number) => void) | undefined

  if (requestIdle && cancelIdle) {
    const id = requestIdle(task, { timeout })
    return () => cancelIdle(id)
  }

  const timer = window.setTimeout(task, Math.min(timeout, 300))
  return () => window.clearTimeout(timer)
}

/**
 * Convert device data source to telemetry with caching to prevent infinite re-renders
 * This function caches the ENTIRE result (including arrays) to ensure reference stability
 */
function getTelemetryDataSource(dataSource: DataSourceOrList | undefined): DataSourceOrList | undefined {
  if (!dataSource) return undefined

  // Create a stable cache key (property-order independent) from the entire input
  const cacheKey = createStableCacheKey(dataSource)

  // Return cached result if exists (reference stability!)
  if (cacheKey in telemetryCache) {
    // Move to end of keys to mark as recently used
    const idx = cacheKeys.indexOf(cacheKey)
    if (idx > -1) {
      cacheKeys.splice(idx, 1)
      cacheKeys.push(cacheKey)
    }
    return telemetryCache[cacheKey]
  }

  const normalizeAndConvert = (ds: DataSource): DataSource => {
    // If already telemetry, return as-is
    if (ds.type === 'telemetry') return ds

    // Convert device type to telemetry for Sparkline
    if (ds.type === 'device' && getSourceId(ds) && ds.property) {
      return {
        type: 'telemetry',
        sourceId: getSourceId(ds),
        metricId: ds.property,
        timeRange: ds.timeRange ?? 1,
        limit: ds.limit ?? 50,
        aggregate: ds.aggregate ?? 'raw',
        refresh: ds.refresh ?? 10,
      }
    }

    return ds
  }

  const result: DataSourceOrList = Array.isArray(dataSource)
    ? dataSource.map(normalizeAndConvert)
    : normalizeAndConvert(dataSource)

  // Implement LRU cache eviction to prevent memory leaks
  if (cacheKeys.length >= MAX_CACHE_SIZE) {
    const oldestKey = cacheKeys.shift()!
    delete telemetryCache[oldestKey]
  }

  // Cache the entire result for reference stability
  telemetryCache[cacheKey] = result
  cacheKeys.push(cacheKey)

  return result
}

// Helper function to determine if title should be in display section
// All components show title in the display tab (unified standard)
function isTitleInDisplayComponent(_componentType?: string): boolean {
  // Unified: all components show title in Display tab
  return true
}

// ============================================================================
// Component Library Data
// ============================================================================

type ComponentIconType = React.ComponentType<{ className?: string }>

interface ComponentItem {
  id: string
  name: string
  description: string
  icon: ComponentIconType
}

interface ComponentCategory {
  category: string
  categoryLabel: string
  categoryIcon: ComponentIconType
  items: ComponentItem[]
}

// Factory function to get component library with translations
function getComponentLibrary(t: (key: string) => string): ComponentCategory[] {
  // Get all components grouped by category from the registry
  const grouped = groupComponentsByCategory()

  // i18n key mapping: component type → translation key
  const nameKeys: Record<string, string> = {
    'value-card': 'valueCard',
    'led-indicator': 'ledIndicator',
    'sparkline': 'sparkline',
    'progress-bar': 'progressBar',
    'line-chart': 'lineChart',
    'area-chart': 'areaChart',
    'bar-chart': 'barChart',
    'pie-chart': 'pieChart',
    'image-display': 'imageDisplay',
    'image-history': 'imageHistory',
    'web-display': 'webDisplay',
    'markdown-display': 'markdownDisplay',
    'map-display': 'mapDisplay',
    'video-display': 'videoDisplay',
    'custom-layer': 'customLayer',
    'toggle-switch': 'toggleSwitch',
    'agent-monitor-widget': 'agentMonitor',
    'vlm-vision': 'aiAnalyst',
    'ai-analyst': 'aiAnalyst',
  }
  const descKeys: Record<string, string> = {
    'value-card': 'valueCardDesc',
    'led-indicator': 'ledIndicatorDesc',
    'sparkline': 'sparklineDesc',
    'progress-bar': 'progressBarDesc',
    'line-chart': 'lineChartDesc',
    'area-chart': 'areaChartDesc',
    'bar-chart': 'barChartDesc',
    'pie-chart': 'pieChartDesc',
    'image-display': 'imageDisplayDesc',
    'image-history': 'imageHistoryDesc',
    'web-display': 'webDisplayDesc',
    'markdown-display': 'markdownDisplayDesc',
    'map-display': 'mapDisplayDesc',
    'video-display': 'videoDisplayDesc',
    'custom-layer': 'customLayerDesc',
    'toggle-switch': 'toggleSwitchDesc',
    'agent-monitor-widget': 'agentMonitorDesc',
    'vlm-vision': 'aiAnalystDesc',
    'ai-analyst': 'aiAnalystDesc',
  }
  // Category i18n keys
  const categoryLabelKeys: Record<string, string> = {
    indicators: 'indicators',
    charts: 'charts',
    display: 'display',
    spatial: 'spatial',
    controls: 'controls',
    business: 'business',
    custom: 'custom',
    community: 'community',
  }

  return grouped.map((group) => {
    const catInfo = getCategoryInfo(group.category as any)
    const labelKey = categoryLabelKeys[group.category]
    const lucideRecord: any = lucideReact

    return {
      category: group.category,
      categoryLabel: labelKey ? t(`componentLibrary.${labelKey}`) : catInfo.name,
      categoryIcon: catInfo.icon,
      items: group.components.map((comp) => {
        const iconName = (comp.icon as any)?.displayName || 'Box'
        const IconComponent = typeof comp.icon === 'function' ? comp.icon : (lucideRecord[iconName] || Box)
        const nKey = nameKeys[comp.type]
        const dKey = descKeys[comp.type]

        return {
          id: comp.type,
          name: nKey ? t(`componentLibrary.${nKey}`) : comp.name,
          description: dKey ? t(`componentLibrary.${dKey}`) : comp.description,
          icon: IconComponent,
        }
      }),
    }
  })
}

// ============================================================================
// Render Component
// ============================================================================

// Helper to extract common display props from component config
export function getCommonDisplayProps(component: DashboardComponent) {
  const config = (component as any).config || {}

  // Auto-calculate size based on component dimensions
  const w = component.position.w
  const h = component.position.h
  const area = w * h

  // Determine size based on component area
  let calculatedSize: 'xs' | 'sm' | 'md' | 'lg' = 'md'
  if (area <= 1) {
    calculatedSize = 'xs'  // 1x1 grid
  } else if (area <= 2) {
    calculatedSize = 'sm'  // 1x2 or 2x1 grid
  } else if (area <= 4) {
    calculatedSize = 'md'  // 2x2 grid
  } else {
    calculatedSize = 'lg'  // larger than 2x2
  }

  // Use config size if explicitly set, otherwise use calculated size
  const size = config.size || calculatedSize

  return {
    size,
    showCard: config.showCard ?? true,
    className: config.className,
    title: component.title,
    color: config.color,
    // Pass dimensions for responsive adjustments
    dimensions: { w, h, area },
  }
}

// Props that can be safely spread to most components
export const getSpreadableProps = (componentType: string, commonProps: ReturnType<typeof getCommonDisplayProps>) => {
  // Components that don't support standard size ('sm' | 'md' | 'lg')
  const noStandardSize = [
    'led-indicator', 'toggle-switch',
    'heading', 'alert-banner',
    'agent-monitor-widget',
    'ai-analyst',
  ]

  // Components that don't support showCard
  const noShowCard = [
    'value-card', 'led-indicator', 'sparkline', 'progress-bar',
    'toggle-switch',
    'heading', 'alert-banner',
    'agent-monitor-widget',
    'ai-analyst',
    'tabs',
  ]

  // Components that don't support title in the spread position
  const noTitle = [
    'sparkline', 'led-indicator', 'progress-bar',
    'toggle-switch',
    'heading', 'alert-banner',
    'tabs',
    'agent-monitor-widget',
  ]

  const result: Record<string, unknown> = {}

  // Include size for components that support it
  if (!noStandardSize.includes(componentType)) {
    result.size = commonProps.size
  }
  if (!noShowCard.includes(componentType)) {
    result.showCard = commonProps.showCard
  }
  if (!noTitle.includes(componentType)) {
    result.title = commonProps.title
  }
  result.className = commonProps.className
  if (commonProps.color) {
    result.color = commonProps.color
  }

  return result
}

// Calculate chart height based on grid dimensions (each grid row ~120px)
export function getChartHeight(component: DashboardComponent): number | 'auto' {
  const h = component.position.h
  // Calculate height: grid rows * 120px - padding (approx 60px for card padding)
  const calculatedHeight = Math.max(h * 120 - 60, 120)
  return calculatedHeight
}

export function renderDashboardComponent(
  component: DashboardComponent,
  editMode?: boolean,
  onDataSourceChange?: (dataSource: Record<string, any>) => void,
  onConfigChange?: (config: Record<string, any>) => void,
  openFullscreen?: (content: React.ReactNode) => void,
  closeFullscreen?: () => void
) {
  const config = (component as any).config || {}
  const normalizedComponent = {
    ...component,
    config: {
      ...config,
      editMode,
      height: config.height ?? getChartHeight(component),
    },
  } as DashboardComponent

  return (
    <ComponentRenderer
      component={normalizedComponent}
      className="w-full h-full"
      onDataSourceChange={onDataSourceChange}
      onConfigChange={onConfigChange}
      openFullscreen={openFullscreen}
      closeFullscreen={closeFullscreen}
    />
  )

  /*
  // Legacy direct renderer kept temporarily for reference. It is intentionally
  // disabled so VisualDashboard no longer statically imports every dashboard
  // widget module before the first dashboard shell can paint.
  const dataSource = (component as any).dataSource

  const commonProps = getCommonDisplayProps(component)
  const spreadableProps = getSpreadableProps(component.type, commonProps)

  try {
    switch (component.type) {
    // Indicators
    case 'value-card':
      return (
        <ValueCard
          {...spreadableProps}
          dataSource={dataSource}
          title={commonProps.title || 'Value'}
          unit={config.unit}
          prefix={config.prefix}
          icon={config.icon}
          iconType={config.iconType || 'entity'}
          description={config.description}
          variant={config.variant || 'default'}
          iconColor={config.iconColor}
          valueColor={config.valueColor}
          showTrend={config.showTrend}
          size={config.size || 'md'}
        />
      )

    case 'led-indicator':
      return (
        <LEDIndicator
          {...spreadableProps}
          dataSource={dataSource}
          rules={config.rules as StateRule[]}
          defaultState={config.defaultState || 'unknown'}
          stateLabels={config.stateLabels as Record<string, string>}
          title={commonProps.title || config.label}
          size={config.size || 'md'}
          showGlow={config.showGlow ?? true}
          showAnimation={config.showAnimation ?? true}
          showCard={config.showCard ?? true}
        />
      )

    case 'sparkline':
      return (
        <Sparkline
          {...spreadableProps}
          dataSource={dataSource}
          data={config.data}
          showCard={commonProps.showCard}
          showThreshold={config.showThreshold ?? false}
          threshold={config.threshold}
          thresholdColor={config.thresholdColor}
          title={commonProps.title}
          color={config.color}
          colorMode={config.colorMode || 'fixed'}
          fill={config.fill ?? true}
          strokeWidth={config.strokeWidth}
          curved={config.curved ?? true}
          showValue={config.showValue ?? true}
          maxValue={config.maxValue}
        />
      )

    case 'progress-bar':
      return (
        <ProgressBar
          {...spreadableProps}
          dataSource={dataSource}
          value={dataSource ? undefined : config.value}
          max={config.max ?? 100}
          title={commonProps.title}
          color={config.color}
          size={config.size || commonProps.size}
          variant={config.variant || 'default'}
          warningThreshold={config.warningThreshold}
          dangerThreshold={config.dangerThreshold}
          dataMapping={config.dataMapping}
          showCard={config.showCard ?? true}
          icon={config.icon}
          iconColor={config.iconColor}
          backgroundColor={config.backgroundColor}
        />
      )

    // Charts
    case 'line-chart':
      return (
        <LineChart
          {...spreadableProps}
          dataSource={dataSource}
          dataMapping={config.dataMapping}
          series={config.series || [{
            name: 'Value',
            data: [20, 22, 21, 24, 23, 26, 25, 28, 27, 30],
            color: chartColorsHex[0]
          }]}
          labels={config.labels || ['1h', '2h', '3h', '4h', '5h', '6h', '7h', '8h', '9h', '10h']}
          height={getChartHeight(component)}
          title={commonProps.title}
          limit={config.limit}
          timeRange={config.timeRange}
          showGrid={config.showGrid ?? true}
          showLegend={config.showLegend ?? false}
          showTooltip={config.showTooltip ?? true}
          smooth={config.smooth ?? true}
          fillArea={config.fillArea ?? false}
          color={config.color}
          size={config.size}
        />
      )

    case 'area-chart':
      return (
        <AreaChart
          {...spreadableProps}
          dataSource={dataSource}
          dataMapping={config.dataMapping}
          series={config.series || [{
            name: 'Value',
            data: [20, 22, 21, 24, 23, 26, 25, 28, 27, 30],
            color: chartColorsHex[0]
          }]}
          labels={config.labels || ['1h', '2h', '3h', '4h', '5h', '6h', '7h', '8h', '9h', '10h']}
          height={getChartHeight(component)}
          title={commonProps.title}
          limit={config.limit}
          timeRange={config.timeRange}
          showGrid={config.showGrid ?? true}
          showLegend={config.showLegend ?? false}
          showTooltip={config.showTooltip ?? true}
          smooth={config.smooth ?? true}
          color={config.color}
          size={config.size}
        />
      )

    case 'bar-chart':
      return (
        <BarChart
          {...spreadableProps}
          dataSource={dataSource}
          dataMapping={config.dataMapping}
          data={config.data}
          title={commonProps.title}
          height={getChartHeight(component)}
          limit={config.limit}
          timeRange={config.timeRange}
          showGrid={config.showGrid ?? true}
          showLegend={config.showLegend ?? false}
          showTooltip={config.showTooltip ?? true}
          layout={config.layout || 'vertical'}
          stacked={config.stacked ?? false}
          color={config.color}
          size={config.size || 'md'}
        />
      )

    case 'pie-chart':
      return (
        <PieChart
          {...spreadableProps}
          dataSource={dataSource}
          dataMapping={config.dataMapping}
          data={config.data}
          title={commonProps.title}
          height={getChartHeight(component)}
          limit={config.limit}
          timeRange={config.timeRange}
          showLegend={config.showLegend ?? false}
          showTooltip={config.showTooltip ?? true}
          showLabels={config.showLabels ?? false}
          variant={config.variant || 'donut'}
          innerRadius={config.innerRadius}
          outerRadius={config.outerRadius}
          size={config.size || 'md'}
          colors={config.colors}
        />
      )

    // Controls
    case 'toggle-switch':
      return (
        <ToggleSwitch
          {...spreadableProps}
          size={config.size || commonProps.size === 'xs' ? 'sm' : commonProps.size}
          dataSource={dataSource}
          title={commonProps.title}
          editMode={editMode}
        />
      )

    // Display & Content
    case 'image-display':
      return (
        <ImageDisplay
          {...spreadableProps}
          dataSource={dataSource}
          src={config.src}
          alt={config.alt || commonProps.title || 'Image'}
          caption={config.caption}
          fit={config.fit || 'contain'}
          rounded={config.rounded ?? true}
          showShadow={config.showShadow}
          zoomable={config.zoomable ?? true}
          downloadable={config.downloadable}
        />
      )

    case 'image-history': {
      const imageLimit = typeof config.limit === 'number' && config.limit >= 1 && config.limit <= 500
        ? config.limit
        : 200
      const imageTimeRange = config.timeRange ?? 48
      return (
        <ImageHistory
          {...spreadableProps}
          dataSource={dataSource}
          images={config.images}
          fit={config.fit || 'fill'}
          rounded={config.rounded ?? true}
          limit={imageLimit}
          timeRange={imageTimeRange}
        />
      )
    }

    case 'web-display':
      return (
        <WebDisplay
          {...spreadableProps}
          dataSource={dataSource}
          src={config.src}
          title={config.title || commonProps.title}
          sandbox={config.sandbox ?? true}
          allowFullscreen={config.allowFullscreen ?? true}
          showHeader={config.showHeader ?? true}
          showUrlBar={config.showUrlBar}
        />
      )

    case 'markdown-display':
      return (
        <MarkdownDisplay
          {...spreadableProps}
          dataSource={dataSource}
          content={config.content}
          variant={config.variant || 'default'}
        />
      )

    case 'video-display':
      return (
        <VideoDisplay
          {...spreadableProps}
          dataSource={dataSource}
          src={config.src}
          type={config.type}
          size={config.size}
          autoplay={config.autoplay ?? false}
          muted={config.muted ?? true}
          controls={config.controls ?? true}
          loop={config.loop ?? false}
          fit={config.fit || 'contain'}
          rounded={config.rounded ?? true}
          showFullscreen={config.showFullscreen ?? true}
        />
      )

    case 'map-display':
      // Convert bindings to markers format for MapDisplay
      // Read devices directly from store to avoid parameter coupling
      // (devices change every 3s from batch polling, would cause gridComponents memo invalidation)
      const storeDevices = useStore.getState().devices
      const storeDeviceMap = new Map<string, Device>()
      for (const device of storeDevices) {
        storeDeviceMap.set(device.id, device)
        if (device.device_id) storeDeviceMap.set(device.device_id, device)
      }

      // Helper to get device name
      const getDeviceName = (deviceId: string) => {
        const device = storeDeviceMap.get(deviceId)
        return device?.name || device?.device_id || deviceId
      }

      // Helper to get device status
      const getDeviceStatus = (deviceId: string): 'online' | 'offline' | 'error' | 'warning' | undefined => {
        const device = storeDeviceMap.get(deviceId)
        if (!device) return undefined
        return device.online ? 'online' : 'offline'
      }

      const bindingsMarkers = (config.bindings as MapBinding[])?.map((binding): MapMarker => {
        // Get type from icon first, then fallback to type
        const markerType = binding.icon || binding.type
        const ds = binding.dataSource as any

        // Get the device for this binding (used for status, metric values, names)
        const sourceId = getSourceId(ds)
        const device = sourceId ? storeDeviceMap.get(sourceId) : undefined

        // Get metric value for metric bindings
        let metricValue: string | undefined = undefined
        if (binding.type === 'metric' && sourceId) {
          const metricKey = ds.metricId || ds.property
          if (device?.current_values && metricKey) {
            const rawValue = device.current_values[metricKey]
            if (rawValue !== undefined && rawValue !== null) {
              metricValue = typeof rawValue === 'number'
                ? rawValue.toFixed(1)
                : String(rawValue)
            }
          }
        }

        return {
          id: binding.id,
          latitude: binding.position === 'auto' || !binding.position
            ? (config.center as { lat: number; lng: number })?.lat ?? 39.9042
            : binding.position.lat,
          longitude: binding.position === 'auto' || !binding.position
            ? (config.center as { lat: number; lng: number })?.lng ?? 116.4074
            : binding.position.lng,
          label: binding.name,
          markerType,
          // Device-specific fields - use actual device status
          deviceId: sourceId,
          sourceId,
          status: binding.type === 'device' && sourceId ? getDeviceStatus(sourceId) : undefined,
          // Metric-specific fields
          metricValue: binding.type === 'metric' ? (metricValue || '-') : undefined,
          // Command-specific fields
          command: binding.type === 'command' ? ds?.command : undefined,
          // Names for display
          deviceName: sourceId ? getDeviceName(sourceId) : undefined,
          metricName: ds?.metricId || ds?.property,
          commandName: binding.type === 'command' ? ds?.command : undefined,
        }
      }) ?? []

      // Use bindings markers if available, otherwise fallback to config.markers or empty array
      const displayMarkers = bindingsMarkers.length > 0 ? bindingsMarkers : (config.markers as MapMarker[] || [])

      return (
        <MapDisplay
          {...spreadableProps}
          dataSource={dataSource}
          markers={displayMarkers}
          center={config.center}
          zoom={config.zoom}
          minZoom={config.minZoom}
          maxZoom={config.maxZoom}
          showControls={config.showControls ?? true}
          showFullscreen={config.showFullscreen ?? true}
          interactive={config.interactive ?? true}
          tileLayer={config.tileLayer || 'osm'}
          deviceBinding={config.deviceBinding}
        />
      )

    case 'custom-layer':
      return (
        <CustomLayer
          {...spreadableProps}
          bindings={config.bindings}
          backgroundType={config.backgroundType || 'grid'}
          backgroundColor={config.backgroundColor}
          backgroundImage={config.backgroundImage}
          gridSize={config.gridSize}
          showControls={config.showControls ?? true}
          showFullscreen={config.showFullscreen ?? true}
          interactive={config.interactive ?? true}
          editable={config.editable}
        />
      )

    // Business Components - handled by ComponentRenderer
    case 'agent-monitor-widget': {
      const widgetAgentId = (component as any).dataSource?.agentId
      return (
        <AgentMonitorWidget
          agentId={widgetAgentId}
          editMode={editMode}
          className="w-full h-full"
        />
      )
    }
    case 'ai-analyst': {
      return (
        <ComponentRenderer
          component={component}
          className="w-full h-full"
          onDataSourceChange={onDataSourceChange}
          onConfigChange={onConfigChange}
          openFullscreen={openFullscreen}
          closeFullscreen={closeFullscreen}
        />
      )
    }
    default: {
      // TS narrows to `never` when all union members are covered above,
      // but runtime may encounter unrecognised types from persisted dashboards.
      const fallback = component as DashboardComponent
      // Always try ComponentRenderer for unknown types — it handles:
      // - Extension components (DynamicRegistry)
      // - Community components (CommunityRegistry)
      // - Late registration (polling mechanism)
      // Falls back to UnknownComponent internally if truly unrecognized.
      return (
        <ComponentRenderer
          component={fallback}
          className="w-full h-full"
          onDataSourceChange={onDataSourceChange}
          onConfigChange={onConfigChange}
          openFullscreen={openFullscreen}
          closeFullscreen={closeFullscreen}
        />
      )
    }
  }
  } catch (error) {
    logError(error, { operation: 'Render dashboard component' })
    return (
      <div className="p-4 text-center text-destructive h-full flex flex-col items-center justify-center hover:bg-muted rounded-lg">
        <p className="text-sm font-medium">{(component as any).type}</p>
        <p className="text-xs mt-1">Error loading component</p>
      </div>
    )
  }
  */
}

// ============================================================================
// Component Wrapper with Edit Mode Actions
// ============================================================================

interface ComponentWrapperProps {
  component: DashboardComponent
  children: React.ReactNode
  editMode: boolean
  onOpenConfig: (componentId: string) => void
  onRemove: (componentId: string) => void
  onDuplicate: (componentId: string) => void
  onSelect?: (component: DashboardComponent | null) => void
  selectedComponentId?: string | null
  isMobile?: boolean
}

// Memoize ComponentWrapper to prevent unnecessary re-renders
// Only re-renders when component.id, editMode, or children reference changes
const ComponentWrapper = memo(function ComponentWrapper({
  component,
  children,
  editMode,
  onOpenConfig,
  onRemove,
  onDuplicate,
  onSelect,
  selectedComponentId,
  isMobile = false,
}: ComponentWrapperProps) {
  const [isHovered, setIsHovered] = useState(false)

  // Use touch hover hook for desktop hover effects
  const { isHovered: isTouchHovered, hoverProps } = useTouchHover({
    enabled: editMode && !isMobile,
  })

  // Memoize event handlers to prevent creating new functions on each render
  const handleMouseEnter = useCallback(() => setIsHovered(true), [])
  const handleMouseLeave = useCallback(() => setIsHovered(false), [])
  const handleConfigClick = useCallback(() => onOpenConfig(component.id), [component.id, onOpenConfig])
  const handleRemoveClick = useCallback(() => onRemove(component.id), [component.id, onRemove])
  const handleDuplicateClick = useCallback(() => onDuplicate(component.id), [component.id, onDuplicate])

  // Mobile: tap edit button to show edit bar
  const handleEditButtonClick = useCallback(() => {
    if (isMobile && editMode && onSelect) {
      onSelect(component)
    }
  }, [isMobile, editMode, onSelect, component])

  // Handle click with stopPropagation for desktop
  const handleEditButtonMouseEvent = useCallback((e: React.MouseEvent) => {
    e.stopPropagation()
    handleEditButtonClick()
  }, [handleEditButtonClick])

  // Handle touch end for mobile - prevent default to avoid ghost clicks
  const handleEditButtonTouchEvent = useCallback((e: React.TouchEvent) => {
    e.preventDefault()
    e.stopPropagation()
    handleEditButtonClick()
  }, [handleEditButtonClick])

  const shouldShowActions = editMode && (isHovered || isTouchHovered) && !isMobile

  return (
    <div
      className={cn(
        'relative h-full transition-all duration-200'
      )}
      {...(!isMobile ? hoverProps : {})}
    >
      {/* Component content */}
      <div className="h-full w-full flex flex-col">
        {children}
      </div>

      {/* Desktop edit mode overlay */}
      {shouldShowActions && (
        <div className="absolute top-2 right-2 z-10 flex gap-1">
          <Button
            variant="secondary"
            size="icon"
            className="h-9 w-9 bg-bg-90 backdrop-blur"
            onClick={handleConfigClick}
          >
            <Settings2 className="h-4 w-4" />
          </Button>
          <Button
            variant="secondary"
            size="icon"
            className="h-9 w-9 bg-bg-90 backdrop-blur"
            onClick={handleDuplicateClick}
          >
            <Copy className="h-4 w-4" />
          </Button>
          <Button
            variant="secondary"
            size="icon"
            className="h-9 w-9 bg-bg-90 backdrop-blur hover:bg-destructive hover:text-destructive-foreground transition-colors"
            onClick={handleRemoveClick}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      )}

      {/* Mobile edit button - top right corner */}
      {isMobile && editMode && (
        <button
          onClick={handleEditButtonMouseEvent}
          onTouchEnd={handleEditButtonTouchEvent}
          className="absolute top-2 right-2 z-50 flex items-center justify-center min-w-[44px] min-h-[44px] rounded-xl bg-bg-90 backdrop-blur text-muted-foreground border border-border shadow-sm transition-all duration-200 active:scale-95 cursor-pointer select-none"
          style={{ touchAction: 'manipulation' }}
        >
          <Settings2 className="w-5 h-5" />
        </button>
      )}
    </div>
  )
})

// ============================================================================
// BindingDataSourceSelector — reusable inline data source picker for Display tab
// ============================================================================

function BindingDataSourceSelector({
  dataSource,
  onConfirm,
  allowedTypes,
  multiple = true,
  maxSources,
  title,
}: {
  dataSource?: DataSourceOrList
  onConfirm: (ds: DataSourceOrList | undefined) => void
  allowedTypes: string[]
  multiple?: boolean
  maxSources?: number
  title: string
}) {
  const { t } = useTranslation('dashboardComponents')
  const [pickerOpen, setPickerOpen] = useState(false)
  const [stagedDataSource, setStagedDataSource] = useState<DataSourceOrList | undefined>(undefined)

  const normalizedSources = dataSource ? normalizeDataSource(dataSource) : []
  const isBound = normalizedSources.length > 0

  const openPicker = useCallback(() => {
    setStagedDataSource(dataSource)
    setPickerOpen(true)
  }, [dataSource])

  const handleConfirm = useCallback(() => {
    onConfirm(stagedDataSource)
    setPickerOpen(false)
  }, [onConfirm, stagedDataSource])

  const handleCancel = useCallback(() => {
    setPickerOpen(false)
  }, [])

  const stagedSources = stagedDataSource ? normalizeDataSource(stagedDataSource) : []
  const stagedChanged = isBound
    ? JSON.stringify(normalizedSources) !== JSON.stringify(stagedSources)
    : stagedSources.length > 0

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <Label className="text-sm font-medium">{title}</Label>
        {isBound && (
          <span className="inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full bg-primary/10 text-primary">
            <Database className="h-3 w-3" />
            {t('bindingSelector.boundCount', { count: normalizedSources.length })}
          </span>
        )}
      </div>

      {isBound ? (
        <Button variant="outline" onClick={openPicker} className="w-full h-9">
          {t('bindingSelector.changeSource')}
        </Button>
      ) : (
        <Button
          variant="outline"
          onClick={openPicker}
          className="w-full h-10 border-dashed text-muted-foreground hover:text-primary"
        >
          <Plus className="h-4 w-4 mr-1.5" />
          {t('bindingSelector.addSource')}
        </Button>
      )}

      <Dialog open={pickerOpen} onOpenChange={(open) => { if (!open) handleCancel() }}>
        <DialogContent className="z-[110] max-w-2xl !h-[70vh] flex flex-col !p-0 overflow-hidden">
          <DialogHeader className="px-5 py-3 border-b shrink-0">
            <DialogTitle className="text-base">{t('dualMode.selectDataSource')}</DialogTitle>
          </DialogHeader>

          <div className="flex-1 min-h-0 overflow-hidden">
            <UnifiedDataSourceConfig
              value={stagedDataSource}
              onChange={setStagedDataSource}
              allowedTypes={allowedTypes as any}
              multiple={multiple}
              maxSources={maxSources}
              className="border-0 h-full"
            />
          </div>

          <DialogFooter className="px-5 py-3 border-t shrink-0 bg-background">
            <Button variant="outline" onClick={handleCancel}>
              {t('bindingSelector.cancel')}
            </Button>
            <Button onClick={handleConfirm} disabled={!stagedChanged}>
              {t('bindingSelector.confirm')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

// ============================================================================
// Main Component
// ============================================================================

// Performance optimization: Memoize VisualDashboard to prevent unnecessary re-renders
// Only re-renders when dashboardId, editMode, or currentDashboard.id changes
const VisualDashboardMemo = memo(function VisualDashboard() {
  const { dashboardId } = useParams<{ dashboardId?: string }>()
  const navigate = useNavigate()
  const { t, i18n } = useTranslation('dashboardComponents')
  const { handleError } = useErrorHandler()

  // Dashboard state — split subscriptions to avoid cascade:
  // `devices` changes every 3s (batch polling) but rarely affects layout.
  // Keep it in a separate selector so the main block doesn't re-render on every batch.
  const {
    currentDashboard,
    currentDashboardId,
    dashboards,
    dashboardsLoading,
    editMode,
    componentLibraryOpen,
  } = useStore((s) => ({
    currentDashboard: s.currentDashboard,
    currentDashboardId: s.currentDashboardId,
    dashboards: s.dashboards,
    dashboardsLoading: s.dashboardsLoading,
    editMode: s.editMode,
    componentLibraryOpen: s.componentLibraryOpen,
  }), shallow)

  // Subscribe to devices LENGTH only — avoids re-rendering every 3s on batch polling
  // (component data comes from useDataSource, not this prop)
  const devicesLength = useStore((s) => s.devices.length)
  // Read devices directly from store when needed (not reactive)
  const devicesRef = useRef<Device[]>([])

  // Action selectors — single subscription for all actions (stable references)
  const {
    setEditMode, addComponent, updateComponent, batchUpdatePositions,
    removeComponent, duplicateComponent, createDashboard, updateDashboard,
    deleteDashboard, persistDashboard, setCurrentDashboard, setComponentLibraryOpen,
    fetchDashboards, fetchDevices, fetchDeviceTypes, fetchDevicesCurrentBatch,
    sendCommand,
  } = useStore((s) => ({
    setEditMode: s.setEditMode, addComponent: s.addComponent, updateComponent: s.updateComponent,
    batchUpdatePositions: s.batchUpdatePositions, removeComponent: s.removeComponent,
    duplicateComponent: s.duplicateComponent, createDashboard: s.createDashboard,
    updateDashboard: s.updateDashboard, deleteDashboard: s.deleteDashboard,
    persistDashboard: s.persistDashboard, setCurrentDashboard: s.setCurrentDashboard,
    setComponentLibraryOpen: s.setComponentLibraryOpen,
    fetchDashboards: s.fetchDashboards, fetchDevices: s.fetchDevices,
    fetchDeviceTypes: s.fetchDeviceTypes, fetchDevicesCurrentBatch: s.fetchDevicesCurrentBatch,
    sendCommand: s.sendCommand,
  }))

  // Marketplace store selectors — single subscription
  const {
    marketComponents, marketLoading, installed: installedComponents,
    fetchMarket, fetchInstalled, installFromMarket, uninstall: uninstallComponent,
  } = useStore((s) => ({
    marketComponents: s.marketComponents, marketLoading: s.marketLoading,
    installed: s.installed, fetchMarket: s.fetchMarket,
    fetchInstalled: s.fetchInstalled, installFromMarket: s.installFromMarket,
    uninstall: s.uninstall,
  }))

  // Extension lifecycle management for hot updates
  const { refreshVersion } = useExtensionLifecycle({
    autoSyncOnRegister: true,
    autoRemoveOnUnregister: true,
  })

  // Community component lifecycle
  useCommunityComponentLifecycle()

  // Pre-batch data loading: 1 batch request + telemetry cache warm-up
  useDashboardPrefetch(currentDashboard?.components ?? [])

  // Memoize component library with refreshVersion dependency to trigger re-renders
  const componentLibrary = useMemo(() => getComponentLibrary(t), [t, refreshVersion, installedComponents.length])

  // Component library search
  const [librarySearch, setLibrarySearch] = useState('')
  const [libraryTab, setLibraryTab] = useState<'components' | 'marketplace'>('components')
  const [importDialogOpen, setImportDialogOpen] = useState(false)
  const [installingId, setInstallingId] = useState<string | null>(null)

  // Fetch marketplace data when marketplace tab is active
  useEffect(() => {
    if (componentLibraryOpen && libraryTab === 'marketplace') {
      fetchMarket()
    }
  }, [componentLibraryOpen, libraryTab, fetchMarket])

  // Fetch installed components on mount (needed for community registry sync)
  // and when component library opens
  useEffect(() => {
    return scheduleDashboardIdleTask(() => {
      fetchInstalled()
    }, 2500)
  }, [fetchInstalled])

  const filteredLibrary = useMemo(() => {
    if (!librarySearch.trim()) return componentLibrary
    const q = librarySearch.toLowerCase()
    return componentLibrary
      .map(cat => ({
        ...cat,
        items: cat.items.filter(item =>
          item.name.toLowerCase().includes(q) ||
          item.description.toLowerCase().includes(q)
        ),
      }))
      .filter(cat => cat.items.length > 0)
  }, [componentLibrary, librarySearch])

  const [configOpen, setConfigOpen] = useState(false)
  const [selectedComponent, setSelectedComponent] = useState<DashboardComponent | null>(null)

  // Extension fullscreen dialog state
  const [extFullscreenContent, setExtFullscreenContent] = useState<React.ReactNode | null>(null)

  const openExtFullscreen = useCallback((content: React.ReactNode) => {
    setExtFullscreenContent(content)
  }, [])
  const closeExtFullscreen = useCallback(() => {
    setExtFullscreenContent(null)
  }, [])

  // Mobile editing state
  const isMobile = useIsMobile()
  const isDesktop = !isMobile
  const [mobileSelectedId, setMobileSelectedId] = useState<string | null>(null)
  const [mobileEditBarOpen, setMobileEditBarOpen] = useState(false)

  // Map editor dialog state
  const [mapEditorOpen, setMapEditorOpen] = useState(false)
  const [mapEditorBindings, setMapEditorBindings] = useState<MapBinding[]>([])

  // Center picker dialog state
  const [centerPickerOpen, setCenterPickerOpen] = useState(false)

  // Layer editor dialog state
  const [layerEditorOpen, setLayerEditorOpen] = useState(false)
  const [layerEditorBindings, setLayerEditorBindings] = useState<LayerBinding[]>([])

  // Agents for agent-monitor-widget config (summaries only)
  const [agents, setAgents] = useState<{ id: string; name: string; status: string }[]>([])
  const [agentsLoading, setAgentsLoading] = useState(false)

  // Vision models for ai-analyst config
  const [visionModels, setVisionModels] = useState<{ id: string; name: string; backendId: string; backendName: string }[]>([])
  const [visionModelsLoading, setVisionModelsLoading] = useState(false)

  // Fullscreen state
  const [isFullscreen, setIsFullscreen] = useState(false)

  // Share dialog state
  const [shareDialogOpen, setShareDialogOpen] = useState(false)

  // Persist sidebar state to localStorage (default to closed on mobile, open on desktop)
  const [sidebarOpen, setSidebarOpen] = useState(() => {
    const saved = localStorage.getItem('neomind_dashboard_sidebar_open')
    if (saved !== null) return saved !== 'false'
    // No saved value - default based on device type
    return window.innerWidth >= 1024 // Default to closed on mobile/tablet, open on desktop
  })

  // Update localStorage when sidebar state changes
  const handleSidebarOpenChange = useCallback((open: boolean) => {
    setSidebarOpen(open)
    localStorage.setItem('neomind_dashboard_sidebar_open', String(open))
  }, [])

  // Fullscreen toggle function - CSS-only fullscreen for dashboard content
  const toggleFullscreen = useCallback(() => {
    setIsFullscreen(prev => !prev)
  }, [])

  // Dashboard list handlers
  const handleDashboardSwitch = useCallback((id: string) => {
    setCurrentDashboard(id)
    // URL will be updated automatically by the Store → URL sync effect
  }, [setCurrentDashboard])

  // Dashboard interaction handlers
  const handleDeviceClick = useCallback(async (deviceId: string) => {
    const device = devicesRef.current.find(d => d.id === deviceId || d.device_id === deviceId)
    if (device) {
      // Navigate to device detail page
      navigate(`/devices/${device.id}`)
    } else {
      toast({
        title: t('visualDashboard.deviceNotFound'),
        description: t('visualDashboard.deviceNotFoundDesc'),
        variant: 'destructive',
      })
    }
  }, [navigate, t])

  const handleMetricClick = useCallback(async (metricId: string, deviceId?: string) => {
    // Show metric info in toast
    toast({
      title: t('visualDashboard.metricInfo'),
      description: `${t('visualDashboard.metric')}: ${metricId}${deviceId ? `\n${t('visualDashboard.device')}: ${deviceId.slice(0, 8)}...` : ''}`,
    })
  }, [t])

  const handleCommandClick = useCallback(async (deviceId: string, command: string) => {
    try {
      const success = await sendCommand(deviceId, command)
      if (success) {
        toast({
          title: t('visualDashboard.commandSent'),
          description: `${t('visualDashboard.command')}: ${command}\n${t('visualDashboard.device')}: ${deviceId.slice(0, 8)}...`,
        })
      } else {
        toast({
          title: t('visualDashboard.commandFailed'),
          description: `${t('visualDashboard.command')}: ${command}`,
          variant: 'destructive',
        })
      }
    } catch (error) {
      handleError(error, t('visualDashboard.commandError'))
    }
  }, [sendCommand, t, handleError])

  const handleDashboardCreate = useCallback(async (name: string) => {
    const newId = await createDashboard({
      name,
      layout: {
        columns: 12,
        rows: 'auto' as const,
        breakpoints: { lg: 1200, md: 996, sm: 768, xs: 480 },
      },
      components: [],
    })
    // Navigate to the new dashboard
    if (newId) {
      navigate(`/visual-dashboard/${newId}`, { replace: true })
    }
  }, [createDashboard, navigate])

  const handleDashboardRename = useCallback((id: string, name: string) => {
    updateDashboard(id, { name })
  }, [updateDashboard])

  const handleDashboardDelete = useCallback(async (id: string) => {
    const confirmed = await confirm({
      title: 'Delete Dashboard',
      description: 'Delete this dashboard?',
      confirmText: 'Delete',
      cancelText: 'Cancel',
      variant: 'destructive'
    })
    if (confirmed) {
      deleteDashboard(id)
    }
  }, [deleteDashboard])

  // Config dialog state
  const [configTitle, setConfigTitle] = useState('')
  const [componentConfig, setComponentConfig] = useState<Record<string, any>>({})
  const [configSchema, setConfigSchema] = useState<ComponentConfigSchema | null>(null)

  // Store original config for revert on cancel
  const [originalComponentConfig, setOriginalComponentConfig] = useState<Record<string, any>>({})
  const [originalTitle, setOriginalTitle] = useState('')

  // Track if we've initialized to avoid duplicate calls
  const hasInitialized = useRef(false)

  // Track URL sync direction to avoid circular updates
  const isSyncingFromUrl = useRef(false)
  const isSyncingFromStore = useRef(false)

  // Track previous components to detect actual changes (not just reference changes)
  // Create a stable key for components to detect actual changes
  // This key only changes when component data actually changes, not on every render
  const componentsStableKey = useMemo(() => {
    const components = currentDashboard?.components ?? []
    // Use JSON.stringify for a reliable content-based comparison.
    // This is cheaper than it looks — components array is typically <20 items.
    return JSON.stringify(components.map((c) => ({
      id: c.id,
      type: c.type,
      title: c.title,
      position: c.position,
      config: c.config,
      dataSource: (c as any).dataSource,
    })))
  }, [currentDashboard])

  // Initialize dashboards on mount
  useEffect(() => {
    if (hasInitialized.current) return
    hasInitialized.current = true

    // Fetch dashboards first so the shell and saved layout can paint quickly.
    fetchDashboards()

    // Device metadata is needed for bindings, but it should not compete with
    // the first dashboard paint in Tauri/WKWebView.
    const cancelIdleFetch = scheduleDashboardIdleTask(() => {
      fetchDevices()
      fetchDeviceTypes()
    }, 500)

    return cancelIdleFetch
  }, [fetchDashboards, fetchDevices, fetchDeviceTypes])

  // Retry device fetching when devices are empty (backend DB may still be loading)
  // Max 10 retries (30s) to avoid polling forever when no devices exist
  useEffect(() => {
    // Only retry if we have dashboard components that need device data
    if (!currentDashboard || currentDashboard.components.length === 0) return
    if (devicesLength > 0) return
    if (dashboardsLoading) return

    let attempts = 0
    const MAX_ATTEMPTS = 10
    const interval = setInterval(() => {
      if (attempts >= MAX_ATTEMPTS) {
        clearInterval(interval)
        return
      }
      attempts++
      fetchDevices()
    }, 5000)

    return () => clearInterval(interval)
  }, [devicesLength, currentDashboard, dashboardsLoading, fetchDevices])

  // Batch fetch current values for devices used in dashboard components
  // Only considers the CURRENT dashboard (not all dashboards) to avoid
  // re-fetching when switching between dashboards with different devices.
  const dashboardDeviceIdsKey = useMemo(() => {
    if (!currentDashboard) return ''
    const deviceIds = new Set<string>()
    for (const component of currentDashboard.components) {
      const genericComponent = component as GenericComponent
      const dataSource = genericComponent.dataSource
      if (dataSource && getSourceId(dataSource)) {
        deviceIds.add(getSourceId(dataSource)!)
      }
      if (genericComponent.type === 'map-display') {
        const bindings = (genericComponent.config as any)?.bindings as MapBinding[] || []
        for (const binding of bindings) {
          const ds = binding.dataSource as DataSource | undefined
          if (ds && getSourceId(ds)) {
            deviceIds.add(getSourceId(ds)!)
          }
        }
      }
    }
    return Array.from(deviceIds).sort().join(',')
  }, [currentDashboard])

  // Fetch batch current values when device set changes.
  // Strategy: initial fetch → up to 3 fast retries (2s) for missing data →
  // switch to slow refresh (120s) for real-time updates. Stops early if all data arrives.
  const batchFetchControllerRef = useRef<{ deviceIds: string[]; interval: ReturnType<typeof setInterval> | null }>({ deviceIds: [], interval: null })
  const batchAbortRef = useRef<AbortController | null>(null)

  useEffect(() => {
    if (devicesLength === 0 || !dashboardDeviceIdsKey) return

    const deviceIds = dashboardDeviceIdsKey.split(',').filter(Boolean)
    if (deviceIds.length === 0) return

    // Abort any in-flight requests from previous effect run
    batchAbortRef.current?.abort()
    const abortController = new AbortController()
    batchAbortRef.current = abortController

    // Clear previous polling if device set changed
    const ctrl = batchFetchControllerRef.current
    if (ctrl.interval) {
      clearInterval(ctrl.interval)
      ctrl.interval = null
    }
    ctrl.deviceIds = deviceIds

    // Initial fetch
    fetchDevicesCurrentBatch(deviceIds, abortController.signal)

    // Phase 1: Fast retries for missing telemetry (up to 3 attempts at 2s intervals)
    let fastRetries = 0
    const FAST_RETRY_MAX = 3
    const FAST_RETRY_MS = 2000
    // Phase 2: Slow background refresh (120s)
    const SLOW_REFRESH_MS = 120_000

    const checkAndRefresh = () => {
      if (abortController.signal.aborted) return

      const freshDevices = useStore.getState().devices
      const ids = new Set(deviceIds)
      const stillMissing = freshDevices.some((d) =>
        ids.has(d.id || d.device_id) &&
        (!d.current_values || Object.keys(d.current_values).length === 0)
      )

      if (stillMissing && fastRetries < FAST_RETRY_MAX) {
        // Phase 1: fast retry for missing data
        fastRetries++
        fetchDevicesCurrentBatch(deviceIds, abortController.signal)
      } else {
        // All data arrived or fast retries exhausted — switch to slow refresh
        if (ctrl.interval) {
          clearInterval(ctrl.interval)
        }
        ctrl.interval = setInterval(checkAndRefresh, SLOW_REFRESH_MS)
        // Fetch once more at the transition point
        fetchDevicesCurrentBatch(deviceIds, abortController.signal)
      }
    }

    // Start with fast retry interval
    ctrl.interval = setInterval(checkAndRefresh, FAST_RETRY_MS)

    return () => {
      abortController.abort()
      if (ctrl.interval) {
        clearInterval(ctrl.interval)
        ctrl.interval = null
      }
    }
  }, [devicesLength, dashboardDeviceIdsKey, fetchDevicesCurrentBatch])

  // Fix 3: Update devicesRef only when devices actually change (not on every render)
  useEffect(() => {
    devicesRef.current = useStore.getState().devices
  }, [devicesLength])

  // Fix 2: On dashboard switch, abort in-flight batch requests and clear polling
  useEffect(() => {
    // Abort any stale batch requests
    batchAbortRef.current?.abort()
    const ctrl = batchFetchControllerRef.current
    if (ctrl.interval) {
      clearInterval(ctrl.interval)
      ctrl.interval = null
    }
  }, [currentDashboardId])

  // Re-load dashboards if array becomes empty but we have a current ID
  useEffect(() => {
    if (dashboards.length === 0 && currentDashboardId) {
      // Try to recover by fetching again
      fetchDashboards()
    }
  }, [dashboards.length, currentDashboardId, fetchDashboards])

  // ==========================================================================
  // URL - Store Sync for Dashboard Sharing
  // ==========================================================================

  // Sync URL → Store: When URL changes (dashboardId changes), load that dashboard
  // Note: Deliberately excluded currentDashboardId from deps to avoid circular sync
  useEffect(() => {
    // Skip if dashboards aren't loaded yet or if we're syncing from store
    if (dashboards.length === 0 || isSyncingFromStore.current) return

    // If URL has a dashboardId
    if (dashboardId) {
      // Check if the dashboard exists
      const exists = dashboards.some(d => d.id === dashboardId)

      if (exists && dashboardId !== currentDashboardId) {
        // URL has valid dashboardId and differs from current, switch to it
        isSyncingFromUrl.current = true
        setCurrentDashboard(dashboardId)
        // Reset flag after state update completes
        setTimeout(() => { isSyncingFromUrl.current = false }, 50)
      } else if (!exists && currentDashboardId) {
        // URL has invalid dashboardId, redirect to current dashboard
        navigate(`/visual-dashboard/${currentDashboardId}`, { replace: true })
      } else if (!exists && dashboards.length > 0) {
        // URL has invalid dashboardId and no current dashboard, redirect to first/default
        const defaultDashboard = dashboards.find(d => d.isDefault) || dashboards[0]
        navigate(`/visual-dashboard/${defaultDashboard.id}`, { replace: true })
      }
    } else if (currentDashboardId) {
      // No dashboardId in URL but we have one in store, update URL
      // This handles initial load from localStorage
      isSyncingFromStore.current = true
      navigate(`/visual-dashboard/${currentDashboardId}`, { replace: true })
      setTimeout(() => { isSyncingFromStore.current = false }, 50)
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dashboardId, dashboards]) // Intentionally exclude currentDashboardId

  // Sync Store → URL: When store currentDashboardId changes, update URL
  // This handles dashboard switching via sidebar
  useEffect(() => {
    // Skip if dashboards aren't loaded yet or if we're syncing from URL
    if (dashboards.length === 0 || isSyncingFromUrl.current) return

    // Only update URL if it's different from current URL
    if (currentDashboardId && currentDashboardId !== dashboardId) {
      isSyncingFromStore.current = true
      navigate(`/visual-dashboard/${currentDashboardId}`, { replace: true })
      setTimeout(() => { isSyncingFromStore.current = false }, 50)
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentDashboardId]) // Only depend on currentDashboardId, not dashboardId

  // Handle edge case: no dashboards exist yet
  useEffect(() => {
    if (dashboards.length === 0 && !dashboardsLoading && !dashboardId) {
      // Stay at /visual-dashboard without an ID - let user create first dashboard
      navigate('/visual-dashboard', { replace: true })
    }
  }, [dashboards.length, dashboardsLoading, dashboardId, navigate])

  // Load agents — preload on mount, refresh when config opens for agent-monitor-widget
  useEffect(() => {
    const loadAgents = async () => {
      if (!fetchCache.shouldFetch('agents-list')) return
      fetchCache.markFetching('agents-list')
      setAgentsLoading(true)
      try {
        const data = await api.listAgentSummaries()
        setAgents(data.agents || [])
        fetchCache.markFetched('agents-list')
      } catch (error) {
        handleError(error, { operation: 'Load agents for dashboard', showToast: false })
        setAgents([])
        fetchCache.invalidate('agents-list')
      } finally {
        setAgentsLoading(false)
      }
    }
    // Preload on mount, or reload when config opens for agent-monitor-widget
    if (!configOpen || selectedComponent?.type === 'agent-monitor-widget') {
      loadAgents()
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [configOpen, selectedComponent?.type])

  // NOTE: agents are accessed directly via the `agents` state variable in
  // generateConfigSchema closure — no need to inject them into componentConfig.

  // Load vision models when config opens for ai-analyst
  useEffect(() => {
    const loadVisionModels = async () => {
      if (configOpen && selectedComponent?.type === 'ai-analyst') {
        setVisionModelsLoading(true)
        try {
          const resp = await api.listLlmBackends()
          const backends = resp?.backends || []
          const models: { id: string; name: string; backendId: string; backendName: string }[] = []
          for (const backend of backends) {
            if (backend.capabilities?.supports_multimodal) {
              models.push({
                id: backend.id,
                name: backend.name || backend.model,
                backendId: backend.id,
                backendName: backend.name || backend.id,
              })
            }
          }
          setVisionModels(models)
        } catch (error) {
          handleError(error, { operation: 'Load vision models for dashboard', showToast: false })
          setVisionModels([])
        } finally {
          setVisionModelsLoading(false)
        }
      }
    }
    loadVisionModels()
  }, [configOpen, selectedComponent?.type, selectedComponent?.id])

  // NOTE: visionModels are accessed directly via the `visionModels` state variable in
  // generateConfigSchema closure — no need to inject them into componentConfig.

  // Note: Removed auto-create dashboard logic
  // Users should explicitly create dashboards via the UI
  // This prevents creating duplicate dashboards on refresh

  // Handle adding a component
  const handleAddComponent = (componentType: string) => {
    const item = componentLibrary
      .flatMap(cat => cat.items)
      .find(i => i.id === componentType)

    // Get size constraints for this component type
    let constraints = COMPONENT_SIZE_CONSTRAINTS[componentType as keyof typeof COMPONENT_SIZE_CONSTRAINTS]

    // For extension components, get constraints from DTO
    if (!constraints) {
      const extensionDto = dynamicRegistry.getMeta(componentType)
      if (extensionDto?.size_constraints) {
        constraints = extensionDto.size_constraints as any
      }
    }

    // Build appropriate default config based on component type
    let defaultConfig: any = {}

    // For extension components, use their default config from DTO
    const extensionDto = dynamicRegistry.getMeta(componentType)
    if (extensionDto?.default_config) {
      defaultConfig = { ...extensionDto.default_config }
    }

    // For extension components, set up dataSource with extensionId
    let dataSource: any = undefined
    if (extensionDto?.extension_id) {
      dataSource = {
        type: 'extension' as const,
        extensionId: extensionDto.extension_id,
      }
      // Also add extensionMetric if available from data_binding
      if (extensionDto.data_binding?.extension_metric) {
        dataSource.extensionMetric = extensionDto.data_binding.extension_metric
      }
    }

    switch (componentType) {
      // Charts
      case 'line-chart':
      case 'area-chart':
        defaultConfig = {
          series: [{ name: 'Value', data: [10, 25, 15, 30, 28, 35, 20], color: chartColorsHex[0] }],
          labels: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
        }
        break
      case 'bar-chart':
        defaultConfig = {
          data: [{ name: 'A', value: 30 }, { name: 'B', value: 50 }, { name: 'C', value: 20 }]
        }
        break
      case 'pie-chart':
        defaultConfig = {
          data: [{ name: 'A', value: 30 }, { name: 'B', value: 50 }, { name: 'C', value: 20 }]
        }
        break
      // Indicators
      case 'sparkline':
        defaultConfig = {
          data: [12, 19, 15, 25, 22, 30, 28]
        }
        break
      case 'progress-bar':
        defaultConfig = {
          value: 65,
          min: 0,
          max: 100
        }
        break
      case 'led-indicator':
        defaultConfig = {
          rules: []
        }
        break
      // Controls
      case 'toggle-switch':
        defaultConfig = {
          size: 'md'
        }
        break
      // Display & Content
      case 'image-display':
        defaultConfig = {
          src: 'https://via.placeholder.com/400x200',
          alt: 'Sample Image',
          fit: 'contain',
          rounded: true,
          zoomable: true,
        }
        break
      case 'image-history':
        defaultConfig = {
          dataSource: undefined,
          fit: 'fill',
          rounded: true,
          limit: 50,
          timeRange: 24,
        }
        break
      case 'web-display':
        defaultConfig = {
          src: 'https://example.com',
          title: 'Website',
          sandbox: true,
          showHeader: true,
        }
        break
      case 'markdown-display':
        defaultConfig = {
          content: '# Title\n\nThis is **markdown** content.\n\n- Item 1\n- Item 2\n\n`code example`',
          variant: 'default',
        }
        break
      case 'video-display':
        defaultConfig = {
          src: '',
          type: 'file',
          autoplay: false,
          muted: true,
          controls: true,
          loop: false,
          fit: 'contain',
          rounded: true,
          showFullscreen: true,
        }
        break
      case 'map-display':
        defaultConfig = {
          center: { lat: 39.9042, lng: 116.4074 }, // Beijing
          zoom: 10,
          minZoom: 2,
          maxZoom: 18,
          showControls: true,
          showLayers: true,
          showFullscreen: true,
          interactive: true,
          tileLayer: 'osm',
          markers: [
            { id: '1', latitude: 39.9042, longitude: 116.4074, label: 'Beijing', status: 'online' },
            { id: '2', latitude: 31.2304, longitude: 121.4737, label: 'Shanghai', status: 'online' },
            { id: '3', latitude: 23.1291, longitude: 113.2644, label: 'Guangzhou', status: 'warning' },
          ],
        }
        break
      case 'custom-layer':
        defaultConfig = {
          backgroundType: 'grid',
          gridSize: 20,
          showControls: true,
          showFullscreen: true,
          interactive: true,
          editable: false,
        }
        break
      // Business Components
      case 'agent-monitor-widget':
      case 'ai-analyst':
        defaultConfig = {}
        break
      default:
        // For extension components, keep the default_config from DTO (already set above)
        if (Object.keys(defaultConfig).length === 0) {
          defaultConfig = {}
        }
    }

    const w = constraints?.defaultW ?? 4
    const h = constraints?.defaultH ?? 3

    // Calculate y position based on existing components.
    // We use the bottom edge of the lowest existing component to place
    // the new component below everything currently visible.
    const existingComponents = currentDashboard?.components ?? []
    let maxY = 0
    for (const c of existingComponents) {
      const bottom = (c.position?.y ?? 0) + (c.position?.h ?? 1)
      if (bottom > maxY) maxY = bottom
    }

    const newComponent: Omit<DashboardComponent, 'id'> = {
      type: componentType as any,
      position: {
        x: 0,
        y: maxY,
        w,
        h,
        minW: constraints?.minW,
        minH: constraints?.minH,
        maxW: constraints?.maxW,
        maxH: constraints?.maxH,
      },
      title: item?.name || componentType,
      config: defaultConfig,
      ...(dataSource && { dataSource }),
    }

    addComponent(newComponent)
    setComponentLibraryOpen(false)
  }

  // Handle layout change - batch update all changed positions in a single store update
  // to prevent infinite re-render loops (store update → grid recalc → onLayoutChange → ...)
  const handleLayoutChange = useCallback((layout: readonly any[]) => {
    const components = currentDashboard?.components ?? []
    const changed: Array<{ id: string; position: { x: number; y: number; w: number; h: number } }> = []

    for (const item of layout) {
      const existing = components.find(c => c.id === item.i)
      if (!existing) continue
      if (
        existing.position.x !== item.x ||
        existing.position.y !== item.y ||
        existing.position.w !== item.w ||
        existing.position.h !== item.h
      ) {
        changed.push({
          id: item.i,
          position: { x: item.x, y: item.y, w: item.w, h: item.h },
        })
      }
    }

    if (changed.length > 0) {
      batchUpdatePositions(changed)
    }
  }, [currentDashboard?.components, batchUpdatePositions])

  // Handle opening config dialog
  const handleOpenConfig = useCallback((componentId: string) => {
    const component = currentDashboard?.components.find(c => c.id === componentId)
    if (!component) return

    setSelectedComponent(component)
    // Extract both config and dataSource (they are separate properties on GenericComponent)
    const config = { ...((component as any).config || {}) }
    const dataSource = (component as any).dataSource
    // Include title in config so style sections can access it
    const configWithTitle = { ...config, title: component.title }
    // Merge dataSource into config for unified state management
    const mergedConfig = dataSource ? { ...configWithTitle, dataSource } : configWithTitle

    // Store original config for revert on cancel
    setOriginalComponentConfig(mergedConfig)
    setOriginalTitle(component.title || '')

    setConfigTitle(component.title || '')
    setComponentConfig(mergedConfig)
    setConfigOpen(true)
  }, [currentDashboard?.components])

  // ============================================================================
  // Unified Select Field Helper
  // ============================================================================

  interface SelectOption {
    value: string
    label: string
  }

  interface SelectFieldProps {
    label: string
    value: string
    onChange: (value: string) => void
    options: SelectOption[]
    className?: string
  }

  // SelectField component - NOT memoized to ensure it receives fresh props
  function SelectField({ label, value, onChange, options, className }: SelectFieldProps) {
    const handleChange = (newValue: string) => {
      onChange(newValue)
    }
    return (
      <Field className={className}>
        <Label>{label}</Label>
        <Select value={value} onValueChange={handleChange}>
          <SelectTrigger>
            <SelectValue placeholder={t('visualDashboard.selectPlaceholder', { label })} />
          </SelectTrigger>
          <SelectContent>
            {options.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
    )
  }

  // ImageSourceField component for image upload support - NOT memoized to ensure fresh props
  interface ImageSourceFieldProps {
    value: string
    onChange: (value: string) => void
  }

  function ImageSourceField({ value, onChange }: ImageSourceFieldProps) {
    const fileInputRef = useRef<HTMLInputElement>(null)

    const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0]
      if (!file) return

      if (!file.type.startsWith('image/')) {
        console.error('Selected file is not an image')
        return
      }

      if (file.size > 10 * 1024 * 1024) {
        console.error('Image file is too large (max 10MB)')
        return
      }

      const reader = new FileReader()
      reader.onload = (event) => {
        const base64 = event.target?.result as string
        if (base64) {
          onChange(base64)
        }
      }
      reader.readAsDataURL(file)
      // Reset input to allow re-uploading the same file
      e.target.value = ''
    }

    const handleUploadClick = () => {
      fileInputRef.current?.click()
    }

    const handleClear = () => {
      onChange('')
    }

    const isBase64Image = value?.startsWith('data:image')

    return (
      <div className="space-y-3">
        <Field>
          <Label>{t('visualDashboard.imageSource')}</Label>
          <div className="flex gap-2">
            <Input
              value={value || ''}
              onChange={(e) => onChange(e.target.value)}
              placeholder={t('visualDashboard.urlPlaceholder')}
              className="h-9 flex-1"
            />
            <Button
              variant="outline"
              size="sm"
              onClick={handleUploadClick}
              className="h-9 px-3 shrink-0"
            >
              <Upload className="h-4 w-4 mr-1.5" />
              {t('visualDashboard.upload')}
            </Button>
          </div>
          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            onChange={handleFileSelect}
            className="hidden"
          />
          <p className="text-xs text-muted-foreground mt-1">
            {isBase64Image
              ? t('visualDashboard.uploadedHint')
              : t('visualDashboard.urlHint')}
          </p>
        </Field>

        {isBase64Image && (
          <div className="flex items-center gap-2">
            <div className="w-12 h-12 rounded border overflow-hidden bg-muted-30">
              <img
                src={value}
                alt="Preview"
                className="w-full h-full object-contain"
              />
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClear}
              className="h-8 text-destructive hover:text-destructive"
            >
              <Trash2 className="h-4 w-4 mr-1" />
              {t('visualDashboard.clear')}
            </Button>
          </div>
        )}
      </div>
    )
  }

  // Memoize grid components to prevent infinite re-renders
  // Only recalculate when actual component data changes (detected via stableKey)
  // Note: handleOpenConfig, removeComponent, duplicateComponent are NOT dependencies
  // because they don't affect the rendered output structure, only event handlers
  // devices.length is deliberately excluded from deps — it changes every 3s due to
  // batch polling and is NOT used in this callback (component data comes via useDataSource).
  // IMPORTANT: Use currentDashboard from props (reactive) to ensure updates are reflected
  const gridComponents = useMemo(() => {
    return (currentDashboard?.components ?? []).map((component) => {
      // Get dataSource from component (it should be a separate property, not in config)
      const componentDataSource = (component as any).dataSource

      // Create callbacks for this component to persist configuration changes
      const handleDataSourceChange = (newDataSource: any) => {
        updateComponent(component.id, { dataSource: newDataSource as DataSource }, false)
      }

      const handleConfigChange = (newConfig: Record<string, any>) => {
        updateComponent(component.id, { config: newConfig }, false)
      }

      return {
        id: component.id,
        position: component.position,
        children: (
          <ComponentWrapper
            key={component.id}
            component={component}
            editMode={editMode}
            onOpenConfig={handleOpenConfig}
            onRemove={removeComponent}
            onDuplicate={duplicateComponent}
            onSelect={isMobile ? (comp) => {
              if (comp) {
                setMobileSelectedId(comp.id)
                setMobileEditBarOpen(true)
              }
            } : undefined}
            selectedComponentId={mobileSelectedId}
            isMobile={isMobile}
          >
            {renderDashboardComponent(component, editMode, handleDataSourceChange, handleConfigChange, openExtFullscreen, closeExtFullscreen)}
          </ComponentWrapper>
        ),
      }
    }) ?? []
  }, [componentsStableKey, editMode, isMobile])

  // Track initial config load to avoid unnecessary updates
  const initialConfigRef = useRef<any>(null)
  const isInitialLoad = useRef(false)
  const lastSyncedConfigRef = useRef<string>('')

  // Live preview: update component in real-time as config changes
  // Applies changes to the store immediately so the grid preview updates.
  // Schema is regenerated so the dialog's own inputs stay responsive.
  useEffect(() => {
    if (configOpen && selectedComponent) {
      // Skip initial load - don't update store with same config
      if (!isInitialLoad.current) {
        initialConfigRef.current = componentConfig
        isInitialLoad.current = true
        lastSyncedConfigRef.current = JSON.stringify(componentConfig)
        setConfigSchema(generateConfigSchema(selectedComponent.type, componentConfig))
        return
      }

      // Check if config actually changed since last sync
      const currentJSON = JSON.stringify(componentConfig)
      if (currentJSON !== lastSyncedConfigRef.current) {
        // Regenerate schema immediately so input values update
        setConfigSchema(generateConfigSchema(selectedComponent.type, componentConfig))

        // Update last synced config
        lastSyncedConfigRef.current = currentJSON

        // Apply to store immediately for live preview in the grid
        const { dataSource, ...configOnly } = componentConfig
        const currentDS = (selectedComponent as any).dataSource
        const updateData: any = { config: configOnly }
        if (dataSource !== undefined || currentDS !== undefined) {
          updateData.dataSource = dataSource
        }
        updateComponent(selectedComponent.id, updateData, false)
      }
    } else {
      // Reset when dialog closes
      isInitialLoad.current = false
      initialConfigRef.current = null
      lastSyncedConfigRef.current = ''
    }
  }, [componentConfig, configOpen, selectedComponent?.id, selectedComponent?.type, updateComponent, setConfigSchema])

  // Handle canceling component config - revert to original
  const handleCancelConfig = useCallback(() => {
    if (selectedComponent && originalComponentConfig) {
      // Revert to original config (no need to persist - reverting to saved state)
      const { dataSource, ...configOnly } = originalComponentConfig
      const currentDS = (selectedComponent as any).dataSource
      const updateData: any = { config: configOnly }
      // Include dataSource if:
      // 1. Original config had dataSource, OR
      // 2. Original config didn't have dataSource but current component does (need to clear it)
      if (dataSource !== undefined || currentDS !== undefined) {
        updateData.dataSource = dataSource
      }
      updateComponent(selectedComponent.id, updateData, false)

      // Revert title
      if (originalTitle !== selectedComponent.title) {
        updateComponent(selectedComponent.id, { title: originalTitle }, false)
      }
    }
    setConfigOpen(false)
  }, [selectedComponent, originalComponentConfig, originalTitle, updateComponent])

  // Handle saving component config - persist the dashboard to localStorage
  const handleSaveConfig = async () => {
    if (selectedComponent) {
      // Get the latest component from the store to merge with local changes
      const latestDashboard = useStore.getState().currentDashboard
      const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent.id)

      // Extract dataSource from multiple possible locations:
      // 1. componentConfig.dataSource (newly selected/changed in config dialog)
      // 2. componentConfig.config.dataSource (from handleOpenConfig merge)
      // 3. mergedConfig.dataSource (already in merged config)
      // 4. latestComponent.dataSource (existing on component, separate property)
      // 5. latestComponent.config.dataSource (existing in component's config)
      const configDataSource = componentConfig.dataSource
      const nestedConfigDataSource = (componentConfig.config as any)?.dataSource
      const latestConfigDataSource = (latestComponent as any)?.config?.dataSource
      const latestComponentDataSource = (latestComponent as any)?.dataSource

      // Priority: componentConfig.dataSource > nested config.dataSource > latest component.dataSource > latest config.dataSource
      const finalDataSource = configDataSource ?? nestedConfigDataSource ?? latestComponentDataSource ?? latestConfigDataSource

      // Merge local config changes with the latest component config
      // Local changes take precedence
      const mergedConfig = {
        ...(latestComponent as any)?.config || {},
        ...componentConfig,
      }

      // IMPORTANT: Remove dataSource from mergedConfig to avoid confusion
      // dataSource should be stored as a separate property, not inside config
      delete (mergedConfig as any).dataSource

      // Update the component in the store
      // CRITICAL: dataSource must be saved as a separate property, not inside config
      const updateData: any = {
        config: mergedConfig,
        title: configTitle,
      }
      if (finalDataSource !== undefined) {
        updateData.dataSource = finalDataSource
      }

      updateComponent(selectedComponent.id, updateData, false)



      // Verify after update
      setTimeout(() => {
        const verifyDashboard = useStore.getState().currentDashboard
        const verifyComponent = verifyDashboard?.components.find(c => c.id === selectedComponent.id)
        // Component updated successfully
      }, 50)
    }
    // Persist all changes to localStorage
    await persistDashboard()
    setConfigOpen(false)
  }

  // Handle saving map editor bindings
  const handleMapEditorSave = async (bindings: MapBinding[]) => {
    // Fix any duplicate IDs in bindings before saving
    const idCount = new Map<string, number>() as Map<string, number>
    const fixedBindings = bindings.map((binding, index) => {
      const ds = binding.dataSource as any
      const currentId = binding.id
      idCount.set(currentId, (idCount.get(currentId) || 0) + 1)

      // If ID is duplicated, regenerate it
      if (idCount.get(currentId)! > 1) {
        let newId: string
        if (binding.type === 'metric' || ds?.type === 'telemetry') {
          newId = `metric-${getSourceId(ds)}-${ds?.metricId || ds?.property || index}`
        } else if (binding.type === 'command') {
          newId = `command-${getSourceId(ds)}-${ds?.command}`
        } else {
          newId = `device-${getSourceId(ds)}-${index}`
        }
        return { ...binding, id: newId }
      }
      return binding
    })

    if (selectedComponent) {
      // CRITICAL FIX: Get the latest component config from the store to avoid stale state
      const latestDashboard = useStore.getState().currentDashboard
      const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent.id)

      const latestConfig = (latestComponent as any)?.config || {}
      const latestDataSource = (latestComponent as any)?.dataSource

      // Merge the latest config with the new bindings, preserving dataSource
      const newConfig = { ...latestConfig, bindings: fixedBindings }
      const updateData: any = { config: newConfig }

      // CRITICAL: Preserve dataSource when updating
      if (latestDataSource) {
        updateData.dataSource = latestDataSource
      }

      // Update the store with both config and dataSource
      updateComponent(selectedComponent.id, updateData, false)



      // Update local config state
      setComponentConfig(prev => ({ ...prev, bindings: fixedBindings }))

      // Verify after update
      setTimeout(() => {
        const verifyDashboard = useStore.getState().currentDashboard
        const verifyComponent = verifyDashboard?.components.find(c => c.id === selectedComponent.id)
        // Component updated successfully
      }, 50)
    }

    // Persist to localStorage
    await persistDashboard()

    setMapEditorOpen(false)
  }

  // Handle saving layer editor bindings
  const handleLayerEditorSave = async (bindings: LayerBinding[]) => {
    if (selectedComponent) {
      const latestDashboard = useStore.getState().currentDashboard
      const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent.id)

      const latestConfig = (latestComponent as any)?.config || {}
      const latestDataSource = (latestComponent as any)?.dataSource

      // Merge the latest config with the new bindings, preserving dataSource
      const newConfig = { ...latestConfig, bindings }
      const updateData: any = { config: newConfig }

      // Preserve dataSource when updating
      if (latestDataSource) {
        updateData.dataSource = latestDataSource
      }

      // Update the store
      updateComponent(selectedComponent.id, updateData, false)

      // Force re-render

      // Update local config state
      setComponentConfig(prev => ({ ...prev, bindings }))
    }

    // Persist to localStorage
    await persistDashboard()

    setLayerEditorOpen(false)
  }

  // Handle saving center picker
  const handleCenterPickerSave = async (newCenter: { lat: number; lng: number }) => {
    if (selectedComponent) {
      const latestDashboard = useStore.getState().currentDashboard
      const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent.id)

      const latestConfig = (latestComponent as any)?.config || {}
      const latestDataSource = (latestComponent as any)?.dataSource

      // Merge the latest config with the new center, preserving dataSource
      const newConfig = { ...latestConfig, center: newCenter }
      const updateData: any = { config: newConfig }

      // Preserve dataSource when updating
      if (latestDataSource) {
        updateData.dataSource = latestDataSource
      }

      // Update the store
      updateComponent(selectedComponent.id, updateData, false)

      // Force re-render

      // Update local config state
      setComponentConfig(prev => ({ ...prev, center: newCenter }))
    }

    // Persist to localStorage
    await persistDashboard()

    setCenterPickerOpen(false)
  }

  // Handle title change (local state only — store updated via debounced live preview)
  const handleTitleChange = (newTitle: string) => {
    setConfigTitle(newTitle)
  }

  // Generate config schema based on component type
  const generateConfigSchema = (componentType: string, currentConfig: any): ComponentConfigSchema | null => {
    const config = currentConfig || {}

    // Helper to create updater functions
    const updateConfig = (key: string) => (value: any) => {
            if (key === 'title') {
        // Title changes need to sync with configTitle state and the component
        setConfigTitle(value)
        if (selectedComponent) {
          updateComponent(selectedComponent.id, { title: value }, false)
        }
      }
      setComponentConfig(prev => {
        const updated = { ...prev, [key]: value }
                return updated
      })
    }

    const updateNestedConfig = (parent: string, key: string) => (value: any) => {
      setComponentConfig(prev => ({
        ...prev,
        [parent]: { ...prev[parent], [key]: value }
      }))
    }

    // Data source updater
    const updateDataSource = (ds: any) => {
      setComponentConfig(prev => ({ ...prev, dataSource: ds }))
    }

    // Data mapping updater
    const updateDataMapping = (newMapping: any) => {
      setComponentConfig(prev => ({ ...prev, dataMapping: newMapping }))
    }

    switch (componentType) {
      // ========== Indicators ==========
      case 'value-card':
      case 'counter':
      case 'metric-card':
        return {
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.size')}
                    value={config.size || 'md'}
                    onChange={updateConfig('size')}
                    options={[
                      { value: 'sm', label: t('sizes.sm') },
                      { value: 'md', label: t('sizes.md') },
                      { value: 'lg', label: t('sizes.lg') },
                    ]}
                  />

                  <SelectField
                    label={t('visualDashboard.style')}
                    value={config.variant || 'default'}
                    onChange={updateConfig('variant')}
                    options={[
                      { value: 'default', label: t('visualDashboard.default') },
                      { value: 'vertical', label: t('visualDashboard.vertical') },
                      { value: 'compact', label: t('visualDashboard.compact') },
                      { value: 'minimal', label: t('visualDashboard.minimal') },
                    ]}
                  />

                  <EntityIconPicker
                    value={config.icon || ''}
                    onChange={(icon) => updateConfig('icon')(icon)}
                    label={t('visualDashboard.icon')}
                  />

                  <SelectField
                    label={t('visualDashboard.iconType')}
                    value={config.iconType || 'entity'}
                    onChange={updateConfig('iconType')}
                    options={[
                      { value: 'entity', label: 'Entity Icon' },
                      { value: 'class', label: 'Lucide Icon' },
                    ]}
                  />

                  <ColorPicker
                    value={config.iconColor || chartColorsHex[0]}
                    onChange={(color) => updateConfig('iconColor')(color)}
                    label={t('visualDashboard.iconColor')}
                    presets="primary"
                  />

                  <ColorPicker
                    value={config.valueColor || chartColorsHex[0]}
                    onChange={(color) => updateConfig('valueColor')(color)}
                    label={t('visualDashboard.valueColor')}
                    presets="primary"
                  />
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <div className="grid grid-cols-2 gap-3">
                    <Field>
                      <Label>{t('visualDashboard.prefix')}</Label>
                      <Input
                        value={config.prefix || ''}
                        onChange={(e) => updateConfig('prefix')(e.target.value)}
                        placeholder={t('visualDashboard.prefixPlaceholder')}
                        className="h-9"
                      />
                    </Field>

                    <Field>
                      <Label>{t('visualDashboard.unit')}</Label>
                      <Input
                        value={config.unit || ''}
                        onChange={(e) => updateConfig('unit')(e.target.value)}
                        placeholder={t('visualDashboard.unitPlaceholder')}
                        className="h-9"
                      />
                    </Field>
                  </div>

                  <Field>
                    <Label>{t('visualDashboard.description')}</Label>
                    <Input
                      value={config.description || ''}
                      onChange={(e) => updateConfig('description')(e.target.value)}
                      placeholder={t('visualDashboard.descriptionPlaceholder')}
                      className="h-9"
                    />
                  </Field>

                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showTrend ?? false}
                        onCheckedChange={(checked) => updateConfig('showTrend')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showTrend')}</span>
                    </label>
                  </div>
                </div>
              ),
            },
          ],
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-metric', 'system', 'extension', 'transform', 'ai-metric'],
              },
            },
          ],
        }

      case 'sparkline':
        return {
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.colorMode')}
                    value={config.colorMode || 'fixed'}
                    onChange={updateConfig('colorMode')}
                    options={[
                      { value: 'auto', label: t('visualDashboard.auto') },
                      { value: 'primary', label: t('visualDashboard.primaryColor') },
                      { value: 'fixed', label: t('visualDashboard.fixedColor') },
                      { value: 'value', label: t('visualDashboard.basedOnValue') },
                    ]}
                  />

                  {(config.colorMode || 'fixed') === 'fixed' && (
                    <ColorPicker
                      value={config.color || chartColorsHex[0]}
                      onChange={(color) => updateConfig('color')(color)}
                      label={t('visualDashboard.fixedModeColor')}
                      presets="primary"
                    />
                  )}

                  <Field>
                    <Label>{t('visualDashboard.maxValue')}</Label>
                    <Input
                      type="number"
                      value={config.maxValue || 100}
                      onChange={(e) => updateConfig('maxValue')(Number(e.target.value))}
                      min={1}
                      className="h-9"
                    />
                  </Field>

                  <Field>
                    <Label>{t('visualDashboard.lineWidth')}</Label>
                    <Input
                      type="number"
                      value={config.strokeWidth ?? 2}
                      onChange={(e) => updateConfig('strokeWidth')(Number(e.target.value))}
                      min={1}
                      max={5}
                      className="h-9"
                    />
                  </Field>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      checked={config.fill ?? true}
                      onCheckedChange={(checked) => updateConfig('fill')(!!checked)}
                    />
                    <span className="text-sm">{t('visualDashboard.fillArea')}</span>
                  </label>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      checked={config.curved ?? true}
                      onCheckedChange={(checked) => updateConfig('curved')(!!checked)}
                    />
                    <span className="text-sm">{t('visualDashboard.curved')}</span>
                  </label>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      checked={config.showValue ?? true}
                      onCheckedChange={(checked) => updateConfig('showValue')(!!checked)}
                    />
                    <span className="text-sm">{t('visualDashboard.showCurrentValue')}</span>
                  </label>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      checked={config.showThreshold ?? false}
                      onCheckedChange={(checked) => updateConfig('showThreshold')(!!checked)}
                    />
                    <span className="text-sm">{t('visualDashboard.showThreshold')}</span>
                  </label>

                  {config.showThreshold && (
                    <>
                      <Field>
                        <Label>{t('visualDashboard.threshold')}</Label>
                        <Input
                          type="number"
                          value={config.threshold ?? 20}
                          onChange={(e) => updateConfig('threshold')(Number(e.target.value))}
                          className="h-9"
                        />
                      </Field>

                      <ColorPicker
                        value={config.thresholdColor || chartColorsHex[3]}
                        onChange={(color) => updateConfig('thresholdColor')(color)}
                        label={t('visualDashboard.thresholdColor')}
                        presets="semantic"
                      />
                    </>
                  )}
                </div>
              ),
            },
          ],
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-metric', 'system', 'extension', 'transform', 'ai-metric'],
              },
            },
          ],
        }

      case 'progress-bar':
        return {
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.variant')}
                    value={config.variant || 'default'}
                    onChange={updateConfig('variant')}
                    options={[
                      { value: 'default', label: t('visualDashboard.default') },
                      { value: 'icon', label: t('visualDashboard.icon') },
                      { value: 'circular', label: t('visualDashboard.circular') },
                    ]}
                  />

                  {/* Icon variant options */}
                  {config.variant === 'icon' && (
                    <>
                      <IconPicker
                        value={config.icon || ''}
                        onChange={(iconName) => updateConfig('icon')(iconName || undefined)}
                        label={t('visualDashboard.selectIcon')}
                      />

                      <div className="grid grid-cols-2 gap-3">
                        <ColorPicker
                          value={config.iconColor || ''}
                          onChange={(color) => updateConfig('iconColor')(color || undefined)}
                          label={t('visualDashboard.iconColor')}
                          presets="primary"
                        />
                        <ColorPicker
                          value={config.backgroundColor || ''}
                          onChange={(color) => updateConfig('backgroundColor')(color || undefined)}
                          label={t('visualDashboard.backgroundColor')}
                          presets="neutral"
                        />
                      </div>
                    </>
                  )}

                  {/* Non-icon variants: custom color */}
                  {config.variant !== 'icon' && (
                    <ColorPicker
                      value={config.color || ''}
                      onChange={(color) => updateConfig('color')(color || undefined)}
                      label={t('visualDashboard.customColor')}
                      presets="primary"
                    />
                  )}

                  <SelectField
                    label={t('visualDashboard.size')}
                    value={config.size || 'md'}
                    onChange={updateConfig('size')}
                    options={[
                      { value: 'sm', label: t('sizes.sm') },
                      { value: 'md', label: t('sizes.md') },
                      { value: 'lg', label: t('sizes.lg') },
                    ]}
                  />

                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      id="showCard"
                      checked={config.showCard ?? true}
                      onCheckedChange={(checked) => updateConfig('showCard')(checked === true)}
                    />
                    <label htmlFor="showCard" className="text-sm cursor-pointer">
                      {t('visualDashboard.showCard')}
                    </label>
                  </label>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <div className="grid grid-cols-2 gap-3">
                    <Field>
                      <Label>{t('visualDashboard.warningThreshold')}</Label>
                      <Input
                        type="number"
                        value={config.warningThreshold ?? 70}
                        onChange={(e) => updateConfig('warningThreshold')(Number(e.target.value))}
                        min={0}
                        max={100}
                        className="h-9"
                      />
                    </Field>
                    <Field>
                      <Label>{t('visualDashboard.dangerThreshold')}</Label>
                      <Input
                        type="number"
                        value={config.dangerThreshold ?? 90}
                        onChange={(e) => updateConfig('dangerThreshold')(Number(e.target.value))}
                        min={0}
                        max={100}
                        className="h-9"
                      />
                    </Field>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {t('visualDashboard.thresholdHint')}
                  </p>

                  <Field>
                    <Label>{t('visualDashboard.maxValue')}</Label>
                    <Input
                      type="number"
                      value={config.max ?? 100}
                      onChange={(e) => updateConfig('max')(Number(e.target.value))}
                      min={1}
                      className="h-9"
                    />
                  </Field>
                </div>
              ),
            },
            {
              type: 'custom' as const,
              render: () => (
                <DataMappingConfig
                  dataMapping={config.dataMapping as SingleValueMappingConfig}
                  onChange={updateDataMapping}
                  mappingType="single"
                />
              ),
            },
          ],
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-metric', 'system', 'extension', 'transform', 'ai-metric'],
              },
            },
          ],
        }

      case 'led-indicator':
        return {
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.size')}
                    value={config.size || 'md'}
                    onChange={updateConfig('size')}
                    options={[
                      { value: 'sm', label: t('sizes.sm') },
                      { value: 'md', label: t('sizes.md') },
                      { value: 'lg', label: t('sizes.lg') },
                    ]}
                  />

                  <div className="flex items-center gap-6">
                    <div className="flex items-center gap-2">
                      <Checkbox
                        id="showGlow"
                        checked={config.showGlow ?? true}
                        onCheckedChange={(checked) => updateConfig('showGlow')(checked === true)}
                      />
                      <label htmlFor="showGlow" className="text-sm cursor-pointer">
                        {t('visualDashboard.glowEffect')}
                      </label>
                    </div>

                    <div className="flex items-center gap-2">
                      <Checkbox
                        id="showAnimation"
                        checked={config.showAnimation ?? true}
                        onCheckedChange={(checked) => updateConfig('showAnimation')(checked === true)}
                      />
                      <label htmlFor="showAnimation" className="text-sm cursor-pointer">
                        {t('visualDashboard.animationEffect')}
                      </label>
                    </div>

                    <div className="flex items-center gap-2">
                      <Checkbox
                        id="showCard"
                        checked={config.showCard ?? true}
                        onCheckedChange={(checked) => updateConfig('showCard')(checked === true)}
                      />
                      <label htmlFor="showCard" className="text-sm cursor-pointer">
                        {t('visualDashboard.showCard')}
                      </label>
                    </div>
                  </div>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  {/* Default State - shown when no data source is configured */}
                  <Field>
                    <Label>{t('visualDashboard.defaultState')}</Label>
                    <Select
                      value={config.defaultState || 'unknown'}
                      onValueChange={updateConfig('defaultState')}
                    >
                      <SelectTrigger className="h-9">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="on">{t('visualDashboard.on')}</SelectItem>
                        <SelectItem value="off">{t('visualDashboard.off')}</SelectItem>
                        <SelectItem value="error">{t('visualDashboard.error')}</SelectItem>
                        <SelectItem value="warning">{t('visualDashboard.warning')}</SelectItem>
                        <SelectItem value="unknown">{t('visualDashboard.unknown')}</SelectItem>
                      </SelectContent>
                    </Select>
                    <p className="text-xs text-muted-foreground mt-1">
                      {t('visualDashboard.defaultStateHint')}
                    </p>
                  </Field>

                  {/* State Labels - custom labels for each LED state */}
                  <div className="pt-2 border-t">
                    <div className="text-sm font-medium mb-3">{t('visualDashboard.stateLabels')}</div>
                    <div className="grid grid-cols-2 gap-2">
                      {(['on', 'off', 'error', 'warning', 'unknown'] as const).map((state) => (
                        <div key={state} className="flex items-center gap-2">
                          <span className={cn(
                            "text-xs font-medium px-2 py-1 rounded shrink-0",
                            state === 'on' && "bg-success-light text-success",
                            state === 'off' && "bg-muted text-muted-foreground",
                            state === 'error' && "bg-error-light text-error",
                            state === 'warning' && "bg-warning-light text-warning",
                            state === 'unknown' && "bg-muted text-muted-foreground"
                          )}>
                            {t(`visualDashboard.${state}`)}
                          </span>
                          <Input
                            value={(config.stateLabels as Record<string, string>)?.[state] || ''}
                            onChange={(e) => {
                              const current = (config.stateLabels as Record<string, string>) || {}
                              updateConfig('stateLabels')({ ...current, [state]: e.target.value || undefined })
                            }}
                            placeholder={t(`visualDashboard.${state}`)}
                            className="h-8 text-sm flex-1"
                          />
                        </div>
                      ))}
                    </div>
                  </div>

                  {/* State Mapping Rules */}
                  <div className="pt-2 border-t">
                    <LEDStateRulesConfig
                      rules={config.rules as StateRule[] || []}
                      onChange={(newRules) => updateConfig('rules')(newRules)}
                      readonly={!config.dataSource}
                    />
                  </div>
                </div>
              ),
            },
          ],
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-metric', 'system', 'extension', 'transform', 'ai-metric'],
              },
            },
          ],
        }

      // ========== Charts ==========
      case 'line-chart':
        return {
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <ColorPicker
                    value={config.color || chartColorsHex[0]}
                    onChange={(color) => updateConfig('color')(color)}
                    label={t('visualDashboard.lineColor')}
                    presets="primary"
                  />

                  <SelectField
                    label={t('visualDashboard.size')}
                    value={config.size || 'md'}
                    onChange={updateConfig('size')}
                    options={[
                      { value: 'sm', label: t('sizes.sm') },
                      { value: 'md', label: t('sizes.md') },
                      { value: 'lg', label: t('sizes.lg') },
                    ]}
                  />

                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.smooth ?? true}
                        onCheckedChange={(checked) => updateConfig('smooth')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.smoothCurve')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.fillArea ?? false}
                        onCheckedChange={(checked) => updateConfig('fillArea')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.fillArea')}</span>
                    </label>
                  </div>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showGrid ?? true}
                        onCheckedChange={(checked) => updateConfig('showGrid')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showGrid')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showLegend ?? false}
                        onCheckedChange={(checked) => updateConfig('showLegend')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showLegend')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showTooltip ?? true}
                        onCheckedChange={(checked) => updateConfig('showTooltip')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showTooltip')}</span>
                    </label>
                  </div>
                </div>
              ),
            },
          ],
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-metric', 'system', 'extension', 'transform', 'ai-metric'],
                multiple: true,
                maxSources: 5,
              },
            },
          ],
        }

      case 'area-chart':
        return {
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <ColorPicker
                    value={config.color || chartColorsHex[0]}
                    onChange={(color) => updateConfig('color')(color)}
                    label={t('visualDashboard.areaColor')}
                    presets="primary"
                  />

                  <SelectField
                    label={t('visualDashboard.size')}
                    value={config.size || 'md'}
                    onChange={updateConfig('size')}
                    options={[
                      { value: 'sm', label: t('sizes.sm') },
                      { value: 'md', label: t('sizes.md') },
                      { value: 'lg', label: t('sizes.lg') },
                    ]}
                  />

                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.smooth ?? true}
                        onCheckedChange={(checked) => updateConfig('smooth')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.smoothCurve')}</span>
                    </label>
                  </div>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showGrid ?? true}
                        onCheckedChange={(checked) => updateConfig('showGrid')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showGrid')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showLegend ?? false}
                        onCheckedChange={(checked) => updateConfig('showLegend')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showLegend')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showTooltip ?? true}
                        onCheckedChange={(checked) => updateConfig('showTooltip')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showTooltip')}</span>
                    </label>
                  </div>
                </div>
              ),
            },
          ],
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-metric', 'system', 'extension', 'transform', 'ai-metric'],
                multiple: true,
                maxSources: 5,
              },
            },
            {
              type: 'custom' as const,
              render: () => (
                <DataMappingConfig
                  dataMapping={config.dataMapping as TimeSeriesMappingConfig}
                  onChange={updateDataMapping}
                  mappingType="time-series"
                  label={t('visualDashboard.dataMappingConfig')}
                  readonly={false}
                />
              ),
            },
          ],
        }

      case 'bar-chart':
        return {
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <ColorPicker
                    value={config.color || chartColorsHex[0]}
                    onChange={(color) => updateConfig('color')(color)}
                    label={t('visualDashboard.barColor')}
                    presets="primary"
                  />

                  <SelectField
                    label={t('visualDashboard.size')}
                    value={config.size || 'md'}
                    onChange={updateConfig('size')}
                    options={[
                      { value: 'sm', label: t('sizes.sm') },
                      { value: 'md', label: t('sizes.md') },
                      { value: 'lg', label: t('sizes.lg') },
                    ]}
                  />

                  <SelectField
                    label={t('visualDashboard.layout')}
                    value={config.layout || 'vertical'}
                    onChange={updateConfig('layout')}
                    options={[
                      { value: 'vertical', label: t('visualDashboard.vertical') },
                      { value: 'horizontal', label: t('visualDashboard.horizontal') },
                    ]}
                  />
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.stacked ?? false}
                        onCheckedChange={(checked) => updateConfig('stacked')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.stacked')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showGrid ?? true}
                        onCheckedChange={(checked) => updateConfig('showGrid')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showGrid')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showLegend ?? false}
                        onCheckedChange={(checked) => updateConfig('showLegend')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showLegend')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showTooltip ?? true}
                        onCheckedChange={(checked) => updateConfig('showTooltip')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showTooltip')}</span>
                    </label>
                  </div>
                </div>
              ),
            },
          ],
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-metric', 'system', 'extension', 'transform', 'ai-metric'],
                multiple: true,
                maxSources: 3,
              },
            },
          ],
        }

      case 'pie-chart':
        return {
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.size')}
                    value={config.size || 'md'}
                    onChange={updateConfig('size')}
                    options={[
                      { value: 'sm', label: t('sizes.sm') },
                      { value: 'md', label: t('sizes.md') },
                      { value: 'lg', label: t('sizes.lg') },
                    ]}
                  />

                  <SelectField
                    label={t('visualDashboard.type')}
                    value={config.variant || 'donut'}
                    onChange={updateConfig('variant')}
                    options={[
                      { value: 'pie', label: t('visualDashboard.pie') },
                      { value: 'donut', label: t('visualDashboard.donut') },
                    ]}
                  />

                  {config.variant === 'donut' && (
                    <Field>
                      <Label>{t('visualDashboard.innerRadius')}</Label>
                      <input
                        type="text"
                        value={config.innerRadius || '60%'}
                        onChange={(e) => updateConfig('innerRadius')(e.target.value)}
                        placeholder="60% or 60"
                        className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                      />
                    </Field>
                  )}

                  <Field>
                    <Label>{t('visualDashboard.outerRadius')}</Label>
                    <input
                      type="text"
                      value={config.outerRadius || '80%'}
                      onChange={(e) => updateConfig('outerRadius')(e.target.value)}
                      placeholder="80% or 80"
                      className="w-full h-9 px-3 rounded-md border border-input bg-background text-sm"
                    />
                  </Field>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showLegend ?? false}
                        onCheckedChange={(checked) => updateConfig('showLegend')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showLegend')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showTooltip ?? true}
                        onCheckedChange={(checked) => updateConfig('showTooltip')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showTooltip')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showLabels ?? false}
                        onCheckedChange={(checked) => updateConfig('showLabels')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showLabel')}</span>
                    </label>
                  </div>
                </div>
              ),
            },
          ],
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-metric', 'system', 'extension', 'transform', 'ai-metric'],
              },
            },
          ],
        }

      // ========== Controls ==========
      case 'toggle-switch':
        return {
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.size')}
                    value={config.size || 'md'}
                    onChange={updateConfig('size')}
                    options={[
                      { value: 'sm', label: t('sizes.sm') },
                      { value: 'md', label: t('sizes.md') },
                      { value: 'lg', label: t('sizes.lg') },
                    ]}
                  />
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <div className="p-3 rounded-lg bg-info-light border border-info">
                    <p className="text-sm text-info">
                      {t('visualDashboard.commandButtonHint')}
                    </p>
                  </div>
                </div>
              ),
            },
          ],
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-command', 'extension-command'],
              },
            },
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <div className="p-3 rounded-lg bg-info-light border border-info">
                    <p className="text-sm text-info">
                      <strong>{t('visualDashboard.commandInterface')}</strong><br />
                      {t('visualDashboard.commandInterfaceDesc')}
                    </p>
                  </div>
                </div>
              ),
            },
          ],
        }

      // ========== Display & Content ==========
      case 'image-display':
        return {
          dataSourceSections: [
            {
              type: 'custom' as const,
              render: () => (
                <DualModeSourceField
                  inputType="image"
                  value={config.src || ''}
                  onValueChange={updateConfig('src')}
                  dataSource={config.dataSource}
                  onDataSourceChange={updateDataSource}
                  allowedTypes={['device-metric', 'system', 'extension', 'transform', 'ai-metric']}
                  label={t('visualDashboard.imageSource')}
                  placeholder={t('visualDashboard.urlPlaceholder')}
                />
              ),
            },
          ],
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.fitMode')}
                    value={config.fit || 'contain'}
                    onChange={updateConfig('fit')}
                    options={[
                      { value: 'contain', label: t('visualDashboard.fitContain') },
                      { value: 'cover', label: t('visualDashboard.fitCover') },
                      { value: 'fill', label: t('visualDashboard.fitFill') },
                      { value: 'none', label: t('visualDashboard.fitNone') },
                      { value: 'scale-down', label: t('visualDashboard.fitScaleDown') },
                    ]}
                  />
                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2">
                      <Checkbox
                        checked={config.rounded ?? true}
                        onCheckedChange={(checked) => updateConfig('rounded')(!!checked)}
                      />
                      <span className="text-xs">{t('visualDashboard.rounded')}</span>
                    </label>
                    <label className="flex items-center gap-2">
                      <Checkbox
                        checked={config.zoomable ?? true}
                        onCheckedChange={(checked) => updateConfig('zoomable')(!!checked)}
                      />
                      <span className="text-xs">{t('visualDashboard.zoomable')}</span>
                    </label>
                    <label className="flex items-center gap-2">
                      <Checkbox
                        checked={config.showShadow ?? false}
                        onCheckedChange={(checked) => updateConfig('showShadow')(!!checked)}
                      />
                      <span className="text-xs">{t('visualDashboard.shadow')}</span>
                    </label>
                  </div>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <Field>
                    <Label>{t('imageDisplay.altText')}</Label>
                    <Input
                      value={config.alt || ''}
                      onChange={(e) => updateConfig('alt')(e.target.value)}
                      placeholder={t('placeholders.imageAltText')}
                      className="h-9"
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      {t('imageDisplay.altHint', 'Alternative text for screen readers and when image fails to load')}
                    </p>
                  </Field>

                  <Field>
                    <Label>{t('visualDashboard.title')}</Label>
                    <Input
                      value={config.title || ''}
                      onChange={(e) => updateConfig('title')(e.target.value)}
                      placeholder={t('placeholders.imageTitle')}
                      className="h-9"
                    />
                  </Field>

                  <Field>
                    <Label>{t('imageDisplay.caption', 'Caption')}</Label>
                    <Input
                      value={config.caption || ''}
                      onChange={(e) => updateConfig('caption')(e.target.value)}
                      placeholder={t('placeholders.imageCaption')}
                      className="h-9"
                    />
                  </Field>

                  <SelectField
                    label={t('placeholders.loadingState')}
                    value={config.loadingState || 'lazy'}
                    onChange={updateConfig('loadingState')}
                    options={[
                      { value: 'eager', label: t('imageDisplay.loadImmediately', 'Load Immediately') },
                      { value: 'lazy', label: t('imageDisplay.lazyLoad', 'Lazy Load') },
                    ]}
                  />
                </div>
              ),
            },
          ],
        }

      case 'image-history':
        return {
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-metric', 'system', 'extension', 'transform', 'ai-metric'],
              },
            },
          ],
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.fitMode')}
                    value={config.fit || 'contain'}
                    onChange={updateConfig('fit')}
                    options={[
                      { value: 'contain', label: t('visualDashboard.fitContain') },
                      { value: 'cover', label: t('visualDashboard.fitCover') },
                      { value: 'fill', label: t('visualDashboard.fitFill') },
                      { value: 'none', label: t('visualDashboard.fitNone') },
                      { value: 'scale-down', label: t('visualDashboard.fitScaleDown') },
                    ]}
                  />
                  <div className="grid grid-cols-2 gap-3">
                    <Field>
                      <Label>{t('visualDashboard.maxImages')}</Label>
                      <Input
                        type="number"
                        value={config.limit !== undefined && config.limit !== null && config.limit !== '' ? config.limit : ''}
                        onChange={(e) => {
                          const raw = e.target.value
                          if (raw === '') {
                            updateConfig('limit')(undefined)
                            return
                          }
                          const v = Number(raw)
                          if (Number.isFinite(v)) updateConfig('limit')(v)
                        }}
                        onBlur={(e) => {
                          const v = Number(e.target.value)
                          if (e.target.value !== '' && (!Number.isFinite(v) || v < 1 || v > 200)) {
                            updateConfig('limit')(50)
                          }
                        }}
                        min={1}
                        max={200}
                        placeholder="50"
                        className="h-9"
                      />
                    </Field>
                    <Field>
                      <Label>{t('visualDashboard.timeRangeHours')}</Label>
                      <Input
                        type="number"
                        value={config.timeRange ?? 1}
                        onChange={(e) => updateConfig('timeRange')(Number(e.target.value))}
                        min={1}
                        max={168}
                        step={1}
                        className="h-9"
                      />
                    </Field>
                  </div>
                  <div className="flex flex-wrap items-center gap-3">
                    <label className="flex items-center gap-2">
                      <Checkbox
                        checked={config.rounded ?? true}
                        onCheckedChange={(checked) => updateConfig('rounded')(!!checked)}
                      />
                      <span className="text-xs">{t('visualDashboard.rounded')}</span>
                    </label>
                  </div>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <Field>
                    <Label>{t('imageHistory.defaultAltText', 'Default Alt Text')}</Label>
                    <Input
                      value={config.alt || ''}
                      onChange={(e) => updateConfig('alt')(e.target.value)}
                      placeholder={t('placeholders.defaultAltText')}
                      className="h-9"
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      {t('imageHistory.altHint', 'Alternative text for accessibility when no specific alt is available')}
                    </p>
                  </Field>

                  <Field>
                    <Label>{t('visualDashboard.title')}</Label>
                    <Input
                      value={config.title || ''}
                      onChange={(e) => updateConfig('title')(e.target.value)}
                      placeholder={t('placeholders.galleryTitle')}
                      className="h-9"
                    />
                  </Field>

                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showNavigation ?? true}
                        onCheckedChange={(checked) => updateConfig('showNavigation')(!!checked)}
                      />
                      <span className="text-sm">{t('imageHistory.showNavigation', 'Show Navigation')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.showDots ?? true}
                        onCheckedChange={(checked) => updateConfig('showDots')(!!checked)}
                      />
                      <span className="text-sm">{t('imageHistory.showDotsIndicator', 'Show Dots Indicator')}</span>
                    </label>

                    <label className="flex items-center gap-2 cursor-pointer">
                      <Checkbox
                        checked={config.autoPlay ?? false}
                        onCheckedChange={(checked) => updateConfig('autoPlay')(!!checked)}
                      />
                      <span className="text-sm">{t('imageHistory.autoPlay', 'Auto Play')}</span>
                    </label>
                  </div>

                  {config.autoPlay && (
                    <Field>
                      <Label>{t('imageHistory.autoPlayInterval', 'Auto Play Interval (seconds)')}</Label>
                      <Input
                        type="number"
                        value={config.autoPlayInterval ?? 3}
                        onChange={(e) => updateConfig('autoPlayInterval')(Number(e.target.value))}
                        min={1}
                        max={60}
                        className="h-9"
                      />
                    </Field>
                  )}
                </div>
              ),
            },
          ],
        }

      case 'web-display':
        return {
          dataSourceSections: [
            {
              type: 'custom' as const,
              render: () => (
                <DualModeSourceField
                  inputType="url"
                  value={config.src || ''}
                  onValueChange={updateConfig('src')}
                  dataSource={config.dataSource}
                  onDataSourceChange={updateDataSource}
                  allowedTypes={['device-metric', 'system', 'extension', 'transform', 'ai-metric']}
                  label={t('webDisplay.websiteUrl', 'Website URL')}
                  placeholder={t('placeholders.urlExample')}
                />
              ),
            },
          ],
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-4">
                  <div className="flex items-center gap-4">
                    <label className="flex items-center gap-2">
                      <Checkbox
                        checked={config.sandbox ?? true}
                        onCheckedChange={(checked) => updateConfig('sandbox')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.sandboxIsolation')}</span>
                    </label>
                    <label className="flex items-center gap-2">
                      <Checkbox
                        checked={config.showHeader ?? true}
                        onCheckedChange={(checked) => updateConfig('showHeader')(!!checked)}
                      />
                      <span className="text-sm">{t('visualDashboard.showHeader')}</span>
                    </label>
                  </div>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <Field>
                    <Label>{t('visualDashboard.title')}</Label>
                    <Input
                      value={config.title || ''}
                      onChange={(e) => updateConfig('title')(e.target.value)}
                      placeholder={t('placeholders.websiteTitle')}
                      className="h-9"
                    />
                  </Field>

                  <Field>
                    <Label>{t('webDisplay.refreshInterval', 'Refresh Interval (seconds)')}</Label>
                    <Input
                      type="number"
                      value={config.refreshInterval ?? 0}
                      onChange={(e) => updateConfig('refreshInterval')(Number(e.target.value))}
                      min={0}
                      max={3600}
                      step={10}
                      placeholder={t('webDisplay.noRefreshPlaceholder', '0 = no refresh')}
                      className="h-9"
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      {t('webDisplay.noRefreshHint', 'Set to 0 to disable auto-refresh')}
                    </p>
                  </Field>

                  <Field>
                    <Label>{t('webDisplay.loadingMessage', 'Loading Message')}</Label>
                    <Input
                      value={config.loadingMessage || 'Loading...'}
                      onChange={(e) => updateConfig('loadingMessage')(e.target.value)}
                      placeholder={t('placeholders.loadingMessage')}
                      className="h-9"
                    />
                  </Field>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      checked={config.allowFullScreen ?? true}
                      onCheckedChange={(checked) => updateConfig('allowFullScreen')(!!checked)}
                    />
                    <span className="text-sm">{t('webDisplay.allowFullscreen', 'Allow Fullscreen')}</span>
                  </label>
                </div>
              ),
            },
          ],
        }

      case 'markdown-display':
        return {
          dataSourceSections: [
            {
              type: 'custom' as const,
              render: () => (
                <DualModeSourceField
                  inputType="text"
                  value={config.content || ''}
                  onValueChange={updateConfig('content')}
                  dataSource={config.dataSource}
                  onDataSourceChange={updateDataSource}
                  allowedTypes={['device-metric', 'system', 'extension', 'transform', 'ai-metric']}
                  label={t('visualDashboard.markdownContent')}
                  placeholder={t('visualDashboard.markdownPlaceholder')}
                  rows={6}
                />
              ),
            },
          ],
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.style')}
                    value={config.variant || 'default'}
                    onChange={updateConfig('variant')}
                    options={[
                      { value: 'default', label: t('visualDashboard.default') },
                      { value: 'compact', label: t('visualDashboard.compact') },
                      { value: 'minimal', label: t('visualDashboard.minimal') },
                    ]}
                  />
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <Field>
                    <Label>{t('visualDashboard.title')}</Label>
                    <Input
                      value={config.title || ''}
                      onChange={(e) => updateConfig('title')(e.target.value)}
                      placeholder={t('placeholders.contentTitle')}
                      className="h-9"
                    />
                  </Field>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      checked={config.showCopyButton ?? false}
                      onCheckedChange={(checked) => updateConfig('showCopyButton')(!!checked)}
                    />
                    <span className="text-sm">{t('markdownDisplay.showCopyButton', 'Show Copy Button')}</span>
                  </label>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      checked={config.sanitizeHtml ?? true}
                      onCheckedChange={(checked) => updateConfig('sanitizeHtml')(!!checked)}
                    />
                    <span className="text-sm">{t('markdownDisplay.sanitizeHtml', 'Sanitize HTML')}</span>
                    <p className="text-xs text-muted-foreground">
      {t('markdownDisplay.sanitizeHtmlHint', 'Remove potentially dangerous HTML tags')}
                    </p>
                  </label>
                </div>
              ),
            },
          ],
        }

      case 'video-display':
        return {
          dataSourceSections: [
            {
              type: 'custom' as const,
              render: () => (
                <DualModeSourceField
                  inputType="url"
                  value={config.src || ''}
                  onValueChange={updateConfig('src')}
                  dataSource={config.dataSource}
                  onDataSourceChange={updateDataSource}
                  allowedTypes={['device', 'device-info', 'device-metric']}
                  label={t('visualDashboard.videoSource')}
                  placeholder={t('visualDashboard.videoUrlPlaceholder')}
                />
              ),
            },
          ],
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.videoType')}
                    value={config.type || 'file'}
                    onChange={updateConfig('type')}
                    options={[
                      { value: 'file', label: t('visualDashboard.videoFile') },
                      { value: 'hls', label: 'HLS (.m3u8)' },
                      { value: 'device-camera', label: t('visualDashboard.deviceCamera') },
                    ]}
                  />

                  {/* Type-specific hints */}
                  {config.type === 'hls' && (
                    <div className="p-2 bg-success-light border border-success-light rounded-md">
                      <p className="text-xs text-success dark:text-success">
                        <strong>HLS URL格式：</strong> http://server/path/index.m3u8
                      </p>
                    </div>
                  )}

                  {config.type === 'device-camera' && (
                    <div className="p-2 bg-info-light border border-info rounded-md">
                      <p className="text-xs text-info">
                        <strong>设备摄像头：</strong> 将请求访问本地摄像头设备
                      </p>
                    </div>
                  )}

                  <SelectField
                    label={t('visualDashboard.fitMethod')}
                    value={config.fit || 'contain'}
                    onChange={updateConfig('fit')}
                    options={[
                      { value: 'contain', label: t('visualDashboard.fitContainFull') },
                      { value: 'cover', label: t('visualDashboard.fitCoverFill') },
                      { value: 'fill', label: t('visualDashboard.fitStretch') },
                    ]}
                  />

                  <div className="grid grid-cols-2 gap-3">
                    <Field>
                      <Label>{t('visualDashboard.autoPlay')}</Label>
                      <Select
                        value={String(config.autoplay ?? false)}
                        onValueChange={(value) => updateConfig('autoplay')(value === 'true')}
                      >
                        <SelectTrigger className="w-full h-9">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="false">{t('visualDashboard.off')}</SelectItem>
                          <SelectItem value="true">{t('visualDashboard.on')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </Field>
                    <Field>
                      <Label>{t('visualDashboard.muted')}</Label>
                      <Select
                        value={String(config.muted ?? true)}
                        onValueChange={(value) => updateConfig('muted')(value === 'true')}
                      >
                        <SelectTrigger className="w-full h-9">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="true">{t('visualDashboard.muted')}</SelectItem>
                          <SelectItem value="false">{t('visualDashboard.unmuted')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </Field>
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <Field>
                      <Label>{t('visualDashboard.showControls')}</Label>
                      <Select
                        value={String(config.controls ?? true)}
                        onValueChange={(value) => updateConfig('controls')(value === 'true')}
                      >
                        <SelectTrigger className="w-full h-9">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="true">{t('visualDashboard.showCard')}</SelectItem>
                          <SelectItem value="false">{t('visualDashboard.hide')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </Field>
                    <Field>
                      <Label>{t('visualDashboard.loop')}</Label>
                      <Select
                        value={String(config.loop ?? false)}
                        onValueChange={(value) => updateConfig('loop')(value === 'true')}
                      >
                        <SelectTrigger className="w-full h-9">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="false">{t('visualDashboard.off')}</SelectItem>
                          <SelectItem value="true">{t('visualDashboard.on')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </Field>
                  </div>

                  <Field>
                    <Label>{t('visualDashboard.fullscreenButton')}</Label>
                    <Select
                      value={String(config.showFullscreen ?? true)}
                      onValueChange={(value) => updateConfig('showFullscreen')(value === 'true')}
                    >
                      <SelectTrigger className="w-full h-9">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="true">{t('visualDashboard.showCard')}</SelectItem>
                        <SelectItem value="false">{t('visualDashboard.hide')}</SelectItem>
                      </SelectContent>
                    </Select>
                  </Field>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <Field>
                    <Label>{t('videoDisplay.posterImageUrl', 'Poster Image URL')}</Label>
                    <Input
                      value={config.poster || ''}
                      onChange={(e) => updateConfig('poster')(e.target.value)}
                      placeholder={t('videoDisplay.posterPlaceholder', 'https://example.com/poster.jpg')}
                      className="h-9"
                    />
                    <p className="text-xs text-muted-foreground mt-1">
                      {t('videoDisplay.posterHint', 'Image shown before video plays')}
                    </p>
                  </Field>

                  <Field>
                    <Label>{t('visualDashboard.title')}</Label>
                    <Input
                      value={config.title || ''}
                      onChange={(e) => updateConfig('title')(e.target.value)}
                      placeholder={t('placeholders.videoTitle')}
                      className="h-9"
                    />
                  </Field>

                  <Field>
                    <Label>{t('common.description', 'Description')}</Label>
                    <Input
                      value={config.description || ''}
                      onChange={(e) => updateConfig('description')(e.target.value)}
                      placeholder={t('placeholders.videoDescription')}
                      className="h-9"
                    />
                  </Field>

                  <label className="flex items-center gap-2 cursor-pointer">
                    <Checkbox
                      checked={config.showTitleOverlay ?? false}
                      onCheckedChange={(checked) => updateConfig('showTitleOverlay')(!!checked)}
                    />
                    <span className="text-sm">{t('videoDisplay.showTitleOverlay', 'Show Title Overlay')}</span>
                  </label>
                </div>
              ),
            },
          ],
        }

      case 'map-display':
        return {
          dataSourceSections: [],
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <div className="flex items-end gap-2">
                    <div className="grid grid-cols-2 gap-3 flex-1">
                      <Field>
                        <Label>{t('visualDashboard.latitude')}</Label>
                        <Input
                          type="number"
                          step="0.0001"
                          value={(config.center as { lat: number } | undefined)?.lat ?? 39.9042}
                          onChange={(e) => updateConfig('center')({ ...(config.center as { lat: number; lng: number } | undefined) || { lat: 39.9042, lng: 116.4074 }, lat: parseFloat(e.target.value) })}
                          placeholder={t('mapDisplay.defaultLatitude', '39.9042')}
                          className="h-9"
                        />
                      </Field>
                      <Field>
                        <Label>{t('visualDashboard.longitude')}</Label>
                        <Input
                          type="number"
                          step="0.0001"
                          value={(config.center as { lng: number } | undefined)?.lng ?? 116.4074}
                          onChange={(e) => updateConfig('center')({ ...(config.center as { lat: number; lng: number } | undefined) || { lat: 39.9042, lng: 116.4074 }, lng: parseFloat(e.target.value) })}
                          placeholder={t('mapDisplay.defaultLongitude', '116.4074')}
                          className="h-9"
                        />
                      </Field>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setCenterPickerOpen(true)}
                      className="h-9 px-3 shrink-0"
                      title={t('mapDisplay.visualSelectCenter', '可视化选择中心点')}
                    >
                      <MapPin className="h-4 w-4" />
                    </Button>
                  </div>

                  <div className="grid grid-cols-3 gap-3">
                    <Field>
                      <Label>{t('visualDashboard.zoomLevel')}</Label>
                      <Input
                        type="number"
                        min={config.minZoom ?? 2}
                        max={config.maxZoom ?? 18}
                        value={config.zoom ?? 10}
                        onChange={(e) => updateConfig('zoom')(parseFloat(e.target.value))}
                        className="h-9"
                      />
                    </Field>
                    <Field>
                      <Label>{t('visualDashboard.maxZoom')}</Label>
                      <Input
                        type="number"
                        min={1}
                        max={10}
                        value={config.minZoom ?? 2}
                        onChange={(e) => updateConfig('minZoom')(parseFloat(e.target.value))}
                        className="h-9"
                      />
                    </Field>
                    <Field>
                      <Label>{t('visualDashboard.maxZoom')}</Label>
                      <Input
                        type="number"
                        min={10}
                        max={20}
                        value={config.maxZoom ?? 18}
                        onChange={(e) => updateConfig('maxZoom')(parseFloat(e.target.value))}
                        className="h-9"
                      />
                    </Field>
                  </div>

                  <SelectField
                    label={t('visualDashboard.mapLayer')}
                    value={config.tileLayer || 'osm'}
                    onChange={updateConfig('tileLayer')}
                    options={[
                      { value: 'osm', label: 'OpenStreetMap' },
                      { value: 'satellite', label: t('visualDashboard.satellite') },
                      { value: 'dark', label: t('visualDashboard.darkMode') },
                      { value: 'terrain', label: t('visualDashboard.terrain') },
                    ]}
                  />

                  <Field>
                    <Label>{t('visualDashboard.markerColor')}</Label>
                    <Input
                      type="color"
                      value={config.markerColor || chartColorsHex[0]}
                      onChange={(e) => updateConfig('markerColor')(e.target.value)}
                      className="h-9 w-full"
                    />
                  </Field>

                  <div className="grid grid-cols-2 gap-3">
                    <Field>
                      <Label>{t('visualDashboard.showControlBar')}</Label>
                      <Select
                        value={String(config.showControls ?? true)}
                        onValueChange={(value) => updateConfig('showControls')(value === 'true')}
                      >
                        <SelectTrigger className="w-full h-9">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="true">{t('visualDashboard.showCard')}</SelectItem>
                          <SelectItem value="false">{t('visualDashboard.hide')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </Field>
                    <Field>
                      <Label>{t('visualDashboard.showLayerControl')}</Label>
                      <Select
                        value={String(config.showLayers ?? true)}
                        onValueChange={(value) => updateConfig('showLayers')(value === 'true')}
                      >
                        <SelectTrigger className="w-full h-9">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="true">{t('visualDashboard.showCard')}</SelectItem>
                          <SelectItem value="false">{t('visualDashboard.hide')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </Field>
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <Field>
                      <Label>{t('visualDashboard.interactive')}</Label>
                      <Select
                        value={String(config.interactive ?? true)}
                        onValueChange={(value) => updateConfig('interactive')(value === 'true')}
                      >
                        <SelectTrigger className="w-full h-9">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="true">{t('visualDashboard.yes')}</SelectItem>
                          <SelectItem value="false">{t('visualDashboard.no')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </Field>
                    <Field>
                      <Label>{t('visualDashboard.fullscreenButton')}</Label>
                      <Select
                        value={String(config.showFullscreen ?? true)}
                        onValueChange={(value) => updateConfig('showFullscreen')(value === 'true')}
                      >
                        <SelectTrigger className="w-full h-9">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="true">{t('visualDashboard.showCard')}</SelectItem>
                          <SelectItem value="false">{t('visualDashboard.hide')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </Field>
                  </div>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-4">
                  {/* Data source selection — merged from Data Source tab */}
                  <BindingDataSourceSelector
                    dataSource={config.dataSource}
                    onConfirm={(newSource) => {
                      updateDataSource(newSource)
                      if (Array.isArray(newSource) && newSource.length > 0) {
                        const newBindings: MapBinding[] = newSource.map((ds, index) => {
                          let bindingType: MapBindingType = 'device'
                          if (ds.type === 'metric' || ds.type === 'telemetry') bindingType = 'metric'
                          else if (ds.type === 'command') bindingType = 'command'

                          const existingBinding = (config.bindings as MapBinding[])?.find(b => {
                            if (!b.dataSource) return false
                            const bDs = b.dataSource as any
                            if (bindingType === 'metric' || ds.type === 'telemetry') {
                              return (getSourceId(bDs) === getSourceId(ds)) && (
                                bDs.metricId === ds.metricId ||
                                bDs.property === ds.metricId ||
                                bDs.property === ds.property
                              )
                            }
                            if (bindingType === 'command') {
                              return (getSourceId(bDs) === getSourceId(ds)) && (bDs.command === ds.command)
                            }
                            return getSourceId(bDs) === getSourceId(ds) && !ds.metricId && !ds.property && !ds.command
                          })

                          const generateBindingId = () => {
                            if (ds.type === 'metric' || ds.type === 'telemetry') {
                              return `${bindingType}-${getSourceId(ds)}-${ds.metricId || ds.property || index}`
                            } else if (ds.type === 'command') {
                              return `${bindingType}-${getSourceId(ds)}-${ds.command}`
                            } else {
                              return `${bindingType}-${getSourceId(ds)}-${index}`
                            }
                          }

                          const baseBinding = existingBinding || {
                            id: generateBindingId(),
                            position: { lat: 39.9042, lng: 116.4074 },
                          }

                          return {
                            ...baseBinding,
                            id: existingBinding?.id || generateBindingId(),
                            type: bindingType,
                            icon: bindingType,
                            name: (ds.type === 'metric' || ds.type === 'telemetry')
                              ? (ds.metricId || ds.property || t('visualDashboard.metricIndex', { index: index + 1 }))
                              : ds.type === 'command'
                                ? `${getSourceId(ds) || ''} → ${ds.command || ''}`
                                : (getSourceId(ds) || t('visualDashboard.deviceIndex', { index: index + 1 })),
                            dataSource: ds,
                            position: existingBinding?.position || baseBinding.position,
                          }
                        })
                        updateConfig('bindings')(newBindings)
                      }
                    }}
                    allowedTypes={['device', 'metric', 'command', 'extension']}
                    maxSources={50}
                    title={t('visualDashboard.markerBinding')}
                  />

                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="text-sm font-medium">{t('visualDashboard.markerBinding')}</h3>
                      <p className="text-xs text-muted-foreground mt-1">
                        {t('visualDashboard.manageMapMarkers')}
                      </p>
                    </div>
                    <Button
                      type="button"
                      variant="default"
                      size="sm"
                      onClick={() => {
                        // Get the latest bindings from the store, not just local state
                        const latestDashboard = useStore.getState().currentDashboard
                        const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent?.id)
                        let latestBindings = (latestComponent as any)?.config?.bindings as MapBinding[] || []

                        // Fix duplicate IDs - regenerate IDs for bindings with duplicate IDs
                        const idCount = new Map<string, number>()
                        latestBindings = latestBindings.map((binding, index) => {
                          const ds = binding.dataSource as any
                          const currentId = binding.id

                          // Check if this ID is duplicated
                          idCount.set(currentId, (idCount.get(currentId) || 0) + 1)

                          // If ID will be duplicated or uses old format, regenerate it
                          if (idCount.get(currentId)! > 1 || binding.type === 'device' && ds?.metricId) {
                            // Generate unique ID based on type and data
                            let newId: string
                            if (binding.type === 'metric' || ds?.type === 'telemetry') {
                              newId = `metric-${getSourceId(ds)}-${ds?.metricId || ds?.property || index}`
                            } else if (binding.type === 'command') {
                              newId = `command-${getSourceId(ds)}-${ds?.command}`
                            } else {
                              newId = `device-${getSourceId(ds)}-${index}`
                            }
                                                        return { ...binding, id: newId }
                          }

                          return binding
                        })

                        setMapEditorBindings(latestBindings)
                        setMapEditorOpen(true)
                      }}
                    >
                      <MapIcon className="h-4 w-4 mr-1" />
                      {t('visualDashboard.openMapEditor')}
                    </Button>
                  </div>

                  {/* Bindings List - Grouped by Type */}
                  <div className="border rounded-lg overflow-hidden">
                    {(() => {
                      // Get the latest bindings from the store for display
                      const latestDashboard = useStore.getState().currentDashboard
                      const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent?.id)
                      let displayBindings = (latestComponent as any)?.config?.bindings as MapBinding[] || []

                      // Fix duplicate IDs for display and interaction
                      const idCount = new Map<string, number>()
                      displayBindings = displayBindings.map((binding, index) => {
                        const ds = binding.dataSource as any
                        const currentId = binding.id
                        idCount.set(currentId, (idCount.get(currentId) || 0) + 1)

                        // If ID is duplicated or binding type is wrong (e.g., telemetry marked as device)
                        if (idCount.get(currentId)! > 1 || (binding.type === 'device' && ds?.type === 'telemetry')) {
                          let newId: string
                          let newType = binding.type

                          // Fix type for telemetry bindings
                          if (ds?.type === 'telemetry' || ds?.type === 'metric') {
                            newType = 'metric'
                          }

                          if (newType === 'metric' || ds?.type === 'telemetry') {
                            newId = `metric-${getSourceId(ds)}-${ds?.metricId || ds?.property || index}`
                          } else if (newType === 'command') {
                            newId = `command-${getSourceId(ds)}-${ds?.command}`
                          } else {
                            newId = `device-${getSourceId(ds)}-${index}`
                          }
                                                    return { ...binding, id: newId, type: newType as any, icon: newType as any }
                        }
                        return binding
                      })

                      // Group by type
                      const groupedBindings = {
                        device: displayBindings.filter(b => b.type === 'device'),
                        metric: displayBindings.filter(b => b.type === 'metric'),
                        command: displayBindings.filter(b => b.type === 'command'),
                        marker: displayBindings.filter(b => b.type === 'marker'),
                      }

                      const TYPE_CONFIG = {
                        device: {
                          label: t('mapDisplay.device'),
                          color: 'bg-success',
                          textColor: 'text-success',
                          bgColor: 'bg-success-light dark:bg-success-light',
                          borderColor: 'border-success-light dark:border-success-light',
                          icon: MapPin,
                          description: t('mapDisplay.deviceDesc')
                        },
                        metric: {
                          label: t('mapDisplay.metric'),
                          color: 'bg-accent-purple',
                          textColor: 'text-accent-purple',
                          bgColor: 'bg-accent-purple-light',
                          borderColor: 'border-accent-purple-light',
                          icon: Activity,
                          description: t('mapDisplay.metricDesc')
                        },
                        command: {
                          label: t('mapDisplay.command'),
                          color: 'bg-info',
                          textColor: 'text-info',
                          bgColor: 'bg-info-light',
                          borderColor: 'border-info',
                          icon: Zap,
                          description: t('mapDisplay.commandDesc')
                        },
                        marker: {
                          label: t('mapDisplay.marker'),
                          color: 'bg-accent-orange',
                          textColor: 'text-accent-orange',
                          bgColor: 'bg-accent-orange-light',
                          borderColor: 'border-accent-orange-light',
                          icon: Monitor,
                          description: t('mapDisplay.markerDesc')
                        },
                      } as const

                      if (displayBindings.length === 0) {
                        return (
                          <div className="p-6 text-center text-muted-foreground">
                            <MapIcon className="h-8 w-8 mx-auto mb-2 opacity-50" />
                            <p className="text-sm">{t('visualDashboard.noMarkers')}</p>
                            <p className="text-xs mt-1">{t('visualDashboard.addMarkerHint')}</p>
                          </div>
                        )
                      }

                      return (Object.keys(groupedBindings) as Array<keyof typeof groupedBindings>).map(type => {
                        const typeBindings = groupedBindings[type]
                        if (typeBindings.length === 0) return null

                        const config = TYPE_CONFIG[type]
                        const Icon = config.icon

                        return (
                          <div key={type} className="border-b last:border-b-0">
                            {/* Type Header */}
                            <div className={`px-3 py-2 ${config.bgColor} border-b ${config.borderColor} flex items-center justify-between`}>
                              <div className="flex items-center gap-2">
                                <div className={`w-5 h-5 rounded-full ${config.color} flex items-center justify-center`}>
                                  <Icon className="h-4 w-4 text-primary-foreground" />
                                </div>
                                <span className="text-sm font-medium">{config.label}</span>
                                <span className="text-xs text-muted-foreground">({typeBindings.length})</span>
                              </div>
                              <span className="text-xs text-muted-foreground">{config.description}</span>
                            </div>

                            {/* Bindings of this type */}
                            <div className="divide-y">
                              {typeBindings.map((binding) => {
                                const positionText = binding.position && binding.position !== 'auto'
                                  ? `(${binding.position.lat.toFixed(4)}, ${binding.position.lng.toFixed(4)})`
                                  : t('visualDashboard.autoLocation')

                                // Get device/metric info from dataSource
                                const deviceId = getSourceId((binding.dataSource as DataSource))
                                const metricId = (binding.dataSource as any)?.metricId
                                const command = (binding.dataSource as any)?.command

                                return (
                                  <div
                                    key={binding.id}
                                    className="flex items-center gap-3 p-3"
                                  >
                                    <div className={`w-8 h-8 rounded-full flex items-center justify-center ${config.color}/20 ${config.textColor}`}>
                                      <Icon className="h-4 w-4" />
                                    </div>
                                    <div className="flex-1 min-w-0">
                                      <div className="text-sm font-medium truncate">{binding.name}</div>
                                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                                        <span>{positionText}</span>
                                        {deviceId && <span>• {deviceId.slice(0, 8)}...</span>}
                                        {metricId && <span>• {metricId}</span>}
                                        {command && <span>• {command}</span>}
                                      </div>
                                    </div>
                                  </div>
                                )
                              })}
                            </div>
                          </div>
                        )
                      })
                    })()}
                  </div>

                  {/* Legend */}
                  <div className="flex items-center gap-4 text-xs text-muted-foreground">
                    <div className="flex items-center gap-1">
                      <div className="w-4 h-4 rounded-full bg-info"></div>
                      <span>{t('mapDisplay.device')}</span>
                    </div>
                    <div className="flex items-center gap-1">
                      <div className="w-4 h-4 rounded-full bg-success"></div>
                      <span>{t('mapDisplay.metric')}</span>
                    </div>
                    <div className="flex items-center gap-1">
                      <div className="w-4 h-4 rounded-full bg-accent-orange"></div>
                      <span>{t('mapDisplay.command')}</span>
                    </div>
                    <div className="flex items-center gap-1">
                      <div className="w-4 h-4 rounded-full bg-accent-purple"></div>
                      <span>{t('mapDisplay.marker')}</span>
                    </div>
                  </div>
                </div>
              ),
            },
          ],
        }

      case 'custom-layer':
        return {
          dataSourceSections: [],
          styleSections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  <SelectField
                    label={t('visualDashboard.backgroundType')}
                    value={config.backgroundType || 'grid'}
                    onChange={updateConfig('backgroundType')}
                    options={[
                      { value: 'grid', label: t('visualDashboard.backgroundTypeGrid') },
                      { value: 'color', label: t('visualDashboard.backgroundTypeColor') },
                      { value: 'image', label: t('visualDashboard.backgroundTypeImage') },
                      { value: 'transparent', label: t('visualDashboard.backgroundTypeTransparent') },
                    ]}
                  />

                  {config.backgroundType === 'color' && (
                    <Field>
                      <Label>{t('visualDashboard.backgroundColor')}</Label>
                      <Input
                        type="color"
                        value={config.backgroundColor || '#e5e5e5'}
                        onChange={(e) => updateConfig('backgroundColor')(e.target.value)}
                        className="h-9 w-full"
                      />
                    </Field>
                  )}

                  {config.backgroundType === 'image' && (
                    <>
                      <Field>
                        <Label>{t('visualDashboard.backgroundImageUrl')}</Label>
                        <Input
                          value={config.backgroundImage || ''}
                          onChange={(e) => updateConfig('backgroundImage')(e.target.value)}
                          placeholder={t('placeholders.urlExample')}
                          className="h-9"
                        />
                      </Field>
                      <Field>
                        <Label>{t('visualDashboard.orUploadImage')}</Label>
                        <div className="flex items-center gap-2">
                          <Input
                            type="file"
                            accept="image/*"
                            onChange={(e) => {
                              const file = e.target.files?.[0]
                              if (file) {
                                const reader = new FileReader()
                                reader.onload = (e) => {
                                  updateConfig('backgroundImage')(e.target?.result as string)
                                }
                                reader.readAsDataURL(file)
                              }
                            }}
                            className="h-9"
                          />
                          {config.backgroundImage && (
                            <Button
                              type="button"
                              variant="outline"
                              size="sm"
                              onClick={() => updateConfig('backgroundImage')('')}
                            >
                              {t('visualDashboard.clear')}
                            </Button>
                          )}
                        </div>
                      </Field>
                      {config.backgroundImage && (
                        <div className="mt-2">
                          <Label className="text-xs text-muted-foreground">{t('visualDashboard.preview')}</Label>
                          <div
                            className="w-full h-24 bg-muted rounded-md bg-cover bg-center border"
                            style={{ backgroundImage: `url(${config.backgroundImage})` }}
                          />
                        </div>
                      )}
                    </>
                  )}

                  {config.backgroundType === 'grid' && (
                    <Field>
                      <Label>{t('visualDashboard.gridSize')}</Label>
                      <Input
                        type="number"
                        min={10}
                        max={50}
                        value={config.gridSize ?? 20}
                        onChange={(e) => updateConfig('gridSize')(Number(e.target.value))}
                        className="h-9"
                      />
                    </Field>
                  )}

                  <div className="grid grid-cols-2 gap-3">
                    <Field>
                      <Label>{t('visualDashboard.showControlBar')}</Label>
                      <Select
                        value={String(config.showControls ?? true)}
                        onValueChange={(value) => updateConfig('showControls')(value === 'true')}
                      >
                        <SelectTrigger className="w-full h-9">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="true">{t('visualDashboard.showCard')}</SelectItem>
                          <SelectItem value="false">{t('visualDashboard.hide')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </Field>
                    <Field>
                      <Label>{t('visualDashboard.showFullscreenButton')}</Label>
                      <Select
                        value={String(config.showFullscreen ?? true)}
                        onValueChange={(value) => updateConfig('showFullscreen')(value === 'true')}
                      >
                        <SelectTrigger className="w-full h-9">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="true">{t('visualDashboard.showCard')}</SelectItem>
                          <SelectItem value="false">{t('visualDashboard.hide')}</SelectItem>
                        </SelectContent>
                      </Select>
                    </Field>
                  </div>
                </div>
              ),
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-4">
                  {/* Data source selection — merged from Data Source tab */}
                  <BindingDataSourceSelector
                    dataSource={config.bindings as any}
                    onConfirm={(newDataSources) => {
                      const sourcesArray = newDataSources
                        ? Array.isArray(newDataSources)
                          ? newDataSources
                          : [newDataSources]
                        : []

                      const newBindings = sourcesArray.map((ds: any, index: number) => {
                        let bindingType: LayerBindingType = 'device'
                        if (ds.type === 'metric' || ds.type === 'telemetry') bindingType = 'metric'
                        else if (ds.type === 'command') bindingType = 'command'

                        const existingBinding = (config.bindings as LayerBinding[])?.find(b => {
                          if (!b.dataSource) return false
                          const bDs = b.dataSource as any
                          return getSourceId(bDs) === getSourceId(ds) &&
                            bDs.metricId === ds.metricId &&
                            bDs.property === ds.property &&
                            bDs.command === ds.command
                        })

                        const generateBindingId = () => {
                          if (ds.type === 'metric' || ds.type === 'telemetry') {
                            return `${bindingType}-${getSourceId(ds)}-${ds.metricId || ds.property || index}`
                          } else if (ds.type === 'command') {
                            return `${bindingType}-${getSourceId(ds)}-${ds.command}`
                          } else {
                            return `${bindingType}-${getSourceId(ds)}-${index}`
                          }
                        }

                        const baseBinding = existingBinding || {
                          id: generateBindingId(),
                          position: { x: 50, y: 50 },
                        }

                        return {
                          ...baseBinding,
                          id: existingBinding?.id || generateBindingId(),
                          type: bindingType,
                          icon: bindingType,
                          name: (ds.type === 'metric' || ds.type === 'telemetry')
                            ? (ds.metricId || ds.property || t('visualDashboard.metricIndex', { index: index + 1 }))
                            : ds.type === 'command'
                              ? `${getSourceId(ds) || ''} → ${ds.command || ''}`
                              : (getSourceId(ds) || t('visualDashboard.deviceIndex', { index: index + 1 })),
                          dataSource: ds,
                          position: existingBinding?.position || baseBinding.position,
                        } as LayerBinding
                      })

                      const existingTextIconBindings = (config.bindings as LayerBinding[])?.filter(b => {
                        if (b.type === 'text' || b.type === 'icon') return true
                        const ds = b.dataSource as any
                        if (ds && getSourceId(ds)) {
                          return !sourcesArray.some((s: any) => getSourceId(s) === getSourceId(ds))
                        }
                        return false
                      }) || []

                      updateConfig('bindings')([...newBindings, ...existingTextIconBindings])
                    }}
                    allowedTypes={['device', 'metric', 'command', 'extension']}
                    maxSources={20}
                    title={t('visualDashboard.layerItemBinding')}
                  />

                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="text-sm font-medium">{t('visualDashboard.layerItemBinding')}</h3>
                      <p className="text-xs text-muted-foreground mt-1">
                        {t('visualDashboard.manageLayerItems')}
                      </p>
                    </div>
                    <Button
                      type="button"
                      variant="default"
                      size="sm"
                      onClick={() => {
                        const latestDashboard = useStore.getState().currentDashboard
                        const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent?.id)
                        const latestBindings = (latestComponent as any)?.config?.bindings as LayerBinding[] || []
                        setLayerEditorBindings(latestBindings)
                        setLayerEditorOpen(true)
                      }}
                    >
                      <Layers className="h-4 w-4 mr-1" />
                      {t('visualDashboard.openLayerEditor')}
                    </Button>
                  </div>

                  {/* Bindings List - Grouped by Type */}
                  <div className="border rounded-lg overflow-hidden">
                    {(() => {
                      const latestDashboard = useStore.getState().currentDashboard
                      const latestComponent = latestDashboard?.components.find(c => c.id === selectedComponent?.id)
                      const displayBindings = (latestComponent as any)?.config?.bindings as LayerBinding[] || []

                      // Group by type
                      const groupedBindings = {
                        device: displayBindings.filter(b => b.type === 'device'),
                        metric: displayBindings.filter(b => b.type === 'metric'),
                        command: displayBindings.filter(b => b.type === 'command'),
                        text: displayBindings.filter(b => b.type === 'text'),
                        icon: displayBindings.filter(b => b.type === 'icon'),
                      }

                      const LAYER_TYPE_CONFIG = {
                        device: {
                          label: t('layerDisplay.device'),
                          color: 'bg-success',
                          textColor: 'text-success',
                          bgColor: 'bg-success-light dark:bg-success-light',
                          borderColor: 'border-success-light dark:border-success-light',
                          icon: MapPin,
                          description: t('layerDisplay.deviceDesc')
                        },
                        metric: {
                          label: t('layerDisplay.metric'),
                          color: 'bg-accent-purple',
                          textColor: 'text-accent-purple',
                          bgColor: 'bg-accent-purple-light',
                          borderColor: 'border-accent-purple-light',
                          icon: Activity,
                          description: t('layerDisplay.metricDesc')
                        },
                        command: {
                          label: t('layerDisplay.command'),
                          color: 'bg-info',
                          textColor: 'text-info',
                          bgColor: 'bg-info-light',
                          borderColor: 'border-info',
                          icon: Zap,
                          description: t('layerDisplay.commandDesc')
                        },
                        text: {
                          label: t('layerDisplay.text'),
                          color: 'bg-muted-foreground',
                          textColor: 'text-muted-foreground',
                          bgColor: 'bg-muted',
                          borderColor: 'border-border',
                          icon: Type,
                          description: t('layerDisplay.textDesc')
                        },
                        icon: {
                          label: t('layerDisplay.icon'),
                          color: 'bg-accent-orange',
                          textColor: 'text-accent-orange',
                          bgColor: 'bg-accent-orange-light',
                          borderColor: 'border-accent-orange-light',
                          icon: Sparkles,
                          description: t('layerDisplay.iconDesc')
                        },
                      } as const

                      if (displayBindings.length === 0) {
                        return (
                          <div className="p-6 text-center text-muted-foreground">
                            <Layers className="h-8 w-8 mx-auto mb-2 opacity-50" />
                            <p className="text-sm">{t('visualDashboard.noLayerItems')}</p>
                            <p className="text-xs mt-1">{t('visualDashboard.addLayerItemHint')}</p>
                          </div>
                        )
                      }

                      return (Object.keys(groupedBindings) as Array<keyof typeof groupedBindings>).map(type => {
                        const typeBindings = groupedBindings[type]
                        if (typeBindings.length === 0) return null

                        const typeConfig = LAYER_TYPE_CONFIG[type]
                        const Icon = typeConfig.icon

                        return (
                          <div key={type} className="border-b last:border-b-0">
                            <div className={`px-3 py-2 ${typeConfig.bgColor} border-b ${typeConfig.borderColor} flex items-center justify-between`}>
                              <div className="flex items-center gap-2">
                                <div className={`w-5 h-5 rounded-full ${typeConfig.color} flex items-center justify-center`}>
                                  <Icon className="h-4 w-4 text-primary-foreground" />
                                </div>
                                <span className="text-sm font-medium">{typeConfig.label}</span>
                                <span className="text-xs text-muted-foreground">({typeBindings.length})</span>
                              </div>
                              <span className="text-xs text-muted-foreground">{typeConfig.description}</span>
                            </div>

                            <div className="divide-y">
                              {typeBindings.map((binding) => {
                                const positionText = binding.position && binding.position !== 'auto'
                                  ? `(${binding.position.x.toFixed(0)}%, ${binding.position.y.toFixed(0)}%)`
                                  : t('visualDashboard.center')

                                const ds = binding.dataSource as any
                                const deviceId = getSourceId(ds)
                                const metricId = ds?.metricId || ds?.property
                                const command = ds?.command

                                return (
                                  <div
                                    key={binding.id}
                                    className="flex items-center gap-3 p-3 hover:bg-muted-50 transition-colors"
                                  >
                                    <div className={`w-8 h-8 rounded-full flex items-center justify-center ${typeConfig.color}/20 ${typeConfig.textColor}`}>
                                      <Icon className="h-4 w-4" />
                                    </div>
                                    <div className="flex-1 min-w-0">
                                      <div className="text-sm font-medium truncate">{binding.name}</div>
                                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                                        <span>{positionText}</span>
                                        {deviceId && <span>• {deviceId.slice(0, 8)}...</span>}
                                        {metricId && <span>• {metricId}</span>}
                                        {command && <span>• {command}</span>}
                                      </div>
                                    </div>
                                  </div>
                                )
                              })}
                            </div>
                          </div>
                        )
                      })
                    })()}
                  </div>
                </div>
              ),
            },
          ],
        }

      // ========== Business Components ==========
      case 'agent-monitor-widget':
        // Agent selection in display config (keeps layout balanced with hasDataSource=true)
        return {
          displaySections: [
            {
              type: 'custom' as const,
              render: () => {
                // Read from componentConfig which is kept up-to-date by updateDataSource
                const currentAgentId = (config.dataSource as any)?.agentId || ''
                // Use agents directly from component state (loaded by the agents loading effect)
                const agentsList = agents
                
                return (
                  <div className="space-y-3">
                    <Field>
                      <Label>{t('dashboardComponents:agentMonitorWidget.selectAgent')}</Label>
                      <Select
                        value={currentAgentId}
                        onValueChange={(value) => {
                          
                          updateDataSource({ type: 'agent', agentId: value })
                        }}
                        disabled={agentsLoading}
                      >
                        <SelectTrigger className="h-9">
                          <SelectValue placeholder={agentsLoading ? t('common:loading') : t('dashboardComponents:agentMonitorWidget.selectAgent')} />
                        </SelectTrigger>
                        <SelectContent>
                          {agentsList.map((agent: any) => (
                            <SelectItem key={agent.id} value={agent.id}>
                              {agent.name}
                            </SelectItem>
                          ))}
                          {agentsList.length === 0 && !agentsLoading && (
                            <div className="px-2 py-4 text-center text-sm text-muted-foreground">
                              {t('agents:noAgents')}
                            </div>
                          )}
                        </SelectContent>
                      </Select>
                    </Field>
                  </div>
                )
              },
            },
          ],
        }

      // ========== AI Analyst ==========
      case 'ai-analyst':
        return {
          dataSourceSections: [
            {
              type: 'data-source' as const,
              props: {
                dataSource: config.dataSource,
                onChange: updateDataSource,
                allowedTypes: ['device-metric', 'extension', 'ai-metric', 'command', 'extension-command'],
                multiple: true,
              },
            },
          ],
          displaySections: [
            {
              type: 'custom' as const,
              render: () => {
                const modelsList = visionModels
                return (
                  <div className="space-y-3">
                    <Field>
                      <Label>{t('dashboardComponents:aiAnalyst.selectModel')}</Label>
                      <Select
                        value={config.modelId || ''}
                        onValueChange={(value) => updateConfig('modelId')(value)}
                        disabled={visionModelsLoading}
                      >
                        <SelectTrigger className="h-9">
                          <SelectValue placeholder={visionModelsLoading ? t('common:loading') : t('dashboardComponents:aiAnalyst.selectModelPlaceholder')} />
                        </SelectTrigger>
                        <SelectContent>
                          {modelsList.map((model: any) => (
                            <SelectItem key={model.id} value={model.id}>
                              <div className="flex items-center gap-2">
                                <span>{model.name}</span>
                                <span className="text-xs text-muted-foreground">({model.backendName})</span>
                              </div>
                            </SelectItem>
                          ))}
                          {modelsList.length === 0 && !visionModelsLoading && (
                            <div className="px-2 py-4 text-center text-sm text-muted-foreground">
                              {t('dashboardComponents:aiAnalyst.noModels')}
                            </div>
                          )}
                        </SelectContent>
                      </Select>
                    </Field>
                    <Field>
                      <Label>{t('dashboardComponents:aiAnalyst.systemPrompt')}</Label>
                      <Textarea
                        value={config.systemPrompt || ''}
                        onChange={(e) => updateConfig('systemPrompt')(e.target.value)}
                        placeholder={t('dashboardComponents:aiAnalyst.systemPromptPlaceholder')}
                        className="resize-y"
                      />
                    </Field>
                    <Field>
                      <Label>{t('dashboardComponents:aiAnalyst.contextWindow')}</Label>
                      <Input
                        type="number"
                        min={1}
                        max={100}
                        value={config.contextWindowSize || 10}
                        onChange={(e) => updateConfig('contextWindowSize')(Number(e.target.value) || 10)}
                        className="h-9"
                      />
                    </Field>
                  </div>
                )
              },
            },
          ],
        }

      default:
        // Check if this is an extension or community component
        const extensionDto = dynamicRegistry.getMeta(componentType)
        const communityMeta = communityRegistry.getMeta(componentType)
        const schemaSource = extensionDto?.config_schema?.properties
          ? extensionDto
          : communityMeta?.config_schema?.properties
            ? communityMeta
            : null

        if (schemaSource?.config_schema?.properties) {
          // Generate config UI from JSON Schema (extension or community)
          const properties = schemaSource.config_schema.properties
          const uiHints = schemaSource.config_schema.ui_hints
          const fieldOrder = uiHints?.field_order || Object.keys(properties)

          // Visibility rules: check if a field should be visible based on current config values
          const isFieldVisible = (fieldName: string): boolean => {
            if (!uiHints?.visibility_rules) return true
            const rules = uiHints.visibility_rules as Array<{ field: string; condition: string; value: any; then_show?: string[]; then_hide?: string[] }>
            for (const rule of rules) {
              if (rule.then_show?.includes(fieldName)) {
                const ruleValue = config[rule.field] ?? schemaSource.default_config?.[rule.field]
                let show = false
                switch (rule.condition) {
                  case 'equals': show = ruleValue === rule.value; break
                  case 'not_equals': show = ruleValue !== rule.value; break
                  case 'contains': show = Array.isArray(ruleValue) && ruleValue.includes(rule.value); break
                  case 'empty': show = !ruleValue || (Array.isArray(ruleValue) && ruleValue.length === 0); break
                  case 'not_empty': show = !!ruleValue && (!Array.isArray(ruleValue) || ruleValue.length > 0); break
                }
                if (show) return true
              }
              if (rule.then_hide?.includes(fieldName)) {
                const ruleValue = config[rule.field] ?? schemaSource.default_config?.[rule.field]
                let hide = false
                switch (rule.condition) {
                  case 'equals': hide = ruleValue === rule.value; break
                  case 'not_equals': hide = ruleValue !== rule.value; break
                }
                if (hide) return false
              }
            }
            // If field appears in any then_show rule but no rule matched, it's hidden
            const appearsInThenShow = rules.some(r => r.then_show?.includes(fieldName))
            return !appearsInThenShow
          }

          const displaySections: ConfigSection[] = [
            {
              type: 'custom' as const,
              render: () => (
                <div className="space-y-3">
                  {fieldOrder.filter(key => properties[key] && isFieldVisible(key)).map((key) => {
                    const propDef = properties[key]
                    const propValue = config[key] ?? schemaSource.default_config?.[key] ?? propDef.default

                    const handleChange = (value: any) => {
                      updateConfig(key)(value)
                    }

                    // Render based on property type
                    const fieldLabel = propDef.title || propDef.description || key

                    switch (propDef.type) {
                      case 'boolean':
                        return (
                          <label key={key} className="flex items-center gap-2 cursor-pointer">
                            <Checkbox
                              checked={propValue ?? false}
                              onCheckedChange={(checked) => handleChange(!!checked)}
                            />
                            <span className="text-sm font-medium">{fieldLabel}</span>
                          </label>
                        )

                      case 'number':
                        return (
                          <Field key={key}>
                            <Label>{fieldLabel}</Label>
                            <Input
                              type="number"
                              value={propValue ?? 0}
                              onChange={(e) => handleChange(Number(e.target.value))}
                              min={propDef.minimum}
                              max={propDef.maximum}
                              step={propDef.type === 'number' ? (propDef.multipleOf || 1) : undefined}
                              className="h-9"
                            />
                          </Field>
                        )

                      case 'integer':
                        return (
                          <Field key={key}>
                            <Label>{fieldLabel}</Label>
                            <Input
                              type="number"
                              value={propValue ?? 0}
                              onChange={(e) => handleChange(Math.floor(Number(e.target.value)))}
                              min={propDef.minimum}
                              max={propDef.maximum}
                              step="1"
                              className="h-9"
                            />
                          </Field>
                        )

                      case 'string':
                        if (propDef.enum) {
                          // Select dropdown for enum values
                          // Support enumTitles for friendly display names
                          const enumLabels = propDef.enumTitles || propDef.enum
                          return (
                            <Field key={key}>
                              <Label>{fieldLabel}</Label>
                              <Select
                                value={propValue ?? propDef.default ?? propDef.enum[0]}
                                onValueChange={(value) => handleChange(value)}
                              >
                                <SelectTrigger className="w-full h-9">
                                  <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                  {propDef.enum.map((enumValue: string, idx: number) => (
                                    <SelectItem key={enumValue} value={enumValue}>
                                      {enumLabels[idx]}
                                    </SelectItem>
                                  ))}
                                </SelectContent>
                              </Select>
                            </Field>
                          )
                        }
                        // Regular text input
                        return (
                          <Field key={key}>
                            <Label>{fieldLabel}</Label>
                            <Input
                              value={propValue ?? ''}
                              onChange={(e) => handleChange(e.target.value)}
                              placeholder={propDef.description || fieldLabel}
                              className="h-9"
                            />
                          </Field>
                        )

                      case 'array':
                        return (
                          <Field key={key}>
                            <Label>{fieldLabel}</Label>
                            <Input
                              value={Array.isArray(propValue) ? propValue.join(', ') : ''}
                              onChange={(e) => handleChange(e.target.value.split(',').map((s: string) => s.trim()))}
                              placeholder="Comma-separated values"
                              className="h-9"
                            />
                          </Field>
                        )

                      default:
                        return null
                    }
                  })}
                </div>
              ),
            },
          ]

          // Add device binding section if component requires it
          if (communityMeta?.has_device_binding) {
            displaySections.push({
              type: 'custom' as const,
              render: () => (
                <DeviceBindingConfig
                  deviceId={config.deviceBinding?.deviceId}
                  deviceTypeFilter={communityMeta.device_type_filter}
                  onChange={(deviceId) => {
                    updateConfig('deviceBinding')({ deviceId: deviceId || undefined })
                  }}
                />
              ),
            })
          }

          // Add data source section if component supports it
          let dataSourceSections: ConfigSection[] = []
          if (schemaSource.has_data_source) {
            // Use custom allowedTypes from manifest if specified, otherwise default
            const dsAllowedTypes = (schemaSource.data_source_allowed_types || ['device-metric', 'extension', 'extension-command']) as any
            dataSourceSections = [
              {
                type: 'data-source' as const,
                props: {
                  dataSource: config.dataSource,
                  onChange: updateDataSource,
                  allowedTypes: dsAllowedTypes,
                },
              },
            ]
          }

          return {
            displaySections,
            dataSourceSections,
            styleSections: [],
          }
        }

        // Community component with device binding but no config_schema
        if (communityMeta?.has_device_binding) {
          return {
            displaySections: [
              {
                type: 'custom' as const,
                render: () => (
                  <DeviceBindingConfig
                    deviceId={config.deviceBinding?.deviceId}
                    deviceTypeFilter={communityMeta.device_type_filter}
                    onChange={(deviceId) => {
                      updateConfig('deviceBinding')({ deviceId: deviceId || undefined })
                    }}
                  />
                ),
              },
            ],
            dataSourceSections: [],
            styleSections: [],
          }
        }

        return null
    }
  }

  if (!currentDashboard) {
    // Show loading state only if we're still loading
    if (dashboardsLoading) {
      return (
        <div className="flex items-center justify-center h-screen">
          <div className="text-center">
            <h2 className="text-lg font-medium mb-2">Loading Dashboard...</h2>
          </div>
        </div>
      )
    }

    // No dashboard found - show empty state with create button
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-center space-y-4 px-4">
          <LayoutDashboard className="h-16 w-16 mx-auto text-muted-foreground" />
          <div>
            <h2 className="text-lg font-medium mb-1">No Dashboard Found</h2>
            <p className="text-sm text-muted-foreground mb-4">
              Create your first dashboard to get started
            </p>
            <Button
              onClick={() => {
                handleDashboardCreate('Overview').catch((err) => {
                  console.error('[VisualDashboard] Failed to create dashboard:', err)
                })
              }}
            >
              <Plus className="h-4 w-4 mr-1" />
              Create Dashboard
            </Button>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="flex h-full overflow-hidden bg-background">
      {/* Sidebar - Dashboard List - hidden in fullscreen */}
      {!isFullscreen && (
        <DashboardListSidebar
          dashboards={dashboards}
          currentDashboardId={currentDashboardId}
          onSwitch={handleDashboardSwitch}
          onCreate={handleDashboardCreate}
          onRename={handleDashboardRename}
          onDelete={handleDashboardDelete}
          open={sidebarOpen}
          onOpenChange={handleSidebarOpenChange}
          isDesktop={isDesktop}
        />
      )}

      {/* Main Content */}
      {isFullscreen && createPortal(
        <div className="fixed inset-0 z-[100] bg-background flex flex-col">
          <div className="flex-1 overflow-auto p-4 relative">
            <Button
              variant="outline"
              size="icon"
              onClick={toggleFullscreen}
              className="absolute top-4 right-4 z-50 shadow-lg bg-bg-90 backdrop-blur"
              title={t('visualDashboard.exitFullscreen')}
            >
              <Minimize className="h-4 w-4" />
            </Button>
            <DashboardGrid
              components={gridComponents}
              editMode={false}
              onLayoutChange={() => {}}
            />
          </div>
        </div>,
        document.body
      )}
      <div className={cn(
        "flex-1 flex flex-col overflow-hidden",
        isFullscreen && "hidden"
      )}>
          <header className="shrink-0 flex items-center justify-between px-4 py-3 border-b border-border bg-background z-10">
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="icon"
                onClick={() => isMobile ? handleSidebarOpenChange(true) : handleSidebarOpenChange(!sidebarOpen)}
                className={cn(
                  "h-6 w-6 active:scale-95",
                  !isDesktop && sidebarOpen && "bg-muted"
                )}
              >
                <PanelsTopLeft className="h-4 w-4" />
              </Button>
              <h1 className="text-sm font-semibold">
                {currentDashboard.name}
              </h1>
            </div>

            <div className="flex items-center gap-1.5">
              <Button
                variant={editMode ? "default" : "outline"}
                size="sm"
                onClick={() => setEditMode(!editMode)}
                className={cn("h-7 text-xs rounded-md", editMode ? "shadow-sm" : "")}
              >
                {editMode ? (
                  <>
                    <Check className="h-4 w-4 mr-1" />
                    <span className="hidden sm:inline">Done</span>
                    <span className="sm:hidden">Done</span>
                  </>
                ) : (
                  <>
                    <Settings2 className="h-4 w-4 mr-1" />
                    <span className="hidden sm:inline">Edit</span>
                    <span className="sm:hidden">Edit</span>
                  </>
                )}
              </Button>

              <Button
                variant="default"
                size="sm"
                className="h-7 text-xs rounded-md shadow-sm"
                disabled={!editMode}
                onClick={() => editMode && setComponentLibraryOpen(true)}
              >
                <Plus className="h-4 w-4 mr-1" />
                <span className="hidden sm:inline">{t('visualDashboard.addComponent')}</span>
                <span className="sm:hidden">{t('visualDashboard.addComponent')}</span>
              </Button>

              <Button
                variant="outline"
                size="sm"
                className="h-7 text-xs rounded-md"
                onClick={() => setShareDialogOpen(true)}
              >
                <Share2 className="h-4 w-4 mr-1" />
                <span className="hidden sm:inline">{t('visualDashboard.share.title')}</span>
              </Button>

              <FullScreenDialog open={componentLibraryOpen} onOpenChange={(open) => {
                setComponentLibraryOpen(open)
                if (!open) { setLibrarySearch(''); setLibraryTab('components') }
              }}>
                <FullScreenDialogHeader
                  icon={<LayoutGrid className="h-5 w-5" />}
                  iconBg="bg-info-light"
                  iconColor="text-info"
                  title={t('visualDashboard.componentLibrary')}
                  onClose={() => {
                    setComponentLibraryOpen(false)
                    setLibrarySearch('')
                    setLibraryTab('components')
                  }}
                />

                <FullScreenDialogContent>
                  <div className="flex-1 overflow-hidden flex flex-col">
                    {/* Tabs */}
                    <div className="px-4 md:px-6 pt-4 pb-2 shrink-0 space-y-3">
                      <div className="flex items-center gap-3">
                        <Tabs value={libraryTab} onValueChange={(v) => setLibraryTab(v as 'components' | 'marketplace')} className="flex-1">
                          <TabsList className="h-8">
                            <TabsTrigger value="components" className="gap-1.5 text-xs px-3">
                              <LayoutGrid className="w-3.5 h-3.5" />
                              {t('componentLibrary.tabComponents')}
                            </TabsTrigger>
                            <TabsTrigger value="marketplace" className="gap-1.5 text-xs px-3">
                              <StoreIcon className="w-3.5 h-3.5" />
                              {t('componentLibrary.tabMarketplace')}
                            </TabsTrigger>
                          </TabsList>
                        </Tabs>
                        {libraryTab === 'marketplace' && (
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-8 gap-1.5 text-xs"
                            onClick={() => setImportDialogOpen(true)}
                          >
                            <PackagePlus className="w-3.5 h-3.5" />
                            {t('componentLibrary.importComponent')}
                          </Button>
                        )}
                      </div>

                      {/* Search (only in components tab) */}
                      {libraryTab === 'components' && (
                        <div className="relative">
                          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
                          <Input
                            value={librarySearch}
                            onChange={(e) => setLibrarySearch(e.target.value)}
                            placeholder={t('componentLibrary.searchPlaceholder')}
                            className="h-9 pl-8"
                          />
                        </div>
                      )}
                    </div>

                    {/* Tab Content */}
                    {libraryTab === 'components' ? (
                      /* Scrollable categories */
                      <div className="flex-1 overflow-y-auto px-4 md:px-6 pb-6 space-y-1">
                        {filteredLibrary.length === 0 ? (
                          <div className="text-center py-12 text-muted-foreground">
                            <p className="text-sm">{t('componentLibrary.noResults')}</p>
                            <p className="text-xs mt-1">{t('componentLibrary.noResultsHint')}</p>
                          </div>
                        ) : (
                          filteredLibrary.map((category) => (
                            <Collapsible
                              key={category.category}
                              defaultOpen={true}
                            >
                              <CollapsibleTrigger className="w-full flex items-center gap-2 py-2 px-1 hover:bg-muted-50 rounded-md transition-colors group">
                                <category.categoryIcon className="h-4 w-4 text-muted-foreground" />
                                <span className="text-sm font-medium flex-1 text-left">{category.categoryLabel}</span>
                                <span className="text-xs text-muted-foreground bg-muted rounded-full px-1.5 py-0.5 min-w-[24px] text-center">
                                  {category.items.length}
                                </span>
                                <ChevronDown className="h-3.5 w-3.5 text-muted-foreground transition-transform group-data-[state=open]:rotate-180" />
                              </CollapsibleTrigger>
                              <CollapsibleContent>
                                <div className="grid grid-cols-4 md:grid-cols-5 lg:grid-cols-6 gap-2 pb-3 px-1">
                                  {category.items.map((item) => {
                                    const Icon = item.icon
                                    return (
                                      <button
                                        key={item.id}
                                        type="button"
                                        onClick={() => handleAddComponent(item.id)}
                                        className="h-auto w-full flex flex-col items-center p-3 text-center rounded-lg border border-input bg-background hover:bg-accent hover:text-accent-foreground transition-colors cursor-pointer active:scale-[0.98]"
                                      >
                                        <Icon className="h-5 w-5 mb-1.5 text-muted-foreground shrink-0" />
                                        <span className="text-xs font-medium w-full truncate">{item.name}</span>
                                        <p className="text-[10px] text-muted-foreground mt-0.5 w-full line-clamp-2 leading-tight">{item.description}</p>
                                      </button>
                                    )
                                  })}
                                </div>
                              </CollapsibleContent>
                            </Collapsible>
                          ))
                        )}
                      </div>
                    ) : (
                      /* Marketplace tab */
                      <div className="flex-1 overflow-y-auto px-4 md:px-6 pt-4 pb-6">
                        {marketLoading ? (
                          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
                            {Array.from({ length: 6 }).map((_, i) => (
                              <div key={i} className="rounded-lg border border-border p-4 space-y-3">
                                <div className="w-10 h-10 rounded-lg bg-muted animate-pulse" />
                                <div className="h-4 bg-muted rounded w-3/4 animate-pulse" />
                                <div className="h-3 bg-muted rounded w-full animate-pulse" />
                              </div>
                            ))}
                          </div>
                        ) : marketComponents.length === 0 ? (
                          <div className="flex flex-col items-center justify-center py-16 text-center">
                            <Upload className="h-10 w-10 text-muted-foreground mb-3" />
                            <p className="text-sm text-muted-foreground">{t('componentLibrary.marketplaceEmpty')}</p>
                          </div>
                        ) : (
                          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                            {marketComponents.map((mc: MarketComponentEntry) => {
                              const isInstalled = installedComponents.some(c => c.id === mc.id)
                              const McIcon = (lucideReact as any)[mc.icon || 'Box'] || Box
                              const mcName = typeof mc.name === 'string' ? mc.name : (mc.name[i18n.language] || mc.name.en || Object.values(mc.name)[0] || mc.id)
                              const mcDesc = typeof mc.description === 'string' ? mc.description : (mc.description[i18n.language] || mc.description.en || Object.values(mc.description)[0] || '')
                              return (
                                <div key={mc.id} className="rounded-lg border border-border bg-card p-3 flex flex-col gap-2 h-[140px]">
                                    <div className="flex items-start gap-2">
                                      <div className="w-8 h-8 rounded-md bg-muted flex items-center justify-center shrink-0">
                                        <McIcon className="w-4 h-4 text-primary" />
                                      </div>
                                      <div className="flex-1 min-w-0">
                                        <div className="flex items-center gap-2">
                                          <span className="text-sm font-medium text-foreground truncate">{mcName}</span>
                                          {isInstalled && <Check className="w-3.5 h-3.5 text-success shrink-0" />}
                                        </div>
                                        <p className="text-xs text-muted-foreground">{t('componentLibrary.version')}: {mc.version}{mc.author ? ` · ${mc.author}` : ''}</p>
                                      </div>
                                    </div>
                                    <p className="text-xs text-muted-foreground line-clamp-2 flex-1 min-h-0">{mcDesc}</p>
                                    <Button
                                      variant={isInstalled ? 'ghost' : 'outline'}
                                      size="sm"
                                      className="w-full h-7 text-xs"
                                      disabled={installingId === mc.id}
                                      onClick={async () => {
                                        setInstallingId(mc.id)
                                        try {
                                          if (isInstalled) {
                                            await uninstallComponent(mc.id)
                                            notifySuccess(t('componentLibrary.uninstallSuccess'))
                                          } else {
                                            await installFromMarket(mc.id)
                                            notifySuccess(t('componentLibrary.installSuccess'))
                                          }
                                        } catch (e) {
                                          notifyError(t('componentLibrary.installError'))
                                        } finally {
                                          setInstallingId(null)
                                        }
                                      }}
                                    >
                                      {installingId === mc.id ? (
                                        <><Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" />{isInstalled ? t('componentLibrary.uninstall') : t('componentLibrary.install')}</>
                                      ) : isInstalled ? (
                                        <><Trash2 className="w-3.5 h-3.5 mr-1" />{t('componentLibrary.uninstall')}</>
                                      ) : (
                                        <><Download className="w-3.5 h-3.5 mr-1" />{t('componentLibrary.install')}</>
                                      )}
                                    </Button>
                                  </div>
                              )
                            })}
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                </FullScreenDialogContent>
              </FullScreenDialog>

              {/* Import Dialog */}
              <InstallComponentDialog open={importDialogOpen} onOpenChange={setImportDialogOpen} />

              {/* Fullscreen toggle button */}
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6"
                onClick={toggleFullscreen}
                title={t('visualDashboard.fullscreen')}
              >
                <Maximize className="h-4 w-4" />
              </Button>
            </div>
          </header>

        {/* Dashboard Grid */}
        <div className="flex-1 overflow-auto p-4 relative">

          {(currentDashboard.components?.length ?? 0) === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-muted-foreground">
              <LayoutDashboard className="h-16 w-16 mb-4 opacity-50" />
              <p className="text-lg font-medium">{t('visualDashboard.emptyDashboard')}</p>
              <p className="text-sm mt-2">
                {editMode ? t('visualDashboard.addComponentsHint') : t('visualDashboard.enterEditModeHint')}
              </p>
              {editMode && (
                <Button
                  variant="outline"
                  size="sm"
                  className="mt-4"
                  onClick={() => setComponentLibraryOpen(true)}
                >
                  <Plus className="h-4 w-4 mr-1" />
                  {t('visualDashboard.addComponent')}
                </Button>
              )}
            </div>
          ) : (
            <DashboardGrid
              components={gridComponents}
              editMode={editMode}
              onLayoutChange={handleLayoutChange}
            />
          )}
        </div>
      </div>

      {/* Config Dialog */}
      <ComponentConfigDialog
        open={configOpen}
        onClose={handleCancelConfig}
        onSave={handleSaveConfig}
        title={configTitle}
        onTitleChange={handleTitleChange}
        configSchema={configSchema}
        componentType={selectedComponent?.type || ''}
        previewDataSource={componentConfig.dataSource}
        previewConfig={componentConfig}
        showTitleInDisplay={isTitleInDisplayComponent(selectedComponent?.type)}
        position={selectedComponent?.position}
      />

      {/* Map Editor Dialog */}
      <MapEditorDialog
        open={mapEditorOpen}
        onOpenChange={setMapEditorOpen}
        bindings={mapEditorBindings}
        center={(componentConfig.center as { lat: number; lng: number }) || { lat: 39.9042, lng: 116.4074 }}
        zoom={componentConfig.zoom as number || 10}
        tileLayer={componentConfig.tileLayer as string || 'osm'}
        onSave={handleMapEditorSave}
      />

      {/* Center Picker Dialog */}
      <CenterPickerDialog
        open={centerPickerOpen}
        onOpenChange={setCenterPickerOpen}
        center={(componentConfig.center as { lat: number; lng: number }) || { lat: 39.9042, lng: 116.4074 }}
        zoom={componentConfig.zoom as number || 10}
        tileLayer={componentConfig.tileLayer as string || 'osm'}
        onSave={handleCenterPickerSave}
      />

      {/* Layer Editor Dialog */}
      <LayerEditorDialog
        open={layerEditorOpen}
        onOpenChange={setLayerEditorOpen}
        bindings={layerEditorBindings}
        backgroundType={componentConfig.backgroundType as 'color' | 'image' | 'transparent' | 'grid' || 'grid'}
        backgroundColor={componentConfig.backgroundColor as string}
        backgroundImage={componentConfig.backgroundImage as string}
        onSave={handleLayerEditorSave}
      />

      {/* Mobile Edit Bar */}
      {isMobile && mobileEditBarOpen && mobileSelectedId && (
        <MobileEditBar
          isOpen={mobileEditBarOpen}
          onClose={() => {
            setMobileEditBarOpen(false)
            setMobileSelectedId(null)
          }}
          onSettings={() => {
            const comp = currentDashboard?.components.find(c => c.id === mobileSelectedId)
            if (comp) {
              handleOpenConfig(comp.id)
              setMobileEditBarOpen(false)
            }
          }}
          onCopy={() => {
            if (mobileSelectedId) {
              duplicateComponent(mobileSelectedId)
              setMobileEditBarOpen(false)
              setMobileSelectedId(null)
            }
          }}
          onDelete={() => {
            if (mobileSelectedId) {
              removeComponent(mobileSelectedId)
              setMobileEditBarOpen(false)
              setMobileSelectedId(null)
            }
          }}
          componentName={currentDashboard?.components.find(c => c.id === mobileSelectedId)?.title}
        />
      )}

      {/* Share Management Dialog */}
      <ShareManagerDialog
        open={shareDialogOpen}
        onOpenChange={setShareDialogOpen}
        dashboardId={currentDashboardId}
        dashboardName={currentDashboard?.name}
      />

      {/* Extension Fullscreen Dialog */}
      <FullScreenDialog
        open={!!extFullscreenContent}
        onOpenChange={(open) => { if (!open) closeExtFullscreen() }}
      >
        <FullScreenDialogHeader
          icon={<Monitor className="w-5 h-5" />}
          title="Edit Content"
          onClose={closeExtFullscreen}
        />
        <FullScreenDialogContent>
          {extFullscreenContent}
        </FullScreenDialogContent>
      </FullScreenDialog>
    </div>
  )
})

// Export the memoized component
export { VisualDashboardMemo as VisualDashboard }
