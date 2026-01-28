/**
 * Agent Status Card Component
 *
 * Displays AI Agent status and activity monitoring for dashboard.
 * Shows agent name, role, current status, execution count, and real-time activity.
 */

import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Bot,
  Activity,
  Eye,
  Zap,
  BarChart3,
  CheckCircle2,
  XCircle,
  Loader2,
  TrendingUp,
  Play,
} from 'lucide-react'
import { Skeleton } from '@/components/ui/skeleton'
import { Badge } from '@/components/ui/badge'
import { cn, getIconForEntity } from '@/lib/utils'
import { formatTimestamp } from '@/lib/utils/format'
import { indicatorColors, dashboardCardBase } from '@/design-system'
import type { DataSourceOrList } from '@/types/dashboard'
import { useDataSource } from '@/hooks/useDataSource'
import { useAgentEvents } from '@/hooks/useAgentEvents'
import { ErrorState } from '../shared'

export interface AgentStatusCardProps {
  dataSource?: DataSourceOrList

  // Display
  agentId?: string
  agentName?: string
  title?: string
  description?: string

  // Configuration
  showExecutions?: boolean
  showSparkline?: boolean
  sparklineData?: number[]
  compact?: boolean

  // Actions
  onExecute?: () => void
  onViewDetails?: () => void

  className?: string
}

// Role configuration factory (uses translations)
function getRoleConfig(t: (key: string) => string) {
  return {
    Monitor: { icon: Eye, label: 'agentStatusCard.roles.monitor', color: 'text-blue-600 bg-blue-50 border-blue-200' },
    Executor: { icon: Zap, label: 'agentStatusCard.roles.executor', color: 'text-orange-600 bg-orange-50 border-orange-200' },
    Analyst: { icon: BarChart3, label: 'agentStatusCard.roles.analyst', color: 'text-purple-600 bg-purple-50 border-purple-200' },
  }
}

// Status configuration factory (uses translations)
function getStatusConfig(t: (key: string) => string) {
  return {
    Active: { icon: CheckCircle2, label: 'agentStatusCard.status.active', color: 'text-green-600 bg-green-50 border-green-200' },
    Paused: { icon: XCircle, label: 'agentStatusCard.status.paused', color: 'text-gray-600 bg-gray-50 border-gray-200' },
    Error: { icon: XCircle, label: 'agentStatusCard.status.error', color: 'text-red-600 bg-red-50 border-red-200' },
    Executing: { icon: Loader2, label: 'agentStatusCard.status.executing', color: 'text-blue-600 bg-blue-50 border-blue-200' },
  }
}

// ============================================================================
// Sparkline Component
// ============================================================================

interface SparklineProps {
  data: number[]
  color?: string
}

