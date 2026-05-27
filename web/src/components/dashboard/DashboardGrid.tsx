/**
 * Dashboard Grid Component
 *
 * Provides a drag-and-drop grid layout using react-grid-layout v2.
 *
 * Layout Strategy:
 * - `layouts` always reflects component positions from props
 * - react-grid-layout is re-driven via `layouts` whenever container width changes
 * - This ensures sidebar toggle / resize always produces correct layout
 * - Positions synced to parent ONLY on drag/resize stop
 */

import { useRef, useState, useEffect, useCallback, useMemo } from 'react'
import { ReactElement } from 'react'
import { ResponsiveGridLayout } from 'react-grid-layout'
import { cn } from '@/lib/utils'
import { responsiveCols } from '@/design-system/tokens/size'
import { useIsMobile } from '@/hooks/useMobile'

import 'react-grid-layout/css/styles.css'
import 'react-resizable/css/styles.css'

const isTouchDevice = () => {
  if (typeof window === 'undefined') return false
  return 'ontouchstart' in window || navigator.maxTouchPoints > 0
}

export interface DashboardGridProps {
  components: Array<{
    id: string
    position: { x: number; y: number; w: number; h: number; minW?: number; minH?: number; maxW?: number; maxH?: number }
    children?: ReactElement
    render?: () => ReactElement
  }>
  rowHeight?: number
  margin?: [number, number]
  containerPadding?: [number, number]
  breakpoints?: Record<string, number>
  cols?: Record<string, number>
  editMode?: boolean
  onLayoutChange?: (layout: readonly any[]) => void
  className?: string
}

