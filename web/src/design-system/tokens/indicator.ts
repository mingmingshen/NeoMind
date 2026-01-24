/**
 * Design Tokens - Indicator Components
 *
 * Unified styling for all indicator components:
 * - ValueCard
 * - LEDIndicator
 * - Sparkline
 * - ProgressBar
 */

import { statusColors, chartColors } from './color'

// ============================================================================
// Font Weight Standards
// ============================================================================

/**
 * Unified font weights for indicator components
 * - title: Primary label/title text
 * - value: Numeric value display
 * - label: Secondary descriptive text
 * - meta: Metadata, timestamps, etc.
 */
export const indicatorFontWeight = {
  title: 'font-medium',      // Primary text (labels, titles)
  value: 'font-semibold',    // Numeric values (slightly bold for emphasis)
  label: 'font-normal',      // Secondary text (descriptions)
  meta: 'font-normal',       // Metadata (timestamps, units)
} as const

export type IndicatorFontWeight = keyof typeof indicatorFontWeight

// ============================================================================
// Unified Color System
// ============================================================================

/**
 * Unified state colors for all indicator components
 * Uses OKLCH for consistent, perceptually uniform colors
 */
export const indicatorColors = {
  // Success/On state - vibrant green
  success: {
    base: statusColors.success,           // oklch(0.646 0.222 142.5)
    text: 'text-emerald-600 dark:text-emerald-400',
    bg: 'bg-emerald-500/10',
    border: 'border-emerald-500/20',
    ring: 'ring-emerald-500/20',
    shadow: 'shadow-emerald-500/20',
  },

  // Warning state - warm amber
  warning: {
    base: statusColors.warning,           // oklch(0.646 0.222 85.85)
    text: 'text-amber-600 dark:text-amber-400',
    bg: 'bg-amber-500/10',
    border: 'border-amber-500/20',
    ring: 'ring-amber-500/20',
    shadow: 'shadow-amber-500/20',
  },

  // Error state - clear red
  error: {
    base: statusColors.error,             // oklch(0.576 0.222 25.85)
    text: 'text-rose-600 dark:text-rose-400',
    bg: 'bg-rose-500/10',
    border: 'border-rose-500/20',
    ring: 'ring-rose-500/20',
    shadow: 'shadow-rose-500/20',
  },

  // Info state - calm blue
  info: {
    base: statusColors.info,              // oklch(0.646 0.222 264.38)
    text: 'text-cyan-600 dark:text-cyan-400',
    bg: 'bg-cyan-500/10',
    border: 'border-cyan-500/20',
    ring: 'ring-cyan-500/20',
    shadow: 'shadow-cyan-500/20',
  },

  // Neutral/Off state - subtle gray
  neutral: {
    base: statusColors.neutral,           // oklch(0.551 0.0 264.38)
    text: 'text-muted-foreground',
    bg: 'bg-muted/50',
    border: 'border-border',
    ring: 'transparent',
    shadow: undefined,
  },

  // Primary accent - brand color
  primary: {
    base: chartColors[1],                 // oklch(0.646 0.222 264.38)
    text: 'text-primary',
    bg: 'bg-primary/10',
    border: 'border-primary/20',
    ring: 'ring-primary/20',
    shadow: 'shadow-primary/20',
  },
} as const

export type IndicatorState = keyof typeof indicatorColors

// ============================================================================
// Gradient Definitions for Decorative Elements
// ============================================================================

/**
 * Gradient definitions for decorative visual elements
 * Used in progress bars, sparkline fills, area charts
 */
