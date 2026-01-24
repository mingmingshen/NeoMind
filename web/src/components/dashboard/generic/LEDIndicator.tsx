/**
 * LED Indicator Component (Enhanced)
 *
 * Features:
 * - Unified color system with OKLCH colors
 * - Enhanced multi-layer glow effects
 * - Smooth animations (pulse, blink)
 * - Multiple variants (dot, pill, badge, card)
 */

import { cn } from '@/lib/utils'
import { useDataSource } from '@/hooks/useDataSource'
import { Skeleton } from '@/components/ui/skeleton'
import { dashboardComponentSize, dashboardCardBase } from '@/design-system/tokens/size'
import {
  indicatorFontWeight,
  indicatorColors,
  getLedGlow,
  getLedAnimation,
  getLinearGradient,
  type IndicatorState,
} from '@/design-system/tokens/indicator'
import type { DataSource } from '@/types/dashboard'

export type LEDState = 'on' | 'off' | 'blinking' | 'error' | 'warning'

export interface LEDIndicatorProps {
  dataSource?: DataSource
  state?: LEDState
  label?: string
  size?: 'sm' | 'md' | 'lg'
  color?: string
  errorColor?: string
  warningColor?: string
  showCard?: boolean
  variant?: 'dot' | 'pill' | 'badge' | 'card'
  showGlow?: boolean
  className?: string
}

// Map LED state to indicator state
function mapLedState(ledState: LEDState): IndicatorState {
  switch (ledState) {
    case 'on':
    case 'blinking':
      return 'success'
    case 'off':
      return 'neutral'
    case 'error':
      return 'error'
    case 'warning':
      return 'warning'
    default:
      return 'neutral'
  }
}

// Size configuration
const getIconSize = (size: 'sm' | 'md' | 'lg') => {
  const config = dashboardComponentSize[size]
  return {
    dot: size === 'sm' ? 'h-2 w-2' : size === 'md' ? 'h-2.5 w-2.5' : 'h-3 w-3',
    dotWithGlow: size === 'sm' ? 'h-3 w-3' : size === 'md' ? 'h-3.5 w-3.5' : 'h-4 w-4',
    ...config,
  }
}

const stateLabels = {
  on: '开启',
  off: '关闭',
  blinking: '运行中',
  error: '错误',
  warning: '警告',
}

