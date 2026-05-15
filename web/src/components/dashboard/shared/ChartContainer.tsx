/**
 * Chart Container Component
 *
 * Pure CSS layout container for chart components.
 * Provides minimum height so ResponsiveContainer always has positive dimensions,
 * eliminating "width(-1)" warnings from Recharts.
 */

import type { ReactNode } from 'react'

export function ChartContainer({ children }: { children: ReactNode }) {
  return (
    <div className="w-full flex-1 min-h-0" style={{ minHeight: 120 }}>
      {children}
    </div>
  )
}
