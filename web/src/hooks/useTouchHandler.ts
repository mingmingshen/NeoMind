/**
 * Touch Event Handler Hook
 *
 * Provides unified event handling for both mouse and touch interactions.
 * Prevents duplicate events and ghost clicks on mobile devices.
 */

import { useCallback } from 'react'

export interface UseTouchHandlerOptions {
  /** Prevent default behavior on touch */
  preventDefault?: boolean
  /** Stop event propagation */
  stopPropagation?: boolean
}

/**
 * Returns handlers for both mouse and touch events that call the same callback.
 * Use this for buttons, list items, and other interactive elements.
 *
 * @example
 * ```tsx
 * const { handleClick, handleTouchEnd } = useTouchHandler(() => {
 *   console.log('Item clicked')
 * })
 *
 * <button onClick={handleClick} onTouchEnd={handleTouchEnd}>
 *   Click me
 * </button>
 * ```
 */
export function useTouchHandler(
  callback: () => void,
  options: UseTouchHandlerOptions = {}
) {
  const { preventDefault = true, stopPropagation = false } = options

  const handleClick = useCallback((e: React.MouseEvent) => {
    if (stopPropagation) e.stopPropagation()
    callback()
  }, [callback, stopPropagation])

  const handleTouchEnd = useCallback((e: React.TouchEvent) => {
    if (preventDefault) e.preventDefault()
    if (stopPropagation) e.stopPropagation()
    callback()
  }, [callback, preventDefault, stopPropagation])

  return { handleClick, handleTouchEnd }
}

/**
 * Simplified version that returns a single handler function.
 * The handler can be used for both onClick and onTouchEnd.
 *
 * @example
 * ```tsx
 * const handlePress = usePressHandler(() => {
 *   console.log('Pressed')
 * })
 *
 * <button onClick={handlePress} onTouchEnd={handlePress}>
 *   Press me
 * </button>
 * ```
 */
export function usePressHandler(
  callback: () => void,
  options: UseTouchHandlerOptions = {}
) {
  const { preventDefault = true, stopPropagation = false } = options

  return useCallback((e: React.MouseEvent | React.TouchEvent) => {
    if (preventDefault) {
      // Only call preventDefault on touch events
      if ('touches' in e) {
        e.preventDefault()
      }
    }
    if (stopPropagation) e.stopPropagation()
    callback()
  }, [callback, preventDefault, stopPropagation])
}
