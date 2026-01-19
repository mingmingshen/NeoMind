import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'

interface PortalProps {
  children: React.ReactNode
}

export function Portal({ children }: PortalProps) {
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
    return () => setMounted(false)
  }, [])

  if (!mounted) return null

  return createPortal(children, document.body)
}

// Get portal root element for dialogs/popovers
export function getPortalRoot(): HTMLElement {
  return document.body
}

// Clean up portal content (utility function)
export function cleanupPortalContent(): void {
  // Portal content is automatically cleaned up by React
  // This is a placeholder for any manual cleanup if needed
}