export const indicatorGradients = {
  success: {
    from: statusColors.success,
    to: 'oklch(0.646 0.222 142.5 / 0)',  // Fade to transparent
    stops: [
      { offset: '0%', color: statusColors.success, opacity: 0.5 },
      { offset: '50%', color: statusColors.success, opacity: 0.2 },
      { offset: '100%', color: statusColors.success, opacity: 0 },
    ],
  },
  warning: {
    from: statusColors.warning,
    to: 'oklch(0.646 0.222 85.85 / 0)',
    stops: [
      { offset: '0%', color: statusColors.warning, opacity: 0.5 },
      { offset: '50%', color: statusColors.warning, opacity: 0.2 },
      { offset: '100%', color: statusColors.warning, opacity: 0 },
    ],
  },
  error: {
    from: statusColors.error,
    to: 'oklch(0.576 0.222 25.85 / 0)',
    stops: [
      { offset: '0%', color: statusColors.error, opacity: 0.5 },
      { offset: '50%', color: statusColors.error, opacity: 0.2 },
      { offset: '100%', color: statusColors.error, opacity: 0 },
    ],
  },
  info: {
    from: statusColors.info,
    to: 'oklch(0.646 0.222 264.38 / 0)',
    stops: [
      { offset: '0%', color: statusColors.info, opacity: 0.5 },
      { offset: '50%', color: statusColors.info, opacity: 0.2 },
      { offset: '100%', color: statusColors.info, opacity: 0 },
    ],
  },
  neutral: {
    from: statusColors.neutral,
    to: 'oklch(0.551 0.0 264.38 / 0)',
    stops: [
      { offset: '0%', color: statusColors.neutral, opacity: 0.3 },
      { offset: '50%', color: statusColors.neutral, opacity: 0.1 },
      { offset: '100%', color: statusColors.neutral, opacity: 0 },
    ],
  },
  primary: {
    from: chartColors[1],
    to: 'oklch(0.646 0.222 264.38 / 0)',
    stops: [
      { offset: '0%', color: chartColors[1], opacity: 0.5 },
      { offset: '50%', color: chartColors[1], opacity: 0.2 },
      { offset: '100%', color: chartColors[1], opacity: 0 },
    ],
  },
} as const

export type IndicatorGradientType = keyof typeof indicatorGradients

// ============================================================================
// LED Glow Effects
// ============================================================================

/**
 * Enhanced glow effect configurations for LED indicators
 * Uses multiple box-shadow layers for realistic glow
 */
export const ledGlowEffects = {
  // No glow for off state
  none: 'none',

  // Subtle glow - gentle ambient light
  subtle: (color: string) => `0 0 4px ${color}40, 0 0 8px ${color}20`,

  // Soft glow - visible but not overwhelming
  soft: (color: string) => `0 0 6px ${color}60, 0 0 12px ${color}30, 0 0 20px ${color}15`,

  // Medium glow - standard active state
  medium: (color: string) => `0 0 8px ${color}80, 0 0 16px ${color}50, 0 0 28px ${color}25, 0 0 40px ${color}10`,

  // Strong glow - emphasized state
  strong: (color: string) => `0 0 12px ${color}aa, 0 0 24px ${color}70, 0 0 40px ${color}40, 0 0 60px ${color}20`,

  // Intense glow - maximum visibility
  intense: (color: string) => `0 0 16px ${color}cc, 0 0 32px ${color}90, 0 0 56px ${color}60, 0 0 80px ${color}30`,
} as const

/**
 * LED animation configurations
 */
export const ledAnimations = {
  // Pulse animation - gentle breathing effect
  pulse: {
    className: 'animate-pulse',
    duration: '2000ms',
    keyframes: {
      '0%, 100%': { opacity: '1', transform: 'scale(1)' },
      '50%': { opacity: '0.7', transform: 'scale(0.95)' },
    },
  },

  // Blink animation - quick on/off
  blink: {
    className: 'animate-blink',
    duration: '1000ms',
    keyframes: {
      '0%, 49%': { opacity: '1' },
      '50%, 100%': { opacity: '0.3' },
    },
  },

  // Glow pulse - animated glow intensity
  glowPulse: {
    className: 'animate-glow-pulse',
    duration: '1500ms',
    keyframes: {
      '0%, 100%': { boxShadow: '0 0 8px currentColor' },
      '50%': { boxShadow: '0 0 20px currentColor, 0 0 30px currentColor' },
    },
  },

  // Ripple animation - outward expanding rings
  ripple: {
    className: 'animate-ripple',
    duration: '2000ms',
    keyframes: {
      '0%': { transform: 'scale(1)', opacity: '0.8' },
      '100%': { transform: 'scale(2)', opacity: '0' },
    },
  },
} as const

