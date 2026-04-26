/**
 * Component Preview
 *
 * Shows a live preview of a dashboard component with real-time data.
 * Used in the configuration dialog to visualize changes as they are made.
 * Auto-scales content proportionally to fit within container.
 */

import { memo, useRef, useEffect, useState, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { Skeleton } from '@/components/ui/skeleton'
import { Eye, AlertCircle, Loader2 } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useDataSource } from '@/hooks/useDataSource'
import ComponentRenderer from '@/components/dashboard/registry/ComponentRenderer'
import { getComponentMeta } from '@/components/dashboard/registry/registry'
import type { DashboardComponent, DataSource, ImplementedComponentType } from '@/types/dashboard'
import { getSourceId } from '@/types/dashboard'

// Helper function to create stable key for comparison
function createStableKey(obj: any): string {
  if (obj === null || obj === undefined) return ''
  if (typeof obj !== 'object') return String(obj)
  if (Array.isArray(obj)) return '[' + obj.map(createStableKey).join(',') + ']'
  const sortedKeys = Object.keys(obj).sort()
  return '{' + sortedKeys.map(k => `"${k}":${createStableKey(obj[k])}`).join(',') + '}'
}

/**
 * Create a simple key to detect dataSource changes
 */
function createDataSourceKey(ds: DataSource | undefined): string {
  if (!ds) return 'no-ds'
  return `${ds.type}:${getSourceId(ds) || ''}:${ds.metricId || ds.property || ds.infoProperty || ''}:${ds.command || ''}`
}

export interface ComponentPreviewProps {
  componentType: string
  config: Record<string, unknown>
  dataSource?: DataSource
  title?: string
  showHeader?: boolean
  className?: string
  /** Maximum height for the preview content area (default: 200) */
  maxContentHeight?: number
}

// Grid dimensions (matching dashboard grid)
const GRID_CELL_WIDTH = 100
const GRID_CELL_HEIGHT = 80

