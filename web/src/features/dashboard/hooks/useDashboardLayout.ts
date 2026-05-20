/**
 * useDashboardLayout Hook
 *
 * Converts between DashboardComponent positions and react-grid-layout Layout objects.
 * Manages layout state and provides handlers for drag/resize events.
 */

import { useMemo, useCallback, useState } from 'react'
import type { Layout, LayoutItem } from 'react-grid-layout'
import type { DashboardComponent, ComponentPosition } from '../types'
import { useDashboardStore } from '../store'

// ============================================================================
// Conversion Helpers
// ============================================================================

/**
 * Convert DashboardComponent[] to react-grid-layout Layout
 */
export function toGridLayout(components: DashboardComponent[]): LayoutItem[] {
  return components.map((component) => ({
    i: component.id,
    x: component.position.x ?? 0,
    y: component.position.y ?? 0,
    w: component.position.w ?? 4,
    h: component.position.h ?? 3,
    minW: component.position.minW,
    minH: component.position.minH,
    maxW: component.position.maxW,
    maxH: component.position.maxH,
  }))
}

/**
 * Convert react-grid-layout Layout back to component position updates.
 */
export function fromGridLayout(
  layout: Layout,
  components: DashboardComponent[]
): Array<{ id: string; position: ComponentPosition }> {
  const componentMap = new Map(components.map((c) => [c.id, c]))

  return layout
    .filter((item) => componentMap.has(item.i))
    .map((item) => {
      const component = componentMap.get(item.i)!
      return {
        id: item.i,
        position: {
          x: item.x,
          y: item.y,
          w: item.w,
          h: item.h,
          minW: component.position.minW,
          minH: component.position.minH,
          maxW: component.position.maxW,
          maxH: component.position.maxH,
        },
      }
    })
}

// ============================================================================
// Hook
// ============================================================================

export function useDashboardLayout(components: DashboardComponent[]) {
  const batchUpdatePositions = useDashboardStore((s) => s.batchUpdatePositions)
  const [isDragging, setIsDragging] = useState(false)

  const layouts = useMemo(() => {
    const layout = toGridLayout(components) as unknown as Layout
    return { lg: layout, md: layout, sm: layout, xs: layout }
  }, [components])

  const onLayoutChange = useCallback(
    (layout: Layout) => {
      const positions = fromGridLayout(layout, components)
      if (positions.length > 0) {
        batchUpdatePositions(positions)
      }
      setIsDragging(false)
    },
    [components, batchUpdatePositions]
  )

  return { layouts, onLayoutChange, isDragging }
}
