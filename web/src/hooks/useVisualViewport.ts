import { useEffect } from 'react'

/**
 * Global VisualViewport tracking for mobile keyboard handling (2025)
 *
 * Fixes the issue where styles don't recover after keyboard dismissal
 * by using direct dvh units and forcing viewport recalculation.
 *
 * iOS PWA standalone note: window.innerHeight does NOT shrink when the soft
 * keyboard opens in standalone mode — only visualViewport.height does.
 * We detect this environment and expose a separate `--keyboard-offset`
 * CSS variable (0 everywhere else) so layout fixes can target iOS PWA
 * without breaking Android / iOS Safari browser (where `100dvh` already
 * shrinks correctly and applying the same offset would double-subtract).
 *
 * @see https://dev.to/franciscomoretti/fix-mobile-keyboard-overlap-with-visualviewport-3a4a
 */

let keyboardHeight = 0
let initialHeight = 0
let topNavHeight = 64 // Default 4rem = 64px

/**
 * Detect iOS PWA standalone mode — the only environment where the soft
 * keyboard doesn't resize the layout viewport. On Android PWA and iOS
 * Safari (browser tab), the webview / 100dvh already shrink to the
 * visible area, so manual offsetting would double-subtract.
 */
function detectIOSPwaStandalone(): boolean {
  if (typeof window === 'undefined') return false
  // display-mode: standalone covers "Add to Home Screen" PWAs.
  // display-mode: fullscreen covers iPad fullscreen PWAs.
  const standalone =
    window.matchMedia?.('(display-mode: standalone)').matches
    || window.matchMedia?.('(display-mode: fullscreen)').matches
  // Legacy iOS Safari property (deprecated but still set on iOS home-screen PWAs).
  const legacyStandalone = (window.navigator as unknown as { standalone?: boolean }).standalone === true
  if (!standalone && !legacyStandalone) return false
  const ios = /\b(iPhone|iPad|iPod)\b/.test(navigator.userAgent)
    || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1)
  return ios
}

const isIOSPwaStandalone = detectIOSPwaStandalone()

/**
 * Initialize global VisualViewport tracking
 * Call this once in your app root
 */
export function initVisualViewport() {
  if (typeof window === 'undefined') return

  // Tag the root element so CSS can target iOS PWA specifically.
  if (isIOSPwaStandalone) {
    document.documentElement.classList.add('ios-pwa-standalone')
  }

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
    //     and elements that should be offset by exactly the keyboard height
    //     regardless of platform behavior — e.g. fixed-bottom bars that need
    //     to clear the keyboard on iOS PWA).
    //   --keyboard-offset: same as --keyboard-height on iOS PWA standalone,
    //     0 elsewhere. Used by layout-level fixes (body height, --app-height)
    //     that must NOT double-subtract on platforms where 100dvh already
    //     shrinks.
    document.documentElement.style.setProperty('--keyboard-height', `${keyboardHeight}px`)
    document.documentElement.style.setProperty(
      '--keyboard-offset',
      isIOSPwaStandalone ? `${keyboardHeight}px` : '0px',
    )

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
 * On iOS PWA standalone, window.innerHeight does NOT shrink when the soft
 * keyboard opens — only visualViewport.height does. If we feed window.innerHeight
 * to the root container, the root stays full-screen-tall while body has shrunk
 * to the visible area; iOS then shifts the layout upward to reveal the focused
 * input, pushing the safe-area-padded header under the notch.
 *
 * So: when keyboard is open on iOS PWA standalone, --app-height tracks
 * visualViewport.height. Everywhere else (Android, iOS Safari browser,
 * desktop), window.innerHeight already reflects the visible area, so we
 * use it directly.
 */
function updateAppHeight() {
  if (typeof window === 'undefined') return

  const appHeight = (keyboardHeight > 0 && isIOSPwaStandalone && window.visualViewport)
    ? window.visualViewport.height
    : window.innerHeight
  document.documentElement.style.setProperty('--app-height', `${appHeight}px`)

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
