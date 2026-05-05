import { ReactNode } from 'react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

export interface Action {
  label: string
  icon?: ReactNode
  variant?: 'default' | 'primary' | 'destructive' | 'outline' | 'ghost' | 'secondary'
  onClick: () => void
  disabled?: boolean
}

export interface ActionBarProps {
  title?: string
  titleIcon?: ReactNode
  description?: string
  actions?: Action[] | ReactNode
  leftContent?: ReactNode
  className?: string
}

/**
 * Action bar component for page headers with actions
 */
export function ActionBar({
  title,
  titleIcon,
  description,
  actions = [],
  leftContent,
  className,
}: ActionBarProps) {
  return (
    <div className={cn('flex items-center justify-between gap-4 mb-6', className)}>
      {/* Left side: title, description, left content */}
      <div className="flex items-center gap-3 flex-1 min-w-0">
        {titleIcon && (
          <div className="flex items-center justify-center w-9 h-9 rounded-lg bg-muted-50">
            {titleIcon}
          </div>
        )}
        <div className="min-w-0">
          {title && <h2 className="text-xl font-semibold truncate">{title}</h2>}
          {description && <p className="text-sm text-muted-foreground">{description}</p>}
        </div>
        {leftContent}
      </div>

      {/* Right side: actions */}
      <div className="flex items-center gap-2 shrink-0">
        {Array.isArray(actions) ? (
          actions.map((action, index) => (
            <Button
              key={index}
              variant={action.variant === 'primary' ? 'default' : action.variant || 'outline'}
              size="sm"
              onClick={action.onClick}
              disabled={action.disabled}
            >
              {action.icon && <span className="mr-2">{action.icon}</span>}
              {action.label}
            </Button>
          ))
        ) : (
          actions
        )}
      </div>
    </div>
  )
}

/**
 * Compact action bar for smaller spaces
 */
export interface ActionBarCompactProps {
  actions: Action[]
  className?: string
}

export function ActionBarCompact({ actions, className }: ActionBarCompactProps) {
  return (
    <div className={cn('flex items-center justify-end gap-2 mb-4', className)}>
      {actions.map((action, index) => (
        <Button
          key={index}
          variant={action.variant === 'primary' ? 'default' : action.variant || 'outline'}
          size="sm"
          onClick={action.onClick}
          disabled={action.disabled}
        >
          {action.icon && <span className="mr-1">{action.icon}</span>}
          {action.label}
        </Button>
      ))}
    </div>
  )
}
