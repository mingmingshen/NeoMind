/**
 * KeepAlive — keeps a component mounted but hidden when the route doesn't match.
 *
 * Avoids the expensive unmount/remount cycle for heavy pages like VisualDashboard
 * where 16+ widget hooks, store subscriptions, and WebSocket listeners are created.
 *
 * Uses CSS `display: none` when inactive — preserves all React state, refs, and DOM.
 * The component is only mounted after its route is visited for the first time (lazy activation).
 */

import { useRef, useEffect } from 'react'
import { useLocation, matchPath } from 'react-router-dom'

interface KeepAliveProps {
  /** Route path patterns to match (e.g. "/visual-dashboard" or "/visual-dashboard/:dashboardId") */
  patterns: string[]
  children: React.ReactNode
}

export function KeepAlive({ patterns, children }: KeepAliveProps) {
  const location = useLocation()
  const isActive = patterns.some(p => matchPath(p, location.pathname) !== null)

  // Only mount after first activation — don't eagerly mount pages never visited
  const hasActivated = useRef(false)
  if (isActive) hasActivated.current = true

  // When re-activated after being hidden, dispatch a resize event so child
  // components (DashboardGrid) re-measure their container width.
  const wasActive = useRef(isActive)
  useEffect(() => {
    if (isActive && !wasActive.current) {
      requestAnimationFrame(() => {
        window.dispatchEvent(new Event('resize'))
      })
    }
    wasActive.current = isActive
  }, [isActive])

  if (!hasActivated.current) return null

  return (
    <div
      className={isActive ? 'h-full w-full' : ''}
      style={isActive ? undefined : { display: 'none' }}
    >
      {children}
    </div>
  )
}