function Sparkline({ data, color = 'hsl(var(--primary))' }: SparklineProps) {
  const validData = data.filter((v): v is number => typeof v === 'number' && !isNaN(v))
  if (validData.length < 2) return null

  const min = Math.min(...validData)
  const max = Math.max(...validData)
  const range = max - min || 1

  const points = validData.map((v, i) => {
    const x = (i / (validData.length - 1)) * 100
    const y = 100 - ((v - min) / range) * 100
    return `${x},${y}`
  }).join(' ')

  const fillPoints = `${points} 100,0 0,0`

  return (
    <svg viewBox="0 0 100 25" className="w-full h-auto opacity-70" preserveAspectRatio="none">
      <defs>
        <linearGradient id={`gradient-agent-${color}`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor={color} stopOpacity="0.2" />
          <stop offset="100%" stopColor={color} stopOpacity="0" />
        </linearGradient>
      </defs>
      <polygon points={fillPoints} fill={`url(#gradient-agent-${color})`} />
      <polyline
        points={points}
        fill="none"
        stroke={color}
        strokeWidth="2"
        vectorEffect="non-scaling-stroke"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export function AgentStatusCard({
  dataSource,
  agentId,
  agentName,
  title,
  description,
  showExecutions = true,
  showSparkline = false,
  sparklineData,
  compact = false,
  onExecute,
  onViewDetails,
  className,
}: AgentStatusCardProps) {
  const { t } = useTranslation('dashboardComponents')
  const ROLE_CONFIG = getRoleConfig(t)
  const STATUS_CONFIG = getStatusConfig(t)

  // Fetch agent data from data source or use props
  const { data, loading, error } = useDataSource<any>(dataSource, {
    fallback: null,
  })

  // Use real-time events if agentId is provided
  const { isConnected, currentExecution } = useAgentEvents(agentId || '', {
    enabled: !!agentId,
  })

  // Extract agent info from data source or props
  const agent = data || {
    id: agentId,
    name: agentName || title,
    role: 'Monitor',
    status: 'Paused',
    execution_count: 0,
    success_count: 0,
    error_count: 0,
    last_execution_at: null,
  }

  // Use real-time execution status if available
  const displayStatus = currentExecution ? 'Executing' : agent?.status || 'Paused'
  const isExecuting = displayStatus === 'Executing'

  const roleConfig = ROLE_CONFIG[agent?.role as keyof typeof ROLE_CONFIG] || ROLE_CONFIG.Monitor
  const RoleIcon = roleConfig.icon
  const statusConfig = STATUS_CONFIG[displayStatus as keyof typeof STATUS_CONFIG] || STATUS_CONFIG.Paused
  const StatusIcon = statusConfig.icon

  // Error state
  if (error && dataSource) {
    return <ErrorState size="md" className={className} />
  }

  // Compact variant
  if (compact) {
    return (
      <div className={cn(dashboardCardBase, 'flex items-center gap-3 p-3', className)}>
        {/* Icon with status indicator */}
        <div className="relative">
          <div className="w-10 h-10 rounded-lg flex items-center justify-center bg-primary/10 text-primary">
            <Bot className="h-5 w-5" />
          </div>
          {isExecuting && (
            <div className="absolute -top-1 -right-1 w-3 h-3 bg-blue-500 rounded-full animate-pulse" />
          )}
        </div>

        {/* Content */}
        <div className="flex flex-col min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="font-medium text-sm truncate">{agent?.name || title}</span>
            <Badge variant="outline" className={cn('text-[10px] gap-1 px-1 h-4 shrink-0', statusConfig.color)}>
              <StatusIcon className={cn('h-2.5 w-2.5', isExecuting && 'animate-spin')} />
            </Badge>
          </div>
          {description && (
            <p className="text-xs text-muted-foreground truncate">{description}</p>
          )}
        </div>

        {/* Execution count */}
        {showExecutions && (
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <Activity className="h-3 w-3" />
            <span className="tabular-nums">{agent?.execution_count || 0}</span>
          </div>
        )}
      </div>
    )
  }

  // Full variant
  return (
    <div className={cn(dashboardCardBase, 'flex flex-col', className)}>
      {/* Header */}
      <div className="flex items-start justify-between p-4 pb-3">
        <div className="flex items-center gap-3">
          {/* Icon with status indicator */}
          <div className="relative">
            <div className={cn(
              "w-11 h-11 rounded-xl flex items-center justify-center transition-colors",
              isExecuting ? "bg-blue-500/10 text-blue-600" : "bg-primary/10 text-primary"
            )}>
              <Bot className="h-5.5 w-5.5" />
            </div>
            {isExecuting && (
              <div className="absolute -top-0.5 -right-0.5 w-3 h-3 bg-blue-500 rounded-full animate-pulse border-2 border-background" />
            )}
          </div>

          {/* Title and badges */}
          <div>
            <div className="flex items-center gap-2">
              <h3 className="font-semibold text-sm">{agent?.name || title || t('agentStatusCard.agent')}</h3>
            </div>
            <div className="flex items-center gap-1.5 mt-1">
              <Badge variant="outline" className={cn("text-[10px] gap-1 px-1.5 h-5", roleConfig.color)}>
                <RoleIcon className="h-2.5 w-2.5" />
                {t(roleConfig.label)}
              </Badge>
              <Badge variant="outline" className={cn("text-[10px] gap-1 px-1.5 h-5", statusConfig.color)}>
                <StatusIcon className={cn("h-2.5 w-2.5", isExecuting && "animate-spin")} />
                {t(statusConfig.label)}
              </Badge>
            </div>
          </div>
        </div>

        {/* Real-time indicator */}
        {isConnected && (
          <div className="flex items-center gap-1 text-xs text-green-600">
            <div className="w-1.5 h-1.5 bg-green-500 rounded-full animate-pulse" />
            <span>{t('agentStatusCard.live')}</span>
          </div>
        )}
      </div>

      {/* Stats */}
      <div className="px-4 pb-3">
        <div className="flex items-center gap-4 text-xs">
          <div className="flex items-center gap-1.5">
            <Activity className="h-3.5 w-3.5 text-muted-foreground" />
            <span className="text-muted-foreground">{t('agentStatusCard.executions')}:</span>
            <span className="font-medium tabular-nums">{agent?.execution_count || 0}</span>
          </div>
          <div className="flex items-center gap-1.5">
            <CheckCircle2 className="h-3.5 w-3.5 text-green-600" />
            <span className="text-muted-foreground">{t('agentStatusCard.success')}:</span>
            <span className="font-medium tabular-nums text-green-600">{agent?.success_count || 0}</span>
          </div>
          {agent?.error_count > 0 && (
            <div className="flex items-center gap-1.5">
              <XCircle className="h-3.5 w-3.5 text-red-500" />
              <span className="text-muted-foreground">{t('agentStatusCard.failure')}:</span>
              <span className="font-medium tabular-nums text-red-500">{agent?.error_count}</span>
            </div>
          )}
        </div>
        <div className="text-xs text-muted-foreground mt-1.5">
          {t('agentStatusCard.lastExecution')}: {agent?.last_execution_at ? formatTimestamp(agent.last_execution_at, false) : '-'}
        </div>
      </div>

      {/* Sparkline */}
      {showSparkline && sparklineData && sparklineData.length >= 2 && (
        <div className="px-4 pb-2">
          <Sparkline data={sparklineData} />
        </div>
      )}

      {/* Description */}
      {description && (
        <div className="px-4 pb-3">
          <p className="text-xs text-muted-foreground line-clamp-2">{description}</p>
        </div>
      )}

      {/* Current execution info */}
      {isExecuting && currentExecution && (
        <div className="mx-4 mb-3 p-2 bg-blue-50 dark:bg-blue-950/30 rounded-md border border-blue-200 dark:border-blue-800">
          <div className="flex items-center gap-2 text-xs text-blue-700 dark:text-blue-400">
            <Loader2 className="h-3 w-3 animate-spin" />
            <span className="font-medium">{t('agentStatusCard.executing')}</span>
            <span className="text-muted-foreground">·</span>
            <span className="text-muted-foreground">#{currentExecution.id?.slice(0, 8)}</span>
          </div>
          {currentExecution.steps && currentExecution.steps.length > 0 && (
            <div className="text-xs text-muted-foreground mt-1">
              {t('agentStatusCard.steps')}: {currentExecution.steps.length}
            </div>
          )}
        </div>
      )}

      {/* Footer */}
      <div className="mt-auto px-4 py-3 bg-muted/20 border-t flex items-center justify-between">
        {loading ? (
          <Skeleton className="h-4 w-24" />
        ) : (
          <span className="text-xs text-muted-foreground">
            {agent?.role || 'Monitor'}
          </span>
        )}

        <div className="flex items-center gap-2">
          {onViewDetails && (
            <button
              onClick={onViewDetails}
              className="text-xs text-muted-foreground hover:text-foreground transition-colors"
            >
              {t('agentStatusCard.viewDetails')}
            </button>
          )}
          {onExecute && (
            <button
              onClick={onExecute}
              disabled={isExecuting}
              className={cn(
                "flex items-center gap-1 text-xs px-2 py-1 rounded-md transition-colors",
                isExecuting
                  ? "bg-muted text-muted-foreground cursor-not-allowed"
                  : "bg-primary text-primary-foreground hover:bg-primary/90"
              )}
            >
              <Play className="h-3 w-3" />
              {t('agentStatusCard.execute')}
            </button>
          )}
        </div>
      </div>
    </div>
  )
}