export function LEDIndicator({
  dataSource,
  state = 'on',
  label,
  size = 'md',
  color,
  errorColor,
  warningColor,
  showCard = true,
  variant = 'badge',
  showGlow = true,
  className,
}: LEDIndicatorProps) {
  const { data, loading, error } = useDataSource<LEDState>(dataSource, { fallback: state })
  const ledState = error ? 'off' : data ?? state
  const config = getIconSize(size)

  const indicatorState = mapLedState(ledState)
  const isBlinking = ledState === 'blinking'
  const isOn = ledState === 'on' || ledState === 'blinking'

  // Get unified color configuration
  const getColorConfig = () => {
    if (ledState === 'off') return indicatorColors.neutral
    if (color && ledState === 'on') {
      return { ...indicatorColors.success, base: color }
    }
    if (errorColor && ledState === 'error') {
      return { ...indicatorColors.error, base: errorColor }
    }
    if (warningColor && ledState === 'warning') {
      return { ...indicatorColors.warning, base: warningColor }
    }
    return indicatorColors[indicatorState]
  }

  const colorConfig = getColorConfig()
  const ledColor = colorConfig.base

  // Get enhanced glow effect
  const glowEffect = isOn && showGlow ? getLedGlow(indicatorState, color) : 'none'

  // Get animation
  const animation = getLedAnimation(indicatorState, isBlinking)

  // Generate unique ID for gradient
  const gradientId = `led-gradient-${Math.random().toString(36).substring(2, 9)}`

  // ============================================================================
  // Dot variant - inline LED dot
  // ============================================================================

  if (variant === 'dot') {
    return (
      <div className={cn('inline-flex items-center gap-2 overflow-hidden', className)}>
        <div
          className={cn(
            'rounded-full transition-all duration-300 shrink-0',
            animation.className,
            config.dot
          )}
          style={{
            backgroundColor: ledColor,
            boxShadow: glowEffect,
          }}
        />
        {label && <span className={cn(indicatorFontWeight.label, colorConfig.text, config.labelText)}>{label}</span>}
      </div>
    )
  }

  // ============================================================================
  // Pill variant - horizontal pill with LED
  // ============================================================================

  if (variant === 'pill') {
    return (
      <div
        className={cn(
          'inline-flex items-center gap-2 rounded-full border transition-all duration-300 overflow-hidden',
          'px-3 py-1.5',
          colorConfig.border,
          ledState === 'off' && colorConfig.bg,
          animation.className
        )}
        style={{
          background: isOn ? getLinearGradient(indicatorState, 'to right', color) : undefined,
        }}
      >
        <div
          className={cn('rounded-full shrink-0', config.dot)}
          style={{
            backgroundColor: ledColor,
            boxShadow: glowEffect,
          }}
        />
        {label && <span className={cn(indicatorFontWeight.title, colorConfig.text || 'text-foreground')}>{label}</span>}
      </div>
    )
  }

  // ============================================================================
  // Badge variant - fills container with gradient background
  // ============================================================================

  if (variant === 'badge') {
    if (loading) {
      return (
        <div className={cn(dashboardCardBase, 'items-center', config.padding, className)}>
          <div className="flex items-center gap-3 w-full h-full min-w-0">
            <Skeleton className={cn('rounded-full shrink-0', config.iconContainer)} />
            <div className="flex flex-col min-w-0 flex-1 gap-2">
              <Skeleton className={cn('h-4 w-20 rounded')} />
              <Skeleton className={cn('h-3 w-12 rounded')} />
            </div>
          </div>
        </div>
      )
    }

    const content = (
      <div className="flex items-center gap-3 w-full h-full min-w-0">
        {/* LED indicator with gradient container */}
        <div
          className={cn(
            'flex items-center justify-center rounded-full shrink-0 transition-all duration-300',
            config.iconContainer,
            ledState === 'off' && colorConfig.bg,
            animation.className
          )}
          style={{
            background: isOn ? getLinearGradient(indicatorState, 'to right', color) : undefined,
          }}
        >
          {/* Inner LED dot with enhanced glow */}
          <div
            className={cn('rounded-full transition-all duration-300', config.dotWithGlow)}
            style={{
              backgroundColor: ledColor,
              boxShadow: glowEffect,
            }}
          />
        </div>
        <div className="flex flex-col min-w-0 flex-1 overflow-hidden">
          {label && <span className={cn(indicatorFontWeight.title, 'text-foreground truncate', config.titleText)}>{label}</span>}
          <span className={cn(indicatorFontWeight.label, colorConfig.text || 'text-muted-foreground', config.labelText)}>
            {stateLabels[ledState]}
          </span>
        </div>
      </div>
    )

    if (showCard) {
      return (
        <div className={cn(dashboardCardBase, 'items-center', config.padding, className)}>
          {content}
        </div>
      )
    }

    return <div className={cn('flex items-center w-full h-full overflow-hidden', config.padding, className)}>{content}</div>
  }

  // ============================================================================
  // Card variant - centered card with large LED and ripple effect
  // ============================================================================

  if (variant === 'card') {
    if (loading) {
      return (
        <div className={cn(dashboardCardBase, 'flex flex-col items-center justify-center gap-3', config.padding, className)}>
          <Skeleton className={cn('rounded-full', size === 'sm' ? 'w-16 h-16' : size === 'md' ? 'w-20 h-20' : 'w-24 h-24')} />
          <div className="flex flex-col items-center gap-2">
            <Skeleton className={cn('h-5 w-16 rounded')} />
            <Skeleton className={cn('h-4 w-12 rounded')} />
          </div>
        </div>
      )
    }

    const cardSize = size === 'sm' ? { container: 'w-16 h-16', dot: 'h-6 w-6' }
                    : size === 'md' ? { container: 'w-20 h-20', dot: 'h-8 w-8' }
                    : { container: 'w-24 h-24', dot: 'h-10 w-10' }

    return (
      <div className={cn(dashboardCardBase, 'flex flex-col items-center justify-center gap-3', config.padding, className)}>
        {/* Large LED indicator with multiple visual layers */}
        <div className="relative">
          {/* Outer glow ring */}
          {isOn && showGlow && (
            <div
              className={cn(
                'absolute inset-0 rounded-full transition-all duration-500',
                animation.className
              )}
              style={{
                background: getLinearGradient(indicatorState, 'to right', color),
                opacity: 0.3,
                filter: 'blur(8px)',
              }}
            />
          )}

          {/* Main LED container */}
          <div
            className={cn(
              'flex items-center justify-center rounded-full transition-all duration-300 relative',
              cardSize.container,
              ledState === 'off' && colorConfig.bg
            )}
            style={{
              background: isOn ? getLinearGradient(indicatorState, 'to right', color) : undefined,
              border: isOn ? `1px solid ${ledColor}40` : undefined,
            }}
          >
            {/* Inner LED dot with enhanced glow */}
            <div
              className={cn(
                'rounded-full transition-all duration-300 relative z-10',
                cardSize.dot,
                animation.className
              )}
              style={{
                backgroundColor: ledColor,
                boxShadow: glowEffect,
              }}
            />
          </div>

          {/* Ripple effect rings for active state */}
          {isOn && showGlow && (
            <>
              <div
                className={cn(
                  'absolute inset-0 rounded-full -z-10',
                  animation.className
                )}
                style={{
                  border: `1px solid ${ledColor}30`,
                  animation: 'ripple 2s ease-out infinite',
                }}
              />
              <div
                className={cn(
                  'absolute inset-0 rounded-full -z-10',
                  animation.className
                )}
                style={{
                  border: `1px solid ${ledColor}20`,
                  animation: 'ripple 2s ease-out infinite 0.5s',
                }}
              />
            </>
          )}
        </div>

        {/* Label and state */}
        <div className="flex flex-col items-center text-center">
          {label && <span className={cn(indicatorFontWeight.title, 'text-foreground', config.titleText)}>{label}</span>}
          <span className={cn(indicatorFontWeight.label, colorConfig.text || 'text-muted-foreground', config.labelText)}>
            {stateLabels[ledState]}
          </span>
        </div>
      </div>
    )
  }

  return null
}
