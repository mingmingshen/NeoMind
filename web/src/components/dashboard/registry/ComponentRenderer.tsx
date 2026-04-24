/**
 * Component Renderer
 *
 * Dynamic component rendering based on registry.
 * Uses component metadata to determine how to render each component type.
 * Supports both static (built-in) and dynamic (extension-provided) components.
 */

import { lazy, Suspense, memo, useMemo, useState, useEffect, useCallback } from 'react'
import { Card } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import type { DashboardComponent, GenericComponentType } from '@/types/dashboard'
import { getComponentMeta } from './registry'
import { dynamicRegistry, dtoToComponentMeta } from './DynamicRegistry'

// ============================================================================
// Lazy Import Components
// ============================================================================

// Indicators
const ValueCard = lazy(() => import('../generic/ValueCard').then(m => ({ default: m.ValueCard })))
const LEDIndicator = lazy(() => import('../generic/LEDIndicator').then(m => ({ default: m.LEDIndicator })))
const Sparkline = lazy(() => import('../generic/Sparkline').then(m => ({ default: m.Sparkline })))
const ProgressBar = lazy(() => import('../generic/ProgressBar').then(m => ({ default: m.ProgressBar })))
const AgentStatusCard = lazy(() => import('../generic/AgentStatusCard').then(m => ({ default: m.AgentStatusCard })))

// Charts
const LineChart = lazy(() => import('../generic/LineChart').then(m => ({ default: m.LineChart })))
const AreaChart = lazy(() => import('../generic/LineChart').then(m => ({ default: m.AreaChart })))
const BarChart = lazy(() => import('../generic/BarChart').then(m => ({ default: m.BarChart })))
const PieChart = lazy(() => import('../generic/PieChart').then(m => ({ default: m.PieChart })))

// Controls
const ToggleSwitch = lazy(() => import('../generic/ToggleSwitch').then(m => ({ default: m.ToggleSwitch })))

// Display & Content
const ImageDisplay = lazy(() => import('../generic/ImageDisplay').then(m => ({ default: m.ImageDisplay })))
const ImageHistory = lazy(() => import('../generic/ImageHistory').then(m => ({ default: m.ImageHistory })))
const WebDisplay = lazy(() => import('../generic/WebDisplay').then(m => ({ default: m.WebDisplay })))
const MarkdownDisplay = lazy(() => import('../generic/MarkdownDisplay').then(m => ({ default: m.MarkdownDisplay })))

// Spatial & Media
const MapDisplay = lazy(() => import('../generic/MapDisplay').then(m => ({ default: m.MapDisplay })))
const VideoDisplay = lazy(() => import('../generic/VideoDisplay').then(m => ({ default: m.VideoDisplay })))
const CustomLayer = lazy(() => import('../generic/CustomLayer').then(m => ({ default: m.CustomLayer })))

// ============================================================================
// Component Map
// ============================================================================

const componentMap: Record<GenericComponentType, React.ComponentType<any>> = {
  // Indicators
  'value-card': ValueCard,
  'led-indicator': LEDIndicator,
  'sparkline': Sparkline,
  'progress-bar': ProgressBar,
  'agent-status-card': AgentStatusCard,

  // Charts
  'line-chart': LineChart,
  'area-chart': AreaChart,
  'bar-chart': BarChart,
  'pie-chart': PieChart,

  // Controls
  'toggle-switch': ToggleSwitch,

  // Display & Content
  'image-display': ImageDisplay,
  'image-history': ImageHistory,
  'web-display': WebDisplay,
  'markdown-display': MarkdownDisplay,

  // Spatial & Media
  'map-display': MapDisplay,
  'video-display': VideoDisplay,
  'custom-layer': CustomLayer,
} as const

// ============================================================================
// Business Components Map
// ============================================================================

const AgentMonitorWidget = lazy(() => import('../generic/AgentMonitorWidget').then(m => ({ default: m.AgentMonitorWidget })))
const VlmVision = lazy(() => import('../generic/VlmVision').then(m => ({ default: m.VlmVision })))

const businessComponentMap: Record<string, React.ComponentType<any>> = {
  'agent-monitor-widget': AgentMonitorWidget,
  'vlm-vision': VlmVision,
} as const