export type LedAnimationType = keyof typeof ledAnimations

/**
 * Get LED glow effect by state
 */
export function getLedGlow(state: IndicatorState, customColor?: string): string {
  if (state === 'neutral') return ledGlowEffects.none

  const color = customColor || indicatorColors[state].base

  // Apply different glow intensity based on state
  switch (state) {
    case 'success':
      return ledGlowEffects.medium(color)
    case 'warning':
      return ledGlowEffects.soft(color)
    case 'error':
      return ledGlowEffects.strong(color)  // Stronger glow for error
    case 'info':
      return ledGlowEffects.soft(color)
    case 'primary':
      return ledGlowEffects.medium(color)
    default:
      return ledGlowEffects.subtle(color)
  }
}

/**
 * Get LED animation by state
 */
export function getLedAnimation(state: IndicatorState, isBlinking = false): {
  className?: string
  style?: React.CSSProperties
} {
  if (isBlinking) {
    return { className: 'animate-pulse' }
  }

  if (state === 'error') {
    // Error state gets a subtle pulse
    return { className: 'animate-pulse' }
  }

  return {}
}

// ============================================================================
// Value State Helpers
// ============================================================================

/**
 * Get gradient type based on percentage/value
 */
export function getValueGradient(
  value: number,
  max: number,
  warningThreshold = 70,
  dangerThreshold = 90,
  customColor?: string
): IndicatorGradientType {
  if (customColor) return 'primary'

  const percentage = (value / max) * 100

  if (percentage >= dangerThreshold) return 'error'
  if (percentage >= warningThreshold) return 'warning'
  return 'success'
}

/**
 * Get color based on value state
 */
export function getValueStateColor(
  value: number,
  max: number,
  warningThreshold = 70,
  dangerThreshold = 90,
  customColor?: string
): string {
  if (customColor) return customColor

  const percentage = (value / max) * 100

  if (percentage >= dangerThreshold) return indicatorColors.error.base
  if (percentage >= warningThreshold) return indicatorColors.warning.base
  return indicatorColors.success.base
}

/**
 * Get indicator state config
 */
export function getIndicatorColors(state: IndicatorState, customColor?: string) {
  if (customColor && state === 'success') {
    return {
      ...indicatorColors.success,
      base: customColor,
    }
  }
  return indicatorColors[state]
}

/**
 * Get text color class based on value state
 */
export function getValueTextColor(
  value: number,
  max: number,
  warningThreshold = 70,
  dangerThreshold = 90
): string {
  const percentage = (value / max) * 100

  if (percentage >= dangerThreshold) return indicatorColors.error.text
  if (percentage >= warningThreshold) return indicatorColors.warning.text
  return indicatorColors.success.text
}

// ============================================================================
// Gradient Helpers
// ============================================================================

/**
 * Get gradient stops array for a given type
 */
export function getGradientStops(
  type: IndicatorGradientType,
  baseColor?: string
): Array<{ offset: string; color: string; opacity: number }> {
  if (baseColor) {
    return [
      { offset: '0%', color: baseColor, opacity: 0.5 },
      { offset: '50%', color: baseColor, opacity: 0.2 },
      { offset: '100%', color: baseColor, opacity: 0 },
    ]
  }

  return [...indicatorGradients[type].stops]
}

/**
 * Get gradient ID for SVG
 */
export function getGradientId(type: IndicatorGradientType, suffix = ''): string {
  return `indicator-gradient-${type}${suffix ? `-${suffix}` : ''}`
}

/**
 * Get CSS linear gradient string
 */
export function getLinearGradient(
  type: IndicatorGradientType,
  direction: 'to right' | 'to bottom' | 'to top' = 'to right',
  customColor?: string
): string {
  const stops = getGradientStops(type, customColor)
  const stopStrings = stops.map(s => `${s.color.replace(')', ` / ${s.opacity})`)} ${s.offset}`)
  return `linear-gradient(${direction}, ${stopStrings.join(', ')})`
}
