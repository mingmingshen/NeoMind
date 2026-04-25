/**
 * Chart Container Component
 *
 * Measures its own dimensions via ResizeObserver, then renders
 * children (which should include a ResponsiveContainer) only when
 * the container has positive width and height.
 *
 * Eliminates the "width(-1) and height(-1)" console warnings from Recharts
 * by ensuring ResponsiveContainer always has a properly sized parent.
 */

import { useRef, useState, useEffect, type ReactNode } from 'react'

interface Size {
  width: number
  height: number
}

export function ChartContainer({ children }: { children: ReactNode }) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState<Size | null>(null)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return

    let alive = true

    const measure = () => {
      if (!alive) return
      const { width, height } = el.getBoundingClientRect()
      if (width > 0 && height > 0) {
        setSize(prev => {
          if (prev && prev.width === width && prev.height === height) return prev
          return { width, height }
        })
      }
    }

    // Wait for flex layout to settle before first measurement
    const raf = requestAnimationFrame(measure)
    const ro = new ResizeObserver(measure)
    ro.observe(el)

    return () => {
      alive = false
      cancelAnimationFrame(raf)
      ro.disconnect()
    }
  }, [])

  return (
    <div ref={containerRef} className="w-full flex-1 min-h-0">
      {size && (
        <div style={{ width: size.width, height: size.height }}>
          {children}
        </div>
      )}
    </div>
  )
}
