/**
 * Chart Container Component
 *
 * Pure CSS layout container for chart components.
 * Provides minimum height so ResponsiveContainer always has positive dimensions,
 * eliminating "width(-1)" warnings from Recharts.
 *
 * IMPORTANT: Must use inline style for minHeight because dashboard-components.css
 * overrides class-based min-height with `min-height: 0 !important`.
 * Inline style with `minHeight` takes precedence over CSS class rules.
 */

import type { ReactNode } from 'react'

export function ChartContainer({ children }: { children: ReactNode }) {
  return (
    <div
      className="w-full flex-1"
      style={{ minHeight: 120, height: '100%', position: 'relative' }}
    >
      {children}
    </div>
  )
}
