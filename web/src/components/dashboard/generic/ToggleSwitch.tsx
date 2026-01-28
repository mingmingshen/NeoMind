/**
 * Toggle Switch Component
 *
 * Command-only toggle for sending device commands.
 * Displays current state and sends toggle command on click.
 */

import { Power, Lightbulb, Fan, Lock } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { useDataSource } from '@/hooks/useDataSource'
import { Skeleton } from '@/components/ui/skeleton'
import { dashboardCardBase, dashboardComponentSize } from '@/design-system/tokens/size'
import { indicatorFontWeight } from '@/design-system/tokens/indicator'
import type { DataSource } from '@/types/dashboard'

export interface ToggleSwitchProps {
  // Command data source (required)
  dataSource?: DataSource

  // Display
  title?: string
  label?: string  // Alternative to title, displayed in style section
  size?: 'sm' | 'md' | 'lg'

  // Initial state for display before command response
  initialState?: boolean

  // Edit mode - disable click when editing dashboard
  editMode?: boolean

  disabled?: boolean
  className?: string
}

// Get icon based on title
function getIconForTitle(title?: string): React.ComponentType<{ className?: string }> {
  if (!title) return Power
  const lower = title.toLowerCase()
  if (lower.includes('light') || lower.includes('lamp')) return Lightbulb
  if (lower.includes('fan')) return Fan
  if (lower.includes('lock')) return Lock
  return Power
}

export function ToggleSwitch({
  dataSource,
  title,
  size = 'md',
  initialState = false,
  editMode = false,
  disabled = false,
  className,
}: ToggleSwitchProps) {
  const { t } = useTranslation('dashboardComponents')
  const { data, loading, sendCommand, sending } = useDataSource<boolean>(dataSource, {
    fallback: initialState,
  })

  // Display current state (from command response) or initial state
  const checked = data ?? initialState
  const hasCommand = dataSource?.type === 'command'

  const handleClick = async () => {
    // Don't execute click in edit mode - allows dragging
    if (editMode) return
    if (disabled || loading || sending || !hasCommand || !sendCommand) return

    // Send toggle command with new state
    await sendCommand(!checked)
  }

  const config = dashboardComponentSize[size]
  const Icon = getIconForTitle(title)

  // Loading state
  if (loading) {
    return (
      <div className={cn(dashboardCardBase, 'flex-row items-center', config.contentGap, config.padding, className)}>
        <Skeleton className={cn(config.iconContainer, 'rounded-full')} />
        <Skeleton className={cn('h-4 w-20 rounded')} />
      </div>
    )
  }

  return (
    <button
      onClick={handleClick}
      disabled={disabled || loading || sending || !hasCommand || editMode}
      className={cn(
        dashboardCardBase,
        'flex-row items-center',
        config.contentGap,
        config.padding,
        'transition-all duration-200',
        !disabled && !sending && hasCommand && !editMode && 'hover:bg-accent/50',
        (disabled || sending || !hasCommand || editMode) && 'opacity-50 cursor-not-allowed',
        editMode && 'pointer-events-none',  // Allow dragging in edit mode
        className
      )}
    >
      {/* Icon Section - left side */}
      <div className={cn(
        'flex items-center justify-center shrink-0 rounded-full transition-all duration-300',
        config.iconContainer,
        checked
          ? 'bg-primary text-primary-foreground shadow-md'
          : 'bg-muted/50 text-muted-foreground'
      )}>
        <Icon className={cn(config.iconSize, checked ? 'opacity-100' : 'opacity-50')} />
      </div>

      {/* Title section - right side */}
      <div className="flex flex-col min-w-0 flex-1 text-left">
        {title ? (
          <span className={cn(indicatorFontWeight.title, 'text-foreground truncate', config.titleText)}>
            {title}
          </span>
        ) : (
          <span className={cn(indicatorFontWeight.title, 'text-foreground', config.titleText)}>
            {checked ? t('toggleSwitch.enabled') : t('toggleSwitch.disabled')}
          </span>
        )}
        {title && (
          <span className={cn(indicatorFontWeight.label, 'text-muted-foreground', config.labelText)}>
            {checked ? t('toggleSwitch.enabled') : t('toggleSwitch.disabled')}
          </span>
        )}
      </div>

      {/* Sending indicator */}
      {sending && (
        <span className="absolute top-3 right-3 w-2 h-2 rounded-full bg-amber-500 animate-pulse" />
      )}

      {/* Warning: no command configured */}
      {!hasCommand && (
        <span className="absolute top-3 right-3 w-2 h-2 rounded-full bg-orange-500" title={t('toggleSwitch.noCommandSource')} />
      )}
    </button>
  )
}
