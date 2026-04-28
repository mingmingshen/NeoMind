import { ReactNode } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'

export interface IconCardProps {
  icon: ReactNode
  title: string
  iconColor?: 'yellow' | 'blue' | 'purple' | 'green' | 'red'
  children: ReactNode
  className?: string
}

const iconColorClass = {
  yellow: 'text-warning',
  blue: 'text-info',
  purple: 'text-purple-500',
  green: 'text-success',
  red: 'text-error',
}

/**
 * Card with icon in header - for stats and overview cards
 *
 * Replaces the repeated pattern in automation.tsx and similar pages.
 *
 * @example
 * <IconCard icon={<Zap className="w-5 h-5" />} title="规则引擎" iconColor="yellow">
 *   <div>Stats content here</div>
 * </IconCard>
 */
export function IconCard({ icon, title, iconColor = 'blue', children, className }: IconCardProps) {
  return (
    <Card className={cn('', className)}>
      <CardHeader className="pb-3">
        <CardTitle className="text-base flex items-center gap-2">
          <span className={cn('w-5 h-5', iconColorClass[iconColor])}>{icon}</span>
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent>{children}</CardContent>
    </Card>
  )
}
