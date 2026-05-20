/**
 * DashboardGrid Component
 *
 * Responsive grid layout using react-grid-layout v2.
 * Supports drag-and-drop and resizing in edit mode.
 */

import { useState, useCallback, useMemo, useRef, memo } from 'react'
import { ResponsiveGridLayout, useContainerWidth } from 'react-grid-layout'
import type { Layout, LayoutItem, ResponsiveLayouts } from 'react-grid-layout'
import { cn } from '@/lib/utils'
import type { DashboardComponent } from '../types'
import { useDashboardLayout } from '../hooks/useDashboardLayout'

import 'react-grid-layout/css/styles.css'
import 'react-resizable/css/styles.css'

const BREAKPOINTS = { lg: 1200, md: 996, sm: 768, xs: 480 }
const COLUMNS = { lg: 12, md: 12, sm: 6, xs: 4 }
const ROW_HEIGHT = 60
const MARGIN: readonly [number, number] = [12, 12]

const isTouchDevice = (): boolean => {
  if (typeof window === 'undefined') return false
  return 'ontouchstart' in window || navigator.maxTouchPoints > 0
}

export interface DashboardGridProps {
  children: React.ReactNode
  components: DashboardComponent[]
  editMode: boolean
}

export const DashboardGrid = memo(function DashboardGrid({
  children,
  components,
  editMode,
}: DashboardGridProps) {
  const { layouts, onLayoutChange } = useDashboardLayout(components)
  const [isDragging, setIsDragging] = useState(false)
  const touchEnabled = isTouchDevice()
  const { width, containerRef, mounted } = useContainerWidth()

  const dragConfig = useMemo(
    () => ({
      enabled: editMode,
      handle: '.widget-drag-handle',
    }),
    [editMode]
  )

  const resizeConfig = useMemo(
    () => ({
      enabled: editMode,
    }),
    [editMode]
  )

  const handleDragStop = useCallback(
    (layout: Layout, _oldItem: LayoutItem | null, _newItem: LayoutItem | null) => {
      setIsDragging(false)
      onLayoutChange(layout)
    },
    [onLayoutChange]
  )

  const handleResizeStop = useCallback(
    (layout: Layout, _oldItem: LayoutItem | null, _newItem: LayoutItem | null) => {
      setIsDragging(false)
      onLayoutChange(layout)
    },
    [onLayoutChange]
  )

  const handleLayoutChange = useCallback(
    (layout: Layout, _layouts: ResponsiveLayouts) => {
      onLayoutChange(layout)
    },
    [onLayoutChange]
  )

  return (
    <div ref={containerRef as React.RefObject<HTMLDivElement>} className={cn('dashboard-grid-container w-full', isDragging && 'is-dragging')}>
      <style>{`
        .dashboard-grid-container .react-grid-layout {
          display: block !important;
          isolation: isolate;
        }
        .dashboard-grid-container .react-grid-item {
          transition: ${editMode ? 'transform 200ms ease' : 'none'};
          contain: paint;
        }
        .dashboard-grid-container .react-grid-item.react-draggable-dragging {
          z-index: 100;
          box-shadow: 0 12px 32px rgba(0, 0, 0, 0.15);
          transition: none;
          opacity: 0.9;
        }
        .dashboard-grid-container .react-grid-placeholder {
          background: rgba(148, 163, 184, 0.15) !important;
          border: 1px dashed rgba(148, 163, 184, 0.3) !important;
          border-radius: 0.5rem;
          transition: all 150ms ease;
        }
        .dashboard-grid-container .react-resizable-handle {
          position: absolute;
          width: ${touchEnabled ? '32px' : '14px'};
          height: ${touchEnabled ? '32px' : '14px'};
          bottom: 0;
          right: 0;
          background: transparent;
          cursor: se-resize;
          z-index: 10;
        }
        ${!editMode ? `
        .dashboard-grid-container .react-resizable-handle { display: none; }
        .dashboard-grid-container .react-grid-item { transition: none !important; }
        ` : ''}
      `}</style>

      {mounted && width > 0 && (
        <ResponsiveGridLayout
          className={cn('dashboard-grid', editMode && 'edit-mode')}
          width={width}
          layouts={layouts}
          breakpoints={BREAKPOINTS}
          cols={COLUMNS}
          rowHeight={ROW_HEIGHT}
          margin={MARGIN}
          dragConfig={dragConfig}
          resizeConfig={resizeConfig}
          onDragStart={() => setIsDragging(true)}
          onDragStop={handleDragStop}
          onResizeStart={() => setIsDragging(true)}
          onResizeStop={handleResizeStop}
          onLayoutChange={handleLayoutChange}
        >
          {children}
        </ResponsiveGridLayout>
      )}

      {editMode && <div className="h-48" />}
    </div>
  )
})