// ============================================================================
// Loading Skeleton
// ============================================================================

interface ComponentSkeletonProps {
  meta: ReturnType<typeof getComponentMeta>
  className?: string
}

function ComponentSkeleton({ meta, className }: ComponentSkeletonProps) {
  if (!meta) {
    return <Skeleton className={cn('w-full h-48', className)} />
  }

  const { sizeConstraints } = meta
  const minHeight = sizeConstraints.minH * 40 // Approximate grid row height

  return (
    <Skeleton
      className={cn('w-full', className)}
      style={{ minHeight: `${minHeight}px` }}
    />
  )
}

// ============================================================================
// Fallback for Unknown Component
// ============================================================================

interface UnknownComponentProps {
  type: string
  className?: string
}

function UnknownComponent({ type, className }: UnknownComponentProps) {
  return (
    <Card className={cn('border-dashed border-2', className)}>
      <div className="flex items-center justify-center h-full min-h-[120px] p-4 text-center">
        <div className="text-muted-foreground">
          <p className="font-medium">Unknown Component</p>
          <p className="text-sm text-muted-foreground/60 mt-1">
            Type: <code className="text-xs bg-muted px-1 py-0.5 rounded">{type}</code>
          </p>
        </div>
      </div>
    </Card>
  )
}

// ============================================================================
// Main Renderer
// ============================================================================

/**
 * Device metric type for extension components
 */
export interface DeviceMetric {
  id: string
  name: string
  type?: string
  data_type?: string
  value?: unknown
  timestamp?: number
}

/**
 * Device type for extension components
 */
export interface DeviceForExtension {
  id: string
  name: string
  type?: string
  metrics?: DeviceMetric[]
}

export interface RenderComponentProps {
  component: DashboardComponent
  className?: string
  style?: React.CSSProperties
  onError?: (error: Error) => void
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  onDataSourceChange?: (dataSource: any) => void
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  onConfigChange?: (config: any) => void
}

/**
 * Render a dashboard component based on its type
 * Uses the component registry to determine the appropriate component to render
 * Supports both static (built-in) and dynamic (extension-provided) components
 *
 * Memoized to prevent unnecessary re-renders when parent updates.
 * Only re-renders when component.id, component.type, component.dataSource,
 * component.config, component.title, component.display, className, or style changes.
 */
