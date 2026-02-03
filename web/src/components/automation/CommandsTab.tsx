import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/lib/api'
import type { CommandDto } from '@/types'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { EmptyStateInline } from '@/components/shared'
import { useApiData } from '@/hooks/useApiData'
import { formatTimestamp } from '@/lib/utils/format'
import { useToast } from '@/hooks/use-toast'
import { RotateCcw, ChevronDown, ChevronUp } from 'lucide-react'

const fetchCommands = async (): Promise<CommandDto[]> => {
  const response = await api.listCommands({ limit: 100 })
  return response.commands || []
}

export function CommandsTab() {
  const { t } = useTranslation(['common', 'commands'])
  const { toast } = useToast()

  // Expandable details state
  const [expandedDetails, setExpandedDetails] = useState<Set<string>>(new Set())

  const { data: commands, loading, refetch } = useApiData(
    () => fetchCommands(),
    { deps: [] }
  )

  const handleRetry = async (id: string) => {
    try {
      await api.retryCommand(id)
      toast({ title: t('commands:retrySuccess'), description: t('commands:retryDesc') })
      refetch()
    } catch (error) {
      toast({ title: t('commands:retryFailed'), description: (error as Error).message || t('commands:retryFailedDesc'), variant: 'destructive' })
    }
  }

  const handleCancel = async (id: string) => {
    try {
      await api.cancelCommand(id)
      toast({ title: t('commands:cancelSuccess'), description: t('commands:cancelDesc') })
      refetch()
    } catch (error) {
      toast({ title: t('commands:cancelFailed'), description: (error as Error).message || t('commands:cancelFailedDesc'), variant: 'destructive' })
    }
  }

  const toggleDetails = (id: string) => {
    setExpandedDetails((prev) => {
      const next = new Set(prev)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })
  }

  const getStatusBadge = (status: string) => {
    const variantMap: Record<string, 'default' | 'secondary' | 'destructive' | 'outline'> = {
      Pending: 'default',
      Queued: 'secondary',
      Sending: 'secondary',
      WaitingAck: 'secondary',
      Completed: 'outline',
      Failed: 'destructive',
      Cancelled: 'outline',
      Timeout: 'destructive',
    }
    const labelMap: Record<string, string> = {
      Pending: t('commands:pending'),
      Queued: t('commands:queued'),
      Sending: t('commands:sending'),
      WaitingAck: t('commands:waitingAck'),
      Completed: t('commands:completed'),
      Failed: t('commands:failed'),
      Cancelled: t('commands:cancelled'),
      Timeout: t('commands:timeout'),
    }
    return (
      <Badge variant={variantMap[status] || 'secondary'}>
        {labelMap[status] || status}
      </Badge>
    )
  }

  const getPriorityBadge = (priority: string) => {
    const colorMap: Record<string, string> = {
      low: 'bg-blue-500',
      normal: 'bg-gray-500',
      high: 'bg-orange-500',
      critical: 'bg-red-500',
      emergency: 'bg-red-600',
    }
    const labelMap: Record<string, string> = {
      low: t('commands:priorityLow'),
      normal: t('commands:priorityNormal'),
      high: t('commands:priorityHigh'),
      critical: t('commands:priorityCritical'),
      emergency: t('commands:priorityEmergency'),
    }
    return (
      <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium text-white ${colorMap[priority] || 'bg-gray-500'}`}>
        {labelMap[priority] || priority}
      </span>
    )
  }

  return (
    <div className="space-y-4">
      {/* Desktop Table */}
      <Card className="hidden md:block">
        <table className="w-full caption-bottom text-sm">
          <thead className="[&_tr]:border-b">
            <tr>
              <th className="h-11 px-4 align-middle text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t('commands:command')}</th>
              <th className="h-11 px-4 align-middle text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t('commands:status')}</th>
              <th className="h-11 px-4 align-middle text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t('commands:priority')}</th>
              <th className="h-11 px-4 align-middle text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t('commands:device')}</th>
              <th className="h-11 px-4 align-middle text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t('commands:createdAt')}</th>
              <th className="h-11 px-4 align-middle text-xs font-semibold uppercase tracking-wide text-muted-foreground text-right">{t('automation:actions')}</th>
            </tr>
          </thead>
          <tbody className="[&_tr:last-child]:border-0">
            {loading ? (
              <tr><td colSpan={6} className="p-8 text-center text-muted-foreground">{t('commands:loading')}</td></tr>
            ) : !commands || commands.length === 0 ? (
              <tr><td colSpan={6} className="p-8 text-center text-muted-foreground">{`${t('commands:noCommands')} - ${t('commands:noCommandsDesc')}`}</td></tr>
            ) : (
              commands.map((command) => {
                const isExpanded = expandedDetails.has(command.id)
                const hasDetails = (command.params && Object.keys(command.params).length > 0) || command.result

                return (
                  <>
                    <tr
                      key={command.id}
                      className={`border-b transition-colors hover:bg-muted/50 ${
                        command.status === 'Pending' || command.status === 'Queued'
                          ? 'bg-blue-500/5'
                          : command.status === 'Failed' || command.status === 'Timeout'
                          ? 'bg-red-500/5'
                          : ''
                      }`}
                    >
                      <td className="p-4 align-middle">
                        <div className="max-w-xs">
                          <div className="font-mono text-sm">{command.command}</div>
                          <div className="text-xs text-muted-foreground">
                            {t('commands:commandId')}: {command.id.slice(0, 8)}...
                          </div>
                        </div>
                      </td>
                      <td className="p-4 align-middle">
                        <div className="flex flex-col gap-1">
                          {getStatusBadge(command.status)}
                          {command.attempt > 1 && (
                            <span className="text-xs text-orange-600">
                              {t('commands:attempt')} {command.attempt}
                            </span>
                          )}
                        </div>
                      </td>
                      <td className="p-4 align-middle">
                        {getPriorityBadge(command.priority)}
                      </td>
                      <td className="p-4 align-middle">
                        <div className="text-sm">
                          <div className="font-mono">{command.device_id}</div>
                          <div className="text-xs text-muted-foreground">{command.source_type}</div>
                        </div>
                      </td>
                      <td className="p-4 align-middle text-sm text-muted-foreground">
                        <div className="flex flex-col gap-1">
                          <div>{formatTimestamp(command.created_at)}</div>
                          {command.executed_at && (
                            <div className="text-xs">
                              {t('commands:executedAt')}: {formatTimestamp(command.executed_at)}
                            </div>
                          )}
                        </div>
                      </td>
                      <td className="p-4 align-middle text-right">
                        <div className="flex items-center justify-end gap-1">
                          {hasDetails && (
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-8 w-8"
                              onClick={() => toggleDetails(command.id)}
                            >
                              {isExpanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                            </Button>
                          )}
                          {(command.status === 'Failed' || command.status === 'Timeout') && (
                            <Button
                              onClick={() => handleRetry(command.id)}
                              variant="outline"
                              size="sm"
                              className="h-8"
                            >
                              <RotateCcw className="h-3 w-3 mr-1" />
                              {t('commands:retry')}
                            </Button>
                          )}
                          {(command.status === 'Pending' || command.status === 'Queued') && (
                            <Button
                              onClick={() => handleCancel(command.id)}
                              variant="outline"
                              size="sm"
                              className="h-8"
                            >
                              {t('commands:cancel')}
                            </Button>
                          )}
                        </div>
                      </td>
                    </tr>

                    {/* Expandable details row */}
                    {isExpanded && hasDetails && (
                      <tr key={`${command.id}-details`}>
                        <td colSpan={6} className="bg-muted/30">
                          <div className="space-y-3 py-2">
                            {command.params && Object.keys(command.params).length > 0 && (
                              <div>
                                <div className="text-sm font-medium mb-1">{t('commands:parameters')}</div>
                                <pre className="p-2 bg-background rounded text-xs overflow-x-auto">
                                  {JSON.stringify(command.params, null, 2)}
                                </pre>
                              </div>
                            )}
                            {command.result && (
                              <div>
                                <div className="text-sm font-medium mb-1">{t('commands:result')}</div>
                                <div className="p-2 bg-background rounded text-xs">
                                  <div className="text-muted-foreground">{command.result.message}</div>
                                  {command.result.response_data && (
                                    <pre className="mt-2 p-2 bg-muted rounded overflow-x-auto">
                                      {JSON.stringify(command.result.response_data, null, 2)}
                                    </pre>
                                  )}
                                </div>
                              </div>
                            )}
                          </div>
                        </td>
                      </tr>
                    )}
                  </>
                )
              })
            )}
          </tbody>
        </table>
      </Card>

      {/* Mobile Card List */}
      <div className="md:hidden space-y-3">
        {loading ? (
          <div className="p-8 text-center text-muted-foreground">{t('commands:loading')}</div>
        ) : !commands || commands.length === 0 ? (
          <div className="p-8 text-center text-muted-foreground">{`${t('commands:noCommands')} - ${t('commands:noCommandsDesc')}`}</div>
        ) : (
          commands.map((command) => {
            const isExpanded = expandedDetails.has(command.id)
            const hasDetails = (command.params && Object.keys(command.params).length > 0) || command.result
            const canRetry = command.status === 'Failed' || command.status === 'Timeout'
            const canCancel = command.status === 'Pending' || command.status === 'Queued'

            return (
              <Card
                key={command.id}
                className={`overflow-hidden ${
                  command.status === 'Pending' || command.status === 'Queued'
                    ? 'border-blue-500/30'
                    : command.status === 'Failed' || command.status === 'Timeout'
                    ? 'border-red-500/30'
                    : ''
                }`}
              >
                {/* Card Header */}
                <div className="bg-muted/30 px-4 py-3 border-b">
                  <div className="flex items-center gap-3">
                    <div className="flex-1 min-w-0">
                      <div className="font-mono text-sm truncate">{command.command}</div>
                      <div className="text-xs text-muted-foreground">
                        {command.device_id} · {command.id.slice(0, 8)}...
                      </div>
                    </div>
                    {getStatusBadge(command.status)}
                  </div>
                </div>

                {/* Card Body */}
                <div className="p-4 space-y-3">
                  {/* Priority */}
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-muted-foreground">{t('commands:priority')}</span>
                    {getPriorityBadge(command.priority)}
                  </div>

                  {/* Attempt count */}
                  {command.attempt > 1 && (
                    <div className="flex items-center justify-between">
                      <span className="text-xs text-muted-foreground">{t('commands:attempt')}</span>
                      <span className="text-xs text-orange-600">{command.attempt}</span>
                    </div>
                  )}

                  {/* Created at */}
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-muted-foreground">{t('commands:createdAt')}</span>
                    <span className="text-xs text-muted-foreground">{formatTimestamp(command.created_at)}</span>
                  </div>

                  {/* Expand button */}
                  {hasDetails && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => toggleDetails(command.id)}
                      className="w-full justify-between"
                    >
                      <span className="text-xs">{isExpanded ? t('common:hide') : t('common:show')} {t('commands:details')}</span>
                      {isExpanded ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                    </Button>
                  )}

                  {/* Expanded details */}
                  {isExpanded && hasDetails && (
                    <div className="space-y-3 pt-3 border-t">
                      {command.params && Object.keys(command.params).length > 0 && (
                        <div>
                          <div className="text-xs font-medium mb-2">{t('commands:parameters')}</div>
                          <pre className="p-2 bg-muted rounded text-xs overflow-x-auto">
                            {JSON.stringify(command.params, null, 2)}
                          </pre>
                        </div>
                      )}
                      {command.result && (
                        <div>
                          <div className="text-xs font-medium mb-2">{t('commands:result')}</div>
                          <div className="p-2 bg-muted rounded text-xs">
                            <div className="text-muted-foreground">{command.result.message}</div>
                            {command.result.response_data && (
                              <pre className="mt-2 p-2 bg-background rounded overflow-x-auto">
                                {JSON.stringify(command.result.response_data, null, 2)}
                              </pre>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  )}

                  {/* Actions */}
                  <div className="flex items-center justify-end gap-2 pt-2 border-t">
                    {canRetry && (
                      <Button
                        onClick={() => handleRetry(command.id)}
                        variant="outline"
                        size="sm"
                      >
                        <RotateCcw className="h-3 w-3 mr-1" />
                        {t('commands:retry')}
                      </Button>
                    )}
                    {canCancel && (
                      <Button
                        onClick={() => handleCancel(command.id)}
                        variant="outline"
                        size="sm"
                      >
                        {t('commands:cancel')}
                      </Button>
                    )}
                  </div>
                </div>
              </Card>
            )
          })
        )}
      </div>
    </div>
  )
}
