import { Bell, Clock, Zap, ChevronDown, ChevronUp, Play, Edit, Trash2 } from 'lucide-react'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { cn } from '@/lib/utils'
import { useState } from 'react'

interface AutomationCardProps {
  id: string
  name: string
  description?: string
  enabled: boolean
  trigger: {
    type: string
    description: string
    icon?: React.ReactNode
  }
  actions: Array<{
    type: string
    description: string
    icon?: React.ReactNode
  }>
  stats?: {
    executions: number
    lastRun?: string
  }
  onToggle: () => void
  onExecute?: () => void
  onEdit: () => void
  onDelete: () => void
  className?: string
}

export function AutomationCard({
  name,
  description,
  enabled,
  trigger,
  actions,
  stats,
  onToggle,
  onExecute,
  onEdit,
  onDelete,
  className,
}: AutomationCardProps) {
  const [expanded, setExpanded] = useState(false)

  const getTriggerIcon = () => {
    if (trigger.icon) return trigger.icon
    switch (trigger.type) {
      case 'device_state':
      case 'device':
        return <Zap className="h-4 w-4" />
      case 'schedule':
      case 'cron':
        return <Clock className="h-4 w-4" />
      case 'manual':
        return <Play className="h-4 w-4" />
      case 'event':
        return <Bell className="h-4 w-4" />
      default:
        return <Zap className="h-4 w-4" />
    }
  }

  return (
    <Card
      className={cn(
        'group overflow-hidden transition-all duration-200 hover:shadow-md',
        !enabled && 'opacity-60',
        className
      )}
    >
      {/* Main Card Content */}
      <div className="p-4">
        {/* Header: Name + Status Toggle */}
        <div className="flex items-start justify-between mb-3">
          <div className="flex items-center gap-3 flex-1 min-w-0">
            {/* Status Indicator */}
            <div
              className={cn(
                'flex items-center justify-center w-10 h-10 rounded-xl transition-all',
                enabled
                  ? 'bg-primary/10 text-primary'
                  : 'bg-muted text-muted-foreground'
              )}
            >
              {getTriggerIcon()}
            </div>

            {/* Name + Description */}
            <div className="flex-1 min-w-0">
              <h3 className="font-semibold text-base truncate">{name}</h3>
              {description && (
                <p className="text-sm text-muted-foreground truncate">{description}</p>
              )}
            </div>
          </div>

          {/* Toggle Switch */}
          <Switch checked={enabled} onCheckedChange={onToggle} />
        </div>

        {/* Trigger Preview */}
        <div className="mb-3">
          <div className="text-xs text-muted-foreground mb-1.5">触发条件</div>
          <div className="flex items-center gap-2 p-2.5 bg-muted/50 rounded-lg">
            <div className="flex items-center justify-center w-7 h-7 rounded-md bg-primary/10 text-primary shrink-0">
              {getTriggerIcon()}
            </div>
            <span className="text-sm truncate">{trigger.description}</span>
          </div>
        </div>

        {/* Actions Preview */}
        <div className="mb-3">
          <div className="flex items-center justify-between mb-1.5">
            <span className="text-xs text-muted-foreground">执行动作</span>
            {actions.length > 2 && (
              <Button
                variant="ghost"
                size="sm"
                className="h-5 px-2 text-xs"
                onClick={() => setExpanded(!expanded)}
              >
                {expanded ? (
                  <>
                    <ChevronUp className="h-3 w-3 mr-0.5" />
                    收起
                  </>
                ) : (
                  <>
                    <ChevronDown className="h-3 w-3 mr-0.5" />
                    全部 {actions.length} 个
                  </>
                )}
              </Button>
            )}
          </div>
          <div className="space-y-1.5">
            {(expanded ? actions : actions.slice(0, 2)).map((action, index) => (
              <div
                key={index}
                className="flex items-center gap-2 p-2 bg-background rounded-md border group/item hover:border-primary/50 transition-colors"
              >
                <div className="flex items-center justify-center w-6 h-6 rounded bg-muted shrink-0">
                  {action.icon || <Zap className="h-3 w-3" />}
                </div>
                <span className="text-sm truncate flex-1">{action.description}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Footer: Stats + Actions */}
        <div className="flex items-center justify-between pt-2 border-t">
          {/* Stats */}
          <div className="flex items-center gap-3 text-xs text-muted-foreground">
            {stats?.executions !== undefined && (
              <span className="flex items-center gap-1">
                <Play className="h-3 w-3" />
                {stats.executions} 次执行
              </span>
            )}
            {stats?.lastRun && (
              <span className="flex items-center gap-1">
                <Clock className="h-3 w-3" />
                {stats.lastRun}
              </span>
            )}
          </div>

          {/* Action Buttons */}
          <div className="flex items-center gap-1">
            {onExecute && enabled && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2"
                onClick={onExecute}
              >
                <Play className="h-3 w-3 mr-1" />
                执行
              </Button>
            )}
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={onEdit}
            >
              <Edit className="h-3.5 w-3.5" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 text-destructive hover:text-destructive"
              onClick={onDelete}
            >
              <Trash2 className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      </div>
    </Card>
  )
}

// Grid container for automation cards
export function AutomationCardGrid({
  children,
  className,
}: {
  children: React.ReactNode
  className?: string
}) {
  return (
    <div
      className={cn(
        'grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4',
        className
      )}
    >
      {children}
    </div>
  )
}

// Empty state for no automations
export function AutomationEmptyState({
  title,
  description,
  onCreate,
  onCreateFromTemplate,
}: {
  title: string
  description: string
  onCreate: () => void
  onCreateFromTemplate?: () => void
}) {
  return (
    <div className="flex flex-col items-center justify-center py-16 px-4 text-center">
      <div className="w-20 h-20 rounded-full bg-muted flex items-center justify-center mb-4">
        <Zap className="h-10 w-10 text-muted-foreground" />
      </div>
      <h3 className="text-lg font-semibold mb-2">{title}</h3>
      <p className="text-sm text-muted-foreground mb-6 max-w-md">{description}</p>
      <div className="flex items-center gap-3">
        <Button onClick={onCreate}>
          <Zap className="h-4 w-4 mr-2" />
          创建自动化
        </Button>
        {onCreateFromTemplate && (
          <Button variant="outline" onClick={onCreateFromTemplate}>
            从模板创建
          </Button>
        )}
      </div>
    </div>
  )
}