export function DashboardGrid({
  components,
  rowHeight = 60,
  margin = [4, 4],
  containerPadding = [4, 4],
  breakpoints = { lg: 1200, md: 996, sm: 768, xs: 480 },
  cols = responsiveCols,
  editMode = false,
  onLayoutChange,
  className,
}: DashboardGridProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [width, setWidth] = useState(0)
  const isMobile = useIsMobile()
  const touchEnabled = isTouchDevice()

  // Component ID string — changes when dashboard switches (different widgets)
  const componentIdKey = useMemo(() => components.map(c => c.id).join(','), [components])

  // Track container width via ResizeObserver
  const widthRef = useRef(0)
  const updateWidth = useCallback(() => {
    if (containerRef.current) {
      const w = containerRef.current.offsetWidth
      widthRef.current = w
      setWidth(w)
    }
  }, [])

  // Re-observe when componentIdKey changes (dashboard switch) so width is
  // re-measured immediately, preventing the "grid disappears" bug caused by
  // stale width=0 after ResponsiveGridLayout remounts via key={componentIdKey}.
  useEffect(() => {
    updateWidth()
    const resizeObserver = new ResizeObserver(() => {
      requestAnimationFrame(updateWidth)
    })
    if (containerRef.current) resizeObserver.observe(containerRef.current)
    return () => {
      resizeObserver.disconnect()
    }
  }, [updateWidth, componentIdKey])

  // Build layouts — include `width` in deps so the object reference changes
  // whenever the container resizes. This forces react-grid-layout to fully
  // recalculate positions from the original component.position values, preventing
  // stale compacted positions from sidebar toggle or window resize.
  const layouts = useMemo(() => {
    const layout = components.map((c) => ({
      i: c.id,
      x: c.position.x ?? 0,
      y: c.position.y ?? 0,
      w: c.position.w ?? 4,
      h: c.position.h ?? 3,
      minW: c.position.minW ?? 1,
      minH: c.position.minH ?? 1,
      maxW: c.position.maxW,
      maxH: c.position.maxH,
      static: false,
    }))
    return { lg: layout, md: layout, sm: layout, xs: layout }
    // width in deps: forces react-grid-layout to recalculate on container resize
  }, [componentIdKey, width, components])

  // When dashboard switches (componentIdKey changes), synchronously re-measure
  // container width to avoid the grid being hidden by the `width > 0` guard.
  // ResizeObserver fires asynchronously which can leave width=0 for one render.
  useEffect(() => {
    if (containerRef.current) {
      const w = containerRef.current.offsetWidth
      if (w > 0) {
        widthRef.current = w
        setWidth(w)
      }
    }
  }, [componentIdKey])

  // Drag/resize handlers — only persist to parent, don't feed back into layouts
  const handleDragStop = useCallback((layout: any) => {
    if (onLayoutChange && editMode) {
      onLayoutChange(layout as readonly any[])
    }
  }, [onLayoutChange, editMode])

  const handleResizeStop = useCallback((layout: any) => {
    if (onLayoutChange && editMode) {
      onLayoutChange(layout as readonly any[])
    }
  }, [onLayoutChange, editMode])

  // Transitions
  const [transitionsEnabled, setTransitionsEnabled] = useState(false)
  useEffect(() => {
    if (width > 0 && !transitionsEnabled) {
      const timer = setTimeout(() => setTransitionsEnabled(true), 300)
      return () => clearTimeout(timer)
    }
  }, [width, transitionsEnabled])

  return (
    <div ref={containerRef} className={cn('w-full', className)}>
      <style>{`
        .react-grid-layout { display: block !important; }
        .react-grid-item {
          ${transitionsEnabled ? 'transition: transform 200ms ease;' : 'transition: none;'}
        }
        ${touchEnabled ? `
        .react-grid-item { touch-action: none; }
        .react-grid-item > * { touch-action: pan-y pinch-zoom; }
        .react-grid-layout.edit-mode .react-grid-item { touch-action: none; }
        .react-grid-layout:not(.edit-mode) .react-grid-item { touch-action: pan-y pan-x pinch-zoom; }
        .react-grid-item button[style*="touch-action: manipulation"] {
          touch-action: manipulation !important;
          pointer-events: auto !important;
        }
        ` : ''}
        .dashboard-item {
          width: 100%; height: 100%;
          display: flex; flex-direction: column; overflow: hidden;
        }
        .dashboard-item > * {
          height: 100%; min-height: 0;
          display: flex; flex-direction: column;
        }
        .react-grid-placeholder {
          background: rgba(148, 163, 184, 0.15) !important;
          border: 1px dashed rgba(148, 163, 184, 0.3) !important;
          border-radius: 0.5rem; margin: 0;
          transition: all 150ms ease;
        }
        .react-grid-layout.edit-mode > .react-grid-item.react-draggable-dragging {
          z-index: 100;
          box-shadow: 0 12px 32px rgba(0, 0, 0, 0.15);
          transition: none; opacity: 0.9;
        }
        .react-grid-layout.edit-mode > .react-grid-item.resizing {
          z-index: 100; transition: none;
        }
        .react-resizable-handle {
          position: absolute;
          width: ${isMobile ? '32px' : '14px'}; height: ${isMobile ? '32px' : '14px'};
          bottom: 0; right: 0; background: transparent; cursor: se-resize; z-index: 10;
        }
        ${isMobile ? `
        .react-resizable-handle { width: 44px; height: 44px; }
        .react-resizable-handle::after {
          width: 20px; height: 20px; right: 8px; bottom: 8px;
          border-right-width: 3px; border-bottom-width: 3px;
        }
        ` : ''}
        .react-resizable-handle::after {
          content: ''; position: absolute; right: 2px; bottom: 2px;
          width: 6px; height: 6px;
          border-right: 2px solid hsl(var(--muted-foreground) / 0.5);
          border-bottom: 2px solid hsl(var(--muted-foreground) / 0.5);
          border-radius: 0 0 1px 0;
        }
        .react-grid-layout.edit-mode > .react-grid-item:hover .react-resizable-handle::after {
          border-right-color: hsl(var(--primary));
          border-bottom-color: hsl(var(--primary));
          width: 8px; height: 8px;
        }
        .react-grid-layout:not(.edit-mode) .react-resizable-handle { display: none; }
        .react-grid-layout:not(.edit-mode) > .react-grid-item {
          transition: none !important;
        }
      `}</style>
      {width > 0 && (
        <ResponsiveGridLayout
          key={componentIdKey}
          className={cn('dashboard-grid', editMode && 'edit-mode')}
          layouts={layouts}
          breakpoints={breakpoints}
          cols={cols}
          width={width}
          rowHeight={rowHeight}
          margin={margin}
          containerPadding={containerPadding}
          maxRows={Infinity}
          dragConfig={{ enabled: editMode, bounded: false }}
          resizeConfig={{ enabled: editMode, handles: ['se'] as const }}
          onDragStop={handleDragStop}
          onResizeStop={handleResizeStop}
        >
          {components.map((component) => (
            <div
              key={component.id}
              className="dashboard-item h-full"
              data-grid={{
                x: component.position.x ?? 0,
                y: component.position.y ?? 0,
                w: component.position.w ?? 4,
                h: component.position.h ?? 3,
                minW: component.position.minW,
                maxW: component.position.maxW,
                minH: component.position.minH,
                maxH: component.position.maxH,
              }}
            >
              {component.children ?? component.render?.()}
            </div>
          ))}
        </ResponsiveGridLayout>
      )}
      {editMode && <div className="h-48" />}
    </div>
  )
}