export const ComponentPreview = memo(function ComponentPreview({
  componentType,
  config,
  dataSource,
  title,
  showHeader = true,
  className,
  maxContentHeight = 200,
}: ComponentPreviewProps) {
  const { t } = useTranslation('dashboardComponents')
  const meta = getComponentMeta(componentType as ImplementedComponentType)

  // Track data source changes to show transition
  const [prevDataSourceKey, setPrevDataSourceKey] = useState<string>(() => createDataSourceKey(dataSource))
  const [isTransitioning, setIsTransitioning] = useState(false)

  // Track config changes for components with static content
  const [prevConfigKey, setPrevConfigKey] = useState<string>(() => createStableKey(config))

  // Use ref to track the active timer for cleanup
  const transitionTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Refs for auto-scaling
  const containerRef = useRef<HTMLDivElement>(null)
  const [scale, setScale] = useState(1)

  // Detect dataSource changes
  useEffect(() => {
    const newKey = createDataSourceKey(dataSource)
    if (newKey !== prevDataSourceKey) {
      if (transitionTimerRef.current) {
        clearTimeout(transitionTimerRef.current)
      }

      setIsTransitioning(true)

      transitionTimerRef.current = setTimeout(() => {
        setIsTransitioning(false)
        transitionTimerRef.current = null
      }, 200)

      setPrevDataSourceKey(newKey)
    }
  }, [dataSource, prevDataSourceKey])

  // Detect config changes
  useEffect(() => {
    const newKey = createStableKey(config)
    if (newKey !== prevConfigKey) {
      if (transitionTimerRef.current) {
        clearTimeout(transitionTimerRef.current)
      }

      setIsTransitioning(true)

      transitionTimerRef.current = setTimeout(() => {
        setIsTransitioning(false)
        transitionTimerRef.current = null
      }, 150)

      setPrevConfigKey(newKey)
    }
  }, [config, prevConfigKey])

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (transitionTimerRef.current) {
        clearTimeout(transitionTimerRef.current)
      }
    }
  }, [])

  // Track previous data to show during loading (prevents flicker)
  const prevDataRef = useRef<any>(null)
  const hasLoadedOnceRef = useRef(false)

  // Try to fetch real data for preview
  const { data, loading, error } = useDataSource(dataSource, {
    enabled: !!dataSource && meta?.hasDataSource,
  })

  // Update prevDataRef when we successfully get data
  useEffect(() => {
    if (!loading && !error) {
      if (data !== null && data !== undefined) {
        prevDataRef.current = data
      }
      hasLoadedOnceRef.current = true
    }
  }, [data, loading, error])

  // Use component's default size from registry
  const defaultW = meta?.sizeConstraints.defaultW ?? 4
  const defaultH = meta?.sizeConstraints.defaultH ?? 3

  // Build a mock component for rendering with actual default size
  const componentDisplayTitle = title || (config.label as string) || (config.title as string) || ''

  const mockComponent: DashboardComponent = {
    id: 'preview',
    type: componentType as ImplementedComponentType,
    position: { x: 0, y: 0, w: defaultW, h: defaultH },
    title: componentDisplayTitle,
    config: {
      ...config,
      editMode: true,
    },
    dataSource,
  }

  // Calculate ideal component dimensions based on grid size
  const idealWidth = defaultW * GRID_CELL_WIDTH
  const idealHeight = defaultH * GRID_CELL_HEIGHT

  // Calculate scale to fit content within container proportionally
  const updateScale = useCallback(() => {
    if (!containerRef.current) return

    const containerWidth = containerRef.current.clientWidth - 16 // Account for padding
    const containerHeight = containerRef.current.clientHeight - 16

    if (containerWidth <= 0 || containerHeight <= 0) return

    // Calculate scale factors for both dimensions
    const scaleX = containerWidth / idealWidth
    const scaleY = containerHeight / idealHeight

    // Use the smaller scale to ensure content fits both dimensions (maintains aspect ratio)
    const newScale = Math.min(scaleX, scaleY, 1) // Don't scale up beyond 100%

    setScale(newScale)
  }, [idealWidth, idealHeight])

  // Update scale on mount and when container/props change
  useEffect(() => {
    // Initial scale calculation with slight delay for layout
    const timer = requestAnimationFrame(() => {
      updateScale()
    })

    // Update on resize
    const resizeObserver = new ResizeObserver(() => {
      updateScale()
    })

    if (containerRef.current) {
      resizeObserver.observe(containerRef.current)
    }

    return () => {
      cancelAnimationFrame(timer)
      resizeObserver.disconnect()
    }
  }, [updateScale])

  // Show previous data during loading (except on first load)
  const showLoading = loading && !hasLoadedOnceRef.current
  const hasError = !!error

  return (
    <div className={cn('flex flex-col overflow-hidden', className)}>
      {/* Header */}
      {showHeader && (
        <div className="flex items-center justify-between px-3 py-2 border-b bg-muted/30 shrink-0">
          <div className="flex items-center gap-2">
            <Eye className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">{t('componentPreview.title')}</span>
            {(loading || isTransitioning) && (
              <Loader2 className="h-4 w-4 animate-spin text-muted-foreground/50" />
            )}
          </div>
        </div>
      )}

      {/* Preview area - fixed height with proportional scaling */}
      <div
        ref={containerRef}
        className={cn(
          'min-h-0 p-2 bg-muted/10 overflow-hidden relative',
          'transition-opacity duration-200',
          isTransitioning && 'opacity-60'
        )}
        style={{ height: `${maxContentHeight}px` }}
      >
        {showLoading ? (
          <div className="w-full h-full flex items-center justify-center">
            <Skeleton className="w-full h-full" />
          </div>
        ) : hasError ? (
          <div className="w-full h-full flex flex-col items-center justify-center text-muted-foreground p-4 text-center">
            <AlertCircle className="h-8 w-8 text-destructive/60 mb-2" />
            <p className="text-sm">{t('componentPreview.loadingFailed')}</p>
            <p className="text-xs text-muted-foreground/60 mt-1">{t('componentPreview.usingStaticData')}</p>
          </div>
        ) : (
          <div className="w-full h-full flex items-center justify-center overflow-hidden">
            {/* Scaled component container */}
            <div
              className={cn(
                'transition-transform duration-200 ease-out origin-center',
                isTransitioning && 'blur-[1px]'
              )}
              style={{
                width: `${idealWidth}px`,
                height: `${idealHeight}px`,
                transform: `scale(${scale})`,
              }}
            >
              <ComponentRenderer
                key={`preview-${componentType}-${(config.backgroundType as string) || 'default'}`}
                component={mockComponent}
              />
            </div>
          </div>
        )}
      </div>

      {/* Footer with component info */}
      <div className="px-3 py-1.5 border-t bg-muted/20 shrink-0">
        <div className="flex items-center justify-between text-xs text-muted-foreground">
          <span>{meta?.name || componentType}</span>
          <div className="flex items-center gap-2">
            {scale < 0.95 && (
              <span className="text-muted-foreground/50 tabular-nums">
                {Math.round(scale * 100)}%
              </span>
            )}
            <span className="text-muted-foreground/60">
              {defaultW}×{defaultH}
            </span>
          </div>
        </div>
      </div>
    </div>
  )
}, (prevProps, nextProps) => {
  const prevConfigKey = createStableKey(prevProps.config)
  const nextConfigKey = createStableKey(nextProps.config)
  const prevDsKey = createDataSourceKey(prevProps.dataSource)
  const nextDsKey = createDataSourceKey(nextProps.dataSource)

  return (
    prevProps.componentType === nextProps.componentType &&
    prevProps.title === nextProps.title &&
    prevConfigKey === nextConfigKey &&
    prevDsKey === nextDsKey &&
    prevProps.maxContentHeight === nextProps.maxContentHeight
  )
})
