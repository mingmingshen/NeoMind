/**
 * LED Indicator Component
 *
 * State indicator with LED-like visual feedback.
 * Layout matches ValueCard: left LED container + right content.
 * Supports data mapping configuration for flexible value handling.
 */

import { useMemo } from 'react'
import { cn } from '@/lib/utils'
import { DataMapper, type SingleValueMappingConfig } from '@/lib/dataMapping'
import { useDataSource } from '@/hooks/useDataSource'
import { Skeleton } from '@/components/ui/skeleton'
import { dashboardComponentSize, dashboardCardBase } from '@/design-system/tokens/size'
import {
  indicatorFontWeight,
  indicatorColors,
  type IndicatorState,
} from '@/design-system/tokens/indicator'
import type { DataSource } from '@/types/dashboard'
import { ErrorState } from '../shared'

export type LEDState = 'on' | 'off' | 'error' | 'warning' | 'unknown'

export interface ValueStateMapping {
  values?: string
  pattern?: string
  state: LEDState
  label?: string
  color?: string
}

export interface LEDIndicatorProps {
  dataSource?: DataSource
  state?: LEDState
  title?: string
  size?: 'sm' | 'md' | 'lg'
  valueMap?: ValueStateMapping[]
  defaultState?: LEDState
  color?: string
  showCard?: boolean
  showGlow?: boolean
  showAnimation?: boolean  // Control pulse/breathing animation

  // Data mapping configuration (enhances valueMap with thresholds)
  // stateThresholds is now part of SingleValueMappingConfig
  dataMapping?: SingleValueMappingConfig

  className?: string
}

// State configuration
const stateConfig = {
  on: {
    indicatorState: 'success' as IndicatorState,
    label: '开启',
    color: indicatorColors.success,
  },
  off: {
    indicatorState: 'neutral' as IndicatorState,
    label: '关闭',
    color: indicatorColors.neutral,
  },
  error: {
    indicatorState: 'error' as IndicatorState,
    label: '错误',
    color: indicatorColors.error,
  },
  warning: {
    indicatorState: 'warning' as IndicatorState,
    label: '警告',
    color: indicatorColors.warning,
  },
  unknown: {
    indicatorState: 'neutral' as IndicatorState,
    label: '未知',
    color: indicatorColors.neutral,
  },
}

// Extract value for matching - handles objects and arrays
function extractValueForMatching(value: unknown): string {
  if (value === null || value === undefined) {
    return ''
  }

  // Direct string or number
  if (typeof value === 'string' || typeof value === 'number') {
    return String(value).trim().toLowerCase()
  }

  // Boolean
  if (typeof value === 'boolean') {
    return value ? 'true' : 'false'
  }

  // Array - take last element (most recent)
  if (Array.isArray(value)) {
    if (value.length > 0) {
      return extractValueForMatching(value[value.length - 1])
    }
    return ''
  }

  // Object - try to extract value field
  if (typeof value === 'object') {
    const obj = value as Record<string, unknown>
    // Try common value fields
    const valueField = obj.value ?? obj.v ?? obj.val ?? obj.result ?? obj.data ?? obj.state
    if (valueField !== undefined) {
      return extractValueForMatching(valueField)
    }
  }

  return String(value).trim().toLowerCase()
}

// Match value to state
function matchValueToState(
  value: unknown,
  valueMap: ValueStateMapping[],
  defaultState: LEDState
): LEDState {
  if (value === null || value === undefined) {
    return defaultState
  }

  const normalizedValue = extractValueForMatching(value)

  for (const mapping of valueMap) {
    if (mapping.values) {
      const values = mapping.values.toLowerCase().split(',').map(v => v.trim())
      if (values.some(v => v === normalizedValue)) {
        return mapping.state
      }
    }

    if (mapping.pattern) {
      try {
        const regex = new RegExp(mapping.pattern, 'i')
        if (regex.test(normalizedValue)) {
          return mapping.state
        }
      } catch {
        // Skip invalid regex
      }
    }
  }

  return defaultState
}

