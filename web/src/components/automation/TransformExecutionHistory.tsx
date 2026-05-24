import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Clock, CheckCircle2, XCircle, Loader2, Activity } from 'lucide-react'
import { api } from '@/lib/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { formatTimestamp } from '@/lib/utils/format'
import { getStatusColorClass, getStatusBgClass } from '@/design-system/utils/format'

interface ExecutionRecord {
  id: string
  automation_id: string
  started_at: number
  ended_at: number | null
  status: 'running' | 'completed' | 'failed' | 'cancelled'
  error: string | null
  output: { metric_count?: number; warning_count?: number } | null
}

interface TransformExecutionHistoryProps {
  transformId: string
  limit?: number
}

const STATUS_ICON = {
  completed: CheckCircle2,
  failed: XCircle,
  running: Loader2,
  cancelled: XCircle,
} as const

export function TransformExecutionHistory({ transformId, limit = 20 }: TransformExecutionHistoryProps) {
  const { t } = useTranslation(['automation', 'common'])
  const [executions, setExecutions] = useState<ExecutionRecord[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    const fetchExecutions = async () => {
      try {
        setLoading(true)
        const result = await api.getAutomationExecutions(transformId, limit)
        if (!cancelled) {
          setExecutions((result.executions as ExecutionRecord[]) || [])
          setError(null)
        }
      } catch (e) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : 'Failed to load executions')
        }
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    fetchExecutions()
    return () => { cancelled = true }
  }, [transformId, limit])

  if (loading) {
    return (
      <div className="flex items-center gap-2 py-3 text-sm text-muted-foreground">
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
        {t('automation:loadingExecutions', { defaultValue: 'Loading executions...' })}
      </div>
    )
  }

  if (error) {
    return (
      <div className="py-3 text-sm text-error">{error}</div>
    )
  }

  if (executions.length === 0) {
    return (
      <div className="py-3 text-sm text-muted-foreground">
        {t('automation:noExecutions', { defaultValue: 'No executions recorded yet' })}
      </div>
    )
  }

  const formatDuration = (started: number, ended: number | null) => {
    if (!ended) return '...'
    const ms = ended - started
    if (ms < 1000) return `${ms}ms`
    return `${(ms / 1000).toFixed(1)}s`
  }

  return (
    <div className="space-y-2">
      {executions.map((exec) => {
        const Icon = STATUS_ICON[exec.status] || Activity
        const statusClass = getStatusColorClass(exec.status)
        const bgClass = getStatusBgClass(exec.status)

        return (
          <div
            key={exec.id}
            className="flex items-center gap-3 rounded-md border border-border/50 px-3 py-2 text-sm"
          >
            <Icon className={`h-4 w-4 shrink-0 ${exec.status === 'running' ? 'animate-spin' : ''} ${statusClass}`} />
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <Badge variant="outline" className={`text-xs ${bgClass} ${statusClass}`}>
                  {exec.status}
                </Badge>
                <span className="text-muted-foreground">
                  <Clock className="inline h-3 w-3 mr-1" />
                  {formatTimestamp(exec.started_at)}
                </span>
                <span className="text-muted-foreground text-xs">
                  {formatDuration(exec.started_at, exec.ended_at)}
                </span>
              </div>
              {exec.error && (
                <p className="mt-1 text-xs text-error truncate">{exec.error}</p>
              )}
              {exec.output && exec.output.metric_count !== undefined && (
                <p className="mt-0.5 text-xs text-muted-foreground">
                  {t('automation:metricsOutput', {
                    count: exec.output.metric_count,
                    defaultValue: `${exec.output.metric_count} metrics`,
                  })}
                  {exec.output.warning_count ? ` · ${exec.output.warning_count} warnings` : ''}
                </p>
              )}
            </div>
          </div>
        )
      })}
    </div>
  )
}