const ComponentRenderer = memo(function ComponentRenderer({
  component,
  className,
  style,
  onError,
  onDataSourceChange,
  onConfigChange,
}: RenderComponentProps) {
  const componentType = component.type
  const isDynamic = dynamicRegistry.isDynamic(componentType)

  // State for dynamic component loading
  const [DynamicComponent, setDynamicComponent] = useState<React.ComponentType<any> | null>(null)
  const [loading, setLoading] = useState(false)
  const [loadError, setLoadError] = useState<Error | null>(null)
  const [attemptCount, setAttemptCount] = useState(0)
  const [registrationPollCount, setRegistrationPollCount] = useState(0)

  // Heuristic: check if this looks like an extension component (not in static registry)
  const isUnknownType = !componentMap[componentType as GenericComponentType] &&
                        !businessComponentMap[componentType]
  const mightBeExtension = isUnknownType && !isDynamic

  // Max auto-retry attempts
  const MAX_LOAD_RETRIES = 5
  const LOAD_RETRY_DELAY = 800
  const MAX_REGISTRATION_POLLS = 20
  const REGISTRATION_POLL_INTERVAL = 200

  // Load dynamic component with auto-retry
  const loadDynamicComponent = useCallback(async (attempt: number): Promise<void> => {
    if (!isDynamic) {
      setDynamicComponent(null)
      setLoadError(null)
      return
    }

    setLoading(true)
    setLoadError(null)

    try {
      const module = await dynamicRegistry.loadComponent(componentType)

      if (module) {
        let Component: React.ComponentType<any> | null = null

        if (typeof module === 'function') {
          Component = module as React.ComponentType
        } else if (typeof module === 'object' && module !== null) {
          if ((module as any).$$typeof || typeof (module as any).render === 'function') {
            Component = module as React.ComponentType
          } else if ('default' in module) {
            const defaultExport = (module as { default: unknown }).default
            if (typeof defaultExport === 'function') {
              Component = defaultExport as React.ComponentType
            } else if (typeof defaultExport === 'object' && defaultExport !== null) {
              if ((defaultExport as any).$$typeof || typeof (defaultExport as any).render === 'function') {
                Component = defaultExport as React.ComponentType
              }
            }
          }
        }

        if (Component) {
          setDynamicComponent(() => Component as React.ComponentType)
          setLoadError(null)
        } else {
          throw new Error(`Invalid component type`)
        }
      } else {
        throw new Error(`Module not found`)
      }
    } catch (err) {
      console.error(`[ComponentRenderer] Load attempt ${attempt} failed for ${componentType}:`, err)

      // Auto-retry if we haven't exceeded max attempts
      if (attempt < MAX_LOAD_RETRIES) {
        console.log(`[ComponentRenderer] Scheduling retry ${attempt + 1}/${MAX_LOAD_RETRIES} in ${LOAD_RETRY_DELAY}ms`)
        setTimeout(() => {
          setAttemptCount(attempt + 1)
        }, LOAD_RETRY_DELAY)
      } else {
        setLoadError(err instanceof Error ? err : new Error(String(err)))
      }
    } finally {
      setLoading(false)
    }
  }, [componentType, isDynamic])

  // Trigger load when isDynamic changes or retry is needed
  useEffect(() => {
    if (isDynamic) {
      loadDynamicComponent(attemptCount)
    }
  }, [isDynamic, attemptCount, loadDynamicComponent])

  // Reset attempt count when component type changes
  useEffect(() => {
    setAttemptCount(0)
    setDynamicComponent(null)
    setLoadError(null)
    setRegistrationPollCount(0)
  }, [componentType])

  // Wait for extension components to be registered (polling)
  useEffect(() => {
    if (!mightBeExtension) return

    // Check immediately
    if (dynamicRegistry.isDynamic(componentType)) {
      return
    }

    // Poll for component registration
    const pollInterval = setInterval(() => {
      setRegistrationPollCount(prev => {
        const next = prev + 1
        if (next >= MAX_REGISTRATION_POLLS) {
          clearInterval(pollInterval)
        }
        return next
      })
    }, REGISTRATION_POLL_INTERVAL)

    return () => clearInterval(pollInterval)
  }, [mightBeExtension, componentType])

  // Re-check isDynamic when poll count changes
  const currentIsDynamic = dynamicRegistry.isDynamic(componentType)

  // Trigger load when component becomes registered
  useEffect(() => {
    if (currentIsDynamic && !isDynamic && !DynamicComponent && !loading && !loadError) {
      // Component just became registered, trigger load
      setAttemptCount(0)
    }
  }, [currentIsDynamic, isDynamic, DynamicComponent, loading, loadError])

  // Get metadata (check both static and dynamic registries)
  let meta = getComponentMeta(componentType)

  // If not found in static registry, try dynamic registry
  if (!meta && isDynamic) {
    const dto = dynamicRegistry.getMeta(componentType)
    if (dto) {
      meta = dtoToComponentMeta(dto)
    }
  }

  // Try to get component from generic or business component map (static components)
  const StaticComponent = componentMap[component.type as GenericComponentType] || businessComponentMap[component.type]

  // Determine which component to render
  const Component = isDynamic ? DynamicComponent : StaticComponent

  // Extract specific values for stable dependencies
  // Use individual properties instead of entire component object to prevent unnecessary re-creates
  const componentId = component.id
  const componentTitle = component.title
  const componentConfig = (component as any).config || {}
  const componentDataSource = (component as any).dataSource
  const componentDisplay = (component as any).display

  // Memoize props to prevent unnecessary re-renders of child components
  // Only recreate when actual component data changes
  // IMPORTANT: Must be before any early returns to follow React Hooks rules
  const props = useMemo(() => {
    const { editMode, ...restConfig } = componentConfig

    // Build props for the component (NOT including key - key must be passed directly)
    const builtProps: Record<string, any> = {
      dataSource: componentDataSource,
      editMode, // Pass editMode as a separate prop for components that need it
      ...restConfig,
      ...componentDisplay,
      title: componentTitle || componentConfig.title,
      className: cn(
        'w-full h-full',
        className
      ),
      style,
      // Pass callbacks for components to persist their configuration
      onDataSourceChange,
      onConfigChange,
    }

    // Special handling for agent-monitor-widget: extract agentId from dataSource
    if (componentType === 'agent-monitor-widget' && componentDataSource?.agentId) {
      builtProps.agentId = componentDataSource.agentId
    }

    // vlm-vision: agentId comes from config (restConfig), no special handling needed

    return builtProps
  }, [componentId, componentType, componentTitle, componentConfig, componentDataSource, componentDisplay, className, style, onDataSourceChange, onConfigChange])

  // Show loading state for dynamic components
  if (isDynamic && loading) {
    return <ComponentSkeleton meta={meta} className={className} />
  }

  // Show loading state for components that might be extension components but not yet registered
  if (mightBeExtension && registrationPollCount < MAX_REGISTRATION_POLLS) {
    return <ComponentSkeleton meta={undefined} className={className} />
  }

  // Show error state for dynamic component load failures (only after all retries exhausted)
  if (isDynamic && loadError && attemptCount >= MAX_LOAD_RETRIES) {
    return (
      <Card className={cn('border-destructive/50', className)}>
        <div className="flex items-center justify-center h-full min-h-[120px] p-4 text-center">
          <div className="text-destructive">
            <p className="font-medium">Component Load Failed</p>
            <p className="text-sm text-muted-foreground mt-1">
              {loadError.message}
            </p>
            <button
              onClick={() => {
                setLoadError(null)
                setAttemptCount(0)
              }}
              className="mt-2 px-3 py-1 text-xs bg-destructive/10 rounded hover:bg-destructive/20 transition-colors"
            >
              Retry
            </button>
          </div>
        </div>
      </Card>
    )
  }

  // Show loading while retrying
  if (isDynamic && loadError && attemptCount < MAX_LOAD_RETRIES) {
    return <ComponentSkeleton meta={meta} className={className} />
  }

  // Handle unknown component types (after hooks to follow React Hooks rules)
  if (!meta || !Component) {
    return <UnknownComponent type={component.type} className={className} />
  }

  return (
    <Suspense fallback={<ComponentSkeleton meta={meta} className={className} />}>
      <Component key={component.id} {...props} />
    </Suspense>
  )
}, (prevProps, nextProps) => {
  // Custom comparison for more precise re-render control
  const prevComp = prevProps.component as any
  const nextComp = nextProps.component as any

  // Quick primitive checks
  if (prevProps.component.id !== nextProps.component.id) return false
  if (prevProps.component.type !== nextProps.component.type) return false
  if (prevProps.component.title !== nextProps.component.title) return false
  if (prevProps.className !== nextProps.className) return false
  if (prevProps.style !== nextProps.style) return false

  // Helper for stable deep comparison of objects
  const deepEqual = (a: unknown, b: unknown): boolean => {
    // Reference equality
    if (a === b) return true
    // Both null/undefined
    if (a == null || b == null) return a === b
    // Type mismatch
    if (typeof a !== typeof b) return false
    // One is array, other isn't
    if (Array.isArray(a) !== Array.isArray(b)) return false

    // For arrays and objects, use JSON.stringify with sorted keys
    // This handles property order differences
    try {
      return JSON.stringify(a) === JSON.stringify(b)
    } catch {
      // Fallback for circular references or non-serializable values
      return false
    }
  }

  // Deep compare complex objects
  if (!deepEqual(prevComp.dataSource, nextComp.dataSource)) return false
  if (!deepEqual(prevComp.config, nextComp.config)) return false
  if (!deepEqual(prevComp.display, nextComp.display)) return false

  // All checks passed - props are equal
  return true
})

export default ComponentRenderer

/**
 * Render multiple components
 */
export interface RenderComponentsProps {
  components: DashboardComponent[]
  className?: string
  onComponentError?: (componentId: string, error: Error) => void
}

export function RenderComponents({
  components,
  className,
  onComponentError,
}: RenderComponentsProps) {
  return (
    <>
      {components.map(component => (
        <ComponentRenderer
          key={component.id}
          component={component}
          className={className}
          onError={(error) => onComponentError?.(component.id, error)}
        />
      ))}
    </>
  )
}
