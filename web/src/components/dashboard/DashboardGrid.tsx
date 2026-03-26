/**
 * Dashboard Grid Component
 *
 * Provides a drag-and-drop grid layout using react-grid-layout v2.
 * Supports responsive layouts and edit mode with visual feedback.
 * Now with touch support for mobile devices.
 *
 * Layout Tracking Strategy:
 * - During drag/resize: track positions in ref without re-rendering
 * - After drag/resize: update state to sync with parent
 * - External changes: only update if not in middle of drag/resize
 */

import { useRef, useState, useEffect, useCallback, useMemo } from 'react'
import { ReactElement } from 'react'
import { ResponsiveGridLayout } from 'react-grid-layout'
import { cn } from '@/lib/utils'
import { responsiveCols } from '@/design-system/tokens/size'
import { useIsMobile } from '@/hooks/useMobile'

// Import styles
import 'react-grid-layout/css/styles.css'
import 'react-resizable/css/styles.css'

// Check if device supports touch
const isTouchDevice = () => {
  if (typeof window === 'undefined') return false
  return 'ontouchstart' in window || navigator.maxTouchPoints > 0
}

export interface DashboardGridProps {
  // Components to render
  components: Array<{
    id: string
    position: { x: number; y: number; w: number; h: number; minW?: number; minH?: number; maxW?: number; maxH?: number }
    children: ReactElement
  }>

  // Layout configuration
  rowHeight?: number
  margin?: [number, number]
  containerPadding?: [number, number]
  breakpoints?: Record<string, number>
  cols?: Record<string, number>

  // Edit mode
  editMode?: boolean
  onLayoutChange?: (layout: readonly any[]) => void

  // Styling
  className?: string
}

