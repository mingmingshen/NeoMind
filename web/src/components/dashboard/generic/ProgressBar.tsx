/**
 * Progress Bar Component (Redesigned)
 *
 * Clean, professional layout that works well across devices.
 * Uses unified indicator fonts and gradient fills.
 */

import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { useDataSource } from '@/hooks/useDataSource'
import { dashboardComponentSize, dashboardCardBase } from '@/design-system/tokens/size'
import { getValueStateColor, getValueTextColor, getGradientStops, indicatorFontWeight } from '@/design-system/tokens/indicator'
import type { DataSourceOrList } from '@/types/dashboard'

export interface ProgressBarProps {
  dataSource?: DataSourceOrList
  value?: number
  max?: number
  label?: string
  size?: 'sm' | 'md' | 'lg'
  color?: string
  warningThreshold?: number
  dangerThreshold?: number
  showCard?: boolean
  variant?: 'default' | 'compact' | 'circular'
  className?: string
}

function safeToNumber(value: unknown): number {
  if (typeof value === 'number') return value
  if (typeof value === 'string') {
    const num = parseFloat(value)
    return isNaN(num) ? 0 : num
  }
  if (typeof value === 'boolean') return value ? 1 : 0
  return 0
}

export function ProgressBar({
  dataSource,
  value: propValue,
  max = 100,
  label,
  size = 'md',
  color,
  warningThreshold = 70,
  dangerThreshold = 90,
  showCard = true,
  variant = 'default',
  className,
}: ProgressBarProps) {
  // When dataSource is provided, don't use propValue as fallback
  const { data, loading, error } = useDataSource<number>(dataSource, {
    fallback: dataSource ? undefined : (propValue ?? 0)
  })
  const rawValue = error ? propValue : data ?? propValue ?? 0
  const value = safeToNumber(rawValue)
  const percentage = Math.min(100, Math.max(0, (value / max) * 100))

  const sizeConfig = dashboardComponentSize[size]
  const barHeight = size === 'sm' ? 'h-1.5' : size === 'md' ? 'h-2' : 'h-2.5'

  // Use unified color and gradient system
  const progressColor = getValueStateColor(value, max, warningThreshold, dangerThreshold, color)
  const textColor = getValueTextColor(value, max, warningThreshold, dangerThreshold)

  // Generate gradient ID for this instance
  const gradientId = `progress-gradient-${Math.random().toString(36).substring(2, 9)}`
  const gradientStops = getGradientStops(
    percentage >= dangerThreshold ? 'error' : percentage >= warningThreshold ? 'warning' : 'success',
    color
  )

  // Compact variant - just the progress bar with gradient
  if (variant === 'compact') {
    return (
      <div className={cn('w-full h-full flex items-center', className)}>
        {loading ? (
          <Skeleton className={cn('w-full h-full rounded-full', barHeight)} />
        ) : (
          <div className={cn('relative w-full h-full rounded-full bg-muted/40 overflow-hidden', barHeight)}>
            <svg className="absolute inset-0 w-full h-full" preserveAspectRatio="none">
              <defs>
                <linearGradient id={gradientId} x1="0%" y1="0%" x2="100%" y2="0%">
                  {gradientStops.map((stop, i) => (
                    <stop key={i} offset={stop.offset} stopColor={stop.color} stopOpacity={stop.opacity} />
                  ))}
                </linearGradient>
              </defs>
              <rect
                x="0"
                y="0"
                width="100%"
                height="100%"
                fill={`url(#${gradientId})`}
                style={{ transition: 'width 0.5s ease-out' }}
              />
            </svg>
            <div
              className={cn('h-full rounded-full transition-all duration-500 ease-out absolute inset-y-0 left-0')}
              style={{
                width: `${percentage}%`,
                background: `linear-gradient(90deg, ${progressColor}, ${progressColor}dd)`,
              }}
            />
          </div>
        )}
      </div>
    )
  }

  // Circular variant with glow effect
  if (variant === 'circular') {
    const radius = size === 'sm' ? 28 : size === 'md' ? 32 : 36
    const strokeWidth = size === 'sm' ? 3 : size === 'md' ? 3.5 : 4
    const circumference = 2 * Math.PI * (radius - strokeWidth / 2)
    const offset = circumference - (percentage / 100) * circumference
    const gradientIdCircular = `circular-gradient-${Math.random().toString(36).substring(2, 9)}`

    return (
      <div className={cn(dashboardCardBase, 'flex flex-col items-center justify-center', sizeConfig.padding, className)}>
        {loading ? (
          <Skeleton className={cn('rounded-full', size === 'sm' ? 'h-16 w-16' : size === 'md' ? 'h-20 w-20' : 'h-24 w-24')} />
        ) : (
          <div className="relative">
            <svg className={cn('transform -rotate-90', size === 'sm' ? 'h-16 w-16' : size === 'md' ? 'h-20 w-20' : 'h-24 w-24')} viewBox={`0 0 ${radius * 2} ${radius * 2}`}>
              <defs>
                <linearGradient id={gradientIdCircular} x1="0%" y1="0%" x2="100%" y2="0%">
                  {gradientStops.map((stop, i) => (
                    <stop key={i} offset={stop.offset} stopColor={stop.color} stopOpacity={stop.opacity + 0.6} />
                  ))}
                </linearGradient>
                <filter id={`glow-${gradientIdCircular}`} x="-50%" y="-50%" width="200%" height="200%">
                  <feGaussianBlur stdDeviation="2" result="coloredBlur" />
                  <feMerge>
                    <feMergeNode in="coloredBlur" />
                    <feMergeNode in="SourceGraphic" />
                  </feMerge>
                </filter>
              </defs>
              {/* Background track */}
              <circle
                cx={radius}
                cy={radius}
                r={radius - strokeWidth / 2}
                fill="none"
                stroke="hsl(var(--muted) / 0.3)"
                strokeWidth={strokeWidth}
              />
              {/* Progress with gradient and glow */}
              <circle
                cx={radius}
                cy={radius}
                r={radius - strokeWidth / 2}
                fill="none"
                stroke={`url(#${gradientIdCircular})`}
                strokeWidth={strokeWidth}
                strokeDasharray={circumference}
                strokeDashoffset={offset}
                strokeLinecap="round"
                className="transition-all duration-500 ease-out"
                filter={`url(#glow-${gradientIdCircular})`}
              />
            </svg>
            <div className="absolute inset-0 flex items-center justify-center">
              <span className={cn(indicatorFontWeight.value, 'text-foreground tabular-nums', sizeConfig.valueText)}>
                {Math.round(percentage)}%
              </span>
            </div>
          </div>
        )}
        {label && (
          <span className={cn(indicatorFontWeight.label, 'text-muted-foreground mt-2', sizeConfig.labelText)}>
            {label}
          </span>
        )}
      </div>
    )
  }

  // Default variant - Sparkline-style card layout with gradient
  const content = (
    <div className="flex flex-col w-full min-h-0">
      {/* Header: label (left) + percentage (right) */}
      <div className="flex items-center justify-between mb-1">
        {label && (
          <span className={cn(indicatorFontWeight.title, 'text-muted-foreground truncate text-xs', sizeConfig.labelText)} title={label}>
            {label}
          </span>
        )}
        {!label && <span />}
        {loading ? (
          <Skeleton className={cn('h-3 w-7 shrink-0 rounded')} />
        ) : (
          <span className={cn(
            indicatorFontWeight.value,
            'tabular-nums text-xs',
            textColor
          )}>
            {Math.round(percentage)}%
          </span>
        )}
      </div>

      {/* Progress bar with gradient fill */}
      <div className="flex-1 min-h-0 flex items-center">
        {loading ? (
          <div className={cn('w-full rounded-full bg-muted/30 overflow-hidden', barHeight)}>
            <Skeleton className={cn('h-full w-full rounded-full', barHeight)} />
          </div>
        ) : (
          <div className={cn('w-full rounded-full bg-muted/30 overflow-hidden relative', barHeight)}>
            <svg className="absolute inset-0 w-full h-full" preserveAspectRatio="none">
              <defs>
                <linearGradient id={gradientId} x1="0%" y1="0%" x2="100%" y2="0%">
                  {gradientStops.map((stop, i) => (
                    <stop key={i} offset={stop.offset} stopColor={stop.color} stopOpacity={stop.opacity} />
                  ))}
                </linearGradient>
              </defs>
            </svg>
            <div
              className={cn('h-full rounded-full transition-all duration-500 ease-out')}
              style={{
                width: `${percentage}%`,
                background: `linear-gradient(90deg, ${gradientStops[0].color}, ${gradientStops[2]?.color || gradientStops[0].color})`,
              }}
            />
          </div>
        )}
      </div>
    </div>
  )

  if (showCard) {
    return (
      <div className={cn(dashboardCardBase, 'flex items-center justify-center', sizeConfig.padding, className)}>
        {content}
      </div>
    )
  }

  return <div className={cn('w-full flex items-center justify-center', sizeConfig.padding, className)}>{content}</div>
}
