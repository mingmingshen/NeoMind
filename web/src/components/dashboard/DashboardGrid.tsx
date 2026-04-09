/**
 * Dashboard Grid Component
 *
 * Provides a drag-and-drop grid layout using react-grid-layout v2.
 *
 * Layout Strategy:
 * - `layouts` prop is ONLY updated when components are added/removed
 * - react-grid-layout manages ALL positions internally (compact, drag, resize)
 * - `onLayoutChange` only updates a ref — never triggers React state/props
 * - Positions synced to parent ONLY on drag/resize stop
 * - This completely avoids the controlled-mode feedback loop
 *   (react-grid-layout#1984: onLayoutChange fires twice)
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
    children: ReactElement
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

  // react-grid-layout's latest internal positions (ref only, never triggers re-render)
  const latestLayoutRef = useRef<Record<string, { x: number; y: number; w: number; h: number }>>({})

  // Stable component ID string — only changes when components are added/removed
  const componentIdKey = useMemo(() => components.map(c => c.id).join(','), [components])

  // Snapshot of component data (position, constraints) captured when components change.
  // This ensures the layouts memo always has correct data even though `components`
  // is not in its dependency array (preventing jitter from reference changes).
  const componentsSnapshotRef = useRef(components)
  useEffect(() => {
    componentsSnapshotRef.current = components
  }, [components])

  // "Settle" mechanism: after new components are added, react-grid-layout compacts
  // them (moving y:9999 to correct position). We need ONE layouts recalculation
  // after compact to bake in the correct positions, then never again.
  const needsSettleRef = useRef(false)
  const [settleVersion, setSettleVersion] = useState(0)

  // Build layouts using latestLayoutRef for settled positions.
  // For new components not yet in ref, use parent's position (e.g., y:9999).
  const layouts = useMemo(() => {
    const snapshot = componentsSnapshotRef.current
    const layout = snapshot.map((c) => {
      const current = latestLayoutRef.current[c.id]
      const pos = current || c.position
      return {
        i: c.id,
        x: pos.x, y: pos.y, w: pos.w, h: pos.h,
        minW: c.position.minW ?? 1,
        minH: c.position.minH ?? 1,
        maxW: c.position.maxW,
        maxH: c.position.maxH,
        static: false,
      }
    })
    return { lg: layout, md: layout, sm: layout, xs: layout }
  }, [componentIdKey, settleVersion])

  // Detect new components → mark for settling
  const prevComponentIdKeyRef = useRef(componentIdKey)
  useEffect(() => {
    if (componentIdKey !== prevComponentIdKeyRef.current) {
      prevComponentIdKeyRef.current = componentIdKey
      needsSettleRef.current = true
    }
  }, [componentIdKey])

  // Handle layout changes from react-grid-layout.
  const handleLayoutChange = useCallback((currentLayout: any) => {
    const newPositions: Record<string, { x: number; y: number; w: number; h: number }> = {}
    currentLayout.forEach((item: any) => {
      newPositions[item.i] = { x: item.x, y: item.y, w: item.w, h: item.h }
    })
    latestLayoutRef.current = newPositions

    // After new component compact, do ONE settle bump so layouts picks up
    // the corrected positions (instead of stale y:9999). After this, no more bumps.
    if (needsSettleRef.current) {
      needsSettleRef.current = false
      setSettleVersion(v => v + 1)
    }
  }, [])

  const isDraggingRef = useRef(false)

  const handleDragStart = useCallback(() => {
    isDraggingRef.current = true
  }, [])

  const handleDragStop = useCallback((layout: any) => {
    isDraggingRef.current = false
    // Update ref with final positions
    const positions: Record<string, { x: number; y: number; w: number; h: number }> = {}
    layout.forEach((item: any) => {
      positions[item.i] = { x: item.x, y: item.y, w: item.w, h: item.h }
    })
    latestLayoutRef.current = positions
    // Sync to parent
    if (onLayoutChange && editMode) {
      onLayoutChange(layout as readonly any[])
    }
  }, [onLayoutChange, editMode])

  const handleResizeStart = useCallback(() => {
    isDraggingRef.current = true
  }, [])

  const handleResizeStop = useCallback((layout: any) => {
    isDraggingRef.current = false
    const positions: Record<string, { x: number; y: number; w: number; h: number }> = {}
    layout.forEach((item: any) => {
      positions[item.i] = { x: item.x, y: item.y, w: item.w, h: item.h }
    })
    latestLayoutRef.current = positions
    if (onLayoutChange && editMode) {
      onLayoutChange(layout as readonly any[])
    }
  }, [onLayoutChange, editMode])

  // Debounced container width
  const updateWidth = useCallback(() => {
    if (containerRef.current) {
      setWidth(containerRef.current.offsetWidth)
    }
  }, [])

  const resizeTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    updateWidth()
    const resizeObserver = new ResizeObserver(() => {
      if (resizeTimeoutRef.current) clearTimeout(resizeTimeoutRef.current)
      resizeTimeoutRef.current = setTimeout(() => requestAnimationFrame(updateWidth), 100)
    })
    if (containerRef.current) resizeObserver.observe(containerRef.current)
    const handleWindowResize = () => {
      if (resizeTimeoutRef.current) clearTimeout(resizeTimeoutRef.current)
      resizeTimeoutRef.current = setTimeout(() => requestAnimationFrame(updateWidth), 100)
    }
    window.addEventListener('resize', handleWindowResize)
    return () => {
      resizeObserver.disconnect()
      window.removeEventListener('resize', handleWindowResize)
      if (resizeTimeoutRef.current) clearTimeout(resizeTimeoutRef.current)
    }
  }, [updateWidth])

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
          ${transitionsEnabled ? 'transition: transform 200ms cubic-bezier(0.4, 0, 0.2, 1), width 200ms cubic-bezier(0.4, 0, 0.2, 1), height 200ms cubic-bezier(0.4, 0, 0.2, 1);' : 'transition: none;'}
        }
      `}</style>
      {width > 0 && (
        <ResponsiveGridLayout
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
          onLayoutChange={handleLayoutChange}
          onDragStart={handleDragStart}
          onDragStop={handleDragStop}
          onResizeStart={handleResizeStart}
          onResizeStop={handleResizeStop}
        >
          {components.map((component) => (
            <div key={component.id} className="dashboard-item h-full">
              {component.children}
            </div>
          ))}
        </ResponsiveGridLayout>
      )}
      {editMode && <div className="h-48" />}
    </div>
  )
}