function getCustomLabel(
  value: unknown,
  valueMap: ValueStateMapping[],
  matchedState: LEDState
): string | undefined {
  if (!valueMap) return undefined

  const normalizedValue = extractValueForMatching(value)

  for (const mapping of valueMap) {
    if (mapping.state === matchedState && mapping.label) {
      if (mapping.values) {
        const values = mapping.values.toLowerCase().split(',').map(v => v.trim())
        if (values.some(v => v === normalizedValue)) {
          return mapping.label
        }
      }
      if (mapping.pattern) {
        try {
          if (new RegExp(mapping.pattern, 'i').test(normalizedValue)) {
            return mapping.label
          }
        } catch {
          // Skip
        }
      }
    }
  }

  return undefined
}

function getCustomColor(
  value: unknown,
  valueMap: ValueStateMapping[],
  matchedState: LEDState
): string | undefined {
  if (!valueMap) return undefined

  const normalizedValue = extractValueForMatching(value)

  for (const mapping of valueMap) {
    if (mapping.state === matchedState && mapping.color) {
      if (mapping.values) {
        const values = mapping.values.toLowerCase().split(',').map(v => v.trim())
        if (values.some(v => v === normalizedValue)) {
          return mapping.color
        }
      }
      if (mapping.pattern) {
        try {
          if (new RegExp(mapping.pattern, 'i').test(normalizedValue)) {
            return mapping.color
          }
        } catch {
          // Skip
        }
      }
    }
  }

  return undefined
}

// Determine state from numeric value using thresholds
function getStateFromThresholds(
  value: number,
  thresholds?: SingleValueMappingConfig['stateThresholds']
): LEDState | null {
  if (!thresholds) return null

  // Check in order: error > warning > on > off
  if (thresholds.error) {
    const { operator, value: threshold } = thresholds.error
    if (DataMapper.evaluateThreshold(value, operator, threshold)) {
      return 'error'
    }
  }

  if (thresholds.warning) {
    const { operator, value: threshold } = thresholds.warning
    if (DataMapper.evaluateThreshold(value, operator, threshold)) {
      return 'warning'
    }
  }

  if (thresholds.on) {
    const { operator, value: threshold } = thresholds.on
    if (DataMapper.evaluateThreshold(value, operator, threshold)) {
      return 'on'
    }
  }

  if (thresholds.off) {
    const { operator, value: threshold } = thresholds.off
    if (DataMapper.evaluateThreshold(value, operator, threshold)) {
      return 'off'
    }
  }

  return null
}

