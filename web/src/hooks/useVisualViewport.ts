import { useEffect } from 'react'

/**
 * Global VisualViewport tracking for mobile keyboard handling (2025)
 *
 * Fixes the issue where styles don't recover after keyboard dismissal
 * by using direct dvh units and forcing viewport recalculation.
 *
 * @see https://dev.to/franciscomoretti/fix-mobile-keyboard-overlap-with-visualviewport-3a4a
 */

let keyboardHeight = 0
let initialHeight = 0
let topNavHeight = 64 // Default 4rem = 64px

/**
 * Initialize global VisualViewport tracking
 * Call this once in your app root
 */
export function initVisualViewport() {
  if (typeof window === 'undefined') return

  // Get the actual viewport height (considering safe area)
  initialHeight = window.innerHeight

  // Store initial viewport height as CSS variable
  document.documentElement.style.setProperty('--initial-viewport-height', `${initialHeight}px`)

  // Set app height to actual viewport height
  updateAppHeight()

  const updateViewport = () => {
    if (!window.visualViewport) return

    const currentHeight = window.visualViewport.height
    const diff = initialHeight - currentHeight

    // Detect keyboard: height decreased by more than 100px
    const wasOpen = keyboardHeight > 0
    const isOpen = diff > 100
    keyboardHeight = isOpen ? diff : 0

    // Update CSS variable
    document.documentElement.style.setProperty('--keyboard-height', `${keyboardHeight}px`)

    // Update app height
    updateAppHeight()

    // Toggle body class for additional styling hooks
    if (isOpen) {
      document.body.classList.add('keyboard-open')
    } else {
      document.body.classList.remove('keyboard-open')
    }

    // Keyboard just closed - force style recalculation
    if (wasOpen && !isOpen) {
      // Small delay to ensure browser has finished animation
      setTimeout(() => {
        // Force layout recalculation
        void document.body.offsetHeight
        // Reset any transforms that might have been applied
        document.body.style.transform = ''
        // Ensure app height is correct after keyboard closes
        updateAppHeight()
      }, 100)
    }
  }

  // Initial update
  updateViewport()

  // Listen to visualViewport changes
  window.visualViewport?.addEventListener('resize', updateViewport, { passive: true })
  window.visualViewport?.addEventListener('scroll', updateViewport, { passive: true })

  // Also listen for blur events (when input loses focus)
  document.addEventListener('blur', (e) => {
    if (e instanceof HTMLInputElement || e instanceof HTMLTextAreaElement) {
      // Keyboard might be closing, force update after delay
      setTimeout(() => {
        updateViewport()
        updateAppHeight()
      }, 150)
    }
  }, true)

  // Listen for window resize (orientation change, etc.)
  window.addEventListener('resize', () => {
    initialHeight = window.innerHeight
    updateAppHeight()
  }, { passive: true })
}

/**
 * Update the app height CSS variable
 * Uses the larger of window.innerHeight or visualViewport.height
 * to ensure the app fills the screen even after keyboard dismissal
 */
function updateAppHeight() {
  if (typeof window === 'undefined') return

  // Use the actual viewport height, not the visual viewport
  // This ensures the app fills the screen even after keyboard closes
  const appHeight = window.innerHeight
  document.documentElement.style.setProperty('--app-height', `${appHeight}px`)

  // Update topnav height based on actual nav element
  const topNavEl = document.querySelector('nav')
  if (topNavEl) {
    topNavHeight = topNavEl.getBoundingClientRect().height
    document.documentElement.style.setProperty('--topnav-height', `${topNavHeight}px`)
  }
}

/**
 * Set the topnav height (call from TopNav component after mount)
 */
export function setTopNavHeight(height: number) {
  topNavHeight = height
  if (typeof window !== 'undefined') {
    document.documentElement.style.setProperty('--topnav-height', `${height}px`)
  }
}

/**
 * Hook for components to track keyboard state
 */
export function useKeyboardState() {
  // This hook provides keyboard state without internal state
  // Components can read --keyboard-height CSS variable directly
  return {
    isOpen: keyboardHeight > 0,
    height: keyboardHeight,
  }
}

/**
 * Force viewport recalculation (call manually if needed)
 */
export function forceViewportReset() {
  if (typeof window === 'undefined') return

  keyboardHeight = 0
  document.documentElement.style.setProperty('--keyboard-height', '0px')

  // Update app height to current window height
  updateAppHeight()

  // Remove keyboard-open class immediately
  document.body.classList.remove('keyboard-open')

  // Blur any focused input
  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur()
  }

  // Force scroll container to top
  const scrollContainer = document.getElementById('main-scroll-container')
  if (scrollContainer) {
    scrollContainer.scrollTop = 0
  }
}
