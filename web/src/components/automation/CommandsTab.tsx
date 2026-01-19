import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/lib/api'
import type { CommandDto } from '@/types'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { EmptyStateInline, ActionBar } from '@/components/shared'
import { useApiData } from '@/hooks/useApiData'
import { formatTimestamp } from '@/lib/utils/format'
import { useToast } from '@/hooks/use-toast'
import { Terminal, RotateCcw, ChevronDown, ChevronUp } from 'lucide-react'

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
      {/* Header */}
      <ActionBar
        title={t('automation:commands')}
        titleIcon={<Terminal className="h-5 w-5" />}
        description={t('automation:commandsDesc')}
        onRefresh={refetch}
      />

      {/* Table */}
      <Card>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t('commands:command')}</TableHead>
              <TableHead>{t('commands:status')}</TableHead>
              <TableHead>{t('commands:priority')}</TableHead>
              <TableHead>{t('commands:device')}</TableHead>
              <TableHead>{t('commands:createdAt')}</TableHead>
              <TableHead align="right">{t('automation:actions')}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <EmptyStateInline title={t('commands:loading')} colSpan={6} />
            ) : !commands || commands.length === 0 ? (
              <EmptyStateInline title={`${t('commands:noCommands')} - ${t('commands:noCommandsDesc')}`} colSpan={6} />
            ) : (
              commands.map((command) => {
                const isExpanded = expandedDetails.has(command.id)
                const hasDetails = (command.params && Object.keys(command.params).length > 0) || command.result

                return (
                  <>
                    <TableRow
                      key={command.id}
                      className={
                        command.status === 'Pending' || command.status === 'Queued'
                          ? 'bg-blue-500/5'
                          : command.status === 'Failed' || command.status === 'Timeout'
                          ? 'bg-red-500/5'
                          : ''
                      }
                    >
                      <TableCell>
                        <div className="max-w-xs">
                          <div className="font-mono text-sm">{command.command}</div>
                          <div className="text-xs text-muted-foreground">
                            {t('commands:commandId')}: {command.id.slice(0, 8)}...
                          </div>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-col gap-1">
                          {getStatusBadge(command.status)}
                          {command.attempt > 1 && (
                            <span className="text-xs text-orange-600">
                              {t('commands:attempt')} {command.attempt}
                            </span>
                          )}
                        </div>
                      </TableCell>
                      <TableCell>
                        {getPriorityBadge(command.priority)}
                      </TableCell>
                      <TableCell>
                        <div className="text-sm">
                          <div className="font-mono">{command.device_id}</div>
                          <div className="text-xs text-muted-foreground">{command.source_type}</div>
                        </div>
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        <div className="flex flex-col gap-1">
                          <div>{formatTimestamp(command.created_at)}</div>
                          {command.executed_at && (
                            <div className="text-xs">
                              {t('commands:executedAt')}: {formatTimestamp(command.executed_at)}
                            </div>
                          )}
                        </div>
                      </TableCell>
                      <TableCell align="right">
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
                      </TableCell>
                    </TableRow>

                    {/* Expandable details row */}
                    {isExpanded && hasDetails && (
                      <TableRow key={`${command.id}-details`}>
                        <TableCell colSpan={6} className="bg-muted/30">
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
                        </TableCell>
                      </TableRow>
                    )}
                  </>
                )
              })
            )}
          </TableBody>
        </Table>
      </Card>
    </div>
  )
}