export function LEDIndicator({
  dataSource,
  state: propState = 'off',
  title,
  size = 'md',
  valueMap,
  defaultState = 'unknown',
  color,
  showCard = true,
  showGlow = true,
  showAnimation = true,
  dataMapping,
  className,
}: LEDIndicatorProps) {
  const { data, loading, error } = useDataSource<unknown>(dataSource)

  // Determine the final state
  const ledState = useMemo(() => {
    if (error) return 'error'
    if (loading) return 'unknown'

    // Extract numeric value if available
    const numericValue = DataMapper.extractValue(data, dataMapping)

    // First try: Use state thresholds from dataMapping
    if (numericValue !== null && numericValue !== undefined) {
      const thresholdState = getStateFromThresholds(numericValue, dataMapping?.stateThresholds)
      if (thresholdState) {
        return thresholdState
      }
    }

    // Second try: Use valueMap for string/enum matching
    if (valueMap && valueMap.length > 0) {
      return matchValueToState(data, valueMap, defaultState)
    }

    // Third try: Auto-detect boolean/string states
    if (data !== null && data !== undefined) {
      // Extract value for matching (handles objects and arrays)
      const dataStr = extractValueForMatching(data)

      // Check for boolean-like values
      const boolValue = DataMapper.mapToBoolean(data, {
        valueMapping: {
          onValues: ['true', 'on', '1', 'yes', 'enabled', 'active', 'online'],
          offValues: ['false', 'off', '0', 'no', 'disabled', 'inactive', 'offline'],
        },
      })

      if (typeof data === 'boolean' || boolValue !== undefined) {
        return boolValue ? 'on' : 'off'
      }

      // Check for string states
      if (['on', 'true', '1', 'yes', 'enabled', 'active', 'online'].includes(dataStr)) {
        return 'on'
      }
      if (['off', 'false', '0', 'no', 'disabled', 'inactive', 'offline'].includes(dataStr)) {
        return 'off'
      }
      if (['error', 'failed', 'failure', 'critical'].includes(dataStr)) {
        return 'error'
      }
      if (['warning', 'warn'].includes(dataStr)) {
        return 'warning'
      }
    }

    return propState
  }, [data, valueMap, defaultState, propState, loading, error, dataMapping])

  const customLabel = useMemo(() => {
    if (data !== undefined && valueMap) {
      return getCustomLabel(data, valueMap, ledState)
    }
    return undefined
  }, [data, valueMap, ledState])

  const customColor = useMemo(() => {
    if (data !== undefined && valueMap) {
      return getCustomColor(data, valueMap, ledState)
    }
    return undefined
  }, [data, valueMap, ledState])

  const config = dashboardComponentSize[size]
  const stateCfg = stateConfig[ledState] || stateConfig.unknown
  const indicatorState = stateCfg.indicatorState
  const isActive = ledState === 'on' || ledState === 'error' || ledState === 'warning'

  const finalColor = customColor || color || stateCfg.color.base
  const colorConfig = stateCfg.color

  // Animation: all active states (on, error, warning) get pulse effect when showAnimation is true
  const animationClassName = showAnimation && isActive ? 'animate-pulse' : ''

  // Convert hex to rgba for background opacity
  const hexToRgba = (hex: string, alpha: number) => {
    const cleanHex = hex.replace('#', '')
    const r = parseInt(cleanHex.substring(0, 2), 16)
    const g = parseInt(cleanHex.substring(2, 4), 16)
    const b = parseInt(cleanHex.substring(4, 6), 16)
    return `rgba(${r}, ${g}, ${b}, ${alpha})`
  }

  // Use custom color or default background
  const useCustomBg = customColor || color
  const containerBgColor = useCustomBg && isActive ? hexToRgba(useCustomBg, 0.15) : undefined

  // Enhanced glow effect
  const glowStyle = showGlow && isActive
    ? `0 0 8px ${finalColor}60, 0 0 16px ${finalColor}40, 0 0 24px ${finalColor}20`
    : 'none'

  const displayLabel = title || customLabel || stateCfg.label

  // Error state
  if (error && dataSource) {
    return <ErrorState size={size} className={className} />
  }

  // Loading state
  if (loading) {
    return (
      <div className={cn(dashboardCardBase, 'flex-row items-center', config.contentGap, config.padding, className)}>
        <Skeleton className={cn(config.iconContainer, 'rounded-full')} />
        <Skeleton className={cn('h-4 w-20 rounded')} />
      </div>
    )
  }

  const content = (
    <>
      {/* LED Section - left side like ValueCard icon */}
      <div className={cn(
        'flex items-center justify-center shrink-0 rounded-full',
        config.iconContainer,
        !containerBgColor && (isActive ? colorConfig.bg : 'bg-muted/30'),
        animationClassName
      )}
      style={{
        backgroundColor: containerBgColor,
        boxShadow: glowStyle !== 'none' ? glowStyle : undefined,
      }}>
        {/* LED dot */}
        <div
          className={cn(
            'rounded-full transition-all duration-300',
            size === 'sm' ? 'h-2.5 w-2.5' : size === 'md' ? 'h-3 w-3' : 'h-4 w-4',
            isActive && 'ring-2 ring-white/20'
          )}
          style={{
            backgroundColor: finalColor,
            boxShadow: isActive ? `inset 0 1px 2px rgba(255,255,255,0.3), inset 0 -1px 2px rgba(0,0,0,0.2)` : undefined,
          }}
        />
      </div>

      {/* Label section - right side */}
      <div className="flex flex-col min-w-0 flex-1">
        <span className={cn(indicatorFontWeight.title, 'text-foreground truncate', config.titleText)}>
          {displayLabel}
        </span>
        {customLabel && (
          <span className={cn(indicatorFontWeight.label, 'text-muted-foreground', config.labelText)}>
            {stateCfg.label}
          </span>
        )}
      </div>
    </>
  )

  if (showCard) {
    return (
      <div className={cn(dashboardCardBase, 'flex-row items-center', config.contentGap, config.padding, className)}>
        {content}
      </div>
    )
  }

  return <div className={cn('flex items-center', config.contentGap, 'w-full', config.padding, className)}>{content}</div>
}