export function DashboardGrid({
  components,
  rowHeight = 60,  // Grid row height in pixels - each h:1 = 60px
  margin = [4, 4],  // Gap between grid cells
  containerPadding = [4, 4],  // Padding around grid
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

  // Track layout positions from parent (props) - this is our "source of truth"
  // We ONLY update this from props, never from drag/resize
  const [parentLayoutPositions, setParentLayoutPositions] = useState<Record<string, { x: number; y: number; w: number; h: number }>>(() => {
    const initial: Record<string, { x: number; y: number; w: number; h: number }> = {}
    components.forEach(c => {
      initial[c.id] = { x: c.position.x, y: c.position.y, w: c.position.w, h: c.position.h }
    })
    return initial
  })

  // Track positions during drag/resize - use ref to avoid re-renders
  const dragLayoutRef = useRef<Record<string, { x: number; y: number; w: number; h: number }>>({})

  // Track if we're in the middle of a drag/resize operation
  const isDraggingRef = useRef(false)

  // Track if the current layout change is from user interaction (drag/resize) vs responsive resize
  const isUserInteractionRef = useRef(false)

  // Track newly added component IDs that need position sync after layout adjustment
  const pendingSyncComponentIdsRef = useRef<Set<string>>(new Set())

  // Sync parent layout positions when components prop changes
  // Use ref to track previous component IDs to detect additions/removals without triggering re-renders
  const prevComponentIdsRef = useRef<Set<string>>(new Set())

  useEffect(() => {
    if (!isDraggingRef.current) {
      const currentIds = new Set(components.map(c => c.id))
      const prevIds = prevComponentIdsRef.current

      // Check if component list actually changed (additions or removals)
      const addedIds = [...currentIds].filter(id => !prevIds.has(id))
      const hasRemovals = [...prevIds].some(id => !currentIds.has(id))

      // Track newly added components for position sync
      if (addedIds.length > 0) {
        addedIds.forEach(id => pendingSyncComponentIdsRef.current.add(id))
      }

      // Only update if there are structural changes or position changes
      if (addedIds.length > 0 || hasRemovals) {
        // Rebuild positions from scratch for additions/removals
        const newPositions: Record<string, { x: number; y: number; w: number; h: number }> = {}
        components.forEach(c => {
          newPositions[c.id] = { x: c.position.x, y: c.position.y, w: c.position.w, h: c.position.h }
        })
        setParentLayoutPositions(newPositions)
        prevComponentIdsRef.current = currentIds
      } else {
        // Only update if positions actually changed (use functional update to avoid dependency)
        setParentLayoutPositions(prev => {
          let hasChanges = false
          const newPositions: Record<string, { x: number; y: number; w: number; h: number }> = {}

          components.forEach(c => {
            const existing = prev[c.id]
            if (!existing || existing.x !== c.position.x || existing.y !== c.position.y ||
                existing.w !== c.position.w || existing.h !== c.position.h) {
              newPositions[c.id] = { x: c.position.x, y: c.position.y, w: c.position.w, h: c.position.h }
              hasChanges = true
            } else {
              newPositions[c.id] = existing
            }
          })

          return hasChanges ? newPositions : prev
        })
      }
    }
  }, [components])

  // Debounced update for container width
  const updateWidth = useCallback(() => {
    if (containerRef.current) {
      setWidth(containerRef.current.offsetWidth)
    }
  }, [])

  // Debounce timeout ref
  const resizeTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Update container width on resize (with debounce to prevent rapid re-renders)
  useEffect(() => {
    updateWidth()

    const resizeObserver = new ResizeObserver(() => {
      // Clear existing timeout
      if (resizeTimeoutRef.current) {
        clearTimeout(resizeTimeoutRef.current)
      }
      // Debounce the width update
      resizeTimeoutRef.current = setTimeout(() => {
        requestAnimationFrame(updateWidth)
      }, 100) // 100ms debounce
    })
    if (containerRef.current) {
      resizeObserver.observe(containerRef.current)
    }

    const handleWindowResize = () => {
      // Clear existing timeout
      if (resizeTimeoutRef.current) {
        clearTimeout(resizeTimeoutRef.current)
      }
      // Debounce the width update
      resizeTimeoutRef.current = setTimeout(() => {
        requestAnimationFrame(updateWidth)
      }, 100) // 100ms debounce
    }
    window.addEventListener('resize', handleWindowResize)

    return () => {
      resizeObserver.disconnect()
      window.removeEventListener('resize', handleWindowResize)
      if (resizeTimeoutRef.current) {
        clearTimeout(resizeTimeoutRef.current)
      }
    }
  }, [updateWidth])

  // Get current layout positions - use drag positions if dragging, otherwise parent positions
  const getCurrentPositions = useCallback(() => {
    if (isDraggingRef.current && Object.keys(dragLayoutRef.current).length > 0) {
      return dragLayoutRef.current
    }
    return parentLayoutPositions
  }, [parentLayoutPositions])

  // Create base layout from current positions
  // We use a state-derived key to force recalculation during drag
  const [dragKey, setDragKey] = useState(0)

  const baseLayout = useMemo(() => {
    const positions = getCurrentPositions()
    return components.map((c) => {
      const pos = positions[c.id] || c.position
      return {
        i: c.id,
        x: pos.x,
        y: pos.y,
        w: pos.w,
        h: pos.h,
        minW: c.position.minW ?? 1,
        minH: c.position.minH ?? 1,
        maxW: c.position.maxW,
        maxH: c.position.maxH,
        static: false,
      }
    })
  }, [components, parentLayoutPositions, dragKey, getCurrentPositions])

  const layout = baseLayout

  // Memoize layouts object to prevent unnecessary re-renders of react-grid-layout
  const layouts = useMemo(() => ({
    lg: layout,
    md: layout,
    sm: layout,
    xs: layout,
  }), [layout])

  // Track if we should enable transitions (disable during initial mount to prevent flicker)
  const [transitionsEnabled, setTransitionsEnabled] = useState(false)

  // Enable transitions after initial mount settles
  useEffect(() => {
    if (width > 0 && !transitionsEnabled) {
      const timer = setTimeout(() => {
        setTransitionsEnabled(true)
      }, 300) // Wait for layout to settle
      return () => clearTimeout(timer)
    }
  }, [width, transitionsEnabled])

  // Handle layout changes from react-grid-layout
  // This includes: drag, resize, collision detection, and compact
  const handleLayoutChange = useCallback((currentLayout: any, allLayouts?: any) => {
    // Update drag ref IMMEDIATELY - this keeps internal positions in sync
    const newPositions: Record<string, { x: number; y: number; w: number; h: number }> = {}
    currentLayout.forEach((item: any) => {
      newPositions[item.i] = { x: item.x, y: item.y, w: item.w, h: item.h }
    })
    dragLayoutRef.current = newPositions

    if (isUserInteractionRef.current) {
      // User is actively dragging/resizing - force re-render for smooth UI
      setDragKey(k => k + 1)
    }

    // Check if any newly added components had their positions adjusted by react-grid-layout
    // This happens when collision detection or compact moves them to a different position
    if (!isDraggingRef.current && editMode && onLayoutChange) {
      const pendingIds = pendingSyncComponentIdsRef.current
      if (pendingIds.size > 0) {
        // Find components whose positions (x/y only) were adjusted
        const adjustedIds: string[] = []

        currentLayout.forEach((item: any) => {
          if (!pendingIds.has(item.i)) return
          const parentPos = parentLayoutPositions[item.i]
          if (!parentPos) {
            adjustedIds.push(item.i)
            return
          }
          // Only check x/y position change, NOT size (w/h)
          if (parentPos.x !== item.x || parentPos.y !== item.y) {
            adjustedIds.push(item.i)
          }
        })

        if (adjustedIds.length > 0) {
          // Build layout to sync - preserve original w/h for newly added components
          const adjustedIdsSet = new Set(adjustedIds)
          const layoutToSync = currentLayout.map((item: any) => {
            if (adjustedIdsSet.has(item.i)) {
              const originalPos = parentLayoutPositions[item.i]
              if (originalPos) {
                // Sync new x/y but preserve original w/h
                return {
                  ...item,
                  w: originalPos.w,
                  h: originalPos.h,
                }
              }
            }
            return item
          })

          // Clear pending sync for these components AFTER building layoutToSync
          adjustedIds.forEach(id => pendingIds.delete(id))

          // Use queueMicrotask to break the synchronous update cycle
          queueMicrotask(() => {
            onLayoutChange(layoutToSync as readonly any[])
          })
        }
      }
    }
  }, [editMode, parentLayoutPositions, onLayoutChange])

  // Track drag start
  const handleDragStart = useCallback(() => {
    isDraggingRef.current = true
    isUserInteractionRef.current = true
  }, [])

  // Track drag end - notify parent with final layout
  const handleDragStop = useCallback((layout: any) => {
    // Notify parent with final layout position
    if (onLayoutChange && editMode) {
      onLayoutChange(layout as readonly any[])
    }
    // Clear drag state - effect will sync from components prop on next render
    isDraggingRef.current = false
    isUserInteractionRef.current = false
    dragLayoutRef.current = {}
  }, [onLayoutChange, editMode])

  // Track resize start
  const handleResizeStart = useCallback(() => {
    isDraggingRef.current = true
    isUserInteractionRef.current = true
  }, [])

  // Track resize end - notify parent with final layout
  const handleResizeStop = useCallback((layout: any) => {
    // Notify parent with final layout position
    if (onLayoutChange && editMode) {
      onLayoutChange(layout as readonly any[])
    }
    // Clear drag state - effect will sync from components prop on next render
    isDraggingRef.current = false
    isUserInteractionRef.current = false
    dragLayoutRef.current = {}
  }, [onLayoutChange, editMode])

  return (
    <div ref={containerRef} className={cn('w-full', className)}>
      <style>{`
        /* Grid layout base */
        .react-grid-layout {
          display: block !important;
        }

        /* Grid items - controlled by react-grid-layout, don't override */
        .react-grid-item {
          /* Only add transition when settled, let react-grid-layout handle position/size */
          ${transitionsEnabled ? 'transition: transform 200ms ease;' : 'transition: none;'}
        }

        /* Touch-friendly touch-action */
        ${touchEnabled ? `
        .react-grid-item {
          touch-action: none;
        }

        .react-grid-item > * {
          touch-action: pan-y pinch-zoom;
        }

        .react-grid-layout.edit-mode .react-grid-item {
          touch-action: none;
        }

        .react-grid-layout:not(.edit-mode) .react-grid-item {
          touch-action: pan-y pan-x pinch-zoom;
        }

        /* Mobile edit button - ensure it's clickable */
        .react-grid-item button[style*="touch-action: manipulation"] {
          touch-action: manipulation !important;
          pointer-events: auto !important;
        }
        ` : ''}

        /* Dashboard item fills the grid cell allocated by react-grid-layout */
        .dashboard-item {
          width: 100%;
          height: 100%;
          display: flex;
          flex-direction: column;
          overflow: hidden;
        }

        /* Drag placeholder - use !important to override library defaults */
        .react-grid-placeholder {
          background: rgba(148, 163, 184, 0.15) !important;
          border: 1px dashed rgba(148, 163, 184, 0.3) !important;
          border-radius: 0.5rem;
          margin: 0;
          transition: all 150ms ease;
        }

        /* Edit mode styles - no additional outline/border, components handle their own styling */
        .react-grid-layout.edit-mode > .react-grid-item {
          /* No outline or border-radius - let components style themselves */
        }

        .react-grid-layout.edit-mode > .react-grid-item:hover {
          /* Hover effect handled by component's shadow transition */
        }

        .react-grid-layout.edit-mode > .react-grid-item.react-draggable-dragging {
          z-index: 100;
          box-shadow: 0 12px 32px rgba(0, 0, 0, 0.15);
          transition: none;
          opacity: 0.9;
        }

        .react-grid-layout.edit-mode > .react-grid-item.resizing {
          z-index: 100;
          transition: none;
        }

        /* Resize handle - visible only in edit mode */
        .react-resizable-handle {
          position: absolute;
          width: ${isMobile ? '32px' : '14px'};
          height: ${isMobile ? '32px' : '14px'};
          bottom: 0;
          right: 0;
          background: transparent;
          cursor: se-resize;
          z-index: 10;
        }

        ${isMobile ? `
        .react-resizable-handle {
          width: 44px;
          height: 44px;
        }

        .react-resizable-handle::after {
          width: 20px;
          height: 20px;
          right: 8px;
          bottom: 8px;
          border-right-width: 3px;
          border-bottom-width: 3px;
        }
        ` : ''}

        .react-resizable-handle::after {
          content: '';
          position: absolute;
          right: 2px;
          bottom: 2px;
          width: 6px;
          height: 6px;
          border-right: 2px solid hsl(var(--muted-foreground) / 0.5);
          border-bottom: 2px solid hsl(var(--muted-foreground) / 0.5);
          border-radius: 0 0 1px 0;
        }

        .react-grid-layout.edit-mode > .react-grid-item:hover .react-resizable-handle::after {
          border-right-color: hsl(var(--primary));
          border-bottom-color: hsl(var(--primary));
          width: 8px;
          height: 8px;
        }

        /* Hide resize handles when not in edit mode */
        .react-grid-layout:not(.edit-mode) .react-resizable-handle {
          display: none;
        }

        /* Smooth transitions when not editing - only after initial mount settles */
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
    </div>
  )
}
