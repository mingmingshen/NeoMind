import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { EmptyStateInline } from "@/components/shared"
import { Clock, AlertCircle, FileText } from "lucide-react"
import { useTranslation } from "react-i18next"
import { formatTimestamp } from "@/lib/utils/format"
import { useIsMobile } from "@/hooks/useMobile"
import { Card } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"
import { textNano } from "@/design-system/tokens/typography"
import type { AgentExecution } from "@/types"

interface AgentExecutionsListProps {
  executions: AgentExecution[]
  loading: boolean
  agentId: string
  onViewDetail?: (agentId: string, executionId: string) => void
}

export function AgentExecutionsList({
  executions,
  loading,
  agentId,
  onViewDetail,
}: AgentExecutionsListProps) {
  const { t } = useTranslation(['common', 'agents'])
  const isMobile = useIsMobile()

  const getStatusLabel = (status: string) => {
    switch (status) {
      case 'Running': return t('agents:executionStatus.running')
      case 'Completed': return t('agents:executionStatus.completed')
      case 'Failed': return t('agents:executionStatus.failed')
      case 'Cancelled': return t('agents:executionStatus.cancelled')
      default: return status
    }
  }

  const getStatusClass = (status: string) => {
    switch (status) {
      case 'Running': return 'bg-info-light text-info'
      case 'Completed': return 'bg-success-light text-success dark:bg-success-light dark:text-success'
      case 'Failed': return 'bg-error-light text-error'
      case 'Cancelled': return 'bg-muted text-muted-foreground'
      default: return 'bg-muted text-muted-foreground'
    }
  }

  if (isMobile) {
    return (
      <div className="space-y-2">
        {loading ? (
          Array.from({ length: 3 }).map((_, i) => (
            <Card key={i} className="overflow-hidden border-border">
              <div className="p-3 space-y-2">
                <div className="flex items-center gap-2">
                  <div className="w-12 h-5 bg-muted rounded animate-pulse" />
                  <div className="flex-1" />
                  <div className="w-14 h-5 bg-muted rounded animate-pulse" />
                </div>
                <div className="h-3 bg-muted rounded w-2/3 animate-pulse" />
              </div>
            </Card>
          ))
        ) : executions.length === 0 ? (
          <div className="flex flex-col items-center justify-center text-center py-8">
            <Clock className="h-8 w-8 text-muted-foreground mb-2" />
            <p className="text-sm text-muted-foreground">{t('agents:noExecutions')}</p>
          </div>
        ) : (
          executions.map((execution, index) => (
            <Card
              key={execution.id}
              className={cn(
                "overflow-hidden border-border shadow-sm transition-all",
                onViewDetail && "cursor-pointer active:scale-[0.99]"
              )}
              onClick={() => onViewDetail?.(agentId, execution.id)}
            >
              <div className="px-3 py-2.5">
                {/* Row 1: index + timestamp + status */}
                <div className="flex items-center gap-2">
                  <span className="text-xs text-muted-foreground">{index + 1}</span>
                  <span className="text-xs text-muted-foreground flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {formatTimestamp(execution.timestamp, false)}
                  </span>
                  <span className={cn(
                    "inline-flex items-center px-1.5 py-0.5 rounded", textNano, "font-medium ml-auto",
                    getStatusClass(execution.status)
                  )}>
                    {getStatusLabel(execution.status)}
                  </span>
                  {onViewDetail && (
                    <FileText className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                  )}
                </div>
                {/* Row 2: trigger + duration + error */}
                <div className="flex items-center gap-1.5 mt-1.5">
                  <Badge variant="outline" className={cn(textNano, "h-5 px-1.5 shrink-0")}>
                    {execution.trigger_type}
                  </Badge>
                  {execution.duration_ms > 0 && (
                    <span className={cn(textNano, "text-muted-foreground")}>
                      {execution.duration_ms}ms
                    </span>
                  )}
                  {execution.error && (
                    <span className={cn(textNano, "text-destructive truncate min-w-0")}>
                      {execution.error}
                    </span>
                  )}
                </div>
              </div>
            </Card>
          ))
        )}
      </div>
    )
  }

  return (
    <div className="p-4">
      <div className="overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-12">#</TableHead>
              <TableHead>{t('agents:timestamp')}</TableHead>
              <TableHead>{t('agents:triggerType')}</TableHead>
              <TableHead>{t('agents:status')}</TableHead>
              <TableHead>{t('agents:duration')}</TableHead>
              <TableHead>{t('agents:error')}</TableHead>
              {onViewDetail && <TableHead className="w-16"></TableHead>}
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <EmptyStateInline
                title={t('common:loading')}
                colSpan={onViewDetail ? 7 : 6}
              />
            ) : executions.length === 0 ? (
              <EmptyStateInline
                title={t('agents:noExecutions')}
                colSpan={onViewDetail ? 7 : 6}
              />
            ) : (
              executions.map((execution, index) => (
                <TableRow key={execution.id} className={onViewDetail ? "cursor-pointer hover:bg-muted-50" : ""}>
                  <TableCell className="text-muted-foreground">{index + 1}</TableCell>
                  <TableCell className="text-sm">
                    <div className="flex items-center gap-2">
                      <Clock className="h-4 w-4 text-muted-foreground" />
                      {formatTimestamp(execution.timestamp, false)}
                    </div>
                  </TableCell>
                  <TableCell>
                    <code className="text-xs bg-muted px-2 py-1 rounded">
                      {execution.trigger_type}
                    </code>
                  </TableCell>
                  <TableCell>
                    <span className={`inline-flex items-center px-2 py-1 rounded text-xs font-medium ${getStatusClass(execution.status)}`}>
                      {getStatusLabel(execution.status)}
                    </span>
                  </TableCell>
                  <TableCell className="text-sm">
                    {execution.duration_ms > 0 ? `${execution.duration_ms}ms` : '-'}
                  </TableCell>
                  <TableCell className="text-sm">
                    {execution.error ? (
                      <div className="flex items-center gap-1 text-destructive">
                        <AlertCircle className="h-4 w-4" />
                        <span className="truncate max-w-xs">{execution.error}</span>
                      </div>
                    ) : '-'}
                  </TableCell>
                  {onViewDetail && (
                    <TableCell className="text-right">
                      <button
                        onClick={() => onViewDetail(agentId, execution.id)}
                        className="p-1 hover:bg-muted rounded transition-colors"
                        title={t('agents:viewDetails') || "View Details"}
                      >
                        <FileText className="h-4 w-4 text-muted-foreground" />
                      </button>
                    </TableCell>
                  )}
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  )
}
