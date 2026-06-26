import { useEffect } from 'react'

/**
 * Global VisualViewport tracking for mobile keyboard handling (2025)
 *
 * With `interactive-widget=resizes-content` in the viewport meta tag, the
 * layout viewport itself shrinks when the soft keyboard opens on every
 * modern platform (iOS 16.4+ Safari + PWA, Android Chrome, desktop). This
 * means `100dvh`, `position: fixed; bottom: 0`, and `window.innerHeight`
 * all adapt automatically — no platform-specific keyboard hacks needed.
 *
 * This hook now exists primarily to:
 *   - Track `--keyboard-height` for components that want to know the real
 *     keyboard size (e.g., chat scroll-padding to keep messages above
 *     the keyboard).
 *   - Toggle `body.keyboard-open` for opt-in styling hooks.
 *   - Refresh `--app-height` / `--initial-viewport-height` on resize.
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

    // Update CSS variables:
    //   --keyboard-height: raw keyboard height (used for keyboard detection
    //     and elements that need to know the real keyboard size for custom
    //     offsetting / scroll-padding).
    //   --keyboard-offset: ALWAYS 0. With `interactive-widget=resizes-content`
    //     in the viewport meta (iOS 16.4+ / Android Chrome), the layout
    //     viewport itself shrinks when the keyboard opens — `100dvh` and
    //     `position: fixed; bottom: 0` already adapt. Setting --keyboard-offset
    //     to a non-zero value would double-subtract and push content above
    //     the visible area.
    document.documentElement.style.setProperty('--keyboard-height', `${keyboardHeight}px`)
    document.documentElement.style.setProperty('--keyboard-offset', '0px')

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
 * Update the app height CSS variable.
 *
 * With `interactive-widget=resizes-content` in the viewport meta tag,
 * window.innerHeight already reflects the visible area on every platform
 * (iOS 16.4+ Safari + PWA, Android Chrome, desktop). No platform-specific
 * branching needed.
 */
function updateAppHeight() {
  if (typeof window === 'undefined') return

  document.documentElement.style.setProperty('--app-height', `${window.innerHeight}px`)

  // Mobile layout: no global TopNav. Each page renders its own MobilePageHeader
  // (with hamburger + page title + actions) as the first child of its content.
  // The header carries its own safe-top padding, so the main element's top
  // padding collapses to 0.
  const isMobileViewport = window.innerWidth < 768
  if (isMobileViewport) {
    document.documentElement.style.setProperty('--topnav-height', '0px')
    document.documentElement.style.setProperty('--bottom-nav-height', '0px')
    return
  }

  // Desktop: measure the actual nav element.
  document.documentElement.style.setProperty('--bottom-nav-height', '0px')
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
  document.documentElement.style.setProperty('--keyboard-offset', '0px')

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
