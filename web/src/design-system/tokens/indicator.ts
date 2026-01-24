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
// Gradient Definitions
// ============================================================================

/**
 * Gradient definitions for indicator visual elements
 * Each gradient has ID, color stops, and CSS string representation
 */
export const indicatorGradients = {
  // Primary gradient (blue/purple)
  primary: {
    id: 'indicator-primary',
    stops: [
      { offset: '0%', color: chartColors[1], opacity: 0.4 },
      { offset: '50%', color: chartColors[1], opacity: 0.15 },
      { offset: '100%', color: chartColors[1], opacity: 0 },
    ],
  },

  // Success gradient (green)
  success: {
    id: 'indicator-success',
    stops: [
      { offset: '0%', color: statusColors.success, opacity: 0.4 },
      { offset: '50%', color: statusColors.success, opacity: 0.15 },
      { offset: '100%', color: statusColors.success, opacity: 0 },
    ],
  },

  // Warning gradient (yellow/amber)
  warning: {
    id: 'indicator-warning',
    stops: [
      { offset: '0%', color: statusColors.warning, opacity: 0.4 },
      { offset: '50%', color: statusColors.warning, opacity: 0.15 },
      { offset: '100%', color: statusColors.warning, opacity: 0 },
    ],
  },

  // Error gradient (red)
  error: {
    id: 'indicator-error',
    stops: [
      { offset: '0%', color: statusColors.error, opacity: 0.4 },
      { offset: '50%', color: statusColors.error, opacity: 0.15 },
      { offset: '100%', color: statusColors.error, opacity: 0 },
    ],
  },

  // Info gradient (cyan)
  info: {
    id: 'indicator-info',
    stops: [
      { offset: '0%', color: statusColors.info, opacity: 0.4 },
      { offset: '50%', color: statusColors.info, opacity: 0.15 },
      { offset: '100%', color: statusColors.info, opacity: 0 },
    ],
  },

  // Neutral gradient (gray)
  neutral: {
    id: 'indicator-neutral',
    stops: [
      { offset: '0%', color: statusColors.neutral, opacity: 0.3 },
      { offset: '50%', color: statusColors.neutral, opacity: 0.1 },
      { offset: '100%', color: statusColors.neutral, opacity: 0 },
    ],
  },
} as const

export type IndicatorGradientType = keyof typeof indicatorGradients

// ============================================================================
// Glow Effects
// ============================================================================

/**
 * Glow effect configurations for LED indicators and highlights
 */
export const indicatorGlow = {
  // Subtle glow for normal states
  subtle: (color: string) => `0 0 8px ${color}40, 0 0 16px ${color}20`,

  // Medium glow for active/enhanced states
  medium: (color: string) => `0 0 12px ${color}60, 0 0 24px ${color}30, 0 0 36px ${color}15`,

  // Strong glow for emphasized states
  strong: (color: string) => `0 0 16px ${color}80, 0 0 32px ${color}40, 0 0 48px ${color}20`,

  // Pulse animation for active indicators
  pulse: (color: string) => ({
    initial: `0 0 8px ${color}40`,
    animate: `0 0 16px ${color}60, 0 0 24px ${color}40`,
  }),
} as const

// ============================================================================
// Color State Mapping
// ============================================================================

/**
 * Get gradient config based on percentage/value
 * Used for progress bars and value indicators
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

  if (percentage >= dangerThreshold) return statusColors.error
  if (percentage >= warningThreshold) return statusColors.warning
  return statusColors.success
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

  if (percentage >= dangerThreshold) {
    return 'text-rose-600 dark:text-rose-400'
  }
  if (percentage >= warningThreshold) {
    return 'text-amber-600 dark:text-amber-400'
  }
  return 'text-emerald-600 dark:text-emerald-400'
}

// ============================================================================
// SVG Gradient Builder
// ============================================================================

/**
 * Get gradient stops array for a given gradient type
 */
export function getGradientStops(
  type: IndicatorGradientType,
  baseColor?: string
): Array<{ offset: string; color: string; opacity: number }> {
  if (baseColor) {
    return [
      { offset: '0%', color: baseColor, opacity: 0.4 },
      { offset: '50%', color: baseColor, opacity: 0.15 },
      { offset: '100%', color: baseColor, opacity: 0 },
    ]
  }

  return indicatorGradients[type].stops.map(s => ({
    offset: s.offset,
    color: s.color,
    opacity: s.opacity,
  }))
}

/**
 * Get gradient ID for a given type
 */
export function getGradientId(type: IndicatorGradientType, suffix = ''): string {
  return `${indicatorGradients[type].id}${suffix ? `-${suffix}` : ''}`
}

// ============================================================================
// CSS Classes
// ============================================================================

/**
 * Unified text color classes for different states
 */
export const indicatorTextColors = {
  primary: 'text-primary',
  success: 'text-emerald-600 dark:text-emerald-400',
  warning: 'text-amber-600 dark:text-amber-400',
  error: 'text-rose-600 dark:text-rose-400',
  info: 'text-cyan-600 dark:text-cyan-400',
  neutral: 'text-muted-foreground',
} as const

/**
 * Background classes with gradient support
 */
export const indicatorBgColors = {
  primary: 'bg-primary/10',
  success: 'bg-emerald-500/10',
  warning: 'bg-amber-500/10',
  error: 'bg-rose-500/10',
  info: 'bg-cyan-500/10',
  neutral: 'bg-muted/50',
} as const

/**
 * Border classes for different states
 */
export const indicatorBorderColors = {
  primary: 'border-primary/20',
  success: 'border-emerald-500/20',
  warning: 'border-amber-500/20',
  error: 'border-rose-500/20',
  info: 'border-cyan-500/20',
  neutral: 'border-border',
} as const
